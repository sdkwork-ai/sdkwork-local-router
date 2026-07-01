use crate::error::{LocalApiProxyNativeError, LocalApiProxyNativeResult as Result};
use crate::response::{extract_token_usage, LocalAiProxyTokenUsage};
use crate::streaming::is_openai_stream_request;
use serde_json::json;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalAiProxyConversationTurn {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalAiProxyConversationProjection {
    pub system: Option<String>,
    pub conversation: Vec<LocalAiProxyConversationTurn>,
}

pub fn extract_openai_text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(extract_openai_text_content)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n"))
        }
        Value::Object(object) => object
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| object.get("content").and_then(extract_openai_text_content)),
        _ => None,
    }
}

pub fn parse_openai_chat_conversation(
    payload: &Value,
) -> Result<LocalAiProxyConversationProjection> {
    let messages = payload
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            LocalApiProxyNativeError::ValidationFailed(
                "OpenAI-compatible chat requests must include a messages array.".to_string(),
            )
        })?;
    let mut system_parts = Vec::new();
    let mut conversation = Vec::new();

    for entry in messages {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let role = object
            .get("role")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let Some(content) = object
            .get("content")
            .and_then(extract_openai_text_content)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        match role {
            "system" => system_parts.push(content),
            "user" | "assistant" => conversation.push(LocalAiProxyConversationTurn {
                role: role.to_string(),
                content,
            }),
            _ => {}
        }
    }

    if conversation.is_empty() {
        return Err(LocalApiProxyNativeError::ValidationFailed(
            "OpenAI-compatible chat requests must include at least one user or assistant message."
                .to_string(),
        ));
    }

    Ok(LocalAiProxyConversationProjection {
        system: (!system_parts.is_empty()).then(|| system_parts.join("\n\n")),
        conversation,
    })
}

pub fn parse_openai_response_conversation(
    payload: &Value,
) -> Result<LocalAiProxyConversationProjection> {
    let system = payload
        .get("instructions")
        .and_then(extract_openai_text_content)
        .filter(|value| !value.is_empty());
    let input = payload.get("input").ok_or_else(|| {
        LocalApiProxyNativeError::ValidationFailed(
            "OpenAI responses requests must include an input field.".to_string(),
        )
    })?;

    let conversation = match input {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty())
                .then(|| {
                    vec![LocalAiProxyConversationTurn {
                        role: "user".to_string(),
                        content: trimmed.to_string(),
                    }]
                })
                .unwrap_or_default()
        }
        Value::Array(items) => items
            .iter()
            .filter_map(|entry| match entry {
                Value::String(text) => {
                    let trimmed = text.trim();
                    (!trimmed.is_empty()).then(|| LocalAiProxyConversationTurn {
                        role: "user".to_string(),
                        content: trimmed.to_string(),
                    })
                }
                Value::Object(object) => {
                    let role = object
                        .get("role")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or("user");
                    let content = object
                        .get("content")
                        .and_then(extract_openai_text_content)
                        .filter(|value| !value.is_empty())?;
                    Some(LocalAiProxyConversationTurn {
                        role: role.to_string(),
                        content,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    if conversation.is_empty() {
        return Err(LocalApiProxyNativeError::ValidationFailed(
            "OpenAI responses requests must include at least one text input.".to_string(),
        ));
    }

    Ok(LocalAiProxyConversationProjection {
        system,
        conversation,
    })
}

pub fn read_request_max_tokens(payload: &Value, fallback: u64) -> u64 {
    payload
        .get("max_tokens")
        .and_then(Value::as_u64)
        .or_else(|| payload.get("max_completion_tokens").and_then(Value::as_u64))
        .or_else(|| payload.get("max_output_tokens").and_then(Value::as_u64))
        .unwrap_or(fallback)
}

pub fn resolve_request_model_id(
    payload: &Value,
    fallback_model_id: Option<&str>,
) -> Result<String> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            fallback_model_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| {
            LocalApiProxyNativeError::ValidationFailed(
                "OpenAI-compatible request must specify a non-empty model id.".to_string(),
            )
        })
}

pub fn build_gemini_generate_content_payload(
    conversation: &[LocalAiProxyConversationTurn],
    system: Option<&str>,
    payload: &Value,
) -> Value {
    let mut contents = Vec::new();
    for turn in conversation.iter() {
        contents.push(json!({
            "role": if turn.role == "assistant" { "model" } else { "user" },
            "parts": [{ "text": turn.content }],
        }));
    }

    let mut generation_config = serde_json::Map::new();
    if let Some(value) = payload.get("temperature").and_then(Value::as_f64) {
        generation_config.insert("temperature".to_string(), Value::from(value));
    }
    if let Some(value) = payload.get("top_p").and_then(Value::as_f64) {
        generation_config.insert("topP".to_string(), Value::from(value));
    }
    generation_config.insert(
        "maxOutputTokens".to_string(),
        Value::from(read_request_max_tokens(payload, 8192)),
    );

    let mut root = serde_json::Map::new();
    root.insert("contents".to_string(), Value::Array(contents));
    if let Some(system) = system.map(str::trim).filter(|value| !value.is_empty()) {
        root.insert(
            "systemInstruction".to_string(),
            json!({
                "parts": [{ "text": system }]
            }),
        );
    }
    root.insert(
        "generationConfig".to_string(),
        Value::Object(generation_config),
    );
    Value::Object(root)
}

pub fn build_gemini_embeddings_request_payload(text: &str) -> Value {
    json!({
        "content": {
            "parts": [{ "text": text }]
        }
    })
}

pub fn build_ollama_messages(
    system: Option<String>,
    conversation: Vec<LocalAiProxyConversationTurn>,
) -> Vec<Value> {
    let mut messages = Vec::new();
    if let Some(system) = system
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        messages.push(json!({
            "role": "system",
            "content": system,
        }));
    }

    for turn in conversation {
        messages.push(json!({
            "role": turn.role,
            "content": turn.content,
        }));
    }

    messages
}

pub fn build_ollama_request_options(payload: &Value) -> Option<Value> {
    let mut options = serde_json::Map::new();
    if let Some(value) = payload.get("temperature").and_then(Value::as_f64) {
        options.insert("temperature".to_string(), Value::from(value));
    }
    if let Some(value) = payload.get("top_p").and_then(Value::as_f64) {
        options.insert("top_p".to_string(), Value::from(value));
    }

    let max_tokens = read_request_max_tokens(payload, 8192);
    if max_tokens > 0 {
        options.insert("num_predict".to_string(), Value::from(max_tokens));
    }

    (!options.is_empty()).then(|| Value::Object(options))
}

pub fn build_ollama_chat_request_payload(
    model_id: &str,
    system: Option<String>,
    conversation: Vec<LocalAiProxyConversationTurn>,
    stream: bool,
    payload: &Value,
    tools: Option<Value>,
) -> Value {
    let mut root = serde_json::Map::new();
    root.insert("model".to_string(), Value::String(model_id.to_string()));
    root.insert(
        "messages".to_string(),
        Value::Array(build_ollama_messages(system, conversation)),
    );
    root.insert("stream".to_string(), Value::Bool(stream));
    if let Some(options) = build_ollama_request_options(payload) {
        root.insert("options".to_string(), options);
    }
    if let Some(tools) = tools {
        root.insert("tools".to_string(), tools);
    }

    Value::Object(root)
}

pub fn build_ollama_embeddings_request_payload(model_id: &str, normalized_input: Value) -> Value {
    json!({
        "model": model_id,
        "input": normalized_input,
    })
}

pub fn build_anthropic_request_from_openai_chat(model_id: &str, payload: &Value) -> Result<Value> {
    let projection = parse_openai_chat_conversation(payload)?;
    Ok(build_anthropic_request_payload(
        model_id,
        projection.system,
        projection.conversation,
        payload,
        false,
        is_openai_stream_request(payload),
    ))
}

pub fn build_anthropic_request_from_openai_response(
    model_id: &str,
    payload: &Value,
) -> Result<Value> {
    let projection = parse_openai_response_conversation(payload)?;
    Ok(build_anthropic_request_payload(
        model_id,
        projection.system,
        projection.conversation,
        payload,
        true,
        is_openai_stream_request(payload),
    ))
}

pub fn build_gemini_request_from_openai_chat(payload: &Value) -> Result<Value> {
    let projection = parse_openai_chat_conversation(payload)?;
    Ok(build_gemini_generate_content_payload(
        &projection.conversation,
        projection.system.as_deref(),
        payload,
    ))
}

pub fn build_gemini_request_from_openai_response(payload: &Value) -> Result<Value> {
    let projection = parse_openai_response_conversation(payload)?;
    Ok(build_gemini_generate_content_payload(
        &projection.conversation,
        projection.system.as_deref(),
        payload,
    ))
}

pub fn build_gemini_request_from_openai_embeddings(payload: &Value) -> Result<Value> {
    let input = payload.get("input").ok_or_else(|| {
        LocalApiProxyNativeError::ValidationFailed(
            "OpenAI embeddings requests must include an input field.".to_string(),
        )
    })?;
    let Some(text) = extract_openai_text_content(input).filter(|value| !value.is_empty()) else {
        return Err(LocalApiProxyNativeError::ValidationFailed(
            "OpenAI embeddings requests must include a non-empty text input.".to_string(),
        ));
    };

    Ok(build_gemini_embeddings_request_payload(&text))
}

pub fn build_ollama_request_from_openai_chat(model_id: &str, payload: &Value) -> Result<Value> {
    let projection = parse_openai_chat_conversation(payload)?;
    Ok(build_ollama_chat_request_payload(
        model_id,
        projection.system,
        projection.conversation,
        is_openai_stream_request(payload),
        payload,
        payload.get("tools").cloned(),
    ))
}

pub fn build_ollama_request_from_openai_response(model_id: &str, payload: &Value) -> Result<Value> {
    let projection = parse_openai_response_conversation(payload)?;
    Ok(build_ollama_chat_request_payload(
        model_id,
        projection.system,
        projection.conversation,
        is_openai_stream_request(payload),
        payload,
        payload.get("tools").cloned(),
    ))
}

pub fn build_ollama_request_from_openai_embeddings(
    model_id: &str,
    payload: &Value,
) -> Result<Value> {
    let input = payload.get("input").ok_or_else(|| {
        LocalApiProxyNativeError::ValidationFailed(
            "OpenAI embeddings requests must include an input field.".to_string(),
        )
    })?;

    let normalized_input = match input {
        Value::String(_) | Value::Array(_) => input.clone(),
        _ => extract_openai_text_content(input)
            .map(Value::String)
            .ok_or_else(|| {
                LocalApiProxyNativeError::ValidationFailed(
                    "OpenAI embeddings requests must include a non-empty text input.".to_string(),
                )
            })?,
    };

    Ok(build_ollama_embeddings_request_payload(
        model_id,
        normalized_input,
    ))
}

pub fn build_anthropic_request_payload(
    model_id: &str,
    system: Option<String>,
    conversation: Vec<LocalAiProxyConversationTurn>,
    payload: &Value,
    normalize_non_assistant_roles_to_user: bool,
    stream: bool,
) -> Value {
    let mut root = serde_json::Map::new();
    root.insert("model".to_string(), Value::String(model_id.to_string()));
    root.insert(
        "max_tokens".to_string(),
        Value::from(read_request_max_tokens(payload, 8192)),
    );
    root.insert(
        "messages".to_string(),
        Value::Array(
            conversation
                .into_iter()
                .map(|turn| {
                    let role = if normalize_non_assistant_roles_to_user && turn.role != "assistant"
                    {
                        "user".to_string()
                    } else {
                        turn.role
                    };
                    json!({
                        "role": role,
                        "content": turn.content,
                    })
                })
                .collect(),
        ),
    );
    if let Some(system) = system
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        root.insert("system".to_string(), Value::String(system));
    }
    if let Some(value) = payload.get("temperature").and_then(Value::as_f64) {
        root.insert("temperature".to_string(), Value::from(value));
    }
    if let Some(value) = payload.get("top_p").and_then(Value::as_f64) {
        root.insert("top_p".to_string(), Value::from(value));
    }
    if stream {
        root.insert("stream".to_string(), Value::Bool(true));
    }

    Value::Object(root)
}

pub fn extract_anthropic_response_text(payload: &Value) -> String {
    payload
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|entry| entry.get("text").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

pub fn map_anthropic_stop_reason(reason: Option<&str>) -> &'static str {
    match reason.unwrap_or_default() {
        "max_tokens" => "length",
        _ => "stop",
    }
}

pub fn extract_gemini_response_text(payload: &Value) -> String {
    payload
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|entry| entry.get("text").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

pub fn extract_ollama_response_text(payload: &Value) -> String {
    payload
        .pointer("/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_default()
}

pub fn map_ollama_done_reason(reason: Option<&str>) -> Option<&'static str> {
    match reason.unwrap_or_default() {
        "length" => Some("length"),
        "stop" | "tool_calls" => Some("stop"),
        "" => None,
        _ => Some("stop"),
    }
}

pub fn build_openai_response_usage(usage: &LocalAiProxyTokenUsage) -> Option<Value> {
    if usage == &LocalAiProxyTokenUsage::default() {
        return None;
    }

    let mut response_usage = json!({
        "input_tokens": usage.input_tokens,
        "output_tokens": usage.output_tokens,
        "total_tokens": usage.total_tokens,
    });
    if usage.cache_tokens > 0 {
        response_usage["input_tokens_details"] = json!({
            "cached_tokens": usage.cache_tokens,
        });
    }

    Some(response_usage)
}

pub fn build_openai_chat_completion_from_anthropic(
    fallback_model_id: &str,
    upstream_body: &Value,
) -> Value {
    let content = extract_anthropic_response_text(upstream_body);
    let prompt_tokens = upstream_body
        .pointer("/usage/input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let completion_tokens = upstream_body
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    json!( {
        "id": upstream_body.get("id").and_then(Value::as_str).unwrap_or("chatcmpl-local-proxy"),
        "object": "chat.completion",
        "created": 0,
        "model": resolve_upstream_model_id(upstream_body, fallback_model_id),
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
            },
            "finish_reason": map_anthropic_stop_reason(upstream_body.get("stop_reason").and_then(Value::as_str)),
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        }
    })
}

pub fn build_openai_chat_completion_from_gemini(
    fallback_model_id: &str,
    upstream_body: &Value,
) -> Value {
    json!( {
        "id": "chatcmpl-local-proxy",
        "object": "chat.completion",
        "created": 0,
        "model": fallback_model_id,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": extract_gemini_response_text(upstream_body),
            },
            "finish_reason": "stop",
        }]
    })
}

pub fn build_openai_response_from_anthropic(
    fallback_model_id: &str,
    upstream_body: &Value,
) -> Value {
    let text = extract_anthropic_response_text(upstream_body);
    let usage = extract_token_usage(upstream_body);
    let mut response = json!( {
        "id": upstream_body.get("id").and_then(Value::as_str).unwrap_or("resp-local-proxy"),
        "object": "response",
        "model": resolve_upstream_model_id(upstream_body, fallback_model_id),
        "output": [openai_output_text_message(&text)]
    });
    if let Some(response_usage) = build_openai_response_usage(&usage) {
        response["usage"] = response_usage;
    }
    response
}

pub fn build_openai_response_from_gemini(fallback_model_id: &str, upstream_body: &Value) -> Value {
    let usage = extract_token_usage(upstream_body);
    let mut response = json!( {
        "id": "resp-local-proxy",
        "object": "response",
        "model": fallback_model_id,
        "output": [openai_output_text_message(&extract_gemini_response_text(upstream_body))]
    });
    if let Some(response_usage) = build_openai_response_usage(&usage) {
        response["usage"] = response_usage;
    }
    response
}

pub fn build_openai_embeddings_from_gemini(upstream_body: &Value) -> Value {
    json!( {
        "object": "list",
        "data": [{
            "object": "embedding",
            "index": 0,
            "embedding": upstream_body.pointer("/embedding/values").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        }]
    })
}

pub fn build_openai_chat_completion_from_ollama(
    fallback_model_id: &str,
    upstream_body: &Value,
) -> Value {
    let content = extract_ollama_response_text(upstream_body);
    let usage = extract_token_usage(upstream_body);
    let tool_calls = build_openai_tool_calls_from_ollama(upstream_body);
    let finish_reason = if !tool_calls.is_empty() {
        "tool_calls"
    } else {
        map_ollama_done_reason(upstream_body.get("done_reason").and_then(Value::as_str))
            .unwrap_or("stop")
    };
    let mut message = json!( {
        "role": "assistant",
        "content": content,
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
        if content.is_empty() {
            message["content"] = Value::Null;
        }
    }

    json!( {
        "id": "chatcmpl-local-proxy",
        "object": "chat.completion",
        "created": 0,
        "model": resolve_upstream_model_id(upstream_body, fallback_model_id),
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }],
        "usage": {
            "prompt_tokens": usage.input_tokens,
            "completion_tokens": usage.output_tokens,
            "total_tokens": usage.total_tokens,
        }
    })
}

pub fn build_openai_response_from_ollama(fallback_model_id: &str, upstream_body: &Value) -> Value {
    let usage = extract_token_usage(upstream_body);
    let mut response = json!( {
        "id": "resp-local-proxy",
        "object": "response",
        "model": resolve_upstream_model_id(upstream_body, fallback_model_id),
        "output": [openai_output_text_message(&extract_ollama_response_text(upstream_body))]
    });
    if let Some(response_usage) = build_openai_response_usage(&usage) {
        response["usage"] = response_usage;
    }
    response
}

pub fn build_openai_embeddings_from_ollama(upstream_body: &Value) -> Value {
    let embedding = upstream_body
        .pointer("/embeddings/0")
        .cloned()
        .or_else(|| upstream_body.get("embedding").cloned())
        .unwrap_or_else(|| Value::Array(Vec::new()));

    json!( {
        "object": "list",
        "data": [{
            "object": "embedding",
            "index": 0,
            "embedding": embedding,
        }]
    })
}

fn build_openai_tool_calls_from_ollama(payload: &Value) -> Vec<Value> {
    payload
        .pointer("/message/tool_calls")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    let name = entry
                        .pointer("/function/name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())?;
                    let arguments = entry
                        .pointer("/function/arguments")
                        .cloned()
                        .unwrap_or_else(|| Value::Object(Default::default()));
                    Some(json!( {
                        "id": format!("call-local-proxy-{index}"),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(&arguments).unwrap_or_else(|_| "{}".to_string()),
                        }
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn resolve_upstream_model_id<'a>(upstream_body: &'a Value, fallback_model_id: &'a str) -> &'a str {
    upstream_body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(fallback_model_id)
}

fn openai_output_text_message(text: &str) -> Value {
    json!({
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "output_text",
            "text": text,
            "annotations": []
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_anthropic_request_from_openai_chat, build_anthropic_request_from_openai_response,
        build_anthropic_request_payload, build_gemini_embeddings_request_payload,
        build_gemini_generate_content_payload, build_gemini_request_from_openai_chat,
        build_gemini_request_from_openai_embeddings, build_gemini_request_from_openai_response,
        build_ollama_chat_request_payload, build_ollama_embeddings_request_payload,
        build_ollama_request_from_openai_chat, build_ollama_request_from_openai_embeddings,
        build_ollama_request_from_openai_response, build_openai_chat_completion_from_anthropic,
        build_openai_chat_completion_from_gemini, build_openai_chat_completion_from_ollama,
        build_openai_embeddings_from_gemini, build_openai_embeddings_from_ollama,
        build_openai_response_from_anthropic, build_openai_response_from_gemini,
        build_openai_response_from_ollama, build_openai_response_usage,
        extract_anthropic_response_text, extract_gemini_response_text,
        extract_ollama_response_text, extract_openai_text_content, map_anthropic_stop_reason,
        map_ollama_done_reason, parse_openai_chat_conversation, parse_openai_response_conversation,
        read_request_max_tokens, resolve_request_model_id, LocalAiProxyConversationProjection,
        LocalAiProxyConversationTurn,
    };
    use serde_json::{json, Value};

    #[test]
    fn parses_openai_chat_conversation_with_system_and_messages() {
        let payload = json!({
            "messages": [
                { "role": "system", "content": "Act as a release manager." },
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "Summarize the latest build." }
                    ]
                },
                { "role": "assistant", "content": "The latest build is healthy." }
            ]
        });

        let projection = parse_openai_chat_conversation(&payload).expect("chat projection");

        assert_eq!(
            projection.system.as_deref(),
            Some("Act as a release manager.")
        );
        assert_eq!(projection.conversation.len(), 2);
        assert_eq!(projection.conversation[0].role, "user");
        assert_eq!(
            projection.conversation[0].content,
            "Summarize the latest build."
        );
        assert_eq!(projection.conversation[1].role, "assistant");
    }

    #[test]
    fn parses_openai_response_conversation_with_instruction_and_array_input() {
        let payload = json!({
            "instructions": "Respond with concise notes.",
            "input": [
                {
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "Generate release notes." }
                    ]
                },
                "Add kernel config changes."
            ]
        });

        let projection = parse_openai_response_conversation(&payload).expect("response projection");

        assert_eq!(
            projection.system.as_deref(),
            Some("Respond with concise notes.")
        );
        assert_eq!(projection.conversation.len(), 2);
        assert_eq!(projection.conversation[0].role, "user");
        assert_eq!(
            projection.conversation[0].content,
            "Generate release notes."
        );
        assert_eq!(
            projection.conversation[1].content,
            "Add kernel config changes."
        );
    }

    #[test]
    fn extracts_text_content_and_max_token_preferences() {
        let payload = json!({
            "max_output_tokens": 4096,
            "content": [
                { "text": "first line" },
                { "content": "second line" }
            ]
        });

        assert_eq!(
            extract_openai_text_content(payload.get("content").expect("content")).as_deref(),
            Some("first line\nsecond line")
        );
        assert_eq!(read_request_max_tokens(&payload, 8192), 4096);
        assert_eq!(read_request_max_tokens(&json!({}), 8192), 8192);
    }

    #[test]
    fn extracts_upstream_response_text_and_stop_reason_mappings() {
        let anthropic = json!({
            "content": [
                { "text": "anthropic line 1" },
                { "text": "anthropic line 2" }
            ]
        });
        let gemini = json!({
            "candidates": [
                {
                    "content": {
                        "parts": [
                            { "text": "gemini answer" }
                        ]
                    }
                }
            ]
        });
        let ollama = json!({
            "message": {
                "content": "ollama answer"
            }
        });

        assert_eq!(
            extract_anthropic_response_text(&anthropic),
            "anthropic line 1\nanthropic line 2"
        );
        assert_eq!(extract_gemini_response_text(&gemini), "gemini answer");
        assert_eq!(extract_ollama_response_text(&ollama), "ollama answer");
        assert_eq!(map_anthropic_stop_reason(Some("max_tokens")), "length");
        assert_eq!(map_anthropic_stop_reason(Some("end_turn")), "stop");
        assert_eq!(map_ollama_done_reason(Some("length")), Some("length"));
        assert_eq!(map_ollama_done_reason(Some("tool_calls")), Some("stop"));
        assert_eq!(map_ollama_done_reason(Some("")), None);
    }

    #[test]
    fn builds_gemini_payloads_from_shared_translation_semantics() {
        let projection = LocalAiProxyConversationProjection {
            system: Some("Keep release notes concise.".to_string()),
            conversation: vec![
                LocalAiProxyConversationTurn {
                    role: "user".to_string(),
                    content: "Summarize the latest deployment.".to_string(),
                },
                LocalAiProxyConversationTurn {
                    role: "assistant".to_string(),
                    content: "Previous deployment summary.".to_string(),
                },
            ],
        };
        let payload = json!({
            "temperature": 0.3,
            "top_p": 0.9,
            "max_output_tokens": 4096
        });

        let request = build_gemini_generate_content_payload(
            &projection.conversation,
            projection.system.as_deref(),
            &payload,
        );
        let embedding = build_gemini_embeddings_request_payload("kernel config details");

        assert_eq!(
            request.pointer("/contents/0/role").and_then(Value::as_str),
            Some("user")
        );
        assert_eq!(
            request.pointer("/contents/1/role").and_then(Value::as_str),
            Some("model")
        );
        assert_eq!(
            request
                .pointer("/systemInstruction/parts/0/text")
                .and_then(Value::as_str),
            Some("Keep release notes concise.")
        );
        assert_eq!(
            request
                .pointer("/generationConfig/maxOutputTokens")
                .and_then(Value::as_u64),
            Some(4096)
        );
        assert_eq!(
            embedding
                .pointer("/content/parts/0/text")
                .and_then(Value::as_str),
            Some("kernel config details")
        );
    }

    #[test]
    fn builds_ollama_payloads_from_shared_translation_semantics() {
        let request = build_ollama_chat_request_payload(
            "glm-4.7-flash",
            Some("System instructions".to_string()),
            vec![LocalAiProxyConversationTurn {
                role: "user".to_string(),
                content: "Describe the deployment.".to_string(),
            }],
            true,
            &json!({
                "temperature": 0.4,
                "top_p": 0.8,
                "max_tokens": 1024
            }),
            Some(json!([{ "type": "function" }])),
        );
        let embeddings = build_ollama_embeddings_request_payload(
            "nomic-embed-text",
            Value::String("embed this".to_string()),
        );

        assert_eq!(
            request.pointer("/messages/0/role").and_then(Value::as_str),
            Some("system")
        );
        assert_eq!(
            request
                .pointer("/messages/1/content")
                .and_then(Value::as_str),
            Some("Describe the deployment.")
        );
        assert_eq!(
            request
                .pointer("/options/num_predict")
                .and_then(Value::as_u64),
            Some(1024)
        );
        assert_eq!(
            request.pointer("/stream").and_then(Value::as_bool),
            Some(true)
        );
        assert!(request.pointer("/tools").is_some());
        assert_eq!(
            embeddings.pointer("/model").and_then(Value::as_str),
            Some("nomic-embed-text")
        );
        assert_eq!(
            embeddings.pointer("/input").and_then(Value::as_str),
            Some("embed this")
        );
    }

    #[test]
    fn builds_anthropic_payloads_from_shared_translation_semantics() {
        let payload = build_anthropic_request_payload(
            "claude-sonnet-4-20250514",
            Some("Keep the answer concise.".to_string()),
            vec![
                LocalAiProxyConversationTurn {
                    role: "user".to_string(),
                    content: "Summarize the deployment.".to_string(),
                },
                LocalAiProxyConversationTurn {
                    role: "developer".to_string(),
                    content: "Prefer bullet points.".to_string(),
                },
            ],
            &json!({
                "temperature": 0.2,
                "top_p": 0.95,
                "max_completion_tokens": 2048
            }),
            true,
            true,
        );

        assert_eq!(
            payload.pointer("/model").and_then(Value::as_str),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(
            payload.pointer("/system").and_then(Value::as_str),
            Some("Keep the answer concise.")
        );
        assert_eq!(
            payload.pointer("/messages/0/role").and_then(Value::as_str),
            Some("user")
        );
        assert_eq!(
            payload.pointer("/messages/1/role").and_then(Value::as_str),
            Some("user")
        );
        assert_eq!(
            payload.pointer("/max_tokens").and_then(Value::as_u64),
            Some(2048)
        );
        assert_eq!(
            payload.pointer("/temperature").and_then(Value::as_f64),
            Some(0.2)
        );
        assert_eq!(
            payload.pointer("/top_p").and_then(Value::as_f64),
            Some(0.95)
        );
        assert_eq!(
            payload.pointer("/stream").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn builds_protocol_requests_from_openai_request_wrappers() {
        let anthropic_chat = build_anthropic_request_from_openai_chat(
            "claude-sonnet",
            &json!({
                "stream": true,
                "messages": [
                    { "role": "system", "content": "Keep it short." },
                    { "role": "user", "content": "Summarize the deployment." }
                ]
            }),
        )
        .expect("anthropic chat request");
        let anthropic_response = build_anthropic_request_from_openai_response(
            "claude-sonnet",
            &json!({
                "input": [{ "role": "user", "content": "Generate release notes." }]
            }),
        )
        .expect("anthropic response request");
        let gemini_chat = build_gemini_request_from_openai_chat(&json!({
            "messages": [{ "role": "user", "content": "Summarize the deployment." }],
            "max_output_tokens": 2048
        }))
        .expect("gemini chat request");
        let gemini_response = build_gemini_request_from_openai_response(&json!({
            "instructions": "Use bullets.",
            "input": ["Generate release notes."]
        }))
        .expect("gemini response request");
        let gemini_embeddings = build_gemini_request_from_openai_embeddings(&json!({
            "input": [{ "text": "kernel config" }]
        }))
        .expect("gemini embeddings request");
        let ollama_chat = build_ollama_request_from_openai_chat(
            "qwen3",
            &json!({
                "stream": true,
                "messages": [{ "role": "user", "content": "Summarize the deployment." }],
                "tools": [{ "type": "function", "function": { "name": "web_search" } }]
            }),
        )
        .expect("ollama chat request");
        let ollama_response = build_ollama_request_from_openai_response(
            "qwen3",
            &json!({
                "input": [{ "role": "user", "content": "Generate release notes." }]
            }),
        )
        .expect("ollama response request");
        let ollama_embeddings = build_ollama_request_from_openai_embeddings(
            "nomic-embed-text",
            &json!({
                "input": {
                    "content": [{ "text": "kernel config" }]
                }
            }),
        )
        .expect("ollama embeddings request");

        assert_eq!(
            anthropic_chat.pointer("/stream").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            anthropic_response
                .pointer("/messages/0/content")
                .and_then(Value::as_str),
            Some("Generate release notes.")
        );
        assert_eq!(
            gemini_chat
                .pointer("/generationConfig/maxOutputTokens")
                .and_then(Value::as_u64),
            Some(2048)
        );
        assert_eq!(
            gemini_response
                .pointer("/systemInstruction/parts/0/text")
                .and_then(Value::as_str),
            Some("Use bullets.")
        );
        assert_eq!(
            gemini_embeddings
                .pointer("/content/parts/0/text")
                .and_then(Value::as_str),
            Some("kernel config")
        );
        assert_eq!(
            ollama_chat.pointer("/stream").and_then(Value::as_bool),
            Some(true)
        );
        assert!(ollama_chat.pointer("/tools").is_some());
        assert_eq!(
            ollama_response
                .pointer("/messages/0/content")
                .and_then(Value::as_str),
            Some("Generate release notes.")
        );
        assert_eq!(
            ollama_embeddings.pointer("/input").and_then(Value::as_str),
            Some("kernel config")
        );
    }

    #[test]
    fn builds_openai_response_usage_payload_from_shared_semantics() {
        let usage = build_openai_response_usage(&crate::response::LocalAiProxyTokenUsage {
            total_tokens: 120,
            input_tokens: 100,
            output_tokens: 20,
            cache_tokens: 7,
        })
        .expect("usage payload");

        assert_eq!(
            usage.pointer("/input_tokens").and_then(Value::as_u64),
            Some(100)
        );
        assert_eq!(
            usage.pointer("/output_tokens").and_then(Value::as_u64),
            Some(20)
        );
        assert_eq!(
            usage
                .pointer("/input_tokens_details/cached_tokens")
                .and_then(Value::as_u64),
            Some(7)
        );
    }

    #[test]
    fn builds_openai_response_adapters_from_shared_translation_semantics() {
        let anthropic = build_openai_chat_completion_from_anthropic(
            "fallback-model",
            &json!({
                "id": "msg_123",
                "model": "claude-sonnet",
                "content": [{ "text": "anthropic answer" }],
                "usage": {
                    "input_tokens": 12,
                    "output_tokens": 4
                },
                "stop_reason": "max_tokens"
            }),
        );
        let gemini = build_openai_response_from_gemini(
            "gemini-fallback",
            &json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "gemini answer" }]
                    }
                }],
                "usageMetadata": {
                    "promptTokenCount": 8,
                    "candidatesTokenCount": 2,
                    "totalTokenCount": 10
                }
            }),
        );
        let ollama = build_openai_chat_completion_from_ollama(
            "ollama-fallback",
            &json!({
                "model": "qwen3",
                "message": {
                    "content": "",
                    "tool_calls": [{
                        "function": {
                            "name": "web_search",
                            "arguments": {
                                "query": "kernel config"
                            }
                        }
                    }]
                },
                "prompt_eval_count": 11,
                "eval_count": 5,
                "done_reason": "tool_calls"
            }),
        );

        assert_eq!(
            anthropic
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str),
            Some("anthropic answer")
        );
        assert_eq!(
            anthropic
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str),
            Some("length")
        );
        assert_eq!(
            anthropic
                .pointer("/usage/total_tokens")
                .and_then(Value::as_u64),
            Some(16)
        );

        assert_eq!(
            gemini.pointer("/model").and_then(Value::as_str),
            Some("gemini-fallback")
        );
        assert_eq!(
            gemini
                .pointer("/output/0/content/0/text")
                .and_then(Value::as_str),
            Some("gemini answer")
        );
        assert_eq!(
            gemini
                .pointer("/usage/total_tokens")
                .and_then(Value::as_u64),
            Some(10)
        );

        assert_eq!(
            ollama
                .pointer("/choices/0/message/tool_calls/0/function/name")
                .and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            ollama.pointer("/choices/0/message/content"),
            Some(&Value::Null)
        );
        assert_eq!(
            ollama
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str),
            Some("tool_calls")
        );
    }

    #[test]
    fn builds_openai_embedding_and_response_variants_from_shared_semantics() {
        let gemini_embeddings = build_openai_embeddings_from_gemini(&json!({
            "embedding": {
                "values": [0.1, 0.2]
            }
        }));
        let ollama_response = build_openai_response_from_ollama(
            "ollama-fallback",
            &json!({
                "model": "qwen3",
                "message": {
                    "content": "ollama answer"
                },
                "prompt_eval_count": 9,
                "eval_count": 3
            }),
        );
        let ollama_embeddings = build_openai_embeddings_from_ollama(&json!({
            "embeddings": [[0.3, 0.4]]
        }));
        let anthropic_response = build_openai_response_from_anthropic(
            "anthropic-fallback",
            &json!({
                "content": [{ "text": "anthropic response body" }],
                "usage": {
                    "input_tokens": 6,
                    "output_tokens": 2,
                    "cache_creation_input_tokens": 1
                }
            }),
        );
        let gemini_chat = build_openai_chat_completion_from_gemini(
            "gemini-fallback",
            &json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "gemini chat answer" }]
                    }
                }]
            }),
        );

        assert_eq!(
            gemini_embeddings
                .pointer("/data/0/embedding/1")
                .and_then(Value::as_f64),
            Some(0.2)
        );
        assert_eq!(
            ollama_response
                .pointer("/usage/total_tokens")
                .and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            ollama_embeddings
                .pointer("/data/0/embedding/0")
                .and_then(Value::as_f64),
            Some(0.3)
        );
        assert_eq!(
            anthropic_response
                .pointer("/usage/input_tokens_details/cached_tokens")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            gemini_chat
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str),
            Some("gemini chat answer")
        );
    }

    #[test]
    fn resolves_request_model_id_from_payload_or_fallback() {
        assert_eq!(
            resolve_request_model_id(
                &json!({
                    "model": " gemini-2.5-pro "
                }),
                Some("fallback-model"),
            )
            .expect("payload model"),
            "gemini-2.5-pro"
        );
        assert_eq!(
            resolve_request_model_id(&json!({}), Some(" fallback-model ")).expect("fallback model"),
            "fallback-model"
        );
        assert!(resolve_request_model_id(&json!({ "model": "   " }), Some("   ")).is_err());
    }
}
