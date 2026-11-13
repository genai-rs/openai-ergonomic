//! Pluggable authentication: credentials supplied, and refreshed, per request.
//!
//! A static API key is part of the [`Provider`](crate::Provider). For credentials that change
//! (Microsoft Entra ID / Azure AD tokens, short-lived gateway tokens, request signing) implement
//! [`AuthProvider`] and set it with
//! [`ConfigBuilder::auth_provider`](crate::ConfigBuilder::auth_provider):
//!
//! - [`AuthProvider::apply`] runs before every request to the provider and returns the request
//!   to send, with its credentials (and can sign it: the URL, headers and body are there);
//! - when the provider answers `401 Unauthorized`, [`AuthProvider::refresh_after_unauthorized`]
//!   gets the headers of the rejected request; if it returns `true`, the request is authenticated
//!   again with [`AuthProvider::apply`] and sent once more. A second `401` is returned as is.
//!
//! The client runs it in an [`AuthLayer`] around its HTTP client, so the generated
//! `openai-client-base` operations and the raw requests are authenticated too. A request can be
//! sent again only when its body can be (JSON bodies and file uploads can; a one-shot stream
//! cannot, and its `401` is returned).
//!
//! When several requests fail with a `401` at the same time, each calls
//! `refresh_after_unauthorized`. An implementation should refresh once: compare the rejected
//! credential with the current one, and only fetch a new one when they are the same.
//! [`BearerTokenAuth`] does that for bearer tokens:
//!
//! ```rust,no_run
//! use openai_ergonomic::auth::{AuthToken, BearerTokenAuth};
//! use openai_ergonomic::{Client, Config, Provider};
//! use std::time::Duration;
//!
//! # async fn fetch_entra_token() -> openai_ergonomic::Result<String> { Ok(String::new()) }
//! # fn main() -> openai_ergonomic::Result<()> {
//! let auth = BearerTokenAuth::new(|| async {
//!     let token = fetch_entra_token().await?;
//!     Ok(AuthToken::new(token).expires_in(Duration::from_secs(3600)))
//! });
//! let config = Config::builder()
//!     .provider(Provider::azure_entra("https://my-resource.openai.azure.com", "gpt-4o"))
//!     .auth_provider(auth)
//!     .build();
//! let client = Client::builder(config)?.build();
//! # let _ = client;
//! # Ok(())
//! # }
//! ```

use crate::{BoxFuture, Error, Result};
use http::header::{HeaderName, HeaderValue, AUTHORIZATION};
use http::{HeaderMap, Request, StatusCode};
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use timvw_rust_runtime::{Body, BoxError, ExposeSecret, SecretString};
use tower::{Layer, Service, ServiceExt};

/// Supplies the credentials of each request, and refreshes them after a `401`.
///
/// Implement it with `async fn`s; the futures must be `Send`. See the
/// [module documentation](self).
///
/// ```rust
/// use http::{HeaderMap, Request};
/// use openai_ergonomic::auth::AuthProvider;
/// use openai_ergonomic::Result;
/// use timvw_rust_runtime::Body;
///
/// /// Signs each request with a header computed from its URL.
/// struct Signer;
///
/// impl AuthProvider for Signer {
///     async fn apply(&self, mut request: Request<Body>) -> Result<Request<Body>> {
///         let signature = format!("sig-{}", request.uri().path().len());
///         request.headers_mut().insert("x-signature", signature.parse().unwrap());
///         Ok(request)
///     }
/// }
/// ```
pub trait AuthProvider: Send + Sync + 'static {
    /// Adds the credentials to a request to the provider, and returns the request to send.
    ///
    /// An error fails the request without sending it; it reaches the caller as
    /// [`Error::Http`](crate::Error::Http) whose source is the error returned here.
    fn apply(&self, request: Request<Body>) -> impl Future<Output = Result<Request<Body>>> + Send;

    /// Called when the provider rejected a request with `401 Unauthorized`, with the headers
    /// that request was sent with. Returns whether new credentials are available, in which case
    /// the request is authenticated again and sent once more.
    ///
    /// The default refreshes nothing: the `401` is returned.
    fn refresh_after_unauthorized(
        &self,
        rejected_headers: &HeaderMap,
    ) -> impl Future<Output = Result<bool>> + Send {
        let _ = rejected_headers;
        async { Ok(false) }
    }
}

/// [`AuthProvider`] with boxed futures, to be stored as a trait object.
pub(crate) trait ErasedAuthProvider: Send + Sync {
    fn apply(&self, request: Request<Body>) -> BoxFuture<'_, Result<Request<Body>>>;
    fn refresh_after_unauthorized<'a>(
        &'a self,
        rejected_headers: &'a HeaderMap,
    ) -> BoxFuture<'a, Result<bool>>;
}

impl<A: AuthProvider> ErasedAuthProvider for A {
    fn apply(&self, request: Request<Body>) -> BoxFuture<'_, Result<Request<Body>>> {
        Box::pin(AuthProvider::apply(self, request))
    }

    fn refresh_after_unauthorized<'a>(
        &'a self,
        rejected_headers: &'a HeaderMap,
    ) -> BoxFuture<'a, Result<bool>> {
        Box::pin(AuthProvider::refresh_after_unauthorized(
            self,
            rejected_headers,
        ))
    }
}

/// A shared [`AuthProvider`].
#[derive(Clone)]
pub(crate) struct SharedAuthProvider(pub(crate) Arc<dyn ErasedAuthProvider>);

impl SharedAuthProvider {
    pub(crate) fn new(provider: impl AuthProvider) -> Self {
        Self(Arc::new(provider))
    }
}

impl fmt::Debug for SharedAuthProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<AuthProvider>")
    }
}

/// A tower layer that authenticates requests with an [`AuthProvider`], and on a `401`
/// refreshes the credentials and sends the request once more.
///
/// The client adds it when [`ConfigBuilder::auth_provider`](crate::ConfigBuilder::auth_provider)
/// is set, scoped to the provider's base URL. Add it to an HTTP client of your own with
/// [`timvw_rust_runtime::Client::layer`].
#[derive(Clone)]
pub struct AuthLayer {
    provider: SharedAuthProvider,
    scope: Option<url::Url>,
}

impl AuthLayer {
    /// A layer that authenticates every request with `provider`.
    pub fn new(provider: impl AuthProvider) -> Self {
        Self::from_shared(SharedAuthProvider::new(provider))
    }

    pub(crate) fn from_shared(provider: SharedAuthProvider) -> Self {
        Self {
            provider,
            scope: None,
        }
    }

    /// Only authenticates requests under `base_url` (same scheme, host and port, and a path
    /// that starts with its path), so that credentials are never sent to another host. Other
    /// requests are passed on unchanged.
    pub fn scope(mut self, base_url: &str) -> Result<Self> {
        let url = url::Url::parse(base_url)
            .map_err(|e| Error::Config(format!("invalid auth scope `{base_url}`: {e}")))?;
        self.scope = Some(url);
        Ok(self)
    }
}

impl fmt::Debug for AuthLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthLayer")
            .field("scope", &self.scope.as_ref().map(url::Url::as_str))
            .finish_non_exhaustive()
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            layer: self.clone(),
        }
    }
}

/// The service of an [`AuthLayer`].
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    layer: AuthLayer,
}

impl<S> fmt::Debug for AuthService<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService")
            .field("layer", &self.layer)
            .finish_non_exhaustive()
    }
}

/// Whether `uri` is under `base`: same scheme, host and port, and a path under the base's at a
/// segment boundary.
fn in_scope(base: &url::Url, uri: &http::Uri) -> bool {
    let Ok(url) = url::Url::parse(&uri.to_string()) else {
        return false;
    };
    if url.scheme() != base.scheme()
        || url.host_str() != base.host_str()
        || url.port_or_known_default() != base.port_or_known_default()
    {
        return false;
    }
    let prefix = base.path().trim_end_matches('/');
    url.path()
        .strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// A copy of a request whose body can be sent again.
fn try_clone_request(request: &Request<Body>) -> Option<Request<Body>> {
    let body = request.body().try_clone()?;
    let mut copy = Request::new(body);
    *copy.method_mut() = request.method().clone();
    *copy.uri_mut() = request.uri().clone();
    *copy.version_mut() = request.version();
    *copy.headers_mut() = request.headers().clone();
    Some(copy)
}

impl<S, B> Service<Request<Body>> for AuthService<S>
where
    S: Service<Request<Body>, Response = http::Response<B>> + Clone + Send + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = http::Response<B>;
    type Error = BoxError;
    type Future = BoxFuture<'static, std::result::Result<http::Response<B>, BoxError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), BoxError>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // The service that was polled ready handles the first attempt; a clone the retry.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let layer = self.layer.clone();
        Box::pin(async move {
            if layer
                .scope
                .as_ref()
                .is_some_and(|scope| !in_scope(scope, request.uri()))
            {
                return inner.call(request).await.map_err(Into::into);
            }
            let auth = &layer.provider.0;
            let retry = try_clone_request(&request);
            let request = auth.apply(request).await.map_err(BoxError::from)?;
            let sent_headers = retry.is_some().then(|| request.headers().clone());
            let response = inner.call(request).await.map_err(Into::into)?;
            if response.status() != StatusCode::UNAUTHORIZED {
                return Ok(response);
            }
            let (Some(retry), Some(sent_headers)) = (retry, sent_headers) else {
                return Ok(response);
            };
            if !auth
                .refresh_after_unauthorized(&sent_headers)
                .await
                .map_err(BoxError::from)?
            {
                return Ok(response);
            }
            drop(response);
            let retry = auth.apply(retry).await.map_err(BoxError::from)?;
            let mut inner = inner;
            inner.ready().await.map_err(Into::into)?;
            inner.call(retry).await.map_err(Into::into)
        })
    }
}

/// A token and when it expires, as [`BearerTokenAuth`]'s fetch function returns it.
#[derive(Clone)]
pub struct AuthToken {
    value: SecretString,
    expires_at: Option<Instant>,
}

impl AuthToken {
    /// A token that does not expire (until the provider rejects it).
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: SecretString::from(value.into()),
            expires_at: None,
        }
    }

    /// The token expires after `lifetime`: it is fetched again shortly (30 s) before that.
    #[must_use]
    pub fn expires_in(mut self, lifetime: Duration) -> Self {
        self.expires_at = Some(Instant::now() + lifetime);
        self
    }

    /// The token expires at `instant`: it is fetched again shortly (30 s) before that.
    #[must_use]
    pub fn expires_at(mut self, instant: Instant) -> Self {
        self.expires_at = Some(instant);
        self
    }

    fn is_fresh(&self) -> bool {
        self.expires_at
            .is_none_or(|at| Instant::now() + EXPIRY_MARGIN < at)
    }
}

impl fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthToken")
            .field("value", &"***")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

const EXPIRY_MARGIN: Duration = Duration::from_secs(30);

type Fetch = Arc<dyn Fn() -> BoxFuture<'static, Result<AuthToken>> + Send + Sync>;

/// An [`AuthProvider`] for bearer tokens fetched with a function of yours.
///
/// It caches the token, fetches a new one before it expires and after a `401`, and sets it as
/// `Authorization: Bearer <token>` (or in another header, see [`BearerTokenAuth::header`]).
///
/// Concurrent requests share one fetch: a request that finds the token being fetched waits for
/// it, and when several requests are rejected with the same token, only the first fetches a new
/// one.
#[derive(Clone)]
pub struct BearerTokenAuth {
    fetch: Fetch,
    header: HeaderName,
    scheme: Option<String>,
    current: Arc<tokio::sync::Mutex<Option<AuthToken>>>,
}

impl BearerTokenAuth {
    /// Tokens fetched with `fetch`.
    pub fn new<F, Fut>(fetch: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<AuthToken>> + Send + 'static,
    {
        Self {
            fetch: Arc::new(move || Box::pin(fetch())),
            header: AUTHORIZATION,
            scheme: Some("Bearer".to_string()),
            current: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Sets the token in header `name` as is, without the `Bearer ` scheme (such as Azure's
    /// `api-key`).
    #[must_use]
    pub fn header(mut self, name: HeaderName) -> Self {
        self.header = name;
        self.scheme = None;
        self
    }

    fn header_value(&self, token: &AuthToken) -> Result<HeaderValue> {
        let secret = token.value.expose_secret();
        let value = self
            .scheme
            .as_ref()
            .map_or_else(|| secret.to_string(), |scheme| format!("{scheme} {secret}"));
        let mut value = HeaderValue::from_str(&value)
            .map_err(|_| Error::Authentication("the token is not a valid header value".into()))?;
        value.set_sensitive(true);
        Ok(value)
    }

    /// The current token, fetched when there is none or it is about to expire.
    async fn token(&self) -> Result<HeaderValue> {
        let mut current = self.current.lock().await;
        if let Some(token) = current.as_ref().filter(|token| token.is_fresh()) {
            return self.header_value(token);
        }
        let token = (self.fetch)().await?;
        let value = self.header_value(&token)?;
        *current = Some(token);
        drop(current);
        Ok(value)
    }
}

impl fmt::Debug for BearerTokenAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BearerTokenAuth")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

impl AuthProvider for BearerTokenAuth {
    async fn apply(&self, mut request: Request<Body>) -> Result<Request<Body>> {
        let value = self.token().await?;
        request.headers_mut().insert(self.header.clone(), value);
        Ok(request)
    }

    async fn refresh_after_unauthorized(&self, rejected_headers: &HeaderMap) -> Result<bool> {
        let mut current = self.current.lock().await;
        let rejected = rejected_headers.get(&self.header);
        let current_value = current
            .as_ref()
            .map(|token| self.header_value(token))
            .transpose()?;
        if current_value.is_some() && current_value.as_ref() != rejected {
            // Another request refreshed the token since this one was sent: use that one.
            return Ok(true);
        }
        let token = (self.fetch)().await?;
        *current = Some(token);
        drop(current);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use timvw_rust_runtime::Client as HttpClient;

    /// A server that accepts only `Bearer token-{valid}`.
    fn server(valid: Arc<AtomicUsize>, calls: Arc<AtomicUsize>) -> HttpClient {
        HttpClient::from_service(tower::service_fn(move |request: Request<Body>| {
            let valid = valid.clone();
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                // Let concurrent requests overlap.
                tokio::time::sleep(Duration::from_millis(20)).await;
                let expected = format!("Bearer token-{}", valid.load(Ordering::SeqCst));
                let status = if request
                    .headers()
                    .get(AUTHORIZATION)
                    .map(HeaderValue::as_bytes)
                    == Some(expected.as_bytes())
                {
                    StatusCode::OK
                } else {
                    StatusCode::UNAUTHORIZED
                };
                let mut response = http::Response::new(Body::from("{}"));
                *response.status_mut() = status;
                Ok::<_, BoxError>(response)
            }
        }))
    }

    fn counting_auth(fetches: Arc<AtomicUsize>) -> BearerTokenAuth {
        BearerTokenAuth::new(move || {
            let n = fetches.fetch_add(1, Ordering::SeqCst) + 1;
            async move { Ok(AuthToken::new(format!("token-{n}"))) }
        })
    }

    #[tokio::test]
    async fn refreshes_once_and_retries_once_after_a_401() {
        let valid = Arc::new(AtomicUsize::new(1));
        let calls = Arc::new(AtomicUsize::new(0));
        let fetches = Arc::new(AtomicUsize::new(0));
        let client = server(valid.clone(), calls.clone())
            .layer(AuthLayer::new(counting_auth(fetches.clone())));

        let response = client
            .request(http::Method::POST, "http://api.example/v1/x")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // The server rotates the token: one 401, one refresh, one retry.
        valid.store(2, Ordering::SeqCst);
        let response = client
            .request(http::Method::POST, "http://api.example/v1/x")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(fetches.load(Ordering::SeqCst), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        // A token the server never accepts: the retry's 401 is returned, no loop.
        valid.store(99, Ordering::SeqCst);
        let response = client
            .request(http::Method::GET, "http://api.example/v1/x")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(fetches.load(Ordering::SeqCst), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn concurrent_401s_refresh_once() {
        let valid = Arc::new(AtomicUsize::new(1));
        let calls = Arc::new(AtomicUsize::new(0));
        let fetches = Arc::new(AtomicUsize::new(0));
        let client = server(valid.clone(), calls.clone())
            .layer(AuthLayer::new(counting_auth(fetches.clone())));
        // Warm the cache with token-1, then rotate.
        client
            .request(http::Method::GET, "http://api.example/v1/x")
            .send()
            .await
            .unwrap();
        valid.store(2, Ordering::SeqCst);

        let requests = (0..8).map(|_| {
            client
                .request(http::Method::GET, "http://api.example/v1/x")
                .send()
        });
        for response in futures::future::join_all(requests).await {
            assert_eq!(response.unwrap().status(), StatusCode::OK);
        }
        // One fetch for the cache, one refresh for all eight 401s.
        assert_eq!(fetches.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn requests_outside_the_scope_are_not_authenticated() {
        let client =
            HttpClient::from_service(tower::service_fn(|request: Request<Body>| async move {
                let auth = request.headers().contains_key(AUTHORIZATION);
                Ok::<_, BoxError>(http::Response::new(Body::from(auth.to_string())))
            }))
            .layer(
                AuthLayer::new(counting_auth(Arc::new(AtomicUsize::new(0))))
                    .scope("https://api.example/v1")
                    .unwrap(),
            );
        let text = |url: &'static str| {
            let client = client.clone();
            async move {
                client
                    .request(http::Method::GET, url)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap()
            }
        };
        assert_eq!(text("https://api.example/v1/models").await, "true");
        assert_eq!(text("https://api.example/v10/models").await, "false");
        assert_eq!(text("https://cdn.example/image.png").await, "false");
    }

    #[tokio::test]
    async fn an_apply_error_fails_the_request() {
        struct Failing;
        impl AuthProvider for Failing {
            #[allow(unknown_lints, clippy::unused_async_trait_impl)]
            async fn apply(&self, _request: Request<Body>) -> Result<Request<Body>> {
                Err(Error::Authentication("no credentials".into()))
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let client =
            server(Arc::new(AtomicUsize::new(1)), calls.clone()).layer(AuthLayer::new(Failing));
        let error = client
            .request(http::Method::GET, "http://api.example/v1/x")
            .send()
            .await
            .unwrap_err();
        assert!(format!("{:?}", std::error::Error::source(&error)).contains("no credentials"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
