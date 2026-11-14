# Getting started

## Installing

```bash
cargo add openai-ergonomic
cargo add tokio --features full
```

The default features are `rustls-tls` (TLS with rustls) and `otel` (the Langfuse /
OpenTelemetry integration, see [interceptors](interceptors.md)). The others:

| Feature | What it adds |
| --- | --- |
| `native-tls` | TLS with the platform's library instead of rustls |
| `webhook` | [webhook](webhooks.md) signature verification and events |
| `tracing-http` | `timvw_rust_runtime::trace::TraceLayer`: logs every request and retry with `tracing` |
| `test-utils` | a mockito-based mock server for your tests (native only) |

For WebAssembly, turn the default features off: see [WebAssembly](webassembly.md). The minimum
supported Rust version is `rust-version` in `Cargo.toml`:
[![MSRV](https://img.shields.io/crates/msrv/openai-ergonomic)](https://crates.io/crates/openai-ergonomic).

## A client

`Client::from_env()` reads the configuration from the environment:

| Variable | Meaning |
| --- | --- |
| `OPENAI_API_KEY` | the API key (required for `OpenAI`) |
| `OPENAI_API_BASE` | another base URL |
| `OPENAI_ORGANIZATION`, `OPENAI_PROJECT` | the organization and project headers |
| `OPENAI_MAX_RETRIES` | retries of the default HTTP client (default 3) |
| `OPENAI_DEFAULT_MODEL` | the model of `client.chat()` and `client.responses()` (default `gpt-4`) |
| `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_DEPLOYMENT`, `AZURE_OPENAI_API_VERSION` | Azure `OpenAI` instead (see [configuration](configuration.md#azure-openai)) |

Or build the configuration yourself:

```rust,no_run
use openai_ergonomic::{Client, Config};

# fn main() -> openai_ergonomic::Result<()> {
let config = Config::builder()
    .api_key("sk-...")
    .organization("org-123")
    .default_model("gpt-4o-mini")
    .max_retries(5)
    .build();
let client = Client::builder(config)?.build();
# let _ = client;
# Ok(())
# }
```

[Configuration](configuration.md) covers providers (Azure `OpenAI`, `OpenRouter`, Groq, Ollama, any
`OpenAI`-compatible API), refreshable credentials, retries, timeouts and custom HTTP clients.

## Chat completions

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .chat()
        .system("You are an upbeat assistant.")
        .user("Suggest a codename for a Rust project building on OpenAI.")
        .temperature(0.7);
    let response = client.send_chat(builder).await?;
    println!("{}", response.content().unwrap_or_default());

    // One user message with the default model
    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

## The Responses API

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .responses()
        .system("You are a concise assistant.")
        .user("Summarise the benefits of type-safe builders in Rust.")
        .temperature(0.4);
    let response = client.send_responses(builder).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

`ResponsesBuilder` builds the same chat-completions request as `ChatCompletionBuilder` and sends
it to `/chat/completions`; for the `/responses` endpoint itself, send a request of your own (see
[bring your own types](chat-and-responses.md#bring-your-own-types)). More in
[chat and responses](chat-and-responses.md) and [Responses workflows](responses_workflows.md);
streaming in [streaming](streaming.md).

## Errors

Every fallible call returns `openai_ergonomic::Result<T>`, whose `Error` has a variant per kind
of failure: `Error::Api` (an error status, with the `OpenAI` error's type and code; a `429` is
one, see `is_rate_limit()`), `Error::Http` (sending failed: `is_timeout()`, `is_connect()`,
`is_io()`), `Error::InvalidRequest` (a builder rejected its input before sending),
`Error::Config`, and more.

```rust,no_run
use openai_ergonomic::{Client, Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    match client.send_chat(client.chat_simple("Hello")).await {
        Ok(response) => println!("{}", response.content().unwrap_or_default()),
        Err(error) if error.is_rate_limit() => eprintln!("rate limited ({error}), retry later"),
        Err(Error::Api { status, message, .. }) => eprintln!("API error {status}: {message}"),
        Err(error) => eprintln!("request failed: {error}"),
    }
    Ok(())
}
```

## Examples

The [`examples/`](../examples) directory has a runnable example per area (chat, streaming,
tools, the Responses API, images, audio, files, assistants, batches, Azure, Langfuse, ...); the
[examples index](examples-index.md) lists them by category.

```bash
export OPENAI_API_KEY="sk-..."
cargo run --example quickstart
cargo run --example responses_streaming
```
