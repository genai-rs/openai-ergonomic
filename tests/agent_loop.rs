//! The agent loop (`ToolLoop`) against a scripted HTTP service: both APIs, streaming with tool
//! calls started before the response ends, ordered parallel dispatch, history repair, max
//! iterations, cancellation, whole-request retries (with `retry-after`), deferred tools and tool
//! search, and the rate limits of every call.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]
#![allow(
    clippy::significant_drop_tightening,
    clippy::type_complexity,
    clippy::unnecessary_literal_bound,
    clippy::map_unwrap_or
)]

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use openai_ergonomic::{
    CancellationSignal, ChatCompletionBuilder, Client, Config, Conversation, Error, Exposure,
    FunctionTool, LoopConfig, LoopEvent, LoopStop, ModelCaps, PromptCacheKey, ResponsesBuilder,
    Result, RetryConfig, ToolKey, ToolLoop, ToolRegistry,
};
use serde_json::{json, Value};
use timvw_rust_runtime::{Body, BoxError};

// ---------------------------------------------------------------------------------------------
// A scripted HTTP service.

/// A reply: status, headers and body chunks, each sent after its delay.
struct Reply {
    status: u16,
    headers: Vec<(&'static str, String)>,
    chunks: Vec<(Duration, String)>,
}

impl Reply {
    fn json(body: &Value) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type", "application/json".into())],
            chunks: vec![(Duration::ZERO, body.to_string())],
        }
    }

    fn sse(events: &[Value]) -> Self {
        Self::slow_sse(
            events
                .iter()
                .map(|event| (Duration::ZERO, event.clone()))
                .collect(),
        )
    }

    /// Server-sent events, each after its delay.
    fn slow_sse(events: Vec<(Duration, Value)>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type", "text/event-stream".into())],
            chunks: events
                .into_iter()
                .map(|(delay, event)| {
                    (
                        delay,
                        format!(
                            "event: {}\ndata: {event}\n\n",
                            event["type"].as_str().unwrap()
                        ),
                    )
                })
                .collect(),
        }
    }

    fn error(status: u16, code: &str) -> Self {
        Self {
            status,
            headers: vec![("content-type", "application/json".into())],
            chunks: vec![(
                Duration::ZERO,
                json!({"error": {"message": "nope", "type": code, "code": code}}).to_string(),
            )],
        }
    }

    fn header(mut self, name: &'static str, value: &str) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }
}

type Seen = Arc<Mutex<Vec<Value>>>;

/// A client whose HTTP service answers request `n` with `script(n, body)` and records the JSON
/// bodies. The client does not retry: the loop does.
fn scripted(script: impl Fn(usize, &Value) -> Reply + Send + Sync + 'static) -> (Client, Seen) {
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let script = Arc::new(script);
    let service = tower::service_fn(move |request: http::Request<Body>| {
        let log = log.clone();
        let script = script.clone();
        async move {
            let body = request.into_body().collect_bytes().await?;
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let index = {
                let mut log = log.lock().unwrap();
                log.push(body.clone());
                log.len() - 1
            };
            let reply = script(index, &body);
            let chunks = futures::stream::iter(reply.chunks).then(|(delay, chunk)| async move {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                Ok::<_, Infallible>(Bytes::from(chunk))
            });
            let mut response = http::Response::builder().status(reply.status);
            for (name, value) in reply.headers {
                response = response.header(name, value);
            }
            Ok::<_, BoxError>(response.body(Body::wrap_stream(chunks))?)
        }
    });
    let config = Config::builder()
        .api_key("test-key")
        .api_base("https://api.example.test/v1")
        .default_model("gpt-4.1-mini")
        .max_retries(0)
        .http_client(timvw_rust_runtime::Client::from_service(service))
        .build();
    (Client::builder(config).unwrap().build(), seen)
}

use futures::StreamExt;

// ---------------------------------------------------------------------------------------------
// Responses API payloads.

fn response(id: &str, output: &[Value]) -> Value {
    json!({
        "id": id,
        "object": "response",
        "created_at": 1_741_476_542,
        "status": "completed",
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "max_output_tokens": null,
        "model": "gpt-4.1-mini",
        "output": output,
        "parallel_tool_calls": true,
        "previous_response_id": null,
        "reasoning": {"effort": null, "summary": null},
        "store": false,
        "temperature": 1.0,
        "text": {"format": {"type": "text"}},
        "tool_choice": "auto",
        "tools": [],
        "top_p": 1.0,
        "truncation": "disabled",
        "usage": {
            "input_tokens": 10,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens": 5,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 15
        },
        "user": null,
        "metadata": {}
    })
}

fn call(call_id: &str, name: &str, arguments: &Value) -> Value {
    json!({
        "type": "function_call",
        "id": format!("fc_{call_id}"),
        "call_id": call_id,
        "name": name,
        "arguments": arguments.to_string(),
        "status": "completed"
    })
}

fn message(text: &str) -> Value {
    json!({
        "type": "message",
        "id": "msg_1",
        "status": "completed",
        "role": "assistant",
        "content": [{"type": "output_text", "text": text, "annotations": []}]
    })
}

/// The events of a streamed response: `response.created`, one `response.output_item.done` per
/// item (with a text delta before a message) and `response.completed`.
fn events(id: &str, output: &[Value]) -> Vec<Value> {
    let mut created = response(id, &[]);
    created["status"] = json!("in_progress");
    let mut events =
        vec![json!({"type": "response.created", "sequence_number": 0, "response": created})];
    for (index, item) in output.iter().enumerate() {
        if item["type"] == "message" {
            events.push(json!({
                "type": "response.output_text.delta", "sequence_number": 1, "item_id": "msg_1",
                "output_index": index, "content_index": 0,
                "delta": item["content"][0]["text"]
            }));
        }
        events.push(json!({
            "type": "response.output_item.done", "sequence_number": 2, "output_index": index,
            "item": item
        }));
    }
    events.push(json!({"type": "response.completed", "sequence_number": 3, "response": response(id, output)}));
    events
}

fn chat_completion(content: Option<&str>, calls: &[(&str, &str, Value)]) -> Value {
    let tool_calls: Vec<Value> = calls
        .iter()
        .map(|(id, name, arguments)| {
            json!({"id": id, "type": "function", "function": {"name": name, "arguments": arguments.to_string()}})
        })
        .collect();
    let mut message = json!({"role": "assistant", "content": content});
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 1_700_000_000,
        "model": "gpt-4.1-mini",
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": if calls.is_empty() { "stop" } else { "tool_calls" },
            "logprobs": null
        }],
        "usage": {"prompt_tokens": 9, "completion_tokens": 4, "total_tokens": 13}
    })
}

/// The `input` items of a recorded Responses request of these types.
fn input_of(body: &Value, kind: &str) -> Vec<Value> {
    body["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == kind)
        .cloned()
        .collect()
}

fn tool_names(body: &Value) -> Vec<String> {
    body["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .map(|tool| {
                    tool["name"]
                        .as_str()
                        .unwrap_or_else(|| tool["type"].as_str().unwrap())
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Tools.

type Log = Arc<Mutex<Vec<(String, &'static str, Instant)>>>;

/// Sleeps, then returns its name; logs its start and end.
struct Sleepy {
    name: &'static str,
    millis: u64,
    parallel: bool,
    exposure: Exposure,
    log: Log,
}

impl Sleepy {
    fn new(name: &'static str, millis: u64, parallel: bool, log: &Log) -> Self {
        Self {
            name,
            millis,
            parallel,
            exposure: Exposure::Direct,
            log: log.clone(),
        }
    }
}

impl FunctionTool for Sleepy {
    type Input = Value;
    type Output = String;
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "Sleep for a while, then say your name."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _input: Value) -> Result<String> {
        self.log
            .lock()
            .unwrap()
            .push((self.name.into(), "start", Instant::now()));
        tokio::time::sleep(Duration::from_millis(self.millis)).await;
        self.log
            .lock()
            .unwrap()
            .push((self.name.into(), "end", Instant::now()));
        Ok(self.name.to_string())
    }
    fn supports_parallel(&self) -> bool {
        self.parallel
    }
    fn exposure(&self) -> Exposure {
        self.exposure
    }
}

struct Weather;

impl FunctionTool for Weather {
    type Input = Value;
    type Output = Value;
    fn name(&self) -> &str {
        "get_weather"
    }
    fn description(&self) -> &str {
        "Get the current weather of a city."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]})
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        Ok(json!({"city": input["city"], "temperature": 12}))
    }
}

fn weather_tools() -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    tools.register(Weather).unwrap();
    tools
}

fn template() -> ResponsesBuilder {
    ResponsesBuilder::new("gpt-4.1-mini").instructions("Be brief.")
}

fn when(log: &Log, name: &str, what: &str) -> Instant {
    log.lock()
        .unwrap()
        .iter()
        .find(|(tool, kind, _)| tool == name && *kind == what)
        .map(|(_, _, at)| *at)
        .unwrap_or_else(|| panic!("{name} has no {what}"))
}

// ---------------------------------------------------------------------------------------------
// Tests.

#[tokio::test]
async fn responses_loop_runs_tools_until_the_model_answers() {
    for stream in [true, false] {
        let (client, seen) = scripted(move |index, _| {
            let output = match index {
                0 => vec![call("call_1", "get_weather", &json!({"city": "Ghent"}))],
                _ => vec![message("12 degrees in Ghent.")],
            };
            let reply = if stream {
                Reply::sse(&events(&format!("resp_{index}"), &output))
            } else {
                Reply::json(&response(&format!("resp_{index}"), &output))
            };
            reply
                .header("x-ratelimit-remaining-requests", &(99 - index).to_string())
                .header("x-ratelimit-limit-requests", "100")
        });
        let mut agent = ToolLoop::responses(&client, weather_tools(), template())
            .with_config(LoopConfig::new().with_stream(stream));
        let mut conversation = Conversation::user("Weather in Ghent?");
        let outcome = agent.run(&mut conversation).await.unwrap();

        assert_eq!(outcome.stop, LoopStop::Completed);
        assert_eq!(outcome.output_text(), "12 degrees in Ghent.");
        assert_eq!(outcome.turns.len(), 2);
        assert_eq!(outcome.turns[0].calls[0].name, "get_weather");
        assert_eq!(outcome.turns[0].response_id.as_deref(), Some("resp_0"));
        // The rate limits of each call, and the client's latest.
        let first = outcome.turns[0].rate_limits.unwrap().requests.unwrap();
        assert_eq!(first.remaining, Some(99));
        let second = outcome.rate_limits().unwrap().requests.unwrap();
        assert_eq!(second.remaining, Some(98));
        assert_eq!(
            client.rate_limits().unwrap().requests.unwrap().remaining,
            Some(98)
        );

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        // Stateless defaults, a stable cache key, the template's options.
        for body in seen.iter() {
            assert_eq!(body["store"], false);
            assert_eq!(body["instructions"], "Be brief.");
            assert_eq!(body["parallel_tool_calls"], true);
            assert_eq!(body["prompt_cache_key"], agent.prompt_cache_key().unwrap());
            assert!(
                body.get("include").is_none(),
                "no reasoning: nothing to include"
            );
            assert!(body.get("previous_response_id").is_none());
            assert_eq!(tool_names(body), ["get_weather"]);
        }
        assert_eq!(seen[0]["stream"] == json!(true), stream);
        // The second request replays the call (without its id) and its output.
        let calls = input_of(&seen[1], "function_call");
        assert_eq!(calls.len(), 1);
        assert!(calls[0].get("id").is_none());
        let outputs = input_of(&seen[1], "function_call_output");
        assert_eq!(outputs[0]["call_id"], "call_1");
        assert_eq!(outputs[0]["output"], r#"{"city":"Ghent","temperature":12}"#);
        // The history holds everything, in order.
        let types: Vec<String> = conversation
            .items()
            .iter()
            .map(|item| {
                serde_json::to_value(item).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            types,
            [
                "message",
                "function_call",
                "function_call_output",
                "message"
            ]
        );
    }
}

#[tokio::test]
async fn chat_loop_runs_tools_until_the_model_answers() {
    let (client, seen) = scripted(|index, _| {
        let body = match index {
            0 => chat_completion(
                Some("Checking."),
                &[("call_1", "get_weather", json!({"city": "Ghent"}))],
            ),
            _ => chat_completion(Some("12 degrees."), &[]),
        };
        Reply::json(&body).header("x-ratelimit-remaining-tokens", "5000")
    });
    let template = ChatCompletionBuilder::new("gpt-4.1-mini").system("Be brief.");
    let mut agent = ToolLoop::chat(&client, weather_tools(), template)
        .with_config(LoopConfig::new().with_prompt_cache_key(PromptCacheKey::Custom("k".into())));
    let mut conversation = Conversation::user("Weather in Ghent?");
    let outcome = agent.run(&mut conversation).await.unwrap();

    assert_eq!(outcome.stop, LoopStop::Completed);
    assert_eq!(outcome.output_text(), "12 degrees.");
    assert_eq!(outcome.turns[0].text, "Checking.");
    assert_eq!(
        outcome.turns[1]
            .rate_limits
            .unwrap()
            .tokens
            .unwrap()
            .remaining,
        Some(5000)
    );
    assert_eq!(outcome.turns[1].input_tokens, Some(9));

    let seen = seen.lock().unwrap();
    let messages = seen[1]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(
        messages[0],
        json!({"role": "system", "content": "Be brief."})
    );
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[2]["role"], "assistant");
    assert_eq!(messages[2]["content"], "Checking.");
    assert_eq!(messages[2]["tool_calls"][0]["id"], "call_1");
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["tool_call_id"], "call_1");
    assert_eq!(seen[1]["prompt_cache_key"], "k");
    assert_eq!(seen[1]["parallel_tool_calls"], true);
    assert_eq!(seen[1]["tools"][0]["function"]["name"], "get_weather");
}

#[tokio::test]
async fn parallel_calls_overlap_and_answers_keep_the_call_order() {
    let log: Log = Arc::default();
    let mut tools = ToolRegistry::new();
    tools
        .register(Sleepy::new("slow", 150, true, &log))
        .unwrap();
    tools.register(Sleepy::new("fast", 20, true, &log)).unwrap();
    tools
        .register(Sleepy::new("alone", 20, false, &log))
        .unwrap();
    tools
        .register(Sleepy::new("after", 20, true, &log))
        .unwrap();
    let (client, seen) = scripted(|index, _| {
        let output = match index {
            0 => vec![
                call("a", "slow", &json!({})),
                call("b", "fast", &json!({})),
                call("c", "alone", &json!({})),
                call("d", "after", &json!({})),
            ],
            _ => vec![message("done")],
        };
        Reply::sse(&events(&format!("resp_{index}"), &output))
    });
    let mut agent = ToolLoop::responses(&client, tools, template());
    let mut answered = Vec::new();
    let outcome = agent
        .run_with(
            &mut Conversation::user("go"),
            &CancellationSignal::new(),
            |event| {
                if let LoopEvent::ToolAnswered(answer) = event {
                    answered.push(answer.call_id.clone());
                }
            },
        )
        .await
        .unwrap();
    assert!(outcome.is_completed());

    // slow and fast ran together; alone after both; after once alone was done.
    assert!(when(&log, "fast", "start") < when(&log, "slow", "end"));
    assert!(when(&log, "alone", "start") >= when(&log, "slow", "end"));
    assert!(when(&log, "alone", "start") >= when(&log, "fast", "end"));
    assert!(when(&log, "after", "start") >= when(&log, "alone", "end"));
    // Answered as they finished, recorded in call order.
    assert_eq!(answered, ["b", "a", "c", "d"]);
    let seen = seen.lock().unwrap();
    let ids: Vec<&str> = input_of(&seen[1], "function_call_output")
        .iter()
        .map(|output| output["call_id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>()
        .iter()
        .map(|id| match id.as_str() {
            "a" => "a",
            "b" => "b",
            "c" => "c",
            _ => "d",
        })
        .collect();
    assert_eq!(ids, ["a", "b", "c", "d"]);
}

#[tokio::test]
async fn without_parallel_tool_calls_every_call_runs_alone() {
    let log: Log = Arc::default();
    let mut tools = ToolRegistry::new();
    tools.register(Sleepy::new("one", 40, true, &log)).unwrap();
    tools.register(Sleepy::new("two", 40, true, &log)).unwrap();
    let (client, seen) = scripted(|index, _| {
        let output = match index {
            0 => vec![call("a", "one", &json!({})), call("b", "two", &json!({}))],
            _ => vec![message("done")],
        };
        Reply::sse(&events("resp", &output))
    });
    let mut agent = ToolLoop::responses(&client, tools, template())
        .with_config(LoopConfig::new().with_parallel_tool_calls(false));
    agent.run(&mut Conversation::user("go")).await.unwrap();
    assert!(when(&log, "two", "start") >= when(&log, "one", "end"));
    assert_eq!(seen.lock().unwrap()[0]["parallel_tool_calls"], false);
}

#[tokio::test]
async fn streamed_calls_start_before_the_response_ends() {
    let log: Log = Arc::default();
    let mut tools = ToolRegistry::new();
    tools
        .register(Sleepy::new("probe", 10, true, &log))
        .unwrap();
    let completed_at: Arc<Mutex<Option<Instant>>> = Arc::default();
    let (client, _) = scripted(|index, _| {
        if index > 0 {
            return Reply::sse(&events("resp_1", &[message("ok")]));
        }
        let output = [call("a", "probe", &json!({})), message("still writing")];
        let mut timed: Vec<(Duration, Value)> = events("resp_0", &output)
            .into_iter()
            .map(|event| (Duration::ZERO, event))
            .collect();
        // The response goes on for a while after the call is done.
        let last = timed.len() - 1;
        timed[last].0 = Duration::from_millis(300);
        Reply::slow_sse(timed)
    });
    let mut agent = ToolLoop::responses(&client, tools, template());
    let completed = completed_at.clone();
    let outcome = agent
        .run_with(
            &mut Conversation::user("go"),
            &CancellationSignal::new(),
            |event| {
                if let LoopEvent::Model(event) = event {
                    if event.event_type() == "response.completed"
                        && completed.lock().unwrap().is_none()
                    {
                        *completed.lock().unwrap() = Some(Instant::now());
                    }
                }
            },
        )
        .await
        .unwrap();
    assert!(outcome.is_completed());
    let completed = completed_at.lock().unwrap().unwrap();
    // The tool finished well before the response did.
    assert!(when(&log, "probe", "end") + Duration::from_millis(150) < completed);
}

#[tokio::test]
async fn max_iterations_stops_with_every_call_answered() {
    let (client, seen) = scripted(|index, _| {
        Reply::sse(&events(
            "resp",
            &[call(
                &format!("call_{index}"),
                "get_weather",
                &json!({"city": "Ghent"}),
            )],
        ))
    });
    let mut agent = ToolLoop::responses(&client, weather_tools(), template())
        .with_config(LoopConfig::new().with_max_iterations(2));
    let mut conversation = Conversation::user("Loop forever");
    let outcome = agent.run(&mut conversation).await.unwrap();
    assert_eq!(outcome.stop, LoopStop::MaxIterations);
    assert_eq!(outcome.turns.len(), 2);
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(
        conversation.unanswered_calls(),
        Vec::<openai_ergonomic::ToolInvocation>::new()
    );
    assert_eq!(conversation.calls().len(), 2);
    // A new run continues where it stopped.
    let outcome = agent.run(&mut conversation).await.unwrap();
    assert_eq!(outcome.stop, LoopStop::MaxIterations);
    assert_eq!(conversation.calls().len(), 4);
}

#[tokio::test]
async fn cancellation_aborts_running_calls_and_leaves_a_valid_history() {
    let log: Log = Arc::default();
    let mut tools = ToolRegistry::new();
    tools.register(Sleepy::new("quick", 5, true, &log)).unwrap();
    tools
        .register(Sleepy::new("forever", 60_000, true, &log))
        .unwrap();
    tools
        .register(Sleepy::new("waiting", 5, false, &log))
        .unwrap();
    let (client, seen) = scripted(|_, _| {
        Reply::sse(&events(
            "resp",
            &[
                call("a", "quick", &json!({})),
                call("b", "forever", &json!({})),
                call("c", "waiting", &json!({})),
            ],
        ))
    });
    let mut agent = ToolLoop::responses(&client, tools, template());
    let cancel = CancellationSignal::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        trigger.cancel();
    });
    let mut conversation = Conversation::user("go");
    let started = Instant::now();
    let outcome = agent
        .run_with(&mut conversation, &cancel, |_| {})
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(outcome.stop, LoopStop::Cancelled);
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(
        conversation.unanswered_calls(),
        Vec::<openai_ergonomic::ToolInvocation>::new()
    );
    let outputs: Vec<Value> = conversation
        .items()
        .iter()
        .map(|item| serde_json::to_value(item).unwrap())
        .filter(|item| item["type"] == "function_call_output")
        .collect();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0]["output"], "\"quick\"");
    assert!(outputs[1]["output"]
        .as_str()
        .unwrap()
        .starts_with("aborted by user after 0."));
    assert_eq!(outputs[2]["output"], "aborted");
    // Cancelled before the start: no request at all.
    let outcome = agent
        .run_with(&mut conversation, &cancel, |_| {})
        .await
        .unwrap();
    assert_eq!(outcome.stop, LoopStop::Cancelled);
    assert_eq!(outcome.turns, Vec::<openai_ergonomic::TurnRecord>::new());
}

#[tokio::test]
async fn the_history_is_repaired_before_the_request() {
    let (client, seen) = scripted(|_, _| Reply::sse(&events("resp", &[message("ok")])));
    let mut conversation = Conversation::user("hi");
    conversation.push_call(&openai_ergonomic::ToolInvocation::function(
        "lost",
        "get_weather",
        "{}",
    ));
    conversation.push_answer(&openai_ergonomic::CallAnswer::new(
        &openai_ergonomic::ToolInvocation::function("ghost", "get_weather", "{}"),
        "orphan",
        true,
    ));
    let mut agent = ToolLoop::responses(&client, weather_tools(), template());
    agent.run(&mut conversation).await.unwrap();
    let seen = seen.lock().unwrap();
    let outputs = input_of(&seen[0], "function_call_output");
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["call_id"], "lost");
    assert_eq!(outputs[0]["output"], "aborted");
}

/// With the default HTTP client, which retries a rate limit itself, the loop does not retry it
/// again: `max_retries(1)` gives two requests in all, not two for each of the loop's attempts.
#[tokio::test]
async fn the_loop_leaves_failures_before_the_body_to_the_clients_retries() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/responses")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("retry-after-ms", "1")
        .with_body(
            json!({"error": {"message": "slow down", "type": "requests", "code": "rate_limit_exceeded"}})
                .to_string(),
        )
        .expect(2)
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4.1-mini")
        .max_retries(1)
        .build();
    let client = Client::builder(config).unwrap().build();
    let mut agent = ToolLoop::responses(&client, weather_tools(), template())
        .with_config(LoopConfig::new().with_retry(RetryConfig::new(5)));
    let mut retries = 0;
    let error = agent
        .run_with(
            &mut Conversation::user("hi"),
            &CancellationSignal::new(),
            |event| {
                if matches!(event, LoopEvent::Retrying { .. }) {
                    retries += 1;
                }
            },
        )
        .await
        .unwrap_err();
    assert!(error.is_rate_limit(), "{error}");
    assert_eq!(retries, 0);
    mock.assert_async().await;
    drop(server);
}

#[tokio::test]
async fn a_rate_limited_request_is_retried_after_retry_after() {
    let (client, seen) = scripted(|index, _| match index {
        0 => Reply::error(429, "rate_limit_exceeded").header("retry-after-ms", "120"),
        _ => Reply::sse(&events("resp", &[message("ok")]))
            .header("x-ratelimit-remaining-requests", "7"),
    });
    let mut agent = ToolLoop::responses(&client, weather_tools(), template());
    let mut delays = Vec::new();
    let started = Instant::now();
    let outcome = agent
        .run_with(
            &mut Conversation::user("hi"),
            &CancellationSignal::new(),
            |event| {
                if let LoopEvent::Retrying {
                    delay,
                    error,
                    attempt,
                } = event
                {
                    assert!(error.is_rate_limit());
                    delays.push((attempt, delay));
                }
            },
        )
        .await
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(120));
    assert_eq!(delays, [(1, Duration::from_millis(120))]);
    assert!(outcome.is_completed());
    assert_eq!(outcome.turns[0].retries, 1);
    assert_eq!(
        outcome.turns[0]
            .rate_limits
            .unwrap()
            .requests
            .unwrap()
            .remaining,
        Some(7)
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_cut_off_stream_is_retried_with_backoff_without_running_calls_twice() {
    let log: Log = Arc::default();
    let mut tools = ToolRegistry::new();
    tools.register(Sleepy::new("once", 5, true, &log)).unwrap();
    let (client, seen) = scripted(|index, _| match index {
        // The call is done, then the stream stops without a terminal event.
        0 => {
            let mut cut = events("resp_0", &[call("a", "once", &json!({}))]);
            cut.pop();
            Reply::sse(&cut)
        }
        _ => Reply::sse(&events("resp_1", &[message("done")])),
    });
    let retry = RetryConfig::new(2).with_base_delay(Duration::from_millis(10));
    let mut agent = ToolLoop::responses(&client, tools, template())
        .with_config(LoopConfig::new().with_retry(retry));
    let mut delays = Vec::new();
    let outcome = agent
        .run_with(
            &mut Conversation::user("hi"),
            &CancellationSignal::new(),
            |event| {
                if let LoopEvent::Retrying { delay, error, .. } = event {
                    assert!(matches!(error, Error::StreamConnection { .. }));
                    delays.push(delay);
                }
            },
        )
        .await
        .unwrap();
    assert!(outcome.is_completed());
    assert_eq!(delays.len(), 1);
    assert!(delays[0] >= Duration::from_millis(9) && delays[0] <= Duration::from_millis(11));
    // The call ran once; the retry carries it with its output.
    let starts = log
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, kind, _)| *kind == "start")
        .count();
    assert_eq!(starts, 1);
    let seen = seen.lock().unwrap();
    assert_eq!(
        input_of(&seen[1], "function_call_output")[0]["output"],
        "\"once\""
    );
}

#[tokio::test]
async fn retries_stop_at_errors_that_cannot_be_retried_and_at_the_limit() {
    let (client, seen) = scripted(|_, _| Reply::error(400, "invalid_request_error"));
    let mut agent = ToolLoop::responses(&client, weather_tools(), template());
    let error = agent.run(&mut Conversation::user("hi")).await.unwrap_err();
    assert!(matches!(error, Error::Api { status: 400, .. }));
    assert_eq!(seen.lock().unwrap().len(), 1);

    let (client, seen) = scripted(|_, _| Reply::error(500, "server_error"));
    let retry = RetryConfig::new(2).with_base_delay(Duration::from_millis(1));
    let mut agent = ToolLoop::responses(&client, weather_tools(), template())
        .with_config(LoopConfig::new().with_retry(retry));
    let error = agent.run(&mut Conversation::user("hi")).await.unwrap_err();
    assert!(error.is_server_error());
    assert_eq!(seen.lock().unwrap().len(), 3);

    // A failed response (response.failed) is retried by its code.
    let (client, seen) = scripted(|index, _| {
        if index == 0 {
            let mut failed = response("resp_0", &[]);
            failed["status"] = json!("failed");
            failed["error"] = json!({"code": "server_error", "message": "boom"});
            Reply::sse(&[
                json!({"type": "response.failed", "sequence_number": 0, "response": failed}),
            ])
        } else {
            Reply::sse(&events("resp_1", &[message("ok")]))
        }
    });
    let retry = RetryConfig::new(1).with_base_delay(Duration::from_millis(1));
    let mut agent = ToolLoop::responses(&client, weather_tools(), template())
        .with_config(LoopConfig::new().with_retry(retry));
    assert!(agent
        .run(&mut Conversation::user("hi"))
        .await
        .unwrap()
        .is_completed());
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_fatal_tool_error_ends_the_run() {
    struct Fatal;
    impl FunctionTool for Fatal {
        type Input = Value;
        type Output = String;
        fn name(&self) -> &str {
            "explode"
        }
        fn description(&self) -> &str {
            "Fails fatally."
        }
        fn parameters_schema(&self) -> Value {
            json!({"type": "object"})
        }
        async fn execute(&self, _: Value) -> Result<String> {
            Err(Error::fatal(std::io::Error::other("disk on fire")))
        }
    }
    let mut tools = weather_tools();
    tools.register(Fatal).unwrap();
    let (client, _) = scripted(|_, _| {
        Reply::json(&response(
            "resp",
            &[
                call("a", "get_weather", &json!({"city": "Ghent"})),
                call("b", "explode", &json!({})),
            ],
        ))
    });
    let mut agent = ToolLoop::responses(&client, tools, template()).with_config(
        LoopConfig::new()
            .with_stream(false)
            .with_parallel_tool_calls(false),
    );
    let mut conversation = Conversation::user("hi");
    let error = agent.run(&mut conversation).await.unwrap_err();
    assert!(matches!(error, Error::Tool(_)), "{error:?}");
    // The answer of the call that finished is kept; the other is answered by normalize.
    assert_eq!(conversation.unanswered_calls().len(), 1);
    let report = conversation.normalize(ModelCaps::default());
    assert_eq!(report.aborted, ["b"]);
}

#[tokio::test]
async fn deferred_tools_are_loaded_by_the_loops_tool_search() {
    let log: Log = Arc::default();
    let mut tools = weather_tools();
    let mut hidden = Sleepy::new("internal_audit", 1, true, &log);
    hidden.exposure = Exposure::Hidden;
    tools.register(hidden).unwrap();
    let mut deferred = Sleepy::new("convert_currency", 1, true, &log);
    deferred.exposure = Exposure::Deferred;
    tools.register(deferred).unwrap();
    let mut other = Sleepy::new("translate_text", 1, true, &log);
    other.exposure = Exposure::Deferred;
    tools.register(other).unwrap();

    for chat in [false, true] {
        let (client, seen) = scripted(move |index, _| {
            let calls: Vec<(&str, &str, Value)> = match index {
                0 => vec![("s", "tool_search", json!({"query": "currency conversion"}))],
                1 => vec![("c", "convert_currency", json!({}))],
                2 => vec![("h", "internal_audit", json!({}))],
                _ => vec![],
            };
            if chat {
                return Reply::json(&chat_completion(calls.is_empty().then_some("done"), &calls));
            }
            let mut output: Vec<Value> = calls
                .iter()
                .map(|(id, name, args)| call(id, name, args))
                .collect();
            if output.is_empty() {
                output.push(message("done"));
            }
            Reply::sse(&events("resp", &output))
        });
        let registry = Arc::new(std::mem::take(&mut tools));
        let mut agent = if chat {
            ToolLoop::chat(
                &client,
                registry.clone(),
                ChatCompletionBuilder::new("gpt-4.1-mini"),
            )
        } else {
            ToolLoop::responses(&client, registry.clone(), template())
        };
        let outcome = agent
            .run(&mut Conversation::user("Convert 10 EUR"))
            .await
            .unwrap();
        assert!(outcome.is_completed());
        assert_eq!(
            agent.loaded_tools().cloned().collect::<Vec<_>>(),
            [ToolKey::new("convert_currency")]
        );
        let seen = seen.lock().unwrap();
        let names = |body: &Value| -> Vec<String> {
            if chat {
                body["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|tool| tool["function"]["name"].as_str().unwrap().to_string())
                    .collect()
            } else {
                tool_names(body)
            }
        };
        assert_eq!(names(&seen[0]), ["get_weather", "tool_search"]);
        assert_eq!(
            names(&seen[1]),
            ["convert_currency", "get_weather", "tool_search"]
        );
        // The search's answer lists what it loaded.
        let search_output = if chat {
            seen[1]["messages"].as_array().unwrap().last().unwrap()["content"].clone()
        } else {
            input_of(&seen[1], "function_call_output")[0]["output"].clone()
        };
        let search_output: Value = serde_json::from_str(search_output.as_str().unwrap()).unwrap();
        assert_eq!(search_output["tools"][0]["name"], "convert_currency");
        // The hidden tool is never advertised, but its call is answered.
        assert!(!names(&seen[3]).contains(&"internal_audit".to_string()));
        drop(seen);
        drop(agent);
        tools = Arc::try_unwrap(registry).unwrap_or_else(|_| panic!("registry still shared"));
    }
}

#[tokio::test]
async fn a_model_that_searches_tools_gets_them_deferred() {
    let log: Log = Arc::default();
    let mut tools = weather_tools();
    let mut deferred = Sleepy::new("convert_currency", 1, true, &log);
    deferred.exposure = Exposure::Deferred;
    tools.register(deferred).unwrap();
    let (client, seen) = scripted(|index, _| {
        let output = match index {
            0 => vec![
                json!({"type": "tool_search_call", "id": "ts_1", "call_id": null, "execution": "server",
                       "arguments": {"query": "currency"}, "status": "completed"}),
                json!({"type": "tool_search_output", "id": "tso_1", "call_id": null, "execution": "server",
                       "tools": [{"type": "function", "name": "convert_currency", "description": "x",
                                  "parameters": {"type": "object", "properties": {}}, "strict": null,
                                  "defer_loading": true}],
                       "status": "completed"}),
                call("c", "convert_currency", &json!({})),
            ],
            _ => vec![message("done")],
        };
        Reply::sse(&events("resp", &output))
    });
    let mut agent = ToolLoop::responses(&client, tools, template())
        .with_config(LoopConfig::new().with_caps(ModelCaps::new().with_tool_search(true)));
    let outcome = agent.run(&mut Conversation::user("Convert")).await.unwrap();
    assert!(outcome.is_completed());
    assert_eq!(
        agent.loaded_tools().cloned().collect::<Vec<_>>(),
        [ToolKey::new("convert_currency")]
    );
    let seen = seen.lock().unwrap();
    let tools = seen[0]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
    let deferred = tools
        .iter()
        .find(|tool| tool["name"] == "convert_currency")
        .unwrap();
    assert_eq!(deferred["defer_loading"], true);
    assert!(tools.iter().any(|tool| tool["type"] == "tool_search"));
    // The search items are replayed, and the deferred call was answered.
    assert_eq!(input_of(&seen[1], "tool_search_output").len(), 1);
    assert_eq!(
        input_of(&seen[1], "function_call_output")[0]["output"],
        "\"convert_currency\""
    );
}

#[tokio::test]
async fn reasoning_is_replayed_encrypted_when_nothing_is_stored() {
    let (client, seen) = scripted(|index, _| {
        let output = match index {
            0 => vec![
                json!({"type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": "gAAAA"}),
                call("a", "get_weather", &json!({"city": "Ghent"})),
            ],
            _ => vec![message("done")],
        };
        Reply::json(&response("resp", &output))
    });
    let mut agent =
        ToolLoop::responses(&client, weather_tools(), template().reasoning_effort("low"))
            .with_config(LoopConfig::new().with_stream(false));
    agent.run(&mut Conversation::user("hi")).await.unwrap();
    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0]["include"], json!(["reasoning.encrypted_content"]));
        let reasoning = input_of(&seen[1], "reasoning");
        assert_eq!(reasoning[0]["encrypted_content"], "gAAAA");
        assert!(reasoning[0].get("id").is_none());
    }

    // Stored: no include (even for a reasoning model), the items keep their ids.
    let caps = ModelCaps::new().with_reasoning(true);
    let mut agent = ToolLoop::responses(&client, weather_tools(), template()).with_config(
        LoopConfig::new()
            .with_stream(false)
            .with_store(true)
            .with_caps(caps),
    );
    let mut conversation = Conversation::user("hi");
    conversation
        .record_output(&serde_json::from_value::<Vec<_>>(json!([message("earlier")])).unwrap());
    agent.run(&mut conversation).await.unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[2]["store"], true);
    assert!(seen[2].get("include").is_none());
    assert_eq!(seen[2]["input"][1]["id"], "msg_1");
}

#[tokio::test]
async fn plain_calls_carry_their_rate_limits() {
    let (client, _) = scripted(|index, _| {
        let reply = if index == 0 {
            Reply::json(&response("resp", &[message("hi")]))
        } else {
            Reply::json(&chat_completion(Some("hi"), &[]))
        };
        reply
            .header("x-ratelimit-remaining-requests", &(10 - index).to_string())
            .header("x-ratelimit-reset-requests", "1s")
    });
    let response = client
        .send_responses(client.responses().user("hi"))
        .await
        .unwrap();
    let limits = response.rate_limits().unwrap().requests.unwrap();
    assert_eq!(limits.remaining, Some(10));
    assert_eq!(limits.reset, Some(Duration::from_secs(1)));
    let chat = client.send_chat(client.chat().user("hi")).await.unwrap();
    assert_eq!(
        chat.rate_limits().unwrap().requests.unwrap().remaining,
        Some(9)
    );
    assert_eq!(
        client.rate_limits().unwrap().requests.unwrap().remaining,
        Some(9)
    );
}

#[test]
fn the_run_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}
    let (client, _) = {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _guard = runtime.enter();
        scripted(|_, _| Reply::json(&json!({})))
    };
    let mut agent = ToolLoop::responses(&client, ToolRegistry::new(), template());
    let mut conversation = Conversation::new();
    let cancel = CancellationSignal::new();
    let future = agent.run_with(&mut conversation, &cancel, |_| {});
    assert_send(&future);
}

#[tokio::test]
async fn an_incomplete_response_ends_the_run() {
    let incomplete = |id: &str, reason: &str, output: &[Value]| {
        let mut body = response(id, output);
        body["status"] = json!("incomplete");
        body["incomplete_details"] = json!({"reason": reason});
        Reply::sse(&[
            json!({"type": "response.incomplete", "sequence_number": 0, "response": body}),
        ])
    };
    let (client, seen) = scripted(move |index, _| match index {
        0 => Reply::sse(&events(
            "resp_0",
            &[call("a", "get_weather", &json!({"city": "Ghent"}))],
        )),
        _ => incomplete("resp_1", "max_output_tokens", &[message("Half an ans")]),
    });
    let mut agent = ToolLoop::responses(&client, weather_tools(), template());
    let mut conversation = Conversation::user("Write a long story");
    let outcome = agent.run(&mut conversation).await.unwrap();
    assert_eq!(
        outcome.stop,
        LoopStop::Incomplete {
            reason: Some("max_output_tokens".into())
        }
    );
    assert_eq!(outcome.turns.len(), 2);
    assert_eq!(outcome.output_text(), "Half an ans");
    assert_eq!(
        conversation.last_assistant_text().as_deref(),
        Some("Half an ans")
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn exposure_filters_the_registrys_definitions() {
    let log: Log = Arc::default();
    let mut tools = weather_tools();
    tools
        .register(Sleepy::new("convert_currency", 1, true, &log))
        .unwrap();
    tools
        .namespace("files", "File tools")
        .unwrap()
        .register(Sleepy::new("read_file", 1, true, &log))
        .unwrap();
    let key = ToolKey::new("convert_currency");
    assert_eq!(tools.exposure(&key), Some(Exposure::Direct));
    assert!(tools.set_exposure(&key, Exposure::Deferred));
    assert!(tools.set_exposure(&ToolKey::namespaced("files", "read_file"), Exposure::Hidden));
    assert!(!tools.set_exposure(&ToolKey::new("nope"), Exposure::Hidden));
    let chat: Vec<String> = tools
        .tool_definitions()
        .iter()
        .map(|tool| {
            serde_json::to_value(tool).unwrap()["function"]["name"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(chat, ["get_weather"]);
    // The namespace has no direct tool left, so it is not advertised either.
    assert_eq!(tools.responses_tools().len(), 1);
    assert_eq!(
        tools.search_deferred("currency", 8),
        std::slice::from_ref(&key)
    );
    assert!(
        tools.search_deferred("weather", 8).is_empty(),
        "direct tools are not searched"
    );
    assert_eq!(
        tools.description(&key),
        Some("Sleep for a while, then say your name.")
    );
}
