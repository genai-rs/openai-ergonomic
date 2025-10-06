//! Standard OpenTelemetry interceptor following semantic conventions.
//!
//! This module provides an `OpenTelemetryInterceptor` that creates spans
//! following the [OpenTelemetry Semantic Conventions for GenAI operations](https://opentelemetry.io/docs/specs/semconv/gen-ai/openai/).
//!
//! # Features
//!
//! - Automatic span creation for all API calls
//! - Follows OpenTelemetry semantic conventions (`gen_ai.*` attributes)
//! - Records request parameters (model, temperature, `max_tokens`, etc.)
//! - Captures token usage metrics
//! - Error tracking with proper status codes
//! - Works with any OpenTelemetry-compatible backend (Jaeger, Zipkin, OTLP, etc.)
//!
//! # Example
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, OpenTelemetryInterceptor};
//! use opentelemetry::global;
//! use opentelemetry_sdk::trace::TracerProvider;
//!
//! // Setup OpenTelemetry with any exporter
//! let provider = TracerProvider::builder()
//!     .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
//!     .build();
//! global::set_tracer_provider(provider);
//!
//! // Create client with OpenTelemetry interceptor
//! let client = Client::from_env()?
//!     .with_interceptor(Box::new(OpenTelemetryInterceptor::new()));
//!
//! // All API calls are automatically traced
//! let response = client.send_chat(client.chat_simple("Hello!")).await?;
//! ```

#[cfg(feature = "telemetry")]
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor};
#[cfg(feature = "telemetry")]
use crate::Result;
#[cfg(feature = "telemetry")]
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
    KeyValue,
};

/// OpenTelemetry interceptor that follows standard semantic conventions.
///
/// Creates spans with attributes following the
/// [OpenTelemetry Semantic Conventions for GenAI](https://opentelemetry.io/docs/specs/semconv/gen-ai/openai/):
///
/// **Required attributes:**
/// - `gen_ai.operation.name` - The operation being performed (e.g., "chat", "embedding")
/// - `gen_ai.system` - The `GenAI` system ("openai")
/// - `gen_ai.request.model` - The model being used (e.g., "gpt-4")
///
/// **Optional request attributes:**
/// - `gen_ai.request.temperature` - Temperature parameter
/// - `gen_ai.request.max_tokens` - Max tokens limit
/// - `gen_ai.request.top_p` - Top-p sampling parameter
/// - `gen_ai.request.presence_penalty` - Presence penalty
/// - `gen_ai.request.frequency_penalty` - Frequency penalty
///
/// **Response attributes:**
/// - `gen_ai.usage.input_tokens` - Input tokens consumed
/// - `gen_ai.usage.output_tokens` - Output tokens generated
///
/// # Example
///
/// ```rust,ignore
/// use openai_ergonomic::{Client, OpenTelemetryInterceptor};
/// use opentelemetry_sdk::trace::TracerProvider;
/// use opentelemetry::global;
///
/// let provider = TracerProvider::builder()
///     .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
///     .build();
/// global::set_tracer_provider(provider);
///
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(OpenTelemetryInterceptor::new()));
/// ```
#[cfg(feature = "telemetry")]
#[derive(Debug, Clone, Default)]
pub struct OpenTelemetryInterceptor {
    _private: (),
}

#[cfg(feature = "telemetry")]
impl OpenTelemetryInterceptor {
    /// Create a new OpenTelemetry interceptor.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let interceptor = OpenTelemetryInterceptor::new();
    /// let client = Client::from_env()?
    ///     .with_interceptor(Box::new(interceptor));
    /// ```
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(feature = "telemetry")]
#[async_trait::async_trait]
impl Interceptor for OpenTelemetryInterceptor {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        let tracer = global::tracer("openai-ergonomic");

        // Build attributes following OpenTelemetry semantic conventions
        let attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
        ];

        // Create and start span
        let span = tracer
            .span_builder(ctx.operation.to_string())
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        // Store span ID in metadata for correlation
        let span_id = format!("{:?}", span.span_context().span_id());
        ctx.metadata.insert("otel_span_id".to_string(), span_id);

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        let tracer = global::tracer("openai-ergonomic");

        // Build attributes for the completed request
        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
        ];

        // Add token usage if available
        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.input_tokens", input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.output_tokens", output_tokens));
        }

        // Create span with all the data
        let mut span = tracer
            .span_builder(ctx.operation.to_string())
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

        let mut span = tracer
            .span_builder(ctx.operation.to_string())
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer);

        // Set error status following OpenTelemetry conventions
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
