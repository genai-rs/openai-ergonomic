//! Tools on both APIs: answering every call (`answer_call`), rich and custom tools, namespaces,
//! dynamic tools, and the Responses API round trip against a mock server.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(
    unknown_lints,
    clippy::unused_async_trait_impl,
    clippy::empty_enums,
    clippy::unnecessary_literal_bound,
    clippy::items_after_statements,
    clippy::significant_drop_tightening
)]

use mockito::{Matcher, Server};
use openai_client_base::models::{
    ChatCompletionMessageToolCallsInner, FunctionToolCall, InputItem,
};
use openai_ergonomic::builders::Builder;
use openai_ergonomic::tool_framework::{GrammarSyntax, OutputBudget, ToolKey};
use openai_ergonomic::{
    CallAnswer, ChatCompletionBuilder, Client, Config, CustomFormat, CustomTool, DynamicTool,
    Error, FunctionTool, OutputContent, Result, RichTool, ToolError, ToolFailure, ToolInvocation,
    ToolKind, ToolOutputBody, ToolRegistry,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CityInput {
    city: String,
}

struct Weather;

impl FunctionTool for Weather {
    type Input = CityInput;
    type Output = Value;
    fn name(&self) -> &str {
        "get_weather"
    }
    fn description(&self) -> &str {
        "Current weather of a city"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"city": {"type": "string"}},
               "required": ["city"], "additionalProperties": false})
    }
    async fn execute(&self, input: CityInput) -> Result<Value> {
        match input.city.as_str() {
            "Atlantis" => Err(Error::application(std::io::Error::other("no such city"))),
            "Mordor" => Err(Error::fatal(std::io::Error::other(
                "weather service is down",
            ))),
            city => Ok(json!({"city": city, "sky": "rain"})),
        }
    }
    fn supports_parallel(&self) -> bool {
        true
    }
    fn output_budget(&self) -> Option<OutputBudget> {
        Some(OutputBudget::Bytes(100))
    }
}

struct Chart;

impl RichTool for Chart {
    type Input = Value;
    fn name(&self) -> &str {
        "chart"
    }
    fn description(&self) -> &str {
        "Draw a chart"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _input: Value) -> Result<ToolOutputBody> {
        Ok(vec![
            OutputContent::text("Here is the chart"),
            OutputContent::image_url("https://example.com/chart.png"),
        ]
        .into())
    }
}

struct Patch;

impl CustomTool for Patch {
    fn name(&self) -> &str {
        "apply_patch"
    }
    fn description(&self) -> &str {
        "Apply a patch to the workspace"
    }
    fn format(&self) -> CustomFormat {
        CustomFormat::lark("start: \"*** Begin Patch\" /.+/s")
    }
    async fn execute(&self, input: String) -> Result<ToolOutputBody> {
        Ok(ToolOutputBody::text(format!(
            "applied {} bytes",
            input.len()
        )))
    }
}

struct Search;

impl FunctionTool for Search {
    type Input = Value;
    type Output = String;
    fn name(&self) -> &str {
        "search"
    }
    fn description(&self) -> &str {
        "Search the documentation"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"q": {"type": "string"}}})
    }
    async fn execute(&self, input: Value) -> Result<String> {
        Ok(format!("docs for {}", input["q"]))
    }
}

fn registry() -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    tools.register(Weather).unwrap();
    tools.register_rich(Chart).unwrap();
    tools.register_custom(Patch).unwrap();
    tools
        .namespace("docs", "Documentation tools")
        .unwrap()
        .register(Search)
        .unwrap();
    tools
}

fn chat_call(id: &str, name: &str, arguments: &str) -> ChatCompletionMessageToolCallsInner {
    serde_json::from_value(json!({
        "id": id, "type": "function",
        "function": {"name": name, "arguments": arguments}
    }))
    .unwrap()
}

#[tokio::test]
async fn answer_call_answers_chat_and_responses_calls() {
    let tools = registry();

    let answer = tools
        .answer_call(&chat_call("call_1", "get_weather", r#"{"city":"Ghent"}"#))
        .await
        .unwrap();
    assert!(answer.success);
    assert_eq!(answer.call_id, "call_1");
    assert_eq!(answer.kind, ToolKind::Function);
    assert_eq!(
        serde_json::from_str::<Value>(answer.output.as_text().unwrap()).unwrap(),
        json!({"city": "Ghent", "sky": "rain"})
    );

    let call: FunctionToolCall = serde_json::from_value(json!({
        "type": "function_call", "call_id": "call_2", "name": "get_weather",
        "arguments": "{\"city\":\"Ghent\"}"
    }))
    .unwrap();
    assert!(tools.answer_call(&call).await.unwrap().success);
}

#[tokio::test]
async fn failures_the_model_can_act_on_are_answered() {
    let tools = registry();
    let answer = |call: ToolInvocation| {
        let tools = &tools;
        async move { tools.answer_call(&call).await.unwrap() }
    };

    let bad = answer(ToolInvocation::function(
        "c1",
        "get_weather",
        "{\"town\":1}",
    ))
    .await;
    assert!(!bad.success);
    assert!(bad
        .output
        .as_text()
        .unwrap()
        .starts_with("failed to parse function arguments:"));

    let not_object = answer(ToolInvocation::function("c2", "get_weather", "[]")).await;
    assert_eq!(
        not_object.output.as_text(),
        Some("function arguments must be a JSON object")
    );

    let unknown = answer(ToolInvocation::function("c3", "nope", "{}")).await;
    assert_eq!(unknown.output.as_text(), Some("unsupported call: nope"));

    let handler = answer(ToolInvocation::function(
        "c4",
        "get_weather",
        r#"{"city":"Atlantis"}"#,
    ))
    .await;
    assert_eq!(handler.output.as_text(), Some("no such city"));
    assert!(!handler.success);

    let wrong_kind = answer(ToolInvocation::custom("c5", "get_weather", "hello")).await;
    assert!(wrong_kind
        .output
        .as_text()
        .unwrap()
        .contains("function tool"));

    let other: ChatCompletionMessageToolCallsInner =
        serde_json::from_value(json!({"id": "c6", "type": "future_tool"})).unwrap();
    let other = tools.answer_call(&other).await.unwrap();
    assert!(!other.success);
    assert_eq!(other.kind, ToolKind::Other("future_tool".into()));
}

#[tokio::test]
async fn fatal_errors_and_missing_ids_are_returned() {
    let tools = registry();
    let error = tools
        .answer_call(ToolInvocation::function(
            "call_9",
            "get_weather",
            r#"{"city":"Mordor"}"#,
        ))
        .await
        .unwrap_err();
    match &error {
        ToolError::Call { call_id, source } => {
            assert_eq!(call_id, "call_9");
            assert!(
                matches!(**source, ToolError::Execution { ref source, .. } if source.is_fatal())
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(matches!(
        tools
            .answer_call(ToolInvocation::function("", "get_weather", "{}"))
            .await,
        Err(ToolError::Call { .. })
    ));

    // A custom classification: hide handler errors from the model.
    let answer = tools
        .answer_call_with(
            ToolInvocation::function("c", "get_weather", r#"{"city":"Atlantis"}"#),
            |error| match error {
                ToolError::Execution { .. } => {
                    ToolFailure::RespondToModel("the tool failed".into())
                }
                other => other.classify(),
            },
        )
        .await
        .unwrap();
    assert_eq!(answer.output.as_text(), Some("the tool failed"));
}

#[tokio::test]
async fn custom_namespaced_and_rich_tools() {
    let tools = registry();

    let patch = tools
        .answer_call(ToolInvocation::custom(
            "c1",
            "apply_patch",
            "*** Begin Patch",
        ))
        .await
        .unwrap();
    assert_eq!(patch.output.as_text(), Some("applied 15 bytes"));
    assert_eq!(
        serde_json::to_value(patch.to_input_item()).unwrap(),
        json!({"type": "custom_tool_call_output", "call_id": "c1", "output": "applied 15 bytes"})
    );

    let search = tools
        .answer_call(
            ToolInvocation::function("c2", "search", r#"{"q":"tools"}"#).in_namespace("docs"),
        )
        .await
        .unwrap();
    assert_eq!(search.output.as_text(), Some(r#""docs for \"tools\"""#));
    // Outside its namespace the tool is unknown.
    let outside = tools
        .answer_call(ToolInvocation::function("c3", "search", "{}"))
        .await
        .unwrap();
    assert_eq!(outside.output.as_text(), Some("unsupported call: search"));

    let chart = tools
        .answer_call(ToolInvocation::function("c4", "chart", "{}"))
        .await
        .unwrap();
    let item = serde_json::to_value(chart.to_input_item()).unwrap();
    assert_eq!(item["output"][1]["type"], "input_image");
    // Chat tool messages are text: the image cannot go there.
    let builder = ChatCompletionBuilder::new("gpt-4o").user("chart please");
    assert!(matches!(
        builder.clone().call_answer(chart),
        Err(Error::Tool(ref error)) if matches!(**error, ToolError::UnsupportedOutput { .. })
    ));
    // execute_call refuses it too, and keeps the call id.
    assert!(matches!(
        tools.execute_call(&chat_call("c5", "chart", "{}")).await,
        Err(ToolError::Call { ref call_id, .. }) if call_id == "c5"
    ));
    let text = CallAnswer::new(&ToolInvocation::function("c6", "f", "{}"), "ok", true);
    let request = builder.call_answer(text).unwrap().build().unwrap();
    assert_eq!(
        serde_json::to_value(&request.messages[1]).unwrap(),
        json!({"role": "tool", "content": "ok", "tool_call_id": "c6"})
    );

    assert!(tools.supports_parallel(&ToolInvocation::function("c", "get_weather", "{}")));
    assert!(!tools.supports_parallel(&ToolInvocation::function("c", "chart", "{}")));
    assert_eq!(
        tools.output_budget(&ToolInvocation::function("c", "get_weather", "{}")),
        Some(OutputBudget::Bytes(100))
    );
}

#[test]
fn definitions_for_each_api() {
    let tools = registry();
    let chat: Vec<String> = tools
        .tool_definitions()
        .into_iter()
        .map(|tool| tool.function.name)
        .collect();
    assert_eq!(chat, ["chart", "get_weather"]);

    let responses = serde_json::to_value(tools.responses_tools()).unwrap();
    let responses = responses.as_array().unwrap();
    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["name"], "apply_patch");
    assert_eq!(responses[0]["type"], "custom");
    assert_eq!(
        responses[0]["format"],
        json!({"type": "grammar", "syntax": "lark", "definition": "start: \"*** Begin Patch\" /.+/s"})
    );
    assert_eq!(responses[1]["name"], "chart");
    assert_eq!(responses[1]["type"], "function");
    assert_eq!(responses[2]["name"], "get_weather");
    assert_eq!(responses[2]["parameters"]["required"], json!(["city"]));
    assert_eq!(responses[3]["type"], "namespace");
    assert_eq!(responses[3]["name"], "docs");
    assert_eq!(responses[3]["description"], "Documentation tools");
    assert_eq!(responses[3]["tools"][0]["name"], "search");
    assert_eq!(responses[3]["tools"][0]["type"], "function");

    let keys: Vec<&ToolKey> = tools.keys().collect();
    assert_eq!(keys.len(), 4);
    assert_eq!(keys[3], &ToolKey::namespaced("docs", "search"));
    assert_eq!(
        CustomFormat::regex("a+"),
        CustomFormat::Grammar {
            syntax: GrammarSyntax::Regex,
            definition: "a+".into()
        }
    );
}

#[test]
fn namespaces_and_tools_do_not_collide() {
    let mut tools = registry();
    assert!(matches!(
        tools.namespace("get_weather", "Weather"),
        Err(ToolError::Duplicate { .. })
    ));
    assert!(matches!(
        tools.namespace("docs", "Other description"),
        Err(ToolError::InvalidDefinition { .. })
    ));
    assert!(matches!(
        tools
            .namespace("docs", "Documentation tools")
            .unwrap()
            .register(Search),
        Err(ToolError::Duplicate { .. })
    ));
    struct Docs;
    impl FunctionTool for Docs {
        type Input = Value;
        type Output = Value;
        fn name(&self) -> &str {
            "docs"
        }
        fn description(&self) -> &str {
            "Clashes with the namespace"
        }
        fn parameters_schema(&self) -> Value {
            json!({"type": "object"})
        }
        async fn execute(&self, input: Value) -> Result<Value> {
            Ok(input)
        }
    }
    assert!(matches!(
        tools.register(Docs),
        Err(ToolError::Duplicate { .. })
    ));
    assert!(matches!(
        tools.namespace("bad name", "x"),
        Err(ToolError::InvalidDefinition { .. })
    ));
}

#[tokio::test]
async fn dynamic_tools_are_sanitized() {
    let mut tools = ToolRegistry::new();
    let name = tools
        .register_dynamic(
            DynamicTool::new(
                "create-issue",
                "",
                json!({"properties": {"title": {"const": "bug"}, "labels": true}}),
                |arguments: Value| async move {
                    Ok(ToolOutputBody::text(format!(
                        "created {}",
                        arguments["title"]
                    )))
                },
            )
            .prefix("github")
            .supports_parallel(true),
        )
        .unwrap();
    assert_eq!(name, "github__create_issue");
    assert_eq!(tools.raw_name(&name), Some("create-issue"));
    let definition = &tools.tool_definitions()[0];
    assert_eq!(
        definition.function.description.as_deref(),
        Some("create-issue")
    );
    let parameters = serde_json::to_value(&definition.function.parameters).unwrap();
    assert_eq!(parameters["type"], "object");
    assert_eq!(
        parameters["properties"]["title"],
        json!({"type": "string", "enum": ["bug"]})
    );
    assert_eq!(
        parameters["properties"]["labels"],
        json!({"type": "string"})
    );

    let answer = tools
        .answer_call(ToolInvocation::function("c1", &name, r#"{"title":"x"}"#))
        .await
        .unwrap();
    assert_eq!(answer.output.as_text(), Some(r#"created "x""#));
    assert!(tools.supports_parallel(&ToolInvocation::function("c", &name, "{}")));
    assert_eq!(
        tools.execute(&name, r#"{"title":"y"}"#).await.unwrap(),
        json!("created \"y\"")
    );

    // The same raw name again gets another name.
    let again = tools
        .register_dynamic(
            DynamicTool::new(
                "create-issue",
                "Create",
                json!({"type": "object"}),
                |_| async { Ok(ToolOutputBody::text("again")) },
            )
            .prefix("github"),
        )
        .unwrap();
    assert_ne!(again, name);
    assert!(again.starts_with("github__create_issue_"));

    // An oversized schema falls back to any object.
    let properties: serde_json::Map<String, Value> = (0..400)
        .map(|i| {
            (
                format!("field_{i}"),
                json!({"anyOf": [{"type": "string"}, {"type": "integer"}]}),
            )
        })
        .collect();
    let big = tools
        .register_dynamic(DynamicTool::new(
            "big",
            "Big",
            json!({"type": "object", "properties": properties}),
            |_| async { Ok(ToolOutputBody::text("ok")) },
        ))
        .unwrap();
    let definition = tools
        .tool_definitions()
        .into_iter()
        .find(|tool| tool.function.name == big)
        .unwrap();
    assert_eq!(
        serde_json::to_value(definition.function.parameters).unwrap(),
        json!({"type": "object", "properties": {}, "additionalProperties": true})
    );

    assert!(matches!(
        tools.register_dynamic(DynamicTool::new(
            "list",
            "x",
            json!({"type": "array"}),
            |_| async { Ok(ToolOutputBody::text("")) }
        )),
        Err(ToolError::InvalidDefinition { .. })
    ));
}

#[test]
fn truncation_uses_the_tool_budget() {
    let call = ToolInvocation::function("c", "get_weather", "{}");
    let answer = CallAnswer::new(&call, "x".repeat(300), true);
    let config = openai_ergonomic::TruncationConfig::default();
    let budget = config.budget_for(Some(OutputBudget::Bytes(100)));
    let cut = answer.truncated(budget, config.header);
    let text = cut.output.as_text().unwrap();
    assert!(text.starts_with("[truncated: original 300 bytes/~75 tokens, 1 lines]\n"));
    assert!(text.contains("…200 chars truncated…"));
    assert_eq!(CallAnswer::aborted(&call).output.as_text(), Some("aborted"));
}

fn response_with(output: &Value) -> Value {
    json!({
        "id": "resp_1", "object": "response", "created_at": 1, "status": "completed",
        "error": null, "incomplete_details": null, "instructions": null,
        "model": "gpt-4.1-mini", "output": output, "parallel_tool_calls": true,
        "tools": [], "metadata": {}
    })
}

#[tokio::test]
async fn responses_round_trip_with_the_registry() {
    let mut server = Server::new_async().await;
    let first = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({
            "input": [{"type": "message", "role": "user", "content": "Weather and a patch"}]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("x-ratelimit-limit-requests", "500")
        .with_header("x-ratelimit-remaining-requests", "499")
        .with_header("x-ratelimit-reset-requests", "120ms")
        .with_body(
            response_with(&json!([
                {"type": "function_call", "id": "fc_1", "call_id": "call_1",
                 "name": "get_weather", "arguments": "{\"city\":\"Ghent\"}", "status": "completed"},
                {"type": "custom_tool_call", "id": "ctc_1", "call_id": "call_2",
                 "name": "apply_patch", "input": "*** Begin Patch"}
            ]))
            .to_string(),
        )
        .create_async()
        .await;

    let client = Client::builder(
        Config::builder()
            .api_key("test-key")
            .api_base(server.url())
            .max_retries(0)
            .build(),
    )
    .unwrap()
    .build();
    assert_eq!(client.rate_limits(), None);
    let tools = registry();
    let builder = client
        .responses()
        .tools(tools.responses_tools())
        .store(false)
        .user("Weather and a patch");
    let response = client.send_responses(builder.clone()).await.unwrap();
    first.assert_async().await;
    let limits = client.rate_limits().unwrap();
    assert_eq!(limits.requests.unwrap().remaining, Some(499));
    assert_eq!(
        limits.requests.unwrap().reset,
        Some(std::time::Duration::from_millis(120))
    );

    let calls = response.tool_invocations();
    assert_eq!(calls.len(), 2);
    let mut answers = Vec::new();
    for call in &calls {
        answers.push(tools.answer_call(call).await.unwrap());
    }

    let second = server
        .mock("POST", "/responses")
        .match_body(Matcher::PartialJson(json!({
            "input": [
                {"type": "message", "role": "user", "content": "Weather and a patch"},
                {"type": "function_call", "call_id": "call_1", "name": "get_weather"},
                {"type": "custom_tool_call", "call_id": "call_2", "name": "apply_patch"},
                {"type": "function_call_output", "call_id": "call_1"},
                {"type": "custom_tool_call_output", "call_id": "call_2", "output": "applied 15 bytes"}
            ]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            response_with(&json!([{
                "type": "message", "id": "msg_1", "status": "completed", "role": "assistant",
                "content": [{"type": "output_text", "text": "Done", "annotations": []}]
            }]))
            .to_string(),
        )
        .create_async()
        .await;
    let next = builder
        .input_items(response.output_as_input().unwrap())
        .call_answers(answers);
    let answer = client.send_responses(next).await.unwrap();
    second.assert_async().await;
    assert_eq!(answer.output_text(), "Done");
    assert!(answer.failure().is_none());
}

#[tokio::test]
async fn list_input_items_reads_a_page() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/responses/resp_1/input_items")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("limit".into(), "2".into()),
            Matcher::UrlEncoded("order".into(), "asc".into()),
            Matcher::UrlEncoded("after".into(), "msg_0".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "object": "list",
                "data": [{"type": "message", "id": "msg_1", "status": "completed", "role": "user",
                          "content": [{"type": "input_text", "text": "Hi"}]}],
                "has_more": false, "first_id": "msg_1", "last_id": "msg_1"
            })
            .to_string(),
        )
        .create_async()
        .await;
    let client = Client::builder(
        Config::builder()
            .api_key("test-key")
            .api_base(server.url())
            .max_retries(0)
            .build(),
    )
    .unwrap()
    .build();
    let page = client
        .responses_api()
        .list_input_items_page("resp_1", Some(2), Some("msg_0"))
        .await
        .unwrap();
    mock.assert_async().await;
    assert_eq!(page.data.len(), 1);
    assert!(!page.has_more);
}

#[test]
fn input_items_from_answers() {
    let call = ToolInvocation::function("call_1", "f", "{}");
    let item: InputItem = CallAnswer::new(&call, "42", true).into();
    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({"type": "function_call_output", "call_id": "call_1", "output": "42"})
    );
}
