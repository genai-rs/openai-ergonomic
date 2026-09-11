//! Multiple tools, async application state and a complete chat replay, offline.

use async_trait::async_trait;
use openai_client_base::models::{
    ChatCompletionMessageToolCallsInner, CreateChatCompletionResponse,
};
use openai_ergonomic::{
    builders::{chat::ChatCompletionBuilder, Builder},
    responses::ChatCompletionResponseWrapper,
    Error, FunctionTool, Result, ToolCallExt, ToolRegistry,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LookupInput {
    key: String,
}
#[derive(Serialize)]
struct LookupOutput {
    value: String,
}
struct Lookup {
    records: Arc<RwLock<BTreeMap<String, String>>>,
}

#[async_trait]
impl FunctionTool for Lookup {
    type Input = LookupInput;
    type Output = LookupOutput;
    fn name(&self) -> &'static str {
        "lookup"
    }
    fn description(&self) -> &'static str {
        "Read a value from the application's record store by exact key."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {
            "key": {"type": "string", "description": "Exact record key"}
        }, "required": ["key"], "additionalProperties": false})
    }
    async fn execute(&self, input: LookupInput) -> Result<LookupOutput> {
        self.records
            .read()
            .await
            .get(&input.key)
            .cloned()
            .map(|value| LookupOutput { value })
            .ok_or_else(|| Error::InvalidRequest("record not found".into()))
    }
}

// Dynamic JSON uses the same trait. Validate application constraints in execute.
struct Echo;
#[async_trait]
impl FunctionTool for Echo {
    type Input = Value;
    type Output = Value;
    fn name(&self) -> &'static str {
        "echo"
    }
    fn description(&self) -> &'static str {
        "Return a JSON object unchanged for debugging."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "additionalProperties": true})
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        Ok(input)
    }
}

fn sample_response() -> Result<ChatCompletionResponseWrapper> {
    let raw: CreateChatCompletionResponse = serde_json::from_value(json!({
        "id": "demo", "object": "chat.completion", "created": 0, "model": "gpt-test",
        "choices": [{"index": 0, "finish_reason": "tool_calls", "logprobs": null,
            "message": {"role": "assistant", "content": "I'll look that up.", "tool_calls": [
                {"id": "call_lookup", "type": "function", "function": {
                    "name": "lookup", "arguments": r#"{"key":"language"}"#}},
                {"id": "call_echo", "type": "function", "function": {
                    "name": "echo", "arguments": r#"{"message":"hello"}"#}}
            ]}}
        ]
    }))?;
    Ok(ChatCompletionResponseWrapper::new(raw))
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let records = Arc::new(RwLock::new(BTreeMap::from([(
        "language".into(),
        "Rust".into(),
    )])));
    let mut tools = ToolRegistry::new();
    tools.register(Lookup { records })?;
    tools.register(Echo)?;
    let mut builder = ChatCompletionBuilder::new("gpt-test")
        .user("Look up language and echo hello.")
        .tools(tools.tool_definitions());
    let response = sample_response()?; // In an application: execute the request with your client.
    let calls: Vec<ChatCompletionMessageToolCallsInner> =
        response.tool_calls().into_iter().cloned().collect();
    // Replay the assistant request BEFORE its tool replies. Keep the prior history.
    builder =
        builder.assistant_with_tool_calls(response.content().unwrap_or_default(), calls.clone());
    for call in &calls {
        // These demo tools are read-only and approved by this application.
        // Put your application's authorization/confirmation step here.
        match tools.execute_call(call).await {
            Ok(output) => builder = builder.tool(output.call_id, output.content),
            Err(error) => {
                eprintln!("{error}");
                // The application chooses what information the model may receive.
                builder = builder.tool(
                    call.id(),
                    json!({"error": "FunctionTool could not complete"}).to_string(),
                );
            }
        }
    }
    let request = builder.build()?;
    assert_eq!(request.messages.len(), 4);
    println!("{}", serde_json::to_string_pretty(&request)?);
    Ok(())
}
