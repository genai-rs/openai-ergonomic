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
- **Azure `OpenAI`** - seamless support for Azure `OpenAI` deployments
- **Well-tested** - extensive test coverage with mock support
- **Well-documented** - rich documentation with examples

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
    // Build client from environment variables
    let client = Client::from_env()?.build();

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
use openai_ergonomic::Client;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build client from environment variables
    let client = Client::from_env()?.build();

    let builder = client
        .chat()
        .user("Tell me a story");

    let mut stream = client.send_chat_stream(builder).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
    Ok(())
}
```

### Custom HTTP Client with Retry Logic

You can provide your own `reqwest::Client` with custom retry, timeout, and middleware configuration.
**Note:** When using a custom HTTP client, you must configure the timeout on the `reqwest::Client` itself:

```rust,ignore
use openai_ergonomic::{Client, Config};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a retry policy with exponential backoff
    let retry_policy = ExponentialBackoff::builder()
        .build_with_max_retries(3);

    // Build a reqwest client with custom timeout
    let reqwest_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))  // Configure timeout here
        .build()?;

    // Add retry middleware
    let http_client = ClientBuilder::new(reqwest_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    // Create OpenAI client with custom HTTP client
    let config = Config::builder()
        .api_key("your-api-key")
        .http_client(http_client)
        .build();

    let client = Client::new(config)?.build();

    // Use the client normally - retries and timeout are handled automatically
    let response = client.chat_simple("Hello!").await?;
    println!("{}", response);
    Ok(())
}
```

### Azure `OpenAI` Support

The crate seamlessly supports Azure `OpenAI` deployments. Azure-specific configuration can be provided through environment variables or programmatically.

#### Using Environment Variables

```bash
export AZURE_OPENAI_ENDPOINT="https://my-resource.openai.azure.com"
export AZURE_OPENAI_API_KEY="your-azure-api-key"
export AZURE_OPENAI_DEPLOYMENT="gpt-4"
export AZURE_OPENAI_API_VERSION="2024-02-01"  # Optional, defaults to 2024-02-01
```

```rust,ignore
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build client from Azure environment variables
    let client = Client::from_env()?.build();

    // Use exactly the same API as standard OpenAI
    let response = client.chat_simple("Hello from Azure!").await?;
    println!("{}", response);
    Ok(())
}
```

#### Manual Configuration

```rust,ignore
use openai_ergonomic::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::builder()
        .api_key("your-azure-api-key")
        .api_base("https://my-resource.openai.azure.com")
        .azure_deployment("gpt-4")
        .azure_api_version("2024-02-01")
        .build();

    let client = Client::new(config)?.build();

    let response = client.chat_simple("Hello!").await?;
    println!("{}", response);
    Ok(())
}
```

**Note:** The library automatically handles the differences between Azure `OpenAI` and standard `OpenAI` (authentication, URL paths, API versioning). You use the same API regardless of the provider.

## Documentation

- [Getting Started Guide](docs/getting-started.md)
- [Architecture Overview](docs/architecture.md)
- **[API Coverage Status](docs/api-coverage.md)** - See what APIs are available
- **[Examples Index](docs/examples-index.md)** - Browse all examples by category
- [Responses-First Workflows](docs/responses_workflows.md)
- [Tool Orchestration](docs/tool_orchestration.md)
- [Vector Store Operations](docs/vector_store_operations.md)
- [Langfuse Integration](docs/langfuse-integration.md)
- [API Documentation](https://docs.rs/openai-ergonomic)

## Examples

The `examples/` directory contains comprehensive examples for all major `OpenAI` API features:

### Core Examples

- [**quickstart.rs**](examples/quickstart.rs) - Quick introduction to the library with basic usage patterns
- [**chat_streaming.rs**](examples/chat_streaming.rs) - Real-time chat streaming with Server-Sent Events (SSE)
- [**tool_calling_multiturn.rs**](examples/tool_calling_multiturn.rs) - Multi-turn tool calling with proper conversation history management
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
