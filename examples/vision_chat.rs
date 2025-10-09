#![allow(clippy::uninlined_format_args)]
//! Vision chat example demonstrating image understanding capabilities.
//!
//! This example demonstrates:
//! - Basic image understanding with URLs
//! - Base64 image encoding and analysis
//! - Multiple image analysis in a single message
//! - Different detail levels (low, high, auto)
//! - Conversation context with images
//! - Comprehensive error handling
//!
//! Run with: `cargo run --example vision_chat`

use openai_ergonomic::{
    image_base64_part_with_detail, image_url_part_with_detail, text_part, Client, Detail, Error,
    Response,
};
use std::io::{self, Write};

/// A sample image for demonstration (small test image in base64)
const SAMPLE_BASE64_IMAGE: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// Sample image URLs for demonstration
const SAMPLE_IMAGE_URLS: &[&str] = &[
    "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
    "https://upload.wikimedia.org/wikipedia/commons/thumb/5/50/Vd-Orig.png/256px-Vd-Orig.png",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenAI Ergonomic - Vision Chat Example");
    println!("======================================");
    println!();

    // Create client from environment variables
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!("✓ Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize client: {e}");
            eprintln!("Make sure OPENAI_API_KEY environment variable is set");
            return Err(e.into());
        }
    };

    println!("✓ Using vision-capable model for image understanding");
    println!();

    // Demonstrate various vision capabilities
    demonstrate_basic_image_analysis(&client).await?;
    demonstrate_multiple_images(&client).await?;
    demonstrate_detail_levels(&client).await?;
    demonstrate_base64_image(&client).await?;
    demonstrate_conversation_with_images(&client).await?;
    demonstrate_error_handling(&client).await?;

    println!("🎉 Vision chat example completed successfully!");
    println!("This example demonstrated:");
    println!("  • Basic image understanding with URLs");
    println!("  • Multiple image analysis in single messages");
    println!("  • Different detail levels (low, high, auto)");
    println!("  • Base64 image encoding and analysis");
    println!("  • Conversation context with images");
    println!("  • Comprehensive error handling");

    Ok(())
}

/// Demonstrate basic image analysis with a URL.
async fn demonstrate_basic_image_analysis(
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🖼️  Example 1: Basic Image Analysis");
    println!("----------------------------------");

    let image_url = SAMPLE_IMAGE_URLS[0];
    let question = "What do you see in this image? Please describe it in detail.";

    println!("Image URL: {image_url}");
    println!("Question: {question}");
    print!("Assistant: ");
    io::stdout().flush()?;

    // Use the convenient user_with_image_url method
    let chat_builder = client
        .chat()
        .system("You are a helpful AI assistant that can analyze images. Provide detailed, accurate descriptions of what you see.")
        .user_with_image_url(question, image_url)
        .temperature(0.3);

    let response = client.send_chat(chat_builder).await?;

    if let Some(content) = response.content() {
        println!("{content}");

        // Show usage information
        if let Some(usage) = response.usage() {
            println!("\n📊 Token usage:");
            println!("  Prompt tokens: {}", usage.prompt_tokens);
            println!("  Completion tokens: {}", usage.completion_tokens);
            println!("  Total tokens: {}", usage.total_tokens);
        }
    } else {
        println!("No response content received");
    }

    println!();
    Ok(())
}

/// Demonstrate analysis of multiple images in a single message.
async fn demonstrate_multiple_images(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🖼️🖼️ Example 2: Multiple Image Analysis");
    println!("---------------------------------------");

    let question = "Compare these two images. What are the differences and similarities?";

    println!("Question: {question}");
    println!("Image 1: {}", SAMPLE_IMAGE_URLS[0]);
    println!("Image 2: {}", SAMPLE_IMAGE_URLS[1]);
    print!("Assistant: ");
    io::stdout().flush()?;

    // Create message parts manually for multiple images
    let parts = vec![
        text_part(question),
        image_url_part_with_detail(SAMPLE_IMAGE_URLS[0], Detail::Auto),
        image_url_part_with_detail(SAMPLE_IMAGE_URLS[1], Detail::Auto),
    ];

    let chat_builder = client
        .chat()
        .system("You are an expert at comparing and analyzing images. Provide thoughtful comparisons focusing on visual elements, composition, and content.")
        .user_with_parts(parts)
        .temperature(0.4);

    let response = client.send_chat(chat_builder).await?;

    if let Some(content) = response.content() {
        println!("{content}");
    } else {
        println!("No response content received");
    }

    println!();
    Ok(())
}

/// Demonstrate different detail levels for image analysis.
async fn demonstrate_detail_levels(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Example 3: Different Detail Levels");
    println!("------------------------------------");

    let image_url = SAMPLE_IMAGE_URLS[0];
    let question = "Analyze this image";

    // Test different detail levels
    let detail_levels = vec![
        (Detail::Low, "Low detail (faster, less detailed)"),
        (Detail::High, "High detail (slower, more detailed)"),
        (Detail::Auto, "Auto detail (balanced)"),
    ];

    for (detail, description) in detail_levels {
        println!("\n{description}:");
        print!("Assistant: ");
        io::stdout().flush()?;

        let chat_builder = client
            .chat()
            .system("Analyze the image and describe what you see. Adjust your response detail based on the image quality provided.")
            .user_with_image_url_and_detail(question, image_url, detail)
            .temperature(0.2)
            .max_completion_tokens(100); // Limit response length for comparison

        let response = client.send_chat(chat_builder).await?;

        if let Some(content) = response.content() {
            println!("{content}");
        }
    }

    println!();
    Ok(())
}

/// Demonstrate base64 image encoding and analysis.
async fn demonstrate_base64_image(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔢 Example 4: Base64 Image Analysis");
    println!("-----------------------------------");

    let question = "What is this image? It's very small, what can you tell about it?";

    println!("Question: {question}");
    println!("Image: Small test image encoded as base64");
    print!("Assistant: ");
    io::stdout().flush()?;

    // Create message parts with base64 image
    let parts = vec![
        text_part(question),
        image_base64_part_with_detail(SAMPLE_BASE64_IMAGE, "image/png", Detail::High),
    ];

    let chat_builder = client
        .chat()
        .system("You are analyzing images provided in base64 format. Even if an image is very small or simple, try to provide what information you can.")
        .user_with_parts(parts)
        .temperature(0.3);

    let response = client.send_chat(chat_builder).await?;

    if let Some(content) = response.content() {
        println!("{content}");
    } else {
        println!("No response content received");
    }

    println!();
    Ok(())
}

/// Demonstrate conversation context with images.
async fn demonstrate_conversation_with_images(
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💬 Example 5: Conversation Context with Images");
    println!("----------------------------------------------");

    let image_url = SAMPLE_IMAGE_URLS[0];

    // First message: Analyze the image
    println!("Step 1: Initial image analysis");
    print!("Assistant: ");
    io::stdout().flush()?;

    let mut chat_builder = client
        .chat()
        .system("You are having a conversation about images. Remember details from previous messages to maintain context.")
        .user_with_image_url("What's the main subject of this image?", image_url)
        .temperature(0.3);

    let response1 = client.send_chat(chat_builder).await?;
    let first_response = response1.content().unwrap_or("No response").to_string();
    println!("{first_response}");

    // Second message: Follow-up question (without re-uploading the image)
    println!("\nStep 2: Follow-up question");
    print!("Assistant: ");
    io::stdout().flush()?;

    chat_builder = client
        .chat()
        .system("You are having a conversation about images. Remember details from previous messages to maintain context.")
        .user_with_image_url("What's the main subject of this image?", image_url)
        .assistant(&first_response)
        .user("What colors are most prominent in the image we just discussed?")
        .temperature(0.3);

    let response2 = client.send_chat(chat_builder).await?;

    if let Some(content) = response2.content() {
        println!("{content}");
    }

    // Third message: Ask for creative interpretation
    println!("\nStep 3: Creative interpretation");
    print!("Assistant: ");
    io::stdout().flush()?;

    let second_response = response2.content().unwrap_or("No response").to_string();

    chat_builder = client
        .chat()
        .system("You are having a conversation about images. Remember details from previous messages to maintain context.")
        .user_with_image_url("What's the main subject of this image?", image_url)
        .assistant(&first_response)
        .user("What colors are most prominent in the image we just discussed?")
        .assistant(second_response)
        .user("Based on our discussion, write a short poem inspired by this image.")
        .temperature(0.7);

    let response3 = client.send_chat(chat_builder).await?;

    if let Some(content) = response3.content() {
        println!("{content}");
    }

    println!();
    Ok(())
}

/// Demonstrate error handling patterns for vision requests.
async fn demonstrate_error_handling(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Example 6: Error Handling Patterns");
    println!("------------------------------------");

    println!("Testing various error scenarios...\n");

    // Test 1: Invalid image URL
    println!("Test 1: Invalid image URL");
    let invalid_url = "https://this-domain-does-not-exist-12345.com/image.jpg";

    let invalid_builder = client
        .chat()
        .user_with_image_url("What do you see?", invalid_url)
        .temperature(0.3);

    match client.send_chat(invalid_builder).await {
        Ok(_) => println!("✗ Invalid URL request unexpectedly succeeded"),
        Err(e) => match &e {
            Error::Api {
                status, message, ..
            } => {
                println!("✓ API properly rejected invalid URL ({status}): {message}");
            }
            Error::Http(reqwest_err) => {
                println!("✓ HTTP error caught: {reqwest_err}");
            }
            Error::InvalidRequest(msg) => {
                println!("✓ Validation caught invalid URL: {msg}");
            }
            _ => {
                println!("ℹ️  Other error type: {e}");
            }
        },
    }

    // Test 2: Empty message with image
    println!("\nTest 2: Empty text with image");
    let empty_text_builder = client
        .chat()
        .user_with_image_url("", SAMPLE_IMAGE_URLS[0])
        .temperature(0.3);

    match client.send_chat(empty_text_builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!(
                    "✓ API handled empty text gracefully: {}",
                    content.chars().take(50).collect::<String>()
                );
            }
        }
        Err(e) => {
            println!("ℹ️  Empty text error: {e}");
        }
    }

    // Test 3: Malformed base64 data
    println!("\nTest 3: Malformed base64 image data");
    let malformed_base64 = "this-is-not-valid-base64!@#$%";
    let malformed_parts = vec![
        text_part("What is this?"),
        image_base64_part_with_detail(malformed_base64, "image/png", Detail::Auto),
    ];

    let malformed_builder = client.chat().user_with_parts(malformed_parts);

    match client.send_chat(malformed_builder).await {
        Ok(_) => println!("✗ Malformed base64 unexpectedly succeeded"),
        Err(e) => match &e {
            Error::Api {
                status, message, ..
            } => {
                println!("✓ API properly rejected malformed base64 ({status}): {message}");
            }
            _ => {
                println!("ℹ️  Other error for malformed base64: {e}");
            }
        },
    }

    println!("\n🛡️  Error handling patterns demonstrated:");
    println!("  • Invalid image URL handling");
    println!("  • Empty text with image handling");
    println!("  • Malformed base64 data validation");
    println!("  • API error classification");
    println!("  • Network error handling");

    println!();
    Ok(())
}
