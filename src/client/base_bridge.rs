//! Bridge between the client's HTTP stack (`timvw-rust-runtime` 0.3) and the generated
//! `openai-client-base` operations, which are still built on runtime 0.2.
//!
//! Transitional: once `openai-client-base` is regenerated against runtime 0.3, its
//! `Configuration.client` is the same [`HttpClient`] as ours and this module goes away
//! (`base_configuration.client = http_client.clone()`, and `ApiError::Http` maps to
//! [`Error::Http`] directly).
//!
//! A 0.2 request is converted to a 0.3 one (a body held in memory stays in memory, so the retry
//! layer can still send it again), sent through the 0.3 client and its layers, and the 0.3
//! response is handed back as a 0.2 one. A transport error of the 0.3 client is passed through
//! inside a [`BridgedError`], from which [`http_error`] recovers it.

use crate::Error;
use base_runtime::Body as BaseBody;
use bytes::Bytes;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Mutex;
use timvw_rust_runtime::{Body, BoxError, Client as HttpClient};

/// A runtime 0.2 client that sends every request through `client`.
pub(super) fn base_client(client: &HttpClient) -> base_runtime::Client {
    let client = client.clone();
    base_runtime::Client::from_service(tower::service_fn(
        move |request: http::Request<BaseBody>| {
            let client = client.clone();
            async move {
                let request = request.map(to_body);
                match client.execute(request).await {
                    Ok(response) => Ok(response.into_http().map(BaseBody::wrap)),
                    Err(error) => Err(BoxError::from(BridgedError::new(error))),
                }
            }
        },
    ))
}

/// A 0.2 body as a 0.3 one: bytes stay bytes (replayable), a stream is wrapped.
fn to_body(body: BaseBody) -> Body {
    if let Some(bytes) = body.as_bytes() {
        return Body::from(Bytes::copy_from_slice(bytes));
    }
    Body::wrap(body)
}

/// A runtime 0.3 error carried through runtime 0.2, which only holds it as a source.
struct BridgedError {
    message: String,
    error: Mutex<Option<timvw_rust_runtime::Error>>,
}

impl BridgedError {
    fn new(error: timvw_rust_runtime::Error) -> Self {
        Self {
            message: error.to_string(),
            error: Mutex::new(Some(error)),
        }
    }
}

impl fmt::Debug for BridgedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Display for BridgedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for BridgedError {}

/// The crate error for a runtime 0.2 error of a generated operation: [`Error::Http`] with the
/// runtime 0.3 error of the client when there is one, or else [`Error::StreamConnection`] for a
/// failure reading the body and [`Error::InvalidRequest`] for a request that could not be built.
pub(super) fn http_error(error: &base_runtime::Error) -> Error {
    let mut source: Option<&(dyn StdError + 'static)> = error.source();
    while let Some(current) = source {
        if let Some(bridged) = current.downcast_ref::<BridgedError>() {
            let taken = bridged
                .error
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            if let Some(error) = taken {
                return Error::Http(error);
            }
        }
        source = current.source();
    }
    if error.is_builder() {
        Error::InvalidRequest(error.to_string())
    } else {
        Error::StreamConnection {
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[tokio::test]
    async fn requests_and_errors_cross_the_bridge() {
        let client = HttpClient::from_service(tower::service_fn(
            |request: http::Request<Body>| async move {
                // A body held in memory stays replayable.
                assert!(request.body().try_clone().is_some());
                if request.uri().path() == "/fail" {
                    return Err(BoxError::from("connection refused"));
                }
                Ok::<_, BoxError>(http::Response::new(Body::from("pong")))
            },
        ));
        let base = base_client(&client);
        let response = base
            .request(http::Method::POST, "http://h.example/ok")
            .body("ping")
            .send()
            .await
            .unwrap();
        assert_eq!(response.text().await.unwrap(), "pong");

        let error = base
            .request(http::Method::GET, "http://h.example/fail")
            .send()
            .await
            .unwrap_err();
        match http_error(&error) {
            Error::Http(error) => {
                assert!(error.is_request());
                assert!(
                    error.to_string().contains("connection refused") || {
                        let source = StdError::source(&error).map(ToString::to_string);
                        source.unwrap_or_default().contains("connection refused")
                    }
                );
            }
            other => panic!("expected an HTTP error, got {other:?}"),
        }
    }
}
