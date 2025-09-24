//! Common test utilities and harness for openai-ergonomic tests.
//!
//! This module provides reusable testing infrastructure including:
//! - Mock clients and servers
//! - Test data fixtures
//! - Custom assertions
//! - Performance testing utilities

#![allow(
    dead_code,
    unused_imports,
    missing_docs,
    clippy::doc_markdown,
    clippy::single_match_else,
    clippy::match_wildcard_for_single_variants,
    clippy::unnecessary_map_or,
    clippy::cast_possible_truncation,
    clippy::significant_drop_tightening,
    clippy::cast_possible_wrap,
    clippy::unused_enumerate_index,
    clippy::let_unit_value,
    clippy::format_push_string,
    clippy::uninlined_format_args,
    clippy::manual_let_else,
    clippy::needless_raw_string_hashes,
    clippy::suboptimal_flops,
    clippy::cast_precision_loss,
    clippy::missing_const_for_fn,
    clippy::assertions_on_constants,
    clippy::const_is_empty
)]

pub mod assertions;
pub mod fixtures;
pub mod mock_client;

// Re-export commonly used items
pub use assertions::*;
// Note: fixtures re-exported from child modules as needed
pub use mock_client::{MockClientBuilder, MockOpenAIClient, ResponseConfig};

use openai_client_base::models::CreateChatCompletionRequest;
use openai_ergonomic::{builders::Builder, Error};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Test configuration constants
pub mod config {
    use std::time::Duration;

    /// Default timeout for tests
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    /// Maximum allowed test execution time
    pub const MAX_TEST_DURATION: Duration = Duration::from_secs(5);

    /// Default retry count for flaky tests
    pub const DEFAULT_RETRY_COUNT: usize = 3;

    /// Default model for testing
    pub const DEFAULT_TEST_MODEL: &str = "gpt-4";

    /// Alternative model for testing
    pub const ALTERNATIVE_TEST_MODEL: &str = "gpt-3.5-turbo";
}

/// Test result wrapper for better error reporting
#[derive(Debug)]
pub struct TestResult<T> {
    pub result: Result<T, TestError>,
    pub duration: Duration,
    pub metadata: TestMetadata,
}

/// Test metadata for result tracking
#[derive(Debug, Default)]
pub struct TestMetadata {
    pub test_name: String,
    pub tags: Vec<String>,
    pub retry_count: usize,
}

/// Test-specific error types
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Test timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Test assertion failed: {message}")]
    AssertionFailed { message: String },

    #[error("Mock setup failed: {reason}")]
    MockSetupFailed { reason: String },

    #[error("Test data invalid: {details}")]
    InvalidTestData { details: String },

    #[error("Test environment error: {details}")]
    Environment { details: String },

    #[error("Underlying error: {0}")]
    Underlying(#[from] Error),
}

/// Helper trait for test builders
pub trait TestBuilder<T> {
    /// Build the test target with validation
    fn build_test(self) -> Result<T, TestError>;

    /// Build and validate the structure
    fn build_and_validate(self) -> Result<T, TestError>;
}

impl<T> TestBuilder<CreateChatCompletionRequest> for T
where
    T: Builder<CreateChatCompletionRequest>,
{
    fn build_test(self) -> Result<CreateChatCompletionRequest, TestError> {
        self.build().map_err(TestError::from)
    }

    fn build_and_validate(self) -> Result<CreateChatCompletionRequest, TestError> {
        let request = self.build().map_err(TestError::from)?;

        // Validate basic structure
        if request.model.is_empty() {
            return Err(TestError::InvalidTestData {
                details: "Model cannot be empty".to_string(),
            });
        }

        if request.messages.is_empty() {
            return Err(TestError::InvalidTestData {
                details: "Messages cannot be empty".to_string(),
            });
        }

        Ok(request)
    }
}

/// Run a test with timeout and retry logic
pub async fn run_test_with_retry<F, Fut, T>(
    test_name: &str,
    test_fn: F,
    max_retries: usize,
    timeout: Duration,
) -> TestResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TestError>>,
{
    let mut metadata = TestMetadata {
        test_name: test_name.to_string(),
        tags: vec!["retry".to_string()],
        retry_count: 0,
    };

    let start = Instant::now();

    for attempt in 0..=max_retries {
        metadata.retry_count = attempt;

        let result = tokio::time::timeout(timeout, test_fn()).await;

        match result {
            Ok(Ok(value)) => {
                return TestResult {
                    result: Ok(value),
                    duration: start.elapsed(),
                    metadata,
                };
            }
            Ok(Err(e)) => {
                if attempt == max_retries {
                    return TestResult {
                        result: Err(e),
                        duration: start.elapsed(),
                        metadata,
                    };
                }
                // Continue to next retry
            }
            Err(_) => {
                if attempt == max_retries {
                    return TestResult {
                        result: Err(TestError::Timeout { duration: timeout }),
                        duration: start.elapsed(),
                        metadata,
                    };
                }
                // Continue to next retry
            }
        }
    }

    unreachable!("Loop should have returned")
}

/// Measure test execution time
pub fn measure_test<F, R>(f: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Assert that a test completes within expected time
pub fn assert_performance<F, R>(f: F, max_duration: Duration, test_name: &str) -> R
where
    F: FnOnce() -> R,
{
    let (result, duration) = measure_test(f);
    let tolerant_max = max_duration.saturating_add(Duration::from_millis(25));
    assert!(
        duration <= tolerant_max,
        "Test '{test_name}' took {duration:?} but should complete within {tolerant_max:?}"
    );
    result
}

/// Alias for assert_performance to maintain compatibility with different test patterns
pub fn assert_completes_within<F, R>(operation: F, max_duration: Duration, description: &str) -> R
where
    F: FnOnce() -> R,
{
    assert_performance(operation, max_duration, description)
}

/// Create test scenarios for parameter validation
pub fn parameter_validation_tests() -> Vec<(&'static str, f64, bool)> {
    vec![
        // Temperature tests
        ("temperature_valid_min", 0.0, true),
        ("temperature_valid_max", 2.0, true),
        ("temperature_invalid_negative", -0.1, false),
        ("temperature_invalid_high", 2.1, false),
        // Top-p tests
        ("top_p_valid_min", 0.0, true),
        ("top_p_valid_max", 1.0, true),
        ("top_p_invalid_negative", -0.1, false),
        ("top_p_invalid_high", 1.1, false),
        // Frequency penalty tests
        ("frequency_penalty_valid_min", -2.0, true),
        ("frequency_penalty_valid_max", 2.0, true),
        ("frequency_penalty_invalid_low", -2.1, false),
        ("frequency_penalty_invalid_high", 2.1, false),
        // Presence penalty tests
        ("presence_penalty_valid_min", -2.0, true),
        ("presence_penalty_valid_max", 2.0, true),
        ("presence_penalty_invalid_low", -2.1, false),
        ("presence_penalty_invalid_high", 2.1, false),
    ]
}

/// Create common model test cases
pub fn model_test_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gpt_4", "gpt-4"),
        ("gpt_4_turbo", "gpt-4-turbo"),
        ("gpt_3_5_turbo", "gpt-3.5-turbo"),
        ("gpt_4_vision", "gpt-4-vision-preview"),
        ("o3_mini", "o3-mini"),
        ("custom_model", "custom-model-name"),
    ]
}

/// Generate test data for different content types
pub fn content_type_tests() -> Vec<(&'static str, Value)> {
    vec![
        ("simple_text", json!("Hello, world!")),
        ("empty_text", json!("")),
        ("unicode_text", json!("Hello 🌍! Здравствуй мир!")),
        ("json_object", json!({"key": "value", "number": 42})),
        ("json_array", json!(["item1", "item2", "item3"])),
        ("multiline_text", json!("Line 1\nLine 2\nLine 3")),
        (
            "special_chars",
            json!("Special: @#$%^&*()_+-={}[]|\\:;\"'<>?,./"),
        ),
    ]
}

/// Test helper macros
#[macro_export]
macro_rules! assert_builder_success {
    ($builder:expr) => {
        $builder.build().expect("Builder should succeed")
    };
}

#[macro_export]
macro_rules! assert_builder_error {
    ($builder:expr, $expected_msg:expr) => {
        let result = $builder.build();
        assert!(result.is_err(), "Expected builder to fail");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains($expected_msg),
            "Error '{}' should contain '{}'",
            error,
            $expected_msg
        );
    };
}

#[macro_export]
macro_rules! assert_field_value {
    ($obj:expr, $field:expr, $expected:expr) => {
        let value = $obj
            .get($field)
            .unwrap_or_else(|| panic!("Field '{}' not found", $field));
        assert_eq!(value, &$expected, "Field '{}' has wrong value", $field);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_constants() {
        assert!(config::DEFAULT_TIMEOUT > Duration::ZERO);
        assert!(config::MAX_TEST_DURATION > Duration::ZERO);
        assert!(config::DEFAULT_RETRY_COUNT > 0);
        assert!(!config::DEFAULT_TEST_MODEL.is_empty());
        assert!(!config::ALTERNATIVE_TEST_MODEL.is_empty());
    }

    #[test]
    fn test_parameter_validation_data() {
        let tests = parameter_validation_tests();
        assert!(!tests.is_empty());

        for (name, value, _is_valid) in tests {
            assert!(!name.is_empty());
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_model_test_cases() {
        let models = model_test_cases();
        assert!(!models.is_empty());

        for (name, model) in models {
            assert!(!name.is_empty());
            assert!(!model.is_empty());
        }
    }

    #[test]
    fn test_content_type_tests() {
        let content_tests = content_type_tests();
        assert!(!content_tests.is_empty());

        for (name, content) in content_tests {
            assert!(!name.is_empty());
            assert!(content.is_string() || content.is_object() || content.is_array());
        }
    }

    #[test]
    fn test_measure_test() {
        let (result, duration) = measure_test(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_assert_performance() {
        let result = assert_performance(|| 42, Duration::from_millis(100), "test_performance");

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_run_test_with_retry_success() {
        let result = run_test_with_retry(
            "test_success",
            || async { Ok::<i32, TestError>(42) },
            3,
            Duration::from_secs(1),
        )
        .await;

        assert!(result.result.is_ok());
        assert_eq!(result.result.unwrap(), 42);
        assert_eq!(result.metadata.retry_count, 0);
    }

    #[tokio::test]
    async fn test_run_test_with_retry_timeout() {
        let result = run_test_with_retry(
            "test_timeout",
            || async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok::<i32, TestError>(42)
            },
            1,
            Duration::from_millis(100),
        )
        .await;

        assert!(result.result.is_err());
        matches!(result.result.unwrap_err(), TestError::Timeout { .. });
    }
}
