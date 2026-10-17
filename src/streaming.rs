//! Streaming support for chat completions.
//!
//! This module provides streaming functionality for `OpenAI` chat completions using
//! Server-Sent Events (SSE). The streaming API allows receiving responses incrementally
//! as they are generated, enabling real-time user experiences.
//!
//! # Example
//!
//! ```rust,no_run
//! use openai_ergonomic::Client;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?.build();
//!
//!     let builder = client.chat().user("Tell me a story");
//!     let mut stream = client.send_chat_stream(builder).await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         if let Some(content) = chunk.content() {
//!             print!("{}", content);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Interceptor Support
//!
//! Streaming responses automatically work with interceptors. When interceptors are
//! configured, they will receive hooks for:
//! - `before_request`: Called before streaming starts
//! - `on_stream_chunk`: Called for each chunk as it arrives
//! - `on_stream_end`: Called when streaming completes
//!
//! See the `langfuse_streaming` example for a complete demonstration.

use crate::interceptor::{StreamChunkContext, StreamEndContext};
use crate::{Error, Result};
use bytes::Bytes;
use futures::stream::Stream;
use futures::StreamExt;
use openai_client_base::models::{
    ChatCompletionStreamResponseDelta, CreateChatCompletionStreamResponse,
};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

/// Type alias for a boxed stream of chat completion chunks.
///
/// This allows returning either `ChatCompletionStream` or `InterceptedStream`
/// from the same method based on whether interceptors are enabled.
pub type BoxedChatStream = Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>;

/// A streaming chunk from a chat completion response.
///
/// Each chunk represents a delta update from the model as it generates the response.
#[derive(Debug, Clone)]
pub struct ChatCompletionChunk {
    /// The underlying stream response
    response: CreateChatCompletionStreamResponse,
}

impl ChatCompletionChunk {
    /// Create a new chunk from a stream response.
    #[must_use]
    pub fn new(response: CreateChatCompletionStreamResponse) -> Self {
        Self { response }
    }

    /// Get the content delta from this chunk, if any.
    ///
    /// Returns the text content that was generated in this chunk.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.content.as_ref().and_then(|c| c.as_deref()))
    }

    /// Get the role from this chunk, if any.
    ///
    /// This is typically only present in the first chunk.
    #[must_use]
    pub fn role(&self) -> Option<&str> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.role.as_ref())
            .map(|role| match role {
                openai_client_base::models::chat_completion_stream_response_delta::Role::System => {
                    "system"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::User => {
                    "user"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Assistant => {
                    "assistant"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Tool => {
                    "tool"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Developer => {
                    "developer"
                }
            })
    }

    /// Get tool calls from this chunk, if any.
    #[must_use]
    pub fn tool_calls(
        &self,
    ) -> Option<&Vec<openai_client_base::models::ChatCompletionMessageToolCallChunk>> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.tool_calls.as_ref())
    }

    /// Get the finish reason, if any.
    ///
    /// This indicates why the generation stopped and is only present in the last chunk.
    #[must_use]
    pub fn finish_reason(&self) -> Option<&str> {
        self.response.choices.first().and_then(|choice| choice.finish_reason.as_ref()).map(|reason| {
            match reason {
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::Stop => "stop",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::Length => "length",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::ToolCalls => "tool_calls",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::ContentFilter => "content_filter",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::FunctionCall => "function_call",
            }
        })
    }

    /// Check if this is the last chunk in the stream.
    #[must_use]
    pub fn is_final(&self) -> bool {
        self.finish_reason().is_some()
    }

    /// Get the underlying raw response.
    #[must_use]
    pub fn raw_response(&self) -> &CreateChatCompletionStreamResponse {
        &self.response
    }

    /// Get the delta object directly.
    #[must_use]
    pub fn delta(&self) -> Option<&ChatCompletionStreamResponseDelta> {
        self.response
            .choices
            .first()
            .map(|choice| choice.delta.as_ref())
    }
}

/// A stream of chat completion chunks.
///
/// This stream yields `ChatCompletionChunk` items as the model generates the response.
/// The stream ends when the model finishes generating or encounters an error.
pub struct ChatCompletionStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
}

impl ChatCompletionStream {
    /// Create a new stream from a byte stream response.
    ///
    /// Parses Server-Sent Events (SSE) format and yields chat completion chunks.
    pub fn new(response: reqwest::Response) -> Self {
        let byte_stream = response.bytes_stream();
        let stream = parse_sse_stream(byte_stream);
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Collect all remaining content from the stream into a single string.
    ///
    /// This is a convenience method that reads all chunks and concatenates their content.
    pub async fn collect_content(mut self) -> Result<String> {
        let mut content = String::new();
        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            if let Some(text) = chunk.content() {
                content.push_str(text);
            }
        }
        Ok(content)
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Wrapper for a stream that calls interceptor hooks.
///
/// This wrapper intercepts chunks as they flow through the stream and calls
/// the appropriate interceptor methods for observability and telemetry.
///
/// This is the primary stream type returned by streaming methods when
/// interceptors are enabled. It provides the same interface as `ChatCompletionStream`.
pub struct InterceptedStream<T = ()> {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
    interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
    operation: String,
    model: String,
    request_json: String,
    state: Arc<T>,
    chunk_index: usize,
    start_time: Instant,
    total_input_tokens: Option<i64>,
    total_output_tokens: Option<i64>,
}

impl<T: Send + Sync + 'static> InterceptedStream<T> {
    /// Create a new intercepted stream.
    pub fn new(
        inner: ChatCompletionStream,
        interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
        operation: String,
        model: String,
        request_json: String,
        state: T,
    ) -> Self {
        Self {
            inner: inner.inner,
            interceptors,
            operation,
            model,
            request_json,
            state: Arc::new(state),
            chunk_index: 0,
            start_time: Instant::now(),
            total_input_tokens: None,
            total_output_tokens: None,
        }
    }

    /// Collect all remaining content from the stream into a single string.
    ///
    /// This is a convenience method that reads all chunks and concatenates their content.
    /// Interceptor hooks are still called for each chunk.
    pub async fn collect_content(mut self) -> Result<String> {
        let mut content = String::new();
        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            if let Some(text) = chunk.content() {
                content.push_str(text);
            }
        }
        Ok(content)
    }
}

impl<T: Send + Sync + 'static> Stream for InterceptedStream<T> {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                // Serialize chunk for interceptor
                let chunk_json = serde_json::to_string(chunk.raw_response())
                    .unwrap_or_else(|_| "{}".to_string());

                // Update token counts if available
                if let Some(Some(usage)) = &chunk.raw_response().usage {
                    this.total_input_tokens = Some(i64::from(usage.prompt_tokens));
                    this.total_output_tokens = Some(i64::from(usage.completion_tokens));
                }

                // Call on_stream_chunk hook (spawn to avoid blocking)
                let interceptors = Arc::clone(&this.interceptors);
                let operation = this.operation.clone();
                let model = this.model.clone();
                let request_json = this.request_json.clone();
                let chunk_index = this.chunk_index;
                let state = Arc::clone(&this.state);

                tokio::spawn(async move {
                    let ctx = StreamChunkContext {
                        operation: &operation,
                        model: &model,
                        request_json: &request_json,
                        chunk_json: &chunk_json,
                        chunk_index,
                        state: &*state,
                    };
                    let _ = interceptors.on_stream_chunk(&ctx).await;
                });

                this.chunk_index += 1;
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => {
                // Note: We skip error interceptor for streaming errors due to
                // lifetime constraints with Error type. The error is still
                // propagated to the caller.
                // TODO: Consider adding error string serialization support

                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                // Stream ended - call on_stream_end hook
                let interceptors = Arc::clone(&this.interceptors);
                let operation = this.operation.clone();
                let model = this.model.clone();
                let request_json = this.request_json.clone();
                let chunk_index = this.chunk_index;
                let duration = this.start_time.elapsed();
                let input_tokens = this.total_input_tokens;
                let output_tokens = this.total_output_tokens;
                let state = Arc::clone(&this.state);

                tokio::spawn(async move {
                    let ctx = StreamEndContext {
                        operation: &operation,
                        model: &model,
                        request_json: &request_json,
                        total_chunks: chunk_index,
                        duration,
                        input_tokens,
                        output_tokens,
                        state: &*state,
                    };
                    let _ = interceptors.on_stream_end(&ctx).await;
                });

                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Parse an SSE stream into chat completion chunks.
///
/// Events are dispatched only when a blank line completes the event. Any partial line or
/// event still buffered at end of stream is discarded, as required by the SSE format.
fn parse_sse_stream<S>(byte_stream: S) -> impl Stream<Item = Result<ChatCompletionChunk>> + Send
where
    S: Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
{
    struct State<S> {
        byte_stream: Pin<Box<S>>,
        buffer: Vec<u8>,
        data_lines: Vec<String>,
        ready: VecDeque<Result<ChatCompletionChunk>>,
        terminated: bool,
    }

    futures::stream::unfold(
        State {
            byte_stream: Box::pin(byte_stream),
            buffer: Vec::new(),
            data_lines: Vec::new(),
            ready: VecDeque::new(),
            terminated: false,
        },
        |mut state| async move {
            loop {
                if let Some(item) = state.ready.pop_front() {
                    return Some((item, state));
                }
                if state.terminated {
                    return None;
                }

                match state.byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        state.buffer.extend_from_slice(&bytes);
                        while let Some(line) = take_sse_line(&mut state.buffer, false) {
                            if process_sse_line(&line, &mut state.data_lines, &mut state.ready) {
                                state.terminated = true;
                                break;
                            }
                        }
                    }
                    Some(Err(error)) => {
                        state.ready.push_back(Err(Error::StreamConnection {
                            message: format!("Stream connection error: {error}"),
                        }));
                        state.terminated = true;
                    }
                    None => {
                        // A final CR is a complete SSE line ending. Other unterminated data
                        // remains buffered and, like an incomplete event, is discarded.
                        while let Some(line) = take_sse_line(&mut state.buffer, true) {
                            if process_sse_line(&line, &mut state.data_lines, &mut state.ready) {
                                state.terminated = true;
                                break;
                            }
                        }
                        state.buffer.clear();
                        state.data_lines.clear();
                        state.terminated = true;
                    }
                }
            }
        },
    )
    .fuse()
}

/// Remove one complete SSE line, retaining a final CR until the next byte can disambiguate CRLF.
fn take_sse_line(buffer: &mut Vec<u8>, end_of_stream: bool) -> Option<Vec<u8>> {
    for (index, byte) in buffer.iter().enumerate() {
        match byte {
            b'\n' => {
                let line = buffer[..index].to_vec();
                buffer.drain(..=index);
                return Some(line);
            }
            b'\r' => {
                if index + 1 == buffer.len() && !end_of_stream {
                    return None;
                }
                let line = buffer[..index].to_vec();
                let terminator_len = usize::from(buffer.get(index + 1) == Some(&b'\n')) + 1;
                buffer.drain(..index + terminator_len);
                return Some(line);
            }
            _ => {}
        }
    }
    None
}

/// Process one decoded SSE line. Returns true when the stream reaches DONE or a parse error.
fn process_sse_line(
    line_bytes: &[u8],
    data_lines: &mut Vec<String>,
    ready: &mut VecDeque<Result<ChatCompletionChunk>>,
) -> bool {
    let line = String::from_utf8_lossy(line_bytes).trim().to_string();
    if line.is_empty() {
        if data_lines.is_empty() {
            return false;
        }

        let data = std::mem::take(data_lines).join("\n");
        if data.is_empty() {
            return false;
        }
        if data == "[DONE]" {
            return true;
        }

        match parse_sse_line(&format!("data: {data}")) {
            Ok(Some(chunk)) => ready.push_back(Ok(chunk)),
            Ok(None) => {}
            Err(error) => {
                ready.push_back(Err(error));
                return true;
            }
        }
        return false;
    }

    if line.starts_with(':') {
        return false;
    }
    if let Some(data) = line.strip_prefix("data:") {
        data_lines.push(data.trim_start().to_string());
    }
    false
}

/// Parse a single SSE line into a chat completion chunk.
fn parse_sse_line(line: &str) -> Result<Option<ChatCompletionChunk>> {
    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return Ok(None);
    }

    // Handle SSE format: "data: {json}"
    if let Some(data) = line.strip_prefix("data:").map(str::trim) {
        // Check for [DONE] marker
        if data == "[DONE]" {
            return Ok(None);
        }

        let response: CreateChatCompletionStreamResponse =
            serde_json::from_str(data).map_err(|e| Error::StreamParsing {
                message: format!("Failed to deserialize chunk: {e}"),
                chunk: data.to_string(),
            })?;

        return Ok(Some(ChatCompletionChunk::new(response)));
    }

    // Ignore other SSE fields (event:, id:, retry:)
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line_with_content() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}"#;

        let result = parse_sse_line(line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.content(), Some("Hello"));
        assert_eq!(chunk.role(), Some("assistant"));
        assert_eq!(chunk.finish_reason(), None);
        assert!(!chunk.is_final());
        assert!(!crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_parse_sse_line_with_finish_reason() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":null}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), Some("stop"));
        assert!(chunk.is_final());
        assert!(crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_usage_only_chunk_is_not_a_finished_choice() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7}}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), None);
        assert!(!crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_parse_sse_line_done_marker() {
        let line = "data: [DONE]";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_empty() {
        let line = "";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_comment() {
        let line = ": this is a comment";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_handles_fragmented_crlf_and_usage_only_events() {
        let first_event = format!(
            "data: {{\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":{}}},\"finish_reason\":null}}]}}\r\n\r\n",
            serde_json::to_string("Hello 🦀").unwrap()
        );
        let crab = first_event
            .as_bytes()
            .windows(4)
            .position(|window| window == "🦀".as_bytes())
            .unwrap();
        let crlf = first_event
            .as_bytes()
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let first_bytes = first_event.as_bytes();
        let fragmented = vec![
            Ok(Bytes::copy_from_slice(&first_bytes[..5]),),
            Ok(Bytes::copy_from_slice(&first_bytes[5..crab + 2]),),
            Ok(Bytes::copy_from_slice(&first_bytes[(crab + 2)..=crlf]),),
            Ok(Bytes::copy_from_slice(&first_bytes[crlf + 1..crlf + 2]),),
            Ok(Bytes::copy_from_slice(&first_bytes[crlf + 2..]),),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" done\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\r\n\r\ndata: [DONE]\r\n\r\n",
            )),
        ];

        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(fragmented)));
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let usage = stream.next().await.unwrap().unwrap();

        assert_eq!(first.content(), Some("Hello 🦀"));
        assert_eq!(second.content(), Some(" done"));
        assert_eq!(usage.raw_response().choices.as_slice(), &[]);
        assert_eq!(
            usage
                .raw_response()
                .usage
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .total_tokens,
            7
        );
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_reports_malformed_json() {
        let bytes = Bytes::from("data: {not-json}\n\n");
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert!(matches!(
            stream.next().await.unwrap(),
            Err(Error::StreamParsing { .. })
        ));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_delivers_valid_event_before_parse_error() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"kept\"},\"finish_reason\":null}]}\n\ndata: {not-json}\n\n",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("kept")
        );
        assert!(matches!(
            stream.next().await.unwrap(),
            Err(Error::StreamParsing { .. })
        ));
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_stops_at_done() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"before\"},\"finish_reason\":null}]}\n\ndata: [DONE]\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"after\"},\"finish_reason\":null}]}\n\n",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("before")
        );
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_discards_incomplete_trailing_event() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"complete\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"missing blank line\"},\"finish_reason\":null}]}\ndata: {partial",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("complete")
        );
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }
}
