//! # `OpenAI` Ergonomic Quickstart Example
//!
//! This example demonstrates basic usage of the openai-ergonomic crate.
//! It's a placeholder example for testing the examples infrastructure.
//!
//! Run with: `cargo run --example quickstart`

use openai_ergonomic::bon;

/// A simple example struct using the bon builder pattern
#[derive(bon::Builder, Debug)]
pub struct ExampleRequest {
    /// The model to use for the request
    pub model: String,
    /// The input text to process
    pub input: String,
    /// Optional temperature parameter
    #[builder(default = 0.7)]
    pub temperature: f32,
    /// Optional max tokens parameter
    #[builder(default = 100)]
    pub max_tokens: u32,
}

// Note: The bon::Builder derive automatically provides a builder() method

/// Example client for demonstration purposes
#[derive(Debug)]
pub struct ExampleClient {
    api_key: String,
    base_url: String,
}

impl ExampleClient {
    /// Create a new example client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Simulate processing a request
    pub async fn process_request(
        &self,
        request: &ExampleRequest,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // This is a placeholder implementation
        println!("Processing request: {request:?}");
        println!("Using API key: {}...", &self.api_key[..8]);
        println!("Base URL: {}", self.base_url);

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(format!(
            "Processed '{}' with model '{}' (temp: {}, max_tokens: {})",
            request.input, request.model, request.temperature, request.max_tokens
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 OpenAI Ergonomic Quickstart Example");
    println!("======================================");

    // Initialize the client (in real usage, get API key from environment)
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "sk-example-key-for-testing".to_string());

    let client = ExampleClient::new(api_key);

    // Example 1: Basic request using builder pattern
    println!("\n📋 Example 1: Basic Request");
    let request = ExampleRequest::builder()
        .model("gpt-4".to_string())
        .input("Hello, world!".to_string())
        .build();

    let response = client.process_request(&request).await?;
    println!("Response: {response}");

    // Example 2: Request with custom parameters
    println!("\n⚙️  Example 2: Custom Parameters");
    let custom_request = ExampleRequest::builder()
        .model("gpt-3.5-turbo".to_string())
        .input("Explain quantum computing in simple terms".to_string())
        .temperature(0.9)
        .max_tokens(200)
        .build();

    let custom_response = client.process_request(&custom_request).await?;
    println!("Response: {custom_response}");

    // Example 3: Default parameters
    println!("\n🔧 Example 3: Using Defaults");
    let default_request = ExampleRequest::builder()
        .model("gpt-4".to_string())
        .input("What is the weather like?".to_string())
        .build(); // Using default temperature and max_tokens

    let default_response = client.process_request(&default_request).await?;
    println!("Response: {default_response}");

    println!("\n✅ Quickstart example completed successfully!");
    println!("   This example demonstrates the ergonomic builder pattern");
    println!("   that will be used throughout the openai-ergonomic crate.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_request_builder() {
        let request = ExampleRequest::builder()
            .model("gpt-4".to_string())
            .input("test input".to_string())
            .build();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.input, "test input");
        assert_eq!(request.temperature, 0.7); // default value
        assert_eq!(request.max_tokens, 100); // default value
    }

    #[test]
    fn test_example_request_with_custom_values() {
        let request = ExampleRequest::builder()
            .model("gpt-3.5-turbo".to_string())
            .input("custom input".to_string())
            .temperature(0.9)
            .max_tokens(200)
            .build();

        assert_eq!(request.temperature, 0.9);
        assert_eq!(request.max_tokens, 200);
    }

    #[tokio::test]
    async fn test_example_client() {
        let client = ExampleClient::new("test-key".to_string());
        let request = ExampleRequest::builder()
            .model("gpt-4".to_string())
            .input("test".to_string())
            .build();

        let result = client.process_request(&request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.contains("test"));
        assert!(response.contains("gpt-4"));
    }
}
