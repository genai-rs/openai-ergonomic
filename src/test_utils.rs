//! Test utilities and mock helpers for openai-ergonomic
//!
//! This module provides utilities for testing, including mock server setup,
//! test fixtures, and common test helpers.

use serde_json::json;

#[cfg(feature = "test-utils")]
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[cfg(feature = "test-utils")]
/// Mock `OpenAI` API server for testing
pub struct MockOpenAIServer {
    /// The underlying mock server instance
    pub server: MockServer,
    /// API key used for authentication in tests
    pub api_key: String,
}

#[cfg(feature = "test-utils")]
impl MockOpenAIServer {
    /// Create a new mock `OpenAI` server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let api_key = "test-api-key".to_string();

        Self { server, api_key }
    }

    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock a successful chat completions response
    pub async fn mock_chat_completions_success(&self) -> &Self {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", format!("Bearer {}", self.api_key).as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion",
                "created": 1_677_652_288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! I'm a test response from the mock server."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 9,
                    "completion_tokens": 12,
                    "total_tokens": 21
                }
            })))
            .mount(&self.server)
            .await;

        self
    }

    /// Mock a streaming chat completions response
    pub async fn mock_chat_completions_streaming(&self) -> &Self {
        let streaming_response = "data: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world!\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", format!("Bearer {}", self.api_key).as_str()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(streaming_response)
                    .insert_header("content-type", "text/event-stream")
                    .insert_header("cache-control", "no-cache")
                    .insert_header("connection", "keep-alive"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock an error response
    pub async fn mock_error_response(
        &self,
        status_code: u16,
        error_type: &str,
        message: &str,
    ) -> &Self {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", format!("Bearer {}", self.api_key).as_str()))
            .respond_with(ResponseTemplate::new(status_code).set_body_json(json!({
                "error": {
                    "type": error_type,
                    "message": message,
                    "code": null
                }
            })))
            .mount(&self.server)
            .await;

        self
    }

    /// Mock embeddings endpoint
    pub async fn mock_embeddings_success(&self) -> &Self {
        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .and(header("authorization", format!("Bearer {}", self.api_key).as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [{
                    "object": "embedding",
                    "index": 0,
                    "embedding": [0.1, 0.2, 0.3, -0.1, -0.2]
                }],
                "model": "text-embedding-ada-002",
                "usage": {
                    "prompt_tokens": 5,
                    "total_tokens": 5
                }
            })))
            .mount(&self.server)
            .await;

        self
    }

    /// Mock models list endpoint
    pub async fn mock_models_list(&self) -> &Self {
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", format!("Bearer {}", self.api_key).as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [
                    {
                        "id": "gpt-4",
                        "object": "model",
                        "created": 1_677_610_602,
                        "owned_by": "openai"
                    },
                    {
                        "id": "gpt-3.5-turbo",
                        "object": "model",
                        "created": 1_677_610_602,
                        "owned_by": "openai"
                    }
                ]
            })))
            .mount(&self.server)
            .await;

        self
    }
}

/// Common test fixtures and utilities
pub mod fixtures {
    use serde_json::{json, Value};

    /// Sample chat message for testing
    pub fn sample_chat_message() -> Value {
        json!({
            "role": "user",
            "content": "Hello, how are you?"
        })
    }

    /// Sample chat completion request
    pub fn sample_chat_completion_request() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [sample_chat_message()],
            "max_tokens": 100,
            "temperature": 0.7
        })
    }

    /// Sample embedding request
    pub fn sample_embedding_request() -> Value {
        json!({
            "model": "text-embedding-ada-002",
            "input": "Sample text for embedding"
        })
    }

    /// Sample function definition for tool calling
    pub fn sample_function_definition() -> Value {
        json!({
            "name": "get_weather",
            "description": "Get the current weather for a location",
            "parameters": {
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
            }
        })
    }
}

/// Test assertion helpers
pub mod assertions {
    use serde_json::Value;

    /// Assert that a response contains expected fields
    pub fn assert_has_required_fields(response: &Value, fields: &[&str]) {
        for field in fields {
            assert!(
                response.get(field).is_some(),
                "Response missing required field: {field}"
            );
        }
    }

    /// Assert that a chat completion response is valid
    pub fn assert_valid_chat_completion(response: &Value) {
        assert_has_required_fields(response, &["id", "object", "created", "model", "choices"]);

        let choices = response["choices"]
            .as_array()
            .expect("choices should be an array");
        assert!(
            !choices.is_empty(),
            "Response should have at least one choice"
        );

        let first_choice = &choices[0];
        assert_has_required_fields(first_choice, &["index", "message"]);

        let message = &first_choice["message"];
        assert_has_required_fields(message, &["role", "content"]);
    }

    /// Assert that an embedding response is valid
    pub fn assert_valid_embedding_response(response: &Value) {
        assert_has_required_fields(response, &["object", "data", "model", "usage"]);

        let data = response["data"]
            .as_array()
            .expect("data should be an array");
        assert!(
            !data.is_empty(),
            "Response should have at least one embedding"
        );

        let first_embedding = &data[0];
        assert_has_required_fields(first_embedding, &["object", "index", "embedding"]);

        let embedding = first_embedding["embedding"]
            .as_array()
            .expect("embedding should be an array");
        assert!(!embedding.is_empty(), "Embedding should not be empty");
    }
}

#[cfg(all(test, feature = "test-utils"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_creation() {
        let mock_server = MockOpenAIServer::new().await;
        assert!(!mock_server.base_url().is_empty());
        assert_eq!(mock_server.api_key, "test-api-key");
    }

    #[test]
    fn test_fixtures() {
        let message = fixtures::sample_chat_message();
        assert_eq!(message["role"], "user");
        assert!(message["content"].is_string());

        let request = fixtures::sample_chat_completion_request();
        assert_eq!(request["model"], "gpt-4");
        assert!(request["messages"].is_array());
    }

    #[test]
    fn test_assertions() {
        let valid_response = json!({
            "id": "test",
            "object": "chat.completion",
            "created": 123_456,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello"
                }
            }]
        });

        assertions::assert_valid_chat_completion(&valid_response);
    }
}
