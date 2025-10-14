#![allow(clippy::uninlined_format_args)]
//! Comprehensive chat streaming example demonstrating real-time response generation.
//!
//! This example demonstrates:
//! - Basic streaming with `send_chat_stream()`
//! - Processing chunks in real-time
//! - Collecting full response from stream
//! - Handling streaming errors
//! - Streaming with different parameters
//! - Streaming with tool calls (basic)
//!
//! Run with: `cargo run --example chat_streaming`

use futures::StreamExt;
use openai_ergonomic::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Chat Streaming Examples ===\n");

    // Initialize client
    let client = Client::from_env()?.build();

    // Example 1: Basic streaming
    println!("1. Basic Streaming:");
    basic_streaming(&client).await?;

    // Example 2: Streaming with parameters
    println!("\n2. Streaming with Parameters:");
    streaming_with_parameters(&client).await?;

    // Example 3: Collect full content
    println!("\n3. Collect Full Content:");
    collect_content(&client).await?;

    // Example 4: Stream with system message
    println!("\n4. Stream with System Message:");
    streaming_with_system(&client).await?;

    // Example 5: Multiple user turns
    println!("\n5. Multiple User Turns:");
    multiple_turns(&client).await?;

    println!("\n=== All examples completed successfully ===");

    Ok(())
}

async fn basic_streaming(client: &Client) -> Result<()> {
    println!("Question: Tell me a short joke");

    let builder = client.chat().user("Tell me a short joke");

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
    println!();

    Ok(())
}

async fn streaming_with_parameters(client: &Client) -> Result<()> {
    println!("Question: Write a creative tagline for a bakery");

    let builder = client
        .chat()
        .user("Write a creative tagline for a bakery")
        .temperature(0.9)
        .max_tokens(50);

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
    println!();

    Ok(())
}

async fn collect_content(client: &Client) -> Result<()> {
    println!("Question: What is the capital of France?");

    let builder = client.chat().user("What is the capital of France?");

    let stream = client.send_chat_stream(builder).await?;

    let content = stream.collect_content().await?;
    println!("Full response: {}", content);

    Ok(())
}

async fn streaming_with_system(client: &Client) -> Result<()> {
    println!("System: You are a helpful assistant that speaks like a pirate");
    println!("Question: Tell me about the weather");

    let builder = client
        .chat()
        .system("You are a helpful assistant that speaks like a pirate")
        .user("Tell me about the weather")
        .max_tokens(100);

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
    println!();

    Ok(())
}

async fn multiple_turns(client: &Client) -> Result<()> {
    println!("Building a conversation with multiple turns...\n");

    // First turn
    println!("User: What is 2+2?");
    let builder = client.chat().user("What is 2+2?");

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Assistant: ");
    let mut first_response = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
            first_response.push_str(content);
        }
    }
    println!();

    // Second turn - continuing the conversation
    println!("\nUser: Now multiply that by 3");
    let builder = client
        .chat()
        .user("What is 2+2?")
        .assistant(&first_response)
        .user("Now multiply that by 3");

    let mut stream = client.send_chat_stream(builder).await?;

    print!("Assistant: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{}", content);
        }
    }
    println!();

    Ok(())
}
