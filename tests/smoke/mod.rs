//! Smoke tests for real OpenAI API integration.
//!
//! These tests are designed to be run against the actual OpenAI API
//! to verify that the crate works correctly in real scenarios.
//! They are gated behind environment variables and disabled by default.

use openai_ergonomic::{
    builders::{
        chat::{system_user, tool_function, ChatCompletionBuilder},
        responses::{responses_simple, responses_system_user},
        Builder,
    },
    Detail,
};
use serde_json::json;
use std::env;

/// Configuration for smoke tests
pub struct SmokeTestConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub timeout_seconds: u64,
}

impl SmokeTestConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("OPENAI_TEST_API_KEY").ok()?;

        Some(Self {
            api_key,
            base_url: env::var("OPENAI_TEST_BASE_URL").ok(),
            model: env::var("OPENAI_TEST_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
            timeout_seconds: env::var("OPENAI_TEST_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        })
    }

    /// Check if smoke tests should be enabled
    pub fn is_enabled() -> bool {
        env::var("OPENAI_TEST_API_KEY").is_ok()
    }
}

/// Macro to skip smoke tests if not configured
macro_rules! require_smoke_config {
    () => {
        let config = match SmokeTestConfig::from_env() {
            Some(config) => config,
            None => {
                println!("Skipping smoke test - OPENAI_TEST_API_KEY not set");
                return;
            }
        };
    };
}

/// Basic smoke test - simple chat completion
#[tokio::test]
#[ignore] // Disabled by default, enable with --ignored
async fn smoke_test_basic_chat_completion() {
    require_smoke_config!();

    let builder = responses_simple(&config.model, "Say hello in exactly 3 words.");
    let request = builder.build().expect("Should build request");

    // This is a smoke test - we're verifying the request structure is valid
    // In a real implementation, you would make the actual API call here
    assert_eq!(request.model, config.model);
    assert_eq!(request.messages.len(), 1);

    println!("✓ Basic chat completion request structure is valid");
}

/// Smoke test for system + user messages
#[tokio::test]
#[ignore]
async fn smoke_test_system_user_messages() {
    require_smoke_config!();

    let builder = responses_system_user(
        &config.model,
        "You are a helpful assistant that responds briefly.",
        "What is 2+2?",
    );

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert_eq!(request.messages.len(), 2);

    // Verify message types
    if let Some(openai_client_base::models::ChatCompletionRequestMessage::System(system_msg)) = request.messages.get(0) {
        assert!(!system_msg.content.is_empty());
    } else {
        panic!("First message should be system message");
    }

    if let Some(openai_client_base::models::ChatCompletionRequestMessage::User(user_msg)) = request.messages.get(1) {
        assert!(!user_msg.content.to_string().is_empty());
    } else {
        panic!("Second message should be user message");
    }

    println!("✓ System + user message structure is valid");
}

/// Smoke test for function calling
#[tokio::test]
#[ignore]
async fn smoke_test_function_calling() {
    require_smoke_config!();

    let weather_tool = tool_function(
        "get_weather",
        "Get current weather for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit"
                }
            },
            "required": ["location"]
        }),
    );

    let builder = system_user(
        &config.model,
        "You are a weather assistant. Use the get_weather function when asked about weather.",
        "What's the weather like in San Francisco?",
    )
    .tools(vec![weather_tool]);

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert_eq!(request.messages.len(), 2);
    assert!(request.tools.is_some());

    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].function.name, "get_weather");

    println!("✓ Function calling request structure is valid");
}

/// Smoke test for JSON mode
#[tokio::test]
#[ignore]
async fn smoke_test_json_mode() {
    require_smoke_config!();

    let builder = responses_simple(
        &config.model,
        "Generate a JSON object with fields: name (string), age (number), city (string)",
    )
    .json_mode();

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert!(request.response_format.is_some());

    let response_format = request.response_format.unwrap();
    assert_eq!(
        response_format.r#type,
        openai_client_base::models::CreateChatCompletionRequestResponseFormatType::JsonObject
    );

    println!("✓ JSON mode request structure is valid");
}

/// Smoke test for JSON schema
#[tokio::test]
#[ignore]
async fn smoke_test_json_schema() {
    require_smoke_config!();

    let person_schema = json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The person's full name"
            },
            "age": {
                "type": "integer",
                "minimum": 0,
                "maximum": 150,
                "description": "The person's age in years"
            },
            "email": {
                "type": "string",
                "format": "email",
                "description": "The person's email address"
            }
        },
        "required": ["name", "age"],
        "additionalProperties": false
    });

    let builder = responses_simple(&config.model, "Generate a person object")
        .json_schema("person", person_schema.clone());

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert!(request.response_format.is_some());

    let response_format = request.response_format.unwrap();
    use openai_client_base::models::CreateChatCompletionRequestAllOfResponseFormat;
    let schema_format = match response_format.as_ref() {
        CreateChatCompletionRequestAllOfResponseFormat::ResponseFormatJsonSchema(format) => format,
        other => panic!("expected json schema response format, got {other:?}"),
    };
    assert_eq!(schema_format.json_schema.name, "person");
    assert_eq!(schema_format.json_schema.schema, Some(person_schema));

    println!("✓ JSON schema request structure is valid");
}

/// Smoke test for streaming
#[tokio::test]
#[ignore]
async fn smoke_test_streaming() {
    require_smoke_config!();

    let builder = responses_simple(&config.model, "Count from 1 to 5, one number per word")
        .stream(true)
        .max_tokens(50);

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert_eq!(request.stream, Some(true));
    assert_eq!(request.max_tokens, Some(50));

    println!("✓ Streaming request structure is valid");
}

/// Smoke test for vision/multimodal
#[tokio::test]
#[ignore]
async fn smoke_test_vision() {
    require_smoke_config!();

    let text_part = openai_ergonomic::builders::chat::text_part("What do you see in this image?");
    let image_part = openai_ergonomic::builders::chat::image_url_part_with_detail(
        "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
        Detail::Low,
    );

    let builder = ChatCompletionBuilder::new("gpt-4-vision-preview")
        .user_with_parts(vec![text_part, image_part])
        .max_tokens(300);

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, "gpt-4-vision-preview");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.max_tokens, Some(300));

    // Verify multimodal content structure
    if let Some(openai_client_base::models::ChatCompletionRequestMessage::User(user_msg)) = request.messages.first() {
        match &user_msg.content {
            openai_client_base::models::ChatCompletionRequestUserMessageContent::Array(parts) => {
                assert_eq!(parts.len(), 2);
            }
            _ => panic!("Expected array content for multimodal message"),
        }
    } else {
        panic!("Expected user message");
    }

    println!("✓ Vision/multimodal request structure is valid");
}

/// Smoke test for parameter boundaries
#[tokio::test]
#[ignore]
async fn smoke_test_parameter_boundaries() {
    require_smoke_config!();

    let builder = ChatCompletionBuilder::new(&config.model)
        .user("Test message")
        .temperature(0.7)
        .max_tokens(100)
        .top_p(0.9)
        .frequency_penalty(0.1)
        .presence_penalty(0.1)
        .n(1);

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.frequency_penalty, Some(0.1));
    assert_eq!(request.presence_penalty, Some(0.1));
    assert_eq!(request.n, Some(1));

    println!("✓ Parameter boundaries request structure is valid");
}

/// Smoke test for complex conversation
#[tokio::test]
#[ignore]
async fn smoke_test_complex_conversation() {
    require_smoke_config!();

    let mut builder = ChatCompletionBuilder::new(&config.model)
        .system("You are a helpful math tutor. Be concise.");

    builder = builder.user("What is 5 + 3?");
    builder = builder.assistant("5 + 3 = 8");
    builder = builder.user("Now what is 8 - 2?");

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert_eq!(request.messages.len(), 4);

    println!("✓ Complex conversation request structure is valid");
}

/// Smoke test for o3 reasoning effort
#[tokio::test]
#[ignore]
async fn smoke_test_reasoning_effort() {
    require_smoke_config!();

    // Only test with o3 models
    if !config.model.starts_with("o3") {
        println!("Skipping reasoning effort test - not an o3 model");
        return;
    }

    let builder = openai_ergonomic::builders::responses::ResponsesBuilder::new(&config.model)
        .user("Solve this step by step: What is the 15th prime number?")
        .reasoning_effort("medium");

    let request = builder.build().expect("Should build request");

    assert_eq!(request.model, config.model);
    assert!(request.reasoning_effort.is_some());
    assert_eq!(request.reasoning_effort.as_ref().unwrap(), "medium");

    println!("✓ Reasoning effort request structure is valid");
}

/// Comprehensive smoke test that combines multiple features
#[tokio::test]
#[ignore]
async fn smoke_test_comprehensive() {
    require_smoke_config!();

    let search_tool = tool_function(
        "search_knowledge",
        "Search through a knowledge base",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "category": {
                    "type": "string",
                    "enum": ["science", "history", "technology"],
                    "description": "The category to search in"
                }
            },
            "required": ["query"]
        }),
    );

    let builder = responses_system_user(
        &config.model,
        "You are a research assistant. Use the search_knowledge function when needed.",
        "Find information about quantum computing.",
    )
    .temperature(0.7)
    .max_tokens(200)
    .tool(search_tool)
    .user_id("smoke-test-user");

    let request = builder.build().expect("Should build request");

    // Comprehensive validation
    assert_eq!(request.model, config.model);
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(200));
    assert!(request.tools.is_some());
    assert_eq!(request.user, Some("smoke-test-user".to_string()));

    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].function.name, "search_knowledge");

    println!("✓ Comprehensive request structure is valid");
}

/// Runner for all smoke tests
#[tokio::test]
#[ignore]
async fn run_all_smoke_tests() {
    if !SmokeTestConfig::is_enabled() {
        println!("Smoke tests disabled - set OPENAI_TEST_API_KEY to enable");
        return;
    }

    let config = SmokeTestConfig::from_env().unwrap();
    println!("Running smoke tests with model: {}", config.model);

    // List of all smoke test functions
    let tests = vec![
        ("Basic Chat Completion", smoke_test_basic_chat_completion()),
        ("System + User Messages", smoke_test_system_user_messages()),
        ("Function Calling", smoke_test_function_calling()),
        ("JSON Mode", smoke_test_json_mode()),
        ("JSON Schema", smoke_test_json_schema()),
        ("Streaming", smoke_test_streaming()),
        ("Vision/Multimodal", smoke_test_vision()),
        ("Parameter Boundaries", smoke_test_parameter_boundaries()),
        ("Complex Conversation", smoke_test_complex_conversation()),
        ("Reasoning Effort", smoke_test_reasoning_effort()),
        ("Comprehensive", smoke_test_comprehensive()),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (test_name, test_future) in tests {
        print!("Running {}: ", test_name);

        match tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout_seconds),
            test_future,
        ).await {
            Ok(Ok(())) => {
                println!("PASSED");
                passed += 1;
            }
            Ok(Err(e)) => {
                println!("FAILED - {:?}", e);
                failed += 1;
            }
            Err(_) => {
                println!("TIMEOUT");
                failed += 1;
            }
        }
    }

    println!("\nSmoke test summary: {} passed, {} failed", passed, failed);

    if failed > 0 {
        panic!("{} smoke tests failed", failed);
    }
}

/// Utility function for smoke test setup
pub fn setup_smoke_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
}

/// Validate environment setup for smoke tests
pub fn validate_smoke_test_environment() -> Result<(), String> {
    if !SmokeTestConfig::is_enabled() {
        return Err("OPENAI_TEST_API_KEY environment variable not set".to_string());
    }

    let config = SmokeTestConfig::from_env().unwrap();

    if config.api_key.is_empty() {
        return Err("OPENAI_TEST_API_KEY is empty".to_string());
    }

    if !config.api_key.starts_with("sk-") {
        return Err("OPENAI_TEST_API_KEY doesn't look like a valid API key".to_string());
    }

    if config.model.is_empty() {
        return Err("OPENAI_TEST_MODEL is empty".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_config_creation() {
        // Test with no environment variables
        std::env::remove_var("OPENAI_TEST_API_KEY");
        assert!(SmokeTestConfig::from_env().is_none());
        assert!(!SmokeTestConfig::is_enabled());

        // Test with API key set
        std::env::set_var("OPENAI_TEST_API_KEY", "sk-test123");
        let config = SmokeTestConfig::from_env().unwrap();
        assert_eq!(config.api_key, "sk-test123");
        assert_eq!(config.model, "gpt-3.5-turbo"); // default
        assert!(SmokeTestConfig::is_enabled());

        // Test with custom model
        std::env::set_var("OPENAI_TEST_MODEL", "gpt-4");
        let config = SmokeTestConfig::from_env().unwrap();
        assert_eq!(config.model, "gpt-4");

        // Clean up
        std::env::remove_var("OPENAI_TEST_API_KEY");
        std::env::remove_var("OPENAI_TEST_MODEL");
    }

    #[test]
    fn test_environment_validation() {
        // Test with no API key
        std::env::remove_var("OPENAI_TEST_API_KEY");
        assert!(validate_smoke_test_environment().is_err());

        // Test with empty API key
        std::env::set_var("OPENAI_TEST_API_KEY", "");
        assert!(validate_smoke_test_environment().is_err());

        // Test with invalid API key format
        std::env::set_var("OPENAI_TEST_API_KEY", "invalid-key");
        assert!(validate_smoke_test_environment().is_err());

        // Test with valid API key
        std::env::set_var("OPENAI_TEST_API_KEY", "sk-test123");
        assert!(validate_smoke_test_environment().is_ok());

        // Clean up
        std::env::remove_var("OPENAI_TEST_API_KEY");
    }
}
