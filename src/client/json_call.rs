//! JSON calls that go the way of the ergonomic calls (the interceptors, the HTTP client's layers,
//! its retries and credentials) and keep what the response's status and headers say about them.
//!
//! [`Client::send_json`](Client::send_json) sends any JSON but skips the interceptors and drops
//! the headers. [`Client::json_call`] and [`Client::json_event_call`] are for a caller that builds
//! the request body itself (a library of its own on top of this client) but wants what an
//! ergonomic call gets: the interceptors see the request, the response or the stream, and the
//! failure; the result carries a [`CallMeta`] with the status, the headers, the rate limits and,
//! for a failure, the delay the server asked for.

// A failure keeps its headers with the error.
#![allow(clippy::result_large_err)]

use std::sync::Arc;

use futures::stream::{Stream, StreamExt};
use http::header::ACCEPT;
use http::Method;
use serde_json::Value;

use super::per_call::{CallFailure, CallMeta};
use super::{raw, Client};
use crate::streaming::{sealed, InterceptedItem, InterceptedStream};
use crate::time::Instant;
use crate::{Error, MaybeSend, MaybeSync, Result};

/// The success of a [`Client::json_call`]: the body, decoded as JSON, and what the response said
/// about the call.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JsonCallResponse {
    /// The body of the response.
    pub body: Value,
    /// The status, the headers and the rate limits of the response.
    pub meta: CallMeta,
}

/// One event of a [`Client::json_event_call`]: the `data:` of a server-sent event, decoded as
/// JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonEvent(pub Value);

impl sealed::Sealed for JsonEvent {}

impl InterceptedItem for JsonEvent {
    fn chunk_json(&self) -> String {
        self.0.to_string()
    }

    fn token_usage(&self) -> Option<(i64, i64)> {
        // A Responses API event carries the response, a chat chunk its usage.
        let usage = self
            .0
            .get("response")
            .and_then(|response| response.get("usage"))
            .or_else(|| self.0.get("usage"))?;
        token_usage(usage)
    }
}

/// The events of a [`Client::json_event_call`].
///
/// It ends at `data: [DONE]` or at the end of the body; an event that is not JSON ends it with
/// [`Error::StreamParsing`], a broken or idle connection with [`Error::StreamConnection`].
pub type JsonEventStream = crate::BoxStream<'static, Result<JsonEvent>>;

/// The input and output tokens of a usage object: `input_tokens`/`output_tokens` (Responses API)
/// or `prompt_tokens`/`completion_tokens` (Chat Completions).
fn token_usage(usage: &Value) -> Option<(i64, i64)> {
    let tokens = |names: [&str; 2]| {
        names
            .iter()
            .find_map(|name| usage.get(*name).and_then(Value::as_i64))
    };
    Some((
        tokens(["input_tokens", "prompt_tokens"])?,
        tokens(["output_tokens", "completion_tokens"])?,
    ))
}

impl<T: Default + MaybeSend + MaybeSync + 'static> Client<T> {
    /// `POST` a JSON body to `{api_base}{path}`, through the interceptors and the HTTP client's
    /// layers, and decode the JSON reply.
    ///
    /// `operation` is the name the interceptors see (for instance
    /// [`operation_names::RESPONSES`](crate::semantic_conventions::operation_names::RESPONSES) or
    /// [`operation_names::CHAT`](crate::semantic_conventions::operation_names::CHAT)); the model is
    /// the body's `model`. A reply with an error status is an [`Error::Api`] with the `type` and
    /// `code` of an `OpenAI` error body; the failure says whether it came before any of the body
    /// ([`CallFailure::is_before_body`]), which is what the client's HTTP retries already retried.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::semantic_conventions::operation_names;
    /// use openai_ergonomic::Client;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), openai_ergonomic::CallFailure> {
    /// let client = Client::from_env()?.build();
    /// let request = json!({"model": "gpt-4.1-mini", "input": "Say hi"});
    /// let response = client
    ///     .json_call(operation_names::RESPONSES, "/responses", &request)
    ///     .await?;
    /// println!("{} {:?}", response.body["id"], response.meta.rate_limits());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn json_call(
        &self,
        operation: &str,
        path: &str,
        body: &Value,
    ) -> std::result::Result<JsonCallResponse, CallFailure> {
        let model = model_of(body);
        let request_json = body.to_string();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();
        let sent = self
            .raw_request(Method::POST, path)
            .header(ACCEPT, "application/json")
            .json(body)
            .send()
            .await;
        let failure = match sent {
            Ok(response) if response.status().is_success() => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let decoded = match response.text().await {
                    Ok(text) => serde_json::from_str::<Value>(&text).map_err(Error::Json),
                    Err(error) => Err(Error::Http(error)),
                };
                match decoded {
                    Ok(body) => {
                        let tokens = body.get("usage").and_then(token_usage);
                        self.call_after_response(
                            &body,
                            operation,
                            &model,
                            &request_json,
                            &state,
                            start_time.elapsed(),
                            tokens.map(|(input, _)| input),
                            tokens.map(|(_, output)| output),
                        )
                        .await;
                        return Ok(JsonCallResponse { body, meta });
                    }
                    Err(error) => CallFailure::after_body(error, meta),
                }
            }
            Ok(response) => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let error = match raw::check_status(response).await {
                    Err(error) => error,
                    Ok(_) => Error::Internal("a failed response was a success".into()),
                };
                CallFailure::before_body_with(error, meta)
            }
            Err(error) => CallFailure::before_body_with(Error::Http(error), CallMeta::default()),
        };
        self.report_error(failure.error(), operation, &model, &request_json, &state)
            .await;
        Err(failure)
    }

    /// `POST` a JSON body to `{api_base}{path}` that answers with server-sent events, through the
    /// interceptors and the HTTP client's layers: the events, each `data:` decoded as JSON, and
    /// what the response's status and headers said.
    ///
    /// The interceptors' stream hooks see every event and the end of the stream. The stream
    /// ends with [`Error::StreamConnection`] when no event arrives for the configured
    /// [stream idle timeout](crate::ConfigBuilder::stream_idle_timeout). A failure before the
    /// stream starts is as for [`json_call`](Self::json_call). The caller sets `stream: true` in
    /// the body when the API asks for it.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// use openai_ergonomic::semantic_conventions::operation_names;
    /// use openai_ergonomic::Client;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?.build();
    /// let request = json!({"model": "gpt-4.1-mini", "input": "Say hi", "stream": true});
    /// let (mut events, meta) = client
    ///     .json_event_call(operation_names::RESPONSES, "/responses", &request)
    ///     .await?;
    /// println!("{:?}", meta.rate_limits());
    /// while let Some(event) = events.next().await {
    ///     let event = event?;
    ///     if event.0["type"] == "response.output_text.delta" {
    ///         print!("{}", event.0["delta"].as_str().unwrap_or_default());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn json_event_call(
        &self,
        operation: &str,
        path: &str,
        body: &Value,
    ) -> std::result::Result<(JsonEventStream, CallMeta), CallFailure> {
        let model = model_of(body);
        let request_json = body.to_string();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let sent = self
            .raw_request(Method::POST, path)
            .header(ACCEPT, "text/event-stream")
            .json(body)
            .send()
            .await;
        let failure = match sent {
            Ok(response) if response.status().is_success() => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let events = crate::time::idle_timeout(
                    json_events(response),
                    self.config.stream_idle_timeout(),
                );
                if self.interceptors.is_empty() {
                    return Ok((events, meta));
                }
                let stream = InterceptedStream::new(
                    events,
                    Arc::clone(&self.interceptors),
                    operation.to_string(),
                    model,
                    request_json,
                    state,
                );
                return Ok((crate::box_stream(stream), meta));
            }
            Ok(response) => {
                let meta = CallMeta::from_response(response.status().as_u16(), response.headers());
                let error = match raw::check_status(response).await {
                    Err(error) => error,
                    Ok(_) => Error::Internal("a failed response was a success".into()),
                };
                CallFailure::before_body_with(error, meta)
            }
            Err(error) => CallFailure::before_body_with(Error::Http(error), CallMeta::default()),
        };
        self.report_error(failure.error(), operation, &model, &request_json, &state)
            .await;
        Err(failure)
    }
}

/// The body's `model`, or nothing.
fn model_of(body: &Value) -> String {
    body.get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The `data:` payloads of a response's server-sent events as JSON, up to `data: [DONE]`.
fn json_events(
    response: timvw_rust_runtime::Response,
) -> impl Stream<Item = Result<JsonEvent>> + MaybeSend + 'static {
    response.events().filter_map(|event| async move {
        match event {
            Ok(event) if event.data.trim().is_empty() => None,
            Ok(event) => Some(
                serde_json::from_str::<Value>(&event.data)
                    .map(JsonEvent)
                    .map_err(|error| Error::StreamParsing {
                        message: format!("Failed to deserialize event: {error}"),
                        chunk: event.data,
                    }),
            ),
            Err(error) => Some(Err(Error::StreamConnection {
                message: format!("Stream connection error: {error}"),
            })),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_token_usage_of_both_apis() {
        let responses = JsonEvent(json!({
            "type": "response.completed",
            "response": {"usage": {"input_tokens": 3, "output_tokens": 4}}
        }));
        assert_eq!(responses.token_usage(), Some((3, 4)));
        let chat = JsonEvent(json!({"usage": {"prompt_tokens": 5, "completion_tokens": 6}}));
        assert_eq!(chat.token_usage(), Some((5, 6)));
        assert_eq!(JsonEvent(json!({"type": "x"})).token_usage(), None);
    }
}
