//! Tests for `OpenTelemetryInterceptor`.

use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, OpenTelemetryInterceptor,
};
use std::time::Duration;

#[cfg(feature = "telemetry")]
#[tokio::test]
async fn test_opentelemetry_interceptor_before_request() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let interceptor = OpenTelemetryInterceptor::new();
    let mut ctx = BeforeRequestContext {
        operation: "chat",
        model: "gpt-4",
        request_json: r#"{"model":"gpt-4","messages":[]}"#,
        metadata: std::collections::HashMap::new(),
    };

    // Should not return an error
    let result = interceptor.before_request(&mut ctx).await;
    assert!(result.is_ok());
}

#[cfg(feature = "telemetry")]
#[tokio::test]
async fn test_opentelemetry_interceptor_after_response() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let interceptor = OpenTelemetryInterceptor::new();
    let metadata = std::collections::HashMap::new();
    let ctx = AfterResponseContext {
        operation: "chat",
        model: "gpt-4",
        request_json: r#"{"model":"gpt-4","messages":[]}"#,
        response_json: r#"{"choices":[{"message":{"content":"Hello"}}]}"#,
        duration: Duration::from_secs(1),
        input_tokens: Some(10),
        output_tokens: Some(20),
        metadata: &metadata,
    };

    // Should not return an error
    let result = interceptor.after_response(&ctx).await;
    assert!(result.is_ok());
}

#[cfg(feature = "telemetry")]
#[tokio::test]
async fn test_opentelemetry_interceptor_on_error() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let interceptor = OpenTelemetryInterceptor::new();
    let metadata = std::collections::HashMap::new();
    let ctx = ErrorContext {
        operation: "chat",
        model: Some("gpt-4"),
        request_json: Some(r#"{"model":"gpt-4","messages":[]}"#),
        error: "API Error: Rate limit exceeded",
        duration: Duration::from_secs(1),
        metadata: Some(&metadata),
    };

    // Should not panic
    interceptor.on_error(&ctx).await;
}

#[cfg(feature = "telemetry")]
#[test]
fn test_opentelemetry_interceptor_creation() {
    let _interceptor = OpenTelemetryInterceptor::new();
    let _interceptor_default = OpenTelemetryInterceptor::default();
}
