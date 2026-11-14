//! The agent loop: [`ToolLoop`] sends a [`Conversation`] to the model, runs the tools it calls,
//! sends their outputs back, and repeats until the model answers.
//!
//! On both APIs it:
//!
//! - repairs the history before every request ([`Conversation::normalize`]): every call
//!   answered, orphaned outputs dropped;
//! - answers every call ([`ToolRegistry::answer_call`]), runs the calls of tools that
//!   [allow it](crate::FunctionTool::supports_parallel) at the same time and the others alone,
//!   and records the answers in call order, cut to their [budget](crate::OutputBudget);
//! - stops after [`LoopConfig::max_iterations`] requests, or when a [`CancellationSignal`]
//!   fires, with a history that can be sent again;
//! - retries a failed request as a whole, after the delay the server asked for or with backoff
//!   ([`RetryConfig`], [`Error::retry_class`](crate::Error::retry_class));
//! - advertises [deferred](crate::tool_framework::Exposure::Deferred) tools once a tool search
//!   found them ([`ToolSearchConfig`], [`ModelCaps::tool_search`]);
//! - keeps the rate limits of every request ([`TurnRecord::rate_limits`]).
//!
//! On the Responses API it streams by default and runs each call as soon as the model finished
//! it (`response.output_item.done`), while the response goes on; its requests are stateless
//! (`store: false`, the encrypted reasoning replayed, one prompt cache key per loop).
//!
//! See the [agent loop guide](https://github.com/genai-rs/openai-ergonomic/blob/main/docs/agent_loop.md).

mod cancel;
mod config;
mod conversation;
mod dispatch;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use openai_client_base::models::{IncludeEnum, OutputItem, ResponseStreamEvent};
use serde_json::{json, Value};

pub use cancel::CancellationSignal;
pub use config::{LoopConfig, ModelCaps, PromptCacheKey, RetryConfig, ToolSearchConfig};
pub use conversation::{Conversation, NormalizeReport, IMAGE_OMITTED};

use crate::builders::Builder;
use crate::client::per_call::{CallFailure, CallMeta};
use crate::responses_api::{ResponsesEvent, ResponsesEventKind, ResponsesResponse, ResponsesTool};
use crate::tool_framework::{Advertise, Exposure, ToolKey};
use crate::{
    CallAnswer, ChatCompletionBuilder, Client, Error, MaybeSend, MaybeSync, RateLimitSnapshot,
    ResponsesBuilder, Result, ToolInvocation, ToolKind, ToolRegistry,
};
use dispatch::Dispatcher;

/// The name of the tool search tool the loop offers and answers itself.
pub const TOOL_SEARCH: &str = "tool_search";

/// Something that happened during a [`ToolLoop`] run, for
/// [`ToolLoop::run_with`]'s callback (to stream text to a UI, log, or show progress).
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum LoopEvent<'a> {
    /// A request is about to be sent: the `iteration`'s `attempt`, from 0.
    Request {
        /// The model request of the run, from 1.
        iteration: u32,
        /// The retry of this request, 0 for the first attempt.
        attempt: u32,
    },
    /// An event of a streamed Responses API response, such as a text delta
    /// ([`ResponsesEvent::text_delta`]).
    Model(&'a ResponsesEvent),
    /// The model called a tool; the call is queued and runs as soon as it may.
    ToolCalled(&'a ToolInvocation),
    /// A call was answered (successfully or not); its answer goes into the history after the
    /// response, in call order.
    ToolAnswered(&'a CallAnswer),
    /// The request failed and will be sent again after `delay`.
    Retrying {
        /// The retry, from 1.
        attempt: u32,
        /// The wait before it.
        delay: Duration,
        /// Why the request failed.
        error: &'a Error,
    },
    /// A model request and its tool calls are done.
    TurnDone(&'a TurnRecord),
}

/// One model request of a run and what came of it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TurnRecord {
    /// The model request of the run, from 1.
    pub iteration: u32,
    /// The id of the response (Responses API) or completion (Chat Completions).
    pub response_id: Option<String>,
    /// The text the model wrote.
    pub text: String,
    /// The tools it called, in order.
    pub calls: Vec<ToolInvocation>,
    /// The rate limits the API reported with the response
    /// (`x-ratelimit-{limit,remaining,reset}-{requests,tokens}`).
    pub rate_limits: Option<RateLimitSnapshot>,
    /// How many times the request was retried.
    pub retries: u32,
    /// The input tokens of the response.
    pub input_tokens: Option<i64>,
    /// The output tokens of the response.
    pub output_tokens: Option<i64>,
}

impl TurnRecord {
    fn new(iteration: u32) -> Self {
        Self {
            iteration,
            response_id: None,
            text: String::new(),
            calls: Vec::new(),
            rate_limits: None,
            retries: 0,
            input_tokens: None,
            output_tokens: None,
        }
    }
}

/// Why a run stopped.
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoopStop {
    /// The model answered without calling a tool.
    Completed,
    /// The model still called tools after [`LoopConfig::max_iterations`] requests. Their answers
    /// are in the history: run again to continue.
    MaxIterations,
    /// The [`CancellationSignal`] fired. Every call in the history has an answer (the calls that
    /// were running say `"aborted by user after …s"`): run again to continue.
    Cancelled,
    /// The response stopped early, such as at `max_output_tokens` (`length` on Chat Completions)
    /// or for its content filter.
    Incomplete {
        /// Why it stopped.
        reason: Option<String>,
    },
}

/// The result of a [`ToolLoop`] run. The history is in the [`Conversation`] it ran on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoopOutcome {
    /// Why the run stopped.
    pub stop: LoopStop,
    /// The model requests of the run, in order.
    pub turns: Vec<TurnRecord>,
}

impl LoopOutcome {
    /// The text of the last response.
    #[must_use]
    pub fn output_text(&self) -> &str {
        self.turns.last().map_or("", |turn| turn.text.as_str())
    }

    /// Whether the model answered ([`LoopStop::Completed`]).
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.stop == LoopStop::Completed
    }

    /// The rate limits of the last response that reported them.
    #[must_use]
    pub fn rate_limits(&self) -> Option<RateLimitSnapshot> {
        self.turns.iter().rev().find_map(|turn| turn.rate_limits)
    }
}

/// The request template of a loop: model and options, and messages sent before the
/// conversation.
#[derive(Debug, Clone)]
enum Template {
    Responses(Box<ResponsesBuilder>),
    Chat(Box<ChatCompletionBuilder>),
}

/// What a streamed attempt waits for.
#[allow(clippy::large_enum_variant)] // short-lived, one at a time
enum Step {
    Event(Option<Result<ResponsesEvent>>),
    Answered(std::result::Result<usize, crate::ToolError>),
    Cancelled,
}

/// How a request attempt ended.
enum AttemptEnd {
    /// A response, complete (`incomplete` is `None`) or stopped early.
    Done {
        incomplete: Option<String>,
        calls: usize,
    },
    Failed(CallFailure),
    Cancelled,
}

/// An agent loop over one API: the request template, the tools and the configuration. See the
/// [module documentation](self).
///
/// ```rust,no_run
/// use openai_ergonomic::{Client, Conversation, ToolLoop, ToolRegistry};
///
/// # async fn example(tools: ToolRegistry) -> openai_ergonomic::Result<()> {
/// let client = Client::from_env()?.build();
/// let template = client.responses().instructions("You are a household assistant.");
/// let mut agent = ToolLoop::responses(&client, tools, template);
/// let mut conversation = Conversation::user("Should I carry an umbrella in Brussels today?");
/// let outcome = agent.run(&mut conversation).await?;
/// println!("{}", outcome.output_text());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ToolLoop<'c, T = ()> {
    client: &'c Client<T>,
    registry: Arc<ToolRegistry>,
    template: Template,
    config: LoopConfig,
    loaded: BTreeSet<ToolKey>,
    cache_key: String,
}

impl<'c, T: Default + MaybeSend + MaybeSync + 'static> ToolLoop<'c, T> {
    /// A loop on the Responses API. `template` sets the model and the options of every request;
    /// its input (if any) is sent before the conversation, every time. The loop sets `input`,
    /// `tools` (adding to the template's), `store`, `include`, `prompt_cache_key` and
    /// `parallel_tool_calls`.
    pub fn responses(
        client: &'c Client<T>,
        registry: impl Into<Arc<ToolRegistry>>,
        template: ResponsesBuilder,
    ) -> Self {
        Self::new(
            client,
            registry.into(),
            Template::Responses(Box::new(template)),
        )
    }

    /// A loop on the Chat Completions API. `template` sets the model and the options of every
    /// request; its messages (such as a system message) are sent before the conversation, every
    /// time. The loop adds the tools and sets `prompt_cache_key` and `parallel_tool_calls`.
    pub fn chat(
        client: &'c Client<T>,
        registry: impl Into<Arc<ToolRegistry>>,
        template: ChatCompletionBuilder,
    ) -> Self {
        Self::new(client, registry.into(), Template::Chat(Box::new(template)))
    }

    fn new(client: &'c Client<T>, registry: Arc<ToolRegistry>, template: Template) -> Self {
        Self {
            client,
            registry,
            template,
            config: LoopConfig::default(),
            loaded: BTreeSet::new(),
            cache_key: format!("openai-ergonomic-{:016x}", random_u64()),
        }
    }

    /// The same loop with this configuration.
    #[must_use]
    pub fn with_config(mut self, config: LoopConfig) -> Self {
        self.config = config;
        self
    }

    /// The configuration.
    #[must_use]
    pub fn config(&self) -> &LoopConfig {
        &self.config
    }

    /// The configuration, to change it between runs.
    pub fn config_mut(&mut self) -> &mut LoopConfig {
        &mut self.config
    }

    /// The tools.
    #[must_use]
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// The prompt cache key the requests carry, if any ([`LoopConfig::prompt_cache_key`]).
    #[must_use]
    pub fn prompt_cache_key(&self) -> Option<&str> {
        match &self.config.prompt_cache_key {
            PromptCacheKey::PerLoop => Some(&self.cache_key),
            PromptCacheKey::Custom(key) => Some(key),
            PromptCacheKey::Omit => None,
        }
    }

    /// The deferred tools loaded so far: found by a tool search (the loop's own, or the
    /// model's when it [searches itself](ModelCaps::tool_search)), or loaded with
    /// [`load_tool`](Self::load_tool). They stay loaded across runs.
    pub fn loaded_tools(&self) -> impl Iterator<Item = &ToolKey> {
        self.loaded.iter()
    }

    /// Load a [deferred](Exposure::Deferred) tool: advertise it from the next request on, as if a
    /// tool search had found it. Returns whether the tool is registered and deferred. (A model
    /// that searches tools itself keeps getting deferred tools with `defer_loading`.)
    pub fn load_tool(&mut self, key: ToolKey) -> bool {
        let deferred = self.registry.exposure(&key) == Some(Exposure::Deferred);
        if deferred {
            self.loaded.insert(key);
        }
        deferred
    }

    /// Run until the model answers, [`LoopConfig::max_iterations`] requests were sent, or a
    /// response stops early. See [`run_with`](Self::run_with).
    pub async fn run(&mut self, conversation: &mut Conversation) -> Result<LoopOutcome> {
        self.run_with(conversation, &CancellationSignal::new(), |_| {})
            .await
    }

    /// Run on `conversation` until the model answers, [`LoopConfig::max_iterations`] requests
    /// were sent, a response stops early, or `cancel` fires; `on_event` sees what happens
    /// ([`LoopEvent`]).
    ///
    /// The model's output items and the answers to its calls are appended to `conversation`,
    /// which stays consistent whatever happens: after an error too, every call it holds has an
    /// answer or gets one ("aborted") at the next [`normalize`](Conversation::normalize).
    ///
    /// Fails with the error of a request that cannot be retried (or still fails after
    /// [`RetryConfig::max_retries`] retries), or with [`Error::Tool`] when a tool fails fatally
    /// ([`ToolFailure::Fatal`](crate::ToolFailure::Fatal)); the answers of the calls that
    /// finished before are in the history.
    pub async fn run_with<F>(
        &mut self,
        conversation: &mut Conversation,
        cancel: &CancellationSignal,
        mut on_event: F,
    ) -> Result<LoopOutcome>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        let registry = Arc::clone(&self.registry);
        let mut turns = Vec::new();
        let mut iteration = 0_u32;
        loop {
            if cancel.is_cancelled() {
                conversation.normalize(self.config.caps);
                return Ok(LoopOutcome {
                    stop: LoopStop::Cancelled,
                    turns,
                });
            }
            if self
                .config
                .max_iterations
                .is_some_and(|max| iteration >= max.get())
            {
                return Ok(LoopOutcome {
                    stop: LoopStop::MaxIterations,
                    turns,
                });
            }
            iteration += 1;
            let mut turn = TurnRecord::new(iteration);
            let mut dispatcher = Dispatcher::new(&registry);
            let end = self
                .turn(
                    conversation,
                    &mut dispatcher,
                    &mut turn,
                    cancel,
                    &mut on_event,
                )
                .await;
            let end = match end {
                Ok(end) => end,
                Err(error) => {
                    let answers = dispatcher.finished_answers();
                    self.record_answers(conversation, answers);
                    return Err(error);
                }
            };
            match end {
                AttemptEnd::Cancelled => {
                    let answers = dispatcher.abort();
                    self.record_answers(conversation, answers);
                    conversation.normalize(self.config.caps);
                    turns.push(turn);
                    return Ok(LoopOutcome {
                        stop: LoopStop::Cancelled,
                        turns,
                    });
                }
                AttemptEnd::Failed(failure) => return Err(failure.error),
                AttemptEnd::Done { incomplete, calls } => {
                    on_event(LoopEvent::TurnDone(&turn));
                    turns.push(turn);
                    match incomplete {
                        Some(_) => {
                            return Ok(LoopOutcome {
                                stop: LoopStop::Incomplete { reason: incomplete },
                                turns,
                            });
                        }
                        None if calls == 0 => {
                            return Ok(LoopOutcome {
                                stop: LoopStop::Completed,
                                turns,
                            });
                        }
                        None => {}
                    }
                }
            }
        }
    }

    /// One model request, with its retries, and its tool calls.
    async fn turn<F>(
        &mut self,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        cancel: &CancellationSignal,
        on_event: &mut F,
    ) -> Result<AttemptEnd>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        let mut attempt = 0_u32;
        loop {
            conversation.normalize(self.config.caps);
            on_event(LoopEvent::Request {
                iteration: turn.iteration,
                attempt,
            });
            let end = match &self.template {
                Template::Responses(_) if self.config.stream => {
                    self.stream_attempt(conversation, dispatcher, turn, cancel, on_event)
                        .await?
                }
                Template::Responses(_) => {
                    self.responses_attempt(conversation, dispatcher, turn, cancel, on_event)
                        .await?
                }
                Template::Chat(_) => {
                    self.chat_attempt(conversation, dispatcher, turn, cancel, on_event)
                        .await?
                }
            };
            if matches!(end, AttemptEnd::Cancelled) {
                return Ok(end);
            }
            // The calls of this attempt (a failed one included) are answered before anything
            // else: the next request carries them.
            if !self
                .drain(conversation, dispatcher, cancel, on_event)
                .await?
            {
                return Ok(AttemptEnd::Cancelled);
            }
            let AttemptEnd::Failed(failure) = end else {
                return Ok(end);
            };
            let class = failure.error.retry_class();
            if !class.is_retryable() || attempt >= self.config.retry.max_retries {
                return Ok(AttemptEnd::Failed(failure));
            }
            attempt += 1;
            turn.retries = attempt;
            let retry = self.config.retry;
            let delay = failure
                .meta
                .retry_after
                .or(match class {
                    crate::RetryClass::ServerDirected { after } => Some(after),
                    _ => None,
                })
                .map_or_else(
                    || retry.backoff(attempt, random_jitter()),
                    |after| after.min(retry.max_delay),
                );
            on_event(LoopEvent::Retrying {
                attempt,
                delay,
                error: &failure.error,
            });
            tracing::debug!(attempt, ?delay, error = %failure.error, "retrying the request");
            let slept = futures::future::select(
                Box::pin(crate::time::sleep(delay)),
                Box::pin(cancel.cancelled()),
            )
            .await;
            if matches!(slept, futures::future::Either::Right(_)) {
                return Ok(AttemptEnd::Cancelled);
            }
        }
    }

    /// Wait for the running calls and record their answers in call order. `false` when
    /// cancelled (the dispatcher keeps its calls, for [`Dispatcher::abort`]).
    async fn drain<F>(
        &self,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        cancel: &CancellationSignal,
        on_event: &mut F,
    ) -> Result<bool>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        while !dispatcher.is_idle() {
            let step = {
                let finished = dispatcher.finished();
                let cancelled = cancel.cancelled();
                futures::pin_mut!(finished, cancelled);
                match futures::future::select(finished, cancelled).await {
                    futures::future::Either::Left((finished, _)) => Some(finished),
                    futures::future::Either::Right(_) => None,
                }
            };
            match step {
                None => return Ok(false),
                Some(finished) => {
                    let index = finished.map_err(Error::from)?;
                    if let Some(answer) = dispatcher.answer(index) {
                        on_event(LoopEvent::ToolAnswered(answer));
                    }
                }
            }
        }
        let answers = dispatcher
            .take()
            .into_iter()
            .filter_map(|(_, answer)| answer)
            .collect();
        self.record_answers(conversation, answers);
        Ok(true)
    }

    /// Append answers to the history, cut to their tools' budgets (and as text on Chat
    /// Completions).
    fn record_answers(&self, conversation: &mut Conversation, answers: Vec<CallAnswer>) {
        if answers.is_empty() {
            return;
        }
        let truncation = self.config.truncation;
        let calls: std::collections::BTreeMap<String, ToolInvocation> = conversation
            .calls()
            .into_iter()
            .map(|call| (call.call_id.clone(), call))
            .collect();
        for answer in answers {
            let call = calls.get(&answer.call_id);
            let budget = call.and_then(|call| self.registry.output_budget(call));
            let mut answer = answer.truncated(truncation.budget_for(budget), truncation.header);
            if let (Template::Chat(_), Some(call)) = (&self.template, call) {
                if let Err(error) = answer.output.to_chat_text() {
                    answer = CallAnswer::failure(call, error.to_string());
                }
            }
            conversation.push_answer(&answer);
        }
    }

    /// Queue a call of the model (or answer it, for the loop's own tool search).
    fn dispatch<F>(
        &mut self,
        call: ToolInvocation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        on_event: &mut F,
    ) where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        on_event(LoopEvent::ToolCalled(&call));
        turn.calls.push(call.clone());
        if self.is_own_search(&call) {
            let answer = self.search(&call);
            on_event(LoopEvent::ToolAnswered(&answer));
            dispatcher.push_answered(call, answer);
            return;
        }
        let parallel = self.config.parallel_tool_calls && self.registry.supports_parallel(&call);
        dispatcher.push(call, parallel);
    }

    /// Record an output item of a response, and dispatch it when it is a call.
    fn take_item<F>(
        &mut self,
        item: &OutputItem,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        on_event: &mut F,
    ) -> usize
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        conversation.record_output_item(item);
        if let OutputItem::ToolSearchOutput(output) = item {
            for tool in &output.tools {
                self.record_loaded(&serde_json::to_value(tool).unwrap_or(Value::Null), None);
            }
        }
        let Some(call) = ToolInvocation::from_output_item(item) else {
            return 0;
        };
        self.dispatch(call, dispatcher, turn, on_event);
        1
    }

    /// Record the tools of a hosted tool search's output as loaded.
    fn record_loaded(&mut self, tool: &Value, namespace: Option<&str>) {
        let Some(name) = tool.get("name").and_then(Value::as_str) else {
            return;
        };
        if tool.get("type").and_then(Value::as_str) == Some("namespace") {
            for member in tool
                .get("tools")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                self.record_loaded(member, Some(name));
            }
            return;
        }
        let key = namespace.map_or_else(
            || ToolKey::new(name),
            |namespace| ToolKey::namespaced(namespace, name),
        );
        if self.registry.exposure(&key) == Some(Exposure::Deferred) {
            self.loaded.insert(key);
        }
    }

    /// A streamed Responses API attempt: calls start as soon as their item is done.
    async fn stream_attempt<F>(
        &mut self,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        cancel: &CancellationSignal,
        on_event: &mut F,
    ) -> Result<AttemptEnd>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        use futures::StreamExt;

        let request = self.responses_request(conversation)?;
        let client = self.client;
        let sent = {
            let call = client.responses_stream_call(request);
            let cancelled = cancel.cancelled();
            futures::pin_mut!(call, cancelled);
            match futures::future::select(call, cancelled).await {
                futures::future::Either::Left((sent, _)) => sent,
                futures::future::Either::Right(_) => return Ok(AttemptEnd::Cancelled),
            }
        };
        let (mut stream, meta) = match sent {
            Ok(sent) => sent,
            Err(failure) => return Ok(AttemptEnd::Failed(failure)),
        };
        turn.rate_limits = meta.rate_limits.or(turn.rate_limits);
        let mut seen = BTreeSet::new();
        let mut calls = 0;
        loop {
            let step = tokio::select! {
                biased;
                () = cancel.cancelled() => Step::Cancelled,
                finished = dispatcher.finished() => Step::Answered(finished),
                event = stream.next() => Step::Event(event),
            };
            let event = match step {
                Step::Cancelled => return Ok(AttemptEnd::Cancelled),
                Step::Answered(finished) => {
                    let index = finished.map_err(Error::from)?;
                    if let Some(answer) = dispatcher.answer(index) {
                        on_event(LoopEvent::ToolAnswered(answer));
                    }
                    continue;
                }
                Step::Event(None) => {
                    return Ok(AttemptEnd::Failed(CallFailure {
                        error: Error::StreamConnection {
                            message: "the stream ended before its response did".into(),
                        },
                        meta: CallMeta::default(),
                    }));
                }
                Step::Event(Some(Err(error))) => {
                    return Ok(AttemptEnd::Failed(CallFailure {
                        error,
                        meta: CallMeta::default(),
                    }));
                }
                Step::Event(Some(Ok(event))) => event,
            };
            on_event(LoopEvent::Model(&event));
            if let Some(error) = event.failure() {
                return Ok(AttemptEnd::Failed(CallFailure {
                    error,
                    meta: CallMeta::default(),
                }));
            }
            if let ResponseStreamEvent::ResponseOutputItemDoneEvent(done) = event.raw() {
                if seen.insert(done.output_index) {
                    calls += self.take_item(&done.item, conversation, dispatcher, turn, on_event);
                }
                continue;
            }
            let (response, incomplete) = match event.kind() {
                ResponsesEventKind::Completed(response) => (response, None),
                ResponsesEventKind::Incomplete { reason, response } => {
                    (response, Some(reason.unwrap_or_else(|| "unknown".into())))
                }
                _ => continue,
            };
            for (index, item) in (0_i64..).zip(&response.output) {
                if seen.insert(index) {
                    calls += self.take_item(item, conversation, dispatcher, turn, on_event);
                }
            }
            describe(turn, &ResponsesResponse::new(response.clone()));
            return Ok(AttemptEnd::Done { incomplete, calls });
        }
    }

    /// A Responses API attempt without streaming.
    async fn responses_attempt<F>(
        &mut self,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        cancel: &CancellationSignal,
        on_event: &mut F,
    ) -> Result<AttemptEnd>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        let request = self.responses_request(conversation)?;
        let client = self.client;
        let sent = {
            let call = client.responses_call(&request);
            let cancelled = cancel.cancelled();
            futures::pin_mut!(call, cancelled);
            match futures::future::select(call, cancelled).await {
                futures::future::Either::Left((sent, _)) => sent,
                futures::future::Either::Right(_) => return Ok(AttemptEnd::Cancelled),
            }
        };
        let response = match sent {
            Ok(response) => response,
            Err(failure) => return Ok(AttemptEnd::Failed(failure)),
        };
        turn.rate_limits = response.rate_limits().or(turn.rate_limits);
        if let Some(error) = response.failure() {
            return Ok(AttemptEnd::Failed(CallFailure {
                error,
                meta: CallMeta::default(),
            }));
        }
        let mut calls = 0;
        for item in response.output() {
            calls += self.take_item(item, conversation, dispatcher, turn, on_event);
        }
        describe(turn, &response);
        let incomplete = response.incomplete_reason().or_else(|| {
            (response.status() == Some(openai_client_base::models::response::Status::Incomplete))
                .then(|| "unknown".to_string())
        });
        Ok(AttemptEnd::Done { incomplete, calls })
    }

    /// A Chat Completions attempt.
    async fn chat_attempt<F>(
        &mut self,
        conversation: &mut Conversation,
        dispatcher: &mut Dispatcher<'_>,
        turn: &mut TurnRecord,
        cancel: &CancellationSignal,
        on_event: &mut F,
    ) -> Result<AttemptEnd>
    where
        F: FnMut(LoopEvent<'_>) + MaybeSend,
    {
        let request = self.chat_request(conversation)?;
        let client = self.client;
        let sent = {
            let call = client.chat_call(&request);
            let cancelled = cancel.cancelled();
            futures::pin_mut!(call, cancelled);
            match futures::future::select(call, cancelled).await {
                futures::future::Either::Left((sent, _)) => sent,
                futures::future::Either::Right(_) => return Ok(AttemptEnd::Cancelled),
            }
        };
        let response = match sent {
            Ok(response) => response,
            Err(failure) => return Ok(AttemptEnd::Failed(failure)),
        };
        turn.rate_limits = response.rate_limits().or(turn.rate_limits);
        let inner = response.inner();
        turn.response_id = Some(inner.id.clone());
        turn.input_tokens = inner.usage.as_ref().map(|usage| usage.prompt_tokens);
        turn.output_tokens = inner.usage.as_ref().map(|usage| usage.completion_tokens);
        let Some(choice) = inner.choices.first() else {
            return Ok(AttemptEnd::Failed(CallFailure::from(Error::Api {
                status: 200,
                message: "the chat completion has no choice".into(),
                error_type: None,
                error_code: None,
            })));
        };
        let message = serde_json::to_value(&choice.message)?;
        turn.text = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let calls = conversation.record_chat_message(&message);
        let count = calls.len();
        for call in calls {
            self.dispatch(call, dispatcher, turn, on_event);
        }
        let incomplete = match response.finish_reason().as_deref() {
            Some(reason @ ("length" | "content_filter")) => Some(reason.to_string()),
            _ => None,
        };
        Ok(AttemptEnd::Done {
            incomplete,
            calls: count,
        })
    }

    /// Whether the loop answers this call itself: its tool search, when no tool of that name is
    /// registered.
    fn is_own_search(&self, call: &ToolInvocation) -> bool {
        call.name == TOOL_SEARCH
            && call.namespace.is_none()
            && call.kind == ToolKind::Function
            && self.offers_own_search()
    }

    /// Whether the loop offers its own `tool_search` function.
    fn offers_own_search(&self) -> bool {
        self.config.tool_search.is_some()
            && !self.native_search()
            && self.registry.exposure(&ToolKey::new(TOOL_SEARCH)).is_none()
            && self.searchable().next().is_some()
    }

    /// Whether the model searches the deferred tools itself.
    fn native_search(&self) -> bool {
        self.config.caps.tool_search && matches!(self.template, Template::Responses(_))
    }

    /// The deferred tools a search can find on this API (chat has no namespaces).
    fn searchable(&self) -> impl Iterator<Item = &ToolKey> {
        let chat = matches!(self.template, Template::Chat(_));
        self.registry.keys().filter(move |key| {
            self.registry.exposure(key) == Some(Exposure::Deferred)
                && !(chat && key.namespace.is_some())
        })
    }

    /// Answer the loop's own tool search: load the deferred tools that match the query.
    fn search(&mut self, call: &ToolInvocation) -> CallAnswer {
        let Some(config) = &self.config.tool_search else {
            return CallAnswer::failure(call, "tool search is off");
        };
        let arguments: Value = serde_json::from_str(&call.input).unwrap_or(Value::Null);
        let Some(query) = arguments.get("query").and_then(Value::as_str) else {
            return CallAnswer::failure(
                call,
                "failed to parse function arguments: a `query` string is required",
            );
        };
        let limit = arguments
            .get("limit")
            .and_then(Value::as_u64)
            .and_then(|limit| usize::try_from(limit).ok())
            .map_or(config.limit, |limit| limit.clamp(1, config.limit.max(1)));
        let chat = matches!(self.template, Template::Chat(_));
        let found: Vec<ToolKey> = self
            .registry
            .search_deferred(query, usize::MAX)
            .into_iter()
            .filter(|key| !(chat && key.namespace.is_some()))
            .take(limit)
            .collect();
        let tools: Vec<Value> = found
            .iter()
            .map(|key| {
                let mut tool = json!({
                    "name": key.name,
                    "description": self.registry.description(key).unwrap_or_default(),
                });
                if let Some(namespace) = &key.namespace {
                    tool["namespace"] = Value::String(namespace.clone());
                }
                tool
            })
            .collect();
        self.loaded.extend(found);
        let message = if tools.is_empty() {
            "No tool matched. Try other words."
        } else {
            "These tools can be called from the next step on."
        };
        CallAnswer::new(
            call,
            json!({"tools": tools, "message": message}).to_string(),
            true,
        )
    }

    /// The loop's `tool_search` function: its name, description and schema.
    fn search_tool(&self) -> Option<(String, Value)> {
        let config = self.config.tool_search.as_ref()?;
        Some((
            config.description.clone(),
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "What the tools should do, in a few words"},
                    "limit": {"type": "integer", "minimum": 1, "description": "The most tools to load"},
                },
                "required": ["query"],
                "additionalProperties": false,
            }),
        ))
    }

    /// The Responses API request for `conversation`.
    fn responses_request(
        &self,
        conversation: &Conversation,
    ) -> Result<openai_client_base::models::CreateResponse> {
        let Template::Responses(template) = &self.template else {
            return Err(Error::Internal("not a Responses API loop".into()));
        };
        let config = &self.config;
        let input = if config.store {
            conversation.items().to_vec()
        } else {
            conversation.stateless_input()
        };
        let native = self.native_search();
        let mut tools = self
            .registry
            .responses_tools_where(|key, exposure| match exposure {
                Exposure::Direct => Some(Advertise::Plain),
                Exposure::Deferred if native => Some(Advertise::Deferred),
                Exposure::Deferred if self.loaded.contains(key) => Some(Advertise::Plain),
                _ => None,
            });
        if native && self.searchable().next().is_some() {
            tools.push(
                serde_json::from_value(json!({"type": "tool_search"}))
                    .unwrap_or_else(|_| ResponsesTool::Unknown(json!({"type": "tool_search"}))),
            );
        }
        if self.offers_own_search() {
            if let Some((description, schema)) = self.search_tool() {
                tools.push(crate::responses_api::responses_function_tool(
                    TOOL_SEARCH,
                    description,
                    schema,
                ));
            }
        }
        let mut builder = template.as_ref().clone().input_items(input);
        let has_tools = !tools.is_empty()
            || builder
                .params_mut()
                .tools
                .as_ref()
                .is_some_and(|tools| !tools.is_empty());
        if !tools.is_empty() {
            builder = builder.tools(tools);
        }
        if has_tools {
            builder = builder.parallel_tool_calls(config.parallel_tool_calls);
        }
        builder = builder.store(config.store);
        if let Some(key) = self.prompt_cache_key() {
            builder = builder.prompt_cache_key(key);
        }
        let mut request = builder.build()?;
        let reasoning = config.caps.reasoning || matches!(request.reasoning, Some(Some(_)));
        if !config.store && config.include_encrypted_reasoning && reasoning {
            let include = request
                .include
                .get_or_insert_with(|| Some(Vec::new()))
                .get_or_insert_with(Vec::new);
            if !include.contains(&IncludeEnum::ReasoningEncryptedContent) {
                include.push(IncludeEnum::ReasoningEncryptedContent);
            }
        }
        Ok(request)
    }

    /// The Chat Completions request for `conversation`.
    fn chat_request(
        &self,
        conversation: &Conversation,
    ) -> Result<openai_client_base::models::CreateChatCompletionRequest> {
        let Template::Chat(template) = &self.template else {
            return Err(Error::Internal("not a Chat Completions loop".into()));
        };
        let mut builder = template.as_ref().clone();
        for message in conversation.to_chat_messages() {
            builder = builder.push_message(message);
        }
        let mut tools = self
            .registry
            .chat_tools_where(|key, exposure| match exposure {
                Exposure::Direct => true,
                Exposure::Deferred => self.loaded.contains(key),
                _ => false,
            });
        if self.offers_own_search() {
            if let Some((description, schema)) = self.search_tool() {
                tools.push(crate::tool_function(TOOL_SEARCH, &description, schema));
            }
        }
        for tool in tools {
            builder = builder.add_tool(tool);
        }
        let mut request = builder.build()?;
        if request
            .tools
            .as_ref()
            .is_some_and(|tools| !tools.is_empty())
        {
            request.parallel_tool_calls = Some(self.config.parallel_tool_calls);
        }
        if let Some(key) = self.prompt_cache_key() {
            request.prompt_cache_key = Some(Some(key.to_string()));
        }
        Ok(request)
    }
}

/// Fill a turn's record from its response.
fn describe(turn: &mut TurnRecord, response: &ResponsesResponse) {
    turn.response_id = Some(response.id().to_string());
    turn.text = response.output_text();
    turn.input_tokens = response.usage().map(|usage| usage.input_tokens);
    turn.output_tokens = response.usage().map(|usage| usage.output_tokens);
}

/// A number that differs between calls and runs: for cache keys and jitter, not for security.
fn random_u64() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = crate::time::SystemTime::now()
        .duration_since(crate::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    #[allow(clippy::cast_possible_truncation)]
    let mut x = (nanos as u64) ^ COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    // splitmix64
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A jitter in `[-1, 1]`.
fn random_jitter() -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let unit = (random_u64() >> 11) as f64 / (1_u64 << 53) as f64;
    unit.mul_add(2.0, -1.0)
}
