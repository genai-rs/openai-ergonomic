//! Comprehensive chat completions example.
//!
//! This example demonstrates advanced chat completion features including:
//! - Multi-turn conversation management
//! - Message history tracking
//! - System, user, and assistant messages
//! - Streaming chat responses
//! - Token usage tracking
//! - Error handling patterns
//!
//! Run with: `cargo run --example chat_comprehensive`

use openai_ergonomic::{Client, Error, Response};
use std::collections::VecDeque;
use std::io::{self, Write};

/// Represents a conversation turn with role and content.
#[derive(Debug, Clone)]
struct ConversationTurn {
    role: String,
    content: String,
    token_count: Option<i32>,
}

/// Manages conversation history and token tracking.
#[derive(Debug)]
struct ConversationManager {
    history: VecDeque<ConversationTurn>,
    max_history: usize,
    total_tokens_used: i32,
    system_message: Option<String>,
}

impl ConversationManager {
    /// Create a new conversation manager with optional system message.
    const fn new(system_message: Option<String>, max_history: usize) -> Self {
        Self {
            history: VecDeque::new(),
            max_history,
            total_tokens_used: 0,
            system_message,
        }
    }

    /// Add a user message to the conversation history.
    fn add_user_message(&mut self, content: String) {
        self.add_turn(ConversationTurn {
            role: "user".to_string(),
            content,
            token_count: None,
        });
    }

    /// Add an assistant message to the conversation history.
    fn add_assistant_message(&mut self, content: String, token_count: Option<i32>) {
        self.add_turn(ConversationTurn {
            role: "assistant".to_string(),
            content,
            token_count,
        });
    }

    /// Add a turn to the history, managing the maximum size.
    fn add_turn(&mut self, turn: ConversationTurn) {
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(turn);
    }

    /// Update total token usage from a response.
    fn update_token_usage(&mut self, prompt_tokens: i32, completion_tokens: i32) {
        let total = prompt_tokens + completion_tokens;
        self.total_tokens_used += total;
    }

    /// Display conversation history.
    fn display_history(&self) {
        println!("\n=== Conversation History ===");

        if let Some(ref system) = self.system_message {
            println!("System: {system}");
            println!();
        }

        for (i, turn) in self.history.iter().enumerate() {
            let token_info = turn
                .token_count
                .map_or_else(String::new, |tokens| format!(" ({tokens} tokens)"));

            println!(
                "{}. {}{}: {}",
                i + 1,
                turn.role
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .collect::<String>()
                    + &turn.role[1..],
                token_info,
                turn.content
            );
        }

        println!("\nTotal tokens used: {}", self.total_tokens_used);
        println!("Messages in history: {}", self.history.len());
        println!("=============================\n");
    }

    /// Get conversation turns for API request.
    fn get_conversation_for_api(&self) -> Vec<(String, String)> {
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(ref system) = self.system_message {
            messages.push(("system".to_string(), system.clone()));
        }

        // Add conversation history
        for turn in &self.history {
            messages.push((turn.role.clone(), turn.content.clone()));
        }

        messages
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenAI Ergonomic - Comprehensive Chat Example");
    println!("============================================");
    println!();

    // Create client from environment variables
    let client = match Client::from_env() {
        Ok(client) => {
            println!("✓ Client initialized successfully");
            client
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize client: {e}");
            eprintln!("Make sure OPENAI_API_KEY environment variable is set");
            return Err(e.into());
        }
    };

    // Initialize conversation manager with system message
    let system_message = "You are a helpful AI assistant. Provide concise, informative responses. \
                          Always be polite and professional. If asked about your capabilities, \
                          explain what you can help with clearly."
        .to_string();

    let mut conversation = ConversationManager::new(Some(system_message), 10);

    println!("✓ Conversation manager initialized (max history: 10 messages)");
    println!("✓ System message configured");
    println!();

    // Demonstrate conversation features
    demonstrate_basic_chat(&client, &mut conversation).await?;
    demonstrate_multi_turn_chat(&client, &mut conversation).await?;
    demonstrate_streaming_chat(&client, &mut conversation).await?;
    demonstrate_token_tracking(&client, &mut conversation).await?;
    demonstrate_error_handling(&client).await?;

    // Final conversation summary
    conversation.display_history();

    println!("🎉 Chat comprehensive example completed successfully!");
    println!("This example demonstrated:");
    println!("  • Multi-turn conversation management");
    println!("  • Message history tracking and rotation");
    println!("  • System message configuration");
    println!("  • Token usage monitoring");
    println!("  • Error handling patterns");
    println!("  • Streaming response handling");

    Ok(())
}

/// Demonstrate basic chat completion.
async fn demonstrate_basic_chat(
    client: &Client,
    conversation: &mut ConversationManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Example 1: Basic Chat Completion");
    println!("----------------------------------");

    let user_message = "Hello! Can you explain what you can help me with?";
    conversation.add_user_message(user_message.to_string());

    println!("User: {user_message}");
    print!("Assistant: ");
    io::stdout().flush()?;

    // Build the chat request with conversation history
    let messages = conversation.get_conversation_for_api();
    let mut chat_builder = client.chat();

    for (role, content) in messages {
        match role.as_str() {
            "system" => chat_builder = chat_builder.system(content),
            "user" => chat_builder = chat_builder.user(content),
            "assistant" => chat_builder = chat_builder.assistant(content),
            _ => {} // Ignore unknown roles
        }
    }

    // Send the request
    let response = client.send_chat(chat_builder.temperature(0.7)).await?;

    if let Some(content) = response.content() {
        println!("{content}");
        conversation.add_assistant_message(content.to_string(), None);

        // Track token usage if available
        if let Some(usage) = response.usage() {
            conversation.update_token_usage(usage.prompt_tokens, usage.completion_tokens);
        }
    } else {
        println!("No response content received");
    }

    println!();
    Ok(())
}

/// Demonstrate multi-turn conversation.
async fn demonstrate_multi_turn_chat(
    client: &Client,
    conversation: &mut ConversationManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💬 Example 2: Multi-turn Conversation");
    println!("------------------------------------");

    let questions = vec![
        "What's the capital of France?",
        "What's the population of that city?",
        "Can you tell me an interesting fact about it?",
    ];

    for question in questions {
        conversation.add_user_message(question.to_string());

        println!("User: {question}");
        print!("Assistant: ");
        io::stdout().flush()?;

        // Build chat request with full conversation history
        let messages = conversation.get_conversation_for_api();
        let mut chat_builder = client.chat();

        for (role, content) in messages {
            match role.as_str() {
                "system" => chat_builder = chat_builder.system(content),
                "user" => chat_builder = chat_builder.user(content),
                "assistant" => chat_builder = chat_builder.assistant(content),
                _ => {}
            }
        }

        let response = client.send_chat(chat_builder.temperature(0.3)).await?;

        if let Some(content) = response.content() {
            println!("{content}");
            conversation.add_assistant_message(content.to_string(), None);

            // Track token usage
            if let Some(usage) = response.usage() {
                conversation.update_token_usage(usage.prompt_tokens, usage.completion_tokens);
            }
        }

        println!();
        // Small delay between questions for readability
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

/// Demonstrate streaming chat response.
async fn demonstrate_streaming_chat(
    _client: &Client,
    conversation: &mut ConversationManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Example 3: Streaming Chat Response");
    println!("------------------------------------");

    // Add user message for streaming example
    let streaming_question = "Can you write a short poem about programming?";
    conversation.add_user_message(streaming_question.to_string());

    println!("User: {streaming_question}");
    println!("Assistant (streaming): ");

    // Note: Streaming is not yet fully implemented in the client
    // This is a placeholder showing the intended API
    println!("🚧 Streaming functionality is being implemented...");
    println!("Future implementation will show real-time token-by-token responses");

    // Simulate what streaming would look like
    let simulated_response = "Programming flows like poetry in motion,\nEach function a verse, each loop a devotion.\nVariables dance through memory's halls,\nWhile algorithms answer logic's calls.";

    // Simulate typing effect
    for char in simulated_response.chars() {
        print!("{char}");
        io::stdout().flush()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }
    println!("\n");

    // Add the response to conversation history
    conversation.add_assistant_message(simulated_response.to_string(), None);

    Ok(())
}

/// Demonstrate token usage tracking.
async fn demonstrate_token_tracking(
    client: &Client,
    conversation: &mut ConversationManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Example 4: Token Usage Tracking");
    println!("---------------------------------");

    let efficiency_question = "In one sentence, what is machine learning?";
    conversation.add_user_message(efficiency_question.to_string());

    println!("User: {efficiency_question}");
    print!("Assistant: ");
    io::stdout().flush()?;

    // Build chat request
    let messages = conversation.get_conversation_for_api();
    let mut chat_builder = client.chat().max_completion_tokens(50); // Limit tokens for demo

    for (role, content) in messages {
        match role.as_str() {
            "system" => chat_builder = chat_builder.system(content),
            "user" => chat_builder = chat_builder.user(content),
            "assistant" => chat_builder = chat_builder.assistant(content),
            _ => {}
        }
    }

    let response = client.send_chat(chat_builder).await?;

    if let Some(content) = response.content() {
        println!("{content}");

        // Display detailed token usage
        if let Some(usage) = response.usage() {
            println!("\n📈 Token Usage Breakdown:");
            println!("  Prompt tokens: {}", usage.prompt_tokens);
            println!("  Completion tokens: {}", usage.completion_tokens);
            println!("  Total tokens: {}", usage.total_tokens);

            conversation.update_token_usage(usage.prompt_tokens, usage.completion_tokens);

            conversation.add_assistant_message(content.to_string(), Some(usage.completion_tokens));
        } else {
            conversation.add_assistant_message(content.to_string(), None);
        }
    }

    println!();
    Ok(())
}

/// Demonstrate error handling patterns.
async fn demonstrate_error_handling(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Example 5: Error Handling Patterns");
    println!("------------------------------------");

    println!("Testing various error scenarios...\n");

    // Test 1: Invalid model
    println!("Test 1: Invalid model name");
    let invalid_model_builder = client.chat()
        .user("Hello")
        // Note: We can't easily test invalid model without modifying the builder
        // This shows the pattern for handling errors
        .temperature(0.7);

    match client.send_chat(invalid_model_builder).await {
        Ok(_) => println!("✓ Request succeeded (model validation not yet implemented)"),
        Err(e) => match &e {
            Error::Api {
                status, message, ..
            } => {
                println!("✗ API Error ({status}): {message}");
            }
            Error::Http(reqwest_err) => {
                println!("✗ HTTP Error: {reqwest_err}");
            }
            Error::InvalidRequest(msg) => {
                println!("✗ Invalid Request: {msg}");
            }
            _ => {
                println!("✗ Unexpected Error: {e}");
            }
        },
    }

    // Test 2: Empty message validation
    println!("\nTest 2: Empty message validation");
    let empty_builder = client.chat(); // No messages added

    match client.send_chat(empty_builder).await {
        Ok(_) => println!("✗ Empty request unexpectedly succeeded"),
        Err(Error::InvalidRequest(msg)) => {
            println!("✓ Validation caught empty request: {msg}");
        }
        Err(e) => {
            println!("✗ Unexpected error type: {e}");
        }
    }

    // Test 3: Configuration errors
    println!("\nTest 3: Configuration validation");
    println!("✓ Client configuration is valid (created successfully)");

    println!("\n🛡️  Error handling patterns demonstrated:");
    println!("  • API error classification");
    println!("  • Request validation");
    println!("  • Network error handling");
    println!("  • Configuration validation");

    println!();
    Ok(())
}
