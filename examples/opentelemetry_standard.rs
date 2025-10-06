//! Example demonstrating standard OpenTelemetry instrumentation.
//!
//! This example shows how to set up OpenTelemetry tracing with the
//! `OpenTelemetryInterceptor` following standard semantic conventions.
//! It can be used with any OpenTelemetry-compatible backend (Jaeger, Zipkin,
//! OTLP, stdout, etc.).
//!
//! # Features
//!
//! - Standard OpenTelemetry semantic conventions (`gen_ai.*` attributes)
//! - Works with any OpenTelemetry backend
//! - No vendor-specific attributes
//! - Production-ready batch exporter
//!
//! # Setup
//!
//! Set your `OpenAI` API key:
//! ```bash
//! export OPENAI_API_KEY="your-openai-api-key"
//! ```
//!
//! # Running
//!
//! ```bash
//! cargo run --example opentelemetry_standard --features telemetry
//! ```
//!
//! This example uses stdout exporter for simplicity. In production, you would
//! typically use:
//! - OTLP exporter for generic OpenTelemetry backends
//! - Jaeger exporter for Jaeger
//! - Zipkin exporter for Zipkin
//! - Or any other OpenTelemetry-compatible exporter

use openai_ergonomic::{Client, OpenTelemetryInterceptor};
use opentelemetry::global;
use opentelemetry_sdk::testing::trace::NoopSpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for local logging
    tracing_subscriber::fmt::init();

    println!("Setting up OpenTelemetry with noop exporter...");
    println!("Note: This example uses a no-op exporter for demonstration.");
    println!("In production, replace with OTLP, Jaeger, Zipkin, or other exporters.\n");

    // Create tracer provider with noop exporter for demonstration
    // In production, use one of these exporters:
    // - opentelemetry-otlp for generic OTLP backends
    // - opentelemetry-jaeger for Jaeger
    // - opentelemetry-zipkin for Zipkin
    // - opentelemetry-prometheus for Prometheus
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_attributes([
                    opentelemetry::KeyValue::new("service.name", "openai-ergonomic-example"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])
                .build(),
        )
        .with_simple_exporter(NoopSpanExporter::new())
        .build();

    // Set the global tracer provider
    global::set_tracer_provider(provider.clone());

    println!("OpenTelemetry configured");

    // Create OpenAI client with OpenTelemetry interceptor
    // This automatically creates spans for all API calls
    let client = Client::from_env()?.with_interceptor(Box::new(OpenTelemetryInterceptor::new()));

    println!("--- Example 1: Simple chat completion ---");

    let response = client
        .send_chat(
            client
                .chat()
                .system("You are a helpful assistant.")
                .user("What is the capital of France?")
                .temperature(0.7),
        )
        .await?;

    println!("Response: {}\n", response.content().unwrap_or("No content"));

    println!("--- Example 2: Math question ---");

    let response2 = client
        .send_chat(client.chat_simple("What is 2 + 2?"))
        .await?;

    println!(
        "Response: {}\n",
        response2.content().unwrap_or("No content")
    );

    println!("--- Example 3: Multiple requests ---");

    for i in 1..=3 {
        let response = client
            .send_chat(client.chat_simple(format!("Tell me fact #{i} about Rust programming")))
            .await?;
        println!("Fact #{i}: {}", response.content().unwrap_or("No content"));
    }

    println!("\n--- Shutting down ---");

    // Shutdown to ensure all spans are flushed
    provider.shutdown()?;

    println!("\nAll requests completed!");
    println!("\nWith a real exporter, each span would contain:");
    println!("- gen_ai.operation.name - The operation type (chat, embedding, etc.)");
    println!("- gen_ai.system - Always 'openai'");
    println!("- gen_ai.request.model - The model used");
    println!("- gen_ai.usage.input_tokens - Input tokens consumed");
    println!("- gen_ai.usage.output_tokens - Output tokens generated");

    Ok(())
}
