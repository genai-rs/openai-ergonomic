# Tool Framework

`openai-ergonomic` ships a lightweight framework for defining, registering, and executing
tools that the Chat API can call. The framework lives in [`tool_framework`](../src/tool_framework.rs)
and provides both traits and helper macros to remove boilerplate.

## Quick start

```rust,no_run
use openai_ergonomic::{
    tool_framework::{ToolRegistry, TypedTool},
    tool_schema, Client,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct TimeParams {
    timezone: Option<String>,
}

struct GetTime;

#[async_trait]
impl TypedTool for GetTime {
    type Params = TimeParams;

    fn name(&self) -> &str { "get_time" }
    fn description(&self) -> &str { "Return the current time" }
    fn parameters_schema(&self) -> Value {
        tool_schema!(
            timezone: "string", "Optional IANA timezone identifier", required: false,
        )
    }

    async fn execute_typed(&self, params: Self::Params) -> openai_ergonomic::Result<Value> {
        let tz = params.timezone.unwrap_or_else(|| "UTC".to_string());
        Ok(serde_json::json!({ "timezone": tz, "time": "12:34" }))
    }
}

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let client = Client::from_env()?.build();
    let registry = ToolRegistry::new().register(GetTime);

    let tool_defs = registry.tool_definitions();
    let mut request = client
        .chat()
        .system("You can look up the time")
        .user("What time is it in Tokyo?")
        .tools(tool_defs)
        .build()?;

    let response = client.execute_chat(request.clone()).await?;

    if !response.tool_calls().is_empty() {
        let tool_results = registry.process_tool_calls(&response).await?;
        for (tool_call_id, json) in tool_results {
            request.messages.push(openai_client_base::models::ChatCompletionRequestMessage::ChatCompletionRequestToolMessage(Box::new(
                openai_client_base::models::ChatCompletionRequestToolMessage {
                    role: openai_client_base::models::chat_completion_request_tool_message::Role::Tool,
                    content: Box::new(openai_client_base::models::ChatCompletionRequestToolMessageContent::TextContent(json)),
                    tool_call_id,
                }
            )));
        }
        let follow_up = client.execute_chat(request).await?;
        if let Some(content) = follow_up.content() {
            println!("Assistant: {content}");
        }
    }

    Ok(())
}
```

## Strongly typed input + output

Use [`StronglyTypedTool`] when you want native Rust types for both inputs and outputs.
The [`strongly_typed_tool!`] macro implements the trait for you:

```rust,no_run
use openai_ergonomic::{
    strongly_typed_tool,
    tool_framework::{StronglyTypedToolAdapter, ToolRegistry},
    tool_schema,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct AddParams { lhs: i64, rhs: i64 }

#[derive(Debug, Serialize)]
struct AddResult { sum: i64 }

strongly_typed_tool!(
    AddTool,
    "add_numbers",
    "Add two integers",
    AddParams,
    AddResult,
    tool_schema!(
        lhs: "integer", "Left operand", required: true,
        rhs: "integer", "Right operand", required: true,
    ),
    |params: AddParams| async move {
        Ok(AddResult { sum: params.lhs + params.rhs })
    }
);

let registry = ToolRegistry::new().register(StronglyTypedToolAdapter::new(AddTool));
```

## Helper macros

Macro | Purpose
----- | -------
[`tool_schema!`] | Tiny DSL to build JSON schema objects inline.
[`simple_tool!`] | Define a tool with no parameters that returns a string.
[`strongly_typed_tool!`] | Generate the boilerplate for [`StronglyTypedTool`].

## Design highlights

- **No hidden globals** – registries are plain structs so you can build different tool sets per request.
- **Async-first** – all trait methods are async so you can call databases or HTTP services from inside tools.
- **Framework agnostic** – works with the chat completion API, the responses API, or any flow where you need to run tool calls returned by OpenAI.
- **Interoperable** – the registry returns vanilla `ChatCompletionTool` entries. You can mix and match with manual definitions if needed.

For runnable demonstrations, see [`examples/tool_framework.rs`](../examples/tool_framework.rs) and
[`examples/tool_framework_strongly_typed.rs`](../examples/tool_framework_strongly_typed.rs).
