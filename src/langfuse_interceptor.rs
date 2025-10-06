//! Langfuse-specific interceptor for LLM observability.
//!
//! This module provides a `LangfuseInterceptor` that creates spans with
//! Langfuse-specific attributes for enhanced observability in the Langfuse platform.
//!
//! # Features
//!
//! - Full OpenTelemetry semantic conventions (`gen_ai.*`)
//! - Langfuse-specific attributes (`langfuse.*`)
//! - User and session tracking (`langfuse.userId`, `langfuse.sessionId`)
//! - Custom tags and metadata
//! - Input/output capture for request/response inspection
//! - Integration with [opentelemetry-langfuse](https://docs.rs/opentelemetry-langfuse)
//!
//! # Langfuse Attributes
//!
//! In addition to standard OpenTelemetry attributes, this interceptor adds:
//!
//! - `langfuse.userId` - User identifier for grouping traces by user
//! - `langfuse.sessionId` - Session identifier for grouping related traces
//! - `langfuse.tags` - Comma-separated tags for categorization
//! - `langfuse.metadata.*` - Custom key-value metadata
//! - `langfuse.observation.input` - Full request JSON
//! - `langfuse.observation.output` - Full response JSON
//!
//! # Example
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, LangfuseInterceptor, TelemetryContext};
//! use opentelemetry_langfuse::ExporterBuilder;
//! use opentelemetry_sdk::runtime::Tokio;
//! use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
//! use opentelemetry_sdk::trace::SdkTracerProvider;
//! use opentelemetry::global;
//!
//! // Setup Langfuse exporter
//! let exporter = ExporterBuilder::from_env()?.build()?;
//! let provider = SdkTracerProvider::builder()
//!     .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
//!     .build();
//! global::set_tracer_provider(provider.clone());
//!
//! // Create client with Langfuse interceptor
//! let context = TelemetryContext::new()
//!     .with_user_id("user-123")
//!     .with_session_id("session-456")
//!     .with_tag("production");
//!
//! let client = Client::from_env()?
//!     .with_interceptor(Box::new(LangfuseInterceptor::with_context(context)));
//!
//! // All API calls are traced with Langfuse attributes
//! let response = client.send_chat(client.chat_simple("Hello!")).await?;
//! ```
//!
//! # Environment Variables
//!
//! When using with `opentelemetry-langfuse`:
//!
//! ```bash
//! export LANGFUSE_PUBLIC_KEY="pk-lf-..."
//! export LANGFUSE_SECRET_KEY="sk-lf-..."
//! export LANGFUSE_HOST="https://cloud.langfuse.com"  # optional
//! ```

#[cfg(feature = "telemetry")]
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor};
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

/// Langfuse interceptor for enhanced LLM observability.
///
/// Creates spans with both OpenTelemetry semantic conventions and
/// Langfuse-specific attributes for use with the Langfuse platform.
///
/// # Attributes Created
///
/// **OpenTelemetry (standard):**
/// - `gen_ai.operation.name` - Operation type (e.g., "chat")
/// - `gen_ai.system` - Always "openai"
/// - `gen_ai.request.model` - Model name (e.g., "gpt-4")
/// - `gen_ai.usage.input_tokens` - Input token count
/// - `gen_ai.usage.output_tokens` - Output token count
///
/// **Langfuse (specific):**
/// - `langfuse.userId` - User identifier (if provided)
/// - `langfuse.sessionId` - Session identifier (if provided)
/// - `langfuse.tags` - Comma-separated tags (if provided)
/// - `langfuse.metadata.*` - Custom metadata (if provided)
/// - `langfuse.observation.input` - Request JSON
/// - `langfuse.observation.output` - Response JSON
///
/// # Example
///
/// ```rust,ignore
/// use openai_ergonomic::{Client, LangfuseInterceptor, TelemetryContext};
///
/// // With context
/// let context = TelemetryContext::new()
///     .with_user_id("user-123")
///     .with_session_id("session-abc")
///     .with_tag("production")
///     .with_metadata("region", "us-east-1");
///
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(LangfuseInterceptor::with_context(context)));
///
/// // Without context (just standard attributes)
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(LangfuseInterceptor::new()));
/// ```
#[cfg(feature = "telemetry")]
#[derive(Debug, Clone)]
pub struct LangfuseInterceptor {
    context: Option<TelemetryContext>,
}

#[cfg(feature = "telemetry")]
impl LangfuseInterceptor {
    /// Create a new Langfuse interceptor without context.
    ///
    /// This will create spans with OpenTelemetry semantic conventions
    /// and input/output capture, but without user/session tracking.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let interceptor = LangfuseInterceptor::new();
    /// let client = Client::from_env()?
    ///     .with_interceptor(Box::new(interceptor));
    /// ```
    pub fn new() -> Self {
        Self { context: None }
    }

    /// Create a Langfuse interceptor with telemetry context.
    ///
    /// The context provides user/session tracking, tags, and custom metadata.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let context = TelemetryContext::new()
    ///     .with_user_id("user-123")
    ///     .with_session_id("session-abc")
    ///     .with_tag("production")
    ///     .with_metadata("team", "ml-platform");
    ///
    /// let interceptor = LangfuseInterceptor::with_context(context);
    /// let client = Client::from_env()?
    ///     .with_interceptor(Box::new(interceptor));
    /// ```
    pub fn with_context(context: TelemetryContext) -> Self {
        Self {
            context: Some(context),
        }
    }
}

#[cfg(feature = "telemetry")]
impl Default for LangfuseInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "telemetry")]
#[async_trait::async_trait]
impl Interceptor for LangfuseInterceptor {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
        let tracer = global::tracer("openai-ergonomic");

        // Build attributes: OpenTelemetry + Langfuse
        let mut attributes = vec![
            // OpenTelemetry semantic conventions
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
            // Langfuse input capture
            KeyValue::new("langfuse.observation.input", ctx.request_json.to_string()),
        ];

        // Add Langfuse context attributes if provided
        if let Some(tel_ctx) = &self.context {
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

        // Store span ID for correlation
        let span_id = format!("{:?}", span.span_context().span_id());
        ctx.metadata.insert("otel_span_id".to_string(), span_id);

        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        let tracer = global::tracer("openai-ergonomic");

        // Build attributes for the completed request
        let mut attributes = vec![
            // OpenTelemetry semantic conventions
            KeyValue::new("gen_ai.operation.name", ctx.operation.to_string()),
            KeyValue::new("gen_ai.system", "openai"),
            KeyValue::new("gen_ai.request.model", ctx.model.to_string()),
            // Langfuse input/output capture
            KeyValue::new("langfuse.observation.input", ctx.request_json.to_string()),
            KeyValue::new("langfuse.observation.output", ctx.response_json.to_string()),
        ];

        // Add token usage
        if let Some(input_tokens) = ctx.input_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.input_tokens", input_tokens));
        }
        if let Some(output_tokens) = ctx.output_tokens {
            attributes.push(KeyValue::new("gen_ai.usage.output_tokens", output_tokens));
        }

        // Add Langfuse context attributes if provided
        if let Some(tel_ctx) = &self.context {
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
        if let Some(request_json) = ctx.request_json {
            attributes.push(KeyValue::new(
                "langfuse.observation.input",
                request_json.to_string(),
            ));
        }

        // Add Langfuse context attributes if provided
        if let Some(tel_ctx) = &self.context {
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
