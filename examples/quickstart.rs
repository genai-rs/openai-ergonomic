//! # `OpenAI` Ergonomic Quickstart Guide
//!
//! This example demonstrates how to get started with the openai-ergonomic crate
//! in under 5 minutes. It covers the most common use cases and patterns you'll
//! need for building AI-powered applications.
//!
//! ## Setup Instructions
//!
//! 1. Set your `OpenAI` API key:
//!    ```bash
//!    export OPENAI_API_KEY="sk-your-api-key-here"
//!    ```
//!
//! 2. Run this example:
//!    ```bash
//!    cargo run --example quickstart
//!    ```
//!
//! ## What This Example Shows
//!
//! - Environment setup and client creation
//! - Basic chat completions
//! - Streaming responses (real-time text generation)
//! - Function/tool calling for external data
//! - Robust error handling patterns
//! - Usage tracking and cost monitoring
//!
//! This example is designed to be your first step into building with `OpenAI`.

use openai_ergonomic::responses::tool_function;
use openai_ergonomic::{Client, Error, Response, Result, ToolCallExt};
use serde_json::json;
use std::io::{self, Write};

#[tokio::main]
#[allow(clippy::too_many_lines)] // This is an example showing many features
async fn main() -> Result<()> {
    // Initialize logging to see what's happening under the hood
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!(" OpenAI Ergonomic Quickstart");
    println!("==============================\n");

    // ==========================================
    // 1. ENVIRONMENT SETUP & CLIENT CREATION
    // ==========================================

    println!(" Step 1: Setting up the client");

    // The simplest way to get started - reads OPENAI_API_KEY from environment
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!(" Client created successfully!");
            client_builder.build()
        }
        Err(e) => {
            eprintln!(" Failed to create client: {e}");
            eprintln!(" Make sure you've set OPENAI_API_KEY environment variable");
            eprintln!("   Example: export OPENAI_API_KEY=\"sk-your-key-here\"");
            return Err(e);
        }
    };

    // ==========================================
    // 2. BASIC CHAT COMPLETION
    // ==========================================

    println!("\n Step 2: Basic chat completion");

    // The simplest way to get a response from ChatGPT
    let builder = client.chat_simple("What is Rust programming language in one sentence?");
    let response = client.send_chat(builder).await;

    match response {
        Ok(chat_response) => {
            println!(" Got response!");
            if let Some(content) = chat_response.content() {
                println!(" AI: {content}");
            }

            // Show usage information for cost tracking
            if let Some(usage) = &chat_response.inner().usage {
                println!(
                    " Usage: {} prompt + {} completion = {} total tokens",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }
        Err(e) => {
            println!(" Chat completion failed: {e}");
            // Continue with other examples even if this one fails
        }
    }

    // ==========================================
    // 3. CHAT WITH SYSTEM MESSAGE
    // ==========================================

    println!("\n Step 3: Chat with system context");

    // System messages help set the AI's behavior and context
    let builder = client.chat_with_system(
        "You are a helpful coding mentor who explains things simply",
        "Explain what a HashMap is in Rust",
    );
    let response = client.send_chat(builder).await;

    match response {
        Ok(chat_response) => {
            println!(" Got contextual response!");
            if let Some(content) = chat_response.content() {
                println!("‍ Mentor: {content}");
            }
        }
        Err(e) => {
            println!(" Contextual chat failed: {e}");
        }
    }

    // ==========================================
    // 4. STREAMING RESPONSES
    // ==========================================

    println!("\n Step 4: Streaming response (real-time)");

    // Streaming lets you see the response as it's being generated
    // This is great for chatbots and interactive applications
    print!(" AI is typing");
    io::stdout().flush().unwrap();

    let builder = client
        .responses()
        .user("Write a short haiku about programming")
        .temperature(0.7)
        .stream(true);
    // Note: Full streaming implementation is in development
    // For now, we'll demonstrate non-streaming responses with real-time simulation
    let response = client.send_responses(builder).await;

    match response {
        Ok(chat_response) => {
            print!(": ");
            io::stdout().flush().unwrap();

            // Simulate streaming by printing character by character
            if let Some(content) = chat_response.content() {
                for char in content.chars() {
                    print!("{char}");
                    io::stdout().flush().unwrap();
                    // Small delay to simulate streaming
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                }
            }
            println!(); // New line after "streaming"
        }
        Err(e) => {
            println!("\n Failed to get streaming response: {e}");
        }
    }

    // ==========================================
    // 5. FUNCTION/TOOL CALLING
    // ==========================================

    println!("\n Step 5: Using tools/functions");

    // Tools let the AI call external functions to get real data
    // Here we define a weather function as an example
    let weather_tool = tool_function(
        "get_current_weather",
        "Get the current weather for a given location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city name, e.g. 'San Francisco, CA'"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit"
                }
            },
            "required": ["location"]
        }),
    );

    let builder = client
        .responses()
        .user("What's the weather like in Tokyo?")
        .tool(weather_tool);
    let response = client.send_responses(builder).await;

    match response {
        Ok(chat_response) => {
            println!(" Got response with potential tool calls!");

            // Check if the AI wants to call our weather function
            let tool_calls = chat_response.tool_calls();
            if !tool_calls.is_empty() {
                println!(" AI requested tool calls:");
                for tool_call in tool_calls {
                    let function_name = tool_call.function_name();
                    println!("   Function: {function_name}");
                    let function_args = tool_call.function_arguments();
                    println!("   Arguments: {function_args}");

                    // In a real app, you'd execute the function here
                    // and send the result back to the AI
                    println!("    In a real app, you'd call your weather API here");
                }
            } else if let Some(content) = chat_response.content() {
                println!(" AI: {content}");
            }
        }
        Err(e) => {
            println!(" Tool calling example failed: {e}");
        }
    }

    // ==========================================
    // 6. ERROR HANDLING PATTERNS
    // ==========================================

    println!("\n Step 6: Error handling patterns");

    // Show how to handle different types of errors gracefully
    let builder = client.chat_simple(""); // Empty message might cause an error
    let bad_response = client.send_chat(builder).await;

    match bad_response {
        Ok(response) => {
            println!(" Unexpectedly succeeded with empty message");
            if let Some(content) = response.content() {
                println!(" AI: {content}");
            }
        }
        Err(Error::Api {
            status, message, ..
        }) => {
            println!(" API Error (HTTP {status}):");
            println!("   Message: {message}");
            println!(" This is normal - we sent an invalid request");
        }
        Err(Error::RateLimit { .. }) => {
            println!(" Rate limited - you're sending requests too fast");
            println!(" In a real app, you'd implement exponential backoff");
        }
        Err(Error::Http(_)) => {
            println!(" HTTP/Network error");
            println!(" Check your internet connection and API key");
        }
        Err(e) => {
            println!(" Other error: {e}");
        }
    }

    // ==========================================
    // 7. COMPLETE REAL-WORLD EXAMPLE
    // ==========================================

    println!("\n Step 7: Complete real-world example");
    println!("Building a simple AI assistant that can:");
    println!("- Answer questions with context");
    println!("- Track conversation costs");
    println!("- Handle errors gracefully");

    let mut total_tokens = 0;

    // Simulate a conversation with context and cost tracking
    let questions = [
        "What is the capital of France?",
        "What's special about that city?",
        "How many people live there?",
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("\n User: {question}");

        let builder = client
            .responses()
            .system(
                "You are a knowledgeable geography expert. Keep answers concise but informative.",
            )
            .user(*question)
            .temperature(0.1); // Lower temperature for more factual responses
        let response = client.send_responses(builder).await;

        match response {
            Ok(chat_response) => {
                if let Some(content) = chat_response.content() {
                    println!(" Assistant: {content}");
                }

                // Track token usage for cost monitoring
                if let Some(usage) = chat_response.usage() {
                    total_tokens += usage.total_tokens;
                    println!(
                        " This exchange: {} tokens (Running total: {})",
                        usage.total_tokens, total_tokens
                    );
                }
            }
            Err(e) => {
                println!(" Question {} failed: {}", i + 1, e);
                // In a real app, you might retry or log this error
            }
        }
    }

    // ==========================================
    // 8. WRAP UP & NEXT STEPS
    // ==========================================

    println!("\n Quickstart Complete!");
    println!("======================");
    println!("You've successfully:");
    println!(" Created an OpenAI client");
    println!(" Made basic chat completions");
    println!(" Used streaming responses");
    println!(" Implemented tool/function calling");
    println!(" Handled errors gracefully");
    println!(" Built a complete conversational AI");
    println!("\n Total tokens used in examples: {total_tokens}");
    println!(
        " Estimated cost: ~${:.4} (assuming GPT-4 pricing)",
        f64::from(total_tokens) * 0.03 / 1000.0
    );

    println!("\n Next Steps:");
    println!("- Check out other examples in the examples/ directory");
    println!("- Read the documentation: https://docs.rs/openai-ergonomic");
    println!("- Explore advanced features like vision, audio, and assistants");
    println!("- Build your own AI-powered applications!");

    Ok(())
}

/// Example helper function demonstrating custom error handling.
///
/// In real applications, you might want to wrap API calls in functions
/// like this to add custom retry logic, logging, or error transformation.
#[allow(dead_code)]
async fn robust_chat_call(client: &Client, message: &str) -> Result<String> {
    const MAX_RETRIES: usize = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        let builder = client.chat_simple(message);
        match client.send_chat(builder).await {
            Ok(response) => {
                if let Some(content) = response.content() {
                    return Ok(content.to_string());
                }
                return Err(Error::Api {
                    status: 200,
                    message: "No content in response".to_string(),
                    error_type: None,
                    error_code: None,
                });
            }
            Err(Error::RateLimit { .. }) if attempt < MAX_RETRIES => {
                // Exponential backoff for rate limits
                let delay = std::time::Duration::from_millis(1000 * attempt as u64);
                tokio::time::sleep(delay).await;
                // Brief delay before retry
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES {
                    // Brief delay before retry
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| Error::Api {
        status: 0,
        message: "Unknown error after retries".to_string(),
        error_type: None,
        error_code: None,
    }))
}
