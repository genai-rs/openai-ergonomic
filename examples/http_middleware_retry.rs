//! HTTP middleware with retry support using reqwest-middleware.
//!
//! This example demonstrates how to use `reqwest-middleware` with `reqwest-retry`
//! to add automatic retry capabilities with exponential backoff to the `OpenAI` client.
//!
//! The middleware approach allows you to:
//! - Automatically retry transient errors (network failures, timeouts, 5xx errors)
//! - Configure exponential backoff strategies
//! - Add custom middleware for logging, metrics, or other cross-cutting concerns
//! - Compose multiple middleware layers
//!
//! Run with: `cargo run --example http_middleware_retry`

use openai_ergonomic::{Client, Config, Response, Result};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== HTTP Middleware with Retry Example ===\n");

    // Example 1: Basic client with retry middleware
    println!("1. Creating client with retry middleware");

    // Create a retry policy with exponential backoff
    // This will retry transient errors up to 3 times with exponential delays
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    // Build an HTTP client with retry middleware
    let http_client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    // Create OpenAI client with custom HTTP client
    let config = Config::builder()
        .api_key(
            std::env::var("OPENAI_API_KEY")
                .expect("OPENAI_API_KEY environment variable must be set"),
        )
        .http_client(http_client)
        .build();

    let client = Client::builder(config)?.build();

    // Use the client normally - retries are handled automatically
    println!("Sending chat completion request (retries are automatic)...");

    let builder = client.chat_simple("Hello! How are you today?");
    match client.send_chat(builder).await {
        Ok(response) => {
            println!("\nSuccess! Response received:");
            if let Some(content) = response.content() {
                println!("{content}");
            }
        }
        Err(e) => {
            eprintln!("\nError after retries: {e}");
        }
    }

    // Example 2: Custom retry policy with more retries and custom delays
    println!("\n2. Creating client with custom retry policy");

    let custom_retry_policy = ExponentialBackoff::builder()
        .retry_bounds(
            std::time::Duration::from_millis(100), // minimum delay
            std::time::Duration::from_secs(30),    // maximum delay
        )
        .build_with_max_retries(5); // up to 5 retries

    let custom_http_client = ClientBuilder::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build reqwest client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        custom_retry_policy,
    ))
    .build();

    let custom_config = Config::builder()
        .api_key(
            std::env::var("OPENAI_API_KEY")
                .expect("OPENAI_API_KEY environment variable must be set"),
        )
        .http_client(custom_http_client)
        .timeout_seconds(60)
        .build();

    let custom_client = Client::builder(custom_config)?.build();

    println!("Sending request with custom retry policy (up to 5 retries)...");

    let builder = custom_client.chat_simple("Explain quantum computing in one sentence.");
    match custom_client.send_chat(builder).await {
        Ok(response) => {
            println!("\nSuccess! Response received:");
            if let Some(content) = response.content() {
                println!("{content}");
            }
        }
        Err(e) => {
            eprintln!("\nError after all retries: {e}");
        }
    }

    // Example 3: Using the builder pattern for more complex requests
    println!("\n3. Using builder pattern with retry middleware");

    let builder = custom_client
        .responses()
        .user("What are the three laws of robotics?")
        .max_completion_tokens(200)
        .temperature(0.7);

    let response = custom_client.send_responses(builder).await?;

    println!("\nResponse received:");
    if let Some(content) = response.content() {
        println!("{content}");
    }

    println!("\nToken usage:");
    if let Some(usage) = response.usage() {
        let prompt = usage.prompt_tokens;
        let completion = usage.completion_tokens;
        let total = usage.total_tokens;
        println!("  Prompt tokens: {prompt}");
        println!("  Completion tokens: {completion}");
        println!("  Total tokens: {total}");
    }

    println!("\n=== Example completed successfully! ===");
    println!("\nKey benefits of using reqwest-middleware:");
    println!("  - Automatic retry of transient failures");
    println!("  - Exponential backoff to avoid overwhelming servers");
    println!("  - Composable middleware for logging, metrics, etc.");
    println!("  - Transparent to application code - works with any request");

    Ok(())
}
