//! Test utilities for mocking `OpenAI` API responses.
//!
//! This module provides testing utilities that are only available
//! with the `test-utils` feature flag enabled.

use mockito::{Mock, ServerGuard};
use serde_json::json;

/// Mock `OpenAI` server for testing.
pub struct MockOpenAIServer {
    server: ServerGuard,
    api_key: String,
}

impl MockOpenAIServer {
    /// Create a new mock server with a test API key.
    pub async fn new() -> Self {
        Self::with_api_key("test-api-key").await
    }

    /// Create a new mock server with a custom API key.
    pub async fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            server: mockito::Server::new_async().await,
            api_key: api_key.into(),
        }
    }

    /// Get the base URL for the mock server.
    pub fn base_url(&self) -> String {
        self.server.url()
    }

    /// Mock a successful chat completions response.
    pub async fn mock_chat_completions_success(&mut self) -> Mock {
        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
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
                })
                .to_string(),
            )
            .create_async()
            .await
    }

    /// Mock a streaming chat completions response.
    pub async fn mock_chat_completions_streaming(&mut self) -> Mock {
        let streaming_response = "data: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-test123\",\"object\":\"chat.completion.chunk\",\"created\":1677652288,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world!\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";

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

    /// Mock an error response.
    pub async fn mock_error_response(
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
            .with_body(
                json!({
                    "error": {
                        "type": error_type,
                        "message": message,
                        "code": null
                    }
                })
                .to_string(),
            )
            .create_async()
            .await
    }

    /// Mock embeddings endpoint.
    pub async fn mock_embeddings_success(&mut self) -> Mock {
        self.server
            .mock("POST", "/v1/embeddings")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
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
                })
                .to_string(),
            )
            .create_async()
            .await
    }

    /// Mock models list endpoint.
    pub async fn mock_models_list(&mut self) -> Mock {
        self.server
            .mock("GET", "/v1/models")
            .match_header("authorization", format!("Bearer {}", self.api_key).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
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
                })
                .to_string(),
            )
            .create_async()
            .await
    }
}

/// Test fixtures for common `OpenAI` API payloads.
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
            "Expected field '{field}' not found in response"
        );
    }

    /// Assert that a value has a specific field with a specific value.
    pub fn assert_field_equals(value: &Value, field: &str, expected: &Value) {
        let actual = value
            .get(field)
            .unwrap_or_else(|| panic!("Field '{field}' not found"));
        assert_eq!(actual, expected, "Field '{field}' has unexpected value");
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

/// Additional test helpers for common testing patterns.
pub mod helpers {
    use crate::{
        builders::{
            chat::{tool_function, ChatCompletionBuilder},
            responses::{responses_tool_function, ResponsesBuilder},
            Builder,
        },
        Error,
    };
    use openai_client_base::models::CreateChatCompletionRequest;
    use serde_json::{json, Value};

    /// Create a minimal valid chat completion request for testing.
    pub fn minimal_chat_request() -> CreateChatCompletionRequest {
        ChatCompletionBuilder::new("gpt-4")
            .user("test message")
            .build()
            .expect("Failed to build minimal chat request")
    }

    /// Create a minimal valid responses request for testing.
    pub fn minimal_responses_request() -> CreateChatCompletionRequest {
        ResponsesBuilder::new("gpt-4")
            .user("test message")
            .build()
            .expect("Failed to build minimal responses request")
    }

    /// Create a complex chat request with multiple features for testing.
    pub fn complex_chat_request() -> CreateChatCompletionRequest {
        let tool = tool_function(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            }),
        );

        ChatCompletionBuilder::new("gpt-4")
            .system("You are a test assistant")
            .user("Test message")
            .temperature(0.7)
            .max_tokens(100)
            .tools(vec![tool])
            .build()
            .expect("Failed to build complex chat request")
    }

    /// Create a complex responses request with multiple features for testing.
    pub fn complex_responses_request() -> CreateChatCompletionRequest {
        let tool = responses_tool_function(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            }),
        );

        ResponsesBuilder::new("gpt-4")
            .system("You are a test assistant")
            .user("Test message")
            .temperature(0.7)
            .max_tokens(100)
            .tool(tool)
            .json_mode()
            .build()
            .expect("Failed to build complex responses request")
    }

    /// Test that a builder produces an error when built.
    pub fn assert_builder_error<T: std::fmt::Debug, B: Builder<T>>(
        builder: B,
        expected_error_contains: &str,
    ) {
        let result = builder.build();
        assert!(result.is_err(), "Expected builder to produce an error");

        let error = result.unwrap_err();
        let error_string = error.to_string();
        assert!(
            error_string.contains(expected_error_contains),
            "Error '{error_string}' does not contain expected text '{expected_error_contains}'"
        );
    }

    /// Test that a builder produces a successful result when built.
    pub fn assert_builder_success<T, B: Builder<T>>(builder: B) -> T {
        builder.build().expect("Expected builder to succeed")
    }

    /// Validate that a JSON value matches expected structure for chat completions.
    pub fn validate_chat_completion_structure(value: &Value) {
        assert!(value.is_object(), "Chat completion should be an object");

        let obj = value.as_object().unwrap();
        assert!(obj.contains_key("model"), "Should contain 'model' field");
        assert!(
            obj.contains_key("messages"),
            "Should contain 'messages' field"
        );

        let messages = obj.get("messages").unwrap();
        assert!(messages.is_array(), "Messages should be an array");
        assert!(
            !messages.as_array().unwrap().is_empty(),
            "Messages should not be empty"
        );
    }

    /// Generate test data for different error scenarios.
    pub fn error_test_cases() -> Vec<(&'static str, Error)> {
        vec![
            ("invalid_request", Error::InvalidRequest("test".to_string())),
            ("authentication", Error::Authentication("test".to_string())),
            ("rate_limit", Error::RateLimit("test".to_string())),
            ("api_400", Error::api(400, "Bad Request")),
            ("api_401", Error::api(401, "Unauthorized")),
            ("api_429", Error::api(429, "Too Many Requests")),
            ("api_500", Error::api(500, "Internal Server Error")),
            ("config", Error::Config("test".to_string())),
            ("builder", Error::Builder("test".to_string())),
            ("internal", Error::Internal("test".to_string())),
            ("stream", Error::Stream("test".to_string())),
        ]
    }

    /// Create a test schema for JSON schema testing.
    pub fn test_json_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name field"
                },
                "age": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "The age field"
                },
                "email": {
                    "type": "string",
                    "format": "email",
                    "description": "The email field"
                },
                "active": {
                    "type": "boolean",
                    "description": "Whether the person is active"
                }
            },
            "required": ["name", "email"],
            "additionalProperties": false
        })
    }

    /// Create test data for various parameter boundary testing.
    pub fn parameter_boundary_tests() -> Vec<(&'static str, f64, bool)> {
        vec![
            ("temperature_min", 0.0, true),
            ("temperature_max", 2.0, true),
            ("temperature_negative", -0.1, false),
            ("temperature_too_high", 2.1, false),
            ("top_p_min", 0.0, true),
            ("top_p_max", 1.0, true),
            ("top_p_negative", -0.1, false),
            ("top_p_too_high", 1.1, false),
            ("frequency_penalty_min", -2.0, true),
            ("frequency_penalty_max", 2.0, true),
            ("frequency_penalty_too_low", -2.1, false),
            ("frequency_penalty_too_high", 2.1, false),
            ("presence_penalty_min", -2.0, true),
            ("presence_penalty_max", 2.0, true),
            ("presence_penalty_too_low", -2.1, false),
            ("presence_penalty_too_high", 2.1, false),
        ]
    }
}

/// Performance testing utilities.
pub mod performance {
    use std::time::{Duration, Instant};

    /// Measure the time it takes to execute a function.
    pub fn measure_time<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// Assert that a function completes within a specified duration.
    pub fn assert_completes_within<F, R>(f: F, max_duration: Duration) -> R
    where
        F: FnOnce() -> R,
    {
        let (result, duration) = measure_time(f);
        assert!(
            duration <= max_duration,
            "Function took {duration:?} but should complete within {max_duration:?}"
        );
        result
    }

    /// Benchmark a function by running it multiple times and returning statistics.
    pub fn benchmark<F, R>(f: F, iterations: usize) -> BenchmarkResult
    where
        F: Fn() -> R,
    {
        let mut durations = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let (_, duration) = measure_time(&f);
            durations.push(duration);
        }

        durations.sort();

        let total: Duration = durations.iter().sum();
        let mean = total / u32::try_from(iterations).unwrap_or(1);
        let median = durations[iterations / 2];
        let min = durations[0];
        let max = durations[iterations - 1];

        BenchmarkResult {
            iterations,
            mean,
            median,
            min,
            max,
            total,
        }
    }

    /// Results from a benchmark run.
    #[derive(Debug, Clone)]
    pub struct BenchmarkResult {
        /// Number of iterations performed
        pub iterations: usize,
        /// Mean execution time
        pub mean: Duration,
        /// Median execution time
        pub median: Duration,
        /// Minimum execution time
        pub min: Duration,
        /// Maximum execution time
        pub max: Duration,
        /// Total execution time
        pub total: Duration,
    }

    impl BenchmarkResult {
        /// Check if the benchmark results meet performance criteria.
        pub fn meets_criteria(&self, max_mean: Duration, max_median: Duration) -> bool {
            self.mean <= max_mean && self.median <= max_median
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn test_mock_server_creation() {
        let server = MockOpenAIServer::new().await;
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

    #[test]
    fn test_helpers() {
        use helpers::*;

        // Test minimal requests
        let chat_req = minimal_chat_request();
        assert_eq!(chat_req.model, "gpt-4");
        assert_eq!(chat_req.messages.len(), 1);

        let responses_req = minimal_responses_request();
        assert_eq!(responses_req.model, "gpt-4");
        assert_eq!(responses_req.messages.len(), 1);

        // Test complex requests
        let complex_chat = complex_chat_request();
        assert_eq!(complex_chat.model, "gpt-4");
        assert_eq!(complex_chat.messages.len(), 2);
        assert!(complex_chat.tools.is_some());

        let complex_responses = complex_responses_request();
        assert_eq!(complex_responses.model, "gpt-4");
        assert_eq!(complex_responses.messages.len(), 2);
        assert!(complex_responses.tools.is_some());
        assert!(complex_responses.response_format.is_some());
    }

    #[test]
    fn test_error_test_cases() {
        let cases = helpers::error_test_cases();
        assert!(!cases.is_empty());

        for (name, error) in cases {
            assert!(!name.is_empty());
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn test_json_schema() {
        let schema = helpers::test_json_schema();
        assertions::assert_has_field(&schema, "type");
        assertions::assert_has_field(&schema, "properties");
        assertions::assert_has_field(&schema, "required");
    }

    #[test]
    fn test_parameter_boundary_tests() {
        let tests = helpers::parameter_boundary_tests();
        assert!(!tests.is_empty());

        for (name, value, _should_be_valid) in tests {
            assert!(!name.is_empty());
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_performance_utilities() {
        use performance::*;
        use std::time::Duration;

        // Test time measurement
        let (result, duration) = measure_time(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(10));

        // Test completion time assertion
        let result = assert_completes_within(|| 42, Duration::from_millis(100));
        assert_eq!(result, 42);

        // Test benchmarking
        let benchmark_result = benchmark(
            || {
                // Simulate some work
                (0..100).sum::<i32>()
            },
            5,
        );

        assert_eq!(benchmark_result.iterations, 5);
        assert!(benchmark_result.min <= benchmark_result.median);
        assert!(benchmark_result.median <= benchmark_result.max);
        assert!(benchmark_result.mean > Duration::ZERO);
    }
}
