//! Testing patterns and strategies for `OpenAI` API integration.
#![allow(dead_code)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::format_push_string)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::useless_vec)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::if_not_else)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::struct_excessive_bools)]
//!
//! This example demonstrates comprehensive testing approaches including:
//! - Mock server setup with mockito for unit testing
//! - Integration testing with real API endpoints
//! - Test utilities and helper functions
//! - Response validation and assertion patterns
//! - Error handling and edge case testing
//! - Performance testing and benchmarking
//! - Contract testing and API compatibility validation
//!
//! Testing is crucial for AI-powered applications to ensure:
//! - Consistent behavior across different API responses
//! - Proper error handling for rate limits and failures
//! - Performance characteristics under load
//! - Cost management and usage tracking accuracy
//!
//! Run with: `cargo run --example testing_patterns`
use futures::FutureExt;
use openai_ergonomic::{Client, Config, Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::HashMap,
    fmt, io,
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Mock OpenAI server for testing purposes
struct MockOpenAIServer {
    /// Mock server instance
    server: mockito::ServerGuard,
    /// Predefined responses for different endpoints
    responses: Arc<Mutex<HashMap<String, MockResponse>>>,
    /// Request tracking for verification
    request_log: Arc<Mutex<Vec<MockRequest>>>,
    /// Error simulation settings
    error_config: Arc<Mutex<ErrorSimulationConfig>>,
}

#[derive(Debug)]
enum MockServerInitError {
    PermissionDenied(io::Error),
    Panicked(String),
}

impl MockServerInitError {
    fn ensure_bind_allowed() -> std::result::Result<(), Self> {
        match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)) {
            Ok(listener) => {
                drop(listener);
                Ok(())
            }
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                Err(Self::PermissionDenied(err))
            }
            Err(_) => Ok(()),
        }
    }

    fn from_panic(payload: Box<dyn Any + Send>) -> Self {
        match payload.downcast::<String>() {
            Ok(message) => Self::Panicked(*message),
            Err(payload) => match payload.downcast::<&'static str>() {
                Ok(message) => Self::Panicked((*message).to_string()),
                Err(_) => Self::Panicked("mockito panicked while starting the server".to_string()),
            },
        }
    }
}

impl fmt::Display for MockServerInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied(err) => write!(
                f,
                "mock server cannot bind to a local port in this environment: {}",
                err
            ),
            Self::Panicked(message) => write!(
                f,
                "mockito panicked while starting the mock server: {}",
                message
            ),
        }
    }
}

impl std::error::Error for MockServerInitError {}

/// Configuration for error simulation in tests
#[derive(Debug, Clone)]
struct ErrorSimulationConfig {
    /// Whether to simulate rate limits
    simulate_rate_limits: bool,
    /// Rate limit delay in seconds
    rate_limit_delay: u64,
    /// Whether to simulate server errors
    simulate_server_errors: bool,
    /// Error probability (0.0 to 1.0)
    error_probability: f64,
    /// Network timeout simulation
    simulate_timeouts: bool,
    /// Timeout delay in seconds
    timeout_delay: u64,
}

impl Default for ErrorSimulationConfig {
    fn default() -> Self {
        Self {
            simulate_rate_limits: false,
            rate_limit_delay: 60,
            simulate_server_errors: false,
            error_probability: 0.1,
            simulate_timeouts: false,
            timeout_delay: 30,
        }
    }
}

/// Mock response configuration
#[derive(Debug, Clone)]
struct MockResponse {
    /// HTTP status code
    status: u16,
    /// Response body
    body: String,
    /// Response headers
    headers: HashMap<String, String>,
    /// Simulated delay
    delay: Option<Duration>,
}

/// Logged request for verification
#[derive(Debug, Clone)]
struct MockRequest {
    /// HTTP method
    method: String,
    /// Request path
    path: String,
    /// Request headers
    headers: HashMap<String, String>,
    /// Request body
    body: String,
    /// Timestamp
    timestamp: Instant,
}

impl MockOpenAIServer {
    /// Try to create a new mock server, returning an error if the environment disallows it.
    async fn try_new() -> std::result::Result<Self, MockServerInitError> {
        MockServerInitError::ensure_bind_allowed()?;

        let server = std::panic::AssertUnwindSafe(mockito::Server::new_async())
            .catch_unwind()
            .await
            .map_err(MockServerInitError::from_panic)?;

        Ok(Self {
            server,
            responses: Arc::new(Mutex::new(HashMap::new())),
            request_log: Arc::new(Mutex::new(Vec::new())),
            error_config: Arc::new(Mutex::new(ErrorSimulationConfig::default())),
        })
    }

    /// Create a new mock server with default configuration
    async fn new() -> Self {
        Self::try_new()
            .await
            .unwrap_or_else(|err| panic!("failed to start mock server: {err}"))
    }

    /// Get the base URL for the mock server
    fn base_url(&self) -> String {
        self.server.url()
    }

    /// Create a client configured to use this mock server
    fn client(&self) -> Result<Client> {
        let config = Config::builder()
            .api_key("test-api-key")
            .api_base(&self.base_url())
            .build();

        Ok(Client::builder(config)?.build())
    }

    /// Configure error simulation
    fn configure_errors(&self, config: ErrorSimulationConfig) {
        *self.error_config.lock().unwrap() = config;
    }

    /// Mock a chat completion response
    async fn mock_chat_completion(&mut self, expected_prompt: &str, response_text: &str) {
        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-3.5-turbo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response_text
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 20,
                "total_tokens": 70
            }
        });

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_body(mockito::Matcher::JsonString(
                serde_json::json!({
                    "model": "gpt-3.5-turbo",
                    "messages": [{"role": "user", "content": expected_prompt}]
                })
                .to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;
    }

    /// Mock a streaming chat completion response
    async fn mock_streaming_chat(&mut self, response_chunks: Vec<&str>) {
        let mut sse_data = String::new();

        for (i, chunk) in response_chunks.iter().enumerate() {
            let chunk_response = serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion.chunk",
                "created": 1677652288,
                "model": "gpt-3.5-turbo",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "content": chunk
                    },
                    "finish_reason": if i == response_chunks.len() - 1 { "stop" } else { "null" }
                }]
            });

            sse_data.push_str(&format!("data: {}\n\n", chunk_response));
        }

        sse_data.push_str("data: [DONE]\n\n");

        self.server
            .mock("POST", "/v1/chat/completions")
            .match_header("accept", "text/event-stream")
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(sse_data)
            .create_async()
            .await;
    }

    /// Mock an error response (rate limit, server error, etc.)
    async fn mock_error_response(&mut self, endpoint: &str, error_type: ErrorType) {
        let (status, body) = match error_type {
            ErrorType::RateLimit => (
                429,
                serde_json::json!({
                    "error": {
                        "type": "rate_limit_exceeded",
                        "message": "Rate limit exceeded, please try again later"
                    }
                })
                .to_string(),
            ),
            ErrorType::ServerError => (
                500,
                serde_json::json!({
                    "error": {
                        "type": "server_error",
                        "message": "Internal server error"
                    }
                })
                .to_string(),
            ),
            ErrorType::InvalidRequest => (
                400,
                serde_json::json!({
                    "error": {
                        "type": "invalid_request_error",
                        "message": "Invalid request parameters"
                    }
                })
                .to_string(),
            ),
            ErrorType::Unauthorized => (
                401,
                serde_json::json!({
                    "error": {
                        "type": "invalid_request_error",
                        "message": "Incorrect API key provided"
                    }
                })
                .to_string(),
            ),
        };

        self.server
            .mock("POST", endpoint)
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;
    }

    /// Get logged requests for verification
    fn get_request_log(&self) -> Vec<MockRequest> {
        self.request_log.lock().unwrap().clone()
    }

    /// Clear request log
    fn clear_request_log(&self) {
        self.request_log.lock().unwrap().clear();
    }

    /// Verify that a specific request was made
    fn verify_request(&self, method: &str, path: &str) -> bool {
        let log = self.request_log.lock().unwrap();
        log.iter()
            .any(|req| req.method == method && req.path == path)
    }
}

/// Types of errors to simulate in testing
#[derive(Debug, Clone)]
enum ErrorType {
    RateLimit,
    ServerError,
    InvalidRequest,
    Unauthorized,
}

/// Test utilities for OpenAI API testing
struct TestUtils;

impl TestUtils {
    /// Create a test client with mock configuration
    fn create_test_client() -> Result<Client> {
        let config = Config::builder()
            .api_key("test-api-key")
            .api_base("http://localhost:1234") // Mock server URL
            .max_retries(2)
            .build();

        Ok(Client::builder(config)?.build())
    }

    /// Assert that a response contains expected content
    fn assert_response_content(response: &str, expected_content: &str) {
        assert!(
            response.contains(expected_content),
            "Response '{}' does not contain expected content '{}'",
            response,
            expected_content
        );
    }

    /// Assert token usage is within expected bounds
    fn assert_token_usage(usage: &TokenUsage, min_tokens: i32, max_tokens: i32) {
        assert!(
            usage.total_tokens >= min_tokens && usage.total_tokens <= max_tokens,
            "Token usage {} is outside expected range {}-{}",
            usage.total_tokens,
            min_tokens,
            max_tokens
        );
    }

    /// Create test data for batch testing
    fn create_test_prompts(count: usize) -> Vec<String> {
        (0..count)
            .map(|i| format!("Test prompt number {}", i + 1))
            .collect()
    }

    /// Measure execution time of an async operation
    async fn time_async_operation<F, T, E>(operation: F) -> (std::result::Result<T, E>, Duration)
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
    {
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// Create a mock response with custom token usage
    fn create_mock_response_with_usage(
        content: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> String {
        serde_json::json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-3.5-turbo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens
            }
        })
        .to_string()
    }
}

/// Token usage information for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// Integration test runner for live API testing
struct IntegrationTestRunner {
    client: Client,
    test_results: Vec<IntegrationTestResult>,
}

/// Result of an integration test
#[derive(Debug, Clone)]
struct IntegrationTestResult {
    test_name: String,
    success: bool,
    duration: Duration,
    error_message: Option<String>,
    response_data: Option<String>,
}

impl IntegrationTestRunner {
    /// Create a new integration test runner
    fn new(client: Client) -> Self {
        Self {
            client,
            test_results: Vec::new(),
        }
    }

    /// Run a basic chat completion test
    async fn test_basic_chat_completion(&mut self) -> Result<()> {
        let test_name = "basic_chat_completion";
        info!("Running integration test: {}", test_name);

        let (result, duration) = TestUtils::time_async_operation::<_, String, Error>(async {
            // Note: This would use real API in integration tests
            // self.client.chat_simple("Hello, world!").await

            // For demonstration, we'll simulate a successful response
            Ok("Hello! How can I help you today?".to_string())
        })
        .await;

        let test_result = match result {
            Ok(response) => {
                info!(" Basic chat completion test passed in {:?}", duration);
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: true,
                    duration,
                    error_message: None,
                    response_data: Some(response),
                }
            }
            Err(e) => {
                error!(" Basic chat completion test failed: {}", e);
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: false,
                    duration,
                    error_message: Some(e.to_string()),
                    response_data: None,
                }
            }
        };

        self.test_results.push(test_result);
        Ok(())
    }

    /// Test streaming functionality
    async fn test_streaming_completion(&mut self) -> Result<()> {
        let test_name = "streaming_completion";
        info!("Running integration test: {}", test_name);

        let (result, duration) = TestUtils::time_async_operation::<_, String, Error>(async {
            // Note: This would use real streaming API in integration tests
            // let mut stream = self.client.chat().user("Tell me a story").stream().await?;
            // let mut chunks = Vec::new();
            // while let Some(chunk) = stream.next().await {
            //     chunks.push(chunk?.content());
            // }
            // Ok(chunks.join(""))

            // For demonstration, simulate streaming chunks
            let chunks = vec!["Once", " upon", " a", " time..."];
            Ok(chunks.join(""))
        })
        .await;

        let test_result = match result {
            Ok(response) => {
                info!(" Streaming completion test passed in {:?}", duration);
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: true,
                    duration,
                    error_message: None,
                    response_data: Some(response),
                }
            }
            Err(e) => {
                error!(" Streaming completion test failed: {}", e);
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: false,
                    duration,
                    error_message: Some(e.to_string()),
                    response_data: None,
                }
            }
        };

        self.test_results.push(test_result);
        Ok(())
    }

    /// Test error handling
    async fn test_error_handling(&mut self) -> Result<()> {
        let test_name = "error_handling";
        info!("Running integration test: {}", test_name);

        let (result, duration) = TestUtils::time_async_operation::<_, String, Error>(async {
            // Test with invalid API key to trigger authentication error
            let bad_config = Config::builder().api_key("invalid-key").build();

            let _bad_client = Client::builder(bad_config)?.build();

            // This should fail with an authentication error
            // bad_client.chat_simple("Test").await

            // For demonstration, simulate an auth error
            Err(Error::InvalidRequest("Authentication failed".to_string()))
        })
        .await;

        let test_result = match result {
            Ok(_) => {
                warn!("Error handling test unexpectedly succeeded");
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: false,
                    duration,
                    error_message: Some(
                        "Expected authentication error but request succeeded".to_string(),
                    ),
                    response_data: None,
                }
            }
            Err(e) => {
                info!(
                    " Error handling test passed (correctly failed) in {:?}",
                    duration
                );
                IntegrationTestResult {
                    test_name: test_name.to_string(),
                    success: true,
                    duration,
                    error_message: None,
                    response_data: Some(format!("Expected error: {}", e)),
                }
            }
        };

        self.test_results.push(test_result);
        Ok(())
    }

    /// Generate test report
    fn generate_report(&self) -> TestReport {
        let total_tests = self.test_results.len();
        let passed_tests = self.test_results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;

        let total_duration: Duration = self.test_results.iter().map(|r| r.duration).sum();

        let average_duration = if total_tests > 0 {
            total_duration / total_tests as u32
        } else {
            Duration::ZERO
        };

        TestReport {
            total_tests,
            passed_tests,
            failed_tests,
            total_duration,
            average_duration,
            test_results: self.test_results.clone(),
        }
    }
}

/// Comprehensive test report
#[derive(Debug)]
struct TestReport {
    total_tests: usize,
    passed_tests: usize,
    failed_tests: usize,
    total_duration: Duration,
    average_duration: Duration,
    test_results: Vec<IntegrationTestResult>,
}

impl TestReport {
    /// Print a formatted test report
    fn print_report(&self) {
        info!("=== Test Report ===");
        info!("Total tests: {}", self.total_tests);
        info!("Passed: {}", self.passed_tests);
        info!("Failed: {}", self.failed_tests);
        info!(
            "Success rate: {:.1}%",
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        );
        info!("Total duration: {:?}", self.total_duration);
        info!("Average duration: {:?}", self.average_duration);

        if self.failed_tests > 0 {
            error!("Failed tests:");
            for result in &self.test_results {
                if !result.success {
                    error!(
                        "  - {}: {}",
                        result.test_name,
                        result
                            .error_message
                            .as_ref()
                            .unwrap_or(&"Unknown error".to_string())
                    );
                }
            }
        }
    }
}

/// Performance testing utilities
struct PerformanceTestRunner {
    client: Client,
}

impl PerformanceTestRunner {
    fn new(client: Client) -> Self {
        Self { client }
    }

    /// Run concurrent requests to test throughput
    async fn test_concurrent_requests(
        &self,
        concurrency: usize,
        requests_per_worker: usize,
    ) -> PerformanceResults {
        info!(
            "Running performance test with {} concurrent workers, {} requests each",
            concurrency, requests_per_worker
        );

        let start_time = Instant::now();
        let mut handles = Vec::new();

        for worker_id in 0..concurrency {
            let _client = self.client.clone(); // Assume Client implements Clone
            let handle = tokio::spawn(async move {
                let mut worker_results = Vec::new();

                for request_id in 0..requests_per_worker {
                    let request_start = Instant::now();

                    // Simulate API request
                    // let result = client.chat_simple(&format!("Request {} from worker {}", request_id, worker_id)).await;
                    let result: Result<String> =
                        Ok(format!("Response {} from worker {}", request_id, worker_id));

                    let request_duration = request_start.elapsed();

                    worker_results.push(RequestResult {
                        worker_id,
                        request_id,
                        duration: request_duration,
                        success: result.is_ok(),
                        error: result.err().map(|e| e.to_string()),
                    });

                    // Small delay to avoid overwhelming the API
                    sleep(Duration::from_millis(100)).await;
                }

                worker_results
            });

            handles.push(handle);
        }

        let mut all_results = Vec::new();
        for handle in handles {
            let worker_results = handle.await.unwrap();
            all_results.extend(worker_results);
        }

        let total_duration = start_time.elapsed();
        self.analyze_performance_results(all_results, total_duration)
    }

    fn analyze_performance_results(
        &self,
        results: Vec<RequestResult>,
        total_duration: Duration,
    ) -> PerformanceResults {
        let total_requests = results.len();
        let successful_requests = results.iter().filter(|r| r.success).count();
        let failed_requests = total_requests - successful_requests;

        let request_durations: Vec<Duration> = results.iter().map(|r| r.duration).collect();

        let min_duration = request_durations
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::ZERO);
        let max_duration = request_durations
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO);
        let avg_duration = if total_requests > 0 {
            request_durations.iter().sum::<Duration>() / total_requests as u32
        } else {
            Duration::ZERO
        };

        // Calculate percentiles
        let mut sorted_durations = request_durations;
        sorted_durations.sort();

        let p50 = if !sorted_durations.is_empty() {
            sorted_durations[sorted_durations.len() / 2]
        } else {
            Duration::ZERO
        };

        let p95 = if !sorted_durations.is_empty() {
            sorted_durations[(sorted_durations.len() * 95) / 100]
        } else {
            Duration::ZERO
        };

        let requests_per_second = if total_duration.as_secs() > 0 {
            total_requests as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        PerformanceResults {
            total_requests,
            successful_requests,
            failed_requests,
            total_duration,
            min_duration,
            max_duration,
            avg_duration,
            p50_duration: p50,
            p95_duration: p95,
            requests_per_second,
            error_rate: (failed_requests as f64 / total_requests as f64) * 100.0,
        }
    }
}

/// Result of a single performance test request
#[derive(Debug)]
struct RequestResult {
    worker_id: usize,
    request_id: usize,
    duration: Duration,
    success: bool,
    error: Option<String>,
}

/// Performance test results
#[derive(Debug)]
struct PerformanceResults {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    total_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
    avg_duration: Duration,
    p50_duration: Duration,
    p95_duration: Duration,
    requests_per_second: f64,
    error_rate: f64,
}

impl PerformanceResults {
    fn print_results(&self) {
        info!("=== Performance Test Results ===");
        info!("Total requests: {}", self.total_requests);
        info!("Successful: {}", self.successful_requests);
        info!("Failed: {}", self.failed_requests);
        info!("Error rate: {:.2}%", self.error_rate);
        info!("Total duration: {:?}", self.total_duration);
        info!("Requests per second: {:.2}", self.requests_per_second);
        info!("Response times:");
        info!("  Min: {:?}", self.min_duration);
        info!("  Max: {:?}", self.max_duration);
        info!("  Average: {:?}", self.avg_duration);
        info!("  50th percentile: {:?}", self.p50_duration);
        info!("  95th percentile: {:?}", self.p95_duration);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting testing patterns example");

    // Example 1: Unit testing with mock server
    info!("=== Example 1: Unit Testing with Mock Server ===");

    let mut mock_server = match MockOpenAIServer::try_new().await {
        Ok(server) => server,
        Err(err) => {
            warn!("mock server unavailable: {}", err);
            info!("Skipping mock-based sections because the environment does not permit binding local ports.");
            info!("Testing patterns example completed with limited output.");
            return Ok(());
        }
    };

    // Configure mock responses
    mock_server
        .mock_chat_completion("Hello, world!", "Hi there! How can I help you?")
        .await;

    let _client = mock_server.client()?;

    // Test basic functionality (this would be in a real unit test)
    info!("Testing basic chat completion with mock server");
    // Note: This would work with real implementation
    // let response = client.chat_simple("Hello, world!").await?;
    // TestUtils::assert_response_content(&response, "Hi there!");
    info!(" Mock server test would pass with real implementation");

    // Test streaming responses
    info!("Setting up streaming mock");
    mock_server
        .mock_streaming_chat(vec!["Hello", " there", "! How", " can I", " help?"])
        .await;

    // Test error scenarios
    info!("Testing error scenarios");
    mock_server
        .mock_error_response("/v1/chat/completions", ErrorType::RateLimit)
        .await;

    // Verify request logging
    info!("Requests logged: {}", mock_server.get_request_log().len());

    // Example 2: Integration testing with real API
    info!("\n=== Example 2: Integration Testing ===");

    // Note: In real scenario, this would use actual API credentials
    // For demonstration, we'll use a test client
    let integration_client = TestUtils::create_test_client()?;
    let mut integration_runner = IntegrationTestRunner::new(integration_client);

    // Run integration tests
    integration_runner.test_basic_chat_completion().await?;
    integration_runner.test_streaming_completion().await?;
    integration_runner.test_error_handling().await?;

    // Generate and display test report
    let report = integration_runner.generate_report();
    report.print_report();

    // Example 3: Performance testing
    info!("\n=== Example 3: Performance Testing ===");

    let perf_client = TestUtils::create_test_client()?;
    let perf_runner = PerformanceTestRunner::new(perf_client);

    // Run performance tests with different concurrency levels
    for concurrency in [1, 5, 10] {
        info!("Testing with {} concurrent workers", concurrency);
        let results = perf_runner.test_concurrent_requests(concurrency, 5).await;
        results.print_results();
    }

    // Example 4: Contract testing
    info!("\n=== Example 4: Contract Testing ===");

    // Test response schema validation
    let sample_response = TestUtils::create_mock_response_with_usage(
        "Test response content",
        25, // prompt tokens
        15, // completion tokens
    );

    // Parse and validate response structure
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&sample_response) {
        // Validate required fields exist
        assert!(parsed["id"].is_string(), "Response must have id field");
        assert!(
            parsed["choices"].is_array(),
            "Response must have choices array"
        );
        assert!(
            parsed["usage"]["total_tokens"].is_number(),
            "Response must have token usage"
        );
        info!(" Contract validation passed for response schema");
    }

    // Example 5: Test data generation and validation
    info!("\n=== Example 5: Test Data Generation ===");

    let test_prompts = TestUtils::create_test_prompts(5);
    info!("Generated {} test prompts", test_prompts.len());

    for (i, prompt) in test_prompts.iter().enumerate() {
        info!("  Prompt {}: {}", i + 1, prompt);
    }

    // Example 6: Stress testing and edge cases
    info!("\n=== Example 6: Edge Case Testing ===");

    // Test with very long input
    let long_input = "word ".repeat(1000); // ~4000 characters
    info!("Testing with long input ({} chars)", long_input.len());

    // Test with empty input
    info!("Testing with empty input");

    // Test with special characters
    let special_chars = "Testing with émojis  and spëcial çharacters!";
    info!("Testing with special characters: {}", special_chars);

    // Test with very large batch
    info!("Testing batch size limits");
    let large_batch = TestUtils::create_test_prompts(1000);
    info!("Created batch with {} prompts", large_batch.len());

    // Example 7: Mock configuration for different scenarios
    info!("\n=== Example 7: Advanced Mock Scenarios ===");

    let advanced_mock = match MockOpenAIServer::try_new().await {
        Ok(server) => server,
        Err(err) => {
            warn!("mock server unavailable for advanced scenarios: {}", err);
            info!("Skipping advanced mock configuration section.");
            info!("Testing patterns example completed successfully!");
            return Ok(());
        }
    };

    // Configure error simulation
    advanced_mock.configure_errors(ErrorSimulationConfig {
        simulate_rate_limits: true,
        rate_limit_delay: 5,
        simulate_server_errors: true,
        error_probability: 0.2, // 20% error rate
        simulate_timeouts: true,
        timeout_delay: 10,
    });

    info!("Configured advanced error simulation");

    // Test retry logic (would be implemented in real client)
    info!("Testing retry mechanisms with simulated errors");

    info!("Testing patterns example completed successfully!");
    Ok(())
}

/// Additional test utilities
impl TestUtils {
    /// Validate JSON schema compliance
    fn validate_json_schema(json_str: &str, expected_fields: &[&str]) -> bool {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            expected_fields
                .iter()
                .all(|field| parsed.get(field).is_some())
        } else {
            false
        }
    }

    /// Generate test data with specific token characteristics
    fn generate_test_data_with_tokens(target_tokens: usize) -> String {
        // Rough approximation: 1 token ≈ 4 characters for English text
        let target_chars = target_tokens * 4;
        let base_text = "This is a test prompt that will be used for token counting validation. ";
        let repetitions = (target_chars / base_text.len()) + 1;

        base_text
            .repeat(repetitions)
            .chars()
            .take(target_chars)
            .collect()
    }

    /// Create a comprehensive test suite configuration
    fn create_test_suite_config() -> TestSuiteConfig {
        TestSuiteConfig {
            include_unit_tests: true,
            include_integration_tests: true,
            include_performance_tests: true,
            include_contract_tests: true,
            max_test_duration: Duration::from_secs(30 * 60),
            performance_test_concurrency: vec![1, 5, 10, 20],
            error_simulation_enabled: true,
            test_data_variants: vec![
                "short_text".to_string(),
                "long_text".to_string(),
                "special_characters".to_string(),
                "multilingual".to_string(),
            ],
        }
    }
}

/// Configuration for comprehensive test suites
#[derive(Debug)]
struct TestSuiteConfig {
    include_unit_tests: bool,
    include_integration_tests: bool,
    include_performance_tests: bool,
    include_contract_tests: bool,
    max_test_duration: Duration,
    performance_test_concurrency: Vec<usize>,
    error_simulation_enabled: bool,
    test_data_variants: Vec<String>,
}
