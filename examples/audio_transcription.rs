#![allow(clippy::uninlined_format_args)]
//! Audio Transcription (Speech-to-Text) example for the openai-ergonomic crate.
//!
//! This example demonstrates speech-to-text and translation functionality using `OpenAI`'s
//! Whisper models. It shows how to transcribe audio files into text with various options.
//!
//! ## Features Demonstrated
//!
//! - Basic speech-to-text transcription
//! - Audio translation to English
//! - Different response formats (json, text, srt, `verbose_json`, vtt)
//! - Timestamp extraction and segment information
//! - Language detection and specification
//! - Temperature control for transcription consistency
//! - Different audio input formats support
//! - Model selection (whisper-1, gpt-4o-mini-transcribe, gpt-4o-transcribe)
//! - Advanced features like word timestamps and log probabilities
//!
//! ## Prerequisites
//!
//! Set your `OpenAI` API key:
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! You'll also need some audio files to transcribe. This example includes
//! functionality to create sample audio files for testing if none are available.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example audio_transcription
//! ```

use openai_client_base::{
    apis::{audio_api, configuration::Configuration},
    models::{
        create_speech_request::ResponseFormat as SpeechResponseFormat,
        AudioResponseFormat,
        CreateSpeechRequest,
        // TranscriptionChunkingStrategy, TranscriptionChunkingStrategyTextVariantEnum,
        TranscriptionInclude,
    },
};
use openai_ergonomic::{Client, Error};
use std::io::Write;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" OpenAI Ergonomic - Audio Transcription (Speech-to-Text) Example\n");

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

    // First, create some sample audio files if they don't exist
    println!("\n Preparing sample audio files...");
    match create_sample_audio_files(&client).await {
        Ok(()) => println!(" Sample audio files ready"),
        Err(e) => {
            eprintln!(" Failed to create sample audio files: {e}");
            eprintln!(" You may need to provide your own audio files");
        }
    }

    // Example 1: Basic Speech-to-Text
    println!("\n Example 1: Basic Speech-to-Text Transcription");
    println!("===============================================");

    match basic_transcription_example(&client).await {
        Ok(()) => println!(" Basic transcription example completed"),
        Err(e) => {
            eprintln!(" Basic transcription example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 2: Response Format Comparison
    println!("\n Example 2: Response Format Comparison");
    println!("==========================================");

    match response_format_example(&client).await {
        Ok(()) => println!(" Response format example completed"),
        Err(e) => {
            eprintln!(" Response format example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 3: Detailed Transcription with Timestamps
    println!("\n⏰ Example 3: Detailed Transcription with Timestamps");
    println!("==================================================");

    match detailed_transcription_example(&client).await {
        Ok(()) => println!(" Detailed transcription example completed"),
        Err(e) => {
            eprintln!(" Detailed transcription example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 4: Audio Translation
    println!("\n Example 4: Audio Translation to English");
    println!("===========================================");

    match translation_example(&client).await {
        Ok(()) => println!(" Translation example completed"),
        Err(e) => {
            eprintln!(" Translation example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 5: Advanced Options
    println!("\n Example 5: Advanced Transcription Options");
    println!("============================================");

    match advanced_options_example(&client).await {
        Ok(()) => println!(" Advanced options example completed"),
        Err(e) => {
            eprintln!(" Advanced options example failed: {e}");
            handle_api_error(&e);
        }
    }

    println!("\n All audio transcription examples completed! Check the console output above for results.");
    Ok(())
}

/// Create sample audio files for testing if they don't exist
async fn create_sample_audio_files(client: &Client) -> Result<(), Error> {
    let sample_files = [
        ("sample_english.mp3", "Hello, this is a sample English audio for transcription testing. The quick brown fox jumps over the lazy dog."),
        ("sample_long.mp3", "This is a longer audio sample that will be used to demonstrate timestamp extraction and segmentation features. It contains multiple sentences with pauses between them. The purpose is to show how the transcription API can break down longer audio into meaningful segments with accurate timing information."),
        ("sample_numbers.mp3", "Here are some numbers for testing: one, two, three, four, five. The year is twenty twenty-four. My phone number is five five five, one two three four."),
    ];

    let configuration = create_configuration(client);

    for (filename, text) in &sample_files {
        let path = PathBuf::from(filename);
        if path.exists() {
            println!("   Sample audio already exists: {filename}");
        } else {
            println!("   Creating sample audio: {filename}");

            let request = CreateSpeechRequest::builder()
                .model("tts-1".to_string())
                .input((*text).to_string())
                .voice("alloy".to_string())
                .response_format(SpeechResponseFormat::Mp3)
                .speed(0.9) // Slightly slower for clearer transcription
                .build();

            match audio_api::create_speech()
                .configuration(&configuration)
                .create_speech_request(request)
                .call()
                .await
            {
                Ok(response) => {
                    let audio_data = response.bytes().await.map_err(Error::Http)?;
                    save_audio_file(&audio_data, filename)?;
                    println!("      Created: {filename}");
                }
                Err(e) => {
                    eprintln!("      Failed to create {filename}: {e}");
                }
            }
        }
    }

    Ok(())
}

/// Example 1: Basic speech-to-text transcription
async fn basic_transcription_example(client: &Client) -> Result<(), Error> {
    println!("Performing basic speech-to-text transcription...");

    let audio_file = PathBuf::from("sample_english.mp3");
    if !audio_file.exists() {
        eprintln!(" Audio file not found: {}", audio_file.display());
        eprintln!(" Run the audio creation step first or provide your own audio file");
        return Ok(());
    }

    // Note: Once audio builders are implemented, this would look like:
    // let transcription = client
    //     .audio()
    //     .transcription()
    //     .file(&audio_file)
    //     .model("whisper-1")
    //     .response_format("json")
    //     .execute()
    //     .await?;

    let configuration = create_configuration(client);

    println!("   Transcribing: {}", audio_file.display());

    match audio_api::create_transcription()
        .configuration(&configuration)
        .file(audio_file.clone())
        .model("whisper-1")
        .response_format(AudioResponseFormat::Json)
        .call()
        .await
    {
        Ok(response) => {
            println!("   Transcription Results:");
            println!("     Text: \"{}\"", response.text);
            println!("     Language: {}", response.language);
            println!("     Duration: {:.2} seconds", response.duration);

            if let Some(usage) = &response.usage {
                println!("     Usage: {} seconds", usage.seconds);
            }

            // Save transcription to file
            let output_file = "basic_transcription.txt";
            save_text_file(&response.text, output_file)?;
            println!("      Saved transcription to: {output_file}");
        }
        Err(e) => {
            eprintln!("      Transcription failed: {e}");
            return Err(Error::Api {
                status: 0,
                message: e.to_string(),
                error_type: None,
                error_code: None,
            });
        }
    }

    Ok(())
}

/// Example 2: Compare different response formats
async fn response_format_example(client: &Client) -> Result<(), Error> {
    println!("Comparing different response formats...");

    let audio_file = PathBuf::from("sample_english.mp3");
    if !audio_file.exists() {
        eprintln!(" Audio file not found: {}", audio_file.display());
        return Ok(());
    }

    let formats = [
        (
            AudioResponseFormat::Json,
            "json",
            "JSON format with metadata",
        ),
        (AudioResponseFormat::Text, "text", "Plain text only"),
        (AudioResponseFormat::Srt, "srt", "SubRip subtitle format"),
        (
            AudioResponseFormat::VerboseJson,
            "verbose_json",
            "JSON with detailed timing",
        ),
        (AudioResponseFormat::Vtt, "vtt", "WebVTT subtitle format"),
    ];

    let configuration = create_configuration(client);

    for (format, extension, description) in &formats {
        println!("   Testing format: {description}");

        match audio_api::create_transcription()
            .configuration(&configuration)
            .file(audio_file.clone())
            .model("whisper-1")
            .response_format(*format)
            .call()
            .await
        {
            Ok(response) => {
                let filename = format!("transcription_sample.{extension}");

                match format {
                    AudioResponseFormat::Text => {
                        // For text format, the response is just the text
                        save_text_file(&response.text, &filename)?;
                        println!("      Saved as: {filename}");
                    }
                    AudioResponseFormat::Json | AudioResponseFormat::VerboseJson => {
                        // For JSON formats, save the full structured response
                        let json_output = serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|_| response.text.clone());
                        save_text_file(&json_output, &filename)?;
                        println!("      Saved as: {filename}");

                        if *format == AudioResponseFormat::VerboseJson {
                            if let Some(segments) = &response.segments {
                                println!("      Found {} segments", segments.len());
                            }
                            if let Some(words) = &response.words {
                                println!("      Found {} words with timestamps", words.len());
                            }
                        }
                    }
                    AudioResponseFormat::Srt | AudioResponseFormat::Vtt => {
                        // For subtitle formats, the text contains the formatted output
                        save_text_file(&response.text, &filename)?;
                        println!("      Saved as: {filename}");
                    }
                }
            }
            Err(e) => {
                eprintln!("      Failed to transcribe in format {extension}: {e}");
            }
        }
    }

    println!("\n Note: Different formats serve different purposes:");
    println!("   - JSON: Basic transcription with metadata");
    println!("   - Text: Just the transcribed text, no metadata");
    println!("   - SRT: SubRip subtitle format for video");
    println!("   - Verbose JSON: Includes word-level timestamps and confidence");
    println!("   - VTT: WebVTT format for web-based subtitles");

    Ok(())
}

/// Example 3: Detailed transcription with timestamps and segments
async fn detailed_transcription_example(client: &Client) -> Result<(), Error> {
    println!("Performing detailed transcription with timestamps...");

    let audio_file = PathBuf::from("sample_long.mp3");
    if !audio_file.exists() {
        eprintln!(" Audio file not found: {}", audio_file.display());
        return Ok(());
    }

    let configuration = create_configuration(client);

    println!("   Transcribing with detailed timing information...");

    // Request detailed transcription with timestamps
    match audio_api::create_transcription()
        .configuration(&configuration)
        .file(audio_file.clone())
        .model("whisper-1")
        .response_format(AudioResponseFormat::VerboseJson)
        .timestamp_granularities(vec!["word".to_string(), "segment".to_string()])
        .include(vec![TranscriptionInclude::Logprobs])
        .temperature(0.0) // Low temperature for consistency
        .call()
        .await
    {
        Ok(response) => {
            println!("   Detailed Transcription Results:");
            println!("     Text: \"{}\"", response.text);
            println!("     Language: {}", response.language);
            println!("     Duration: {:.2} seconds", response.duration);

            // Display segment information
            if let Some(segments) = &response.segments {
                println!("\n   Segment Analysis ({} segments):", segments.len());
                for (i, segment) in segments.iter().enumerate() {
                    println!(
                        "     Segment {}: [{:.2}s - {:.2}s] \"{}\"",
                        i + 1,
                        segment.start,
                        segment.end,
                        segment.text
                    );
                    let avg_logprob = segment.avg_logprob;
                    if avg_logprob != 0.0 {
                        println!("       Confidence: {avg_logprob:.3}");
                    }
                }
            }

            // Display word-level timestamps
            if let Some(words) = &response.words {
                println!("\n   Word-level Timestamps (first 10 words):");
                for (i, word) in words.iter().take(10).enumerate() {
                    println!(
                        "     {}: [{:.2}s - {:.2}s] \"{}\"",
                        i + 1,
                        word.start,
                        word.end,
                        word.word
                    );
                }
                if words.len() > 10 {
                    println!("     ... and {} more words", words.len() - 10);
                }
            }

            // Save detailed results
            let json_output =
                serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.text.clone());
            save_text_file(&json_output, "detailed_transcription.json")?;
            println!("      Saved detailed results to: detailed_transcription.json");
        }
        Err(e) => {
            eprintln!("      Detailed transcription failed: {e}");
            return Err(Error::Api {
                status: 0,
                message: e.to_string(),
                error_type: None,
                error_code: None,
            });
        }
    }

    Ok(())
}

/// Example 4: Audio translation to English
async fn translation_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating audio translation to English...");

    // For this example, we'll use one of our existing audio files
    // In a real scenario, you might have audio in different languages
    let audio_file = PathBuf::from("sample_english.mp3");
    if !audio_file.exists() {
        eprintln!(" Audio file not found: {}", audio_file.display());
        return Ok(());
    }

    let configuration = create_configuration(client);

    println!("   Translating audio to English...");
    println!("     Note: This example uses English audio, but translation works with any language");

    match audio_api::create_translation()
        .configuration(&configuration)
        .file(audio_file.clone())
        .model("whisper-1")
        .response_format("json")
        .temperature(0.2)
        .call()
        .await
    {
        Ok(response) => {
            println!("   Translation Results:");
            println!("     Translated Text: \"{}\"", response.text);

            // Save translation
            save_text_file(&response.text, "translation_result.txt")?;
            println!("      Saved translation to: translation_result.txt");

            println!("\n Translation Notes:");
            println!("   - Translation always outputs English text regardless of input language");
            println!(
                "   - It's different from transcription which preserves the original language"
            );
            println!("   - Useful for creating English subtitles from foreign language audio");
            println!("   - Works with the same audio formats as transcription");
        }
        Err(e) => {
            eprintln!("      Translation failed: {e}");
            return Err(Error::Api {
                status: 0,
                message: e.to_string(),
                error_type: None,
                error_code: None,
            });
        }
    }

    Ok(())
}

/// Example 5: Advanced transcription options
async fn advanced_options_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating advanced transcription options...");

    let audio_file = PathBuf::from("sample_numbers.mp3");
    if !audio_file.exists() {
        eprintln!(" Audio file not found: {}", audio_file.display());
        return Ok(());
    }

    let configuration = create_configuration(client);

    // Example with language specification and prompt
    println!("   Advanced transcription with language and prompt...");

    let prompt = "This audio contains numbers and phone numbers. Please transcribe them accurately as digits where appropriate.";

    match audio_api::create_transcription()
        .configuration(&configuration)
        .file(audio_file.clone())
        .model("whisper-1")
        .language("en") // Specify language for better accuracy
        .prompt(prompt) // Provide context to improve accuracy
        .response_format(AudioResponseFormat::VerboseJson)
        .temperature(0.0) // Deterministic output
        // .chunking_strategy(TranscriptionChunkingStrategy::TextVariant(TranscriptionChunkingStrategyTextVariantEnum::Auto)) // Commented out due to type mismatch
        .include(vec![TranscriptionInclude::Logprobs])
        .call()
        .await
    {
        Ok(response) => {
            println!("   Advanced Transcription Results:");
            println!("     Text: \"{}\"", response.text);
            println!("     Language: {}", response.language);
            println!("     Duration: {:.2} seconds", response.duration);

            // Show confidence information if available
            if let Some(logprobs) = &response.logprobs {
                println!(
                    "     Log Probabilities: {} tokens with confidence scores",
                    logprobs.len()
                );
            }

            // Analyze segments for number detection
            if let Some(segments) = &response.segments {
                println!("\n   Number Detection Analysis:");
                for (i, segment) in segments.iter().enumerate() {
                    let contains_numbers = segment.text.chars().any(|c| c.is_ascii_digit());
                    if contains_numbers {
                        println!(
                            "     Segment {} (contains numbers): \"{}\"",
                            i + 1,
                            segment.text
                        );
                        let confidence = segment.avg_logprob;
                        if confidence != 0.0 {
                            println!("       Confidence: {confidence:.3}");
                        }
                    }
                }
            }

            // Save results
            let json_output =
                serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.text.clone());
            save_text_file(&json_output, "advanced_transcription.json")?;
            println!("      Saved advanced results to: advanced_transcription.json");
        }
        Err(e) => {
            eprintln!("      Advanced transcription failed: {e}");
            return Err(Error::Api {
                status: 0,
                message: e.to_string(),
                error_type: None,
                error_code: None,
            });
        }
    }

    println!("\n Advanced Options Summary:");
    println!("   - Language: Specify input language for better accuracy");
    println!("   - Prompt: Provide context to guide transcription style");
    println!("   - Temperature: Control randomness (0.0 = deterministic)");
    println!("   - Chunking Strategy: How to split long audio (auto/hierarchical)");
    println!("   - Include: Additional data like log probabilities");
    println!("   - Timestamp Granularities: Word-level or segment-level timing");

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

/// Helper function to save text data to file
fn save_text_file(text: &str, filename: &str) -> Result<(), Error> {
    let path = PathBuf::from(filename);
    let mut file = std::fs::File::create(&path).map_err(Error::File)?;
    file.write_all(text.as_bytes()).map_err(Error::File)?;
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
            eprintln!(" Check your request parameters and audio file format");
        }
        Error::Config(msg) => {
            eprintln!(" Configuration Error: {msg}");
            eprintln!(" Check your client configuration");
        }
        Error::Http(err) => {
            eprintln!(" HTTP Error: {err}");
            eprintln!(" Check your network connection");
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
            eprintln!(" Check file permissions and paths, ensure audio file exists");
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
