//! Simple example of using the Langfuse middleware for LLM observability.
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
//! cargo run --example langfuse_simple
//! ```

use openai_ergonomic::{middleware::langfuse::LangfuseMiddleware, Builder, Client, Config};
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

    println!("🚀 Initializing OpenAI client with Langfuse observability...\n");

    // Create Langfuse middleware from environment variables
    let langfuse_middleware = LangfuseMiddleware::from_env()?;

    // Create the OpenAI client with middleware
    let client = Client::builder()
        .config(Config::from_env()?)
        .with_middleware(Arc::new(langfuse_middleware))
        .build()?;

    println!("📊 Making API call with Langfuse tracing enabled\n");

    // Make a simple chat request
    let chat_builder = client
        .chat_simple("What is Rust programming language? Answer in one sentence.")
        .build()?;

    let response = client.execute_chat(chat_builder).await?;
    println!("Response: {:?}\n", response.content());

    println!("✅ Request completed!");
    println!("📊 Check your Langfuse dashboard to see the trace");

    // Wait for traces to be exported
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
