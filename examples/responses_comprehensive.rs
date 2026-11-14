//! The Responses API (`POST /responses`): text, function calling, stateless conversations,
//! structured outputs, built-in tools and error handling.
//!
//! ## Prerequisites
//!
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example responses_comprehensive
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openai_ergonomic::responses_api::{IncludeEnum, ResponsesTool};
use openai_ergonomic::{Client, Error, ResponsesResponse};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OpenAI Ergonomic - Responses API\n");

    let client = Client::from_env()?.build();

    for (name, result) in [
        ("Basic response", basic(&client).await),
        ("Function calling", function_calling(&client).await),
        ("Stateless conversation", stateless(&client).await),
        ("Structured output", structured_output(&client).await),
        ("Built-in web search", web_search(&client).await),
    ] {
        match result {
            Ok(()) => println!("{name}: done\n"),
            Err(error) => {
                eprintln!("{name}: failed: {error}\n");
                explain(&error);
            }
        }
    }
    Ok(())
}

/// Instructions and a user message; text, status and usage of the response.
async fn basic(client: &Client) -> Result<(), Error> {
    let builder = client
        .responses()
        .instructions("You are a concise assistant.")
        .user("In one sentence: what does the Responses API add over chat completions?")
        .max_output_tokens(200);
    let response = client.send_responses(builder).await?;
    print_response(&response);
    Ok(())
}

/// The model calls a function; the output goes back with `previous_response_id`.
async fn function_calling(client: &Client) -> Result<(), Error> {
    let weather = json!({
        "type": "object",
        "properties": {"city": {"type": "string", "description": "City name"}},
        "required": ["city"],
        "additionalProperties": false
    });
    let builder = client
        .responses()
        .function_tool("get_weather", "Current weather of a city", weather.clone())
        .user("What's the weather in Ghent?");
    let first = client.send_responses(builder).await?;

    let calls = first.function_calls();
    if calls.is_empty() {
        println!("No function call: {}", first.output_text());
        return Ok(());
    }

    // Answer every call; the stored response holds the calls themselves.
    let mut follow_up = client
        .responses()
        .function_tool("get_weather", "Current weather of a city", weather)
        .previous_response_id(first.id());
    for call in calls {
        println!("Model called {}({})", call.name, call.arguments);
        follow_up =
            follow_up.function_call_output(&call.call_id, r#"{"celsius":12,"sky":"cloudy"}"#);
    }
    let second = client.send_responses(follow_up).await?;
    print_response(&second);
    Ok(())
}

/// Nothing stored: the next request carries the earlier output items (with encrypted
/// reasoning, for reasoning models).
async fn stateless(client: &Client) -> Result<(), Error> {
    let first = client
        .send_responses(
            client
                .responses()
                .store(false)
                .include(vec![IncludeEnum::ReasoningEncryptedContent])
                .user("Pick a random fruit and name it."),
        )
        .await?;
    println!("Turn 1: {}", first.output_text());

    let second = client
        .send_responses(
            client
                .responses()
                .store(false)
                .user("Pick a random fruit and name it.")
                .input_items(first.output_as_input()?)
                .user("What colour is it?"),
        )
        .await?;
    println!("Turn 2: {}", second.output_text());
    Ok(())
}

/// JSON output that follows a schema.
async fn structured_output(client: &Client) -> Result<(), Error> {
    let schema = json!({
        "type": "object",
        "properties": {
            "city": {"type": "string"},
            "country": {"type": "string"},
            "population": {"type": "integer"}
        },
        "required": ["city", "country", "population"],
        "additionalProperties": false
    });
    let builder = client
        .responses()
        .user("Give me facts about the capital of Belgium.")
        .json_schema("city_facts", schema);
    let response = client.send_responses(builder).await?;
    let facts: serde_json::Value = serde_json::from_str(&response.output_text())?;
    println!("{}", serde_json::to_string_pretty(&facts)?);
    Ok(())
}

/// A built-in tool, written as its JSON definition.
async fn web_search(client: &Client) -> Result<(), Error> {
    let web_search: ResponsesTool = serde_json::from_value(json!({"type": "web_search"}))?;
    let builder = client
        .responses()
        .tool(web_search)
        .user("What is one notable Rust release this year? Answer in one sentence.");
    let response = client.send_responses(builder).await?;
    print_response(&response);
    Ok(())
}

fn print_response(response: &ResponsesResponse) {
    println!("[{}] {}", response.id(), response.output_text());
    if let Some(status) = response.status() {
        println!("Status: {status}");
    }
    if let Some(reason) = response.incomplete_reason() {
        println!("Incomplete: {reason}");
    }
    if let Some(usage) = response.usage() {
        println!(
            "Tokens: {} in, {} out, {} total",
            usage.input_tokens, usage.output_tokens, usage.total_tokens
        );
    }
}

fn explain(error: &Error) {
    if error.is_rate_limit() {
        eprintln!("  Rate limited: retry later (the client already retried).");
    } else if error.is_auth_error() {
        eprintln!("  Check OPENAI_API_KEY.");
    } else if let Error::Api {
        status,
        error_code: Some(code),
        ..
    } = error
    {
        eprintln!("  API error {status}: {code}");
    }
}
