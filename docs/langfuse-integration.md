# Langfuse Integration for OpenAI Ergonomic

This document describes the Langfuse middleware integration for comprehensive LLM observability using OpenTelemetry.

## Overview

The Langfuse interceptor provides OpenTelemetry-based observability for OpenAI API interactions. It captures detailed traces including:
- Request/response payloads
- Token usage metrics
- Streaming chunk tracking
- Error capture and reporting
- Performance metrics (duration, latency)

## Installation

The Langfuse integration is included in the main crate. Ensure your `Cargo.toml` includes:

```toml
[dependencies]
openai-ergonomic = "=0.6.0"
tokio = { version = "1", features = ["full"] }
opentelemetry = { version = "0.32", features = ["trace"] }
opentelemetry_sdk = { version = "0.32.1", features = ["rt-tokio", "trace", "experimental_trace_batch_span_processor_with_async_runtime"] }
```

For the current, unpublished repository source, use the exact source revision and
OpenTelemetry 0.33.1 manifest shown in the [README's development setup](../README.md#langfuse-setup).
The published 0.6.0 crate and that development revision require different
OpenTelemetry minor versions; do not mix the dependency sets.

## Configuration

### Environment Variables

Configure Langfuse using environment variables:

- `LANGFUSE_PUBLIC_KEY` (required): Your Langfuse public key (starts with "pk-lf-")
- `LANGFUSE_SECRET_KEY` (required): Your Langfuse secret key (starts with "sk-lf-")
- `LANGFUSE_HOST` (optional): Langfuse API host (defaults to "https://cloud.langfuse.com")
- `LANGFUSE_SESSION_ID` (optional): Session ID for grouping related traces
- `LANGFUSE_USER_ID` (optional): User ID for attribution
- `LANGFUSE_RELEASE` (optional): Release version
- `LANGFUSE_DEBUG` (optional): Enable debug logging ("true"/"false", defaults to "false")

### Programmatic Configuration

You can also configure Langfuse programmatically:

```rust
use openai_ergonomic::{LangfuseConfig, LangfuseInterceptor};
use std::time::Duration;

let config = LangfuseConfig::new(
    "https://cloud.langfuse.com",
    "pk-lf-your-public-key",
    "sk-lf-your-secret-key"
)
.with_session_id("session-123")
.with_user_id("user-456")
.with_release("v1.0.0")
.with_timeout(Duration::from_secs(15))
.with_debug(true);

let interceptor = LangfuseInterceptor::new(config)?;
```

## Usage

### Basic Usage

Follow the [compiled Langfuse setup in the README](../README.md#langfuse-setup).
That single source is included in crate documentation and checked by
`cargo test --doc --all-features` as a `no_run` doctest. It constructs the exporter,
tracer provider, and interceptor, sends a request with `send_chat`, and flushes
spans before exiting. Compilation needs no credentials or live API calls.

The dependencies above match the published 0.6.0 package. The README also records a separately pinned development-source manifest.
Set `OPENAI_API_KEY`, `LANGFUSE_PUBLIC_KEY`, and `LANGFUSE_SECRET_KEY` only when
running it.

### Advanced Configuration with Builder

```rust
use openai_ergonomic::{Client, LangfuseInterceptorBuilder};
use std::time::Duration;

let interceptor = LangfuseInterceptorBuilder::new()
    .with_credentials("pk-lf-...", "sk-lf-...")
    .with_host("https://cloud.langfuse.com")
    .with_session_id("session-123")
    .with_user_id("user-456")
    .with_release("v1.0.0")
    .with_batch_size(50)
    .with_export_interval(Duration::from_secs(10))
    .with_timeout(Duration::from_secs(30))
    .with_debug(true)
    .build()?;

let client = Client::from_env()?
    .with_interceptor(Box::new(interceptor))
    .build();
```

## Traced Operations

The interceptor automatically traces:

1. **Chat Completions**: Captures messages, model, temperature, max tokens, and response content
2. **Embeddings**: Tracks input text and embedding generation
3. **Streaming**: Records individual chunks and aggregates streaming metrics
4. **Errors**: Captures error details with full context

## Span Attributes

Spans include semantic attributes following OpenTelemetry conventions:

- `gen_ai.system`: Always "openai"
- `gen_ai.operation.name`: Operation type (e.g., "chat", "embedding")
- `gen_ai.request.model`: Model used
- `gen_ai.request.temperature`: Temperature parameter
- `gen_ai.request.max_tokens`: Max tokens parameter
- `gen_ai.usage.input_tokens`: Input token count
- `gen_ai.usage.output_tokens`: Output token count
- `gen_ai.response.id`: Response ID from OpenAI
- Custom attributes for duration, streaming metrics, etc.

## Batching and Performance

The interceptor uses OpenTelemetry's batch span processor for efficient trace export:

- Default batch size: 100 spans
- Default export interval: 5 seconds
- Default timeout: 10 seconds

These can be customized via configuration.

## Examples

See the `examples/` directory for complete examples:

- `langfuse_simple.rs`: Basic usage with environment variables
- `langfuse.rs`: Advanced usage with multiple scenarios
- `langfuse_streaming.rs`: Streaming chat completions with real-time observability

Run an example:

```bash
export OPENAI_API_KEY="your-key"
export LANGFUSE_PUBLIC_KEY="pk-lf-..."
export LANGFUSE_SECRET_KEY="sk-lf-..."
cargo run --example langfuse_simple
cargo run --example langfuse_streaming  # For streaming example
```

### Streaming Support

The interceptor fully supports streaming responses. When using `send_chat_stream()` or `send_responses_stream()`, the interceptor automatically:

1. Calls `before_request` when streaming starts
2. Calls `on_stream_chunk` for each chunk (non-blocking)
3. Calls `on_stream_end` when streaming completes with token usage

Example:

```rust
use openai_ergonomic::{Client, LangfuseInterceptor};
use futures::StreamExt;

let interceptor = LangfuseInterceptor::from_env()?;
let client = Client::from_env()?
    .with_interceptor(Box::new(interceptor))
    .build();

let builder = client.chat().user("Tell me a story");
let mut stream = client.send_chat_stream(builder).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = chunk.content() {
        print!("{}", content);
    }
    // Each chunk is automatically traced to Langfuse
}
// on_stream_end is called automatically with token usage
```

**Note**: When using streaming with Langfuse, ensure your application has a small delay before shutting down the tracer provider to allow spawned interceptor tasks to complete. See `langfuse_streaming.rs` example for details.

## Debugging

Enable debug logging to see detailed trace information:

```bash
export LANGFUSE_DEBUG=true
export RUST_LOG=openai_ergonomic=debug
```

## Integration with Langfuse Dashboard

Once configured, traces appear in your Langfuse dashboard at:
- Cloud: https://cloud.langfuse.com
- Self-hosted: Your configured `LANGFUSE_HOST`

Features available in the dashboard:
- Trace visualization with timing information
- Token usage tracking and cost calculation
- Error rate monitoring
- Latency percentiles
- Model performance comparison
- Session and user analytics

## Troubleshooting

1. **No traces appearing**: Check credentials and host configuration
2. **Incomplete traces**: Ensure the application waits for final batch export before exiting
3. **High latency**: Adjust batch size and export interval
4. **Debug issues**: Enable `LANGFUSE_DEBUG=true` and check logs

## License

Same as the parent crate - MIT OR Apache-2.0