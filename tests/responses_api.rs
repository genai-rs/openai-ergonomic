//! The Responses API client (`/responses`) against openai-mock-server: request shape, response
//! parsing, streaming events and error mapping. The mock decodes each request with the types of the
//! spec, so a request the spec does not describe fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]
#![allow(
    clippy::significant_drop_tightening,
    clippy::type_complexity,
    clippy::format_collect
)]

use futures::StreamExt;
use openai_ergonomic::interceptor::{AfterResponseContext, ErrorContext};
use openai_ergonomic::responses_api::{OutputItem, ResponsesEvent};
use openai_ergonomic::{Client, Config, Error, Interceptor, Result};
use openai_mock_server::server::responses_api::CreateResponseRequest;
use openai_mock_server::{op, MockServer, Reply, Stream};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

fn config_for(mock: &MockServer) -> Config {
    Config::builder()
        .api_key("test-key")
        .api_base(mock.base_url())
        .default_model("gpt-4.1-mini")
        .max_retries(0)
        .build()
}

fn client_for(mock: &MockServer) -> Client {
    Client::builder(config_for(mock)).unwrap().build()
}

/// The bodies of the `/responses` requests the mock received, as JSON, in order.
fn sent(mock: &MockServer) -> Vec<Value> {
    assert!(mock.rejected().is_empty(), "{:?}", mock.rejected());
    mock.requests::<CreateResponseRequest>()
        .into_iter()
        .map(|request| serde_json::to_value(request.body).unwrap())
        .collect()
}

/// Every field of `expected` is in `actual`, with the same value (objects compared field by field,
/// arrays item by item).
fn assert_contains(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            for (key, value) in expected {
                let found = actual.get(key).unwrap_or(&Value::Null);
                assert_contains(found, value);
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
            for (actual, expected) in actual.iter().zip(expected) {
                assert_contains(actual, expected);
            }
        }
        _ => assert_eq!(actual, expected),
    }
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
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .reply(Reply::json(response_body()));

    let client = client_for(&mock);
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
    assert_eq!(
        sent(&mock),
        [json!({
            "model": "gpt-4.1-mini",
            "instructions": "Be brief",
            "input": [{"type": "message", "role": "user", "content": "Weather in Ghent?"}],
            "store": false,
            "max_output_tokens": 64
        })]
    );
    assert_eq!(
        mock.requests::<CreateResponseRequest>()[0].headers["authorization"],
        "Bearer test-key"
    );

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
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .reply(Reply::json(response_body()));

    let client = client_for(&mock);
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
    assert_contains(
        &sent(&mock)[0],
        &json!({
            "tools": [
                {"type": "function", "name": "lookup", "strict": null},
                {"type": "function", "name": "exact", "strict": true}
            ]
        }),
    );
}

#[tokio::test]
async fn function_call_outputs_and_earlier_output_are_sent_back_as_input() {
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .always(Reply::json(response_body()));

    let client = client_for(&mock);
    let response = client
        .send_responses(client.responses().input_text("Weather in Ghent?"))
        .await
        .unwrap();

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
    let sent = sent(&mock);
    assert_contains(&sent[0], &json!({"input": "Weather in Ghent?"}));
    assert_contains(
        &sent[1],
        &json!({
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
        }),
    );
}

#[tokio::test]
async fn rate_limits_and_api_errors_map_to_error_api() {
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE).always(
        Reply::json(json!({"error": {
            "message": "Rate limit reached",
            "type": "requests",
            "code": "rate_limit_exceeded"
        }}))
        .status(429),
    );

    let client = client_for(&mock);
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

    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE).reply(
        Reply::json(json!({"error": {
            "message": "Unknown parameter",
            "type": "invalid_request_error",
            "code": "unknown_parameter"
        }}))
        .status(400),
    );
    let client = client_for(&mock);
    let error = client
        .send_responses(client.responses().user("Hi"))
        .await
        .unwrap_err();
    assert!(error.is_client_error());
    assert!(!error.is_retryable());
    assert!(matches!(error, Error::Api { status: 400, .. }));
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
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .reply(Reply::stream(Stream::new(stream_events())));

    let client = client_for(&mock);
    let events: Vec<ResponsesEvent> = client
        .send_responses_stream(client.responses().user("Hi"))
        .await
        .unwrap()
        .map(|event| event.unwrap())
        .collect()
        .await;
    assert_eq!(
        sent(&mock),
        [json!({
            "model": "gpt-4.1-mini",
            "input": [{"type": "message", "role": "user", "content": "Hi"}],
            "stream": true
        })]
    );
    assert_eq!(
        mock.requests::<CreateResponseRequest>()[0].headers["accept"],
        "text/event-stream"
    );

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
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE).reply(Reply::stream(
        Stream::new(stream_events().into_iter().take(3)).without_done(),
    ));

    let client = client_for(&mock);
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

#[tokio::test]
async fn a_stream_with_keep_alives_reads_the_same_events() {
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .reply(Reply::stream(Stream::new(stream_events()).keep_alive()));

    let client = client_for(&mock);
    let events: Vec<ResponsesEvent> = client
        .send_responses_stream(client.responses().user("Hi"))
        .await
        .unwrap()
        .map(|event| event.unwrap())
        .collect()
        .await;
    assert_eq!(events.len(), 6);
    assert!(events[5].is_terminal());
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
    let mock = MockServer::start().await;
    mock.on(op::CREATE_RESPONSE)
        .reply(Reply::json(response_body()))
        .reply(Reply::json(json!({})).status(500));

    let recorder = Arc::new(Recorder::default());
    let client = Client::builder(config_for(&mock))
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
    let mock = MockServer::start().await;
    let mut cancelled = response_body();
    cancelled["status"] = json!("cancelled");
    mock.on(op::GET_RESPONSE)
        .reply(Reply::json(response_body()));
    mock.on(op::CANCEL_RESPONSE).reply(Reply::json(cancelled));
    mock.on(op::DELETE_RESPONSE).reply(Reply::json(
        json!({"id": "resp_123", "object": "response", "deleted": true}),
    ));

    let client = client_for(&mock);
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
    for operation in [op::GET_RESPONSE, op::CANCEL_RESPONSE, op::DELETE_RESPONSE] {
        assert_eq!(mock.calls(operation), 1, "{operation}");
    }
}
