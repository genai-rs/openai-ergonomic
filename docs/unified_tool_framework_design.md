# Unified Tool Framework Design

This document captures the design decisions behind the unified tool framework introduced in the 0.6.0 development cycle.

## Goals

1. **Single mental model** – expose one public trait that covers JSON-only, typed-input, and typed-output scenarios.
2. **Ergonomic defaults** – JSON is the default, but upgrading to typed data requires minimal boilerplate.
3. **Async ready** – tool execution should remain async-friendly.
4. **Low ceremony** – offer macros and registry helpers so applications can wire tools quickly.

## Core Trait

```rust
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    type Input: serde::de::DeserializeOwned + Send;
    type Output: serde::Serialize + Send;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;
}
```

- Implementations choose their own `Input` and `Output` types; the `tool!` macro defaults to JSON when you omit `input_type`/`output_type`.
- `execute` is async to support IO-bound workloads.

## Registry Erasure

`ToolRegistry` stores tools behind an internal trait object:

```rust
#[async_trait::async_trait]
trait ErasedTool: Send + Sync {
    fn definition(&self) -> ChatCompletionTool;
    async fn execute(&self, args: &str) -> Result<serde_json::Value>;
}

impl<T> ErasedTool for T
where
    T: Tool + 'static,
{ /* deserialize -> execute -> serialize */ }
```

This keeps the public API simple:

```rust
let registry = ToolRegistry::new().register(MyTool);
let defs = registry.tool_definitions();
let json = registry.execute("my_tool", r#"{"foo":"bar"}"#).await?;
```

`process_tool_calls` iterates the tool calls returned in a `ChatCompletionResponseWrapper`, executes them, and returns `(tool_call_id, json)` tuples for easy reenqueuing.

## Macro Strategy

The framework exposes two macros:

- `tool_schema!` – JSON schema shorthand for required/optional parameters.
- `tool!` – declares a struct + `Tool` implementation in one block.

Example:

```rust
#[derive(Deserialize)]
pub struct SearchParams { query: String }

tool! {
    pub struct SearchTool;

    name: "search";
    description: "Search indexed documents";
    input_type: SearchParams;
    schema: tool_schema!(
        query: "string", "Query to run", required: true,
    );

    async fn handle(params: SearchParams) -> Result<serde_json::Value> {
        Ok(serde_json::json!({ "results": [] }))
    }
}
```

- `input_type` defaults to the handler argument type; specify it only when you need something different.
- `output_type` defaults to `serde_json::Value` and should be set when returning native structs.
- `schema` defaults to an empty schema.

## Follow-ups

The redesigned framework includes `ToolRegistry::process_tool_calls_into_builder`, which appends executed tool results back into chat requests to reduce boilerplate. Future enhancements may still layer additional orchestration helpers on top of it.

## Alternatives Considered

- **Multiple traits (`Tool` / `TypedTool` / `StronglyTypedTool`)**: rejected because it introduced decision paralysis and extra adapters.
- **Builder-style API instead of macros**: viable, but macro ergonomics felt closer to the original design while still being opt-in.
- **Compile-time duplicate name detection**: left as future work; runtime overwrite keeps the API straightforward for now.
