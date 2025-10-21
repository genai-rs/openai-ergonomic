# Tool Framework

`openai-ergonomic` ships a lightweight framework for defining, registering, and executing OpenAI Chat API tools. It exposes a single [`Tool`](../src/tool_framework.rs) trait with customizable input and output types, a [`ToolRegistry`](../src/tool_framework.rs), and helper macros to eliminate boilerplate.

## Quick Start

```rust
use openai_ergonomic::{
    tool,
    tool_framework::ToolRegistry,
    tool_schema,
    Result,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WeatherParams {
    location: String,
    #[serde(default)]
    units: Option<String>,
}

tool! {
    pub struct WeatherTool;

    name: "get_weather";
    description: "Fetch current weather conditions";
    input_type: WeatherParams;
    schema: tool_schema!(
        location: "string", "City or postal code", required: true,
        units: "string", "Measurement units (metric/imperial)", required: false,
    );

    async fn handle(params: WeatherParams) -> Result<serde_json::Value> {
        let units = params.units.unwrap_or_else(|| "metric".into());
        Ok(serde_json::json!({
            "location": params.location,
            "units": units,
            "temperature": 21.4,
            "conditions": "Partly cloudy"
        }))
    }
}

# tokio_test::block_on(async {
let registry = ToolRegistry::new().register(WeatherTool);
let response = registry
    .execute("get_weather", r#"{"location":"Brussels"}"#)
    .await?;

assert_eq!(response["location"], "Brussels");
# Result::<()>::Ok(())
# })?;
```

The macro expands to a struct and a `Tool` implementation with the appropriate associated types.

## Trait Overview

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

- **Input type**: specify the handler argument type (the `tool!` macro defaults to the handler parameter when `input_type` is omitted).
- **Output type**: specify the return type you want serialized (the `tool!` macro defaults to `serde_json::Value` when `output_type` is omitted).
- **`execute`**: async by design so your tool implementation can call APIs, databases, etc.

## Tool Registry

```rust
let registry = ToolRegistry::new()
    .register(WeatherTool)
    .register(TimeTool);

// Send tool definitions with your chat request
let definitions = registry.tool_definitions();

// Execute a single call
let json = registry.execute("get_weather", r#"{"location":"Berlin"}"#).await?;

// Handle every tool call returned by a response
let tool_results = registry.process_tool_calls(&response).await?;
```

- `register` returns the registry for builder-style chaining.
- `register_mut` mutates in-place if you already have a `ToolRegistry`.
- `execute_to_string` is a convenience for the raw JSON string expected by the Chat API.
- `process_tool_calls` finds tool calls in a `ChatCompletionResponseWrapper`, executes them, and returns `(tool_call_id, json)` tuples. You can attach these JSON strings to your chat request via `assistant_with_tool_calls`.

## Macro Reference

### `tool_schema!`

Creates JSON Schema fragments for tool parameters.

```rust
let schema = tool_schema!(
    query: "string", "Search query", required: true,
    limit: "integer", "Maximum results", required: false,
);
```

### `tool!`

> Note: If you export the generated tool (e.g. `pub struct MyTool`), make the associated input/output types `pub` as well so they remain visible outside the module.

```rust
tool! {
    pub struct SearchTool;

    name: "search";
    description: "Search indexed documents";
    input_type: SearchParams;        // Optional (defaults to handler argument type)
    output_type: SearchResult;       // Optional (defaults to serde_json::Value)
    schema: tool_schema!( ... );     // Optional (defaults to empty schema)

    async fn handle(params: SearchParams) -> Result<SearchResult> {
        /* ... */
    }
}
```

Fields:
- `name` and `description` feed directly into the OpenAI tool definition.
- `input_type` and `output_type` are optional helpers when you want the type to differ from the handler signature defaults.
- `schema` lets you provide a custom JSON schema expression; omit it to use an empty schema.

The macro returns the struct type so you can register it immediately:

```rust
let registry = ToolRegistry::new().register(SearchTool);
```

## Typed Outputs Example

```rust
#[derive(Deserialize)]
pub struct AddParams {
    lhs: i64,
    rhs: i64,
}

#[derive(Serialize)]
pub struct AddResult {
    sum: i64,
}

tool! {
    pub struct AddTool;

    name: "add_numbers";
    description: "Add two integers";
    input_type: AddParams;
    output_type: AddResult;
    schema: tool_schema!(
        lhs: "integer", "Left operand", required: true,
        rhs: "integer", "Right operand", required: true,
    );

    async fn handle(params: AddParams) -> Result<AddResult> {
        Ok(AddResult { sum: params.lhs + params.rhs })
    }
}

let result = registry
    .execute("add_numbers", r#"{"lhs":2,"rhs":3}"#)
    .await?;
assert_eq!(result["sum"], 5);
```

## Workflow Helper (Follow-up)

`ToolRegistry::process_tool_calls` returns tool call payloads but intentionally stops short of rewiring the chat loop. The companion issue `genai-rs-28` tracks a higher-level helper that will append those tool messages back onto a chat builder, further reducing boilerplate for multi-step workflows.

## Additional Resources

- [Tool framework examples](../examples/tool_framework.rs)
- [Typed output example](../examples/tool_framework_typed.rs)
- [Tool orchestration guide](tool_orchestration.md)
- [Unified design notes](unified_tool_framework_design.md)
