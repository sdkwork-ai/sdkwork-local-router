pub const LOCAL_API_PROXY_DEFAULT_PORT: u16 = 21_281;
pub const LOCAL_API_PROXY_DYNAMIC_PORT_WINDOW_SIZE: u16 = 32;
pub const LOCAL_API_PROXY_FALLBACK_PORT_RANGE_START: u16 = 21_300;
pub const LOCAL_API_PROXY_FALLBACK_PORT_RANGE_END: u16 =
    LOCAL_API_PROXY_FALLBACK_PORT_RANGE_START + 9;
pub const LOCAL_API_PROXY_SCHEMA_VERSION: u32 = 1;
pub const LOCAL_API_PROXY_CONFIG_SCHEMA_VERSION: u32 = 1;
pub const LOCAL_API_PROXY_DEFAULT_BIND_HOST: &str = "127.0.0.1";
pub const LOCAL_API_PROXY_DEFAULT_UPSTREAM_BASE_URL: &str = "https://ai.sdkwork.com";
pub const LOCAL_API_PROXY_DEFAULT_CLIENT_API_KEY: &str = "sk_sdkwork_api_key";
pub const LOCAL_API_PROXY_DEFAULT_CLIENT_PROTOCOL: &str = "openai-compatible";
pub const LOCAL_API_PROXY_DEFAULT_ROUTE_ID: &str =
    "local-ai-proxy-system-default-openai-compatible";
pub const LOCAL_API_PROXY_DEFAULT_PROVIDER_ID: &str = "sdkwork";
pub const LOCAL_API_PROXY_PROVIDER_CENTER_NAMESPACE: &str = "lap.provider-routes";
pub const LOCAL_API_PROXY_PROVIDER_CENTER_CATALOG_SCHEMA_VERSION: u32 = 1;
pub const LOCAL_API_PROXY_PUBLIC_BASE_HOST_CANDIDATES: [&str; 3] =
    ["ai.sdkwork.localhost", "localhost", "127.0.0.1"];

pub const LOCAL_API_PROXY_PROVIDER_ID: &str = "sdkwork-local-proxy";
pub const LOCAL_API_PROXY_PROVIDER_OPENAI_API: &str = "openai-completions";
pub const LOCAL_API_PROXY_PROVIDER_ANTHROPIC_API: &str = "anthropic-messages";
pub const LOCAL_API_PROXY_PROVIDER_GEMINI_API: &str = "google-generative-ai";
pub const LOCAL_API_PROXY_PROVIDER_AUTH: &str = "api-key";

pub const LOCAL_API_PROXY_TOKEN_ENV_VAR: &str = "SDKWORK_LOCAL_PROXY_TOKEN";
pub const LOCAL_API_PROXY_API_KEY_PLACEHOLDER: &str = "${SDKWORK_LOCAL_PROXY_TOKEN}";

pub const fn local_api_proxy_fallback_port_range_start(requested_port: u16) -> u16 {
    if requested_port == LOCAL_API_PROXY_DEFAULT_PORT {
        LOCAL_API_PROXY_FALLBACK_PORT_RANGE_START
    } else {
        requested_port
    }
}

pub const fn local_api_proxy_fallback_port_range_end(requested_port: u16) -> u16 {
    if requested_port == LOCAL_API_PROXY_DEFAULT_PORT {
        LOCAL_API_PROXY_FALLBACK_PORT_RANGE_END
    } else {
        requested_port.saturating_add(LOCAL_API_PROXY_DYNAMIC_PORT_WINDOW_SIZE - 1)
    }
}
