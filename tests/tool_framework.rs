//! Consumer-level tool definition, dispatch and replay contracts.
use async_trait::async_trait;
use openai_client_base::models::ChatCompletionMessageToolCallsInner;
use openai_ergonomic::{
    builders::{chat::ChatCompletionBuilder, Builder},
    Error, FunctionTool, Result, ToolError, ToolRegistry,
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    message: String,
    suffix: Option<String>,
}
struct Echo {
    name: String,
    calls: Arc<AtomicUsize>,
    schema: Value,
    description: String,
}
impl Echo {
    fn new(name: &str, calls: &Arc<AtomicUsize>) -> Self {
        Self {
            name: name.into(),
            calls: calls.clone(),
            description: "Echo a message with an optional suffix".into(),
            schema: json!({"type": "object", "properties": {
                "message": {"type": "string", "description": "Text to echo"},
                "suffix": {"type": ["string", "null"], "description": "Optional suffix"}
            }, "required": ["message"], "additionalProperties": false}),
        }
    }
}
#[async_trait]
impl FunctionTool for Echo {
    type Input = Input;
    type Output = String;
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }
    async fn execute(&self, input: Input) -> Result<String> {
        tokio::task::yield_now().await;
        self.calls.fetch_add(1, Ordering::SeqCst);
        if input.message == "fail" {
            return Err(Error::InvalidRequest("application rejected message".into()));
        }
        Ok(input.message + &input.suffix.unwrap_or_default())
    }
}
fn call(id: &str, name: &str, arguments: &str) -> ChatCompletionMessageToolCallsInner {
    serde_json::from_value(json!({"id": id, "type": "function", "function": {
        "name": name, "arguments": arguments
    }}))
    .unwrap()
}

#[tokio::test]
async fn definitions_decode_and_replay_match_public_client_models() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tools = ToolRegistry::new();
    tools.register(Echo::new("z_echo", &calls)).unwrap();
    tools.register(Echo::new("a_echo", &calls)).unwrap();
    let definitions = serde_json::to_value(tools.tool_definitions()).unwrap();
    assert_eq!(definitions[0]["function"]["name"], "a_echo");
    assert_eq!(definitions[1]["function"]["name"], "z_echo");
    assert_eq!(definitions[0]["type"], "function");
    assert_eq!(
        definitions[0]["function"]["description"],
        "Echo a message with an optional suffix"
    );
    assert_eq!(
        definitions[0]["function"]["parameters"],
        Echo::new("unused", &calls).schema
    );
    assert!(definitions[0]["function"]["strict"].is_null());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let model_calls = vec![
        call("first", "z_echo", r#"{"message":"Hello"}"#),
        call("second", "a_echo", r#"{"message":"Hi","suffix":"!"}"#),
    ];
    let mut builder = ChatCompletionBuilder::new("gpt-test")
        .user("Greet me")
        .tools(tools.tool_definitions())
        .assistant_with_tool_calls("Greeting", model_calls.clone());
    for model_call in &model_calls {
        let output = tools.execute_call(model_call).await.unwrap();
        builder = builder.tool(output.call_id, output.content);
    }
    let request = serde_json::to_value(builder.build().unwrap()).unwrap();
    let messages = &request["messages"];
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "Greeting");
    assert_eq!(
        messages[1]["tool_calls"],
        serde_json::to_value(model_calls).unwrap()
    );
    assert_eq!(messages[2]["tool_call_id"], "first");
    assert_eq!(messages[2]["content"], r#""Hello""#);
    assert_eq!(messages[3]["tool_call_id"], "second");
    assert_eq!(messages[3]["content"], r#""Hi!""#);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        tools
            .execute("z_echo", r#"{"message":"Hi","suffix":null}"#)
            .await
            .unwrap(),
        "Hi"
    );
}

#[tokio::test]
async fn rejects_bad_arguments_before_running_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tools = ToolRegistry::new();
    tools.register(Echo::new("echo", &calls)).unwrap();
    for args in [
        "{",
        "{}",
        r#"{"message":3}"#,
        r#"{"message":"a","extra":1}"#,
        r#"{"message":"a","message":"b"}"#,
        r#"{"message":"a"} trailing"#,
    ] {
        assert!(
            matches!(
                tools.execute("echo", args).await,
                Err(ToolError::Json { .. })
            ),
            "{args}"
        );
    }
    for args in ["[]", "null", "3", r#""hello""#] {
        assert!(matches!(
            tools.execute("echo", args).await,
            Err(ToolError::InvalidArguments { .. })
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_registration_preserves_existing_tools() {
    let calls = Arc::new(AtomicUsize::new(0));
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let mut tools = ToolRegistry::new();
    tools.register(Echo::new("echo", &calls)).unwrap();
    assert!(
        matches!(tools.register(Echo::new("echo", &replacement_calls)), Err(ToolError::Duplicate { name }) if name == "echo")
    );
    for name in ["", "space name", "écho", &"a".repeat(65)] {
        assert!(matches!(
            tools.register(Echo::new(name, &calls)),
            Err(ToolError::InvalidDefinition { .. })
        ));
    }
    for schema in [Value::Null, json!([]), json!({}), json!({"type": "array"})] {
        let mut tool = Echo::new("invalid", &calls);
        tool.schema = schema;
        assert!(matches!(
            tools.register(tool),
            Err(ToolError::InvalidDefinition { .. })
        ));
    }
    let mut blank = Echo::new("blank", &calls);
    blank.description = "  ".into();
    assert!(matches!(
        tools.register(blank),
        Err(ToolError::InvalidDefinition { .. })
    ));
    assert_eq!(tools.tool_definitions().len(), 1);
    tools.execute("echo", r#"{"message":"ok"}"#).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(replacement_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn call_errors_keep_id_and_source_without_losing_previous_results() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tools = ToolRegistry::new();
    tools.register(Echo::new("echo", &calls)).unwrap();
    let first = tools
        .execute_call(&call("first", "echo", r#"{"message":"ok"}"#))
        .await
        .unwrap();
    let error = tools
        .execute_call(&call("failed", "echo", r#"{"message":"fail"}"#))
        .await
        .unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    match error {
        ToolError::Call { call_id, source } => {
            assert_eq!(call_id, "failed");
            assert!(matches!(
                *source,
                ToolError::Execution {
                    source: Error::InvalidRequest(_),
                    ..
                }
            ));
        }
        other => panic!("unexpected {other}"),
    }
    assert_eq!(first.content, r#""ok""#);
    assert!(
        matches!(tools.execute("missing", "{}").await, Err(ToolError::Unknown { name }) if name == "missing")
    );
    for model_call in [
        call("missing-id", "missing", "{}"),
        call("bad-json", "echo", "{"),
    ] {
        let expected = serde_json::to_value(&model_call).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            matches!(tools.execute_call(&model_call).await, Err(ToolError::Call { call_id, .. }) if call_id == expected)
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn unsupported_and_empty_id_calls_never_execute() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tools = ToolRegistry::new();
    tools.register(Echo::new("echo", &calls)).unwrap();
    let custom = serde_json::from_value(json!({"id": "custom-id", "type": "custom", "custom": {
        "name": "echo", "input": "hello"
    }}))
    .unwrap();
    for model_call in [custom, call("", "echo", r#"{"message":"ok"}"#)] {
        match tools.execute_call(&model_call).await.unwrap_err() {
            ToolError::Call { source, .. } => {
                assert!(matches!(*source, ToolError::InvalidCall { .. }));
            }
            other => panic!("unexpected {other}"),
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

struct Dynamic;
#[async_trait]
impl FunctionTool for Dynamic {
    type Input = Value;
    type Output = Value;
    fn name(&self) -> &'static str {
        "dynamic"
    }
    fn description(&self) -> &'static str {
        "Echo any object"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": true})
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        Ok(input)
    }
}
struct BadOutput;
impl Serialize for BadOutput {
    fn serialize<S: Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("cannot encode output"))
    }
}
struct BadEncoder;
#[async_trait]
impl FunctionTool for BadEncoder {
    type Input = Value;
    type Output = BadOutput;
    fn name(&self) -> &'static str {
        "bad_encoder"
    }
    fn description(&self) -> &'static str {
        "Demonstrate output failures"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }
    async fn execute(&self, _: Value) -> Result<BadOutput> {
        Ok(BadOutput)
    }
}
#[tokio::test]
async fn dynamic_json_and_output_encoding_failures() {
    let mut tools = ToolRegistry::default();
    assert!(tools.tool_definitions().is_empty());
    tools.register(Dynamic).unwrap();
    tools.register(BadEncoder).unwrap();
    let input = json!({"nested": {"items": [true, null, 3]}});
    assert_eq!(
        tools.execute("dynamic", &input.to_string()).await.unwrap(),
        input
    );
    assert!(matches!(
        tools.execute("dynamic", "[]").await,
        Err(ToolError::InvalidArguments { .. })
    ));
    assert!(matches!(
        tools.execute("bad_encoder", "{}").await,
        Err(ToolError::Json {
            stage: "output encoding failed",
            ..
        })
    ));
}

struct Stateful {
    revision: Arc<AtomicUsize>,
}
#[async_trait]
impl FunctionTool for Stateful {
    type Input = Value;
    type Output = usize;
    fn name(&self) -> &'static str {
        "stateful"
    }
    fn description(&self) -> &'static str {
        "Read the current application revision"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "description": format!("Revision {}", self.revision.load(Ordering::SeqCst))})
    }
    async fn execute(&self, _: Value) -> Result<usize> {
        tokio::task::yield_now().await;
        Ok(self.revision.load(Ordering::SeqCst))
    }
}

#[tokio::test]
async fn definitions_are_snapshots_while_handler_state_remains_live() {
    let revision = Arc::new(AtomicUsize::new(1));
    let mut tools = ToolRegistry::new();
    tools
        .register(Stateful {
            revision: revision.clone(),
        })
        .unwrap();
    let before = tools.tool_definitions();
    revision.store(2, Ordering::SeqCst);
    assert_eq!(before, tools.tool_definitions());
    assert_eq!(tools.execute("stateful", "{}").await.unwrap(), 2);
    // Released model aliases remain source-compatible with the returned definitions.
    let root_model: openai_ergonomic::Tool = before[0].clone();
    let module_model: openai_ergonomic::responses::Tool = root_model;
    assert_eq!(module_model.function.name, "stateful");
    // Modifying exported definitions cannot alter subsequent exports.
    let mut exported = tools.tool_definitions();
    exported[0].function.name = "changed".into();
    assert_eq!(tools.tool_definitions()[0].function.name, "stateful");
}
