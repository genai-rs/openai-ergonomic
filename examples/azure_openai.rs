//! Azure `OpenAI` Integration Example
//!
//! This example demonstrates how to use the openai-ergonomic library with Azure `OpenAI`.
//!
//! # Azure `OpenAI` Configuration
//!
//! Azure `OpenAI` requires different configuration than standard `OpenAI`:
//! 1. An Azure `OpenAI` endpoint (e.g., `<https://my-resource.openai.azure.com>`)
//! 2. An API key from your Azure `OpenAI` resource
//! 3. A deployment name (not a model name)
//! 4. An API version (defaults to 2024-02-01)
//!
//! # Setup
//!
//! ## Option 1: Environment Variables
//!
//! ```bash
//! export AZURE_OPENAI_ENDPOINT="https://my-resource.openai.azure.com"
//! export AZURE_OPENAI_API_KEY="your-azure-api-key"
//! export AZURE_OPENAI_DEPLOYMENT="gpt-4"
//! export AZURE_OPENAI_API_VERSION="2024-02-01"  # Optional, defaults to 2024-02-01
//! ```
//!
//! ## Option 2: Manual Configuration
//!
//! See the examples below for programmatic configuration.
//!
//! # Run the Example
//!
//! ```bash
//! cargo run --example azure_openai
//! ```

use openai_ergonomic::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Azure OpenAI Integration Example");
    println!("=================================\n");

    // Example 1: Using environment variables
    println!("Example 1: Using environment variables");
    match Client::from_env() {
        Ok(client) => {
            let client = client.build();
            println!("Client created from environment variables");

            // Make a simple chat request
            let builder = client.chat_simple("Hello from Azure OpenAI!");
            match client.send_chat(builder).await {
                Ok(response) => {
                    if let Some(content) = response.content() {
                        println!("Response: {content}");
                    }
                }
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
        Err(e) => {
            println!("Could not create client from environment: {e}");
            println!("Make sure to set AZURE_OPENAI_ENDPOINT, AZURE_OPENAI_API_KEY, and AZURE_OPENAI_DEPLOYMENT");
        }
    }

    println!("\n---\n");

    // Example 2: Manual configuration
    println!("Example 2: Manual configuration");

    // This example shows how to configure Azure `OpenAI` programmatically.
    // Replace these values with your actual Azure `OpenAI` resource details.
    let config = Config::builder()
        .api_key("your-azure-api-key")
        .api_base("https://my-resource.openai.azure.com")
        .azure_deployment("gpt-4")
        .azure_api_version("2024-02-01")
        .build();

    println!("Config: {config:?}");
    println!("Is Azure: {}", config.is_azure());

    // Note: This will fail unless you provide valid credentials above
    // Uncomment the following to test with your actual credentials:
    /*
    let client = Client::builder(config)?.build();

    // Simple chat completion
    let response = client
        .chat_simple("Tell me a short joke about Azure")
        .await?;
    println!("Response: {}", response);

    // More advanced chat with custom parameters
    let response = client
        .chat()
        .user("What are the main features of Azure OpenAI?")
        .temperature(0.7)
        .max_tokens(500)
        .send()
        .await?;

    println!("\nAdvanced response:");
    println!("{}", response.content());

    // Streaming example
    use futures::StreamExt;

    println!("\nStreaming example:");
    let mut stream = client
        .chat()
        .user("Count from 1 to 5")
        .stream()
        .await?;

    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?.content());
    }
    println!();
    */

    println!("\n---\n");

    // Example 3: Key differences between `OpenAI` and Azure `OpenAI`
    println!("Example 3: Key differences between OpenAI and Azure OpenAI");
    println!("\nOpenAI:");
    println!("  - Endpoint: https://api.openai.com/v1");
    println!("  - Authentication: Bearer token in Authorization header");
    println!("  - Model specification: Use model names like 'gpt-4', 'gpt-3.5-turbo'");
    println!("  - Example: client.chat().model('gpt-4').send().await?\n");

    println!("Azure OpenAI:");
    println!("  - Endpoint: https://{{{{resource-name}}}}.openai.azure.com");
    println!("  - Authentication: api-key header");
    println!("  - Deployment specification: Use your deployment name");
    println!("  - API version required as query parameter");
    println!("  - Example: Configure deployment in Config, then use client normally\n");

    println!("With this library, you only need to configure the endpoint and deployment,");
    println!("and the library handles all the differences automatically!");

    Ok(())
}
