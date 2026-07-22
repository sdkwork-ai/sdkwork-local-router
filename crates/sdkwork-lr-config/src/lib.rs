use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeTomlConfig {
    pub server: ServerSectionConfig,
    pub storage: StorageSectionConfig,
    pub logging: LoggingSectionConfig,
    pub upstreams: Vec<UpstreamSectionConfig>,
    pub fallback: FallbackSectionConfig,
    pub rate_limit: RateLimitSectionConfig,
    pub cors: CorsSectionConfig,
    pub routing: RoutingSectionConfig,
    pub recording: RecordingSectionConfig,
    pub circuit_breaker: CircuitBreakerSectionConfig,
    pub health_probe: HealthProbeSectionConfig,
    pub plugins: PluginSectionConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSectionConfig {
    pub bind: Option<String>,
    pub port: Option<u16>,
    pub max_body_size_mb: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageSectionConfig {
    pub database_url: Option<String>,
    pub encryption_secret: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingSectionConfig {
    pub level: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UpstreamSectionConfig {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub upstream_api_key: Option<String>,
    pub upstream_auth_scheme: Option<String>,
    pub models: Vec<String>,
    pub priority: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub max_retries: Option<u32>,
    pub retry_delay_ms: Option<u64>,
    pub anthropic_version: Option<String>,
    pub default_headers: BTreeMap<String, String>,
    pub model_route_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FallbackSectionConfig {
    pub enabled: Option<bool>,
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimitSectionConfig {
    pub enabled: Option<bool>,
    pub max_requests: Option<u64>,
    pub window_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CorsSectionConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoutingSectionConfig {
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecordingSectionConfig {
    pub save_request_body: Option<bool>,
    pub save_response_body: Option<bool>,
    pub retention_days: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CircuitBreakerSectionConfig {
    pub failure_threshold: Option<u32>,
    pub success_threshold: Option<u32>,
    pub open_duration_secs: Option<u64>,
    pub half_open_max_requests: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HealthProbeSectionConfig {
    pub enabled: Option<bool>,
    pub interval_secs: Option<u64>,
    pub probe_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PluginSectionConfig {
    pub enabled: Option<bool>,
    pub policy: Option<String>,
    pub expose_decision_headers: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
    pub base_paths: BasePathConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub fallback: FallbackConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
    pub routing: RoutingConfig,
    pub recording: RecordingConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub health_probe: HealthProbeConfig,
    pub plugins: PluginConfig,
    pub config_file_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    pub max_body_size_mb: u64,
}

impl ServerConfig {
    pub fn max_body_size_bytes(&self) -> usize {
        (self.max_body_size_mb as usize) * 1024 * 1024
    }
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub database_url: String,
    pub encryption_secret: String,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Text,
}

#[derive(Debug, Clone)]
pub struct BasePathConfig {
    pub openai: String,
    pub anthropic: String,
    pub google: String,
}

pub const LOCAL_ROUTER_OPENAI_PREFIX: &str = "/local-router/v1";
pub const LOCAL_ROUTER_ANTHROPIC_PREFIX: &str = "/local-router/anthropic";
pub const LOCAL_ROUTER_GOOGLE_PREFIX: &str = "/local-router/google";

impl Default for BasePathConfig {
    fn default() -> Self {
        Self {
            openai: LOCAL_ROUTER_OPENAI_PREFIX.to_owned(),
            anthropic: LOCAL_ROUTER_ANTHROPIC_PREFIX.to_owned(),
            google: LOCAL_ROUTER_GOOGLE_PREFIX.to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpstreamConfig {
    pub name: String,
    pub provider: sdkwork_lr_core::ProviderKind,
    pub base_url: String,
    pub upstream_api_key: String,
    pub upstream_auth_scheme: Option<String>,
    pub models: Vec<String>,
    pub priority: u32,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub anthropic_version: Option<String>,
    pub default_headers: BTreeMap<String, String>,
    pub model_route_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct FallbackConfig {
    pub enabled: bool,
    pub max_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub max_requests: u64,
    pub window_secs: u64,
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl CorsConfig {
    pub fn is_allow_any(&self) -> bool {
        self.allowed_origins.iter().any(|o| o == "*")
    }
}

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub strategy: sdkwork_lr_core::RoutingStrategy,
    pub strategy_explicit: bool,
}

impl RoutingConfig {
    pub fn strategy_for_account_count(
        &self,
        account_count: usize,
    ) -> sdkwork_lr_core::RoutingStrategy {
        if self.strategy_explicit {
            self.strategy
        } else {
            default_routing_strategy_for_account_count(account_count)
        }
    }
}

fn default_routing_strategy_for_account_count(
    account_count: usize,
) -> sdkwork_lr_core::RoutingStrategy {
    if account_count > 1 {
        sdkwork_lr_core::RoutingStrategy::RoundRobin
    } else {
        sdkwork_lr_core::RoutingStrategy::Priority
    }
}

#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub save_request_body: bool,
    pub save_response_body: bool,
    pub retention_days: i64,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub open_duration: Duration,
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthProbeConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub probe_timeout_secs: u64,
}

impl Default for HealthProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 30,
            probe_timeout_secs: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub enabled: bool,
    pub policy: sdkwork_lr_plugin::PluginPolicy,
    pub expose_decision_headers: bool,
}

impl RuntimeTomlConfig {
    pub fn from_str_strict(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("invalid TOML: {e}"))
    }

    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read config file {}: {e}", path.display()))?;
        Self::from_str_strict(&content)
            .map_err(|e| format!("invalid TOML in {}: {e}", path.display()))
    }

    pub fn from_env_config_file() -> Result<Option<Self>, String> {
        let Some(path) = env_optional("SDKWORK_LR_CONFIG_FILE") else {
            return Ok(None);
        };
        let path = PathBuf::from(path);
        if !path.exists() {
            return Ok(None);
        }
        Self::from_config_file(path).map(Some)
    }
}

impl RuntimeConfig {
    pub fn from_toml(toml_config: &RuntimeTomlConfig) -> Result<Self, String> {
        let bind = config_value("SDKWORK_LR_BIND", toml_config.server.bind.as_deref())
            .unwrap_or_else(|| "127.0.0.1".to_owned());
        let port = env_u16("SDKWORK_LR_PORT")
            .or(toml_config.server.port)
            .unwrap_or(8080);

        let database_url = config_value(
            "SDKWORK_LR_DATABASE_URL",
            toml_config.storage.database_url.as_deref(),
        )
        .unwrap_or_else(|| format!("sqlite:./data/local-router-{port}.db?mode=rwc"));

        let log_level = config_value("SDKWORK_LR_LOG_LEVEL", toml_config.logging.level.as_deref())
            .unwrap_or_else(|| "info".to_owned());
        let log_format_str = config_value(
            "SDKWORK_LR_LOG_FORMAT",
            toml_config.logging.format.as_deref(),
        )
        .unwrap_or_else(|| "text".to_owned());
        let log_format = match log_format_str.to_ascii_lowercase().as_str() {
            "json" => LogFormat::Json,
            _ => LogFormat::Text,
        };

        let base_paths = BasePathConfig::default();

        let max_body_size_mb = toml_config.server.max_body_size_mb.unwrap_or(50);
        if max_body_size_mb < 1 {
            return Err(format!(
                "server.max_body_size_mb must be at least 1, got {}",
                max_body_size_mb
            ));
        }

        let mut upstreams = Vec::new();
        for u in &toml_config.upstreams {
            if u.name.is_empty() {
                return Err("upstream name must not be blank".to_owned());
            }
            if u.base_url.is_empty() {
                return Err(format!("upstream {}: base_url must not be blank", u.name));
            }
            let upstream_auth_scheme = match canonical_upstream_auth_scheme(
                u.upstream_auth_scheme.as_deref(),
            ) {
                Some(Some(auth_scheme)) => Some(auth_scheme.to_owned()),
                Some(None) => None,
                None => {
                    return Err(format!(
                            "upstream {}: upstream_auth_scheme must be one of: bearer, x-api-key, x-goog-api-key, query-key",
                            u.name
                        ));
                }
            };
            let upstream_api_key =
                resolve_env_or_value(u.upstream_api_key.as_deref()).unwrap_or_default();
            upstreams.push(UpstreamConfig {
                name: u.name.clone(),
                provider: sdkwork_lr_core::ProviderKind::from_str_loose(&u.provider),
                base_url: u.base_url.trim_end_matches('/').to_owned(),
                upstream_api_key,
                upstream_auth_scheme,
                models: u.models.clone(),
                priority: u.priority.unwrap_or(10),
                timeout: Duration::from_secs(u.timeout_secs.unwrap_or(120)),
                max_retries: u.max_retries.unwrap_or(0),
                retry_delay_ms: u.retry_delay_ms.unwrap_or(500),
                anthropic_version: u.anthropic_version.clone(),
                default_headers: u.default_headers.clone(),
                model_route_mappings: u.model_route_mappings.clone(),
            });
        }
        let upstream_count = upstreams.len();

        let window_secs = toml_config.rate_limit.window_secs.unwrap_or(60);
        if window_secs < 1 {
            return Err(format!(
                "rate_limit.window_secs must be at least 1, got {}",
                window_secs
            ));
        }
        let fallback_max_attempts = toml_config.fallback.max_attempts.unwrap_or(3);
        if fallback_max_attempts < 1 || fallback_max_attempts > 20 {
            return Err(format!(
                "fallback.max_attempts must be between 1 and 20, got {}",
                fallback_max_attempts
            ));
        }
        let health_interval = toml_config.health_probe.interval_secs.unwrap_or(30);
        if health_interval < 5 {
            return Err(format!(
                "health_probe.interval_secs must be at least 5, got {}",
                health_interval
            ));
        }
        let probe_timeout = toml_config.health_probe.probe_timeout_secs.unwrap_or(10);
        if probe_timeout < 1 || probe_timeout > health_interval {
            return Err(format!(
                "health_probe.probe_timeout_secs ({}) must be >= 1 and < interval_secs ({})",
                probe_timeout, health_interval
            ));
        }
        let encryption_secret = toml_config
            .storage
            .encryption_secret
            .clone()
            .or_else(|| std::env::var("SDKWORK_LR_ENCRYPTION_SECRET").ok())
            .unwrap_or_default();
        if !encryption_secret.is_empty() && encryption_secret.len() < 8 {
            return Err(
                "encryption_secret must be at least 8 characters for adequate security".to_owned(),
            );
        }

        let plugin_policy_code = config_value(
            "SDKWORK_LR_PLUGIN_POLICY",
            toml_config.plugins.policy.as_deref(),
        )
        .unwrap_or_else(|| "auto".to_owned());
        let plugin_policy = sdkwork_lr_plugin::PluginPolicy::from_code(&plugin_policy_code)
            .ok_or_else(|| {
                format!(
                    "unknown plugins.policy '{}': expected 'auto', 'strict', 'passthrough', or 'force_transform'",
                    plugin_policy_code
                )
            })?;

        Ok(Self {
            server: ServerConfig {
                bind,
                port,
                max_body_size_mb,
            },
            storage: StorageConfig {
                database_url,
                encryption_secret,
            },
            logging: LoggingConfig {
                level: log_level,
                format: log_format,
            },
            base_paths,
            upstreams,
            fallback: FallbackConfig {
                enabled: toml_config.fallback.enabled.unwrap_or(true),
                max_attempts: fallback_max_attempts,
            },
            rate_limit: RateLimitConfig {
                enabled: toml_config.rate_limit.enabled.unwrap_or(false),
                max_requests: toml_config.rate_limit.max_requests.unwrap_or(60),
                window_secs,
            },
            cors: CorsConfig {
                allowed_origins: if toml_config.cors.allowed_origins.is_empty() {
                    vec!["http://localhost:*".to_owned()]
                } else {
                    toml_config.cors.allowed_origins.clone()
                },
            },
            routing: {
                let strategy_code = toml_config
                    .routing
                    .strategy
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                let strategy_explicit = strategy_code
                    .map(|value| !value.eq_ignore_ascii_case("auto"))
                    .unwrap_or(false);
                let strategy = match strategy_code.map(|value| value.to_ascii_lowercase()) {
                    Some(value) if value == "auto" => {
                        default_routing_strategy_for_account_count(upstream_count)
                    }
                    Some(value) if value == "round_robin" || value == "roundrobin" => {
                        sdkwork_lr_core::RoutingStrategy::RoundRobin
                    }
                    Some(value) if value == "random" => sdkwork_lr_core::RoutingStrategy::Random,
                    Some(value) if value == "least_latency" || value == "leastlatency" => {
                        sdkwork_lr_core::RoutingStrategy::LeastLatency
                    }
                    Some(value) if value == "priority" => {
                        sdkwork_lr_core::RoutingStrategy::Priority
                    }
                    Some(other) => return Err(format!("unknown routing.strategy '{}': expected 'auto', 'priority', 'round_robin', 'random', or 'least_latency'", other)),
                    None => default_routing_strategy_for_account_count(upstream_count),
                };
                RoutingConfig {
                    strategy,
                    strategy_explicit,
                }
            },
            recording: RecordingConfig {
                save_request_body: toml_config.recording.save_request_body.unwrap_or(false),
                save_response_body: toml_config.recording.save_response_body.unwrap_or(false),
                retention_days: toml_config.recording.retention_days.unwrap_or(30).max(1),
            },
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: toml_config.circuit_breaker.failure_threshold.unwrap_or(5),
                success_threshold: toml_config.circuit_breaker.success_threshold.unwrap_or(3),
                open_duration: Duration::from_secs(
                    toml_config.circuit_breaker.open_duration_secs.unwrap_or(60),
                ),
                half_open_max_requests: toml_config
                    .circuit_breaker
                    .half_open_max_requests
                    .unwrap_or(2),
            },
            health_probe: HealthProbeConfig {
                enabled: toml_config.health_probe.enabled.unwrap_or(true),
                interval_secs: health_interval,
                probe_timeout_secs: probe_timeout,
            },
            plugins: PluginConfig {
                enabled: env_bool("SDKWORK_LR_PLUGINS_ENABLED")
                    .or(toml_config.plugins.enabled)
                    .unwrap_or(true),
                policy: plugin_policy,
                expose_decision_headers: env_bool("SDKWORK_LR_PLUGIN_DECISION_HEADERS")
                    .or(toml_config.plugins.expose_decision_headers)
                    .unwrap_or(true),
            },
            config_file_path: None,
        })
    }

    pub fn from_file_or_env(config_path: Option<&str>) -> Result<Self, String> {
        let resolved_path = config_path
            .map(|p| p.to_owned())
            .or_else(|| env_optional("SDKWORK_LR_CONFIG_FILE"))
            .filter(|p| !p.trim().is_empty() && std::path::Path::new(p).exists());

        let toml_config = if let Some(ref path) = resolved_path {
            RuntimeTomlConfig::from_config_file(path)?
        } else {
            RuntimeTomlConfig::from_env_config_file()?.unwrap_or_default()
        };
        let mut config = Self::from_toml(&toml_config)?;
        config.config_file_path = resolved_path;
        Ok(config)
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.bind, self.server.port)
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.server.port = port;
        self
    }

    pub fn with_bind(mut self, bind: impl Into<String>) -> Self {
        self.server.bind = bind.into();
        self
    }
}

fn env_optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn config_value(env_name: &str, config_value: Option<&str>) -> Option<String> {
    env_optional(env_name).or_else(|| {
        config_value
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())
    })
}

fn env_u16(name: &str) -> Option<u16> {
    env_optional(name).and_then(|v| v.parse::<u16>().ok())
}

fn env_bool(name: &str) -> Option<bool> {
    env_optional(name).and_then(|v| match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    })
}

fn resolve_env_or_value(value: Option<&str>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        let inner = &trimmed[2..trimmed.len() - 1];
        if let Some(colon_pos) = inner.find(":-") {
            let var_name = &inner[..colon_pos];
            let default = &inner[colon_pos + 2..];
            match std::env::var(var_name) {
                Ok(v) if !v.trim().is_empty() => Some(v),
                _ => {
                    if default.is_empty() {
                        None
                    } else {
                        Some(default.to_owned())
                    }
                }
            }
        } else {
            let var_name = inner;
            match std::env::var(var_name) {
                Ok(v) if !v.trim().is_empty() => Some(v),
                _ => {
                    tracing::warn!(var = var_name, "environment variable not set or empty");
                    None
                }
            }
        }
    } else {
        Some(trimmed.to_owned())
    }
}

fn canonical_upstream_auth_scheme(value: Option<&str>) -> Option<Option<&'static str>> {
    let Some(value) = value else {
        return Some(None);
    };
    match value.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "" => Some(None),
        "bearer" | "authorization_bearer" => Some(Some("bearer")),
        "x_api_key" | "anthropic_x_api_key" => Some(Some("x-api-key")),
        "x_goog_api_key" | "google_api_key" => Some(Some("x-goog-api-key")),
        "query_key" | "key_query" | "google_query_key" => Some(Some("query-key")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_lr_plugin::PluginPolicy;

    fn upstream(name: &str) -> UpstreamSectionConfig {
        UpstreamSectionConfig {
            name: name.to_owned(),
            provider: "openai".to_owned(),
            base_url: format!("https://{name}.example/v1"),
            upstream_api_key: Some("sk-test".to_owned()),
            upstream_auth_scheme: None,
            models: vec!["*".to_owned()],
            priority: Some(10),
            timeout_secs: None,
            max_retries: None,
            retry_delay_ms: None,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            model_route_mappings: BTreeMap::new(),
        }
    }

    #[test]
    fn plugin_config_defaults_to_auto_enabled_with_decision_headers() {
        let config = RuntimeConfig::from_toml(&RuntimeTomlConfig::default()).unwrap();

        assert!(config.plugins.enabled);
        assert_eq!(config.plugins.policy, PluginPolicy::Auto);
        assert!(config.plugins.expose_decision_headers);
    }

    #[test]
    fn provider_compatible_paths_use_the_canonical_local_router_namespace() {
        let config = RuntimeConfig::from_toml(&RuntimeTomlConfig::default()).unwrap();

        assert_eq!(config.base_paths.openai, LOCAL_ROUTER_OPENAI_PREFIX);
        assert_eq!(config.base_paths.anthropic, LOCAL_ROUTER_ANTHROPIC_PREFIX);
        assert_eq!(config.base_paths.google, LOCAL_ROUTER_GOOGLE_PREFIX);
    }

    #[test]
    fn routing_defaults_to_priority_for_single_upstream() {
        let toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream("primary")],
            ..RuntimeTomlConfig::default()
        };

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert_eq!(
            config.routing.strategy,
            sdkwork_lr_core::RoutingStrategy::Priority
        );
    }

    #[test]
    fn routing_defaults_to_round_robin_for_multiple_upstreams() {
        let toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream("primary"), upstream("secondary")],
            ..RuntimeTomlConfig::default()
        };

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert_eq!(
            config.routing.strategy,
            sdkwork_lr_core::RoutingStrategy::RoundRobin
        );
    }

    #[test]
    fn explicit_routing_strategy_overrides_multi_account_default() {
        let mut toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream("primary"), upstream("secondary")],
            ..RuntimeTomlConfig::default()
        };
        toml_config.routing.strategy = Some("priority".to_owned());

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert_eq!(
            config.routing.strategy,
            sdkwork_lr_core::RoutingStrategy::Priority
        );
    }

    #[test]
    fn routing_auto_strategy_uses_account_count_default() {
        let mut toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream("primary"), upstream("secondary")],
            ..RuntimeTomlConfig::default()
        };
        toml_config.routing.strategy = Some("auto".to_owned());

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert!(!config.routing.strategy_explicit);
        assert_eq!(
            config.routing.strategy,
            sdkwork_lr_core::RoutingStrategy::RoundRobin
        );
    }

    #[test]
    fn plugin_config_parses_strict_policy_and_header_toggle() {
        let toml_config: RuntimeTomlConfig = toml::from_str(
            r#"
            [plugins]
            enabled = true
            policy = "strict"
            expose_decision_headers = false
            "#,
        )
        .unwrap();

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert!(config.plugins.enabled);
        assert_eq!(config.plugins.policy, PluginPolicy::Strict);
        assert!(!config.plugins.expose_decision_headers);
    }

    #[test]
    fn plugin_config_rejects_unknown_policy() {
        let toml_config: RuntimeTomlConfig = toml::from_str(
            r#"
            [plugins]
            policy = "invalid-shim"
            "#,
        )
        .unwrap();

        let error = RuntimeConfig::from_toml(&toml_config).unwrap_err();

        assert!(error.contains("unknown plugins.policy"));
    }

    #[test]
    fn upstream_auth_scheme_accepts_aliases_and_stores_canonical_code() {
        let mut upstream = upstream("deepseek-anthropic");
        upstream.provider = "anthropic".to_owned();
        upstream.upstream_auth_scheme = Some("authorization_bearer".to_owned());
        let toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream],
            ..RuntimeTomlConfig::default()
        };

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert_eq!(
            config.upstreams[0].upstream_auth_scheme.as_deref(),
            Some("bearer")
        );
    }

    #[test]
    fn upstream_auth_scheme_rejects_unknown_code() {
        let mut upstream = upstream("bad-auth");
        upstream.upstream_auth_scheme = Some("unsupported-unknown-key".to_owned());
        let toml_config = RuntimeTomlConfig {
            upstreams: vec![upstream],
            ..RuntimeTomlConfig::default()
        };

        let error = RuntimeConfig::from_toml(&toml_config).unwrap_err();

        assert!(error.contains("upstream_auth_scheme must be one of"));
    }

    #[test]
    fn upstream_model_route_mappings_are_canonical_config() {
        let toml_config: RuntimeTomlConfig = toml::from_str(
            r#"
            [[upstreams]]
            name = "gemini"
            provider = "google"
            base_url = "https://generativelanguage.googleapis.com"
            upstream_api_key = "sk-test"
            models = ["gemini-*"]

            [upstreams.model_route_mappings]
            "gpt-5.5" = "gemini-2.5-pro"
            "#,
        )
        .unwrap();

        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert_eq!(
            config.upstreams[0]
                .model_route_mappings
                .get("gpt-5.5")
                .map(String::as_str),
            Some("gemini-2.5-pro")
        );
    }

    #[test]
    fn unknown_upstream_mapping_section_is_rejected() {
        let error = RuntimeTomlConfig::from_str_strict(
            r#"
            [[upstreams]]
            name = "gemini"
            provider = "google"
            base_url = "https://generativelanguage.googleapis.com"
            upstream_api_key = "sk-test"
            models = ["gemini-*"]

            [upstreams.model_routes]
            "gpt-5.5" = "gemini-2.5-flash"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("unknown field `model_routes`"));
        assert!(error.contains("model_route_mappings"));
    }

    #[test]
    fn config_rejects_ambiguous_upstream_api_key_alias() {
        let error = RuntimeTomlConfig::from_str_strict(
            r#"
            [[upstreams]]
            name = "ambiguous-key"
            provider = "openai"
            base_url = "https://api.example/v1"
            api_key = "sk-upstream"
            models = ["*"]
            "#,
        )
        .unwrap_err();

        assert!(error.contains("unknown field `api_key`"));
        assert!(error.contains("upstream_api_key"));
    }

    #[test]
    fn config_rejects_unknown_nested_fields() {
        let error = RuntimeTomlConfig::from_str_strict(
            r#"
            [plugins]
            plugin_policy = "strict"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("unknown field `plugin_policy`"));
        assert!(error.contains("policy"));
    }

    #[test]
    fn repository_example_config_is_strictly_valid() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let example_path = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("config.example.toml");
        let content = std::fs::read_to_string(&example_path).unwrap();

        let toml_config = RuntimeTomlConfig::from_str_strict(&content).unwrap();
        let config = RuntimeConfig::from_toml(&toml_config).unwrap();

        assert!(config
            .upstreams
            .iter()
            .any(|upstream| upstream.model_route_mappings.contains_key("gpt-5.5")));
    }

    #[test]
    fn config_rejects_unknown_top_level_section() {
        let error = RuntimeTomlConfig::from_str_strict(
            r#"
            [removed_section]
            token = "removed"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("unknown field `removed_section`"));
    }
}
