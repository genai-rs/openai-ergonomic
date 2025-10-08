//! Langfuse interceptor module (stub for backward compatibility)
//!
//! This module provides stub types for backward compatibility.
//! The Langfuse interceptor functionality has been removed.
//! Please use the middleware system instead.

/// Stub Langfuse configuration
#[derive(Debug, Clone, Default)]
pub struct LangfuseConfig {
    _private: (),
}

impl LangfuseConfig {
    /// Create a new Langfuse config
    pub fn new() -> Self {
        Self { _private: () }
    }
}

/// Builder for LangfuseInterceptor (stub)
#[derive(Debug, Default)]
pub struct LangfuseInterceptorBuilder {
    _private: (),
}

impl LangfuseInterceptorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Build the interceptor (returns stub)
    pub fn build(self) -> LangfuseInterceptor {
        LangfuseInterceptor { _private: () }
    }
}

/// Stub Langfuse interceptor
#[derive(Debug)]
pub struct LangfuseInterceptor {
    _private: (),
}

impl LangfuseInterceptor {
    /// Create a new Langfuse interceptor (stub)
    pub fn new(_config: LangfuseConfig) -> Self {
        Self { _private: () }
    }

    /// Create a builder
    pub fn builder() -> LangfuseInterceptorBuilder {
        LangfuseInterceptorBuilder::new()
    }
}
