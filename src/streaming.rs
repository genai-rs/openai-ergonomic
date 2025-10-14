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
//!     let mut stream = client
//!         .chat()
//!         .user("Tell me a story")
//!         .send_stream()
//!         .await?;
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

use crate::{Error, Result};
use bytes::Bytes;
use futures::stream::Stream;
use futures::StreamExt;
use openai_client_base::models::{
    ChatCompletionStreamResponseDelta, CreateChatCompletionStreamResponse,
};
use std::pin::Pin;
use std::task::{Context, Poll};

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
        self.response.choices.first().map(|choice| {
            match &choice.finish_reason {
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

/// Parse an SSE (Server-Sent Events) stream into chat completion chunks.
fn parse_sse_stream(
    byte_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
) -> impl Stream<Item = Result<ChatCompletionChunk>> + Send {
    let mut buffer = Vec::new();

    byte_stream
        .map(move |result| {
            let bytes = result.map_err(|e| Error::StreamConnection {
                message: format!("Stream connection error: {e}"),
            })?;

            buffer.extend_from_slice(&bytes);

            // Process complete lines from buffer
            let mut chunks = Vec::new();
            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                let line = String::from_utf8_lossy(&line_bytes).trim().to_string();

                if let Some(chunk) = parse_sse_line(&line)? {
                    chunks.push(chunk);
                }
            }

            Ok(chunks)
        })
        .flat_map(|result: Result<Vec<ChatCompletionChunk>>| match result {
            Ok(chunks) => futures::stream::iter(chunks.into_iter().map(Ok)).left_stream(),
            Err(e) => futures::stream::once(async move { Err(e) }).right_stream(),
        })
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

        // Parse JSON data - use Value first to handle null finish_reason
        let mut value: serde_json::Value =
            serde_json::from_str(data).map_err(|e| Error::StreamParsing {
                message: format!("Failed to parse chunk JSON: {e}"),
                chunk: data.to_string(),
            })?;

        // Workaround: Remove finish_reason if it's null, since base library
        // doesn't properly handle Option<FinishReason>
        if let Some(choices) = value.get_mut("choices").and_then(|c| c.as_array_mut()) {
            for choice in choices {
                if let Some(finish_reason) = choice.get("finish_reason") {
                    if finish_reason.is_null() {
                        // Set to default value instead of null
                        choice["finish_reason"] = serde_json::json!("stop");
                    }
                }
            }
        }

        let response: CreateChatCompletionStreamResponse =
            serde_json::from_value(value).map_err(|e| Error::StreamParsing {
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
}
