//! Error types for the `OpenAI` ergonomic wrapper.
//!
//! This module provides comprehensive error handling with detailed error
//! information and proper error chaining.

use thiserror::Error;

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the `OpenAI` ergonomic wrapper.
///
/// New variants may be added in minor releases: match with a wildcard arm, or prefer the
/// classification methods ([`Error::is_rate_limit`], [`Error::is_retryable`], ...).
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Tool registration or execution failure, including any model call context.
    #[error(transparent)]
    Tool(Box<crate::tool_framework::ToolError>),

    /// Application-defined failure with its original error source preserved.
    #[error("Application error: {0}")]
    Application(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Invalid request parameters or configuration.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Authentication errors.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limiting errors.
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// HTTP client errors: building the request, sending it through the client's tower
    /// service (including its layers, such as retries and timeouts), or reading the response.
    #[error("HTTP error: {0}")]
    Http(#[from] timvw_rust_runtime::Error),

    /// JSON serialization/deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// `OpenAI` API errors with status code and message.
    #[error("`OpenAI` API error (status {status}): {message}")]
    Api {
        /// HTTP status code returned by the API
        status: u16,
        /// Error message from the API
        message: String,
        /// Type of error (if provided by API)
        error_type: Option<String>,
        /// Error code (if provided by API)
        error_code: Option<String>,
    },

    /// Streaming connection errors.
    #[error("Stream connection error: {message}")]
    StreamConnection {
        /// Error message describing the connection issue
        message: String,
    },

    /// Streaming data parsing errors.
    #[error("Stream parsing error: {message}, chunk: {chunk}")]
    StreamParsing {
        /// Error message describing the parsing issue
        message: String,
        /// The problematic chunk data
        chunk: String,
    },

    /// Streaming buffer management errors.
    #[error("Stream buffer error: {message}")]
    StreamBuffer {
        /// Error message describing the buffer issue
        message: String,
    },

    /// Generic streaming errors.
    #[error("Stream error: {0}")]
    Stream(String),

    /// File operation errors.
    #[error("File error: {0}")]
    File(#[from] std::io::Error),

    /// Configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Builder validation errors.
    #[error("Builder validation error: {0}")]
    Builder(String),

    /// Generic internal errors.
    #[error("Internal error: {0}")]
    Internal(String),

    /// A failure that must stop the work instead of being reported to the model: a tool
    /// handler returns it with [`Error::fatal`], and
    /// [`ToolRegistry::answer_call`](crate::ToolRegistry::answer_call) returns it instead of
    /// answering the call.
    #[error("Fatal error: {0}")]
    Fatal(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// A response that failed on the model's side: a `response.failed` event or response, or an
    /// `error` event of a Responses stream ([`ResponsesEvent::failure`](crate::ResponsesEvent::failure),
    /// [`ResponsesResponse::failure`](crate::ResponsesResponse::failure)).
    #[error("Response failed ({}): {message}", code.as_deref().unwrap_or("no code"))]
    #[non_exhaustive]
    ResponseFailed {
        /// The error code, such as `server_error`, `rate_limit_exceeded` or
        /// `context_length_exceeded`.
        code: Option<String>,
        /// The error message.
        message: String,
    },
}

/// How a failed request may be retried ([`Error::retry_class`]).
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RetryClass {
    /// Retrying will not help: a client error, an exhausted quota, a context that is too long.
    Never,
    /// Retry after the delay the server asked for.
    ServerDirected {
        /// The delay before the next attempt.
        after: std::time::Duration,
    },
    /// Retry with exponential backoff: a transient failure (rate limit without a known delay,
    /// server error, cut-off stream, connection failure).
    Backoff,
}

impl RetryClass {
    /// Whether a retry may help.
    #[must_use]
    pub fn is_retryable(self) -> bool {
        !matches!(self, Self::Never)
    }
}

impl From<crate::tool_framework::ToolError> for Error {
    fn from(error: crate::tool_framework::ToolError) -> Self {
        Self::Tool(Box::new(error))
    }
}

impl Error {
    /// Preserve an application or domain error when returning the crate's `Result`.
    pub fn application(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Application(Box::new(error))
    }

    /// Create a new API error with status and message.
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
            error_type: None,
            error_code: None,
        }
    }

    /// Create a new API error with full details.
    pub fn api_detailed(
        status: u16,
        message: impl Into<String>,
        error_type: Option<String>,
        error_code: Option<String>,
    ) -> Self {
        Self::Api {
            status,
            message: message.into(),
            error_type,
            error_code,
        }
    }

    /// Check if this is a rate limit error.
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Error::RateLimit(_)) || matches!(self, Error::Api { status: 429, .. })
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Error::Authentication(_)) || matches!(self, Error::Api { status: 401, .. })
    }

    /// Check if this is a client error (4xx status codes).
    pub fn is_client_error(&self) -> bool {
        match self {
            Error::Api { status, .. } => (400..500).contains(status),
            Error::Authentication(_) | Error::RateLimit(_) | Error::InvalidRequest(_) => true,
            _ => false,
        }
    }

    /// Check if this is a server error (5xx status codes).
    pub fn is_server_error(&self) -> bool {
        match self {
            Error::Api { status, .. } => (500..600).contains(status),
            _ => false,
        }
    }

    /// Check if this error might be retryable.
    ///
    /// A rate limit or a server error. [`Error::retry_class`] is finer: it knows exhausted
    /// quotas, delays the server asks for, cut-off streams and failed responses.
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limit() || self.is_server_error()
    }

    /// A failure that must stop instead of being reported to the model: `handler` errors
    /// wrapped in it end [`ToolRegistry::answer_call`](crate::ToolRegistry::answer_call) with an
    /// error ([`ToolFailure::Fatal`](crate::ToolFailure::Fatal)).
    pub fn fatal(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Fatal(Box::new(error))
    }

    /// Whether this is an [`Error::Fatal`].
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    /// A failed response ([`Error::ResponseFailed`]).
    pub fn response_failed(code: Option<String>, message: impl Into<String>) -> Self {
        Self::ResponseFailed {
            code,
            message: message.into(),
        }
    }

    /// Whether the input was too long for the model's context window: an API error or a failed
    /// response with code `context_length_exceeded`.
    pub fn is_context_length_exceeded(&self) -> bool {
        self.code() == Some("context_length_exceeded")
    }

    /// The error code of an API error or a failed response.
    fn code(&self) -> Option<&str> {
        match self {
            Self::Api { error_code, .. } => error_code.as_deref(),
            Self::ResponseFailed { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// How the request that failed with this error may be retried:
    ///
    /// - [`RetryClass::Never`] for client errors, an exhausted quota (`insufficient_quota`), a
    ///   context that is too long, an invalid prompt or image, and every error that is not about
    ///   the request (configuration, JSON, files, tools, [`Error::Fatal`]);
    /// - [`RetryClass::ServerDirected`] for a rate limit (`429`, `rate_limit_exceeded`) whose
    ///   message says when to try again (`"Please try again in 1.5s"`);
    /// - [`RetryClass::Backoff`] for other rate limits, `408`, server errors (`5xx`), failed
    ///   responses with any other code, connection failures and cut-off streams.
    ///
    /// It does not retry anything itself: the client retries an HTTP request that fails
    /// before its body ([`Config::max_retries`](crate::Config::max_retries)); a stream that
    /// fails midway must be retried as a whole by the caller.
    pub fn retry_class(&self) -> RetryClass {
        let directed = |message: &str| {
            retry_after_in_message(message).map_or(RetryClass::Backoff, |after| {
                RetryClass::ServerDirected { after }
            })
        };
        match self {
            Self::Api {
                status,
                message,
                error_type,
                error_code,
            } => {
                let quota = [error_type, error_code]
                    .iter()
                    .any(|value| value.as_deref() == Some("insufficient_quota"));
                match status {
                    429 if quota => RetryClass::Never,
                    408 | 429 | 500..=599 => directed(message),
                    _ => RetryClass::Never,
                }
            }
            Self::RateLimit(message) => directed(message),
            Self::ResponseFailed { code, message } => match code.as_deref() {
                Some(
                    "insufficient_quota"
                    | "context_length_exceeded"
                    | "invalid_prompt"
                    | "data_residency_mismatch"
                    | "bio_policy"
                    | "misalignment_policy_violation",
                ) => RetryClass::Never,
                Some(code) if code.contains("image") => RetryClass::Never,
                _ => directed(message),
            },
            Self::StreamConnection { .. } => RetryClass::Backoff,
            Self::Http(error) if error.is_timeout() || error.is_connect() || error.is_body() => {
                RetryClass::Backoff
            }
            _ => RetryClass::Never,
        }
    }

    /// Check if this is a file error: [`Error::File`], or an [`Error::Http`] whose
    /// [`is_io`](timvw_rust_runtime::Error::is_io) is true, such as an upload of a file that does
    /// not exist (or, on WebAssembly, any upload from a path: there is no file system).
    pub fn is_io(&self) -> bool {
        match self {
            Error::File(_) => true,
            Error::Http(error) => error.is_io(),
            _ => false,
        }
    }

    /// The I/O error behind a file error ([`Error::is_io`]), to read its
    /// [`kind`](std::io::Error::kind) (`NotFound` for a missing file). Its message names the path.
    pub fn io_error(&self) -> Option<&std::io::Error> {
        match self {
            Error::File(error) => Some(error),
            Error::Http(error) if error.is_io() => {
                let mut source = std::error::Error::source(error);
                while let Some(current) = source {
                    if let Some(io) = current.downcast_ref::<std::io::Error>() {
                        return Some(io);
                    }
                    source = current.source();
                }
                None
            }
            _ => None,
        }
    }
}

/// The delay of a message such as `"Please try again in 1.5s"` or `"try again in 20ms"`.
fn retry_after_in_message(message: &str) -> Option<std::time::Duration> {
    let lower = message.to_ascii_lowercase();
    let start = lower.find("try again in ")? + "try again in ".len();
    let rest = &message[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
        .unwrap_or(rest.len());
    parse_duration(rest[..end].trim_end_matches('.'))
}

/// A duration such as `1s`, `20ms`, `6m0s` or `1h2m3.5s` (Go's format, as the rate limit
/// headers send it). A bare number is seconds.
pub(crate) fn parse_duration(text: &str) -> Option<std::time::Duration> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(seconds) = text.parse::<f64>() {
        return std::time::Duration::try_from_secs_f64(seconds).ok();
    }
    let mut total = 0.0_f64;
    let mut rest = text;
    while !rest.is_empty() {
        let number_end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .filter(|end| *end > 0)?;
        let value: f64 = rest[..number_end].parse().ok()?;
        rest = &rest[number_end..];
        let unit_end = rest
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(rest.len());
        let scale = match &rest[..unit_end] {
            "h" => 3600.0,
            "m" => 60.0,
            "s" => 1.0,
            "ms" => 1e-3,
            "us" | "µs" => 1e-6,
            "ns" => 1e-9,
            _ => return None,
        };
        total = value.mul_add(scale, total);
        rest = &rest[unit_end..];
    }
    std::time::Duration::try_from_secs_f64(total).ok()
}

/// Specialized error types for different API endpoints.
pub mod chat {
    use super::Error;

    /// Create an error for invalid chat messages.
    pub fn invalid_messages(msg: impl Into<String>) -> Error {
        Error::InvalidRequest(format!("Invalid chat messages: {}", msg.into()))
    }

    /// Create an error for unsupported model.
    pub fn unsupported_model(model: impl Into<String>) -> Error {
        Error::InvalidRequest(format!("Unsupported model: {}", model.into()))
    }
}

/// Specialized error types for responses API.
pub mod responses {
    use super::Error;

    /// Create an error for invalid tool definition.
    pub fn invalid_tool(msg: impl Into<String>) -> Error {
        Error::InvalidRequest(format!("Invalid tool definition: {}", msg.into()))
    }

    /// Create an error for missing required response format.
    pub fn missing_response_format() -> Error {
        Error::InvalidRequest("Response format is required for structured outputs".to_string())
    }
}

/// Specialized error types for file operations.
pub mod files {
    use super::Error;

    /// Create an error for file upload failures.
    pub fn upload_failed(msg: impl Into<String>) -> Error {
        Error::File(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("File upload failed: {}", msg.into()),
        ))
    }

    /// Create an error for unsupported file type.
    pub fn unsupported_type(file_type: impl Into<String>) -> Error {
        Error::InvalidRequest(format!("Unsupported file type: {}", file_type.into()))
    }
}

/// Specialized error types for streaming operations.
pub mod streaming {
    use super::Error;

    /// Create an error for stream connection failures.
    pub fn connection_failed(msg: impl Into<String>) -> Error {
        Error::Stream(format!("Stream connection failed: {}", msg.into()))
    }

    /// Create an error for stream parsing failures.
    pub fn parse_failed(msg: impl Into<String>) -> Error {
        Error::Stream(format!("Stream parsing failed: {}", msg.into()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::io;

    #[tokio::test]
    async fn test_io_errors() {
        let missing = timvw_rust_runtime::Upload::path("/no/such/dir/data.jsonl")
            .resolve()
            .await
            .unwrap_err();
        let error = Error::Http(missing);
        assert!(error.is_io());
        assert_eq!(error.io_error().unwrap().kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("data.jsonl"));

        let error = Error::File(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        assert!(error.is_io());
        assert_eq!(
            error.io_error().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );

        let error = Error::InvalidRequest("x".to_string());
        assert!(!error.is_io());
        assert!(error.io_error().is_none());
    }

    #[test]
    fn test_retry_class() {
        use std::time::Duration;
        let quota = Error::api_detailed(429, "quota", None, Some("insufficient_quota".into()));
        assert_eq!(quota.retry_class(), RetryClass::Never);
        assert_eq!(
            Error::api(429, "Rate limit reached. Please try again in 1.5s.").retry_class(),
            RetryClass::ServerDirected {
                after: Duration::from_millis(1500)
            }
        );
        assert_eq!(
            Error::api(429, "slow down").retry_class(),
            RetryClass::Backoff
        );
        assert_eq!(
            Error::api(503, "overloaded").retry_class(),
            RetryClass::Backoff
        );
        assert_eq!(Error::api(400, "bad").retry_class(), RetryClass::Never);
        assert_eq!(
            Error::response_failed(Some("context_length_exceeded".into()), "too long")
                .retry_class(),
            RetryClass::Never
        );
        assert!(
            Error::response_failed(Some("context_length_exceeded".into()), "too long")
                .is_context_length_exceeded()
        );
        assert_eq!(
            Error::response_failed(Some("rate_limit_exceeded".into()), "try again in 20ms")
                .retry_class(),
            RetryClass::ServerDirected {
                after: Duration::from_millis(20)
            }
        );
        assert_eq!(
            Error::response_failed(Some("server_error".into()), "boom").retry_class(),
            RetryClass::Backoff
        );
        assert_eq!(
            Error::response_failed(Some("invalid_image_url".into()), "bad").retry_class(),
            RetryClass::Never
        );
        let cut = Error::StreamConnection {
            message: "cut".into(),
        };
        assert_eq!(cut.retry_class(), RetryClass::Backoff);
        assert!(cut.retry_class().is_retryable());
        assert_eq!(
            Error::fatal(io::Error::other("x")).retry_class(),
            RetryClass::Never
        );
        assert!(Error::fatal(io::Error::other("x")).is_fatal());
    }

    #[test]
    fn test_parse_duration() {
        use std::time::Duration;
        assert_eq!(parse_duration("1s"), Some(Duration::from_secs(1)));
        assert_eq!(parse_duration("6m0s"), Some(Duration::from_secs(360)));
        assert_eq!(parse_duration("20ms"), Some(Duration::from_millis(20)));
        assert_eq!(
            parse_duration("1h2m3.5s"),
            Some(Duration::from_millis(3_723_500))
        );
        assert_eq!(parse_duration("2"), Some(Duration::from_secs(2)));
        assert_eq!(parse_duration("soon"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn test_error_display() {
        let error = Error::InvalidRequest("test message".to_string());
        assert_eq!(error.to_string(), "Invalid request: test message");

        let error = Error::Authentication("invalid API key".to_string());
        assert_eq!(error.to_string(), "Authentication failed: invalid API key");

        let error = Error::RateLimit("rate limit exceeded".to_string());
        assert_eq!(
            error.to_string(),
            "Rate limit exceeded: rate limit exceeded"
        );
    }

    #[test]
    fn test_api_error_constructors() {
        let error = Error::api(400, "Bad request");
        match error {
            Error::Api {
                status,
                message,
                error_type,
                error_code,
            } => {
                assert_eq!(status, 400);
                assert_eq!(message, "Bad request");
                assert!(error_type.is_none());
                assert!(error_code.is_none());
            }
            _ => panic!("Expected API error"),
        }

        let error = Error::api_detailed(
            429,
            "Rate limit exceeded",
            Some("rate_limit_exceeded".to_string()),
            Some("RL001".to_string()),
        );
        match error {
            Error::Api {
                status,
                message,
                error_type,
                error_code,
            } => {
                assert_eq!(status, 429);
                assert_eq!(message, "Rate limit exceeded");
                assert_eq!(error_type, Some("rate_limit_exceeded".to_string()));
                assert_eq!(error_code, Some("RL001".to_string()));
            }
            _ => panic!("Expected API error"),
        }
    }

    #[test]
    fn test_is_rate_limit() {
        let error = Error::RateLimit("exceeded".to_string());
        assert!(error.is_rate_limit());

        let error = Error::api(429, "Too Many Requests");
        assert!(error.is_rate_limit());

        let error = Error::api(400, "Bad Request");
        assert!(!error.is_rate_limit());

        let error = Error::InvalidRequest("invalid".to_string());
        assert!(!error.is_rate_limit());
    }

    #[test]
    fn test_is_auth_error() {
        let error = Error::Authentication("invalid key".to_string());
        assert!(error.is_auth_error());

        let error = Error::api(401, "Unauthorized");
        assert!(error.is_auth_error());

        let error = Error::api(403, "Forbidden");
        assert!(!error.is_auth_error());

        let error = Error::InvalidRequest("invalid".to_string());
        assert!(!error.is_auth_error());
    }

    #[test]
    fn test_is_client_error() {
        let error = Error::api(400, "Bad Request");
        assert!(error.is_client_error());

        let error = Error::api(404, "Not Found");
        assert!(error.is_client_error());

        let error = Error::api(499, "Client Error");
        assert!(error.is_client_error());

        let error = Error::Authentication("invalid".to_string());
        assert!(error.is_client_error());

        let error = Error::RateLimit("exceeded".to_string());
        assert!(error.is_client_error());

        let error = Error::InvalidRequest("invalid".to_string());
        assert!(error.is_client_error());

        let error = Error::api(500, "Server Error");
        assert!(!error.is_client_error());

        let error = Error::Internal("internal".to_string());
        assert!(!error.is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        let error = Error::api(500, "Internal Server Error");
        assert!(error.is_server_error());

        let error = Error::api(502, "Bad Gateway");
        assert!(error.is_server_error());

        let error = Error::api(599, "Server Error");
        assert!(error.is_server_error());

        let error = Error::api(400, "Client Error");
        assert!(!error.is_server_error());

        let error = Error::Internal("internal".to_string());
        assert!(!error.is_server_error());
    }

    #[test]
    fn test_is_retryable() {
        // Rate limit errors are retryable
        let error = Error::RateLimit("exceeded".to_string());
        assert!(error.is_retryable());

        let error = Error::api(429, "Too Many Requests");
        assert!(error.is_retryable());

        // Server errors are retryable
        let error = Error::api(500, "Internal Server Error");
        assert!(error.is_retryable());

        let error = Error::api(502, "Bad Gateway");
        assert!(error.is_retryable());

        // Client errors are not retryable
        let error = Error::api(400, "Bad Request");
        assert!(!error.is_retryable());

        let error = Error::Authentication("invalid".to_string());
        assert!(!error.is_retryable());

        let error = Error::InvalidRequest("invalid".to_string());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_from_http_error() {
        // Test that the conversion trait exists by creating a function that uses it
        #[allow(clippy::items_after_statements)]
        fn _test_http_error_conversion(http_error: timvw_rust_runtime::Error) -> Error {
            http_error.into()
        }

        // The test passes if this compiles - we verify the trait implementation exists
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error: Error = json_error.into();
        assert!(matches!(error, Error::Json(_)));
    }

    #[test]
    fn test_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::File(_)));
    }

    #[test]
    fn test_stream_errors() {
        let error = Error::StreamConnection {
            message: "connection lost".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Stream connection error: connection lost"
        );

        let error = Error::StreamParsing {
            message: "invalid data".to_string(),
            chunk: "bad chunk".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Stream parsing error: invalid data, chunk: bad chunk"
        );

        let error = Error::StreamBuffer {
            message: "buffer overflow".to_string(),
        };
        assert_eq!(error.to_string(), "Stream buffer error: buffer overflow");

        let error = Error::Stream("generic stream error".to_string());
        assert_eq!(error.to_string(), "Stream error: generic stream error");
    }

    #[test]
    fn test_specialized_error_modules() {
        // Test chat module errors
        let error = chat::invalid_messages("empty messages");
        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("Invalid chat messages"));

        let error = chat::unsupported_model("gpt-5");
        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("Unsupported model"));

        // Test responses module errors
        let error = responses::invalid_tool("missing name");
        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("Invalid tool definition"));

        let error = responses::missing_response_format();
        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("Response format is required"));

        // Test files module errors
        let error = files::upload_failed("network error");
        assert!(matches!(error, Error::File(_)));

        let error = files::unsupported_type("txt");
        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("Unsupported file type"));

        // Test streaming module errors
        let error = streaming::connection_failed("timeout");
        assert!(matches!(error, Error::Stream(_)));
        assert!(error.to_string().contains("Stream connection failed"));

        let error = streaming::parse_failed("invalid JSON");
        assert!(matches!(error, Error::Stream(_)));
        assert!(error.to_string().contains("Stream parsing failed"));
    }

    #[test]
    fn test_error_debug_format() {
        let error = Error::InvalidRequest("test".to_string());
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("InvalidRequest"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_chains() {
        // Test that errors properly chain when using From traits
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let wrapped_error: Error = io_error.into();

        match wrapped_error {
            Error::File(ref err) => {
                assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected File error"),
        }
    }

    #[test]
    fn test_config_error() {
        let error = Error::Config("missing API key".to_string());
        assert_eq!(error.to_string(), "Configuration error: missing API key");
    }

    #[test]
    fn test_builder_error() {
        let error = Error::Builder("validation failed".to_string());
        assert_eq!(
            error.to_string(),
            "Builder validation error: validation failed"
        );
    }

    #[test]
    fn test_internal_error() {
        let error = Error::Internal("unexpected state".to_string());
        assert_eq!(error.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_error_status_boundaries() {
        // Test edge cases for status code ranges
        let error = Error::api(399, "Client Error");
        assert!(!error.is_client_error());

        let error = Error::api(400, "Client Error");
        assert!(error.is_client_error());

        let error = Error::api(499, "Client Error");
        assert!(error.is_client_error());

        let error = Error::api(500, "Server Error");
        assert!(!error.is_client_error());
        assert!(error.is_server_error());

        let error = Error::api(599, "Server Error");
        assert!(error.is_server_error());

        let error = Error::api(600, "Unknown");
        assert!(!error.is_server_error());
    }
}
