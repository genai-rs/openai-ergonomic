//! Example of using the Langfuse middleware for LLM observability.
//!
//! This example demonstrates how to integrate Langfuse tracing with `OpenAI` API calls
//! using the built-in middleware system with proper OpenTelemetry context management.
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

use openai_ergonomic::{
    middleware::langfuse::{LangfuseConfig, LangfuseMiddleware},
    Builder, ChatCompletionBuilder, Client, Config,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("openai_ergonomic=debug".parse()?),
        )
        .init();

    // Method 1: Create Langfuse middleware from environment variables
    let langfuse_middleware = LangfuseMiddleware::from_env()?;

    // Method 2: Create with custom configuration
    // let langfuse_config = LangfuseConfig {
    //     host: "https://cloud.langfuse.com".to_string(),
    //     public_key: std::env::var("LANGFUSE_PUBLIC_KEY")?,
    //     secret_key: std::env::var("LANGFUSE_SECRET_KEY")?,
    //     session_id: Some("example-session".to_string()),
    //     user_id: Some("user-123".to_string()),
    //     release: Some("v1.0.0".to_string()),
    //     timeout: Duration::from_secs(15),
    //     batch_size: 50,
    //     export_interval: Duration::from_secs(10),
    //     debug: true,
    // };
    // let langfuse_middleware = LangfuseMiddleware::new(langfuse_config)?;

    // Create the OpenAI client using ClientBuilder with middleware
    let client = Client::builder()
        .config(Config::from_env()?)
        .with_middleware(Arc::new(langfuse_middleware))
        .build()?;

    println!("🚀 OpenAI client initialized with Langfuse observability");
    println!("📊 Traces will be sent to Langfuse with proper parent-child relationships\n");

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

    // Example 6: Using custom session and user IDs
    println!("Example 6: Custom metadata");
    println!("--------------------------");

    // Create a new middleware with specific session/user for this request
    let custom_config = LangfuseConfig {
        host: std::env::var("LANGFUSE_HOST")
            .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string()),
        public_key: std::env::var("LANGFUSE_PUBLIC_KEY")?,
        secret_key: std::env::var("LANGFUSE_SECRET_KEY")?,
        session_id: Some("demo-session-123".to_string()),
        user_id: Some("demo-user-456".to_string()),
        release: Some("example-v1.0.0".to_string()),
        ..Default::default()
    };
    let custom_middleware = LangfuseMiddleware::new(custom_config)?;

    let custom_client = Client::builder()
        .config(Config::from_env()?)
        .with_middleware(Arc::new(custom_middleware))
        .build()?;

    let chat_builder = custom_client
        .chat_simple("Say 'Hello from custom session!'")
        .build()?;
    let response = custom_client.execute_chat(chat_builder).await?;
    println!("Response with custom metadata: {:?}\n", response.content());

    println!("✅ All examples completed!");
    println!("📊 Check your Langfuse dashboard to see the traces");
    println!("   - Traces now have proper parent-child relationships");
    println!("   - Root trace: 'OpenAI-generation'");
    println!("   - Child observations: 'OpenAI chat', 'OpenAI embeddings', etc.");
    println!("   - Each observation includes request/response details, token usage, and timing");

    // Give some time for the final batch export
    println!("\n⏳ Waiting for traces to be exported...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    Ok(())
}
