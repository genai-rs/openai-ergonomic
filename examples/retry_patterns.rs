#![allow(clippy::uninlined_format_args)]
//! Resilience and retry strategies for OpenAI API calls.
//!
//! This example demonstrates:
//! - Exponential backoff
//! - Retry with jitter
//! - Circuit breaker pattern
//! - Timeout and deadline management
//! - Request hedging
//! - Fallback strategies
//! - Idempotency keys
//!
//! Run with: `cargo run --example retry_patterns`

use openai_ergonomic::{Client, Config, Error, Result};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Retry and Resilience Patterns ===\n");

    // Initialize client
    let client = Client::from_env()?;

    // Example 1: Simple retry
    println!("1. Simple Retry:");
    simple_retry(&client).await?;

    // Example 2: Exponential backoff
    println!("\n2. Exponential Backoff:");
    exponential_backoff(&client).await?;

    // Example 3: Retry with jitter
    println!("\n3. Retry with Jitter:");
    retry_with_jitter(&client).await?;

    // Example 4: Circuit breaker
    println!("\n4. Circuit Breaker:");
    circuit_breaker_example(&client).await?;

    // Example 5: Timeout management
    println!("\n5. Timeout Management:");
    timeout_management(&client).await;

    // Example 6: Request hedging
    println!("\n6. Request Hedging:");
    request_hedging(&client).await?;

    // Example 7: Fallback chain
    println!("\n7. Fallback Chain:");
    fallback_chain(&client).await?;

    // Example 8: Idempotency
    println!("\n8. Idempotency:");
    idempotency_example(&client).await?;

    Ok(())
}

async fn simple_retry(client: &Client) -> Result<()> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        println!("Attempt {}/{}", attempt, MAX_RETRIES);

        match client.send_chat(client.chat_simple("Hello")).await {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("Success: {}", content);
                } else {
                    println!("Success: (no content)");
                }
                return Ok(());
            }
            Err(e) if attempt < MAX_RETRIES => {
                println!("Failed (attempt {}): {}. Retrying...", attempt, e);
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                println!("All retries exhausted");
                return Err(e);
            }
        }
    }

    Ok(())
}

async fn exponential_backoff(client: &Client) -> Result<()> {
    const MAX_RETRIES: u32 = 5;
    const BASE_DELAY: Duration = Duration::from_millis(100);
    const MAX_DELAY: Duration = Duration::from_secs(32);

    let mut delay = BASE_DELAY;

    for attempt in 1..=MAX_RETRIES {
        match client
            .send_chat(client.chat_simple("Hello with backoff"))
            .await
        {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("Success after {} attempts: {}", attempt, content);
                } else {
                    println!("Success after {} attempts: (no content)", attempt);
                }
                return Ok(());
            }
            Err(Error::RateLimit(_message)) => {
                // Use default delay for rate limiting
                let wait_time = delay;
                println!(
                    "Rate limited (attempt {}). Waiting {:?}...",
                    attempt, wait_time
                );
                sleep(wait_time).await;

                // Double the delay for next attempt
                delay = (delay * 2).min(MAX_DELAY);
            }
            Err(e) if attempt < MAX_RETRIES => {
                println!("Error (attempt {}): {}. Waiting {:?}...", attempt, e, delay);
                sleep(delay).await;

                // Exponential increase with cap
                delay = (delay * 2).min(MAX_DELAY);
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

async fn retry_with_jitter(client: &Client) -> Result<()> {
    use rand::Rng;

    const MAX_RETRIES: u32 = 5;
    const BASE_DELAY_MS: u64 = 100;

    let mut rng = rand::thread_rng();

    for attempt in 1..=MAX_RETRIES {
        match client
            .send_chat(client.chat_simple("Hello with jitter"))
            .await
        {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("Success: {}", content);
                } else {
                    println!("Success: (no content)");
                }
                return Ok(());
            }
            Err(e) if attempt < MAX_RETRIES => {
                // Calculate delay with jitter
                let base = BASE_DELAY_MS * 2_u64.pow(attempt - 1);
                let jitter = rng.gen_range(0..=base / 2);
                let delay = Duration::from_millis(base + jitter);

                println!(
                    "Attempt {} failed: {}. Retrying in {:?} (with jitter)...",
                    attempt, e, delay
                );
                sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

async fn circuit_breaker_example(client: &Client) -> Result<()> {
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, Duration::from_secs(5)));

    for i in 1..=10 {
        println!("Request {}: ", i);

        // Check circuit state
        match circuit_breaker
            .call(|| async {
                client
                    .send_chat(client.chat_simple("Circuit breaker test"))
                    .await
            })
            .await
        {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("  Success: {}", content);
                } else {
                    println!("  Success: (no content)");
                }
            }
            Err(CircuitBreakerError::Open) => {
                println!("  Circuit is OPEN - skipping request");
                sleep(Duration::from_secs(1)).await;
            }
            Err(CircuitBreakerError::RequestFailed(e)) => {
                println!("  Request failed: {}", e);
            }
        }

        // Small delay between requests
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn timeout_management(client: &Client) {
    // Example 1: Per-request timeout
    println!("Per-request timeout:");
    match timeout(
        Duration::from_secs(5),
        client.send_chat(client.chat_simple("Hello")),
    )
    .await
    {
        Ok(Ok(response)) => {
            if let Some(content) = response.content() {
                println!("Response received: {}", content);
            } else {
                println!("Response received: (no content)");
            }
        }
        Ok(Err(e)) => println!("API error: {}", e),
        Err(_) => println!("Request timed out after 5 seconds"),
    }

    // Example 2: Deadline-based timeout
    println!("\nDeadline-based timeout:");
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        let remaining = deadline - Instant::now();
        println!("Time remaining: {:?}", remaining);

        match timeout(
            remaining,
            client.send_chat(client.chat_simple("Quick response")),
        )
        .await
        {
            Ok(Ok(response)) => {
                if let Some(content) = response.content() {
                    println!("Got response: {}", content);
                } else {
                    println!("Got response: (no content)");
                }
                break;
            }
            Ok(Err(e)) => {
                println!("Error: {}. Retrying...", e);
                sleep(Duration::from_secs(1)).await;
            }
            Err(_) => {
                println!("Deadline exceeded");
                break;
            }
        }
    }

    // Example 3: Adaptive timeout
    println!("\nAdaptive timeout:");
    let mut adaptive_timeout = Duration::from_secs(2);

    for _attempt in 1..=3 {
        let start = Instant::now();

        match timeout(
            adaptive_timeout,
            client.send_chat(client.chat_simple("Adaptive")),
        )
        .await
        {
            Ok(Ok(response)) => {
                let elapsed = start.elapsed();
                println!(
                    "Success in {:?}. Adjusting timeout for next request.",
                    elapsed
                );
                // Adjust timeout based on actual response time
                adaptive_timeout = elapsed * 2; // Update timeout based on response time
                if let Some(content) = response.content() {
                    println!("Response: {}", content);
                } else {
                    println!("Response: (no content)");
                }
                break;
            }
            Ok(Err(e)) => println!("Error: {}", e),
            Err(_) => {
                println!(
                    "Timeout after {:?}. Increasing for next attempt.",
                    adaptive_timeout
                );
                adaptive_timeout *= 2;
            }
        }
    }
}

async fn request_hedging(client: &Client) -> Result<()> {
    use futures::future::{select, Either};
    use std::pin::pin;

    println!("Launching hedged requests...");

    // Launch multiple requests with staggered starts
    let request1 = async {
        println!("Request 1 started");
        client
            .send_chat(client.chat_simple("Hedged request 1"))
            .await
    };

    let request2 = async {
        sleep(Duration::from_millis(200)).await;
        println!("Request 2 started (200ms delay)");
        client
            .send_chat(client.chat_simple("Hedged request 2"))
            .await
    };

    let fut1 = pin!(request1);
    let fut2 = pin!(request2);

    // Return first successful response
    match select(fut1, fut2).await {
        Either::Left((result, _)) => {
            println!("Request 1 won the race");
            result.map(|r| {
                if let Some(content) = r.content() {
                    println!("Result: {}", content);
                } else {
                    println!("Result: (no content)");
                }
            })
        }
        Either::Right((result, _)) => {
            println!("Request 2 won the race");
            result.map(|r| {
                if let Some(content) = r.content() {
                    println!("Result: {}", content);
                } else {
                    println!("Result: (no content)");
                }
            })
        }
    }
}

async fn fallback_chain(client: &Client) -> Result<()> {
    // Define fallback chain
    let strategies = vec![
        ("GPT-4o", "gpt-4o", 1024),
        ("GPT-4o-mini", "gpt-4o-mini", 512),
        ("GPT-3.5", "gpt-3.5-turbo", 256),
    ];

    let prompt = "Explain quantum computing";

    for (name, _model, max_tokens) in strategies {
        println!("Trying {} (max_tokens: {})", name, max_tokens);

        let builder = client.chat().user(prompt).max_completion_tokens(max_tokens);
        match client.send_chat(builder).await {
            Ok(response) => {
                println!("Success with {}", name);
                if let Some(content) = response.content() {
                    println!("Response: {}...", &content[..content.len().min(100)]);
                }
                return Ok(());
            }
            Err(e) => {
                println!("Failed with {}: {}", name, e);
            }
        }
    }

    println!("All fallback strategies exhausted");
    Ok(())
}

async fn idempotency_example(_client: &Client) -> Result<()> {
    // Generate idempotency key
    let idempotency_key = generate_idempotency_key();
    println!("Using idempotency key: {}", idempotency_key);

    // Simulate retrying the same request
    for attempt in 1..=3 {
        println!("\nAttempt {} with same idempotency key", attempt);

        // In a real implementation, you'd pass the idempotency key in headers
        let mut headers = std::collections::HashMap::new();
        headers.insert("Idempotency-Key".to_string(), idempotency_key.clone());

        let config = Config::builder()
            .api_key(std::env::var("OPENAI_API_KEY").unwrap_or_default())
            .build();

        // Note: Headers (including idempotency key) are not yet supported in current API

        let client_with_idempotency = Client::new(config)?;

        match client_with_idempotency
            .send_chat(client_with_idempotency.chat_simple("Idempotent request"))
            .await
        {
            Ok(response) => {
                if let Some(content) = response.content() {
                    println!("Response: {}", content);
                } else {
                    println!("Response: (no content)");
                }
                // Server should return same response for same idempotency key
            }
            Err(e) => println!("Error: {}", e),
        }

        if attempt < 3 {
            sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

fn generate_idempotency_key() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let random: u32 = rand::random();
    format!("req-{}-{}", timestamp, random)
}

// Circuit Breaker Implementation
#[derive(Debug)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

struct CircuitBreaker {
    state: Arc<tokio::sync::RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicU64>,
    threshold: u32,
    timeout: Duration,
}

#[derive(Debug)]
enum CircuitBreakerError {
    Open,
    RequestFailed(Error),
}

impl CircuitBreaker {
    fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(tokio::sync::RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            threshold,
            timeout,
        }
    }

    async fn call<F, Fut, T>(&self, f: F) -> std::result::Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit should transition from Open to HalfOpen
        let mut state = self.state.write().await;
        match *state {
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if now - last_failure > self.timeout.as_secs() {
                    println!("  Circuit transitioning to HALF-OPEN");
                    *state = CircuitState::HalfOpen;
                } else {
                    return Err(CircuitBreakerError::Open);
                }
            }
            _ => {}
        }
        drop(state);

        // Execute the request
        match f().await {
            Ok(result) => {
                let mut state = self.state.write().await;
                if matches!(*state, CircuitState::HalfOpen) {
                    println!("  Circuit transitioning to CLOSED");
                    *state = CircuitState::Closed;
                }
                self.failure_count.store(0, Ordering::Relaxed);
                Ok(result)
            }
            Err(e) => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                self.last_failure_time.store(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    Ordering::Relaxed,
                );

                let mut state = self.state.write().await;
                if count >= self.threshold {
                    println!("  Circuit transitioning to OPEN (failures: {})", count);
                    *state = CircuitState::Open;
                }

                Err(CircuitBreakerError::RequestFailed(e))
            }
        }
    }
}
