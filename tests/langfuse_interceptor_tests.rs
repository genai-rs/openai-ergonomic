//! Tests for `LangfuseInterceptor`.

use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, LangfuseInterceptor,
    TelemetryContext,
};
use std::time::Duration;

#[cfg(feature = "telemetry")]
#[tokio::test]
async fn test_langfuse_interceptor_without_context() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let interceptor = LangfuseInterceptor::new();
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
async fn test_langfuse_interceptor_with_context() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("user-123")
        .with_session_id("session-456")
        .with_tag("production")
        .with_metadata("region", "us-east-1");

    let interceptor = LangfuseInterceptor::with_context(telemetry_ctx);
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
async fn test_langfuse_interceptor_after_response() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("user-123")
        .with_session_id("session-456")
        .with_tag("production")
        .with_tag("testing")
        .with_metadata("region", "us-east-1")
        .with_metadata("version", "1.0");

    let interceptor = LangfuseInterceptor::with_context(telemetry_ctx);
    let metadata = std::collections::HashMap::new();
    let ctx = AfterResponseContext {
        operation: "chat",
        model: "gpt-4",
        request_json: r#"{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}"#,
        response_json: r#"{"choices":[{"message":{"content":"Hi there!"}}],"usage":{"prompt_tokens":10,"completion_tokens":20}}"#,
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
async fn test_langfuse_interceptor_on_error() {
    use opentelemetry::global;
    use opentelemetry_sdk::testing::trace::NoopSpanExporter;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    // Setup tracer
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(NoopSpanExporter::new())
        .build();
    global::set_tracer_provider(provider);

    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("user-123")
        .with_session_id("session-456");

    let interceptor = LangfuseInterceptor::with_context(telemetry_ctx);
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
fn test_langfuse_interceptor_creation() {
    let _interceptor = LangfuseInterceptor::new();
    let _interceptor_default = LangfuseInterceptor::default();

    let telemetry_ctx = TelemetryContext::new().with_user_id("user-123");
    let _interceptor_with_ctx = LangfuseInterceptor::with_context(telemetry_ctx);
}
