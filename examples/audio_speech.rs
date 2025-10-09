#![allow(clippy::uninlined_format_args)]
//! Audio Speech (Text-to-Speech) example for the openai-ergonomic crate.
//!
//! This example demonstrates text-to-speech functionality using `OpenAI`'s TTS models.
//! It shows how to generate audio from text with different voices, formats, and options.
//!
//! ## Features Demonstrated
//!
//! - Basic text-to-speech conversion
//! - Different voice options (alloy, echo, fable, onyx, nova, shimmer)
//! - Multiple audio formats (mp3, opus, aac, flac, wav, pcm)
//! - Speed control for generated audio
//! - Streaming audio output
//! - File handling for audio output
//! - Model selection (tts-1, tts-1-hd, gpt-4o-mini-tts)
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
//! cargo run --example audio_speech
//! ```

use openai_client_base::{
    apis::{audio_api, configuration::Configuration},
    models::{
        create_speech_request::{ResponseFormat, StreamFormat},
        CreateSpeechRequest,
    },
};
use openai_ergonomic::{Client, Error};
use std::io::Write;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔊 OpenAI Ergonomic - Audio Speech (Text-to-Speech) Example\n");

    // Initialize client from environment variables
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!("✅ Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {e}");
            eprintln!("💡 Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Example 1: Basic Text-to-Speech
    println!("\n🎙️ Example 1: Basic Text-to-Speech");
    println!("===================================");

    match basic_text_to_speech(&client).await {
        Ok(()) => println!("✅ Basic TTS example completed"),
        Err(e) => {
            eprintln!("❌ Basic TTS example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 2: Voice Comparison
    println!("\n🎭 Example 2: Voice Comparison");
    println!("===============================");

    match voice_comparison_example(&client).await {
        Ok(()) => println!("✅ Voice comparison example completed"),
        Err(e) => {
            eprintln!("❌ Voice comparison example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 3: Audio Format Options
    println!("\n🎵 Example 3: Audio Format Options");
    println!("===================================");

    match audio_format_example(&client).await {
        Ok(()) => println!("✅ Audio format example completed"),
        Err(e) => {
            eprintln!("❌ Audio format example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 4: Speed Control
    println!("\n⚡ Example 4: Speed Control");
    println!("===========================");

    match speed_control_example(&client).await {
        Ok(()) => println!("✅ Speed control example completed"),
        Err(e) => {
            eprintln!("❌ Speed control example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 5: Streaming Audio (Note: requires model support)
    println!("\n📡 Example 5: Streaming Audio");
    println!("==============================");

    match streaming_audio_example(&client).await {
        Ok(()) => println!("✅ Streaming audio example completed"),
        Err(e) => {
            eprintln!("❌ Streaming audio example failed: {e}");
            handle_api_error(&e);
        }
    }

    println!("\n🎉 All audio speech examples completed! Check the output files in the current directory.");
    Ok(())
}

/// Example 1: Basic text-to-speech with default settings
async fn basic_text_to_speech(client: &Client) -> Result<(), Error> {
    println!("Converting text to speech with default settings...");

    let text = "Hello! This is a demonstration of OpenAI's text-to-speech capabilities using the openai-ergonomic crate.";

    // Create speech request with basic settings
    let request = CreateSpeechRequest::builder()
        .model("tts-1".to_string())
        .input(text.to_string())
        .voice("alloy".to_string())
        .response_format(ResponseFormat::Mp3)
        .build();

    // Note: Once audio builders are implemented, this would look like:
    // let audio_response = client
    //     .audio()
    //     .speech()
    //     .model("tts-1")
    //     .input(text)
    //     .voice("alloy")
    //     .format("mp3")
    //     .execute()
    //     .await?;

    // For now, we'll use the base client directly
    let configuration = create_configuration(client);
    let response = audio_api::create_speech()
        .configuration(&configuration)
        .create_speech_request(request)
        .call()
        .await
        .map_err(|e| Error::Api {
            status: 0,
            message: e.to_string(),
            error_type: None,
            error_code: None,
        })?;

    // Save the audio data to file
    let audio_data = response.bytes().await.map_err(Error::Http)?;
    let filename = "basic_speech.mp3";
    save_audio_file(&audio_data, filename)?;

    println!("🎵 Generated speech saved to: {filename}");
    println!("   Text: \"{text}\"");
    println!("   Voice: alloy");
    println!("   Format: mp3");
    println!("   Size: {} bytes", audio_data.len());

    Ok(())
}

/// Example 2: Demonstrate different voice options
async fn voice_comparison_example(client: &Client) -> Result<(), Error> {
    println!("Generating speech with different voices...");

    let text = "The quick brown fox jumps over the lazy dog.";
    let voices = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];

    let configuration = create_configuration(client);

    for voice in &voices {
        println!("  🎤 Generating with voice: {voice}");

        let request = CreateSpeechRequest::builder()
            .model("tts-1".to_string())
            .input(text.to_string())
            .voice((*voice).to_string())
            .response_format(ResponseFormat::Mp3)
            .build();

        match audio_api::create_speech()
            .configuration(&configuration)
            .create_speech_request(request)
            .call()
            .await
        {
            Ok(response) => {
                let audio_data = response.bytes().await.map_err(Error::Http)?;
                let filename = format!("voice_{voice}.mp3");
                save_audio_file(&audio_data, &filename)?;
                println!("     ✅ Saved to: {filename} ({} bytes)", audio_data.len());
            }
            Err(e) => {
                eprintln!("     ❌ Failed to generate audio for voice {voice}: {e}");
            }
        }
    }

    println!("\n💡 Note: Listen to the generated files to compare different voice characteristics");

    Ok(())
}

/// Example 3: Demonstrate different audio formats
async fn audio_format_example(client: &Client) -> Result<(), Error> {
    println!("Generating speech in different audio formats...");

    let text = "This demonstrates various audio format options.";
    let formats = [
        (ResponseFormat::Mp3, "mp3"),
        (ResponseFormat::Opus, "opus"),
        (ResponseFormat::Aac, "aac"),
        (ResponseFormat::Flac, "flac"),
        (ResponseFormat::Wav, "wav"),
        (ResponseFormat::Pcm, "pcm"),
    ];

    let configuration = create_configuration(client);

    for (format, extension) in &formats {
        println!("  🎵 Generating in format: {extension}");

        let request = CreateSpeechRequest::builder()
            .model("tts-1".to_string())
            .input(text.to_string())
            .voice("nova".to_string())
            .response_format(*format)
            .build();

        match audio_api::create_speech()
            .configuration(&configuration)
            .create_speech_request(request)
            .call()
            .await
        {
            Ok(response) => {
                let audio_data = response.bytes().await.map_err(Error::Http)?;
                let filename = format!("format_example.{extension}");
                save_audio_file(&audio_data, &filename)?;
                println!("     ✅ Saved to: {filename} ({} bytes)", audio_data.len());
            }
            Err(e) => {
                eprintln!("     ❌ Failed to generate audio in format {extension}: {e}");
            }
        }
    }

    println!("\n💡 Note: Different formats have different quality/compression trade-offs:");
    println!("   - MP3: Good compression, widely supported");
    println!("   - OPUS: Excellent compression for voice, modern codec");
    println!("   - AAC: Good compression, Apple ecosystem friendly");
    println!("   - FLAC: Lossless compression, larger files");
    println!("   - WAV: Uncompressed, largest files, universal support");
    println!("   - PCM: Raw audio data, suitable for further processing");

    Ok(())
}

/// Example 4: Demonstrate speed control
async fn speed_control_example(client: &Client) -> Result<(), Error> {
    println!("Generating speech at different speeds...");

    let text = "This sentence will be spoken at different speeds to demonstrate the speed control feature.";
    let speeds = [0.25, 0.5, 1.0, 1.5, 2.0, 4.0];

    let configuration = create_configuration(client);

    for &speed in &speeds {
        println!("  ⚡ Generating at speed: {speed}x");

        let request = CreateSpeechRequest::builder()
            .model("tts-1".to_string())
            .input(text.to_string())
            .voice("echo".to_string())
            .response_format(ResponseFormat::Mp3)
            .speed(speed)
            .build();

        match audio_api::create_speech()
            .configuration(&configuration)
            .create_speech_request(request)
            .call()
            .await
        {
            Ok(response) => {
                let audio_data = response.bytes().await.map_err(Error::Http)?;
                let filename = format!("speed_{speed}.mp3");
                save_audio_file(&audio_data, &filename)?;
                println!("     ✅ Saved to: {filename} ({} bytes)", audio_data.len());
            }
            Err(e) => {
                eprintln!("     ❌ Failed to generate audio at speed {speed}x: {e}");
            }
        }
    }

    println!("\n💡 Note: Speed range is 0.25x to 4.0x normal speed");
    println!("   - 0.25x: Very slow, good for learning pronunciation");
    println!("   - 1.0x: Normal speed");
    println!("   - 4.0x: Very fast, good for quick content consumption");

    Ok(())
}

/// Example 5: Demonstrate streaming audio (where supported)
async fn streaming_audio_example(client: &Client) -> Result<(), Error> {
    println!("Attempting to generate streaming audio...");

    let text = "This is a longer text that demonstrates streaming audio capabilities. Streaming allows you to start playing audio before the entire generation is complete, which is useful for real-time applications and longer content.";

    let configuration = create_configuration(client);

    // Try with gpt-4o-mini-tts which supports streaming
    let request = CreateSpeechRequest::builder()
        .model("gpt-4o-mini-tts".to_string())
        .input(text.to_string())
        .voice("shimmer".to_string())
        .response_format(ResponseFormat::Mp3)
        .stream_format(StreamFormat::Audio)
        .build();

    println!("  📡 Attempting streaming generation...");

    match audio_api::create_speech()
        .configuration(&configuration)
        .create_speech_request(request.clone())
        .call()
        .await
    {
        Ok(response) => {
            let audio_data = response.bytes().await.map_err(Error::Http)?;
            let filename = "streaming_example.mp3";
            save_audio_file(&audio_data, filename)?;
            println!(
                "     ✅ Streaming audio saved to: {filename} ({} bytes)",
                audio_data.len()
            );

            println!("\n💡 Note: In a real streaming implementation, you would:");
            println!("   - Process audio chunks as they arrive");
            println!("   - Start playback before full generation is complete");
            println!("   - Handle streaming format appropriately");
        }
        Err(e) => {
            eprintln!("     ⚠️ Streaming with gpt-4o-mini-tts failed, trying fallback: {e}");

            // Fallback to regular generation
            let fallback_request = CreateSpeechRequest::builder()
                .model("tts-1-hd".to_string())
                .input(text.to_string())
                .voice("shimmer".to_string())
                .response_format(ResponseFormat::Mp3)
                .build();

            match audio_api::create_speech()
                .configuration(&configuration)
                .create_speech_request(fallback_request)
                .call()
                .await
            {
                Ok(response) => {
                    let audio_data = response.bytes().await.map_err(Error::Http)?;
                    let filename = "fallback_example.mp3";
                    save_audio_file(&audio_data, filename)?;
                    println!(
                        "     ✅ Fallback audio saved to: {filename} ({} bytes)",
                        audio_data.len()
                    );
                }
                Err(e) => {
                    eprintln!("     ❌ Fallback generation also failed: {e}");
                }
            }
        }
    }

    println!("\n💡 Note: Streaming support varies by model:");
    println!("   - gpt-4o-mini-tts: Supports streaming");
    println!("   - tts-1, tts-1-hd: No streaming support");
    println!("   - Stream formats: 'sse' (Server-Sent Events) or 'audio' (raw audio chunks)");

    Ok(())
}

/// Helper function to create configuration from client
fn create_configuration(client: &Client) -> Configuration {
    let mut configuration = Configuration::new();
    configuration.bearer_access_token = Some(client.config().api_key().to_string());

    if let Some(base_url) = client.config().base_url() {
        configuration.base_path = base_url.to_string();
    }

    if let Some(org_id) = client.config().organization_id() {
        configuration.user_agent = Some(format!(
            "openai-ergonomic/{} org/{}",
            env!("CARGO_PKG_VERSION"),
            org_id
        ));
    }

    configuration
}

/// Helper function to save audio data to file
fn save_audio_file(audio_data: &[u8], filename: &str) -> Result<(), Error> {
    let path = PathBuf::from(filename);
    let mut file = std::fs::File::create(&path).map_err(Error::File)?;
    file.write_all(audio_data).map_err(Error::File)?;
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
            eprintln!("🚫 API Error [{status}]: {message}");
            if let Some(error_type) = error_type {
                eprintln!("   Type: {error_type}");
            }
            if let Some(error_code) = error_code {
                eprintln!("   Code: {error_code}");
            }

            // Provide specific guidance based on error type
            match *status {
                401 => eprintln!("💡 Check your API key: export OPENAI_API_KEY=\"your-key\""),
                429 => eprintln!("💡 Rate limited - try again in a moment"),
                500..=599 => eprintln!("💡 Server error - try again later"),
                _ => {}
            }
        }
        Error::InvalidRequest(msg) => {
            eprintln!("🚫 Invalid Request: {msg}");
            eprintln!("💡 Check your request parameters");
        }
        Error::Config(msg) => {
            eprintln!("🚫 Configuration Error: {msg}");
            eprintln!("💡 Check your client configuration");
        }
        Error::Http(err) => {
            eprintln!("🚫 HTTP Error: {err}");
            eprintln!("💡 Check your network connection");
        }
        Error::Json(err) => {
            eprintln!("🚫 JSON Error: {err}");
            eprintln!("💡 Response parsing failed - may be a temporary issue");
        }
        Error::Authentication(msg) => {
            eprintln!("🚫 Authentication Error: {msg}");
            eprintln!("💡 Check your API key");
        }
        Error::RateLimit(msg) => {
            eprintln!("🚫 Rate Limit Error: {msg}");
            eprintln!("💡 Try again in a moment");
        }
        Error::Stream(msg) => {
            eprintln!("🚫 Stream Error: {msg}");
            eprintln!("💡 Connection issue with streaming");
        }
        Error::File(err) => {
            eprintln!("🚫 File Error: {err}");
            eprintln!("💡 Check file permissions and paths");
        }
        Error::Builder(msg) => {
            eprintln!("🚫 Builder Error: {msg}");
            eprintln!("💡 Check your request builder configuration");
        }
        Error::Internal(msg) => {
            eprintln!("🚫 Internal Error: {msg}");
            eprintln!("💡 This may be a bug, please report it");
        }
        Error::StreamConnection { message } => {
            eprintln!("🚫 Stream Connection Error: {message}");
            eprintln!("💡 Check your network connection");
        }
        Error::StreamParsing { message, chunk } => {
            eprintln!("🚫 Stream Parsing Error: {message}");
            eprintln!("   Problematic chunk: {chunk}");
            eprintln!("💡 The response stream may be corrupted");
        }
        Error::StreamBuffer { message } => {
            eprintln!("🚫 Stream Buffer Error: {message}");
            eprintln!("💡 The stream buffer encountered an issue");
        }
    }
}
