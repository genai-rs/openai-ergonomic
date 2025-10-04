#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::useless_vec)]
//! Completions API example.
//!
//! This example demonstrates:
//! - Creating text completions
//! - Using different parameters
//! - Handling completion responses
//! - Best practices for the legacy Completions API
//!
//! Note: The Completions API is legacy. For new applications, use the Chat API instead.
//!
//! Run with: `cargo run --example completions`

use openai_ergonomic::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Completions API Examples ===\n");

    // Initialize client
    let client = Client::from_env()?;

    // Example 1: Basic completion
    println!("1. Basic Text Completion:");
    basic_completion(&client).await?;

    // Example 2: Completion with parameters
    println!("\n2. Completion with Parameters:");
    completion_with_parameters(&client).await?;

    // Example 3: Multiple completions
    println!("\n3. Multiple Completions:");
    multiple_completions(&client).await?;

    // Example 4: Completion with stop sequences
    println!("\n4. Completion with Stop Sequences:");
    completion_with_stop(&client).await?;

    // Example 5: Completion with suffix (insert mode)
    println!("\n5. Completion with Suffix (Insert Mode):");
    completion_with_suffix(&client).await?;

    // Example 6: Completion with echo
    println!("\n6. Completion with Echo:");
    completion_with_echo(&client).await?;

    println!("\n=== All examples completed successfully ===");

    Ok(())
}

async fn basic_completion(client: &Client) -> Result<()> {
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Write a tagline for an ice cream shop")
        .max_tokens(60);

    let response = client.completions().create(builder).await?;

    println!("Prompt: Write a tagline for an ice cream shop");
    if let Some(choice) = response.choices.first() {
        println!("Completion: {}", choice.text);
        println!("Finish reason: {:?}", choice.finish_reason);
    }

    if let Some(usage) = response.usage {
        println!(
            "Tokens used: {} prompt + {} completion = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}

async fn completion_with_parameters(client: &Client) -> Result<()> {
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Explain quantum computing in simple terms:")
        .max_tokens(100)
        .temperature(0.7)
        .top_p(0.9)
        .frequency_penalty(0.5)
        .presence_penalty(0.0);

    let response = client.completions().create(builder).await?;

    println!("Parameters:");
    println!("  Temperature: 0.7");
    println!("  Top P: 0.9");
    println!("  Frequency Penalty: 0.5");
    println!("  Presence Penalty: 0.0");
    println!();

    if let Some(choice) = response.choices.first() {
        println!("Completion: {}", choice.text);
    }

    Ok(())
}

async fn multiple_completions(client: &Client) -> Result<()> {
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Brainstorm three names for a pet cat:")
        .max_tokens(50)
        .n(3) // Generate 3 different completions
        .temperature(0.9); // Higher temperature for more variety

    let response = client.completions().create(builder).await?;

    println!("Generating {} completions:", response.choices.len());
    for (i, choice) in response.choices.iter().enumerate() {
        println!("  {}. {}", i + 1, choice.text.trim());
    }

    Ok(())
}

async fn completion_with_stop(client: &Client) -> Result<()> {
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("List three programming languages:\n1.")
        .max_tokens(100)
        .temperature(0.0)
        .add_stop("\n4.") // Stop at the fourth item
        .add_stop("\n\n"); // Also stop at double newline

    let response = client.completions().create(builder).await?;

    println!("Prompt: List three programming languages:");
    if let Some(choice) = response.choices.first() {
        println!("Completion:\n1.{}", choice.text);
        println!("Stopped because: {:?}", choice.finish_reason);
    }

    Ok(())
}

async fn completion_with_suffix(client: &Client) -> Result<()> {
    // Insert mode: provide text before and after the insertion point
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("def hello_world():\n    print(\"Hello, ")
        .suffix("\")\n    return True")
        .max_tokens(10)
        .temperature(0.0);

    let response = client.completions().create(builder).await?;

    println!("Insert mode example:");
    println!("Before: def hello_world():\\n    print(\"Hello, ");
    if let Some(choice) = response.choices.first() {
        println!("Inserted: {}", choice.text);
    }
    println!("After: \")\\n    return True");

    Ok(())
}

async fn completion_with_echo(client: &Client) -> Result<()> {
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("The capital of France is")
        .max_tokens(10)
        .echo(true) // Echo back the prompt
        .temperature(0.0);

    let response = client.completions().create(builder).await?;

    println!("Echo enabled:");
    if let Some(choice) = response.choices.first() {
        println!("Full text (prompt + completion): {}", choice.text);
    }

    Ok(())
}
