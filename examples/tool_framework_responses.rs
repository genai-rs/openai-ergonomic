//! Tools on the Responses API, offline: a function tool, a tool that returns an image, a custom
//! (freeform) tool in a namespace and a tool only known at runtime answer the calls of a
//! replayed Responses API reply. No API key required.
//!
//! `cargo run --example tool_framework_responses`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Plain `async fn` tools without `.await` are fine; newer Clippy's pedantic
// `unused_async_trait_impl` would flag them (`unknown_lints` keeps Rust 1.88 quiet).
#![allow(unknown_lints, clippy::unused_async_trait_impl)]

use openai_ergonomic::builders::Builder;
use openai_ergonomic::tool_framework::OutputBudget;
use openai_ergonomic::{
    CustomFormat, CustomTool, DynamicTool, Error, FunctionTool, OutputContent, ResponsesBuilder,
    ResponsesResponse, Result, RichTool, ToolOutputBody, ToolRegistry, TruncationConfig,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CityInput {
    city: String,
}

/// A typed function tool.
struct Weather;

impl FunctionTool for Weather {
    type Input = CityInput;
    type Output = Value;

    fn name(&self) -> &'static str {
        "get_weather"
    }
    fn description(&self) -> &'static str {
        "Current weather of a city"
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"city": {"type": "string", "description": "The city"}},
            "required": ["city"],
            "additionalProperties": false
        })
    }
    async fn execute(&self, input: CityInput) -> Result<Value> {
        if input.city.is_empty() {
            // The model sees this message and can correct its call.
            return Err(Error::InvalidRequest("name a city".into()));
        }
        Ok(json!({"city": input.city, "sky": "light rain", "celsius": 14}))
    }
    fn supports_parallel(&self) -> bool {
        true
    }
}

/// A tool whose output has an image: the Responses API takes it as the call's output.
struct RadarMap;

impl RichTool for RadarMap {
    type Input = CityInput;

    fn name(&self) -> &'static str {
        "radar_map"
    }
    fn description(&self) -> &'static str {
        "The rain radar map of a city"
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        })
    }
    async fn execute(&self, input: CityInput) -> Result<ToolOutputBody> {
        Ok(vec![
            OutputContent::text(format!("Radar map of {}", input.city)),
            OutputContent::image_url("https://example.com/radar/ghent.png"),
        ]
        .into())
    }
}

/// A custom tool: the model sends text that matches a grammar instead of JSON.
struct Note;

impl CustomTool for Note {
    fn name(&self) -> &'static str {
        "add_note"
    }
    fn description(&self) -> &'static str {
        "Add a note to the travel plan: one line starting with `NOTE: `"
    }
    fn format(&self) -> CustomFormat {
        CustomFormat::lark(r#"start: "NOTE: " /[^\n]+/"#)
    }
    async fn execute(&self, input: String) -> Result<ToolOutputBody> {
        Ok(ToolOutputBody::text(format!("saved {input:?}")))
    }
}

/// A Responses API reply with four tool calls, as the API would send it.
fn replayed_response() -> ResponsesResponse {
    serde_json::from_value(json!({
        "id": "resp_1", "object": "response", "created_at": 1, "status": "completed",
        "error": null, "incomplete_details": null, "instructions": null,
        "model": "gpt-4.1-mini", "parallel_tool_calls": true, "tools": [], "metadata": {},
        "output": [
            {"type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "get_weather",
             "arguments": "{\"city\":\"Ghent\"}", "status": "completed"},
            {"type": "function_call", "id": "fc_2", "call_id": "call_2", "name": "radar_map",
             "arguments": "{\"city\":\"Ghent\"}", "status": "completed"},
            {"type": "custom_tool_call", "id": "ctc_1", "call_id": "call_3", "namespace": "plan",
             "name": "add_note", "input": "NOTE: bring an umbrella"},
            {"type": "function_call", "id": "fc_3", "call_id": "call_4",
             "name": "calendar__next_event", "arguments": "{\"days\":2}", "status": "completed"},
            {"type": "function_call", "id": "fc_4", "call_id": "call_5", "name": "get_weather",
             "arguments": "{\"town\":\"Ghent\"}", "status": "completed"}
        ]
    }))
    .unwrap()
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut tools = ToolRegistry::new();
    tools.register(Weather)?;
    tools.register_rich(RadarMap)?;
    tools
        .namespace("plan", "Edit the travel plan")?
        .register_custom(Note)?;
    // A tool only known at runtime (an MCP server, a plugin): its name and schema are cleaned up.
    let calendar = tools.register_dynamic(
        DynamicTool::new(
            "next-event",
            "The next calendar event within some days",
            json!({"properties": {"days": {"type": "integer", "const": 2}}}),
            |arguments: Value| async move {
                Ok(ToolOutputBody::text(format!(
                    "Train to Brussels in {} days",
                    arguments["days"]
                )))
            },
        )
        .prefix("calendar"),
    )?;
    println!("dynamic tool registered as {calendar}");

    // The tools of a Responses API request.
    let request = ResponsesBuilder::new("gpt-4.1-mini")
        .tools(tools.responses_tools())
        .store(false)
        .user("Plan my trip to Ghent");
    let tool_list: Vec<String> = request
        .clone()
        .build()?
        .tools
        .unwrap_or_default()
        .iter()
        .map(|tool| serde_json::to_value(tool).unwrap()["name"].to_string())
        .collect();
    println!("tools sent: {}", tool_list.join(", "));

    // Answer every call of the reply: failures become answers the model can act on.
    let response = replayed_response();
    let truncation = TruncationConfig::new(OutputBudget::Bytes(2_000));
    let mut next = request.input_items(response.output_as_input()?);
    for call in response.tool_invocations() {
        let parallel = tools.supports_parallel(&call);
        let answer = tools.answer_call(&call).await?;
        let answer = answer.truncated(
            truncation.budget_for(tools.output_budget(&call)),
            truncation.header,
        );
        println!(
            "{} {}{} -> success={} (parallel-safe: {parallel})",
            answer.call_id,
            call.namespace
                .as_deref()
                .map(|namespace| format!("{namespace}."))
                .unwrap_or_default(),
            call.name,
            answer.success
        );
        println!("    {}", answer.output.to_json());
        next = next.call_answer(answer);
    }

    // The next request carries the model's output and one answer per call.
    let next = next.build()?;
    println!(
        "next request: {}",
        serde_json::to_string_pretty(&next.input).unwrap()
    );
    Ok(())
}
