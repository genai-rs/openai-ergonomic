//! Comprehensive Azure `OpenAI` API Test
//!
//! This example tests various Azure `OpenAI` endpoints to verify that the
//! integration works correctly across different API features.

use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Azure OpenAI Comprehensive API Test ===\n");

    let client = Client::from_env()?.build();

    // Test 1: Simple chat completion
    println!("1. Testing simple chat completion...");
    let builder = client.chat_simple("What is 2+2? Answer in one word.");
    match client.send_chat(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ Chat completion: {content}");
            }
        }
        Err(e) => println!("   ✗ Chat completion failed: {e}"),
    }

    // Test 2: Chat with system message
    println!("\n2. Testing chat with system message...");
    let builder = client.chat_with_system(
        "You are a helpful assistant that responds in one sentence.",
        "What is Rust?",
    );
    match client.send_chat(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ System message chat: {content}");
            }
        }
        Err(e) => println!("   ✗ System message chat failed: {e}"),
    }

    // Test 3: Chat with temperature
    println!("\n3. Testing chat with custom parameters...");
    let builder = client
        .chat()
        .user("Say 'test' in a creative way")
        .temperature(0.7)
        .max_tokens(50);
    match client.send_chat(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ Custom parameters: {content}");
            }
        }
        Err(e) => println!("   ✗ Custom parameters failed: {e}"),
    }

    // Test 4: Multiple messages conversation
    println!("\n4. Testing multi-message conversation...");
    let builder = client
        .chat()
        .system("You are a helpful assistant")
        .user("My name is Alice")
        .assistant("Hello Alice! Nice to meet you.")
        .user("What's my name?");
    match client.send_chat(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ Multi-message: {content}");
            }
        }
        Err(e) => println!("   ✗ Multi-message failed: {e}"),
    }

    // Test 5: Chat with max_tokens limit
    println!("\n5. Testing with max_tokens limit...");
    let builder = client.chat().user("Explain quantum physics").max_tokens(20);
    match client.send_chat(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ Limited tokens: {content}");
                println!("   (Note: response is truncated due to max_tokens=20)");
            }
        }
        Err(e) => println!("   ✗ Max tokens test failed: {e}"),
    }

    // Test 6: Using responses API
    println!("\n6. Testing responses API...");
    let builder = client.responses().user("What is the capital of France?");
    match client.send_responses(builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("   ✓ Responses API: {content}");
            }
        }
        Err(e) => println!("   ✗ Responses API failed: {e}"),
    }

    println!("\n=== Test Summary ===");
    println!("Azure OpenAI integration tested across multiple endpoints!");
    println!("\nNote: Some advanced features like embeddings, streaming, and");
    println!("tool calling may require specific Azure OpenAI deployments.");

    Ok(())
}
