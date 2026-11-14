# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic)](https://crates.io/crates/openai-ergonomic)
[![docs.rs](https://img.shields.io/docsrs/openai-ergonomic)](https://docs.rs/openai-ergonomic)
[![MSRV](https://img.shields.io/crates/msrv/openai-ergonomic)](https://github.com/genai-rs/openai-ergonomic/blob/main/Cargo.toml)
[![Dependencies](https://deps.rs/crate/openai-ergonomic/latest/status.svg)](https://deps.rs/crate/openai-ergonomic)
[![CI](https://github.com/genai-rs/openai-ergonomic/actions/workflows/ci.yml/badge.svg)](https://github.com/genai-rs/openai-ergonomic/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/openai-ergonomic)](#license)

Ergonomic Rust client for the `OpenAI` API and the services that speak it (Azure `OpenAI`,
`OpenRouter`, Groq, Ollama, gateways): fluent builders for chat completions, the Responses API,
embeddings, images, audio, files, assistants, batches and more, on top of the generated
[openai-client-base](https://github.com/genai-rs/openai-client-base). It sends through a
[tower](https://docs.rs/tower)-based HTTP client
([`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime)), natively and on WebAssembly.

## Quick start

```bash
cargo add openai-ergonomic
cargo add tokio --features full
```

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads OPENAI_API_KEY (and the other OPENAI_* / AZURE_OPENAI_* variables).
    let client = Client::from_env()?.build();
    let response = client
        .send_chat(client.chat().system("Be brief.").user("Hello, world!"))
        .await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

[Getting started](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/getting-started.md)
goes on from here: the Responses API, streaming, errors and the examples.

## Key features

- **Builders** for chat completions, the Responses API (`/responses`, with typed streaming
  events and stored-response operations), embeddings, images, audio, files,
  uploads, vector stores, assistants and threads, batches, fine-tuning, moderations and usage
  ([getting started](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/getting-started.md),
  [API coverage](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/api-coverage.md)).
- **Providers**: `OpenAI`, Azure `OpenAI`, `OpenRouter`, Groq, Ollama or any `OpenAI`-compatible
  API, with refreshable credentials (Entra ID tokens, gateways) through an `AuthProvider`
  ([configuration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/configuration.md)).
- **A tower HTTP stack** with retries (honouring `Retry-After`), timeouts, stream idle timeouts,
  rate-limit headers, a retry classification of errors and layers of your own
  ([configuration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/configuration.md#retries-timeouts-and-custom-http-clients)).
- **Streaming** of chat completions and of Responses API events, with completion reports
  ([streaming](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/streaming.md)).
- **Tools** for chat completions and the Responses API: typed function tools, tools that return
  images or files, custom (grammar) tools, namespaces and tools only known at runtime (MCP), in a
  registry that answers every call
  ([tools](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/tool_framework.md)).
- **Interceptors** around every request and stream, with a Langfuse / OpenTelemetry integration
  ([interceptors](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/interceptors.md)).
- **Bring your own types**: send and decode types of your own through the same client
  ([chat and responses](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/chat-and-responses.md#bring-your-own-types)).
- **Files and media**: parallel, retried large uploads; images, speech and file contents
  streamed to disk ([files and media](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/files-and-media.md)).
- **Webhooks**: verify `OpenAI` webhook signatures and decode their events
  ([webhooks](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/webhooks.md)).
- **WebAssembly**: builds for `wasm32-unknown-unknown` (browsers, Node.js, Cloudflare Workers)
  ([WebAssembly](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/webassembly.md)).

## Documentation

| | |
| --- | --- |
| [Getting started](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/getting-started.md) | installing, a client, chat, the Responses API, errors, the examples |
| [Configuration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/configuration.md) | providers (Azure, `OpenRouter`, Groq, Ollama, custom), authentication, retries, timeouts, custom HTTP clients, per-call options |
| [Chat and responses](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/chat-and-responses.md), [Responses workflows](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/responses_workflows.md) | the builders, structured output, vision, bring your own types |
| [Streaming](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/streaming.md), [completion contract](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/streaming-completion-contract.md) | streamed chat completions and responses, completion reports |
| [Tool framework](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/tool_framework.md), [tool orchestration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/tool_orchestration.md) | typed tools, the registry, answering calls on both APIs, multi-turn tool calling |
| [Interceptors](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/interceptors.md), [Langfuse integration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/langfuse-integration.md) | hooks around requests and streams, typed state, Langfuse / OpenTelemetry |
| [Files and media](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/files-and-media.md), [vector stores](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/vector_store_operations.md) | uploads (`Upload`), large files, images, speech, downloads |
| [Webhooks](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/webhooks.md) | signature verification and events (feature `webhook`) |
| [WebAssembly](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/webassembly.md) | building for `wasm32-unknown-unknown`, what differs, the Node.js tests |
| [Migration](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/migration.md) | upgrading release by release |
| [Examples index](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/examples-index.md), [architecture](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/architecture.md) | the `examples/` directory; how the crate is put together |
| [Development](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/development.md), [contributing](https://github.com/genai-rs/openai-ergonomic/blob/main/CONTRIBUTING.md) | building and testing, local CI, the WebAssembly suite |
| [API reference](https://docs.rs/openai-ergonomic) | docs.rs |

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/genai-rs/openai-ergonomic/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/genai-rs/openai-ergonomic/blob/main/LICENSE-MIT)
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
