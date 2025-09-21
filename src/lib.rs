#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Ergonomic Rust wrapper for the OpenAI API.
//!
//! This crate provides a type-safe, builder-pattern interface to interact with
//! OpenAI API endpoints, making it easy to integrate AI capabilities into
//! your Rust applications.
//!
//! # Quick Start
//!
//! ```rust
//! # use openai_ergonomic::{Client, Config};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client from environment variables
//! let client = Client::from_env()?;
//!
//! // TODO: Add usage examples once implementations are complete
//! # Ok(())
//! # }
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
pub mod responses;

// Re-export commonly used types
pub use client::Client;
pub use config::{Config, ConfigBuilder};
pub use errors::{Error, Result};

// Re-export builder types for convenience
pub use builders::*;

// Re-export response types for convenience
pub use responses::*;

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
