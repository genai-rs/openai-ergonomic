# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic.svg)](https://crates.io/crates/openai-ergonomic)
[![Documentation](https://docs.rs/openai-ergonomic/badge.svg)](https://docs.rs/openai-ergonomic)
[![CI](https://github.com/genai-rs/openai-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/openai-ergonomic/actions)
[![Coverage](https://codecov.io/gh/genai-rs/openai-ergonomic/branch/main/graph/badge.svg)](https://codecov.io/gh/genai-rs/openai-ergonomic)
[![MSRV](https://img.shields.io/badge/MSRV-1.82-blue)](https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html)
[![License](https://img.shields.io/crates/l/openai-ergonomic.svg)](https://github.com/genai-rs/openai-ergonomic#license)

Ergonomic Rust wrapper for the `OpenAI` API, providing type-safe builder patterns and async/await support.

## Features

- 🛡️ **Type-safe** - Full type safety with builder patterns using `bon`
- ⚡ **Async/await** - Built on `tokio` and `reqwest` for modern async Rust
- 🔄 **Streaming** - First-class support for streaming responses
- 📝 **Comprehensive** - Covers all `OpenAI` API endpoints
- 🧪 **Well-tested** - Extensive test coverage with mock support
- 📚 **Well-documented** - Rich documentation with examples

## Status

🚧 **Under Construction** - This crate is currently being developed and is not yet ready for production use.

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

## Documentation

- [Getting Started Guide](docs/getting-started.md)
- [Architecture Overview](docs/architecture.md)
- [API Documentation](https://docs.rs/openai-ergonomic)
- [Examples](examples/)

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
- ✅ Comprehensive documentation and inline comments
- ✅ Error handling best practices
- ✅ Real-world use cases and patterns
- ✅ Progressive complexity from basic to advanced usage

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