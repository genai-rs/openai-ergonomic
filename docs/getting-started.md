# Getting Started with openai-ergonomic

This guide will help you get up and running with the `openai-ergonomic` crate quickly.

## Installation

Add `openai-ergonomic` to your `Cargo.toml`:

```toml
[dependencies]
openai-ergonomic = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Prerequisites

### OpenAI API Key

You'll need an OpenAI API key to use this crate. You can get one from the [OpenAI Platform](https://platform.openai.com/api-keys).

Set your API key as an environment variable:

```bash
export OPENAI_API_KEY="your-api-key-here"
```

Or pass it directly when creating the client (see examples below).

### Rust Version

This crate requires Rust 1.82 or later.

## Basic Usage

### Creating a Client

The first step is to create an `OpenAIClient`:

```rust
use openai_ergonomic::OpenAIClient;

// Using environment variable OPENAI_API_KEY
let client = OpenAIClient::new().build();

// Or specify the API key directly
let client = OpenAIClient::new()
    .api_key("your-api-key-here")
    .build();

// With additional configuration
let client = OpenAIClient::new()
    .api_key("your-api-key-here")
    .base_url("https://api.openai.com/v1")
    .timeout(std::time::Duration::from_secs(30))
    .build();
```

### Your First Chat Completion

Here's a simple example that sends a message to GPT-4:

```rust
use openai_ergonomic::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let response = client
        .chat_completions()
        .model("gpt-4")
        .message("user", "What is the capital of France?")
        .send()
        .await?;

    println!("Response: {}", response.choices[0].message.content);
    Ok(())
}
```

### Working with Conversations

You can build up conversations by adding multiple messages:

```rust
use openai_ergonomic::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let response = client
        .chat_completions()
        .model("gpt-4")
        .message("system", "You are a helpful assistant that speaks like a pirate.")
        .message("user", "Hello!")
        .message("assistant", "Ahoy there, matey! How can I help ye today?")
        .message("user", "What's the weather like?")
        .temperature(0.7)
        .max_tokens(150)
        .send()
        .await?;

    println!("Pirate says: {}", response.choices[0].message.content);
    Ok(())
}
```

## Streaming Responses

For real-time response generation, use streaming:

```rust
use openai_ergonomic::OpenAIClient;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let mut stream = client
        .chat_completions()
        .model("gpt-4")
        .message("user", "Write a short story about a robot learning to paint")
        .stream()
        .await?;

    print!("Story: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.choices[0].delta.content {
            print!("{}", content);
            // Flush stdout to see text as it streams
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }
    println!(); // New line at the end
    Ok(())
}
```

## Error Handling

The crate provides structured error handling:

```rust
use openai_ergonomic::{OpenAIClient, OpenAIError};

#[tokio::main]
async fn main() {
    let client = OpenAIClient::new().build();

    match client
        .chat_completions()
        .model("gpt-4")
        .message("user", "Hello!")
        .send()
        .await
    {
        Ok(response) => {
            println!("Success: {}", response.choices[0].message.content);
        }
        Err(OpenAIError::Api { message, status }) => {
            eprintln!("API Error ({}): {}", status, message);
        }
        Err(OpenAIError::Network(err)) => {
            eprintln!("Network Error: {}", err);
        }
        Err(OpenAIError::Config { message }) => {
            eprintln!("Configuration Error: {}", message);
        }
        Err(err) => {
            eprintln!("Other Error: {}", err);
        }
    }
}
```

## Working with Different API Endpoints

### Text Embeddings

Generate embeddings for text:

```rust
use openai_ergonomic::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let response = client
        .embeddings()
        .model("text-embedding-3-small")
        .input("The quick brown fox jumps over the lazy dog")
        .send()
        .await?;

    println!("Embedding length: {}", response.data[0].embedding.len());
    Ok(())
}
```

### Image Generation

Create images with DALL-E:

```rust
use openai_ergonomic::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let response = client
        .images()
        .prompt("A serene lake surrounded by mountains at sunset")
        .model("dall-e-3")
        .size("1024x1024")
        .quality("standard")
        .send()
        .await?;

    println!("Generated image URL: {}", response.data[0].url);
    Ok(())
}
```

### Audio Transcription

Transcribe audio files:

```rust
use openai_ergonomic::OpenAIClient;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let response = client
        .audio_transcriptions()
        .file(Path::new("audio.mp3"))
        .model("whisper-1")
        .language("en")
        .send()
        .await?;

    println!("Transcription: {}", response.text);
    Ok(())
}
```

## Advanced Configuration

### Custom HTTP Client

You can provide your own HTTP client configuration:

```rust
use openai_ergonomic::OpenAIClient;
use std::time::Duration;

let client = OpenAIClient::new()
    .api_key("your-api-key")
    .timeout(Duration::from_secs(60))
    .max_retries(3)
    .build();
```

### Using with Proxies

Configure proxy settings:

```rust
use openai_ergonomic::OpenAIClient;

let client = OpenAIClient::new()
    .api_key("your-api-key")
    .proxy("http://proxy.example.com:8080")
    .build();
```

## Function Calling / Tools

Use function calling to extend ChatGPT's capabilities:

```rust
use openai_ergonomic::{OpenAIClient, Function, FunctionParameter};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new().build();

    let get_weather = Function::new("get_weather")
        .description("Get the current weather for a location")
        .parameter("location", FunctionParameter::string()
            .description("The city and state/country")
            .required())
        .parameter("unit", FunctionParameter::string()
            .description("Temperature unit")
            .enum_values(["celsius", "fahrenheit"])
            .default("celsius"));

    let response = client
        .chat_completions()
        .model("gpt-4")
        .message("user", "What's the weather like in Tokyo?")
        .function(get_weather)
        .send()
        .await?;

    if let Some(function_call) = &response.choices[0].message.function_call {
        println!("Function called: {}", function_call.name);
        println!("Arguments: {}", function_call.arguments);
    }

    Ok(())
}
```

## Testing

When writing tests, you can use mock servers:

```rust
#[cfg(test)]
mod tests {
    use openai_ergonomic::OpenAIClient;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_chat_completion() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "choices": [{
                        "message": {
                            "content": "Hello from mock!",
                            "role": "assistant"
                        }
                    }]
                })))
            .mount(&mock_server)
            .await;

        let client = OpenAIClient::new()
            .base_url(&mock_server.uri())
            .api_key("test-key")
            .build();

        let response = client
            .chat_completions()
            .model("gpt-4")
            .message("user", "Hello")
            .send()
            .await
            .unwrap();

        assert_eq!(response.choices[0].message.content, "Hello from mock!");
    }
}
```

## Best Practices

### 1. Error Handling
Always handle errors appropriately in production code:

```rust
use openai_ergonomic::{OpenAIClient, OpenAIError};

async fn safe_chat_completion(prompt: &str) -> Result<String, String> {
    let client = OpenAIClient::new().build();

    match client
        .chat_completions()
        .model("gpt-4")
        .message("user", prompt)
        .send()
        .await
    {
        Ok(response) => Ok(response.choices[0].message.content.clone()),
        Err(OpenAIError::Api { status: 429, .. }) => {
            Err("Rate limit exceeded. Please try again later.".to_string())
        }
        Err(OpenAIError::Api { status: 401, .. }) => {
            Err("Invalid API key.".to_string())
        }
        Err(err) => Err(format!("Request failed: {}", err)),
    }
}
```

### 2. Rate Limiting
Be mindful of OpenAI's rate limits:

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn with_backoff<F, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
{
    let mut delay = Duration::from_millis(100);

    for _ in 0..3 {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }

    operation().await // Final attempt
}
```

### 3. Resource Management
Reuse clients when possible:

```rust
use openai_ergonomic::OpenAIClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct ChatService {
    client: Arc<OpenAIClient>,
}

impl ChatService {
    pub fn new(api_key: &str) -> Self {
        let client = Arc::new(
            OpenAIClient::new()
                .api_key(api_key)
                .build()
        );

        Self { client }
    }

    pub async fn chat(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .chat_completions()
            .model("gpt-4")
            .message("user", message)
            .send()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}
```

## Next Steps

- Explore the [Examples](../examples/) directory for more detailed usage patterns
- Read the [Architecture Guide](architecture.md) to understand the crate design
- Check out the [API Documentation](https://docs.rs/openai-ergonomic) for complete reference
- See the [Contributing Guide](../CONTRIBUTING.md) if you'd like to contribute

## Getting Help

- Check the [API documentation](https://docs.rs/openai-ergonomic)
- Look at [examples](../examples/) for common patterns
- Open an issue on [GitHub](https://github.com/genai-rs/openai-ergonomic/issues) for bugs or questions
- Review [OpenAI's API documentation](https://platform.openai.com/docs/api-reference) for API details

Happy coding with `openai-ergonomic`!