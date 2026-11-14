//! Streaming responses: the server-sent events of `POST /responses` with `stream: true`.

use crate::sse_frames::Frame;
use crate::tool_framework::ToolInvocation;
use crate::{Error, Result};
use futures::stream::{Stream, StreamExt};
use openai_client_base::models::{self, OutputItem, ResponseStreamEvent};
use std::pin::Pin;
use std::task::{Context, Poll};

/// One event of a streaming response, wrapping the generated [`ResponseStreamEvent`].
///
/// Every event the API sends is yielded, in order. [`event_type`](Self::event_type) is the
/// event's `type` (such as `"response.output_text.delta"`); an event of a type this crate does
/// not know, or whose payload does not match its generated model, is
/// [`ResponseStreamEvent::Unknown`] with the JSON as sent.
#[derive(Debug, Clone, PartialEq)]
pub struct ResponsesEvent {
    event_type: String,
    inner: ResponseStreamEvent,
}

impl ResponsesEvent {
    /// Decode an event from its JSON payload. Fails only when `data` is not a JSON object.
    pub fn from_json(data: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(data).map_err(|error| Error::StreamParsing {
                message: format!("Failed to deserialize event: {error}"),
                chunk: data.to_string(),
            })?;
        let Some(event_type) = value.get("type").and_then(serde_json::Value::as_str) else {
            return Err(Error::StreamParsing {
                message: "The event has no `type`".to_string(),
                chunk: data.to_string(),
            });
        };
        let event_type = event_type.to_string();
        let inner = serde_json::from_value(value.clone()).unwrap_or_else(|error| {
            tracing::debug!(%event_type, %error, "Responses stream event kept as Unknown");
            ResponseStreamEvent::Unknown(value)
        });
        Ok(Self { event_type, inner })
    }

    /// The event's `type`, such as `"response.output_text.delta"` or `"response.completed"`.
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// The text of a `response.output_text.delta` event.
    #[must_use]
    pub fn text_delta(&self) -> Option<&str> {
        match &self.inner {
            ResponseStreamEvent::ResponseTextDeltaEvent(event) => Some(&event.delta),
            _ => None,
        }
    }

    /// The arguments fragment of a `response.function_call_arguments.delta` event.
    #[must_use]
    pub fn function_call_arguments_delta(&self) -> Option<&str> {
        match &self.inner {
            ResponseStreamEvent::ResponseFunctionCallArgumentsDeltaEvent(event) => {
                Some(&event.delta)
            }
            _ => None,
        }
    }

    /// The finished item of a `response.output_item.done` event, such as a complete function
    /// call that can be run before the response ends.
    #[must_use]
    pub fn output_item_done(&self) -> Option<&OutputItem> {
        match &self.inner {
            ResponseStreamEvent::ResponseOutputItemDoneEvent(event) => Some(&event.item),
            _ => None,
        }
    }

    /// The response a lifecycle event carries: `response.created`, `.in_progress`, `.queued`,
    /// `.completed`, `.incomplete` or `.failed`.
    #[must_use]
    pub fn response(&self) -> Option<&models::Response> {
        match &self.inner {
            ResponseStreamEvent::ResponseCreatedEvent(event) => Some(&event.response),
            ResponseStreamEvent::ResponseInProgressEvent(event) => Some(&event.response),
            ResponseStreamEvent::ResponseQueuedEvent(event) => Some(&event.response),
            ResponseStreamEvent::ResponseCompletedEvent(event) => Some(&event.response),
            ResponseStreamEvent::ResponseIncompleteEvent(event) => Some(&event.response),
            ResponseStreamEvent::ResponseFailedEvent(event) => Some(&event.response),
            _ => None,
        }
    }

    /// The final response of a `response.completed` event.
    #[must_use]
    pub fn completed_response(&self) -> Option<&models::Response> {
        match &self.inner {
            ResponseStreamEvent::ResponseCompletedEvent(event) => Some(&event.response),
            _ => None,
        }
    }

    /// The event as a typed view, to `match` on the events a client acts on.
    ///
    /// ```
    /// use openai_ergonomic::responses_api::{ResponsesEvent, ResponsesEventKind};
    ///
    /// let event = ResponsesEvent::from_json(
    ///     r#"{"type":"response.output_text.delta","item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hi","sequence_number":1}"#,
    /// )?;
    /// match event.kind() {
    ///     ResponsesEventKind::TextDelta(text) => assert_eq!(text, "Hi"),
    ///     other => panic!("unexpected {other:?}"),
    /// }
    /// # Ok::<(), openai_ergonomic::Error>(())
    /// ```
    #[must_use]
    pub fn kind(&self) -> ResponsesEventKind<'_> {
        match &self.inner {
            ResponseStreamEvent::ResponseCreatedEvent(event) => {
                ResponsesEventKind::Created(&event.response)
            }
            ResponseStreamEvent::ResponseOutputItemAddedEvent(event) => {
                ResponsesEventKind::OutputItemAdded(&event.item)
            }
            ResponseStreamEvent::ResponseOutputItemDoneEvent(event) => {
                ResponsesEventKind::OutputItemDone(&event.item)
            }
            ResponseStreamEvent::ResponseTextDeltaEvent(event) => {
                ResponsesEventKind::TextDelta(&event.delta)
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDeltaEvent(event) => {
                ResponsesEventKind::FunctionCallArgumentsDelta {
                    item_id: &event.item_id,
                    delta: &event.delta,
                }
            }
            ResponseStreamEvent::ResponseCustomToolCallInputDeltaEvent(event) => {
                ResponsesEventKind::CustomToolCallInputDelta {
                    item_id: &event.item_id,
                    delta: &event.delta,
                }
            }
            ResponseStreamEvent::ResponseCompletedEvent(event) => {
                ResponsesEventKind::Completed(&event.response)
            }
            ResponseStreamEvent::ResponseIncompleteEvent(event) => ResponsesEventKind::Incomplete {
                reason: incomplete_reason(&event.response),
                response: &event.response,
            },
            ResponseStreamEvent::ResponseFailedEvent(event) => {
                ResponsesEventKind::Failed(&event.response)
            }
            ResponseStreamEvent::ResponseErrorEvent(event) => ResponsesEventKind::Error {
                code: event.code.as_deref(),
                message: &event.message,
            },
            other => ResponsesEventKind::Other(other),
        }
    }

    /// The call of a `response.output_item.done` event whose item is a function or custom tool
    /// call: complete, so it can be run before the response ends
    /// ([`ToolRegistry::answer_call`](crate::ToolRegistry::answer_call)).
    #[must_use]
    pub fn tool_invocation(&self) -> Option<ToolInvocation> {
        self.output_item_done()
            .and_then(ToolInvocation::from_output_item)
    }

    /// The error of a `response.failed` or `error` event ([`Error::ResponseFailed`]), whose
    /// [`retry_class`](Error::retry_class) says whether to try again. `None` for other events.
    #[must_use]
    pub fn failure(&self) -> Option<Error> {
        match &self.inner {
            ResponseStreamEvent::ResponseFailedEvent(event) => {
                Some(response_failure(&event.response))
            }
            ResponseStreamEvent::ResponseErrorEvent(event) => Some(Error::response_failed(
                event.code.clone(),
                event.message.clone(),
            )),
            _ => None,
        }
    }

    /// Whether this event ends the stream: `response.completed`, `response.incomplete`,
    /// `response.failed` or `error`.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        is_terminal(&self.event_type)
    }

    /// The generated event.
    #[must_use]
    pub fn raw(&self) -> &ResponseStreamEvent {
        &self.inner
    }

    /// The generated event, by value.
    #[must_use]
    pub fn into_raw(self) -> ResponseStreamEvent {
        self.inner
    }
}

/// A typed view of a [`ResponsesEvent`] ([`ResponsesEvent::kind`]), borrowing from it.
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResponsesEventKind<'a> {
    /// `response.created`: the response started.
    Created(&'a models::Response),
    /// `response.output_item.added`: an output item started (a message, a call, reasoning).
    OutputItemAdded(&'a OutputItem),
    /// `response.output_item.done`: an output item is complete; a function call can be run now
    /// ([`ResponsesEvent::tool_invocation`]).
    OutputItemDone(&'a OutputItem),
    /// `response.output_text.delta`: a piece of text.
    TextDelta(&'a str),
    /// `response.function_call_arguments.delta`: a piece of a function call's arguments.
    FunctionCallArgumentsDelta {
        /// The output item of the call.
        item_id: &'a str,
        /// The piece of the arguments.
        delta: &'a str,
    },
    /// `response.custom_tool_call_input.delta`: a piece of a custom tool call's input.
    CustomToolCallInputDelta {
        /// The output item of the call.
        item_id: &'a str,
        /// The piece of the input.
        delta: &'a str,
    },
    /// `response.completed`: the final response (terminal).
    Completed(&'a models::Response),
    /// `response.incomplete`: the response stopped early, such as at `max_output_tokens`
    /// (terminal).
    Incomplete {
        /// Why it stopped: `max_output_tokens`, `content_filter`, ...
        reason: Option<String>,
        /// The response so far.
        response: &'a models::Response,
    },
    /// `response.failed`: the response failed (terminal); see [`ResponsesEvent::failure`].
    Failed(&'a models::Response),
    /// `error`: the stream failed (terminal); see [`ResponsesEvent::failure`].
    Error {
        /// The error code, such as `rate_limit_exceeded`.
        code: Option<&'a str>,
        /// The error message.
        message: &'a str,
    },
    /// Any other event: [`ResponsesEvent::event_type`] says which.
    Other(&'a ResponseStreamEvent),
}

fn incomplete_reason(response: &models::Response) -> Option<String> {
    response
        .incomplete_details
        .as_ref()
        .and_then(|details| details.reason)
        .map(|reason| reason.to_string())
}

/// The error of a failed response.
pub fn response_failure(response: &models::Response) -> Error {
    response.error.as_ref().map_or_else(
        || Error::response_failed(None, "the response failed without an error"),
        |error| Error::response_failed(Some(error.code.to_string()), error.message.clone()),
    )
}

fn is_terminal(event_type: &str) -> bool {
    matches!(
        event_type,
        "response.completed" | "response.incomplete" | "response.failed" | "error"
    )
}

impl crate::streaming::sealed::Sealed for ResponsesEvent {}

impl crate::streaming::InterceptedItem for ResponsesEvent {
    fn chunk_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_else(|_| "{}".to_string())
    }

    fn token_usage(&self) -> Option<(i64, i64)> {
        self.response()
            .and_then(|response| response.usage.as_ref())
            .and_then(Option::as_ref)
            .map(|usage| (usage.input_tokens, usage.output_tokens))
    }
}

/// Type alias for a boxed stream of Responses API events, as the client returns it.
///
/// The stream is `Send` natively; on WebAssembly, where the host's streams cannot leave its
/// thread, it is not.
#[cfg(not(target_family = "wasm"))]
pub type BoxedResponsesStream = Pin<Box<dyn Stream<Item = Result<ResponsesEvent>> + Send>>;
/// Type alias for a boxed stream of Responses API events, as the client returns it.
///
/// The stream is `Send` natively; on WebAssembly, where the host's streams cannot leave its
/// thread, it is not.
#[cfg(target_family = "wasm")]
pub type BoxedResponsesStream = Pin<Box<dyn Stream<Item = Result<ResponsesEvent>>>>;

/// Message of the error that ends a stream cut off before its terminal event.
pub const RESPONSES_STREAM_CUT_OFF: &str =
    "the stream ended before response.completed, response.incomplete, response.failed or error";

/// The events of a streaming response.
///
/// - Every event is yielded; the stream ends after the terminal one (`response.completed`,
///   `response.incomplete`, `response.failed` or `error`), which is yielded too: a failed or
///   incomplete response is an event, not an error.
/// - A body that ends before a terminal event yields [`Error::StreamConnection`].
/// - A payload that is not a JSON object with a `type` is skipped (logged with
///   `tracing::debug!`).
/// - A failure reading the body yields [`Error::StreamConnection`] and ends the stream.
pub struct ResponsesStream {
    inner: crate::BoxStream<'static, Result<ResponsesEvent>>,
}

impl ResponsesStream {
    /// The events of a response's `text/event-stream` body. The status is not checked.
    pub fn new(response: timvw_rust_runtime::Response) -> Self {
        Self::from_frames(crate::sse_frames::frames(response.bytes_stream()))
    }

    /// The events of a response parsed elsewhere, such as those of the generated
    /// `create_response_stream` operation (`EventStream::into_events`).
    pub fn from_events(events: timvw_rust_runtime::sse::Events) -> Self {
        Self::from_frames(crate::sse_frames::from_events(events))
    }

    pub(crate) fn from_frames(
        frames: crate::BoxStream<'static, std::result::Result<Frame, timvw_rust_runtime::Error>>,
    ) -> Self {
        Self {
            inner: crate::box_stream(decode_frames(frames)),
        }
    }

    /// Collect the text deltas of the remaining events into one string.
    pub async fn collect_text(self) -> Result<String> {
        collect_text(self).await
    }
}

/// Concatenate the text deltas of a stream of events, such as the
/// [`BoxedResponsesStream`] of [`Client::send_responses_stream`](crate::Client::send_responses_stream).
/// The first error is returned.
pub async fn collect_text<S>(mut stream: S) -> Result<String>
where
    S: Stream<Item = Result<ResponsesEvent>> + Unpin,
{
    let mut text = String::new();
    while let Some(event) = stream.next().await {
        if let Some(delta) = event?.text_delta() {
            text.push_str(delta);
        }
    }
    Ok(text)
}

impl Stream for ResponsesStream {
    type Item = Result<ResponsesEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl std::fmt::Debug for ResponsesStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponsesStream").finish_non_exhaustive()
    }
}

fn decode_frames(
    frames: crate::BoxStream<'static, std::result::Result<Frame, timvw_rust_runtime::Error>>,
) -> impl Stream<Item = Result<ResponsesEvent>> + crate::MaybeSend {
    futures::stream::unfold(Some(frames), |state| async move {
        let mut frames = state?;
        loop {
            match frames.next().await {
                Some(Ok(Frame::Event(event))) if event.data.trim().is_empty() => {}
                Some(Ok(Frame::Event(event))) => match ResponsesEvent::from_json(&event.data) {
                    // Dropping the frames after the terminal event drops the HTTP body.
                    Ok(event) if event.is_terminal() => return Some((Ok(event), None)),
                    Ok(event) => return Some((Ok(event), Some(frames))),
                    Err(error) => {
                        tracing::debug!(%error, "malformed Responses stream event skipped");
                    }
                },
                Some(Ok(Frame::Done)) | None => {
                    return Some((
                        Err(Error::StreamConnection {
                            message: RESPONSES_STREAM_CUT_OFF.to_string(),
                        }),
                        None,
                    ));
                }
                Some(Err(error)) => {
                    return Some((
                        Err(Error::StreamConnection {
                            message: format!("Stream connection error: {error}"),
                        }),
                        None,
                    ));
                }
            }
        }
    })
    .fuse()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use bytes::Bytes;

    fn stream_of(body: &str) -> ResponsesStream {
        let chunks = vec![Ok::<_, timvw_rust_runtime::Error>(Bytes::from(
            body.to_string(),
        ))];
        ResponsesStream::from_frames(crate::sse_frames::frames(futures::stream::iter(chunks)))
    }

    /// A `response` object with the fields the API always sends.
    fn response(status: &str, extra: &str) -> String {
        format!(
            r#"{{"id":"resp_1","object":"response","created_at":1,"status":"{status}","model":"gpt-4.1","instructions":null,"error":null,"incomplete_details":null,"output":[],"parallel_tool_calls":true{extra}}}"#
        )
    }

    fn completed_event() -> String {
        format!(
            r#"{{"type":"response.completed","sequence_number":3,"response":{}}}"#,
            response(
                "completed",
                r#","usage":{"input_tokens":5,"input_tokens_details":{"cached_tokens":0},"output_tokens":2,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":7}"#
            )
        )
    }

    #[tokio::test]
    async fn yields_events_until_the_terminal_one() {
        let completed = completed_event();
        let body = format!(
            "event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hel\",\"sequence_number\":1}}\n\n\
             data: {{\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\",\"sequence_number\":2}}\n\n\
             data: {completed}\n\n\
             data: {{\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"ignored\",\"sequence_number\":4}}\n\n"
        );
        let events: Vec<_> = stream_of(&body).collect().await;
        assert_eq!(events.len(), 3);
        let last = events[2].as_ref().unwrap();
        assert!(last.is_terminal());
        assert_eq!(last.completed_response().unwrap().id, "resp_1");
        assert_eq!(
            crate::streaming::InterceptedItem::token_usage(last),
            Some((5, 2))
        );
        assert_eq!(stream_of(&body).collect_text().await.unwrap(), "Hello");
    }

    #[tokio::test]
    async fn a_cut_off_stream_ends_with_an_error() {
        let body = format!(
            "data: {{\"type\":\"response.created\",\"sequence_number\":0,\"response\":{}}}\n\n",
            response("in_progress", "")
        );
        let events: Vec<_> = stream_of(&body).collect().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].as_ref().unwrap().response().unwrap().id, "resp_1");
        assert!(matches!(
            &events[1],
            Err(Error::StreamConnection { message }) if message == RESPONSES_STREAM_CUT_OFF
        ));
    }

    #[tokio::test]
    async fn malformed_events_are_skipped_and_unknown_types_kept() {
        let completed = completed_event();
        let body = format!(
            "data: {{not json}}\n\ndata: {{\"type\":\"response.future_event\",\"x\":1}}\n\ndata: {completed}\n\n"
        );
        let events: Vec<_> = stream_of(&body).collect().await;
        assert_eq!(events.len(), 2);
        let unknown = events[0].as_ref().unwrap();
        assert_eq!(unknown.event_type(), "response.future_event");
        assert!(matches!(unknown.raw(), ResponseStreamEvent::Unknown(_)));
    }

    #[tokio::test]
    async fn failed_and_error_events_end_the_stream_as_events() {
        let failed = format!(
            r#"{{"type":"response.failed","sequence_number":1,"response":{}}}"#,
            response("failed", "").replace(
                r#""error":null"#,
                r#""error":{"code":"server_error","message":"boom"}"#
            )
        );
        let events: Vec<_> = stream_of(&format!("data: {failed}\n\n")).collect().await;
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        assert!(event.is_terminal());
        assert_eq!(
            event.response().unwrap().error.as_ref().unwrap().message,
            "boom"
        );

        let error = r#"{"type":"error","code":"rate_limit_exceeded","message":"slow down","param":null,"sequence_number":1}"#;
        let events: Vec<_> = stream_of(&format!("data: {error}\n\n")).collect().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap().event_type(), "error");
    }

    #[test]
    fn typed_kinds_and_failures() {
        let failed = format!(
            r#"{{"type":"response.failed","sequence_number":1,"response":{}}}"#,
            response("failed", "").replace(
                r#""error":null"#,
                r#""error":{"code":"server_error","message":"boom"}"#
            )
        );
        let event = ResponsesEvent::from_json(&failed).unwrap();
        assert!(matches!(event.kind(), ResponsesEventKind::Failed(_)));
        let error = event.failure().unwrap();
        assert!(
            matches!(&error, Error::ResponseFailed { code: Some(code), message } if code == "server_error" && message == "boom")
        );
        assert_eq!(error.retry_class(), crate::RetryClass::Backoff);

        let event = ResponsesEvent::from_json(
            r#"{"type":"error","code":"rate_limit_exceeded","message":"Please try again in 2s","param":null,"sequence_number":1}"#,
        )
        .unwrap();
        assert!(matches!(
            event.kind(),
            ResponsesEventKind::Error {
                code: Some("rate_limit_exceeded"),
                ..
            }
        ));
        assert_eq!(
            event.failure().unwrap().retry_class(),
            crate::RetryClass::ServerDirected {
                after: std::time::Duration::from_secs(2)
            }
        );

        let incomplete = format!(
            r#"{{"type":"response.incomplete","sequence_number":1,"response":{}}}"#,
            response("incomplete", "").replace(
                r#""incomplete_details":null"#,
                r#""incomplete_details":{"reason":"max_output_tokens"}"#
            )
        );
        let event = ResponsesEvent::from_json(&incomplete).unwrap();
        assert!(matches!(
            event.kind(),
            ResponsesEventKind::Incomplete { reason: Some(ref reason), .. } if reason == "max_output_tokens"
        ));
        assert!(event.failure().is_none());

        let event = ResponsesEvent::from_json(
            r#"{"type":"response.function_call_arguments.delta","item_id":"fc_1","output_index":0,"delta":"{\"a","sequence_number":1}"#,
        )
        .unwrap();
        assert!(matches!(
            event.kind(),
            ResponsesEventKind::FunctionCallArgumentsDelta {
                item_id: "fc_1",
                delta: "{\"a"
            }
        ));
        let event = ResponsesEvent::from_json(r#"{"type":"response.future","x":1}"#).unwrap();
        assert!(matches!(event.kind(), ResponsesEventKind::Other(_)));
    }

    #[tokio::test]
    async fn output_item_done_carries_a_function_call() {
        let completed = completed_event();
        let body = format!(
            "data: {{\"type\":\"response.output_item.done\",\"output_index\":0,\"sequence_number\":1,\"item\":{{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"{{}}\",\"status\":\"completed\"}}}}\n\ndata: {completed}\n\n"
        );
        let events: Vec<_> = stream_of(&body).collect().await;
        match events[0].as_ref().unwrap().output_item_done() {
            Some(OutputItem::FunctionToolCall(call)) => {
                assert_eq!(call.call_id, "call_1");
                assert_eq!(call.name, "get_weather");
            }
            other => panic!("expected a function call, got {other:?}"),
        }
        let call = events[0].as_ref().unwrap().tool_invocation().unwrap();
        assert_eq!(
            call,
            ToolInvocation::function("call_1", "get_weather", "{}")
        );
        assert!(events[1].as_ref().unwrap().tool_invocation().is_none());
    }
}
