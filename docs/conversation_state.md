# ConversationState

`ConversationState` wraps the chat completion builder with a stateful API that
keeps the full message history in memory, eliminates repeated `clone` calls,
and caches tool payloads so JSON is serialized only once.

## Why use it?

- **Mutable history** – append user, assistant, or tool messages without
  rebuilding the entire request.
- **Tool-friendly** – store tool outputs as `serde_json::Value` and reuse the
  compact string across follow-up turns.
- **Interoperable** – produce a `ChatCompletionBuilder` or
  `CreateChatCompletionRequest` whenever you need to call `Client::execute_chat`.

## Quick Start

```rust,no_run
use openai_ergonomic::{Client, ConversationState, ToolResult, tool_function};
use serde_json::json;

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let client = Client::from_env()?.build();
    let mut state = ConversationState::new("gpt-4o-mini")
        .with_system("You can access the user's calendar.");

    state.set_tools(vec![tool_function(
        "get_calendar",
        "Return mocked events for the supplied date.",
        json!({
            "type": "object",
            "properties": {
                "date": { "type": "string", "description": "ISO date" }
            },
            "required": ["date"]
        }),
    )]);

    state.push_user("Do I have meetings today?");
    let mut response = client.execute_chat(state.build_request()?).await?;
    state.apply_response(&response);

    while !response.tool_calls().is_empty() {
        for tool_call in response.tool_calls() {
            let args: serde_json::Value = serde_json::from_str(tool_call.function_arguments())?;
            let date = args["date"].as_str().unwrap_or("2024-01-01");

            let result = ToolResult::new(json!({
                "date": date,
                "events": [
                    { "time": "09:00", "title": "Team sync" },
                    { "time": "14:30", "title": "Project review" }
                ]
            }))?;

            state.push_tool_result(tool_call.id(), result);
        }

        response = client.execute_chat(state.build_request()?).await?;
        state.apply_response(&response);
    }

    if let Some(content) = response.content() {
        println!("{content}");
    }

    Ok(())
}
```

## API Highlights

- `push_system`, `push_user`, `push_assistant`, `push_tool_result` mutate the
  conversation in place.
- `tool_result`/`ToolResult::new` create cached JSON payloads for tool outputs.
- `apply_response` appends the first choice from a `ChatCompletionResponseWrapper`.
- `build_request`/`to_builder` hand back the familiar types used by the client,
  so you can mix this API with existing workflows.

See `examples/conversation_state.rs` for a runnable end-to-end example.
