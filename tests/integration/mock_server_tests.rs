//! Integration tests using mock server
//!
//! These tests validate the testing infrastructure by using the mock `OpenAI` server
//! to simulate API interactions without hitting the real `OpenAI` endpoints.

#![allow(clippy::significant_drop_tightening)]

use openai_ergonomic::test_utils::{assertions, fixtures, MockOpenAIServer};
use reqwest::Client;
use serde_json::{json, Value};

/// Test that the mock server can be created and responds correctly
#[tokio::test]
async fn test_mock_server_creation() {
    let mock_server = MockOpenAIServer::new().await;

    // Verify the server is running
    assert!(!mock_server.base_url().is_empty());
    assert!(mock_server.base_url().starts_with("http://"));
}

/// Test mock chat completions endpoint
#[tokio::test]
async fn test_mock_chat_completions_success() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_chat_completions_success().await;

    let client = Client::new();
    let request_body = fixtures::chat_completion_request();

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse response");
    assertions::assert_has_field(&body, "id");
    assertions::assert_has_field(&body, "choices");
    assertions::assert_success_response(&body);
}

/// Test mock streaming chat completions
#[tokio::test]
async fn test_mock_chat_completions_streaming() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_chat_completions_streaming().await;

    let client = Client::new();
    let mut request_body = fixtures::chat_completion_request();
    request_body["stream"] = json!(true);

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    let body = response.text().await.expect("Failed to read response");
    assert!(body.contains("data: "));
    assert!(body.contains("[DONE]"));
}

/// Test mock error responses
#[tokio::test]
async fn test_mock_error_responses() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server
        .mock_error_response(429, "rate_limit_exceeded", "Too many requests")
        .await;

    let client = Client::new();
    let request_body = fixtures::chat_completion_request();

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 429);

    let body: Value = response.json().await.expect("Failed to parse response");
    assertions::assert_has_field(&body, "error");
    let error = body.get("error").unwrap();
    assertions::assert_field_equals(error, "type", &json!("rate_limit_exceeded"));
    assertions::assert_field_equals(error, "message", &json!("Too many requests"));
}

/// Test mock embeddings endpoint
#[tokio::test]
async fn test_mock_embeddings_success() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_embeddings_success().await;

    let client = Client::new();
    let request_body = fixtures::embeddings_request();

    let response = client
        .post(format!("{}/v1/embeddings", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse response");
    assertions::assert_has_field(&body, "data");
    assertions::assert_has_field(&body, "usage");
    assertions::assert_success_response(&body);
}

/// Test mock models list endpoint
#[tokio::test]
async fn test_mock_models_list() {
    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_models_list().await;

    let client = Client::new();

    let response = client
        .get(format!("{}/v1/models", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse response");
    assertions::assert_has_field(&body, "data");
    assertions::assert_field_equals(&body, "object", &json!("list"));

    let models = body["data"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m["id"] == "gpt-4"));
}

/// Test fixture validation
#[test]
fn test_fixture_validation() {
    let chat_request = fixtures::chat_completion_request();
    assert!(chat_request["model"].is_string());
    assert!(chat_request["messages"].is_array());

    let embeddings_request = fixtures::embeddings_request();
    assert!(embeddings_request["model"].is_string());
    assert!(embeddings_request["input"].is_string());

    let tool_def = fixtures::tool_definition();
    assert_eq!(tool_def["type"], "function");
    assert!(tool_def["function"]["parameters"].is_object());
}

/// Test concurrent mock requests
#[tokio::test]
async fn test_concurrent_requests() {
    use futures::future::join_all;

    let mut mock_server = MockOpenAIServer::new().await;
    let _mock = mock_server.mock_chat_completions_success().await;

    let client = Client::new();
    let request_body = fixtures::chat_completion_request();

    let mut tasks = vec![];
    for _ in 0..5 {
        let client = client.clone();
        let url = format!("{}/v1/chat/completions", mock_server.base_url());
        let body = request_body.clone();

        tasks.push(async move {
            client
                .post(url)
                .header("Authorization", "Bearer test-api-key")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .expect("Failed to send request")
        });
    }

    let responses = join_all(tasks).await;

    for response in responses {
        assert_eq!(response.status(), 200);
    }
}

/// Test realistic API flow
#[tokio::test]
async fn test_realistic_api_flow() {
    let mut mock_server = MockOpenAIServer::new().await;

    // Step 1: List available models
    let _models_mock = mock_server.mock_models_list().await;

    // Step 2: Create chat completion
    let _chat_mock = mock_server.mock_chat_completions_success().await;

    // Step 3: Generate embeddings
    let _embeddings_mock = mock_server.mock_embeddings_success().await;

    let client = Client::new();

    // List models
    let models_response = client
        .get(format!("{}/v1/models", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .send()
        .await
        .expect("Failed to list models");
    assert_eq!(models_response.status(), 200);

    // Chat completion
    let chat_response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&fixtures::chat_completion_request())
        .send()
        .await
        .expect("Failed to create chat completion");
    assert_eq!(chat_response.status(), 200);

    // Embeddings
    let embeddings_response = client
        .post(format!("{}/v1/embeddings", mock_server.base_url()))
        .header("Authorization", "Bearer test-api-key")
        .header("Content-Type", "application/json")
        .json(&fixtures::embeddings_request())
        .send()
        .await
        .expect("Failed to create embeddings");
    assert_eq!(embeddings_response.status(), 200);
}
