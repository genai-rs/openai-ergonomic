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

### OpenTelemetry Observability with Langfuse

Track and monitor your `OpenAI` API calls with OpenTelemetry and Langfuse:

```rust,ignore
use openai_ergonomic::{Client, TelemetryContext};
use opentelemetry::global;
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup OpenTelemetry with Langfuse
    let exporter = ExporterBuilder::from_env()?.build()?;
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_attributes([
                    opentelemetry::KeyValue::new("service.name", "my-app"),
                ])
                .build(),
        )
        .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
        .build();
    global::set_tracer_provider(provider.clone());

    // Use the client with telemetry context
    let client = Client::from_env()?;

    let response = client
        .chat()
        .model("gpt-4o-mini")
        .user("Hello, world!")
        .with_user_id("user-123")           // Track by user
        .with_session_id("session-456")     // Group by session
        .with_tag("production")              // Add tags
        .send()
        .await?;

    println!("{}", response.content().unwrap_or("No content"));

    // Flush telemetry
    provider.shutdown()?;
    Ok(())
}
```

Enable the `telemetry` feature and set environment variables:

```toml
[dependencies]
openai-ergonomic = { version = "0.1", features = ["telemetry"] }
opentelemetry-langfuse = "0.5"
```

```bash
export OPENAI_API_KEY="sk-..."
export LANGFUSE_PUBLIC_KEY="pk-lf-..."
export LANGFUSE_SECRET_KEY="sk-lf-..."
export LANGFUSE_HOST="https://cloud.langfuse.com"  # optional, this is the default
```

The `telemetry` feature includes `opentelemetry_sdk` with the `experimental_trace_batch_span_processor_with_async_runtime` feature enabled for production-ready, efficient batch exporting with proper Tokio runtime support.

**Features provided:**
- Automatic span creation following OpenAI semantic conventions
- User ID and session ID tracking for grouping traces
- Custom tags and metadata
- Token usage metrics
- Model parameters tracking
- Non-blocking async batch export to Langfuse

See the [telemetry example](examples/telemetry_langfuse.rs) for a complete demonstration with multiple API calls and detailed usage patterns.

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

- [**telemetry_langfuse.rs**](examples/telemetry_langfuse.rs) - OpenTelemetry instrumentation with Langfuse for tracking API calls

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
