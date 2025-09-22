//! Integration tests for the openai-ergonomic crate.
//!
//! These tests validate the end-to-end functionality of the crate,
//! including builder patterns, serialization, and API integration.

use openai_ergonomic::{
    builders::{
        chat::{image_url_part, system_user, text_part, tool_function},
        responses::{responses_simple, responses_system_user, responses_tool_function},
        Builder,
    },
    errors::Error,
    Detail,
};
use openai_client_base::models::ChatCompletionRequestUserMessageContentPart;
use serde_json::json;

/// Test basic responses builder functionality
#[test]
fn test_responses_builder_integration() {
    let builder = responses_simple("gpt-4", "Hello, world!");
    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 1);
    assert!(request.temperature.is_none());
    assert!(request.tools.is_none());
}

/// Test complex responses builder with system and user messages
#[test]
fn test_responses_builder_system_user_integration() {
    let builder = responses_system_user(
        "gpt-4",
        "You are a helpful AI assistant",
        "What's the capital of France?",
    )
    .temperature(0.7)
    .max_tokens(100);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));
}

/// Test responses builder with tools
#[test]
fn test_responses_builder_with_tools() {
    let tool = responses_tool_function(
        "get_weather",
        "Get current weather information",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The location to get weather for"
                }
            },
            "required": ["location"]
        }),
    );

    let builder = responses_simple("gpt-4", "What's the weather in Paris?").tool(tool);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 1);
    assert!(request.tools.is_some());
    assert_eq!(request.tools.as_ref().unwrap().len(), 1);
}

/// Test chat builder functionality
#[test]
fn test_chat_builder_integration() {
    let builder = system_user("gpt-4", "You are a helpful assistant", "Hello!");

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 2);
}

/// Test chat builder with vision (multimodal) content
#[test]
fn test_chat_builder_with_vision() {
    let text_part = text_part("What do you see in this image?");
    let image_part = image_url_part("https://example.com/image.jpg");
    let parts = vec![text_part, image_part];

    let builder =
        openai_ergonomic::builders::chat::ChatCompletionBuilder::new("gpt-4-vision-preview")
            .user_with_parts(parts);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4-vision-preview");
    assert_eq!(request.messages.len(), 1);
}

/// Test chat builder with tools
#[test]
fn test_chat_builder_with_tools() {
    let tool = tool_function(
        "calculate",
        "Perform mathematical calculations",
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate"
                }
            },
            "required": ["expression"]
        }),
    );

    let builder =
        system_user("gpt-4", "You are a math assistant", "Calculate 2 + 2").tools(vec![tool]);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 2);
    assert!(request.tools.is_some());
    assert_eq!(request.tools.as_ref().unwrap().len(), 1);
}

/// Test JSON schema functionality
#[test]
fn test_responses_builder_json_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "number"},
            "email": {"type": "string", "format": "email"}
        },
        "required": ["name", "age"]
    });

    let builder =
        responses_simple("gpt-4", "Generate a person object").json_schema("person", schema);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert!(request.response_format.is_some());

    let response_format = request.response_format.unwrap();
    assert_eq!(response_format.json_schema.name, "person");
}

/// Test builder validation - empty messages should fail
#[test]
fn test_builder_validation_empty_messages() {
    let builder = openai_ergonomic::builders::responses::ResponsesBuilder::new("gpt-4");
    let result = builder.build();

    assert!(result.is_err());
    match result {
        Err(Error::InvalidRequest(msg)) => {
            assert!(msg.contains("At least one message is required"));
        }
        _ => panic!("Expected InvalidRequest error"),
    }
}

/// Test builder parameter boundaries and edge cases
#[test]
fn test_builder_parameter_validation() {
    // Test with extreme but valid parameters
    let builder = responses_simple("gpt-4", "Test")
        .temperature(0.0)
        .max_tokens(1)
        .n(1)
        .top_p(0.1)
        .frequency_penalty(-2.0)
        .presence_penalty(2.0);

    let request = builder.build().unwrap();

    assert_eq!(request.temperature, Some(0.0));
    assert_eq!(request.max_tokens, Some(1));
    assert_eq!(request.n, Some(1));
    assert_eq!(request.top_p, Some(0.1));
    assert_eq!(request.frequency_penalty, Some(-2.0));
    assert_eq!(request.presence_penalty, Some(2.0));
}

/// Test serialization of complex requests
#[test]
fn test_request_serialization() {
    let tool = responses_tool_function(
        "search",
        "Search for information",
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        }),
    );

    let builder = responses_system_user(
        "gpt-4",
        "You are a search assistant",
        "Find information about Rust programming",
    )
    .temperature(0.8)
    .max_completion_tokens(500)
    .tool(tool)
    .json_mode();

    let request = builder.build().unwrap();

    // Verify that the request can be serialized to JSON
    let json_result = serde_json::to_string(&request);
    assert!(json_result.is_ok());

    let json_string = json_result.unwrap();
    assert!(json_string.contains("gpt-4"));
    assert!(json_string.contains("search"));
    assert!(json_string.contains("temperature"));
}

/// Test that different detail levels work correctly
#[test]
fn test_image_detail_levels() {
    let details = vec![Detail::Auto, Detail::Low, Detail::High];

    for detail in details {
        let image_part = openai_ergonomic::builders::chat::image_url_part_with_detail(
            "https://example.com/image.jpg",
            detail.clone(),
        );

        match image_part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(img) => {
                assert_eq!(img.image_url.detail, Some(detail));
            }
            _ => panic!("Expected image part"),
        }
    }
}

/// Test base64 image handling
#[test]
fn test_base64_image_handling() {
    let base64_data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
    let media_type = "image/png";

    let image_part = openai_ergonomic::builders::chat::image_base64_part(base64_data, media_type);

    match image_part {
        ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(img) => {
            assert!(img.image_url.url.starts_with("data:image/png;base64,"));
            assert!(img.image_url.url.contains(base64_data));
        }
        _ => panic!("Expected image part"),
    }
}

/// Test helper function consistency between chat and responses modules
#[test]
fn test_helper_function_consistency() {
    // Both modules should create equivalent tools
    let chat_tool = openai_ergonomic::builders::chat::tool_function(
        "test_function",
        "A test function",
        json!({"type": "object", "properties": {}}),
    );

    let responses_tool = openai_ergonomic::builders::responses::responses_tool_function(
        "test_function",
        "A test function",
        json!({"type": "object", "properties": {}}),
    );

    assert_eq!(chat_tool.function.name, responses_tool.function.name);
    assert_eq!(
        chat_tool.function.description,
        responses_tool.function.description
    );
}

/// Test reasoning effort parameter for o3 models
#[test]
fn test_reasoning_effort_parameter() {
    let builder = openai_ergonomic::builders::responses::ResponsesBuilder::new("o3-mini")
        .user("Solve this complex problem")
        .reasoning_effort("high");

    let request = builder.build().unwrap();

    assert_eq!(request.model, "o3-mini");
    assert!(request.reasoning_effort.is_some());
}

/// Test stream parameter
#[test]
fn test_stream_parameter() {
    let builder = responses_simple("gpt-4", "Hello").stream(true);

    let request = builder.build().unwrap();

    assert_eq!(request.stream, Some(true));
}

/// Test stop sequences
#[test]
fn test_stop_sequences() {
    let stop_sequences = vec!["STOP".to_string(), "END".to_string()];

    let builder = responses_simple("gpt-4", "Count to 10").stop(stop_sequences.clone());

    let request = builder.build().unwrap();

    assert!(request.stop.is_some());
    // Note: The actual implementation wraps stop sequences in a StopConfiguration enum
}
