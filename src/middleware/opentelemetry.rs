//! Generic OpenTelemetry middleware following `GenAI` semantic conventions.
//!
//! This middleware creates spans with standardized attributes for any OpenTelemetry backend.
//! Follows: <https://opentelemetry.io/docs/specs/semconv/gen-ai/>

use crate::middleware::{Middleware, MiddlewareRequest, MiddlewareResponse, Next};
use crate::Result;
use async_trait::async_trait;
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
    KeyValue,
};
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MAX_TOKENS, GEN_AI_REQUEST_MODEL,
    GEN_AI_REQUEST_TEMPERATURE, GEN_AI_RESPONSE_ID, GEN_AI_SYSTEM, GEN_AI_USAGE_INPUT_TOKENS,
    GEN_AI_USAGE_OUTPUT_TOKENS,
};
use serde_json::Value;
use std::time::Instant;

/// OpenTelemetry middleware for `GenAI` semantic conventions.
///
/// Creates spans with standardized `GenAI` attributes that work with any
/// OpenTelemetry backend (Jaeger, Zipkin, etc.).
pub struct OpenTelemetryMiddleware {
    tracer_name: String,
}

impl OpenTelemetryMiddleware {
    /// Create a new OpenTelemetry middleware.
    pub fn new() -> Self {
        Self {
            tracer_name: "openai-ergonomic".to_string(),
        }
    }

    /// Create with a custom tracer name.
    pub fn with_tracer_name(tracer_name: impl Into<String>) -> Self {
        Self {
            tracer_name: tracer_name.into(),
        }
    }
}

impl Default for OpenTelemetryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for OpenTelemetryMiddleware {
    async fn handle(
        &self,
        req: MiddlewareRequest<'_>,
        next: Next<'_>,
    ) -> Result<MiddlewareResponse> {
        let tracer_name = self.tracer_name.clone();
        let tracer = global::tracer(tracer_name);
        let start_time = Instant::now();

        // Create span with GenAI semantic conventions
        let mut span = tracer
            .span_builder(format!("gen_ai {}", req.operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(GEN_AI_SYSTEM, "openai"),
                KeyValue::new(GEN_AI_OPERATION_NAME, req.operation.to_string()),
                KeyValue::new(GEN_AI_REQUEST_MODEL, req.model.to_string()),
            ])
            .start(&tracer);

        // Add request attributes
        if let Ok(params) = serde_json::from_str::<Value>(req.request_json) {
            if let Some(temp) = params.get("temperature").and_then(Value::as_f64) {
                span.set_attribute(KeyValue::new(GEN_AI_REQUEST_TEMPERATURE, temp));
            }
            if let Some(max_tokens) = params.get("max_tokens").and_then(Value::as_i64) {
                span.set_attribute(KeyValue::new(GEN_AI_REQUEST_MAX_TOKENS, max_tokens));
            }
        }

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

                if let Ok(response_json) = serde_json::from_str::<Value>(&resp.response_json) {
                    if let Some(id) = response_json.get("id").and_then(|v| v.as_str()) {
                        span.set_attribute(KeyValue::new(GEN_AI_RESPONSE_ID, id.to_string()));
                    }
                }

                span.set_status(Status::Ok);
            }
            Err(e) => {
                span.set_status(Status::error(format!("Error: {e}")));
                span.set_attribute(KeyValue::new("error.type", e.to_string()));
            }
        }

        span.end();
        response
    }
}
