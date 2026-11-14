//! Streaming the Responses API: text deltas, function calls as soon as they are complete, the
//! final response, and waiting for interceptor callbacks.
//!
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! cargo run --example responses_streaming
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use futures::StreamExt;
use openai_ergonomic::responses_api::OutputItem;
use openai_ergonomic::{Client, Result};
use serde_json::json;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?.build();

    // 1. Text as it is generated.
    println!("1. Streaming text");
    let mut events = client
        .send_responses_stream(client.responses().user("Write a haiku about Rust."))
        .await?;
    while let Some(event) = events.next().await {
        let event = event?;
        if let Some(text) = event.text_delta() {
            print!("{text}");
            std::io::stdout().flush().ok();
        }
        if let Some(response) = event.completed_response() {
            println!("\n[completed {}]", response.id);
        } else if event.is_terminal() {
            // `response.incomplete`, `response.failed` or `error`: the stream ends here too.
            println!("\n[ended with {}]", event.event_type());
        }
    }

    // 2. Function calls are complete at `response.output_item.done`, before the response is.
    println!("\n2. Function calls while streaming");
    let builder = client
        .responses()
        .function_tool(
            "get_weather",
            "Current weather of a city",
            json!({
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"],
                "additionalProperties": false
            }),
        )
        .user("What's the weather in Ghent and in Lisbon?");
    let mut events = client.send_responses_stream(builder).await?;
    while let Some(event) = events.next().await {
        if let Some(OutputItem::FunctionToolCall(call)) = event?.output_item_done() {
            println!("call {}: {}({})", call.call_id, call.name, call.arguments);
        }
    }

    // 3. With interceptors, wait for their callbacks before shutting down.
    println!("\n3. Waiting for interceptor callbacks");
    let (events, completion) = client
        .send_responses_stream_with_completion(client.responses().user("Say hello."))
        .await?;
    let text = openai_ergonomic::responses_api::collect_text(events).await?;
    println!("{text}");
    if let Ok(report) = completion.wait().await {
        println!(
            "outcome: {:?}, events: {}, tokens: {:?} in / {:?} out",
            report.outcome, report.total_chunks, report.input_tokens, report.output_tokens
        );
    }
    Ok(())
}
