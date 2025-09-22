//! Mock-based integration tests using the test utilities.
//!
//! These tests demonstrate how to use the testing infrastructure
//! and validate the interaction between builders and mock servers.
#![allow(clippy::significant_drop_tightening)]
#![cfg(feature = "test-utils")]

use openai_ergonomic::{
    builders::{responses::responses_simple, Builder},
    test_utils::{
        assertions::{assert_has_field, assert_success_response},
        fixtures, MockOpenAIServer,
    },
};
use serde_json::Value;

/// Test basic mock server functionality
#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn test_mock_server_setup() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_chat_completions_success().await;

    // Verify server is running
    assert!(!mock_server.base_url().is_empty());
    assert!(mock_server.base_url().starts_with("http://"));
}

/// Test mock server with custom API key
#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn test_mock_server_with_custom_api_key() {
    let custom_key = "sk-test-custom-key";
    let mut mock_server = MockOpenAIServer::with_api_key(custom_key).await;
    let _mock = mock_server.mock_chat_completions_success().await;

    assert!(!mock_server.base_url().is_empty());
}

/// Test chat completions mock response
#[tokio::test]
async fn test_mock_chat_completions_success() {
    let mut mock_server = MockOpenAIServer::new().await;
    let mock = mock_server.mock_chat_completions_success().await;

    // Make a request to the mock server
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let json: Value = response.json().await.unwrap();
    assert_has_field(&json, "id");
    assert_has_field(&json, "object");
    assert_has_field(&json, "choices");
    assert_success_response(&json);

    // Verify mock was called
    mock.assert_async().await;
}

/// Test streaming mock response
#[tokio::test]
async fn test_mock_streaming_response() {
    let mut mock_server = MockOpenAIServer::new().await;
    let mock = mock_server.mock_chat_completions_streaming().await;

    // Make a streaming request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
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

    mock.assert_async().await;
}

/// Test error response mocking
#[tokio::test]
async fn test_mock_error_response() {
    let mut mock_server = MockOpenAIServer::new().await;
    let mock = mock_server
        .mock_error_response(400, "invalid_request_error", "Missing required parameter")
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4"
            // Missing messages parameter
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let json: Value = response.json().await.unwrap();
    assert_has_field(&json, "error");

    let error = json.get("error").unwrap();
    assert_has_field(error, "type");
    assert_has_field(error, "message");

    mock.assert_async().await;
}

/// Test embeddings endpoint mock
#[tokio::test]
async fn test_mock_embeddings_endpoint() {
    let mut mock_server = MockOpenAIServer::new().await;
    let mock = mock_server.mock_embeddings_success().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/embeddings", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "text-embedding-ada-002",
            "input": "Hello, world!"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let json: Value = response.json().await.unwrap();
    assert_has_field(&json, "object");
    assert_has_field(&json, "data");
    assert_has_field(&json, "usage");

    mock.assert_async().await;
}

/// Test models list endpoint mock
#[tokio::test]
async fn test_mock_models_list() {
    let mut mock_server = MockOpenAIServer::new().await;
    let mock = mock_server.mock_models_list().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/v1/models", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let json: Value = response.json().await.unwrap();
    assert_has_field(&json, "object");
    assert_has_field(&json, "data");

    let data = json.get("data").unwrap().as_array().unwrap();
    assert!(!data.is_empty());

    for model in data {
        assert_has_field(model, "id");
        assert_has_field(model, "object");
    }

    mock.assert_async().await;
}

/// Test fixture utilities
#[test]
fn test_fixtures() {
    let chat_request = fixtures::chat_completion_request();
    assert_has_field(&chat_request, "model");
    assert_has_field(&chat_request, "messages");
    assert_has_field(&chat_request, "temperature");

    let embeddings_request = fixtures::embeddings_request();
    assert_has_field(&embeddings_request, "model");
    assert_has_field(&embeddings_request, "input");

    let tool_def = fixtures::tool_definition();
    assert_has_field(&tool_def, "type");
    assert_has_field(&tool_def, "function");
}

/// Test assertion helpers
#[test]
fn test_assertion_helpers() {
    let response = serde_json::json!({
        "id": "test-123",
        "model": "gpt-4",
        "choices": [{"message": {"content": "Hello"}}]
    });

    assert_has_field(&response, "id");
    assert_has_field(&response, "model");
    assert_success_response(&response);

    // Test field value assertion
    openai_ergonomic::test_utils::assertions::assert_field_equals(
        &response,
        "model",
        &serde_json::json!("gpt-4"),
    );
}

/// Test builder integration with fixtures
#[test]
fn test_builder_with_fixtures() {
    let _fixture = fixtures::chat_completion_request();

    // Verify we can create a similar request with our builder
    let builder = responses_simple("gpt-4", "Hello!")
        .temperature(0.7)
        .max_tokens(150);

    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(150));
}

/// Test multiple mock scenarios
#[tokio::test]
async fn test_multiple_mock_scenarios() {
    let mut mock_server = MockOpenAIServer::new().await;

    // Set up multiple mocks
    let success_mock = mock_server.mock_chat_completions_success().await;
    let error_mock = mock_server
        .mock_error_response(429, "rate_limit_exceeded", "Rate limit exceeded")
        .await;

    let client = reqwest::Client::new();

    // First request should succeed
    let response1 = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response1.status(), 200);

    // Second request should return rate limit error
    let response2 = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("authorization", "Bearer test-api-key")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello again"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response2.status(), 429);

    // Verify both mocks were called
    success_mock.assert_async().await;
    error_mock.assert_async().await;
}

/// Test that mock server handles concurrent requests
#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn test_concurrent_mock_requests() {
    let mut mock_server = MockOpenAIServer::new().await;

    // Create multiple identical mocks for concurrent requests
    let _mock1 = mock_server.mock_chat_completions_success().await;
    let _mock2 = mock_server.mock_chat_completions_success().await;
    let _mock3 = mock_server.mock_chat_completions_success().await;

    let client = reqwest::Client::new();
    let base_url = mock_server.base_url();

    // Create concurrent requests
    let request1 = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("authorization", "Bearer test-api-key")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Request 1"}]
        }))
        .send();

    let request2 = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("authorization", "Bearer test-api-key")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Request 2"}]
        }))
        .send();

    let request3 = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("authorization", "Bearer test-api-key")
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Request 3"}]
        }))
        .send();

    // Wait for all requests to complete
    let results = tokio::join!(request1, request2, request3);

    // All should succeed
    assert_eq!(results.0.unwrap().status(), 200);
    assert_eq!(results.1.unwrap().status(), 200);
    assert_eq!(results.2.unwrap().status(), 200);
}
