//! Providers: `OpenAI` and the services that serve the same API under another URL, with their
//! own authentication, headers, query parameters and paths.
//!
//! A [`Provider`] says where requests go and how they are authenticated. Set it with
//! [`ConfigBuilder::provider`](crate::ConfigBuilder::provider): the client then sends every
//! request (the ergonomic builders, the generated `openai-client-base` operations and the raw
//! [`Client::send_json`](crate::Client::send_json) calls) to the provider's base URL, through a
//! [`ProviderLayer`] that adds the provider's credentials, headers and query parameters and
//! rewrites its paths.
//!
//! The layer only touches requests under the provider's base URL: a request to another host
//! (the download of a generated image, say) is sent as is, without the provider's credentials.
//!
//! Per-call [`RequestOptions`](timvw_rust_runtime::RequestOptions), which the generated
//! operations take as `.options(..)`, win over the provider: a header or query parameter they set
//! (an Azure `api-version`, say) is sent instead of the provider's, once. A per-call
//! [`base_url`](timvw_rust_runtime::RequestOptions::base_url) under the provider's base URL is
//! still mapped and authenticated; any other base URL gets nothing from the provider (no
//! credentials, headers or query parameters), and no [`AuthProvider`](crate::AuthProvider)
//! credentials either.
//!
//! ```rust
//! use openai_ergonomic::{Client, Config, Provider};
//!
//! # fn main() -> openai_ergonomic::Result<()> {
//! // OpenRouter, with the optional attribution headers.
//! let config = Config::builder()
//!     .provider(
//!         Provider::openrouter("sk-or-...")
//!             .http_referer("https://my-app.example")
//!             .title("My App"),
//!     )
//!     .default_model("openai/gpt-4o-mini")
//!     .build();
//! let client = Client::builder(config)?.build();
//!
//! // Azure OpenAI: `api-key` header, `/openai/deployments/{deployment}` paths, `api-version`.
//! let config = Config::builder()
//!     .provider(
//!         Provider::azure("https://my-resource.openai.azure.com", "azure-key", "gpt-4o")
//!             .api_version("2024-10-21"),
//!     )
//!     .build();
//!
//! // A local Ollama server: no key.
//! let config = Config::builder().provider(Provider::ollama()).build();
//!
//! // Anything else that speaks the OpenAI API.
//! let config = Config::builder()
//!     .provider(
//!         Provider::custom("https://gateway.example.com/v1")
//!             .bearer("gateway-token")
//!             .header("x-team", "search")
//!             .query("tenant", "acme"),
//!     )
//!     .build();
//! # let _ = (client, config);
//! # Ok(())
//! # }
//! ```

use crate::{Error, Result};
use http::header::{HeaderName, HeaderValue, AUTHORIZATION};
use std::fmt;
use std::sync::Arc;
use timvw_rust_runtime::{ExposeSecret, SecretString};

pub use timvw_rust_runtime::provider::ProviderLayer;

/// The base URL of the `OpenAI` API.
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
/// The base URL of the `OpenRouter` API.
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
/// The base URL of Groq's `OpenAI`-compatible API.
pub const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
/// The base URL of a local Ollama server's `OpenAI`-compatible API.
pub const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
/// The Azure `OpenAI` `api-version` used when none is set.
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-02-01";

/// Which preset a [`Provider`] was made with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProviderKind {
    /// `OpenAI` ([`Provider::openai`]).
    OpenAi,
    /// Azure `OpenAI` with deployments ([`Provider::azure`], [`Provider::azure_entra`]).
    Azure,
    /// `OpenRouter` ([`Provider::openrouter`]).
    OpenRouter,
    /// Groq ([`Provider::groq`]).
    Groq,
    /// Ollama ([`Provider::ollama`]).
    Ollama,
    /// Any other `OpenAI`-compatible API ([`Provider::custom`]).
    Custom,
}

/// How a provider authenticates requests.
#[derive(Clone)]
enum Credential {
    None,
    /// `Authorization: Bearer <token>`.
    Bearer(SecretString),
    /// A header that holds the key as is, such as Azure's `api-key`.
    Header(String, SecretString),
}

type MapPath = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// Where requests go and how they are authenticated: `OpenAI` (the default), Azure `OpenAI`,
/// `OpenRouter`, Groq, Ollama, or any `OpenAI`-compatible API.
///
/// See the [module documentation](self) for examples. Every preset can be adjusted: another
/// [base URL](Provider::base_url) (a proxy, a mock server), extra [headers](Provider::header)
/// and [query parameters](Provider::query), or another [credential](Provider::bearer).
///
/// Invalid header names or values are reported when the client is built, as
/// [`Error::Config`].
#[derive(Clone)]
pub struct Provider {
    kind: ProviderKind,
    base_url: String,
    credential: Credential,
    headers: Vec<(String, String, bool)>,
    query: Vec<(String, String)>,
    map_path: Option<MapPath>,
    azure_deployment: Option<String>,
}

impl Provider {
    fn new(kind: ProviderKind, base_url: impl Into<String>, credential: Credential) -> Self {
        Self {
            kind,
            base_url: trim_base(&base_url.into()),
            credential,
            headers: Vec::new(),
            query: Vec::new(),
            map_path: None,
            azure_deployment: None,
        }
    }

    /// `OpenAI` at `https://api.openai.com/v1`, with `Authorization: Bearer <api_key>`.
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::new(
            ProviderKind::OpenAi,
            OPENAI_BASE_URL,
            bearer(api_key.into()),
        )
    }

    /// Azure `OpenAI` with a deployment: the key in the `api-key` header, requests under
    /// `{endpoint}/openai/deployments/{deployment}`, and an `api-version` query parameter
    /// ([`AZURE_DEFAULT_API_VERSION`] unless set with [`Provider::api_version`]).
    ///
    /// `endpoint` is the resource's endpoint, such as `https://my-resource.openai.azure.com`
    /// (anything from `/openai` on is ignored).
    pub fn azure(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        deployment: impl Into<String>,
    ) -> Self {
        Self::azure_with(
            &endpoint.into(),
            Credential::Header("api-key".to_string(), SecretString::from(api_key.into())),
            deployment.into(),
        )
    }

    /// Azure `OpenAI` with a deployment and Microsoft Entra ID (Azure AD) authentication: like
    /// [`Provider::azure`] without a key. Add an
    /// [`AuthProvider`](crate::auth::AuthProvider) that sets the token, such as
    /// [`BearerTokenAuth`](crate::auth::BearerTokenAuth), with
    /// [`ConfigBuilder::auth_provider`](crate::ConfigBuilder::auth_provider).
    pub fn azure_entra(endpoint: impl Into<String>, deployment: impl Into<String>) -> Self {
        Self::azure_with(&endpoint.into(), Credential::None, deployment.into())
    }

    fn azure_with(endpoint: &str, credential: Credential, deployment: String) -> Self {
        let endpoint = trim_base(endpoint);
        let endpoint = match endpoint.find("/openai") {
            Some(index) if endpoint[..index].contains("://") => endpoint[..index].to_string(),
            _ => endpoint,
        };
        let mut provider = Self::new(
            ProviderKind::Azure,
            format!("{endpoint}/openai"),
            credential,
        );
        let prefix = format!("/deployments/{deployment}");
        provider.map_path = Some(Arc::new(move |path: &str| {
            if path.starts_with("/deployments/") {
                path.to_string()
            } else {
                format!("{prefix}{path}")
            }
        }));
        provider.azure_deployment = Some(deployment);
        provider.query = vec![(
            "api-version".to_string(),
            AZURE_DEFAULT_API_VERSION.to_string(),
        )];
        provider
    }

    /// `OpenRouter` at `https://openrouter.ai/api/v1`, with `Authorization: Bearer <api_key>`.
    /// See [`Provider::http_referer`] and [`Provider::title`] for its attribution headers.
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::new(
            ProviderKind::OpenRouter,
            OPENROUTER_BASE_URL,
            bearer(api_key.into()),
        )
    }

    /// Groq at `https://api.groq.com/openai/v1`, with `Authorization: Bearer <api_key>`.
    pub fn groq(api_key: impl Into<String>) -> Self {
        Self::new(ProviderKind::Groq, GROQ_BASE_URL, bearer(api_key.into()))
    }

    /// A local Ollama server at `http://localhost:11434/v1`, without a key. Use
    /// [`Provider::base_url`] for another host.
    pub fn ollama() -> Self {
        Self::new(ProviderKind::Ollama, OLLAMA_BASE_URL, Credential::None)
    }

    /// Any `OpenAI`-compatible API at `base_url` (such as `https://gateway.example.com/v1`),
    /// without credentials: add them with [`Provider::bearer`], [`Provider::api_key_header`] or
    /// [`Provider::header`].
    pub fn custom(base_url: impl Into<String>) -> Self {
        Self::new(ProviderKind::Custom, base_url, Credential::None)
    }

    /// Sends the requests to another base URL: a proxy, a regional endpoint, a mock server.
    /// The provider's credentials, headers, query parameters and paths stay as they are.
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        let base_url = trim_base(&base_url.into());
        // For Azure the paths live under `/openai`.
        self.base_url = if self.kind == ProviderKind::Azure && !base_url.ends_with("/openai") {
            format!("{base_url}/openai")
        } else {
            base_url
        };
        self
    }

    /// Authenticates with `Authorization: Bearer <token>`, replacing the provider's credential.
    #[must_use]
    pub fn bearer(mut self, token: impl Into<String>) -> Self {
        self.credential = bearer(token.into());
        self
    }

    /// Authenticates with the key as is in header `name` (such as `api-key` or `x-api-key`),
    /// replacing the provider's credential.
    #[must_use]
    pub fn api_key_header(mut self, name: impl Into<String>, key: impl Into<String>) -> Self {
        self.credential = Credential::Header(name.into(), SecretString::from(key.into()));
        self
    }

    /// Sends no credential (for a server without authentication, or one whose credentials an
    /// [`AuthProvider`](crate::auth::AuthProvider) sets).
    #[must_use]
    pub fn without_credential(mut self) -> Self {
        self.credential = Credential::None;
        self
    }

    /// Sets a header on every request to the provider, replacing any value it has.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_header(name.into(), value.into(), false);
        self
    }

    /// Like [`Provider::header`], for a secret: the value is left out of `Debug` output.
    #[must_use]
    pub fn sensitive_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_header(name.into(), value.into(), true);
        self
    }

    fn set_header(&mut self, name: String, value: String, sensitive: bool) {
        self.headers
            .retain(|(existing, _, _)| !existing.eq_ignore_ascii_case(&name));
        self.headers.push((name, value, sensitive));
    }

    /// Sets a query parameter on every request to the provider, replacing any parameter of that
    /// name the request has.
    #[must_use]
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.query.retain(|(existing, _)| *existing != name);
        self.query.push((name, value.into()));
        self
    }

    /// Rewrites the path of every request to the provider: `f` gets the path after the base URL
    /// (such as `/chat/completions`) and returns the one to send.
    #[must_use]
    pub fn map_path<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.map_path = Some(Arc::new(f));
        self
    }

    /// Sets the Azure `api-version` query parameter (the `api-version` query parameter of any
    /// other provider).
    #[must_use]
    pub fn api_version(self, version: impl Into<String>) -> Self {
        self.query("api-version", version)
    }

    /// `OpenRouter`'s `HTTP-Referer` header: the URL of your app, for its rankings.
    #[must_use]
    pub fn http_referer(self, url: impl Into<String>) -> Self {
        self.header("HTTP-Referer", url)
    }

    /// `OpenRouter`'s `X-Title` header: the name of your app, for its rankings.
    #[must_use]
    pub fn title(self, title: impl Into<String>) -> Self {
        self.header("X-Title", title)
    }

    /// The preset the provider was made with.
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// The base URL requests are sent to, without a trailing `/`.
    pub fn api_base(&self) -> &str {
        &self.base_url
    }

    /// The key or token of the provider's credential, if it has one.
    pub fn api_key(&self) -> Option<&str> {
        match &self.credential {
            Credential::None => None,
            Credential::Bearer(token) | Credential::Header(_, token) => Some(token.expose_secret()),
        }
    }

    /// The Azure deployment, for [`Provider::azure`] and [`Provider::azure_entra`].
    pub fn azure_deployment(&self) -> Option<&str> {
        self.azure_deployment.as_deref()
    }

    /// The value of query parameter `name`, such as Azure's `api-version`.
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, value)| value.as_str())
    }

    /// The tower layer that sends requests to this provider. The client adds it; use it to
    /// point an HTTP client of your own at the provider.
    ///
    /// `extra_headers` are set on every request too (the client passes the organization and
    /// project headers).
    pub fn layer(&self) -> Result<ProviderLayer> {
        self.layer_with(Vec::new())
    }

    pub(crate) fn layer_with(
        &self,
        extra_headers: Vec<(HeaderName, HeaderValue)>,
    ) -> Result<ProviderLayer> {
        let config_error = |error: timvw_rust_runtime::Error| {
            Error::Config(format!(
                "invalid {:?} provider configuration: {error}{}",
                self.kind,
                std::error::Error::source(&error)
                    .map(|source| format!(": {source}"))
                    .unwrap_or_default()
            ))
        };
        let mut layer = ProviderLayer::new(&self.base_url).map_err(config_error)?;
        match &self.credential {
            Credential::None => {}
            Credential::Bearer(token) => {
                if !token.expose_secret().is_empty() {
                    layer = layer
                        .sensitive_header(
                            AUTHORIZATION,
                            format!("Bearer {}", token.expose_secret()),
                        )
                        .map_err(config_error)?;
                }
            }
            Credential::Header(name, key) => {
                layer = layer
                    .sensitive_header(name.as_str(), key.expose_secret())
                    .map_err(config_error)?;
            }
        }
        for (name, value, sensitive) in &self.headers {
            layer = if *sensitive {
                layer.sensitive_header(name.as_str(), value.as_str())
            } else {
                layer.header(name.as_str(), value.as_str())
            }
            .map_err(config_error)?;
        }
        for (name, value) in extra_headers {
            layer = layer.header(name, value).map_err(config_error)?;
        }
        if !self.query.is_empty() {
            layer = layer.query(&self.query).map_err(config_error)?;
        }
        if let Some(map_path) = self.map_path.clone() {
            layer = layer.map_path(move |path| map_path(path));
        }
        Ok(layer)
    }
}

impl Default for Provider {
    /// `OpenAI` without a key.
    fn default() -> Self {
        Self::openai(String::new())
    }
}

impl fmt::Debug for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let credential = match &self.credential {
            Credential::None => "none".to_string(),
            Credential::Bearer(_) => "bearer ***".to_string(),
            Credential::Header(name, _) => format!("{name}: ***"),
        };
        let headers: Vec<(&str, &str)> = self
            .headers
            .iter()
            .map(|(name, value, sensitive)| {
                (
                    name.as_str(),
                    if *sensitive { "***" } else { value.as_str() },
                )
            })
            .collect();
        f.debug_struct("Provider")
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .field("credential", &credential)
            .field("headers", &headers)
            .field("query", &self.query)
            .field("map_path", &self.map_path.is_some())
            .field("azure_deployment", &self.azure_deployment)
            .finish()
    }
}

fn bearer(token: String) -> Credential {
    Credential::Bearer(SecretString::from(token))
}

fn trim_base(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn rewrite(provider: &Provider, uri: &str) -> String {
        provider
            .layer()
            .unwrap()
            .rewrite_uri(&uri.parse().unwrap())
            .unwrap()
            .to_string()
    }

    #[test]
    fn azure_paths_and_api_version() {
        let provider = Provider::azure("https://res.openai.azure.com/", "key", "my-dep");
        assert_eq!(provider.api_base(), "https://res.openai.azure.com/openai");
        assert_eq!(
            rewrite(
                &provider,
                "https://res.openai.azure.com/openai/chat/completions"
            ),
            "https://res.openai.azure.com/openai/deployments/my-dep/chat/completions?api-version=2024-02-01"
        );
        let provider = Provider::azure(
            "https://res.openai.azure.com/openai/deployments/x",
            "key",
            "dep",
        )
        .api_version("2024-10-21");
        assert_eq!(provider.api_base(), "https://res.openai.azure.com/openai");
        assert_eq!(provider.query_param("api-version"), Some("2024-10-21"));
        assert_eq!(
            rewrite(
                &provider,
                "https://res.openai.azure.com/openai/deployments/dep/embeddings?api-version=old"
            ),
            "https://res.openai.azure.com/openai/deployments/dep/embeddings?api-version=2024-10-21"
        );
        assert_eq!(
            provider.layer().unwrap().header_map()["api-key"]
                .to_str()
                .unwrap(),
            "key"
        );
    }

    #[test]
    fn presets() {
        assert_eq!(Provider::openai("k").api_base(), OPENAI_BASE_URL);
        assert_eq!(Provider::groq("k").api_base(), GROQ_BASE_URL);
        assert_eq!(Provider::openrouter("k").api_base(), OPENROUTER_BASE_URL);
        assert_eq!(Provider::ollama().api_base(), OLLAMA_BASE_URL);
        assert_eq!(Provider::ollama().api_key(), None);
        assert!(Provider::ollama().layer().unwrap().header_map().is_empty());
        let layer = Provider::openrouter("sk-or")
            .http_referer("https://app.example")
            .title("App")
            .layer()
            .unwrap();
        let headers = layer.header_map();
        assert_eq!(headers["authorization"], "Bearer sk-or");
        assert!(headers["authorization"].is_sensitive());
        assert_eq!(headers["http-referer"], "https://app.example");
        assert_eq!(headers["x-title"], "App");
    }

    #[test]
    fn invalid_configuration_is_a_config_error() {
        assert!(matches!(
            Provider::custom("not a url").layer(),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            Provider::custom("https://p.example")
                .header("bad name", "v")
                .layer(),
            Err(Error::Config(_))
        ));
    }

    #[test]
    fn debug_hides_secrets() {
        let provider = Provider::custom("https://p.example")
            .bearer("secret-token")
            .sensitive_header("x-secret", "hidden-value")
            .header("x-visible", "shown");
        let debug = format!("{provider:?}");
        assert!(!debug.contains("secret-token"));
        assert!(!debug.contains("hidden-value"));
        assert!(debug.contains("shown"));
    }
}
