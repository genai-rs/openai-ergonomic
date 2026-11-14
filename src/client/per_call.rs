//! Calls that keep what the response headers say about them: the rate limits
//! (`x-ratelimit-*`) and, for a failure, how long to wait before a retry (`retry-after`). They use
//! the generated `<operation>_response` functions, which return the status and headers with the
//! body; the plain calls and the tool loop build on them.

// A failure keeps its headers' retry delay with the error.
#![allow(clippy::result_large_err)]

use std::sync::Arc;
use std::time::Duration;

use http::header::ACCEPT;
use http::{HeaderMap, Method};
use openai_client_base::apis::{chat_api, responses_api, Error as ApiError};
use openai_client_base::models::{CreateChatCompletionRequest, CreateResponse};

use super::{raw, Client};
use crate::rate_limits::{retry_after, RateLimitSnapshot};
use crate::responses::ChatCompletionResponseWrapper;
use crate::responses_api::{BoxedResponsesStream, ResponsesResponse, ResponsesStream};
use crate::semantic_conventions::operation_names;
use crate::streaming::InterceptedStream;
use crate::time::Instant;
use crate::Error;

/// What one response said about its call.
///
/// The status, the headers, the rate limits (`x-ratelimit-*`) and, for a failure, how long to
/// wait before a retry (`retry-after-ms`, `retry-after`). Empty when no response arrived (a
/// connection error).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallMeta {
    pub(crate) status: Option<u16>,
    pub(crate) headers: HeaderMap,
    pub(crate) rate_limits: Option<RateLimitSnapshot>,
    pub(crate) retry_after: Option<Duration>,
}

impl CallMeta {
    /// What these headers say, without a status.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            status: None,
            headers: headers.clone(),
            rate_limits: RateLimitSnapshot::from_headers(headers),
            retry_after: retry_after(headers),
        }
    }

    /// What a response with this status and these headers says.
    pub fn from_response(status: u16, headers: &HeaderMap) -> Self {
        Self {
            status: Some(status),
            ..Self::from_headers(headers)
        }
    }

    /// The status of the response; `None` when there was none.
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// The headers of the response; empty when there was none.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The rate limits the response reported (`x-ratelimit-*`).
    #[must_use]
    pub fn rate_limits(&self) -> Option<RateLimitSnapshot> {
        self.rate_limits
    }

    /// The delay the response asked for before a retry (`retry-after-ms`, else `retry-after` in
    /// seconds).
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

/// A failed call, with what its response said ([`CallMeta`]).
#[derive(Debug)]
pub struct CallFailure {
    pub(crate) error: Error,
    pub(crate) meta: CallMeta,
    /// The request failed before any of the body arrived: the connection, or an error status.
    /// The client's HTTP retries (when it has its own) already retried such a failure.
    pub(crate) before_body: bool,
}

impl CallFailure {
    /// A failure of the request itself, before any of the body: what the HTTP retries cover.
    fn before_body(error: Error, meta: CallMeta) -> Self {
        Self {
            error,
            meta,
            before_body: true,
        }
    }

    pub(crate) fn before_body_with(error: Error, meta: CallMeta) -> Self {
        Self::before_body(error, meta)
    }

    /// A failure while reading or decoding the body of a success.
    pub(crate) fn after_body(error: Error, meta: CallMeta) -> Self {
        Self {
            error,
            meta,
            before_body: false,
        }
    }

    /// The error.
    #[must_use]
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// The error, without what the response said.
    #[must_use]
    pub fn into_error(self) -> Error {
        self.error
    }

    /// What the response said: its status, headers, rate limits and the delay it asked for.
    #[must_use]
    pub fn meta(&self) -> &CallMeta {
        &self.meta
    }

    /// Whether the call failed before any of the body arrived: the connection failed, or the
    /// response had an error status. The client's own HTTP retries (`ConfigBuilder::max_retries`
    /// with the default HTTP client) already retried such a failure; one after the body started
    /// (a stream cut off midway) they did not.
    #[must_use]
    pub fn is_before_body(&self) -> bool {
        self.before_body
    }
}

impl std::fmt::Display for CallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for CallFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<Error> for CallFailure {
    fn from(error: Error) -> Self {
        Self {
            error,
            meta: CallMeta::default(),
            before_body: false,
        }
    }
}

impl From<CallFailure> for Error {
    fn from(failure: CallFailure) -> Self {
        failure.error
    }
}

/// The headers of an error response, if the error is one.
fn error_meta<E>(error: &ApiError<E>) -> CallMeta {
    match error {
        ApiError::ResponseError(response) => {
            CallMeta::from_response(response.status.as_u16(), &response.headers)
        }
        _ => CallMeta::default(),
    }
}

impl<T: Default + crate::MaybeSend + crate::MaybeSync + 'static> Client<T> {
    /// `POST /chat/completions` with `create_chat_completion_response`: the response carries its
    /// rate limits; a failure carries its headers' retry delay.
    pub(crate) async fn chat_call(
        &self,
        request: &CreateChatCompletionRequest,
    ) -> std::result::Result<ChatCompletionResponseWrapper, CallFailure> {
        let mut state = T::default();
        let operation = operation_names::CHAT;
        let model = request.model.clone();
        let request_json = serde_json::to_string(request).unwrap_or_default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();
        let result = chat_api::create_chat_completion_response()
            .configuration(&self.base_configuration)
            .create_chat_completion_request(request)
            .call()
            .await
            .and_then(|api| {
                let response = api.json().map_err(ApiError::Serde)?;
                Ok((response, RateLimitSnapshot::from_headers(&api.headers)))
            });
        let (response, rate_limits) = match result {
            Ok(decoded) => decoded,
            Err(error) => {
                let meta = error_meta(&error);
                let error = self
                    .handle_api_error(error, operation, &model, &request_json, &state)
                    .await;
                return Err(CallFailure::before_body(error, meta));
            }
        };
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            start_time.elapsed(),
            response.usage.as_ref().map(|u| u.prompt_tokens),
            response.usage.as_ref().map(|u| u.completion_tokens),
        )
        .await;
        Ok(ChatCompletionResponseWrapper::new(response).with_rate_limits(rate_limits))
    }

    /// `POST /responses` with `create_response_response`: the response carries its rate limits;
    /// a failure carries its headers' retry delay.
    pub(crate) async fn responses_call(
        &self,
        request: &CreateResponse,
    ) -> std::result::Result<ResponsesResponse, CallFailure> {
        let operation = operation_names::RESPONSES;
        let model = request.model.clone().unwrap_or_default();
        let request_json = serde_json::to_string(request).unwrap_or_default();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();
        let result = responses_api::create_response_response()
            .configuration(&self.base_configuration)
            .create_response(request)
            .call()
            .await
            .and_then(|api| {
                let response = api.json().map_err(ApiError::Serde)?;
                Ok(ResponsesResponse::new(response)
                    .with_rate_limits(RateLimitSnapshot::from_headers(&api.headers)))
            });
        let response = match result {
            Ok(response) => response,
            Err(error) => {
                let meta = error_meta(&error);
                let error = self
                    .handle_api_error(error, operation, &model, &request_json, &state)
                    .await;
                return Err(CallFailure::before_body(error, meta));
            }
        };
        let usage = response.usage();
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            start_time.elapsed(),
            usage.map(|usage| usage.input_tokens),
            usage.map(|usage| usage.output_tokens),
        )
        .await;
        Ok(response)
    }

    /// `POST /responses` with `stream: true`, sent as the generated `create_response_stream`
    /// sends it but keeping the response headers (the generated streaming operations do not
    /// return them): the events, with the interceptors' stream hooks, and what the headers said.
    pub(crate) async fn responses_stream_call(
        &self,
        mut request: CreateResponse,
    ) -> std::result::Result<(BoxedResponsesStream, CallMeta), CallFailure> {
        let operation = operation_names::RESPONSES;
        request.stream = Some(Some(true));
        let model = request.model.clone().unwrap_or_default();
        let request_json = serde_json::to_string(&request).unwrap_or_default();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let sent = self
            .raw_request(Method::POST, "/responses")
            .header(ACCEPT, "text/event-stream")
            .json(&request)
            .send()
            .await;
        let failure = match sent {
            Ok(response) if response.status().is_success() => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let (stream, _completion) = InterceptedStream::new_with_completion(
                    crate::time::idle_timeout(
                        ResponsesStream::new(response),
                        self.config.stream_idle_timeout(),
                    ),
                    Arc::clone(&self.interceptors),
                    operation.to_string(),
                    model,
                    request_json,
                    state,
                )?;
                let stream: BoxedResponsesStream = Box::pin(stream);
                return Ok((stream, meta));
            }
            Ok(response) => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let error = match raw::check_status(response).await {
                    Err(error) => error,
                    Ok(_) => Error::Internal("a failed response was a success".into()),
                };
                CallFailure::before_body(error, meta)
            }
            Err(error) => CallFailure::before_body(Error::Http(error), CallMeta::default()),
        };
        self.report_error(&failure.error, operation, &model, &request_json, &state)
            .await;
        Err(failure)
    }
}
