//! Langfuse middleware for LLM observability via OpenTelemetry.
//!
//! This middleware implements proper Langfuse tracing with:
//! - Root trace creation when needed
//! - Child span (observation) for each operation
//! - Full lifecycle span management
//! - Langfuse-specific attributes

use crate::middleware::{Middleware, MiddlewareRequest, MiddlewareResponse, Next};
use crate::Result;
use async_trait::async_trait;
use opentelemetry::{
    trace::{FutureExt, Span, SpanKind, Status, TraceContextExt, Tracer, TracerProvider as _},
    Context, KeyValue,
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
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Configuration for Langfuse middleware.
#[derive(Debug, Clone)]
pub struct LangfuseConfig {
    /// Langfuse API host
    pub host: String,
    /// Langfuse public key
    pub public_key: String,
    /// Langfuse secret key
    pub secret_key: String,
    /// Session ID for grouping traces
    pub session_id: Option<String>,
    /// User ID for attribution
    pub user_id: Option<String>,
    /// Release version
    pub release: Option<String>,
    /// Timeout for exporting spans
    pub timeout: Duration,
    /// Batch size for exports
    pub batch_size: usize,
    /// Export interval
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
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
        }
    }
}

/// Langfuse middleware for OpenTelemetry-based observability.
pub struct LangfuseMiddleware {
    config: LangfuseConfig,
    tracer_provider: Arc<SdkTracerProvider>,
}

impl LangfuseMiddleware {
    /// Create a new Langfuse middleware with the given configuration.
    pub fn new(config: LangfuseConfig) -> Result<Self> {
        // Validate
        if config.public_key.is_empty() || config.secret_key.is_empty() {
            return Err(crate::Error::Config(
                "Langfuse public_key and secret_key required".to_string(),
            ));
        }

        // Build exporter
        let exporter = ExporterBuilder::new()
            .with_host(&config.host)
            .with_basic_auth(&config.public_key, &config.secret_key)
            .with_timeout(config.timeout)
            .build()
            .map_err(|e| {
                crate::Error::Config(format!("Failed to build Langfuse exporter: {e}"))
            })?;

        // Create batch processor
        let batch_config = BatchConfigBuilder::default()
            .with_max_export_batch_size(config.batch_size)
            .with_scheduled_delay(config.export_interval)
            .with_max_export_timeout(config.timeout)
            .build();

        let span_processor = BatchSpanProcessor::builder(exporter, Tokio)
            .with_batch_config(batch_config)
            .build();

        // Build resource
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
            info!("Langfuse middleware initialized with host: {}", config.host);
        }

        Ok(Self {
            config,
            tracer_provider: Arc::new(provider),
        })
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self> {
        Self::new(LangfuseConfig::default())
    }

    fn tracer(&self) -> opentelemetry_sdk::trace::Tracer {
        self.tracer_provider.tracer("openai-ergonomic-langfuse")
    }
}

#[async_trait]
impl Middleware for LangfuseMiddleware {
    async fn handle(
        &self,
        req: MiddlewareRequest<'_>,
        next: Next<'_>,
    ) -> Result<MiddlewareResponse> {
        let tracer = self.tracer();
        let start_time = Instant::now();

        // Get current context
        let parent_cx = Context::current();
        let needs_root = !parent_cx.span().span_context().is_valid();

        // Parse request for attributes
        let request_json = serde_json::from_str::<Value>(req.request_json);

        if needs_root {
            // Create root trace
            let mut root_attrs = vec![
                KeyValue::new("service.name", "openai-ergonomic"),
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
            ];

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

            let root_cx = Context::current_with_span(root_span);

            // Create observation span as child
            let result = self
                .handle_with_observation(req, next, &tracer, request_json, start_time)
                .with_context(root_cx.clone())
                .await;

            // End root span
            root_cx.span().end();

            result
        } else {
            // Just create observation span
            self.handle_with_observation(req, next, &tracer, request_json, start_time)
                .await
        }
    }
}

impl LangfuseMiddleware {
    async fn handle_with_observation(
        &self,
        req: MiddlewareRequest<'_>,
        next: Next<'_>,
        tracer: &opentelemetry_sdk::trace::Tracer,
        request_json: std::result::Result<Value, serde_json::Error>,
        start_time: Instant,
    ) -> Result<MiddlewareResponse> {
        // Save operation name before moving req
        let operation = req.operation.to_string();

        // Create observation span
        let mut attrs = vec![
            KeyValue::new(GEN_AI_SYSTEM, "openai"),
            KeyValue::new(GEN_AI_OPERATION_NAME, operation.clone()),
            KeyValue::new(GEN_AI_REQUEST_MODEL, req.model.to_string()),
            KeyValue::new("langfuse.observation.type", "generation"),
            KeyValue::new("langfuse.observation.model.name", req.model.to_string()),
        ];

        // Add request attributes
        if let Ok(ref params) = request_json {
            if let Some(temp) = params.get("temperature").and_then(serde_json::Value::as_f64) {
                attrs.push(KeyValue::new(GEN_AI_REQUEST_TEMPERATURE, temp));
            }
            if let Some(max_tokens) = params.get("max_tokens").and_then(serde_json::Value::as_i64) {
                attrs.push(KeyValue::new(GEN_AI_REQUEST_MAX_TOKENS, max_tokens));
            }

            // Add observation input
            if let Some(messages) = params.get("messages") {
                let input = serde_json::json!({"messages": messages});
                attrs.push(KeyValue::new(
                    "langfuse.observation.input",
                    input.to_string(),
                ));
            } else if let Some(prompt) = params.get("prompt") {
                let input = serde_json::json!({"prompt": prompt});
                attrs.push(KeyValue::new(
                    "langfuse.observation.input",
                    input.to_string(),
                ));
            }
        }

        let mut span = tracer
            .span_builder(format!("OpenAI {operation}"))
            .with_kind(SpanKind::Client)
            .with_attributes(attrs)
            .start(tracer);

        // Execute request
        let response = next.run(req).await;

        // Add response attributes
        match &response {
            Ok(resp) => {
                #[allow(clippy::cast_possible_truncation)]
                span.set_attribute(KeyValue::new(
                    "duration_ms",
                    start_time.elapsed().as_millis() as i64,
                ));

                if let Some(input_tokens) = resp.input_tokens {
                    span.set_attribute(KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input_tokens));
                }
                if let Some(output_tokens) = resp.output_tokens {
                    span.set_attribute(KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens));
                }

                // Parse response
                if let Ok(response_json) = serde_json::from_str::<Value>(&resp.response_json) {
                    if let Some(id) = response_json.get("id").and_then(|v| v.as_str()) {
                        span.set_attribute(KeyValue::new(GEN_AI_RESPONSE_ID, id.to_string()));
                    }

                    // Add observation output
                    if let Some(choices) = response_json.get("choices").and_then(|v| v.as_array()) {
                        if let Some(first_choice) = choices.first() {
                            if let Some(message) = first_choice.get("message") {
                                let output = serde_json::json!({
                                    "choices": [{
                                        "message": message
                                    }]
                                });
                                span.set_attribute(KeyValue::new(
                                    "langfuse.observation.output",
                                    output.to_string(),
                                ));
                            }
                        }
                    }

                    // Total tokens
                    if let Some(usage) = response_json.get("usage") {
                        if let Some(total) = usage.get("total_tokens").and_then(serde_json::Value::as_i64) {
                            span.set_attribute(KeyValue::new(
                                "langfuse.observation.usage.total",
                                total,
                            ));
                        }
                    }
                }

                span.set_status(Status::Ok);
            }
            Err(e) => {
                span.set_status(Status::error(format!("Error: {e}")));
            }
        }

        span.end();

        if self.config.debug {
            debug!("Langfuse span completed for operation: {}", operation);
        }

        response
    }
}
