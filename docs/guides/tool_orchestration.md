# Tool Orchestration

Modern assistants often combine model reasoning with external tools such as file search, code execution, or custom business APIs. `openai-ergonomic` provides builders that make it straightforward to wire these moving pieces together while keeping your code readable.

## 1. Describe Available Tools

Start by defining tools the assistant may call. The `responses_tool_function` helper mirrors OpenAI's JSON schema surface, keeping your definitions type-safe.

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

let builder = ResponsesBuilder::new("gpt-4o-mini")
    .system("You are a household assistant")
    .user("Should I carry an umbrella in Brussels today?")
    .tool(weather_tool)
    .json_mode();
```

Calling `client.send_responses(builder).await?` yields a `ChatCompletionResponseWrapper`. When the model decides to invoke `get_weather`, you will see a `ToolCall` via `response.tool_calls()`.

## 2. Stage Threads with Attachments

When orchestrating longer-lived assistants it is useful to seed a thread with files or metadata. The new `ThreadRequestBuilder` and `MessageAttachment` types handle the fiddly JSON payloads for you.

```rust
use openai_ergonomic::builders::threads::{
    AttachmentTool, MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};

let thread_builder = ThreadRequestBuilder::new()
    .user_message("Summarise the uploaded architecture docs")
    .message_builder(
        ThreadMessageBuilder::assistant("Sure—please upload the doc.")
            .attachment(
                MessageAttachment::for_file_search("file-abc123")
                    .with_tool(AttachmentTool::CodeInterpreter),
            ),
    )?;

let thread = client.threads().create(thread_builder).await?;
println!("Thread {} is ready", thread.id);
```

The attachment builders ensure each file is associated with the correct tool (`file_search`, `code_interpreter`, or both) so the model can retrieve the content later. Metadata support uses the same fluent interface—call `builder.metadata("project", "alpha")` to tag the thread.

## 3. Execute Tool Results

Once a tool call appears, execute it and feed the result back to the model. The response wrapper exposes helper methods to streamline the loop:

```rust
use openai_ergonomic::tool_function;

let response = client.send_responses(builder).await?;
if let Some(tool_call) = response.tool_calls().first() {
    if tool_call.name() == "get_weather" {
        let args = tool_call.arguments_as::<serde_json::Value>()?;
        let city = args["city"].as_str().unwrap_or_default();

        // Run your custom integration and capture the result
        let summary = format!("Weather report for {city}: 18°C, light rain");

        // Feed the result back into the conversation
        let follow_up = client
            .responses()
            .assistant(format!("Tool result: {summary}"));
        let final_message = client.send_responses(follow_up).await?;
        println!("{}", final_message.content().unwrap_or("(no reply)"));
    }
}
```

## 4. Combine with Runs (Optional)

For full Assistants API flows you can still leverage the ergonomic builders to construct payloads before dispatching via `openai-client-base`:

- `AssistantBuilder` defines assistants with tools.
- `ThreadRequestBuilder` seeds conversation state.
- `RunBuilder` from `builders::assistants` configures temperature, streaming, and instructions for each run.

Even while higher-level client helpers are evolving, these builders ensure your request payloads stay small, readable, and validated.

## Summary

1. **Define tools** once using `responses_tool_function` and add them to your responses builder.
2. **Seed threads** with `ThreadRequestBuilder` and `MessageAttachment` so the assistant can see uploaded context.
3. **Inspect tool calls** via `ChatCompletionResponseWrapper::tool_calls()` and execute external logic.
4. **Loop the result back** into the model using another `ResponsesBuilder` (or by appending to the thread) to close the orchestration cycle.

Ergonomic builders keep each step declarative, letting you focus on business logic instead of hand-writing JSON payloads.
