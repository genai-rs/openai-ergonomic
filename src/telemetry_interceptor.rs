//! OpenTelemetry interceptor for automatic instrumentation.
//!
//! This module provides a `TelemetryInterceptor` that implements the `Interceptor` trait
//! to automatically create OpenTelemetry spans for API calls.

#[cfg(feature = "telemetry")]
use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext,
};
#[cfg(feature = "telemetry")]
use crate::telemetry::TelemetryContext;
#[cfg(feature = "telemetry")]
use crate::Result;
#[cfg(feature = "telemetry")]
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
    KeyValue,
};

/// OpenTelemetry interceptor for automatic span creation.
///
/// This interceptor creates spans following the `OpenAI` semantic conventions
/// and Langfuse-specific attributes.
///
/// # Example
///
/// ```rust,ignore
/// use openai_ergonomic::{Client, TelemetryInterceptor};
/// use opentelemetry_langfuse::ExporterBuilder;
/// use opentelemetry_sdk::runtime::Tokio;
/// use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
/// use opentelemetry_sdk::trace::SdkTracerProvider;
///
/// // Setup OpenTelemetry
/// let exporter = ExporterBuilder::from_env()?.build()?;
/// let provider = SdkTracerProvider::builder()
///     .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
///     .build();
/// global::set_tracer_provider(provider.clone());
///
/// // Create client with telemetry interceptor
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(TelemetryInterceptor::new()))
///     .build();
/// ```
#[cfg(feature = "telemetry")]
pub struct TelemetryInterceptor {
    telemetry_ctx: Option<TelemetryContext>,
}

#[cfg(feature = "telemetry")]
impl TelemetryInterceptor {
    /// Create a new telemetry interceptor.
    pub fn new() -> Self {
        Self {
            telemetry_ctx: None,
        }
    }

    /// Create a new telemetry interceptor with context.
    pub fn with_context(telemetry_ctx: TelemetryContext) -> Self {
        Self {
            telemetry_ctx: Some(telemetry_ctx),
        }
    }
}

#[cfg(feature = "telemetry")]
impl Default for TelemetryInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "telemetry")]
#[async_trait::async_trait]
impl Interceptor for TelemetryInterceptor {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        let tracer = global::tracer("openai-ergonomic");

        // Build attributes
        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
            KeyValue::new("langfuse.observation.input", ctx.request_json.to_string()),
        ];

        // Add telemetry context attributes if provided
        if let Some(tel_ctx) = &self.telemetry_ctx {
            if let Some(user_id) = &tel_ctx.user_id {
                attributes.push(KeyValue::new("langfuse.userId", user_id.clone()));
            }
            if let Some(session_id) = &tel_ctx.session_id {
                attributes.push(KeyValue::new("langfuse.sessionId", session_id.clone()));
            }
            if !tel_ctx.tags.is_empty() {
                attributes.push(KeyValue::new("langfuse.tags", tel_ctx.tags.join(",")));
            }
            for (key, value) in &tel_ctx.metadata {
                attributes.push(KeyValue::new(
                    format!("langfuse.metadata.{key}"),
                    value.clone(),
                ));
            }
        }

        // Create and start span
        let span = tracer
            .span_builder(ctx.operation.to_string())
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        // Store span in metadata so we can access it in after_response
        let span_id = format!("{:?}", span.span_context().span_id());
        ctx.metadata.insert("otel_span_id".to_string(), span_id);

        // We need to keep the span alive, so store it in the context
        // This is a workaround since we can't easily pass the span between hooks
        ctx.metadata
            .insert("otel_span_started".to_string(), "true".to_string());

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        // In a real implementation, we'd retrieve the span and set attributes
        // For now, we'll create a new span and immediately end it with the response data
        let tracer = global::tracer("openai-ergonomic");

        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
            KeyValue::new("langfuse.observation.input", ctx.request_json.to_string()),
            KeyValue::new("langfuse.observation.output", ctx.response_json.to_string()),
        ];

        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.input_tokens", input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.output_tokens", output_tokens));
        }

        // Create and immediately end a span with all the data
        let mut span = tracer
            .span_builder(ctx.operation.to_string())
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        span.end();

        Ok(())
    }

    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        // For streaming, we could emit events on the span
        // This is a simplified implementation
        let _tracer = global::tracer("openai-ergonomic");

        // TODO: Implement streaming span updates
        tracing::debug!(
            operation = ctx.operation,
            chunk_index = ctx.chunk_index,
            "Stream chunk received"
        );

        Ok(())
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        // Create a final span for the streaming operation
        let tracer = global::tracer("openai-ergonomic");

        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
            KeyValue::new("langfuse.observation.input", ctx.request_json.to_string()),
            KeyValue::new("stream.total_chunks", ctx.total_chunks as i64),
        ];

        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.input_tokens", input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.output_tokens", output_tokens));
        }

        let mut span = tracer
            .span_builder(format!("{}.stream", ctx.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        span.end();

        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        let tracer = global::tracer("openai-ergonomic");

        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
        ];

        if let Some(model) = ctx.model {
            attributes.push(KeyValue::new("gen_ai.request.model", model.to_string()));
        }
        if let Some(request_json) = ctx.request_json {
            attributes.push(KeyValue::new(
                "langfuse.observation.input",
                request_json.to_string(),
            ));
        }

        let mut span = tracer
            .span_builder(ctx.operation.to_string())
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        // Set error status
        span.set_status(Status::Error {
            description: ctx.error.to_string().into(),
        });

        // Add exception event
        span.add_event(
            "exception",
            vec![
                KeyValue::new("exception.message", ctx.error.to_string()),
                KeyValue::new("exception.type", "api_error"),
            ],
        );

        span.end();
    }
}
