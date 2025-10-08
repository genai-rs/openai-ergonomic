//! Langfuse interceptor for OpenTelemetry-based LLM observability.
//!
//! This module provides a Langfuse interceptor that integrates with OpenTelemetry
//! to send traces to Langfuse for comprehensive LLM observability.

use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext,
};
use crate::Result;
use opentelemetry::{
    trace::{Span, SpanKind, Status, Tracer, TracerProvider as _},
    KeyValue,
};
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{
        span_processor_with_async_runtime::BatchSpanProcessor, BatchConfigBuilder,
        RandomIdGenerator, Sampler, SdkTracerProvider,
    },
    Resource,
};
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MAX_TOKENS, GEN_AI_REQUEST_MODEL,
    GEN_AI_REQUEST_TEMPERATURE, GEN_AI_RESPONSE_ID, GEN_AI_SYSTEM, GEN_AI_USAGE_INPUT_TOKENS,
    GEN_AI_USAGE_OUTPUT_TOKENS,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Configuration for the Langfuse interceptor.
#[derive(Debug, Clone)]
pub struct LangfuseConfig {
    /// Langfuse API host
    pub host: String,
    /// Langfuse public key
    pub public_key: String,
    /// Langfuse secret key
    pub secret_key: String,
    /// Session ID for grouping related traces
    pub session_id: Option<String>,
    /// User ID for attribution
    pub user_id: Option<String>,
    /// Release version
    pub release: Option<String>,
    /// Timeout for exporting spans
    pub timeout: Duration,
    /// Maximum batch size for exporting spans
    pub batch_size: usize,
    /// Interval between batch exports
    pub export_interval: Duration,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("LANGFUSE_HOST")
                .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string()),
            public_key: std::env::var("LANGFUSE_PUBLIC_KEY").unwrap_or_default(),
            secret_key: std::env::var("LANGFUSE_SECRET_KEY").unwrap_or_default(),
            session_id: std::env::var("LANGFUSE_SESSION_ID").ok(),
            user_id: std::env::var("LANGFUSE_USER_ID").ok(),
            release: std::env::var("LANGFUSE_RELEASE").ok(),
            timeout: Duration::from_secs(10),
            batch_size: 100,
            export_interval: Duration::from_secs(5),
            debug: std::env::var("LANGFUSE_DEBUG")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}

impl LangfuseConfig {
    /// Create a new configuration with the given credentials.
    pub fn new(
        host: impl Into<String>,
        public_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            public_key: public_key.into(),
            secret_key: secret_key.into(),
            ..Default::default()
        }
    }

    /// Set the session ID.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the user ID.
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the release version.
    #[must_use]
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    /// Set the timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable debug logging.
    #[must_use]
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

/// Langfuse interceptor for OpenTelemetry-based observability.
///
/// This interceptor captures OpenAI API interactions and sends them to Langfuse
/// as OpenTelemetry spans.
pub struct LangfuseInterceptor {
    config: LangfuseConfig,
    tracer_provider: Arc<SdkTracerProvider>,
}

impl LangfuseInterceptor {
    /// Create a new Langfuse interceptor with the given configuration.
    pub fn new(config: LangfuseConfig) -> Result<Self> {
        // Validate configuration
        if config.public_key.is_empty() || config.secret_key.is_empty() {
            return Err(crate::Error::Config(
                "Langfuse public_key and secret_key must be provided".to_string(),
            ));
        }

        // Build the Langfuse exporter
        let exporter = ExporterBuilder::new()
            .with_host(&config.host)
            .with_basic_auth(&config.public_key, &config.secret_key)
            .with_timeout(config.timeout)
            .build()
            .map_err(|e| {
                crate::Error::Config(format!("Failed to build Langfuse exporter: {}", e))
            })?;

        // Create the batch span processor with proper configuration and Tokio runtime
        let batch_config = BatchConfigBuilder::default()
            .with_max_export_batch_size(config.batch_size)
            .with_scheduled_delay(config.export_interval)
            .with_max_export_timeout(config.timeout)
            .build();

        let span_processor = BatchSpanProcessor::builder(exporter, Tokio)
            .with_batch_config(batch_config)
            .build();

        // Build the tracer provider with resource attributes
        let mut resource_attrs = vec![
            KeyValue::new("service.name", "openai-ergonomic"),
            KeyValue::new("langfuse.public_key", config.public_key.clone()),
        ];

        if let Some(ref session_id) = config.session_id {
            resource_attrs.push(KeyValue::new("langfuse.session_id", session_id.clone()));
        }
        if let Some(ref user_id) = config.user_id {
            resource_attrs.push(KeyValue::new("langfuse.user_id", user_id.clone()));
        }
        if let Some(ref release) = config.release {
            resource_attrs.push(KeyValue::new("service.version", release.clone()));
        }

        let resource = Resource::builder().with_attributes(resource_attrs).build();

        let provider = SdkTracerProvider::builder()
            .with_span_processor(span_processor)
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn)
            .with_resource(resource)
            .build();

        if config.debug {
            info!(
                "Langfuse interceptor initialized with host: {}",
                config.host
            );
        }

        Ok(Self {
            config,
            tracer_provider: Arc::new(provider),
        })
    }

    /// Create a new interceptor from environment variables.
    pub fn from_env() -> Result<Self> {
        Self::new(LangfuseConfig::default())
    }

    /// Get a tracer for creating spans.
    fn tracer(&self) -> opentelemetry_sdk::trace::Tracer {
        self.tracer_provider.tracer("openai-ergonomic-langfuse")
    }

    /// Extract request parameters from JSON.
    fn extract_request_params(request_json: &str) -> serde_json::Result<Value> {
        serde_json::from_str(request_json)
    }
}

#[async_trait::async_trait]
impl Interceptor for LangfuseInterceptor {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        use opentelemetry::trace::{TraceContextExt};

        let tracer = self.tracer();

        // Get the current OpenTelemetry context
        let parent_cx = opentelemetry::Context::current();

        // Check if we need to create a root trace
        let needs_root = !parent_cx.span().span_context().is_valid();

        let (span_cx, mut span) = if needs_root {
            // Create a root trace for this operation
            let mut root_attrs = vec![
                KeyValue::new("service.name", "openai-ergonomic"),
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
            ];

            // Add session/user/release if available
            if let Some(ref session_id) = self.config.session_id {
                root_attrs.push(KeyValue::new("langfuse.session_id", session_id.clone()));
            }
            if let Some(ref user_id) = self.config.user_id {
                root_attrs.push(KeyValue::new("langfuse.user_id", user_id.clone()));
            }
            if let Some(ref release) = self.config.release {
                root_attrs.push(KeyValue::new("service.version", release.clone()));
            }

            let root_span = tracer
                .span_builder("OpenAI-generation")
                .with_kind(SpanKind::Internal)
                .with_attributes(root_attrs)
                .start(&tracer);

            let root_cx = opentelemetry::Context::current_with_span(root_span);

            // Now create the operation span as a child
            let mut op_attrs = vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
                KeyValue::new("langfuse.observation.type", "generation"),
            ];

            // Add model for Langfuse
            op_attrs.push(KeyValue::new("langfuse.observation.model.name", ctx.model.to_string()));

            let operation_span = tracer
                .span_builder(format!("OpenAI {}", ctx.operation))
                .with_kind(SpanKind::Client)
                .with_attributes(op_attrs)
                .start_with_context(&tracer, &root_cx);

            // Store root context as well
            ctx.metadata.insert("langfuse.root_cx".to_string(), serde_json::to_string(&true).unwrap_or_default());

            (root_cx, operation_span)
        } else {
            // Use existing context, just create child span
            let mut attrs = vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
                KeyValue::new("langfuse.observation.type", "generation"),
                KeyValue::new("langfuse.observation.model.name", ctx.model.to_string()),
            ];

            let span = tracer
                .span_builder(format!("OpenAI {}", ctx.operation))
                .with_kind(SpanKind::Client)
                .with_attributes(attrs)
                .start_with_context(&tracer, &parent_cx);

            (parent_cx, span)
        };

        // Parse request JSON and add input/attributes
        if let Ok(params) = Self::extract_request_params(ctx.request_json) {
            if let Some(temperature) = params.get("temperature").and_then(|v| v.as_f64()) {
                span.set_attribute(KeyValue::new(GEN_AI_REQUEST_TEMPERATURE, temperature));
            }
            if let Some(max_tokens) = params.get("max_tokens").and_then(|v| v.as_i64()) {
                span.set_attribute(KeyValue::new(GEN_AI_REQUEST_MAX_TOKENS, max_tokens));
            }

            // Add observation input for Langfuse (simplified - just include messages or prompt)
            if let Some(messages) = params.get("messages") {
                let input = serde_json::json!({"messages": messages});
                span.set_attribute(KeyValue::new("langfuse.observation.input", input.to_string()));
            } else if let Some(prompt) = params.get("prompt") {
                let input = serde_json::json!({"prompt": prompt});
                span.set_attribute(KeyValue::new("langfuse.observation.input", input.to_string()));
            }
        }

        // Store span context in metadata for after_response
        // We'll use a simple marker - the span will be accessible via the tracer
        ctx.metadata.insert("langfuse.span_started".to_string(), "true".to_string());

        // DON'T end the span yet - it needs to stay alive for the entire operation
        // We'll end it in after_response

        if self.config.debug {
            debug!("Started Langfuse span for operation: {} (span will be ended in after_response)", ctx.operation);
        }

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        // Check if we started a span in before_request
        if ctx.metadata.get("langfuse.span_started").is_none() {
            return Ok(());
        }

        let tracer = self.tracer();

        // Get the current context which should have our span
        let current_cx = opentelemetry::Context::current();

        // Get the current span and add response attributes to it
        let span = current_cx.span();

        // Add duration
        span.set_attribute(KeyValue::new("duration_ms", ctx.duration.as_millis() as i64));

        // Add usage metrics if available
        if let Some(input_tokens) = ctx.input_tokens {
            span.set_attribute(KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            span.set_attribute(KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens));
        }

        // Parse response and add observation output for Langfuse
        if let Ok(response) = Self::extract_request_params(ctx.response_json) {
            // Add response ID if available
            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                span.set_attribute(KeyValue::new(GEN_AI_RESPONSE_ID, id.to_string()));
            }

            // Add observation output for Langfuse
            if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
                if let Some(first_choice) = choices.first() {
                    if let Some(message) = first_choice.get("message") {
                        let output = serde_json::json!({
                            "choices": [{
                                "message": message
                            }]
                        });
                        span.set_attribute(KeyValue::new("langfuse.observation.output", output.to_string()));
                    }
                }
            }

            // Add total tokens if available
            if let Some(usage) = response.get("usage") {
                if let Some(total_tokens) = usage.get("total_tokens").and_then(|v| v.as_i64()) {
                    span.set_attribute(KeyValue::new("langfuse.observation.usage.total", total_tokens));
                }
            }
        }

        span.set_status(Status::Ok);
        span.end();

        // If we created a root trace, end it too
        if ctx.metadata.get("langfuse.root_cx").is_some() {
            // The root span should be the parent of the current span
            // It will be ended when the context is dropped
        }

        if self.config.debug {
            debug!("Completed Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        // For simplicity, we'll just log stream chunks
        // In production, you'd accumulate these and create a single span
        if self.config.debug && ctx.chunk_index % 10 == 0 {
            debug!(
                "Recorded stream chunk {} for operation: {}",
                ctx.chunk_index, ctx.operation
            );
        }
        Ok(())
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        let tracer = self.tracer();

        let mut span = tracer
            .span_builder(format!("{}_stream", ctx.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
                KeyValue::new("stream.total_chunks", ctx.total_chunks as i64),
                KeyValue::new("stream.duration_ms", ctx.duration.as_millis() as i64),
            ])
            .start(&tracer);

        if let Some(input_tokens) = ctx.input_tokens {
            span.set_attributes(vec![KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens)]);
        }
        if let Some(output_tokens) = ctx.output_tokens {
            span.set_attributes(vec![KeyValue::new(
                GEN_AI_USAGE_OUTPUT_TOKENS,
                output_tokens,
            )]);
        }

        span.set_status(Status::Ok);
        span.end();

        if self.config.debug {
            info!(
                "Completed streaming span for operation: {} with {} chunks",
                ctx.operation, ctx.total_chunks
            );
        }

        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        let tracer = self.tracer();

        let mut span = tracer
            .span_builder(format!("{}_error", ctx.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new("error.type", format!("{:?}", ctx.error)),
                KeyValue::new("error.message", ctx.error.to_string()),
            ])
            .start(&tracer);

        if let Some(model) = ctx.model {
            span.set_attributes(vec![KeyValue::new(GEN_AI_REQUEST_MODEL, model.to_string())]);
        }

        span.set_status(Status::error(ctx.error.to_string()));
        span.end();

        if self.config.debug {
            error!(
                "Recorded error for operation {}: {}",
                ctx.operation, ctx.error
            );
        }
    }
}

impl Drop for LangfuseInterceptor {
    fn drop(&mut self) {
        // Force flush any pending spans
        if let Err(e) = self.tracer_provider.force_flush() {
            error!("Failed to flush Langfuse spans on drop: {}", e);
        }
    }
}

/// Builder for creating a Langfuse interceptor with custom configuration.
pub struct LangfuseInterceptorBuilder {
    config: LangfuseConfig,
}

impl LangfuseInterceptorBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: LangfuseConfig::default(),
        }
    }

    /// Set the Langfuse API host.
    #[must_use]
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set the Langfuse credentials.
    #[must_use]
    pub fn with_credentials(
        mut self,
        public_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        self.config.public_key = public_key.into();
        self.config.secret_key = secret_key.into();
        self
    }

    /// Set the session ID.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.config.session_id = Some(session_id.into());
        self
    }

    /// Set the user ID.
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.config.user_id = Some(user_id.into());
        self
    }

    /// Set the release version.
    #[must_use]
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.config.release = Some(release.into());
        self
    }

    /// Set the export timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the batch size.
    #[must_use]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size;
        self
    }

    /// Set the export interval.
    #[must_use]
    pub fn with_export_interval(mut self, interval: Duration) -> Self {
        self.config.export_interval = interval;
        self
    }

    /// Enable debug logging.
    #[must_use]
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.config.debug = debug;
        self
    }

    /// Build the Langfuse interceptor.
    pub fn build(self) -> Result<LangfuseInterceptor> {
        LangfuseInterceptor::new(self.config)
    }
}

impl Default for LangfuseInterceptorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("LANGFUSE_HOST", "https://test.langfuse.com");
        std::env::set_var("LANGFUSE_PUBLIC_KEY", "pk-test");
        std::env::set_var("LANGFUSE_SECRET_KEY", "sk-test");

        let config = LangfuseConfig::default();
        assert_eq!(config.host, "https://test.langfuse.com");
        assert_eq!(config.public_key, "pk-test");
        assert_eq!(config.secret_key, "sk-test");

        // Cleanup
        std::env::remove_var("LANGFUSE_HOST");
        std::env::remove_var("LANGFUSE_PUBLIC_KEY");
        std::env::remove_var("LANGFUSE_SECRET_KEY");
    }

    #[test]
    fn test_builder_configuration() {
        let config = LangfuseConfig::new("https://custom.langfuse.com", "pk-custom", "sk-custom")
            .with_session_id("session-123")
            .with_user_id("user-456")
            .with_release("v1.0.0")
            .with_timeout(Duration::from_secs(30))
            .with_debug(true);

        assert_eq!(config.host, "https://custom.langfuse.com");
        assert_eq!(config.public_key, "pk-custom");
        assert_eq!(config.secret_key, "sk-custom");
        assert_eq!(config.session_id, Some("session-123".to_string()));
        assert_eq!(config.user_id, Some("user-456".to_string()));
        assert_eq!(config.release, Some("v1.0.0".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.debug);
    }
}
