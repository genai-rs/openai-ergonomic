//! Langfuse interceptor stub for backward compatibility (deprecated).
//!
//! This module is deprecated. Use `middleware::langfuse::LangfuseMiddleware` instead.

#![allow(dead_code)]

/// Langfuse interceptor (deprecated).
///
/// Use `middleware::langfuse::LangfuseMiddleware` instead.
#[deprecated(note = "Use middleware::langfuse::LangfuseMiddleware instead")]
pub struct LangfuseInterceptor;

impl LangfuseInterceptor {
    /// Create a new Langfuse interceptor (deprecated).
    #[deprecated(note = "Use middleware::langfuse::LangfuseMiddleware::new() instead")]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LangfuseInterceptor {
    fn default() -> Self {
        Self::new()
    }
}
