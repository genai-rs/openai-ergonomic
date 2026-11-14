# Langfuse integration

With the `otel` feature (on by default), `LangfuseInterceptor` records every request the
builders send as an OpenTelemetry span with the GenAI semantic conventions, and
`langfuse_exporter_from_env` exports the spans to [Langfuse](https://langfuse.com) over
OTLP/HTTP. The interceptor only creates spans: your application owns the tracer provider and
shuts it down. [Interceptors](interceptors.md#langfuse-and-opentelemetry) has the basic setup;
this page has the rest.

## Installing

```bash
cargo add openai-ergonomic
cargo add tokio --features full
cargo add opentelemetry --features trace
cargo add opentelemetry_sdk --features rt-tokio,trace,experimental_trace_batch_span_processor_with_async_runtime
```

Use the OpenTelemetry release family that `openai-ergonomic` depends on (see its `Cargo.toml`, or
`cargo tree -i opentelemetry`): two OpenTelemetry versions in one build have incompatible tracer
types.

## Configuration

| Variable | Read by | Meaning |
| --- | --- | --- |
| `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` | `langfuse_exporter_from_env` | the project's keys (required) |
| `LANGFUSE_HOST` | `langfuse_exporter_from_env` | another Langfuse (default `https://cloud.langfuse.com`) |
| `LANGFUSE_DEBUG` | `LangfuseConfig::new()` | `true` logs each span with `tracing` at debug level |

The exporter sends to `{LANGFUSE_HOST}/api/public/otel/v1/traces` with Basic authentication.
`LangfuseConfig` has a single setting, `with_debug(bool)`; batching, export intervals and
timeouts are the tracer provider's (the `BatchSpanProcessor` builder), not the interceptor's.

To build the exporter in code instead (another host, extra headers such as
`x-langfuse-ingestion-version`, a custom HTTP client), use any `opentelemetry_otlp::SpanExporter`
of the same OpenTelemetry family, for example
[opentelemetry-langfuse](https://github.com/genai-rs/opentelemetry-langfuse)'s
`ExporterBuilder`. This snippet is `ignore`d because opentelemetry-langfuse is not a dependency
of this crate:

```rust,ignore
use opentelemetry_langfuse::ExporterBuilder;

let exporter = ExporterBuilder::new()
    .with_host("https://cloud.langfuse.com")
    .with_basic_auth("pk-lf-...", "sk-lf-...")
    .build()?;
// ExporterBuilder::from_env()? reads the same LANGFUSE_* variables.
```

## Sessions, users, tags and metadata

The interceptor adds the attributes of its `LangfuseContext` to every span it starts:
`set_session_id`, `set_user_id`, `add_tag` / `add_tags` and `set_metadata` on the interceptor
(or on `interceptor.context()`, which also has `set_trace_name` and `set_attribute`), and
`clear_context` to remove them. The context is shared by every request of the client, so set it
per client (or per application), not per concurrent request.

```rust,no_run
# #[cfg(feature = "otel")]
# mod demo {
use openai_ergonomic::{langfuse_exporter_from_env, Client, LangfuseConfig, LangfuseInterceptor};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(langfuse_exporter_from_env()?, Tokio).build())
        .build();

    let interceptor = LangfuseInterceptor::new(
        provider.tracer("openai-ergonomic"),
        LangfuseConfig::new().with_debug(true),
    );
    interceptor.set_session_id("session-123");
    interceptor.set_user_id("user-456");
    interceptor.add_tag("docs");
    interceptor.set_metadata(json!({"release": "2026-11"}));
    interceptor.context().set_trace_name("greeting");

    let client = Client::from_env()?.with_interceptor(interceptor).build();
    let result = client.send_chat(client.chat_simple("Hello!")).await;

    // The Tokio batch processor needs the runtime while shutdown waits for export.
    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    println!("{}", result?.content().unwrap_or_default());
    Ok(())
}
# }
```

## What a span holds

One span per request, named after the operation (`chat`, `embeddings`, ...), of kind `Client`:

- `gen_ai.system` (`openai`), `gen_ai.operation.name`, `gen_ai.request.model`,
  `gen_ai.request.temperature` and `gen_ai.request.max_tokens` when set;
- the prompt (`gen_ai.prompt.{i}.role` / `.content`) and the completion
  (`gen_ai.completion.{i}.role` / `.content`);
- `gen_ai.usage.input_tokens` / `output_tokens`, `gen_ai.response.id`, `duration_ms`;
- for streams, `stream.total_chunks`, `stream.duration_ms` and `stream.outcome` (`completed`,
  `failed` or `cancelled`);
- on failure, `error.type` and `error.message` and an error status;
- the `langfuse.*` attributes of the context (`langfuse.session.id`, `langfuse.user.id`,
  `langfuse.trace.tags`, `langfuse.trace.metadata`, `langfuse.trace.name`).

The spans start from the current OpenTelemetry context, so they nest under an active span of
your own.

## Streaming

Streamed requests (`send_chat_stream_with_completion`, `send_responses_stream_with_completion`)
are traced too: `before_request` starts the span, `on_stream_chunk` sees each accepted chunk in
order and `on_stream_terminal` ends the span once, also when the stream fails or is dropped.
The callbacks run in the background: wait for the `StreamCompletion` before shutting the
provider down, or the last spans are lost (see the
[completion contract](streaming-completion-contract.md)).

```rust,no_run
# #[cfg(feature = "otel")]
# mod demo {
use futures::StreamExt;
use openai_ergonomic::{langfuse_exporter_from_env, Client, LangfuseConfig, LangfuseInterceptor};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider},
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(langfuse_exporter_from_env()?, Tokio).build())
        .build();
    let interceptor = LangfuseInterceptor::new(provider.tracer("openai-ergonomic"), LangfuseConfig::new());
    let client = Client::from_env()?.with_interceptor(interceptor).build();

    let (mut stream, completion) = client
        .send_chat_stream_with_completion(client.chat().user("Tell me a story"))
        .await?;
    let mut stream_error = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => print!("{}", chunk.content().unwrap_or_default()),
            Err(error) => {
                stream_error = Some(error);
                break;
            }
        }
    }
    // Drop the stream before waiting: after an early stop the outcome is then a cancellation.
    drop(stream);

    // A timeout means the callbacks have not drained: the last spans may be missing.
    match tokio::time::timeout(Duration::from_secs(10), completion.wait()).await {
        Ok(Ok(report)) => {
            println!("\n{:?}; {} callback failures", report.outcome, report.callback_failure_count);
            for failure in &report.callback_failures {
                eprintln!("{:?}: {}", failure.hook, failure.diagnostic);
            }
        }
        Ok(Err(unavailable)) => eprintln!("no completion report: {unavailable:?}"),
        Err(_) => eprintln!("interceptor callbacks still running after 10 s"),
    }

    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    match stream_error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}
# }
```

## With langfuse-ergonomic

[langfuse-ergonomic](https://github.com/genai-rs/langfuse-ergonomic) traces your own code with
`#[observe]`. With its `otel-context` feature (and Langfuse v4), an observation is the current
OpenTelemetry context while it runs, so the interceptor's spans for the requests made inside an
observed function become its children in the same trace. See its
[Observe guide](https://github.com/genai-rs/langfuse-ergonomic/blob/main/docs/observe.md#opentelemetry-and-openai-ergonomic).
This snippet is `ignore`d because langfuse-ergonomic is not a dependency of this crate:

```toml
[dependencies]
langfuse-ergonomic = { version = "x.y", features = ["otel-context"] }  # see crates.io
openai-ergonomic = "x.y"                                                # `otel` is a default feature
```

```rust,ignore
use langfuse_ergonomic::{observe, ClientBuilder};
use openai_ergonomic::{
    langfuse_exporter_from_env, Client, LangfuseConfig, LangfuseInterceptor, LangfuseState,
};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
use opentelemetry_sdk::trace::{SdkTracerProvider, Span};

type OpenAi = Client<LangfuseState<Span>>;

#[observe(as_type = "agent")]
async fn agent(openai: &OpenAi, question: &str) -> Result<String, Box<dyn std::error::Error>> {
    // The interceptor's span for this request is a child of the `agent` observation.
    let response = openai.send_chat(openai.chat_simple(question)).await?;
    Ok(response.content().unwrap_or_default().to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // OpenAI requests: OpenTelemetry spans, exported by openai-ergonomic.
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(langfuse_exporter_from_env()?, Tokio).build())
        .build();
    let interceptor = LangfuseInterceptor::new(provider.tracer("openai-ergonomic"), LangfuseConfig::new());
    let openai = Client::from_env()?.with_interceptor(interceptor).build();

    // Your own code: #[observe] observations, sent by langfuse-ergonomic.
    let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
    observe::install_batcher(batcher.clone());

    agent(&openai, "What is the capital of Belgium?").await?;

    batcher.shutdown().await?;
    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    Ok(())
}
```

langfuse-ergonomic reads `LANGFUSE_BASE_URL` for its host, the exporter `LANGFUSE_HOST`: set both
for a self-hosted Langfuse.

## Examples

```bash
export OPENAI_API_KEY="sk-..."
export LANGFUSE_PUBLIC_KEY="pk-lf-..."
export LANGFUSE_SECRET_KEY="sk-lf-..."
cargo run --example langfuse_simple     # one request
cargo run --example langfuse            # several scenarios, sessions and tags
cargo run --example langfuse_streaming  # streams, waiting for the callbacks
```

## Troubleshooting

- **No traces**: check the keys and `LANGFUSE_HOST`, and that the provider is shut down (or
  flushed) before the process exits.
- **Missing spans of streams**: wait for `StreamCompletion::wait()` before the shutdown.
- **What is sent**: `LANGFUSE_DEBUG=true` (or `LangfuseConfig::with_debug(true)`) with
  `RUST_LOG=openai_ergonomic=debug` logs each span.
