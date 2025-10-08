# Architecture Overview

This document outlines how `openai-ergonomic` is structured and how the major
modules interact to provide an ergonomic layer on top of
[`openai-client-base`](https://github.com/openai/openai-client-base).

## Design Principles

### Ergonomic by Default
- **Builder pattern everywhere** – complex request payloads are assembled through
  fluent, type-safe builders
- **Sensible defaults** – most helpers pick a reasonable model and configuration
  so the developer can focus on intent
- **Progressive disclosure** – the public API starts simple but exposes lower
  level control when needed

### Async-first
- Runs on Tokio and Reqwest for modern async usage
- Streaming helpers return future-friendly abstractions (planned expansion)
- Builders avoid blocking by deferring work until `Client` executes them

### Layered Architecture
```
┌─────────────────────────┐
│  openai-ergonomic       │  ← ergonomic client, builders, helpers
├─────────────────────────┤
│  openai-client-base     │  ← generated OpenAPI client (models + apis)
├─────────────────────────┤
│  reqwest, tokio, bon    │  ← transport + builder foundations
└─────────────────────────┘
```

## Module Structure

```
src/
├── lib.rs         # Crate root & public re-exports
├── client.rs      # High-level Client wrapper around openai-client-base
├── config.rs      # Builder for configuration and env loading helpers
├── errors.rs      # Unified Error type + streaming helpers
├── responses/     # Thin response wrappers + convenience helpers
├── builders/      # Fluent request builders organised by API surface
└── test_utils.rs  # Mock server helpers for examples/tests
```

### Client (`client.rs`)

The `Client` struct owns shared configuration and exposes ergonomic entry
points:

```rust
pub struct Client {
    config: Arc<Config>,
    http: reqwest::Client,
    base_configuration: openai_client_base::apis::configuration::Configuration,
}
```

Key capabilities:
- `Client::from_env()` builds configuration from environment variables
- `Client::chat()` and `Client::responses()` return request builders
- `Client::send_chat` / `Client::send_responses` execute the generated payloads
- Placeholder accessors (`assistants()`, `audio()`, …) mark future expansion

### Configuration (`config.rs`)

`Config::builder()` provides validation for API keys, custom base URLs, timeouts,
and default model selection. `Config::from_env()` pulls from `OPENAI_API_KEY`,
`OPENAI_ORG_ID`, etc.

### Error Handling (`errors.rs`)

A single `Error` enum wraps:
- HTTP issues (`Error::Http`)
- API error payloads (`Error::Api`)
- Builder validation problems (`Error::InvalidRequest`)
- Streaming-specific helpers via `errors::streaming`

### Builders (`builders/`)

Each endpoint family implements a dedicated builder module. Implemented modules
today include:

- `responses.rs`, `chat.rs`, `assistants.rs`, `audio.rs`, `vector_stores.rs`,
  `files.rs`, `fine_tuning.rs`, `moderations.rs`, `batch.rs`
- Placeholders exist for `embeddings.rs`, `threads.rs`, `uploads.rs`

Common traits defined in `builders/mod.rs`:

```rust
pub trait Builder<T> {
    fn build(self) -> crate::Result<T>;
}

pub trait Sendable<R> {
    async fn send(self) -> crate::Result<R>;
}
```

(Implementations for `Sendable` are planned; currently builders are executed
through `Client` helper methods.)

### Responses (`responses/`)

Wrap generated response payloads with ergonomic helpers like
`ChatCompletionResponseWrapper::primary_text()` and tool parsing utilities.

### Test Utilities (`test_utils.rs`)

Provides mock server scaffolding and builder factories to support unit and
integration tests without hitting the real API.

## Flow of a Typical Request

1. Call `Client::responses()` (or `chat()`) to construct a builder with defaults
2. Chain builder methods (`.system(..).user(..).temperature(0.5)`)
3. Pass the builder into `Client::send_responses` which
   - Validates the payload via `Builder::build`
   - Delegates to `openai-client-base` to perform the HTTP call
   - Wraps the response inside ergonomic helpers

## Testing Strategy

- **Unit tests** validate builder serialization and helper behaviour
- **Integration tests** (with `mockito`) verify round-trips against fake servers
- **Examples** double as documentation and compile-time regression tests
- **Smoke tests** (opt-in) hit the real API when appropriate environment
  variables are set