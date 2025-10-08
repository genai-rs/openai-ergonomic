//! Langfuse interceptor for OpenTelemetry-based LLM observability.
//!
//! This interceptor automatically instruments `OpenAI` API calls with OpenTelemetry spans.
//! You must configure the OpenTelemetry tracer with Langfuse exporter separately.
//!
//! # Usage
//!
//! ```no_run
//! # use openai_ergonomic::{Builder, Client};
//! # use openai_ergonomic::langfuse_interceptor::{LangfuseInterceptor, LangfuseConfig};
//! # use opentelemetry_langfuse::ExporterBuilder;
//! # use opentelemetry_sdk::runtime::Tokio;
//! # use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
//! # use opentelemetry_sdk::trace::SdkTracerProvider;
//! # use opentelemetry::global;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Build Langfuse exporter
//! let exporter = ExporterBuilder::from_env()?.build()?;
//!
//! // 2. Create tracer provider
//! let provider = SdkTracerProvider::builder()
//!     .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
//!     .build();
//!
//! global::set_tracer_provider(provider.clone());
//!
//! // 3. Create interceptor with tracer
//! let tracer = provider.tracer("openai-ergonomic");
//! let interceptor = LangfuseInterceptor::new(tracer, LangfuseConfig::new());
//! let client = Client::from_env()?.with_interceptor(Box::new(interceptor));
//!
//! // Traces are automatically sent to Langfuse
//! let request = client.chat_simple("Hello!").build()?;
//! let response = client.execute_chat(request).await?;
//! # Ok(())
//! # }
//! ```

use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext,
};
use crate::Result;
use opentelemetry::{
    trace::{SpanKind, Tracer},
    KeyValue,
};
use opentelemetry_langfuse::{span_storage, LangfuseContext};
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MAX_TOKENS, GEN_AI_REQUEST_MODEL,
    GEN_AI_REQUEST_TEMPERATURE, GEN_AI_RESPONSE_ID, GEN_AI_SYSTEM, GEN_AI_USAGE_INPUT_TOKENS,
    GEN_AI_USAGE_OUTPUT_TOKENS,
};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Configuration for the Langfuse interceptor.
#[derive(Debug, Clone)]
pub struct LangfuseConfig {
    /// Enable debug logging
    pub debug: bool,
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self {
            debug: std::env::var("LANGFUSE_DEBUG")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}

impl LangfuseConfig {
    /// Create a new configuration.
    pub fn new() -> Self {
        Self::default()
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
/// This interceptor automatically creates spans for API calls.
/// Spans are maintained across `before_request` and `after_response` using a global registry
/// and request metadata, requiring no user code changes.
///
/// The tracer must be configured externally - this interceptor only instruments API calls.
pub struct LangfuseInterceptor<T: Tracer + Send + Sync> {
    config: LangfuseConfig,
    tracer: Arc<T>,
    context: Arc<LangfuseContext>,
}

impl<T: Tracer + Send + Sync> LangfuseInterceptor<T>
where
    T::Span: Send + Sync + 'static,
{
    /// Create a new Langfuse interceptor with the given tracer.
    ///
    /// The tracer should be configured to export to Langfuse using
    /// `opentelemetry_langfuse::ExporterBuilder`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use opentelemetry::global;
    /// use opentelemetry_langfuse::ExporterBuilder;
    /// use opentelemetry_sdk::runtime::Tokio;
    /// use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
    /// use opentelemetry_sdk::trace::SdkTracerProvider;
    ///
    /// # async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    /// // Build exporter
    /// let exporter = ExporterBuilder::from_env()?.build()?;
    ///
    /// // Create tracer provider with batch processor
    /// let provider = SdkTracerProvider::builder()
    ///     .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
    ///     .build();
    ///
    /// // Set as global provider
    /// global::set_tracer_provider(provider.clone());
    ///
    /// // Get tracer for interceptor
    /// let tracer = provider.tracer("openai-ergonomic");
    ///
    /// // Create interceptor with tracer
    /// use openai_ergonomic::langfuse_interceptor::{LangfuseInterceptor, LangfuseConfig};
    /// let interceptor = LangfuseInterceptor::new(tracer, LangfuseConfig::new());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(tracer: T, config: LangfuseConfig) -> Self {
        if config.debug {
            info!("Langfuse interceptor initialized");
        }

        Self {
            config,
            tracer: Arc::new(tracer),
            context: Arc::new(LangfuseContext::new()),
        }
    }

    /// Set the session ID for traces created by this interceptor.
    pub fn set_session_id(&self, session_id: impl Into<String>) {
        self.context.set_session_id(session_id);
    }

    /// Set the user ID for traces created by this interceptor.
    pub fn set_user_id(&self, user_id: impl Into<String>) {
        self.context.set_user_id(user_id);
    }

    /// Add tags to traces created by this interceptor.
    pub fn add_tags(&self, tags: Vec<String>) {
        self.context.add_tags(tags);
    }

    /// Add a single tag to traces created by this interceptor.
    pub fn add_tag(&self, tag: impl Into<String>) {
        self.context.add_tag(tag);
    }

    /// Set metadata for traces created by this interceptor.
    pub fn set_metadata(&self, metadata: serde_json::Value) {
        self.context.set_metadata(metadata);
    }

    /// Clear all context attributes.
    pub fn clear_context(&self) {
        self.context.clear();
    }

    /// Get a reference to the Langfuse context.
    pub fn context(&self) -> &Arc<LangfuseContext> {
        &self.context
    }

    /// Extract request parameters from JSON.
    fn extract_request_params(request_json: &str) -> serde_json::Result<Value> {
        serde_json::from_str(request_json)
    }
}

#[async_trait::async_trait]
impl<T: Tracer + Send + Sync> Interceptor<std::collections::HashMap<String, String>> for LangfuseInterceptor<T>
where
    T::Span: Send + Sync + 'static,
{
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        let tracer = self.tracer.as_ref();

        // Build initial attributes
        let mut attributes = vec![
            KeyValue::new(GEN_AI_SYSTEM, "openai"),
            KeyValue::new(GEN_AI_OPERATION_NAME, ctx.operation.to_string()),
            KeyValue::new(GEN_AI_REQUEST_MODEL, ctx.model.to_string()),
        ];

        // Add Langfuse context attributes from this interceptor's context
        attributes.extend(self.context.get_attributes());

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

        // Create and store the span in global registry
        let span_id = span_storage::create_and_store_span(
            tracer,
            ctx.operation.to_string(),
            SpanKind::Client,
            attributes,
        );

        // Store span ID in metadata so after_response can retrieve it
        ctx.state.insert("langfuse_span_id".to_string(), span_id);

        if self.config.debug {
            debug!("Started Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        // Retrieve span ID from metadata
        let Some(span_id) = ctx.state.get("langfuse_span_id") else {
            if self.config.debug {
                debug!("No span ID found in metadata for operation: {}", ctx.operation);
            }
            return Ok(());
        };

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
        span_storage::add_span_attributes(span_id, attributes);
        span_storage::end_span(span_id, vec![]);

        if self.config.debug {
            debug!("Completed Langfuse span for operation: {}", ctx.operation);
        }

        Ok(())
    }

    async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        // Stream chunks can add attributes to the current span if needed
        // For now, we just let them pass through
        Ok(())
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        // Retrieve span ID from metadata
        let Some(span_id) = ctx.state.get("langfuse_span_id") else {
            if self.config.debug {
                debug!("No span ID found in metadata for stream operation: {}", ctx.operation);
            }
            return Ok(());
        };

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

        span_storage::add_span_attributes(span_id, attributes);
        span_storage::end_span(span_id, vec![]);

        if self.config.debug {
            info!(
                "Completed streaming span for operation: {} with {} chunks",
                ctx.operation, ctx.total_chunks
            );
        }

        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_, std::collections::HashMap<String, String>>) {
        // Retrieve span ID from metadata if available
        let Some(metadata) = ctx.state else {
            if self.config.debug {
                debug!("No metadata available for error in operation: {}", ctx.operation);
            }
            return;
        };

        let Some(span_id) = metadata.get("langfuse_span_id") else {
            if self.config.debug {
                debug!("No span ID found in metadata for error in operation: {}", ctx.operation);
            }
            return;
        };

        // Set the span status to error (required for Langfuse to flag it as error)
        span_storage::set_span_error(span_id, ctx.error.to_string());

        // Add error attributes to the span
        let mut attributes = vec![
            KeyValue::new("error.type", format!("{:?}", ctx.error)),
            KeyValue::new("error.message", ctx.error.to_string()),
        ];

        if let Some(model) = ctx.model {
            attributes.push(KeyValue::new(GEN_AI_REQUEST_MODEL, model.to_string()));
        }

        span_storage::add_span_attributes(span_id, attributes);
        span_storage::end_span(span_id, vec![]);

        if self.config.debug {
            error!(
                "Recorded error for operation {}: {}",
                ctx.operation, ctx.error
            );
        }
    }
}

// Implement Interceptor for Arc<LangfuseInterceptor<T>> to allow sharing the interceptor
#[async_trait::async_trait]
impl<T: Tracer + Send + Sync> Interceptor<std::collections::HashMap<String, String>> for Arc<LangfuseInterceptor<T>>
where
    T::Span: Send + Sync + 'static,
{
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        (**self).before_request(ctx).await
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        (**self).after_response(ctx).await
    }

    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        (**self).on_stream_chunk(ctx).await
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_, std::collections::HashMap<String, String>>) -> Result<()> {
        (**self).on_stream_end(ctx).await
    }

    async fn on_error(&self, ctx: &ErrorContext<'_, std::collections::HashMap<String, String>>) {
        (**self).on_error(ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::noop::NoopTracer;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("LANGFUSE_DEBUG", "true");

        let config = LangfuseConfig::default();
        assert!(config.debug);

        // Cleanup
        std::env::remove_var("LANGFUSE_DEBUG");
    }

    #[test]
    fn test_interceptor_creation() {
        let tracer = NoopTracer::new();
        let config = LangfuseConfig::new().with_debug(true);
        let _interceptor = LangfuseInterceptor::new(tracer, config);
        // No assertion needed - just verify it compiles and constructs
    }
}
