//! Streaming support for chat completions.
//!
//! This module provides streaming functionality for `OpenAI` chat completions using
//! Server-Sent Events (SSE). The streaming API allows receiving responses incrementally
//! as they are generated, enabling real-time user experiences.
//!
//! # Example
//!
//! ```rust,no_run
//! use openai_ergonomic::Client;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?.build();
//!
//!     let builder = client.chat().user("Tell me a story");
//!     let mut stream = client.send_chat_stream(builder).await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         if let Some(content) = chunk.content() {
//!             print!("{}", content);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Interceptor Support
//!
//! Streaming responses automatically work with interceptors. When interceptors are
//! configured, they will receive hooks for:
//! - `before_request`: Called before streaming starts
//! - `on_stream_chunk`: Called for each chunk as it arrives
//! - `on_stream_terminal`: Called once on completion, failure, or cancellation
//!
//! Use `send_chat_stream_with_completion` or `send_responses_stream_with_completion` when
//! application shutdown needs to wait for all managed interceptor callbacks. Drop a stream after
//! stopping early, then await `StreamCompletion::wait` before flushing telemetry exporters.
//!
//! See the `langfuse_streaming` example for a complete demonstration.

use crate::interceptor::{StreamChunkContext, StreamEndContext, StreamTerminalContext};
use crate::{Error, Result};
use bytes::Bytes;
use futures::future::FutureExt;
use futures::stream::Stream;
use futures::StreamExt;
use openai_client_base::models::{
    ChatCompletionStreamResponseDelta, CreateChatCompletionStreamResponse,
};
use std::any::Any;
use std::collections::VecDeque;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, OwnedPermit};
use tokio::sync::watch;

const STREAM_CALLBACK_QUEUE_CAPACITY: usize = 1;
const MAX_RETAINED_CALLBACK_FAILURES: usize = 64;
const MAX_CALLBACK_DIAGNOSTIC_CHARS: usize = 512;

/// Classification of the terminal stream failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamFailureKind {
    /// The underlying response stream failed while receiving data.
    Connection,
    /// An SSE event could not be decoded.
    Parsing,
    /// The stream exceeded an internal buffer limit or otherwise failed buffering.
    Buffer,
    /// Any other stream error.
    Other,
}

/// Safe, owned classification of an error yielded by a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamFailure {
    /// The broad error category. Raw response data is not retained.
    pub kind: StreamFailureKind,
}

impl StreamFailure {
    fn from_error(error: &Error) -> Self {
        let kind = match error {
            Error::StreamConnection { .. } | Error::Http(_) => StreamFailureKind::Connection,
            Error::StreamParsing { .. } | Error::Json(_) => StreamFailureKind::Parsing,
            Error::StreamBuffer { .. } => StreamFailureKind::Buffer,
            _ => StreamFailureKind::Other,
        };
        Self { kind }
    }
}

/// The first terminal event observed for an intercepted stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamOutcome {
    /// The underlying parser reached normal EOF.
    Completed,
    /// The stream yielded an error item.
    StreamFailed(StreamFailure),
    /// The consumer dropped the stream before EOF or an error item.
    Cancelled,
}

/// The interceptor hook associated with a callback failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCallbackHook {
    /// A per-chunk callback.
    Chunk,
    /// The once-per-stream terminal callback.
    Terminal,
}

/// How an interceptor callback failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCallbackFailureKind {
    /// The callback returned an error.
    ReturnedError,
    /// The callback panicked with unwinding enabled.
    Panicked,
}

/// A bounded diagnostic for one callback failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCallbackFailure {
    /// The hook that failed.
    pub hook: StreamCallbackHook,
    /// Zero-based position of the interceptor in the registered chain.
    pub interceptor_index: usize,
    /// Zero-based chunk index for chunk hooks; `None` for the terminal hook.
    pub chunk_index: Option<usize>,
    /// Whether the callback returned an error or panicked.
    pub kind: StreamCallbackFailureKind,
    /// A short diagnostic. It never includes serialized request or chunk data.
    pub diagnostic: String,
    /// Whether the diagnostic was shortened to the configured character limit.
    pub diagnostic_truncated: bool,
}

/// Immutable summary sealed after all managed stream callbacks have finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCompletionReport {
    /// The latched terminal outcome.
    pub outcome: StreamOutcome,
    /// Number of successfully decoded chunks admitted to callback processing.
    pub total_chunks: usize,
    /// Time from intercepted stream construction to terminal processing.
    pub duration: Duration,
    /// Last observed input token count, if supplied by the stream.
    pub input_tokens: Option<i64>,
    /// Last observed output token count, if supplied by the stream.
    pub output_tokens: Option<i64>,
    /// Number of interceptor callbacks attempted.
    pub callback_attempts: usize,
    /// Number of interceptor callbacks that returned successfully.
    pub callback_successes: usize,
    /// Exact number of interceptor callbacks that failed.
    pub callback_failure_count: usize,
    /// Retained callback failure details, capped at 64 entries.
    pub callback_failures: Vec<StreamCallbackFailure>,
    /// Exact number of failure details omitted after the retention cap.
    pub truncated_failure_details: usize,
}

/// The dispatcher ended without sealing a completion report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("stream callback dispatcher ended before producing a completion report")]
pub struct CompletionUnavailable;

/// Cloneable wait handle for one intercepted stream's callback drain.
#[derive(Clone, Debug)]
pub struct StreamCompletion {
    report: watch::Receiver<Option<Arc<StreamCompletionReport>>>,
}

impl StreamCompletion {
    /// Wait for the terminal event and all accepted callback work to finish.
    ///
    /// Repeated and concurrent waits return the same immutable report. Dropping a waiter does
    /// not cancel the stream or the dispatcher.
    pub async fn wait(
        &self,
    ) -> std::result::Result<Arc<StreamCompletionReport>, CompletionUnavailable> {
        let mut report = self.report.clone();
        loop {
            let current = { report.borrow().clone() };
            if let Some(value) = current {
                return Ok(value);
            }
            if report.changed().await.is_err() {
                return Err(CompletionUnavailable);
            }
        }
    }
}

/// Type alias for a boxed stream of chat completion chunks.
///
/// Client methods use this type to preserve one public stream shape while managing ordered
/// interceptor callbacks and stream lifecycle reporting.
pub type BoxedChatStream = Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>;

/// A streaming chunk from a chat completion response.
///
/// Each chunk represents a delta update from the model as it generates the response.
#[derive(Debug, Clone)]
pub struct ChatCompletionChunk {
    /// The underlying stream response
    response: CreateChatCompletionStreamResponse,
}

impl ChatCompletionChunk {
    /// Create a new chunk from a stream response.
    #[must_use]
    pub fn new(response: CreateChatCompletionStreamResponse) -> Self {
        Self { response }
    }

    /// Get the content delta from this chunk, if any.
    ///
    /// Returns the text content that was generated in this chunk.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.content.as_ref().and_then(|c| c.as_deref()))
    }

    /// Get the role from this chunk, if any.
    ///
    /// This is typically only present in the first chunk.
    #[must_use]
    pub fn role(&self) -> Option<&str> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.role.as_ref())
            .map(|role| match role {
                openai_client_base::models::chat_completion_stream_response_delta::Role::System => {
                    "system"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::User => {
                    "user"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Assistant => {
                    "assistant"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Tool => {
                    "tool"
                }
                openai_client_base::models::chat_completion_stream_response_delta::Role::Developer => {
                    "developer"
                }
            })
    }

    /// Get tool calls from this chunk, if any.
    #[must_use]
    pub fn tool_calls(
        &self,
    ) -> Option<&Vec<openai_client_base::models::ChatCompletionMessageToolCallChunk>> {
        self.response
            .choices
            .first()
            .and_then(|choice| choice.delta.tool_calls.as_ref())
    }

    /// Get the finish reason, if any.
    ///
    /// This indicates why the generation stopped and is only present in the last chunk.
    #[must_use]
    pub fn finish_reason(&self) -> Option<&str> {
        self.response.choices.first().and_then(|choice| choice.finish_reason.as_ref()).and_then(Option::as_ref).map(|reason| {
            match reason {
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::Stop => "stop",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::Length => "length",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::ToolCalls => "tool_calls",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::ContentFilter => "content_filter",
                openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason::FunctionCall => "function_call",
            }
        })
    }

    /// Check if this is the last chunk in the stream.
    #[must_use]
    pub fn is_final(&self) -> bool {
        self.finish_reason().is_some()
    }

    /// Get the underlying raw response.
    #[must_use]
    pub fn raw_response(&self) -> &CreateChatCompletionStreamResponse {
        &self.response
    }

    /// Get the delta object directly.
    #[must_use]
    pub fn delta(&self) -> Option<&ChatCompletionStreamResponseDelta> {
        self.response
            .choices
            .first()
            .map(|choice| choice.delta.as_ref())
    }
}

/// A stream of chat completion chunks.
///
/// This stream yields `ChatCompletionChunk` items as the model generates the response.
/// The stream ends when the model finishes generating or encounters an error.
pub struct ChatCompletionStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
}

impl ChatCompletionStream {
    /// Create a new stream from a byte stream response.
    ///
    /// Parses Server-Sent Events (SSE) format and yields chat completion chunks.
    pub fn new(response: reqwest::Response) -> Self {
        let byte_stream = response.bytes_stream();
        let stream = parse_sse_stream(byte_stream);
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Collect all remaining content from the stream into a single string.
    ///
    /// This is a convenience method that reads all chunks and concatenates their content.
    pub async fn collect_content(mut self) -> Result<String> {
        let mut content = String::new();
        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            if let Some(text) = chunk.content() {
                content.push_str(text);
            }
        }
        Ok(content)
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

struct ChunkEvent {
    chunk_json: String,
    chunk_index: usize,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

type ReserveFuture = Pin<
    Box<
        dyn Future<
                Output = std::result::Result<OwnedPermit<ChunkEvent>, mpsc::error::SendError<()>>,
            > + Send,
    >,
>;

/// Wrapper for a stream that dispatches bounded, ordered interceptor callbacks.
///
/// The wrapper's direct constructor may be called outside a Tokio runtime, but it must be polled
/// inside one. Use [`Self::new_with_completion`] to validate the runtime and obtain a drain handle.
pub struct InterceptedStream<T: Send + Sync + 'static = ()> {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
    interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
    operation: String,
    model: String,
    request_json: String,
    state: Arc<T>,
    start_time: Instant,
    chunk_index: usize,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    started: bool,
    queue_tx: Option<mpsc::Sender<ChunkEvent>>,
    reserve_future: Option<ReserveFuture>,
    reserved: Option<OwnedPermit<ChunkEvent>>,
    terminal_tx: Option<watch::Sender<Option<StreamOutcome>>>,
    report_tx: Option<watch::Sender<Option<Arc<StreamCompletionReport>>>>,
    terminated: bool,
}

impl<T: Send + Sync + 'static> InterceptedStream<T> {
    /// Create an intercepted stream.
    ///
    /// Construction does not require a runtime. The first poll must occur inside an active
    /// Tokio runtime. Dropping the stream before its first poll starts no dispatcher.
    pub fn new(
        inner: ChatCompletionStream,
        interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
        operation: String,
        model: String,
        request_json: String,
        state: T,
    ) -> Self {
        Self {
            inner: Box::pin(inner),
            interceptors,
            operation,
            model,
            request_json,
            state: Arc::new(state),
            start_time: Instant::now(),
            chunk_index: 0,
            input_tokens: None,
            output_tokens: None,
            started: false,
            queue_tx: None,
            reserve_future: None,
            reserved: None,
            terminal_tx: None,
            report_tx: None,
            terminated: false,
        }
    }

    /// Create an intercepted stream and a handle that resolves after callback drain.
    ///
    /// An active Tokio runtime is required. This method returns an error instead of panicking
    /// when called outside one.
    pub fn new_with_completion(
        inner: ChatCompletionStream,
        interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
        operation: String,
        model: String,
        request_json: String,
        state: T,
    ) -> Result<(Self, StreamCompletion)> {
        let runtime = Handle::try_current().map_err(|_| {
            Error::Config(
                "InterceptedStream::new_with_completion requires an active Tokio runtime".into(),
            )
        })?;
        let mut stream = Self::new(inner, interceptors, operation, model, request_json, state);
        let completion = stream.start_lifecycle(&runtime);
        Ok((stream, completion))
    }

    fn start_lifecycle(&mut self, runtime: &Handle) -> StreamCompletion {
        let (report_tx, report_rx) = watch::channel(None);
        if self.interceptors.is_empty() {
            self.started = true;
            self.report_tx = Some(report_tx);
            return StreamCompletion { report: report_rx };
        }

        let (queue_tx, queue_rx) = mpsc::channel(STREAM_CALLBACK_QUEUE_CAPACITY);
        let (terminal_tx, terminal_rx) = watch::channel(None);

        let interceptors = Arc::clone(&self.interceptors);
        let operation = self.operation.clone();
        let model = self.model.clone();
        let request_json = self.request_json.clone();
        let state = Arc::clone(&self.state);
        let start_time = self.start_time;

        runtime.spawn(async move {
            dispatch_stream_callbacks(
                queue_rx,
                terminal_rx,
                report_tx,
                interceptors,
                operation,
                model,
                request_json,
                state,
                start_time,
            )
            .await;
        });

        self.queue_tx = Some(queue_tx);
        self.terminal_tx = Some(terminal_tx);
        self.started = true;
        StreamCompletion { report: report_rx }
    }

    fn finish(&mut self, outcome: StreamOutcome) {
        self.terminated = true;
        self.reserve_future.take();
        self.reserved.take();
        self.queue_tx.take();
        if let Some(report_tx) = self.report_tx.take() {
            let report = StreamCompletionReport {
                outcome,
                total_chunks: self.chunk_index,
                duration: self.start_time.elapsed(),
                input_tokens: self.input_tokens,
                output_tokens: self.output_tokens,
                callback_attempts: 0,
                callback_successes: 0,
                callback_failure_count: 0,
                callback_failures: Vec::new(),
                truncated_failure_details: 0,
            };
            let _ = report_tx.send(Some(Arc::new(report)));
        }
        if let Some(terminal_tx) = self.terminal_tx.take() {
            terminal_tx.send_if_modified(|current| {
                if current.is_none() {
                    *current = Some(outcome);
                    true
                } else {
                    false
                }
            });
        }
    }

    /// Collect all remaining content from the stream into a single string.
    ///
    /// The stream's callbacks remain ordered and the dispatcher is allowed to finish after this
    /// method returns. Use [`StreamCompletion::wait`] when application shutdown needs a drain barrier.
    pub async fn collect_content(mut self) -> Result<String> {
        let mut content = String::new();
        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            if let Some(text) = chunk.content() {
                content.push_str(text);
            }
        }
        Ok(content)
    }
}

impl<T: Send + Sync + 'static> Drop for InterceptedStream<T> {
    fn drop(&mut self) {
        if !self.terminated {
            self.finish(StreamOutcome::Cancelled);
        }
    }
}

impl<T: Send + Sync + 'static> Stream for InterceptedStream<T> {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminated {
            return Poll::Ready(None);
        }

        if !this.started {
            let Ok(runtime) = Handle::try_current() else {
                this.terminated = true;
                return Poll::Ready(Some(Err(Error::Config(
                    "InterceptedStream must be polled inside an active Tokio runtime".into(),
                ))));
            };
            this.start_lifecycle(&runtime);
        }

        if this.interceptors.is_empty() {
            return match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    update_usage(this, &chunk);
                    this.chunk_index = this.chunk_index.saturating_add(1);
                    Poll::Ready(Some(Ok(chunk)))
                }
                Poll::Ready(Some(Err(error))) => {
                    let outcome = StreamOutcome::StreamFailed(StreamFailure::from_error(&error));
                    this.finish(outcome);
                    Poll::Ready(Some(Err(error)))
                }
                Poll::Ready(None) => {
                    this.finish(StreamOutcome::Completed);
                    Poll::Ready(None)
                }
                Poll::Pending => Poll::Pending,
            };
        }

        if this.reserved.is_none() {
            if this.reserve_future.is_none() {
                let sender = this
                    .queue_tx
                    .as_ref()
                    .expect("started stream has queue sender")
                    .clone();
                this.reserve_future = Some(Box::pin(sender.reserve_owned()));
            }
            match this
                .reserve_future
                .as_mut()
                .expect("reserve future was just initialized")
                .as_mut()
                .poll(cx)
            {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(permit)) => {
                    this.reserve_future.take();
                    this.reserved = Some(permit);
                }
                Poll::Ready(Err(_)) => {
                    this.finish(StreamOutcome::StreamFailed(StreamFailure {
                        kind: StreamFailureKind::Other,
                    }));
                    return Poll::Ready(Some(Err(Error::Internal(
                        "stream callback dispatcher stopped unexpectedly".into(),
                    ))));
                }
            }
        }

        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                update_usage(this, &chunk);
                let event = ChunkEvent {
                    chunk_json: serde_json::to_string(chunk.raw_response())
                        .unwrap_or_else(|_| "{}".to_string()),
                    chunk_index: this.chunk_index,
                    input_tokens: this.input_tokens,
                    output_tokens: this.output_tokens,
                };
                this.reserved
                    .take()
                    .expect("polling the inner stream requires a reserved callback slot")
                    .send(event);
                this.chunk_index = this.chunk_index.saturating_add(1);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(error))) => {
                let outcome = StreamOutcome::StreamFailed(StreamFailure::from_error(&error));
                this.finish(outcome);
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                this.finish(StreamOutcome::Completed);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

fn update_usage<T: Send + Sync + 'static>(
    stream: &mut InterceptedStream<T>,
    chunk: &ChatCompletionChunk,
) {
    if let Some(Some(usage)) = &chunk.raw_response().usage {
        stream.input_tokens = Some(i64::from(usage.prompt_tokens));
        stream.output_tokens = Some(i64::from(usage.completion_tokens));
    }
}

async fn dispatch_stream_callbacks<T: Send + Sync + 'static>(
    mut queue: mpsc::Receiver<ChunkEvent>,
    mut terminal: watch::Receiver<Option<StreamOutcome>>,
    report_tx: watch::Sender<Option<Arc<StreamCompletionReport>>>,
    interceptors: Arc<crate::interceptor::InterceptorChain<T>>,
    operation: String,
    model: String,
    request_json: String,
    state: Arc<T>,
    start_time: Instant,
) {
    let mut report = StreamCompletionReport {
        outcome: StreamOutcome::Completed,
        total_chunks: 0,
        duration: Duration::ZERO,
        input_tokens: None,
        output_tokens: None,
        callback_attempts: 0,
        callback_successes: 0,
        callback_failure_count: 0,
        callback_failures: Vec::new(),
        truncated_failure_details: 0,
    };

    let outcome = loop {
        tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() {
                    return;
                }
                let current = *terminal.borrow();
                if let Some(outcome) = current {
                    break outcome;
                }
            }
            event = queue.recv() => if let Some(event) = event {
                    dispatch_chunk_event(
                        &mut report,
                        &interceptors,
                        &operation,
                        &model,
                        &request_json,
                        &state,
                        event,
                    ).await;
                } else {
                    // Closing the chunk sender can race with the separate terminal signal in
                    // this select. Wait for that signal instead of treating queue closure as
                    // an unavailable report by itself.
                    let current = *terminal.borrow();
                    if let Some(outcome) = current {
                        break outcome;
                    }
                    if terminal.changed().await.is_err() {
                        return;
                    }
                    let current = *terminal.borrow();
                    if let Some(outcome) = current {
                        break outcome;
                    }
                }
        }
    };

    // The producer closes the chunk queue before signaling terminal. Drain all admitted chunks
    // before sending the one terminal callback to every interceptor.
    while let Some(event) = queue.recv().await {
        dispatch_chunk_event(
            &mut report,
            &interceptors,
            &operation,
            &model,
            &request_json,
            &state,
            event,
        )
        .await;
    }

    report.outcome = outcome;
    report.duration = start_time.elapsed();
    dispatch_terminal_event(
        &mut report,
        &interceptors,
        &operation,
        &model,
        &request_json,
        &state,
        outcome,
    )
    .await;
    let _ = report_tx.send(Some(Arc::new(report)));
}

async fn dispatch_chunk_event<T: Send + Sync + 'static>(
    report: &mut StreamCompletionReport,
    interceptors: &crate::interceptor::InterceptorChain<T>,
    operation: &str,
    model: &str,
    request_json: &str,
    state: &T,
    event: ChunkEvent,
) {
    report.total_chunks = report.total_chunks.saturating_add(1);
    if let Some(input_tokens) = event.input_tokens {
        report.input_tokens = Some(input_tokens);
    }
    if let Some(output_tokens) = event.output_tokens {
        report.output_tokens = Some(output_tokens);
    }

    let ctx = StreamChunkContext {
        operation,
        model,
        request_json,
        chunk_json: &event.chunk_json,
        chunk_index: event.chunk_index,
        state,
    };
    for (interceptor_index, interceptor) in interceptors.iter().enumerate() {
        let callback = interceptor.on_stream_chunk(&ctx);
        record_callback(
            report,
            StreamCallbackHook::Chunk,
            interceptor_index,
            Some(event.chunk_index),
            callback,
        )
        .await;
    }
}

async fn dispatch_terminal_event<T: Send + Sync + 'static>(
    report: &mut StreamCompletionReport,
    interceptors: &crate::interceptor::InterceptorChain<T>,
    operation: &str,
    model: &str,
    request_json: &str,
    state: &T,
    outcome: StreamOutcome,
) {
    let summary = StreamEndContext {
        operation,
        model,
        request_json,
        total_chunks: report.total_chunks,
        duration: report.duration,
        input_tokens: report.input_tokens,
        output_tokens: report.output_tokens,
        state,
    };
    let ctx = StreamTerminalContext { summary, outcome };
    for (interceptor_index, interceptor) in interceptors.iter().enumerate() {
        let callback = interceptor.on_stream_terminal(&ctx);
        record_callback(
            report,
            StreamCallbackHook::Terminal,
            interceptor_index,
            None,
            callback,
        )
        .await;
    }
}

async fn record_callback<F>(
    report: &mut StreamCompletionReport,
    hook: StreamCallbackHook,
    interceptor_index: usize,
    chunk_index: Option<usize>,
    callback: F,
) where
    F: Future<Output = Result<()>>,
{
    report.callback_attempts = report.callback_attempts.saturating_add(1);
    match AssertUnwindSafe(callback).catch_unwind().await {
        Ok(Ok(())) => report.callback_successes = report.callback_successes.saturating_add(1),
        Ok(Err(error)) => {
            let diagnostic = error.to_string();
            record_callback_failure(
                report,
                hook,
                interceptor_index,
                chunk_index,
                StreamCallbackFailureKind::ReturnedError,
                &diagnostic,
            );
        }
        Err(payload) => {
            let diagnostic = panic_diagnostic(payload.as_ref());
            record_callback_failure(
                report,
                hook,
                interceptor_index,
                chunk_index,
                StreamCallbackFailureKind::Panicked,
                &diagnostic,
            );
        }
    }
}

fn record_callback_failure(
    report: &mut StreamCompletionReport,
    hook: StreamCallbackHook,
    interceptor_index: usize,
    chunk_index: Option<usize>,
    kind: StreamCallbackFailureKind,
    diagnostic: &str,
) {
    report.callback_failure_count = report.callback_failure_count.saturating_add(1);
    if report.callback_failures.len() == MAX_RETAINED_CALLBACK_FAILURES {
        report.truncated_failure_details = report.truncated_failure_details.saturating_add(1);
        return;
    }
    let (diagnostic, diagnostic_truncated) = truncate_diagnostic(diagnostic);
    report.callback_failures.push(StreamCallbackFailure {
        hook,
        interceptor_index,
        chunk_index,
        kind,
        diagnostic,
        diagnostic_truncated,
    });
}

fn truncate_diagnostic(value: &str) -> (String, bool) {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(MAX_CALLBACK_DIAGNOSTIC_CHARS).collect();
    if chars.next().is_some() {
        (truncated, true)
    } else {
        (truncated, false)
    }
}

fn panic_diagnostic(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&'static str>()
                .map(|value| (*value).to_string())
        })
        .unwrap_or_else(|| "callback panicked with a non-string payload".into())
}

/// Parse an SSE stream into chat completion chunks.
///
/// Events are dispatched only when a blank line completes the event. Any partial line or
/// event still buffered at end of stream is discarded, as required by the SSE format.
fn parse_sse_stream<S>(byte_stream: S) -> impl Stream<Item = Result<ChatCompletionChunk>> + Send
where
    S: Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
{
    struct State<S> {
        byte_stream: Pin<Box<S>>,
        buffer: Vec<u8>,
        swallow_lf: bool,
        data_lines: Vec<String>,
        ready: VecDeque<Result<ChatCompletionChunk>>,
        terminated: bool,
    }

    futures::stream::unfold(
        State {
            byte_stream: Box::pin(byte_stream),
            buffer: Vec::new(),
            swallow_lf: false,
            data_lines: Vec::new(),
            ready: VecDeque::new(),
            terminated: false,
        },
        |mut state| async move {
            loop {
                if let Some(item) = state.ready.pop_front() {
                    return Some((item, state));
                }
                if state.terminated {
                    return None;
                }

                match state.byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        state.buffer.extend_from_slice(&bytes);
                        while let Some(line) =
                            take_sse_line(&mut state.buffer, &mut state.swallow_lf)
                        {
                            if process_sse_line(&line, &mut state.data_lines, &mut state.ready) {
                                state.terminated = true;
                                break;
                            }
                        }
                    }
                    Some(Err(error)) => {
                        state.ready.push_back(Err(Error::StreamConnection {
                            message: format!("Stream connection error: {error}"),
                        }));
                        state.terminated = true;
                    }
                    None => {
                        // Other unterminated data remains buffered and, like an incomplete
                        // event, is discarded.
                        while let Some(line) =
                            take_sse_line(&mut state.buffer, &mut state.swallow_lf)
                        {
                            if process_sse_line(&line, &mut state.data_lines, &mut state.ready) {
                                state.terminated = true;
                                break;
                            }
                        }
                        state.buffer.clear();
                        state.data_lines.clear();
                        state.terminated = true;
                    }
                }
            }
        },
    )
    .fuse()
}

/// Remove one complete SSE line. A CR ends a line immediately; a following LF is swallowed.
fn take_sse_line(buffer: &mut Vec<u8>, swallow_lf: &mut bool) -> Option<Vec<u8>> {
    if *swallow_lf {
        if buffer.first() == Some(&b'\n') {
            buffer.drain(..1);
            *swallow_lf = false;
        } else if !buffer.is_empty() {
            *swallow_lf = false;
        }
        if buffer.is_empty() {
            return None;
        }
    }

    for index in 0..buffer.len() {
        match buffer[index] {
            b'\n' => {
                let line = buffer[..index].to_vec();
                buffer.drain(..=index);
                return Some(line);
            }
            b'\r' => {
                let line = buffer[..index].to_vec();
                buffer.drain(..=index);
                *swallow_lf = true;
                return Some(line);
            }
            _ => {}
        }
    }
    None
}

/// Process one decoded SSE line. Returns true when the stream reaches DONE or a parse error.
fn process_sse_line(
    line_bytes: &[u8],
    data_lines: &mut Vec<String>,
    ready: &mut VecDeque<Result<ChatCompletionChunk>>,
) -> bool {
    let line = String::from_utf8_lossy(line_bytes).into_owned();
    if line.is_empty() {
        if data_lines.is_empty() {
            return false;
        }

        let data = std::mem::take(data_lines).join("\n");
        if data.is_empty() {
            return false;
        }
        if data == "[DONE]" {
            return true;
        }

        match parse_sse_line(&format!("data: {data}")) {
            Ok(Some(chunk)) => ready.push_back(Ok(chunk)),
            Ok(None) => {}
            Err(error) => {
                ready.push_back(Err(error));
                return true;
            }
        }
        return false;
    }

    if line.starts_with(':') {
        return false;
    }
    if let Some(data) = line.strip_prefix("data:") {
        data_lines.push(data.strip_prefix(' ').unwrap_or(data).to_string());
    }
    false
}

/// Parse a single SSE line into a chat completion chunk.
fn parse_sse_line(line: &str) -> Result<Option<ChatCompletionChunk>> {
    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return Ok(None);
    }

    // Handle SSE format: "data: {json}"
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.strip_prefix(' ').unwrap_or(data);
        // Check for [DONE] marker
        if data == "[DONE]" {
            return Ok(None);
        }

        let response: CreateChatCompletionStreamResponse =
            serde_json::from_str(data).map_err(|e| Error::StreamParsing {
                message: format!("Failed to deserialize chunk: {e}"),
                chunk: data.to_string(),
            })?;

        return Ok(Some(ChatCompletionChunk::new(response)));
    }

    // Ignore other SSE fields (event:, id:, retry:)
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line_with_content() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}"#;

        let result = parse_sse_line(line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.content(), Some("Hello"));
        assert_eq!(chunk.role(), Some("assistant"));
        assert_eq!(chunk.finish_reason(), None);
        assert_eq!(chunk.raw_response().choices[0].finish_reason, Some(None));
        assert!(!chunk.is_final());
        assert!(!crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_parse_sse_line_with_missing_finish_reason() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{}}]}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), None);
        assert_eq!(chunk.raw_response().choices[0].finish_reason, None);
        assert!(!chunk.is_final());
    }

    #[test]
    fn test_parse_sse_line_with_finish_reason() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":null}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), Some("stop"));
        assert!(chunk.is_final());
        assert!(crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_finish_reason_helper_maps_all_supported_reasons() {
        for (wire, expected) in [
            ("stop", "stop"),
            ("length", "length"),
            ("tool_calls", "tool_calls"),
            ("content_filter", "content_filter"),
            ("function_call", "function_call"),
        ] {
            let event = serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "gpt-4",
                "choices": [{"index": 0, "delta": {}, "finish_reason": wire}],
            });
            let chunk = parse_sse_line(&format!("data: {event}")).unwrap().unwrap();
            assert_eq!(chunk.finish_reason(), Some(expected));
        }
    }

    #[test]
    fn test_final_audio_update_has_no_finish_reason() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1,"model":"gpt-4o-audio-preview","choices":[{"index":0,"delta":{"audio":{"expires_at":1700000000}}}]}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), None);
        assert_eq!(chunk.raw_response().choices[0].finish_reason, None);
        assert!(!chunk.is_final());
    }

    #[test]
    fn test_empty_choices_without_usage_are_not_finished() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1,"model":"gpt-4","choices":[]}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), None);
        assert!(!chunk.is_final());
    }

    #[test]
    fn test_usage_only_chunk_is_not_a_finished_choice() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7}}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.finish_reason(), None);
        assert!(!crate::responses::ChatCompletionStreamResponseWrapper::new(
            chunk.raw_response().clone()
        )
        .is_finished());
    }

    #[test]
    fn test_parse_sse_line_done_marker() {
        let line = "data: [DONE]";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_empty() {
        let line = "";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_comment() {
        let line = ": this is a comment";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_removes_only_one_space_after_colon() {
        let result = parse_sse_line("data:  {not-json}").unwrap_err();
        assert!(matches!(
            result,
            Error::StreamParsing { chunk, .. } if chunk == " {not-json}"
        ));
    }

    #[tokio::test]
    async fn test_parse_sse_stream_handles_cr_only_while_connection_is_open() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"cr-only\"},\"finish_reason\":null}]}\r\r",
        );
        let source = futures::stream::iter(vec![Ok::<Bytes, reqwest::Error>(bytes)])
            .chain(futures::stream::pending());
        let mut stream = Box::pin(parse_sse_stream(source));

        let chunk = tokio::time::timeout(std::time::Duration::from_millis(250), stream.next())
            .await
            .expect("CR-only event should dispatch without waiting for more bytes")
            .unwrap()
            .unwrap();
        assert_eq!(chunk.content(), Some("cr-only"));
    }

    #[tokio::test]
    async fn test_parse_sse_stream_handles_split_crlf_before_lf_blank_line() {
        let json = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"split"},"finish_reason":null}]}"#;
        let chunks = vec![
            Ok::<Bytes, reqwest::Error>(Bytes::from(format!("data: {json}\r"))),
            Ok(Bytes::from("\n")),
            Ok(Bytes::from("\n")),
        ];
        let source = futures::stream::iter(chunks).chain(futures::stream::pending());
        let mut stream = Box::pin(parse_sse_stream(source));

        let chunk = tokio::time::timeout(std::time::Duration::from_millis(250), stream.next())
            .await
            .expect("split CRLF followed by a blank LF line should dispatch promptly")
            .unwrap()
            .unwrap();
        assert_eq!(chunk.content(), Some("split"));
    }

    #[tokio::test]
    async fn test_parse_sse_stream_ignores_space_only_field_between_data_lines() {
        let bytes = Bytes::from(
            "data: {\n \ndata: \"id\":\"chatcmpl-123\",\ndata: \"object\":\"chat.completion.chunk\",\ndata: \"created\":1,\ndata: \"model\":\"gpt-4\",\ndata: \"choices\":[{\"index\":0,\"delta\":{\"content\":\"multiline\"},\"finish_reason\":null}]\ndata: }\n\n",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.content(), Some("multiline"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_handles_fragmented_crlf_and_usage_only_events() {
        let first_event = format!(
            "data: {{\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":{}}},\"finish_reason\":null}}]}}\r\n\r\n",
            serde_json::to_string("Hello 🦀").unwrap()
        );
        let crab = first_event
            .as_bytes()
            .windows(4)
            .position(|window| window == "🦀".as_bytes())
            .unwrap();
        let crlf = first_event
            .as_bytes()
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let first_bytes = first_event.as_bytes();
        let fragmented = vec![
            Ok(Bytes::copy_from_slice(&first_bytes[..5]),),
            Ok(Bytes::copy_from_slice(&first_bytes[5..crab + 2]),),
            Ok(Bytes::copy_from_slice(&first_bytes[(crab + 2)..=crlf]),),
            Ok(Bytes::copy_from_slice(&first_bytes[crlf + 1..crlf + 2]),),
            Ok(Bytes::copy_from_slice(&first_bytes[crlf + 2..]),),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" done\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\r\n\r\ndata: [DONE]\r\n\r\n",
            )),
        ];

        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(fragmented)));
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let usage = stream.next().await.unwrap().unwrap();

        assert_eq!(first.content(), Some("Hello 🦀"));
        assert_eq!(second.content(), Some(" done"));
        assert_eq!(usage.raw_response().choices.as_slice(), &[]);
        assert_eq!(
            usage
                .raw_response()
                .usage
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .total_tokens,
            7
        );
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_reports_malformed_json() {
        let bytes = Bytes::from("data: {not-json}\n\n");
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert!(matches!(
            stream.next().await.unwrap(),
            Err(Error::StreamParsing { .. })
        ));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_delivers_valid_event_before_parse_error() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"kept\"},\"finish_reason\":null}]}\n\ndata: {not-json}\n\n",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("kept")
        );
        assert!(matches!(
            stream.next().await.unwrap(),
            Err(Error::StreamParsing { .. })
        ));
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_stops_at_done() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"before\"},\"finish_reason\":null}]}\n\ndata: [DONE]\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"after\"},\"finish_reason\":null}]}\n\n",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("before")
        );
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_sse_stream_discards_incomplete_trailing_event() {
        let bytes = Bytes::from(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"complete\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"missing blank line\"},\"finish_reason\":null}]}\ndata: {partial",
        );
        let mut stream = Box::pin(parse_sse_stream(futures::stream::iter(vec![Ok(bytes)])));

        assert_eq!(
            stream.next().await.unwrap().unwrap().content(),
            Some("complete")
        );
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn completion_wait_reports_unavailable_if_worker_sender_closes() {
        let (sender, report) = watch::channel(None);
        let completion = StreamCompletion { report };
        drop(sender);
        assert_eq!(completion.wait().await, Err(CompletionUnavailable));
    }
}
