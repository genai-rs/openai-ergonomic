//! Deterministic tests for the streaming interceptor completion contract.

use futures::StreamExt;
use openai_ergonomic::interceptor::{
    ErrorContext, InterceptorChain, StreamChunkContext, StreamEndContext, StreamTerminalContext,
};
use openai_ergonomic::{
    ChatCompletionStream, Error, InterceptedStream, Interceptor, Result, StreamCallbackFailureKind,
    StreamCallbackHook, StreamCompletion, StreamOutcome,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Semaphore};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    hook: StreamCallbackHook,
    interceptor: usize,
    chunk_index: Option<usize>,
    outcome: Option<StreamOutcome>,
}

#[derive(Clone, Copy)]
enum Behavior {
    Pass,
    ReturnError,
    Panic,
}

struct Probe {
    index: usize,
    events: mpsc::UnboundedSender<Call>,
    chunk_behavior: Behavior,
    terminal_behavior: Behavior,
    blocked_chunk: Option<(usize, Arc<Semaphore>)>,
    terminal_gate: Option<Arc<Semaphore>>,
}

impl Probe {
    fn new(index: usize, events: mpsc::UnboundedSender<Call>) -> Self {
        Self {
            index,
            events,
            chunk_behavior: Behavior::Pass,
            terminal_behavior: Behavior::Pass,
            blocked_chunk: None,
            terminal_gate: None,
        }
    }

    async fn record(
        &self,
        hook: StreamCallbackHook,
        chunk_index: Option<usize>,
        outcome: Option<StreamOutcome>,
        behavior: Behavior,
        gate: Option<&Arc<Semaphore>>,
    ) -> Result<()> {
        self.events
            .send(Call {
                hook,
                interceptor: self.index,
                chunk_index,
                outcome,
            })
            .unwrap();
        if let Some(gate) = gate {
            gate.acquire().await.unwrap().forget();
        }
        match behavior {
            Behavior::Pass => Ok(()),
            Behavior::ReturnError => Err(Error::Internal("mock callback failure".into())),
            Behavior::Panic => panic!("mock callback panic"),
        }
    }
}

#[async_trait::async_trait]
impl Interceptor for Probe {
    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        let gate = self
            .blocked_chunk
            .as_ref()
            .filter(|(index, _)| *index == ctx.chunk_index)
            .map(|(_, gate)| gate);
        self.record(
            StreamCallbackHook::Chunk,
            Some(ctx.chunk_index),
            None,
            self.chunk_behavior,
            gate,
        )
        .await
    }

    async fn on_stream_terminal(&self, ctx: &StreamTerminalContext<'_>) -> Result<()> {
        self.record(
            StreamCallbackHook::Terminal,
            None,
            Some(ctx.outcome),
            self.terminal_behavior,
            self.terminal_gate.as_ref(),
        )
        .await
    }

    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        self.events
            .send(Call {
                hook: StreamCallbackHook::Terminal,
                interceptor: self.index,
                chunk_index: None,
                outcome: None,
            })
            .unwrap();
    }
}

struct LegacyEndProbe(mpsc::UnboundedSender<Call>);

#[async_trait::async_trait]
impl Interceptor for LegacyEndProbe {
    async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        self.0
            .send(Call {
                hook: StreamCallbackHook::Terminal,
                interceptor: 0,
                chunk_index: Some(ctx.total_chunks),
                outcome: Some(StreamOutcome::Completed),
            })
            .unwrap();
        Ok(())
    }
}

type TestStream = (
    InterceptedStream,
    StreamCompletion,
    mpsc::UnboundedReceiver<Call>,
);

async fn intercepted(
    body: String,
    build_probes: impl FnOnce(mpsc::UnboundedSender<Call>) -> Vec<Box<dyn Interceptor>>,
) -> TestStream {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/stream")
        .with_header("content-type", "text/event-stream")
        .with_body(body)
        .create_async()
        .await;
    let response = reqwest::Client::new()
        .get(format!("{}/stream", server.url()))
        .send()
        .await
        .unwrap();
    let bytes = response.bytes().await.unwrap();
    mock.assert_async().await;
    drop(mock);
    drop(server);

    let response = reqwest::Response::from(http::Response::new(bytes));
    let (events, receiver) = mpsc::unbounded_channel();
    let mut chain = InterceptorChain::new();
    for probe in build_probes(events) {
        chain.add(probe);
    }
    let (stream, completion) = InterceptedStream::new_with_completion(
        ChatCompletionStream::new(response),
        Arc::new(chain),
        "chat".into(),
        "mock-model".into(),
        "{}".into(),
        (),
    )
    .expect("active Tokio runtime is available in async tests");
    (stream, completion, receiver)
}

fn chunk(content: &str) -> String {
    format!(
        "data: {{\"id\":\"mock\",\"object\":\"chat.completion.chunk\",\
         \"created\":1,\"model\":\"mock-model\",\"choices\":[{{\"index\":0,\
         \"delta\":{{\"content\":\"{content}\"}},\"finish_reason\":null}}]}}\n\n"
    )
}

fn usage_chunk() -> String {
    "data: {\"id\":\"mock\",\"object\":\"chat.completion.chunk\",\"created\":1,\
     \"model\":\"mock-model\",\"choices\":[],\"usage\":{\"prompt_tokens\":3,\
     \"completion_tokens\":4,\"total_tokens\":7}}\n\n"
        .into()
}

fn probe(index: usize, events: &mpsc::UnboundedSender<Call>) -> Probe {
    Probe::new(index, events.clone())
}

#[tokio::test]
async fn callbacks_are_ordered_and_report_includes_usage_only_chunks() {
    let (mut stream, completion, mut calls) =
        intercepted(format!("{}{}", chunk("hello"), usage_chunk()), |events| {
            vec![Box::new(probe(0, &events)), Box::new(probe(1, &events))]
        })
        .await;

    assert!(stream.next().await.unwrap().is_ok());
    assert!(stream.next().await.unwrap().is_ok());
    assert!(stream.next().await.is_none());
    let report = tokio::time::timeout(Duration::from_secs(5), completion.wait())
        .await
        .expect("completion must resolve")
        .unwrap();

    assert_eq!(report.outcome, StreamOutcome::Completed);
    assert_eq!(report.total_chunks, 2);
    assert_eq!(report.input_tokens, Some(3));
    assert_eq!(report.output_tokens, Some(4));
    assert_eq!(report.callback_attempts, 6);
    assert_eq!(report.callback_successes, 6);
    assert_eq!(report.callback_failure_count, 0);
    assert_eq!(
        calls.try_recv().unwrap(),
        Call {
            hook: StreamCallbackHook::Chunk,
            interceptor: 0,
            chunk_index: Some(0),
            outcome: None
        }
    );
    assert_eq!(calls.try_recv().unwrap().interceptor, 1);
    assert_eq!(calls.try_recv().unwrap().chunk_index, Some(1));
    assert_eq!(calls.try_recv().unwrap().interceptor, 1);
    assert_eq!(
        calls.try_recv().unwrap().outcome,
        Some(StreamOutcome::Completed)
    );
    assert_eq!(
        calls.try_recv().unwrap().outcome,
        Some(StreamOutcome::Completed)
    );
    assert!(calls.try_recv().is_err());
    drop(stream);
}

#[tokio::test]
async fn terminal_callback_must_finish_before_report_and_wait_is_repeatable() {
    let gate = Arc::new(Semaphore::new(0));
    let gate_for_probe = gate.clone();
    let (mut stream, completion, mut calls) = intercepted(String::new(), move |events| {
        let mut delayed = probe(0, &events);
        delayed.terminal_gate = Some(gate_for_probe);
        vec![Box::new(delayed)]
    })
    .await;

    assert!(stream.next().await.is_none());
    assert_eq!(
        calls.recv().await.unwrap().hook,
        StreamCallbackHook::Terminal
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), completion.wait())
            .await
            .is_err()
    );
    gate.add_permits(1);

    let report = tokio::time::timeout(Duration::from_secs(5), completion.wait())
        .await
        .expect("terminal callback drain must finish")
        .unwrap();
    let (again, concurrent) = tokio::join!(completion.wait(), completion.wait());
    assert!(Arc::ptr_eq(&report, &again.unwrap()));
    assert!(Arc::ptr_eq(&report, &concurrent.unwrap()));
    assert_eq!(report.outcome, StreamOutcome::Completed);
    assert_eq!(report.callback_attempts, 1);
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
    assert!(calls.try_recv().is_err());
    drop(stream);
}

#[tokio::test]
async fn dropping_a_full_queue_drains_admitted_chunks_before_cancel_terminal() {
    let gate = Arc::new(Semaphore::new(0));
    let gate_for_probe = gate.clone();
    let (mut stream, completion, mut calls) = intercepted(
        format!("{}{}{}", chunk("one"), chunk("two"), chunk("three")),
        move |events| {
            let mut blocked = probe(0, &events);
            blocked.blocked_chunk = Some((0, gate_for_probe));
            vec![Box::new(blocked)]
        },
    )
    .await;

    assert!(stream.next().await.unwrap().is_ok());
    let first_call = calls.recv().await.unwrap();
    assert_eq!(first_call.chunk_index, Some(0));
    assert!(stream.next().await.unwrap().is_ok());

    let mut blocked_next = Box::pin(stream.next());
    assert!(futures::poll!(&mut blocked_next).is_pending());
    drop(blocked_next);
    drop(stream);

    assert!(
        tokio::time::timeout(Duration::from_millis(10), completion.wait())
            .await
            .is_err()
    );
    gate.add_permits(1);
    let report = tokio::time::timeout(Duration::from_secs(5), completion.wait())
        .await
        .expect("admitted callbacks and cancellation terminal must drain")
        .unwrap();

    assert_eq!(report.outcome, StreamOutcome::Cancelled);
    assert_eq!(report.total_chunks, 2);
    assert_eq!(report.callback_attempts, 3);
    assert_eq!(calls.recv().await.unwrap().chunk_index, Some(1));
    assert_eq!(
        calls.recv().await.unwrap().outcome,
        Some(StreamOutcome::Cancelled)
    );
    assert!(calls.try_recv().is_err());
}

#[tokio::test]
async fn stream_error_is_yielded_once_and_only_terminal_hook_is_called() {
    let (mut stream, completion, mut calls) = intercepted("data: not-json\n\n".into(), |events| {
        vec![Box::new(probe(0, &events))]
    })
    .await;

    assert!(matches!(
        stream.next().await.unwrap(),
        Err(Error::StreamParsing { .. })
    ));
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
    let report = completion.wait().await.unwrap();
    assert_eq!(
        report.outcome,
        StreamOutcome::StreamFailed(openai_ergonomic::StreamFailure {
            kind: openai_ergonomic::StreamFailureKind::Parsing
        })
    );
    assert_eq!(report.total_chunks, 0);
    assert_eq!(report.callback_attempts, 1);
    assert_eq!(calls.recv().await.unwrap().outcome, Some(report.outcome));
    assert!(calls.try_recv().is_err()); // No success end adapter and no on_error hook.
    drop(stream);
}

#[tokio::test]
async fn callback_errors_and_panics_are_recorded_and_later_callbacks_run() {
    let (mut stream, completion, mut calls) = intercepted(chunk("one"), |events| {
        let mut first = probe(0, &events);
        first.chunk_behavior = Behavior::Panic;
        let mut second = probe(1, &events);
        second.terminal_behavior = Behavior::ReturnError;
        vec![Box::new(first), Box::new(second)]
    })
    .await;

    assert!(stream.next().await.unwrap().is_ok());
    assert!(stream.next().await.is_none());
    let report = completion.wait().await.unwrap();
    assert_eq!(report.callback_attempts, 4);
    assert_eq!(report.callback_successes, 2);
    assert_eq!(report.callback_failure_count, 2);
    assert_eq!(report.callback_failures.len(), 2);
    assert_eq!(
        report.callback_failures[0].kind,
        StreamCallbackFailureKind::Panicked
    );
    assert_eq!(
        report.callback_failures[1].kind,
        StreamCallbackFailureKind::ReturnedError
    );
    let actual: Vec<_> = std::iter::from_fn(|| calls.try_recv().ok()).collect();
    assert_eq!(actual.len(), 4);
    assert_eq!(actual[0].hook, StreamCallbackHook::Chunk);
    assert_eq!(actual[0].interceptor, 0);
    assert_eq!(actual[1].interceptor, 1);
    assert_eq!(actual[2].hook, StreamCallbackHook::Terminal);
    assert_eq!(actual[2].interceptor, 0);
    assert_eq!(actual[3].interceptor, 1);
    drop(stream);
}

#[tokio::test]
async fn legacy_on_stream_end_is_forwarded_once_on_success() {
    let (mut stream, completion, mut calls) = intercepted(String::new(), |events| {
        vec![Box::new(LegacyEndProbe(events))]
    })
    .await;

    assert!(stream.next().await.is_none());
    let report = completion.wait().await.unwrap();
    assert_eq!(report.outcome, StreamOutcome::Completed);
    assert_eq!(report.callback_attempts, 1);
    assert_eq!(
        calls.recv().await.unwrap().outcome,
        Some(StreamOutcome::Completed)
    );
    assert!(calls.try_recv().is_err());
    drop(stream);
}

#[tokio::test]
async fn empty_interceptor_chain_reports_completion_without_callbacks() {
    let (mut stream, completion, _calls) = intercepted(String::new(), |_| Vec::new()).await;
    assert!(stream.next().await.is_none());
    let report = completion.wait().await.unwrap();
    assert_eq!(report.outcome, StreamOutcome::Completed);
    assert_eq!(report.total_chunks, 0);
    assert_eq!(report.callback_attempts, 0);
    assert_eq!(report.callback_failure_count, 0);
    drop(stream);
}

#[tokio::test]
async fn drop_before_first_poll_is_cancelled_and_drains_terminal_hook() {
    let (events, mut calls) = mpsc::unbounded_channel();
    let mut chain = InterceptorChain::new();
    chain.add(Box::new(probe(0, &events)));
    let response = reqwest::Response::from(http::Response::new(bytes::Bytes::new()));
    let (stream, completion) = InterceptedStream::new_with_completion(
        ChatCompletionStream::new(response),
        Arc::new(chain),
        "chat".into(),
        "mock-model".into(),
        "{}".into(),
        (),
    )
    .unwrap();

    drop(stream);
    let report = completion.wait().await.unwrap();
    assert_eq!(report.outcome, StreamOutcome::Cancelled);
    assert_eq!(report.total_chunks, 0);
    assert_eq!(report.callback_attempts, 1);
    assert_eq!(
        calls.recv().await.unwrap().outcome,
        Some(StreamOutcome::Cancelled)
    );
    assert!(calls.try_recv().is_err());
}

#[tokio::test]
async fn shared_interceptors_can_process_different_streams_concurrently() {
    let (events, mut calls) = mpsc::unbounded_channel();
    let mut chain = InterceptorChain::new();
    chain.add(Box::new(probe(0, &events)));
    let chain = Arc::new(chain);
    let make_stream = |body: String| {
        let response = reqwest::Response::from(http::Response::new(bytes::Bytes::from(body)));
        InterceptedStream::new_with_completion(
            ChatCompletionStream::new(response),
            Arc::clone(&chain),
            "chat".into(),
            "mock-model".into(),
            "{}".into(),
            (),
        )
        .unwrap()
    };
    let (mut first, first_completion) = make_stream(chunk("first"));
    let (mut second, second_completion) = make_stream(chunk("second"));

    let (first_item, second_item) = tokio::join!(first.next(), second.next());
    assert!(first_item.unwrap().is_ok());
    assert!(second_item.unwrap().is_ok());
    assert!(first.next().await.is_none());
    assert!(second.next().await.is_none());
    let (first_report, second_report) =
        tokio::join!(first_completion.wait(), second_completion.wait());
    assert_eq!(first_report.unwrap().outcome, StreamOutcome::Completed);
    assert_eq!(second_report.unwrap().outcome, StreamOutcome::Completed);
    assert_eq!(calls.recv().await.unwrap().hook, StreamCallbackHook::Chunk);
    assert_eq!(calls.recv().await.unwrap().hook, StreamCallbackHook::Chunk);
    assert_eq!(
        calls.recv().await.unwrap().hook,
        StreamCallbackHook::Terminal
    );
    assert_eq!(
        calls.recv().await.unwrap().hook,
        StreamCallbackHook::Terminal
    );
}

#[tokio::test]
async fn callback_failure_details_are_bounded_with_exact_truncation_count() {
    let (mut stream, completion, _calls) = intercepted(String::new(), |events| {
        (0..66)
            .map(|index| {
                let mut failing = probe(index, &events);
                failing.terminal_behavior = Behavior::ReturnError;
                Box::new(failing) as Box<dyn Interceptor>
            })
            .collect()
    })
    .await;

    assert!(stream.next().await.is_none());
    let report = completion.wait().await.unwrap();
    assert_eq!(report.callback_attempts, 66);
    assert_eq!(report.callback_successes, 0);
    assert_eq!(report.callback_failure_count, 66);
    assert_eq!(report.callback_failures.len(), 64);
    assert_eq!(report.truncated_failure_details, 2);
    drop(stream);
}

#[tokio::test]
async fn client_chat_and_responses_methods_return_completion_handles() {
    use openai_ergonomic::{Client, Config};

    let mut server = mockito::Server::new_async().await;
    let chat_mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(chunk("chat"))
        .create_async()
        .await;
    let responses_mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(chunk("responses"))
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
        .build();
    let client = Client::builder(config).unwrap().build();

    let (mut chat_stream, chat_completion) = client
        .send_chat_stream_with_completion(client.chat().user("hello"))
        .await
        .unwrap();
    assert!(chat_stream.next().await.unwrap().is_ok());
    assert!(chat_stream.next().await.is_none());
    assert_eq!(
        chat_completion.wait().await.unwrap().outcome,
        StreamOutcome::Completed
    );

    let (mut responses_stream, responses_completion) = client
        .send_responses_stream_with_completion(client.responses().user("hello"))
        .await
        .unwrap();
    assert!(responses_stream.next().await.unwrap().is_ok());
    assert!(responses_stream.next().await.is_none());
    assert_eq!(
        responses_completion.wait().await.unwrap().outcome,
        StreamOutcome::Completed
    );
    chat_mock.assert_async().await;
    responses_mock.assert_async().await;
    drop(chat_stream);
    drop(responses_stream);
    drop(server);
}

#[test]
fn direct_completion_constructor_reports_missing_runtime_without_panicking() {
    let response = reqwest::Response::from(http::Response::new(bytes::Bytes::new()));
    let chain = Arc::new(InterceptorChain::new());
    let result = InterceptedStream::new_with_completion(
        ChatCompletionStream::new(response),
        Arc::clone(&chain),
        "chat".into(),
        "mock-model".into(),
        "{}".into(),
        (),
    );
    assert!(
        matches!(result, Err(Error::Config(message)) if message.contains("active Tokio runtime"))
    );

    let response = reqwest::Response::from(http::Response::new(bytes::Bytes::new()));
    let stream = InterceptedStream::new(
        ChatCompletionStream::new(response),
        chain,
        "chat".into(),
        "mock-model".into(),
        "{}".into(),
        (),
    );
    drop(stream); // Construction and drop before first poll need no runtime.
}
