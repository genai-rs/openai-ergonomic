#![allow(clippy::uninlined_format_args)]
//! Modern tool/function calling example with streaming support.
//!
//! This example demonstrates:
//! - Function tool definition with parameters
//! - Tool calling in chat completions
//! - Handling tool responses
//! - Streaming with tool calls
//! - Error handling for tool execution
//!
//! Run with: `cargo run --example tool_calling`

use openai_ergonomic::{
    builders::chat::tool_function,
    responses::{chat::ToolCallExt, ToolChoiceHelper},
    Client, Result,
};
// Note: Complex message types are commented out for simplification
// use openai_client_base::models::{
//     ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
//     ChatCompletionRequestToolMessage, ChatCompletionRequestUserMessage,
// };
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct WeatherParams {
    location: String,
    unit: Option<String>,
}

#[derive(Debug, Serialize)]
struct WeatherResponse {
    temperature: i32,
    unit: String,
    description: String,
}

fn get_weather_tool() -> openai_client_base::models::ChatCompletionTool {
    tool_function(
        "get_weather",
        "Get the current weather in a given location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit to use"
                }
            },
            "required": ["location"]
        }),
    )
}

fn get_time_tool() -> openai_client_base::models::ChatCompletionTool {
    tool_function(
        "get_current_time",
        "Get the current time in a specific timezone",
        json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "The timezone, e.g. America/New_York"
                }
            },
            "required": ["timezone"]
        }),
    )
}

fn execute_weather_function(params: WeatherParams) -> Result<String> {
    // Simulated weather API call
    let response = WeatherResponse {
        temperature: 72,
        unit: params.unit.unwrap_or("fahrenheit".to_string()),
        description: format!("Sunny in {}", params.location),
    };

    Ok(serde_json::to_string(&response)?)
}

fn execute_time_function(timezone: &str) -> Result<String> {
    // Simulated time API call
    Ok(format!("Current time in {}: 2:30 PM", timezone))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize client from environment
    let client = Client::from_env()?;

    println!("=== Tool Calling Example ===\n");

    // Example 1: Simple tool call
    println!("1. Simple Tool Call:");
    simple_tool_call(&client).await?;

    // Example 2: Multiple tools
    println!("\n2. Multiple Tools:");
    multiple_tools(&client).await?;

    // Example 3: Tool choice control
    println!("\n3. Tool Choice Control:");
    tool_choice_control(&client).await?;

    // Example 4: Conversation with tool calls
    println!("\n4. Conversation with Tool Calls:");
    conversation_with_tools(&client).await?;

    // Example 5: Streaming with tools (simplified)
    println!("\n5. Streaming with Tools (Simplified):");
    streaming_with_tools(&client);

    // Example 6: Parallel tool calls (simplified)
    println!("\n6. Parallel Tool Calls (Simplified):");
    parallel_tool_calls(&client).await?;

    Ok(())
}

async fn simple_tool_call(client: &Client) -> Result<()> {
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
    }

    Ok(())
}

async fn multiple_tools(client: &Client) -> Result<()> {
    let builder = client
        .chat()
        .user("What's the weather in NYC and what time is it there?")
        .tools(vec![get_weather_tool(), get_time_tool()]);
    let response = client.send_chat(builder).await?;

    for tool_call in response.tool_calls() {
        match tool_call.function_name() {
            "get_weather" => {
                let params: WeatherParams = serde_json::from_str(&tool_call.function_arguments())?;
                let result = execute_weather_function(params)?;
                println!("Weather result: {}", result);
            }
            "get_current_time" => {
                let params: serde_json::Value =
                    serde_json::from_str(&tool_call.function_arguments())?;
                if let Some(timezone) = params["timezone"].as_str() {
                    let result = execute_time_function(timezone)?;
                    println!("Time result: {}", result);
                }
            }
            _ => println!("Unknown tool: {}", tool_call.function_name()),
        }
    }

    Ok(())
}

async fn tool_choice_control(client: &Client) -> Result<()> {
    // Force specific tool
    println!("Forcing weather tool:");
    let builder = client
        .chat()
        .user("Tell me about Paris")
        .tools(vec![get_weather_tool(), get_time_tool()])
        .tool_choice(ToolChoiceHelper::specific("get_weather"));
    let response = client.send_chat(builder).await?;

    for tool_call in response.tool_calls() {
        println!("Forced tool: {}", tool_call.function_name());
    }

    // Disable tools
    println!("\nDisabling tools:");
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

async fn conversation_with_tools(client: &Client) -> Result<()> {
    // This is a simplified version that demonstrates the concept
    // without getting into the complexities of message history management

    println!("=== Conversation with Tools (Simplified) ===");

    // First request with tool call
    let builder = client
        .chat()
        .user("What's the weather in Tokyo?")
        .tools(vec![get_weather_tool()]);
    let response = client.send_chat(builder).await?;

    // Check for tool calls and simulate responses
    for tool_call in response.tool_calls() {
        println!("Tool called: {}", tool_call.function_name());
        println!("Arguments: {}", tool_call.function_arguments());

        // In a real implementation, you would:
        // 1. Parse the arguments
        // 2. Execute the actual function
        // 3. Create tool messages with results
        // 4. Send another request with the tool results

        println!("Simulated weather result: Sunny, 24°C");
    }

    println!("Note: Full conversation with tool results requires complex message handling");
    println!("This simplified version demonstrates tool calling detection");

    Ok(())
}

fn streaming_with_tools(_client: &Client) {
    println!("Streaming response with tools:");

    // Note: Streaming with tool calls is more complex and requires
    // proper handling of partial tool call chunks. For now, this is
    // a placeholder showing the concept.

    println!("This would demonstrate streaming tool calls if streaming API was available");
    println!("In streaming mode, tool calls would arrive as chunks that need to be assembled");
}

async fn parallel_tool_calls(client: &Client) -> Result<()> {
    let builder = client
        .chat()
        .user("Check the weather in Tokyo, London, and New York")
        .tools(vec![get_weather_tool()]);
    let response = client.send_chat(builder).await?;

    // Modern models can call multiple tools in parallel
    let tool_calls = response.tool_calls();
    println!("Parallel tool calls: {}", tool_calls.len());

    // Collect arguments first to avoid lifetime issues
    let args_vec: Vec<String> = tool_calls
        .iter()
        .map(|tc| tc.function_arguments().to_string())
        .collect();

    // Execute all in parallel using tokio
    let mut handles = Vec::new();
    for args in args_vec {
        let handle = tokio::spawn(async move {
            let params: WeatherParams = serde_json::from_str(&args)?;
            execute_weather_function(params)
        });
        handles.push(handle);
    }

    // Wait for all results
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(result)) => println!("Location {}: {}", i + 1, result),
            Ok(Err(e)) => println!("Location {} error: {}", i + 1, e),
            Err(e) => println!("Task {} panicked: {}", i + 1, e),
        }
    }

    Ok(())
}
