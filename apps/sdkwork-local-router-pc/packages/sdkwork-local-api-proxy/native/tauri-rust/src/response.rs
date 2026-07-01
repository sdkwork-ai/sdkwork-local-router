use crate::runtime::extract_response_preview_from_value;
use axum::{http::StatusCode, Json};
use serde_json::Value;
use std::fmt::Display;

const MAX_NORMALIZED_RESPONSE_CHARS: usize = 4_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyTokenUsage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
}

pub fn normalize_response_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = trimmed
        .chars()
        .take(MAX_NORMALIZED_RESPONSE_CHARS)
        .collect::<String>();
    if trimmed.chars().count() > MAX_NORMALIZED_RESPONSE_CHARS {
        normalized.push_str("...");
    }
    Some(normalized)
}

pub fn format_json_response_body(body: &Value) -> Option<String> {
    serde_json::to_string_pretty(body).ok()
}

pub fn resolve_response_preview(payload: Option<&Value>, text: &str) -> Option<String> {
    payload
        .and_then(extract_response_preview_from_value)
        .and_then(|value| normalize_response_text(&value))
        .or_else(|| normalize_response_text(text))
}

pub fn extract_error_message_from_payload(payload: Option<&Value>) -> Option<String> {
    let Some(payload) = payload else {
        return None;
    };

    payload
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| payload.pointer("/error").and_then(Value::as_str))
        .or_else(|| payload.pointer("/message").and_then(Value::as_str))
        .or_else(|| payload.pointer("/detail").and_then(Value::as_str))
        .and_then(normalize_response_text)
}

pub fn resolve_error_message(payload: Option<&Value>, text: &str, status: impl Display) -> String {
    extract_error_message_from_payload(payload)
        .or_else(|| normalize_response_text(text))
        .unwrap_or_else(|| format!("upstream returned status {status}"))
}

pub fn extract_http_error_message(error: &(StatusCode, Json<Value>)) -> String {
    resolve_error_message(Some(&error.1 .0), "", error.0)
}

pub fn extract_token_usage(payload: &Value) -> LocalAiProxyTokenUsage {
    let input_tokens = value_u64(payload, "/usage/prompt_tokens")
        .or_else(|| value_u64(payload, "/usage/input_tokens"))
        .or_else(|| value_u64(payload, "/usageMetadata/promptTokenCount"))
        .or_else(|| value_u64(payload, "/prompt_eval_count"))
        .unwrap_or(0);
    let output_tokens = value_u64(payload, "/usage/completion_tokens")
        .or_else(|| value_u64(payload, "/usage/output_tokens"))
        .or_else(|| value_u64(payload, "/usageMetadata/candidatesTokenCount"))
        .or_else(|| value_u64(payload, "/eval_count"))
        .unwrap_or(0);
    let anthropic_cache_tokens = value_u64(payload, "/usage/cache_creation_input_tokens")
        .unwrap_or(0)
        .saturating_add(value_u64(payload, "/usage/cache_read_input_tokens").unwrap_or(0));
    let cache_tokens = value_u64(payload, "/usage/cache_tokens")
        .or_else(|| value_u64(payload, "/usage/prompt_tokens_details/cached_tokens"))
        .or_else(|| value_u64(payload, "/usage/input_tokens_details/cached_tokens"))
        .or_else(|| value_u64(payload, "/usageMetadata/cachedContentTokenCount"))
        .unwrap_or(anthropic_cache_tokens);
    let prompt_completion_total = input_tokens.saturating_add(output_tokens);
    let total_tokens = value_u64(payload, "/usage/total_tokens")
        .or_else(|| value_u64(payload, "/usageMetadata/totalTokenCount"))
        .unwrap_or_else(|| {
            if prompt_completion_total > 0 {
                prompt_completion_total
            } else {
                cache_tokens
            }
        });

    LocalAiProxyTokenUsage {
        total_tokens,
        input_tokens,
        output_tokens,
        cache_tokens,
    }
}

fn value_u64(payload: &Value, pointer: &str) -> Option<u64> {
    payload.pointer(pointer).and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::{
        extract_error_message_from_payload, extract_token_usage, format_json_response_body,
        normalize_response_text, resolve_error_message, resolve_response_preview,
        LocalAiProxyTokenUsage,
    };
    use serde_json::json;

    #[test]
    fn resolves_error_message_from_payload_text_and_status_fallbacks() {
        let payload = json!({
            "error": {
                "message": "  upstream denied the request  "
            }
        });

        assert_eq!(
            extract_error_message_from_payload(Some(&payload)).as_deref(),
            Some("upstream denied the request")
        );
        assert_eq!(
            resolve_error_message(Some(&payload), "ignored", 429).as_str(),
            "upstream denied the request"
        );
        assert_eq!(
            resolve_error_message(None, "  upstream timed out  ", 504).as_str(),
            "upstream timed out"
        );
        assert_eq!(
            resolve_error_message(None, "   ", 503).as_str(),
            "upstream returned status 503"
        );
    }

    #[test]
    fn resolves_response_preview_from_payload_before_text_fallback() {
        let payload = json!({
            "choices": [{
                "message": {
                    "content": "structured response preview"
                }
            }]
        });

        assert_eq!(
            resolve_response_preview(Some(&payload), "plain text preview").as_deref(),
            Some("structured response preview")
        );
        assert_eq!(
            resolve_response_preview(None, "  plain text preview  ").as_deref(),
            Some("plain text preview")
        );
    }

    #[test]
    fn normalizes_response_text_for_logging() {
        let source = format!("  {}  ", "x".repeat(4_001));
        let normalized = normalize_response_text(&source).expect("normalized text");

        assert_eq!(normalized.len(), 4_003);
        assert!(normalized.ends_with("..."));
        assert_eq!(normalize_response_text("   "), None);
    }

    #[test]
    fn formats_json_response_body_for_audit_storage() {
        let payload = json!({
            "ok": true,
            "nested": {
                "value": 1
            }
        });

        let formatted = format_json_response_body(&payload).expect("formatted json");

        assert!(formatted.contains("\"ok\": true"));
        assert!(formatted.contains("\"nested\""));
    }

    #[test]
    fn extracts_token_usage_across_supported_upstream_shapes() {
        let openai = json!({
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 30,
                "total_tokens": 150
            }
        });
        let gemini = json!({
            "usageMetadata": {
                "promptTokenCount": 20,
                "candidatesTokenCount": 5,
                "cachedContentTokenCount": 3,
                "totalTokenCount": 25
            }
        });
        let anthropic = json!({
            "usage": {
                "input_tokens": 10,
                "output_tokens": 3,
                "cache_creation_input_tokens": 4,
                "cache_read_input_tokens": 6
            }
        });
        let ollama = json!({
            "prompt_eval_count": 11,
            "eval_count": 7
        });

        assert_eq!(
            extract_token_usage(&openai),
            LocalAiProxyTokenUsage {
                total_tokens: 150,
                input_tokens: 120,
                output_tokens: 30,
                cache_tokens: 0,
            }
        );
        assert_eq!(
            extract_token_usage(&gemini),
            LocalAiProxyTokenUsage {
                total_tokens: 25,
                input_tokens: 20,
                output_tokens: 5,
                cache_tokens: 3,
            }
        );
        assert_eq!(
            extract_token_usage(&anthropic),
            LocalAiProxyTokenUsage {
                total_tokens: 13,
                input_tokens: 10,
                output_tokens: 3,
                cache_tokens: 10,
            }
        );
        assert_eq!(
            extract_token_usage(&ollama),
            LocalAiProxyTokenUsage {
                total_tokens: 18,
                input_tokens: 11,
                output_tokens: 7,
                cache_tokens: 0,
            }
        );
    }
}
