//! Typed, asynchronous function tools for the Chat Completions API.
//!
//! Implement [`FunctionTool`], register instances with [`ToolRegistry::register`], and
//! pass [`ToolRegistry::tool_definitions`] to your chat builder. Dispatch each
//! approved call with [`ToolRegistry::execute_call`]. Registration grants no
//! execution authority; the application decides which calls to execute.
//!
//! ```
//! use async_trait::async_trait;
//! use openai_ergonomic::{FunctionTool, ToolRegistry, Result};
//! use serde::Deserialize;
//! use serde_json::{json, Value};
//!
//! #[derive(Deserialize)]
//! #[serde(deny_unknown_fields)]
//! struct Greeting { name: String }
//! struct Greet;
//! #[async_trait]
//! impl FunctionTool for Greet {
//!     type Input = Greeting;
//!     type Output = String;
//!     fn name(&self) -> &str { "greet" }
//!     fn description(&self) -> &str { "Return a greeting for a person." }
//!     fn parameters_schema(&self) -> Value {
//!         json!({"type": "object", "properties": {
//!             "name": {"type": "string", "description": "Person to greet"}
//!         }, "required": ["name"], "additionalProperties": false})
//!     }
//!     async fn execute(&self, input: Greeting) -> Result<String> {
//!         Ok(format!("Hello, {}!", input.name))
//!     }
//! }
//! # #[tokio::main]
//! # async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! let mut tools = ToolRegistry::new();
//! tools.register(Greet)?;
//! let result = tools.execute("greet", r#"{"name":"Tim"}"#).await?;
//! assert_eq!(result, "Hello, Tim!");
//! # Ok(())
//! # }
//! ```

use std::collections::{btree_map::Entry, BTreeMap};

use async_trait::async_trait;
use openai_client_base::models::{ChatCompletionMessageToolCallsInner, ChatCompletionTool};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{builders::chat::tool_function, responses::chat::ToolCallExt, Error, Result};

/// A function tool. Store application state (for example an `Arc` or HTTP client)
/// in the implementing struct and access it through `&self` during execution.
#[async_trait]
pub trait FunctionTool: Send + Sync {
    /// Arguments decoded with Serde. Use `Value` for a dynamic JSON tool.
    type Input: DeserializeOwned + Send;
    /// Result encoded as JSON, including quotes for string outputs.
    type Output: Serialize + Send;
    /// Stable name: 1–64 ASCII letters, digits, underscores or hyphens.
    fn name(&self) -> &str;
    /// Describe what the tool does and when the model should use it.
    fn description(&self) -> &str;
    /// An object JSON Schema matching `Input`, including property descriptions.
    ///
    /// The registry checks only the root object type, not full JSON Schema
    /// validity or conformance. Serde handles argument decoding; constraints such
    /// as numeric bounds must also be enforced by the implementation. Strict mode
    /// is not enabled. Optional fields may be omitted. For exact object fields,
    /// pair `additionalProperties: false` with `#[serde(deny_unknown_fields)]`.
    fn parameters_schema(&self) -> Value;
    /// Execute after the caller has made any required authorization decisions.
    /// Errors remain local unless the application chooses to send them to a model.
    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;
}

/// Registration or execution failure, with the tool name and original source.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// An existing registration was retained.
    #[error("Tool {name:?} is already registered")]
    Duplicate {
        /// Conflicting tool name.
        name: String,
    },
    /// Tool metadata is unsuitable for a function definition.
    #[error("Invalid definition for tool {name:?}: {reason}")]
    InvalidDefinition {
        /// Tool name supplied by the implementation.
        name: String,
        /// Actionable reason for rejection.
        reason: String,
    },
    /// No tool with this name is registered.
    #[error("Unknown tool {name:?}")]
    Unknown {
        /// Requested tool name.
        name: String,
    },
    /// Arguments could not be decoded, or the result could not be encoded.
    #[error("Tool {name:?} {stage}: {source}")]
    Json {
        /// Registered tool name.
        name: String,
        /// Whether argument decoding or output encoding failed.
        stage: &'static str,
        /// Original Serde error.
        #[source]
        source: serde_json::Error,
    },
    /// The arguments are JSON but not an object.
    #[error("Tool {name:?} arguments must be a JSON object")]
    InvalidArguments {
        /// Registered tool name.
        name: String,
    },
    /// The tool handler returned an error.
    #[error("Tool {name:?} execution failed: {source}")]
    Execution {
        /// Registered tool name.
        name: String,
        /// Original application error.
        #[source]
        source: Error,
    },
    /// The call cannot be dispatched as a function call.
    #[error("Invalid tool call: {reason}")]
    InvalidCall {
        /// Actionable reason for rejection.
        reason: String,
    },
    /// Call context retained on dispatch failure, including unsupported calls.
    #[error("Tool call {call_id:?} failed: {source}")]
    Call {
        /// Identifier supplied by the model.
        call_id: String,
        /// Underlying dispatch or execution failure.
        #[source]
        source: Box<Self>,
    },
}

/// A successful tool reply ready for `ChatCompletionBuilder::tool`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    /// Original model-issued call identifier.
    pub call_id: String,
    /// JSON-encoded output, not debug formatting or unquoted text.
    pub content: String,
}

#[async_trait]
trait ErasedTool: Send + Sync {
    async fn execute(&self, name: &str, args: &str) -> std::result::Result<Value, ToolError>;
}

#[async_trait]
impl<T: FunctionTool> ErasedTool for T {
    async fn execute(&self, name: &str, args: &str) -> std::result::Result<Value, ToolError> {
        let json_error = |stage, source| ToolError::Json {
            name: name.into(),
            stage,
            source,
        };
        let value: Value =
            serde_json::from_str(args).map_err(|e| json_error("argument decoding failed", e))?;
        if !value.is_object() {
            return Err(ToolError::InvalidArguments { name: name.into() });
        }
        // Decode the original text so Serde still detects duplicate struct fields.
        let input =
            serde_json::from_str(args).map_err(|e| json_error("argument decoding failed", e))?;
        let output =
            FunctionTool::execute(self, input)
                .await
                .map_err(|source| ToolError::Execution {
                    name: name.into(),
                    source,
                })?;
        serde_json::to_value(output).map_err(|e| json_error("output encoding failed", e))
    }
}

struct RegisteredTool {
    definition: ChatCompletionTool,
    handler: Box<dyn ErasedTool>,
}

/// Heterogeneous tools, with definitions snapshotted at registration and returned
/// in name order for reproducible prompts. Execution never happens implicitly.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, RegisteredTool>,
}

impl ToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an instance without replacing an existing tool.
    ///
    /// Rejects duplicate names, invalid names, blank descriptions and schemas
    /// without a root `"type": "object"`. Failure leaves the registry unchanged.
    /// Metadata is captured once; later state changes do not alter definitions.
    pub fn register<T: FunctionTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        let name = tool.name().to_owned();
        let invalid = |reason: &str| ToolError::InvalidDefinition {
            name: name.clone(),
            reason: reason.into(),
        };
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(invalid(
                "name must contain 1–64 ASCII letters, digits, underscores or hyphens",
            ));
        }
        let description = tool.description();
        if description.trim().is_empty() {
            return Err(invalid("description must not be blank"));
        }
        let schema = tool.parameters_schema();
        if !schema.is_object() || schema.get("type").and_then(Value::as_str) != Some("object") {
            return Err(invalid("parameters schema must have root type object"));
        }
        let definition = tool_function(&name, description, schema);
        match self.tools.entry(name) {
            Entry::Occupied(entry) => Err(ToolError::Duplicate {
                name: entry.key().clone(),
            }),
            Entry::Vacant(entry) => {
                entry.insert(RegisteredTool {
                    definition,
                    handler: Box::new(tool),
                });
                Ok(())
            }
        }
    }

    /// Return function definitions suitable for `ChatCompletionBuilder::tools`.
    #[must_use]
    pub fn tool_definitions(&self) -> Vec<ChatCompletionTool> {
        self.tools
            .values()
            .map(|tool| tool.definition.clone())
            .collect()
    }

    /// Dispatch by name. Arguments must be a JSON object matching `FunctionTool::Input`.
    /// Returns JSON output; does not retry, catch panics, or enforce timeouts.
    pub async fn execute(
        &self,
        name: &str,
        arguments: &str,
    ) -> std::result::Result<Value, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::Unknown { name: name.into() })?;
        tool.handler.execute(name, arguments).await
    }

    /// Execute one approved model-issued function call and retain its identifier.
    ///
    /// On error, `ToolError::Call` retains the identifier and original error. No
    /// automatic batch execution is provided: callers choose error reporting,
    /// ordering, concurrency and authorization per call, and retain earlier results.
    /// Custom (non-function) calls and empty identifiers are rejected before execution.
    pub async fn execute_call(
        &self,
        call: &ChatCompletionMessageToolCallsInner,
    ) -> std::result::Result<ToolOutput, ToolError> {
        let result = async {
            if call.id().is_empty() {
                return Err(ToolError::InvalidCall {
                    reason: "call identifier must not be empty".into(),
                });
            }
            let ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(function_call) =
                call
            else {
                return Err(ToolError::InvalidCall {
                    reason: "only function tool calls are supported".into(),
                });
            };
            let output = self
                .execute(
                    &function_call.function.name,
                    &function_call.function.arguments,
                )
                .await?;
            Ok(ToolOutput {
                call_id: call.id().into(),
                content: output.to_string(),
            })
        }
        .await;
        result.map_err(|source| ToolError::Call {
            call_id: call.id().into(),
            source: Box::new(source),
        })
    }
}
