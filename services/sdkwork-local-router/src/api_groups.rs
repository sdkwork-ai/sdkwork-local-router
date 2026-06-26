use serde::Serialize;

pub const LOCAL_ROUTER_OPEN_API: &str = "local-router-open-api";
pub const LOCAL_ROUTER_APP_API: &str = "local-router-app-api";
pub const LOCAL_ROUTER_BACKEND_API: &str = "local-router-backend-api";

pub const LOCAL_ROUTER_APP_API_PREFIX: &str = "/app/v3/api/local_router";
pub const LOCAL_ROUTER_BACKEND_API_PREFIX: &str = "/backend/v3/api/local_router";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocalRouterApiGroup {
    pub code: &'static str,
    pub display_name: &'static str,
    pub canonical_prefix: String,
    pub path_prefixes: Vec<String>,
    pub audience: &'static str,
    pub auth_scheme: &'static str,
    pub user_id_source: &'static str,
    pub data_scope: &'static str,
    pub capabilities: Vec<&'static str>,
}

pub fn local_router_api_groups(
    base_paths: &sdkwork_lr_config::BasePathConfig,
) -> Vec<LocalRouterApiGroup> {
    vec![
        LocalRouterApiGroup {
            code: LOCAL_ROUTER_OPEN_API,
            display_name: "Local Router Open API",
            canonical_prefix: "provider_compatible_proxy_paths".to_owned(),
            path_prefixes: vec![
                wildcard_path(&base_paths.openai),
                wildcard_path(&base_paths.anthropic),
                wildcard_path(&base_paths.google),
            ],
            audience:
                "Codex, Claude Code, Gemini CLI, OpenAI-compatible SDKs, and other model clients",
            auth_scheme: "client_api_key",
            user_id_source: "local_router_client_api_keys.user_id",
            data_scope: "per_user",
            capabilities: vec![
                "model_proxy",
                "api_surface_transform",
                "streaming",
                "fallback",
                "usage_recording",
                "request_recording",
            ],
        },
        LocalRouterApiGroup {
            code: LOCAL_ROUTER_APP_API,
            display_name: "Local Router App API",
            canonical_prefix: LOCAL_ROUTER_APP_API_PREFIX.to_owned(),
            path_prefixes: vec![format!("{LOCAL_ROUTER_APP_API_PREFIX}/{{*path}}")],
            audience: "SDKWork app clients and local/private app integrations",
            auth_scheme: "sdkwork_subject_projection",
            user_id_source: "auth_token_or_jwt_subject",
            data_scope: "per_user",
            capabilities: vec!["router_status", "model_listing"],
        },
        LocalRouterApiGroup {
            code: LOCAL_ROUTER_BACKEND_API,
            display_name: "Local Router Backend API",
            canonical_prefix: LOCAL_ROUTER_BACKEND_API_PREFIX.to_owned(),
            path_prefixes: vec![format!("{LOCAL_ROUTER_BACKEND_API_PREFIX}/{{*path}}")],
            audience: "SDKWork backend SDKs, admin consoles, operators, and control-plane services",
            auth_scheme: "sdkwork_subject_projection",
            user_id_source: "auth_token_or_jwt_subject",
            data_scope: "per_user",
            capabilities: vec![
                "account_management",
                "client_api_key_management",
                "usage_records",
                "invocation_logs",
                "health_control",
                "plugin_registry",
                "routing_strategy_management",
                "api_group_manifest",
            ],
        },
    ]
}

fn wildcard_path(prefix: &str) -> String {
    format!("{}/{{*path}}", prefix.trim_end_matches('/'))
}
