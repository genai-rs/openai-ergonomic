//! OpenTelemetry instrumentation for `OpenAI` API calls.
//!
//! This module provides span creation utilities following the `OpenAI` semantic conventions
//! as specified at: <https://opentelemetry.io/docs/specs/semconv/gen-ai/openai/>
//!
//! # Architecture
//!
//! This module does NOT own:
//! - Tracer provider setup
//! - Exporter configuration (Langfuse, OTLP, etc.)
//! - Resource attributes
//! - Trace propagation
//!
//! Users are responsible for setting up the global tracer provider in their application.
//!
//! # Example
//!
//! ```rust,ignore
//! use opentelemetry_langfuse::ExporterBuilder;
//! use opentelemetry_sdk::runtime::Tokio;
//! use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
//! use opentelemetry_sdk::trace::TracerProvider;
//!
//! // User sets up OpenTelemetry once
//! let exporter = ExporterBuilder::from_env()?.build()?;
//! let provider = TracerProvider::builder()
//!     .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
//!     .build();
//! opentelemetry::global::set_tracer_provider(provider);
//!
//! // Client automatically emits spans
//! let client = Client::from_env()?;
//! let response = client.chat_simple("Hello").await?;
//! ```

#[cfg(feature = "telemetry")]
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
    KeyValue,
};

/// Telemetry context for API calls.
///
/// This allows users to attach custom attributes to spans for better observability.
/// These attributes follow Langfuse conventions when applicable.
#[derive(Debug, Clone, Default)]
pub struct TelemetryContext {
    /// User identifier (sets `langfuse.userId`)
    pub user_id: Option<String>,
    /// Session identifier (sets `langfuse.sessionId`)
    pub session_id: Option<String>,
    /// Tags for categorizing traces (sets `langfuse.tags`)
    pub tags: Vec<String>,
    /// Additional metadata as key-value pairs (sets `langfuse.metadata.*`)
    pub metadata: Vec<(String, String)>,
}

impl TelemetryContext {
    /// Create a new empty telemetry context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the user identifier.
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the session identifier.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    #[cfg(feature = "telemetry")]
    fn to_attributes(&self) -> Vec<KeyValue> {
        let mut attrs = Vec::new();

        if let Some(user_id) = &self.user_id {
            attrs.push(KeyValue::new("langfuse.userId", user_id.clone()));
        }

        if let Some(session_id) = &self.session_id {
            attrs.push(KeyValue::new("langfuse.sessionId", session_id.clone()));
        }

        if !self.tags.is_empty() {
            // OpenTelemetry doesn't have native array support in KeyValue
            // Use comma-separated string as workaround
            let tags_str = self.tags.join(",");
            attrs.push(KeyValue::new("langfuse.tags", tags_str));
        }

        for (key, value) in &self.metadata {
            attrs.push(KeyValue::new(
                format!("langfuse.metadata.{key}"),
                value.clone(),
            ));
        }

        attrs
    }
}

/// Helper to create a span for `OpenAI` API operations.
///
/// This follows the semantic conventions specified at:
/// <https://opentelemetry.io/docs/specs/semconv/gen-ai/openai/>
#[cfg(feature = "telemetry")]
pub(crate) struct SpanBuilder {
    operation: String,
    model: Option<String>,
    attributes: Vec<KeyValue>,
}

#[cfg(feature = "telemetry")]
impl SpanBuilder {
    /// Create a new span builder for the given operation.
    pub fn new(operation: impl Into<String>) -> Self {
        let operation = operation.into();
        Self {
            operation,
            model: None,
            attributes: Vec::new(),
        }
    }

    /// Set the model being used.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a custom attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes
            .push(KeyValue::new(key.into(), value.into()));
        self
    }

    /// Add an integer attribute.
    pub fn attribute_i64(mut self, key: impl Into<String>, value: i64) -> Self {
        self.attributes.push(KeyValue::new(key.into(), value));
        self
    }

    /// Add a float attribute.
    pub fn attribute_f64(mut self, key: impl Into<String>, value: f64) -> Self {
        self.attributes.push(KeyValue::new(key.into(), value));
        self
    }

    /// Add telemetry context attributes.
    pub fn context(mut self, ctx: &TelemetryContext) -> Self {
        self.attributes.extend(ctx.to_attributes());
        self
    }

    /// Build and start the span.
    pub fn start(self) -> impl Span {
        let tracer = global::tracer("openai-ergonomic");

        let mut attributes = vec![
            KeyValue::new("gen_ai.operation.name", self.operation.clone()),
            KeyValue::new("gen_ai.system", "openai"),
        ];

        if let Some(model) = self.model {
            attributes.push(KeyValue::new("gen_ai.request.model", model));
        }

        attributes.extend(self.attributes);

        tracer
            .span_builder(self.operation)
            .with_kind(SpanKind::Client)
            .with_attributes(attributes)
            .start(&tracer)
    }
}

/// Record token usage on a span.
#[cfg(feature = "telemetry")]
pub(crate) fn record_token_usage<S: Span>(
    span: &mut S,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) {
    if let Some(tokens) = input_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.input_tokens", tokens));
    }

    if let Some(tokens) = output_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.output_tokens", tokens));
    }
}

/// Record an error on a span.
#[cfg(feature = "telemetry")]
pub(crate) fn record_error<S: Span>(span: &mut S, error: &dyn std::error::Error) {
    span.set_status(Status::Error {
        description: error.to_string().into(),
    });

    // Record exception details
    span.add_event(
        "exception",
        vec![
            KeyValue::new("exception.type", std::any::type_name_of_val(error)),
            KeyValue::new("exception.message", error.to_string()),
        ],
    );
}

#[cfg(test)]
#[cfg(feature = "telemetry")]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_context_builder() {
        let ctx = TelemetryContext::new()
            .with_user_id("user-123")
            .with_session_id("session-456")
            .with_tag("production")
            .with_tag("chatbot")
            .with_metadata("region", "us-east-1");

        assert_eq!(ctx.user_id, Some("user-123".to_string()));
        assert_eq!(ctx.session_id, Some("session-456".to_string()));
        assert_eq!(ctx.tags.len(), 2);
        assert_eq!(ctx.metadata.len(), 1);
    }

    #[test]
    fn test_telemetry_context_to_attributes() {
        let ctx = TelemetryContext::new()
            .with_user_id("user-123")
            .with_session_id("session-456")
            .with_tags(vec!["prod".to_string(), "chat".to_string()])
            .with_metadata("key", "value");

        let attrs = ctx.to_attributes();

        // Verify attributes are created
        assert!(attrs.iter().any(|kv| kv.key.as_str() == "langfuse.userId"));
        assert!(attrs
            .iter()
            .any(|kv| kv.key.as_str() == "langfuse.sessionId"));
        assert!(attrs.iter().any(|kv| kv.key.as_str() == "langfuse.tags"));
        assert!(attrs
            .iter()
            .any(|kv| kv.key.as_str() == "langfuse.metadata.key"));
    }
}
