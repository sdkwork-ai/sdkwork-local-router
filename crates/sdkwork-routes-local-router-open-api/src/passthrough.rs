use std::sync::{Arc, LazyLock};

use axum::extract::{Request, State};
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::json;

use sdkwork_lr_core::{Account, InterceptorError, Invocation, Protocol, ProviderKind};
use sdkwork_lr_plugin::{
    ApiSurface, CompatibilityDecision, ModelCompatibilityResolver, PluginPolicy, TransformContext,
};
use sdkwork_lr_proxy::{AuthInjection, ForwardTarget};
use sdkwork_lr_store::{NewInvocation as StoreInvocation, NewUsage};
use sdkwork_routes_local_router_support::auth;
use sdkwork_routes_local_router_support::state::AppState;
use sdkwork_routes_local_router_support::streaming;
use sdkwork_routes_local_router_support::upstream_auth::auth_from_scheme;

static GEMINI_MODEL_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"/models/([^:]+)").unwrap());

async fn pool_for_user_id(
    state: &AppState,
    user_id: i64,
) -> Result<Arc<parking_lot::RwLock<sdkwork_lr_core::AccountPool>>, String> {
    if let Some(pool) = state.user_pools.read().get(&user_id).cloned() {
        return Ok(pool);
    }

    let rows = state
        .store
        .list_accounts_for_user(user_id)
        .await
        .map_err(|error| error.to_string())?;
    let enabled_route_mappings = state
        .store
        .list_enabled_model_route_mappings_for_user(user_id)
        .await
        .map_err(|error| error.to_string())?;
    let accounts: Vec<Account> = rows
        .into_iter()
        .map(|row| {
            let provider = sdkwork_lr_core::ProviderKind::from_str_loose(&row.provider);
            let models = serde_json::from_str(&row.models).unwrap_or_default();
            let default_headers =
                serde_json::from_str(&row.default_headers.unwrap_or_default()).unwrap_or_default();
            let mut model_route_mappings: std::collections::BTreeMap<String, String> =
                serde_json::from_str(&row.model_route_mappings.unwrap_or_default())
                    .unwrap_or_default();
            for route_mapping in enabled_route_mappings
                .iter()
                .filter(|route_mapping| route_mapping.account_id == row.id)
            {
                model_route_mappings.insert(
                    route_mapping.client_model.clone(),
                    route_mapping.upstream_model.clone(),
                );
            }
            Account {
                name: row.name,
                provider,
                base_url: row.base_url,
                upstream_api_key: row.upstream_api_key,
                upstream_auth_scheme: row.upstream_auth_scheme,
                models,
                priority: row.priority as u32,
                timeout: std::time::Duration::from_secs(row.timeout_secs as u64),
                max_retries: row.max_retries as u32,
                retry_delay_ms: row.retry_delay_ms as u64,
                anthropic_version: row.anthropic_version,
                default_headers,
                enabled: row.enabled,
                model_route_mappings,
            }
        })
        .collect();
    let health_manager = Arc::new(sdkwork_lr_core::HealthManager::new(
        sdkwork_lr_core::CircuitBreakerConfig {
            failure_threshold: state.config.circuit_breaker.failure_threshold,
            success_threshold: state.config.circuit_breaker.success_threshold,
            open_duration: state.config.circuit_breaker.open_duration,
            half_open_max_requests: state.config.circuit_breaker.half_open_max_requests,
        },
    ));
    let strategy = state
        .routing_strategy_overrides
        .read()
        .get(&user_id)
        .copied()
        .unwrap_or_else(|| {
            state
                .config
                .routing
                .strategy_for_account_count(accounts.len())
        });
    let pool = Arc::new(parking_lot::RwLock::new(
        sdkwork_lr_core::AccountPool::with_health_manager(accounts, strategy, health_manager),
    ));
    state.user_pools.write().insert(user_id, pool.clone());
    Ok(pool)
}

#[derive(Debug, Clone)]
struct TransformPlan {
    context: TransformContext,
    policy: PluginPolicy,
    client_api_code: String,
    decision: Option<CompatibilityDecision>,
    plugin_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransformPreflightError {
    status: StatusCode,
    message: String,
}

impl TransformPreflightError {
    fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            message: message.into(),
        }
    }
}

pub fn routes(
    openai_prefix: &str,
    anthropic_prefix: &str,
    google_prefix: &str,
) -> Router<AppState> {
    Router::new()
        .route(&format!("{openai_prefix}/{{*path}}"), any(handle_openai))
        .route(
            &format!("{anthropic_prefix}/{{*path}}"),
            any(handle_anthropic),
        )
        .route(&format!("{google_prefix}/{{*path}}"), any(handle_google))
}

async fn handle_openai(State(state): State<AppState>, req: Request) -> Response {
    handle_provider_passthrough(state, req, Protocol::Openai).await
}

async fn handle_anthropic(State(state): State<AppState>, req: Request) -> Response {
    handle_provider_passthrough(state, req, Protocol::Anthropic).await
}

async fn handle_google(State(state): State<AppState>, req: Request) -> Response {
    handle_provider_passthrough(state, req, Protocol::Google).await
}

async fn handle_provider_passthrough(
    state: AppState,
    req: Request,
    client_protocol: Protocol,
) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().map(|q| q.to_owned());
    let method = req.method().clone();
    let headers = req.headers().clone();
    let request_context = auth::context_from_request(&req);
    let user_id = request_context.user_id;

    let mut invocation = Invocation::new(
        request_id.clone(),
        client_protocol,
        method.to_string(),
        path.clone(),
    )
    .with_user_id(user_id)
    .with_api_group(request_context.api_group);

    let body_limit = state.config.server.max_body_size_bytes();
    let body = match axum::body::to_bytes(req.into_body(), body_limit).await {
        Ok(b) => b,
        Err(e) => {
            return protocol_error(
                client_protocol,
                StatusCode::BAD_REQUEST,
                &format!("failed to read body: {e}"),
                &request_id,
            );
        }
    };

    let model = extract_model(&body, &path, client_protocol);
    sdkwork_lr_observability::metrics::REQUESTS_TOTAL.inc();
    if let Some(ref m) = model {
        invocation = invocation.with_model(m);
    }

    if let Some(ref q) = query {
        invocation = invocation.with_query(q);
    }

    if state.config.recording.save_request_body {
        invocation.set_request_body(&String::from_utf8_lossy(&body));
    }

    if let Err(error) = state.interceptor_chain.before_request(&mut invocation) {
        return interceptor_error_response(
            &state,
            user_id,
            client_protocol,
            &request_id,
            &mut invocation,
            error,
        )
        .await;
    }
    if let Err(error) = state
        .interceptor_chain
        .after_request_body_read(&mut invocation, body.len())
    {
        return interceptor_error_response(
            &state,
            user_id,
            client_protocol,
            &request_id,
            &mut invocation,
            error,
        )
        .await;
    }

    let is_streaming = detect_streaming(&body, client_protocol, &headers, query.as_deref());

    let client_base_path = match client_protocol {
        Protocol::Openai => state.config.base_paths.openai.as_str(),
        Protocol::Anthropic => state.config.base_paths.anthropic.as_str(),
        Protocol::Google => state.config.base_paths.google.as_str(),
    };

    let pool_arc = match pool_for_user_id(&state, user_id).await {
        Ok(pool) => pool,
        Err(error) => {
            invocation.set_error(&error);
            persist_invocation(&state, user_id, &mut invocation).await;
            return protocol_error(
                client_protocol,
                StatusCode::INTERNAL_SERVER_ERROR,
                &error,
                &request_id,
            );
        }
    };

    let mut candidates: Vec<Arc<Account>> = match model.as_deref() {
        Some(m) => pool_arc.read().select_all(m),
        None => pool_arc.read().select_available_accounts(),
    };
    filter_non_generation_candidates(&mut candidates, client_protocol, &path, client_base_path);

    if let Err(error) = state
        .interceptor_chain
        .route_candidates(&mut invocation, &mut candidates)
    {
        return interceptor_error_response(
            &state,
            user_id,
            client_protocol,
            &request_id,
            &mut invocation,
            error,
        )
        .await;
    }

    if candidates.is_empty() {
        invocation.set_error("no upstream account available");
        let _ = state
            .interceptor_chain
            .notify_error(&invocation, "no upstream account available");
        persist_invocation(&state, user_id, &mut invocation).await;
        return protocol_error(
            client_protocol,
            StatusCode::SERVICE_UNAVAILABLE,
            "no upstream account available",
            &request_id,
        );
    }

    let max_attempts = if state.config.fallback.enabled {
        state.config.fallback.max_attempts.min(5) as usize
    } else {
        1
    };

    let mut last_error = String::new();

    for (attempt, account) in candidates.iter().enumerate() {
        if attempt >= max_attempts {
            break;
        }

        if attempt > 0 {
            let backoff_ms = 100u64 * (1 << (attempt - 1)).min(4);
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        }

        invocation = invocation.with_account(&account.name);
        invocation.set_forwarding();
        if let Err(error) = state
            .interceptor_chain
            .account_selected(&mut invocation, account.as_ref())
        {
            return interceptor_error_response(
                &state,
                user_id,
                client_protocol,
                &request_id,
                &mut invocation,
                error,
            )
            .await;
        }

        let upstream_protocol = account.provider.to_protocol();
        let upstream_model = model
            .as_deref()
            .and_then(|requested_model| resolve_upstream_model(account.as_ref(), requested_model))
            .map(str::to_owned);
        let transform_plan = build_transform_plan(
            &state,
            client_protocol,
            upstream_protocol,
            &path,
            client_base_path,
            model.as_deref(),
            upstream_model.as_deref(),
            is_streaming,
            &headers,
        );
        let transform_context = &transform_plan.context;
        let needs_transform = transform_context.source != transform_context.target;

        if let Err(error) = preflight_transform_plan(&state.transform_registry, &transform_plan) {
            invocation.set_error(&error.message);
            let _ = state
                .interceptor_chain
                .notify_error(&invocation, &error.message);
            persist_invocation(&state, user_id, &mut invocation).await;
            return protocol_error(client_protocol, error.status, &error.message, &request_id);
        }

        let upstream_path = map_upstream_path(&state, transform_context).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "plugin path mapping failed, using protocol fallback");
            sdkwork_lr_transform::streaming::map_upstream_path(
                &path,
                client_base_path,
                upstream_protocol,
                upstream_model.as_deref(),
                is_streaming,
            )
        });

        let upstream_path_and_query = append_query(upstream_path, query.as_deref());

        let account_max_retries = account.max_retries.min(3) as usize;
        let account_retry_delay = account.retry_delay_ms.max(100);

        let mut account_attempt = 0;
        let total_account_attempts = 1 + account_max_retries;

        loop {
            invocation.set_routing_metadata(
                upstream_protocol.to_string(),
                upstream_path_and_query.clone(),
                transform_plan.client_api_code.clone(),
                transform_plan.context.source.canonical_token().to_owned(),
                transform_plan.context.target.canonical_token().to_owned(),
                transform_plan.policy.as_str().to_owned(),
                transform_plan.plugin_id.clone(),
                transform_plan
                    .decision
                    .as_ref()
                    .map(|decision| decision.vendor_code.clone()),
                is_streaming,
                (attempt + account_attempt + 1) as i32,
            );
            if let Err(error) = state.interceptor_chain.before_transform(&mut invocation) {
                return interceptor_error_response(
                    &state,
                    user_id,
                    client_protocol,
                    &request_id,
                    &mut invocation,
                    error,
                )
                .await;
            }
            let forward_body = match build_forward_body(
                &body,
                transform_context,
                account,
                transform_plan.policy,
            ) {
                Ok(body) => body,
                Err(error) => {
                    invocation.set_error(&error);
                    let _ = state.interceptor_chain.notify_error(&invocation, &error);
                    persist_invocation(&state, user_id, &mut invocation).await;
                    return protocol_error(
                        client_protocol,
                        StatusCode::BAD_REQUEST,
                        &error,
                        &request_id,
                    );
                }
            };

            let auth = build_auth_for_provider(account);
            let target = ForwardTarget {
                base_url: account.base_url.clone(),
                auth,
                default_headers: account
                    .default_headers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                anthropic_version: effective_anthropic_version(account),
                timeout: Some(account.timeout),
                request_id: request_id.clone(),
            };

            tracing::info!(attempt = attempt + 1, account = %account.name, upstream_protocol = %upstream_protocol, request_id = %request_id, retry = account_attempt, "forwarding request");

            if let Err(error) = state.interceptor_chain.before_forward(&mut invocation) {
                return interceptor_error_response(
                    &state,
                    user_id,
                    client_protocol,
                    &request_id,
                    &mut invocation,
                    error,
                )
                .await;
            }

            let result = sdkwork_lr_proxy::forward_raw(
                &state.client,
                method.clone(),
                &headers,
                forward_body,
                &target,
                &upstream_path_and_query,
            )
            .await;

            match result {
                Ok(hyper_response) => {
                    let status = hyper_response.status();
                    if let Err(error) = state
                        .interceptor_chain
                        .after_upstream_response(&mut invocation, status.as_u16())
                    {
                        return interceptor_error_response(
                            &state,
                            user_id,
                            client_protocol,
                            &request_id,
                            &mut invocation,
                            error,
                        )
                        .await;
                    }

                    if should_fallback(status) {
                        if account_attempt + 1 < total_account_attempts
                            && is_retryable_status(status)
                        {
                            account_attempt += 1;
                            let delay = account_retry_delay * (1 << account_attempt.min(3));
                            tracing::warn!(status = %status, account = %account.name, request_id = %request_id, retry = account_attempt, delay_ms = delay, "retryable error, retrying same account");
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            continue;
                        }

                        {
                            let pool = pool_arc.read();
                            pool.health_manager().record_failure(&account.name);
                        }

                        if attempt + 1 < max_attempts {
                            last_error = format!("upstream {} returned {}", account.name, status);
                            tracing::warn!(status = %status, account = %account.name, request_id = %request_id, "upstream error, trying fallback");
                            break;
                        }
                    }

                    {
                        let pool = pool_arc.read();
                        pool.record_latency(&account.name, invocation.latency_ms() as u64);

                        if status.is_success() {
                            pool.health_manager().record_success(&account.name);
                            sdkwork_lr_observability::metrics::REQUESTS_SUCCESS.inc();
                        } else if should_fallback(status) {
                            pool.health_manager().record_failure(&account.name);
                            sdkwork_lr_observability::metrics::REQUESTS_FAILURE.inc();
                            sdkwork_lr_observability::metrics::FALLBACK_TOTAL.inc();
                        }
                    }

                    sdkwork_lr_observability::metrics::REQUEST_DURATION
                        .with_label_values(&[
                            account.provider.to_string().as_str(),
                            model.as_deref().unwrap_or("unknown"),
                        ])
                        .observe(invocation.latency_ms() as f64 / 1000.0);

                    let response = handle_upstream_response(
                        hyper_response,
                        client_protocol,
                        upstream_protocol,
                        &transform_plan,
                        needs_transform,
                        is_streaming,
                        model.as_deref(),
                        upstream_model.as_deref(),
                        &request_id,
                        &state,
                        user_id,
                        &mut invocation,
                    )
                    .await;

                    return response;
                }
                Err(e) => {
                    if account_attempt + 1 < total_account_attempts {
                        account_attempt += 1;
                        let delay = account_retry_delay * (1 << account_attempt.min(3));
                        tracing::warn!(error = %e, account = %account.name, request_id = %request_id, retry = account_attempt, delay_ms = delay, "connection error, retrying same account");
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    last_error = e.clone();
                    {
                        let pool = pool_arc.read();
                        pool.health_manager().record_failure(&account.name);
                    }
                    sdkwork_lr_observability::metrics::REQUESTS_FAILURE.inc();
                    tracing::warn!(error = %e, account = %account.name, request_id = %request_id, "forward failed, trying fallback");
                    break;
                }
            }
        }
    }

    invocation.set_error(&last_error);
    let _ = state
        .interceptor_chain
        .notify_error(&invocation, &last_error);
    persist_invocation(&state, user_id, &mut invocation).await;

    protocol_error(
        client_protocol,
        StatusCode::BAD_GATEWAY,
        &last_error,
        &request_id,
    )
}

async fn handle_upstream_response(
    hyper_response: hyper::Response<hyper::body::Incoming>,
    client_protocol: Protocol,
    upstream_protocol: Protocol,
    transform_plan: &TransformPlan,
    needs_transform: bool,
    is_streaming: bool,
    model: Option<&str>,
    upstream_model: Option<&str>,
    request_id: &str,
    state: &AppState,
    user_id: i64,
    invocation: &mut Invocation,
) -> Response {
    let transform_context = &transform_plan.context;
    let response_model = model
        .or(transform_context.model.as_deref())
        .unwrap_or("unknown");
    let stream_model = upstream_model
        .or(transform_context.upstream_model.as_deref())
        .unwrap_or(response_model);
    let status = hyper_response.status();
    let (parts, incoming_body) = hyper_response.into_parts();

    if status.is_client_error() {
        let resp_bytes = match read_body_bytes(incoming_body).await {
            Ok(b) => b,
            Err(e) => {
                return protocol_error(client_protocol, status, &e, request_id);
            }
        };

        let final_body = if needs_transform {
            match transform_response(&resp_bytes, transform_context, response_model) {
                Ok(transformed) => serde_json_to_bytes(&transformed),
                Err(_) => resp_bytes,
            }
        } else {
            resp_bytes
        };

        invocation.set_response(status.as_u16(), None);
        let _ = state.interceptor_chain.after_response(invocation);
        persist_invocation(state, user_id, invocation).await;

        let mut response = build_response_from_parts(parts, final_body, request_id);
        add_plugin_decision_headers(
            &mut response,
            transform_plan,
            state.config.plugins.expose_decision_headers,
        );
        return response;
    }

    if is_streaming && needs_transform {
        let hyper_resp = hyper::Response::from_parts(parts, incoming_body);
        invocation.set_response(status.as_u16(), None);
        let completion_state = state.clone();
        let mut completion_invocation = invocation.clone();

        let mut response = streaming::wrap_streaming_response(
            hyper_resp,
            upstream_protocol,
            client_protocol,
            transform_context.target,
            transform_context.source,
            response_model.to_owned(),
            request_id.to_owned(),
            move |usage| {
                Box::pin(async move {
                    if let Some(usage) = usage {
                        record_token_metrics(&usage);
                        completion_invocation.set_token_usage(usage);
                    }
                    let _ = completion_state
                        .interceptor_chain
                        .after_response(&mut completion_invocation);
                    persist_invocation(&completion_state, user_id, &mut completion_invocation)
                        .await;
                })
            },
        );
        add_plugin_decision_headers(
            &mut response,
            transform_plan,
            state.config.plugins.expose_decision_headers,
        );
        return response;
    }

    if is_streaming {
        invocation.set_response(status.as_u16(), None);
        let completion_state = state.clone();
        let mut completion_invocation = invocation.clone();

        let mut response = streaming::wrap_streaming_response(
            hyper::Response::from_parts(parts, incoming_body),
            upstream_protocol,
            upstream_protocol,
            transform_context.target,
            transform_context.target,
            stream_model.to_owned(),
            request_id.to_owned(),
            move |usage| {
                Box::pin(async move {
                    if let Some(usage) = usage {
                        record_token_metrics(&usage);
                        completion_invocation.set_token_usage(usage);
                    }
                    let _ = completion_state
                        .interceptor_chain
                        .after_response(&mut completion_invocation);
                    persist_invocation(&completion_state, user_id, &mut completion_invocation)
                        .await;
                })
            },
        );
        add_request_id_header(&mut response, request_id);
        add_plugin_decision_headers(
            &mut response,
            transform_plan,
            state.config.plugins.expose_decision_headers,
        );
        return response;
    }

    let resp_bytes = match read_body_bytes(incoming_body).await {
        Ok(b) => b,
        Err(e) => {
            return protocol_error(
                client_protocol,
                StatusCode::BAD_GATEWAY,
                &format!("failed to read response: {e}"),
                request_id,
            );
        }
    };

    let final_body = if needs_transform {
        match transform_response(&resp_bytes, transform_context, response_model) {
            Ok(transformed) => {
                let parsed = transformed;
                invocation.extract_usage_from_response(&parsed, client_protocol);
                if let Some(ref usage) = invocation.token_usage {
                    record_token_metrics(usage);
                }
                serde_json_to_bytes(&parsed)
            }
            Err(e) => {
                tracing::warn!(error = %e, "response transform failed, returning original body");
                resp_bytes
            }
        }
    } else {
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&resp_bytes) {
            invocation.extract_usage_from_response(&parsed, upstream_protocol);
            if let Some(ref usage) = invocation.token_usage {
                record_token_metrics(usage);
            }
        }
        resp_bytes
    };

    let response_body_str = if state.config.recording.save_response_body {
        Some(String::from_utf8_lossy(&final_body).to_string())
    } else {
        None
    };

    invocation.set_response(status.as_u16(), response_body_str.as_deref());
    let _ = state.interceptor_chain.after_response(invocation);
    persist_invocation(state, user_id, invocation).await;

    let mut response = build_response_from_parts(parts, final_body, request_id);
    add_plugin_decision_headers(
        &mut response,
        transform_plan,
        state.config.plugins.expose_decision_headers,
    );
    response
}

async fn interceptor_error_response(
    state: &AppState,
    user_id: i64,
    protocol: Protocol,
    request_id: &str,
    invocation: &mut Invocation,
    error: InterceptorError,
) -> Response {
    let status =
        StatusCode::from_u16(error.http_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    invocation
        .metadata
        .insert("interceptor_error_code".to_owned(), error.code.clone());
    invocation.set_error(&error.message);
    state
        .interceptor_chain
        .notify_error(invocation, &error.message);
    persist_invocation(state, user_id, invocation).await;
    protocol_error_with_code(protocol, status, &error.message, &error.code, request_id)
}

async fn persist_invocation(state: &AppState, user_id: i64, invocation: &mut Invocation) {
    if let Err(error) = state.interceptor_chain.before_persist(invocation) {
        tracing::warn!(
            request_id = %invocation.id,
            error_code = %error.code,
            error = %error.message,
            "before_persist interceptor failed; continuing invocation persistence"
        );
        invocation
            .metadata
            .insert("before_persist_error_code".to_owned(), error.code);
    }

    let store = state.store.clone();
    let metadata = serde_json::to_string(&invocation.metadata).unwrap_or_else(|_| "{}".to_owned());
    let store_inv = StoreInvocation {
        user_id,
        request_id: invocation.id.clone(),
        account_name: invocation.account_name.clone(),
        protocol: invocation.protocol.to_string(),
        method: invocation.method.clone(),
        path: invocation.path.clone(),
        query: invocation.query.clone(),
        model: invocation.model.clone(),
        status: invocation.status.to_string(),
        status_code: invocation.status_code.map(|s| s as i32),
        latency_ms: Some(invocation.latency_ms()),
        error_message: invocation.error.clone(),
        request_body: invocation.request_body.clone(),
        response_body: invocation.response_body.clone(),
        request_body_size: invocation
            .request_body
            .as_ref()
            .map(|body| body.len() as i64),
        response_body_size: invocation
            .response_body
            .as_ref()
            .map(|body| body.len() as i64),
        upstream_protocol: invocation.upstream_protocol.clone(),
        upstream_path: invocation.upstream_path.clone(),
        client_api: invocation.client_api.clone(),
        request_surface: invocation.request_surface.clone(),
        target_surface: invocation.target_surface.clone(),
        plugin_policy: invocation.plugin_policy.clone(),
        plugin_id: invocation.plugin_id.clone(),
        model_vendor: invocation.model_vendor.clone(),
        metadata: Some(metadata),
        streaming: invocation.streaming,
        attempt_count: invocation.attempt_count,
    };

    let usage = invocation.token_usage.clone();
    let usage_model = invocation.model.clone();
    let usage_request_id = invocation.id.clone();

    tokio::spawn(async move {
        if let Err(e) = store.insert_invocation(&store_inv).await {
            tracing::error!(request_id = %store_inv.request_id, error = %e, "failed to persist invocation");
        }
        if let Some(u) = usage {
            let new_usage = NewUsage {
                user_id,
                request_id: usage_request_id,
                model: usage_model,
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            };
            if let Err(e) = store.insert_usage(&new_usage).await {
                tracing::error!(request_id = %new_usage.request_id, error = %e, "failed to persist usage");
            }
        }
    });
}

fn record_token_metrics(usage: &sdkwork_lr_core::TokenUsage) {
    if let Some(tokens) = usage.prompt_tokens.filter(|tokens| *tokens > 0) {
        sdkwork_lr_observability::metrics::TOKENS_INPUT_TOTAL.inc_by(tokens as u64);
    }
    if let Some(tokens) = usage.completion_tokens.filter(|tokens| *tokens > 0) {
        sdkwork_lr_observability::metrics::TOKENS_OUTPUT_TOTAL.inc_by(tokens as u64);
    }
}

fn build_forward_body(
    body: &[u8],
    context: &TransformContext,
    account: &Account,
    policy: PluginPolicy,
) -> Result<Bytes, String> {
    let needs_transform = context.source != context.target;
    if matches!(policy, PluginPolicy::Passthrough) {
        return match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(mut parsed) => {
                apply_model_route_mapping(&mut parsed, account);
                ensure_target_model_field(&mut parsed, context);
                Ok(serde_json_to_bytes(&parsed))
            }
            Err(e) => {
                tracing::debug!(error = %e, "body is not valid JSON, forwarding as-is");
                Ok(Bytes::copy_from_slice(body))
            }
        };
    }

    if needs_transform {
        match transform_request(body, context) {
            Ok(mut transformed) => {
                apply_model_route_mapping(&mut transformed, account);
                ensure_target_model_field(&mut transformed, context);
                Ok(serde_json_to_bytes(&transformed))
            }
            Err(e) => {
                if matches!(policy, PluginPolicy::Strict) {
                    return Err(format!("request transform failed: {e}"));
                }
                tracing::warn!(error = %e, "request transform failed, forwarding original body");
                Ok(Bytes::copy_from_slice(body))
            }
        }
    } else {
        match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(mut parsed) => {
                apply_model_route_mapping(&mut parsed, account);
                ensure_target_model_field(&mut parsed, context);
                Ok(serde_json_to_bytes(&parsed))
            }
            Err(e) => {
                tracing::debug!(error = %e, "body is not valid JSON, forwarding as-is");
                Ok(Bytes::copy_from_slice(body))
            }
        }
    }
}

fn build_response_from_parts(
    parts: axum::http::response::Parts,
    body: Bytes,
    request_id: &str,
) -> Response<axum::body::Body> {
    let mut response = Response::new(axum::body::Body::from(body));
    *response.status_mut() = parts.status;
    for (name, value) in parts.headers.iter() {
        if name != header::CONTENT_LENGTH && name != header::TRANSFER_ENCODING {
            response.headers_mut().append(name, value.clone());
        }
    }
    add_request_id_header(&mut response, request_id);
    response
}

fn add_request_id_header(response: &mut Response<axum::body::Body>, request_id: &str) {
    if let Ok(val) = header::HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", val);
    }
}

fn add_plugin_decision_headers(
    response: &mut Response<axum::body::Body>,
    plan: &TransformPlan,
    expose_headers: bool,
) {
    if !expose_headers {
        return;
    }

    insert_header(response, "x-sdkwork-plugin-policy", plan.policy.as_str());
    insert_header(
        response,
        "x-sdkwork-plugin-source",
        plan.context.source.canonical_token(),
    );
    insert_header(
        response,
        "x-sdkwork-plugin-target",
        plan.context.target.canonical_token(),
    );
    if let Some(plugin_id) = plan.plugin_id.as_deref() {
        insert_header(response, "x-sdkwork-plugin-id", plugin_id);
    }
    insert_header(response, "x-sdkwork-client-api", &plan.client_api_code);
    if let Some(decision) = plan.decision.as_ref() {
        insert_header(response, "x-sdkwork-model-vendor", &decision.vendor_code);
    }
}

fn insert_header(response: &mut Response<axum::body::Body>, name: &'static str, value: &str) {
    if let Ok(val) = header::HeaderValue::from_str(value) {
        response
            .headers_mut()
            .insert(header::HeaderName::from_static(name), val);
    }
}

fn serde_json_to_bytes(value: &serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(value).unwrap_or_default())
}

fn protocol_error(
    protocol: Protocol,
    status: StatusCode,
    message: &str,
    request_id: &str,
) -> Response {
    let mut response = match protocol {
        Protocol::Anthropic => (
            status,
            Json(json!({"type": "error", "error": {"type": "api_error", "message": message}}))
        ).into_response(),
        Protocol::Google => (
            status,
            Json(json!({"error": {"code": status.as_u16(), "message": message, "status": "INTERNAL"}}))
        ).into_response(),
        _ => (
            status,
            Json(json!({"error": {"message": message, "type": "upstream_error", "code": status.as_u16()}}))
        ).into_response(),
    };
    add_request_id_header(&mut response, request_id);
    response
}

fn protocol_error_with_code(
    protocol: Protocol,
    status: StatusCode,
    message: &str,
    code: &str,
    request_id: &str,
) -> Response {
    let mut response = match protocol {
        Protocol::Anthropic => (
            status,
            Json(json!({"type": "error", "error": {"type": code, "message": message}}))
        ).into_response(),
        Protocol::Google => (
            status,
            Json(json!({"error": {"code": status.as_u16(), "message": message, "status": "INTERNAL"}}))
        ).into_response(),
        _ => (
            status,
            Json(json!({"error": {"message": message, "type": "interceptor_error", "code": code}}))
        ).into_response(),
    };
    add_request_id_header(&mut response, request_id);
    response
}

async fn read_body_bytes(body: hyper::body::Incoming) -> Result<Bytes, String> {
    use http_body_util::BodyExt;
    body.collect()
        .await
        .map(|c| c.to_bytes())
        .map_err(|e| format!("failed to read response body: {e}"))
}

fn build_auth_for_provider(account: &Account) -> Option<AuthInjection> {
    if account.upstream_api_key.is_empty() {
        return None;
    }
    if let Some(auth) = auth_from_scheme(
        account.upstream_auth_scheme.as_deref(),
        &account.upstream_api_key,
    ) {
        return Some(auth);
    }
    match account.provider {
        ProviderKind::Openai | ProviderKind::Custom(_) => {
            Some(AuthInjection::Bearer(account.upstream_api_key.clone()))
        }
        ProviderKind::Anthropic => Some(AuthInjection::Header(
            "x-api-key".to_owned(),
            account.upstream_api_key.clone(),
        )),
        ProviderKind::Google => Some(AuthInjection::Header(
            "x-goog-api-key".to_owned(),
            account.upstream_api_key.clone(),
        )),
    }
}

fn effective_anthropic_version(account: &Account) -> Option<String> {
    match account.provider {
        ProviderKind::Anthropic => account
            .anthropic_version
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| Some("2023-06-01".to_owned())),
        _ => account.anthropic_version.clone(),
    }
}

fn extract_model(body: &[u8], path: &str, protocol: Protocol) -> Option<String> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(model) = value.get("model").and_then(|v| v.as_str()) {
            return Some(model.to_owned());
        }
    }

    if protocol == Protocol::Google {
        if let Some(captures) = GEMINI_MODEL_REGEX.captures(path) {
            if let Some(m) = captures.get(1) {
                return Some(
                    urlencoding::decode(m.as_str())
                        .map(|model| model.into_owned())
                        .unwrap_or_else(|_| m.as_str().to_owned()),
                );
            }
        }
    }

    None
}

fn detect_streaming(
    body: &[u8],
    protocol: Protocol,
    headers: &HeaderMap,
    query: Option<&str>,
) -> bool {
    if let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) {
        if accept.contains("text/event-stream") {
            return true;
        }
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return false;
    };
    match protocol {
        Protocol::Openai => value
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        Protocol::Anthropic => value
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        Protocol::Google => {
            value
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || query_has_value(query, "alt", "sse")
                || query_contains_case_insensitive(query, "streamgeneratecontent")
        }
    }
}

fn build_transform_plan(
    state: &AppState,
    client_protocol: Protocol,
    upstream_protocol: Protocol,
    path: &str,
    client_base_path: &str,
    requested_model: Option<&str>,
    upstream_model: Option<&str>,
    is_streaming: bool,
    headers: &HeaderMap,
) -> TransformPlan {
    let request_surface =
        ApiSurface::from_protocol_and_path(client_protocol, path, client_base_path);
    let upstream_surface =
        upstream_surface_for_request(client_protocol, upstream_protocol, request_surface);
    let client_api_code = resolve_client_api_code(client_protocol, request_surface, headers);
    let decision_model = upstream_model.or(requested_model);
    let decision = decision_model.and_then(|model_id| {
        state.model_catalog.as_ref().map(|catalog| {
            let resolver = ModelCompatibilityResolver::new(catalog);
            resolver.decide(
                model_id,
                &client_api_code,
                request_surface,
                upstream_surface,
            )
        })
    });
    if let Some(decision) = decision.as_ref() {
        tracing::debug!(
            policy = %effective_plugin_policy(&state.config.plugins).as_str(),
            model = %decision.model_id,
            vendor = %decision.vendor_code,
            client_api = %decision.client_api_code,
            source = %decision.source,
            target = %decision.target,
            plugin_id = ?decision.plugin_id,
            reason = %decision.reason,
            "model compatibility plugin decision"
        );
    }
    let catalog_target_surface = decision.as_ref().map(|decision| decision.target);
    let plugin_policy = effective_plugin_policy(&state.config.plugins);
    let target_surface = sdkwork_lr_plugin::select_target_surface_for_policy(
        plugin_policy,
        request_surface,
        upstream_surface,
        catalog_target_surface,
    );
    let plugin_id = if request_surface == target_surface {
        None
    } else {
        Some(sdkwork_lr_plugin::canonical_plugin_id(
            request_surface,
            target_surface,
        ))
    };

    let context = TransformContext {
        source: request_surface,
        target: target_surface,
        source_protocol: client_protocol,
        target_protocol: upstream_protocol,
        client_path: path.to_owned(),
        client_base_path: client_base_path.to_owned(),
        upstream_model: upstream_model.map(str::to_owned),
        model: requested_model.map(str::to_owned),
        is_streaming,
    };

    TransformPlan {
        context,
        policy: plugin_policy,
        client_api_code,
        decision,
        plugin_id,
    }
}

fn upstream_surface_for_request(
    client_protocol: Protocol,
    upstream_protocol: Protocol,
    request_surface: ApiSurface,
) -> ApiSurface {
    if client_protocol == upstream_protocol {
        request_surface
    } else {
        sdkwork_lr_transform::plugins::protocol_default_surface(upstream_protocol)
    }
}

fn filter_non_generation_candidates(
    candidates: &mut Vec<Arc<Account>>,
    client_protocol: Protocol,
    path: &str,
    client_base_path: &str,
) {
    if is_generation_request_path(path, client_base_path) {
        return;
    }

    candidates.retain(|account| account.provider.to_protocol() == client_protocol);
}

fn is_generation_request_path(path: &str, client_base_path: &str) -> bool {
    let stripped = sdkwork_lr_plugin::strip_base_path(path, client_base_path);
    let normalized = stripped.trim_start_matches('/').to_ascii_lowercase();
    normalized == "chat/completions"
        || normalized == "v1/chat/completions"
        || normalized == "responses"
        || normalized == "v1/responses"
        || normalized == "messages"
        || normalized == "v1/messages"
        || normalized == "batches"
        || normalized == "v1/batches"
        || normalized.contains(":generatecontent")
        || normalized.contains(":streamgeneratecontent")
}

fn preflight_transform_plan(
    registry: &sdkwork_lr_plugin::PluginRegistry,
    plan: &TransformPlan,
) -> Result<(), TransformPreflightError> {
    if plan.context.source == plan.context.target {
        return Ok(());
    }

    let plugin_id = plan.plugin_id.clone().unwrap_or_else(|| {
        sdkwork_lr_plugin::canonical_plugin_id(plan.context.source, plan.context.target)
    });
    let Some(plugin) = registry.resolve(plan.context.source, plan.context.target) else {
        return Err(TransformPreflightError::not_implemented(format!(
            "missing transform plugin {plugin_id} for {} -> {}",
            plan.context.source, plan.context.target
        )));
    };

    let capabilities = plugin.manifest().capabilities;
    if !capabilities.request_body {
        return Err(TransformPreflightError::not_implemented(format!(
            "transform plugin {plugin_id} does not support request body transforms"
        )));
    }
    if !capabilities.response_body {
        return Err(TransformPreflightError::not_implemented(format!(
            "transform plugin {plugin_id} does not support response body transforms"
        )));
    }
    if !capabilities.path {
        return Err(TransformPreflightError::not_implemented(format!(
            "transform plugin {plugin_id} does not support path mapping"
        )));
    }
    if plan.context.is_streaming && !capabilities.stream {
        return Err(TransformPreflightError::not_implemented(format!(
            "transform plugin {plugin_id} does not support streaming transforms"
        )));
    }

    Ok(())
}

fn effective_plugin_policy(config: &sdkwork_lr_config::PluginConfig) -> PluginPolicy {
    if config.enabled {
        config.policy
    } else {
        PluginPolicy::Passthrough
    }
}

fn infer_client_api_code(client_protocol: Protocol, request_surface: ApiSurface) -> &'static str {
    match (client_protocol, request_surface) {
        (Protocol::Openai, ApiSurface::OpenAiResponses) => "codex",
        (Protocol::Anthropic, ApiSurface::AnthropicMessages) => "claude_code",
        (Protocol::Google, ApiSurface::GeminiGenerateContent) => "gemini_cli",
        _ => "openai_compatible",
    }
}

fn resolve_client_api_code(
    client_protocol: Protocol,
    request_surface: ApiSurface,
    headers: &HeaderMap,
) -> String {
    headers
        .get("x-sdkwork-client-api")
        .and_then(|value| value.to_str().ok())
        .map(sdkwork_lr_plugin::normalize_client_api_code)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| infer_client_api_code(client_protocol, request_surface).to_owned())
}

fn map_upstream_path(state: &AppState, context: &TransformContext) -> Result<String, String> {
    let Some(plugin) = state
        .transform_registry
        .resolve(context.source, context.target)
    else {
        return Ok(map_upstream_path_with_model_route_mapping(context));
    };
    plugin
        .map_upstream_path(context)
        .map(|path| rewrite_gemini_passthrough_model_path(path, context))
        .map_err(|error| error.to_string())
}

fn map_upstream_path_with_model_route_mapping(context: &TransformContext) -> String {
    rewrite_gemini_passthrough_model_path(
        sdkwork_lr_plugin::map_standard_upstream_path(context),
        context,
    )
}

fn rewrite_gemini_passthrough_model_path(path: String, context: &TransformContext) -> String {
    if context.target != ApiSurface::GeminiGenerateContent
        || context.upstream_model == context.model
        || context.source != ApiSurface::GeminiGenerateContent
    {
        return path;
    }

    let Some(requested_model) = context.model.as_deref() else {
        return path;
    };
    let Some(upstream_model) = context.upstream_model.as_deref() else {
        return path;
    };

    let encoded_requested = urlencoding::encode(requested_model).into_owned();
    if path.contains(requested_model) {
        return path.replacen(requested_model, upstream_model, 1);
    }
    if path.contains(&encoded_requested) {
        let encoded_upstream = urlencoding::encode(upstream_model);
        return path.replacen(&encoded_requested, encoded_upstream.as_ref(), 1);
    }

    path
}

fn append_query(path: String, query: Option<&str>) -> String {
    let Some(query) = query else {
        return path;
    };
    let sanitized_query = strip_local_router_query_credentials(query);
    if sanitized_query.is_empty() {
        return path;
    }
    if path.contains('?') {
        format!("{path}&{sanitized_query}")
    } else {
        format!("{path}?{sanitized_query}")
    }
}

fn strip_local_router_query_credentials(query: &str) -> String {
    query
        .split('&')
        .filter_map(parse_query_pair)
        .filter(|(name, _)| !name.eq_ignore_ascii_case("key"))
        .map(|(name, value)| {
            format!(
                "{}={}",
                urlencoding::encode(&name),
                urlencoding::encode(&value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn query_has_value(query: Option<&str>, key: &str, expected_value: &str) -> bool {
    let Some(query) = query else {
        return false;
    };
    query
        .split('&')
        .filter_map(parse_query_pair)
        .any(|(name, value)| {
            name.eq_ignore_ascii_case(key) && value.eq_ignore_ascii_case(expected_value)
        })
}

fn query_contains_case_insensitive(query: Option<&str>, needle: &str) -> bool {
    query
        .map(|value| value.to_ascii_lowercase().contains(needle))
        .unwrap_or(false)
}

fn parse_query_pair(pair: &str) -> Option<(String, String)> {
    if pair.is_empty() {
        return None;
    }
    let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
    let name = urlencoding::decode(name).ok()?.into_owned();
    let value = urlencoding::decode(value).ok()?.into_owned();
    Some((name, value))
}

fn transform_request(body: &[u8], context: &TransformContext) -> Result<serde_json::Value, String> {
    let parsed: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("invalid JSON body: {e}"))?;
    sdkwork_lr_transform::transform_request_body_with_context(&parsed, context)
}

fn transform_response(
    body: &[u8],
    context: &TransformContext,
    model: &str,
) -> Result<serde_json::Value, String> {
    let parsed: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("invalid JSON response: {e}"))?;
    let mut context = context.clone();
    if context.model.is_none() {
        context.model = Some(model.to_owned());
    }
    sdkwork_lr_transform::transform_response_body_with_context(&parsed, &context)
}

fn apply_model_route_mapping(body: &mut serde_json::Value, account: &Account) {
    if account.model_route_mappings.is_empty() {
        return;
    }
    if let Some(model) = body.get("model").and_then(|v| v.as_str()) {
        if let Some(upstream_model) = resolve_upstream_model(account, model) {
            if upstream_model != model {
                body["model"] = serde_json::Value::String(upstream_model.to_owned());
            }
        }
    }
}

fn ensure_target_model_field(body: &mut serde_json::Value, context: &TransformContext) {
    if !matches!(
        context.target,
        ApiSurface::OpenAiChatCompletions
            | ApiSurface::OpenAiResponses
            | ApiSurface::AnthropicMessages
    ) || body.get("model").is_some()
    {
        return;
    }

    if let Some(model) = context
        .upstream_model
        .as_deref()
        .or(context.model.as_deref())
        .filter(|model| !model.trim().is_empty())
    {
        body["model"] = serde_json::Value::String(model.to_owned());
    }
}

fn resolve_upstream_model<'a>(account: &'a Account, requested_model: &'a str) -> Option<&'a str> {
    account
        .model_route_mappings
        .get(requested_model)
        .map(String::as_str)
        .or(Some(requested_model))
}

fn should_fallback(status: axum::http::StatusCode) -> bool {
    status.is_server_error()
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::SERVICE_UNAVAILABLE
}

fn is_retryable_status(status: axum::http::StatusCode) -> bool {
    status.is_server_error()
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::SERVICE_UNAVAILABLE
        || status == StatusCode::GATEWAY_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use sdkwork_lr_core::{Interceptor, InterceptorChain, InterceptorResult};
    use sdkwork_lr_plugin::PluginPolicy;
    use sdkwork_models::{
        CatalogManifest, ModelCatalog, ModelInfo, ModelVendor, SourceEvidence, VendorCatalog,
    };
    use std::collections::BTreeMap;
    use std::time::Duration;

    #[test]
    fn strict_policy_rejects_request_transform_failures() {
        let body = br#"{"model":"gpt-5","messages":[]}"#;
        let context = context(
            ApiSurface::OpenAiResponses,
            ApiSurface::OpenAiChatCompletions,
        );

        let error = build_forward_body(body, &context, &account(), PluginPolicy::Strict)
            .expect_err("strict policy must fail before forwarding invalid transformed body");

        assert!(error.contains("request transform failed"));
        assert!(error.contains("missing input field"));
    }

    #[test]
    fn auto_policy_preserves_passthrough_on_request_transform_failures() {
        let body = br#"{"model":"gpt-5","messages":[]}"#;
        let context = context(
            ApiSurface::OpenAiResponses,
            ApiSurface::OpenAiChatCompletions,
        );

        let forwarded = build_forward_body(body, &context, &account(), PluginPolicy::Auto)
            .expect("auto policy should preserve existing warn-and-passthrough behavior");

        assert_eq!(forwarded, Bytes::copy_from_slice(body));
    }

    #[test]
    fn passthrough_policy_keeps_client_surface_even_when_catalog_requires_plugin() {
        let target = sdkwork_lr_plugin::select_target_surface_for_policy(
            PluginPolicy::Passthrough,
            ApiSurface::OpenAiResponses,
            ApiSurface::AnthropicMessages,
            Some(ApiSurface::AnthropicMessages),
        );

        assert_eq!(target, ApiSurface::OpenAiResponses);
    }

    #[test]
    fn force_transform_policy_targets_upstream_surface_instead_of_catalog_surface() {
        let target = sdkwork_lr_plugin::select_target_surface_for_policy(
            PluginPolicy::ForceTransform,
            ApiSurface::OpenAiResponses,
            ApiSurface::OpenAiChatCompletions,
            Some(ApiSurface::GeminiGenerateContent),
        );

        assert_eq!(target, ApiSurface::OpenAiChatCompletions);
    }

    #[test]
    fn strict_policy_uses_catalog_surface_like_auto() {
        let target = sdkwork_lr_plugin::select_target_surface_for_policy(
            PluginPolicy::Strict,
            ApiSurface::OpenAiResponses,
            ApiSurface::OpenAiChatCompletions,
            Some(ApiSurface::AnthropicMessages),
        );

        assert_eq!(target, ApiSurface::AnthropicMessages);
    }

    #[test]
    fn plugin_decision_headers_include_policy_surfaces_plugin_and_vendor_when_enabled() {
        let mut response = Response::new(axum::body::Body::empty());
        let plan = TransformPlan {
            context: context(ApiSurface::OpenAiResponses, ApiSurface::AnthropicMessages),
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: Some(compatibility_decision()),
            plugin_id: Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API".to_owned()),
        };

        add_plugin_decision_headers(&mut response, &plan, true);

        assert_eq!(response.headers()["x-sdkwork-plugin-policy"], "strict");
        assert_eq!(
            response.headers()["x-sdkwork-plugin-source"],
            "OPENAI_RESPONSES"
        );
        assert_eq!(
            response.headers()["x-sdkwork-plugin-target"],
            "ANTHROPIC_MESSAGES"
        );
        assert_eq!(
            response.headers()["x-sdkwork-plugin-id"],
            "OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API"
        );
        assert_eq!(response.headers()["x-sdkwork-model-vendor"], "anthropic");
        assert_eq!(response.headers()["x-sdkwork-client-api"], "codex");
    }

    #[test]
    fn plugin_decision_headers_are_suppressed_when_disabled() {
        let mut response = Response::new(axum::body::Body::empty());
        let plan = TransformPlan {
            context: context(ApiSurface::OpenAiResponses, ApiSurface::AnthropicMessages),
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: Some(compatibility_decision()),
            plugin_id: Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API".to_owned()),
        };

        add_plugin_decision_headers(&mut response, &plan, false);

        assert!(!response.headers().contains_key("x-sdkwork-plugin-policy"));
        assert!(!response.headers().contains_key("x-sdkwork-plugin-id"));
    }

    #[test]
    fn plugin_decision_headers_include_client_api_without_catalog_decision() {
        let mut response = Response::new(axum::body::Body::empty());
        let plan = TransformPlan {
            context: context(
                ApiSurface::OpenAiResponses,
                ApiSurface::OpenAiChatCompletions,
            ),
            policy: PluginPolicy::Auto,
            client_api_code: "claude_code".to_owned(),
            decision: None,
            plugin_id: Some("OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API".to_owned()),
        };

        add_plugin_decision_headers(&mut response, &plan, true);

        assert_eq!(response.headers()["x-sdkwork-client-api"], "claude_code");
        assert!(!response.headers().contains_key("x-sdkwork-model-vendor"));
    }

    #[test]
    fn plugin_preflight_allows_same_surface_without_plugin() {
        let registry = sdkwork_lr_plugin::PluginRegistry::new();
        let plan = TransformPlan {
            context: context(
                ApiSurface::OpenAiChatCompletions,
                ApiSurface::OpenAiChatCompletions,
            ),
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: None,
            plugin_id: None,
        };

        preflight_transform_plan(&registry, &plan).expect("same-surface plan must not need plugin");
    }

    #[test]
    fn plugin_preflight_rejects_missing_transform_plugin_before_forwarding() {
        let registry = sdkwork_lr_plugin::PluginRegistry::new();
        let plan = TransformPlan {
            context: context(ApiSurface::OpenAiResponses, ApiSurface::AnthropicMessages),
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: Some(compatibility_decision()),
            plugin_id: Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API".to_owned()),
        };

        let error = preflight_transform_plan(&registry, &plan)
            .expect_err("missing transform plugin must block before upstream forwarding");

        assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
        assert!(error.message.contains("missing transform plugin"));
        assert!(error
            .message
            .contains("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API"));
    }

    #[test]
    fn plugin_preflight_rejects_reserved_batch_plugin_before_forwarding() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let plan = TransformPlan {
            context: context(ApiSurface::OpenAiBatch, ApiSurface::OpenAiResponses),
            policy: PluginPolicy::Auto,
            client_api_code: "codex".to_owned(),
            decision: None,
            plugin_id: Some("OPENAI_BATCH_TO_OPENAI_RESPONSES_API".to_owned()),
        };

        let error = preflight_transform_plan(&registry, &plan)
            .expect_err("reserved batch plugin must block before upstream forwarding");

        assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
        assert!(error
            .message
            .contains("does not support request body transforms"));
        assert!(error
            .message
            .contains("OPENAI_BATCH_TO_OPENAI_RESPONSES_API"));
    }

    #[test]
    fn plugin_preflight_allows_openai_responses_chat_streaming_transforms() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let mut context = context(
            ApiSurface::OpenAiResponses,
            ApiSurface::OpenAiChatCompletions,
        );
        context.is_streaming = true;
        let plan = TransformPlan {
            context,
            policy: PluginPolicy::ForceTransform,
            client_api_code: "codex".to_owned(),
            decision: None,
            plugin_id: Some("OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API".to_owned()),
        };

        preflight_transform_plan(&registry, &plan)
            .expect("Codex Responses streaming to OpenAI Chat Completions must be supported");
    }

    #[test]
    fn plugin_preflight_allows_codex_responses_streaming_to_claude_messages() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let mut context = context(ApiSurface::OpenAiResponses, ApiSurface::AnthropicMessages);
        context.is_streaming = true;
        let plan = TransformPlan {
            context,
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: Some(compatibility_decision()),
            plugin_id: Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API".to_owned()),
        };

        preflight_transform_plan(&registry, &plan)
            .expect("Codex Responses streaming to Claude Messages must be supported");
    }

    #[test]
    fn plugin_preflight_allows_codex_responses_streaming_to_gemini_generate_content() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let mut context = context(
            ApiSurface::OpenAiResponses,
            ApiSurface::GeminiGenerateContent,
        );
        context.is_streaming = true;
        let plan = TransformPlan {
            context,
            policy: PluginPolicy::Strict,
            client_api_code: "codex".to_owned(),
            decision: None,
            plugin_id: Some("OPENAI_RESPONSES_TO_GEMINI_GENERATE_CONTENT_API".to_owned()),
        };

        preflight_transform_plan(&registry, &plan)
            .expect("Codex Responses streaming to Gemini GenerateContent must be supported");
    }

    #[test]
    fn plugin_preflight_allows_claude_messages_streaming_to_openai_responses() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let mut context = context(ApiSurface::AnthropicMessages, ApiSurface::OpenAiResponses);
        context.is_streaming = true;
        let plan = TransformPlan {
            context,
            policy: PluginPolicy::Strict,
            client_api_code: "claude_code".to_owned(),
            decision: None,
            plugin_id: Some("ANTHROPIC_MESSAGES_TO_OPENAI_RESPONSES_API".to_owned()),
        };

        preflight_transform_plan(&registry, &plan)
            .expect("Claude Code streaming to OpenAI Responses must be supported");
    }

    #[test]
    fn plugin_preflight_allows_claude_messages_streaming_to_gemini_generate_content() {
        let registry = sdkwork_lr_transform::plugins::built_in_plugin_registry();
        let mut context = context(
            ApiSurface::AnthropicMessages,
            ApiSurface::GeminiGenerateContent,
        );
        context.is_streaming = true;
        let plan = TransformPlan {
            context,
            policy: PluginPolicy::Strict,
            client_api_code: "claude_code".to_owned(),
            decision: None,
            plugin_id: Some("ANTHROPIC_MESSAGES_TO_GEMINI_GENERATE_CONTENT_API".to_owned()),
        };

        preflight_transform_plan(&registry, &plan)
            .expect("Claude Code streaming to Gemini GenerateContent must be supported");
    }

    #[test]
    fn explicit_client_api_header_overrides_protocol_inference() {
        let mut headers = HeaderMap::new();
        headers.insert("x-sdkwork-client-api", "claude-code".parse().unwrap());

        let client_api =
            resolve_client_api_code(Protocol::Openai, ApiSurface::OpenAiResponses, &headers);

        assert_eq!(client_api, "claude_code");
    }

    #[test]
    fn client_api_header_falls_back_to_route_inference_when_absent() {
        let headers = HeaderMap::new();

        let client_api =
            resolve_client_api_code(Protocol::Openai, ApiSurface::OpenAiResponses, &headers);

        assert_eq!(client_api, "codex");
    }

    #[tokio::test]
    async fn transform_plan_uses_vendor_client_api_support_to_skip_unneeded_plugin() {
        let catalog = catalog_with_vendor_model(
            vendor(
                "openai",
                "OpenAI",
                &[("codex", "supported", &["openai_responses"])],
                &["openai_responses", "openai_compatible"],
            ),
            model("gpt-compatible", "openai", "openai_compatible"),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Openai,
            "/v1/responses",
            "/v1",
            Some("gpt-compatible"),
            Some("gpt-compatible"),
            false,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::OpenAiResponses);
        assert_eq!(plan.client_api_code, "codex");
        assert_eq!(plan.plugin_id, None);
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| &decision.client_support_status),
            Some(&sdkwork_lr_plugin::ClientSupportStatus::Supported)
        );
    }

    #[tokio::test]
    async fn same_protocol_openai_responses_route_stays_responses_without_catalog_match() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Openai,
            "/v1/responses",
            "/v1",
            Some("gpt-5"),
            Some("gpt-5"),
            false,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::OpenAiResponses);
        assert_eq!(plan.plugin_id, None);
    }

    #[tokio::test]
    async fn same_protocol_openai_batch_route_stays_batch_without_reserved_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Openai,
            "/v1/batches",
            "/v1",
            None,
            None,
            false,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiBatch);
        assert_eq!(plan.context.target, ApiSurface::OpenAiBatch);
        assert_eq!(plan.plugin_id, None);
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("same-protocol Batch API passthrough must not use reserved execution plugin");
    }

    #[tokio::test]
    async fn codex_responses_route_to_openai_chat_upstream_supports_streaming_transform() {
        let catalog = catalog_with_vendor_model(
            vendor(
                "custom-openai-compatible",
                "Custom OpenAI Compatible",
                &[("codex", "unsupported", &[])],
                &["openai_compatible"],
            ),
            model(
                "custom-chat-model",
                "custom-openai-compatible",
                "openai_compatible",
            ),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Openai,
            "/v1/responses",
            "/v1",
            Some("custom-chat-model"),
            Some("custom-chat-model"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::OpenAiChatCompletions);
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Codex Responses streaming to OpenAI-compatible Chat upstream must work");
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("plugin path"),
            "/chat/completions"
        );
    }

    #[tokio::test]
    async fn codex_responses_route_to_anthropic_upstream_selects_responses_to_messages_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Anthropic,
            "/v1/responses",
            "/v1",
            Some("claude-sonnet-4-20250514"),
            Some("claude-sonnet-4-20250514"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.source_protocol, Protocol::Openai);
        assert_eq!(plan.context.target_protocol, Protocol::Anthropic);
        assert_eq!(plan.client_api_code, "codex");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Codex Responses streaming to Anthropic Messages must be preflight-valid");
    }

    #[tokio::test]
    async fn codex_responses_route_to_gemini_upstream_selects_responses_to_generate_content_plugin()
    {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Google,
            "/v1/responses",
            "/v1",
            Some("gemini-2.5-pro"),
            Some("gemini-2.5-pro"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.source_protocol, Protocol::Openai);
        assert_eq!(plan.context.target_protocol, Protocol::Google);
        assert_eq!(plan.client_api_code, "codex");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_RESPONSES_TO_GEMINI_GENERATE_CONTENT_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Codex Responses streaming to Gemini GenerateContent must be preflight-valid");
    }

    #[tokio::test]
    async fn codex_gpt_route_mapping_route_to_gemini_uses_upstream_model_for_decision_path_and_body(
    ) {
        let catalog = catalog_with_vendor_model(
            vendor(
                "google",
                "Google",
                &[("codex", "unsupported", &[])],
                &["google_gemini"],
            ),
            model("gemini-2.5-pro", "google", "google_gemini"),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();
        let mut account = account_with_provider("gemini-route_mapping", ProviderKind::Google);
        account.models = vec!["gemini-*".to_owned()];
        account
            .model_route_mappings
            .insert("gpt-5.5".to_owned(), "gemini-2.5-pro".to_owned());
        let upstream_model = resolve_upstream_model(&account, "gpt-5.5");

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Google,
            "/v1/responses",
            "/v1",
            Some("gpt-5.5"),
            upstream_model,
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiResponses);
        assert_eq!(plan.context.target, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.model.as_deref(), Some("gpt-5.5"));
        assert_eq!(
            plan.context.upstream_model.as_deref(),
            Some("gemini-2.5-pro")
        );
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| decision.vendor_code.as_str()),
            Some("google")
        );
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_RESPONSES_TO_GEMINI_GENERATE_CONTENT_API")
        );
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("plugin path"),
            "/v1/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );

        let forward_body = build_forward_body(
            br#"{"model":"gpt-5.5","input":"hello","stream":true}"#,
            &plan.context,
            &account,
            plan.policy,
        )
        .expect("forward body");
        let parsed: serde_json::Value = serde_json::from_slice(&forward_body).unwrap();
        assert!(
            parsed.get("model").is_none(),
            "Gemini model is encoded in the upstream path, not the GenerateContent body"
        );
    }

    #[test]
    fn forward_body_applies_configured_official_model_route_mapping_for_openai_upstream() {
        let mut account = account_with_provider("openai-official", ProviderKind::Openai);
        account
            .model_route_mappings
            .insert("gpt-5.5".to_owned(), "gpt-5.5-2026-05-01".to_owned());
        let context = TransformContext {
            source: ApiSurface::OpenAiChatCompletions,
            target: ApiSurface::OpenAiChatCompletions,
            source_protocol: Protocol::Openai,
            target_protocol: Protocol::Openai,
            client_path: "/v1/chat/completions".to_owned(),
            client_base_path: "/v1".to_owned(),
            upstream_model: Some("gpt-5.5-2026-05-01".to_owned()),
            model: Some("gpt-5.5".to_owned()),
            is_streaming: false,
        };

        let forward_body = build_forward_body(
            br#"{"model":"gpt-5.5","messages":[{"role":"user","content":"hi"}]}"#,
            &context,
            &account,
            PluginPolicy::Auto,
        )
        .expect("forward body");
        let parsed: serde_json::Value = serde_json::from_slice(&forward_body).unwrap();

        assert_eq!(parsed["model"], "gpt-5.5-2026-05-01");
    }

    #[test]
    fn forward_body_preserves_model_when_responses_request_converts_to_claude_messages() {
        let account = account_with_provider("claude", ProviderKind::Anthropic);
        let context = TransformContext {
            source: ApiSurface::OpenAiResponses,
            target: ApiSurface::AnthropicMessages,
            source_protocol: Protocol::Openai,
            target_protocol: Protocol::Anthropic,
            client_path: "/v1/responses".to_owned(),
            client_base_path: "/v1".to_owned(),
            upstream_model: Some("claude-sonnet-4-20250514".to_owned()),
            model: Some("claude-sonnet-4-20250514".to_owned()),
            is_streaming: false,
        };

        let forward_body = build_forward_body(
            br#"{"model":"claude-sonnet-4-20250514","input":"hello"}"#,
            &context,
            &account,
            PluginPolicy::Auto,
        )
        .expect("forward body");
        let parsed: serde_json::Value = serde_json::from_slice(&forward_body).unwrap();

        assert_eq!(parsed["model"], "claude-sonnet-4-20250514");
        assert!(parsed.get("messages").is_some());
    }

    #[test]
    fn forward_body_injects_upstream_model_when_gemini_cli_request_converts_to_openai_chat() {
        let mut account = account_with_provider("openai-route_mapping", ProviderKind::Openai);
        account.models = vec!["gpt-*".to_owned()];
        account
            .model_route_mappings
            .insert("gemini-2.5-pro".to_owned(), "gpt-5.5".to_owned());
        let context = TransformContext {
            source: ApiSurface::GeminiGenerateContent,
            target: ApiSurface::OpenAiChatCompletions,
            source_protocol: Protocol::Google,
            target_protocol: Protocol::Openai,
            client_path: "/google/v1/models/gemini-2.5-pro:generateContent".to_owned(),
            client_base_path: "/google".to_owned(),
            upstream_model: Some("gpt-5.5".to_owned()),
            model: Some("gemini-2.5-pro".to_owned()),
            is_streaming: false,
        };

        let forward_body = build_forward_body(
            br#"{"contents":[{"role":"user","parts":[{"text":"hello"}]}]}"#,
            &context,
            &account,
            PluginPolicy::Auto,
        )
        .expect("forward body");
        let parsed: serde_json::Value = serde_json::from_slice(&forward_body).unwrap();

        assert_eq!(parsed["model"], "gpt-5.5");
        assert!(parsed.get("messages").is_some());
    }

    #[tokio::test]
    async fn gemini_passthrough_path_rewrites_client_model_route_mapping_to_upstream_model() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();
        let mut account = account_with_provider("gemini-route_mapping", ProviderKind::Google);
        account.models = vec!["gemini-*".to_owned()];
        account
            .model_route_mappings
            .insert("gpt-5.5".to_owned(), "gemini-2.5-pro".to_owned());
        let upstream_model = resolve_upstream_model(&account, "gpt-5.5");

        let plan = build_transform_plan(
            &state,
            Protocol::Google,
            Protocol::Google,
            "/google/v1/models/gpt-5.5:generateContent",
            "/google",
            Some("gpt-5.5"),
            upstream_model,
            false,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.target, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.plugin_id, None);
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("upstream path"),
            "/v1/models/gemini-2.5-pro:generateContent"
        );
    }

    #[test]
    fn extract_model_decodes_gemini_path_model_segment() {
        assert_eq!(
            extract_model(
                b"{}",
                "/google/v1/models/publishers%2Fgoogle%2Fmodels%2Fgemini-2.5-pro:generateContent",
                Protocol::Google,
            )
            .as_deref(),
            Some("publishers/google/models/gemini-2.5-pro")
        );
    }

    #[tokio::test]
    async fn chat_completions_route_to_anthropic_upstream_selects_chat_to_messages_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Anthropic,
            "/v1/chat/completions",
            "/v1",
            Some("claude-sonnet-4-20250514"),
            Some("claude-sonnet-4-20250514"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiChatCompletions);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.source_protocol, Protocol::Openai);
        assert_eq!(plan.context.target_protocol, Protocol::Anthropic);
        assert_eq!(plan.client_api_code, "openai_compatible");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_CHAT_COMPLETIONS_TO_ANTHROPIC_MESSAGES_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Chat Completions streaming to Anthropic Messages must be preflight-valid");
    }

    #[tokio::test]
    async fn chat_completions_route_to_gemini_upstream_selects_chat_to_generate_content_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Openai,
            Protocol::Google,
            "/v1/chat/completions",
            "/v1",
            Some("gemini-2.5-pro"),
            Some("gemini-2.5-pro"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::OpenAiChatCompletions);
        assert_eq!(plan.context.target, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.source_protocol, Protocol::Openai);
        assert_eq!(plan.context.target_protocol, Protocol::Google);
        assert_eq!(plan.client_api_code, "openai_compatible");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("OPENAI_CHAT_COMPLETIONS_TO_GEMINI_GENERATE_CONTENT_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Chat Completions streaming to Gemini GenerateContent must be preflight-valid");
    }

    #[tokio::test]
    async fn claude_code_route_to_gemini_upstream_selects_messages_to_generate_content_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Google,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("gemini-2.5-pro"),
            Some("gemini-2.5-pro"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.source_protocol, Protocol::Anthropic);
        assert_eq!(plan.context.target_protocol, Protocol::Google);
        assert_eq!(plan.client_api_code, "claude_code");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("ANTHROPIC_MESSAGES_TO_GEMINI_GENERATE_CONTENT_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Claude Code streaming to Gemini GenerateContent must be preflight-valid");
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("plugin path"),
            "/v1/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );
    }

    #[tokio::test]
    async fn gemini_cli_route_to_openai_chat_upstream_selects_generate_content_to_chat_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Google,
            Protocol::Openai,
            "/google/v1/models/gemini-2.5-pro:streamGenerateContent",
            "/google",
            Some("gpt-4o"),
            Some("gpt-4o"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.target, ApiSurface::OpenAiChatCompletions);
        assert_eq!(plan.context.source_protocol, Protocol::Google);
        assert_eq!(plan.context.target_protocol, Protocol::Openai);
        assert_eq!(plan.client_api_code, "gemini_cli");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("GEMINI_GENERATE_CONTENT_TO_OPENAI_CHAT_COMPLETIONS_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Gemini CLI streaming to OpenAI Chat must be preflight-valid");
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("plugin path"),
            "/chat/completions"
        );
    }

    #[tokio::test]
    async fn gemini_cli_route_to_anthropic_upstream_selects_generate_content_to_messages_plugin() {
        let state = app_state_with_catalog(empty_catalog()).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Google,
            Protocol::Anthropic,
            "/google/v1/models/gemini-2.5-pro:streamGenerateContent",
            "/google",
            Some("claude-sonnet-4-20250514"),
            Some("claude-sonnet-4-20250514"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::GeminiGenerateContent);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.source_protocol, Protocol::Google);
        assert_eq!(plan.context.target_protocol, Protocol::Anthropic);
        assert_eq!(plan.client_api_code, "gemini_cli");
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("GEMINI_GENERATE_CONTENT_TO_ANTHROPIC_MESSAGES_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Gemini CLI streaming to Anthropic Messages must be preflight-valid");
        assert_eq!(
            map_upstream_path(&state, &plan.context).expect("plugin path"),
            "/v1/messages"
        );
    }

    #[tokio::test]
    async fn claude_code_route_to_openai_compatible_deepseek_model_selects_messages_to_chat_plugin()
    {
        let catalog = catalog_with_vendor_model(
            vendor(
                "deepseek",
                "DeepSeek",
                &[("claude_code", "unsupported", &[])],
                &["openai_compatible", "anthropic_messages"],
            ),
            model("deepseek-v4-pro", "deepseek", "openai_compatible"),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Openai,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("deepseek-v4-pro"),
            Some("deepseek-v4-pro"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::OpenAiChatCompletions);
        assert_eq!(plan.context.source_protocol, Protocol::Anthropic);
        assert_eq!(plan.context.target_protocol, Protocol::Openai);
        assert_eq!(plan.client_api_code, "claude_code");
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| decision.vendor_code.as_str()),
            Some("deepseek")
        );
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("ANTHROPIC_MESSAGES_TO_OPENAI_CHAT_COMPLETIONS_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Claude Code streaming to OpenAI-compatible DeepSeek must be transformed");
    }

    #[tokio::test]
    async fn claude_code_route_to_openai_compatible_chatglm_model_selects_messages_to_chat_plugin()
    {
        let catalog = catalog_with_vendor_model(
            vendor(
                "zhipu",
                "Zhipu AI",
                &[("claude_code", "unsupported", &[])],
                &["openai_compatible", "anthropic_messages"],
            ),
            model("glm-5.1", "zhipu", "openai_compatible"),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Openai,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("glm-5.1"),
            Some("glm-5.1"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::OpenAiChatCompletions);
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| decision.vendor_code.as_str()),
            Some("zhipu")
        );
        assert_eq!(
            plan.plugin_id.as_deref(),
            Some("ANTHROPIC_MESSAGES_TO_OPENAI_CHAT_COMPLETIONS_API")
        );
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("Claude Code streaming to OpenAI-compatible ChatGLM must be transformed");
    }

    #[tokio::test]
    async fn claude_code_route_to_direct_anthropic_deepseek_model_skips_plugin() {
        let catalog = sdkwork_models::load_bundled_catalog().expect("bundled catalog loads");
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Anthropic,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("deepseek-v4-pro"),
            Some("deepseek-v4-pro"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.client_api_code, "claude_code");
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| decision.vendor_code.as_str()),
            Some("deepseek")
        );
        assert_eq!(plan.plugin_id, None);
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("DeepSeek Anthropic-compatible upstream should passthrough Claude Code shape");
    }

    #[tokio::test]
    async fn claude_code_route_to_direct_anthropic_chatglm_model_skips_plugin() {
        let catalog = sdkwork_models::load_bundled_catalog().expect("bundled catalog loads");
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Anthropic,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("glm-5.1"),
            Some("glm-5.1"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.client_api_code, "claude_code");
        assert_eq!(
            plan.decision
                .as_ref()
                .map(|decision| decision.vendor_code.as_str()),
            Some("zhipu")
        );
        assert_eq!(plan.plugin_id, None);
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("ChatGLM Anthropic-compatible upstream should passthrough Claude Code shape");
    }

    #[tokio::test]
    async fn claude_code_route_to_native_anthropic_messages_model_skips_plugin() {
        let catalog = catalog_with_vendor_model(
            vendor(
                "anthropic",
                "Anthropic",
                &[("claude_code", "supported", &["anthropic_messages"])],
                &["anthropic_messages"],
            ),
            model(
                "claude-sonnet-4-20250514",
                "anthropic",
                "anthropic_messages",
            ),
        );
        let state = app_state_with_catalog(catalog).await;
        let headers = HeaderMap::new();

        let plan = build_transform_plan(
            &state,
            Protocol::Anthropic,
            Protocol::Anthropic,
            "/anthropic/v1/messages",
            "/anthropic",
            Some("claude-sonnet-4-20250514"),
            Some("claude-sonnet-4-20250514"),
            true,
            &headers,
        );

        assert_eq!(plan.context.source, ApiSurface::AnthropicMessages);
        assert_eq!(plan.context.target, ApiSurface::AnthropicMessages);
        assert_eq!(plan.plugin_id, None);
        preflight_transform_plan(&state.transform_registry, &plan)
            .expect("native Claude Code Messages surface should passthrough");
    }

    #[test]
    fn non_generation_openai_paths_only_use_openai_protocol_accounts() {
        let mut candidates = vec![
            Arc::new(account_with_provider("claude", ProviderKind::Anthropic)),
            Arc::new(account_with_provider("openai", ProviderKind::Openai)),
            Arc::new(account_with_provider(
                "openai-compatible",
                ProviderKind::Custom("openai-compatible".to_owned()),
            )),
            Arc::new(account_with_provider("gemini", ProviderKind::Google)),
        ];

        filter_non_generation_candidates(&mut candidates, Protocol::Openai, "/v1/files", "/v1");

        let names: Vec<&str> = candidates
            .iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(names, vec!["openai", "openai-compatible"]);
    }

    #[test]
    fn anthropic_accounts_get_default_messages_api_version_when_unset() {
        let account = account_with_provider("claude", ProviderKind::Anthropic);

        assert_eq!(
            effective_anthropic_version(&account).as_deref(),
            Some("2023-06-01")
        );
    }

    #[test]
    fn upstream_auth_scheme_can_use_bearer_for_anthropic_compatible_accounts() {
        let mut account = account_with_provider("deepseek-anthropic", ProviderKind::Anthropic);
        account.upstream_api_key = "sk-upstream".to_owned();
        account.upstream_auth_scheme = Some("bearer".to_owned());

        let auth = build_auth_for_provider(&account).expect("auth injection");

        match auth {
            AuthInjection::Bearer(token) => assert_eq!(token, "sk-upstream"),
            other => panic!("expected bearer auth, got {other:?}"),
        }
    }

    #[test]
    fn append_query_strips_local_router_client_key_before_forwarding() {
        assert_eq!(
            append_query(
                "/v1/models/gemini-pro:generateContent".to_owned(),
                Some("key=local-client-key&alt=sse")
            ),
            "/v1/models/gemini-pro:generateContent?alt=sse"
        );
        assert_eq!(
            append_query("/v1/models".to_owned(), Some("key=local-client-key")),
            "/v1/models"
        );
    }

    #[test]
    fn append_query_strips_encoded_and_case_variant_local_router_client_keys() {
        assert_eq!(
            append_query(
                "/v1/models/gemini-pro:streamGenerateContent?alt=sse".to_owned(),
                Some("Key=local-client-key&%6B%65%79=encoded-local-key&foo=a%20b&foo=c")
            ),
            "/v1/models/gemini-pro:streamGenerateContent?alt=sse&foo=a%20b&foo=c"
        );
    }

    #[test]
    fn gemini_streaming_query_detection_is_case_insensitive_and_decoded() {
        let body = br#"{"contents":[{"role":"user","parts":[{"text":"hi"}]}]}"#;

        assert!(detect_streaming(
            body,
            Protocol::Google,
            &HeaderMap::new(),
            Some("Alt=SSE")
        ));
        assert!(detect_streaming(
            body,
            Protocol::Google,
            &HeaderMap::new(),
            Some("method=streamGenerateContent")
        ));
    }

    #[tokio::test]
    async fn passthrough_interceptor_can_block_request_before_routing() {
        struct BlockingInterceptor;

        impl Interceptor for BlockingInterceptor {
            fn name(&self) -> &str {
                "blocking"
            }

            fn on_request(&self, _invocation: &mut Invocation) -> InterceptorResult {
                Err(sdkwork_lr_core::InterceptorError::forbidden(
                    "blocked by policy",
                ))
            }
        }

        let mut state = app_state_with_catalog(empty_catalog()).await;
        state.interceptor_chain = Arc::new(InterceptorChain::with_interceptors(vec![Arc::new(
            BlockingInterceptor,
        )]));

        let request = Request::builder()
            .method("POST")
            .uri("/v1/responses")
            .body(Body::from(r#"{"model":"gpt-5","input":"hello"}"#))
            .unwrap();

        let response = handle_provider_passthrough(state, request, Protocol::Openai).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    fn context(source: ApiSurface, target: ApiSurface) -> TransformContext {
        TransformContext {
            source,
            target,
            source_protocol: Protocol::Openai,
            target_protocol: Protocol::Openai,
            client_path: "/v1/responses".to_owned(),
            client_base_path: "/v1".to_owned(),
            upstream_model: None,
            model: Some("gpt-5".to_owned()),
            is_streaming: false,
        }
    }

    fn account() -> Account {
        Account {
            name: "test".to_owned(),
            provider: ProviderKind::Openai,
            base_url: "https://example.test/v1".to_owned(),
            upstream_api_key: String::new(),
            upstream_auth_scheme: None,
            models: vec![],
            priority: 10,
            timeout: Duration::from_secs(30),
            max_retries: 0,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            enabled: true,
            model_route_mappings: BTreeMap::new(),
        }
    }

    fn account_with_provider(name: &str, provider: ProviderKind) -> Account {
        Account {
            name: name.to_owned(),
            provider,
            base_url: "https://example.test".to_owned(),
            upstream_api_key: String::new(),
            upstream_auth_scheme: None,
            models: vec!["*".to_owned()],
            priority: 10,
            timeout: Duration::from_secs(30),
            max_retries: 0,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            enabled: true,
            model_route_mappings: BTreeMap::new(),
        }
    }

    fn compatibility_decision() -> sdkwork_lr_plugin::CompatibilityDecision {
        sdkwork_lr_plugin::CompatibilityDecision {
            model_id: "claude-sonnet-4-6".to_owned(),
            vendor_code: "anthropic".to_owned(),
            client_api_code: "codex".to_owned(),
            client_support_status: sdkwork_lr_plugin::ClientSupportStatus::Unsupported,
            client_surface: ApiSurface::OpenAiResponses,
            source: ApiSurface::OpenAiResponses,
            target: ApiSurface::AnthropicMessages,
            plugin_id: Some("OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API".to_owned()),
            reason: "test".to_owned(),
        }
    }

    async fn app_state_with_catalog(catalog: ModelCatalog) -> AppState {
        let mut toml_config = sdkwork_lr_config::RuntimeTomlConfig::default();
        toml_config.storage.database_url = Some("sqlite::memory:".to_owned());
        let config = sdkwork_lr_config::RuntimeConfig::from_toml(&toml_config).unwrap();
        let store = sdkwork_lr_store::Store::new("sqlite::memory:")
            .await
            .unwrap();
        store.run_migrations().await.unwrap();
        let health_manager = Arc::new(sdkwork_lr_core::HealthManager::new(Default::default()));
        let pool = Arc::new(parking_lot::RwLock::new(sdkwork_lr_core::AccountPool::new(
            Vec::new(),
        )));
        let mut user_pools = std::collections::HashMap::new();
        user_pools.insert(sdkwork_lr_store::DEFAULT_USER_ID, pool.clone());

        AppState {
            config: Arc::new(config),
            pool,
            user_pools: Arc::new(parking_lot::RwLock::new(user_pools)),
            routing_strategy_overrides: Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
            client: sdkwork_lr_proxy::build_proxy_client(),
            store,
            rate_limiter: Arc::new(sdkwork_routes_local_router_support::RateLimiter::new(
                60, 60,
            )),
            interceptor_chain: Arc::new(sdkwork_lr_core::InterceptorChain::new()),
            health_manager,
            transform_registry: Arc::new(sdkwork_lr_transform::plugins::built_in_plugin_registry()),
            model_catalog: Some(Arc::new(catalog)),
        }
    }

    fn catalog_with_vendor_model(vendor: ModelVendor, model: ModelInfo) -> ModelCatalog {
        ModelCatalog {
            manifest: catalog_manifest(),
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

    fn empty_catalog() -> ModelCatalog {
        ModelCatalog {
            manifest: catalog_manifest(),
            meters: vec![],
            protocols: vec![],
            vendors: vec![],
        }
    }

    fn catalog_manifest() -> CatalogManifest {
        CatalogManifest {
            name: "test".to_owned(),
            schema_version: "1.0.0".to_owned(),
            catalog_version: "test".to_owned(),
            generated_at: "2026-06-04T00:00:00Z".to_owned(),
            models_root: "models".to_owned(),
            schemas_root: "schemas".to_owned(),
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
