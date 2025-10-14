#![allow(clippy::uninlined_format_args)]
//! Streaming chat completions with Langfuse observability.
//!
//! This example demonstrates how to use streaming responses with the Langfuse interceptor
//! for real-time observability and tracing.
//!
//! ## Setup
//!
//! Before running this example, set the following environment variables:
//! - `OPENAI_API_KEY`: Your `OpenAI` API key
//! - `LANGFUSE_PUBLIC_KEY`: Your Langfuse public key (starts with "pk-lf-")
//! - `LANGFUSE_SECRET_KEY`: Your Langfuse secret key (starts with "sk-lf-")
//! - `LANGFUSE_HOST` (optional): Langfuse API host (defaults to <https://cloud.langfuse.com>)
//!
//! ## Running the example
//!
//! ```bash
//! cargo run --example langfuse_streaming
//! ```

use futures::StreamExt;
use openai_ergonomic::{Client, LangfuseConfig, LangfuseInterceptor, LangfuseState, Result};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider, Span},
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("openai_ergonomic=debug".parse()?),
        )
        .init();

    println!("🚀 Initializing OpenAI client with Langfuse streaming observability...\n");

    // 1. Build Langfuse exporter from environment variables
    let exporter = ExporterBuilder::from_env()?.build()?;

    // 2. Create tracer provider with batch processor
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
        .build();

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // 3. Get tracer and create interceptor
    let tracer = provider.tracer("openai-ergonomic");
    let langfuse_interceptor = LangfuseInterceptor::new(tracer, LangfuseConfig::new());

    // 4. Create the OpenAI client and add the Langfuse interceptor
    let client = Client::from_env()?
        .with_interceptor(Box::new(langfuse_interceptor))
        .build();

    println!("✅ Client initialized successfully!");
    println!("📊 Streaming traces will be sent to Langfuse for monitoring\n");

    // Example 1: Basic streaming with tracing
    println!("=== Example 1: Basic Streaming ===");
    basic_streaming(&client).await?;

    // Example 2: Streaming with parameters
    println!("\n=== Example 2: Streaming with Parameters ===");
    streaming_with_parameters(&client).await?;

    // Example 3: Collect full content
    println!("\n=== Example 3: Collect Full Content ===");
    collect_content(&client).await?;

    println!("\n✅ Done! Check your Langfuse dashboard to see the streaming traces.");
    println!("   - Look for traces with operation names 'chat' or 'responses'");
    println!("   - Each trace includes:");
    println!("     • before_request: Initial request details");
    println!("     • on_stream_chunk: Each chunk as it arrives (real-time)");
    println!("     • on_stream_end: Final token usage and duration");

    // Shutdown the tracer provider to flush all spans
    println!("\n⏳ Flushing spans to Langfuse...");
    provider.shutdown()?;

    Ok(())
}

async fn basic_streaming(client: &Client<LangfuseState<Span>>) -> Result<()> {
    println!("Question: Tell me a short joke");

    let builder = client.chat().user("Tell me a short joke");

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Response: ");
    let mut chunk_count = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
            chunk_count += 1;
        }
    }
    println!(
        "\n(Received {} chunks, all traced to Langfuse)",
        chunk_count
    );

    Ok(())
}

async fn streaming_with_parameters(client: &Client<LangfuseState<Span>>) -> Result<()> {
    println!("Question: Write a creative tagline for a bakery");

    let builder = client
        .chat()
        .user("Write a creative tagline for a bakery")
        .temperature(0.9)
        .max_tokens(50);

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Response: ");
    let mut chunk_count = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
            chunk_count += 1;
        }
    }
    println!(
        "\n(Received {} chunks, all traced to Langfuse)",
        chunk_count
    );

    Ok(())
}

async fn collect_content(client: &Client<LangfuseState<Span>>) -> Result<()> {
    println!("Question: What is the capital of France?");

    let builder = client.chat().user("What is the capital of France?");

    let mut stream = client.send_chat_stream(builder).await?;

    // Manually collect content (interceptor hooks are still called for each chunk)
    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(text) = chunk.content() {
            content.push_str(text);
        }
    }
    println!("Full response: {}", content);
    println!("(All chunks were traced to Langfuse during collection)");

    Ok(())
}
