use crate::{
    constants::{
        LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION, LOCAL_API_PROXY_DEFAULT_BIND_HOST,
        LOCAL_API_PROXY_DEFAULT_CLIENT_API_KEY, LOCAL_API_PROXY_DEFAULT_PORT,
        LOCAL_API_PROXY_PUBLIC_BASE_HOST_CANDIDATES,
    },
    error::{LocalApiProxyNativeError, LocalApiProxyNativeResult},
};
use serde_json::Value;
use std::{
    fs,
    net::{IpAddr, ToSocketAddrs},
    path::Path,
};
use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyConfigFile {
    pub schema_version: u32,
    pub bind_host: String,
    pub public_base_host: String,
    pub requested_port: u16,
    pub client_api_key: String,
}

pub fn ensure_local_ai_proxy_client_api_key(
    config_file_path: &Path,
) -> LocalApiProxyNativeResult<String> {
    Ok(ensure_local_ai_proxy_config(config_file_path)?.client_api_key)
}

pub fn ensure_local_ai_proxy_config(
    config_file_path: &Path,
) -> LocalApiProxyNativeResult<LocalAiProxyConfigFile> {
    if !config_file_path.exists() {
        let config = create_default_local_ai_proxy_config();
        write_local_ai_proxy_config_file(config_file_path, &config)?;
        return Ok(config);
    }

    let content = fs::read_to_string(config_file_path)?;
    let parsed = json5::from_str::<Value>(&content).map_err(|error| {
        LocalApiProxyNativeError::ValidationFailed(format!(
            "invalid local api proxy config: {error}"
        ))
    })?;
    let object = parsed.as_object().ok_or_else(|| {
        LocalApiProxyNativeError::ValidationFailed(
            "local api proxy config must be a JSON object".to_string(),
        )
    })?;

    let raw_schema_version = object
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION);
    let raw_bind_host = object
        .get("bindHost")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let bind_host = raw_bind_host
        .clone()
        .unwrap_or_else(|| LOCAL_API_PROXY_DEFAULT_BIND_HOST.to_string());
    let raw_requested_port = object
        .get("requestedPort")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok());
    let requested_port = normalize_local_ai_proxy_requested_port(raw_requested_port);
    let existing_client_api_key = object
        .get("clientApiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let raw_public_base_host = object
        .get("publicBaseHost")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let client_api_key = existing_client_api_key
        .as_deref()
        .filter(|value| *value != LOCAL_API_PROXY_DEFAULT_CLIENT_API_KEY)
        .map(str::to_string)
        .unwrap_or_else(generate_local_ai_proxy_client_api_key);
    let public_base_host =
        normalize_local_ai_proxy_public_host(object.get("publicBaseHost").and_then(Value::as_str));

    let should_persist = raw_schema_version != LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION
        || raw_bind_host.as_deref() != Some(bind_host.as_str())
        || raw_requested_port != Some(requested_port)
        || raw_public_base_host.as_deref() != Some(public_base_host.as_str())
        || existing_client_api_key.as_deref() != Some(client_api_key.as_str());

    let config = LocalAiProxyConfigFile {
        schema_version: LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION,
        bind_host,
        public_base_host,
        requested_port,
        client_api_key,
    };

    if should_persist {
        write_local_ai_proxy_config_file(config_file_path, &config)?;
    }

    Ok(config)
}

pub fn default_local_ai_proxy_public_host() -> String {
    let mut resolver = resolve_local_ai_proxy_public_host_addresses;
    resolve_default_local_ai_proxy_public_host_with_resolver(&mut resolver)
}

pub fn resolve_default_local_ai_proxy_public_host_with_resolver<F>(resolver: &mut F) -> String
where
    F: FnMut(&str) -> Vec<IpAddr>,
{
    for candidate in LOCAL_API_PROXY_PUBLIC_BASE_HOST_CANDIDATES {
        if local_ai_proxy_public_host_is_loopback_safe_with_resolver(candidate, resolver) {
            return candidate.to_string();
        }
    }

    "127.0.0.1".to_string()
}

pub fn normalize_local_ai_proxy_public_host(value: Option<&str>) -> String {
    let mut resolver = resolve_local_ai_proxy_public_host_addresses;
    normalize_local_ai_proxy_public_host_with_resolver(value, &mut resolver)
}

fn create_default_local_ai_proxy_config() -> LocalAiProxyConfigFile {
    LocalAiProxyConfigFile {
        schema_version: LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION,
        bind_host: LOCAL_API_PROXY_DEFAULT_BIND_HOST.to_string(),
        public_base_host: default_local_ai_proxy_public_host(),
        requested_port: LOCAL_API_PROXY_DEFAULT_PORT,
        client_api_key: generate_local_ai_proxy_client_api_key(),
    }
}

fn normalize_local_ai_proxy_requested_port(raw_requested_port: Option<u16>) -> u16 {
    match raw_requested_port {
        Some(port) if port >= 20_000 => port,
        _ => LOCAL_API_PROXY_DEFAULT_PORT,
    }
}

fn write_local_ai_proxy_config_file(
    path: &Path,
    config: &LocalAiProxyConfigFile,
) -> LocalApiProxyNativeResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{}\n", serde_json::to_string_pretty(config)?))?;
    Ok(())
}

fn generate_local_ai_proxy_client_api_key() -> String {
    format!("sk_sdkwork_local_proxy_{}", Uuid::new_v4().simple())
}

fn normalize_local_ai_proxy_public_host_with_resolver<F>(
    value: Option<&str>,
    resolver: &mut F,
) -> String
where
    F: FnMut(&str) -> Vec<IpAddr>,
{
    let Some(candidate) = value
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
    else {
        return resolve_default_local_ai_proxy_public_host_with_resolver(resolver);
    };

    if local_ai_proxy_public_host_is_loopback_safe_with_resolver(candidate, resolver) {
        return candidate.to_string();
    }

    resolve_default_local_ai_proxy_public_host_with_resolver(resolver)
}

fn local_ai_proxy_public_host_is_loopback_safe_with_resolver<F>(
    host: &str,
    resolver: &mut F,
) -> bool
where
    F: FnMut(&str) -> Vec<IpAddr>,
{
    let candidate = host.trim().trim_matches(['[', ']']);
    if candidate.is_empty() {
        return false;
    }

    let addresses = resolver(candidate);
    !addresses.is_empty() && addresses.iter().all(IpAddr::is_loopback)
}

fn resolve_local_ai_proxy_public_host_addresses(host: &str) -> Vec<IpAddr> {
    if let Ok(address) = host.parse::<IpAddr>() {
        return vec![address];
    }

    (host, 0)
        .to_socket_addrs()
        .map(|entries| entries.map(|entry| entry.ip()).collect())
        .unwrap_or_default()
}
