//! The rate limits the API reports in its response headers
//! (`x-ratelimit-{limit,remaining,reset}-{requests,tokens}`).
//!
//! The client records the limits of every response it receives;
//! [`Client::rate_limits`](crate::Client::rate_limits) returns the latest, streaming responses
//! included (their headers arrive before the first event).

use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};
use std::time::Duration;

use http::{HeaderMap, Request};
use timvw_rust_runtime::{Body, BoxError};
use tower::{Layer, Service};

use crate::{BoxFuture, MaybeSend};

/// The limits of one kind (requests or tokens).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RateLimitWindow {
    /// The maximum allowed in the window (`x-ratelimit-limit-*`).
    pub limit: Option<u64>,
    /// What is left in the window (`x-ratelimit-remaining-*`).
    pub remaining: Option<u64>,
    /// When the window resets, from the response (`x-ratelimit-reset-*`, such as `6m0s`).
    pub reset: Option<Duration>,
}

/// The rate limits of a response: requests and tokens.
///
/// ```
/// use openai_ergonomic::RateLimitSnapshot;
/// use std::time::Duration;
///
/// let mut headers = http::HeaderMap::new();
/// headers.insert("x-ratelimit-limit-requests", "500".parse()?);
/// headers.insert("x-ratelimit-remaining-requests", "499".parse()?);
/// headers.insert("x-ratelimit-reset-requests", "120ms".parse()?);
/// headers.insert("x-ratelimit-remaining-tokens", "29000".parse()?);
/// let limits = RateLimitSnapshot::from_headers(&headers).unwrap();
/// let requests = limits.requests.unwrap();
/// assert_eq!(requests.remaining, Some(499));
/// assert_eq!(requests.reset, Some(Duration::from_millis(120)));
/// assert_eq!(limits.tokens.unwrap().remaining, Some(29_000));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RateLimitSnapshot {
    /// The request limits.
    pub requests: Option<RateLimitWindow>,
    /// The token limits.
    pub tokens: Option<RateLimitWindow>,
}

impl RateLimitSnapshot {
    /// The limits in `headers`; `None` when there are none.
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let snapshot = Self {
            requests: window(headers, "requests"),
            tokens: window(headers, "tokens"),
        };
        (snapshot != Self::default()).then_some(snapshot)
    }
}

fn window(headers: &HeaderMap, kind: &str) -> Option<RateLimitWindow> {
    let header = |name: &str| {
        headers
            .get(format!("x-ratelimit-{name}-{kind}"))
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
    };
    let window = RateLimitWindow {
        limit: header("limit").and_then(|value| value.parse().ok()),
        remaining: header("remaining").and_then(|value| value.parse().ok()),
        reset: header("reset").and_then(crate::errors::parse_duration),
    };
    (window != RateLimitWindow::default()).then_some(window)
}

/// The latest rate limits a client saw, shared by its clones.
#[derive(Debug, Clone, Default)]
pub(crate) struct LatestRateLimits(Arc<Mutex<Option<RateLimitSnapshot>>>);

impl LatestRateLimits {
    pub(crate) fn get(&self) -> Option<RateLimitSnapshot> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn record(&self, headers: &HeaderMap) {
        if let Some(snapshot) = RateLimitSnapshot::from_headers(headers) {
            *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(snapshot);
        }
    }

    pub(crate) fn layer(&self) -> RateLimitLayer {
        RateLimitLayer(self.clone())
    }
}

/// Records the rate limits of every response.
#[derive(Debug, Clone)]
pub(crate) struct RateLimitLayer(LatestRateLimits);

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            latest: self.0.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RateLimitService<S> {
    inner: S,
    latest: LatestRateLimits,
}

impl<S, B> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = http::Response<B>> + MaybeSend + 'static,
    S::Error: Into<BoxError>,
    S::Future: MaybeSend + 'static,
    B: MaybeSend + 'static,
{
    type Response = http::Response<B>;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<http::Response<B>, BoxError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let future = self.inner.call(request);
        let latest = self.latest.clone();
        Box::pin(async move {
            let response = future.await.map_err(Into::into)?;
            latest.record(response.headers());
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn no_headers_no_snapshot() {
        assert_eq!(RateLimitSnapshot::from_headers(&HeaderMap::new()), None);
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit-tokens", "abc".parse().unwrap());
        assert_eq!(RateLimitSnapshot::from_headers(&headers), None);
        headers.insert("x-ratelimit-reset-tokens", "1m30s".parse().unwrap());
        let snapshot = RateLimitSnapshot::from_headers(&headers).unwrap();
        assert_eq!(snapshot.requests, None);
        assert_eq!(
            snapshot.tokens.unwrap().reset,
            Some(Duration::from_secs(90))
        );
    }
}
