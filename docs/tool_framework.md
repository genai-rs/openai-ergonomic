# Tools

Define a tool by implementing `FunctionTool`, register instances in `ToolRegistry`, then answer approved model calls with `answer_call` (or dispatch them with `execute_call`). The same trait supports typed inputs/outputs, application state, and dynamic JSON. The registry serves both the Chat Completions API (`tool_definitions`) and the Responses API (`responses_tools`), and also holds tools that return images or files (`RichTool`), freeform-text tools (`CustomTool`), namespaces and tools only known at runtime (`register_dynamic`).

## Start with a working example

Run the examples without an API key:

```sh
cargo run --example tool_framework
cargo run --example tool_framework_typed
cargo run --example tool_framework_responses
```

The [simple example](../examples/tool_framework.rs) defines a typed search tool with a required query and optional limit. The [chat example](../examples/tool_framework_typed.rs) registers two tools, reads shared application state asynchronously, and builds a complete assistant/tool exchange using actual client models. The [Responses example](../examples/tool_framework_responses.rs) answers function and custom tool calls of a Responses API reply, with a rich output, a namespace and a dynamic tool. The crate's [`tool_framework` rustdoc](../src/tool_framework/mod.rs) also contains a compiling minimal greeting tool.

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

The snippet stops on error with `?`. For a model-visible error reply and continuation, use `answer_call` (below), or match each dispatch result, keep successful replies, and send an application-selected error payload as the complete chat example does. Do not blindly expose raw errors that may contain private application data. For a denied call, the application can supply its own reply without executing it.

There is no automatic batch execution, approval, retry, rollback, timeout or panic handling. Decide these in the application. Preserve successful results if a later call fails; retrying a whole response can repeat external effects. Use the first-choice calls exposed by the response wrapper, or explicitly select another choice yourself. Avoid replaying the same response twice. For responses without calls, handle the assistant content normally.

## Answer every call

`answer_call` runs one call of either API and always gives the model something back: the tool's
output, or, when the call fails in a way the model can act on, the failure as the output with
`success: false`. Only a fatal failure (or a call without an id) is returned as an error, so a
conversation never ends up with a call that has no answer (which the API rejects).

`ToolError::classify` decides, and `answer_call` applies it:

| Failure | The model sees |
| --- | --- |
| arguments that are not JSON or not the tool's input | `failed to parse function arguments: …` |
| arguments that are not an object | `function arguments must be a JSON object` |
| an unknown tool (or one outside its namespace) | `unsupported call: <name>` |
| a function call to a custom tool, or the reverse; an unsupported call type | what to send instead |
| the handler's error (`ToolError::Execution`) | the error's message |
| a handler that returned `Error::fatal(..)`, an output that cannot be encoded | nothing: `Err(ToolError::Call { .. })` |

The model sees the message of the handler's error: return errors whose message is fit for it, or
classify yourself with `answer_call_with` (to hide the details, say). `CallAnswer` has the
`call_id`, the `kind` of call, the `output` (a `ToolOutputBody`) and `success`, which is not sent.
`CallAnswer::aborted(&call)` answers a call you decided not to run.

```rust,no_run
use openai_ergonomic::{ChatCompletionBuilder, Client, ToolError, ToolFailure, ToolRegistry};

async fn answer_calls(
    client: &Client,
    tools: &ToolRegistry,
    builder: ChatCompletionBuilder,
) -> openai_ergonomic::Result<String> {
    let builder = builder.tools(tools.tool_definitions());
    let response = client.send_chat(builder.clone()).await?;
    let calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
    let mut next = builder.assistant_with_tool_calls(response.content().unwrap_or_default(), calls.clone());
    for call in &calls {
        // Apply application authorization before dispatching.
        let answer = tools
            .answer_call_with(call, |error| match error {
                ToolError::Execution { .. } => ToolFailure::RespondToModel("the tool failed".into()),
                other => other.classify(),
            })
            .await?;
        next = next.call_answer(answer)?;
    }
    let response = client.send_chat(next).await?;
    Ok(response.content().unwrap_or_default().to_string())
}
```

`ChatCompletionBuilder::call_answer` appends the tool message; it fails for an output with images
or files, which chat tool messages cannot carry.

## The Responses API

`responses_tools()` gives the registry's tools as Responses API tools: function and custom tools,
then one `namespace` tool per namespace. `ResponsesResponse::tool_invocations()` gives the model's
function and custom tool calls, and `ResponsesBuilder::call_answer` appends each answer as a
`function_call_output` (or `custom_tool_call_output`) item:

```rust,no_run
use openai_ergonomic::{Client, ToolRegistry};

async fn run(client: &Client, tools: &ToolRegistry) -> openai_ergonomic::Result<String> {
    let builder = client
        .responses()
        .tools(tools.responses_tools())
        .store(false)
        .user("What is the weather in Ghent?");
    let mut response = client.send_responses(builder.clone()).await?;
    let mut builder = builder;
    // At most a few rounds: never loop on a model that keeps calling tools.
    for _ in 0..5 {
        let calls = response.tool_invocations();
        if calls.is_empty() {
            break;
        }
        // Without stored responses, send the model's output back before the answers.
        builder = builder.input_items(response.output_as_input()?);
        for call in &calls {
            builder = builder.call_answer(tools.answer_call(call).await?);
        }
        response = client.send_responses(builder.clone()).await?;
    }
    Ok(response.output_text())
}
```

With stored responses, send only the answers with `previous_response_id(response.id())`. While
streaming, `ResponsesEvent::tool_invocation()` gives a call as soon as its
`response.output_item.done` event arrives, so it can run before the response ends.
`ResponsesBuilder::tool_output` appends a `ToolOutput` of `execute_call`.

## Images and files: rich tools

A `RichTool` is a function tool that returns a `ToolOutputBody`: text, or content items
(`OutputContent::text`, `image_url`, `image_file`, `file_id`, `file_url`, `file_data`). Register
it with `register_rich`. The Responses API takes the items as the call's output; Chat Completions
tool messages take text only (`to_chat_text` fails for images and files).

```rust
use openai_ergonomic::{OutputContent, Result, RichTool, ToolOutputBody, ToolRegistry};
use serde_json::{json, Value};

struct Screenshot;

impl RichTool for Screenshot {
    type Input = Value;
    fn name(&self) -> &str { "screenshot" }
    fn description(&self) -> &str { "Take a screenshot of the dashboard" }
    fn parameters_schema(&self) -> Value { json!({"type": "object", "properties": {}}) }
    async fn execute(&self, _input: Value) -> Result<ToolOutputBody> {
        Ok(vec![
            OutputContent::text("The dashboard right now:"),
            OutputContent::image_url("data:image/png;base64,iVBORw0KGgo="),
        ]
        .into())
    }
}

let mut tools = ToolRegistry::new();
tools.register_rich(Screenshot)?;
# Ok::<(), openai_ergonomic::ToolError>(())
```

## Custom tools and namespaces

A `CustomTool` takes freeform text instead of JSON arguments (a patch, a query, a program),
optionally constrained by a grammar: `CustomFormat::lark(..)` or `CustomFormat::regex(..)`.
`register_custom` registers it; it is advertised to the Responses API only.

`namespace(name, description)` groups tools for the Responses API; the model calls them with the
namespace on the call. A namespace and a tool cannot share a name; `tool_definitions()` (chat)
leaves namespaced and custom tools out.

```rust
use openai_ergonomic::{CustomFormat, CustomTool, Result, ToolInvocation, ToolOutputBody, ToolRegistry};

struct ApplyPatch;

impl CustomTool for ApplyPatch {
    fn name(&self) -> &str { "apply_patch" }
    fn description(&self) -> &str { "Apply a patch in the V4A diff format" }
    fn format(&self) -> CustomFormat { CustomFormat::lark(r#"start: "*** Begin Patch" /(.|\n)*/"#) }
    async fn execute(&self, input: String) -> Result<ToolOutputBody> {
        Ok(ToolOutputBody::text(format!("applied {} bytes", input.len())))
    }
}

# #[tokio::main]
# async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let mut tools = ToolRegistry::new();
tools.namespace("workspace", "Edit the workspace")?.register_custom(ApplyPatch)?;
let call = ToolInvocation::custom("call_1", "apply_patch", "*** Begin Patch").in_namespace("workspace");
let answer = tools.answer_call(&call).await?;
assert_eq!(answer.output.as_text(), Some("applied 15 bytes"));
assert_eq!(tools.responses_tools().len(), 1); // the namespace
# Ok(())
# }
```

## Tools known at runtime

Tools of an MCP server or a plugin come with names and schemas that are not written for the API.
`register_dynamic(DynamicTool::new(name, description, schema, handler))` adapts them instead of
rejecting them, and returns the name the model sees:

- the name, with an optional `prefix` (`server__tool`), is sanitized (`sanitize_tool_name`: every
  character other than an ASCII letter, digit or `_` becomes `_`); a name that is too long or
  taken gets a stable hash suffix (`ToolNamer`); `raw_name(name)` gives the original back;
- the schema is sanitized (`schema::sanitize`: `true`/`false` schemas, `const`, missing types,
  unreachable definitions), compacted above 5 000 bytes (`schema::compact`: descriptions,
  definitions, deep nesting, compositions) and replaced with one that accepts any object if it is
  still above 8 000 bytes.

```rust
use openai_ergonomic::{DynamicTool, ToolOutputBody, ToolRegistry};
use serde_json::{json, Value};

let mut tools = ToolRegistry::new();
let name = tools.register_dynamic(
    DynamicTool::new(
        "create-issue",
        "Create an issue",
        json!({"properties": {"title": {"type": "string"}}}),
        |arguments: Value| async move {
            Ok(ToolOutputBody::text(format!("created {}", arguments["title"])))
        },
    )
    .prefix("github"),
)?;
assert_eq!(name, "github__create_issue");
assert_eq!(tools.raw_name(&name), Some("create-issue"));
# Ok::<(), openai_ergonomic::ToolError>(())
```

The sanitizers are public (`tool_framework::{sanitize_tool_name, ToolNamer, schema}`) for code
that builds definitions itself.

## Output budgets and parallel tools

A large output costs context. `truncate_middle(text, budget)` keeps the head and the tail of a
text and replaces the middle with `…N chars truncated…`; `CallAnswer::truncated(budget, header)`
and `ToolOutputBody::truncated` apply it to an answer (images and files are kept, a header line
says how long the output was). `TruncationConfig` holds a default budget (10 000 bytes) and
`budget_for(tools.output_budget(&call))` picks a tool's own budget, which it sets with
`FunctionTool::output_budget`. `answer_call` itself never truncates.

`FunctionTool::supports_parallel` (default `false`) says whether a tool may run at the same time
as other tools that allow it; `tools.supports_parallel(&call)` reads it. The registry does not run
anything concurrently: run calls of parallel-safe tools together (for example with
`futures::future::join_all`), the others one at a time, and append the answers in the order of
the calls.

## Design

`FunctionTool` and the existing `Tool` alias (the generated `ChatCompletionTool` definition) are
different things: `Tool` stays the definition type, the executable trait is `FunctionTool`. The
explicit trait costs a few metadata methods but supports state naturally, uses familiar compiler
diagnostics and avoids repeating handler types in a macro DSL. Explicit dispatch keeps control
over side effects and error policy. Schemas remain manual to avoid a new mandatory dependency;
their limits are documented and covered by examples and tests. See the
[design notes](unified_tool_framework_design.md).
