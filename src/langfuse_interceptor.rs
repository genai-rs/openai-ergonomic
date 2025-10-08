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
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the user ID.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the release version.
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable debug logging.
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
        let tracer = self.tracer();

        // Create a new context and span
        let mut span = tracer
            .span_builder(format!("{}_request", ctx.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
            ])
            .start(&tracer);

        // Parse request JSON and add relevant attributes if possible
        if let Ok(params) = Self::extract_request_params(ctx.request_json) {
            if let Some(temperature) = params.get("temperature").and_then(|v| v.as_f64()) {
                span.set_attributes(vec![KeyValue::new(GEN_AI_REQUEST_TEMPERATURE, temperature)]);
            }
            if let Some(max_tokens) = params.get("max_tokens").and_then(|v| v.as_i64()) {
                span.set_attributes(vec![KeyValue::new(GEN_AI_REQUEST_MAX_TOKENS, max_tokens)]);
            }

            // Add messages as gen_ai.prompt attributes for Langfuse
            if let Some(messages) = params.get("messages").and_then(|v| v.as_array()) {
                let mut prompt_attrs = Vec::new();
                for (i, message) in messages.iter().enumerate() {
                    if let Some(obj) = message.as_object() {
                        let role = obj
                            .get("role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let content = obj
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Set gen_ai.prompt attributes as expected by Langfuse
                        prompt_attrs.push(KeyValue::new(format!("gen_ai.prompt.{}.role", i), role));
                        prompt_attrs.push(KeyValue::new(
                            format!("gen_ai.prompt.{}.content", i),
                            content,
                        ));
                    }
                }
                if !prompt_attrs.is_empty() {
                    span.set_attributes(prompt_attrs);
                }
            }
        }

        // End the span immediately - in production, you'd store it for later
        span.end();

        if self.config.debug {
            debug!("Started Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        let tracer = self.tracer();

        // Create a span for the response
        let mut span = tracer
            .span_builder(format!("{}_response", ctx.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
                KeyValue::new("duration_ms", ctx.duration.as_millis() as i64),
            ])
            .start(&tracer);

        // Add usage metrics if available
        if let Some(input_tokens) = ctx.input_tokens {
            span.set_attributes(vec![KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens)]);
        }
        if let Some(output_tokens) = ctx.output_tokens {
            span.set_attributes(vec![KeyValue::new(
                GEN_AI_USAGE_OUTPUT_TOKENS,
                output_tokens,
            )]);
        }

        // Parse response and add completion content
        if let Ok(response) = Self::extract_request_params(ctx.response_json) {
            // Add response ID if available
            if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
                span.set_attributes(vec![KeyValue::new(GEN_AI_RESPONSE_ID, id.to_string())]);
            }

            // Add completion content for Langfuse
            if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
                let mut completion_attrs = Vec::new();
                for (i, choice) in choices.iter().enumerate() {
                    if let Some(message) = choice.get("message") {
                        if let Some(role) = message.get("role").and_then(|v| v.as_str()) {
                            completion_attrs.push(KeyValue::new(
                                format!("gen_ai.completion.{}.role", i),
                                role.to_string(),
                            ));
                        }
                        if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                            completion_attrs.push(KeyValue::new(
                                format!("gen_ai.completion.{}.content", i),
                                content.to_string(),
                            ));
                        }
                    }
                }
                if !completion_attrs.is_empty() {
                    span.set_attributes(completion_attrs);
                }
            }
        }

        span.set_status(Status::Ok);
        span.end();

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
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set the Langfuse credentials.
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
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.config.session_id = Some(session_id.into());
        self
    }

    /// Set the user ID.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.config.user_id = Some(user_id.into());
        self
    }

    /// Set the release version.
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.config.release = Some(release.into());
        self
    }

    /// Set the export timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size;
        self
    }

    /// Set the export interval.
    pub fn with_export_interval(mut self, interval: Duration) -> Self {
        self.config.export_interval = interval;
        self
    }

    /// Enable debug logging.
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
