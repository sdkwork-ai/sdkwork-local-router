use crate::{
    error::{LocalApiProxyNativeError, LocalApiProxyNativeResult as Result},
    response::{extract_token_usage, LocalAiProxyTokenUsage},
    translation::{
        build_openai_response_usage, extract_gemini_response_text, extract_ollama_response_text,
        map_anthropic_stop_reason, map_ollama_done_reason,
    },
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenAiStreamFrameProjection {
    pub stream_id: Option<String>,
    pub model: Option<String>,
    pub usage: Option<LocalAiProxyTokenUsage>,
    pub text_delta: Option<String>,
    pub finish_reason: Option<String>,
    pub ensure_response_created: bool,
    pub should_complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAiStreamEndpoint {
    ChatCompletions,
    Responses,
}

pub fn merge_stream_usage(current: &mut LocalAiProxyTokenUsage, usage: &LocalAiProxyTokenUsage) {
    current.input_tokens = current.input_tokens.max(usage.input_tokens);
    current.output_tokens = current.output_tokens.max(usage.output_tokens);
    current.cache_tokens = current.cache_tokens.max(usage.cache_tokens);
    let prompt_completion_total = current.input_tokens.saturating_add(current.output_tokens);
    let merged_total = usage.total_tokens.max(if prompt_completion_total > 0 {
        prompt_completion_total
    } else {
        current.cache_tokens
    });
    current.total_tokens = current.total_tokens.max(merged_total);
}

pub fn openai_stream_endpoint_for_suffix(endpoint_suffix: &str) -> Result<OpenAiStreamEndpoint> {
    match endpoint_suffix {
        "chat/completions" => Ok(OpenAiStreamEndpoint::ChatCompletions),
        "responses" => Ok(OpenAiStreamEndpoint::Responses),
        _ => Err(LocalApiProxyNativeError::InvalidOperation(format!(
            "Unsupported OpenAI-compatible streaming endpoint: {endpoint_suffix}"
        ))),
    }
}

pub fn is_openai_stream_request(payload: &Value) -> bool {
    payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn drain_sse_frames(buffer: &mut String) -> Vec<ParsedSseEvent> {
    *buffer = buffer.replace("\r\n", "\n");
    let mut frames = Vec::new();

    while let Some(index) = buffer.find("\n\n") {
        let frame_text = buffer[..index].to_string();
        *buffer = buffer[index + 2..].to_string();
        if let Some(frame) = parse_sse_frame(&frame_text) {
            frames.push(frame);
        }
    }

    frames
}

pub fn flush_sse_frame(buffer: &mut String) -> Option<ParsedSseEvent> {
    *buffer = buffer.replace("\r\n", "\n");
    let trailing = buffer.trim();
    if trailing.is_empty() {
        return None;
    }

    parse_sse_frame(trailing)
}

pub fn drain_json_line_payloads(buffer: &mut String) -> Vec<Value> {
    *buffer = buffer.replace("\r\n", "\n");
    let mut frames = Vec::new();

    while let Some(index) = buffer.find('\n') {
        let frame_text = buffer[..index].trim().to_string();
        *buffer = buffer[index + 1..].to_string();
        if frame_text.is_empty() {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(&frame_text) {
            frames.push(payload);
        }
    }

    frames
}

pub fn flush_json_line_payload(buffer: &mut String) -> Option<Value> {
    *buffer = buffer.replace("\r\n", "\n");
    let trailing = buffer.trim();
    if trailing.is_empty() {
        return None;
    }

    serde_json::from_str::<Value>(trailing).ok()
}

pub fn build_openai_chat_stream_chunk(
    id: &str,
    model: &str,
    delta: Value,
    finish_reason: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": finish_reason
        }]
    })
}

pub fn build_openai_response_created_event(id: &str, model: &str) -> Value {
    json!({
        "type": "response.created",
        "response": {
            "id": id,
            "object": "response",
            "model": model,
            "output": []
        }
    })
}

pub fn build_openai_response_delta_event(id: &str, delta: &str) -> Value {
    json!({
        "type": "response.output_text.delta",
        "response_id": id,
        "output_index": 0,
        "content_index": 0,
        "delta": delta
    })
}

pub fn build_openai_response_completed_event(
    id: &str,
    model: &str,
    text: &str,
    usage: &LocalAiProxyTokenUsage,
) -> Value {
    let mut response = build_openai_response_object_from_stream(id, model, text);
    if let Some(response_usage) = build_openai_response_usage(usage) {
        response["usage"] = response_usage;
    }

    json!({
        "type": "response.completed",
        "response": response
    })
}

pub fn map_gemini_finish_reason(reason: Option<&str>) -> Option<&'static str> {
    match reason.unwrap_or_default() {
        "MAX_TOKENS" => Some("length"),
        "STOP" | "FINISH_REASON_UNSPECIFIED" => Some("stop"),
        "" => None,
        _ => Some("stop"),
    }
}

pub fn project_anthropic_openai_stream_frame(
    frame: &ParsedSseEvent,
) -> Option<OpenAiStreamFrameProjection> {
    if frame.data.trim() == "[DONE]" {
        return Some(OpenAiStreamFrameProjection {
            should_complete: true,
            ..Default::default()
        });
    }

    let Ok(payload) = serde_json::from_str::<Value>(&frame.data) else {
        return None;
    };

    let event_name = frame
        .event
        .as_deref()
        .or_else(|| payload.get("type").and_then(Value::as_str))
        .unwrap_or_default();

    match event_name {
        "message_start" => Some(OpenAiStreamFrameProjection {
            stream_id: payload
                .pointer("/message/id")
                .and_then(Value::as_str)
                .map(str::to_string),
            model: payload
                .pointer("/message/model")
                .and_then(Value::as_str)
                .map(str::to_string),
            usage: payload.get("message").map(extract_token_usage),
            ensure_response_created: true,
            ..Default::default()
        }),
        "content_block_delta" => {
            if payload.pointer("/delta/type").and_then(Value::as_str) != Some("text_delta") {
                return None;
            }

            payload
                .pointer("/delta/text")
                .and_then(Value::as_str)
                .map(|text| OpenAiStreamFrameProjection {
                    text_delta: Some(text.to_string()),
                    ..Default::default()
                })
        }
        "message_delta" => Some(OpenAiStreamFrameProjection {
            usage: Some(extract_token_usage(&payload)),
            finish_reason: payload
                .pointer("/delta/stop_reason")
                .and_then(Value::as_str)
                .map(|reason| map_anthropic_stop_reason(Some(reason)).to_string()),
            ..Default::default()
        }),
        "message_stop" => Some(OpenAiStreamFrameProjection {
            should_complete: true,
            ..Default::default()
        }),
        _ => None,
    }
}

pub fn project_gemini_openai_stream_frame(
    frame: &ParsedSseEvent,
) -> Option<OpenAiStreamFrameProjection> {
    if frame.data.trim() == "[DONE]" {
        return Some(OpenAiStreamFrameProjection {
            should_complete: true,
            ..Default::default()
        });
    }

    let Ok(payload) = serde_json::from_str::<Value>(&frame.data) else {
        return None;
    };

    let usage = extract_token_usage(&payload);
    let text_delta = extract_gemini_response_text(&payload);
    let finish_reason = map_gemini_finish_reason(
        payload
            .pointer("/candidates/0/finishReason")
            .and_then(Value::as_str),
    )
    .map(str::to_string);

    if usage == LocalAiProxyTokenUsage::default()
        && text_delta.is_empty()
        && finish_reason.is_none()
    {
        return None;
    }

    Some(OpenAiStreamFrameProjection {
        usage: Some(usage),
        text_delta: (!text_delta.is_empty()).then_some(text_delta),
        should_complete: finish_reason.is_some(),
        finish_reason,
        ..Default::default()
    })
}

pub fn project_ollama_openai_stream_frame(payload: &Value) -> Option<OpenAiStreamFrameProjection> {
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    let usage = extract_token_usage(payload);
    let text_delta = extract_ollama_response_text(payload);
    let has_tool_calls = payload
        .pointer("/message/tool_calls")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let should_complete = payload
        .get("done")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let finish_reason = if should_complete {
        if has_tool_calls {
            Some("tool_calls".to_string())
        } else {
            map_ollama_done_reason(payload.get("done_reason").and_then(Value::as_str))
                .map(str::to_string)
        }
    } else {
        None
    };

    if model.is_none()
        && usage == LocalAiProxyTokenUsage::default()
        && text_delta.is_empty()
        && !should_complete
    {
        return None;
    }

    Some(OpenAiStreamFrameProjection {
        model,
        usage: Some(usage),
        text_delta: (!text_delta.is_empty()).then_some(text_delta),
        should_complete,
        finish_reason,
        ..Default::default()
    })
}

fn build_openai_response_object_from_stream(id: &str, model: &str, text: &str) -> Value {
    json!({
        "id": id,
        "object": "response",
        "model": model,
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": text,
                "annotations": []
            }]
        }]
    })
}

fn parse_sse_frame(frame: &str) -> Option<ParsedSseEvent> {
    let mut event = None;
    let mut data_lines = Vec::new();

    for line in frame.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event = Some(value.trim().to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim_start().to_string());
        }
    }

    if data_lines.is_empty() {
        return None;
    }

    Some(ParsedSseEvent {
        event,
        data: data_lines.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_openai_chat_stream_chunk, build_openai_response_completed_event,
        build_openai_response_created_event, build_openai_response_delta_event,
        drain_json_line_payloads, drain_sse_frames, flush_json_line_payload, flush_sse_frame,
        is_openai_stream_request, map_gemini_finish_reason, merge_stream_usage,
        openai_stream_endpoint_for_suffix, project_anthropic_openai_stream_frame,
        project_gemini_openai_stream_frame, project_ollama_openai_stream_frame,
        OpenAiStreamEndpoint, ParsedSseEvent,
    };
    use crate::response::LocalAiProxyTokenUsage;
    use serde_json::{json, Value};

    #[test]
    fn parses_sse_frames_and_flushes_trailing_frame() {
        let mut buffer =
            "event: message_start\r\ndata: {\"ok\":true}\r\n\r\ndata: [DONE]".to_string();

        let frames = drain_sse_frames(&mut buffer);
        let trailing = flush_sse_frame(&mut buffer).expect("trailing frame");

        assert_eq!(
            frames,
            vec![ParsedSseEvent {
                event: Some("message_start".to_string()),
                data: "{\"ok\":true}".to_string(),
            }]
        );
        assert_eq!(trailing.data, "[DONE]");
    }

    #[test]
    fn parses_json_line_payloads_and_flushes_trailing_payload() {
        let mut buffer = "{\"a\":1}\r\n\n{\"b\":2}".to_string();

        let payloads = drain_json_line_payloads(&mut buffer);
        let trailing = flush_json_line_payload(&mut buffer).expect("trailing payload");

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].pointer("/a").and_then(Value::as_u64), Some(1));
        assert_eq!(trailing.pointer("/b").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn builds_openai_stream_events_from_shared_semantics() {
        let chunk = build_openai_chat_stream_chunk(
            "chatcmpl-1",
            "gemini-2.5-pro",
            json!({ "content": "hello" }),
            Some("stop"),
        );
        let created = build_openai_response_created_event("resp-1", "gemini-2.5-pro");
        let delta = build_openai_response_delta_event("resp-1", "partial text");
        let completed = build_openai_response_completed_event(
            "resp-1",
            "gemini-2.5-pro",
            "final text",
            &LocalAiProxyTokenUsage {
                total_tokens: 12,
                input_tokens: 9,
                output_tokens: 3,
                cache_tokens: 2,
            },
        );

        assert_eq!(
            chunk
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str),
            Some("hello")
        );
        assert_eq!(
            created.pointer("/type").and_then(Value::as_str),
            Some("response.created")
        );
        assert_eq!(
            delta.pointer("/delta").and_then(Value::as_str),
            Some("partial text")
        );
        assert_eq!(
            completed
                .pointer("/response/usage/total_tokens")
                .and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            completed
                .pointer("/response/usage/input_tokens_details/cached_tokens")
                .and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn maps_gemini_finish_reasons() {
        assert_eq!(map_gemini_finish_reason(Some("MAX_TOKENS")), Some("length"));
        assert_eq!(map_gemini_finish_reason(Some("STOP")), Some("stop"));
        assert_eq!(map_gemini_finish_reason(Some("")), None);
    }

    #[test]
    fn merges_stream_usage_without_double_counting_cached_prompt_tokens() {
        let mut usage = LocalAiProxyTokenUsage {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            cache_tokens: 0,
        };

        merge_stream_usage(
            &mut usage,
            &LocalAiProxyTokenUsage {
                total_tokens: 12_307,
                input_tokens: 12_307,
                output_tokens: 0,
                cache_tokens: 4_096,
            },
        );
        merge_stream_usage(
            &mut usage,
            &LocalAiProxyTokenUsage {
                total_tokens: 0,
                input_tokens: 12_307,
                output_tokens: 6,
                cache_tokens: 4_096,
            },
        );

        assert_eq!(usage.input_tokens, 12_307);
        assert_eq!(usage.output_tokens, 6);
        assert_eq!(usage.cache_tokens, 4_096);
        assert_eq!(usage.total_tokens, 12_313);
    }

    #[test]
    fn resolves_openai_stream_request_intent_and_endpoint_suffix() {
        assert!(is_openai_stream_request(&json!({ "stream": true })));
        assert!(!is_openai_stream_request(&json!({ "stream": false })));
        assert!(!is_openai_stream_request(&json!({})));

        assert_eq!(
            openai_stream_endpoint_for_suffix("chat/completions").expect("chat endpoint"),
            OpenAiStreamEndpoint::ChatCompletions
        );
        assert_eq!(
            openai_stream_endpoint_for_suffix("responses").expect("responses endpoint"),
            OpenAiStreamEndpoint::Responses
        );
        assert!(openai_stream_endpoint_for_suffix("embeddings").is_err());
    }

    #[test]
    fn projects_anthropic_stream_frames() {
        let start = project_anthropic_openai_stream_frame(&ParsedSseEvent {
            event: Some("message_start".to_string()),
            data: json!({
                "message": {
                    "id": "msg_123",
                    "model": "claude-sonnet",
                    "usage": {
                        "input_tokens": 12
                    }
                }
            })
            .to_string(),
        })
        .expect("start projection");
        let delta = project_anthropic_openai_stream_frame(&ParsedSseEvent {
            event: Some("content_block_delta".to_string()),
            data: json!({
                "delta": {
                    "type": "text_delta",
                    "text": "hello"
                }
            })
            .to_string(),
        })
        .expect("delta projection");
        let stop = project_anthropic_openai_stream_frame(&ParsedSseEvent {
            event: Some("message_delta".to_string()),
            data: json!({
                "delta": {
                    "stop_reason": "max_tokens"
                },
                "usage": {
                    "input_tokens": 12,
                    "output_tokens": 4
                }
            })
            .to_string(),
        })
        .expect("stop projection");

        assert_eq!(start.stream_id.as_deref(), Some("msg_123"));
        assert_eq!(start.model.as_deref(), Some("claude-sonnet"));
        assert!(start.ensure_response_created);
        assert_eq!(delta.text_delta.as_deref(), Some("hello"));
        assert_eq!(stop.finish_reason.as_deref(), Some("length"));
        assert_eq!(
            stop.usage.as_ref().map(|value| value.total_tokens),
            Some(16)
        );
    }

    #[test]
    fn projects_gemini_stream_frames() {
        let projection = project_gemini_openai_stream_frame(&ParsedSseEvent {
            event: None,
            data: json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "gemini hello" }]
                    },
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 10,
                    "candidatesTokenCount": 2,
                    "totalTokenCount": 12
                }
            })
            .to_string(),
        })
        .expect("gemini projection");

        assert_eq!(projection.text_delta.as_deref(), Some("gemini hello"));
        assert_eq!(projection.finish_reason.as_deref(), Some("stop"));
        assert!(projection.should_complete);
        assert_eq!(
            projection.usage.as_ref().map(|value| value.total_tokens),
            Some(12)
        );
    }

    #[test]
    fn projects_ollama_stream_frames() {
        let projection = project_ollama_openai_stream_frame(&json!({
            "model": "qwen3",
            "message": {
                "content": "ollama hello",
                "tool_calls": [{
                    "function": {
                        "name": "web_search"
                    }
                }]
            },
            "prompt_eval_count": 9,
            "eval_count": 3,
            "done": true,
            "done_reason": "tool_calls"
        }))
        .expect("ollama projection");

        assert_eq!(projection.model.as_deref(), Some("qwen3"));
        assert_eq!(projection.text_delta.as_deref(), Some("ollama hello"));
        assert_eq!(projection.finish_reason.as_deref(), Some("tool_calls"));
        assert!(projection.should_complete);
        assert_eq!(
            projection.usage.as_ref().map(|value| value.total_tokens),
            Some(12)
        );
    }
}
