use crate::{
    error::{LocalApiProxyNativeError, LocalApiProxyNativeResult as Result},
    response::{extract_http_error_message, resolve_error_message},
    runtime::LocalAiProxyRouteTestRecord,
    snapshot::LocalAiProxyRouteSnapshot,
    support::current_time_ms,
    upstream,
};
use axum::{
    body::Bytes,
    http::{header::CONTENT_TYPE, HeaderValue},
};
use serde_json::{json, Value};
use std::{result::Result as StdResult, time::Instant};

const OLLAMA_UPSTREAM_PROTOCOL: &str = "ollama";
const ANTHROPIC_VERSION_HEADER: &str = "anthropic-version";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const X_API_KEY_HEADER: &str = "x-api-key";
const X_GOOG_API_KEY_HEADER: &str = "x-goog-api-key";

pub fn probe_route(route: &LocalAiProxyRouteSnapshot) -> Result<LocalAiProxyRouteTestRecord> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            LocalApiProxyNativeError::Internal(format!(
                "failed to create tokio runtime for local ai proxy route probe: {error}"
            ))
        })?;

    Ok(runtime.block_on(async { probe_route_async(route).await }))
}

async fn probe_route_async(route: &LocalAiProxyRouteSnapshot) -> LocalAiProxyRouteTestRecord {
    let tested_at = current_time_ms();
    let started_at = Instant::now();
    let capability = route_probe_capability(route).to_string();
    let model_id =
        (!route.default_model_id.trim().is_empty()).then(|| route.default_model_id.clone());

    let outcome = if !route.enabled {
        Err("route is disabled".to_string())
    } else if model_id.is_none() {
        Err("route is missing a default model id".to_string())
    } else {
        let client = reqwest::Client::new();
        match route.upstream_protocol.as_str() {
            "anthropic" => probe_anthropic_route(&client, route).await,
            "gemini" => probe_gemini_route(&client, route).await,
            OLLAMA_UPSTREAM_PROTOCOL => probe_ollama_route(&client, route).await,
            _ => probe_openai_compatible_route(&client, route).await,
        }
    };

    LocalAiProxyRouteTestRecord {
        route_id: route.id.clone(),
        status: if outcome.is_ok() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        tested_at,
        latency_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        checked_capability: capability,
        model_id,
        error: outcome.err(),
    }
}

fn route_probe_capability(route: &LocalAiProxyRouteSnapshot) -> &'static str {
    match route.upstream_protocol.as_str() {
        "anthropic" => "messages",
        "gemini" => "generateContent",
        _ => "chat",
    }
}

async fn probe_openai_compatible_route(
    client: &reqwest::Client,
    route: &LocalAiProxyRouteSnapshot,
) -> StdResult<(), String> {
    let body = Bytes::from(
        json!({
            "model": route.default_model_id,
            "messages": [{ "role": "user", "content": "ping" }],
            "max_tokens": 1,
            "temperature": 0,
        })
        .to_string(),
    );
    let request = upstream::build_openai_compatible_upstream_request(
        client,
        route,
        "chat/completions",
        None,
        body,
    )
    .map_err(|error| extract_http_error_message(&error))?;
    let response = request
        .send()
        .await
        .map_err(|error| format!("route probe upstream request failed: {error}"))?;
    ensure_probe_response_success(response).await
}

async fn probe_anthropic_route(
    client: &reqwest::Client,
    route: &LocalAiProxyRouteSnapshot,
) -> StdResult<(), String> {
    let response = client
        .post(format!(
            "{}/messages",
            route.upstream_base_url.trim_end_matches('/')
        ))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(X_API_KEY_HEADER, route.api_key.trim())
        .header(ANTHROPIC_VERSION_HEADER, DEFAULT_ANTHROPIC_VERSION)
        .body(
            json!({
                "model": route.default_model_id,
                "max_tokens": 1,
                "messages": [{ "role": "user", "content": "ping" }],
            })
            .to_string(),
        )
        .send()
        .await
        .map_err(|error| format!("route probe upstream request failed: {error}"))?;
    ensure_probe_response_success(response).await
}

async fn probe_gemini_route(
    client: &reqwest::Client,
    route: &LocalAiProxyRouteSnapshot,
) -> StdResult<(), String> {
    let api_version = upstream::infer_gemini_default_api_version(&route.upstream_base_url);
    let response = client
        .post(upstream::build_gemini_upstream_request_url(
            route,
            api_version,
            &format!("{}:generateContent", route.default_model_id),
            None,
        ))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(X_GOOG_API_KEY_HEADER, route.api_key.trim())
        .body(
            json!({
                "contents": [{
                    "role": "user",
                    "parts": [{ "text": "ping" }],
                }],
                "generationConfig": {
                    "maxOutputTokens": 1,
                }
            })
            .to_string(),
        )
        .send()
        .await
        .map_err(|error| format!("route probe upstream request failed: {error}"))?;
    ensure_probe_response_success(response).await
}

async fn probe_ollama_route(
    client: &reqwest::Client,
    route: &LocalAiProxyRouteSnapshot,
) -> StdResult<(), String> {
    let response = client
        .post(upstream::build_ollama_upstream_request_url(
            route,
            "/api/chat",
        ))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(
            json!({
                "model": route.default_model_id,
                "messages": [{ "role": "user", "content": "ping" }],
                "stream": false,
            })
            .to_string(),
        )
        .send()
        .await
        .map_err(|error| format!("route probe upstream request failed: {error}"))?;
    ensure_probe_response_success(response).await
}

async fn ensure_probe_response_success(response: reqwest::Response) -> StdResult<(), String> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let text = response
        .text()
        .await
        .unwrap_or_else(|error| format!("failed to read upstream probe error: {error}"));
    let payload = serde_json::from_str::<Value>(&text).ok();
    Err(resolve_error_message(payload.as_ref(), text.trim(), status))
}
