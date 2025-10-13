#![allow(clippy::uninlined_format_args)]
//! Multi-turn tool calling example demonstrating proper conversation history management.
//!
//! This example demonstrates:
//! - Multi-turn tool calling with proper message history
//! - Using `assistant_with_tool_calls()` to maintain context
//! - Complete tool calling loop implementation
//! - Real-world tool execution patterns
//!
//! Run with: `cargo run --example tool_calling_multiturn`

use openai_ergonomic::{
    builders::chat::tool_function, responses::chat::ToolCallExt, Client, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct CalculatorParams {
    operation: String,
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryParams {
    key: String,
    value: Option<String>,
}

// Simple in-memory storage for the memory tool
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn get_calculator_tool() -> openai_client_base::models::ChatCompletionTool {
    tool_function(
        "calculator",
        "Perform basic arithmetic operations: add, subtract, multiply, divide",
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "The first number"
                },
                "b": {
                    "type": "number",
                    "description": "The second number"
                }
            },
            "required": ["operation", "a", "b"]
        }),
    )
}

fn get_memory_tool() -> openai_client_base::models::ChatCompletionTool {
    tool_function(
        "memory",
        "Store or retrieve a value from memory. If value is provided, store it. Otherwise, retrieve it.",
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The key to store or retrieve"
                },
                "value": {
                    "type": "string",
                    "description": "The value to store (omit to retrieve)"
                }
            },
            "required": ["key"]
        }),
    )
}

fn execute_calculator(params: &CalculatorParams) -> String {
    let result = match params.operation.as_str() {
        "add" => params.a + params.b,
        "subtract" => params.a - params.b,
        "multiply" => params.a * params.b,
        "divide" => {
            if params.b == 0.0 {
                return json!({ "error": "Division by zero" }).to_string();
            }
            params.a / params.b
        }
        _ => return json!({ "error": "Unknown operation" }).to_string(),
    };

    json!({
        "operation": params.operation,
        "a": params.a,
        "b": params.b,
        "result": result
    })
    .to_string()
}

fn execute_memory(
    params: &MemoryParams,
    storage: &Arc<Mutex<HashMap<String, String>>>,
) -> String {
    let mut store = storage.lock().unwrap();

    if let Some(value) = &params.value {
        // Store value
        store.insert(params.key.clone(), value.clone());
        json!({
            "action": "stored",
            "key": params.key,
            "value": value
        })
        .to_string()
    } else {
        // Retrieve value
        store.get(&params.key).map_or_else(
            || {
                json!({
                    "action": "not_found",
                    "key": params.key,
                    "message": "Key not found in memory"
                })
                .to_string()
            },
            |value| {
                json!({
                    "action": "retrieved",
                    "key": params.key,
                    "value": value
                })
                .to_string()
            },
        )
    }
}

// Execute a tool call and return the result
fn execute_tool(
    tool_name: &str,
    arguments: &str,
    storage: &Arc<Mutex<HashMap<String, String>>>,
) -> Result<String> {
    match tool_name {
        "calculator" => {
            let params: CalculatorParams = serde_json::from_str(arguments)?;
            Ok(execute_calculator(&params))
        }
        "memory" => {
            let params: MemoryParams = serde_json::from_str(arguments)?;
            Ok(execute_memory(&params, storage))
        }
        _ => Ok(json!({ "error": format!("Unknown tool: {}", tool_name) }).to_string()),
    }
}

// Handle the complete tool calling loop - this is the key function!
async fn handle_tool_loop(
    client: &Client,
    mut chat_builder: openai_ergonomic::builders::chat::ChatCompletionBuilder,
    tools: &[openai_client_base::models::ChatCompletionTool],
    storage: &Arc<Mutex<HashMap<String, String>>>,
) -> Result<String> {
    const MAX_ITERATIONS: usize = 10; // Prevent infinite loops
    let mut iteration = 0;

    loop {
        iteration += 1;
        if iteration > MAX_ITERATIONS {
            return Err(std::io::Error::other("Max iterations reached in tool loop").into());
        }

        println!("\n  [Iteration {}]", iteration);

        // Send request with tools
        let request = chat_builder.clone().tools(tools.to_vec());
        let response = client.send_chat(request).await?;

        // Check if there are tool calls
        let tool_calls = response.tool_calls();
        if tool_calls.is_empty() {
            // No more tool calls, return the final response
            if let Some(content) = response.content() {
                return Ok(content.to_string());
            }
            return Err(std::io::Error::other("No content in final response").into());
        }

        // Process tool calls
        println!("  Tool calls: {}", tool_calls.len());

        // IMPORTANT: Add assistant message with tool calls to history
        // This is the key step that maintains proper conversation context!
        chat_builder = chat_builder.assistant_with_tool_calls(
            response.content().unwrap_or(""),
            tool_calls.iter().map(|tc| (*tc).clone()).collect(),
        );

        // Execute each tool call and add results to history
        for tool_call in tool_calls {
            let tool_name = tool_call.function_name();
            let tool_args = tool_call.function_arguments();
            let tool_id = tool_call.id();

            println!("    → {}: {}", tool_name, tool_args);

            let result = match execute_tool(tool_name, tool_args, storage) {
                Ok(result) => {
                    println!("    ✓ Result: {}", result);
                    result
                }
                Err(e) => {
                    let error_msg = format!("Error: {}", e);
                    eprintln!("    ✗ {}", error_msg);
                    error_msg
                }
            };

            // Add tool result to the conversation
            chat_builder = chat_builder.tool(tool_id, result);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Multi-turn Tool Calling Example ===\n");

    // Initialize client
    let client = Client::from_env()?.build();

    // Create storage for the memory tool
    let storage = Arc::new(Mutex::new(HashMap::new()));

    // Define available tools
    let tools = vec![get_calculator_tool(), get_memory_tool()];

    println!("Available tools:");
    println!("  - calculator: Perform arithmetic operations");
    println!("  - memory: Store and retrieve values");
    println!();

    // Example 1: Single tool call
    println!("Example 1: Single Tool Call");
    println!("User: What is 15 + 27?");
    {
        let chat_builder = client
            .chat()
            .system("You are a helpful assistant with access to a calculator and memory storage.")
            .user("What is 15 + 27?");

        let result = handle_tool_loop(&client, chat_builder, &tools, &storage).await?;
        println!("Assistant: {}", result);
    }

    // Example 2: Multiple sequential tool calls
    println!("\n\nExample 2: Multiple Sequential Tool Calls");
    println!("User: Calculate 10 * 5 and store the result in memory as 'product'");
    {
        let chat_builder = client
            .chat()
            .system("You are a helpful assistant with access to a calculator and memory storage.")
            .user("Calculate 10 * 5 and store the result in memory as 'product'");

        let result = handle_tool_loop(&client, chat_builder, &tools, &storage).await?;
        println!("Assistant: {}", result);
    }

    // Example 3: Retrieve from memory
    println!("\n\nExample 3: Retrieve from Memory");
    println!("User: What did I store in 'product'?");
    {
        let chat_builder = client
            .chat()
            .system("You are a helpful assistant with access to a calculator and memory storage.")
            .user("What did I store in 'product'?");

        let result = handle_tool_loop(&client, chat_builder, &tools, &storage).await?;
        println!("Assistant: {}", result);
    }

    // Example 4: Complex multi-step task
    println!("\n\nExample 4: Complex Multi-step Task");
    println!("User: Calculate 100 / 4, multiply that by 3, and tell me the final result");
    {
        let chat_builder = client
            .chat()
            .system("You are a helpful assistant with access to a calculator and memory storage.")
            .user("Calculate 100 / 4, multiply that by 3, and tell me the final result");

        let result = handle_tool_loop(&client, chat_builder, &tools, &storage).await?;
        println!("Assistant: {}", result);
    }

    // Example 5: Conversation with history
    println!("\n\nExample 5: Conversation with History");
    {
        let mut chat_builder = client
            .chat()
            .system("You are a helpful assistant with access to a calculator and memory storage.");

        // First question
        println!("User: What is 8 + 7?");
        chat_builder = chat_builder.user("What is 8 + 7?");
        let result = handle_tool_loop(&client, chat_builder.clone(), &tools, &storage).await?;
        println!("Assistant: {}", result);

        // Add assistant response to history
        chat_builder = chat_builder.assistant(&result);

        // Follow-up question that depends on previous context
        println!("\nUser: Now multiply that by 3");
        chat_builder = chat_builder.user("Now multiply that by 3");
        let result = handle_tool_loop(&client, chat_builder.clone(), &tools, &storage).await?;
        println!("Assistant: {}", result);
    }

    println!("\n\n=== All examples completed successfully ===");
    println!("\nKey Takeaway:");
    println!("  When implementing multi-turn tool calling, ALWAYS use");
    println!("  assistant_with_tool_calls() to maintain proper conversation");
    println!("  history. This is essential for the model to understand the");
    println!("  tool results and continue the conversation correctly.");

    Ok(())
}
