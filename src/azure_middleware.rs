//! Middleware for Azure `OpenAI` authentication.
//!
//! Deprecated: the client now sends requests to Azure `OpenAI` with
//! [`Provider::azure`](crate::Provider::azure), whose [`ProviderLayer`](crate::provider::ProviderLayer)
//! sets the `api-key` header, the deployment path and the `api-version` query parameter, also
//! around an HTTP client of your own (set with
//! [`ConfigBuilder::http_client`](crate::ConfigBuilder::http_client)). Adding this layer to such a
//! client as well is harmless: the paths it sees are already Azure paths.
//!
//! This module provides a [tower](https://docs.rs/tower) layer that adds the appropriate
//! authentication header for Azure `OpenAI` API requests and transforms paths to Azure format,
//! whatever the host.
//!
//! ```rust
//! # #![allow(deprecated)]
//! use openai_ergonomic::azure_middleware::AzureAuthMiddleware;
//!
//! let client = timvw_rust_runtime::Client::default().layer(AzureAuthMiddleware::new(
//!     "azure-api-key".to_string(),
//!     Some("2024-02-01".to_string()),
//!     Some("my-deployment".to_string()),
//! ));
//! # let _ = client;
//! ```

#![allow(deprecated)]

use http::header::{HeaderName, HeaderValue};
use http::Request;
use std::sync::Arc;
use std::task::{Context, Poll};
use timvw_rust_runtime::Body;
use tower::{Layer, Service};

const API_KEY: HeaderName = HeaderName::from_static("api-key");

/// Tower layer that adds Azure `OpenAI` authentication headers and transforms paths.
///
/// For each request it:
/// - sets the `api-key` header (marked sensitive);
/// - when a deployment is configured, rewrites `/v1/...` to `/openai/deployments/{deployment}/...`
///   (and any other path not under `/openai/` to `/openai/deployments/{deployment}{path}`), adding
///   an `api-version` query parameter unless one is present.
#[derive(Clone)]
#[deprecated(
    since = "0.20.0",
    note = "use `Provider::azure` (`Config::builder().provider(..)`), or `Provider::azure(..).layer()` for a client of your own"
)]
pub struct AzureAuthMiddleware {
    api_key: Arc<String>,
    api_version: Arc<String>,
    deployment: Arc<Option<String>>,
}

impl std::fmt::Debug for AzureAuthMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureAuthMiddleware")
            .field("api_key", &"***")
            .field("api_version", &self.api_version)
            .field("deployment", &self.deployment)
            .finish()
    }
}

impl AzureAuthMiddleware {
    /// Create a new Azure authentication middleware.
    pub fn new(api_key: String, api_version: Option<String>, deployment: Option<String>) -> Self {
        Self {
            api_key: Arc::new(api_key),
            api_version: Arc::new(api_version.unwrap_or_else(|| "2024-02-01".to_string())),
            deployment: Arc::new(deployment),
        }
    }

    /// Apply the Azure header and path rewrite to a request.
    fn apply(&self, request: &mut Request<Body>) -> std::result::Result<(), String> {
        let mut api_key = HeaderValue::from_str(&self.api_key)
            .map_err(|_| "the Azure API key is not a valid header value".to_string())?;
        api_key.set_sensitive(true);
        request.headers_mut().insert(API_KEY, api_key);

        let Some(deployment) = self.deployment.as_ref() else {
            return Ok(());
        };

        // Azure `OpenAI` uses paths like: /openai/deployments/{deployment-id}/chat/completions
        // Standard `OpenAI` uses: /v1/chat/completions
        // We need to transform both /v1/* and /* to /openai/deployments/{deployment}/*
        let path = request.uri().path();
        let new_path = if path.starts_with("/v1/") {
            path.replacen("/v1/", &format!("/openai/deployments/{deployment}/"), 1)
        } else if !path.starts_with("/openai/") {
            format!("/openai/deployments/{deployment}{path}")
        } else {
            return Ok(());
        };

        let mut url = url::Url::parse(&request.uri().to_string())
            .map_err(|e| format!("invalid request URL: {e}"))?;
        url.set_path(&new_path);
        if url.query_pairs().all(|(key, _)| key != "api-version") {
            url.query_pairs_mut()
                .append_pair("api-version", &self.api_version);
        }
        *request.uri_mut() = url
            .as_str()
            .parse()
            .map_err(|e| format!("invalid request URL: {e}"))?;
        Ok(())
    }
}

impl<S> Layer<S> for AzureAuthMiddleware {
    type Service = AzureAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AzureAuthService {
            inner,
            middleware: self.clone(),
        }
    }
}

/// The service made by [`AzureAuthMiddleware`].
#[derive(Clone, Debug)]
pub struct AzureAuthService<S> {
    inner: S,
    middleware: AzureAuthMiddleware,
}

impl<S> Service<Request<Body>> for AzureAuthService<S>
where
    S: Service<Request<Body>>,
    S::Error: Into<timvw_rust_runtime::BoxError>,
{
    type Response = S::Response;
    type Error = timvw_rust_runtime::BoxError;
    type Future = futures::future::Either<
        futures::future::MapErr<S::Future, fn(S::Error) -> timvw_rust_runtime::BoxError>,
        futures::future::Ready<std::result::Result<S::Response, timvw_rust_runtime::BoxError>>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        use futures::TryFutureExt;
        match self.middleware.apply(&mut request) {
            Ok(()) => futures::future::Either::Left(
                self.inner
                    .call(request)
                    .map_err(Into::into as fn(S::Error) -> timvw_rust_runtime::BoxError),
            ),
            Err(message) => {
                futures::future::Either::Right(futures::future::ready(Err(message.into())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn rewrite(middleware: &AzureAuthMiddleware, uri: &str) -> Request<Body> {
        let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        middleware.apply(&mut request).unwrap();
        request
    }

    #[test]
    fn rewrites_v1_paths_and_adds_api_version() {
        let middleware = AzureAuthMiddleware::new("key".into(), None, Some("my-deployment".into()));
        let request = rewrite(
            &middleware,
            "https://res.openai.azure.com/v1/chat/completions",
        );
        assert_eq!(
            request.uri(),
            "https://res.openai.azure.com/openai/deployments/my-deployment/chat/completions?api-version=2024-02-01"
        );
        assert_eq!(request.headers()["api-key"], "key");
        assert!(request.headers()["api-key"].is_sensitive());
    }

    #[test]
    fn prefixes_other_paths_and_keeps_existing_api_version() {
        let middleware =
            AzureAuthMiddleware::new("key".into(), Some("2025-01-01".into()), Some("dep".into()));
        let request = rewrite(
            &middleware,
            "https://res.openai.azure.com/embeddings?api-version=2023-05-15&x=1",
        );
        assert_eq!(
            request.uri(),
            "https://res.openai.azure.com/openai/deployments/dep/embeddings?api-version=2023-05-15&x=1"
        );

        let request = rewrite(&middleware, "https://res.openai.azure.com/models");
        assert_eq!(
            request.uri(),
            "https://res.openai.azure.com/openai/deployments/dep/models?api-version=2025-01-01"
        );
    }

    #[test]
    fn leaves_azure_paths_and_deploymentless_requests_alone() {
        let middleware = AzureAuthMiddleware::new("key".into(), None, Some("dep".into()));
        let uri = "https://res.openai.azure.com/openai/deployments/dep/chat/completions";
        assert_eq!(rewrite(&middleware, uri).uri(), uri);

        let middleware = AzureAuthMiddleware::new("key".into(), None, None);
        let uri = "https://res.openai.azure.com/v1/chat/completions";
        let request = rewrite(&middleware, uri);
        assert_eq!(request.uri(), uri);
        assert_eq!(request.headers()["api-key"], "key");
    }

    #[tokio::test]
    async fn works_as_a_client_layer() {
        use timvw_rust_runtime::{BoxError, Client, Method};
        let client = Client::from_service(tower::service_fn(|request: Request<Body>| async move {
            assert_eq!(
                request.uri().path(),
                "/openai/deployments/dep/chat/completions"
            );
            assert_eq!(request.headers()["api-key"], "key");
            Ok::<_, BoxError>(http::Response::new(Body::from("ok")))
        }))
        .layer(AzureAuthMiddleware::new(
            "key".into(),
            None,
            Some("dep".into()),
        ));
        let response = client
            .request(
                Method::POST,
                "https://res.openai.azure.com/v1/chat/completions",
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.text().await.unwrap(), "ok");
    }
}
