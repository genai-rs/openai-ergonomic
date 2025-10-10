//! Simple example of using the Langfuse interceptor for LLM observability.
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

use openai_ergonomic::{Builder, Client, LangfuseConfig, LangfuseInterceptor};
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

    println!(" Initializing OpenAI client with Langfuse observability...\n");

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

    println!(" Client initialized successfully!");
    println!(" Traces will be sent to Langfuse for monitoring\n");

    // Make a simple chat completion - tracing is automatic!
    println!(" Making a simple chat completion request...");
    let request = client
        .chat_simple("What is 2 + 2? Answer with just the number.")
        .build()?;
    let response = client.execute_chat(request).await?;

    println!(" Response: {:?}", response.content());

    println!("\n Done! Check your Langfuse dashboard to see the traces.");
    println!("   - Look for traces with the operation name 'chat'");
    println!("   - Each trace includes request/response details and token usage");

    // Shutdown the tracer provider to flush all spans
    println!("\n⏳ Flushing spans to Langfuse...");
    provider.shutdown()?;

    Ok(())
}
