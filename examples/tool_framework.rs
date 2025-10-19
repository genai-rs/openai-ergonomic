//! Demonstrates defining and registering tools with the tool framework.

use async_trait::async_trait;
use openai_ergonomic::{
    tool_framework::{ToolRegistry, TypedTool},
    tool_schema, Builder, Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct GetWeatherParams {
    location: String,
}

struct GetWeatherTool;

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl TypedTool for GetWeatherTool {
    type Params = GetWeatherParams;

    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "Get the current weather for a location"
    }

    fn parameters_schema(&self) -> Value {
        tool_schema!(
            location: "string", "City name or zip code", required: true,
        )
    }

    async fn execute_typed(&self, params: Self::Params) -> openai_ergonomic::Result<Value> {
        Ok(serde_json::json!({
            "location": params.location,
            "forecast": "Sunny",
            "temperature_c": 22.5
        }))
    }
}

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let client = Client::from_env()?.build();

    let registry = ToolRegistry::new().register(GetWeatherTool);
    let tool_defs = registry.tool_definitions();

    let mut request = client
        .chat()
        .system("You can look up the weather")
        .user("What's the weather like in Rome?")
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
    } else if let Some(content) = response.content() {
        println!("Assistant: {content}");
    }

    Ok(())
}
