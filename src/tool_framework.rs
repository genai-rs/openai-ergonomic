//! Extensible framework for defining and executing `OpenAI` tools.
//!
//! This module provides a set of traits, adapters, and helper macros that make it easy to
//! declare tools with strongly typed inputs/outputs, register them with a [`ToolRegistry`],
//! and execute tool calls returned by chat completions.
//!
//! ## Key concepts
//! - [`Tool`] – trait for any tool that consumes/produces JSON.
//! - [`TypedTool`] – derives the JSON handling automatically from an input type.
//! - [`StronglyTypedTool`] – strongly typed inputs **and** outputs.
//! - [`ToolRegistry`] – holds tool definitions and executes tool calls for you.
//! - [`tool_schema!`], [`simple_tool!`], [`strongly_typed_tool!`] – helper macros to eliminate boilerplate.
//!
//! See the crate examples (`tool_framework.rs`, `tool_framework_strongly_typed.rs`) for end-to-end usage.

use crate::Result;
use async_trait::async_trait;
use openai_client_base::models::ChatCompletionTool;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    builders::chat::tool_function,
    responses::{chat::ToolCallExt, ChatCompletionResponseWrapper},
};

/// Trait implemented by tools that can be invoked by the language model.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Machine-readable name, e.g. `get_weather`.
    fn name(&self) -> &str;

    /// Human-friendly description presented to the model.
    fn description(&self) -> &str;

    /// JSON schema describing the expected parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with raw JSON arguments, returning a JSON value.
    async fn execute(&self, params: Value) -> Result<Value>;
}

/// Trait for tools that accept strongly typed inputs but still return JSON.
#[async_trait]
pub trait TypedTool: Send + Sync {
    /// Parameter type parsed from JSON.
    type Params: DeserializeOwned + Serialize + Send;

    /// Name of the tool.
    fn name(&self) -> &str;

    /// Human description of the tool.
    fn description(&self) -> &str;

    /// Parameter schema as JSON.
    fn parameters_schema(&self) -> Value;

    /// Execute using typed params, returning JSON.
    async fn execute_typed(&self, params: Self::Params) -> Result<Value>;
}

#[async_trait]
impl<T: TypedTool> Tool for T {
    fn name(&self) -> &str {
        TypedTool::name(self)
    }

    fn description(&self) -> &str {
        TypedTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        TypedTool::parameters_schema(self)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let parsed: T::Params = serde_json::from_value(params)?;
        self.execute_typed(parsed).await
    }
}

/// Strongly typed tool with structured output.
#[async_trait]
pub trait StronglyTypedTool: Send + Sync {
    /// Input parameters.
    type Params: DeserializeOwned + Send;

    /// Output type.
    type Output: Serialize + Send;

    /// Name of the tool.
    fn name(&self) -> &str;

    /// Human description of the tool.
    fn description(&self) -> &str;

    /// JSON schema for parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute using typed params to produce typed output.
    async fn execute_strongly_typed(&self, params: Self::Params) -> Result<Self::Output>;
}

/// Adapter so a [`StronglyTypedTool`] can live inside a [`ToolRegistry`].
pub struct StronglyTypedToolAdapter<T: StronglyTypedTool> {
    tool: T,
}

impl<T: StronglyTypedTool> StronglyTypedToolAdapter<T> {
    /// Wrap a strongly typed tool for use in a registry.
    pub fn new(tool: T) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl<T: StronglyTypedTool + 'static> Tool for StronglyTypedToolAdapter<T> {
    fn name(&self) -> &str {
        self.tool.name()
    }

    fn description(&self) -> &str {
        self.tool.description()
    }

    fn parameters_schema(&self) -> Value {
        self.tool.parameters_schema()
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let typed_params: T::Params = serde_json::from_value(params)?;
        let output = self.tool.execute_strongly_typed(typed_params).await?;
        Ok(serde_json::to_value(output)?)
    }
}

/// Registry that manages a collection of tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool. Later registrations with the same name overwrite the previous entry.
    #[must_use]
    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Box::new(tool));
        self
    }

    /// Returns `OpenAI` tool definitions suitable for chat completion requests.
    pub fn tool_definitions(&self) -> Vec<ChatCompletionTool> {
        self.tools
            .values()
            .map(|tool| tool_function(tool.name(), tool.description(), tool.parameters_schema()))
            .collect()
    }

    /// Execute a tool by name using raw JSON arguments.
    pub async fn execute(&self, tool_name: &str, arguments: &str) -> Result<Value> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| crate::Error::InvalidRequest(format!("Unknown tool: {tool_name}")))?;

        let params: Value = serde_json::from_str(arguments)?;
        tool.execute(params).await
    }

    /// Execute a tool by name and return the JSON string expected by the Chat API.
    pub async fn execute_to_string(&self, tool_name: &str, arguments: &str) -> Result<String> {
        let value = self.execute(tool_name, arguments).await?;
        Ok(value.to_string())
    }

    /// Execute all tool calls present in the response, returning `(tool_call_id, json)` tuples.
    pub async fn process_tool_calls(
        &self,
        response: &ChatCompletionResponseWrapper,
    ) -> Result<Vec<(String, String)>> {
        let tool_calls = response.tool_calls();
        if tool_calls.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(tool_calls.len());
        for tool_call in tool_calls {
            let tool_name = tool_call.function_name();
            let tool_args = tool_call.function_arguments();
            let tool_id = tool_call.id().to_string();

            let result = self.execute_to_string(tool_name, tool_args).await?;
            results.push((tool_id, result));
        }

        Ok(results)
    }
}

/// Helper macro to build JSON schema objects.
#[macro_export]
macro_rules! tool_schema {
    () => {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    };
    ($($field:ident: $type:expr, $desc:expr, required: $req:expr),* $(,)?) => {{
        let mut required_fields: Vec<&str> = Vec::new();
        $(
            if $req {
                required_fields.push(stringify!($field));
            }
        )*

        serde_json::json!({
            "type": "object",
            "properties": {
                $(
                    stringify!($field): {
                        "type": $type,
                        "description": $desc
                    }
                ),*
            },
            "required": required_fields
        })
    }};
}

/// Macro to implement a simple tool with no parameters that returns a string.
#[macro_export]
macro_rules! simple_tool {
    ($name:ident, $tool_name:expr, $description:expr, $body:block) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl $crate::tool_framework::Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn parameters_schema(&self) -> serde_json::Value {
                $crate::tool_schema!()
            }

            async fn execute(
                &self,
                _params: serde_json::Value,
            ) -> $crate::Result<serde_json::Value> {
                let result: String = $body;
                Ok(serde_json::Value::String(result))
            }
        }
    };
}

/// Macro that implements [`StronglyTypedTool`] for you.
#[macro_export]
macro_rules! strongly_typed_tool {
    (
        $name:ident,
        $tool_name:expr,
        $description:expr,
        $params:ty,
        $output:ty,
        $schema:expr,
        $executor:expr
    ) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl $crate::tool_framework::StronglyTypedTool for $name {
            type Params = $params;
            type Output = $output;

            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn parameters_schema(&self) -> serde_json::Value {
                $schema
            }

            async fn execute_strongly_typed(
                &self,
                params: Self::Params,
            ) -> $crate::Result<Self::Output> {
                let executor = $executor;
                executor(params).await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unnecessary_literal_bound)]
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize)]
    struct EchoParams {
        message: String,
    }

    struct EchoTool;

    #[async_trait]
    impl TypedTool for EchoTool {
        type Params = EchoParams;

        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo back the message"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Message to echo"
                    }
                },
                "required": ["message"]
            })
        }

        async fn execute_typed(&self, params: Self::Params) -> Result<Value> {
            Ok(json!({ "message": params.message }))
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct AddParams {
        lhs: i64,
        rhs: i64,
    }

    #[derive(Debug, Serialize)]
    pub struct AddResponse {
        sum: i64,
    }

    strongly_typed_tool!(
        AddTool,
        "add_numbers",
        "Add two integers",
        AddParams,
        AddResponse,
        serde_json::json!({
            "type": "object",
            "properties": {
                "lhs": {"type": "integer"},
                "rhs": {"type": "integer"}
            },
            "required": ["lhs", "rhs"]
        }),
        |params: AddParams| async move {
            Ok(AddResponse {
                sum: params.lhs + params.rhs,
            })
        }
    );

    #[tokio::test]
    async fn registry_executes_typed_tool() {
        let registry = ToolRegistry::new().register(EchoTool);

        let result = registry
            .execute("echo", r#"{"message": "Hi"}"#)
            .await
            .unwrap();

        assert_eq!(result["message"], "Hi");
    }

    #[tokio::test]
    async fn registry_executes_strongly_typed_tool() {
        let registry = ToolRegistry::new().register(StronglyTypedToolAdapter::new(AddTool));

        let result = registry
            .execute("add_numbers", r#"{"lhs": 2, "rhs": 3}"#)
            .await
            .unwrap();

        assert_eq!(result["sum"], 5);
    }

    #[test]
    fn registry_provides_definitions() {
        let registry = ToolRegistry::new()
            .register(EchoTool)
            .register(StronglyTypedToolAdapter::new(AddTool));

        let defs = registry.tool_definitions();
        assert_eq!(defs.len(), 2);
        let names: Vec<_> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"add_numbers"));
    }
}
