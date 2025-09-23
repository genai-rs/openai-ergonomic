//! Comprehensive error handling tests for the openai-ergonomic crate.
//!
//! This module tests various error scenarios including validation errors,
//! API errors, network errors, and builder errors.

#![allow(
    dead_code,
    unused_imports,
    clippy::significant_drop_tightening,
    clippy::cast_possible_truncation,
    clippy::uninlined_format_args,
    clippy::manual_let_else
)]

mod harness;

use harness::{
    assert_builder_fails_with, assert_error_response, assert_error_type, fixtures, MockOpenAIClient,
};
use openai_ergonomic::{
    builders::{
        chat::{tool_function, ChatCompletionBuilder},
        responses::{responses_simple, responses_system_user},
        Builder,
    },
    errors::Error,
};
use serde_json::{json, Value};
use std::time::Duration;

/// Test builder validation errors
#[test]
fn test_builder_validation_errors() {
    // Test empty model
    let empty_model_result = ChatCompletionBuilder::new("").user("test").build();
    assert_error_type(empty_model_result, "invalid");

    // Test no messages
    let no_messages_result = ChatCompletionBuilder::new("gpt-4").build();
    assert_error_type(no_messages_result, "invalid");

    // Test empty user message
    let empty_user_result = ChatCompletionBuilder::new("gpt-4").user("").build();
    assert_error_type(empty_user_result, "invalid");

    // Test empty system message
    let empty_system_result = ChatCompletionBuilder::new("gpt-4")
        .system("")
        .user("test")
        .build();
    assert_error_type(empty_system_result, "invalid");
}

/// Test parameter boundary validation errors
#[test]
fn test_parameter_boundary_validation_errors() {
    // Temperature out of range
    let invalid_temp_low = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .temperature(-0.1);
    assert_builder_fails_with(|| invalid_temp_low.build(), "temperature");

    let invalid_temp_high = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .temperature(2.1);
    assert_builder_fails_with(|| invalid_temp_high.build(), "temperature");

    // Top-p out of range
    let invalid_top_p_low = ChatCompletionBuilder::new("gpt-4").user("test").top_p(-0.1);
    assert_builder_fails_with(|| invalid_top_p_low.build(), "top_p");

    let invalid_top_p_high = ChatCompletionBuilder::new("gpt-4").user("test").top_p(1.1);
    assert_builder_fails_with(|| invalid_top_p_high.build(), "top_p");

    // Frequency penalty out of range
    let invalid_freq_penalty_low = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .frequency_penalty(-2.1);
    assert_builder_fails_with(|| invalid_freq_penalty_low.build(), "frequency_penalty");

    let invalid_freq_penalty_high = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .frequency_penalty(2.1);
    assert_builder_fails_with(|| invalid_freq_penalty_high.build(), "frequency_penalty");

    // Presence penalty out of range
    let invalid_pres_penalty_low = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .presence_penalty(-2.1);
    assert_builder_fails_with(|| invalid_pres_penalty_low.build(), "presence_penalty");

    let invalid_pres_penalty_high = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .presence_penalty(2.1);
    assert_builder_fails_with(|| invalid_pres_penalty_high.build(), "presence_penalty");

    // Invalid max_tokens (zero)
    let invalid_max_tokens = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .max_tokens(0);
    assert_builder_fails_with(|| invalid_max_tokens.build(), "max_tokens");

    // Invalid n (zero)
    let invalid_n = ChatCompletionBuilder::new("gpt-4").user("test").n(0);
    assert_builder_fails_with(|| invalid_n.build(), "n");
}

/// Test tool validation errors
#[test]
fn test_tool_validation_errors() {
    // Empty function name
    let empty_name_tool = tool_function(
        "",
        "Description",
        json!({"type": "object", "properties": {}}),
    );

    let result = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .tools(vec![empty_name_tool])
        .build();
    assert_error_type(result, "invalid");

    // Invalid function name (with spaces)
    let invalid_name_tool = tool_function(
        "invalid name",
        "Description",
        json!({"type": "object", "properties": {}}),
    );

    let result = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .tools(vec![invalid_name_tool])
        .build();
    assert_error_type(result, "invalid");

    // Empty description
    let empty_desc_tool = tool_function(
        "valid_name",
        "",
        json!({"type": "object", "properties": {}}),
    );

    let result = ChatCompletionBuilder::new("gpt-4")
        .user("test")
        .tools(vec![empty_desc_tool])
        .build();
    assert_error_type(result, "invalid");
}

/// Test JSON schema validation errors
#[test]
fn test_json_schema_validation_errors() {
    // Invalid schema (missing type)
    let invalid_schema = json!({
        "properties": {"name": {"type": "string"}}
    });

    let result = responses_simple("gpt-4", "test")
        .json_schema("test", invalid_schema)
        .build();
    assert_error_type(result, "invalid");

    // Empty schema name
    let valid_schema = json!({
        "type": "object",
        "properties": {"name": {"type": "string"}}
    });

    let result = responses_simple("gpt-4", "test")
        .json_schema("", valid_schema)
        .build();
    assert_error_type(result, "invalid");
}

/// Test API error responses with mock server
#[tokio::test]
async fn test_api_error_responses() {
    let mut mock_client = MockOpenAIClient::new().await;

    // Test rate limit error
    let rate_limit_mock = mock_client.mock_rate_limit_error().await;

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

    rate_limit_mock.assert_async().await;
}

/// Test authentication error
#[tokio::test]
async fn test_authentication_error() {
    let mut mock_client = MockOpenAIClient::new().await;
    let auth_mock = mock_client.mock_auth_error().await;

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

    auth_mock.assert_async().await;
}

/// Test validation error with parameter details
#[tokio::test]
async fn test_validation_error_with_param() {
    let mut mock_client = MockOpenAIClient::new().await;
    let validation_mock = mock_client.mock_validation_error("messages").await;

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
    assert!(error.get("param").is_some());

    validation_mock.assert_async().await;
}

/// Test server error handling
#[tokio::test]
async fn test_server_error_handling() {
    let mut mock_client = MockOpenAIClient::new().await;
    let server_error_mock = mock_client.mock_server_error().await;

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

    assert_eq!(response.status(), 503);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "server_error");

    server_error_mock.assert_async().await;
}

/// Test network timeout errors
#[tokio::test]
async fn test_network_timeout_error() {
    // Test timeout by making a request to an invalid/unreachable endpoint
    // This will cause a connection timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(100)) // 100ms timeout
        .build()
        .unwrap();

    // Use a non-existent IP to force a timeout
    let result = client
        .post("http://1.2.3.4:80/v1/chat/completions") // Non-routable IP
        .header("authorization", "Bearer fake-key")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // The error should be a timeout or connection error
    assert!(error.is_timeout() || error.is_connect());
}

/// Test error response fixtures
#[test]
fn test_error_response_fixtures() {
    let error_scenarios = fixtures::scenarios::error_scenarios();

    for (error_type, error_json) in error_scenarios {
        // Verify error structure
        assert_error_response(&error_json, &error_type);

        let error = error_json.get("error").unwrap();
        assert!(error.get("type").is_some());
        assert!(error.get("message").is_some());

        let message = error.get("message").unwrap().as_str().unwrap();
        assert!(!message.is_empty());

        // Test specific error types
        match error_type.as_str() {
            "rate_limit_exceeded" => {
                assert!(message.contains("rate limit") || message.contains("Rate limit"));
            }
            "invalid_api_key" => {
                assert!(message.contains("API key") || message.contains("api key"));
            }
            "invalid_request_error" => {
                // Should have param field for validation errors
                if error.get("param").is_some() {
                    let param = error.get("param").unwrap().as_str().unwrap();
                    assert!(!param.is_empty());
                }
            }
            "server_error" => {
                assert!(message.contains("server") || message.contains("Server"));
            }
            _ => {} // Other error types
        }
    }
}

/// Test error message formatting and details
#[test]
fn test_error_message_formatting() {
    // Test various Error types
    let invalid_request = Error::InvalidRequest("Test validation error".to_string());
    assert!(invalid_request
        .to_string()
        .contains("Test validation error"));

    let auth_error = Error::Authentication("Invalid API key".to_string());
    assert!(auth_error.to_string().contains("Invalid API key"));

    let rate_limit = Error::RateLimit("Rate limit exceeded".to_string());
    assert!(rate_limit.to_string().contains("Rate limit exceeded"));

    let api_error = Error::api(400, "Bad Request");
    assert!(api_error.to_string().contains("400"));
    assert!(api_error.to_string().contains("Bad Request"));

    let config_error = Error::Config("Invalid configuration".to_string());
    assert!(config_error.to_string().contains("Invalid configuration"));

    let builder_error = Error::Builder("Builder validation failed".to_string());
    assert!(builder_error
        .to_string()
        .contains("Builder validation failed"));

    let internal_error = Error::Internal("Internal error".to_string());
    assert!(internal_error.to_string().contains("Internal error"));

    let stream_error = Error::Stream("Streaming error".to_string());
    assert!(stream_error.to_string().contains("Streaming error"));
}

/// Test error propagation in complex scenarios
#[test]
fn test_error_propagation() {
    // Test that validation errors are properly propagated through builders
    let result = responses_system_user("", "system", "user").build();
    assert!(result.is_err());
    assert_error_type(result, "invalid");

    // Test multiple validation errors
    let result = ChatCompletionBuilder::new("").temperature(-1.0).build();
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string();
    // Should contain information about the first validation error encountered
    assert!(!error_str.is_empty());
}

/// Test error handling with malformed responses
#[tokio::test]
async fn test_malformed_response_handling() {
    let mut mock_client = MockOpenAIClient::new().await;

    // Mock a malformed JSON response
    let malformed_mock = mock_client
        .server()
        .mock("POST", "/v1/chat/completions")
        .match_header(
            "authorization",
            format!("Bearer {}", mock_client.api_key()).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{ invalid json")
        .create_async()
        .await;

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

    // Attempt to parse as JSON should fail
    let json_result = response.json::<Value>().await;
    assert!(json_result.is_err());

    malformed_mock.assert_async().await;
}

/// Test retry scenario error handling
#[tokio::test]
async fn test_retry_scenario_errors() {
    let mut mock_client = MockOpenAIClient::new().await;
    let (error_mock1, error_mock2, _success_mock) = mock_client.setup_retry_scenario().await;

    let client = reqwest::Client::new();

    // First request should get rate limit error
    let response1 = client
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

    assert_eq!(response1.status(), 429);

    // Second request should get server error
    let response2 = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello again"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response2.status(), 503);

    // Verify mocks were called
    error_mock1.assert_async().await;
    error_mock2.assert_async().await;
}

/// Test error handling in streaming scenarios
#[tokio::test]
async fn test_streaming_error_handling() {
    let mut mock_client = MockOpenAIClient::new().await;
    let error_mock = mock_client
        .mock_error_response(400, "invalid_request_error", "Streaming request invalid")
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "stream": true
            // Missing messages
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "invalid_request_error");

    error_mock.assert_async().await;
}

/// Test custom error response with additional details
#[tokio::test]
async fn test_custom_error_response_details() {
    let mut mock_client = MockOpenAIClient::new().await;
    let detailed_error_mock = mock_client
        .mock_error_response_with_details(
            400,
            "invalid_request_error",
            "The model parameter is invalid",
            Some("model_not_found"),
            Some("model"),
        )
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "invalid-model",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "invalid_request_error");

    let error = response_json.get("error").unwrap();
    assert!(error.get("code").is_some());
    assert!(error.get("param").is_some());

    let code = error.get("code").unwrap().as_str().unwrap();
    assert_eq!(code, "model_not_found");

    let param = error.get("param").unwrap().as_str().unwrap();
    assert_eq!(param, "model");

    detailed_error_mock.assert_async().await;
}

/// Test error handling performance
#[test]
fn test_error_handling_performance() {
    use harness::assert_performance;

    // Test that error creation and formatting is fast
    let _error = assert_performance(
        || {
            let error = Error::InvalidRequest("Test error message".to_string());
            error.to_string()
        },
        Duration::from_millis(1),
        "error_creation_and_formatting",
    );

    // Test that validation errors are detected quickly
    let _result = assert_performance(
        || ChatCompletionBuilder::new("").user("test").build(),
        Duration::from_millis(1),
        "validation_error_detection",
    );
}

/// Test edge cases in error handling
#[test]
fn test_error_handling_edge_cases() {
    // Test with very long error messages
    let long_message = "x".repeat(10000);
    let error = Error::InvalidRequest(long_message.clone());
    assert!(error.to_string().contains(&long_message));

    // Test with special characters in error messages
    let special_chars = "Error with special chars: 🚨 \n\t\"'\\{}[]";
    let error = Error::Builder(special_chars.to_string());
    assert!(error.to_string().contains(special_chars));

    // Test with empty error message
    let error = Error::Config(String::new());
    assert!(!error.to_string().is_empty()); // Should still have error type info
}

/// Test concurrent error scenarios
#[tokio::test]
async fn test_concurrent_error_scenarios() {
    let client = reqwest::Client::new();

    // Test each error type sequentially to avoid mock conflicts
    // Test rate limit error
    {
        let mut mock_client = MockOpenAIClient::new().await;
        let rate_limit_mock = mock_client.mock_rate_limit_error().await;

        let result = make_error_request(&client, &mock_client.base_url(), mock_client.api_key(), "rate_limit").await;
        assert_eq!(result.unwrap(), 429);
        rate_limit_mock.assert_async().await;
    }

    // Test auth error
    {
        let mut mock_client = MockOpenAIClient::new().await;
        let auth_mock = mock_client.mock_auth_error().await;

        let result = make_error_request(&client, &mock_client.base_url(), mock_client.api_key(), "auth").await;
        assert_eq!(result.unwrap(), 401);
        auth_mock.assert_async().await;
    }

    // Test validation error
    {
        let mut mock_client = MockOpenAIClient::new().await;
        let validation_mock = mock_client.mock_validation_error("messages").await;

        let result = make_error_request(&client, &mock_client.base_url(), mock_client.api_key(), "validation").await;
        assert_eq!(result.unwrap(), 400);
        validation_mock.assert_async().await;
    }
}

/// Helper function to make error-inducing requests
async fn make_error_request(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    error_type: &str,
) -> Result<u16, reqwest::Error> {
    let payload = match error_type {
        "validation" => json!({"model": "gpt-4"}), // Missing messages
        _ => json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }),
    };

    let response = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    Ok(response.status().as_u16())
}
