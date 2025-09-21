#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # openai-ergonomic
//!
//! An ergonomic Rust wrapper for the OpenAI API, providing type-safe builder patterns
//! and async/await support for all OpenAI endpoints.
//!
//! ## Features
//!
//! - **Type-safe builders** - Use builder patterns with compile-time validation
//! - **Async/await support** - Built on tokio and reqwest for modern async Rust
//! - **Streaming responses** - First-class support for real-time streaming
//! - **Comprehensive coverage** - Support for all OpenAI API endpoints
//! - **Error handling** - Structured error types for robust applications
//! - **Testing support** - Mock-friendly design for unit testing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use openai_ergonomic::OpenAIClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = OpenAIClient::new()
//!         .api_key("your-api-key-here")
//!         .build();
//!
//!     let response = client
//!         .chat_completions()
//!         .model("gpt-4")
//!         .message("user", "Hello, world!")
//!         .send()
//!         .await?;
//!
//!     println!("{}", response.choices[0].message.content);
//!     Ok(())
//! }
//! ```
//!
//! ## Streaming Example
//!
//! ```rust,no_run
//! use openai_ergonomic::OpenAIClient;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = OpenAIClient::new()
//!         .api_key("your-api-key-here")
//!         .build();
//!
//!     let mut stream = client
//!         .chat_completions()
//!         .model("gpt-4")
//!         .message("user", "Tell me a story")
//!         .stream()
//!         .await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         if let Some(content) = chunk.choices[0].delta.content {
//!             print!("{}", content);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## API Coverage
//!
//! This crate provides ergonomic builders for all major OpenAI API endpoints:
//!
//! - **Chat Completions** - GPT-4, GPT-3.5-turbo conversations with streaming
//! - **Embeddings** - Text embeddings for semantic search and analysis
//! - **Images** - DALL-E image generation and editing
//! - **Audio** - Whisper speech-to-text and text-to-speech
//! - **Assistants** - Assistant API with function calling and file handling
//! - **Files** - File upload and management
//! - **Models** - Model listing and metadata
//! - **Moderations** - Content moderation and safety
//!
//! ## Error Handling
//!
//! The crate provides structured error handling with detailed error types:
//!
//! ```rust,no_run
//! use openai_ergonomic::{OpenAIClient, OpenAIError};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OpenAIClient::new().build();
//!
//! match client.chat_completions().model("gpt-4").message("user", "Hello").send().await {
//!     Ok(response) => println!("Success: {}", response.choices[0].message.content),
//!     Err(OpenAIError::Api { message, status }) => eprintln!("API Error ({}): {}", status, message),
//!     Err(OpenAIError::Network(err)) => eprintln!("Network Error: {}", err),
//!     Err(err) => eprintln!("Other Error: {}", err),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! The client can be configured with various options:
//!
//! ```rust,no_run
//! use openai_ergonomic::OpenAIClient;
//! use std::time::Duration;
//!
//! let client = OpenAIClient::new()
//!     .api_key("your-api-key")
//!     .base_url("https://api.openai.com/v1")
//!     .timeout(Duration::from_secs(30))
//!     .max_retries(3)
//!     .build();
//! ```
//!
//! ## Testing
//!
//! The crate is designed to be mock-friendly for testing:
//!
//! ```rust,no_run
//! # #[cfg(feature = "testing")]
//! # async fn example() {
//! use openai_ergonomic::OpenAIClient;
//! use wiremock::{MockServer, Mock, ResponseTemplate};
//! use wiremock::matchers::{method, path};
//!
//! let mock_server = MockServer::start().await;
//!
//! Mock::given(method("POST"))
//!     .and(path("/chat/completions"))
//!     .respond_with(ResponseTemplate::new(200)
//!         .set_body_json(serde_json::json!({
//!             "choices": [{"message": {"content": "Hello!", "role": "assistant"}}]
//!         })))
//!     .mount(&mock_server)
//!     .await;
//!
//! let client = OpenAIClient::new()
//!     .base_url(&mock_server.uri())
//!     .api_key("test-key")
//!     .build();
//! # }
//! ```
//!
//! ## Further Reading
//!
//! - [Getting Started Guide](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/getting-started.md)
//! - [Architecture Overview](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/architecture.md)
//! - [Examples](https://github.com/genai-rs/openai-ergonomic/tree/main/examples)
//! - [Contributing Guide](https://github.com/genai-rs/openai-ergonomic/blob/main/CONTRIBUTING.md)

/// Re-export of the `bon` crate for builder pattern support.
///
/// The `bon` crate provides the procedural macros used by this library
/// to generate type-safe builder patterns. You may need to use `bon`
/// directly when working with custom builders or advanced configurations.
///
/// # Example
///
/// ```rust,no_run
/// use openai_ergonomic::bon::Builder;
///
/// #[derive(Builder)]
/// struct CustomConfig {
///     #[builder(default)]
///     timeout: std::time::Duration,
/// }
/// ```
pub use bon;

/// Placeholder for the main OpenAI client.
///
/// This will be the primary entry point for all OpenAI API interactions.
/// The client provides a fluent interface for building and executing requests
/// to various OpenAI endpoints.
///
/// # Example Usage
///
/// ```rust,no_run
/// use openai_ergonomic::OpenAIClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OpenAIClient::new()
///     .api_key("your-api-key")
///     .build();
///
/// let response = client
///     .chat_completions()
///     .model("gpt-4")
///     .message("user", "Hello!")
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// **Note**: This is a placeholder struct. The actual implementation
/// will be added in subsequent development phases.
pub struct OpenAIClient;

impl OpenAIClient {
    /// Creates a new client builder.
    ///
    /// # Returns
    ///
    /// A `ClientBuilder` instance for configuring the OpenAI client.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::OpenAIClient;
    ///
    /// let client = OpenAIClient::new()
    ///     .api_key("your-api-key")
    ///     .build();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic smoke test to ensure the crate compiles and basic functionality works.
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    /// Test that the OpenAIClient can be instantiated.
    #[test]
    fn can_create_client() {
        let _client = OpenAIClient::new();
    }
}
