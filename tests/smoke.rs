//! Smoke tests against the real `OpenAI` API.
//!
//! Every test that calls the API is `#[ignore]`d: run them with an API key of a test project,
//!
//! ```bash
//! OPENAI_TEST_API_KEY=... cargo test --test smoke -- --ignored
//! ```
//!
//! (or `cargo make test-smoke`). `OPENAI_TEST_MODEL` (default `gpt-4.1-mini`) and
//! `OPENAI_TEST_BASE_URL` (default: the `OpenAI` API) select the model and the endpoint. The
//! tests compile in CI, so they keep up with the API of the crate.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use futures::StreamExt;
use openai_ergonomic::responses_api::OutputItem;
use openai_ergonomic::{Client, Config};
use serde_json::json;

/// The smoke test settings from the environment.
struct SmokeConfig {
    api_key: String,
    base_url: Option<String>,
    model: String,
}

impl SmokeConfig {
    fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_TEST_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())?;
        Some(Self {
            api_key,
            base_url: std::env::var("OPENAI_TEST_BASE_URL").ok(),
            model: std::env::var("OPENAI_TEST_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".into()),
        })
    }

    fn client(&self) -> Client {
        let mut config = Config::builder()
            .api_key(&self.api_key)
            .default_model(&self.model);
        if let Some(base_url) = &self.base_url {
            config = config.api_base(base_url);
        }
        Client::builder(config.build()).expect("client").build()
    }
}

/// The client for a smoke test; panics with a hint when no API key is set.
fn client() -> Client {
    SmokeConfig::from_env()
        .expect("set OPENAI_TEST_API_KEY to run the smoke tests")
        .client()
}

#[tokio::test]
#[ignore = "calls the OpenAI API; needs OPENAI_TEST_API_KEY"]
async fn chat_completion() {
    let client = client();
    let response = client
        .send_chat(client.chat_simple("Reply with the single word: pong"))
        .await
        .unwrap();
    assert!(response
        .content()
        .unwrap_or_default()
        .to_lowercase()
        .contains("pong"));
}

#[tokio::test]
#[ignore = "calls the OpenAI API; needs OPENAI_TEST_API_KEY"]
async fn chat_completion_stream() {
    let client = client();
    let (stream, completion) = client
        .send_chat_stream_with_completion(client.chat_simple("Count from 1 to 3."))
        .await
        .unwrap();
    let chunks: Vec<_> = stream.collect().await;
    assert!(chunks.iter().all(Result::is_ok));
    let report = completion.wait().await.unwrap();
    assert_eq!(report.outcome, openai_ergonomic::StreamOutcome::Completed);
}

#[tokio::test]
#[ignore = "calls the OpenAI API; needs OPENAI_TEST_API_KEY"]
async fn responses_text() {
    let client = client();
    let response = client
        .send_responses(
            client
                .responses()
                .instructions("Answer with one word.")
                .user("Reply with the single word: pong"),
        )
        .await
        .unwrap();
    assert!(response.output_text().to_lowercase().contains("pong"));
    assert!(response.usage().is_some());
}

#[tokio::test]
#[ignore = "calls the OpenAI API; needs OPENAI_TEST_API_KEY"]
async fn responses_stream() {
    let client = client();
    let (events, completion) = client
        .send_responses_stream_with_completion(client.responses().user("Count from 1 to 3."))
        .await
        .unwrap();
    let events: Vec<_> = events.map(Result::unwrap).collect().await;
    assert_eq!(events.last().unwrap().event_type(), "response.completed");
    assert!(events.iter().any(|event| event.text_delta().is_some()));
    let report = completion.wait().await.unwrap();
    assert_eq!(report.outcome, openai_ergonomic::StreamOutcome::Completed);
}

#[tokio::test]
#[ignore = "calls the OpenAI API; needs OPENAI_TEST_API_KEY"]
async fn responses_function_call_round_trip() {
    let client = client();
    let schema = json!({
        "type": "object",
        "properties": {"city": {"type": "string"}},
        "required": ["city"],
        "additionalProperties": false
    });
    let first = client
        .send_responses(
            client
                .responses()
                .function_tool("get_weather", "Current weather of a city", schema.clone())
                .user("Use the tool: what is the weather in Ghent?"),
        )
        .await
        .unwrap();
    let call = first
        .output()
        .iter()
        .find_map(|item| match item {
            OutputItem::FunctionToolCall(call) => Some(call.clone()),
            _ => None,
        })
        .expect("a function call");
    assert_eq!(call.name, "get_weather");

    let second = client
        .send_responses(
            client
                .responses()
                .function_tool("get_weather", "Current weather of a city", schema)
                .previous_response_id(first.id())
                .function_call_output(&call.call_id, r#"{"celsius":12}"#),
        )
        .await
        .unwrap();
    assert!(second.output_text().contains("12"));
}

#[test]
fn smoke_config_requires_a_key() {
    // Runs everywhere: without OPENAI_TEST_API_KEY there is no configuration.
    if std::env::var("OPENAI_TEST_API_KEY").is_err() {
        assert!(SmokeConfig::from_env().is_none());
    }
}
