//! Integration tests using mock server
//!
//! These tests validate the testing infrastructure by using the mock `OpenAI` server
//! to simulate API interactions without hitting the real `OpenAI` endpoints.

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

    // Verify API key is set
    assert_eq!(mock_server.api_key, "test-api-key");
}

/// Test mock chat completions endpoint
#[tokio::test]
async fn test_mock_chat_completions_success() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server.mock_chat_completions_success().await;

    let client = Client::new();
    let request_body = fixtures::sample_chat_completion_request();

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", format!("Bearer {}", mock_server.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Failed to parse response");

    // Validate the response structure
    assertions::assert_valid_chat_completion(&response_body);

    // Check specific response content
    assert_eq!(response_body["id"], "chatcmpl-test123");
    assert_eq!(response_body["model"], "gpt-4");
    assert_eq!(
        response_body["choices"][0]["message"]["content"],
        "Hello! I'm a test response from the mock server."
    );
}

/// Test mock streaming chat completions
#[tokio::test]
async fn test_mock_chat_completions_streaming() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server.mock_chat_completions_streaming().await;

    let client = Client::new();
    let mut request_body = fixtures::sample_chat_completion_request();

    // Add stream parameter
    request_body["stream"] = json!(true);

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", format!("Bearer {}", mock_server.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Verify content type is set (wiremock may not preserve exact header values)
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(!content_type.is_empty());

    let response_text = response.text().await.expect("Failed to get response text");

    // Verify streaming format
    assert!(response_text.contains("data: {"));
    assert!(response_text.contains("data: [DONE]"));
    assert!(response_text.contains("chat.completion.chunk"));
}

/// Test mock error responses
#[tokio::test]
async fn test_mock_error_responses() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server
        .mock_error_response(401, "invalid_request_error", "Invalid API key")
        .await;

    let client = Client::new();
    let request_body = fixtures::sample_chat_completion_request();

    let response = client
        .post(format!("{}/v1/chat/completions", mock_server.base_url()))
        .header("Authorization", format!("Bearer {}", mock_server.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 401);

    let error_response: Value = response
        .json()
        .await
        .expect("Failed to parse error response");

    assert_eq!(error_response["error"]["type"], "invalid_request_error");
    assert_eq!(error_response["error"]["message"], "Invalid API key");
}

/// Test mock embeddings endpoint
#[tokio::test]
async fn test_mock_embeddings_success() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server.mock_embeddings_success().await;

    let client = Client::new();
    let request_body = fixtures::sample_embedding_request();

    let response = client
        .post(format!("{}/v1/embeddings", mock_server.base_url()))
        .header("Authorization", format!("Bearer {}", mock_server.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Failed to parse response");

    // Validate the response structure
    assertions::assert_valid_embedding_response(&response_body);

    // Check specific response content
    assert_eq!(response_body["model"], "text-embedding-ada-002");
    let embedding = response_body["data"][0]["embedding"].as_array().unwrap();
    assert_eq!(embedding.len(), 5); // Our mock has 5 dimensions
}

/// Test mock models list endpoint
#[tokio::test]
async fn test_mock_models_list() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server.mock_models_list().await;

    let client = Client::new();

    let response = client
        .get(format!("{}/v1/models", mock_server.base_url()))
        .header("Authorization", format!("Bearer {}", mock_server.api_key))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(response_body["object"], "list");
    let models = response_body["data"].as_array().unwrap();
    assert!(!models.is_empty());

    // Check that we have both GPT-4 and GPT-3.5-turbo in the mock response
    let model_ids: Vec<&str> = models
        .iter()
        .map(|model| model["id"].as_str().unwrap())
        .collect();

    assert!(model_ids.contains(&"gpt-4"));
    assert!(model_ids.contains(&"gpt-3.5-turbo"));
}

/// Test fixture loading and validation
#[tokio::test]
async fn test_fixture_validation() {
    // Test chat completion fixtures
    let chat_fixtures = include_str!("../fixtures/chat_completions.json");
    let chat_data: Value =
        serde_json::from_str(chat_fixtures).expect("Failed to parse chat completions fixtures");

    assertions::assert_valid_chat_completion(&chat_data["success_response"]);

    // Test embeddings fixtures
    let embeddings_fixtures = include_str!("../fixtures/embeddings.json");
    let embeddings_data: Value =
        serde_json::from_str(embeddings_fixtures).expect("Failed to parse embeddings fixtures");

    assertions::assert_valid_embedding_response(&embeddings_data["success_response"]);

    // Test models fixtures
    let models_fixtures = include_str!("../fixtures/models.json");
    let models_data: Value =
        serde_json::from_str(models_fixtures).expect("Failed to parse models fixtures");

    assert_eq!(models_data["list_response"]["object"], "list");
    assert!(models_data["list_response"]["data"].is_array());
}

/// Test concurrent requests to mock server
#[tokio::test]
async fn test_concurrent_requests() {
    let mock_server = MockOpenAIServer::new().await;
    mock_server.mock_chat_completions_success().await;

    let client = Client::new();
    let request_body = fixtures::sample_chat_completion_request();

    // Send multiple concurrent requests
    let futures: Vec<_> = (0..5)
        .map(|_| {
            let client = client.clone();
            let url = format!("{}/v1/chat/completions", mock_server.base_url());
            let auth_header = format!("Bearer {}", mock_server.api_key);
            let body = request_body.clone();

            async move {
                client
                    .post(&url)
                    .header("Authorization", auth_header)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .expect("Failed to send request")
            }
        })
        .collect();

    let responses = futures::future::join_all(futures).await;

    // Verify all requests succeeded
    for response in responses {
        assert_eq!(response.status(), 200);
        let response_body: Value = response.json().await.expect("Failed to parse response");
        assertions::assert_valid_chat_completion(&response_body);
    }
}

/// Integration test demonstrating a realistic API interaction flow
#[tokio::test]
async fn test_realistic_api_flow() {
    let mock_server = MockOpenAIServer::new().await;

    // Set up mocks for different endpoints
    mock_server.mock_models_list().await;
    mock_server.mock_chat_completions_success().await;
    mock_server.mock_embeddings_success().await;

    let client = Client::new();
    let base_url = mock_server.base_url();
    let auth_header = format!("Bearer {}", mock_server.api_key);

    // Step 1: List available models
    let models_response = client
        .get(format!("{base_url}/v1/models"))
        .header("Authorization", &auth_header)
        .send()
        .await
        .expect("Failed to list models");

    assert_eq!(models_response.status(), 200);
    let models: Value = models_response.json().await.unwrap();
    assert!(!models["data"].as_array().unwrap().is_empty());

    // Step 2: Create chat completion
    let chat_request = json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello!"}],
        "max_tokens": 50
    });

    let chat_response = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .json(&chat_request)
        .send()
        .await
        .expect("Failed to create chat completion");

    assert_eq!(chat_response.status(), 200);
    let chat_result: Value = chat_response.json().await.unwrap();
    assertions::assert_valid_chat_completion(&chat_result);

    // Step 3: Generate embeddings
    let embeddings_request = json!({
        "model": "text-embedding-ada-002",
        "input": "Hello world"
    });

    let embeddings_response = client
        .post(format!("{base_url}/v1/embeddings"))
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .json(&embeddings_request)
        .send()
        .await
        .expect("Failed to create embeddings");

    assert_eq!(embeddings_response.status(), 200);
    let embeddings_result: Value = embeddings_response.json().await.unwrap();
    assertions::assert_valid_embedding_response(&embeddings_result);

    println!("✅ Realistic API flow test completed successfully");
}
