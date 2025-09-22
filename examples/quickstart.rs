//! Quickstart example for the openai-ergonomic crate.
//!
//! This example demonstrates basic usage of the client and builders.

use openai_ergonomic::{Client, Config, ResponsesBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client from environment variables
    // Expects OPENAI_API_KEY to be set
    let client = Client::from_env()?;

    // Example 1: Simple chat completion
    println!("Example 1: Simple chat completion");
    let response_builder = client.chat_simple("What is 2+2?");
    // Note: Full implementation requires fixing the type issues first
    println!("Builder created for: What is 2+2?");

    // Example 2: Chat with system message
    println!("\nExample 2: Chat with system message");
    let response_builder = client.chat_with_system(
        "You are a helpful math tutor",
        "Explain the Pythagorean theorem",
    );
    println!("Builder created with system context");

    // Example 3: Using the Responses API builder directly
    println!("\nExample 3: Responses API");
    let responses_builder = client
        .responses()
        .system("You are a creative writer")
        .user("Write a haiku about programming")
        .temperature(0.7);
    println!("Responses builder configured");

    // Example 4: Function calling with tools
    println!("\nExample 4: Function calling");
    use openai_ergonomic::responses::tool_function;

    let tool = tool_function(
        "get_weather",
        "Get the current weather for a location",
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city name"
                }
            },
            "required": ["location"]
        }),
    );

    let responses_builder = client
        .responses()
        .user("What's the weather in San Francisco?")
        .tool(tool);
    println!("Tool calling configured");

    println!("\nExamples demonstrate the builder pattern API");
    println!("Note: Actual API calls require completing type fixes");

    Ok(())
}
