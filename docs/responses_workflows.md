# Responses workflows

`client.responses()` starts a `ResponsesBuilder` with the configured default model (`gpt-4` when
none is set); `responses_simple(model, user)` and `responses_system_user(model, system, user)`
start one for another model. `send_responses` returns a `ChatCompletionResponseWrapper`, like
`send_chat`.

What it sends: `ResponsesBuilder` builds a `CreateChatCompletionRequest` (system, user and
assistant messages, sampling parameters, tools, a response format) and sends it to the Chat
Completions endpoint (`/chat/completions`), as `ChatCompletionBuilder` does. To call the
`/responses` endpoint itself, send a request of your own with `post_json` or `send_json_stream`
(see [bring your own types](chat-and-responses.md#bring-your-own-types)).

## Messages and parameters

```rust,no_run
use openai_ergonomic::{responses_system_user, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .responses()
        .system("You are a concise assistant.")
        .user("Give me three bullet points about Rust.")
        .temperature(0.4)
        .max_completion_tokens(200)
        .presence_penalty(0.1);
    let response = client.send_responses(builder).await?;
    println!("{}", response.content().unwrap_or_default());

    // Another model, in one call
    let builder = responses_system_user(
        "gpt-4o-mini",
        "You are a code reviewer.",
        "Review: fn add(a: i32, b: i32) -> i32 { a - b }",
    );
    let response = client.send_responses(builder).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

The builder checks its input before sending (a model, at least one message, `temperature` in
`0.0..=2.0`, ...) and returns `Error::InvalidRequest` otherwise.

## Structured output

`json_schema(name, schema)` asks for JSON that matches a schema (`strict` is not set);
`json_mode()` for any JSON object. See
[structured output](chat-and-responses.md#structured-output) and
[`examples/structured_outputs.rs`](../examples/structured_outputs.rs).

## Tools

`responses_tool_function(name, description, parameters)` makes a function tool from a JSON
schema (`tool_function` does the same); `tool(..)` / `tools(..)` add tools and `tool_choice(..)`
picks how the model uses them (`ToolChoiceHelper::auto()`, `required()`, `none()`,
`specific(name)`). The model's calls are in `response.tool_calls()`; `ToolCallExt` reads their
id, name and arguments.

```rust,no_run
use openai_ergonomic::builders::responses::responses_tool_function;
use openai_ergonomic::responses::ToolChoiceHelper;
use openai_ergonomic::{Client, ToolCallExt};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let weather = responses_tool_function(
        "get_weather",
        "Fetch the current weather for a city",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string", "description": "The city"}},
            "required": ["city"],
        }),
    );
    let builder = client
        .responses()
        .system("You are a smart home assistant.")
        .user("Should I carry an umbrella in Brussels today?")
        .tool(weather)
        .tool_choice(ToolChoiceHelper::auto());
    let response = client.send_responses(builder).await?;

    for call in response.tool_calls() {
        let arguments: Value = call.parse_arguments()?;
        println!("{} ({}) wants {}", call.function_name(), call.id(), arguments);
    }
    Ok(())
}
```

`ResponsesBuilder` has no tool-result messages: to answer the calls and continue the
conversation, use a `ChatCompletionBuilder` (`assistant_with_tool_calls` and `tool`), as in
[tool orchestration](tool_orchestration.md), or the [tool framework](tool_framework.md) to run
typed tools. `responses_tool_web_search()` (and `tool_web_search()`) is a *function* tool named
`web_search`: your application performs the search when the model calls it.

## Streaming

`send_responses_stream` and `send_responses_stream_with_completion` stream the same request:
see [streaming](streaming.md).

## Reading the response

`ChatCompletionResponseWrapper` has `content()`, `tool_calls()`, `finish_reason()`,
`is_refusal()` / `refusal()`, `choices()` and `inner()` (the generated
`CreateChatCompletionResponse`); with the `Response` trait in scope, `id()`, `model()` and
`usage()` (the token counts).

Examples: [`responses_comprehensive.rs`](../examples/responses_comprehensive.rs),
[`responses_streaming.rs`](../examples/responses_streaming.rs),
[`structured_outputs.rs`](../examples/structured_outputs.rs).
