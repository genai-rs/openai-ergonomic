//! Mock OpenAI client implementation for testing.
//!
//! This module provides a comprehensive mock client that can simulate
//! various OpenAI API behaviors for testing purposes.

use mockito::{Mock, ServerGuard};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock OpenAI client for comprehensive testing scenarios.
pub struct MockOpenAIClient {
    server: ServerGuard,
    api_key: String,
    request_log: Arc<Mutex<Vec<MockRequest>>>,
    response_config: ResponseConfig,
}

/// Configuration for mock responses.
#[derive(Debug, Clone)]
pub struct ResponseConfig {
    pub default_model: String,
    pub default_completion_tokens: u32,
    pub default_prompt_tokens: u32,
    pub rate_limit_remaining: Option<u32>,
    pub rate_limit_reset: Option<u64>,
    pub custom_headers: HashMap<String, String>,
}

impl Default for ResponseConfig {
    fn default() -> Self {
        Self {
            default_model: "gpt-4".to_string(),
            default_completion_tokens: 20,
            default_prompt_tokens: 10,
            rate_limit_remaining: Some(4999),
            rate_limit_reset: Some(1677652288),
            custom_headers: HashMap::new(),
        }
    }
}

/// Logged request information for test validation.
#[derive(Debug, Clone)]
pub struct MockRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub timestamp: std::time::SystemTime,
}

impl MockOpenAIClient {
    /// Create a new mock client with default configuration.
    pub async fn new() -> Self {
        Self::with_config("test-api-key", ResponseConfig::default()).await
    }

    /// Create a mock client with custom API key.
    pub async fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::with_config(api_key, ResponseConfig::default()).await
    }

    /// Create a mock client with full configuration.
    pub async fn with_config(
        api_key: impl Into<String>,
        config: ResponseConfig,
    ) -> Self {
        Self {
            server: mockito::Server::new_async().await,
            api_key: api_key.into(),
            request_log: Arc::new(Mutex::new(Vec::new())),
            response_config: config,
        }
    }

    /// Get the base URL for the mock server.
    pub fn base_url(&self) -> String {
        self.server.url()
    }

    /// Get the API key being used.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Update response configuration.
    pub fn update_config(&mut self, config: ResponseConfig) {
        self.response_config = config;
    }

    /// Get logged requests for validation.
    pub async fn get_requests(&self) -> Vec<MockRequest> {
        self.request_log.lock().await.clone()
    }

    /// Clear the request log.
    pub async fn clear_requests(&self) {
        self.request_log.lock().await.clear();
    }

    /// Mock a successful chat completions response.
    pub async fn mock_chat_completions_success(&mut self) -> Mock {
        self.mock_chat_completions_success_with_content("Hello! How can I help you today?")
            .await
    }

    /// Mock a chat completions response with custom content.
    pub async fn mock_chat_completions_success_with_content(
        &mut self,
        content: &str,
    ) -> Mock {
        let mut mock = self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json");

        // Add rate limiting headers if configured
        if let Some(remaining) = self.response_config.rate_limit_remaining {
            mock = mock.with_header("x-ratelimit-remaining-requests", &remaining.to_string());
        }

        if let Some(reset) = self.response_config.rate_limit_reset {
            mock = mock.with_header("x-ratelimit-reset-requests", &reset.to_string());
        }

        // Add custom headers
        for (key, value) in &self.response_config.custom_headers {
            mock = mock.with_header(key, value);
        }

        let response_body = json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1_677_652_288,
            "model": self.response_config.default_model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": self.response_config.default_prompt_tokens,
                "completion_tokens": self.response_config.default_completion_tokens,
                "total_tokens": self.response_config.default_prompt_tokens + self.response_config.default_completion_tokens
            }
        });

        mock.with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock a streaming chat completions response.
    pub async fn mock_chat_completions_streaming(&mut self) -> Mock {
        self.mock_chat_completions_streaming_with_chunks(vec![
            "Hello",
            " there!",
            " How",
            " can",
            " I",
            " help",
            " you",
            " today?"
        ]).await
    }

    /// Mock streaming response with custom chunks.
    pub async fn mock_chat_completions_streaming_with_chunks(
        &mut self,
        chunks: Vec<&str>,
    ) -> Mock {
        let mut streaming_response = String::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_data = json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion.chunk",
                "created": 1_677_652_288,
                "model": self.response_config.default_model,
                "choices": [{
                    "index": 0,
                    "delta": {
                        "content": chunk
                    },
                    "finish_reason": if i == chunks.len() - 1 { "stop" } else { null }
                }]
            });

            streaming_response.push_str(&format!("data: {}\n\n", chunk_data));
        }

        streaming_response.push_str("data: [DONE]\n\n");

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_header("cache-control", "no-cache")
            .with_header("connection", "keep-alive")
            .with_body(streaming_response)
            .create_async()
            .await
    }

    /// Mock various error responses.
    pub async fn mock_error_response(
        &mut self,
        status_code: u16,
        error_type: &str,
        message: &str,
    ) -> Mock {
        self.mock_error_response_with_details(status_code, error_type, message, None, None)
            .await
    }

    /// Mock error response with additional details.
    pub async fn mock_error_response_with_details(
        &mut self,
        status_code: u16,
        error_type: &str,
        message: &str,
        code: Option<&str>,
        param: Option<&str>,
    ) -> Mock {
        let mut error_obj = json!({
            "type": error_type,
            "message": message
        });

        if let Some(code_val) = code {
            error_obj["code"] = json!(code_val);
        }

        if let Some(param_val) = param {
            error_obj["param"] = json!(param_val);
        }

        let response_body = json!({
            "error": error_obj
        });

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(status_code as usize)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock rate limit error.
    pub async fn mock_rate_limit_error(&mut self) -> Mock {
        self.mock_error_response_with_details(
            429,
            "rate_limit_exceeded",
            "Rate limit exceeded. Please try again later.",
            Some("rate_limit_exceeded"),
            None,
        ).await
    }

    /// Mock authentication error.
    pub async fn mock_auth_error(&mut self) -> Mock {
        self.mock_error_response_with_details(
            401,
            "invalid_api_key",
            "Invalid API key provided",
            Some("invalid_api_key"),
            None,
        ).await
    }

    /// Mock validation error.
    pub async fn mock_validation_error(&mut self, param: &str) -> Mock {
        self.mock_error_response_with_details(
            400,
            "invalid_request_error",
            &format!("Invalid parameter: {param}"),
            Some("invalid_request_error"),
            Some(param),
        ).await
    }

    /// Mock embeddings endpoint.
    pub async fn mock_embeddings_success(&mut self) -> Mock {
        self.mock_embeddings_success_with_dimensions(1536).await
    }

    /// Mock embeddings with custom dimensions.
    pub async fn mock_embeddings_success_with_dimensions(&mut self, dimensions: usize) -> Mock {
        let embedding: Vec<f64> = (0..dimensions).map(|i| (i as f64) * 0.1 - 0.5).collect();

        let response_body = json!({
            "object": "list",
            "data": [{
                "object": "embedding",
                "index": 0,
                "embedding": embedding
            }],
            "model": "text-embedding-ada-002",
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        });

        self.server
            .mock("POST", "/v1/embeddings")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock models list endpoint.
    pub async fn mock_models_list(&mut self) -> Mock {
        self.mock_models_list_with_models(vec![
            ("gpt-4", "openai"),
            ("gpt-3.5-turbo", "openai"),
            ("text-embedding-ada-002", "openai"),
        ]).await
    }

    /// Mock models list with custom models.
    pub async fn mock_models_list_with_models(&mut self, models: Vec<(&str, &str)>) -> Mock {
        let model_data: Vec<Value> = models
            .into_iter()
            .map(|(id, owned_by)| {
                json!({
                    "id": id,
                    "object": "model",
                    "created": 1_677_610_602,
                    "owned_by": owned_by
                })
            })
            .collect();

        let response_body = json!({
            "object": "list",
            "data": model_data
        });

        self.server
            .mock("GET", "/v1/models")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock a specific model details endpoint.
    pub async fn mock_model_details(&mut self, model_id: &str) -> Mock {
        let response_body = json!({
            "id": model_id,
            "object": "model",
            "created": 1_677_610_602,
            "owned_by": "openai"
        });

        self.server
            .mock("GET", &format!("/v1/models/{model_id}"))
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock function calling response.
    pub async fn mock_function_calling_response(&mut self, function_name: &str, arguments: Value) -> Mock {
        let response_body = json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1_677_652_288,
            "model": self.response_config.default_model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test123",
                        "type": "function",
                        "function": {
                            "name": function_name,
                            "arguments": arguments.to_string()
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": self.response_config.default_prompt_tokens,
                "completion_tokens": self.response_config.default_completion_tokens,
                "total_tokens": self.response_config.default_prompt_tokens + self.response_config.default_completion_tokens
            }
        });

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock JSON mode response.
    pub async fn mock_json_mode_response(&mut self, json_content: Value) -> Mock {
        let response_body = json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1_677_652_288,
            "model": self.response_config.default_model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": json_content.to_string()
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": self.response_config.default_prompt_tokens,
                "completion_tokens": self.response_config.default_completion_tokens,
                "total_tokens": self.response_config.default_prompt_tokens + self.response_config.default_completion_tokens
            }
        });

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response_body.to_string())
            .create_async()
            .await
    }

    /// Mock server unavailable error.
    pub async fn mock_server_error(&mut self) -> Mock {
        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(503)
            .with_header("content-type", "application/json")
            .with_body(json!({"error": {"type": "server_error", "message": "Service temporarily unavailable"}}).to_string())
            .create_async()
            .await
    }

    /// Mock network timeout by adding delay.
    pub async fn mock_slow_response(&mut self, delay_ms: u64) -> Mock {
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        self.mock_chat_completions_success().await
    }

    /// Set up multiple sequential mocks for testing retry logic.
    pub async fn setup_retry_scenario(&mut self) -> (Mock, Mock, Mock) {
        let error_mock1 = self.mock_rate_limit_error().await;
        let error_mock2 = self.mock_server_error().await;
        let success_mock = self.mock_chat_completions_success().await;

        (error_mock1, error_mock2, success_mock)
    }
}

/// Builder for creating mock client configurations.
pub struct MockClientBuilder {
    api_key: String,
    config: ResponseConfig,
}

impl MockClientBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            api_key: "test-api-key".to_string(),
            config: ResponseConfig::default(),
        }
    }

    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// Set the default model.
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.config.default_model = model.into();
        self
    }

    /// Set default token counts.
    pub fn token_counts(mut self, prompt_tokens: u32, completion_tokens: u32) -> Self {
        self.config.default_prompt_tokens = prompt_tokens;
        self.config.default_completion_tokens = completion_tokens;
        self
    }

    /// Set rate limiting headers.
    pub fn rate_limits(mut self, remaining: Option<u32>, reset: Option<u64>) -> Self {
        self.config.rate_limit_remaining = remaining;
        self.config.rate_limit_reset = reset;
        self
    }

    /// Add custom headers.
    pub fn custom_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.custom_headers.insert(key.into(), value.into());
        self
    }

    /// Build the mock client.
    pub async fn build(self) -> MockOpenAIClient {
        MockOpenAIClient::with_config(self.api_key, self.config).await
    }
}

impl Default for MockClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_creation() {
        let client = MockOpenAIClient::new().await;
        assert!(!client.base_url().is_empty());
        assert_eq!(client.api_key(), "test-api-key");
    }

    #[tokio::test]
    async fn test_mock_client_builder() {
        let client = MockClientBuilder::new()
            .api_key("custom-key")
            .default_model("gpt-3.5-turbo")
            .token_counts(15, 25)
            .custom_header("x-test", "value")
            .build()
            .await;

        assert_eq!(client.api_key(), "custom-key");
        assert_eq!(client.response_config.default_model, "gpt-3.5-turbo");
        assert_eq!(client.response_config.default_prompt_tokens, 15);
        assert_eq!(client.response_config.default_completion_tokens, 25);
        assert_eq!(client.response_config.custom_headers.get("x-test"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_request_logging() {
        let client = MockOpenAIClient::new().await;
        let requests = client.get_requests().await;
        assert!(requests.is_empty());
    }
}