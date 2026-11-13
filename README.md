# openai-ergonomic

[![Crates.io](https://img.shields.io/crates/v/openai-ergonomic.svg)](https://crates.io/crates/openai-ergonomic)
[![Documentation](https://docs.rs/openai-ergonomic/badge.svg)](https://docs.rs/openai-ergonomic)
[![CI](https://github.com/genai-rs/openai-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/openai-ergonomic/actions)
[![Coverage](https://codecov.io/gh/genai-rs/openai-ergonomic/branch/main/graph/badge.svg)](https://codecov.io/gh/genai-rs/openai-ergonomic)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://github.com/genai-rs/openai-ergonomic/blob/main/Cargo.toml)
[![License](https://img.shields.io/crates/l/openai-ergonomic.svg)](https://github.com/genai-rs/openai-ergonomic#license)

Ergonomic Rust wrapper for the `OpenAI` API, providing type-safe builder patterns and async/await support.

## Features

- **Type-safe** - full type safety with fluent builder patterns
- **Async/await** - built on `tokio`, with a [tower](https://docs.rs/tower)-based HTTP client (reqwest underneath) whose layers you can customise
- **Streaming** - first-class support for streaming responses
- **Bring your own types** - send and decode request/response types of your own, for fewer fields or `OpenAI`-compatible providers
- **Comprehensive** - covers all `OpenAI` API endpoints
- **Providers** - `OpenAI`, Azure `OpenAI`, `OpenRouter`, Groq, Ollama or any `OpenAI`-compatible API, for the builders, the generated operations and raw requests alike
- **Pluggable authentication** - per-request credentials (Entra ID / Azure AD tokens, gateways) refreshed after a `401`
- **Webhooks** - verify `OpenAI` webhook signatures and decode their events (feature `webhook`)
- **Large files** - parallel, retried uploads through the Uploads API; images, speech and file contents streamed to disk
- **Well-tested** - extensive test coverage with mock support
- **Well-documented** - rich documentation with examples

## Status

**Status:** under construction. The crate is still in active development and not yet ready for production use.

## Quick Start

Add `openai-ergonomic` to your `Cargo.toml`:

```toml
[dependencies]
openai-ergonomic = "0.20"
tokio = { version = "1.0", features = ["full"] }
```

Version 0.19 moves the HTTP stack from `reqwest-middleware` to
[`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime), a [tower](https://docs.rs/tower)-based
client (with `openai-client-base` 0.30; see its
[0.29.0 to 0.30.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.29.0-to-0.30.0.md)). `Config::builder().http_client(..)` and
`Client::http_client()` now take and return a `timvw_rust_runtime::Client` instead of a
`reqwest_middleware::ClientWithMiddleware`: build one with
`timvw_rust_runtime::Client::from_reqwest(reqwest_client)` (with default retries) or
`Client::from_service(ReqwestService::new(reqwest_client))` and add tower layers with `.layer(..)`
(see [Retries, Timeouts and Custom HTTP Clients](#retries-timeouts-and-custom-http-clients)).
`Error::Http` wraps a `timvw_rust_runtime::Error` (with `is_timeout()`, `is_connect()`, ...), and
`Error::HttpMiddleware` is gone. `AzureAuthMiddleware` is now a tower layer, and
`ChatCompletionStream::new` takes a `timvw_rust_runtime::Response`. The default client now honours
`Config::max_retries` (default 3): set it to `0` to turn retries off.

Version 0.18 uses `openai-client-base` 0.29 and requires Rust 1.88 or newer.
Re-exported client types such as `Detail`, `UploadPurpose` and `CreateImageRequest`
now come from `openai-client-base` 0.29, so code that names them directly from
`openai-client-base` must move to 0.29 as well. The re-exported image `OutputFormat`
is now `openai-client-base`'s `ImagesApiOutputFormat`, which adds `Gif`, `Svg` and
`Unknown`, so an exhaustive `match` on it must handle those. `ImageEditBuilder::output_format`
still takes a string; a value that is not a known format is rejected with
`Error::InvalidRequest` before the request is sent. See the
[0.28.0 to 0.29.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.28.0-to-0.29.0.md)
for the other generated changes. 0.28 changed two generated items:
the `bucket_width` parameter of `organization_decisionsusage` is now
`Option<models::DecisionsUsageBucketWidth>` instead of `Option<&str>`, and the values
of `ChatkitWorkflow.state_variables` are now `Option<..>`, so a `null` value decodes.
See the
[0.27.0 to 0.28.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.27.0-to-0.28.0.md).
Code that calls the generated
`openai-client-base` functions itself passes request bodies by reference (`&request`)
from 0.27 on, and every generated model gains a `validate()` method
(`openai_client_base::models::validation::Validate`) that checks the spec's limits;
nothing calls it for you. Error responses changed too: `apis::ResponseContent` gained
a `headers` field, so code that builds one by hand adds
`headers: reqwest::header::HeaderMap::new()`, and the variant in `entity` now follows
the response status (exact code, then range, then the default response) instead of
the first variant whose type accepts the body. See the
[0.26.0 to 0.27.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.26.0-to-0.27.0.md)
for the full list. Since 0.26, optional nullable fields
such as `temperature`, `top_p`, `user` and `metadata` are `Option<Option<T>>`, and
`CreateMessageRequest.content` and `PredictionContent.content` are typed unions
instead of `serde_json::Value`. The builders still take plain values.
Generated request and response types follow their matching client release, so
check the [changelog](CHANGELOG.md) when you upgrade across a minor version. Image
generation and editing require an explicit `.model(...)`; choose the model before
building or sending those requests. See the [0.7 migration guide](docs/migration-0.7.md)
for the generated request field changes introduced in 0.7.

### Upgrading to 0.20

Version 0.20 moves to [`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime) 0.3 and
`openai-client-base` 0.31.

- **Runtime 0.3.** `Config::builder().http_client(..)`, `Client::http_client()`, `Error::Http`,
  `ChatCompletionStream::new` and `ChatCompletionStream::from_events` use
  `timvw_rust_runtime` 0.3 types (`Client`, `Error`, `Response`, `sse::Events`): depend on
  `timvw-rust-runtime = "0.3"` if you name them.
- **Native async traits.** `FunctionTool` and `Interceptor` no longer use `async-trait` (see
  [below](#upgrading-native-async-traits)).
- **Secret credentials.** The credentials of the generated `Configuration`
  (`bearer_access_token`, `api_key`, ...) are `SecretString`s: code that drives
  `openai-client-base` itself sets them with `Some("sk-...".into())` and reads them with
  `ExposeSecret::expose_secret`. Their `Debug` output is redacted.
- **Quota errors are not retried.** A `429` whose error code is `insufficient_quota` is
  returned at once as `Error::Api` instead of being retried: waiting does not refill a quota.
- **Providers.** `Config::builder().provider(Provider::...)` sets the base URL, credentials,
  headers, query parameters and paths in one go (see [Providers](#providers)). The existing
  settings keep working: `api_key`/`api_base` describe an `OpenAI` provider, and
  `azure_deployment` (or an `*.openai.azure.com` base) an Azure one.
- **Credentials only go to the provider.** The `Authorization` (or Azure `api-key`) header and
  the organization and project headers are added by the provider's layer, and only to requests
  under the provider's base URL. The provider's layer now also wraps a client set with
  `http_client(..)`, so a custom client no longer needs `AzureAuthMiddleware`, which is
  deprecated in favour of `Provider::azure` (adding it anyway does no harm).
- **Streaming errors.** Chat and Responses streams are sent as raw requests: an error status
  before the first event is an `Error::Api` with the `type` and `code` of an `OpenAI` error body.
- **New:** `AuthProvider`/`BearerTokenAuth` ([Authentication](#authentication)), webhook
  verification ([Webhooks](#webhooks)), `Client::upload_large_file`, `ImagesClient::save`,
  `AudioClient::speech_to_file`, `FilesClient::upload_file` and `FilesClient::download_to`
  ([Large Files and Media](#large-files-and-media)).

### Upgrading: native async traits

`FunctionTool` and `Interceptor` are now native async traits; the crate no longer
depends on `async-trait`. Drop `#[async_trait]` from your `impl FunctionTool` /
`impl Interceptor` blocks and keep the methods as plain `async fn` (their futures
must be `Send`). `with_interceptor`, `add_interceptor` and `InterceptorChain::add`
take the interceptor by value; `Box<I>` and `Arc<I>` still work because they
implement `Interceptor` too. `dyn Interceptor` is no longer a valid type: collect
heterogeneous interceptors in an `InterceptorChain` instead.

```rust,ignore
// Before
#[async_trait::async_trait]
impl Interceptor for Logger {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> { Ok(()) }
}
let client = Client::from_env()?.with_interceptor(Box::new(Logger)).build();

// After
impl Interceptor for Logger {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> { Ok(()) }
}
let client = Client::from_env()?.with_interceptor(Logger).build();
```

### Upgrading to 0.7

The generated request structs now use nested options for fields that distinguish
an omitted JSON property from an explicit `null`. Set values with two `Some`s,
use `Some(None)` for `null`, and leave fields at `None` to omit them. Read values
with `.flatten()`. Ergonomic builders generally omit unset optional fields and
preserve explicit values; `RunBuilder` defaults to sending `stream: false`.

```rust
use openai_client_base::models::CreateCompletionRequest;

let mut request = CreateCompletionRequest::new("gpt-4".into(), "Hello".into());
request.stream = Some(Some(false));
request.stop = Some(None);
request.n = None;

let max_tokens: Option<i64> = request.max_tokens.flatten();
let json = serde_json::to_value(request).unwrap();
assert_eq!(json["stream"], false);
assert!(json["stop"].is_null());
assert!(json.get("n").is_none());
```

### Basic Usage

Set `OPENAI_API_KEY` before running this snippet. Documentation tests compile it
without reading credentials or making API calls.

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let request = client.chat().user("Hello, world!");
    let response = client.send_chat(request).await?;

    if let Some(content) = response.content() {
        println!("{content}");
    }
    Ok(())
}
```

### Langfuse Setup

The Langfuse integration (interceptor, exporter and context) lives behind the `otel`
cargo feature, which is enabled by default. The interceptor needs an OpenTelemetry tracer
and a Langfuse exporter:

```toml
openai-ergonomic = "0.19"
tokio = { version = "1", features = ["full"] }
opentelemetry = { version = "0.33.1", features = ["trace"] }
opentelemetry_sdk = { version = "0.33.1", features = ["rt-tokio", "trace", "experimental_trace_batch_span_processor_with_async_runtime"] }
```

The README's `no_run` doctest checks the same Rust code against the current crate
source without credentials or a live service.

At runtime, set `OPENAI_API_KEY`, `LANGFUSE_PUBLIC_KEY`, and
`LANGFUSE_SECRET_KEY`. Optionally set `LANGFUSE_HOST` for a self-hosted instance.
This setup is compiled as a `no_run` doctest, so testing needs no credentials or
live services.

```rust,no_run
# #[cfg(feature = "otel")]
# mod demo {
use openai_ergonomic::{
    langfuse_exporter_from_env, Client, LangfuseConfig, LangfuseInterceptor,
};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exporter = langfuse_exporter_from_env()?;
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
        .build();
    let interceptor = LangfuseInterceptor::new(
        provider.tracer("openai-ergonomic"),
        LangfuseConfig::new(),
    );
    let client = Client::from_env()?
        .with_interceptor(interceptor)
        .build();

    // Keep the result so spans are flushed even when the request fails.
    let result = client.send_chat(client.chat_simple("Hello, world!")).await;
    // The Tokio batch processor needs the runtime while shutdown waits for export.
    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    let response = result?;
    if let Some(content) = response.content() {
        println!("{content}");
    }
    Ok(())
}
# }
```

See the [Langfuse integration guide](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/langfuse-integration.md)
for configuration and observability details.

### Streaming Example

```rust,no_run
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

### Retries, Timeouts and Custom HTTP Clients

Requests go through a [`timvw_rust_runtime::Client`](https://docs.rs/timvw-rust-runtime): a
[tower](https://docs.rs/tower) service stack around reqwest. The default client has a 120 second
timeout and retries `429`, `408` and `5xx` responses and failed connections up to
`Config::max_retries` times (default 3, `0` disables retries), with exponential backoff that
honours `Retry-After`.

To change the stack (a proxy, other timeouts, a retry policy, logging, rate limits, layers of
your own), build a client and pass it with `Config::builder().http_client(..)`. For a custom
client `max_retries` and the default timeout do not apply; the client still gets the provider's
layer (base URL, credentials, headers, paths, including Azure's) and the `AuthProvider`, if any.
Layers apply from the inside out: the last one added sees a request first.

```rust,no_run
use openai_ergonomic::{Client, Config};
use std::time::Duration;
use timvw_rust_runtime::retry::RetryPolicy;
use timvw_rust_runtime::{Client as HttpClient, ReqwestService};
use tower::retry::RetryLayer;
use tower::timeout::TimeoutLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a reqwest client with custom connection settings
    let reqwest_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    // Each attempt times out after 60 s; up to 5 retries with exponential backoff
    let http_client = HttpClient::from_service(ReqwestService::new(reqwest_client))
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(RetryLayer::new(RetryPolicy::new(5).max_delay(Duration::from_secs(30))));

    // Create OpenAI client with custom HTTP client
    let config = Config::builder()
        .api_key("your-api-key")
        .http_client(http_client)
        .build();

    let client = Client::builder(config)?.build();

    // Use the client normally - retries and timeout are handled automatically
    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

With the `tracing-http` feature, `timvw_rust_runtime::trace::TraceLayer` logs every request
and retry with `tracing`. See [`examples/http_middleware_retry.rs`](examples/http_middleware_retry.rs).

### Bring Your Own Types

The builders use the `OpenAI` spec's models. To send and decode types of your own (a struct with
only the fields you need, `serde_json::Value`, or an `OpenAI`-compatible provider whose replies
differ from the spec), use `post_json`, `get_json`, `send_json` and `send_json_stream`. They send
to `{api_base}{path}` through the same HTTP client (retries, layers, the provider's layer,
organization and project headers) with the same authentication, and map an error status to `Error::Api`.
Interceptors (such as Langfuse) are not called for these requests.

```rust,no_run
use futures::StreamExt;
use openai_ergonomic::{Client, Config};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Any OpenAI-compatible endpoint
    let config = Config::builder()
        .api_key("provider-key")
        .api_base("https://openai-compatible.example.com/v1")
        .build();
    let client = Client::builder(config)?.build();

    let request = json!({
        "model": "some-model",
        "messages": [{"role": "user", "content": "Hello!"}]
    });
    let completion: Completion = client.post_json("/chat/completions", &request).await?;
    println!("{:?}", completion.choices[0].message.content);

    // Streaming: each server-sent event decoded into a type of your own
    let mut request = request;
    request["stream"] = json!(true);
    let mut events = client
        .send_json_stream::<_, serde_json::Value>("/chat/completions", &request)
        .await?;
    while let Some(event) = events.next().await {
        print!("{}", event?["choices"][0]["delta"]["content"].as_str().unwrap_or_default());
    }
    Ok(())
}
```

### Providers

A `Provider` says where requests go and how they are authenticated. Set it with
`Config::builder().provider(..)`: the builders, the generated `openai-client-base` operations and
the raw `send_json` calls then all go to the provider, through a
[`ProviderLayer`](https://docs.rs/timvw-rust-runtime/latest/timvw_rust_runtime/provider/struct.ProviderLayer.html)
that adds its credentials, headers and query parameters and rewrites its paths. Requests to other
hosts (an image download, say) never get the provider's credentials.

| Preset | Base URL | Authentication | Extras |
|---|---|---|---|
| `Provider::openai(key)` (default) | `https://api.openai.com/v1` | `Authorization: Bearer` | |
| `Provider::azure(endpoint, key, deployment)` | `{endpoint}/openai` | `api-key` | `/deployments/{deployment}` paths, `api-version` (`.api_version(..)`) |
| `Provider::azure_entra(endpoint, deployment)` | same | an `AuthProvider` | |
| `Provider::openrouter(key)` | `https://openrouter.ai/api/v1` | `Authorization: Bearer` | `.http_referer(..)`, `.title(..)` |
| `Provider::groq(key)` | `https://api.groq.com/openai/v1` | `Authorization: Bearer` | |
| `Provider::ollama()` | `http://localhost:11434/v1` | none | |
| `Provider::custom(base_url)` | `base_url` | `.bearer(..)` or `.api_key_header(..)` | |

Every preset takes `.base_url(..)` (a proxy, another host), `.header(..)`,
`.sensitive_header(..)`, `.query(..)` and `.map_path(..)`.

```rust,no_run
use openai_ergonomic::{Client, Config, Provider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::builder()
        .provider(
            Provider::openrouter(std::env::var("OPENROUTER_API_KEY")?)
                .http_referer("https://my-app.example")
                .title("My App"),
        )
        .default_model("openai/gpt-4o-mini")
        .build();
    let client = Client::builder(config)?.build();

    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());

    // A local Ollama server, without a key
    let local = Client::builder(
        Config::builder()
            .provider(Provider::ollama())
            .default_model("llama3.2")
            .build(),
    )?
    .build();
    let response = local.send_chat(local.chat_simple("Hi")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

`Provider::layer()` gives the layer on its own, for an HTTP client you build yourself.

### Authentication

For credentials that change (Microsoft Entra ID / Azure AD tokens, short-lived gateway tokens,
request signing), implement `AuthProvider` and set it with `Config::builder().auth_provider(..)`.
`apply` gets every request to the provider and returns it with its credentials;
`refresh_after_unauthorized` is called when the provider answers `401` with the headers of the
rejected request, and when it returns `true` the request is authenticated again and sent once
more. It runs as a tower layer around the HTTP client, so the generated operations and the raw
requests are covered too. Both methods are plain `async fn`s (no `async-trait`).

`BearerTokenAuth` covers the common case: it fetches a token with a function of yours, caches it
until shortly before it expires, and refreshes it once when several concurrent requests are
rejected with the same token.

```rust,no_run
use openai_ergonomic::auth::{AuthToken, BearerTokenAuth};
use openai_ergonomic::{Client, Config, Provider};
use std::time::Duration;

async fn fetch_entra_token() -> openai_ergonomic::Result<String> {
    // e.g. with the azure_identity crate
    Ok("eyJ0eXAi...".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = BearerTokenAuth::new(|| async {
        let token = fetch_entra_token().await?;
        Ok(AuthToken::new(token).expires_in(Duration::from_secs(3600)))
    });
    let config = Config::builder()
        .provider(Provider::azure_entra("https://my-resource.openai.azure.com", "gpt-4o"))
        .auth_provider(auth)
        .build();
    let client = Client::builder(config)?.build();
    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

A request is sent again after a `401` only when its body can be sent again (JSON bodies and file
uploads can, a one-shot stream cannot); a second `401` is returned as is.

### Webhooks

With the `webhook` feature, `webhooks::WebhookVerifier` verifies `OpenAI` webhook deliveries
([Standard Webhooks](https://www.standardwebhooks.com/) signatures: the `webhook-id`,
`webhook-timestamp` and `webhook-signature` headers, HMAC-SHA256 with your `whsec_...` secret,
any of several signatures, timestamps within 5 minutes by default) and decodes them into
`WebhookEvent`, whose variants hold the `openai-client-base` `Webhook*` models. Pass the body
exactly as received.

```toml
openai-ergonomic = { version = "0.20", features = ["webhook"] }
```

```rust,ignore
use openai_ergonomic::webhooks::{WebhookEvent, WebhookVerifier};

let verifier = WebhookVerifier::new(&std::env::var("OPENAI_WEBHOOK_SECRET")?)?;
// In your HTTP handler, with the request's headers and raw body:
match verifier.unwrap(&headers, &body)? {
    WebhookEvent::ResponseCompleted(event) => println!("response {} completed", event.data.id),
    WebhookEvent::BatchCompleted(event) => println!("batch {} completed", event.data.id),
    other => println!("ignoring {}", other.event_type()),
}
```

`WebhookVerifier::sign` produces a valid `webhook-signature`, to test your handler.

### Large Files and Media

`Client::upload_large_file` sends a file of up to 8 GB through the Uploads API: it creates an
upload, sends parts of at most 64 MB several at a time (4 by default), each read straight from
its offset in the file and retried on its own, and completes the upload. It takes anything that
converts into a `timvw_rust_runtime::Upload`: a path, bytes, or a reader or stream with a known
length.

```rust,no_run
use openai_ergonomic::{Client, UploadPurpose};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let upload = client
        .upload_large_file(PathBuf::from("training.jsonl"), UploadPurpose::FineTune)
        .mime_type("text/jsonl")
        .concurrency(8)
        .await?;
    println!("uploaded: {:?}", upload.file);

    // Generated media, straight to disk
    let images = client
        .images()
        .create(client.images().generate("A lighthouse at dusk").model("gpt-image-1"))
        .await?;
    let paths = client.images().save_all(&images, ".", "lighthouse").await?;
    println!("saved {paths:?}");

    let speech = client.audio().speech("gpt-4o-mini-tts", "Hello there", "alloy");
    client.audio().speech_to_file(speech, "hello.mp3").await?;
    Ok(())
}
```

`ImagesClient::save` decodes `b64_json` or downloads the image's `url`;
`AudioClient::speech_to_file` and `FilesClient::download_to` stream the body to a temporary file
that replaces the target once complete. `FilesClient::upload_file` uploads a path, bytes or a
stream without copying it to a temporary file first.

### Azure `OpenAI` Support

The crate seamlessly supports Azure `OpenAI` deployments. Azure-specific configuration can be provided through environment variables or programmatically.

#### Using Environment Variables

```bash
export AZURE_OPENAI_ENDPOINT="https://my-resource.openai.azure.com"
export AZURE_OPENAI_API_KEY="your-azure-api-key"
export AZURE_OPENAI_DEPLOYMENT="gpt-4"
export AZURE_OPENAI_API_VERSION="2024-02-01"  # Optional, defaults to 2024-02-01
```

```rust,no_run
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build client from Azure environment variables
    let client = Client::from_env()?.build();

    // Use exactly the same API as standard OpenAI
    let response = client.send_chat(client.chat_simple("Hello from Azure!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

#### Manual Configuration

```rust,no_run
use openai_ergonomic::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::builder()
        .api_key("your-azure-api-key")
        .api_base("https://my-resource.openai.azure.com")
        .azure_deployment("gpt-4")
        .azure_api_version("2024-02-01")
        .build();

    let client = Client::builder(config)?.build();

    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

Or, equivalently, with a [provider](#providers):
`Config::builder().provider(Provider::azure("https://my-resource.openai.azure.com", "your-azure-api-key", "gpt-4").api_version("2024-02-01"))`.
For Microsoft Entra ID (Azure AD) tokens, use `Provider::azure_entra` with an
[`AuthProvider`](#authentication).

**Note:** The library automatically handles the differences between Azure `OpenAI` and standard `OpenAI` (authentication, URL paths, API versioning). You use the same API regardless of the provider.

## Documentation

- [Getting Started Guide](docs/getting-started.md)
- [Architecture Overview](docs/architecture.md)
- **[API Coverage Status](docs/api-coverage.md)** - See what APIs are available
- **[Examples Index](docs/examples-index.md)** - Browse all examples by category
- [Responses-First Workflows](docs/responses_workflows.md)
- [Tool Orchestration](docs/tool_orchestration.md)
- [Tool Framework](docs/tool_framework.md)
- [Vector Store Operations](docs/vector_store_operations.md)
- [Langfuse Integration](docs/langfuse-integration.md)
- [API Documentation](https://docs.rs/openai-ergonomic)

## Examples

The `examples/` directory contains comprehensive examples for all major `OpenAI` API features:

### Core Examples

- [**quickstart.rs**](examples/quickstart.rs) - Quick introduction to the library with basic usage patterns
- [**chat_streaming.rs**](examples/chat_streaming.rs) - Real-time chat streaming with Server-Sent Events (SSE)
- [**tool_framework.rs**](examples/tool_framework.rs) - Unified tool framework with typed inputs and registry execution
- [**tool_framework_typed.rs**](examples/tool_framework_typed.rs) - Strongly typed inputs/outputs and processing tool calls
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
