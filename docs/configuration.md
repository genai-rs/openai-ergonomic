# Configuration

`Config::builder()` (or `Config::from_env()`, see [getting started](getting-started.md#a-client))
holds where requests go, how they are authenticated and how they are sent;
`Client::builder(config)?.build()` makes a client of it.

## Providers

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

### Azure `OpenAI`

The client handles what differs at Azure (the `api-key` header, the deployment paths, the
`api-version` query parameter); you use the same builders. From the environment:

```bash
export AZURE_OPENAI_ENDPOINT="https://my-resource.openai.azure.com"
export AZURE_OPENAI_API_KEY="your-azure-api-key"
export AZURE_OPENAI_DEPLOYMENT="gpt-4o"
export AZURE_OPENAI_API_VERSION="2024-10-21"  # optional
```

```rust,no_run
use openai_ergonomic::{Client, Config, Provider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // From the AZURE_OPENAI_* variables
    let client = Client::from_env()?.build();
    let response = client.send_chat(client.chat_simple("Hello from Azure!")).await?;
    println!("{}", response.content().unwrap_or_default());

    // Or explicitly
    let config = Config::builder()
        .provider(
            Provider::azure("https://my-resource.openai.azure.com", "your-azure-api-key", "gpt-4o")
                .api_version("2024-10-21"),
        )
        .build();
    let client = Client::builder(config)?.build();
    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

The older settings still work: `api_key(..)`, `api_base("https://my-resource.openai.azure.com")`,
`azure_deployment(..)` and `azure_api_version(..)` describe the same Azure provider. For Microsoft
Entra ID (Azure AD) tokens, use `Provider::azure_entra` with an [`AuthProvider`](#authentication).

## Authentication

For credentials that change (Microsoft Entra ID / Azure AD tokens, short-lived gateway tokens,
request signing), implement `AuthProvider` and set it with `Config::builder().auth_provider(..)`.
`apply` gets every request to the provider and returns it with its credentials;
`refresh_after_unauthorized` is called when the provider answers `401` with the headers of the
rejected request, and when it returns `true` the request is authenticated again and sent once
more. It runs as a tower layer around the HTTP client, so the generated operations and the raw
requests are covered too. Both methods are plain `async fn`s.

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

## Retries, timeouts and custom HTTP clients

Requests go through a [`timvw_rust_runtime::Client`](https://docs.rs/timvw-rust-runtime): a
[tower](https://docs.rs/tower) service stack around reqwest. The default client has a 120 second
timeout (natively) and retries `429`, `408` and `5xx` responses and failed connections up to
`Config::max_retries` times (default 3, `0` disables retries), with exponential backoff that
honours `Retry-After`. A `429` whose error code is `insufficient_quota` is not retried: waiting
does not refill a quota.

To change the stack (a proxy, other timeouts, a retry policy, logging, rate limits, layers of
your own), build a client and pass it with `Config::builder().http_client(..)`. For a custom
client `max_retries` and the default timeout do not apply; the client still gets the provider's
layer (base URL, credentials, headers, paths, including Azure's) and the `AuthProvider`, if any.
Layers apply from the inside out: the last one added sees a request first.

```rust,no_run
use openai_ergonomic::{Client, Config};
use std::time::Duration;
use timvw_rust_runtime::retry::{RetryLayer, RetryPolicy};
use timvw_rust_runtime::timeout::TimeoutLayer;
use timvw_rust_runtime::{Client as HttpClient, ReqwestService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A reqwest client with custom connection settings
    let reqwest_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    // Each attempt times out after 60 s; up to 5 retries with exponential backoff
    let http_client = HttpClient::from_service(ReqwestService::new(reqwest_client))
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(RetryLayer::new(RetryPolicy::new(5).max_delay(Duration::from_secs(30))));

    let config = Config::builder()
        .api_key("your-api-key")
        .http_client(http_client)
        .build();
    let client = Client::builder(config)?.build();

    let response = client.send_chat(client.chat_simple("Hello!")).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

`timvw_rust_runtime::timeout::TimeoutLayer` works on WebAssembly too (tower's does not). With the
`tracing-http` feature, `timvw_rust_runtime::trace::TraceLayer` logs every request and retry with
`tracing`. See [`examples/http_middleware_retry.rs`](../examples/http_middleware_retry.rs).

A timeout of the whole request does not suit a stream that may run for minutes:
`ConfigBuilder::stream_idle_timeout(Duration)` ends a streaming response (chat or Responses API)
with `Error::StreamConnection` when no event arrives for that long instead (not set by default).
`Error::retry_class()` says how a failed request may be retried (`Never`, `ServerDirected { after }`
or `Backoff`), and `client.rate_limits()` returns the `x-ratelimit-*` headers of the latest
response (`RateLimitSnapshot`), whatever the HTTP client:

```rust,no_run
use openai_ergonomic::{Client, Config};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::builder()
        .api_key("your-api-key")
        .stream_idle_timeout(Duration::from_secs(300))
        .build();
    let client = Client::builder(config)?.build();

    match client.send_chat(client.chat_simple("Hello!")).await {
        Ok(response) => println!("{}", response.content().unwrap_or_default()),
        Err(error) => eprintln!("{error}: {:?}", error.retry_class()),
    }
    if let Some(tokens) = client.rate_limits().and_then(|limits| limits.tokens) {
        println!("tokens left: {:?}, reset in {:?}", tokens.remaining, tokens.reset);
    }
    Ok(())
}
```

## Per-call options

The builders send with the client's configuration. When you call the generated
`openai-client-base` operations yourself, `.options(&RequestOptions)` adds headers, query
parameters or another base URL to one call. Since runtime 0.4 these per-call options **win over
the provider**:

- a header or query parameter the options set (an Azure `api-version`, say) is sent instead of
  the provider's, once;
- a per-call `base_url` under the provider's base URL is still mapped and authenticated;
- any other base URL gets nothing from the provider (no credentials, headers or query
  parameters) and nothing from the `AuthProvider`.

```rust,no_run
use openai_client_base::apis::{configuration::Configuration, models_api};
use openai_ergonomic::{Client, Config, Provider};
use timvw_rust_runtime::RequestOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Provider::azure("https://my-resource.openai.azure.com", "key", "gpt-4o");
    let client = Client::builder(Config::builder().provider(provider.clone()).build())?.build();

    // The generated operations, through the client's HTTP stack (and so its provider layer)
    let mut configuration = Configuration::new();
    configuration.client = client.http_client().clone();
    configuration.base_path = provider.api_base().to_string();

    // This call uses a preview api-version instead of the provider's
    let options = RequestOptions::new().query(&[("api-version", "2025-04-01-preview")])?;
    let models = models_api::list_models()
        .configuration(&configuration)
        .options(&options)
        .call()
        .await?;
    println!("{} models", models.data.len());
    Ok(())
}
```
