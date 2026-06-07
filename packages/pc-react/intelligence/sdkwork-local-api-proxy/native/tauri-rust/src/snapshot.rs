use crate::{
    constants::{
        LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION, LOCAL_API_PROXY_DEFAULT_BIND_HOST,
        LOCAL_API_PROXY_DEFAULT_CLIENT_API_KEY, LOCAL_API_PROXY_DEFAULT_CLIENT_PROTOCOL,
        LOCAL_API_PROXY_DEFAULT_PORT, LOCAL_API_PROXY_DEFAULT_PROVIDER_ID,
        LOCAL_API_PROXY_DEFAULT_ROUTE_ID, LOCAL_API_PROXY_DEFAULT_UPSTREAM_BASE_URL,
        LOCAL_API_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION,
        LOCAL_API_PROXY_PROVIDER_CENTER_NAMESPACE, LOCAL_API_PROXY_SCHEMA_VERSION,
    },
    error::{LocalApiProxyNativeError, LocalApiProxyNativeResult},
};
use serde_json::{Number, Value};
use std::{collections::BTreeMap, fs, path::Path};

const LOCAL_AI_PROXY_REQUIRED_DEFAULT_CLIENT_PROTOCOLS: [&str; 3] =
    ["anthropic", "gemini", "openai-compatible"];

pub const LOCAL_AI_PROXY_SCHEMA_VERSION: u32 = LOCAL_API_PROXY_SCHEMA_VERSION;
pub const LOCAL_AI_PROXY_CONFIG_SCHEMA_VERSION: u32 = LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION;
pub const LOCAL_AI_PROXY_DEFAULT_BIND_HOST: &str = LOCAL_API_PROXY_DEFAULT_BIND_HOST;
pub const LOCAL_AI_PROXY_DEFAULT_PORT: u16 = LOCAL_API_PROXY_DEFAULT_PORT;
pub const LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL: &str =
    LOCAL_API_PROXY_DEFAULT_UPSTREAM_BASE_URL;
pub const LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY: &str = LOCAL_API_PROXY_DEFAULT_CLIENT_API_KEY;
pub const LOCAL_AI_PROXY_PROVIDER_CENTER_NAMESPACE: &str =
    LOCAL_API_PROXY_PROVIDER_CENTER_NAMESPACE;
pub const LOCAL_AI_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION: u32 =
    LOCAL_API_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION;
pub const LOCAL_AI_PROXY_DEFAULT_ROUTE_ID: &str = LOCAL_API_PROXY_DEFAULT_ROUTE_ID;
pub const LOCAL_AI_PROXY_DEFAULT_PROVIDER_ID: &str = LOCAL_API_PROXY_DEFAULT_PROVIDER_ID;
pub const LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL: &str = LOCAL_API_PROXY_DEFAULT_CLIENT_PROTOCOL;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyModelSnapshot {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyRouteRuntimeConfigSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
}

fn local_ai_proxy_runtime_config_is_empty(value: &LocalAiProxyRouteRuntimeConfigSnapshot) -> bool {
    value.temperature.is_none()
        && value.top_p.is_none()
        && value.max_tokens.is_none()
        && value.timeout_ms.is_none()
        && value.streaming.is_none()
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyRouteSnapshot {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub is_default: bool,
    pub managed_by: String,
    pub client_protocol: String,
    pub upstream_protocol: String,
    pub provider_id: String,
    pub upstream_base_url: String,
    pub api_key: String,
    pub default_model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model_id: Option<String>,
    pub models: Vec<LocalAiProxyModelSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub expose_to: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "local_ai_proxy_runtime_config_is_empty"
    )]
    pub runtime_config: LocalAiProxyRouteRuntimeConfigSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxySnapshot {
    pub schema_version: u32,
    pub bind_host: String,
    pub requested_port: u16,
    pub auth_token: String,
    pub default_route_id: String,
    pub routes: Vec<LocalAiProxyRouteSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyProviderCenterRouteRecord {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyProviderCenterCatalogSnapshot {
    pub schema_version: u32,
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub routes: Vec<LocalAiProxyProviderCenterRouteRecord>,
}

impl LocalAiProxySnapshot {
    pub fn route_for_client_protocol(
        &self,
        client_protocol: &str,
    ) -> Option<&LocalAiProxyRouteSnapshot> {
        self.routes
            .iter()
            .find(|route| {
                route.enabled && route.client_protocol == client_protocol && route.is_default
            })
            .or_else(|| {
                self.routes
                    .iter()
                    .find(|route| route.enabled && route.client_protocol == client_protocol)
            })
    }

    pub fn default_route(&self) -> Option<&LocalAiProxyRouteSnapshot> {
        self.route_for_client_protocol(LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL)
            .or_else(|| {
                self.routes
                    .iter()
                    .find(|route| route.enabled && route.is_default)
            })
            .or_else(|| self.routes.iter().find(|route| route.enabled))
    }
}

pub fn create_system_default_local_ai_proxy_snapshot(
    requested_port: u16,
    auth_token: impl Into<String>,
) -> LocalAiProxySnapshot {
    LocalAiProxySnapshot {
        schema_version: LOCAL_AI_PROXY_SCHEMA_VERSION,
        bind_host: LOCAL_AI_PROXY_DEFAULT_BIND_HOST.to_string(),
        requested_port,
        auth_token: auth_token.into(),
        default_route_id: LOCAL_AI_PROXY_DEFAULT_ROUTE_ID.to_string(),
        routes: create_required_system_default_local_ai_proxy_routes(),
    }
}

pub fn materialize_local_ai_proxy_snapshot_from_routes(
    routes: Vec<LocalAiProxyRouteSnapshot>,
    requested_port: u16,
    auth_token: impl Into<String>,
) -> LocalAiProxySnapshot {
    let auth_token = auth_token.into();
    if routes.is_empty() {
        return create_system_default_local_ai_proxy_snapshot(requested_port, auth_token);
    }

    let normalized_routes = normalize_local_ai_proxy_routes(routes);
    let default_route_id = normalized_routes
        .iter()
        .find(|route| {
            route.enabled
                && route.client_protocol == LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL
                && route.is_default
        })
        .or_else(|| {
            normalized_routes.iter().find(|route| {
                route.enabled && route.client_protocol == LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL
            })
        })
        .or_else(|| {
            normalized_routes
                .iter()
                .find(|route| route.enabled && route.is_default)
        })
        .or_else(|| normalized_routes.iter().find(|route| route.enabled))
        .map(|route| route.id.clone())
        .unwrap_or_else(|| LOCAL_AI_PROXY_DEFAULT_ROUTE_ID.to_string());

    LocalAiProxySnapshot {
        schema_version: LOCAL_AI_PROXY_SCHEMA_VERSION,
        bind_host: LOCAL_AI_PROXY_DEFAULT_BIND_HOST.to_string(),
        requested_port,
        auth_token,
        default_route_id,
        routes: normalized_routes,
    }
}

pub fn write_local_ai_proxy_snapshot(
    path: &Path,
    snapshot: &LocalAiProxySnapshot,
) -> LocalApiProxyNativeResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(snapshot)?),
    )?;
    Ok(())
}

pub fn load_local_ai_proxy_snapshot(
    path: &Path,
) -> LocalApiProxyNativeResult<LocalAiProxySnapshot> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn create_local_ai_proxy_provider_center_catalog(
    profile_id: Option<String>,
    mut routes: Vec<LocalAiProxyProviderCenterRouteRecord>,
) -> LocalAiProxyProviderCenterCatalogSnapshot {
    routes.sort_by(|left, right| left.key.cmp(&right.key));

    LocalAiProxyProviderCenterCatalogSnapshot {
        schema_version: LOCAL_AI_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION,
        namespace: LOCAL_AI_PROXY_PROVIDER_CENTER_NAMESPACE.to_string(),
        profile_id,
        routes,
    }
}

pub fn validate_local_ai_proxy_provider_center_catalog(
    catalog: &LocalAiProxyProviderCenterCatalogSnapshot,
) -> LocalApiProxyNativeResult<()> {
    if catalog.schema_version != LOCAL_AI_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION {
        return Err(LocalApiProxyNativeError::ValidationFailed(format!(
            "unsupported local ai proxy provider center catalog schema version: {}",
            catalog.schema_version
        )));
    }

    if catalog.namespace != LOCAL_AI_PROXY_PROVIDER_CENTER_NAMESPACE {
        return Err(LocalApiProxyNativeError::ValidationFailed(format!(
            "unsupported local ai proxy provider center catalog namespace: {}",
            catalog.namespace
        )));
    }

    Ok(())
}

pub fn parse_local_ai_proxy_route_snapshot(value: &str) -> Option<LocalAiProxyRouteSnapshot> {
    let parsed = serde_json::from_str::<Value>(value).ok()?;
    let object = parsed.as_object()?;
    let managed_by =
        normalize_string(object.get("managedBy")).unwrap_or_else(|| "user".to_string());
    let provider_id = normalize_provider_id(
        object
            .get("providerId")
            .or_else(|| object.get("channelId"))
            .or_else(|| object.get("id")),
    );
    let client_protocol = normalize_client_protocol(object.get("clientProtocol"))
        .unwrap_or_else(|| infer_client_protocol(&provider_id));
    if managed_by == "system-default" {
        return Some(create_system_default_local_ai_proxy_route_for_protocol(
            &client_protocol,
        ));
    }

    let upstream_protocol = normalize_upstream_protocol(object.get("upstreamProtocol"))
        .unwrap_or_else(|| infer_upstream_protocol(&provider_id));
    let mut models = normalize_models(object.get("models"));
    let default_model_id = normalize_string(object.get("defaultModelId"))
        .or_else(|| models.first().map(|model| model.id.clone()))
        .unwrap_or_default();
    if !default_model_id.is_empty() && !models.iter().any(|model| model.id == default_model_id) {
        models.push(LocalAiProxyModelSnapshot {
            id: default_model_id.clone(),
            name: default_model_id.clone(),
        });
    }
    let reasoning_model_id = normalize_optional_model_id(object.get("reasoningModelId"), &models);
    let embedding_model_id = normalize_optional_model_id(object.get("embeddingModelId"), &models);
    let runtime_config = normalize_runtime_config(object.get("config"));

    Some(LocalAiProxyRouteSnapshot {
        id: normalize_string(object.get("id"))
            .unwrap_or_else(|| format!("local-ai-route-{}", provider_id)),
        name: normalize_string(object.get("name"))
            .unwrap_or_else(|| titleize(&provider_id))
            .trim()
            .to_string(),
        enabled: object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        is_default: object
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        managed_by,
        client_protocol,
        upstream_protocol,
        provider_id: provider_id.clone(),
        upstream_base_url: normalize_string(object.get("upstreamBaseUrl"))
            .or_else(|| normalize_string(object.get("baseUrl")))
            .unwrap_or_else(|| LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL.to_string()),
        api_key: normalize_string(object.get("apiKey")).unwrap_or_default(),
        default_model_id,
        reasoning_model_id,
        embedding_model_id,
        models,
        notes: normalize_string(object.get("notes")),
        expose_to: normalize_string_list(object.get("exposeTo"), "sdkwork"),
        runtime_config,
    })
}

pub fn normalize_local_ai_proxy_routes(
    routes: Vec<LocalAiProxyRouteSnapshot>,
) -> Vec<LocalAiProxyRouteSnapshot> {
    let mut grouped = BTreeMap::<String, Vec<LocalAiProxyRouteSnapshot>>::new();
    for route in routes {
        grouped
            .entry(route.client_protocol.clone())
            .or_default()
            .push(route);
    }

    let mut normalized = Vec::new();
    for (_client_protocol, mut protocol_routes) in grouped {
        let default_index = protocol_routes
            .iter()
            .position(|route| route.enabled && route.is_default)
            .or_else(|| protocol_routes.iter().position(|route| route.enabled));
        if let Some(default_index) = default_index {
            for (index, route) in protocol_routes.iter_mut().enumerate() {
                route.is_default = index == default_index;
            }
        }
        protocol_routes.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.managed_by.cmp(&right.managed_by))
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });

        normalized.extend(protocol_routes);
    }

    for client_protocol in LOCAL_AI_PROXY_REQUIRED_DEFAULT_CLIENT_PROTOCOLS {
        let has_enabled_default = normalized.iter().any(|route| {
            route.client_protocol == client_protocol && route.enabled && route.is_default
        });
        if has_enabled_default {
            continue;
        }

        normalized.push(create_system_default_local_ai_proxy_route_for_protocol(
            client_protocol,
        ));
    }

    normalized.sort_by(|left, right| {
        left.client_protocol
            .cmp(&right.client_protocol)
            .then_with(|| right.is_default.cmp(&left.is_default))
            .then_with(|| left.managed_by.cmp(&right.managed_by))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    normalized
}

fn create_required_system_default_local_ai_proxy_routes() -> Vec<LocalAiProxyRouteSnapshot> {
    LOCAL_AI_PROXY_REQUIRED_DEFAULT_CLIENT_PROTOCOLS
        .iter()
        .map(|client_protocol| {
            create_system_default_local_ai_proxy_route_for_protocol(client_protocol)
        })
        .collect()
}

fn create_system_default_local_ai_proxy_route_for_protocol(
    client_protocol: &str,
) -> LocalAiProxyRouteSnapshot {
    let models = vec![
        LocalAiProxyModelSnapshot {
            id: "sdkwork-chat".to_string(),
            name: "SDKWork Chat".to_string(),
        },
        LocalAiProxyModelSnapshot {
            id: "sdkwork-reasoning".to_string(),
            name: "SDKWork Reasoning".to_string(),
        },
        LocalAiProxyModelSnapshot {
            id: "sdkwork-embedding".to_string(),
            name: "SDKWork Embedding".to_string(),
        },
    ];

    LocalAiProxyRouteSnapshot {
        id: if client_protocol == LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL {
            LOCAL_AI_PROXY_DEFAULT_ROUTE_ID.to_string()
        } else {
            format!("local-ai-proxy-system-default-{client_protocol}")
        },
        name: if client_protocol == LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL {
            "SDKWork Default".to_string()
        } else {
            format!("SDKWork {} Default", titleize(client_protocol))
        },
        enabled: true,
        is_default: true,
        managed_by: "system-default".to_string(),
        client_protocol: client_protocol.to_string(),
        upstream_protocol: "sdkwork".to_string(),
        provider_id: LOCAL_AI_PROXY_DEFAULT_PROVIDER_ID.to_string(),
        upstream_base_url: LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL.to_string(),
        api_key: LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY.to_string(),
        default_model_id: models[0].id.clone(),
        reasoning_model_id: Some(models[1].id.clone()),
        embedding_model_id: Some(models[2].id.clone()),
        models,
        notes: None,
        expose_to: if client_protocol == LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL {
            vec!["sdkwork".to_string()]
        } else {
            vec!["desktop-clients".to_string(), "sdkwork".to_string()]
        },
        runtime_config: LocalAiProxyRouteRuntimeConfigSnapshot {
            temperature: Number::from_f64(0.2),
            top_p: Number::from_f64(1.0),
            max_tokens: Some(8_192),
            timeout_ms: Some(60_000),
            streaming: Some(true),
        },
    }
}

fn normalize_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_provider_id(value: Option<&Value>) -> String {
    normalize_string(value)
        .unwrap_or_else(|| LOCAL_AI_PROXY_DEFAULT_PROVIDER_ID.to_string())
        .trim()
        .to_lowercase()
}

fn normalize_client_protocol(value: Option<&Value>) -> Option<String> {
    let normalized = normalize_string(value)?;
    match normalized.as_str() {
        "openai-compatible" | "anthropic" | "gemini" => Some(normalized),
        _ => None,
    }
}

fn normalize_upstream_protocol(value: Option<&Value>) -> Option<String> {
    let normalized = normalize_string(value)?;
    match normalized.as_str() {
        "openai-compatible" | "anthropic" | "gemini" | "ollama" | "azure-openai" | "openrouter"
        | "sdkwork" => Some(normalized),
        _ => None,
    }
}

fn normalize_models(value: Option<&Value>) -> Vec<LocalAiProxyModelSnapshot> {
    let mut models = Vec::new();
    let Some(items) = value.and_then(Value::as_array) else {
        return models;
    };

    for entry in items {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let Some(id) = normalize_string(object.get("id")) else {
            continue;
        };
        if models.iter().any(|model| model.id == id) {
            continue;
        }
        models.push(LocalAiProxyModelSnapshot {
            name: normalize_string(object.get("name")).unwrap_or_else(|| id.clone()),
            id,
        });
    }

    models
}

fn normalize_optional_model_id(
    value: Option<&Value>,
    models: &[LocalAiProxyModelSnapshot],
) -> Option<String> {
    let normalized = normalize_string(value)?;
    models
        .iter()
        .any(|model| model.id == normalized)
        .then_some(normalized)
}

fn normalize_string_list(value: Option<&Value>, fallback: &str) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        return vec![fallback.to_string()];
    };

    let mut entries = items
        .iter()
        .filter_map(|entry| entry.as_str())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    entries.sort();
    entries.dedup();
    if entries.is_empty() {
        return vec![fallback.to_string()];
    }
    entries
}

fn normalize_json_number(value: Option<&Value>) -> Option<Number> {
    match value {
        Some(Value::Number(number)) => Some(number.clone()),
        Some(Value::String(raw)) => raw.trim().parse::<f64>().ok().and_then(Number::from_f64),
        _ => None,
    }
}

fn normalize_runtime_config(value: Option<&Value>) -> LocalAiProxyRouteRuntimeConfigSnapshot {
    let object = value.and_then(Value::as_object);

    LocalAiProxyRouteRuntimeConfigSnapshot {
        temperature: normalize_json_number(object.and_then(|object| object.get("temperature"))),
        top_p: normalize_json_number(object.and_then(|object| object.get("topP"))),
        max_tokens: object
            .and_then(|object| object.get("maxTokens"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        timeout_ms: object
            .and_then(|object| object.get("timeoutMs"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        streaming: object
            .and_then(|object| object.get("streaming"))
            .and_then(Value::as_bool),
    }
}

fn infer_upstream_protocol(provider_id: &str) -> String {
    match provider_id {
        "anthropic" => "anthropic".to_string(),
        "google" | "gemini" => "gemini".to_string(),
        "ollama" => "ollama".to_string(),
        "azure" | "azure-openai" => "azure-openai".to_string(),
        "openrouter" => "openrouter".to_string(),
        "sdkwork" => "sdkwork".to_string(),
        _ => "openai-compatible".to_string(),
    }
}

fn infer_client_protocol(provider_id: &str) -> String {
    match infer_upstream_protocol(provider_id).as_str() {
        "anthropic" => "anthropic".to_string(),
        "gemini" => "gemini".to_string(),
        _ => LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL.to_string(),
    }
}

fn titleize(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
