//! Integration tests for OpenTelemetry instrumentation.
//!
//! These tests verify that spans are created with correct semantic conventions
//! following the `OpenAI` semantic conventions spec.

#![cfg(feature = "telemetry")]

use openai_ergonomic::{Client, TelemetryContext};
use opentelemetry::global;
use opentelemetry_sdk::trace::InMemorySpanExporter;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use std::sync::Arc;

/// Setup function that creates a tracer provider with an in-memory exporter for testing.
fn setup_telemetry() -> (SdkTracerProvider, Arc<InMemorySpanExporter>) {
    let exporter = Arc::new(InMemorySpanExporter::default());

    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(InMemorySpanExporter::default())
        .with_sampler(Sampler::AlwaysOn)
        .build();

    global::set_tracer_provider(provider.clone());

    (provider, exporter)
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY"]
async fn test_chat_completion_creates_span() {
    let (provider, exporter) = setup_telemetry();

    let client = Client::from_env().expect("OPENAI_API_KEY must be set");

    let builder = client.chat().user("Hello, world!");
    let _response = client
        .send_chat(builder)
        .await
        .expect("API call should succeed");

    // Force export
    let _ = provider.shutdown();

    // Verify span was created
    let spans = exporter.get_finished_spans().expect("Should have spans");
    assert!(!spans.is_empty(), "Should have at least one span");

    let chat_span = spans
        .iter()
        .find(|s| s.name == "chat")
        .expect("Should have a 'chat' span");

    // Verify required OpenAI semantic convention attributes
    let attributes: std::collections::HashMap<_, _> = chat_span
        .attributes
        .iter()
        .map(|kv| (kv.key.as_str(), kv.value.clone()))
        .collect();

    // Required attributes
    assert!(
        attributes.contains_key("gen_ai.operation.name"),
        "Should have gen_ai.operation.name"
    );
    assert_eq!(
        attributes.get("gen_ai.operation.name").unwrap().as_str(),
        "chat"
    );

    assert!(
        attributes.contains_key("gen_ai.system"),
        "Should have gen_ai.system"
    );
    assert_eq!(attributes.get("gen_ai.system").unwrap().as_str(), "openai");

    assert!(
        attributes.contains_key("gen_ai.request.model"),
        "Should have gen_ai.request.model"
    );
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY"]
async fn test_telemetry_context_attributes() {
    let (provider, exporter) = setup_telemetry();

    let client = Client::from_env().expect("OPENAI_API_KEY must be set");

    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("test-user-123")
        .with_session_id("test-session-456")
        .with_tag("test")
        .with_tag("integration")
        .with_metadata("test_key", "test_value");

    let builder = client
        .chat()
        .user("Test message")
        .with_telemetry_context(telemetry_ctx);
    let _response = client
        .send_chat(builder)
        .await
        .expect("API call should succeed");

    // Force export
    let _ = provider.shutdown();

    // Verify Langfuse-specific attributes
    let spans = exporter.get_finished_spans().expect("Should have spans");
    let chat_span = spans
        .iter()
        .find(|s| s.name == "chat")
        .expect("Should have a 'chat' span");

    let attributes: std::collections::HashMap<_, _> = chat_span
        .attributes
        .iter()
        .map(|kv| (kv.key.as_str(), kv.value.clone()))
        .collect();

    // Verify Langfuse attributes
    assert!(
        attributes.contains_key("langfuse.userId"),
        "Should have langfuse.userId"
    );
    assert_eq!(
        attributes.get("langfuse.userId").unwrap().as_str(),
        "test-user-123"
    );

    assert!(
        attributes.contains_key("langfuse.sessionId"),
        "Should have langfuse.sessionId"
    );
    assert_eq!(
        attributes.get("langfuse.sessionId").unwrap().as_str(),
        "test-session-456"
    );

    assert!(
        attributes.contains_key("langfuse.tags"),
        "Should have langfuse.tags"
    );
    assert_eq!(
        attributes.get("langfuse.tags").unwrap().as_str(),
        "test,integration"
    );

    assert!(
        attributes.contains_key("langfuse.metadata.test_key"),
        "Should have langfuse.metadata.test_key"
    );
    assert_eq!(
        attributes
            .get("langfuse.metadata.test_key")
            .unwrap()
            .as_str(),
        "test_value"
    );
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY"]
async fn test_token_usage_recorded() {
    let (provider, exporter) = setup_telemetry();

    let client = Client::from_env().expect("OPENAI_API_KEY must be set");

    let builder = client.chat().user("Count to 5");
    let _response = client
        .send_chat(builder)
        .await
        .expect("API call should succeed");

    // Force export
    let _ = provider.shutdown();

    // Verify token usage attributes
    let spans = exporter.get_finished_spans().expect("Should have spans");
    let chat_span = spans
        .iter()
        .find(|s| s.name == "chat")
        .expect("Should have a 'chat' span");

    let attributes: std::collections::HashMap<_, _> = chat_span
        .attributes
        .iter()
        .map(|kv| (kv.key.as_str(), kv.value.clone()))
        .collect();

    // Verify token usage
    assert!(
        attributes.contains_key("gen_ai.usage.input_tokens"),
        "Should have gen_ai.usage.input_tokens"
    );
    assert!(
        attributes.contains_key("gen_ai.usage.output_tokens"),
        "Should have gen_ai.usage.output_tokens"
    );

    // Tokens should be positive integers
    let input_tokens = match attributes.get("gen_ai.usage.input_tokens").unwrap() {
        opentelemetry::Value::I64(v) => *v,
        _ => panic!("Expected I64 value for input_tokens"),
    };
    let output_tokens = match attributes.get("gen_ai.usage.output_tokens").unwrap() {
        opentelemetry::Value::I64(v) => *v,
        _ => panic!("Expected I64 value for output_tokens"),
    };

    assert!(input_tokens > 0, "Input tokens should be positive");
    assert!(output_tokens > 0, "Output tokens should be positive");
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY"]
async fn test_request_parameters_recorded() {
    let (provider, exporter) = setup_telemetry();

    let client = Client::from_env().expect("OPENAI_API_KEY must be set");

    let builder = client
        .chat()
        .user("Test")
        .temperature(0.7)
        .max_tokens(100)
        .top_p(0.9);
    let _response = client
        .send_chat(builder)
        .await
        .expect("API call should succeed");

    // Force export
    let _ = provider.shutdown();

    // Verify request parameters
    let spans = exporter.get_finished_spans().expect("Should have spans");
    let chat_span = spans
        .iter()
        .find(|s| s.name == "chat")
        .expect("Should have a 'chat' span");

    let attributes: std::collections::HashMap<_, _> = chat_span
        .attributes
        .iter()
        .map(|kv| (kv.key.as_str(), kv.value.clone()))
        .collect();

    // Verify parameters
    assert!(
        attributes.contains_key("gen_ai.request.temperature"),
        "Should have gen_ai.request.temperature"
    );
    let temp = match attributes.get("gen_ai.request.temperature").unwrap() {
        opentelemetry::Value::F64(v) => *v,
        _ => panic!("Expected F64 value for temperature"),
    };
    assert!((temp - 0.7).abs() < f64::EPSILON);

    assert!(
        attributes.contains_key("gen_ai.request.max_tokens"),
        "Should have gen_ai.request.max_tokens"
    );
    let max_tokens = match attributes.get("gen_ai.request.max_tokens").unwrap() {
        opentelemetry::Value::I64(v) => *v,
        _ => panic!("Expected I64 value for max_tokens"),
    };
    assert_eq!(max_tokens, 100);

    assert!(
        attributes.contains_key("gen_ai.request.top_p"),
        "Should have gen_ai.request.top_p"
    );
    let top_p = match attributes.get("gen_ai.request.top_p").unwrap() {
        opentelemetry::Value::F64(v) => *v,
        _ => panic!("Expected F64 value for top_p"),
    };
    assert!((top_p - 0.9).abs() < f64::EPSILON);
}

#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY and LANGFUSE_PUBLIC_KEY, LANGFUSE_SECRET_KEY"]
async fn test_real_langfuse_integration() {
    use opentelemetry_langfuse::ExporterBuilder;

    // Create Langfuse exporter from environment
    let exporter = ExporterBuilder::from_env()
        .expect("Langfuse env vars must be set")
        .build()
        .expect("Failed to build exporter");

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());

    let client = Client::from_env().expect("OPENAI_API_KEY must be set");

    let builder = client
        .chat()
        .user("Integration test message")
        .with_user_id("test-integration-user")
        .with_session_id("test-integration-session")
        .with_tag("integration-test");
    let _response = client
        .send_chat(builder)
        .await
        .expect("API call should succeed");

    // Flush to Langfuse
    // Dropping the provider will flush remaining spans
    drop(provider);

    println!("Successfully sent trace to Langfuse");
    println!("Check your Langfuse dashboard to verify the trace was received");
}
