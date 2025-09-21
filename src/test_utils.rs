//! Test utilities for mocking OpenAI API responses.
//!
//! This module provides testing utilities that are only available
//! with the `test-utils` feature flag enabled.

use mockito::{Mock, Server};
use serde_json::json;

/// Mock OpenAI server for testing.
pub struct MockOpenAIServer {
    server: Server,
    api_key: String,
}

impl MockOpenAIServer {
    /// Create a new mock server with a test API key.
    pub fn new() -> Self {
        Self::with_api_key("test-api-key")
    }

    /// Create a new mock server with a custom API key.
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            server: Server::new(),
            api_key: api_key.into(),
        }
    }

    /// Get the base URL for the mock server.
    pub fn base_url(&self) -> String {
        self.server.url()
    }

    /// Mock a successful chat completions response.
    pub fn mock_chat_completions_success(&mut self) -> Mock {
        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion",
                "created": 1_677_652_288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                }
            }).to_string())
            .create()
    }

    /// Mock a streaming chat completions response.
    pub fn mock_chat_completions_streaming(&mut self) -> Mock {
        let streaming_response = "data: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world!\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_header("cache-control", "no-cache")
            .with_header("connection", "keep-alive")
            .with_body(streaming_response)
            .create()
    }

    /// Mock an error response.
    pub fn mock_error_response(
        &mut self,
        status_code: u16,
        error_type: &str,
        message: &str,
    ) -> Mock {
        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(status_code as usize)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "error": {
                    "type": error_type,
                    "message": message,
                    "code": null
                }
            }).to_string())
            .create()
    }

    /// Mock embeddings endpoint.
    pub fn mock_embeddings_success(&mut self) -> Mock {
        self.server
            .mock("POST", "/v1/embeddings")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
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
            }).to_string())
            .create()
    }

    /// Mock models list endpoint.
    pub fn mock_models_list(&mut self) -> Mock {
        self.server
            .mock("GET", "/v1/models")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
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
            }).to_string())
            .create()
    }
}

impl Default for MockOpenAIServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Test fixtures for common OpenAI API payloads.
pub mod fixtures {
    use serde_json::{json, Value};

    /// Get a sample chat completion request payload.
    pub fn chat_completion_request() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7,
            "max_tokens": 150
        })
    }

    /// Get a sample embeddings request payload.
    pub fn embeddings_request() -> Value {
        json!({
            "model": "text-embedding-ada-002",
            "input": "Hello, world!"
        })
    }

    /// Get a sample tool/function definition.
    pub fn tool_definition() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the current weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state"
                        }
                    },
                    "required": ["location"]
                }
            }
        })
    }
}

/// Assertion helpers for testing API responses.
pub mod assertions {
    use serde_json::Value;

    /// Assert that a value has a specific field.
    pub fn assert_has_field(value: &Value, field: &str) {
        assert!(
            value.get(field).is_some(),
            "Expected field '{}' not found in response",
            field
        );
    }

    /// Assert that a value has a specific field with a specific value.
    pub fn assert_field_equals(value: &Value, field: &str, expected: &Value) {
        let actual = value
            .get(field)
            .unwrap_or_else(|| panic!("Field '{}' not found", field));
        assert_eq!(
            actual, expected,
            "Field '{}' has unexpected value",
            field
        );
    }

    /// Assert that a response has a successful status.
    pub fn assert_success_response(value: &Value) {
        assert!(
            value.get("error").is_none(),
            "Expected success but got error: {:?}",
            value.get("error")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_server_creation() {
        let server = MockOpenAIServer::new();
        assert!(!server.base_url().is_empty());
    }

    #[test]
    fn test_fixtures() {
        let request = fixtures::chat_completion_request();
        assert!(request.get("model").is_some());
        assert!(request.get("messages").is_some());
    }

    #[test]
    fn test_assertions() {
        let response = json!({"id": "test", "model": "gpt-4"});
        assertions::assert_has_field(&response, "id");
        assertions::assert_field_equals(&response, "model", &json!("gpt-4"));
    }
}