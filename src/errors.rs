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

    /// Streaming errors.
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
