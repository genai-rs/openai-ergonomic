//! Langfuse interceptor for OpenTelemetry-based LLM observability.
//!
//! This module provides an example Langfuse interceptor that demonstrates how to use
//! OpenTelemetry with task-local span storage for proper span lifecycle management.
//!
//! # Important: Using with Span Storage
//!
//! To use this interceptor, you must wrap your API calls with `span_storage::with_storage`:
//!
//! ```no_run
//! use opentelemetry_langfuse::span_storage;
//! # use openai_ergonomic::Client;
//! # use openai_ergonomic::langfuse_interceptor::LangfuseInterceptor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let interceptor = LangfuseInterceptor::from_env()?;
//! let client = Client::new("api-key").with_interceptor(interceptor);
//!
//! // IMPORTANT: Wrap API calls in with_storage
//! let response = span_storage::with_storage(async {
//!     client.chat()
//!         .model("gpt-4")
//!         .messages(vec![/* ... */])
//!         .create()
//!         .await
//! }).await?;
//! # Ok(())
//! # }
//! ```

use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext,
};
use crate::Result;
use opentelemetry::{
    trace::{SpanKind, TracerProvider as _},
    KeyValue,
};
use opentelemetry_langfuse::{span_storage, ExporterBuilder};
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{
        span_processor_with_async_runtime::BatchSpanProcessor, RandomIdGenerator, Sampler,
        SdkTracerProvider,
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
    /// Timeout for exporting spans
    pub timeout: Duration,
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
            timeout: Duration::from_secs(10),
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
/// This interceptor demonstrates how to use task-local span storage to maintain
/// a single span across `before_request` and `after_response` calls.
///
/// **Important**: Client code must wrap API calls in `span_storage::with_storage`.
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
                crate::Error::Config(format!("Failed to build Langfuse exporter: {e}"))
            })?;

        // Create the batch span processor with Tokio runtime
        let span_processor = BatchSpanProcessor::builder(exporter, Tokio).build();

        // Build the tracer provider
        let resource = Resource::builder()
            .with_attributes(vec![
                KeyValue::new("service.name", "openai-ergonomic"),
                KeyValue::new("langfuse.public_key", config.public_key.clone()),
            ])
            .build();

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

        // Build initial attributes
        let mut attributes = vec![
            KeyValue::new(GEN_AI_SYSTEM, "openai"),
            KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
            KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
        ];

        // Add Langfuse context attributes if available
        attributes.extend(opentelemetry_langfuse::context::GLOBAL_CONTEXT.get_attributes());

        // Parse request JSON and add relevant attributes
        if let Ok(params) = Self::extract_request_params(ctx.request_json) {
            if let Some(temperature) = params.get("temperature").and_then(serde_json::Value::as_f64) {
                attributes.push(KeyValue::new(GEN_AI_REQUEST_TEMPERATURE, temperature));
            }
            if let Some(max_tokens) = params.get("max_tokens").and_then(serde_json::Value::as_i64) {
                attributes.push(KeyValue::new(GEN_AI_REQUEST_MAX_TOKENS, max_tokens));
            }

            // Add messages as gen_ai.prompt attributes
            if let Some(messages) = params.get("messages").and_then(serde_json::Value::as_array) {
                for (i, message) in messages.iter().enumerate() {
                    if let Some(obj) = message.as_object() {
                        let role = obj
                            .get("role")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        let content = obj
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                            .to_string();

                        attributes.push(KeyValue::new(format!("gen_ai.prompt.{i}.role"), role));
                        attributes.push(KeyValue::new(
                            format!("gen_ai.prompt.{i}.content"),
                            content,
                        ));
                    }
                }
            }
        }

        // Create and store the span using task-local storage
        span_storage::create_and_store_span(
            &tracer,
            ctx.operation.to_string(),
            SpanKind::Client,
            attributes,
        );

        if self.config.debug {
            debug!("Started Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        // Add response attributes to the existing span
        #[allow(clippy::cast_possible_truncation)]
        let mut attributes = vec![KeyValue::new(
            "duration_ms",
            ctx.duration.as_millis() as i64,
        )];

        // Add usage metrics if available
        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens));
        }

        // Parse response and add completion content
        if let Ok(response) = Self::extract_request_params(ctx.response_json) {
            // Add response ID if available
            if let Some(id) = response.get("id").and_then(serde_json::Value::as_str) {
                attributes.push(KeyValue::new(GEN_AI_RESPONSE_ID, id.to_string()));
            }

            // Add completion content
            if let Some(choices) = response.get("choices").and_then(serde_json::Value::as_array) {
                for (i, choice) in choices.iter().enumerate() {
                    if let Some(message) = choice.get("message") {
                        if let Some(role) = message.get("role").and_then(serde_json::Value::as_str) {
                            attributes.push(KeyValue::new(
                                format!("gen_ai.completion.{i}.role"),
                                role.to_string(),
                            ));
                        }
                        if let Some(content) = message.get("content").and_then(serde_json::Value::as_str) {
                            attributes.push(KeyValue::new(
                                format!("gen_ai.completion.{i}.content"),
                                content.to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // Add attributes and end the span
        span_storage::add_span_attributes(attributes);
        span_storage::end_span_with_attributes(vec![]);

        if self.config.debug {
            debug!("Completed Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_>) -> Result<()> {
        // Stream chunks can add attributes to the current span if needed
        // For now, we just let them pass through
        Ok(())
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        // Similar to after_response, add final streaming attributes
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let mut attributes = vec![
            KeyValue::new("stream.total_chunks", ctx.total_chunks as i64),
            KeyValue::new("stream.duration_ms", ctx.duration.as_millis() as i64),
        ];

        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens));
        }

        span_storage::add_span_attributes(attributes);
        span_storage::end_span_with_attributes(vec![]);

        if self.config.debug {
            info!(
                "Completed streaming span for operation: {} with {} chunks",
                ctx.operation, ctx.total_chunks
            );
        }

        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        // Add error attributes to the span if it exists
        let attributes = vec![
            KeyValue::new("error.type", format!("{:?}", ctx.error)),
            KeyValue::new("error.message", ctx.error.to_string()),
        ];

        if let Some(model) = ctx.model {
            span_storage::add_span_attributes(vec![KeyValue::new(
                GEN_AI_REQUEST_MODEL,
                model.to_string(),
            )]);
        }

        span_storage::add_span_attributes(attributes);
        span_storage::end_span_with_attributes(vec![]);

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

    /// Set the export timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
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
            .with_timeout(Duration::from_secs(30))
            .with_debug(true);

        assert_eq!(config.host, "https://custom.langfuse.com");
        assert_eq!(config.public_key, "pk-custom");
        assert_eq!(config.secret_key, "sk-custom");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.debug);
    }
}
