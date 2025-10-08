//! Interceptor module (stub for backward compatibility)
//!
//! This module provides stub types for backward compatibility.
//! The interceptor functionality has been removed.

use crate::Error;
use async_trait::async_trait;
use std::collections::HashMap;

/// Interceptor trait (stub for backward compatibility)
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Before request hook (stub - does nothing)
    async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> crate::Result<()> {
        Ok(())
    }

    /// After response hook (stub - does nothing)
    async fn after_response(&self, _ctx: &AfterResponseContext<'_>) {
        // No-op
    }

    /// On error hook (stub - does nothing)
    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // No-op
    }

    /// On stream chunk hook (stub - does nothing)
    async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_>) {
        // No-op
    }

    /// On stream end hook (stub - does nothing)
    async fn on_stream_end(&self, _ctx: &StreamEndContext<'_>) {
        // No-op
    }
}

/// Stub interceptor chain
#[derive(Default)]
pub struct InterceptorChain;

impl InterceptorChain {
    /// Create a new empty interceptor chain
    pub fn new() -> Self {
        Self
    }

    /// Check if chain is empty (always true for stub)
    pub fn is_empty(&self) -> bool {
        true
    }

    /// Add an interceptor (stub - does nothing)
    pub fn add(&mut self, _interceptor: Box<dyn Interceptor>) {
        // No-op - interceptors are not actually stored or used
    }

    /// Stub before_request method
    pub async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> crate::Result<()> {
        Ok(())
    }

    /// Stub on_error method
    pub async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // No-op
    }

    /// Stub after_response method
    pub async fn after_response(&self, _ctx: &AfterResponseContext<'_>) {
        // No-op
    }
}

/// Stub context for before_request hooks
pub struct BeforeRequestContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model name
    pub model: &'a str,
    /// Request JSON
    pub request_json: &'a str,
    /// Metadata
    pub metadata: &'a mut HashMap<String, String>,
}

/// Stub context for after_response hooks
pub struct AfterResponseContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model name
    pub model: &'a str,
    /// Request JSON
    pub request_json: &'a str,
    /// Response JSON
    pub response_json: &'a str,
    /// Metadata
    pub metadata: &'a HashMap<String, String>,
    /// Duration
    pub duration: std::time::Duration,
    /// Input tokens
    pub input_tokens: Option<i64>,
    /// Output tokens
    pub output_tokens: Option<i64>,
}

/// Stub context for error hooks
pub struct ErrorContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model name
    pub model: Option<&'a str>,
    /// Request JSON
    pub request_json: Option<&'a str>,
    /// Error
    pub error: &'a Error,
    /// Metadata
    pub metadata: Option<&'a HashMap<String, String>>,
}

/// Stub context for stream chunk hooks
pub struct StreamChunkContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model name
    pub model: &'a str,
    /// Chunk content
    pub content: &'a str,
    /// Metadata
    pub metadata: &'a HashMap<String, String>,
}

/// Stub context for stream end hooks
pub struct StreamEndContext<'a> {
    /// Operation name
    pub operation: &'a str,
    /// Model name
    pub model: &'a str,
    /// Metadata
    pub metadata: &'a HashMap<String, String>,
    /// Duration
    pub duration: std::time::Duration,
}
