# Function tools

Define a tool by implementing `FunctionTool`, register instances in `ToolRegistry`, then dispatch approved model calls with `execute_call`. The same trait supports typed inputs/outputs, application state, and dynamic JSON. This module targets Chat Completions function tools, not Responses API tools or custom text tools.

## Start with a working example

Run either example without an API key:

```sh
cargo run --example tool_framework
cargo run --example tool_framework_typed
```

The [simple example](../examples/tool_framework.rs) defines a typed search tool with a required query and optional limit. The [chat example](../examples/tool_framework_typed.rs) registers two tools, reads shared application state asynchronously, and builds a complete assistant/tool exchange using actual client models. The crate's [`tool_framework` rustdoc](../src/tool_framework.rs) also contains a compiling minimal greeting tool.

Implementations write `execute` as a plain `async fn` (no attribute macro; natively the returned future must be `Send`, on WebAssembly nothing needs to be: the bounds are `MaybeSend`/`MaybeSync`), use Serde input/output types, and `serde_json::json!` for the schema. Add `serde` (with `derive`) and `serde_json` as direct application dependencies. Store clients or `Arc` state in your tool struct; `execute(&self, input)` can await I/O. The handler returns the crate's `Result<Output>`. Use `Error::application(domain_error)` to preserve custom error sources. Registry errors convert into the crate's `Error`, so `?` works in a function returning `openai_ergonomic::Result`.

```rust
use openai_ergonomic::{FunctionTool, Result, ToolRegistry};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchInput {
    query: String,
    limit: Option<u32>,
}

struct Search;

impl FunctionTool for Search {
    type Input = SearchInput;
    type Output = Vec<String>;

    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search the documentation and return matching page titles."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "What to look for"},
                "limit": {"type": ["integer", "null"], "description": "At most this many results"}
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: SearchInput) -> Result<Vec<String>> {
        let limit = input.limit.unwrap_or(3) as usize;
        Ok(vec![format!("Results for {}", input.query)].into_iter().take(limit).collect())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut tools = ToolRegistry::new();
    tools.register(Search)?;
    let definitions = tools.tool_definitions(); // pass these to `ChatCompletionBuilder::tools`
    assert_eq!(definitions.len(), 1);
    let value = tools.execute("search", r#"{"query":"Rust"}"#).await?;
    assert_eq!(value, json!(["Results for Rust"]));
    Ok(())
}
```

`register` mutates the registry and returns `Result<(), ToolError>`; handle errors with `?`. It rejects duplicates without replacing the original instance. Names must be 1–64 ASCII letters, digits, underscores or hyphens; descriptions must be nonblank; schemas must declare an object root. Definitions are captured once and returned in alphabetical name order. The client model stores schema parameters in a `HashMap`, so serialized key ordering is not guaranteed.

`Error` has `Tool` (a `ToolError`) and `Application` (your handler's own error) variants.

## Schemas and arguments

Write schemas for the model, with a clear purpose and property descriptions. Keep them aligned with the Serde input type:

- List fields without defaults in `required`.
- For `Option<T>`, omit the field from `required` and allow `null` if your interface accepts it. Describe what omission/null means.
- Pair `additionalProperties: false` with `#[serde(deny_unknown_fields)]` when extra fields should be errors.
- Express enums, bounds and nested shapes in standard JSON Schema. Also enforce application constraints in Rust; the registry does not run a JSON Schema validator.

No schema is inferred and strict mode is not enabled. Registration checks only the object root, not full schema validity or equivalence to `Input`. The framework parses JSON objects and then uses Serde to decode the original argument string, preserving Serde's rejection of duplicate struct fields. Malformed JSON, missing required fields and incompatible types return errors before the handler runs. Defaults, custom deserializers and unknown-field behavior follow your Serde configuration. Dynamic `Value` inputs follow Serde JSON semantics, including last-value handling of duplicate keys; validate stricter constraints yourself when needed.

For a zero-argument tool, use an empty `#[derive(Deserialize)] struct NoArgs {}` with an object schema and empty properties. Use `#[serde(deny_unknown_fields)]` if no fields should be accepted. `()` expects JSON null and therefore does not accept the `{}` argument object. Empty argument strings are invalid JSON and are not silently converted to `{}`.

`type Input = serde_json::Value` and/or `type Output = serde_json::Value` provide the dynamic escape hatch without a second trait. Dynamic inputs must still be JSON objects. Output is encoded using Serde JSON, including quotes around strings; non-finite floats follow Serde JSON's null encoding. Choose output types and validation accordingly.

## Execute a model call and reply

Keep the earlier conversation, append the assistant's calls, then append one tool reply per call:

```rust,no_run
use openai_ergonomic::{ChatCompletionBuilder, Client, ToolRegistry};

async fn answer_calls(
    client: &Client,
    tools: &ToolRegistry,
    mut builder: ChatCompletionBuilder,
) -> openai_ergonomic::Result<String> {
    builder = builder.tools(tools.tool_definitions());
    let response = client.send_chat(builder.clone()).await?;
    let calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
    if calls.is_empty() {
        return Ok(response.content().unwrap_or_default().to_string());
    }
    builder = builder.assistant_with_tool_calls(response.content().unwrap_or_default(), calls.clone());
    for call in &calls {
        // Apply application authorization before dispatching.
        let output = tools.execute_call(call).await?;
        builder = builder.tool(output.call_id, output.content);
    }
    let response = client.send_chat(builder).await?;
    Ok(response.content().unwrap_or_default().to_string())
}
```

`ToolOutput` contains `call_id` and JSON `content`. `execute_call` rejects non-function calls and empty IDs before running a handler. On failure, `ToolError::Call { call_id, source }` preserves the original ID and underlying error; errors identify the tool and retain Serde/application sources. `execute(name, arguments)` is the lower-level API returning a JSON `Value` without call context.

The snippet stops on error with `?`. For a model-visible error reply and continuation, use the complete chat example: match each dispatch result, keep successful replies, and send an application-selected error payload. Do not blindly expose raw errors that may contain private application data. For a denied call, the application can supply its own reply without executing it.

There is no automatic batch execution, approval, retry, rollback, timeout or panic handling. Decide these in the application. Preserve successful results if a later call fails; retrying a whole response can repeat external effects. Use the first-choice calls exposed by the response wrapper, or explicitly select another choice yourself. Avoid replaying the same response twice. For responses without calls, handle the assistant content normally.

## Design

`FunctionTool` and the existing `Tool` alias (the generated `ChatCompletionTool` definition) are
different things: `Tool` stays the definition type, the executable trait is `FunctionTool`. The
explicit trait costs a few metadata methods but supports state naturally, uses familiar compiler
diagnostics and avoids repeating handler types in a macro DSL. Explicit dispatch keeps control
over side effects and error policy. Schemas remain manual to avoid a new mandatory dependency;
their limits are documented and covered by examples and tests. See the
[design notes](unified_tool_framework_design.md).
