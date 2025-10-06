//! Example demonstrating the interceptor system for logging and observability.
//!
//! This example shows how to create custom interceptors to log API calls,
//! measure performance, and collect metrics.
//!
//! # Running
//!
//! ```bash
//! export OPENAI_API_KEY="your-api-key"
//! cargo run --example interceptor_logging
//! ```

use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, Client, ErrorContext, Interceptor,
};

/// Simple logging interceptor that prints API call details.
struct LoggingInterceptor;

#[async_trait::async_trait]
impl Interceptor for LoggingInterceptor {
    async fn before_request(
        &self,
        ctx: &mut BeforeRequestContext<'_>,
    ) -> openai_ergonomic::Result<()> {
        println!(
            "\n🚀 Starting {} request with model: {}",
            ctx.operation, ctx.model
        );
        println!(
            "📝 Request: {}",
            &ctx.request_json[..ctx.request_json.len().min(200)]
        );
        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> openai_ergonomic::Result<()> {
        println!(
            "✅ Completed {} request in {:?}",
            ctx.operation, ctx.duration
        );
        if let (Some(input), Some(output)) = (ctx.input_tokens, ctx.output_tokens) {
            println!(
                "📊 Tokens: {} input + {} output = {} total",
                input,
                output,
                input + output
            );
        }
        println!(
            "💬 Response: {}",
            &ctx.response_json[..ctx.response_json.len().min(200)]
        );
        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        println!(
            "❌ Error in {} request after {:?}: {}",
            ctx.operation, ctx.duration, ctx.error
        );
    }
}

/// Metrics interceptor that tracks API usage.
struct MetricsInterceptor {
    total_calls: std::sync::Arc<std::sync::atomic::AtomicU64>,
    total_tokens: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl MetricsInterceptor {
    fn new() -> Self {
        Self {
            total_calls: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_tokens: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    fn print_stats(&self) {
        let calls = self.total_calls.load(std::sync::atomic::Ordering::Relaxed);
        let tokens = self.total_tokens.load(std::sync::atomic::Ordering::Relaxed);
        println!("\n📈 API Statistics:");
        println!("   Total calls: {calls}");
        println!("   Total tokens: {tokens}");
        if calls > 0 {
            println!("   Avg tokens/call: {}", tokens / calls);
        }
    }
}

#[async_trait::async_trait]
impl Interceptor for MetricsInterceptor {
    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> openai_ergonomic::Result<()> {
        self.total_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let (Some(input), Some(output)) = (ctx.input_tokens, ctx.output_tokens) {
            let total = u64::try_from(input + output).unwrap_or(0);
            self.total_tokens
                .fetch_add(total, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Interceptor Example ===\n");

    // Create metrics interceptor (keep reference to print stats later)
    let metrics = MetricsInterceptor::new();
    let metrics_clone = MetricsInterceptor {
        total_calls: metrics.total_calls.clone(),
        total_tokens: metrics.total_tokens.clone(),
    };

    // Create client with multiple interceptors
    let client = Client::from_env()?
        .with_interceptor(Box::new(LoggingInterceptor))
        .with_interceptor(Box::new(metrics));

    println!("--- Example 1: Simple chat ---");
    let response = client.send_chat(client.chat_simple("What is 2+2?")).await?;
    println!(
        "\nFinal answer: {}\n",
        response.content().unwrap_or("No content")
    );

    println!("\n--- Example 2: Another chat ---");
    let response2 = client
        .send_chat(client.chat_simple("What is the capital of France?"))
        .await?;
    println!(
        "\nFinal answer: {}\n",
        response2.content().unwrap_or("No content")
    );

    // Print final statistics
    metrics_clone.print_stats();

    Ok(())
}
