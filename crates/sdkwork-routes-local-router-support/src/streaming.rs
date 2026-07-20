use axum::body::Body;
use axum::http::header;
use axum::response::Response;
use bytes::Bytes;
use futures_core::Stream;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use sdkwork_lr_core::{Protocol, StreamUsageAccumulator, TokenUsage};
use sdkwork_lr_plugin::ApiSurface;
use sdkwork_lr_transform::streaming::{SseEvent, StreamTransformState};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_SSE_BUFFER_SIZE: usize = 1024 * 1024;
const STREAM_TIMEOUT_SECS: u64 = 300;

pub fn wrap_streaming_response(
    upstream_response: hyper::Response<Incoming>,
    source: Protocol,
    target: Protocol,
    source_surface: ApiSurface,
    target_surface: ApiSurface,
    model: String,
    request_id: String,
    on_complete: impl FnOnce(Option<TokenUsage>) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + 'static,
) -> Response {
    let (parts, body) = upstream_response.into_parts();
    wrap_streaming_parts(
        parts,
        body.into_data_stream(),
        source,
        target,
        source_surface,
        target_surface,
        model,
        request_id,
        on_complete,
    )
}

fn wrap_streaming_parts<S, E>(
    parts: axum::http::response::Parts,
    mut data_stream: S,
    source: Protocol,
    target: Protocol,
    source_surface: ApiSurface,
    target_surface: ApiSurface,
    model: String,
    request_id: String,
    on_complete: impl FnOnce(Option<TokenUsage>) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + 'static,
) -> Response
where
    S: Stream<Item = Result<Bytes, E>> + Send + Unpin + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);

    let request_id_for_log = request_id.clone();
    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut state = StreamTransformState::new_for_surfaces(
            source,
            target,
            source_surface,
            target_surface,
            &model,
        );
        let mut usage_accumulator = StreamUsageAccumulator::default();
        let mut on_complete = Some(on_complete);

        use futures_util::StreamExt;

        let timeout = Duration::from_secs(STREAM_TIMEOUT_SECS);

        loop {
            let chunk_result = tokio::time::timeout(timeout, data_stream.next()).await;

            match chunk_result {
                Ok(Some(Ok(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    if buffer.len() + chunk_str.len() > MAX_SSE_BUFFER_SIZE {
                        let last_end = buffer.rfind("\n\n").or_else(|| buffer.rfind("\r\n\r\n"));
                        if let Some(pos) = last_end {
                            let sep_len = if buffer.as_bytes().get(pos + 1) == Some(&b'\r') {
                                4
                            } else {
                                2
                            };
                            buffer.truncate(pos + sep_len);
                        } else {
                            buffer.clear();
                        }
                        tracing::warn!(request_id = %request_id_for_log, "SSE buffer exceeded limit, trimmed");
                    }

                    buffer.push_str(&chunk_str);

                    while let Some(event) = SseEvent::parse_from_buffer(&mut buffer) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                            usage_accumulator.observe_event_data(source, &parsed);
                        }
                        let transformed = state.transform(event);
                        for output in transformed {
                            let bytes = Bytes::from(output.serialize());
                            if tx.send(Ok(bytes)).await.is_err() {
                                finalize_stream_usage(&mut on_complete, &usage_accumulator).await;
                                return;
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let _ = tx
                        .send(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )))
                        .await;
                    finalize_stream_usage(&mut on_complete, &usage_accumulator).await;
                    return;
                }
                Ok(None) => {
                    let remaining = state.flush();
                    for output in remaining {
                        let bytes = Bytes::from(output.serialize());
                        if tx.send(Ok(bytes)).await.is_err() {
                            finalize_stream_usage(&mut on_complete, &usage_accumulator).await;
                            return;
                        }
                    }
                    finalize_stream_usage(&mut on_complete, &usage_accumulator).await;
                    return;
                }
                Err(_) => {
                    tracing::warn!(request_id = %request_id_for_log, "stream timeout after {}s, closing", STREAM_TIMEOUT_SECS);
                    let _ = tx
                        .send(Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("stream timeout after {}s", STREAM_TIMEOUT_SECS),
                        )))
                        .await;
                    finalize_stream_usage(&mut on_complete, &usage_accumulator).await;
                    return;
                }
            }
        }
    });

    let new_body = Body::from_stream(SseReceiverStream { rx });

    let mut response = Response::new(new_body);
    *response.status_mut() = parts.status;

    let connection_header_names = collect_connection_headers(&parts.headers);
    for (name, value) in parts.headers.iter() {
        if should_forward_header(name, &connection_header_names) {
            response.headers_mut().append(name, value.clone());
        }
    }

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        header::CONNECTION,
        header::HeaderValue::from_static("keep-alive"),
    );

    if let Ok(val) = header::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", val);
    }

    response
}

async fn finalize_stream_usage<F>(
    on_complete: &mut Option<F>,
    usage_accumulator: &StreamUsageAccumulator,
) where
    F: FnOnce(Option<TokenUsage>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static,
{
    if let Some(on_complete) = on_complete.take() {
        on_complete(usage_accumulator.token_usage()).await;
    }
}

fn collect_connection_headers(
    headers: &axum::http::HeaderMap,
) -> std::collections::HashSet<String> {
    headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

fn should_forward_header(
    name: &axum::http::header::HeaderName,
    connection_headers: &std::collections::HashSet<String>,
) -> bool {
    let name_str = name.as_str();
    !matches!(
        name_str,
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    ) && !connection_headers.contains(name_str)
}

struct SseReceiverStream {
    rx: mpsc::Receiver<Result<Bytes, std::io::Error>>,
}

impl Stream for SseReceiverStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn streaming_wrapper_reports_usage_to_completion_callback() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"id\":\"chatcmpl_1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}],\"usage\":null}\n\n\
                  data: {\"id\":\"chatcmpl_1\",\"object\":\"chat.completion.chunk\",\"choices\":[],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":3,\"total_tokens\":11}}\n\n\
                  data: [DONE]\n\n",
            ),
        )]);
        let captured_usage: Arc<Mutex<Option<TokenUsage>>> = Arc::new(Mutex::new(None));
        let captured_usage_for_callback = captured_usage.clone();

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Openai,
            ApiSurface::OpenAiChatCompletions,
            ApiSurface::OpenAiChatCompletions,
            "gpt-5".to_owned(),
            "req_stream_usage".to_owned(),
            move |usage| {
                Box::pin(async move {
                    *captured_usage_for_callback.lock().await = usage;
                })
            },
        );

        let _body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();

        let usage = captured_usage
            .lock()
            .await
            .clone()
            .expect("usage should be reported");
        assert_eq!(usage.prompt_tokens, Some(8));
        assert_eq!(usage.completion_tokens, Some(3));
        assert_eq!(usage.total_tokens, Some(11));
    }

    #[tokio::test]
    async fn streaming_wrapper_uses_api_surfaces_for_codex_responses_from_claude() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"event: message_start\n\
                  data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_abc\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
                  event: content_block_start\n\
                  data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
                  event: content_block_delta\n\
                  data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
                  event: message_delta\n\
                  data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":3}}\n\n\
                  event: message_stop\n\
                  data: {\"type\":\"message_stop\"}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Anthropic,
            Protocol::Openai,
            ApiSurface::AnthropicMessages,
            ApiSurface::OpenAiResponses,
            "claude-sonnet-4-20250514".to_owned(),
            "req_codex_claude_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: response.created"));
        assert!(body.contains("event: response.output_text.delta"));
        assert!(body.contains("event: response.completed"));
        assert!(!body.contains("chat.completion.chunk"));
    }

    #[tokio::test]
    async fn streaming_wrapper_uses_api_surfaces_for_codex_responses_from_gemini() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":2}}\n\n\
                  data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":3,\"totalTokenCount\":11}}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Google,
            Protocol::Openai,
            ApiSurface::GeminiGenerateContent,
            ApiSurface::OpenAiResponses,
            "gemini-2.5-pro".to_owned(),
            "req_codex_gemini_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: response.created"));
        assert!(body.contains("event: response.output_text.delta"));
        assert!(body.contains("event: response.completed"));
        assert!(!body.contains("chat.completion.chunk"));
    }

    #[tokio::test]
    async fn streaming_wrapper_converts_chat_chunks_to_codex_responses_surface() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"id\":\"chatcmpl_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n\
                  data: {\"id\":\"chatcmpl_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n\
                  data: {\"id\":\"chatcmpl_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":3,\"total_tokens\":11}}\n\n\
                  data: [DONE]\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Openai,
            ApiSurface::OpenAiChatCompletions,
            ApiSurface::OpenAiResponses,
            "gpt-4o".to_owned(),
            "req_codex_chat_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: response.created"));
        assert!(body.contains("event: response.output_text.delta"));
        assert!(body.contains("\"delta\":\"ok\""));
        assert!(body.contains("event: response.completed"));
        assert!(body.contains("\"input_tokens\":8"));
        assert!(body.contains("\"output_tokens\":3"));
        assert!(!body.contains("chat.completion.chunk"));
    }

    #[tokio::test]
    async fn streaming_wrapper_converts_chat_chunks_to_claude_messages_with_final_usage() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"id\":\"chatcmpl_usage\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n\
                  data: {\"id\":\"chatcmpl_usage\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
                  data: {\"id\":\"chatcmpl_usage\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[],\"usage\":{\"prompt_tokens\":13,\"completion_tokens\":5,\"total_tokens\":18}}\n\n\
                  data: [DONE]\n\n",
            ),
        )]);
        let captured_usage: Arc<Mutex<Option<TokenUsage>>> = Arc::new(Mutex::new(None));
        let captured_usage_for_callback = captured_usage.clone();

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Anthropic,
            ApiSurface::OpenAiChatCompletions,
            ApiSurface::AnthropicMessages,
            "gpt-5".to_owned(),
            "req_chat_to_claude_usage_stream".to_owned(),
            move |usage| {
                Box::pin(async move {
                    *captured_usage_for_callback.lock().await = usage;
                })
            },
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: message_start"));
        assert!(body.contains("event: content_block_delta"));
        assert!(body.contains("\"text\":\"ok\""));
        assert!(body.contains("event: message_delta"));
        assert!(body.contains("\"stop_reason\":\"end_turn\""));
        assert!(body.contains("\"output_tokens\":5"));
        assert!(body.contains("event: message_stop"));
        assert!(!body.contains("chat.completion.chunk"));

        let usage = captured_usage
            .lock()
            .await
            .clone()
            .expect("usage should be reported");
        assert_eq!(usage.prompt_tokens, Some(13));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(18));
    }

    #[tokio::test]
    async fn streaming_wrapper_returns_gemini_surface_for_chat_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"id\":\"chatcmpl_gemini\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n\
                  data: {\"id\":\"chatcmpl_gemini\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
                  data: {\"id\":\"chatcmpl_gemini\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5\",\"choices\":[],\"usage\":{\"prompt_tokens\":13,\"completion_tokens\":5,\"total_tokens\":18}}\n\n\
                  data: [DONE]\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Google,
            ApiSurface::OpenAiChatCompletions,
            ApiSurface::GeminiGenerateContent,
            "gemini-2.5-pro".to_owned(),
            "req_gemini_chat_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("\"text\":\"ok\""));
        assert!(body.contains("\"finishReason\":\"STOP\""));
        assert!(body.contains("\"usageMetadata\""));
        assert!(body.contains("\"promptTokenCount\":13"));
        assert!(body.contains("\"candidatesTokenCount\":5"));
        assert!(body.contains("\"totalTokenCount\":18"));
        assert!(!body.contains("chat.completion.chunk"));
        assert!(!body.contains("data: [DONE]"));
        assert!(!body.contains("event: response."));
        assert!(!body.contains("event: message_"));
    }

    #[tokio::test]
    async fn streaming_wrapper_returns_gemini_surface_for_openai_responses_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"event: response.created\n\
                  data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_gemini\",\"object\":\"response\",\"status\":\"in_progress\",\"model\":\"gpt-5.5\",\"output\":[],\"usage\":{\"input_tokens\":7,\"output_tokens\":0,\"total_tokens\":7}}}\n\n\
                  event: response.output_text.delta\n\
                  data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"ok\"}\n\n\
                  event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini\",\"object\":\"response\",\"status\":\"completed\",\"model\":\"gpt-5.5\",\"output\":[],\"usage\":{\"input_tokens\":7,\"output_tokens\":3,\"total_tokens\":10}}}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Google,
            ApiSurface::OpenAiResponses,
            ApiSurface::GeminiGenerateContent,
            "gemini-2.5-pro".to_owned(),
            "req_gemini_responses_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("\"text\":\"ok\""));
        assert!(body.contains("\"finishReason\":\"STOP\""));
        assert!(body.contains("\"promptTokenCount\":7"));
        assert!(body.contains("\"candidatesTokenCount\":3"));
        assert!(body.contains("\"totalTokenCount\":10"));
        assert!(!body.contains("event: response."));
        assert!(!body.contains("chat.completion.chunk"));
        assert!(!body.contains("data: [DONE]"));
    }

    #[tokio::test]
    async fn streaming_wrapper_returns_gemini_surface_for_claude_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"event: message_start\n\
                  data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_gemini\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":9,\"output_tokens\":0}}}\n\n\
                  event: content_block_delta\n\
                  data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n\
                  event: message_delta\n\
                  data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n\n\
                  event: message_stop\n\
                  data: {\"type\":\"message_stop\"}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Anthropic,
            Protocol::Google,
            ApiSurface::AnthropicMessages,
            ApiSurface::GeminiGenerateContent,
            "gemini-2.5-pro".to_owned(),
            "req_gemini_claude_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("\"text\":\"hi\""));
        assert!(body.contains("\"finishReason\":\"STOP\""));
        assert!(body.contains("\"promptTokenCount\":9"));
        assert!(body.contains("\"candidatesTokenCount\":2"));
        assert!(body.contains("\"totalTokenCount\":11"));
        assert!(!body.contains("event: message_"));
        assert!(!body.contains("event: content_block_"));
        assert!(!body.contains("chat.completion.chunk"));
        assert!(!body.contains("data: [DONE]"));
    }

    #[tokio::test]
    async fn streaming_wrapper_keeps_chat_completions_surface_for_claude_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"event: message_start\n\
                  data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_abc\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
                  event: content_block_delta\n\
                  data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
                  event: message_delta\n\
                  data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":3}}\n\n\
                  event: message_stop\n\
                  data: {\"type\":\"message_stop\"}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Anthropic,
            Protocol::Openai,
            ApiSurface::AnthropicMessages,
            ApiSurface::OpenAiChatCompletions,
            "claude-sonnet-4-20250514".to_owned(),
            "req_chat_claude_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("\"object\":\"chat.completion.chunk\""));
        assert!(body.contains("\"delta\":{\"role\":\"assistant\"}"));
        assert!(body.contains("\"content\":\"ok\""));
        assert!(body.contains("\"finish_reason\":\"stop\""));
        assert!(body.contains("data: [DONE]"));
        assert!(!body.contains("event: response."));
    }

    #[tokio::test]
    async fn streaming_wrapper_keeps_chat_completions_surface_for_gemini_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":2}}\n\n\
                  data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":3,\"totalTokenCount\":11}}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Google,
            Protocol::Openai,
            ApiSurface::GeminiGenerateContent,
            ApiSurface::OpenAiChatCompletions,
            "gemini-2.5-pro".to_owned(),
            "req_chat_gemini_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("\"object\":\"chat.completion.chunk\""));
        assert!(body.contains("\"delta\":{\"role\":\"assistant\"}"));
        assert!(body.contains("\"content\":\"ok\""));
        assert!(body.contains("\"finish_reason\":\"stop\""));
        assert!(body.contains("data: [DONE]"));
        assert!(!body.contains("event: response."));
    }

    #[tokio::test]
    async fn streaming_wrapper_returns_claude_messages_surface_for_gemini_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":2}}\n\n\
                  data: {\"candidates\":[{\"index\":0,\"content\":{\"role\":\"model\",\"parts\":[]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":8,\"candidatesTokenCount\":3,\"totalTokenCount\":11}}\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Google,
            Protocol::Anthropic,
            ApiSurface::GeminiGenerateContent,
            ApiSurface::AnthropicMessages,
            "gemini-2.5-pro".to_owned(),
            "req_claude_gemini_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: message_start"));
        assert!(body.contains("event: content_block_delta"));
        assert!(body.contains("\"text_delta\""));
        assert!(body.contains("\"text\":\"ok\""));
        assert!(body.contains("event: message_delta"));
        assert!(body.contains("\"stop_reason\":\"end_turn\""));
        assert!(body.contains("event: message_stop"));
        assert!(!body.contains("chat.completion.chunk"));
        assert!(!body.contains("event: response."));
        assert!(!body.contains("data: [DONE]"));
    }

    #[tokio::test]
    async fn streaming_wrapper_returns_claude_messages_surface_for_openai_responses_upstream() {
        let parts = axum::http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let upstream_events = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(
                b"event: response.created\n\
                  data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_abc\",\"object\":\"response\",\"status\":\"in_progress\",\"model\":\"gpt-5.5\",\"output\":[],\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"total_tokens\":8}}}\n\n\
                  event: response.output_text.delta\n\
                  data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"ok\"}\n\n\
                  event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_abc\",\"object\":\"response\",\"status\":\"completed\",\"model\":\"gpt-5.5\",\"output\":[{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}],\"output_text\":\"ok\",\"usage\":{\"input_tokens\":8,\"output_tokens\":3,\"total_tokens\":11}}}\n\n\
                  data: [DONE]\n\n",
            ),
        )]);

        let response = wrap_streaming_parts(
            parts,
            upstream_events,
            Protocol::Openai,
            Protocol::Anthropic,
            ApiSurface::OpenAiResponses,
            ApiSurface::AnthropicMessages,
            "gpt-5.5".to_owned(),
            "req_claude_openai_responses_stream".to_owned(),
            |_| Box::pin(async move {}),
        );

        let body = response
            .into_body()
            .collect()
            .await
            .expect("stream body")
            .to_bytes();
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("event: message_start"));
        assert!(body.contains("event: content_block_delta"));
        assert!(body.contains("\"text_delta\""));
        assert!(body.contains("\"text\":\"ok\""));
        assert!(body.contains("event: message_delta"));
        assert!(body.contains("\"stop_reason\":\"end_turn\""));
        assert!(body.contains("\"output_tokens\":3"));
        assert!(body.contains("event: message_stop"));
        assert!(!body.contains("chat.completion.chunk"));
        assert!(!body.contains("event: response."));
        assert!(!body.contains("data: [DONE]"));
    }
}
