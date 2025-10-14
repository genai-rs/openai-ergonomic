#![allow(clippy::uninlined_format_args)]
//! Comprehensive Responses API example demonstrating modern `OpenAI` usage patterns.
//!
//! This example showcases the Responses API, which is `OpenAI`'s recommended modern interface
//! for chat completions, function calling, web search, and structured outputs.
//!
//! ## Features Demonstrated
//!
//! - Basic chat completions using the Responses API
//! - Function calling with custom tools
//! - Web search integration (if supported)
//! - Structured JSON outputs with schemas
//! - Comprehensive error handling patterns
//! - Multiple message types (system, user, assistant)
//! - Different model configurations
//!
//! ## Prerequisites
//!
//! Set your `OpenAI` API key:
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example responses_comprehensive
//! ```

use openai_ergonomic::{
    responses::{tool_function, tool_web_search, Response, ToolChoiceHelper},
    Client, Error, ToolCallExt,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" OpenAI Ergonomic - Comprehensive Responses API Example\n");

    // Initialize client from environment variables
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!(" Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!(" Failed to initialize client: {e}");
            eprintln!(" Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Example 1: Basic Responses API Usage
    println!("\n Example 1: Basic Responses API Usage");
    println!("=====================================");

    match basic_responses_example(&client).await {
        Ok(()) => println!(" Basic responses example completed"),
        Err(e) => {
            eprintln!(" Basic responses example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 2: Function Calling
    println!("\n Example 2: Function Calling");
    println!("===============================");

    match function_calling_example(&client).await {
        Ok(()) => println!(" Function calling example completed"),
        Err(e) => {
            eprintln!(" Function calling example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 3: Web Search Integration
    println!("\n Example 3: Web Search Integration");
    println!("====================================");

    match web_search_example(&client).await {
        Ok(()) => println!(" Web search example completed"),
        Err(e) => {
            eprintln!(" Web search example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 4: Structured Outputs
    println!("\n Example 4: Structured JSON Outputs");
    println!("======================================");

    match structured_output_example(&client).await {
        Ok(()) => println!(" Structured output example completed"),
        Err(e) => {
            eprintln!(" Structured output example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 5: Advanced Configuration
    println!("\n  Example 5: Advanced Configuration");
    println!("====================================");

    match advanced_configuration_example(&client).await {
        Ok(()) => println!(" Advanced configuration example completed"),
        Err(e) => {
            eprintln!(" Advanced configuration example failed: {e}");
            handle_api_error(&e);
        }
    }

    println!("\n All examples completed! Check the console output above for results.");
    Ok(())
}

/// Example 1: Basic Responses API usage with system and user messages
async fn basic_responses_example(client: &Client) -> Result<(), Error> {
    println!("Creating a basic response with system context...");

    // Build a simple request with system and user messages
    let builder = client
        .responses()
        .system("You are a helpful assistant who provides concise, accurate answers.")
        .user("What is the capital of France?")
        .temperature(0.7)
        .max_completion_tokens(100);

    let response = client.send_responses(builder).await?;

    // Extract and display the response
    if let Some(content) = response.content() {
        println!(" Assistant: {content}");
    } else {
        println!("  No content in response");
    }

    // Show response metadata
    println!(" Response metadata:");
    println!("   - Model: {}", response.model().unwrap_or("unknown"));
    println!(
        "   - Finish reason: {}",
        response
            .finish_reason()
            .unwrap_or_else(|| "unknown".to_string())
    );

    if let Some(usage) = response.usage() {
        println!(
            "   - Tokens used: {} prompt + {} completion = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}

/// Example 2: Function calling with custom tools
async fn function_calling_example(client: &Client) -> Result<(), Error> {
    println!("Setting up function calling with custom tools...");

    // Define a weather function tool
    let weather_tool = tool_function(
        "get_weather",
        "Get the current weather information for a specific location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city name, e.g., 'San Francisco, CA'"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit preference"
                }
            },
            "required": ["location"],
            "additionalProperties": false
        }),
    );

    // Define a time function tool
    let time_tool = tool_function(
        "get_current_time",
        "Get the current time in a specific timezone",
        json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "Timezone name, e.g., 'America/New_York'"
                }
            },
            "required": ["timezone"],
            "additionalProperties": false
        }),
    );

    // Make a request that should trigger function calling
    let builder = client
        .responses()
        .system("You are a helpful assistant with access to weather and time information. Use the provided tools when users ask about weather or time.")
        .user("What's the weather like in London and what time is it there?")
        .tool(weather_tool)
        .tool(time_tool)
        .tool_choice(ToolChoiceHelper::auto())
        .temperature(0.3);

    let response = client.send_responses(builder).await?;

    // Check if the model wants to call functions
    let tool_calls = response.tool_calls();
    if !tool_calls.is_empty() {
        println!(" Model requested {} tool call(s):", tool_calls.len());

        for (i, tool_call) in tool_calls.iter().enumerate() {
            println!("   {}. Function: {}", i + 1, tool_call.function_name());
            println!("      Arguments: {}", tool_call.function_arguments());

            // In a real application, you would:
            // 1. Parse the arguments
            // 2. Execute the actual function
            // 3. Send the results back to the model
            println!("      [Simulated] Executing function call...");
            match tool_call.function_name() {
                "get_weather" => {
                    println!("      [Simulated] Weather: 22°C, partly cloudy");
                }
                "get_current_time" => {
                    println!("      [Simulated] Time: 14:30 GMT");
                }
                _ => {
                    println!("      [Simulated] Unknown function");
                }
            }
        }
    } else if let Some(content) = response.content() {
        println!(" Assistant response: {content}");
    }

    Ok(())
}

/// Example 3: Web search integration
async fn web_search_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating web search tool integration...");

    // Create a web search tool
    let web_search_tool = tool_web_search();

    // Ask a question that would benefit from current information
    let builder = client
        .responses()
        .system("You are a helpful assistant with access to web search. When users ask about current events, recent information, or real-time data, use the web search tool to find accurate, up-to-date information.")
        .user("What are the latest developments in artificial intelligence this week?")
        .tool(web_search_tool)
        .tool_choice(ToolChoiceHelper::auto())
        .temperature(0.3)
        .max_completion_tokens(200);

    let response = client.send_responses(builder).await?;

    // Handle the response
    let tool_calls = response.tool_calls();
    if !tool_calls.is_empty() {
        println!(" Model requested web search:");

        for tool_call in &tool_calls {
            if tool_call.function_name() == "web_search" {
                println!("   Search query: {}", tool_call.function_arguments());
                println!("   [Simulated] Performing web search...");
                println!("   [Simulated] Found recent AI news and developments");

                // In a real implementation:
                // 1. Parse the search query from arguments
                // 2. Perform actual web search
                // 3. Return results to the model
                // 4. Get final response with search results
            }
        }
    } else if let Some(content) = response.content() {
        println!(" Assistant response: {content}");
    }

    println!(" Note: Web search requires additional implementation to execute actual searches");

    Ok(())
}

/// Example 4: Structured JSON outputs with schemas
async fn structured_output_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating structured JSON outputs...");

    // Define a schema for recipe information
    let recipe_schema = json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the recipe"
            },
            "ingredients": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Ingredient name"
                        },
                        "amount": {
                            "type": "string",
                            "description": "Amount needed"
                        }
                    },
                    "required": ["name", "amount"],
                    "additionalProperties": false
                },
                "description": "List of ingredients"
            },
            "instructions": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": "Step-by-step cooking instructions"
            },
            "prep_time_minutes": {
                "type": "integer",
                "description": "Preparation time in minutes"
            },
            "difficulty": {
                "type": "string",
                "enum": ["easy", "medium", "hard"],
                "description": "Recipe difficulty level"
            }
        },
        "required": ["name", "ingredients", "instructions", "prep_time_minutes", "difficulty"],
        "additionalProperties": false
    });

    // Request a recipe in structured JSON format
    let builder = client
        .responses()
        .system("You are a cooking expert. Provide recipes in the exact JSON format specified.")
        .user("Give me a simple recipe for chocolate chip cookies")
        .json_schema("recipe", recipe_schema)
        .temperature(0.5);

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!(" Structured recipe output:");

        // Try to parse and pretty-print the JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            Err(_) => {
                println!("Raw response: {content}");
            }
        }
    }

    // Example of simple JSON mode (without schema)
    println!("\n Simple JSON mode example:");
    let simple_builder = client
        .responses()
        .system("Respond in valid JSON format with keys: summary, key_points, sentiment")
        .user("Analyze this text: 'The new product launch exceeded expectations with great customer feedback.'")
        .json_mode()
        .temperature(0.3);

    let simple_response = client.send_responses(simple_builder).await?;

    if let Some(content) = simple_response.content() {
        println!(" Analysis result: {content}");
    }

    Ok(())
}

/// Example 5: Advanced configuration and parameters
async fn advanced_configuration_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating advanced response configuration...");

    // Example with multiple completions and various parameters
    let builder = client
        .responses()
        .system("You are a creative writing assistant. Write in different styles when asked.")
        .user("Write a short tagline for a futuristic coffee shop")
        .temperature(0.9)  // High creativity
        .max_completion_tokens(50)
        .n(1)  // Generate 1 completion
        .top_p(0.9)
        .frequency_penalty(0.1)
        .presence_penalty(0.1)
        .stop(vec!["\n".to_string(), ".".to_string()])
        .seed(42)  // For reproducible results
        .user_id("example_user_123");

    let response = client.send_responses(builder).await?;

    println!(" Creative tagline generation:");
    if let Some(content) = response.content() {
        println!("   Result: {content}");
    }

    // Example with reasoning effort (for o3 models)
    println!("\n Example with reasoning effort (o3 models):");
    let reasoning_builder = client
        .responses()
        .system("You are a logic puzzle solver. Think through problems step by step.")
        .user("If a train leaves Station A at 2 PM going 60 mph, and another train leaves Station B at 3 PM going 80 mph, and the stations are 280 miles apart, when do they meet?")
        .reasoning_effort("medium")
        .temperature(0.1); // Low temperature for accuracy

    let reasoning_response = client.send_responses(reasoning_builder).await?;

    if let Some(content) = reasoning_response.content() {
        println!("   Solution: {content}");
    } else {
        println!("   Note: Reasoning effort requires compatible model (e.g., o3)");
    }

    // Show model information
    println!("\n Model and usage information:");
    println!("   Model used: {}", response.model().unwrap_or("unknown"));
    if let Some(usage) = response.usage() {
        println!(
            "   Token usage: {} total ({} prompt + {} completion)",
            usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
        );
    }

    Ok(())
}

/// Comprehensive error handling helper
fn handle_api_error(error: &Error) {
    match error {
        Error::Api {
            status,
            message,
            error_type,
            error_code,
        } => {
            eprintln!(" API Error [{status}]: {message}");
            if let Some(error_type) = error_type {
                eprintln!("   Type: {error_type}");
            }
            if let Some(error_code) = error_code {
                eprintln!("   Code: {error_code}");
            }

            // Provide specific guidance based on error type
            match *status {
                401 => eprintln!(" Check your API key: export OPENAI_API_KEY=\"your-key\""),
                429 => eprintln!(" Rate limited - try again in a moment"),
                500..=599 => eprintln!(" Server error - try again later"),
                _ => {}
            }
        }
        Error::InvalidRequest(msg) => {
            eprintln!(" Invalid Request: {msg}");
            eprintln!(" Check your request parameters");
        }
        Error::Config(msg) => {
            eprintln!(" Configuration Error: {msg}");
            eprintln!(" Check your client configuration");
        }
        Error::Http(err) => {
            eprintln!(" HTTP Error: {err}");
            eprintln!(" Check your network connection");
        }
        Error::HttpMiddleware(err) => {
            eprintln!(" HTTP Middleware Error: {err}");
            eprintln!(" Check your network connection and middleware configuration");
        }
        Error::Json(err) => {
            eprintln!(" JSON Error: {err}");
            eprintln!(" Response parsing failed - may be a temporary issue");
        }
        Error::Authentication(msg) => {
            eprintln!(" Authentication Error: {msg}");
            eprintln!(" Check your API key");
        }
        Error::RateLimit(msg) => {
            eprintln!(" Rate Limit Error: {msg}");
            eprintln!(" Try again in a moment");
        }
        Error::Stream(msg) => {
            eprintln!(" Stream Error: {msg}");
            eprintln!(" Connection issue with streaming");
        }
        Error::File(err) => {
            eprintln!(" File Error: {err}");
            eprintln!(" Check file permissions and paths");
        }
        Error::Builder(msg) => {
            eprintln!(" Builder Error: {msg}");
            eprintln!(" Check your request builder configuration");
        }
        Error::Internal(msg) => {
            eprintln!(" Internal Error: {msg}");
            eprintln!(" This may be a bug, please report it");
        }
        Error::StreamConnection { message } => {
            eprintln!(" Stream Connection Error: {message}");
            eprintln!(" Check your network connection");
        }
        Error::StreamParsing { message, chunk } => {
            eprintln!(" Stream Parsing Error: {message}");
            eprintln!("   Problematic chunk: {chunk}");
            eprintln!(" The response stream may be corrupted");
        }
        Error::StreamBuffer { message } => {
            eprintln!(" Stream Buffer Error: {message}");
            eprintln!(" The stream buffer encountered an issue");
        }
    }
}
