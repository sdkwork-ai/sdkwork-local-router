use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::state::AppState;
use sdkwork_lr_store::DEFAULT_USER_ID;

pub const ACCESS_TOKEN_HEADER: &str = "access-token";
pub const X_API_KEY_HEADER: &str = "x-api-key";
pub const X_GOOG_API_KEY_HEADER: &str = "x-goog-api-key";
pub const X_SDKWORK_CLIENT_API_KEY_ID_HEADER: &str = "x-sdkwork-client-api-key-id";
pub const X_SDKWORK_SUBJECT_USER_ID_HEADER: &str = "x-sdkwork-subject-user-id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestContextSource {
    Default,
    ClientApiKey,
    SignedSubjectHeader,
}

impl RequestContextSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ClientApiKey => "client_api_key",
            Self::SignedSubjectHeader => "signed_subject_header",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    pub user_id: i64,
    pub source: RequestContextSource,
    pub client_api_key_id: Option<i64>,
    pub api_group: &'static str,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            user_id: DEFAULT_USER_ID,
            source: RequestContextSource::Default,
            client_api_key_id: None,
            api_group: crate::api_groups::LOCAL_ROUTER_OPEN_API,
        }
    }
}

impl RequestContext {
    pub fn new(user_id: i64, source: RequestContextSource) -> Self {
        Self {
            user_id: normalize_user_id(user_id),
            source,
            client_api_key_id: None,
            api_group: crate::api_groups::LOCAL_ROUTER_OPEN_API,
        }
    }

    pub fn with_client_api_key_id(mut self, client_api_key_id: Option<i64>) -> Self {
        self.client_api_key_id = client_api_key_id;
        self
    }

    pub fn with_api_group(mut self, api_group: &'static str) -> Self {
        self.api_group = api_group;
        self
    }
}

pub fn context_from_request(req: &Request) -> RequestContext {
    req.extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default()
}

pub fn context_from_headers(headers: &HeaderMap) -> RequestContext {
    app_api_context(headers)
}

pub async fn proxy_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    match resolve_proxy_context(&state, req.headers(), req.uri()).await {
        Ok(context) => {
            req.extensions_mut().insert(context);
            next.run(req).await
        }
        Err(response) => response,
    }
}

pub async fn app_auth_middleware(
    State(_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let context = app_api_context(req.headers());
    req.extensions_mut().insert(context);
    next.run(req).await
}

pub async fn admin_auth_middleware(
    State(_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let context = backend_api_context(req.headers());
    req.extensions_mut().insert(context);
    next.run(req).await
}

async fn resolve_proxy_context(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<RequestContext, Response> {
    if let Some(secret) = client_api_key_secret_from_headers_and_uri(headers, uri) {
        match state
            .store
            .authenticate_client_api_key_secret(&secret)
            .await
        {
            Ok(Some(client_api_key)) => {
                return Ok(RequestContext::new(
                    client_api_key.user_id,
                    RequestContextSource::ClientApiKey,
                )
                .with_client_api_key_id(Some(client_api_key.id))
                .with_api_group(crate::api_groups::LOCAL_ROUTER_OPEN_API));
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = %error, "failed to authenticate local-router client API key");
                return Err(auth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "auth_error",
                    "failed to authenticate client API key",
                ));
            }
        }
    }

    Err(auth_error(
        StatusCode::UNAUTHORIZED,
        "invalid_client_api_key",
        "missing or invalid client API key",
    ))
}

pub fn app_api_context(headers: &HeaderMap) -> RequestContext {
    resolve_app_backend_context(headers)
        .unwrap_or_default()
        .with_api_group(crate::api_groups::LOCAL_ROUTER_APP_API)
}

pub fn backend_api_context(headers: &HeaderMap) -> RequestContext {
    resolve_app_backend_context(headers)
        .unwrap_or_default()
        .with_api_group(crate::api_groups::LOCAL_ROUTER_BACKEND_API)
}

pub fn resolve_app_backend_context(headers: &HeaderMap) -> Option<RequestContext> {
    signed_subject_user_id(headers)
        .map(|user_id| RequestContext::new(user_id, RequestContextSource::SignedSubjectHeader))
}

pub fn client_api_key_secret_from_headers_and_uri(
    headers: &HeaderMap,
    uri: &Uri,
) -> Option<String> {
    bearer_token(headers)
        .or_else(|| header_string(headers, X_API_KEY_HEADER))
        .or_else(|| header_string(headers, X_GOOG_API_KEY_HEADER))
        .or_else(|| query_key(uri))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = header_string(headers, axum::http::header::AUTHORIZATION.as_str())?;
    let mut parts = value.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;
    if parts.next().is_some() || !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    Some(token.to_owned())
}

fn signed_subject_user_id(headers: &HeaderMap) -> Option<i64> {
    header_string(headers, X_SDKWORK_SUBJECT_USER_ID_HEADER).and_then(|value| parse_user_id(&value))
}

fn parse_user_id(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    trimmed
        .parse::<i64>()
        .ok()
        .map(normalize_user_id)
        .or_else(|| {
            trimmed
                .strip_prefix("user_")
                .and_then(|suffix| suffix.parse::<i64>().ok())
                .map(normalize_user_id)
        })
}

fn normalize_user_id(user_id: i64) -> i64 {
    user_id.max(DEFAULT_USER_ID)
}

fn query_key(uri: &Uri) -> Option<String> {
    uri.query()?.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = urlencoding::decode(name).ok()?;
        name.eq_ignore_ascii_case("key")
            .then(|| {
                urlencoding::decode(value)
                    .ok()
                    .map(|value| value.into_owned())
            })
            .flatten()
    })
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn auth_error(status: StatusCode, code: &'static str, message: impl ToString) -> Response {
    (
        status,
        axum::Json(json!({
            "error": {
                "message": message.to_string(),
                "type": "auth_error",
                "param": null,
                "code": code,
            }
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn claw_app_session_context_requires_matching_auth_and_access_subjects() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer v1.10.20.42.1.999.signature"),
        );
        headers.insert(
            ACCESS_TOKEN_HEADER,
            HeaderValue::from_static("v1.10.20.42.2.1000.signature"),
        );

        assert!(resolve_app_backend_context(&headers).is_none());
    }

    #[test]
    fn claw_app_session_context_rejects_mismatched_access_subject() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer v1.10.20.42.1.999.signature"),
        );
        headers.insert(
            ACCESS_TOKEN_HEADER,
            HeaderValue::from_static("v1.10.20.43.2.1000.signature"),
        );

        assert!(resolve_app_backend_context(&headers).is_none());
    }

    #[test]
    fn jwt_context_is_not_accepted_without_subject_projection() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer header.payload.signature"),
        );

        assert!(resolve_app_backend_context(&headers).is_none());
    }

    #[test]
    fn app_backend_context_reads_token_user_without_local_client_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer v1.10.20.77.1.999.signature"),
        );
        headers.insert(
            ACCESS_TOKEN_HEADER,
            HeaderValue::from_static("v1.10.20.77.2.1000.signature"),
        );

        assert!(resolve_app_backend_context(&headers).is_none());
    }

    #[test]
    fn app_backend_context_does_not_treat_client_api_key_as_user_identity() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer sk-local-client-key"),
        );
        headers.insert(
            X_API_KEY_HEADER,
            HeaderValue::from_static("sk-local-client-key"),
        );
        headers.insert(
            X_GOOG_API_KEY_HEADER,
            HeaderValue::from_static("sk-local-client-key"),
        );

        assert!(resolve_app_backend_context(&headers).is_none());
    }

    #[test]
    fn app_and_backend_contexts_keep_distinct_api_groups() {
        let mut headers = HeaderMap::new();
        headers.insert(
            X_SDKWORK_SUBJECT_USER_ID_HEADER,
            HeaderValue::from_static("42"),
        );

        let app_context = app_api_context(&headers);
        let backend_context = backend_api_context(&headers);

        assert_eq!(app_context.user_id, 42);
        assert_eq!(
            app_context.api_group,
            crate::api_groups::LOCAL_ROUTER_APP_API
        );
        assert_eq!(backend_context.user_id, 42);
        assert_eq!(
            backend_context.api_group,
            crate::api_groups::LOCAL_ROUTER_BACKEND_API
        );
    }

    #[test]
    fn client_api_key_query_auth_reads_encoded_and_case_variant_key_name() {
        let headers = HeaderMap::new();
        let uri: Uri = "/google/v1/models/gemini-pro:generateContent?Key=sk%2Dlocal%2Dclient"
            .parse()
            .unwrap();

        let secret = client_api_key_secret_from_headers_and_uri(&headers, &uri)
            .expect("query key should be decoded");

        assert_eq!(secret, "sk-local-client");

        let uri: Uri = "/google/v1/models/gemini-pro:generateContent?%6B%65%79=sk%2Dencoded"
            .parse()
            .unwrap();
        let secret = client_api_key_secret_from_headers_and_uri(&headers, &uri)
            .expect("encoded query key should be decoded");

        assert_eq!(secret, "sk-encoded");
    }
}
