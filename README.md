# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic.svg)](https://crates.io/crates/openai-ergonomic)
[![Documentation](https://docs.rs/openai-ergonomic/badge.svg)](https://docs.rs/openai-ergonomic)
[![CI](https://github.com/genai-rs/openai-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/openai-ergonomic/actions)
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

```rust
use openai_ergonomic::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new()
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

```rust
use openai_ergonomic::OpenAIClient;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new()
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

- **Basic Usage**: Simple chat completions and responses
- **Streaming**: Real-time response streaming
- **Function Calling**: Tool integration and function calling
- **Vision**: Image understanding and analysis
- **Audio**: Speech-to-text and text-to-speech
- **Assistants**: Assistant API with file handling
- **Embeddings**: Vector embeddings generation
- **Images**: Image generation and editing

Run an example:

```bash
cargo run --example quickstart
```

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