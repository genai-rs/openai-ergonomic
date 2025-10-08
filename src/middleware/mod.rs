//! Middleware system for intercepting and instrumenting `OpenAI` API calls.
//!
//! This module provides a middleware pattern that allows full access to the request/response
//! lifecycle, enabling proper OpenTelemetry tracing, logging, and other cross-cutting concerns.

use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Context passed to middleware containing request information.
#[derive(Debug)]
pub struct MiddlewareRequest<'a> {
    /// The operation being performed (e.g., "chat", "embeddings")
    pub operation: &'a str,
    /// The model being used
    pub model: &'a str,
    /// The request payload as JSON string
    pub request_json: &'a str,
    /// Metadata that can be shared between middleware
    pub metadata: HashMap<String, String>,
}

/// Response returned from middleware containing result information.
#[derive(Debug)]
pub struct MiddlewareResponse {
    /// The response payload as JSON string
    pub response_json: String,
    /// Duration of the operation
    pub duration: std::time::Duration,
    /// Input tokens used (if available)
    pub input_tokens: Option<i64>,
    /// Output tokens generated (if available)
    pub output_tokens: Option<i64>,
    /// Metadata from the request (can be modified)
    pub metadata: HashMap<String, String>,
}

/// Middleware trait for intercepting API calls with full lifecycle access.
///
/// Unlike interceptors which split before/after, middleware has access to the
/// entire request/response cycle in a single method, enabling proper span management.
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Handle an API call, with access to the full request/response lifecycle.
    ///
    /// The middleware can:
    /// - Inspect/modify the request
    /// - Call `next.run()` to proceed to the next middleware or actual API call
    /// - Inspect/modify the response
    /// - Manage OpenTelemetry spans that live for the entire operation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn handle(&self, req: MiddlewareRequest<'_>, next: Next<'_>) -> Result<MiddlewareResponse> {
    ///     // Create span before request
    ///     let span = create_span(&req);
    ///
    ///     // Execute the request
    ///     let response = next.run(req).await?;
    ///
    ///     // Add response attributes to span
    ///     span.set_attributes(/* ... */);
    ///     span.end();
    ///
    ///     Ok(response)
    /// }
    /// ```
    async fn handle(
        &self,
        req: MiddlewareRequest<'_>,
        next: Next<'_>,
    ) -> Result<MiddlewareResponse>;
}

/// The Next handler in the middleware chain.
///
/// Call `next.run()` to proceed to the next middleware or the actual API call.
pub struct Next<'a> {
    middlewares: &'a [Arc<dyn Middleware>],
    index: usize,
    executor: &'a (dyn Fn(MiddlewareRequest<'_>) -> futures::future::BoxFuture<'_, Result<MiddlewareResponse>>
             + Send
             + Sync),
}

impl<'a> Next<'a> {
    /// Create a new Next handler.
    pub(crate) fn new(
        middlewares: &'a [Arc<dyn Middleware>],
        index: usize,
        executor: &'a (dyn Fn(
            MiddlewareRequest<'_>,
        ) -> futures::future::BoxFuture<'_, Result<MiddlewareResponse>>
                 + Send
                 + Sync),
    ) -> Self {
        Self {
            middlewares,
            index,
            executor,
        }
    }

    /// Execute the next middleware in the chain, or the final executor if this is the last one.
    pub async fn run(self, req: MiddlewareRequest<'_>) -> Result<MiddlewareResponse> {
        if self.index < self.middlewares.len() {
            // Call next middleware
            let middleware = &self.middlewares[self.index];
            let next = Next::new(self.middlewares, self.index + 1, self.executor);
            middleware.handle(req, next).await
        } else {
            // No more middleware, call the executor (actual API call)
            (self.executor)(req).await
        }
    }
}

/// Chain of middleware that will be executed in order.
#[derive(Clone)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the chain.
    #[must_use]
    pub fn with(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Execute the middleware chain with the given request and executor.
    pub async fn execute<F>(
        &self,
        req: MiddlewareRequest<'_>,
        executor: F,
    ) -> Result<MiddlewareResponse>
    where
        F: Fn(MiddlewareRequest<'_>) -> futures::future::BoxFuture<'_, Result<MiddlewareResponse>>
            + Send
            + Sync,
    {
        let next = Next::new(&self.middlewares, 0, &executor);
        next.run(req).await
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Get the number of middleware in the chain.
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

// Middleware implementations
pub mod langfuse;
pub mod opentelemetry;
