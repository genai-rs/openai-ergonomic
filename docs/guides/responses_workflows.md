# Responses-First Workflows

The Responses API is OpenAI's modern interface for conversational completion, tool calling, and structured outputs. This guide shows how to use `openai-ergonomic` to build **responses-first** applications that lean on the `ResponsesBuilder` helpers and ergonomic client methods.

## Quickstart

```rust,no_run
use openai_ergonomic::{Client, Config};
use openai_ergonomic::builders::responses::ResponsesBuilder;
use tokio::runtime::Runtime;

fn main() -> openai_ergonomic::Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async move {
        let client = Client::from_env()?;

        let builder = client
            .responses()
            .system("You are a concise assistant")
            .user("Give me three bullet points about Rust")
            .temperature(0.4)
            .max_completion_tokens(200);

        let response = client.send_responses(builder).await?;
        if let Some(text) = response.content() {
            println!("{}", text);
        }
        Ok(())
    })
}
```

Key points:

- `Client::responses()` seeds a `ResponsesBuilder` using the configured default model (falls back to `gpt-4`).
- `client.send_responses(builder)` validates the builder, executes the request, and wraps the response in `ChatCompletionResponseWrapper` for ergonomic access (e.g. `primary_text()`).

## Layering System + User Context

`ResponsesBuilder` provides dedicated helpers for the most common prompt patterns:

```rust
let builder = client
    .responses()
    .system("You are a documentation assistant")
    .user("Summarise the 'VectorStoreBuilder' API")
    .presence_penalty(0.1)
    .frequency_penalty(0.1);
```

Under the hood this produces well-formed chat messages (`system`, `user`) against the Responses endpoint.

For quick experiments there are ergonomic short-cuts:

```rust
let builder = openai_ergonomic::responses_system_user(
    "gpt-4o-mini",
    "You are a code reviewer",
    "Review this diff for potential issues",
);
```

## Tool Calling and JSON Mode

Tool invocation is first-class in `ResponsesBuilder` via helper functions that mirror the OpenAI schema.

```rust
use openai_ergonomic::builders::responses::{responses_tool_function, ResponsesBuilder};
use serde_json::json;

let weather_tool = responses_tool_function(
    "get_weather",
    "Fetch the current weather for a city",
    json!({
        "type": "object",
        "properties": {
            "city": {"type": "string"}
        },
        "required": ["city"],
    }),
);

let builder = client
    .responses()
    .system("You are a smart home assistant")
    .user("Should I carry an umbrella in Brussels today?")
    .tool(weather_tool)
    .json_mode();
```

`json_mode()` selects `response_format = { "type": "json_object" }` so downstream code can parse the assistant reply deterministically. When the model selects the `get_weather` function you will receive a `ToolCall` inside the wrapped response (`response.tool_calls()`), making it easy to execute the tool and feed the result back using `ResponsesBuilder::assistant`.

## Streaming Strategy

Set `stream(true)` on the builder when you need incremental delivery. The crate already exposes `ChatCompletionStreamResponseWrapper` for parsing streaming chunks; higher-level helpers on `Client` will follow once the SSE plumbing is stabilised. In the meantime you can combine the ergonomic builder with the lower-level `openai-client-base` streaming APIs:

1. Build the request with `.stream(true)` and call `builder.build()?` to obtain the `CreateChatCompletionRequest` payload.
2. Hand that payload to your preferred streaming transport (for example a thin wrapper over `openai-client-base` or a custom SSE client) and parse each chunk with `ChatCompletionStreamResponseWrapper::new`.

This keeps the builder ergonomics for request construction while still allowing you to drive the streaming loop directly against the generated bindings.

## Response Post-Processing

`ChatCompletionResponseWrapper` exposes utilities tailored for responses-first usage:

- `primary_text()` returns the first non-empty chunk of assistant text.
- `tool_calls()` yields resolved `ToolCall` objects ready for dispatch.
- `usage()` surfaces token accounting, making it easy to log or budget per request.

Combine these helpers to orchestrate sophisticated workflows while keeping the builder ergonomics uniform across streaming and non-streaming responses.

## Putting It Together

Use responses-first workflows as the default for modern OpenAI features:

1. **Shape the conversation** with `system()` + `user()` calls.
2. **Register tools** (`responses_tool_function`, `tool_web_search`) and enable `json_mode()` when a structured reply is required.
3. **Execute** using `Client::send_responses` (for buffered replies) or `Client::execute_responses` + streaming for incremental delivery.
4. **Inspect results** through `ChatCompletionResponseWrapper` to route tool calls, capture tokens, or render text back to the user.

The remaining guides build on this foundation to show how tools, vector stores, and migration paths fit into a complete ergonomic assistant stack.
