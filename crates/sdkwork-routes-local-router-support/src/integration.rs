use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::RequestContext;
use crate::state::AppState;
use crate::upstream_auth::{
    canonical_upstream_auth_scheme, is_supported_auth_scheme, SUPPORTED_UPSTREAM_AUTH_SCHEMES,
};
use sdkwork_lr_core::Account;
use sdkwork_lr_store::{NewAccount, NewClientApiKey, NewModelRouteMapping};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page_size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct UsageQueryParams {
    pub page_size: Option<i64>,
    pub offset: Option<i64>,
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct PluginDecisionQueryParams {
    pub model: String,
    pub client_api: String,
    pub source: String,
    pub upstream: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAccountRequest {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub upstream_api_key: String,
    pub upstream_auth_scheme: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: i64,
    #[serde(default)]
    pub max_retries: i32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: i64,
    pub anthropic_version: Option<String>,
    #[serde(default)]
    pub default_headers: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub model_route_mappings: std::collections::BTreeMap<String, String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_priority() -> i32 {
    10
}
fn default_timeout() -> i64 {
    120
}
fn default_enabled() -> bool {
    true
}
fn default_retry_delay() -> i64 {
    500
}

impl From<&CreateAccountRequest> for NewAccount {
    fn from(req: &CreateAccountRequest) -> Self {
        NewAccount {
            user_id: 0,
            name: req.name.clone(),
            provider: req.provider.clone(),
            base_url: req.base_url.clone(),
            upstream_api_key: req.upstream_api_key.clone(),
            upstream_auth_scheme: req.upstream_auth_scheme.clone(),
            models: req.models.clone(),
            priority: req.priority,
            timeout_secs: req.timeout_secs,
            max_retries: req.max_retries,
            retry_delay_ms: req.retry_delay_ms,
            anthropic_version: req.anthropic_version.clone(),
            default_headers: req.default_headers.clone(),
            model_route_mappings: effective_create_model_route_mappings(req),
            enabled: req.enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub upstream_api_key: Option<String>,
    #[serde(default)]
    pub upstream_auth_scheme: PatchField<String>,
    pub models: Option<Vec<String>>,
    pub priority: Option<i32>,
    pub timeout_secs: Option<i64>,
    pub max_retries: Option<i32>,
    pub retry_delay_ms: Option<i64>,
    pub anthropic_version: Option<String>,
    pub default_headers: Option<std::collections::BTreeMap<String, String>>,
    pub model_route_mappings: Option<std::collections::BTreeMap<String, String>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateModelRouteMappingRequest {
    pub account_id: i64,
    pub client_model: String,
    pub upstream_model: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub notes: Option<String>,
}

fn effective_create_model_route_mappings(
    req: &CreateAccountRequest,
) -> std::collections::BTreeMap<String, String> {
    req.model_route_mappings.clone()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PatchField<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => PatchField::Value(value),
            None => PatchField::Null,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateClientApiKeyRequest {
    pub name: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToggleAccountRequest {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

async fn pool_for_user_id(
    state: &AppState,
    user_id: i64,
) -> Result<std::sync::Arc<parking_lot::RwLock<sdkwork_lr_core::AccountPool>>, String> {
    if let Some(pool) = state.user_pools.read().get(&user_id).cloned() {
        return Ok(pool);
    }
    reload_pool_from_store_for_user_id(state, user_id).await?;
    state
        .user_pools
        .read()
        .get(&user_id)
        .cloned()
        .ok_or_else(|| format!("failed to initialize account pool for user_id {user_id}"))
}

fn context_payload(context: &RequestContext) -> serde_json::Value {
    json!({
        "user_id": context.user_id,
        "source": context.source.as_str(),
        "api_group": context.api_group,
        "client_api_key_id": context.client_api_key_id,
    })
}

fn generated_client_api_key_secret() -> String {
    format!("sk-lr-{}", uuid::Uuid::new_v4().simple())
}

fn normalize_optional_upstream_auth_scheme(
    scheme: Option<String>,
) -> Result<Option<String>, String> {
    match scheme {
        Some(scheme) => {
            let Some(canonical) = canonical_upstream_auth_scheme(&scheme) else {
                return Err(
                    "upstream_auth_scheme must be one of: bearer, x-api-key, x-goog-api-key, query-key"
                        .to_owned(),
                );
            };
            if canonical.is_empty() {
                Ok(None)
            } else {
                Ok(Some(canonical.to_owned()))
            }
        }
        None => Ok(None),
    }
}

fn resolve_upstream_auth_scheme_patch(
    patch: PatchField<String>,
    existing: Option<String>,
) -> Result<Option<String>, String> {
    match patch {
        PatchField::Missing => Ok(existing),
        PatchField::Null => Ok(None),
        PatchField::Value(value) => normalize_optional_upstream_auth_scheme(Some(value)),
    }
}

pub async fn router_status(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    let upstreams: Vec<serde_json::Value> = pool
        .enabled_accounts()
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "provider": a.provider.to_string(),
                "base_url": a.base_url,
                "models": a.models,
                "priority": a.priority,
            })
        })
        .collect();
    let routing_strategy = format!("{:?}", pool.strategy());
    drop(pool);

    Json(json!({
        "status": "ok",
        "service": "sdkwork-local-router",
        "version": env!("CARGO_PKG_VERSION"),
        "context": context_payload(&context),
        "user_id": context.user_id,
        "upstream_count": upstreams.len(),
        "upstreams": upstreams,
        "routing_strategy": routing_strategy,
        "fallback_enabled": state.config.fallback.enabled,
        "plugins": {
            "enabled": state.config.plugins.enabled,
            "policy": state.config.plugins.policy.as_str(),
            "expose_decision_headers": state.config.plugins.expose_decision_headers,
            "registry_count": state.transform_registry.manifests().len(),
            "model_catalog_enabled": state.model_catalog.is_some(),
        },
        "model_catalog": state.model_catalog.as_ref().map(|catalog| json!({
            "name": catalog.manifest.name,
            "catalog_version": catalog.manifest.catalog_version,
            "schema_version": catalog.manifest.schema_version,
            "vendor_count": catalog.vendors.len(),
        })),
        "proxy_auth": {
            "type": "client_api_key",
            "storage": "local_router_client_api_keys",
            "user_id_source": "database_record",
        },
    }))
    .into_response()
}

pub async fn list_models(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    let mut models: Vec<serde_json::Value> = pool
        .enabled_accounts()
        .iter()
        .flat_map(|a| {
            a.models.iter().map(move |m| {
                json!({
                    "id": m,
                    "object": "model",
                    "owned_by": a.provider.to_string(),
                })
            })
        })
        .collect();
    models.sort_by(|a, b| {
        a.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("id").and_then(|v| v.as_str()).unwrap_or(""))
    });
    models.dedup_by(|a, b| a.get("id") == b.get("id"));
    Json(json!({
        "object": "list",
        "context": context_payload(&context),
        "user_id": context.user_id,
        "data": models
    }))
    .into_response()
}

pub async fn list_upstreams(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    let upstreams: Vec<serde_json::Value> = pool
        .accounts()
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "provider": a.provider.to_string(),
                "base_url": a.base_url,
                "models": a.models,
                "priority": a.priority,
                "enabled": a.enabled,
            })
        })
        .collect();
    Json(json!({"context": context_payload(&context), "user_id": context.user_id, "upstreams": upstreams})).into_response()
}

pub async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let limit = params.page_size.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    match state
        .store
        .list_invocations_for_user(context.user_id, limit, offset)
        .await
    {
        Ok(logs) => Json(json!({"context": context_payload(&context), "user_id": context.user_id, "logs": logs, "limit": limit, "offset": offset})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    Json(plugin_registry_payload(
        &state.transform_registry,
        &state.config.base_paths,
        state.config.plugins.enabled,
        state.config.plugins.policy,
        state.config.plugins.expose_decision_headers,
        state.model_catalog.is_some(),
    ))
}

pub async fn list_api_groups(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "service": "sdkwork-local-router",
        "api_groups": crate::api_groups::local_router_api_groups(&state.config.base_paths),
    }))
}

pub async fn plugin_decision(
    State(state): State<AppState>,
    Query(params): Query<PluginDecisionQueryParams>,
) -> impl IntoResponse {
    let Some(source) = sdkwork_lr_plugin::ApiSurface::from_protocol_code(&params.source) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "source must be a known API surface code"})),
        )
            .into_response();
    };
    let Some(upstream) = sdkwork_lr_plugin::ApiSurface::from_protocol_code(&params.upstream) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "upstream must be a known API surface code"})),
        )
            .into_response();
    };

    let payload = plugin_decision_payload(
        &state.transform_registry,
        state.model_catalog.as_deref(),
        if state.config.plugins.enabled {
            state.config.plugins.policy
        } else {
            sdkwork_lr_plugin::PluginPolicy::Passthrough
        },
        &params.model,
        &params.client_api,
        source,
        upstream,
    );

    Json(payload).into_response()
}

fn plugin_registry_payload(
    registry: &sdkwork_lr_plugin::PluginRegistry,
    base_paths: &sdkwork_lr_config::BasePathConfig,
    enabled: bool,
    policy: sdkwork_lr_plugin::PluginPolicy,
    expose_decision_headers: bool,
    model_catalog_loaded: bool,
) -> serde_json::Value {
    let manifests = registry.manifests();
    let route_capabilities = registry.standard_route_capabilities();
    let ready_count = route_capabilities
        .iter()
        .filter(|capability| capability.reason == "ready")
        .count();
    let missing_count = route_capabilities
        .iter()
        .filter(|capability| capability.reason == "missing")
        .count();
    let reserved_count = route_capabilities
        .iter()
        .filter(|capability| capability.reason == "reserved")
        .count();
    let partial_count = route_capabilities
        .iter()
        .filter(|capability| capability.reason == "partial")
        .count();

    json!({
        "api_version": sdkwork_lr_plugin::PLUGIN_API_VERSION,
        "enabled": enabled,
        "policy": policy.as_str(),
        "expose_decision_headers": expose_decision_headers,
        "registry_count": manifests.len(),
        "model_catalog_loaded": model_catalog_loaded,
        "capability_summary": {
            "ready": ready_count,
            "missing": missing_count,
            "reserved": reserved_count,
            "partial": partial_count,
            "total": route_capabilities.len(),
        },
        "route_capabilities": route_capabilities,
        "api_groups": crate::api_groups::local_router_api_groups(base_paths),
        "api_surfaces": api_surface_standards_payload(),
        "client_apis": client_api_standards_payload(),
        "upstream_auth_schemes": upstream_auth_scheme_standards_payload(),
        "routing_strategies": routing_strategy_standards_payload(),
        "pipeline_stages": pipeline_stage_standards_payload(),
        "standard_components": standard_component_standards_payload(),
        "plugins": manifests,
        "runtime_plugins": [
            {"name": "logging", "enabled": true},
            {"name": "metrics", "enabled": true},
            {"name": "audit_metadata", "enabled": true},
            {"name": "fallback", "enabled": true},
            {"name": "model_compatibility", "enabled": model_catalog_loaded},
            {"name": "security_policy", "enabled": false, "available": true},
            {"name": "concurrency_control", "enabled": false, "available": true},
            {"name": "billing_meter", "enabled": false, "available": true},
        ]
    })
}

fn upstream_auth_scheme_standards_payload() -> Vec<serde_json::Value> {
    vec![
        json!({
            "code": "provider_default",
            "display_name": "Provider Default",
            "description": "Omit upstream_auth_scheme and inject credentials using the selected provider default: OpenAI/custom Bearer, Anthropic x-api-key, Google x-goog-api-key.",
        }),
        json!({
            "code": "bearer",
            "display_name": "Bearer",
            "header": "Authorization",
            "description": "Inject Authorization: Bearer <upstream_api_key>. Use for OpenAI-compatible APIs and Anthropic-compatible non-Anthropic upstreams that document bearer auth.",
        }),
        json!({
            "code": "x-api-key",
            "display_name": "x-api-key",
            "header": "x-api-key",
            "description": "Inject x-api-key: <upstream_api_key>. This is the default for Anthropic provider accounts.",
        }),
        json!({
            "code": "x-goog-api-key",
            "display_name": "x-goog-api-key",
            "header": "x-goog-api-key",
            "description": "Inject x-goog-api-key: <upstream_api_key>. This is the default for Google/Gemini provider accounts.",
        }),
        json!({
            "code": "query-key",
            "display_name": "Query key",
            "query_parameter": "key",
            "description": "Append key=<upstream_api_key> to the upstream query string for providers that require query-parameter API key auth.",
        }),
    ]
}

fn routing_strategy_standards_payload() -> Vec<serde_json::Value> {
    vec![
        json!({
            "code": "auto",
            "display_name": "Auto",
            "description": "Clear any per-user runtime routing override and select priority for single-account pools or round_robin for multi-account pools.",
            "multi_account_behavior": "round_robin_when_account_count_gt_1",
            "automatic_default_when": "routing.strategy is omitted or set to auto",
            "runtime_update_behavior": "clear_user_override",
            "recommended_for_multi_account": true,
        }),
        json!({
            "code": "priority",
            "display_name": "Priority",
            "description": "Sort available accounts by ascending priority and use fallback in that order. This is the automatic default for single-account pools and useful for active/passive routing.",
            "multi_account_behavior": "stable_priority_order",
            "automatic_default_when": "account_count <= 1",
        }),
        json!({
            "code": "round_robin",
            "display_name": "Round Robin",
            "description": "Rotate the ordered candidate list on every request so multi-account fallback starts from a different account. This is the automatic default for multi-account pools when routing.strategy is omitted.",
            "multi_account_behavior": "rotating_candidate_order",
            "automatic_default_when": "account_count > 1",
            "recommended_for_multi_account": true,
        }),
        json!({
            "code": "random",
            "display_name": "Random",
            "description": "Shuffle available candidates for lightweight distribution across equivalent accounts.",
            "multi_account_behavior": "random_candidate_order",
        }),
        json!({
            "code": "least_latency",
            "display_name": "Least Latency",
            "description": "Sort candidates by recorded average latency and use priority as a deterministic tie-breaker.",
            "multi_account_behavior": "latency_weighted_candidate_order",
        }),
    ]
}

fn pipeline_stage_standards_payload() -> Vec<serde_json::Value> {
    vec![
        json!({
            "code": "request_received",
            "extension_method": "on_request",
            "blocking": true,
            "purpose": "Initialize request metadata, enforce coarse security policy, attach trace or billing context.",
        }),
        json!({
            "code": "request_body_read",
            "extension_method": "on_request_body_read",
            "blocking": true,
            "purpose": "Inspect body size and request metadata after the body is available but before account routing.",
        }),
        json!({
            "code": "route_candidates",
            "extension_method": "on_route_candidates",
            "blocking": true,
            "purpose": "Inspect, filter, or reorder upstream account candidates for security, quota, cost, region, or policy routing.",
        }),
        json!({
            "code": "account_selected",
            "extension_method": "on_account_selected",
            "blocking": true,
            "purpose": "Validate the selected upstream account before transform and forwarding.",
        }),
        json!({
            "code": "before_transform",
            "extension_method": "on_before_transform",
            "blocking": true,
            "purpose": "Run pre-transform validation, model policy checks, and request compatibility checks.",
        }),
        json!({
            "code": "before_forward",
            "extension_method": "on_before_forward",
            "blocking": true,
            "purpose": "Apply concurrency control, final quota checks, outbound safety checks, and monitoring correlation.",
        }),
        json!({
            "code": "upstream_response",
            "extension_method": "on_upstream_response",
            "blocking": true,
            "purpose": "Observe upstream status before fallback handling and final response transformation.",
        }),
        json!({
            "code": "response_ready",
            "extension_method": "on_response",
            "blocking": false,
            "purpose": "Add response metrics, usage metadata, and completion observations after the final response is known.",
        }),
        json!({
            "code": "before_persist",
            "extension_method": "on_before_persist",
            "blocking": false,
            "purpose": "Finalize audit, billing, usage, and redaction metadata before invocation persistence.",
        }),
        json!({
            "code": "error",
            "extension_method": "on_error",
            "blocking": false,
            "purpose": "Observe failures for security events, audit, metrics, and alerting.",
        }),
    ]
}

fn standard_component_standards_payload() -> Vec<serde_json::Value> {
    vec![
        json!({
            "code": "auth_context_resolver",
            "component_type": "middleware",
            "extension_points": ["request_received"],
            "built_in_available": true,
            "default_enabled": true,
            "blocking_stages": ["request_received"],
            "non_blocking_stages": [],
            "configuration_source": "service_middleware",
            "purpose": "Resolve user_id from database-backed client_api_key on proxy routes or upstream sdkwork-iam subject projection on app/backend routes.",
        }),
        json!({
            "code": "routing_policy",
            "component_type": "account_pool",
            "extension_points": ["route_candidates", "account_selected"],
            "built_in_available": true,
            "default_enabled": true,
            "blocking_stages": ["route_candidates", "account_selected"],
            "non_blocking_stages": [],
            "configuration_source": "routing.strategy and per-user account pool",
            "purpose": "Apply priority, round_robin, random, least_latency, model, health, user, and fallback account selection.",
        }),
        json!({
            "code": "security_policy",
            "component_type": "interceptor",
            "extension_points": ["request_received", "request_body_read", "route_candidates", "before_transform", "before_forward"],
            "built_in_available": true,
            "default_enabled": false,
            "blocking_stages": ["request_body_read", "route_candidates", "account_selected", "before_transform", "before_forward"],
            "non_blocking_stages": [],
            "configuration_source": "SecurityPolicyInterceptor / SecurityPolicyConfig",
            "purpose": "Block unsafe requests, forbidden models, forbidden vendors, disallowed regions, or sensitive payload patterns.",
        }),
        json!({
            "code": "concurrency_control",
            "component_type": "interceptor",
            "extension_points": ["before_forward", "response_ready", "error"],
            "built_in_available": true,
            "default_enabled": false,
            "blocking_stages": ["before_forward"],
            "non_blocking_stages": ["response_ready", "error"],
            "configuration_source": "ConcurrencyLimitInterceptor / ConcurrencyLimitConfig",
            "purpose": "Enforce per-user, per-model, per-account, or global in-flight limits and return HTTP 429 when saturated.",
        }),
        json!({
            "code": "billing_meter",
            "component_type": "interceptor",
            "extension_points": ["before_forward", "response_ready", "before_persist"],
            "built_in_available": true,
            "default_enabled": false,
            "blocking_stages": ["before_forward"],
            "non_blocking_stages": ["response_ready", "before_persist"],
            "configuration_source": "BillingQuotaInterceptor / BillingQuotaConfig",
            "purpose": "Reserve quota before forwarding, read token usage after response parsing, and finalize usage/billing metadata before persistence.",
        }),
        json!({
            "code": "observability",
            "component_type": "interceptor",
            "extension_points": ["request_received", "route_candidates", "upstream_response", "response_ready", "error"],
            "built_in_available": true,
            "default_enabled": true,
            "blocking_stages": [],
            "non_blocking_stages": ["request_received", "route_candidates", "upstream_response", "response_ready", "error"],
            "configuration_source": "LoggingInterceptor, MetricsInterceptor, observability crate",
            "purpose": "Emit structured logs, metrics, traces, account health, fallback, latency, and token counters without logging secrets.",
        }),
        json!({
            "code": "audit_metadata",
            "component_type": "interceptor",
            "extension_points": ["before_persist"],
            "built_in_available": true,
            "default_enabled": true,
            "blocking_stages": [],
            "non_blocking_stages": ["before_persist"],
            "configuration_source": "AuditMetadataInterceptor",
            "purpose": "Attach non-secret request, user, route, plugin, model, status, and account metadata before invocation persistence.",
        }),
        json!({
            "code": "invocation_recorder",
            "component_type": "persistence",
            "extension_points": ["before_persist"],
            "built_in_available": true,
            "default_enabled": true,
            "blocking_stages": [],
            "non_blocking_stages": ["before_persist"],
            "configuration_source": "local_router_invocations and local_router_usages",
            "purpose": "Persist request, routing, plugin, response, error, and token usage records into local_router_ tables with user isolation.",
        }),
    ]
}

fn api_surface_standards_payload() -> Vec<serde_json::Value> {
    sdkwork_lr_plugin::standard_api_surfaces()
        .into_iter()
        .map(|surface| {
            json!({
                "code": surface.code,
                "token": surface.token,
                "display_name": surface.display_name,
                "protocol": surface.protocol.to_string(),
                "request_path": surface.request_path,
                "stream_path": surface.stream_path,
                "required_request_fields": surface.required_request_fields,
                "optional_request_fields": surface.optional_request_fields,
                "response_fields": surface.response_fields,
                "streaming_event_types": surface.streaming_event_types,
            })
        })
        .collect()
}

fn client_api_standards_payload() -> Vec<serde_json::Value> {
    sdkwork_lr_plugin::standard_client_apis()
        .into_iter()
        .map(|standard| {
            json!({
                "code": standard.code,
                "display_name": standard.display_name,
                "default_surface": standard.default_surface.protocol_code(),
                "default_surface_token": standard.default_surface.canonical_token(),
                "aliases": standard.aliases,
            })
        })
        .collect()
}

fn plugin_decision_payload(
    registry: &sdkwork_lr_plugin::PluginRegistry,
    catalog: Option<&sdkwork_models::ModelCatalog>,
    policy: sdkwork_lr_plugin::PluginPolicy,
    model: &str,
    client_api: &str,
    source: sdkwork_lr_plugin::ApiSurface,
    upstream: sdkwork_lr_plugin::ApiSurface,
) -> serde_json::Value {
    let client_api = sdkwork_lr_plugin::normalize_client_api_code(client_api);
    let (decision, model_catalog_loaded) = match catalog {
        Some(catalog) => {
            let resolver = sdkwork_lr_plugin::ModelCompatibilityResolver::new(catalog);
            (resolver.decide(model, &client_api, source, upstream), true)
        }
        None => (
            sdkwork_lr_plugin::fallback_compatibility_decision(
                model,
                &client_api,
                source,
                upstream,
            ),
            false,
        ),
    };
    let effective = sdkwork_lr_plugin::EffectivePluginDecision::from_decision(
        policy,
        upstream,
        model_catalog_loaded,
        decision,
    );
    let route_capability = effective.effective_plugin_id.as_ref().and_then(|_| {
        registry.route_capability(effective.decision.source, effective.effective_target)
    });

    json!({
        "model": model,
        "client_api": client_api,
        "model_catalog_loaded": effective.model_catalog_loaded,
        "policy": policy.as_str(),
        "source": source.canonical_token(),
        "upstream": upstream.canonical_token(),
        "decision": {
            "model_id": effective.decision.model_id,
            "vendor_code": effective.decision.vendor_code,
            "client_api_code": effective.decision.client_api_code,
            "client_support_status": client_support_status_code(&effective.decision.client_support_status),
            "client_surface": effective.decision.client_surface.canonical_token(),
            "source": effective.decision.source.canonical_token(),
            "target": effective.decision.target.canonical_token(),
            "effective_target": effective.effective_target.canonical_token(),
            "needs_plugin": effective.needs_plugin(),
            "plugin_id": effective.decision.plugin_id,
            "effective_plugin_id": effective.effective_plugin_id,
            "reason": effective.decision.reason,
        },
        "route_capability": route_capability,
    })
}

fn client_support_status_code(status: &sdkwork_lr_plugin::ClientSupportStatus) -> &'static str {
    match status {
        sdkwork_lr_plugin::ClientSupportStatus::Supported => "supported",
        sdkwork_lr_plugin::ClientSupportStatus::Unsupported => "unsupported",
        sdkwork_lr_plugin::ClientSupportStatus::Partial => "partial",
        sdkwork_lr_plugin::ClientSupportStatus::Unspecified => "unspecified",
    }
}

pub async fn list_client_api_keys(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state
        .store
        .list_client_api_keys_for_user(context.user_id)
        .await
    {
        Ok(keys) => {
            let items: Vec<serde_json::Value> = keys
                .into_iter()
                .map(|key| {
                    json!({
                        "id": key.id,
                        "user_id": key.user_id,
                        "name": key.name,
                        "key_prefix": key.key_prefix,
                        "enabled": key.enabled,
                        "last_used_at": key.last_used_at,
                        "created_at": key.created_at,
                        "updated_at": key.updated_at,
                    })
                })
                .collect();
            Json(json!({
                "context": context_payload(&context),
                "user_id": context.user_id,
                "client_api_keys": items,
            }))
            .into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

pub async fn create_client_api_key(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<CreateClientApiKeyRequest>,
) -> impl IntoResponse {
    let secret = generated_client_api_key_secret();
    let name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| "local-router client API key".to_owned());
    let new_key = NewClientApiKey {
        user_id: context.user_id,
        name: name.clone(),
        key_hash: sdkwork_lr_store::Store::client_api_key_hash(&secret),
        key_prefix: sdkwork_lr_store::Store::client_api_key_prefix(&secret),
        enabled: req.enabled,
    };
    let key_prefix = new_key.key_prefix.clone();
    let enabled = new_key.enabled;

    match state.store.insert_client_api_key(&new_key).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({
                "context": context_payload(&context),
                "user_id": context.user_id,
                "id": id,
                "name": name,
                "client_api_key": secret,
                "key_prefix": key_prefix,
                "enabled": enabled,
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

pub async fn delete_client_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state
        .store
        .delete_client_api_key_for_user(context.user_id, id)
        .await
    {
        Ok(()) => Json(json!({
            "context": context_payload(&context),
            "user_id": context.user_id,
            "id": id,
            "status": "deleted",
        }))
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state.store.list_accounts_for_user(context.user_id).await {
        Ok(accounts) => {
            let route_mappings = match state
                .store
                .list_model_route_mappings_for_user(context.user_id)
                .await
            {
                Ok(route_mappings) => route_mappings,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": e.to_string()})),
                    )
                        .into_response()
                }
            };
            let masked: Vec<serde_json::Value> = accounts
                .iter()
                .map(|a| {
                    let masked_key = if a.upstream_api_key.is_empty() {
                        String::new()
                    } else if a.upstream_api_key.len() > 8 {
                        format!(
                            "{}...{}",
                            &a.upstream_api_key[..4],
                            &a.upstream_api_key[a.upstream_api_key.len() - 4..]
                        )
                    } else {
                        "****".to_owned()
                    };
                    let mut merged_model_route_mappings: std::collections::BTreeMap<
                        String,
                        String,
                    > = serde_json::from_str(&a.model_route_mappings.clone().unwrap_or_default())
                        .unwrap_or_default();
                    let account_route_mapping_rows: Vec<_> = route_mappings
                        .iter()
                        .filter(|route_mapping| route_mapping.account_id == a.id)
                        .cloned()
                        .collect();
                    for route_mapping in account_route_mapping_rows
                        .iter()
                        .filter(|route_mapping| route_mapping.enabled)
                    {
                        merged_model_route_mappings.insert(
                            route_mapping.client_model.clone(),
                            route_mapping.upstream_model.clone(),
                        );
                    }
                    json!({
                        "id": a.id,
                        "user_id": a.user_id,
                        "name": a.name,
                        "provider": a.provider,
                        "base_url": a.base_url,
                        "upstream_api_key": masked_key,
                        "upstream_auth_scheme": a.upstream_auth_scheme,
                        "models": a.models,
                        "priority": a.priority,
                        "timeout_secs": a.timeout_secs,
                        "max_retries": a.max_retries,
                        "retry_delay_ms": a.retry_delay_ms,
                        "anthropic_version": a.anthropic_version,
                        "default_headers": a.default_headers,
                        "model_route_mappings": merged_model_route_mappings,
                        "model_route_mapping_rows": account_route_mapping_rows,
                        "enabled": a.enabled,
                    })
                })
                .collect();
            Json(json!({"context": context_payload(&context), "user_id": context.user_id, "accounts": masked})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn create_account(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name is required"})),
        )
            .into_response();
    }
    if req.base_url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "base_url is required"})),
        )
            .into_response();
    }
    if !req.base_url.starts_with("http://") && !req.base_url.starts_with("https://") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "base_url must start with http:// or https://"})),
        )
            .into_response();
    }
    let valid_providers = ["openai", "anthropic", "google"];
    if !req.provider.is_empty()
        && !valid_providers.contains(&req.provider.to_ascii_lowercase().as_str())
    {
        if !req
            .provider
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "provider must be one of: openai, anthropic, google, or a custom alphanumeric name"}))).into_response();
        }
    }
    if req.priority < 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "priority must be non-negative"})),
        )
            .into_response();
    }
    if req.timeout_secs < 1 || req.timeout_secs > 600 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "timeout_secs must be between 1 and 600"})),
        )
            .into_response();
    }
    let upstream_auth_scheme =
        match normalize_optional_upstream_auth_scheme(req.upstream_auth_scheme.clone()) {
            Ok(value) => value,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": error,
                        "supported": SUPPORTED_UPSTREAM_AUTH_SCHEMES,
                    })),
                )
                    .into_response();
            }
        };
    if req.max_retries < 0 || req.max_retries > 10 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "max_retries must be between 0 and 10"})),
        )
            .into_response();
    }

    let mut new_account: NewAccount = (&req).into();
    new_account.user_id = context.user_id;
    new_account.upstream_auth_scheme = upstream_auth_scheme;
    match state.store.insert_account(&new_account).await {
        Ok(id) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            (
                StatusCode::CREATED,
                Json(json!({"context": context_payload(&context), "user_id": context.user_id, "id": id, "name": req.name})),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<UpdateAccountRequest>,
) -> impl IntoResponse {
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "name must not be empty"})),
            )
                .into_response();
        }
    }
    if let Some(ref base_url) = req.base_url {
        if base_url.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "base_url must not be empty"})),
            )
                .into_response();
        }
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "base_url must start with http:// or https://"})),
            )
                .into_response();
        }
    }
    if let Some(ref provider) = req.provider {
        let valid_providers = ["openai", "anthropic", "google"];
        if !provider.is_empty()
            && !valid_providers.contains(&provider.to_ascii_lowercase().as_str())
        {
            if !provider
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return (StatusCode::BAD_REQUEST, Json(json!({"error": "provider must be one of: openai, anthropic, google, or a custom alphanumeric name"}))).into_response();
            }
        }
    }
    if let Some(priority) = req.priority {
        if priority < 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "priority must be non-negative"})),
            )
                .into_response();
        }
    }
    if let Some(timeout_secs) = req.timeout_secs {
        if timeout_secs < 1 || timeout_secs > 600 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "timeout_secs must be between 1 and 600"})),
            )
                .into_response();
        }
    }
    if let Some(max_retries) = req.max_retries {
        if max_retries < 0 || max_retries > 10 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "max_retries must be between 0 and 10"})),
            )
                .into_response();
        }
    }
    if let PatchField::Value(ref upstream_auth_scheme) = req.upstream_auth_scheme {
        if !is_supported_auth_scheme(Some(upstream_auth_scheme)) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "upstream_auth_scheme must be one of: bearer, x-api-key, x-goog-api-key, query-key",
                    "supported": SUPPORTED_UPSTREAM_AUTH_SCHEMES,
                })),
            )
                .into_response();
        }
    }

    let existing = match state.store.get_account_for_user(context.user_id, id).await {
        Ok(account) => account,
        Err(sdkwork_lr_store::StoreError::NotFound(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "account not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let upstream_auth_scheme = match resolve_upstream_auth_scheme_patch(
        req.upstream_auth_scheme,
        existing.upstream_auth_scheme,
    ) {
        Ok(value) => value,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": error,
                    "supported": SUPPORTED_UPSTREAM_AUTH_SCHEMES,
                })),
            )
                .into_response();
        }
    };

    let route_mapping_patch = req.model_route_mappings;
    let sync_model_route_mappings = route_mapping_patch.is_some();
    let updated = NewAccount {
        user_id: context.user_id,
        name: req.name.unwrap_or(existing.name),
        provider: req.provider.unwrap_or(existing.provider),
        base_url: req.base_url.unwrap_or(existing.base_url),
        upstream_api_key: req
            .upstream_api_key
            .filter(|k| !k.is_empty())
            .unwrap_or(existing.upstream_api_key),
        upstream_auth_scheme,
        models: req
            .models
            .unwrap_or_else(|| serde_json::from_str(&existing.models).unwrap_or_default()),
        priority: req.priority.unwrap_or(existing.priority),
        timeout_secs: req.timeout_secs.unwrap_or(existing.timeout_secs),
        max_retries: req.max_retries.unwrap_or(existing.max_retries),
        retry_delay_ms: req.retry_delay_ms.unwrap_or(existing.retry_delay_ms),
        anthropic_version: req.anthropic_version.or(existing.anthropic_version),
        default_headers: req.default_headers.unwrap_or_else(|| {
            serde_json::from_str(&existing.default_headers.unwrap_or_default()).unwrap_or_default()
        }),
        model_route_mappings: route_mapping_patch.unwrap_or_else(|| {
            serde_json::from_str(&existing.model_route_mappings.unwrap_or_default())
                .unwrap_or_default()
        }),
        enabled: req.enabled.unwrap_or(existing.enabled),
    };

    let update_result = if sync_model_route_mappings {
        state
            .store
            .update_account_for_user(context.user_id, id, &updated)
            .await
    } else {
        state
            .store
            .update_account_for_user_without_route_mapping_sync(context.user_id, id, &updated)
            .await
    };

    match update_result {
        Ok(()) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            Json(json!({"context": context_payload(&context), "user_id": context.user_id, "id": id, "status": "updated"}))
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state
        .store
        .delete_account_for_user(context.user_id, id)
        .await
    {
        Ok(()) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            Json(json!({"context": context_payload(&context), "user_id": context.user_id, "id": id, "status": "deleted"}))
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_model_route_mappings(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state
        .store
        .list_model_route_mappings_for_user(context.user_id)
        .await
    {
        Ok(route_mappings) => Json(json!({
            "context": context_payload(&context),
            "user_id": context.user_id,
            "model_route_mappings": route_mappings,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn upsert_model_route_mapping(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<CreateModelRouteMappingRequest>,
) -> impl IntoResponse {
    if req.account_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "account_id must be positive"})),
        )
            .into_response();
    }
    if req.client_model.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "client_model is required"})),
        )
            .into_response();
    }
    if req.upstream_model.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "upstream_model is required"})),
        )
            .into_response();
    }

    let account = match state
        .store
        .get_account_for_user(context.user_id, req.account_id)
        .await
    {
        Ok(account) => account,
        Err(sdkwork_lr_store::StoreError::NotFound(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "account not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let route_mapping = NewModelRouteMapping {
        user_id: context.user_id,
        account_id: account.id,
        account_name: account.name,
        client_model: req.client_model.trim().to_owned(),
        upstream_model: req.upstream_model.trim().to_owned(),
        enabled: req.enabled,
        notes: req.notes,
    };

    match state.store.upsert_model_route_mapping(&route_mapping).await {
        Ok(id) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            Json(json!({
                "context": context_payload(&context),
                "user_id": context.user_id,
                "id": id,
                "status": "upserted",
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn delete_model_route_mapping(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    match state
        .store
        .delete_model_route_mapping_for_user(context.user_id, id)
        .await
    {
        Ok(()) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            Json(json!({
                "context": context_payload(&context),
                "user_id": context.user_id,
                "id": id,
                "status": "deleted",
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn toggle_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<ToggleAccountRequest>,
) -> impl IntoResponse {
    let enabled = req.enabled;
    match state
        .store
        .toggle_account_for_user(context.user_id, id, enabled)
        .await
    {
        Ok(()) => {
            reload_pool_from_store_for_user_id(&state, context.user_id)
                .await
                .ok();
            Json(json!({"context": context_payload(&context), "user_id": context.user_id, "id": id, "enabled": enabled}))
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_usages(
    State(state): State<AppState>,
    Query(params): Query<UsageQueryParams>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let limit = params.page_size.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    match state
        .store
        .list_usages_for_user(context.user_id, limit, offset)
        .await
    {
        Ok(usages) => {
            Json(json!({"context": context_payload(&context), "user_id": context.user_id, "usages": usages, "limit": limit, "offset": offset})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn usage_summary(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let totals = state.store.usage_totals_for_user(context.user_id).await;
    let by_model = state.store.usage_by_model_for_user(context.user_id).await;
    let invocation_count = state
        .store
        .count_invocations_for_user(context.user_id)
        .await;
    let account_count = state.store.count_accounts_for_user(context.user_id).await;

    match (totals, by_model, invocation_count, account_count) {
        (Ok(totals), Ok(by_model), Ok(invocations), Ok(accounts)) => Json(json!({
            "context": context_payload(&context),
            "user_id": context.user_id,
            "totals": totals,
            "by_model": by_model,
            "invocation_count": invocations,
            "account_count": accounts,
        }))
        .into_response(),
        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("failed to aggregate usage: {e}")})),
        )
            .into_response(),
    }
}

pub async fn list_health(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    let snapshots = pool.health_manager().snapshots();
    let healthy = pool.health_manager().healthy_count();
    let degraded = pool.health_manager().degraded_count();
    let circuit_open = pool.health_manager().circuit_open_count();
    drop(pool);
    Json(json!({
        "context": context_payload(&context),
        "user_id": context.user_id,
        "summary": {
            "healthy": healthy,
            "degraded": degraded,
            "circuit_open": circuit_open,
            "total": snapshots.len(),
        },
        "accounts": snapshots,
    }))
    .into_response()
}

pub async fn force_open_account(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    if pool.health_manager().force_open(&name) {
        drop(pool);
        Json(json!({"context": context_payload(&context), "user_id": context.user_id, "account": name, "action": "force_open", "status": "ok"})).into_response()
    } else {
        drop(pool);
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "account not found in health registry"})),
        )
            .into_response()
    }
}

pub async fn force_close_account(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(context): Extension<RequestContext>,
) -> impl IntoResponse {
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    if pool.health_manager().force_close(&name) {
        drop(pool);
        Json(json!({"context": context_payload(&context), "user_id": context.user_id, "account": name, "action": "force_close", "status": "ok"})).into_response()
    } else {
        drop(pool);
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "account not found in health registry"})),
        )
            .into_response()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetStrategyRequest {
    pub strategy: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutingStrategyUpdate {
    Auto,
    Explicit(sdkwork_lr_core::RoutingStrategy),
}

fn parse_routing_strategy_update(value: &str) -> Result<RoutingStrategyUpdate, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "auto" => Ok(RoutingStrategyUpdate::Auto),
        "priority" => Ok(RoutingStrategyUpdate::Explicit(
            sdkwork_lr_core::RoutingStrategy::Priority,
        )),
        "round_robin" | "roundrobin" => Ok(RoutingStrategyUpdate::Explicit(
            sdkwork_lr_core::RoutingStrategy::RoundRobin,
        )),
        "random" => Ok(RoutingStrategyUpdate::Explicit(
            sdkwork_lr_core::RoutingStrategy::Random,
        )),
        "least_latency" | "leastlatency" => Ok(RoutingStrategyUpdate::Explicit(
            sdkwork_lr_core::RoutingStrategy::LeastLatency,
        )),
        _ => Err(
            "invalid strategy, valid: auto, priority, round_robin, random, least_latency"
                .to_owned(),
        ),
    }
}

pub async fn set_routing_strategy(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(req): Json<SetStrategyRequest>,
) -> impl IntoResponse {
    let update = match parse_routing_strategy_update(&req.strategy) {
        Ok(update) => update,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": error}))).into_response()
        }
    };
    let pool_arc = match pool_for_user_id(&state, context.user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response()
        }
    };
    let pool = pool_arc.read();
    let old = pool.strategy();
    let (strategy_mode, new_strategy) = match update {
        RoutingStrategyUpdate::Auto => {
            let strategy = state.config.routing.strategy_for_account_count(pool.len());
            pool.set_strategy(strategy);
            ("auto", strategy)
        }
        RoutingStrategyUpdate::Explicit(strategy) => {
            pool.set_strategy(strategy);
            ("explicit", strategy)
        }
    };
    drop(pool);
    match update {
        RoutingStrategyUpdate::Auto => {
            state
                .routing_strategy_overrides
                .write()
                .remove(&context.user_id);
        }
        RoutingStrategyUpdate::Explicit(strategy) => {
            state
                .routing_strategy_overrides
                .write()
                .insert(context.user_id, strategy);
        }
    }
    Json(json!({
        "context": context_payload(&context),
        "user_id": context.user_id,
        "strategy_mode": strategy_mode,
        "old_strategy": format!("{:?}", old),
        "new_strategy": format!("{:?}", new_strategy),
    }))
    .into_response()
}

async fn reload_pool_from_store_for_user_id(state: &AppState, user_id: i64) -> Result<(), String> {
    match state.store.list_accounts_for_user(user_id).await {
        Ok(rows) => {
            let enabled_route_mappings = state
                .store
                .list_enabled_model_route_mappings_for_user(user_id)
                .await
                .map_err(|error| error.to_string())?;
            let accounts: Vec<Account> = rows
                .into_iter()
                .filter_map(|r| {
                    let provider = match r.provider.as_str() {
                        "openai" => sdkwork_lr_core::ProviderKind::Openai,
                        "anthropic" => sdkwork_lr_core::ProviderKind::Anthropic,
                        "google" => sdkwork_lr_core::ProviderKind::Google,
                        other => sdkwork_lr_core::ProviderKind::Custom(other.to_owned()),
                    };
                    let models: Vec<String> = serde_json::from_str(&r.models).unwrap_or_default();
                    let default_headers: std::collections::BTreeMap<String, String> =
                        serde_json::from_str(&r.default_headers.unwrap_or_default())
                            .unwrap_or_default();
                    let mut model_route_mappings: std::collections::BTreeMap<String, String> =
                        serde_json::from_str(&r.model_route_mappings.unwrap_or_default())
                            .unwrap_or_default();
                    for route_mapping in enabled_route_mappings
                        .iter()
                        .filter(|route_mapping| route_mapping.account_id == r.id)
                    {
                        model_route_mappings.insert(
                            route_mapping.client_model.clone(),
                            route_mapping.upstream_model.clone(),
                        );
                    }

                    Some(Account {
                        name: r.name,
                        provider,
                        base_url: r.base_url,
                        upstream_api_key: r.upstream_api_key,
                        upstream_auth_scheme: r.upstream_auth_scheme,
                        models,
                        priority: r.priority as u32,
                        timeout: std::time::Duration::from_secs(r.timeout_secs as u64),
                        max_retries: r.max_retries as u32,
                        retry_delay_ms: r.retry_delay_ms as u64,
                        anthropic_version: r.anthropic_version,
                        default_headers,
                        enabled: r.enabled,
                        model_route_mappings,
                    })
                })
                .collect();

            let existing_pool = state.user_pools.read().get(&user_id).cloned();
            let default_strategy = state
                .config
                .routing
                .strategy_for_account_count(accounts.len());
            let strategy = state
                .routing_strategy_overrides
                .read()
                .get(&user_id)
                .copied()
                .unwrap_or(default_strategy);
            let health_manager = existing_pool
                .as_ref()
                .map(|pool| pool.read().health_manager().clone())
                .unwrap_or_else(|| {
                    std::sync::Arc::new(sdkwork_lr_core::HealthManager::new(
                        sdkwork_lr_core::CircuitBreakerConfig {
                            failure_threshold: state.config.circuit_breaker.failure_threshold,
                            success_threshold: state.config.circuit_breaker.success_threshold,
                            open_duration: state.config.circuit_breaker.open_duration,
                            half_open_max_requests: state
                                .config
                                .circuit_breaker
                                .half_open_max_requests,
                        },
                    ))
                });
            let new_pool = sdkwork_lr_core::AccountPool::with_health_manager(
                accounts,
                strategy,
                health_manager,
            );
            let pool_arc = if user_id == sdkwork_lr_store::DEFAULT_USER_ID {
                *state.pool.write() = new_pool.clone();
                state.pool.clone()
            } else {
                std::sync::Arc::new(parking_lot::RwLock::new(new_pool))
            };
            state.user_pools.write().insert(user_id, pool_arc);
            tracing::info!(user_id = user_id, "account pool hot-reloaded from database");
            Ok(())
        }
        Err(e) => {
            tracing::error!(user_id = user_id, error = %e, "failed to reload account pool from database");
            Err(e.to_string())
        }
    }
}

pub async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    let pool = state.pool.read();
    let hm = pool.health_manager();
    sdkwork_lr_observability::metrics::ACTIVE_ACCOUNTS.set(hm.healthy_count() as i64);
    sdkwork_lr_observability::metrics::DEGRADED_ACCOUNTS.set(hm.degraded_count() as i64);
    sdkwork_lr_observability::metrics::CIRCUIT_OPEN_ACCOUNTS.set(hm.circuit_open_count() as i64);
    drop(pool);

    let body = sdkwork_lr_observability::metrics::gather_metrics();
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_models::{
        CatalogManifest, ModelCatalog, ModelInfo, ModelVendor, SourceEvidence, VendorCatalog,
    };
    use std::collections::BTreeMap;

    #[test]
    fn plugins_payload_includes_route_capability_health() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();

        let payload = plugin_registry_payload(
            &registry,
            &sdkwork_lr_config::BasePathConfig {
                openai: sdkwork_lr_config::LOCAL_ROUTER_OPENAI_PREFIX.to_owned(),
                anthropic: sdkwork_lr_config::LOCAL_ROUTER_ANTHROPIC_PREFIX.to_owned(),
                google: sdkwork_lr_config::LOCAL_ROUTER_GOOGLE_PREFIX.to_owned(),
            },
            true,
            sdkwork_lr_plugin::PluginPolicy::Auto,
            true,
            true,
        );

        let capabilities = payload["route_capabilities"]
            .as_array()
            .expect("route capability list");
        let responses_to_anthropic = capabilities
            .iter()
            .find(|item| item["plugin_id"] == "OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API")
            .expect("Responses to Anthropic capability");
        assert_eq!(responses_to_anthropic["ready"], true);
        assert_eq!(responses_to_anthropic["reason"], "ready");

        let batch_to_responses = capabilities
            .iter()
            .find(|item| item["plugin_id"] == "OPENAI_BATCH_TO_OPENAI_RESPONSES_API")
            .expect("Batch to Responses capability");
        assert_eq!(batch_to_responses["ready"], false);
        assert_eq!(batch_to_responses["reason"], "reserved");

        assert!(payload["capability_summary"]["ready"].as_u64().unwrap() > 0);
        assert!(payload["capability_summary"]["reserved"].as_u64().unwrap() > 0);

        let client_apis = payload["client_apis"].as_array().expect("client APIs");
        let codex = client_apis
            .iter()
            .find(|item| item["code"] == "codex")
            .expect("codex client API standard");
        assert_eq!(codex["default_surface"], "openai_responses");

        let api_surfaces = payload["api_surfaces"].as_array().expect("API surfaces");
        let responses = api_surfaces
            .iter()
            .find(|item| item["code"] == "openai_responses")
            .expect("OpenAI Responses API surface");
        assert_eq!(responses["request_path"], "/v1/responses");
        assert!(responses["required_request_fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "input"));
        assert!(responses["streaming_event_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event == "response.output_text.delta"));

        let gemini = api_surfaces
            .iter()
            .find(|item| item["code"] == "google_gemini")
            .expect("Gemini GenerateContent API surface");
        assert_eq!(
            gemini["stream_path"],
            "/v1/models/{model}:streamGenerateContent?alt=sse"
        );

        let routing_strategies = payload["routing_strategies"]
            .as_array()
            .expect("routing strategies");
        let auto = routing_strategies
            .iter()
            .find(|item| item["code"] == "auto")
            .expect("auto strategy");
        assert_eq!(auto["runtime_update_behavior"], "clear_user_override");
        let round_robin = routing_strategies
            .iter()
            .find(|item| item["code"] == "round_robin")
            .expect("round_robin strategy");
        assert_eq!(
            round_robin["multi_account_behavior"],
            "rotating_candidate_order"
        );

        let pipeline_stages = payload["pipeline_stages"]
            .as_array()
            .expect("pipeline stages");
        assert!(pipeline_stages
            .iter()
            .any(|item| item["code"] == "before_forward"
                && item["extension_method"] == "on_before_forward"
                && item["blocking"] == true));
        assert!(pipeline_stages
            .iter()
            .any(|item| item["code"] == "before_persist"
                && item["extension_method"] == "on_before_persist"
                && item["blocking"] == false));

        let standard_components = payload["standard_components"]
            .as_array()
            .expect("standard components");
        assert!(standard_components
            .iter()
            .any(|item| item["code"] == "billing_meter"));
        assert!(standard_components
            .iter()
            .any(|item| item["code"] == "concurrency_control"));
        let security_policy = standard_components
            .iter()
            .find(|item| item["code"] == "security_policy")
            .expect("security policy standard component");
        assert_eq!(security_policy["built_in_available"], true);
        assert_eq!(security_policy["default_enabled"], false);
        assert_eq!(
            security_policy["configuration_source"],
            "SecurityPolicyInterceptor / SecurityPolicyConfig"
        );
        let audit_metadata = standard_components
            .iter()
            .find(|item| item["code"] == "audit_metadata")
            .expect("audit metadata standard component");
        assert_eq!(audit_metadata["built_in_available"], true);
        assert_eq!(audit_metadata["default_enabled"], true);

        let runtime_plugins = payload["runtime_plugins"]
            .as_array()
            .expect("runtime plugins");
        assert!(runtime_plugins
            .iter()
            .any(|item| item["name"] == "audit_metadata" && item["enabled"] == true));
        assert!(runtime_plugins
            .iter()
            .any(|item| item["name"] == "billing_meter"
                && item["enabled"] == false
                && item["available"] == true));

        let api_groups = payload["api_groups"].as_array().expect("api groups");
        assert!(api_groups
            .iter()
            .any(|item| item["code"] == "local-router-open-api"
                && item["auth_scheme"] == "client_api_key"));
        assert!(api_groups
            .iter()
            .any(|item| item["code"] == "local-router-app-api"
                && item["canonical_prefix"] == "/app/v3/api/local_router"));
        assert!(api_groups
            .iter()
            .any(|item| item["code"] == "local-router-backend-api"
                && item["canonical_prefix"] == "/backend/v3/api/local_router"));
    }

    #[test]
    fn routing_strategy_request_accepts_auto_and_route_mappings() {
        assert!(matches!(
            parse_routing_strategy_update("auto"),
            Ok(RoutingStrategyUpdate::Auto)
        ));
        assert!(matches!(
            parse_routing_strategy_update("roundrobin"),
            Ok(RoutingStrategyUpdate::Explicit(
                sdkwork_lr_core::RoutingStrategy::RoundRobin
            ))
        ));
        assert!(parse_routing_strategy_update("invalid").is_err());
    }

    #[test]
    fn update_account_auth_scheme_patch_distinguishes_missing_null_and_value() {
        let missing: UpdateAccountRequest = serde_json::from_value(json!({})).unwrap();
        let clear: UpdateAccountRequest =
            serde_json::from_value(json!({"upstream_auth_scheme": null})).unwrap();
        let value: UpdateAccountRequest =
            serde_json::from_value(json!({"upstream_auth_scheme": "authorization_bearer"}))
                .unwrap();

        assert!(matches!(missing.upstream_auth_scheme, PatchField::Missing));
        assert!(matches!(clear.upstream_auth_scheme, PatchField::Null));
        assert!(matches!(
            value.upstream_auth_scheme,
            PatchField::Value(ref scheme) if scheme == "authorization_bearer"
        ));
        assert_eq!(
            resolve_upstream_auth_scheme_patch(PatchField::Missing, Some("x-api-key".to_owned()))
                .unwrap(),
            Some("x-api-key".to_owned())
        );
        assert_eq!(
            resolve_upstream_auth_scheme_patch(PatchField::Null, Some("bearer".to_owned()))
                .unwrap(),
            None
        );
        assert_eq!(
            resolve_upstream_auth_scheme_patch(
                PatchField::Value("authorization_bearer".to_owned()),
                None,
            )
            .unwrap(),
            Some("bearer".to_owned())
        );
    }

    #[test]
    fn account_requests_reject_ambiguous_api_key_field() {
        let create_error = serde_json::from_value::<CreateAccountRequest>(json!({
            "name": "ambiguous-key",
            "provider": "openai",
            "base_url": "https://api.example/v1",
            "api_key": "sk-upstream"
        }))
        .unwrap_err();
        assert!(create_error.to_string().contains("unknown field `api_key`"));

        let update_error = serde_json::from_value::<UpdateAccountRequest>(json!({
            "api_key": "sk-upstream"
        }))
        .unwrap_err();
        assert!(update_error.to_string().contains("unknown field `api_key`"));
    }

    #[test]
    fn account_requests_accept_only_explicit_upstream_api_key() {
        let create: CreateAccountRequest = serde_json::from_value(json!({
            "name": "explicit-key",
            "provider": "openai",
            "base_url": "https://api.example/v1",
            "upstream_api_key": "sk-upstream"
        }))
        .unwrap();
        assert_eq!(create.upstream_api_key, "sk-upstream");

        let update: UpdateAccountRequest = serde_json::from_value(json!({
            "upstream_api_key": "sk-upstream-2"
        }))
        .unwrap();
        assert_eq!(update.upstream_api_key.as_deref(), Some("sk-upstream-2"));
    }

    #[test]
    fn management_requests_reject_unknown_fields() {
        let client_key_error = serde_json::from_value::<CreateClientApiKeyRequest>(json!({
            "name": "codex",
            "api_key": "sk-should-not-be-accepted"
        }))
        .unwrap_err();
        assert!(client_key_error
            .to_string()
            .contains("unknown field `api_key`"));

        let mapping_error = serde_json::from_value::<CreateModelRouteMappingRequest>(json!({
            "account_id": 1,
            "client_model": "gpt-5.5",
            "upstream_model": "gemini-2.5-pro",
            "unexpected_mapping_name": "invalid"
        }))
        .unwrap_err();
        assert!(mapping_error
            .to_string()
            .contains("unknown field `unexpected_mapping_name`"));

        let toggle_error = serde_json::from_value::<ToggleAccountRequest>(json!({
            "enabled": true,
            "account_enabled": true
        }))
        .unwrap_err();
        assert!(toggle_error
            .to_string()
            .contains("unknown field `account_enabled`"));

        let strategy_error = serde_json::from_value::<SetStrategyRequest>(json!({
            "strategy": "round_robin",
            "routing_strategy": "random"
        }))
        .unwrap_err();
        assert!(strategy_error
            .to_string()
            .contains("unknown field `routing_strategy`"));
    }

    #[test]
    fn toggle_account_request_defaults_to_enabled() {
        let req: ToggleAccountRequest = serde_json::from_value(json!({})).unwrap();
        assert!(req.enabled);
    }

    #[test]
    fn plugin_decision_payload_includes_model_decision_and_route_capability() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let catalog = catalog_with_vendor_model(
            vendor(
                "anthropic",
                "Anthropic",
                &[("codex", "unsupported", &[])],
                &["anthropic_messages"],
            ),
            model("claude-sonnet-4-6", "anthropic", "anthropic_messages"),
        );

        let payload = plugin_decision_payload(
            &registry,
            Some(&catalog),
            sdkwork_lr_plugin::PluginPolicy::Auto,
            "claude-sonnet-4-6",
            "codex",
            sdkwork_lr_plugin::ApiSurface::OpenAiResponses,
            sdkwork_lr_plugin::ApiSurface::AnthropicMessages,
        );

        assert_eq!(payload["model"], "claude-sonnet-4-6");
        assert_eq!(payload["client_api"], "codex");
        assert_eq!(payload["decision"]["vendor_code"], "anthropic");
        assert_eq!(payload["decision"]["needs_plugin"], true);
        assert_eq!(
            payload["decision"]["plugin_id"],
            "OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API"
        );
        assert_eq!(payload["decision"]["client_support_status"], "unsupported");
        assert_eq!(payload["route_capability"]["ready"], true);
        assert_eq!(payload["route_capability"]["reason"], "ready");
    }

    #[test]
    fn plugin_decision_payload_reflects_passthrough_policy_effective_target() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let catalog = catalog_with_vendor_model(
            vendor(
                "anthropic",
                "Anthropic",
                &[("codex", "unsupported", &[])],
                &["anthropic_messages"],
            ),
            model("claude-sonnet-4-6", "anthropic", "anthropic_messages"),
        );

        let payload = plugin_decision_payload(
            &registry,
            Some(&catalog),
            sdkwork_lr_plugin::PluginPolicy::Passthrough,
            "claude-sonnet-4-6",
            "codex",
            sdkwork_lr_plugin::ApiSurface::OpenAiResponses,
            sdkwork_lr_plugin::ApiSurface::AnthropicMessages,
        );

        assert_eq!(payload["policy"], "passthrough");
        assert_eq!(payload["decision"]["target"], "ANTHROPIC_MESSAGES");
        assert_eq!(payload["decision"]["effective_target"], "OPENAI_RESPONSES");
        assert_eq!(payload["decision"]["needs_plugin"], false);
        assert_eq!(
            payload["decision"]["effective_plugin_id"],
            serde_json::Value::Null
        );
        assert_eq!(payload["route_capability"], serde_json::Value::Null);
    }

    #[test]
    fn plugin_decision_payload_reflects_force_transform_policy_effective_target() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let catalog = catalog_with_vendor_model(
            vendor(
                "google",
                "Google",
                &[("codex", "unsupported", &[])],
                &["google_gemini"],
            ),
            model("gemini-3.5-flash", "google", "google_gemini"),
        );

        let payload = plugin_decision_payload(
            &registry,
            Some(&catalog),
            sdkwork_lr_plugin::PluginPolicy::ForceTransform,
            "gemini-3.5-flash",
            "codex",
            sdkwork_lr_plugin::ApiSurface::OpenAiResponses,
            sdkwork_lr_plugin::ApiSurface::OpenAiChatCompletions,
        );

        assert_eq!(payload["policy"], "force_transform");
        assert_eq!(payload["decision"]["target"], "GEMINI_GENERATE_CONTENT");
        assert_eq!(
            payload["decision"]["effective_target"],
            "OPENAI_CHAT_COMPLETIONS"
        );
        assert_eq!(payload["decision"]["needs_plugin"], true);
        assert_eq!(
            payload["decision"]["effective_plugin_id"],
            "OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API"
        );
        assert_eq!(
            payload["route_capability"]["plugin_id"],
            "OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API"
        );
        assert_eq!(payload["route_capability"]["ready"], true);
    }

    #[test]
    fn plugin_decision_payload_reports_capability_when_policy_introduces_plugin() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let catalog = catalog_with_vendor_model(
            vendor(
                "openai",
                "OpenAI",
                &[("codex", "supported", &["openai_responses"])],
                &["openai_responses", "openai_compatible"],
            ),
            model("gpt-5.5", "openai", "openai_responses"),
        );

        let payload = plugin_decision_payload(
            &registry,
            Some(&catalog),
            sdkwork_lr_plugin::PluginPolicy::ForceTransform,
            "gpt-5.5",
            "codex",
            sdkwork_lr_plugin::ApiSurface::OpenAiResponses,
            sdkwork_lr_plugin::ApiSurface::OpenAiChatCompletions,
        );

        assert_eq!(payload["decision"]["plugin_id"], serde_json::Value::Null);
        assert_eq!(
            payload["decision"]["effective_plugin_id"],
            "OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API"
        );
        assert_eq!(
            payload["route_capability"]["plugin_id"],
            "OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API"
        );
        assert_eq!(payload["route_capability"]["ready"], true);
    }

    #[test]
    fn plugin_decision_payload_falls_back_to_upstream_when_catalog_missing() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();

        let payload = plugin_decision_payload(
            &registry,
            None,
            sdkwork_lr_plugin::PluginPolicy::Auto,
            "unknown-model",
            "codex",
            sdkwork_lr_plugin::ApiSurface::OpenAiResponses,
            sdkwork_lr_plugin::ApiSurface::AnthropicMessages,
        );

        assert_eq!(payload["model_catalog_loaded"], false);
        assert_eq!(payload["decision"]["vendor_code"], "unknown");
        assert_eq!(payload["decision"]["target"], "ANTHROPIC_MESSAGES");
        assert_eq!(
            payload["decision"]["effective_plugin_id"],
            "OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API"
        );
        assert_eq!(payload["route_capability"]["ready"], true);
    }

    fn catalog_with_vendor_model(vendor: ModelVendor, model: ModelInfo) -> ModelCatalog {
        ModelCatalog {
            manifest: CatalogManifest {
                name: "test".to_owned(),
                schema_version: "1.0.0".to_owned(),
                catalog_version: "test".to_owned(),
                generated_at: "2026-06-04T00:00:00Z".to_owned(),
                models_root: "models".to_owned(),
                schemas_root: "schemas".to_owned(),
            },
            meters: vec![],
            protocols: vec![],
            vendors: vec![VendorCatalog {
                vendor_code: vendor.vendor_code.clone(),
                region_code: vendor.region_code.clone(),
                vendor,
                families: vec![],
                models: vec![model],
                pricing: vec![],
                rankings: vec![],
            }],
        }
    }

    fn vendor(
        code: &str,
        display_name: &str,
        compatibility: &[(&str, &str, &[&str])],
        supported_protocols: &[&str],
    ) -> ModelVendor {
        let client_api_compatibility = compatibility
            .iter()
            .map(|(client, status, protocols)| {
                (
                    (*client).to_owned(),
                    sdkwork_models::ClientApiCompatibility {
                        client_api_code: (*client).to_owned(),
                        display_name: (*client).to_owned(),
                        support_status: (*status).to_owned(),
                        protocol_codes: protocols.iter().map(|p| (*p).to_owned()).collect(),
                        api_codes: vec![],
                        resource_codes: vec![],
                        notes: "test".to_owned(),
                        source: source(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        ModelVendor {
            vendor_code: code.to_owned(),
            region_code: "global".to_owned(),
            display_name: display_name.to_owned(),
            legal_name: None,
            description: None,
            website_url: None,
            docs_url: None,
            country_region: None,
            vendor_type: "commercial".to_owned(),
            market_scope: "global".to_owned(),
            billing_currency: "USD".to_owned(),
            billing_jurisdiction: "US".to_owned(),
            operating_regions: vec!["GLOBAL".to_owned()],
            model_families: vec![],
            capabilities: vec!["chat".to_owned()],
            supported_protocols: supported_protocols
                .iter()
                .map(|p| (*p).to_owned())
                .collect(),
            client_api_compatibility,
            open_source: Some(false),
            sort_order: None,
            source: source(),
        }
    }

    fn model(model_id: &str, vendor_code: &str, api_format: &str) -> ModelInfo {
        ModelInfo {
            catalog_key: format!("{vendor_code}/{model_id}"),
            model_id: model_id.to_owned(),
            display_name: model_id.to_owned(),
            vendor_code: vendor_code.to_owned(),
            region_code: "global".to_owned(),
            vendor_name: None,
            family_code: "test".to_owned(),
            primary_capability: "chat".to_owned(),
            capabilities: vec!["chat".to_owned()],
            input_modalities: vec!["text".to_owned()],
            output_modalities: vec!["text".to_owned()],
            api_format: api_format.to_owned(),
            context_tokens: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            supports_tools: true,
            supports_json_schema: true,
            rank_score: None,
            lifecycle: "current".to_owned(),
            release_stage: "active".to_owned(),
            shelf_state: "listed".to_owned(),
            routing_state: "enabled".to_owned(),
            replacement_model: None,
            description: None,
            strengths: vec![],
            color_token: None,
            latency_p50_ms: None,
            latency_p95_ms: None,
            win_rate: None,
            trend_score: None,
            source: source(),
        }
    }

    fn source() -> SourceEvidence {
        SourceEvidence {
            source_url: "https://sdkwork.cloud/test".to_owned(),
            observed_at: "2026-06-04T00:00:00Z".to_owned(),
            source_hash: None,
        }
    }
}
