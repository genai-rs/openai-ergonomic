//! Interceptor system for observability and middleware.
//!
//! Interceptors provide hooks into the request/response lifecycle, enabling:
//! - Telemetry and tracing
//! - Logging
//! - Metrics collection
//! - Request/response transformation
//! - Custom error handling
//!
//! # Example
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, Interceptor, BeforeRequestContext};
//!
//! struct LoggingInterceptor;
//!
//! #[async_trait::async_trait]
//! impl Interceptor for LoggingInterceptor {
//!     async fn before_request(&self, ctx: &BeforeRequestContext<'_>) -> Result<()> {
//!         println!("Calling {} with model {}", ctx.operation, ctx.model);
//!         Ok(())
//!     }
//! }
//!
//! let client = Client::from_env()?
//!     .with_interceptor(Box::new(LoggingInterceptor))
//!     .build();
//! ```

use crate::Result;
use std::time::Duration;

/// Context provided before a request is sent.
#[derive(Debug)]
pub struct BeforeRequestContext<'a> {
    /// Operation name (e.g., "chat", "embedding", "`image_generation`")
    pub operation: &'a str,
    /// Model being used
    pub model: &'a str,
    /// Request body as JSON string
    pub request_json: &'a str,
    /// Custom metadata that can be set by interceptors
    pub metadata: std::collections::HashMap<String, String>,
}

/// Context provided after a successful non-streaming response.
#[derive(Debug)]
pub struct AfterResponseContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model used
    pub model: &'a str,
    /// Request body as JSON string
    pub request_json: &'a str,
    /// Response body as JSON string
    pub response_json: &'a str,
    /// Request duration
    pub duration: Duration,
    /// Input tokens used (if available)
    pub input_tokens: Option<i64>,
    /// Output tokens used (if available)
    pub output_tokens: Option<i64>,
    /// Metadata from `before_request`
    pub metadata: &'a std::collections::HashMap<String, String>,
}

/// Context provided for each chunk in a streaming response.
#[derive(Debug)]
pub struct StreamChunkContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model used
    pub model: &'a str,
    /// Request body as JSON string
    pub request_json: &'a str,
    /// Chunk data as JSON string
    pub chunk_json: &'a str,
    /// Chunk index (0-based)
    pub chunk_index: usize,
    /// Metadata from `before_request`
    pub metadata: &'a std::collections::HashMap<String, String>,
}

/// Context provided when a streaming response completes.
#[derive(Debug)]
pub struct StreamEndContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model used
    pub model: &'a str,
    /// Request body as JSON string
    pub request_json: &'a str,
    /// Total chunks received
    pub total_chunks: usize,
    /// Stream duration
    pub duration: Duration,
    /// Total input tokens (if available)
    pub input_tokens: Option<i64>,
    /// Total output tokens (if available)
    pub output_tokens: Option<i64>,
    /// Metadata from `before_request`
    pub metadata: &'a std::collections::HashMap<String, String>,
}

/// Context provided when an error occurs.
#[derive(Debug)]
pub struct ErrorContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model being used (if known)
    pub model: Option<&'a str>,
    /// Request body as JSON string (if available)
    pub request_json: Option<&'a str>,
    /// Error message
    pub error: &'a str,
    /// Time since request started
    pub duration: Duration,
    /// Metadata from `before_request` (if available)
    pub metadata: Option<&'a std::collections::HashMap<String, String>>,
}

/// Interceptor trait for hooking into the request/response lifecycle.
///
/// All methods have default implementations that do nothing, so you only
/// need to implement the hooks you care about.
#[async_trait::async_trait]
pub trait Interceptor: Send + Sync {
    /// Called before a request is sent.
    ///
    /// You can inspect and modify the context (e.g., add metadata).
    /// Return an error to abort the request.
    async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called after a successful non-streaming response.
    async fn after_response(&self, _ctx: &AfterResponseContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called for each chunk in a streaming response.
    async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called when a streaming response completes successfully.
    async fn on_stream_end(&self, _ctx: &StreamEndContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called when an error occurs at any stage.
    ///
    /// This is informational only - you cannot modify the error.
    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // Default: do nothing
    }
}

/// Helper to execute all interceptors for a given hook.
#[derive(Default)]
pub(crate) struct InterceptorChain {
    pub(crate) interceptors: Vec<Box<dyn Interceptor>>,
}

impl Clone for InterceptorChain {
    fn clone(&self) -> Self {
        // We can't directly clone Box<dyn Interceptor>, so we create a new empty chain
        // The with_interceptor method will handle adding interceptors properly
        Self {
            interceptors: Vec::new(),
        }
    }
}

impl InterceptorChain {
    #[allow(dead_code)] // Will be used for streaming support
    pub fn new(interceptors: Vec<Box<dyn Interceptor>>) -> Self {
        Self { interceptors }
    }

    pub async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.before_request(ctx).await?;
        }
        Ok(())
    }

    pub async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.after_response(ctx).await?;
        }
        Ok(())
    }

    #[allow(dead_code)] // Will be used for streaming support
    pub async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.on_stream_chunk(ctx).await?;
        }
        Ok(())
    }

    #[allow(dead_code)] // Will be used for streaming support
    pub async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.on_stream_end(ctx).await?;
        }
        Ok(())
    }

    pub async fn on_error(&self, ctx: &ErrorContext<'_>) {
        for interceptor in &self.interceptors {
            interceptor.on_error(ctx).await;
        }
    }
}
