# Responses-First Workflows

The `responses` surface is the recommended entry point when you want consistent reasoning behaviour across modalities. This guide walks through the pieces provided by `openai-ergonomic` so you can configure a responses workflow end-to-end without dropping down to the generated client.

## Prerequisites

- Add `openai-ergonomic` to `Cargo.toml` and install the `tokio` runtime.
- Export `OPENAI_API_KEY` (or provide a key manually via [`Config::builder`](../../src/config.rs)).
- Decide on a default model (for example `gpt-4.1-mini` or `o4-mini`).

```rust,no_run
use openai_ergonomic::Config;

let config = Config::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .default_model("gpt-4.1-mini")
    .build();
```

## Building a Conversation

`Client::responses()` returns a [`ResponsesBuilder`](../../src/builders/responses.rs) initialised with your default model. Compose the request with chained helpers for common message patterns.

```rust,no_run
use openai_ergonomic::{Client, Config};

let client = Client::new(config)?;
let builder = client
    .responses()
    .system("You are a precise, helpful travel assistant.")
    .user("Plan a weekend in Brussels with museums and local food suggestions.")
    .temperature(0.7)
    .max_completion_tokens(600);

let response = client.send_responses(builder).await?;
println!("{}", response.primary_text());
```

Behind the scenes the builder creates `CreateChatCompletionRequest` structures, so anything supported by the Responses REST endpoint is available via fluent setters (parallel tool calls, reasoning effort, truncation strategy, etc.).

## JSON Mode and Schemas

Switching to structured output does not require manual payload editing. Call `.json_mode()` to opt in to `'json_object'`, or `.json_schema(name, schema)` to embed a full schema using `serde_json::json!`.

```rust,no_run
use openai_ergonomic::builders::ResponsesBuilder;
use serde_json::json;

let itinerary_schema = json!({
    "type": "object",
    "required": ["days"],
    "properties": {
        "days": {
            "type": "array",
            "items": {
                "type": "object",
                "required": ["theme", "activities"],
                "properties": {
                    "theme": {"type": "string"},
                    "activities": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                }
            }
        }
    }
});

let schema_builder = ResponsesBuilder::new("gpt-4.1-mini")
    .user("Summarise the latest sprint review.")
    .json_schema("sprint_summary", itinerary_schema)
    .max_completion_tokens(400);
```

The crate’s [`ChatCompletionResponseWrapper`](../../src/responses/chat.rs) exposes helpers such as `primary_text()`, `iter_json_values()`, or `tool_calls()` so you can consume the structured data directly.

## Tool Calls Inside Responses

When you need tool orchestration, register tools via `.tool(...)` or `.tool_choice(...)`. Helpers like [`tool_function`](../../src/responses/mod.rs) and [`tool_web_search`](../../src/responses/mod.rs) build valid descriptors without boilerplate.

```rust,no_run
use openai_ergonomic::responses::{tool_function, ToolChoice};

let weather_tool = tool_function(
    "get_weather",
    "Fetch the current weather for a city",
    serde_json::json!({
        "type": "object",
        "properties": {"city": {"type": "string"}},
        "required": ["city"]
    }),
);

let run = client
    .responses()
    .user("Do I need an umbrella in Rome tomorrow?")
    .tool(weather_tool)
    .tool_choice(ToolChoice::Auto)
    .stream(true);
```

Tool handling pairs naturally with the [`Client::responses()`](../../src/client.rs) API—after the response resolves you can inspect tool calls through the wrapper utilities.

## Streaming Considerations

Enable streaming with `.stream(true)` and iterate over the server-sent events via `ResponsesBuilder::stream()` helpers. The crate’s response wrapper converts the chunked protocol into ergonomic state transitions, so you can progressively render tokens or collect tool call deltas before finalising the run.

```rust,no_run
use futures::StreamExt;

let mut stream = client
    .responses()
    .user("Explain how async/await works in Rust using an analogy.")
    .stream(true)
    .await?;

while let Some(chunk) = stream.next().await {
    let delta = chunk?;
    if let Some(text) = delta.delta_text() {
        print!("{}", text);
    }
}
```

## Putting It Together

1. Configure the client using `Config::builder()`.
2. Build a `ResponsesBuilder` with messages, parameters, and (optionally) tools.
3. Opt into JSON mode or schemas when you need structured outputs.
4. Use streaming for responsive UX; otherwise call `Client::send_responses` for a buffered result.
5. Consume the rich helpers on `ChatCompletionResponseWrapper` to post-process model output.

With these pieces in place you can keep responses-focused code self-contained inside the ergonomic crate, avoiding direct calls to the generated `openai-client-base` API.
