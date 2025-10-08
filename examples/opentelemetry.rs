//! Example of using OpenTelemetry middleware for observability.
//!
//! This example demonstrates how to use the generic OpenTelemetry middleware
//! that follows GenAI semantic conventions. This works with any OpenTelemetry
//! backend (Jaeger, Zipkin, etc.).
//!
//! ## Setup
//!
//! Before running this example, set the following environment variable:
//! - `OPENAI_API_KEY`: Your OpenAI API key
//!
//! ## Running the example
//!
//! ```bash
//! cargo run --example opentelemetry
//! ```

use openai_ergonomic::{
    middleware::opentelemetry::OpenTelemetryMiddleware, Builder, Client, Config,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("openai_ergonomic=debug".parse()?),
        )
        .init();

    // Create OpenTelemetry middleware
    // This creates spans following GenAI semantic conventions
    let otel_middleware = OpenTelemetryMiddleware::new();

    // Or with custom tracer name:
    // let otel_middleware = OpenTelemetryMiddleware::with_tracer_name("my-app");

    // Create the OpenAI client using ClientBuilder with middleware
    let client = Client::builder()
        .config(Config::from_env()?)
        .with_middleware(Arc::new(otel_middleware))
        .build()?;

    println!("🚀 OpenAI client initialized with OpenTelemetry middleware");
    println!("📊 Spans will follow GenAI semantic conventions\n");

    // Example 1: Simple chat completion
    println!("Example 1: Simple chat completion");
    println!("---------------------------------");
    let chat_builder = client
        .chat_simple("What is the capital of France? Answer in one word.")
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    // Example 2: Chat completion with parameters
    println!("Example 2: Chat with parameters");
    println!("--------------------------------");
    let chat_builder = client
        .chat()
        .system("You are a helpful assistant.")
        .user("Explain quantum computing in one sentence.")
        .temperature(0.7)
        .max_tokens(50)
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    // Example 3: Error handling
    println!("Example 3: Error handling");
    println!("-------------------------");
    let chat_builder = ChatCompletionBuilder::new("non-existent-model")
        .user("This should fail")
        .build()?;
    let result = client.execute_chat(chat_builder).await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error captured: {}\n", e),
    }

    println!("✅ All examples completed!");
    println!("📊 OpenTelemetry spans created with GenAI semantic conventions:");
    println!("   - gen_ai.system: openai");
    println!("   - gen_ai.operation.name: chat");
    println!("   - gen_ai.request.model: gpt-4");
    println!("   - gen_ai.request.temperature: 0.7");
    println!("   - gen_ai.request.max_tokens: 50");
    println!("   - gen_ai.response.id: <response-id>");
    println!("   - gen_ai.usage.input_tokens: <count>");
    println!("   - gen_ai.usage.output_tokens: <count>");
    println!("   - duration_ms: <milliseconds>");

    Ok(())
}
