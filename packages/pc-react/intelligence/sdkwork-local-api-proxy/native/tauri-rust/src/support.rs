use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub type LocalApiProxyHttpError = (StatusCode, Json<Value>);
pub type LocalApiProxyHttpResult<T> = std::result::Result<T, LocalApiProxyHttpError>;

pub fn duration_to_ms(duration: Duration) -> u64 {
    if duration.is_zero() {
        return 0;
    }

    let millis = duration.as_millis().min(u128::from(u64::MAX)) as u64;
    millis.max(1)
}

pub fn trim_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = trimmed.chars().take(4_000).collect::<String>();
    if trimmed.chars().count() > 4_000 {
        normalized.push_str("...");
    }
    Some(normalized)
}

pub fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

pub fn proxy_error(status: StatusCode, message: &str) -> LocalApiProxyHttpError {
    (status, Json(json!({ "error": message })))
}
