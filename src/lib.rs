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

//! ## Modules
//!
//! - [`builders`] - Builder pattern implementations for API requests
//! - [`responses`] - Response type wrappers with ergonomic helpers
//! - [`responses_api`] - The Responses API (`/responses`): builder, response, streaming events
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
pub mod rate_limits;
pub mod responses;
pub mod responses_api;
pub mod semantic_conventions;
pub mod streaming;
pub mod tool_framework;
#[cfg(feature = "webhook")]
pub mod webhooks;

mod sse_frames;
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
pub use errors::{Error, Result, RetryClass};
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
pub use rate_limits::{RateLimitSnapshot, RateLimitWindow};
pub use tool_framework::{
    CallAnswer, CustomFormat, CustomTool, DynamicTool, FunctionTool, OutputBudget, OutputContent,
    RichTool, ToolError, ToolFailure, ToolInvocation, ToolKind, ToolOutput, ToolOutputBody,
    ToolRegistry, TruncationConfig,
};

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
pub use builders::{Builder, ChatCompletionBuilder, Sendable};
pub use openai_client_base::models::chat_completion_request_message_content_part_image_image_url::Detail;
pub use openai_client_base::models::create_upload_request::Purpose as UploadPurpose;
pub use responses::chat::{
    ChatChoice, ChatCompletionResponse, ChatCompletionResponseExt,
    ChatMessage as ResponseChatMessage, FunctionCall, ToolCall, ToolCallExt,
};
pub use responses::{tool_function, tool_web_search, ChatCompletionResponseWrapper};
pub use responses::{Response, Tool, ToolChoice, Usage};
pub use responses_api::{
    responses_simple, responses_system_user, BoxedResponsesStream, ResponsesBuilder,
    ResponsesEvent, ResponsesEventKind, ResponsesResponse, ResponsesStream,
};
pub use streaming::{
    ChatCompletionChunk, ChatCompletionStream, CompletionUnavailable, InterceptedItem,
    InterceptedStream, StreamCallbackFailure, StreamCallbackFailureKind, StreamCallbackHook,
    StreamCompletion, StreamCompletionReport, StreamFailure, StreamFailureKind, StreamOutcome,
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

/// The guides under `docs/`, compiled as doc tests so that their examples keep working (the README
/// is the crate documentation, so its examples are doc tests already).
#[cfg(doctest)]
mod doc_pages {
    #[doc = include_str!("../docs/getting-started.md")]
    struct GettingStarted;
    #[doc = include_str!("../docs/configuration.md")]
    struct Configuration;
    #[doc = include_str!("../docs/chat-and-responses.md")]
    struct ChatAndResponses;
    #[doc = include_str!("../docs/streaming.md")]
    struct Streaming;
    #[doc = include_str!("../docs/interceptors.md")]
    struct Interceptors;
    #[cfg(feature = "webhook")]
    #[doc = include_str!("../docs/webhooks.md")]
    struct Webhooks;
    #[doc = include_str!("../docs/files-and-media.md")]
    struct FilesAndMedia;
    #[doc = include_str!("../docs/webassembly.md")]
    struct WebAssembly;
    #[doc = include_str!("../docs/migration.md")]
    struct Migration;
    #[doc = include_str!("../docs/development.md")]
    struct Development;
    #[doc = include_str!("../docs/responses_workflows.md")]
    struct ResponsesWorkflows;
    #[doc = include_str!("../docs/tool_framework.md")]
    struct ToolFramework;
    #[doc = include_str!("../docs/tool_orchestration.md")]
    struct ToolOrchestration;
    #[doc = include_str!("../docs/vector_store_operations.md")]
    struct VectorStoreOperations;
    #[doc = include_str!("../docs/streaming-completion-contract.md")]
    struct StreamingCompletionContract;
    #[cfg(feature = "otel")]
    #[doc = include_str!("../docs/langfuse-integration.md")]
    struct LangfuseIntegration;
    #[doc = include_str!("../docs/api-coverage.md")]
    struct ApiCoverage;
    #[doc = include_str!("../docs/architecture.md")]
    struct Architecture;
    #[doc = include_str!("../docs/examples-index.md")]
    struct ExamplesIndex;
}
