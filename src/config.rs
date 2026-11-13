//! Configuration for the `OpenAI` ergonomic client.
//!
//! This module provides configuration options for the `OpenAI` client,
//! including API key management, base URLs, timeouts, and retry settings.

use crate::auth::{AuthProvider, SharedAuthProvider};
use crate::provider::{Provider, ProviderKind, OPENAI_BASE_URL};
use crate::{errors::Result, Error};
use std::env;
use timvw_rust_runtime::Client as HttpClient;

/// Configuration for the `OpenAI` client.
///
/// The configuration can be created from environment variables or
/// manually constructed with the builder pattern.
///
/// # Environment Variables
///
/// ## Standard `OpenAI`
/// - `OPENAI_API_KEY`: The `OpenAI` API key (required)
/// - `OPENAI_API_BASE`: Custom base URL for the API (optional)
/// - `OPENAI_ORGANIZATION`: Organization ID (optional)
/// - `OPENAI_PROJECT`: Project ID (optional)
/// - `OPENAI_MAX_RETRIES`: Maximum number of retries (optional, default: 3)
///
/// ## Azure `OpenAI`
/// - `AZURE_OPENAI_API_KEY`: The Azure `OpenAI` API key (alternative to `OPENAI_API_KEY`)
/// - `AZURE_OPENAI_ENDPOINT`: Azure `OpenAI` endpoint (e.g., `<https://my-resource.openai.azure.com>`)
/// - `AZURE_OPENAI_DEPLOYMENT`: Deployment name (required for Azure)
/// - `AZURE_OPENAI_API_VERSION`: API version (optional, default: 2024-02-01)
///
/// # Example
///
/// ```rust,no_run
/// # use openai_ergonomic::Config;
/// // From environment variables
/// let config = Config::from_env().unwrap();
///
/// // Manual configuration for OpenAI
/// let config = Config::builder()
///     .api_key("your-api-key")
///     .max_retries(5)
///     .build();
///
/// // Manual configuration for Azure OpenAI
/// let config = Config::builder()
///     .api_key("your-azure-api-key")
///     .api_base("https://my-resource.openai.azure.com")
///     .azure_deployment("my-deployment")
///     .azure_api_version("2024-02-01")
///     .build();
///
/// // Another provider: base URL, authentication, headers and paths in one go
/// use openai_ergonomic::Provider;
/// let config = Config::builder()
///     .provider(Provider::openrouter("sk-or-..."))
///     .build();
/// ```
#[derive(Clone)]
pub struct Config {
    api_key: String,
    api_base: String,
    organization: Option<String>,
    project: Option<String>,
    max_retries: u32,
    default_model: String,
    http_client: Option<HttpClient>,
    azure_deployment: Option<String>,
    azure_api_version: Option<String>,
    provider: Option<Provider>,
    auth_provider: Option<SharedAuthProvider>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("api_key", &"***")
            .field("api_base", &self.api_base)
            .field("organization", &self.organization)
            .field("project", &self.project)
            .field("max_retries", &self.max_retries)
            .field("default_model", &self.default_model)
            .field(
                "http_client",
                &self
                    .http_client
                    .as_ref()
                    .map(|_| "<timvw_rust_runtime::Client>"),
            )
            .field("azure_deployment", &self.azure_deployment)
            .field("azure_api_version", &self.azure_api_version)
            .field("provider", &self.provider)
            .field("auth_provider", &self.auth_provider)
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
    ///
    /// Supports both standard `OpenAI` and Azure `OpenAI` configurations.
    /// For Azure `OpenAI`, set `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY`, and `AZURE_OPENAI_DEPLOYMENT`.
    pub fn from_env() -> Result<Self> {
        // Check for Azure OpenAI configuration first
        let azure_endpoint = env::var("AZURE_OPENAI_ENDPOINT").ok();
        let azure_deployment = env::var("AZURE_OPENAI_DEPLOYMENT").ok();
        let azure_api_version = env::var("AZURE_OPENAI_API_VERSION").ok();

        let (api_key, api_base) = if let Some(endpoint) = azure_endpoint {
            // Azure OpenAI configuration
            let key = env::var("AZURE_OPENAI_API_KEY")
                .or_else(|_| env::var("OPENAI_API_KEY"))
                .map_err(|_| {
                    Error::Config(
                        "AZURE_OPENAI_API_KEY or OPENAI_API_KEY environment variable is required"
                            .to_string(),
                    )
                })?;
            // Trim trailing slash from Azure endpoint
            let endpoint = endpoint.trim_end_matches('/').to_string();
            (key, endpoint)
        } else {
            // Standard OpenAI configuration
            let key = env::var("OPENAI_API_KEY").map_err(|_| {
                Error::Config("OPENAI_API_KEY environment variable is required".to_string())
            })?;
            let base = env::var("OPENAI_API_BASE").unwrap_or_else(|_| OPENAI_BASE_URL.to_string());
            (key, base)
        };

        let organization = env::var("OPENAI_ORGANIZATION").ok();
        let project = env::var("OPENAI_PROJECT").ok();

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
            max_retries,
            default_model,
            http_client: None,
            azure_deployment,
            azure_api_version,
            provider: None,
            auth_provider: None,
        })
    }

    /// Get the API key: the key of the [provider](ConfigBuilder::provider) when one is set
    /// (empty when it has none).
    pub fn api_key(&self) -> &str {
        match &self.provider {
            Some(provider) => provider.api_key().unwrap_or_default(),
            None => &self.api_key,
        }
    }

    /// Get the API base URL: the base URL of the [provider](ConfigBuilder::provider) when one
    /// is set.
    pub fn api_base(&self) -> &str {
        match &self.provider {
            Some(provider) => provider.api_base(),
            None => &self.api_base,
        }
    }

    /// The provider requests go to: the one set with [`ConfigBuilder::provider`], or else the
    /// one the other settings describe (Azure `OpenAI` when [`Config::is_azure`], `OpenAI` at
    /// [`Config::api_base`] otherwise).
    pub fn provider(&self) -> Provider {
        if let Some(provider) = &self.provider {
            return provider.clone();
        }
        if self.is_azure() {
            // Without a deployment: the key, and the paths as they are.
            let mut provider = self.azure_deployment.as_ref().map_or_else(
                || {
                    Provider::custom(self.api_base.clone())
                        .api_key_header("api-key", self.api_key.clone())
                },
                |deployment| {
                    Provider::azure(self.api_base.clone(), self.api_key.clone(), deployment)
                },
            );
            if let Some(version) = &self.azure_api_version {
                provider = provider.api_version(version.clone());
            }
            return provider;
        }
        let provider = Provider::openai(self.api_key.clone());
        if self.api_base.trim_end_matches('/') == OPENAI_BASE_URL {
            provider
        } else {
            provider.base_url(self.api_base.clone())
        }
    }

    /// The [`AuthProvider`] set with [`ConfigBuilder::auth_provider`], if any.
    pub(crate) fn auth_provider(&self) -> Option<&SharedAuthProvider> {
        self.auth_provider.as_ref()
    }

    /// Whether an [`AuthProvider`] is set.
    pub fn has_auth_provider(&self) -> bool {
        self.auth_provider.is_some()
    }

    /// Get the organization ID, if set.
    pub fn organization(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    /// Get the project ID, if set.
    pub fn project(&self) -> Option<&str> {
        self.project.as_deref()
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
        let base = self.api_base();
        if base.trim_end_matches('/') == OPENAI_BASE_URL {
            None
        } else {
            Some(base)
        }
    }

    /// Get the organization ID, if set.
    pub fn organization_id(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    /// Create an authorization header value.
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key())
    }

    /// Get the custom HTTP client, if set.
    pub fn http_client(&self) -> Option<&HttpClient> {
        self.http_client.as_ref()
    }

    /// Get the Azure deployment name, if set.
    pub fn azure_deployment(&self) -> Option<&str> {
        self.provider
            .as_ref()
            .map_or(self.azure_deployment.as_deref(), Provider::azure_deployment)
    }

    /// Get the Azure API version, if set.
    pub fn azure_api_version(&self) -> Option<&str> {
        match &self.provider {
            Some(provider) if provider.kind() == ProviderKind::Azure => {
                provider.query_param("api-version")
            }
            Some(_) => None,
            None => self.azure_api_version.as_deref(),
        }
    }

    /// Check if this configuration is for Azure `OpenAI`.
    pub fn is_azure(&self) -> bool {
        if let Some(provider) = &self.provider {
            return provider.kind() == ProviderKind::Azure;
        }
        self.azure_deployment.is_some() || self.api_base.contains(".openai.azure.com")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: "https://api.openai.com/v1".to_string(),
            organization: None,
            project: None,
            max_retries: 3,
            default_model: "gpt-4".to_string(),
            http_client: None,
            azure_deployment: None,
            azure_api_version: None,
            provider: None,
            auth_provider: None,
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
    max_retries: Option<u32>,
    default_model: Option<String>,
    http_client: Option<HttpClient>,
    azure_deployment: Option<String>,
    azure_api_version: Option<String>,
    provider: Option<Provider>,
    auth_provider: Option<SharedAuthProvider>,
}

impl std::fmt::Debug for ConfigBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("api_base", &self.api_base)
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
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

    /// Set the maximum number of retries (default: 3).
    ///
    /// The default HTTP client retries `429`, `408` and `5xx` responses and failed connections
    /// up to this many times, with exponential backoff that honours `Retry-After`. `0` disables
    /// retries. It has no effect on a client set with [`ConfigBuilder::http_client`].
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
    /// The client is a [`timvw_rust_runtime::Client`]: a [tower](https://docs.rs/tower) service
    /// stack around an HTTP client (reqwest by default). Use it to configure retries, timeouts,
    /// proxies, connection pooling, logging or any other tower layer.
    ///
    /// When a custom client is set, [`ConfigBuilder::max_retries`] and the default 120 second
    /// timeout do not apply: the client's own layers do. The client still gets the
    /// [provider](ConfigBuilder::provider)'s layer (base URL, credentials, headers, query
    /// parameters and paths, including Azure's), the organization and project headers, and the
    /// [`AuthProvider`], if any.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Config;
    /// use std::time::Duration;
    /// use timvw_rust_runtime::retry::RetryPolicy;
    /// use timvw_rust_runtime::{Client, ReqwestService};
    /// use tower::retry::RetryLayer;
    ///
    /// let reqwest_client = reqwest::Client::builder()
    ///     .connect_timeout(Duration::from_secs(5))
    ///     .timeout(Duration::from_secs(60))
    ///     .build()
    ///     .unwrap();
    ///
    /// // A bare reqwest service, wrapped in a retry layer with up to 5 retries.
    /// let client = Client::from_service(ReqwestService::new(reqwest_client))
    ///     .layer(RetryLayer::new(RetryPolicy::new(5)));
    ///
    /// let config = Config::builder()
    ///     .api_key("sk-...")
    ///     .http_client(client)
    ///     .build();
    /// ```
    #[must_use]
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Set the Azure deployment name.
    ///
    /// Required when using Azure `OpenAI`.
    #[must_use]
    pub fn azure_deployment(mut self, deployment: impl Into<String>) -> Self {
        self.azure_deployment = Some(deployment.into());
        self
    }

    /// Set the Azure API version.
    ///
    /// Defaults to [`AZURE_DEFAULT_API_VERSION`](crate::provider::AZURE_DEFAULT_API_VERSION) ("2024-02-01") if not specified.
    #[must_use]
    pub fn azure_api_version(mut self, version: impl Into<String>) -> Self {
        self.azure_api_version = Some(version.into());
        self
    }

    /// Set the provider: where requests go and how they are authenticated (see
    /// [`Provider`]). It takes the place of [`ConfigBuilder::api_key`],
    /// [`ConfigBuilder::api_base`] and the Azure settings.
    ///
    /// ```rust
    /// use openai_ergonomic::{Config, Provider};
    ///
    /// let config = Config::builder()
    ///     .provider(Provider::groq("gsk-..."))
    ///     .default_model("llama-3.3-70b-versatile")
    ///     .build();
    /// assert_eq!(config.api_base(), "https://api.groq.com/openai/v1");
    /// ```
    #[must_use]
    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set an [`AuthProvider`] that supplies (and refreshes) the credentials of each request to
    /// the provider, such as [`BearerTokenAuth`](crate::auth::BearerTokenAuth). See the
    /// [`auth`](crate::auth) module.
    #[must_use]
    pub fn auth_provider(mut self, auth_provider: impl AuthProvider) -> Self {
        self.auth_provider = Some(SharedAuthProvider::new(auth_provider));
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
            max_retries: self.max_retries.unwrap_or(3),
            default_model: self.default_model.unwrap_or_else(|| "gpt-4".to_string()),
            http_client: self.http_client,
            azure_deployment: self.azure_deployment,
            azure_api_version: self.azure_api_version,
            provider: self.provider,
            auth_provider: self.auth_provider,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_config_builder() {
        let config = Config::builder().api_key("test-key").max_retries(5).build();

        assert_eq!(config.api_key(), "test-key");
        assert_eq!(config.max_retries(), 5);
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
        assert_eq!(config.max_retries(), 3);
        assert_eq!(config.default_model(), Some("gpt-4"));
    }

    #[test]
    fn test_config_with_custom_http_client() {
        let http_client = HttpClient::from_reqwest(
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        );

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
        let http_client = HttpClient::default();
        let config = Config::builder()
            .api_key("test-key")
            .http_client(http_client)
            .build();

        let debug_output = format!("{config:?}");

        // Should show placeholder for HTTP client
        assert!(debug_output.contains("<timvw_rust_runtime::Client>"));
    }
}
