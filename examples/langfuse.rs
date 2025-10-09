//! Example of using the Langfuse interceptor for LLM observability.
//!
//! This example demonstrates how to integrate Langfuse tracing with `OpenAI` API calls
//! using the built-in interceptor system with task-local span storage.
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
//! cargo run --example langfuse
//! ```

use openai_ergonomic::{Builder, ChatCompletionBuilder, Client, LangfuseConfig, LangfuseInterceptor};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("openai_ergonomic=debug".parse()?),
        )
        .init();

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
    let langfuse_interceptor = std::sync::Arc::new(LangfuseInterceptor::new(tracer, LangfuseConfig::new()));

    // 4. Create the OpenAI client and add the Langfuse interceptor
    // Keep a reference to the interceptor so we can update context later
    let client = Client::from_env()?
        .with_interceptor(Box::new(langfuse_interceptor.clone()))
        .build();

    println!("🚀 OpenAI client initialized with Langfuse observability");
    println!("📊 Traces will be sent to Langfuse for monitoring\n");

    // Example 1: Simple chat completion
    println!("Example 1: Simple chat completion");
    println!("---------------------------------");
    let chat_builder = client
        .chat_simple("What is the capital of France? Answer in one word.")
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    // Example 2: Chat completion with builder pattern
    println!("Example 2: Chat with builder pattern");
    println!("-------------------------------------");
    let chat_builder = client
        .chat()
        .system("You are a helpful assistant that speaks like a pirate.")
        .user("Tell me about the ocean in 2 sentences.")
        .temperature(0.7)
        .max_tokens(100)
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    // Example 3: Multiple messages in a conversation
    println!("Example 3: Conversation");
    println!("-----------------------");
    let chat_builder = client
        .chat()
        .system("You are a math tutor.")
        .user("What is 2 + 2?")
        .assistant("2 + 2 equals 4.")
        .user("And what about 3 + 3?")
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    // Example 4: Error handling (intentionally trigger an error)
    println!("Example 4: Error handling");
    println!("-------------------------");
    // Create a builder with a non-existent model
    let chat_builder = ChatCompletionBuilder::new("non-existent-model")
        .user("This should fail")
        .build()?;
    let result = client.execute_chat(chat_builder).await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error captured: {e}\n"),
    }

    // Example 5: Embeddings
    println!("Example 5: Embeddings");
    println!("--------------------");
    let embeddings_builder = client.embeddings().text(
        "text-embedding-ada-002",
        "The quick brown fox jumps over the lazy dog",
    );
    let embeddings = client.embeddings().create(embeddings_builder).await?;
    println!("Generated {} embedding(s)\n", embeddings.data.len());

    // Example 6: Using custom metadata via interceptor context
    println!("Example 6: Custom metadata via interceptor context");
    println!("---------------------------------------------------");

    // Set session and user IDs on the interceptor's context
    langfuse_interceptor.set_session_id("demo-session-123");
    langfuse_interceptor.set_user_id("demo-user-456");
    langfuse_interceptor.add_tags(vec!["example".to_string(), "demo".to_string()]);

    let chat_builder = client
        .chat_simple("Say 'Hello from custom session!'")
        .build()?;
    let response = client.execute_chat(chat_builder).await?;
    println!("Response with custom metadata: {:?}\n", response.content());

    // Clear context for subsequent calls
    langfuse_interceptor.clear_context();

    println!("✅ All examples completed!");
    println!("📊 Check your Langfuse dashboard to see the traces");
    println!("   - Look for traces with operation name 'chat'");
    println!("   - Each trace includes request/response details, token usage, and timing");
    println!("   - Example 6 will have custom session_id, user_id, and tags");

    // Shutdown the tracer provider to flush all spans
    println!("\n⏳ Flushing spans to Langfuse...");
    provider.shutdown()?;

    Ok(())
}
