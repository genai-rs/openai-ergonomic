//! The agent loop against the API: a `ToolLoop` on the Responses API (streamed, stateless) with
//! a weather tool, printing the text as it streams. Needs `OPENAI_API_KEY`; without it, the
//! example says so and exits. See `agent_loop` for the offline version.
//!
//! `OPENAI_API_KEY=... cargo run --example agent_loop_live` (`OPENAI_DEFAULT_MODEL` picks the
//! model).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]

use std::io::Write;

use openai_ergonomic::{
    CancellationSignal, Client, Conversation, FunctionTool, LoopEvent, Result, ToolLoop,
    ToolRegistry,
};
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
        // Call a weather service here.
        Ok(json!({"city": input.city, "temperature_c": 14, "sky": "light rain"}))
    }
    fn supports_parallel(&self) -> bool {
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var_os("OPENAI_API_KEY").is_none() {
        println!("Set OPENAI_API_KEY to run this example (or run `agent_loop`, which is offline).");
        return Ok(());
    }
    let client = Client::from_env()?.build();

    let mut tools = ToolRegistry::new();
    tools.register(Weather)?;
    let template = client
        .responses()
        .instructions("You are a household assistant. Be brief.");
    let mut agent = ToolLoop::responses(&client, tools, template);

    // Ctrl-C cancels the run; the history stays valid.
    let cancel = CancellationSignal::new();
    let on_ctrl_c = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            on_ctrl_c.cancel();
        }
    });

    let mut conversation =
        Conversation::user("Should I carry an umbrella in Brussels or in Ghent today?");
    let outcome = agent
        .run_with(&mut conversation, &cancel, |event| match event {
            LoopEvent::Model(event) => {
                if let Some(text) = event.text_delta() {
                    print!("{text}");
                    std::io::stdout().flush().ok();
                }
            }
            LoopEvent::ToolCalled(call) => println!("[{}({})]", call.name, call.input),
            LoopEvent::Retrying { delay, error, .. } => {
                println!("[retrying in {delay:?}: {error}]");
            }
            _ => {}
        })
        .await?;
    println!();
    println!(
        "stop: {:?}, {} request(s)",
        outcome.stop,
        outcome.turns.len()
    );
    if let Some(limits) = outcome.rate_limits() {
        println!("rate limits after the last request: {limits:?}");
    }
    Ok(())
}
