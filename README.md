# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic.svg)](https://crates.io/crates/openai-ergonomic)
[![Documentation](https://docs.rs/openai-ergonomic/badge.svg)](https://docs.rs/openai-ergonomic)
[![CI](https://github.com/genai-rs/openai-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/openai-ergonomic/actions)
[![Coverage](https://codecov.io/gh/genai-rs/openai-ergonomic/branch/main/graph/badge.svg)](https://codecov.io/gh/genai-rs/openai-ergonomic)
[![MSRV](https://img.shields.io/badge/MSRV-1.82-blue)](https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html)
[![License](https://img.shields.io/crates/l/openai-ergonomic.svg)](https://github.com/genai-rs/openai-ergonomic#license)

Ergonomic Rust wrapper for the `OpenAI` API, providing type-safe builder patterns and async/await support.

## Features

- **Type-safe** - full type safety with builder patterns using `bon`
- **Async/await** - built on `tokio` and `reqwest` for modern async Rust
- **Streaming** - first-class support for streaming responses
- **Comprehensive** - covers all `OpenAI` API endpoints
- **Well-tested** - extensive test coverage with mock support
- **Well-documented** - rich documentation with examples
- **Observable** - optional OpenTelemetry instrumentation with Langfuse support

## Status

**Status:** under construction. The crate is still in active development and not yet ready for production use.

## Quick Start

Add `openai-ergonomic` to your `Cargo.toml`:

```toml
[dependencies]
openai-ergonomic = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust,ignore
use openai_ergonomic::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?
        .api_key("your-api-key-here")
        .build();

    let response = client
        .chat_completions()
        .model("gpt-4")
        .message("user", "Hello, world!")
        .send()
        .await?;

    println!("{}", response.choices[0].message.content);
    Ok(())
}
```

### Streaming Example

```rust,ignore
use openai_ergonomic::{Client, Config};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?
        .api_key("your-api-key-here")
        .build();

    let mut stream = client
        .chat_completions()
        .model("gpt-4")
        .message("user", "Tell me a story")
        .stream()
        .await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.choices[0].delta.content {
            print!("{}", content);
        }
    }
    Ok(())
}
```

### Custom HTTP Client with Retry Logic

You can provide your own `reqwest::Client` with custom retry, timeout, and middleware configuration:

```rust,ignore
use openai_ergonomic::{Client, Config};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a retry policy with exponential backoff
    let retry_policy = ExponentialBackoff::builder()
        .build_with_max_retries(3);

    // Build a client with retry middleware
    let http_client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    // Create OpenAI client with custom HTTP client
    let config = Config::builder()
        .api_key("your-api-key")
        .http_client(http_client.into())
        .build();

    let client = Client::new(config)?;

    // Use the client normally - retries are handled automatically
    let response = client.chat_simple("Hello!").await?;
    println!("{}", response);
    Ok(())
}
```

### Observability with Interceptors

Track and monitor your `OpenAI` API calls using interceptors with OpenTelemetry support.

#### Standard OpenTelemetry

Use the `OpenTelemetryInterceptor` with any OpenTelemetry-compatible backend (Jaeger, Zipkin, OTLP, etc.):

```rust,ignore
use openai_ergonomic::{Client, OpenTelemetryInterceptor};
use opentelemetry::global;
use opentelemetry_sdk::trace::SdkTracerProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup OpenTelemetry with your preferred exporter
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(/* your exporter */)
        .build();
    global::set_tracer_provider(provider.clone());

    // Create client with OpenTelemetry interceptor
    let client = Client::from_env()?
        .with_interceptor(Box::new(OpenTelemetryInterceptor::new()));

    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or("No content"));

    provider.shutdown()?;
    Ok(())
}
```

#### Langfuse Integration

Use the `LangfuseInterceptor` for enhanced observability with user/session tracking:

```rust,ignore
use openai_ergonomic::{Client, LangfuseInterceptor, TelemetryContext};
use opentelemetry::global;
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
use opentelemetry_sdk::trace::SdkTracerProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup OpenTelemetry with Langfuse
    let exporter = ExporterBuilder::from_env()?.build()?;
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
        .build();
    global::set_tracer_provider(provider.clone());

    // Create context with user/session tracking
    let context = TelemetryContext::new()
        .with_user_id("user-123")
        .with_session_id("session-456")
        .with_tag("production")
        .with_metadata("region", "us-east-1");

    // Create client with Langfuse interceptor
    let client = Client::from_env()?
        .with_interceptor(Box::new(LangfuseInterceptor::with_context(context)));

    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or("No content"));

    provider.shutdown()?;
    Ok(())
}
```

Enable the `telemetry` feature:

```toml
[dependencies]
openai-ergonomic = { version = "0.1", features = ["telemetry"] }
# For Langfuse:
opentelemetry-langfuse = "0.5"
```

Set environment variables for Langfuse:

```bash
export OPENAI_API_KEY="sk-..."
export LANGFUSE_PUBLIC_KEY="pk-lf-..."
export LANGFUSE_SECRET_KEY="sk-lf-..."
export LANGFUSE_HOST="https://cloud.langfuse.com"  # optional
```

**Interceptor Features:**
- **OpenTelemetryInterceptor**: Standard semantic conventions (`gen_ai.*` attributes)
- **LangfuseInterceptor**: Enhanced with user/session tracking, tags, metadata, and full request/response capture
- Production-ready async batch exporting with Tokio runtime support
- Token usage metrics and model parameters tracking
- Composable middleware pattern (chain multiple interceptors)

See [telemetry_langfuse.rs](examples/telemetry_langfuse.rs) and [opentelemetry_standard.rs](examples/opentelemetry_standard.rs) for complete examples.

## Documentation

- [Getting Started Guide](docs/getting-started.md)
- [Architecture Overview](docs/architecture.md)
- **[📊 API Coverage Status](docs/api-coverage.md)** - See what APIs are available
- **[📚 Examples Index](docs/examples-index.md)** - Browse all examples by category
- [Responses-First Workflows](docs/guides/responses_workflows.md)
- [Tool Orchestration](docs/guides/tool_orchestration.md)
- [Vector Store Operations](docs/guides/vector_store_operations.md)
- [Migrating from `openai-builders`](docs/guides/migrating_from_builders.md)
- [API Documentation](https://docs.rs/openai-ergonomic)

## Examples

The `examples/` directory contains comprehensive examples for all major `OpenAI` API features:

### Core Examples

- [**quickstart.rs**](examples/quickstart.rs) - Quick introduction to the library with basic usage patterns
- [**responses_comprehensive.rs**](examples/responses_comprehensive.rs) - Complete responses API demonstration including function calling and web search
- [**responses_streaming.rs**](examples/responses_streaming.rs) - Real-time streaming responses with progress indicators
- [**chat_comprehensive.rs**](examples/chat_comprehensive.rs) - Full chat completions API with conversation history
- [**structured_outputs.rs**](examples/structured_outputs.rs) - JSON mode and schema-based structured outputs
- [**vision_chat.rs**](examples/vision_chat.rs) - Image understanding with GPT-4 Vision

### Media & AI Capabilities

- [**audio_speech.rs**](examples/audio_speech.rs) - Text-to-speech generation with multiple voices
- [**audio_transcription.rs**](examples/audio_transcription.rs) - Speech-to-text transcription and translation
- [**images_comprehensive.rs**](examples/images_comprehensive.rs) - Image generation, editing, and variations
- [**embeddings.rs**](examples/embeddings.rs) - Vector embeddings with similarity search patterns

### Advanced APIs

- [**assistants_basic.rs**](examples/assistants_basic.rs) - Introduction to the Assistants API with threads and tools

### Observability & Monitoring

- [**opentelemetry_standard.rs**](examples/opentelemetry_standard.rs) - Standard OpenTelemetry interceptor with any backend (Jaeger, Zipkin, OTLP, etc.)
- [**telemetry_langfuse.rs**](examples/telemetry_langfuse.rs) - Langfuse interceptor with user/session tracking and enhanced observability
- [**interceptor_logging.rs**](examples/interceptor_logging.rs) - Custom interceptors for logging and metrics collection

Run any example with:

```bash
# Set your OpenAI API key
export OPENAI_API_KEY="your-api-key-here"

# Run an example
cargo run --example quickstart
cargo run --example responses_streaming
cargo run --example vision_chat
```

Each example includes:
- Comprehensive documentation and inline comments
- Error handling best practices
- Real-world use cases and patterns
- Progressive complexity from basic to advanced usage

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
