# Responses workflows

The Responses API (`POST /responses`) is `OpenAI`'s interface for text, tools and multi-turn
conversations. `client.responses()` starts a `ResponsesBuilder` with the configured default
model (`gpt-4` when none is set); `ResponsesBuilder::new(model)`, `responses_simple(model, input)`
and `responses_system_user(model, instructions, input)` start one for another model.
`send_responses` sends it and returns a `ResponsesResponse`; `send_responses_stream` streams its
events. The builder, the response and the events live in the `responses_api` module and wrap
the generated `openai-client-base` models (`CreateResponse`, `Response`,
`ResponseStreamEvent`).

The builder is neutral: it sends only what you set, so the API's defaults apply (for example,
responses are stored unless `store(false)`).

## Input, instructions and parameters

```rust,no_run
use openai_ergonomic::{responses_system_user, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .responses()
        .instructions("You are a concise assistant.")
        .user("Give me three bullet points about Rust.")
        .temperature(0.4)
        .max_output_tokens(200);
    let response = client.send_responses(builder).await?;
    println!("{}", response.output_text());

    // Another model, in one call
    let builder = responses_system_user(
        "gpt-4.1-mini",
        "You are a code reviewer.",
        "Review: fn add(a: i32, b: i32) -> i32 { a - b }",
    );
    let response = client.send_responses(builder).await?;
    println!("{} ({:?})", response.output_text(), response.status());
    Ok(())
}
```

The input is text (`input_text`) or a list of items: `user`, `system`, `developer` and
`assistant` append messages, `function_call_output` the answer to a function call, and
`input_item` / `input_items` any generated `InputItem`. Other parameters: `previous_response_id`,
`store`, `include`, `prompt_cache_key`, `parallel_tool_calls`, `max_tool_calls`,
`reasoning_effort` / `reasoning`, `metadata`, `safety_identifier`, `top_p`. `params_mut()`
reaches every field of the generated `CreateResponse`, which `build()` returns; the client
sends it with the generated `create_response` and `create_response_stream` operations.

The builder checks its input before sending (a model; input, a previous response or a prompt;
`temperature` in `0.0..=2.0`; a known reasoning effort) and returns `Error::InvalidRequest`
otherwise.

## Reading the response

`ResponsesResponse` has `id()`, `model()`, `status()` (`Completed`, `Incomplete`, `Failed`,
...), `error()`, `incomplete_reason()`, `output()` (the generated `OutputItem`s),
`output_text()` (the text of the output messages), `refusal()`, `function_calls()`, `usage()`
and `inner()` / `into_inner()` (the generated `Response`). A response that failed or stopped
early on the model's side is a successful call: check `status()`. Errors of the call itself
(an error status, a network failure) are `Error`s, as for every other call; a `429` is
`Error::Api { status: 429, .. }`, for which `is_rate_limit()` is true.

## Tools

`function_tool(name, description, parameters)` adds a function tool from a JSON schema;
`tool(..)` / `tools(..)` add any generated `Tool` (built-in tools such as web search or file
search too) and `tool_choice(..)` picks how the model uses them. The model's calls are
`response.function_calls()`: answer each with `function_call_output(call_id, output)` and
continue the conversation, either from the stored response (`previous_response_id`) or by
sending the earlier output back (`output_as_input()`) when nothing is stored.

```rust,no_run
use openai_ergonomic::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let weather = json!({
        "type": "object",
        "properties": {"city": {"type": "string", "description": "The city"}},
        "required": ["city"],
        "additionalProperties": false
    });

    let first = client
        .send_responses(
            client
                .responses()
                .function_tool("get_weather", "Current weather of a city", weather.clone())
                .user("Should I carry an umbrella in Brussels today?"),
        )
        .await?;

    let mut next = client
        .responses()
        .function_tool("get_weather", "Current weather of a city", weather)
        .previous_response_id(first.id());
    for call in first.function_calls() {
        println!("{} wants {}({})", call.call_id, call.name, call.arguments);
        next = next.function_call_output(&call.call_id, r#"{"sky":"rain"}"#);
    }
    let answer = client.send_responses(next).await?;
    println!("{}", answer.output_text());
    Ok(())
}
```

Without stored responses (`store(false)`), send the conversation each time: the earlier input,
then `input_items(first.output_as_input()?)`, then the function call outputs. With
`include(vec![IncludeEnum::ReasoningEncryptedContent])` the reasoning items of reasoning models
come back encrypted and can be sent back the same way.

Built-in tools are generated `Tool`s; their JSON definition decodes into one:

```rust,no_run
use openai_ergonomic::responses_api::ResponsesTool;
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let web_search: ResponsesTool = serde_json::from_value(serde_json::json!({"type": "web_search"}))?;
    let response = client
        .send_responses(client.responses().tool(web_search).user("Rust news this week?"))
        .await?;
    println!("{}", response.output_text());
    Ok(())
}
```

## Structured output

`json_schema(name, schema)` asks for JSON that matches a schema (strict); `json_mode()` for
any JSON object. See [structured output](chat-and-responses.md#structured-output) and
[`examples/structured_outputs.rs`](../examples/structured_outputs.rs).

## Streaming

`send_responses_stream` yields every event of the response as a `ResponsesEvent`:
`event_type()` (such as `"response.output_text.delta"`), `text_delta()`,
`function_call_arguments_delta()`, `output_item_done()` (a finished item, such as a complete
function call, before the response ends), `response()` / `completed_response()`,
`is_terminal()` and `raw()` (the generated `ResponseStreamEvent`).

```rust,no_run
use futures::StreamExt;
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let mut events = client
        .send_responses_stream(client.responses().user("Write a haiku about Rust."))
        .await?;
    while let Some(event) = events.next().await {
        let event = event?;
        if let Some(text) = event.text_delta() {
            print!("{text}");
        }
        if let Some(response) = event.completed_response() {
            println!("\n[{}]", response.id);
        }
    }
    Ok(())
}
```

The stream ends after the terminal event (`response.completed`, `response.incomplete`,
`response.failed` or `error`), which it yields: a failed response is an event, not an error. A
stream that ends before a terminal event yields `Error::StreamConnection`; an event that is not
JSON is skipped; an event of a type this crate does not know is yielded with
`ResponseStreamEvent::Unknown`. `send_responses_stream_with_completion` also returns the
interceptors' completion handle, as for chat: see [streaming](streaming.md).

## Stored responses

`client.responses_api()` retrieves (`retrieve(id)`), cancels (`cancel(id)`, for background
responses) and deletes (`delete(id)`) stored responses; `create` and `create_stream` are the
same as `send_responses` and `send_responses_stream`.

Examples: [`responses_comprehensive.rs`](../examples/responses_comprehensive.rs),
[`responses_streaming.rs`](../examples/responses_streaming.rs),
[`structured_outputs.rs`](../examples/structured_outputs.rs).
