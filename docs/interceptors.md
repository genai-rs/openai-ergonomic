# Interceptors

An `Interceptor` has hooks around every request the builders send: `before_request`,
`after_response`, `on_stream_chunk`, `on_stream_end` / `on_stream_terminal` and `on_error`.
Implement the ones you need as plain `async fn`s and add the interceptor with
`with_interceptor` (or several with `add_interceptor`). Raw requests (`send_json`, ...) do not go
through the interceptors.

```rust,no_run
use openai_ergonomic::{AfterResponseContext, Client, Interceptor, Result};

struct LogUsage;

impl Interceptor for LogUsage {
    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        println!(
            "{} with {} took {:?}: {:?} in, {:?} out",
            ctx.operation, ctx.model, ctx.duration, ctx.input_tokens, ctx.output_tokens
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.with_interceptor(LogUsage).build();
    client.send_chat(client.chat_simple("Hello")).await?;
    Ok(())
}
```

`Box<I>` and `Arc<I>` are interceptors when `I` is. Interceptors of different types go into one
`InterceptorChain` (there is no `dyn Interceptor`).

## State across hooks

`Interceptor<T>` passes a value of type `T` from `before_request` (where it is `&mut`) to the
later hooks of the same request: a span, a start time, a request ID. The client is then a
`Client<T>`, and `T` must be `Default` and `MaybeSend + MaybeSync`: `Send + Sync` natively,
anything on WebAssembly (where it may hold an `Rc`).

```rust,no_run
use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, Client, Interceptor, Result,
};

#[derive(Default)]
struct RequestId(String);

struct Correlate;

impl Interceptor<RequestId> for Correlate {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_, RequestId>) -> Result<()> {
        ctx.state.0 = format!("{}-{}", ctx.operation, ctx.model);
        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_, RequestId>) -> Result<()> {
        println!("{} done in {:?}", ctx.state.0, ctx.duration);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let client: Client<RequestId> = Client::from_env()?.with_interceptor(Correlate).build();
    client.send_chat(client.chat_simple("Hello")).await?;
    Ok(())
}
```

Streamed requests run the stream hooks in the background; see
[streaming](streaming.md#waiting-for-the-interceptors) for waiting until they are done.

## Langfuse and OpenTelemetry

The Langfuse integration (interceptor, exporter and context) lives behind the `otel` feature,
which is on by default. `LangfuseInterceptor` records each request as an OpenTelemetry span with
the GenAI semantic conventions; `langfuse_exporter_from_env` exports them to Langfuse. It needs
an OpenTelemetry tracer provider in your application:

```bash
cargo add opentelemetry --features trace
cargo add opentelemetry_sdk --features rt-tokio,trace,experimental_trace_batch_span_processor_with_async_runtime
```

At runtime, set `OPENAI_API_KEY`, `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY`, and
`LANGFUSE_HOST` for a self-hosted instance.

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
    let client = Client::from_env()?.with_interceptor(interceptor).build();

    // Keep the result so spans are flushed even when the request fails.
    let result = client.send_chat(client.chat_simple("Hello, world!")).await;
    // The Tokio batch processor needs the runtime while shutdown waits for export.
    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    let response = result?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
# }
```

The [Langfuse integration guide](langfuse-integration.md) covers sessions, users, tags and
metadata (`LangfuseContext`), streaming and shutdown. Examples:
[`langfuse.rs`](../examples/langfuse.rs), [`langfuse_simple.rs`](../examples/langfuse_simple.rs),
[`langfuse_streaming.rs`](../examples/langfuse_streaming.rs).
