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
//! # Production Usage
//!
//! For production applications with high throughput, consider using `with_batch_exporter`
//! instead of `with_simple_exporter`. You'll need to enable the
//! `experimental_trace_batch_span_processor_with_async_runtime` feature in opentelemetry_sdk.
//!
//! ```toml
//! [dependencies]
//! opentelemetry_sdk = { version = "0.31", features = [
//!     "trace",
//!     "rt-tokio",
//!     "experimental_trace_batch_span_processor_with_async_runtime"
//! ]}
//! ```

use openai_ergonomic::{Client, TelemetryContext};
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

    // Create tracer provider with resource attributes
    // Using simple exporter for ease of use in examples
    // For production, consider using with_batch_exporter with proper async runtime setup
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_attributes([
                    opentelemetry::KeyValue::new("service.name", "openai-ergonomic-example"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])
                .build(),
        )
        .with_simple_exporter(exporter)
        .build();

    // Set the global tracer provider
    global::set_tracer_provider(provider.clone());

    println!("OpenTelemetry configured with Langfuse");

    // Create OpenAI client
    let client = Client::from_env()?;

    println!("\n--- Example 1: Simple chat with telemetry context ---");

    // Create telemetry context with user and session information
    let telemetry_ctx = TelemetryContext::new()
        .with_user_id("user-12345")
        .with_session_id("session-abc-789")
        .with_tag("production")
        .with_tag("chatbot")
        .with_metadata("region", "us-east-1")
        .with_metadata("experiment", "temperature-test");

    let builder = client
        .chat()
        .system("You are a helpful assistant.")
        .user("What is the capital of France?")
        .temperature(0.7)
        .with_telemetry_context(telemetry_ctx);

    let response = client.send_chat(builder).await?;

    println!("Response: {}", response.content().unwrap_or("No content"));

    println!("\n--- Example 2: Using convenience methods ---");

    // You can also use convenience methods to set context directly
    let builder2 = client
        .chat()
        .user("What is 2 + 2?")
        .with_user_id("user-67890")
        .with_session_id("session-xyz-123")
        .with_tag("math");

    let response2 = client.send_chat(builder2).await?;

    println!("Response: {}", response2.content().unwrap_or("No content"));

    println!("\n--- Example 3: Multiple requests in same session ---");

    // Simulate a conversation
    for i in 1..=3 {
        let builder = client
            .chat()
            .user(format!("Tell me fact #{i} about Rust programming"))
            .with_user_id("user-conversation")
            .with_session_id("session-rust-facts")
            .with_tag("conversation");

        let response = client.send_chat(builder).await?;
        println!("Fact #{i}: {}", response.content().unwrap_or("No content"));
    }

    println!("\n--- Flushing telemetry data ---");

    // Ensure all telemetry data is exported before exiting
    // Dropping the provider will flush remaining spans
    drop(provider);

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
