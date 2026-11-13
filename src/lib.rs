#![doc = include_str!("../README.md")]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::use_self)]
#![allow(clippy::io_other_error)]
#![allow(async_fn_in_trait)]
// On WebAssembly futures, clients and their configuration are not `Send`: the JavaScript host's
// cannot leave its thread.
#![cfg_attr(
    target_family = "wasm",
    allow(clippy::future_not_send, clippy::arc_with_non_send_sync)
)]

//! # openai-ergonomic
//!
//! An ergonomic Rust wrapper for the `OpenAI` API, providing type-safe builder patterns
//! and async/await support for all `OpenAI` endpoints.
//!
//! ## Features
//!
//! - **Type-safe builders** - Use builder patterns with compile-time validation
//! - **Async/await support** - Built on tokio and a tower-based HTTP client
//!   ([`timvw_rust_runtime::Client`], reqwest underneath) whose layers you can customise
//! - **Streaming responses** - First-class support for real-time streaming
//! - **Comprehensive coverage** - Support for all `OpenAI` API endpoints
//! - **Error handling** - Structured error types for robust applications
//! - **Testing support** - Mock-friendly design for unit testing
//! - **Bring your own types** - [`Client::post_json`], [`Client::get_json`], [`Client::send_json`]
//!   and [`Client::send_json_stream`] send and decode types of your own
//!
//! The [basic usage](#basic-usage) and [Langfuse setup](#langfuse-setup)
//! examples above are compiled directly from the README as `no_run` doctests.
//!
//! ## Streaming Example
//!
//! ```rust,no_run
//! use openai_ergonomic::Client;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?.build();
//!
//!     // Stream chat completions
//!     let builder = client
//!         .chat()
//!         .user("Tell me a story");
//!
//!     let mut stream = client.send_chat_stream(builder).await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         if let Some(content) = chunk.content() {
//!             print!("{}", content);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use openai_ergonomic::{Client, Error};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::from_env().expect("API key required").build();
//!
//!     match client.send_chat(client.chat_simple("Hello")).await {
//!         Ok(response) => println!("{}", response.content().unwrap_or_default()),
//!         Err(Error::RateLimit { .. }) => {
//!             println!("Rate limited, please retry later");
//!         }
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust,no_run
//! use openai_ergonomic::{Client, Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::builder()
//!         .api_key("your-api-key")
//!         .organization("org-123")
//!         .max_retries(5)
//!         .build();
//!
//!     let client = Client::builder(config)?.build();
//!     Ok(())
//! }
//! ```
//!
//! ## Testing with Mocks
//!
//! ```rust,no_run
//! #[cfg(test)]
//! mod tests {
//!     use openai_ergonomic::test_utils::MockOpenAIServer;
//!
//!     #[tokio::test]
//!     async fn test_chat_completion() {
//!         let mock = MockOpenAIServer::new();
//!         mock.mock_chat_completion("Hello!", "Hi there!");
//!
//!         let client = mock.client();
//!         let response = client.chat_simple("Hello!").await.unwrap();
//!         assert_eq!(response, "Hi there!");
//!     }
//! }
//! ```
//!
//! # Modules
//!
//! - [`builders`] - Builder pattern implementations for API requests
//! - [`responses`] - Response type wrappers with ergonomic helpers
//! - [`client`] - Main client for API interactions
//! - [`config`] - Configuration management
//! - [`provider`] - Providers: `OpenAI`, Azure `OpenAI`, `OpenRouter`, Groq, Ollama, custom
//! - [`auth`] - Pluggable, refreshable authentication
//! - `webhooks` - Webhook signature verification and events (feature `webhook`)
//! - [`errors`] - Error types and handling

// Core modules
pub mod auth;
pub mod azure_middleware;
pub mod builders;
pub mod client;
pub mod config;
pub mod errors;
pub mod interceptor;
#[cfg(feature = "otel")]
pub mod langfuse_context;
#[cfg(feature = "otel")]
pub mod langfuse_exporter;
#[cfg(feature = "otel")]
pub mod langfuse_interceptor;
pub mod provider;
pub mod responses;
pub mod semantic_conventions;
pub mod streaming;
pub mod tool_framework;
#[cfg(feature = "webhook")]
pub mod webhooks;

mod time;

/// Boxed future used internally to store heterogeneous async trait
/// implementations (`ErasedTool`, `ErasedInterceptor`): `Send` natively, not on
/// WebAssembly, whose futures cannot leave the JavaScript host's thread.
#[cfg(not(target_family = "wasm"))]
pub(crate) type BoxFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
/// Boxed future used internally to store heterogeneous async trait
/// implementations (`ErasedTool`, `ErasedInterceptor`): `Send` natively, not on
/// WebAssembly, whose futures cannot leave the JavaScript host's thread.
#[cfg(target_family = "wasm")]
pub(crate) type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>;

/// Boxed stream used internally: `Send` natively, not on WebAssembly.
#[cfg(not(target_family = "wasm"))]
pub(crate) type BoxStream<'a, T> = futures::stream::BoxStream<'a, T>;
/// Boxed stream used internally: `Send` natively, not on WebAssembly.
#[cfg(target_family = "wasm")]
pub(crate) type BoxStream<'a, T> = futures::stream::LocalBoxStream<'a, T>;

/// Boxes a stream: `Send` natively, not on WebAssembly.
pub(crate) fn box_stream<'a, S>(stream: S) -> BoxStream<'a, S::Item>
where
    S: futures::Stream + MaybeSend + 'a,
{
    Box::pin(stream)
}

/// `Send` natively, any type on WebAssembly; `Sync` natively, any type on WebAssembly.
///
/// The bounds of [`FunctionTool`], [`Interceptor`], [`AuthProvider`], their futures and the
/// interceptor state `T` of [`Client<T>`](Client): on
/// WebAssembly (`wasm32-unknown-unknown`) futures and streams of the JavaScript host are not
/// `Send`, so nothing there needs to be. Natively they are exactly `Send` and `Sync`.
pub use timvw_rust_runtime::{MaybeSend, MaybeSync};

// Re-export commonly used types
pub use auth::{AuthLayer, AuthProvider, AuthToken, BearerTokenAuth};
pub use client::Client;
pub use config::{Config, ConfigBuilder};
pub use errors::{Error, Result};
pub use interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext, StreamTerminalContext,
};
#[cfg(feature = "otel")]
pub use langfuse_context::LangfuseContext;
#[cfg(feature = "otel")]
pub use langfuse_exporter::{langfuse_exporter_from_env, LangfuseExporterError};
#[cfg(feature = "otel")]
pub use langfuse_interceptor::{LangfuseConfig, LangfuseInterceptor, LangfuseState};
pub use provider::{Provider, ProviderKind};
pub use tool_framework::{FunctionTool, ToolError, ToolOutput, ToolRegistry};

// Re-export specific builder and response types for convenience
// NOTE: We avoid wildcard re-exports to prevent naming conflicts between modules
pub use builders::audio::{
    SpeechBuilder, TimestampGranularity, TranscriptionBuilder, TranscriptionRequest,
    TranslationBuilder, TranslationRequest,
};
pub use builders::chat::{
    image_base64_part, image_base64_part_with_detail, image_url_part, image_url_part_with_detail,
    system_user, text_part, user_message,
};
pub use builders::embeddings::{EmbeddingInput, EmbeddingsBuilder};
pub use builders::images::{
    Background, ImageEditBuilder, ImageEditRequest, ImageGenerationBuilder, ImageInputFidelity,
    ImageVariationBuilder, ImageVariationRequest, Moderation, OutputFormat, Quality,
    ResponseFormat, Size, Style,
};
pub use builders::threads::{
    AttachmentTool, MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};
pub use builders::uploads::UploadBuilder;
// Re-export vision types for convenience
pub use builders::responses::{responses_simple, responses_system_user, ResponsesBuilder};
pub use builders::{Builder, ChatCompletionBuilder, Sendable};
pub use openai_client_base::models::chat_completion_request_message_content_part_image_image_url::Detail;
pub use openai_client_base::models::create_upload_request::Purpose as UploadPurpose;
pub use responses::chat::{
    ChatChoice, ChatCompletionResponse, ChatCompletionResponseExt,
    ChatMessage as ResponseChatMessage, FunctionCall, ToolCall, ToolCallExt,
};
pub use responses::{tool_function, tool_web_search, ChatCompletionResponseWrapper};
pub use responses::{Response, Tool, ToolChoice, Usage};
pub use streaming::{
    ChatCompletionChunk, ChatCompletionStream, CompletionUnavailable, InterceptedStream,
    StreamCallbackFailure, StreamCallbackFailureKind, StreamCallbackHook, StreamCompletion,
    StreamCompletionReport, StreamFailure, StreamFailureKind, StreamOutcome,
};

// Test utilities (feature-gated; native only: mockito)
#[cfg(all(feature = "test-utils", not(target_family = "wasm")))]
pub mod test_utils;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::builder().api_key("test-key").build();
        assert_eq!(config.api_key(), "test-key");
    }

    #[test]
    fn test_client_creation_with_config() {
        let config = Config::builder().api_key("test-key").build();
        let result = Client::builder(config);
        assert!(result.is_ok());
    }
}
