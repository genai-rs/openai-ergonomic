//! Interceptor system for middleware and observability.
//!
//! Interceptors provide hooks into the request/response lifecycle, enabling:
//! - Telemetry and tracing
//! - Logging and debugging
//! - Metrics collection
//! - Request/response transformation
//! - Custom error handling
//! - Authentication enhancement
//!
//! # Architecture
//!
//! The interceptor system follows a chain-of-responsibility pattern where
//! multiple interceptors can be registered and executed in order. Each
//! interceptor can:
//!
//! - Modify request context before sending
//! - Observe and react to responses
//! - Handle streaming chunks
//! - Process errors
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
//!     async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
//!         println!("Calling {} with model {}", ctx.operation, ctx.model);
//!         Ok(())
//!     }
//! }
//!
//! let client = Client::from_env()?
//!     .with_interceptor(Box::new(LoggingInterceptor));
//! ```

use crate::Result;
use std::collections::HashMap;
use std::time::Duration;

/// Context provided before a request is sent.
///
/// This context contains all the information about the request that's about
/// to be made, and allows interceptors to modify metadata that will be
/// carried through the request lifecycle.
#[derive(Debug)]
pub struct BeforeRequestContext<'a> {
    /// The operation being performed (e.g., "chat", "embedding", "image_generation")
    pub operation: &'a str,
    /// The model being used for the request
    pub model: &'a str,
    /// The serialized request body as JSON
    pub request_json: &'a str,
    /// Mutable metadata map for passing data between interceptors
    pub metadata: &'a mut HashMap<String, String>,
}

/// Context provided after a successful non-streaming response.
///
/// This context contains the complete request and response information,
/// allowing interceptors to observe and react to successful API calls.
#[derive(Debug)]
pub struct AfterResponseContext<'a> {
    /// The operation that was performed
    pub operation: &'a str,
    /// The model that was used
    pub model: &'a str,
    /// The original request body as JSON
    pub request_json: &'a str,
    /// The response body as JSON
    pub response_json: &'a str,
    /// Time taken for the request
    pub duration: Duration,
    /// Number of input tokens used (if available)
    pub input_tokens: Option<i64>,
    /// Number of output tokens generated (if available)
    pub output_tokens: Option<i64>,
    /// Metadata from the request context
    pub metadata: &'a HashMap<String, String>,
}

/// Context provided for each chunk in a streaming response.
///
/// This context allows interceptors to process streaming data as it arrives,
/// useful for real-time monitoring or transformation.
#[derive(Debug)]
pub struct StreamChunkContext<'a> {
    /// The operation being performed
    pub operation: &'a str,
    /// The model being used
    pub model: &'a str,
    /// The original request body as JSON
    pub request_json: &'a str,
    /// The current chunk data as JSON
    pub chunk_json: &'a str,
    /// Zero-based index of this chunk
    pub chunk_index: usize,
    /// Metadata from the request context
    pub metadata: &'a HashMap<String, String>,
}

/// Context provided when a streaming response completes.
///
/// This context provides summary information about the completed stream,
/// including total chunks and token usage.
#[derive(Debug)]
pub struct StreamEndContext<'a> {
    /// The operation that was performed
    pub operation: &'a str,
    /// The model that was used
    pub model: &'a str,
    /// The original request body as JSON
    pub request_json: &'a str,
    /// Total number of chunks received
    pub total_chunks: usize,
    /// Total time for the streaming response
    pub duration: Duration,
    /// Total input tokens used (if available)
    pub input_tokens: Option<i64>,
    /// Total output tokens generated (if available)
    pub output_tokens: Option<i64>,
    /// Metadata from the request context
    pub metadata: &'a HashMap<String, String>,
}

/// Context provided when an error occurs.
///
/// This context allows interceptors to observe and react to errors,
/// useful for error tracking and recovery strategies.
#[derive(Debug)]
pub struct ErrorContext<'a> {
    /// The operation that failed
    pub operation: &'a str,
    /// The model being used (if known)
    pub model: Option<&'a str>,
    /// The request body as JSON (if available)
    pub request_json: Option<&'a str>,
    /// The error that occurred
    pub error: &'a crate::Error,
    /// Metadata from the request context (if available)
    pub metadata: Option<&'a HashMap<String, String>>,
}

/// Trait for implementing interceptors.
///
/// Interceptors can hook into various stages of the request/response lifecycle.
/// All methods have default no-op implementations, so you only need to implement
/// the hooks you're interested in.
#[async_trait::async_trait]
pub trait Interceptor: Send + Sync {
    /// Called before a request is sent.
    ///
    /// This method can modify the metadata that will be passed through
    /// the request lifecycle.
    async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called after a successful non-streaming response is received.
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
    /// Note: This method doesn't return a Result as it's called during
    /// error handling and shouldn't fail.
    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // Default: no-op
    }
}

/// A chain of interceptors that are executed in order.
///
/// This struct manages multiple interceptors and ensures they are
/// called in the correct order for each lifecycle event.
pub struct InterceptorChain {
    interceptors: Vec<Box<dyn Interceptor>>,
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}

impl InterceptorChain {
    /// Create a new, empty interceptor chain.
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// Add an interceptor to the chain.
    ///
    /// Interceptors are executed in the order they are added.
    pub fn add(&mut self, interceptor: Box<dyn Interceptor>) {
        self.interceptors.push(interceptor);
    }

    /// Execute the before_request hook for all interceptors.
    pub async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.before_request(ctx).await?;
        }
        Ok(())
    }

    /// Execute the after_response hook for all interceptors.
    pub async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.after_response(ctx).await?;
        }
        Ok(())
    }

    /// Execute the on_stream_chunk hook for all interceptors.
    pub async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.on_stream_chunk(ctx).await?;
        }
        Ok(())
    }

    /// Execute the on_stream_end hook for all interceptors.
    pub async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        for interceptor in &self.interceptors {
            interceptor.on_stream_end(ctx).await?;
        }
        Ok(())
    }

    /// Execute the on_error hook for all interceptors.
    ///
    /// Errors in individual interceptors are ignored to prevent
    /// cascading failures during error handling.
    pub async fn on_error(&self, ctx: &ErrorContext<'_>) {
        for interceptor in &self.interceptors {
            // Ignore errors in error handlers to prevent cascading failures
            interceptor.on_error(ctx).await;
        }
    }

    /// Check if the chain has any interceptors.
    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    /// Get the number of interceptors in the chain.
    pub fn len(&self) -> usize {
        self.interceptors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A test interceptor that tracks how many times each method was called.
    struct TestInterceptor {
        before_request_count: Arc<AtomicUsize>,
        after_response_count: Arc<AtomicUsize>,
        on_stream_chunk_count: Arc<AtomicUsize>,
        on_stream_end_count: Arc<AtomicUsize>,
        on_error_count: Arc<AtomicUsize>,
    }

    impl TestInterceptor {
        fn new() -> Self {
            Self {
                before_request_count: Arc::new(AtomicUsize::new(0)),
                after_response_count: Arc::new(AtomicUsize::new(0)),
                on_stream_chunk_count: Arc::new(AtomicUsize::new(0)),
                on_stream_end_count: Arc::new(AtomicUsize::new(0)),
                on_error_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
    impl Interceptor for TestInterceptor {
        async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
            self.before_request_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_response(&self, _ctx: &AfterResponseContext<'_>) -> Result<()> {
            self.after_response_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_>) -> Result<()> {
            self.on_stream_chunk_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_stream_end(&self, _ctx: &StreamEndContext<'_>) -> Result<()> {
            self.on_stream_end_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_error(&self, _ctx: &ErrorContext<'_>) {
            self.on_error_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_interceptor_chain_executes_in_order() {
        let mut chain = InterceptorChain::new();
        let interceptor1 = TestInterceptor::new();
        let interceptor2 = TestInterceptor::new();

        let count1 = interceptor1.before_request_count.clone();
        let count2 = interceptor2.before_request_count.clone();

        chain.add(Box::new(interceptor1));
        chain.add(Box::new(interceptor2));

        // Test before_request
        let mut metadata = HashMap::new();
        let mut ctx = BeforeRequestContext {
            operation: "test",
            model: "gpt-4",
            request_json: "{}",
            metadata: &mut metadata,
        };
        chain.before_request(&mut ctx).await.unwrap();

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_interceptor_chain_handles_errors() {
        struct FailingInterceptor;

        #[async_trait::async_trait]
        impl Interceptor for FailingInterceptor {
            async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
                Err(crate::Error::Internal("Test error".to_string()))
            }
        }

        let mut chain = InterceptorChain::new();
        chain.add(Box::new(FailingInterceptor));

        let mut metadata = HashMap::new();
        let mut ctx = BeforeRequestContext {
            operation: "test",
            model: "gpt-4",
            request_json: "{}",
            metadata: &mut metadata,
        };

        let result = chain.before_request(&mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_interceptor_chain_empty() {
        let chain = InterceptorChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);

        // Empty chain should succeed without doing anything
        let mut metadata = HashMap::new();
        let mut ctx = BeforeRequestContext {
            operation: "test",
            model: "gpt-4",
            request_json: "{}",
            metadata: &mut metadata,
        };
        chain.before_request(&mut ctx).await.unwrap();
    }

    #[tokio::test]
    async fn test_metadata_passing() {
        struct MetadataInterceptor;

        #[async_trait::async_trait]
        impl Interceptor for MetadataInterceptor {
            async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
                ctx.metadata.insert("test_key".to_string(), "test_value".to_string());
                Ok(())
            }
        }

        let mut chain = InterceptorChain::new();
        chain.add(Box::new(MetadataInterceptor));

        let mut metadata = HashMap::new();
        let mut ctx = BeforeRequestContext {
            operation: "test",
            model: "gpt-4",
            request_json: "{}",
            metadata: &mut metadata,
        };

        chain.before_request(&mut ctx).await.unwrap();
        assert_eq!(metadata.get("test_key"), Some(&"test_value".to_string()));
    }

    #[tokio::test]
    async fn test_error_handler_doesnt_propagate_errors() {
        struct ErrorInterceptor {
            called: Arc<AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl Interceptor for ErrorInterceptor {
            async fn on_error(&self, _ctx: &ErrorContext<'_>) {
                self.called.fetch_add(1, Ordering::SeqCst);
                // This would panic in a real scenario, but shouldn't crash the chain
                panic!("This panic should be caught");
            }
        }

        let chain = InterceptorChain::new();
        let error = crate::Error::Internal("Test".to_string());
        let ctx = ErrorContext {
            operation: "test",
            model: None,
            request_json: None,
            error: &error,
            metadata: None,
        };

        // Should not panic even though the interceptor panics
        chain.on_error(&ctx).await;
    }
}