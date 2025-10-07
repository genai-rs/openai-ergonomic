#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::use_self)]
#![allow(clippy::io_other_error)]
#![allow(async_fn_in_trait)]

//! # openai-ergonomic
//!
//! An ergonomic Rust wrapper for the `OpenAI` API, providing type-safe builder patterns
//! and async/await support for all `OpenAI` endpoints.
//!
//! ## Features
//!
//! - **Type-safe builders** - Use builder patterns with compile-time validation
//! - **Async/await support** - Built on tokio and reqwest for modern async Rust
//! - **Streaming responses** - First-class support for real-time streaming
//! - **Comprehensive coverage** - Support for all `OpenAI` API endpoints
//! - **Error handling** - Structured error types for robust applications
//! - **Testing support** - Mock-friendly design for unit testing
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client from environment variables
//!     let client = Client::from_env()?;
//!
//!     // Simple chat completion
//!     let response = client
//!         .chat_simple("Hello, how are you?")
//!         .await?;
//!
//!     println!("{}", response);
//!     Ok(())
//! }
//! ```
//!
//! ## Streaming Example
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, Config};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?;
//!
//!     // Stream chat completions
//!     let mut stream = client
//!         .chat()
//!         .user("Tell me a story")
//!         .stream()
//!         .await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         print!("{}", chunk?.content());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, Error};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::from_env().expect("API key required");
//!
//!     match client.chat_simple("Hello").await {
//!         Ok(response) => println!("{}", response),
//!         Err(Error::RateLimit { .. }) => {
//!             println!("Rate limited, please retry later");
//!         }
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust,ignore
//! use openai_ergonomic::{Client, Config};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::builder()
//!         .api_key("your-api-key")
//!         .organization_id("org-123")
//!         .timeout(Duration::from_secs(30))
//!         .max_retries(5)
//!         .build();
//!
//!     let client = Client::new(config)?;
//!     Ok(())
//! }
//! ```
//!
//! ## Testing with Mocks
//!
//! ```rust,ignore
//! #[cfg(test)]
//! mod tests {
//!     use openai_ergonomic::test_utils::MockOpenAIServer;
//!
//!     #[tokio::test]
//!     async fn test_chat_completion() {
//!         let mock = MockOpenAIServer::new();
//!         mock.mock_chat_completion("Hello!", "Hi there!");
//!
//!         let client = mock.client();
//!         let response = client.chat_simple("Hello!").await.unwrap();
//!         assert_eq!(response, "Hi there!");
//!     }
//! }
//! ```
//!
//! # Modules
//!
//! - [`builders`] - Builder pattern implementations for API requests
//! - [`responses`] - Response type wrappers with ergonomic helpers
//! - [`client`] - Main client for API interactions
//! - [`config`] - Configuration management
//! - [`errors`] - Error types and handling

// Re-export bon for builder macros
pub use bon;

// Core modules
pub mod builders;
pub mod client;
pub mod config;
pub mod errors;
pub mod interceptor;
pub mod responses;
pub mod semantic_conventions;

// Re-export commonly used types
pub use client::Client;
pub use config::{Config, ConfigBuilder};
pub use errors::{Error, Result};
pub use interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor, StreamChunkContext,
    StreamEndContext,
};

// Re-export specific builder and response types for convenience
// NOTE: We avoid wildcard re-exports to prevent naming conflicts between modules
pub use builders::audio::{
    SpeechBuilder, TimestampGranularity, TranscriptionBuilder, TranscriptionRequest,
    TranslationBuilder, TranslationRequest,
};
pub use builders::chat::{
    image_base64_part, image_base64_part_with_detail, image_url_part, image_url_part_with_detail,
    system_user, text_part, user_message,
};
pub use builders::embeddings::{EmbeddingInput, EmbeddingsBuilder};
pub use builders::images::{
    Background, ImageEditBuilder, ImageEditRequest, ImageGenerationBuilder, ImageInputFidelity,
    ImageInputFidelityTextVariantEnum, ImageVariationBuilder, ImageVariationRequest, Moderation,
    OutputFormat, Quality, ResponseFormat, Size, Style,
};
pub use builders::threads::{
    AttachmentTool, MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};
pub use builders::uploads::UploadBuilder;
// Re-export vision types for convenience
pub use builders::responses::{responses_simple, responses_system_user, ResponsesBuilder};
pub use builders::{Builder, ChatCompletionBuilder, Sendable};
pub use openai_client_base::models::chat_completion_request_message_content_part_image_image_url::Detail;
pub use openai_client_base::models::create_upload_request::Purpose as UploadPurpose;
pub use responses::chat::{
    ChatChoice, ChatCompletionResponse, ChatCompletionResponseExt,
    ChatMessage as ResponseChatMessage, FunctionCall, ToolCall, ToolCallExt,
};
pub use responses::{tool_function, tool_web_search, ChatCompletionResponseWrapper};
pub use responses::{Response, Tool, ToolChoice, Usage};

// Test utilities (feature-gated)
#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::builder().api_key("test-key").build();
        assert_eq!(config.api_key(), "test-key");
    }

    #[test]
    fn test_client_creation_with_config() {
        let config = Config::builder().api_key("test-key").build();
        let client = Client::new(config);
        assert!(client.is_ok());
    }
}
