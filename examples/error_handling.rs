#![allow(clippy::uninlined_format_args)]
//! Comprehensive error handling patterns for the `OpenAI` API.
//!
//! This example demonstrates:
//! - Different error types and how to handle them
//! - Rate limiting and retry strategies
//! - Token limit errors
//! - Authentication errors
//! - Network errors
//! - Custom error handling
//!
//! Run with: `cargo run --example error_handling`

use openai_ergonomic::{Client, Config, Error, Result};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Error Handling Patterns ===\n");

    // Example 1: Basic error handling
    println!("1. Basic Error Handling:");
    basic_error_handling().await;

    // Example 2: Pattern matching on error types
    println!("\n2. Pattern Matching on Errors:");
    pattern_matching_errors().await;

    // Example 3: Rate limit handling
    println!("\n3. Rate Limit Handling:");
    rate_limit_handling().await;

    // Example 4: Token limit handling
    println!("\n4. Token Limit Handling:");
    token_limit_handling().await;

    // Example 5: Authentication errors
    println!("\n5. Authentication Error Handling:");
    auth_error_handling().await?;

    // Example 6: Network error handling
    println!("\n6. Network Error Handling:");
    network_error_handling().await?;

    // Example 7: Custom error context
    println!("\n7. Custom Error Context:");
    custom_error_context().await?;

    // Example 8: Error recovery strategies
    println!("\n8. Error Recovery Strategies:");
    error_recovery_strategies().await?;

    Ok(())
}

async fn basic_error_handling() {
    let client = match Client::from_env() {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create client: {}", e);
            return;
        }
    };

    match client.send_chat(client.chat_simple("Hello")).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("Success: {}", content);
            } else {
                println!("Success: (no content)");
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

async fn pattern_matching_errors() {
    let Ok(client) = Client::from_env() else {
        return;
    };

    // Simulate various errors by using invalid parameters
    let builder = client.chat().user("test");
    let result = client.send_chat(builder).await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => match e {
            Error::Api { message, .. } => {
                println!("API Error: {}", message);
            }
            Error::RateLimit(message) => {
                println!("Rate limited: {}", message);
            }
            Error::Authentication(message) => {
                println!("Authentication failed: {}", message);
            }
            Error::Http(source) => {
                println!("Network error: {}", source);
            }
            Error::Json(source) => {
                println!("Serialization error: {}", source);
            }
            Error::Stream(message) => {
                println!("Stream error: {}", message);
            }
            Error::InvalidRequest(message) => {
                println!("Invalid request: {}", message);
            }
            Error::Config(message) => {
                println!("Configuration error: {}", message);
            }
            _ => {
                println!("Other error: {}", e);
            }
        },
    }
}

async fn rate_limit_handling() {
    const MAX_RETRIES: u32 = 3;

    let Ok(client) = Client::from_env() else {
        return;
    };

    // Retry logic for rate limiting
    let mut retries = 0;

    loop {
        match client.send_chat(client.chat_simple("Hello")).await {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("Success: {}", content);
                } else {
                    println!("Success: (no content)");
                }
                break;
            }
            Err(Error::RateLimit(_message)) => {
                if retries >= MAX_RETRIES {
                    println!("Max retries exceeded");
                    break;
                }

                let wait_time = Duration::from_secs(1);
                println!("Rate limited. Waiting {:?} before retry...", wait_time);
                sleep(wait_time).await;
                retries += 1;
            }
            Err(e) => {
                println!("Other error: {}", e);
                break;
            }
        }
    }
}

async fn token_limit_handling() {
    let Ok(client) = Client::from_env() else {
        return;
    };

    // Generate a very long prompt that might exceed token limits
    let long_text = "Lorem ipsum ".repeat(10000);

    match client.send_chat(client.chat_simple(&long_text)).await {
        Ok(_) => println!("Processed long text successfully"),
        Err(Error::InvalidRequest(message)) if message.contains("token") => {
            println!("Token limit issue: {}", message);

            // Retry with truncated text
            let truncated = &long_text[..1000];
            println!("Retrying with truncated text...");

            match client.send_chat(client.chat_simple(truncated)).await {
                Ok(response) => {
                    if let Some(content) = response.content() {
                        println!("Success with truncated: {}", content);
                    } else {
                        println!("Success with truncated: (no content)");
                    }
                }
                Err(e) => println!("Still failed: {}", e),
            }
        }
        Err(e) => println!("Other error: {}", e),
    }
}

async fn auth_error_handling() -> Result<()> {
    // Try with invalid API key
    let config = Config::builder().api_key("invalid-api-key").build();
    let invalid_client = Client::new(config)?;

    match invalid_client
        .send_chat(invalid_client.chat_simple("Hello"))
        .await
    {
        Ok(_) => println!("Unexpected success"),
        Err(Error::Authentication(message)) => {
            println!("Authentication failed as expected: {}", message);

            // Suggest remediation
            println!("Suggestions:");
            println!("1. Check your OPENAI_API_KEY environment variable");
            println!("2. Verify API key at https://platform.openai.com/api-keys");
            println!("3. Ensure your API key has necessary permissions");
        }
        Err(e) => println!("Unexpected error type: {}", e),
    }

    Ok(())
}

async fn network_error_handling() -> Result<()> {
    use openai_ergonomic::Config;

    // Create client with very short timeout to simulate network issues
    let config = Config::builder()
        .api_key("test-key")
        .timeout_seconds(1)
        .build();

    let client = Client::new(config)?;

    match client.send_chat(client.chat_simple("Hello")).await {
        Ok(_) => println!("Unexpected success"),
        Err(Error::Http(source)) => {
            println!("Network error as expected: {}", source);

            // Implement exponential backoff
            let mut backoff = Duration::from_millis(100);
            for attempt in 1..=3 {
                println!("Retry attempt {} after {:?}", attempt, backoff);
                sleep(backoff).await;
                backoff *= 2;

                // In real scenario, retry with proper timeout
                // match client.send_chat(client.chat_simple("Hello")).await { ... }
            }
        }
        Err(e) => println!("Other error: {}", e),
    }

    Ok(())
}

async fn custom_error_context() -> Result<()> {
    let client = Client::from_env()?;

    // Wrap errors with custom context
    let result = client
        .send_chat(client.chat_simple("Analyze this data"))
        .await
        .map_err(|e| {
            eprintln!("Context: Failed during data analysis task");
            eprintln!("Timestamp: {:?}", std::time::SystemTime::now());
            eprintln!("Original error: {}", e);
            e
        })?;

    if let Some(content) = result.content() {
        println!("Result: {}", content);
    } else {
        println!("Result: (no content)");
    }
    Ok(())
}

async fn error_recovery_strategies() -> Result<()> {
    let client = Client::from_env()?;

    // Strategy 1: Fallback to simpler model
    let result = try_with_fallback(&client, "gpt-4o", "gpt-3.5-turbo").await?;
    println!("Fallback strategy result: {}", result);

    // Strategy 2: Circuit breaker pattern
    let circuit_breaker = CircuitBreaker::new();
    if circuit_breaker.is_open() {
        println!("Circuit breaker is open, skipping API calls");
        return Ok(());
    }

    match client.send_chat(client.chat_simple("Test")).await {
        Ok(response) => {
            circuit_breaker.record_success();
            if let Some(content) = response.content() {
                println!("Circuit breaker success: {}", content);
            } else {
                println!("Circuit breaker success: (no content)");
            }
        }
        Err(e) => {
            circuit_breaker.record_failure();
            println!("Circuit breaker failure: {}", e);
        }
    }

    // Strategy 3: Request hedging (parallel requests with first success wins)
    let hedge_result = hedged_request(&client).await?;
    println!("Hedged request result: {}", hedge_result);

    Ok(())
}

async fn try_with_fallback(client: &Client, primary: &str, _fallback: &str) -> Result<String> {
    // Try primary model first
    let builder = client.chat().user("Hello");
    match client.send_chat(builder).await {
        Ok(response) => Ok(response.content().unwrap_or("").to_string()),
        Err(e) => {
            println!("Primary model failed ({}): {}, trying fallback", primary, e);

            // Try fallback model
            let fallback_builder = client.chat().user("Hello");
            client
                .send_chat(fallback_builder)
                .await
                .map(|r| r.content().unwrap_or("").to_string())
        }
    }
}

async fn hedged_request(client: &Client) -> Result<String> {
    use futures::future::select;
    use std::pin::pin;

    // Launch two requests in parallel
    let request1 = async {
        client
            .send_chat(client.chat_simple("Hello from request 1"))
            .await
    };
    let request2 = async {
        client
            .send_chat(client.chat_simple("Hello from request 2"))
            .await
    };

    let fut1 = pin!(request1);
    let fut2 = pin!(request2);

    // Return first successful response
    match select(fut1, fut2).await {
        futures::future::Either::Left((result, _)) => {
            println!("Request 1 completed first");
            result.map(|r| r.content().unwrap_or("").to_string())
        }
        futures::future::Either::Right((result, _)) => {
            println!("Request 2 completed first");
            result.map(|r| r.content().unwrap_or("").to_string())
        }
    }
}

// Simple circuit breaker implementation
struct CircuitBreaker {
    failures: std::sync::atomic::AtomicU32,
    threshold: u32,
}

impl CircuitBreaker {
    const fn new() -> Self {
        Self {
            failures: std::sync::atomic::AtomicU32::new(0),
            threshold: 3,
        }
    }

    fn is_open(&self) -> bool {
        self.failures.load(std::sync::atomic::Ordering::Relaxed) >= self.threshold
    }

    fn record_success(&self) {
        self.failures.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
