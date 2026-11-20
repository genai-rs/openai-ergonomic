#![allow(clippy::uninlined_format_args)]
//! Vision chat example demonstrating image understanding capabilities.
//!
//! This example demonstrates:
//! - Images sent as base64 data URLs, built from bytes (no third-party image host needed)
//! - Images by URL (set `VISION_IMAGE_URL` to a publicly fetchable image)
//! - Multiple image analysis in a single message
//! - Different detail levels (low, high, auto)
//! - Conversation context with images
//! - Comprehensive error handling
//!
//! The images are drawn by the example itself and sent inline, so it does not depend on an
//! image host: `OpenAI` downloads image URLs itself, and some hosts refuse it. To send a file
//! instead, read its bytes (`std::fs::read("photo.jpg")`) and encode them the same way.
//!
//! Run with: `cargo run --example vision_chat`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use base64::Engine as _;
use openai_ergonomic::{
    image_base64_part_with_detail, image_url_part_with_detail, text_part, Client, Detail, Error,
    Response,
};
use std::io::{self, Write};

/// Size of the generated images, in pixels.
const IMAGE_SIZE: u32 = 128;

/// A red disc on a blue background, as base64-encoded PNG bytes.
fn red_disc_png_base64() -> String {
    let center = f64::from(IMAGE_SIZE) / 2.0;
    let png = encode_png(IMAGE_SIZE, IMAGE_SIZE, |x, y| {
        let (dx, dy) = (f64::from(x) - center, f64::from(y) - center);
        if dx.hypot(dy) < 40.0 {
            [220, 30, 30]
        } else {
            [30, 60, 200]
        }
    });
    base64::engine::general_purpose::STANDARD.encode(png)
}

/// A yellow triangle on a black background, as base64-encoded PNG bytes.
fn yellow_triangle_png_base64() -> String {
    let png = encode_png(IMAGE_SIZE, IMAGE_SIZE, |x, y| {
        let half_width = (f64::from(y) - 20.0) / 2.0;
        if (20..108).contains(&y) && (f64::from(x) - 64.0).abs() <= half_width {
            [240, 200, 20]
        } else {
            [20, 20, 20]
        }
    });
    base64::engine::general_purpose::STANDARD.encode(png)
}

/// Encode an RGB image as a PNG (uncompressed deflate blocks), to keep the example free of an
/// image crate. Any PNG, JPEG, GIF or WebP bytes work the same way.
fn encode_png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        png.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let start = png.len();
        png.extend_from_slice(&kind);
        png.extend_from_slice(data);
        let crc = crc32(&png[start..]);
        png.extend_from_slice(&crc.to_be_bytes());
    }

    // Scanlines: a filter byte (0, none) and the RGB pixels.
    let mut raw = Vec::new();
    for y in 0..height {
        raw.push(0);
        for x in 0..width {
            raw.extend_from_slice(&pixel(x, y));
        }
    }

    // zlib stream of stored (uncompressed) deflate blocks, then the Adler-32 checksum.
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(i + 1 == blocks.len()));
        let len = u16::try_from(block.len()).unwrap();
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB, no interlace

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, *b"IHDR", &header);
    chunk(&mut png, *b"IDAT", &zlib);
    chunk(&mut png, *b"IEND", &[]);
    png
}

/// A `data:` URL for base64-encoded PNG bytes, for the `*_image_url` builder methods.
fn png_data_url(base64_png: &str) -> String {
    format!("data:image/png;base64,{base64_png}")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenAI Ergonomic - Vision Chat Example");
    println!("======================================");
    println!();

    // Create client from environment variables
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!(" Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!(" Failed to initialize client: {e}");
            eprintln!("Make sure OPENAI_API_KEY environment variable is set");
            return Err(e.into());
        }
    };

    println!(" Using vision-capable model for image understanding");
    println!();

    let red_disc = red_disc_png_base64();
    let yellow_triangle = yellow_triangle_png_base64();

    // Demonstrate various vision capabilities
    demonstrate_base64_image(&client, &red_disc).await?;
    demonstrate_multiple_images(&client, &red_disc, &yellow_triangle).await?;
    demonstrate_detail_levels(&client, &red_disc).await?;
    demonstrate_image_url(&client).await?;
    demonstrate_conversation_with_images(&client, &yellow_triangle).await?;
    demonstrate_error_handling(&client, &red_disc).await?;

    println!(" Vision chat example completed successfully!");
    println!("This example demonstrated:");
    println!("  • Images sent as base64 data URLs built from bytes");
    println!("  • Multiple image analysis in single messages");
    println!("  • Different detail levels (low, high, auto)");
    println!("  • Images by URL (with VISION_IMAGE_URL)");
    println!("  • Conversation context with images");
    println!("  • Comprehensive error handling");

    Ok(())
}

/// Demonstrate image analysis of base64-encoded bytes.
async fn demonstrate_base64_image(
    client: &Client,
    base64_png: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Example 1: Base64 Image Analysis");
    println!("-----------------------------------");

    let question = "What shape and colours do you see in this image?";

    println!("Image: a generated {IMAGE_SIZE}x{IMAGE_SIZE} PNG, sent as a base64 data URL");
    println!("Question: {question}");
    print!("Assistant: ");
    io::stdout().flush()?;

    // `image_base64_part_with_detail` wraps the base64 bytes in a `data:` URL
    let parts = vec![
        text_part(question),
        image_base64_part_with_detail(base64_png, "image/png", Detail::High),
    ];

    let chat_builder = client
        .chat()
        .system("You are a helpful AI assistant that can analyze images. Describe what you see accurately and briefly.")
        .user_with_parts(parts)
        .temperature(0.3);

    let response = client.send_chat(chat_builder).await?;

    if let Some(content) = response.content() {
        println!("{content}");

        // Show usage information
        if let Some(usage) = response.usage() {
            println!("\n Token usage:");
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
async fn demonstrate_multiple_images(
    client: &Client,
    first_png: &str,
    second_png: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(" Example 2: Multiple Image Analysis");
    println!("---------------------------------------");

    let question = "Compare these two images. What are the differences and similarities?";

    println!("Question: {question}");
    print!("Assistant: ");
    io::stdout().flush()?;

    // Create message parts manually for multiple images
    let parts = vec![
        text_part(question),
        image_base64_part_with_detail(first_png, "image/png", Detail::Auto),
        // The same, as a data URL for the URL-based helpers
        image_url_part_with_detail(png_data_url(second_png), Detail::Auto),
    ];

    let chat_builder = client
        .chat()
        .system("You are an expert at comparing and analyzing images. Provide brief comparisons focusing on shapes, colours and composition.")
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
async fn demonstrate_detail_levels(
    client: &Client,
    base64_png: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(" Example 3: Different Detail Levels");
    println!("------------------------------------");

    let image_url = png_data_url(base64_png);
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
            .user_with_image_url_and_detail(question, &image_url, detail)
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

/// Demonstrate an image by URL, which `OpenAI` downloads itself.
async fn demonstrate_image_url(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!(" Example 4: Image by URL");
    println!("-------------------------");

    // Image hosts may refuse `OpenAI`'s download (rate limits, user-agent policies), so the
    // URL is yours to choose.
    let Ok(image_url) = std::env::var("VISION_IMAGE_URL") else {
        println!("Skipped: set VISION_IMAGE_URL to a publicly fetchable image to try a URL.");
        println!();
        return Ok(());
    };
    let question = "What do you see in this image? Please describe it briefly.";

    println!("Image URL: {image_url}");
    println!("Question: {question}");
    print!("Assistant: ");
    io::stdout().flush()?;

    let chat_builder = client
        .chat()
        .system("You are a helpful AI assistant that can analyze images.")
        .user_with_image_url(question, &image_url)
        .temperature(0.3);

    match client.send_chat(chat_builder).await {
        Ok(response) => println!("{}", response.content().unwrap_or("No response content")),
        Err(e) => println!("\n The image could not be used (is it publicly fetchable?): {e}"),
    }

    println!();
    Ok(())
}

/// Demonstrate conversation context with images.
async fn demonstrate_conversation_with_images(
    client: &Client,
    base64_png: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(" Example 5: Conversation Context with Images");
    println!("----------------------------------------------");

    let image_url = png_data_url(base64_png);

    // First message: Analyze the image
    println!("Step 1: Initial image analysis");
    print!("Assistant: ");
    io::stdout().flush()?;

    let mut chat_builder = client
        .chat()
        .system("You are having a conversation about images. Remember details from previous messages to maintain context.")
        .user_with_image_url("What's the main subject of this image?", &image_url)
        .temperature(0.3);

    let response1 = client.send_chat(chat_builder).await?;
    let first_response = response1.content().unwrap_or("No response").to_string();
    println!("{first_response}");

    // Second message: Follow-up question (the history carries the image)
    println!("\nStep 2: Follow-up question");
    print!("Assistant: ");
    io::stdout().flush()?;

    chat_builder = client
        .chat()
        .system("You are having a conversation about images. Remember details from previous messages to maintain context.")
        .user_with_image_url("What's the main subject of this image?", &image_url)
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
        .user_with_image_url("What's the main subject of this image?", &image_url)
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
async fn demonstrate_error_handling(
    client: &Client,
    base64_png: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Example 6: Error Handling Patterns");
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
        Ok(_) => println!(" Invalid URL request unexpectedly succeeded"),
        Err(e) => match &e {
            Error::Api {
                status, message, ..
            } => {
                println!(" API properly rejected invalid URL ({status}): {message}");
            }
            Error::Http(reqwest_err) => {
                println!(" HTTP error caught: {reqwest_err}");
            }
            Error::InvalidRequest(msg) => {
                println!(" Validation caught invalid URL: {msg}");
            }
            _ => {
                println!("ℹ  Other error type: {e}");
            }
        },
    }

    // Test 2: Empty message with image
    println!("\nTest 2: Empty text with image");
    let empty_text_builder = client
        .chat()
        .user_with_image_url("", png_data_url(base64_png))
        .temperature(0.3);

    match client.send_chat(empty_text_builder).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!(
                    " API handled empty text gracefully: {}",
                    content.chars().take(50).collect::<String>()
                );
            }
        }
        Err(e) => {
            println!("ℹ  Empty text error: {e}");
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
        Ok(_) => println!(" Malformed base64 unexpectedly succeeded"),
        Err(e) => match &e {
            Error::Api {
                status, message, ..
            } => {
                println!(" API properly rejected malformed base64 ({status}): {message}");
            }
            _ => {
                println!("ℹ  Other error for malformed base64: {e}");
            }
        },
    }

    println!("\n  Error handling patterns demonstrated:");
    println!("  • Invalid image URL handling");
    println!("  • Empty text with image handling");
    println!("  • Malformed base64 data validation");
    println!("  • API error classification");
    println!("  • Network error handling");

    println!();
    Ok(())
}
