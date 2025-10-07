//! Example demonstrating OpenTelemetry instrumentation with Langfuse.
//!
//! This example shows how to set up OpenTelemetry tracing with Langfuse as the backend
//! to observe and monitor `OpenAI` API calls with full context including user IDs, session IDs,
//! and custom tags.
//!
//! # Setup
//!
//! 1. Sign up for a free Langfuse account at <https://langfuse.com>
//! 2. Create a new project and get your API keys
//! 3. Set the following environment variables:
//!    ```bash
//!    export OPENAI_API_KEY="your-openai-api-key"
//!    export LANGFUSE_PUBLIC_KEY="pk-lf-..."
//!    export LANGFUSE_SECRET_KEY="sk-lf-..."
//!    export LANGFUSE_HOST="https://cloud.langfuse.com"  # optional, this is the default
//!    ```
//!
//! # Running
//!
//! ```bash
//! cargo run --example telemetry_langfuse --features telemetry
//! ```
//!
//! After running, visit your Langfuse dashboard to see the captured traces with:
//! - Operation details (model, temperature, tokens)
//! - Custom context (user ID, session ID, tags)
//! - Token usage metrics
//! - Full request/response data
//!
//! # Batch Exporting
//!
//! This example uses `with_batch_exporter` for production-ready async span export.
//! `BatchSpanProcessor` provides:
//! - Non-blocking span export
//! - Efficient batching for reduced network overhead
//! - Better performance for high-throughput applications
//!
//! The `telemetry` feature includes `opentelemetry_sdk` with the
//! `experimental_trace_batch_span_processor_with_async_runtime` feature enabled.

use openai_ergonomic::{Client, LangfuseInterceptor, TelemetryContext};
use opentelemetry::global;
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for local logging
    tracing_subscriber::fmt::init();

    println!("Setting up OpenTelemetry with Langfuse...");

    // Create Langfuse exporter from environment variables
    let exporter = ExporterBuilder::from_env()?.build()?;

    // Create tracer provider with batch exporter for production use
    // BatchSpanProcessor batches spans for efficient network usage
    // The batch processor uses the Tokio runtime from #[tokio::main]
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_attributes([
                    opentelemetry::KeyValue::new("service.name", "openai-ergonomic-example"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])
                .build(),
        )
        .with_batch_exporter(exporter)
        .build();

    // Set the global tracer provider
    global::set_tracer_provider(provider.clone());

    println!("OpenTelemetry configured with Langfuse");

    // Create OpenAI client with Langfuse interceptor
    // This automatically creates spans for all API calls with Langfuse attributes
    let client = Client::from_env()?.with_interceptor(Box::new(LangfuseInterceptor::new()));

    println!("\n--- Example 1: Simple chat ---");

    // With the interceptor, telemetry is automatic for all requests
    let response = client
        .send_chat(
            client
                .chat()
                .system("You are a helpful assistant.")
                .user("What is the capital of France?")
                .temperature(0.7),
        )
        .await?;

    println!("Response: {}", response.content().unwrap_or("No content"));

    println!("\n--- Example 2: Another request ---");

    let response2 = client
        .send_chat(client.chat_simple("What is 2 + 2?"))
        .await?;

    println!("Response: {}", response2.content().unwrap_or("No content"));

    println!("\n--- Example 3: Multiple requests (simulating a conversation) ---");

    // All requests are automatically traced with the interceptor
    for i in 1..=3 {
        let response = client
            .send_chat(client.chat_simple(format!("Tell me fact #{i} about Rust programming")))
            .await?;
        println!("Fact #{i}: {}", response.content().unwrap_or("No content"));
    }

    println!("\n--- Example 4: Using LangfuseInterceptor with context ---");

    // You can also create a client with context-specific interceptor
    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("user-12345")
        .with_session_id("session-abc-789")
        .with_tag("production")
        .with_metadata("region", "us-east-1");

    let client_with_context = Client::from_env()?
        .with_interceptor(Box::new(LangfuseInterceptor::with_context(telemetry_ctx)));

    let response = client_with_context
        .send_chat(client_with_context.chat_simple("Hello with user context!"))
        .await?;
    println!("Response: {}", response.content().unwrap_or("No content"));

    println!("\n--- Flushing telemetry data ---");

    // Wait for batch export
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Shutdown to ensure all spans are flushed
    provider.shutdown()?;

    println!("All telemetry data flushed to Langfuse");
    println!("\nCheck your Langfuse dashboard at https://cloud.langfuse.com to see the traces!");
    println!("You should see:");
    println!("- User IDs and session IDs for grouping");
    println!("- Tags for filtering (production, chatbot, math, conversation)");
    println!("- Metadata (region, experiment)");
    println!("- Token usage metrics");
    println!("- Model parameters (temperature, etc.)");

    Ok(())
}
