//! `json_call` and `json_event_call`: JSON bodies of the caller's own, through the interceptors,
//! with the status, headers, rate limits and retry delay of the response.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]
// Test hooks are plain `async fn` without `.await`, as users write them; the lint
// exists only in newer Clippy, hence `unknown_lints` for the MSRV toolchain.
#![allow(unknown_lints, clippy::unused_async_trait_impl)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use mockito::{Matcher, Server};
use openai_ergonomic::semantic_conventions::operation_names;
use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, Client, Config, Error, ErrorContext, Interceptor,
    StreamChunkContext, StreamTerminalContext,
};
use serde_json::json;

/// Records what the interceptor hooks saw.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<String>>>);

impl Recorder {
    fn seen(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl Interceptor for Recorder {
    async fn before_request(
        &self,
        ctx: &mut BeforeRequestContext<'_>,
    ) -> openai_ergonomic::Result<()> {
        self.0
            .lock()
            .unwrap()
            .push(format!("before {} {}", ctx.operation, ctx.model));
        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> openai_ergonomic::Result<()> {
        self.0.lock().unwrap().push(format!(
            "after {} {:?} {:?}",
            ctx.operation, ctx.input_tokens, ctx.output_tokens
        ));
        Ok(())
    }

    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> openai_ergonomic::Result<()> {
        self.0
            .lock()
            .unwrap()
            .push(format!("chunk {}", ctx.chunk_index));
        Ok(())
    }

    async fn on_stream_terminal(
        &self,
        ctx: &StreamTerminalContext<'_>,
    ) -> openai_ergonomic::Result<()> {
        self.0.lock().unwrap().push(format!(
            "end {:?} {:?}",
            ctx.summary.input_tokens, ctx.summary.output_tokens
        ));
        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        self.0
            .lock()
            .unwrap()
            .push(format!("error {}", ctx.operation));
    }
}

fn client(server: &Server, recorder: &Recorder) -> Client {
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .max_retries(0)
        .build();
    Client::builder(config)
        .unwrap()
        .with_interceptor(Box::new(recorder.clone()))
        .build()
}

#[tokio::test]
async fn a_json_call_goes_through_the_interceptors_and_keeps_the_headers() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/responses")
        .match_header("authorization", "Bearer test-key")
        .match_body(Matcher::Json(json!({"model": "m", "input": "hi", "extra": 1})))
        .with_header("content-type", "application/json")
        .with_header("x-request-id", "req_1")
        .with_header("x-ratelimit-limit-requests", "100")
        .with_header("x-ratelimit-remaining-requests", "99")
        .with_header("x-ratelimit-reset-requests", "1s")
        .with_body(r#"{"id":"resp_1","output":[],"usage":{"input_tokens":3,"output_tokens":4},"vendor":true}"#)
        .create_async()
        .await;
    let recorder = Recorder::default();
    let client = client(&server, &recorder);

    let response = client
        .json_call(
            operation_names::RESPONSES,
            "/responses",
            &json!({"model": "m", "input": "hi", "extra": 1}),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    // The body as it came, fields the spec does not know included.
    assert_eq!(response.body["vendor"], true);
    assert_eq!(response.meta.status(), Some(200));
    assert_eq!(response.meta.headers()["x-request-id"], "req_1");
    let requests = response.meta.rate_limits().unwrap().requests.unwrap();
    assert_eq!(requests.remaining, Some(99));
    assert_eq!(
        recorder.seen(),
        ["before responses m", "after responses Some(3) Some(4)"]
    );
}

#[tokio::test]
async fn a_failed_json_call_keeps_its_retry_delay_and_is_before_the_body() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_header("retry-after-ms", "250")
        .with_body(
            r#"{"error":{"message":"slow down","type":"requests","code":"rate_limit_exceeded"}}"#,
        )
        .create_async()
        .await;
    let recorder = Recorder::default();
    let client = client(&server, &recorder);

    let failure = client
        .json_call(
            operation_names::CHAT,
            "/chat/completions",
            &json!({"model": "m", "messages": []}),
        )
        .await
        .unwrap_err();

    assert!(failure.is_before_body());
    assert_eq!(failure.meta().status(), Some(429));
    assert_eq!(
        failure.meta().retry_after(),
        Some(Duration::from_millis(250))
    );
    match failure.error() {
        Error::Api {
            status, error_code, ..
        } => {
            assert_eq!(*status, 429);
            assert_eq!(error_code.as_deref(), Some("rate_limit_exceeded"));
        }
        other => panic!("an API error expected, got {other:?}"),
    }
    assert_eq!(recorder.seen(), ["before chat m", "error chat"]);
    // It converts back into the crate's error.
    let error: Error = failure.into();
    assert!(error.is_rate_limit());
}

#[tokio::test]
async fn a_json_event_call_streams_the_events_through_the_interceptors() {
    let mut server = Server::new_async().await;
    let body = [
        r#"data: {"type":"response.output_text.delta","delta":"Hel"}"#,
        r#"data: {"type":"response.output_text.delta","delta":"lo"}"#,
        r#"data: {"type":"response.completed","response":{"id":"r","usage":{"input_tokens":1,"output_tokens":2}}}"#,
        "data: [DONE]",
    ]
    .join("\n\n")
        + "\n\n";
    let mock = server
        .mock("POST", "/responses")
        .match_header("accept", "text/event-stream")
        .with_header("content-type", "text/event-stream")
        .with_header("x-ratelimit-remaining-tokens", "1000")
        .with_body(body)
        .create_async()
        .await;
    let recorder = Recorder::default();
    let client = client(&server, &recorder);

    let (events, meta) = client
        .json_event_call(
            operation_names::RESPONSES,
            "/responses",
            &json!({"model": "m", "input": "hi", "stream": true}),
        )
        .await
        .unwrap();
    let events: Vec<_> = events.map(Result::unwrap).collect().await;

    mock.assert_async().await;
    assert_eq!(meta.status(), Some(200));
    assert_eq!(
        meta.rate_limits().unwrap().tokens.unwrap().remaining,
        Some(1000)
    );
    let deltas: String = events
        .iter()
        .filter_map(|event| event.0["delta"].as_str())
        .collect();
    assert_eq!(deltas, "Hello");
    assert_eq!(events.len(), 3);
    // The stream hooks run on a task of their own; give them a moment to finish.
    for _ in 0..50 {
        if recorder.seen().iter().any(|seen| seen.starts_with("end")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        recorder.seen(),
        [
            "before responses m",
            "chunk 0",
            "chunk 1",
            "chunk 2",
            "end Some(1) Some(2)"
        ]
    );
}

#[tokio::test]
async fn a_json_event_call_without_interceptors_needs_none() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/chat/completions")
        .with_header("content-type", "text/event-stream")
        .with_body("data: {\"choices\":[]}\n\ndata: not json\n\n")
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .max_retries(0)
        .build();
    let client = Client::builder(config).unwrap().build();

    let (mut events, _) = client
        .json_event_call(
            operation_names::CHAT,
            "/chat/completions",
            &json!({"model": "m", "stream": true}),
        )
        .await
        .unwrap();
    assert_eq!(
        events.next().await.unwrap().unwrap().0,
        json!({"choices": []})
    );
    assert!(matches!(
        events.next().await.unwrap(),
        Err(Error::StreamParsing { .. })
    ));
}
