//! The Responses API client (`/responses`) against a mock server: request shape, response
//! parsing, streaming events and error mapping.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]
#![allow(
    clippy::significant_drop_tightening,
    clippy::type_complexity,
    clippy::format_collect
)]

use futures::StreamExt;
use mockito::{Matcher, Server};
use openai_ergonomic::interceptor::{AfterResponseContext, ErrorContext};
use openai_ergonomic::responses_api::{OutputItem, ResponsesEvent};
use openai_ergonomic::{Client, Config, Error, Interceptor, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

fn config_for(server: &Server) -> Config {
    Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4.1-mini")
        .max_retries(0)
        .build()
}

fn client_for(server: &Server) -> Client {
    Client::builder(config_for(server)).unwrap().build()
}

/// A response as the API reference shows it: `instructions: null`, no `access_programs`, an
/// output text without `logprobs`, usage without `cache_write_tokens` (all of which the
/// generated models accept).
fn response_body() -> Value {
    json!({
        "id": "resp_123",
        "object": "response",
        "created_at": 1_741_476_542,
        "status": "completed",
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "max_output_tokens": null,
        "model": "gpt-4.1-mini-2025-04-14",
        "output": [
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "get_weather",
                "arguments": "{\"city\":\"Ghent\"}",
                "status": "completed"
            },
            {
                "type": "message",
                "id": "msg_1",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "Hello", "annotations": []},
                    {"type": "output_text", "text": " there", "annotations": []}
                ]
            }
        ],
        "parallel_tool_calls": true,
        "previous_response_id": null,
        "reasoning": {"effort": null, "summary": null},
        "store": true,
        "temperature": 1.0,
        "text": {"format": {"type": "text"}},
        "tool_choice": "auto",
        "tools": [],
        "top_p": 1.0,
        "truncation": "disabled",
        "usage": {
            "input_tokens": 36,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens": 87,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 123
        },
        "user": null,
        "metadata": {}
    })
}

#[tokio::test]
async fn send_responses_posts_the_request_and_parses_the_response() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/responses")
        .match_header("authorization", "Bearer test-key")
        .match_body(Matcher::Json(json!({
            "model": "gpt-4.1-mini",
            "instructions": "Be brief",
            "input": [{"type": "message", "role": "user", "content": "Weather in Ghent?"}],
            "store": false,
            "max_output_tokens": 64
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;

    let client = client_for(&server);
    let response = client
        .send_responses(
            client
                .responses()
                .instructions("Be brief")
                .user("Weather in Ghent?")
                .store(false)
                .max_output_tokens(64),
        )
        .await
        .unwrap();
    mock.assert_async().await;

    assert_eq!(response.id(), "resp_123");
    assert_eq!(
        response.status(),
        Some(openai_client_base::models::response::Status::Completed)
    );
    assert_eq!(response.output_text(), "Hello there");
    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].call_id, "call_1");
    assert_eq!(calls[0].arguments, r#"{"city":"Ghent"}"#);
    let usage = response.usage().unwrap();
    assert_eq!((usage.input_tokens, usage.output_tokens), (36, 87));
    assert!(response.error().is_none());
}

#[tokio::test]
async fn function_tools_send_strict_null_when_unset() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({
            "tools": [
                {"type": "function", "name": "lookup", "strict": null},
                {"type": "function", "name": "exact", "strict": true}
            ]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;

    let client = client_for(&server);
    let mut exact = openai_ergonomic::responses_api::responses_function_tool(
        "exact",
        "Strict function",
        json!({"type": "object", "properties": {}, "additionalProperties": false}),
    );
    if let openai_ergonomic::responses_api::ResponsesTool::FunctionTool(function) = &mut exact {
        function.strict = Some(true);
    }
    client
        .send_responses(
            client
                .responses()
                .function_tool("lookup", "Look a term up", json!({"type": "object"}))
                .tool(exact)
                .user("Hi"),
        )
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn function_call_outputs_and_earlier_output_are_sent_back_as_input() {
    let mut server = Server::new_async().await;
    let first = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({"input": "Weather in Ghent?"})))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;

    let client = client_for(&server);
    let response = client
        .send_responses(client.responses().input_text("Weather in Ghent?"))
        .await
        .unwrap();
    first.assert_async().await;

    let second = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({
            "input": [
                {"type": "message", "role": "user", "content": "Weather in Ghent?"},
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "get_weather",
                    "arguments": "{\"city\":\"Ghent\"}"
                },
                {"type": "message", "role": "assistant"},
                {"type": "function_call_output", "call_id": "call_1", "output": "12 degrees"}
            ]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;
    let call_id = response.function_calls()[0].call_id.clone();
    client
        .send_responses(
            client
                .responses()
                .user("Weather in Ghent?")
                .input_items(response.output_as_input().unwrap())
                .function_call_output(call_id, "12 degrees"),
        )
        .await
        .unwrap();
    second.assert_async().await;
}

#[tokio::test]
async fn rate_limits_and_api_errors_map_to_error_api() {
    let mut server = Server::new_async().await;
    let _limited = server
        .mock("POST", "/responses")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(
            json!({"error": {
                "message": "Rate limit reached",
                "type": "requests",
                "code": "rate_limit_exceeded"
            }})
            .to_string(),
        )
        .create_async()
        .await;

    let client = client_for(&server);
    let error = client
        .send_responses(client.responses().user("Hi"))
        .await
        .unwrap_err();
    assert!(error.is_rate_limit(), "{error:?}");
    assert!(error.is_retryable());
    match &error {
        Error::Api {
            status,
            error_code,
            error_type,
            ..
        } => {
            assert_eq!(*status, 429);
            assert_eq!(error_code.as_deref(), Some("rate_limit_exceeded"));
            assert_eq!(error_type.as_deref(), Some("requests"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }

    // The same mapping applies before a stream starts.
    let error = client
        .send_responses_stream(client.responses().user("Hi"))
        .await
        .err()
        .unwrap();
    assert!(error.is_rate_limit(), "{error:?}");

    let mut server = Server::new_async().await;
    let _invalid = server
        .mock("POST", "/responses")
        .with_status(400)
        .with_body(
            json!({"error": {
                "message": "Unknown parameter",
                "type": "invalid_request_error",
                "code": "unknown_parameter"
            }})
            .to_string(),
        )
        .create_async()
        .await;
    let client = client_for(&server);
    let error = client
        .send_responses(client.responses().user("Hi"))
        .await
        .unwrap_err();
    assert!(error.is_client_error());
    assert!(!error.is_retryable());
    assert!(matches!(error, Error::Api { status: 400, .. }));
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            )
        })
        .collect()
}

fn stream_events() -> Vec<Value> {
    let mut created = response_body();
    created["status"] = json!("in_progress");
    created["output"] = json!([]);
    created["usage"] = Value::Null;
    vec![
        json!({"type": "response.created", "sequence_number": 0, "response": created}),
        json!({
            "type": "response.output_item.added",
            "sequence_number": 1,
            "output_index": 0,
            "item": {"type": "message", "id": "msg_1", "status": "in_progress", "role": "assistant", "content": []}
        }),
        json!({
            "type": "response.output_text.delta",
            "sequence_number": 2,
            "item_id": "msg_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "Hel"
        }),
        json!({
            "type": "response.output_text.delta",
            "sequence_number": 3,
            "item_id": "msg_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "lo"
        }),
        json!({
            "type": "response.output_item.done",
            "sequence_number": 4,
            "output_index": 1,
            "item": {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "get_weather",
                "arguments": "{}",
                "status": "completed"
            }
        }),
        json!({"type": "response.completed", "sequence_number": 5, "response": response_body()}),
    ]
}

#[tokio::test]
async fn send_responses_stream_yields_typed_events() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/responses")
        .match_header("accept", "text/event-stream")
        .match_body(Matcher::Json(json!({
            "model": "gpt-4.1-mini",
            "input": [{"type": "message", "role": "user", "content": "Hi"}],
            "stream": true
        })))
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(sse(&stream_events()))
        .create_async()
        .await;

    let client = client_for(&server);
    let events: Vec<ResponsesEvent> = client
        .send_responses_stream(client.responses().user("Hi"))
        .await
        .unwrap()
        .map(|event| event.unwrap())
        .collect()
        .await;
    mock.assert_async().await;

    let types: Vec<_> = events.iter().map(ResponsesEvent::event_type).collect();
    assert_eq!(
        types,
        [
            "response.created",
            "response.output_item.added",
            "response.output_text.delta",
            "response.output_text.delta",
            "response.output_item.done",
            "response.completed"
        ]
    );
    let text: String = events
        .iter()
        .filter_map(ResponsesEvent::text_delta)
        .collect();
    assert_eq!(text, "Hello");
    match events[4].output_item_done() {
        Some(OutputItem::FunctionToolCall(call)) => assert_eq!(call.name, "get_weather"),
        other => panic!("expected a function call, got {other:?}"),
    }
    let completed = events[5].completed_response().unwrap();
    assert_eq!(completed.id, "resp_123");
    assert!(events[5].is_terminal());
}

#[tokio::test]
async fn a_stream_cut_off_before_completion_ends_with_an_error() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/responses")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(sse(&stream_events()[..3]))
        .create_async()
        .await;

    let client = client_for(&server);
    let items: Vec<_> = client
        .send_responses_stream(client.responses().user("Hi"))
        .await
        .unwrap()
        .collect()
        .await;
    assert_eq!(items.len(), 4);
    assert!(items[..3].iter().all(std::result::Result::is_ok));
    assert!(matches!(items[3], Err(Error::StreamConnection { .. })));
}

#[derive(Default)]
struct Recorder {
    calls: Mutex<Vec<(String, Option<i64>, Option<i64>)>>,
    errors: Mutex<Vec<String>>,
}

struct Recording(Arc<Recorder>);

impl Interceptor for Recording {
    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> Result<()> {
        self.0.calls.lock().unwrap().push((
            ctx.operation.to_string(),
            ctx.input_tokens,
            ctx.output_tokens,
        ));
        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        self.0
            .errors
            .lock()
            .unwrap()
            .push(ctx.operation.to_string());
    }
}

#[tokio::test]
async fn interceptors_see_responses_calls() {
    let mut server = Server::new_async().await;
    let _ok = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({"input": [{"content": "ok"}]})))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;
    let _failing = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(
            json!({"input": [{"content": "fail"}]}),
        ))
        .with_status(500)
        .with_body("{}")
        .create_async()
        .await;

    let recorder = Arc::new(Recorder::default());
    let client = Client::builder(config_for(&server))
        .unwrap()
        .add_interceptor(Recording(Arc::clone(&recorder)))
        .build();
    client
        .send_responses(client.responses().user("ok"))
        .await
        .unwrap();
    assert!(client
        .send_responses(client.responses().user("fail"))
        .await
        .unwrap_err()
        .is_server_error());

    assert_eq!(
        *recorder.calls.lock().unwrap(),
        [("responses".to_string(), Some(36), Some(87))]
    );
    assert_eq!(*recorder.errors.lock().unwrap(), ["responses"]);
}

#[tokio::test]
async fn stored_responses_can_be_retrieved_cancelled_and_deleted() {
    let mut server = Server::new_async().await;
    let retrieve = server
        .mock("GET", "/responses/resp_123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body().to_string())
        .create_async()
        .await;
    let mut cancelled = response_body();
    cancelled["status"] = json!("cancelled");
    let cancel = server
        .mock("POST", "/responses/resp_123/cancel")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(cancelled.to_string())
        .create_async()
        .await;
    let delete = server
        .mock("DELETE", "/responses/resp_123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"resp_123","object":"response","deleted":true}"#)
        .create_async()
        .await;

    let client = client_for(&server);
    let responses = client.responses_api();
    assert_eq!(
        responses.retrieve("resp_123").await.unwrap().id(),
        "resp_123"
    );
    assert_eq!(
        responses.cancel("resp_123").await.unwrap().status(),
        Some(openai_client_base::models::response::Status::Cancelled)
    );
    assert!(responses.delete("resp_123").await.unwrap());
    retrieve.assert_async().await;
    cancel.assert_async().await;
    delete.assert_async().await;
}
