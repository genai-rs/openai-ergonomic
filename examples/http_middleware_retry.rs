//! HTTP middleware with retry support using tower layers.
//!
//! The client sends every request through a [`timvw_rust_runtime::Client`]: a tower service
//! stack around reqwest. Everything around sending a request is a tower layer: retries,
//! timeouts, logging, headers, rate limits.
//!
//! This example shows how to:
//! - Tune the retries of the default client with `Config::max_retries`
//! - Build your own client with a retry policy (exponential backoff, `Retry-After`)
//! - Bound each attempt with a timeout layer
//! - Log each request and retry with `TraceLayer` (feature `tracing-http`)
//!
//! Layers apply from the inside out: the last one added sees a request first.
//!
//! Run with: `cargo run --example http_middleware_retry --features tracing-http`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openai_ergonomic::{Client, Config, Response, Result};
use std::time::Duration;
use timvw_rust_runtime::retry::RetryPolicy;
use timvw_rust_runtime::{Client as HttpClient, ReqwestService};
use tower::retry::RetryLayer;
use tower::timeout::TimeoutLayer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== HTTP Middleware with Retry Example ===\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");

    // Example 1: The default client already retries 429, 408, 5xx and failed connections.
    // `max_retries` sets how many times (0 disables retries).
    println!("1. Default client with 5 retries");

    let config = Config::builder()
        .api_key(api_key.clone())
        .max_retries(5)
        .build();
    let client = Client::builder(config)?.build();

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

    // Example 2: A client of your own: a custom reqwest client, a timeout per attempt, logging
    // of each attempt, and a custom retry policy around them.
    println!("\n2. Creating client with custom tower layers");

    let reqwest_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build reqwest client");

    let retry_policy = RetryPolicy::new(5) // up to 5 retries
        .base_delay(Duration::from_millis(100)) // first backoff delay, doubled each retry
        .max_delay(Duration::from_secs(30)); // cap, also on `Retry-After`

    let http_client = HttpClient::from_service(ReqwestService::new(reqwest_client))
        // Innermost: bounds each attempt.
        .layer(TimeoutLayer::new(Duration::from_secs(60)));

    // Logs each attempt (`RUST_LOG=timvw_rust_runtime=debug`).
    #[cfg(feature = "tracing-http")]
    let http_client = http_client.layer(timvw_rust_runtime::trace::TraceLayer::new());

    // Outermost: retries the attempts above.
    let http_client = http_client.layer(RetryLayer::new(retry_policy));

    // A custom HTTP client is used as is: `max_retries` and the default timeout do not apply.
    let custom_config = Config::builder()
        .api_key(api_key)
        .http_client(http_client)
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
    println!("\n3. Using builder pattern with the custom client");

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
    println!("\nKey benefits of tower layers:");
    println!("  - Automatic retry of transient failures, honouring Retry-After");
    println!("  - Exponential backoff to avoid overwhelming servers");
    println!("  - Composable layers for timeouts, logging, metrics, rate limits, etc.");
    println!("  - Transparent to application code - works with any request");

    Ok(())
}
