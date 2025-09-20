# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic)](https://crates.io/crates/openai-ergonomic)
[![Documentation](https://docs.rs/openai-ergonomic/badge.svg)](https://docs.rs/openai-ergonomic)
[![CI](https://github.com/timvw/openai-ergonomic/actions/workflows/ci.yml/badge.svg)](https://github.com/timvw/openai-ergonomic/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.82%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/crates/l/openai-ergonomic)](LICENSE-MIT)

An ergonomic Rust wrapper for the OpenAI API, providing a builder-pattern interface for easy interaction with OpenAI's services.

## Features

- 🚀 **Builder Pattern API** - Intuitive, chainable methods for constructing API requests
- 🔄 **Streaming Support** - First-class support for streaming responses
- 🛡️ **Type Safety** - Leverage Rust's type system for compile-time correctness
- 📝 **Comprehensive Coverage** - Support for Chat, Assistants, Audio, Images, and more
- ⚡ **Async/Await** - Built on modern async Rust patterns
- 🔧 **Flexible Configuration** - Multiple ways to configure authentication and settings

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
openai-ergonomic = "0.1.0"
```

## Quick Start

```rust
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (uses OPENAI_API_KEY environment variable by default)
    let client = Client::new();

    // Create a chat completion
    let response = client
        .chat()
        .model("gpt-4")
        .user("What is the capital of France?")
        .send()
        .await?;

    println!("{}", response.content()?);
    Ok(())
}
```

## Configuration

### Environment Variables

The client automatically reads from environment variables:

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_ORG_ID="org-..."  # Optional
export OPENAI_PROJECT_ID="proj-..."  # Optional
```

### Explicit Configuration

```rust
use openai_ergonomic::{Client, Config};

let client = Client::from_config(Config {
    api_key: "sk-...".to_string(),
    org_id: Some("org-...".to_string()),
    project_id: Some("proj-...".to_string()),
    base_url: None,  // Uses default OpenAI API URL
});
```

## Examples

The `examples/` directory contains comprehensive examples for all major features:

### Core Examples
- [`quickstart.rs`](examples/quickstart.rs) - Get started in 5 minutes
- [`responses_comprehensive.rs`](examples/responses_comprehensive.rs) - Modern API patterns
- [`chat_comprehensive.rs`](examples/chat_comprehensive.rs) - Chat completions with history
- [`assistants_basic.rs`](examples/assistants_basic.rs) - Assistant API introduction

### Specialized Examples
- [`audio_speech.rs`](examples/audio_speech.rs) - Text-to-speech generation
- [`images_comprehensive.rs`](examples/images_comprehensive.rs) - Image generation and editing
- [`embeddings.rs`](examples/embeddings.rs) - Vector embeddings
- [`structured_outputs.rs`](examples/structured_outputs.rs) - JSON mode and schemas

Run examples with:

```bash
cargo run --example quickstart
```

## API Coverage

### ✅ Implemented
- [ ] Chat Completions
- [ ] Assistants API
- [ ] Audio (Speech, Transcription, Translation)
- [ ] Images (Generation, Edit, Variations)
- [ ] Embeddings
- [ ] Files
- [ ] Fine-tuning
- [ ] Batch Processing
- [ ] Vector Stores
- [ ] Moderations
- [ ] Models

### 🚧 Coming Soon
- [ ] Realtime API
- [ ] Advanced streaming patterns
- [ ] Middleware support
- [ ] Request/response interceptors

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

For security concerns, please see [SECURITY.md](SECURITY.md).

## License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Links

- [API Documentation](https://docs.rs/openai-ergonomic)
- [Crate on crates.io](https://crates.io/crates/openai-ergonomic)
- [GitHub Repository](https://github.com/timvw/openai-ergonomic)
- [OpenAI API Documentation](https://platform.openai.com/docs)