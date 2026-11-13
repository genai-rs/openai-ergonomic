//! WebAssembly: the client in a JavaScript host against a scripted HTTP service (a tower service
//! that is not `Send`, in place of a server): a chat completion, a streamed one with an
//! interceptor, an interceptor whose state holds an `Rc`, a tool call through the tool framework, a webhook verification, and retries with
//! backoff (the runtime's retry layer, and the parts of a large upload).
//!
//! `cargo test --target wasm32-unknown-unknown --no-default-features --features webhook --test wasm`, with
//! `wasm-bindgen-test-runner` as the target's runner (it runs the tests in Node.js); see
//! docs/webassembly.md.
#![cfg(target_family = "wasm")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]
// Nothing here is `Send`: WebAssembly futures stay on the host's thread.
#![allow(clippy::future_not_send, clippy::unnecessary_literal_bound)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use futures::StreamExt;
use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, Client, Config, FunctionTool, Interceptor,
    StreamChunkContext, StreamTerminalContext, ToolRegistry, UploadPurpose,
};
use serde_json::{json, Value};
use timvw_rust_runtime::retry::{RetryLayer, RetryPolicy};
use timvw_rust_runtime::{Body, BoxError, Upload};
use wasm_bindgen_test::wasm_bindgen_test;

/// A reply of the scripted service.
struct Reply {
    status: u16,
    content_type: &'static str,
    body: String,
}

fn json_reply(body: &Value) -> Reply {
    Reply {
        status: 200,
        content_type: "application/json",
        body: body.to_string(),
    }
}

/// A request the scripted service received: its method, path and body.
#[derive(Debug, Clone)]
struct Seen {
    path: String,
    body: Vec<u8>,
}

/// An HTTP client whose service answers with `script(request number, path)`, recording each
/// request. It holds `Rc`s: it is not `Send`.
fn scripted(
    script: impl Fn(usize, &str) -> Reply + 'static,
) -> (timvw_rust_runtime::Client, Rc<RefCell<Vec<Seen>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let script = Rc::new(script);
    let service = tower::service_fn(move |request: http::Request<Body>| {
        let log = log.clone();
        let script = script.clone();
        async move {
            let path = request.uri().path().to_owned();
            let body = request.into_body().collect_bytes().await?.to_vec();
            let reply = script(log.borrow().len(), &path);
            log.borrow_mut().push(Seen { path, body });
            Ok::<_, BoxError>(
                http::Response::builder()
                    .status(reply.status)
                    .header("content-type", reply.content_type)
                    .body(Body::from(reply.body))?,
            )
        }
    });
    (timvw_rust_runtime::Client::from_service(service), seen)
}

fn client(http: timvw_rust_runtime::Client) -> Client {
    let config = Config::builder()
        .api_key("sk-test")
        .api_base("https://api.example/v1")
        .http_client(http)
        .build();
    Client::builder(config).unwrap().build()
}

fn completion(message: &Value) -> Value {
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-4o-mini",
        "choices": [{"index": 0, "finish_reason": "stop", "logprobs": null, "message": message}],
        "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
    })
}

#[wasm_bindgen_test]
async fn chat_completion() {
    let (http, seen) = scripted(|_, _| {
        json_reply(&completion(
            &json!({"role": "assistant", "content": "Hello from wasm", "refusal": null}),
        ))
    });
    let client = client(http);
    let response = client
        .send_chat(client.chat_simple("Say hello"))
        .await
        .unwrap();
    assert_eq!(response.content(), Some("Hello from wasm"));
    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/v1/chat/completions");
    let request: Value = serde_json::from_slice(&seen[0].body).unwrap();
    assert_eq!(request["messages"][0]["content"], "Say hello");
}

/// Counts chunks and records the terminal outcome; holds `Rc`s, so it is not `Send` or `Sync`.
struct CountChunks {
    chunks: Rc<Cell<usize>>,
    finished: Rc<Cell<bool>>,
}

impl Interceptor for CountChunks {
    async fn on_stream_chunk(&self, _ctx: &StreamChunkContext<'_>) -> openai_ergonomic::Result<()> {
        self.chunks.set(self.chunks.get() + 1);
        Ok(())
    }

    async fn on_stream_terminal(
        &self,
        ctx: &StreamTerminalContext<'_>,
    ) -> openai_ergonomic::Result<()> {
        self.finished.set(matches!(
            ctx.outcome,
            openai_ergonomic::StreamOutcome::Completed
        ));
        Ok(())
    }
}

#[wasm_bindgen_test]
async fn streamed_chat_completion_with_an_interceptor() {
    let chunk = |content: &str| {
        json!({
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "created": 1,
            "model": "gpt-4o-mini",
            "choices": [{"index": 0, "delta": {"content": content}, "finish_reason": null}]
        })
    };
    let events = format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        chunk("Hel"),
        chunk("lo")
    );
    let (http, _seen) = scripted(move |_, _| Reply {
        status: 200,
        content_type: "text/event-stream",
        body: events.clone(),
    });
    let chunks = Rc::new(Cell::new(0));
    let finished = Rc::new(Cell::new(false));
    let config = Config::builder()
        .api_key("sk-test")
        .api_base("https://api.example/v1")
        .http_client(http)
        .build();
    let client = Client::builder(config)
        .unwrap()
        .with_interceptor(CountChunks {
            chunks: chunks.clone(),
            finished: finished.clone(),
        })
        .build();

    let (mut stream, completion) = client
        .send_chat_stream_with_completion(client.chat_simple("Hi"))
        .await
        .unwrap();
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        text.push_str(chunk.unwrap().content().unwrap_or_default());
    }
    assert_eq!(text, "Hello");
    drop(stream);
    // The callbacks run on the host's event loop (`spawn_local`).
    let report = completion.wait().await.unwrap();
    assert_eq!(report.total_chunks, 2);
    assert_eq!(chunks.get(), 2);
    assert!(finished.get());
}

/// Interceptor state that holds an `Rc`: neither `Send` nor `Sync`, which WebAssembly allows.
#[derive(Default)]
struct RcState {
    log: Rc<RefCell<Vec<String>>>,
}

/// Puts its log into the state before the request and appends to it after the response.
struct RecordWithRcState {
    log: Rc<RefCell<Vec<String>>>,
}

impl Interceptor<RcState> for RecordWithRcState {
    async fn before_request(
        &self,
        ctx: &mut BeforeRequestContext<'_, RcState>,
    ) -> openai_ergonomic::Result<()> {
        ctx.state.log = self.log.clone();
        ctx.state
            .log
            .borrow_mut()
            .push(format!("before {}", ctx.operation));
        Ok(())
    }

    async fn after_response(
        &self,
        ctx: &AfterResponseContext<'_, RcState>,
    ) -> openai_ergonomic::Result<()> {
        ctx.state.log.borrow_mut().push(format!(
            "after {} ({} output tokens)",
            ctx.operation,
            ctx.output_tokens.unwrap_or_default()
        ));
        Ok(())
    }
}

#[wasm_bindgen_test]
async fn interceptor_state_that_is_not_send() {
    let (http, _seen) = scripted(|_, _| {
        json_reply(&completion(
            &json!({"role": "assistant", "content": "Hi", "refusal": null}),
        ))
    });
    let log = Rc::new(RefCell::new(Vec::new()));
    let config = Config::builder()
        .api_key("sk-test")
        .api_base("https://api.example/v1")
        .http_client(http)
        .build();
    let client: Client<RcState> = Client::builder(config)
        .unwrap()
        .with_interceptor(RecordWithRcState { log: log.clone() })
        .build();
    let response = client.send_chat(client.chat_simple("Hi")).await.unwrap();
    assert_eq!(response.content(), Some("Hi"));
    // A sub-client works with the state too.
    let _ = client.models();
    assert_eq!(
        *log.borrow(),
        ["before chat", "after chat (2 output tokens)"]
    );
}

/// A tool that keeps its state in an `Rc`: not `Send` or `Sync`, as WebAssembly allows.
struct Add {
    calls: Rc<Cell<usize>>,
}

#[derive(serde::Deserialize)]
struct AddInput {
    a: i64,
    b: i64,
}

impl FunctionTool for Add {
    type Input = AddInput;
    type Output = i64;

    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> &str {
        "Adds two integers"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, input: AddInput) -> openai_ergonomic::Result<i64> {
        // Yield to the host's event loop once, as a real tool would.
        gloo_timers::future::sleep(Duration::from_millis(1)).await;
        self.calls.set(self.calls.get() + 1);
        Ok(input.a + input.b)
    }
}

#[wasm_bindgen_test]
async fn tool_call_through_the_tool_framework() {
    let (http, seen) = scripted(|index, _| {
        if index == 0 {
            json_reply(&completion(&json!({
                "role": "assistant",
                "content": null,
                "refusal": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "add", "arguments": "{\"a\": 2, \"b\": 40}"}
                }]
            })))
        } else {
            json_reply(&completion(
                &json!({"role": "assistant", "content": "42", "refusal": null}),
            ))
        }
    });
    let client = client(http);
    let calls = Rc::new(Cell::new(0));
    let mut tools = ToolRegistry::new();
    tools
        .register(Add {
            calls: calls.clone(),
        })
        .unwrap();

    let builder = client
        .chat_simple("What is 2 + 40?")
        .tools(tools.tool_definitions());
    let response = client.send_chat(builder.clone()).await.unwrap();
    let tool_calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
    assert_eq!(tool_calls.len(), 1);
    let mut follow_up = builder.assistant_with_tool_calls("", tool_calls.clone());
    for call in &tool_calls {
        let output = tools.execute_call(call).await.unwrap();
        assert_eq!(output.call_id, "call_1");
        assert_eq!(output.content, "42");
        follow_up = follow_up.tool(output.call_id, output.content);
    }
    let answer = client.send_chat(follow_up).await.unwrap();
    assert_eq!(answer.content(), Some("42"));
    assert_eq!(calls.get(), 1);

    let seen = seen.borrow();
    let second: Value = serde_json::from_slice(&seen[1].body).unwrap();
    assert_eq!(second["tools"][0]["function"]["name"], "add");
    assert_eq!(second["messages"][2]["tool_call_id"], "call_1");
}

#[wasm_bindgen_test]
fn webhook_verification() {
    use openai_ergonomic::webhooks::{
        WebhookEvent, WebhookVerifier, WEBHOOK_ID, WEBHOOK_SIGNATURE, WEBHOOK_TIMESTAMP,
    };

    let verifier = WebhookVerifier::new("whsec_c2VjcmV0LWtleS1mb3ItdGVzdHM=").unwrap();
    let body = json!({
        "id": "evt_1",
        "object": "event",
        "created_at": 1_750_000_000,
        "type": "response.completed",
        "data": {"id": "resp_1"}
    })
    .to_string()
    .into_bytes();
    // The host's clock (`Date.now()`): `std::time::SystemTime::now()` panics here.
    #[allow(clippy::cast_possible_truncation)]
    let now = (js_sys_now_millis() / 1000.0) as i64;
    let mut headers = http::HeaderMap::new();
    headers.insert(WEBHOOK_ID, "msg_1".parse().unwrap());
    headers.insert(WEBHOOK_TIMESTAMP, now.to_string().parse().unwrap());
    headers.insert(
        WEBHOOK_SIGNATURE,
        verifier.sign("msg_1", now, &body).parse().unwrap(),
    );
    let event = verifier.unwrap(&headers, &body).unwrap();
    assert!(matches!(event, WebhookEvent::ResponseCompleted(ref e) if e.data.id == "resp_1"));

    // A tampered body, and an old delivery, are rejected.
    assert!(verifier.verify(&headers, b"{}").is_err());
    let mut old = headers.clone();
    old.insert(WEBHOOK_TIMESTAMP, (now - 3600).to_string().parse().unwrap());
    old.insert(
        WEBHOOK_SIGNATURE,
        verifier.sign("msg_1", now - 3600, &body).parse().unwrap(),
    );
    assert!(verifier.verify(&old, &body).is_err());
}

/// Milliseconds since the Unix epoch, from the host (`Date.now()`).
fn js_sys_now_millis() -> f64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}

#[wasm_bindgen_test]
async fn retries_with_backoff() {
    // Two 503s, then the answer: the runtime's retry layer waits with the host's `setTimeout`.
    let (http, seen) = scripted(|index, _| {
        if index < 2 {
            Reply {
                status: 503,
                content_type: "application/json",
                body: json!({"error": {"message": "busy", "type": "server_error"}}).to_string(),
            }
        } else {
            json_reply(&completion(
                &json!({"role": "assistant", "content": "finally", "refusal": null}),
            ))
        }
    });
    let http = http.layer(RetryLayer::new(
        RetryPolicy::new(3)
            .base_delay(Duration::from_millis(20))
            .jitter(false),
    ));
    let client = client(http);
    let start = web_time::Instant::now();
    let response = client.send_chat(client.chat_simple("Hi")).await.unwrap();
    assert_eq!(response.content(), Some("finally"));
    assert_eq!(seen.borrow().len(), 3);
    // 20 ms, then 40 ms.
    assert!(
        start.elapsed() >= Duration::from_millis(60),
        "{:?}",
        start.elapsed()
    );
}

#[wasm_bindgen_test]
async fn large_upload_parts_are_retried_with_backoff_concurrently() {
    let upload = |status: &str| {
        json!({
            "id": "upload_1", "object": "upload", "created_at": 1, "expires_at": 2,
            "filename": "data.jsonl", "purpose": "fine-tune", "bytes": 6, "status": status,
            "file": null
        })
    };
    let failed_once = Rc::new(Cell::new(false));
    let flag = failed_once.clone();
    let (http, seen) = scripted(move |_, path| match path {
        "/v1/uploads" => json_reply(&upload("pending")),
        "/v1/uploads/upload_1/parts" if !flag.replace(true) => Reply {
            status: 503,
            content_type: "application/json",
            body: json!({"error": {"message": "busy", "type": "server_error"}}).to_string(),
        },
        "/v1/uploads/upload_1/parts" => json_reply(&json!({
            "id": "part", "object": "upload.part", "created_at": 1, "upload_id": "upload_1"
        })),
        _ => json_reply(&upload("completed")),
    });
    let client = client(http);
    let start = web_time::Instant::now();
    let completed = client
        .upload_large_file(
            Upload::bytes(b"abcdef".to_vec()).file_name("data.jsonl"),
            UploadPurpose::FineTune,
        )
        .mime_type("text/jsonl")
        .part_size(2)
        .concurrency(2)
        .retry_backoff(Duration::from_millis(30))
        .await
        .unwrap();
    assert_eq!(completed.status.to_string(), "completed");
    assert!(start.elapsed() >= Duration::from_millis(30));
    let seen = seen.borrow();
    let parts = seen
        .iter()
        .filter(|seen| seen.path.ends_with("/parts"))
        .count();
    // Three parts, one of them sent twice.
    assert_eq!(parts, 4);
    assert!(seen.last().unwrap().path.ends_with("/complete"));
}

#[wasm_bindgen_test]
async fn an_upload_from_a_path_fails_without_a_file_system() {
    let (http, seen) = scripted(|_, _| json_reply(&json!({})));
    let client = client(http);
    let error = client
        .files()
        .upload(client.files().upload_from_path(
            "data.jsonl",
            openai_ergonomic::builders::files::FilePurpose::Batch,
        ))
        .await
        .unwrap_err();
    assert!(error.is_io(), "{error:?}");
    assert_eq!(
        error.io_error().unwrap().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert!(seen.borrow().is_empty());
}
