//! Simple tool calling example that compiles successfully.
//!
//! This example demonstrates basic tool calling functionality.

use openai_ergonomic::{
    builders::chat::tool_function,
    responses::{chat::ToolCallExt, ToolChoiceHelper},
    Client, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct WeatherParams {
    location: String,
}

fn get_weather_tool() -> openai_ergonomic::responses::Tool {
    tool_function(
        "get_weather",
        "Get the current weather for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city name"
                }
            },
            "required": ["location"]
        }),
    )
}

fn execute_weather_function(params: WeatherParams) -> Result<String> {
    // Simulate weather lookup
    Ok(format!("The weather in {} is sunny, 24°C", params.location))
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Tool Calling Example ===");

    let client = Client::from_env()?;

    // Simple tool call
    let builder = client
        .chat()
        .user("What's the weather like in San Francisco?")
        .tools(vec![get_weather_tool()]);
    let response = client.send_chat(builder).await?;

    // Check for tool calls
    let tool_calls = response.tool_calls();
    if !tool_calls.is_empty() {
        for tool_call in tool_calls {
            println!("Tool called: {}", tool_call.function_name());
            println!("Arguments: {}", tool_call.function_arguments());

            // Execute the function
            let params: WeatherParams = serde_json::from_str(&tool_call.function_arguments())?;
            let result = execute_weather_function(params)?;
            println!("Function result: {}", result);
        }
    } else if let Some(content) = response.content() {
        println!("Response: {}", content);
    }

    // Forced tool choice
    println!("\n=== Forced Tool Choice ===");
    let builder = client
        .chat()
        .user("Tell me about Paris")
        .tools(vec![get_weather_tool()])
        .tool_choice(ToolChoiceHelper::specific("get_weather"));
    let response = client.send_chat(builder).await?;

    for tool_call in response.tool_calls() {
        println!("Forced tool: {}", tool_call.function_name());
    }

    // No tools
    println!("\n=== No Tools Mode ===");
    let builder = client
        .chat()
        .user("What's the weather?")
        .tools(vec![get_weather_tool()])
        .tool_choice(ToolChoiceHelper::none());
    let response = client.send_chat(builder).await?;

    if let Some(content) = response.content() {
        println!("Response without tools: {}", content);
    }

    Ok(())
}
