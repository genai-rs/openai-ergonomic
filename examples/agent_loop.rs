//! The agent loop, offline: a `ToolLoop` on the Responses API runs two tools the model calls
//! (at the same time, as they allow it), finds a deferred tool with its tool search, and stops
//! when the model answers. A scripted HTTP service replays the model's replies, so no API key is
//! required; `agent_loop_live` runs the same loop against the API.
//!
//! `cargo run --example agent_loop`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Plain `async fn` tools without `.await` are fine; newer Clippy's pedantic
// `unused_async_trait_impl` would flag them (`unknown_lints` keeps Rust 1.88 quiet).
#![allow(unknown_lints, clippy::unused_async_trait_impl)]

use std::sync::{Arc, Mutex};

use openai_ergonomic::{
    CancellationSignal, Client, Config, Conversation, Exposure, FunctionTool, LoopEvent,
    ResponsesBuilder, Result, ToolLoop, ToolRegistry,
};
use serde::Deserialize;
use serde_json::{json, Value};
use timvw_rust_runtime::{Body, BoxError};

#[derive(Deserialize)]
struct City {
    city: String,
}

/// The weather of a city; may run at the same time as other such tools.
struct Weather;

impl FunctionTool for Weather {
    type Input = City;
    type Output = Value;

    fn name(&self) -> &'static str {
        "get_weather"
    }
    fn description(&self) -> &'static str {
        "Current weather of a city"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]})
    }
    async fn execute(&self, input: City) -> Result<Value> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let temperature = if input.city == "Ghent" { 12 } else { 15 };
        Ok(json!({"city": input.city, "temperature_c": temperature}))
    }
    fn supports_parallel(&self) -> bool {
        true
    }
}

#[derive(Deserialize)]
struct Amount {
    celsius: f64,
}

/// A tool the model only sees once a tool search found it.
struct ToFahrenheit;

impl FunctionTool for ToFahrenheit {
    type Input = Amount;
    type Output = f64;

    fn name(&self) -> &'static str {
        "celsius_to_fahrenheit"
    }
    fn description(&self) -> &'static str {
        "Convert a temperature from Celsius to Fahrenheit"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"celsius": {"type": "number"}}, "required": ["celsius"]})
    }
    async fn execute(&self, input: Amount) -> Result<f64> {
        Ok(input.celsius.mul_add(9.0 / 5.0, 32.0))
    }
    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (client, requests) = scripted_client();

    let mut tools = ToolRegistry::new();
    tools.register(Weather)?;
    tools.register(ToFahrenheit)?;

    let template = ResponsesBuilder::new("gpt-4.1-mini")
        .instructions("You are a weather assistant. Answer in one sentence.");
    let mut agent = ToolLoop::responses(&client, tools, template);
    let mut conversation =
        Conversation::user("Is it warmer in Ghent or in Paris? Give both in Fahrenheit.");

    let outcome = agent
        .run_with(
            &mut conversation,
            &CancellationSignal::new(),
            |event| match event {
                LoopEvent::Request { iteration, attempt } => {
                    println!("request {iteration} (attempt {attempt})");
                }
                LoopEvent::ToolCalled(call) => println!("  calls {}({})", call.name, call.input),
                LoopEvent::ToolAnswered(answer) => {
                    println!("  answer {}: {:?}", answer.call_id, answer.output.as_text());
                }
                LoopEvent::Model(event) => {
                    if let Some(text) = event.text_delta() {
                        println!("  text: {text}");
                    }
                }
                _ => {}
            },
        )
        .await?;

    println!("stop: {:?}", outcome.stop);
    println!("answer: {}", outcome.output_text());
    println!(
        "loaded tools: {:?}",
        agent
            .loaded_tools()
            .map(|key| key.name.as_str())
            .collect::<Vec<_>>()
    );
    for turn in &outcome.turns {
        let remaining = turn
            .rate_limits
            .and_then(|limits| limits.requests?.remaining);
        println!(
            "turn {}: {} call(s), requests left {remaining:?}",
            turn.iteration,
            turn.calls.len()
        );
    }

    // What the loop sent: stateless requests that carry the whole history.
    let requests = requests.lock().unwrap().clone();
    let last = requests.last().unwrap();
    println!(
        "{} requests; the last one has {} input items, store={}, tools {:?}",
        requests.len(),
        last["input"].as_array().unwrap().len(),
        last["store"],
        last["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap_or_default())
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// A client whose HTTP service replays the model's replies, in order: two weather calls, a tool
/// search, two conversions, then the answer. It records the request bodies.
fn scripted_client() -> (Client, Arc<Mutex<Vec<Value>>>) {
    let requests: Arc<Mutex<Vec<Value>>> = Arc::default();
    let log = requests.clone();
    let service = tower::service_fn(move |request: http::Request<Body>| {
        let log = log.clone();
        async move {
            let body = request.into_body().collect_bytes().await?;
            let index = {
                let mut log = log.lock().unwrap();
                log.push(serde_json::from_slice(&body)?);
                log.len() - 1
            };
            let output = match index {
                0 => vec![
                    call("call_1", "get_weather", &json!({"city": "Ghent"})),
                    call("call_2", "get_weather", &json!({"city": "Paris"})),
                ],
                1 => vec![call(
                    "call_3",
                    "tool_search",
                    &json!({"query": "convert celsius to fahrenheit"}),
                )],
                2 => vec![
                    call("call_4", "celsius_to_fahrenheit", &json!({"celsius": 12})),
                    call("call_5", "celsius_to_fahrenheit", &json!({"celsius": 15})),
                ],
                _ => vec![message("Paris is warmer: 59 °F against 53.6 °F in Ghent.")],
            };
            Ok::<_, BoxError>(
                http::Response::builder()
                    .header("content-type", "text/event-stream")
                    .header("x-ratelimit-remaining-requests", (499 - index).to_string())
                    .body(Body::from(events(&format!("resp_{index}"), &output)))?,
            )
        }
    });
    let config = Config::builder()
        .api_key("offline")
        .api_base("https://api.example.test/v1")
        .http_client(timvw_rust_runtime::Client::from_service(service))
        .build();
    (Client::builder(config).unwrap().build(), requests)
}

fn call(call_id: &str, name: &str, arguments: &Value) -> Value {
    json!({"type": "function_call", "id": format!("fc_{call_id}"), "call_id": call_id,
           "name": name, "arguments": arguments.to_string(), "status": "completed"})
}

fn message(text: &str) -> Value {
    json!({"type": "message", "id": "msg_1", "status": "completed", "role": "assistant",
           "content": [{"type": "output_text", "text": text, "annotations": []}]})
}

/// The server-sent events of a response: each output item, then `response.completed`.
fn events(id: &str, output: &[Value]) -> String {
    let response = json!({
        "id": id, "object": "response", "created_at": 1, "status": "completed", "error": null,
        "incomplete_details": null, "instructions": null, "model": "gpt-4.1-mini",
        "output": output, "parallel_tool_calls": true, "tools": [], "metadata": {}
    });
    let mut events = Vec::new();
    for (index, item) in output.iter().enumerate() {
        if item["type"] == "message" {
            events.push(
                json!({"type": "response.output_text.delta", "sequence_number": index,
                               "item_id": "msg_1", "output_index": index, "content_index": 0,
                               "delta": item["content"][0]["text"]}),
            );
        }
        events.push(
            json!({"type": "response.output_item.done", "sequence_number": index,
                           "output_index": index, "item": item}),
        );
    }
    events.push(json!({"type": "response.completed", "sequence_number": 99, "response": response}));
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<Vec<_>>()
        .concat()
}
