use std::sync::OnceLock;

use sdkwork_database_id::{max_snowflake_node_id, SnowflakeIdGenerator};

use crate::error::StoreError;

const DEFAULT_LOCAL_ROUTER_NODE_ID: u16 = 41;
const NODE_ID_ENV: &str = "SDKWORK_LR_SNOWFLAKE_NODE_ID";

static RUNTIME_ID_GENERATOR: OnceLock<SnowflakeIdGenerator> = OnceLock::new();

pub(crate) fn next_runtime_id(context: &str) -> Result<i64, StoreError> {
    let generator = runtime_id_generator()?;
    generator.generate().map_err(|error| {
        tracing::error!(
            context,
            node_id = generator.node_id(),
            error = ?error,
            "failed to generate local-router runtime id"
        );
        StoreError::Query(format!("failed to generate {context} id: {error:?}"))
    })
}

fn runtime_id_generator() -> Result<&'static SnowflakeIdGenerator, StoreError> {
    if let Some(generator) = RUNTIME_ID_GENERATOR.get() {
        return Ok(generator);
    }

    let node_id = runtime_node_id()?;
    Ok(RUNTIME_ID_GENERATOR.get_or_init(|| {
        SnowflakeIdGenerator::new(node_id).expect("validated local-router snowflake node id")
    }))
}

fn runtime_node_id() -> Result<u16, StoreError> {
    let Some(value) = std::env::var(NODE_ID_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return Ok(DEFAULT_LOCAL_ROUTER_NODE_ID);
    };

    parse_runtime_node_id(&value)
}

fn parse_runtime_node_id(value: &str) -> Result<u16, StoreError> {
    let node_id = value.parse::<u16>().map_err(|error| {
        StoreError::Config(format!(
            "{NODE_ID_ENV} must be an integer between 0 and {}, got '{}': {error}",
            max_snowflake_node_id(),
            value
        ))
    })?;

    if node_id > max_snowflake_node_id() {
        return Err(StoreError::Config(format!(
            "{NODE_ID_ENV} must be between 0 and {}, got {}",
            max_snowflake_node_id(),
            node_id
        )));
    }

    Ok(node_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_node_id_defaults_to_local_router_node() {
        assert_eq!(
            parse_runtime_node_id("41").unwrap(),
            DEFAULT_LOCAL_ROUTER_NODE_ID
        );
    }

    #[test]
    fn runtime_node_id_parser_rejects_invalid_env_value() {
        let error = parse_runtime_node_id("not-a-number").unwrap_err();

        assert!(error.to_string().contains(NODE_ID_ENV));
    }
}
