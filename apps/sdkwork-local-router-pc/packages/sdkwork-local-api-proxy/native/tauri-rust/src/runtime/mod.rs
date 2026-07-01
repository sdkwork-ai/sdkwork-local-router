use crate::{
    constants::{
        local_api_proxy_fallback_port_range_end, local_api_proxy_fallback_port_range_start,
    },
    error::{LocalApiProxyNativeError, LocalApiProxyNativeResult as Result},
    observability::LocalAiProxyLoggedMessage,
    snapshot::{
        LocalAiProxyRouteSnapshot, LocalAiProxySnapshot, LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
    },
};
use axum::Router;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};
use tokio::sync::oneshot;

const ANTHROPIC_CLIENT_PROTOCOL: &str = "anthropic";
const GEMINI_CLIENT_PROTOCOL: &str = "gemini";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalApiProxyRuntimeMode {
    DesktopLocal,
    ServerManaged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalAiProxyLifecycle {
    Running,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalAiProxyDefaultRouteHealth {
    pub client_protocol: String,
    pub id: String,
    pub name: String,
    pub managed_by: String,
    pub upstream_protocol: String,
    pub upstream_base_url: String,
    pub model_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalAiProxyServiceHealth {
    pub base_url: String,
    pub active_port: u16,
    pub loopback_only: bool,
    pub default_route_id: String,
    pub default_route_name: String,
    pub default_routes: Vec<LocalAiProxyDefaultRouteHealth>,
    pub upstream_base_url: String,
    pub model_count: usize,
    pub snapshot_path: String,
    pub log_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalAiProxyServiceStatus {
    pub lifecycle: LocalAiProxyLifecycle,
    pub health: Option<LocalAiProxyServiceHealth>,
    pub route_metrics: Vec<LocalAiProxyRouteRuntimeMetrics>,
    pub route_tests: Vec<LocalAiProxyRouteTestRecord>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyRouteRuntimeMetrics {
    pub route_id: String,
    pub client_protocol: String,
    pub upstream_protocol: String,
    pub health: String,
    pub request_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub rpm: u64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub average_latency_ms: u64,
    pub last_latency_ms: Option<u64>,
    pub last_used_at: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyRouteTestRecord {
    pub route_id: String,
    pub status: String,
    pub tested_at: u64,
    pub latency_ms: Option<u64>,
    pub checked_capability: String,
    pub model_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalApiProxyRuntimeDescriptor {
    pub mode: LocalApiProxyRuntimeMode,
    pub loopback_only: bool,
    pub authority_model: &'static str,
}

pub struct LocalApiProxyServerHandle {
    shutdown: Option<oneshot::Sender<()>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for LocalApiProxyServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalApiProxyServerHandle")
            .field("shutdown_pending", &self.shutdown.is_some())
            .field("join_handle_pending", &self.join_handle.is_some())
            .finish()
    }
}

#[derive(Debug)]
pub struct LocalApiProxyServerStartResult {
    pub active_port: u16,
    pub handle: LocalApiProxyServerHandle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyObservabilityStore {
    pub route_metrics: HashMap<String, LocalAiProxyRouteMetricsState>,
    pub route_tests: HashMap<String, LocalAiProxyRouteTestRecord>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyRouteMetricsState {
    pub client_protocol: String,
    pub upstream_protocol: String,
    pub request_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub cumulative_latency_ms: u64,
    pub last_latency_ms: Option<u64>,
    pub last_used_at: Option<u64>,
    pub last_error: Option<String>,
    pub recent_request_timestamps_ms: VecDeque<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyRequestAuditProjection {
    pub model_id: Option<String>,
    pub request_preview: Option<String>,
    pub request_body: Option<String>,
    pub messages: Vec<LocalAiProxyLoggedMessage>,
}

pub fn runtime_descriptor() -> LocalApiProxyRuntimeDescriptor {
    LocalApiProxyRuntimeDescriptor {
        mode: LocalApiProxyRuntimeMode::DesktopLocal,
        loopback_only: true,
        authority_model: "LocalApiProxyConfig",
    }
}

impl LocalApiProxyServerHandle {
    pub fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub fn start_local_api_proxy_server<G>(
    router: Router,
    bind_host: String,
    requested_port: u16,
    ready_timeout: Duration,
    on_serve_error: G,
) -> Result<LocalApiProxyServerStartResult>
where
    G: Fn(String) + Send + 'static,
{
    let (ready_tx, ready_rx) = mpsc::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let join_handle = thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = ready_tx.send(Err(format!("failed to build tokio runtime: {error}")));
                return;
            }
        };

        runtime.block_on(async move {
            let listener =
                match tokio::net::TcpListener::bind((bind_host.as_str(), requested_port)).await {
                    Ok(listener) => listener,
                    Err(error)
                        if requested_port > 0
                            && error.kind() == std::io::ErrorKind::AddrInUse =>
                    {
                        let mut resolved_listener = None;
                        let fallback_start =
                            local_api_proxy_fallback_port_range_start(requested_port);
                        let fallback_end =
                            local_api_proxy_fallback_port_range_end(requested_port);
                        for candidate_port in fallback_start..=fallback_end {
                            match tokio::net::TcpListener::bind((
                                bind_host.as_str(),
                                candidate_port,
                            ))
                            .await
                            {
                                Ok(listener) => {
                                    resolved_listener = Some(listener);
                                    break;
                                }
                                Err(candidate_error)
                                    if candidate_error.kind()
                                        == std::io::ErrorKind::AddrInUse => {}
                                Err(candidate_error) => {
                                    let _ = ready_tx.send(Err(format!(
                                        "failed to bind local ai proxy on fallback port {candidate_port}: {candidate_error}"
                                    )));
                                    return;
                                }
                            }
                        }

                        match resolved_listener {
                            Some(listener) => listener,
                            None => {
                                let _ = ready_tx.send(Err(format!(
                                    "failed to bind local ai proxy within the canonical fallback window {fallback_start}-{fallback_end}"
                                )));
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "failed to bind local ai proxy: {error}"
                        )));
                        return;
                    }
                };
            let active_port = match listener.local_addr() {
                Ok(address) => address.port(),
                Err(error) => {
                    let _ = ready_tx.send(Err(format!(
                        "failed to resolve local ai proxy address: {error}"
                    )));
                    return;
                }
            };
            if ready_tx.send(Ok(active_port)).is_err() {
                return;
            }

            if let Err(error) = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
            {
                on_serve_error(error.to_string());
            }
        });
    });

    let active_port = ready_rx
        .recv_timeout(ready_timeout)
        .map_err(|_| {
            LocalApiProxyNativeError::Timeout(
                "timed out waiting for the local ai proxy to bind a loopback port".to_string(),
            )
        })?
        .map_err(LocalApiProxyNativeError::Internal)?;

    Ok(LocalApiProxyServerStartResult {
        active_port,
        handle: LocalApiProxyServerHandle {
            shutdown: Some(shutdown_tx),
            join_handle: Some(join_handle),
        },
    })
}

pub fn lock_observability(
    observability: &Arc<Mutex<LocalAiProxyObservabilityStore>>,
) -> Result<MutexGuard<'_, LocalAiProxyObservabilityStore>> {
    observability.lock().map_err(|_| {
        LocalApiProxyNativeError::Internal("local ai proxy observability lock poisoned".to_string())
    })
}

pub fn reconcile_observability_store(
    store: &mut LocalAiProxyObservabilityStore,
    snapshot: &LocalAiProxySnapshot,
) {
    let route_ids = snapshot
        .routes
        .iter()
        .map(|route| route.id.clone())
        .collect::<HashSet<_>>();
    store
        .route_metrics
        .retain(|route_id, _| route_ids.contains(route_id));
    store
        .route_tests
        .retain(|route_id, _| route_ids.contains(route_id));

    for route in &snapshot.routes {
        let entry = store
            .route_metrics
            .entry(route.id.clone())
            .or_insert_with(|| LocalAiProxyRouteMetricsState {
                client_protocol: route.client_protocol.clone(),
                upstream_protocol: route.upstream_protocol.clone(),
                ..Default::default()
            });
        entry.client_protocol = route.client_protocol.clone();
        entry.upstream_protocol = route.upstream_protocol.clone();
    }
}

pub fn build_route_metrics(
    snapshot: &LocalAiProxySnapshot,
    store: &LocalAiProxyObservabilityStore,
) -> Vec<LocalAiProxyRouteRuntimeMetrics> {
    let mut metrics = snapshot
        .routes
        .iter()
        .map(|route| {
            let route_state = store.route_metrics.get(&route.id);
            let latest_test = store.route_tests.get(&route.id);
            LocalAiProxyRouteRuntimeMetrics {
                route_id: route.id.clone(),
                client_protocol: route.client_protocol.clone(),
                upstream_protocol: route.upstream_protocol.clone(),
                health: derive_route_health(route, route_state, latest_test).to_string(),
                request_count: route_state.map(|value| value.request_count).unwrap_or(0),
                success_count: route_state.map(|value| value.success_count).unwrap_or(0),
                failure_count: route_state.map(|value| value.failure_count).unwrap_or(0),
                rpm: route_state
                    .map(|value| value.recent_request_timestamps_ms.len() as u64)
                    .unwrap_or(0),
                total_tokens: route_state.map(|value| value.total_tokens).unwrap_or(0),
                input_tokens: route_state.map(|value| value.input_tokens).unwrap_or(0),
                output_tokens: route_state.map(|value| value.output_tokens).unwrap_or(0),
                cache_tokens: route_state.map(|value| value.cache_tokens).unwrap_or(0),
                average_latency_ms: route_state
                    .filter(|value| value.request_count > 0)
                    .map(|value| value.cumulative_latency_ms / value.request_count)
                    .unwrap_or(0),
                last_latency_ms: route_state.and_then(|value| value.last_latency_ms),
                last_used_at: route_state.and_then(|value| value.last_used_at),
                last_error: route_state.and_then(|value| value.last_error.clone()),
            }
        })
        .collect::<Vec<_>>();
    metrics.sort_by(|left, right| left.route_id.cmp(&right.route_id));
    metrics
}

pub fn collect_route_tests(
    snapshot: &LocalAiProxySnapshot,
    store: &LocalAiProxyObservabilityStore,
) -> Vec<LocalAiProxyRouteTestRecord> {
    let valid_route_ids = snapshot
        .routes
        .iter()
        .map(|route| route.id.as_str())
        .collect::<HashSet<_>>();
    let mut tests = store
        .route_tests
        .values()
        .filter(|record| valid_route_ids.contains(record.route_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    tests.sort_by(|left, right| left.route_id.cmp(&right.route_id));
    tests
}

pub fn collect_default_route_health(
    snapshot: &LocalAiProxySnapshot,
) -> Vec<LocalAiProxyDefaultRouteHealth> {
    [
        LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
        ANTHROPIC_CLIENT_PROTOCOL,
        GEMINI_CLIENT_PROTOCOL,
    ]
    .iter()
    .filter_map(|client_protocol| {
        snapshot
            .route_for_client_protocol(client_protocol)
            .map(|route| LocalAiProxyDefaultRouteHealth {
                client_protocol: route.client_protocol.clone(),
                id: route.id.clone(),
                name: route.name.clone(),
                managed_by: route.managed_by.clone(),
                upstream_protocol: route.upstream_protocol.clone(),
                upstream_base_url: route.upstream_base_url.clone(),
                model_count: route.models.len(),
            })
    })
    .collect()
}

pub fn build_request_audit_projection(
    route: &LocalAiProxyRouteSnapshot,
    endpoint: &str,
    body: &[u8],
) -> LocalAiProxyRequestAuditProjection {
    let payload = serde_json::from_slice::<Value>(body).ok();
    let messages = payload
        .as_ref()
        .map(|value| extract_logged_messages(route, endpoint, value))
        .unwrap_or_default();
    let request_body = payload
        .as_ref()
        .and_then(|value| serde_json::to_string_pretty(value).ok())
        .or_else(|| trim_optional_text(&String::from_utf8_lossy(body)));

    LocalAiProxyRequestAuditProjection {
        model_id: payload
            .as_ref()
            .and_then(|value| extract_logged_model_id(route, endpoint, value)),
        request_preview: resolve_request_preview(&messages)
            .or_else(|| trim_optional_text(&String::from_utf8_lossy(body))),
        request_body,
        messages,
    }
}

pub fn extract_response_preview_from_value(value: &Value) -> Option<String> {
    for pointer in [
        "/choices/0/message/content",
        "/choices/0/delta/content",
        "/output_text",
        "/output/0/content/0/text",
        "/content/0/text",
        "/candidates/0/content/parts",
        "/message/content/0/text",
    ] {
        if let Some(candidate) = value.pointer(pointer) {
            let preview = extract_text_from_value(candidate);
            if let Some(preview) = trim_optional_text(&preview) {
                return Some(preview);
            }
        }
    }

    None
}

fn derive_route_health(
    route: &LocalAiProxyRouteSnapshot,
    route_state: Option<&LocalAiProxyRouteMetricsState>,
    latest_test: Option<&LocalAiProxyRouteTestRecord>,
) -> &'static str {
    if !route.enabled {
        return "disabled";
    }
    if latest_test
        .map(|value| value.status.as_str() == "failed")
        .unwrap_or(false)
    {
        return "failed";
    }
    if let Some(value) = route_state {
        if value.failure_count > 0 && value.success_count == 0 {
            return "failed";
        }
        if value.failure_count > 0 {
            return "degraded";
        }
        if value.success_count > 0 {
            return "healthy";
        }
    }
    if latest_test
        .map(|value| value.status.as_str() == "passed")
        .unwrap_or(false)
    {
        return "healthy";
    }

    "degraded"
}

fn extract_logged_model_id(
    route: &LocalAiProxyRouteSnapshot,
    endpoint: &str,
    payload: &Value,
) -> Option<String> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| parse_model_id_from_endpoint(endpoint))
        .or_else(|| {
            (!route.default_model_id.trim().is_empty()).then(|| route.default_model_id.clone())
        })
}

fn parse_model_id_from_endpoint(endpoint: &str) -> Option<String> {
    endpoint
        .split("models/")
        .nth(1)
        .and_then(|value| value.split(':').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn extract_logged_messages(
    route: &LocalAiProxyRouteSnapshot,
    endpoint: &str,
    payload: &Value,
) -> Vec<LocalAiProxyLoggedMessage> {
    match route.client_protocol.as_str() {
        ANTHROPIC_CLIENT_PROTOCOL => collect_anthropic_logged_messages(payload),
        GEMINI_CLIENT_PROTOCOL => collect_gemini_logged_messages(payload),
        _ if endpoint.contains("responses") => collect_openai_response_logged_messages(payload),
        _ => collect_openai_logged_messages(payload),
    }
}

fn collect_openai_logged_messages(payload: &Value) -> Vec<LocalAiProxyLoggedMessage> {
    let mut messages = Vec::new();
    if let Some(system) = payload.get("instructions").and_then(Value::as_str) {
        push_logged_message(&mut messages, "system", system, None, Some("instructions"));
    }
    if let Some(array) = payload.get("messages").and_then(Value::as_array) {
        for entry in array {
            push_logged_message(
                &mut messages,
                entry.get("role").and_then(Value::as_str).unwrap_or("user"),
                &extract_text_from_value(entry.get("content").unwrap_or(entry)),
                entry.get("name").and_then(Value::as_str),
                Some("message"),
            );
        }
    } else if let Some(input) = payload.get("input") {
        push_logged_message(
            &mut messages,
            "user",
            &extract_text_from_value(input),
            None,
            Some("input"),
        );
    }
    messages
}

fn collect_openai_response_logged_messages(payload: &Value) -> Vec<LocalAiProxyLoggedMessage> {
    let mut messages = Vec::new();
    if let Some(instructions) = payload.get("instructions").and_then(Value::as_str) {
        push_logged_message(
            &mut messages,
            "system",
            instructions,
            None,
            Some("instructions"),
        );
    }
    match payload.get("input") {
        Some(Value::Array(entries)) => {
            for entry in entries {
                push_logged_message(
                    &mut messages,
                    entry.get("role").and_then(Value::as_str).unwrap_or("user"),
                    &extract_text_from_value(entry),
                    None,
                    Some("input"),
                );
            }
        }
        Some(value) => {
            push_logged_message(
                &mut messages,
                "user",
                &extract_text_from_value(value),
                None,
                Some("input"),
            );
        }
        None => {}
    }
    messages
}

fn collect_anthropic_logged_messages(payload: &Value) -> Vec<LocalAiProxyLoggedMessage> {
    let mut messages = Vec::new();
    if let Some(system) = payload.get("system") {
        push_logged_message(
            &mut messages,
            "system",
            &extract_text_from_value(system),
            None,
            Some("system"),
        );
    }
    if let Some(array) = payload.get("messages").and_then(Value::as_array) {
        for entry in array {
            push_logged_message(
                &mut messages,
                entry.get("role").and_then(Value::as_str).unwrap_or("user"),
                &extract_text_from_value(entry.get("content").unwrap_or(entry)),
                None,
                Some("message"),
            );
        }
    }
    messages
}

fn collect_gemini_logged_messages(payload: &Value) -> Vec<LocalAiProxyLoggedMessage> {
    let mut messages = Vec::new();
    if let Some(system) = payload.get("systemInstruction") {
        push_logged_message(
            &mut messages,
            "system",
            &extract_text_from_value(system),
            None,
            Some("systemInstruction"),
        );
    }
    if let Some(array) = payload.get("contents").and_then(Value::as_array) {
        for entry in array {
            push_logged_message(
                &mut messages,
                entry.get("role").and_then(Value::as_str).unwrap_or("user"),
                &extract_text_from_value(entry.get("parts").unwrap_or(entry)),
                None,
                Some("content"),
            );
        }
    } else if let Some(content) = payload.get("content") {
        push_logged_message(
            &mut messages,
            "user",
            &extract_text_from_value(content),
            None,
            Some("content"),
        );
    }
    messages
}

fn push_logged_message(
    messages: &mut Vec<LocalAiProxyLoggedMessage>,
    role: &str,
    content: &str,
    name: Option<&str>,
    kind: Option<&str>,
) {
    let Some(content) = trim_optional_text(content) else {
        return;
    };
    messages.push(LocalAiProxyLoggedMessage {
        index: messages.len() as u32,
        role: role.trim().to_string(),
        content,
        name: name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        kind: kind
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    });
}

fn extract_text_from_value(value: &Value) -> String {
    let mut parts = Vec::new();
    collect_text_fragments(value, &mut parts);
    parts.join("\n").trim().to_string()
}

fn collect_text_fragments(value: &Value, parts: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                parts.push(text.trim().to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text_fragments(item, parts);
            }
        }
        Value::Object(object) => {
            for key in ["text", "content", "parts", "input_text", "output_text"] {
                if let Some(candidate) = object.get(key) {
                    collect_text_fragments(candidate, parts);
                    return;
                }
            }
        }
        _ => {}
    }
}

fn resolve_request_preview(messages: &[LocalAiProxyLoggedMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .or_else(|| messages.last())
        .and_then(|message| trim_optional_text(&message.content))
}

fn trim_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        build_request_audit_projection, build_route_metrics, collect_default_route_health,
        collect_route_tests, extract_response_preview_from_value, reconcile_observability_store,
        LocalAiProxyObservabilityStore, LocalAiProxyRouteMetricsState, LocalAiProxyRouteTestRecord,
    };
    use crate::snapshot::{
        LocalAiProxyModelSnapshot, LocalAiProxyRouteRuntimeConfigSnapshot,
        LocalAiProxyRouteSnapshot, LocalAiProxySnapshot, LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY,
        LOCAL_AI_PROXY_SCHEMA_VERSION,
    };
    use serde_json::json;
    use std::collections::VecDeque;

    #[test]
    fn observability_projection_reconciles_routes_and_filters_unknown_records() {
        let snapshot = LocalAiProxySnapshot {
            schema_version: LOCAL_AI_PROXY_SCHEMA_VERSION,
            bind_host: "127.0.0.1".to_string(),
            requested_port: 21_280,
            auth_token: LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY.to_string(),
            default_route_id: "openai-route".to_string(),
            routes: vec![
                create_route("anthropic-route", "anthropic", "anthropic", true, true),
                create_route("disabled-route", "gemini", "gemini", false, true),
                create_route(
                    "openai-route",
                    "openai-compatible",
                    "openai-compatible",
                    true,
                    true,
                ),
            ],
        };
        let mut store = LocalAiProxyObservabilityStore::default();
        store.route_metrics.insert(
            "obsolete-route".to_string(),
            LocalAiProxyRouteMetricsState {
                client_protocol: "openai-compatible".to_string(),
                upstream_protocol: "openai-compatible".to_string(),
                request_count: 99,
                ..Default::default()
            },
        );
        store.route_tests.insert(
            "obsolete-route".to_string(),
            LocalAiProxyRouteTestRecord {
                route_id: "obsolete-route".to_string(),
                status: "failed".to_string(),
                tested_at: 1,
                checked_capability: "chat".to_string(),
                ..Default::default()
            },
        );

        reconcile_observability_store(&mut store, &snapshot);
        assert_eq!(store.route_metrics.len(), 3);
        assert_eq!(store.route_tests.len(), 0);
        assert!(store.route_metrics.contains_key("anthropic-route"));
        assert!(store.route_metrics.contains_key("disabled-route"));
        assert!(store.route_metrics.contains_key("openai-route"));
        assert!(!store.route_metrics.contains_key("obsolete-route"));

        store.route_metrics.insert(
            "anthropic-route".to_string(),
            LocalAiProxyRouteMetricsState {
                client_protocol: "anthropic".to_string(),
                upstream_protocol: "anthropic".to_string(),
                request_count: 2,
                success_count: 2,
                cumulative_latency_ms: 160,
                last_latency_ms: Some(70),
                last_used_at: Some(2_000),
                recent_request_timestamps_ms: VecDeque::from([1_800, 1_900]),
                ..Default::default()
            },
        );
        store.route_metrics.insert(
            "openai-route".to_string(),
            LocalAiProxyRouteMetricsState {
                client_protocol: "openai-compatible".to_string(),
                upstream_protocol: "openai-compatible".to_string(),
                request_count: 3,
                failure_count: 1,
                total_tokens: 90,
                input_tokens: 60,
                output_tokens: 30,
                cache_tokens: 10,
                cumulative_latency_ms: 300,
                last_latency_ms: Some(120),
                last_used_at: Some(3_000),
                last_error: Some("upstream timeout".to_string()),
                recent_request_timestamps_ms: VecDeque::from([2_700, 2_800, 2_900]),
                ..Default::default()
            },
        );
        store.route_tests.insert(
            "openai-route".to_string(),
            LocalAiProxyRouteTestRecord {
                route_id: "openai-route".to_string(),
                status: "failed".to_string(),
                tested_at: 3_100,
                checked_capability: "chat".to_string(),
                error: Some("upstream timeout".to_string()),
                ..Default::default()
            },
        );
        store.route_tests.insert(
            "obsolete-route".to_string(),
            LocalAiProxyRouteTestRecord {
                route_id: "obsolete-route".to_string(),
                status: "passed".to_string(),
                tested_at: 3_200,
                checked_capability: "chat".to_string(),
                ..Default::default()
            },
        );

        let metrics = build_route_metrics(&snapshot, &store);
        assert_eq!(
            metrics
                .iter()
                .map(|record| record.route_id.as_str())
                .collect::<Vec<_>>(),
            vec!["anthropic-route", "disabled-route", "openai-route"]
        );
        assert_eq!(metrics[0].health, "healthy");
        assert_eq!(metrics[0].average_latency_ms, 80);
        assert_eq!(metrics[0].rpm, 2);
        assert_eq!(metrics[1].health, "disabled");
        assert_eq!(metrics[2].health, "failed");
        assert_eq!(metrics[2].request_count, 3);
        assert_eq!(metrics[2].failure_count, 1);
        assert_eq!(metrics[2].total_tokens, 90);
        assert_eq!(metrics[2].average_latency_ms, 100);
        assert_eq!(metrics[2].last_error.as_deref(), Some("upstream timeout"));

        let route_tests = collect_route_tests(&snapshot, &store);
        assert_eq!(route_tests.len(), 1);
        assert_eq!(route_tests[0].route_id, "openai-route");
        assert_eq!(route_tests[0].status, "failed");
    }

    #[test]
    fn default_route_health_projects_supported_client_protocols() {
        let snapshot = LocalAiProxySnapshot {
            schema_version: LOCAL_AI_PROXY_SCHEMA_VERSION,
            bind_host: "127.0.0.1".to_string(),
            requested_port: 21_280,
            auth_token: LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY.to_string(),
            default_route_id: "openai-route".to_string(),
            routes: vec![
                create_route(
                    "openai-secondary",
                    "openai-compatible",
                    "openai-compatible",
                    true,
                    false,
                ),
                create_route("openai-route", "openai-compatible", "sdkwork", true, true),
                create_route("anthropic-route", "anthropic", "anthropic", true, true),
                create_route("gemini-disabled", "gemini", "gemini", false, true),
                create_route("gemini-route", "gemini", "gemini", true, false),
            ],
        };

        let default_routes = collect_default_route_health(&snapshot);

        assert_eq!(
            default_routes
                .iter()
                .map(|route| route.client_protocol.as_str())
                .collect::<Vec<_>>(),
            vec!["openai-compatible", "anthropic", "gemini"]
        );
        assert_eq!(default_routes[0].id, "openai-route");
        assert_eq!(default_routes[1].id, "anthropic-route");
        assert_eq!(default_routes[2].id, "gemini-route");
    }

    #[test]
    fn request_audit_projection_collects_openai_responses_messages_and_preview() {
        let route = create_route(
            "openai-route",
            "openai-compatible",
            "openai-compatible",
            true,
            true,
        );
        let projection = build_request_audit_projection(
            &route,
            "/v1/responses",
            br#"{
              "instructions": "Follow the product style guide.",
              "input": [
                {
                  "role": "user",
                  "content": [
                    { "type": "input_text", "text": "Summarize the latest route status." }
                  ]
                }
              ]
            }"#,
        );

        assert_eq!(projection.model_id.as_deref(), Some("model-default"));
        assert_eq!(
            projection.request_preview.as_deref(),
            Some("Summarize the latest route status.")
        );
        assert_eq!(projection.messages.len(), 2);
        assert_eq!(projection.messages[0].role, "system");
        assert_eq!(
            projection.messages[0].content,
            "Follow the product style guide."
        );
        assert_eq!(projection.messages[1].role, "user");
        assert_eq!(
            projection.messages[1].content,
            "Summarize the latest route status."
        );
        assert!(projection
            .request_body
            .as_deref()
            .is_some_and(|value| value.contains("\"instructions\"")));
    }

    #[test]
    fn request_audit_projection_collects_gemini_messages_and_response_preview() {
        let route = create_route("gemini-route", "gemini", "gemini", true, true);
        let projection = build_request_audit_projection(
            &route,
            "/v1beta/models/gemini-2.5-pro:generateContent",
            br#"{
              "systemInstruction": { "parts": [{ "text": "Use concise release notes." }] },
              "contents": [
                {
                  "role": "user",
                  "parts": [{ "text": "Generate a changelog entry." }]
                }
              ]
            }"#,
        );

        assert_eq!(projection.model_id.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(
            projection.request_preview.as_deref(),
            Some("Generate a changelog entry.")
        );
        assert_eq!(projection.messages.len(), 2);
        assert_eq!(
            projection.messages[0].kind.as_deref(),
            Some("systemInstruction")
        );
        assert_eq!(projection.messages[1].kind.as_deref(), Some("content"));

        let preview = extract_response_preview_from_value(&json!({
          "candidates": [
            {
              "content": {
                "parts": [
                  { "text": "Gemini preview text" }
                ]
              }
            }
          ]
        }));
        assert_eq!(preview.as_deref(), Some("Gemini preview text"));
    }

    fn create_route(
        id: &str,
        client_protocol: &str,
        upstream_protocol: &str,
        enabled: bool,
        is_default: bool,
    ) -> LocalAiProxyRouteSnapshot {
        LocalAiProxyRouteSnapshot {
            id: id.to_string(),
            name: id.to_string(),
            enabled,
            is_default,
            managed_by: "user".to_string(),
            client_protocol: client_protocol.to_string(),
            upstream_protocol: upstream_protocol.to_string(),
            provider_id: format!("{client_protocol}-provider"),
            upstream_base_url: format!("https://{}.example.com", client_protocol),
            api_key: "test-key".to_string(),
            default_model_id: "model-default".to_string(),
            reasoning_model_id: None,
            embedding_model_id: None,
            models: vec![LocalAiProxyModelSnapshot {
                id: "model-default".to_string(),
                name: "Model Default".to_string(),
            }],
            notes: None,
            expose_to: vec!["sdkwork".to_string()],
            runtime_config: LocalAiProxyRouteRuntimeConfigSnapshot::default(),
        }
    }
}
