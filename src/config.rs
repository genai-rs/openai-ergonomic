//! Configuration for the `OpenAI` ergonomic client.
//!
//! This module provides configuration options for the `OpenAI` client,
//! including API key management, base URLs, timeouts, and retry settings.

use crate::{errors::Result, Error};
use reqwest_middleware::ClientWithMiddleware;
use std::env;
use tokio::time::Duration;

/// Configuration for the `OpenAI` client.
///
/// The configuration can be created from environment variables or
/// manually constructed with the builder pattern.
///
/// # Environment Variables
///
/// - `OPENAI_API_KEY`: The `OpenAI` API key (required)
/// - `OPENAI_API_BASE`: Custom base URL for the API (optional)
/// - `OPENAI_ORGANIZATION`: Organization ID (optional)
/// - `OPENAI_PROJECT`: Project ID (optional)
/// - `OPENAI_TIMEOUT`: Request timeout in seconds (optional, default: 120)
/// - `OPENAI_MAX_RETRIES`: Maximum number of retries (optional, default: 3)
///
/// # Example
///
/// ```rust,ignore
/// # use openai_ergonomic::Config;
/// // From environment variables
/// let config = Config::from_env().unwrap();
///
/// // Manual configuration
/// let config = Config::builder()
///     .api_key("your-api-key")
///     .timeout_seconds(60)
///     .max_retries(5)
///     .build();
/// ```
#[derive(Clone)]
pub struct Config {
    api_key: String,
    api_base: String,
    organization: Option<String>,
    project: Option<String>,
    timeout_seconds: u64,
    max_retries: u32,
    default_model: String,
    http_client: Option<ClientWithMiddleware>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("api_key", &"***")
            .field("api_base", &self.api_base)
            .field("organization", &self.organization)
            .field("project", &self.project)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_retries", &self.max_retries)
            .field("default_model", &self.default_model)
            .field(
                "http_client",
                &self.http_client.as_ref().map(|_| "<ClientWithMiddleware>"),
            )
            .finish()
    }
}

impl Config {
    /// Create a new configuration builder.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| {
            Error::Config("OPENAI_API_KEY environment variable is required".to_string())
        })?;

        let api_base =
            env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let organization = env::var("OPENAI_ORGANIZATION").ok();
        let project = env::var("OPENAI_PROJECT").ok();

        let timeout_seconds = env::var("OPENAI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);

        let max_retries = env::var("OPENAI_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let default_model =
            env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string());

        Ok(Self {
            api_key,
            api_base,
            organization,
            project,
            timeout_seconds,
            max_retries,
            default_model,
            http_client: None,
        })
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the API base URL.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Get the organization ID, if set.
    pub fn organization(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    /// Get the project ID, if set.
    pub fn project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    /// Get the request timeout in seconds.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    /// Get the request timeout as a Duration.
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    /// Get the maximum number of retries.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the default model to use.
    pub fn default_model(&self) -> Option<&str> {
        if self.default_model.is_empty() {
            None
        } else {
            Some(&self.default_model)
        }
    }

    /// Get the base URL, if different from default.
    pub fn base_url(&self) -> Option<&str> {
        if self.api_base == "https://api.openai.com/v1" {
            None
        } else {
            Some(&self.api_base)
        }
    }

    /// Get the organization ID, if set.
    pub fn organization_id(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    /// Create an authorization header value.
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Get the custom HTTP client, if set.
    pub fn http_client(&self) -> Option<&ClientWithMiddleware> {
        self.http_client.as_ref()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: "https://api.openai.com/v1".to_string(),
            organization: None,
            project: None,
            timeout_seconds: 120,
            max_retries: 3,
            default_model: "gpt-4".to_string(),
            http_client: None,
        }
    }
}

/// Builder for creating `OpenAI` client configuration.
#[derive(Clone, Default)]
pub struct ConfigBuilder {
    api_key: Option<String>,
    api_base: Option<String>,
    organization: Option<String>,
    project: Option<String>,
    timeout_seconds: Option<u64>,
    max_retries: Option<u32>,
    default_model: Option<String>,
    http_client: Option<ClientWithMiddleware>,
}

impl ConfigBuilder {
    /// Set the API key.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the API base URL.
    #[must_use]
    pub fn api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = Some(api_base.into());
        self
    }

    /// Set the organization ID.
    #[must_use]
    pub fn organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    /// Set the project ID.
    #[must_use]
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set the request timeout in seconds.
    #[must_use]
    pub fn timeout_seconds(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Set the maximum number of retries.
    #[must_use]
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set the default model to use.
    #[must_use]
    pub fn default_model(mut self, default_model: impl Into<String>) -> Self {
        self.default_model = Some(default_model.into());
        self
    }

    /// Set a custom HTTP client.
    ///
    /// This allows you to provide a pre-configured `ClientWithMiddleware` with
    /// custom settings like retry policies, connection pooling, proxies, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use reqwest_middleware::ClientBuilder;
    /// use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
    ///
    /// let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    /// let client = ClientBuilder::new(reqwest::Client::new())
    ///     .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    ///     .build();
    ///
    /// let config = Config::builder()
    ///     .api_key("sk-...")
    ///     .http_client(client)
    ///     .build();
    /// ```
    #[must_use]
    pub fn http_client(mut self, client: ClientWithMiddleware) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the configuration.
    #[must_use]
    pub fn build(self) -> Config {
        Config {
            api_key: self.api_key.unwrap_or_default(),
            api_base: self
                .api_base
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            organization: self.organization,
            project: self.project,
            timeout_seconds: self.timeout_seconds.unwrap_or(120),
            max_retries: self.max_retries.unwrap_or(3),
            default_model: self.default_model.unwrap_or_else(|| "gpt-4".to_string()),
            http_client: self.http_client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = Config::builder()
            .api_key("test-key")
            .timeout_seconds(60)
            .build();

        assert_eq!(config.api_key(), "test-key");
        assert_eq!(config.timeout_seconds(), 60);
        assert_eq!(config.api_base(), "https://api.openai.com/v1");
    }

    #[test]
    fn test_auth_header() {
        let config = Config::builder().api_key("test-key").build();

        assert_eq!(config.auth_header(), "Bearer test-key");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.timeout_seconds(), 120);
        assert_eq!(config.max_retries(), 3);
        assert_eq!(config.default_model(), Some("gpt-4"));
    }

    #[test]
    fn test_config_with_custom_http_client() {
        let http_client = reqwest_middleware::ClientBuilder::new(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        )
        .build();

        let config = Config::builder()
            .api_key("test-key")
            .http_client(http_client)
            .build();

        assert!(config.http_client().is_some());
    }

    #[test]
    fn test_config_without_custom_http_client() {
        let config = Config::builder().api_key("test-key").build();

        assert!(config.http_client().is_none());
    }

    #[test]
    fn test_config_debug_hides_sensitive_data() {
        let config = Config::builder().api_key("secret-key-12345").build();

        let debug_output = format!("{config:?}");

        // Should not contain the actual API key
        assert!(!debug_output.contains("secret-key-12345"));
        // Should contain the masked version
        assert!(debug_output.contains("***"));
    }

    #[test]
    fn test_config_debug_with_http_client() {
        let http_client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();
        let config = Config::builder()
            .api_key("test-key")
            .http_client(http_client)
            .build();

        let debug_output = format!("{config:?}");

        // Should show placeholder for HTTP client
        assert!(debug_output.contains("<ClientWithMiddleware>"));
    }
}
