# The agent loop

`ToolLoop` sends a conversation to the model, runs the tools it calls, sends their outputs back,
and repeats until the model answers. It works on the Responses API and on the Chat Completions
API, with the tools of a [`ToolRegistry`](tool_framework.md). It does what a hand-written
[loop](tool_orchestration.md) leaves to you:

- every call gets an answer, failures included, and the history is repaired before each request;
- the calls of tools that allow it run at the same time, the others alone, and the answers go
  into the history in call order, cut to their budget;
- on the Responses API, a call starts as soon as the model finished it, while the response is
  still streaming;
- a run stops after a number of requests, or when you cancel it, with a history you can send
  again;
- a failed request is sent again, after the delay the server asked for or with backoff;
- tools can wait until a tool search finds them, so that large tool sets do not fill the context;
- every request keeps its rate limits.

Run the examples: `cargo run --example agent_loop` (offline, with replayed replies) and
`OPENAI_API_KEY=... cargo run --example agent_loop_live`.

## A loop

```rust,no_run
use openai_ergonomic::{Client, Conversation, FunctionTool, Result, ToolLoop, ToolRegistry};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct City {
    city: String,
}

struct Weather;

impl FunctionTool for Weather {
    type Input = City;
    type Output = Value;

    fn name(&self) -> &str {
        "get_weather"
    }
    fn description(&self) -> &str {
        "Current weather of a city"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]})
    }
    async fn execute(&self, input: City) -> Result<Value> {
        Ok(json!({"city": input.city, "sky": "light rain"}))
    }
    fn supports_parallel(&self) -> bool {
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?.build();
    let mut tools = ToolRegistry::new();
    tools.register(Weather)?;

    // The template: model and options of every request, and input sent before the conversation.
    let template = client.responses().instructions("You are a household assistant.");
    let mut agent = ToolLoop::responses(&client, tools, template);

    let mut conversation = Conversation::user("Umbrella in Brussels or in Ghent today?");
    let outcome = agent.run(&mut conversation).await?;
    println!("{}", outcome.output_text());

    // The conversation holds the whole exchange: ask a follow-up and run again.
    conversation.push_user("And tomorrow?");
    let outcome = agent.run(&mut conversation).await?;
    println!("{} ({:?})", outcome.output_text(), outcome.stop);
    Ok(())
}
```

`ToolLoop::chat(&client, tools, ChatCompletionBuilder)` is the same loop on the Chat Completions
API: the template's messages (a system message, say) come before the conversation, and the
history becomes chat messages (consecutive calls in one assistant message, outputs as tool
messages; reasoning and built-in tool items are left out). Chat requests are not streamed, and
their tool outputs are text: an answer with images or files is sent as a failure.

`run` returns a `LoopOutcome`: why it stopped (`LoopStop::Completed`, `MaxIterations`,
`Cancelled`, `Incomplete { reason }`) and a `TurnRecord` per request (the response id, its text,
the calls, the retries, the tokens and the rate limits).

## Events

`run_with(&mut conversation, &cancel, on_event)` calls `on_event` with a `LoopEvent`: a request
about to be sent, each event of a streamed response (`LoopEvent::Model`, for text deltas), a call
queued and a call answered, a retry, and a finished request.

```rust,no_run
use openai_ergonomic::{CancellationSignal, Conversation, LoopEvent, ToolLoop};

# async fn example(agent: &mut ToolLoop<'_>) -> openai_ergonomic::Result<()> {
let mut conversation = Conversation::user("Plan my day.");
let outcome = agent
    .run_with(&mut conversation, &CancellationSignal::new(), |event| match event {
        LoopEvent::Model(event) => {
            if let Some(text) = event.text_delta() {
                print!("{text}");
            }
        }
        LoopEvent::ToolCalled(call) => println!("[{}]", call.name),
        LoopEvent::Retrying { delay, error, .. } => println!("[retry in {delay:?}: {error}]"),
        _ => {}
    })
    .await?;
# let _ = outcome;
# Ok(())
# }
```

## The history

A `Conversation` holds Responses API input items: messages (`push_user`, `push_system`,
`push_developer`, `push_assistant`), the model's output items (`record_output`), calls
(`push_call`) and their answers (`push_answer`). The loop appends the model's items as they
arrive and the answers after each response.

Before every request, retries included, the loop calls `normalize`, which makes any history one
the API accepts:

- a call without an output is answered `"aborted"` (after the call and the calls and outputs
  that directly follow it, so a chat assistant message keeps its tool messages together);
- an output whose call is not earlier in the conversation is dropped;
- when the model takes no images (`ModelCaps::with_images(false)`), images in messages and tool
  outputs become a short text.

```rust
use openai_ergonomic::{CallAnswer, Conversation, ModelCaps, ToolInvocation};

let mut conversation = Conversation::user("Weather in Ghent and Paris?");
let ghent = ToolInvocation::function("call_1", "get_weather", r#"{"city":"Ghent"}"#);
let paris = ToolInvocation::function("call_2", "get_weather", r#"{"city":"Paris"}"#);
conversation.push_call(&ghent);
conversation.push_call(&paris);
conversation.push_answer(&CallAnswer::new(&paris, r#"{"sky":"sun"}"#, true));
// An answer to a call the conversation does not have.
let stray = ToolInvocation::function("call_9", "get_weather", "{}");
conversation.push_answer(&CallAnswer::new(&stray, "{}", true));

let report = conversation.normalize(ModelCaps::default());
assert_eq!(report.aborted, ["call_1"]);
assert_eq!(report.orphans, ["call_9"]);
assert!(conversation.unanswered_calls().is_empty());
// Idempotent: a second pass changes nothing.
assert!(conversation.normalize(ModelCaps::default()).is_clean());
```

`remove(index)` removes an item with its partner (a call with its outputs, an output with its
call).

## Stateless requests

On the Responses API the loop's requests are stateless by default (`LoopConfig`):

- `store: false`, and every request carries the whole conversation (the items without their ids,
  which the server did not keep). The loop never chains requests with `previous_response_id`; a
  template's `previous_response_id` applies to every request.
- With a reasoning model (`ModelCaps::with_reasoning(true)`, or reasoning options on the
  template), it asks for the encrypted reasoning (`include: ["reasoning.encrypted_content"]`) and
  sends the reasoning items back, so the model keeps its reasoning between requests. Reasoning
  items without encrypted content are not sent.
- One `prompt_cache_key` per loop (`PromptCacheKey::PerLoop`), the same for all its requests;
  `Custom(key)` or `Omit` instead.
- `parallel_tool_calls: true` when there are tools.

`LoopConfig::new().with_store(true)` stores the responses (and sends the items with their ids).

## Running the calls

Each call is answered with `ToolRegistry::answer_call`: failures the model can act on (bad
arguments, an unknown tool, a handler error) become its answer, a `ToolFailure::Fatal` ends the run
with `Error::Tool` (the answers of the calls that finished are kept; the others are answered
`"aborted"` at the next `normalize`).

The calls run on the loop's task, through a FIFO read/write gate: a call of a tool whose
`supports_parallel()` is true starts while only such calls run; any other call waits until
everything before it finished, and everything after it waits for it. `on_event` sees the answers
as they come; the history gets them in call order. `LoopConfig::with_parallel_tool_calls(false)`
asks the model for one call at a time and runs every call alone. Nothing is spawned, so the loop
needs no runtime of its own and runs on WebAssembly with tools that are not `Send`.

On the Responses API (streamed, the default) a call starts at its `response.output_item.done`
event, while the model is still writing the rest of the response; the loop waits for both before
the next request. `with_stream(false)` waits for the whole response first.

Outputs are cut to their tool's budget (`FunctionTool::output_budget`), else to
`LoopConfig::truncation` (10 000 bytes, with a header line).

## Limits and cancellation

`LoopConfig::with_max_iterations(n)` (default 16; 0 for no limit) stops a run after `n` requests
when the model still calls tools: `LoopStop::MaxIterations`, with every call answered. Run again
to continue.

A `CancellationSignal` stops a run at its next await: the request or stream is dropped, the
running calls are dropped and answered `"aborted by user after 1.2s"`, the calls not started yet
`"aborted"`, and the run returns `LoopStop::Cancelled`.

```rust,no_run
use std::time::Duration;
use openai_ergonomic::{CancellationSignal, Conversation, LoopStop, ToolLoop};

# async fn example(agent: &mut ToolLoop<'_>) -> openai_ergonomic::Result<()> {
let cancel = CancellationSignal::new();
let timeout = cancel.clone();
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(30)).await;
    timeout.cancel();
});
let mut conversation = Conversation::user("Research this for me.");
let outcome = agent.run_with(&mut conversation, &cancel, |_| {}).await?;
if outcome.stop == LoopStop::Cancelled {
    // The history is valid: show what there is, or run again later.
}
# Ok(())
# }
```

## Retries

A request that fails as a whole (a connection error, a stream cut off or failed midway, a
`response.failed` or `error` event, a rate limit, a server error) is sent again when its
[`retry_class`](responses_workflows.md#failures-retries-and-rate-limits) allows it, up to
`RetryConfig::max_retries` times (default 5). The delay is the one the server asked for (a
`retry-after-ms` or `retry-after` header, or a "try again in 1.5s" message), else
`base_delay · 2^(n − 1)` with ±10 % jitter (default from 200 ms), at most `max_delay` (60 s).

The calls the failed attempt started are not run again: the loop waits for them and records
their answers, and the retry carries them. The client's own HTTP retries
(`ConfigBuilder::max_retries`) come first, for failures before the body.

```rust
use std::time::Duration;
use openai_ergonomic::{LoopConfig, RetryConfig};

let config = LoopConfig::new()
    .with_max_iterations(8)
    .with_retry(RetryConfig::new(3).with_base_delay(Duration::from_millis(500)));
assert_eq!(config.retry.max_retries, 3);
```

## Deferred tools and tool search

A tool's `Exposure` says when the model sees it: `Direct` (every request, the default),
`Deferred` (once a tool search found it) or `Hidden` (never advertised, but its calls are
answered). Set it with `FunctionTool::exposure` (also on `RichTool` and `CustomTool`),
`DynamicTool::exposure` or `ToolRegistry::set_exposure`; the registry's own definitions
(`tool_definitions`, `responses_tools`) list only direct tools.

When there are deferred tools, the loop offers a `tool_search` function (`ToolSearchConfig`: at
most 8 tools per search, its description). It answers it itself with
`ToolRegistry::search_deferred` (words of the query against the tools' names and descriptions),
records the tools found (`ToolLoop::loaded_tools`), and advertises them from the next request
on. `ToolLoop::load_tool(key)` loads one by hand; `LoopConfig::with_tool_search(None)` turns the
search off.

A model that searches tools itself (`ModelCaps::with_tool_search(true)`, Responses API only) gets
the deferred tools with `defer_loading: true` and the hosted `tool_search` tool instead; the loop
records the tools in its `tool_search_output` items as loaded.

```rust
use openai_ergonomic::{Exposure, FunctionTool, Result, ToolKey, ToolRegistry};
use serde_json::{json, Value};

struct Convert;

impl FunctionTool for Convert {
    type Input = Value;
    type Output = f64;
    fn name(&self) -> &str {
        "celsius_to_fahrenheit"
    }
    fn description(&self) -> &str {
        "Convert a temperature from Celsius to Fahrenheit"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"celsius": {"type": "number"}}})
    }
    async fn execute(&self, input: Value) -> Result<f64> {
        Ok(input["celsius"].as_f64().unwrap_or_default() * 1.8 + 32.0)
    }
    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }
}

let mut tools = ToolRegistry::new();
tools.register(Convert)?;
assert!(tools.tool_definitions().is_empty());
assert_eq!(
    tools.search_deferred("convert to fahrenheit", 8),
    [ToolKey::new("celsius_to_fahrenheit")]
);
# Ok::<(), openai_ergonomic::ToolError>(())
```

## Rate limits

Every `TurnRecord` has the `x-ratelimit-*` headers of its response (`RateLimitSnapshot`), streamed
ones included, and `LoopOutcome::rate_limits()` the latest. The plain calls carry them too:
`ResponsesResponse::rate_limits()` and `ChatCompletionResponseWrapper::rate_limits()`, next to the
client-wide `Client::rate_limits()` (the latest response of any call).
