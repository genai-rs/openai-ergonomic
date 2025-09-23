//! Comprehensive integration tests for response handling in the openai-ergonomic crate.
//!
//! This module tests response parsing, validation, and integration with various
//! OpenAI API response formats including streaming, function calling, and error responses.

#![allow(
    dead_code,
    unused_imports,
    clippy::cast_possible_truncation,
    clippy::significant_drop_tightening,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::manual_let_else
)]

mod harness;

use harness::{
    assert_complete_streaming_message, assert_error_response, assert_field_equals,
    assert_has_field, assert_json_equivalent, assert_success_response, assert_valid_chat_response,
    assert_valid_stream_chunk, fixtures, MockClientBuilder, MockOpenAIClient,
};
use openai_client_base::models::{
    CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
};
use serde_json::{json, Value};
use std::time::Duration;

/// Test parsing of basic successful chat completion responses
#[test]
fn test_basic_response_parsing() {
    let response_json = fixtures::chat_responses::basic_success();

    // Verify the fixture structure
    assert_success_response(&response_json);
    assert_has_field(&response_json, "id");
    assert_has_field(&response_json, "object");
    assert_has_field(&response_json, "choices");
    assert_has_field(&response_json, "usage");

    assert_field_equals(&response_json, "object", &json!("chat.completion"));
    assert_field_equals(&response_json, "model", &json!("gpt-4"));

    // Test that the response can be deserialized
    let response: CreateChatCompletionResponse =
        serde_json::from_value(response_json).expect("Should be able to deserialize response");

    assert_valid_chat_response(&response);
}

/// Test parsing of function calling responses
#[test]
fn test_function_calling_response_parsing() {
    let response_json = fixtures::chat_responses::with_function_call();

    assert_success_response(&response_json);
    assert_has_field(&response_json, "choices");

    let choices = response_json.get("choices").unwrap().as_array().unwrap();
    assert!(!choices.is_empty());

    let choice = &choices[0];
    assert_has_field(choice, "message");

    let message = choice.get("message").unwrap();
    assert_has_field(message, "tool_calls");

    let tool_calls = message.get("tool_calls").unwrap().as_array().unwrap();
    assert!(!tool_calls.is_empty());

    let tool_call = &tool_calls[0];
    assert_has_field(tool_call, "id");
    assert_has_field(tool_call, "type");
    assert_has_field(tool_call, "function");

    assert_field_equals(tool_call, "type", &json!("function"));

    let function = tool_call.get("function").unwrap();
    assert_has_field(function, "name");
    assert_has_field(function, "arguments");

    // Test deserialization
    let response: CreateChatCompletionResponse = serde_json::from_value(response_json)
        .expect("Should be able to deserialize function calling response");

    assert_valid_chat_response(&response);
}

/// Test parsing of streaming response chunks
#[test]
fn test_streaming_response_parsing() {
    let chunks = fixtures::chat_responses::streaming_chunks();
    assert!(!chunks.is_empty());

    for (i, chunk_json) in chunks.iter().enumerate() {
        assert_has_field(chunk_json, "id");
        assert_has_field(chunk_json, "object");
        assert_has_field(chunk_json, "choices");

        assert_field_equals(chunk_json, "object", &json!("chat.completion.chunk"));

        let choices = chunk_json.get("choices").unwrap().as_array().unwrap();
        assert!(!choices.is_empty());

        let choice = &choices[0];
        assert_has_field(choice, "delta");

        // Test deserialization
        let chunk: CreateChatCompletionStreamResponse = serde_json::from_value(chunk_json.clone())
            .expect("Should be able to deserialize streaming chunk");

        assert_valid_stream_chunk(&chunk);

        // Last chunk should have finish_reason
        if i == chunks.len() - 1 {
            assert_has_field(choice, "finish_reason");
            assert!(!choice.get("finish_reason").unwrap().is_null());
        }
    }

    // Test that chunks form a complete message
    assert_complete_streaming_message(&chunks, "Hello there!");
}

/// Test parsing of JSON mode responses
#[test]
fn test_json_mode_response_parsing() {
    let response_json = fixtures::chat_responses::json_mode();

    assert_success_response(&response_json);

    let choices = response_json.get("choices").unwrap().as_array().unwrap();
    let choice = &choices[0];
    let message = choice.get("message").unwrap();
    let content = message.get("content").unwrap().as_str().unwrap();

    // Content should be valid JSON
    let json_content: Value = serde_json::from_str(content).expect("Content should be valid JSON");

    assert!(json_content.is_object());
    assert_has_field(&json_content, "name");
    assert_has_field(&json_content, "age");
    assert_has_field(&json_content, "city");

    // Test deserialization
    let response: CreateChatCompletionResponse = serde_json::from_value(response_json)
        .expect("Should be able to deserialize JSON mode response");

    assert_valid_chat_response(&response);
}

/// Test parsing of various error responses
#[test]
fn test_error_response_parsing() {
    let error_scenarios = fixtures::scenarios::error_scenarios();

    for (error_type, error_json) in error_scenarios {
        assert_error_response(&error_json, &error_type);

        let error = error_json.get("error").unwrap();
        assert_has_field(error, "type");
        assert_has_field(error, "message");

        let message = error.get("message").unwrap().as_str().unwrap();
        assert!(!message.is_empty());

        // Specific error type validations
        match error_type.as_str() {
            "rate_limit_exceeded" => {
                assert!(message.to_lowercase().contains("rate limit"));
            }
            "invalid_api_key" => {
                assert!(message.to_lowercase().contains("api key"));
            }
            "invalid_request_error" => {
                assert_has_field(error, "param");
            }
            "server_error" => {
                assert!(message.to_lowercase().contains("server"));
            }
            _ => {} // Other error types
        }
    }
}

/// Test response validation with mock server
#[tokio::test]
async fn test_mock_server_response_validation() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_chat_completions_success().await;

    // Make request to mock server
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let response_json: Value = response.json().await.unwrap();
    assert_success_response(&response_json);

    // Verify response structure
    assert_has_field(&response_json, "id");
    assert_has_field(&response_json, "choices");
    assert_has_field(&response_json, "usage");

    mock.assert_async().await;
}

/// Test custom response content with mock server
#[tokio::test]
async fn test_custom_response_content() {
    let mut mock_client = MockOpenAIClient::new().await;
    let custom_content = "This is a custom response for testing.";
    let mock = mock_client
        .mock_chat_completions_success_with_content(custom_content)
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test message"}]
        }))
        .send()
        .await
        .unwrap();

    let response_json: Value = response.json().await.unwrap();
    let choices = response_json.get("choices").unwrap().as_array().unwrap();
    let message_content = choices[0]
        .get("message")
        .unwrap()
        .get("content")
        .unwrap()
        .as_str()
        .unwrap();

    assert_eq!(message_content, custom_content);

    mock.assert_async().await;
}

/// Test streaming response validation
#[tokio::test]
async fn test_streaming_response_validation() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_chat_completions_streaming().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    let text = response.text().await.unwrap();
    assert!(text.contains("data:"));
    assert!(text.contains("[DONE]"));

    // Parse streaming chunks
    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();

    for line in lines {
        if line.starts_with("data: ") && !line.contains("[DONE]") {
            let json_str = &line[6..]; // Remove "data: " prefix
            if let Ok(chunk_json) = serde_json::from_str::<Value>(json_str) {
                chunks.push(chunk_json);
            }
        }
    }

    assert!(!chunks.is_empty());

    // Validate each chunk
    for chunk in &chunks {
        assert_has_field(chunk, "id");
        assert_has_field(chunk, "object");
        assert_has_field(chunk, "choices");
        assert_field_equals(chunk, "object", &json!("chat.completion.chunk"));
    }

    mock.assert_async().await;
}

/// Test function calling response with mock server
#[tokio::test]
async fn test_function_calling_mock_response() {
    let mut mock_client = MockOpenAIClient::new().await;
    let function_args = json!({"location": "Paris", "units": "celsius"});
    let mock = mock_client
        .mock_function_calling_response("get_weather", function_args.clone())
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "What's the weather in Paris?"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather information",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"},
                            "units": {"type": "string"}
                        }
                    }
                }
            }]
        }))
        .send()
        .await
        .unwrap();

    let response_json: Value = response.json().await.unwrap();
    assert_success_response(&response_json);

    let choices = response_json.get("choices").unwrap().as_array().unwrap();
    let message = choices[0].get("message").unwrap();
    let tool_calls = message.get("tool_calls").unwrap().as_array().unwrap();

    assert!(!tool_calls.is_empty());
    let tool_call = &tool_calls[0];
    let function = tool_call.get("function").unwrap();

    assert_field_equals(function, "name", &json!("get_weather"));

    let arguments_str = function.get("arguments").unwrap().as_str().unwrap();
    let parsed_args: Value = serde_json::from_str(arguments_str).unwrap();
    assert_json_equivalent(&parsed_args, &function_args);

    mock.assert_async().await;
}

/// Test JSON mode response with mock server
#[tokio::test]
async fn test_json_mode_mock_response() {
    let mut mock_client = MockOpenAIClient::new().await;
    let json_content = json!({"name": "John", "age": 30, "city": "New York"});
    let mock = mock_client
        .mock_json_mode_response(json_content.clone())
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Generate a JSON object"}],
            "response_format": {"type": "json_object"}
        }))
        .send()
        .await
        .unwrap();

    let response_json: Value = response.json().await.unwrap();
    assert_success_response(&response_json);

    let choices = response_json.get("choices").unwrap().as_array().unwrap();
    let message_content = choices[0]
        .get("message")
        .unwrap()
        .get("content")
        .unwrap()
        .as_str()
        .unwrap();

    let parsed_content: Value = serde_json::from_str(message_content).unwrap();
    assert_json_equivalent(&parsed_content, &json_content);

    mock.assert_async().await;
}

/// Test error response handling with mock server
#[tokio::test]
async fn test_error_response_mock_handling() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client
        .mock_error_response(400, "invalid_request_error", "Missing required parameter")
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4"
            // Missing messages parameter
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "invalid_request_error");

    let error = response_json.get("error").unwrap();
    let message = error.get("message").unwrap().as_str().unwrap();
    assert!(message.contains("Missing required parameter"));

    mock.assert_async().await;
}

/// Test rate limit error handling
#[tokio::test]
async fn test_rate_limit_error_handling() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_rate_limit_error().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 429);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "rate_limit_exceeded");

    mock.assert_async().await;
}

/// Test authentication error handling
#[tokio::test]
async fn test_auth_error_handling() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_auth_error().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "invalid_api_key");

    mock.assert_async().await;
}

/// Test custom streaming chunks
#[tokio::test]
async fn test_custom_streaming_chunks() {
    let mut mock_client = MockOpenAIClient::new().await;
    let custom_chunks = vec!["Once", " upon", " a", " time", "..."];
    let mock = mock_client
        .mock_chat_completions_streaming_with_chunks(custom_chunks.clone())
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Tell me a story"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    let text = response.text().await.unwrap();

    // Verify all chunks are present
    for chunk in &custom_chunks {
        assert!(text.contains(chunk));
    }

    assert!(text.contains("[DONE]"));

    mock.assert_async().await;
}

/// Test response with custom configuration
#[tokio::test]
async fn test_response_with_custom_config() {
    let mut mock_client = MockClientBuilder::new()
        .default_model("gpt-3.5-turbo")
        .token_counts(25, 50)
        .rate_limits(Some(1000), Some(1_234_567_890))
        .custom_header("x-test-header", "test-value")
        .build()
        .await;

    let mock = mock_client.mock_chat_completions_success().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    // Check rate limiting headers before consuming response
    let headers = response.headers().clone();

    let response_json: Value = response.json().await.unwrap();
    assert_success_response(&response_json);

    assert_field_equals(&response_json, "model", &json!("gpt-3.5-turbo"));

    let usage = response_json.get("usage").unwrap();
    assert_field_equals(usage, "prompt_tokens", &json!(25));
    assert_field_equals(usage, "completion_tokens", &json!(50));
    assert_field_equals(usage, "total_tokens", &json!(75));

    // Check rate limiting headers
    assert_eq!(
        headers.get("x-ratelimit-remaining-requests").unwrap(),
        "1000"
    );
    assert_eq!(
        headers.get("x-ratelimit-reset-requests").unwrap(),
        "1234567890"
    );

    mock.assert_async().await;
}

/// Performance test for response parsing
#[test]
fn test_response_parsing_performance() {
    use harness::assert_performance;

    let response_json = fixtures::chat_responses::basic_success();

    let _response = assert_performance(
        || {
            serde_json::from_value::<CreateChatCompletionResponse>(response_json.clone())
                .expect("Should parse successfully")
        },
        Duration::from_millis(5),
        "response_parsing",
    );
}

/// Test response validation edge cases
#[test]
fn test_response_validation_edge_cases() {
    // Test response with minimal required fields
    let minimal_response = json!({
        "id": "test",
        "object": "chat.completion",
        "created": 1_677_652_288,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello",
                "refusal": null
            },
            "logprobs": null,
            "finish_reason": "stop"
        }]
        // Note: usage is optional
    });

    let response: CreateChatCompletionResponse =
        serde_json::from_value(minimal_response).expect("Should parse minimal response");

    assert_valid_chat_response(&response);

    // Test response with multiple choices
    let multi_choice_response = json!({
        "id": "test",
        "object": "chat.completion",
        "created": 1_677_652_288,
        "model": "gpt-4",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "First response",
                    "refusal": null
                },
                "logprobs": null,
                "finish_reason": "stop"
            },
            {
                "index": 1,
                "message": {
                    "role": "assistant",
                    "content": "Second response",
                    "refusal": null
                },
                "logprobs": null,
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    });

    let response: CreateChatCompletionResponse =
        serde_json::from_value(multi_choice_response).expect("Should parse multi-choice response");

    assert_valid_chat_response(&response);
    assert_eq!(response.choices.len(), 2);
}
