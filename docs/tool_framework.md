# Function tools

Define a tool by implementing `FunctionTool`, register instances in `ToolRegistry`, then dispatch approved model calls with `execute_call`. The same trait supports typed inputs/outputs, application state, and dynamic JSON. This module targets Chat Completions function tools, not Responses API tools or custom text tools.

## Start with a working example

Run either example without an API key:

```sh
cargo run --example tool_framework
cargo run --example tool_framework_typed
```

The [simple example](../examples/tool_framework.rs) defines a typed search tool with a required query and optional limit. The [chat example](../examples/tool_framework_typed.rs) registers two tools, reads shared application state asynchronously, and builds a complete assistant/tool exchange using actual client models. The crate's [`tool_framework` rustdoc](../src/tool_framework.rs) also contains a compiling minimal greeting tool.

Implementations use `#[async_trait::async_trait]`, Serde input/output types, and `serde_json::json!` for the schema. Add `async-trait`, `serde` (with `derive`), and `serde_json` as direct application dependencies. Store clients or `Arc` state in your tool struct; `execute(&self, input)` can await I/O. The handler returns the crate's `Result<Output>`.

```rust,ignore
let mut tools = ToolRegistry::new();
tools.register(Search)?;
tools.register(Lookup { records })?;
let definitions = tools.tool_definitions();
let value = tools.execute("search", r#"{"query":"Rust"}"#).await?;
```

This fragment uses the tool types from the examples. `register` mutates the registry and returns `Result<(), ToolError>`; handle errors with `?`. It rejects duplicates without replacing the original instance. Names must be 1–64 ASCII letters, digits, underscores or hyphens; descriptions must be nonblank; schemas must declare an object root. Definitions are captured once and returned in alphabetical name order.

## Schemas and arguments

Write schemas for the model, with a clear purpose and property descriptions. Keep them aligned with the Serde input type:

- List fields without defaults in `required`.
- For `Option<T>`, omit the field from `required` and allow `null` if your interface accepts it. Describe what omission/null means.
- Pair `additionalProperties: false` with `#[serde(deny_unknown_fields)]` when extra fields should be errors.
- Express enums, bounds and nested shapes in standard JSON Schema. Also enforce application constraints in Rust; the registry does not run a JSON Schema validator.

No schema is inferred and strict mode is not enabled. Registration checks only the object root, not full schema validity or equivalence to `Input`. The framework parses JSON objects and then uses Serde to decode the original argument string, preserving Serde's rejection of duplicate struct fields. Malformed JSON, missing required fields and incompatible types return errors before the handler runs. Defaults, custom deserializers and unknown-field behavior follow your Serde configuration. Dynamic `Value` inputs follow Serde JSON semantics, including last-value handling of duplicate keys; validate stricter constraints yourself when needed.

`type Input = serde_json::Value` and/or `type Output = serde_json::Value` provide the dynamic escape hatch without a second trait. Dynamic inputs must still be JSON objects. Output is encoded using Serde JSON, including quotes around strings; non-finite floats follow Serde JSON's null encoding. Choose output types and validation accordingly.

## Execute a model call and reply

Keep the earlier conversation, append the assistant's calls, then append one tool reply per call. This fragment assumes `builder` is the request history used to obtain `response`:

```rust,ignore
let calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
if !calls.is_empty() {
    builder = builder.assistant_with_tool_calls(
        response.content().unwrap_or_default(), calls.clone(),
    );
    for call in &calls {
        // Apply application authorization before dispatching.
        let output = tools.execute_call(call).await?;
        builder = builder.tool(output.call_id, output.content);
    }
}
// Send builder.build()? through your existing chat client.
```

`ToolOutput` contains `call_id` and JSON `content`. `execute_call` rejects non-function calls and empty IDs before running a handler. On failure, `ToolError::Call { call_id, source }` preserves the original ID and underlying error; errors identify the tool and retain Serde/application sources. `execute(name, arguments)` is the lower-level API returning a JSON `Value` without call context.

The snippet stops on error with `?`. For a model-visible error reply and continuation, use the complete chat example: match each dispatch result, keep successful replies, and send an application-selected error payload. Do not blindly expose raw errors that may contain private application data. For a denied call, the application can supply its own reply without executing it.

There is no automatic batch execution, approval, retry, rollback, timeout or panic handling. Decide these in the application. Preserve successful results if a later call fails; retrying a whole response can repeat external effects. Use the first-choice calls exposed by the response wrapper, or explicitly select another choice yourself. Avoid replaying the same response twice. For responses without calls, handle the assistant content normally.

## Migration from the earlier PR drafts

This is an unreleased feature; existing released chat builders and helpers retain their behavior. The existing root `Tool` and `responses::Tool` aliases still mean the client definition type; the executable trait is named `FunctionTool` to avoid breaking those imports.

| Earlier draft | Current API |
| --- | --- |
| `tool!` and `tool_schema!` macros | Ordinary `impl FunctionTool` and `serde_json::json!` |
| `Tool`, `TypedTool`, `StronglyTypedTool` in #70 | One `FunctionTool` with associated `Input` and `Output` |
| Chained `register` / `register_mut` | `let mut tools = ToolRegistry::new(); tools.register(tool)?;` |
| Duplicate name replaces an instance | `ToolError::Duplicate`, original retained |
| `execute_to_string` | `execute_call` for model calls; `execute(...).await?.to_string()` otherwise |
| `process_tool_calls` / `process_tool_calls_into_builder` | Loop over calls and use `execute_call`; retain per-call results |

The explicit trait costs a few metadata methods but supports state naturally, uses familiar compiler diagnostics and avoids repeating handler types in a macro DSL. Explicit dispatch keeps control over side effects and error policy. Schemas remain manual to avoid a new mandatory dependency; their limits are documented and covered by examples/tests. See [design notes](unified_tool_framework_design.md).
