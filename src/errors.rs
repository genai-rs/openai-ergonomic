//! Error types for the `OpenAI` ergonomic wrapper.
//!
//! This module provides comprehensive error handling with detailed error
//! information and proper error chaining.

use thiserror::Error;

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the `OpenAI` ergonomic wrapper.
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid request parameters or configuration.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Authentication errors.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limiting errors.
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// HTTP client errors.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// HTTP middleware errors.
    #[error("HTTP middleware error: {0}")]
    HttpMiddleware(#[from] reqwest_middleware::Error),

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
}

impl Error {
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
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limit() || self.is_server_error()
    }
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
    use super::*;
    use std::io;

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
    fn test_from_reqwest_error() {
        // Test that the conversion trait exists by creating a function that uses it
        #[allow(clippy::items_after_statements)]
        fn _test_reqwest_error_conversion(reqwest_error: reqwest::Error) -> Error {
            reqwest_error.into()
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
