//! Interceptor stub for backward compatibility (deprecated).
//!
//! This module provides minimal stubs to allow existing code to compile
//! while migrating to the middleware system.

use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Context passed to before_request interceptors (deprecated).
#[deprecated(note = "Use the middleware system instead")]
pub struct BeforeRequestContext<'a> {
    /// The operation being performed
    pub operation: &'a str,
    /// The model being used
    pub model: &'a str,
    /// The request JSON
    pub request_json: &'a str,
    /// Metadata for sharing state
    pub metadata: &'a mut HashMap<String, String>,
}

/// Context passed to after_response interceptors (deprecated).
#[deprecated(note = "Use the middleware system instead")]
pub struct AfterResponseContext<'a> {
    /// The operation being performed
    pub operation: &'a str,
    /// The model being used
    pub model: &'a str,
    /// The request JSON
    pub request_json: &'a str,
    /// The response JSON
    pub response_json: &'a str,
    /// Duration of the operation
    pub duration: std::time::Duration,
    /// Input tokens used
    pub input_tokens: Option<i64>,
    /// Output tokens generated
    pub output_tokens: Option<i64>,
    /// Metadata
    pub metadata: &'a HashMap<String, String>,
}

/// Context passed to error interceptors (deprecated).
#[deprecated(note = "Use the middleware system instead")]
pub struct ErrorContext<'a> {
    /// The operation being performed
    pub operation: &'a str,
    /// The model being used
    pub model: Option<&'a str>,
    /// The request JSON
    pub request_json: Option<&'a str>,
    /// The error that occurred
    pub error: &'a crate::Error,
    /// Metadata
    pub metadata: Option<&'a HashMap<String, String>>,
}

/// Interceptor trait (deprecated).
#[async_trait]
#[deprecated(note = "Use the middleware system instead")]
pub trait Interceptor: Send + Sync {
    /// Called before making a request (no-op stub).
    async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called after receiving a response (no-op stub).
    async fn after_response(&self, _ctx: &AfterResponseContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Called when an error occurs (no-op stub).
    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // No-op
    }
}

/// Chain of interceptors (deprecated, no-op stub).
#[deprecated(note = "Use the middleware system instead")]
pub struct InterceptorChain {
    _interceptors: Vec<Box<dyn Interceptor>>,
}

impl InterceptorChain {
    /// Create a new empty chain.
    pub fn new() -> Self {
        Self {
            _interceptors: Vec::new(),
        }
    }

    /// Add an interceptor (no-op).
    pub fn add(&mut self, _interceptor: Box<dyn Interceptor>) {
        // No-op
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        true  // Always empty
    }

    /// Call before_request hooks (no-op).
    pub async fn before_request(&self, _ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Call after_response hooks (no-op).
    pub async fn after_response(&self, _ctx: &AfterResponseContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Call error hooks (no-op).
    pub async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        // No-op
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}
