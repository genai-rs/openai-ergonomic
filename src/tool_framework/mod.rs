//! Typed, asynchronous tools for the Chat Completions and the Responses API.
//!
//! Implement [`FunctionTool`], register instances with [`ToolRegistry::register`], and pass
//! [`ToolRegistry::tool_definitions`] to your chat builder or [`ToolRegistry::responses_tools`]
//! to a [`ResponsesBuilder`](crate::ResponsesBuilder). Answer each approved call with
//! [`ToolRegistry::answer_call`], which turns failures the model can act on (bad arguments, an
//! unknown tool, a handler error) into an answer for the model, or dispatch it yourself with
//! [`ToolRegistry::execute_call`]. Registration grants no execution authority; the application
//! decides which calls to execute.
//!
//! Beyond JSON function tools: [`RichTool`]s return images and files, [`CustomTool`]s take
//! freeform text (optionally constrained by a grammar), [`ToolRegistry::namespace`] groups tools
//! (Responses API), and [`ToolRegistry::register_dynamic`] registers tools only known at
//! runtime, with their names and schemas cleaned up ([`sanitize_tool_name`], [`schema`]).
//!
//! ```
//! use openai_ergonomic::{FunctionTool, ToolRegistry, Result};
//! use serde::Deserialize;
//! use serde_json::{json, Value};
//!
//! #[derive(Deserialize)]
//! #[serde(deny_unknown_fields)]
//! struct Greeting { name: String }
//! struct Greet;
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
use std::future::Future;

use openai_client_base::models::{ChatCompletionMessageToolCallsInner, ChatCompletionTool};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

use crate::{
    builders::chat::tool_function, responses::chat::ToolCallExt, responses_api::ResponsesTool,
    BoxFuture, Error, MaybeSend, MaybeSync, Result,
};

mod names;
mod output;
pub mod schema;

pub use names::{sanitize_tool_name, ToolNamer, CHAT_TOOL_NAME_MAX, RESPONSES_TOOL_NAME_MAX};
pub use output::{
    truncate_middle, CallAnswer, ImageDetail, OutputBudget, OutputContent, ToolInvocation,
    ToolKind, ToolOutputBody, TruncationConfig, ABORTED_OUTPUT,
};

/// A function tool. Store application state (for example an `Arc` or HTTP client)
/// in the implementing struct and access it through `&self` during execution.
///
/// Implement [`FunctionTool::execute`] with a plain `async fn`; no attribute macro
/// is needed. The returned future must be `Send` ([`MaybeSend`]); on WebAssembly neither
/// the tool nor its future needs to be `Send` or `Sync`.
pub trait FunctionTool: MaybeSend + MaybeSync {
    /// Arguments decoded with Serde. Use `Value` for a dynamic JSON tool.
    type Input: DeserializeOwned + MaybeSend;
    /// Result encoded as JSON, including quotes for string outputs.
    type Output: Serialize + MaybeSend;
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
    /// Use `Error::application` to preserve custom domain errors, and `Error::fatal` for a
    /// failure that must not be reported to the model ([`ToolRegistry::answer_call`]).
    fn execute(
        &self,
        input: Self::Input,
    ) -> impl std::future::Future<Output = Result<Self::Output>> + MaybeSend;
    /// Whether this tool may run at the same time as other tools that allow it. Read once, at
    /// registration ([`ToolRegistry::supports_parallel`]); the application or a tool loop
    /// decides what to run concurrently. Default `false`.
    fn supports_parallel(&self) -> bool {
        false
    }
    /// How much of this tool's output the model should see, instead of the default of the
    /// [`TruncationConfig`]. Read once, at registration ([`ToolRegistry::output_budget`]).
    fn output_budget(&self) -> Option<OutputBudget> {
        None
    }
}

/// A function tool whose output is more than JSON text.
///
/// It returns a [`ToolOutputBody`], with content items such as images or files. Register it with
/// [`ToolRegistry::register_rich`]; everything else is as for [`FunctionTool`].
///
/// Chat Completions tool messages take text only: a rich output with images or files can only
/// be sent to the Responses API.
pub trait RichTool: MaybeSend + MaybeSync {
    /// Arguments decoded with Serde.
    type Input: DeserializeOwned + MaybeSend;
    /// Stable name: 1–64 ASCII letters, digits, underscores or hyphens.
    fn name(&self) -> &str;
    /// Describe what the tool does and when the model should use it.
    fn description(&self) -> &str;
    /// An object JSON Schema matching `Input` (see [`FunctionTool::parameters_schema`]).
    fn parameters_schema(&self) -> Value;
    /// Execute after the caller has made any required authorization decisions.
    fn execute(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<ToolOutputBody>> + MaybeSend;
    /// See [`FunctionTool::supports_parallel`].
    fn supports_parallel(&self) -> bool {
        false
    }
    /// See [`FunctionTool::output_budget`].
    fn output_budget(&self) -> Option<OutputBudget> {
        None
    }
}

/// The syntax of a [`CustomFormat::Grammar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GrammarSyntax {
    /// A [Lark](https://lark-parser.readthedocs.io/) grammar.
    Lark,
    /// A regular expression.
    Regex,
}

impl GrammarSyntax {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lark => "lark",
            Self::Regex => "regex",
        }
    }
}

/// The input format of a [`CustomTool`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CustomFormat {
    /// Any text.
    Text,
    /// Text that matches a grammar.
    Grammar {
        /// The grammar's syntax.
        syntax: GrammarSyntax,
        /// The grammar.
        definition: String,
    },
}

impl CustomFormat {
    /// Text that matches a Lark grammar.
    #[must_use]
    pub fn lark(definition: impl Into<String>) -> Self {
        Self::Grammar {
            syntax: GrammarSyntax::Lark,
            definition: definition.into(),
        }
    }

    /// Text that matches a regular expression.
    #[must_use]
    pub fn regex(definition: impl Into<String>) -> Self {
        Self::Grammar {
            syntax: GrammarSyntax::Regex,
            definition: definition.into(),
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Text => json!({"type": "text"}),
            Self::Grammar { syntax, definition } => {
                json!({"type": "grammar", "syntax": syntax.as_str(), "definition": definition})
            }
        }
    }
}

/// A custom (freeform) tool: the model sends text instead of JSON arguments.
///
/// The text can be a patch, a query or a program, optionally constrained by a grammar
/// ([`CustomFormat`]). Register it with [`ToolRegistry::register_custom`]; it is advertised to
/// the Responses API ([`ToolRegistry::responses_tools`]).
pub trait CustomTool: MaybeSend + MaybeSync {
    /// Stable name: 1–64 ASCII letters, digits, underscores or hyphens.
    fn name(&self) -> &str;
    /// Describe what the tool does, when the model should use it and what to send.
    fn description(&self) -> &str;
    /// The input format. Default: any text.
    fn format(&self) -> CustomFormat {
        CustomFormat::Text
    }
    /// Execute with the model's input text.
    fn execute(&self, input: String) -> impl Future<Output = Result<ToolOutputBody>> + MaybeSend;
    /// See [`FunctionTool::supports_parallel`].
    fn supports_parallel(&self) -> bool {
        false
    }
    /// See [`FunctionTool::output_budget`].
    fn output_budget(&self) -> Option<OutputBudget> {
        None
    }
}

#[cfg(not(target_family = "wasm"))]
type DynamicHandler =
    Box<dyn Fn(Value) -> BoxFuture<'static, Result<ToolOutputBody>> + Send + Sync>;
#[cfg(target_family = "wasm")]
type DynamicHandler = Box<dyn Fn(Value) -> BoxFuture<'static, Result<ToolOutputBody>>>;

/// A function tool only known at runtime, such as one of an MCP server or a plugin.
///
/// It has a name, a description, a JSON Schema and a handler that takes the JSON arguments.
/// Register it with [`ToolRegistry::register_dynamic`], which cleans up its name and schema.
///
/// Natively the handler must be `Send` and `Sync` and its future `Send`; on WebAssembly neither
/// needs to be.
pub struct DynamicTool {
    raw_name: String,
    prefix: Option<String>,
    description: String,
    schema: Value,
    handler: DynamicHandler,
    supports_parallel: bool,
    output_budget: Option<OutputBudget>,
}

impl std::fmt::Debug for DynamicTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicTool")
            .field("raw_name", &self.raw_name)
            .field("prefix", &self.prefix)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

impl DynamicTool {
    /// A tool named `name` (any text) whose arguments match `schema`, run by `handler`.
    #[cfg(not(target_family = "wasm"))]
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolOutputBody>> + Send + 'static,
    {
        Self::from_handler(
            name.into(),
            description.into(),
            schema,
            Box::new(move |arguments| Box::pin(handler(arguments))),
        )
    }

    /// A tool named `name` (any text) whose arguments match `schema`, run by `handler`.
    #[cfg(target_family = "wasm")]
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + 'static,
        Fut: Future<Output = Result<ToolOutputBody>> + 'static,
    {
        Self::from_handler(
            name.into(),
            description.into(),
            schema,
            Box::new(move |arguments| Box::pin(handler(arguments))),
        )
    }

    fn from_handler(
        raw_name: String,
        description: String,
        schema: Value,
        handler: DynamicHandler,
    ) -> Self {
        Self {
            raw_name,
            prefix: None,
            description,
            schema,
            handler,
            supports_parallel: false,
            output_budget: None,
        }
    }

    /// Prefix the advertised name with `prefix__`, such as the MCP server's name.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Whether the tool may run at the same time as other tools that allow it.
    #[must_use]
    pub fn supports_parallel(mut self, parallel: bool) -> Self {
        self.supports_parallel = parallel;
        self
    }

    /// How much of the tool's output the model should see.
    #[must_use]
    pub fn output_budget(mut self, budget: OutputBudget) -> Self {
        self.output_budget = Some(budget);
        self
    }
}

/// The size above which [`ToolRegistry::register_dynamic`] compacts a schema
/// ([`schema::compact`]).
pub const DYNAMIC_SCHEMA_COMPACT_BYTES: usize = 5_000;
/// The size above which [`ToolRegistry::register_dynamic`] replaces a schema that compaction
/// could not shrink with one that accepts any object.
pub const DYNAMIC_SCHEMA_MAX_BYTES: usize = 8_000;

/// Registration or execution failure, with the tool name and original source.
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    /// Arguments could not be decoded as JSON or as the tool's input type.
    #[error("Tool {name:?} argument decoding failed: {source}")]
    Arguments {
        /// Registered tool name.
        name: String,
        /// Original Serde error.
        #[source]
        source: serde_json::Error,
    },
    /// The handler's output could not be encoded as JSON.
    #[error("Tool {name:?} output encoding failed: {source}")]
    Output {
        /// Registered tool name.
        name: String,
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
    /// The output cannot be sent where it was asked for, such as an image in a Chat
    /// Completions tool message.
    #[error("Unsupported tool output: {reason}")]
    UnsupportedOutput {
        /// Actionable reason for rejection.
        reason: String,
    },
}

/// What to do with a failed tool call ([`ToolError::classify`]).
///
/// New variants may be added in minor releases: match with a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum ToolFailure {
    /// Send this text to the model as the call's output (the answer is `success: false`), so
    /// it can correct the call or carry on.
    RespondToModel(String),
    /// Stop: the error is returned to the application, and nothing is sent for this call.
    Fatal(ToolError),
}

impl ToolError {
    /// The default classification of a call failure, the one
    /// [`ToolRegistry::answer_call`] applies:
    ///
    /// - arguments that are not JSON, not an object or not the tool's input
    ///   (`"failed to parse function arguments: …"`), an unknown tool (`"unsupported call: …"`),
    ///   an invalid call and an unsupported output: [`ToolFailure::RespondToModel`];
    /// - a handler error ([`ToolError::Execution`]): `RespondToModel` with the error's message,
    ///   unless the handler returned [`Error::fatal`](crate::Error::fatal);
    /// - an output that cannot be encoded, and registration errors: [`ToolFailure::Fatal`].
    ///
    /// The model sees the handler's error message: return an error whose message is fit for it,
    /// or classify with [`ToolRegistry::answer_call_with`].
    #[must_use]
    pub fn classify(self) -> ToolFailure {
        match self {
            Self::Arguments { source, .. } => {
                ToolFailure::RespondToModel(format!("failed to parse function arguments: {source}"))
            }
            Self::InvalidArguments { .. } => {
                ToolFailure::RespondToModel("function arguments must be a JSON object".into())
            }
            Self::Unknown { name } => {
                ToolFailure::RespondToModel(format!("unsupported call: {name}"))
            }
            Self::InvalidCall { reason } | Self::UnsupportedOutput { reason } => {
                ToolFailure::RespondToModel(reason)
            }
            Self::Execution { source, .. } if !source.is_fatal() => {
                ToolFailure::RespondToModel(model_message(&source))
            }
            Self::Call { call_id, source } => match source.classify() {
                ToolFailure::Fatal(source) => ToolFailure::Fatal(Self::Call {
                    call_id,
                    source: Box::new(source),
                }),
                other => other,
            },
            other => ToolFailure::Fatal(other),
        }
    }
}

/// The message of a handler error for the model: an application error's own message.
fn model_message(error: &Error) -> String {
    match error {
        Error::Application(error) => error.to_string(),
        Error::Tool(error) => error.to_string(),
        other => other.to_string(),
    }
}

/// A successful tool reply ready for `ChatCompletionBuilder::tool`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    /// Original model-issued call identifier.
    pub call_id: String,
    /// JSON-encoded output, not debug formatting or unquoted text.
    pub content: String,
}

impl ToolOutput {
    /// The content as a [`ToolOutputBody`].
    #[must_use]
    pub fn body(&self) -> ToolOutputBody {
        ToolOutputBody::Text(self.content.clone())
    }
}

/// The identity of a registered tool: its name, and its namespace if it has one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ToolKey {
    /// The namespace ([`ToolRegistry::namespace`]), if any.
    pub namespace: Option<String>,
    /// The tool name.
    pub name: String,
}

impl ToolKey {
    /// A tool outside any namespace.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }

    /// A tool of `namespace`.
    #[must_use]
    pub fn namespaced(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            name: name.into(),
        }
    }

    fn of(call: &ToolInvocation) -> Self {
        Self {
            namespace: call.namespace.clone(),
            name: call.name.clone(),
        }
    }
}

/// What a handler produced: JSON from a [`FunctionTool`], a body from the others.
enum ErasedOutput {
    Json(Value),
    Body(ToolOutputBody),
}

impl ErasedOutput {
    fn into_body(self) -> ToolOutputBody {
        match self {
            Self::Json(value) => ToolOutputBody::Text(value.to_string()),
            Self::Body(body) => body,
        }
    }

    fn into_value(self) -> Value {
        match self {
            Self::Json(value) => value,
            Self::Body(ToolOutputBody::Text(text)) => Value::String(text),
            Self::Body(body) => body.to_json(),
        }
    }
}

/// Object-safe form of the tool traits: decodes the input, boxes the future and encodes the
/// output.
trait ErasedTool: MaybeSend + MaybeSync {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: &'a str,
    ) -> BoxFuture<'a, std::result::Result<ErasedOutput, ToolError>>;
}

/// Decode a JSON object of arguments; the original text is decoded so that Serde still detects
/// duplicate struct fields.
fn decode_arguments<I: DeserializeOwned>(
    name: &str,
    arguments: &str,
) -> std::result::Result<I, ToolError> {
    let argument_error = |source| ToolError::Arguments {
        name: name.into(),
        source,
    };
    let value: Value = serde_json::from_str(arguments).map_err(argument_error)?;
    if !value.is_object() {
        return Err(ToolError::InvalidArguments { name: name.into() });
    }
    serde_json::from_str(arguments).map_err(argument_error)
}

fn execution_error(name: &str) -> impl FnOnce(Error) -> ToolError + '_ {
    move |source| ToolError::Execution {
        name: name.into(),
        source,
    }
}

struct FunctionAdapter<T>(T);

impl<T: FunctionTool> ErasedTool for FunctionAdapter<T> {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: &'a str,
    ) -> BoxFuture<'a, std::result::Result<ErasedOutput, ToolError>> {
        Box::pin(async move {
            let input = decode_arguments(name, input)?;
            let output = FunctionTool::execute(&self.0, input)
                .await
                .map_err(execution_error(name))?;
            serde_json::to_value(output)
                .map(ErasedOutput::Json)
                .map_err(|source| ToolError::Output {
                    name: name.into(),
                    source,
                })
        })
    }
}

struct RichAdapter<T>(T);

impl<T: RichTool> ErasedTool for RichAdapter<T> {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: &'a str,
    ) -> BoxFuture<'a, std::result::Result<ErasedOutput, ToolError>> {
        Box::pin(async move {
            let input = decode_arguments(name, input)?;
            RichTool::execute(&self.0, input)
                .await
                .map(ErasedOutput::Body)
                .map_err(execution_error(name))
        })
    }
}

struct CustomAdapter<T>(T);

impl<T: CustomTool> ErasedTool for CustomAdapter<T> {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: &'a str,
    ) -> BoxFuture<'a, std::result::Result<ErasedOutput, ToolError>> {
        Box::pin(async move {
            CustomTool::execute(&self.0, input.to_string())
                .await
                .map(ErasedOutput::Body)
                .map_err(execution_error(name))
        })
    }
}

struct DynamicAdapter(DynamicHandler);

impl ErasedTool for DynamicAdapter {
    fn call<'a>(
        &'a self,
        name: &'a str,
        input: &'a str,
    ) -> BoxFuture<'a, std::result::Result<ErasedOutput, ToolError>> {
        Box::pin(async move {
            let arguments: Value = decode_arguments(name, input)?;
            (self.0)(arguments)
                .await
                .map(ErasedOutput::Body)
                .map_err(execution_error(name))
        })
    }
}

struct RegisteredTool {
    kind: ToolKind,
    description: String,
    /// The parameters schema of a function tool.
    parameters: Value,
    /// The input format of a custom tool.
    format: CustomFormat,
    handler: Box<dyn ErasedTool>,
    supports_parallel: bool,
    output_budget: Option<OutputBudget>,
    /// The name a dynamic tool was registered with, before sanitizing.
    raw_name: Option<String>,
}

impl RegisteredTool {
    fn function(
        description: &str,
        parameters: Value,
        handler: Box<dyn ErasedTool>,
        supports_parallel: bool,
        output_budget: Option<OutputBudget>,
    ) -> Self {
        Self {
            kind: ToolKind::Function,
            description: description.to_string(),
            parameters,
            format: CustomFormat::Text,
            handler,
            supports_parallel,
            output_budget,
            raw_name: None,
        }
    }

    fn responses_json(&self, name: &str) -> Value {
        match self.kind {
            ToolKind::Custom => json!({
                "type": "custom",
                "name": name,
                "description": self.description,
                "format": self.format.to_json(),
            }),
            _ => json!({
                "type": "function",
                "name": name,
                "description": self.description,
                "parameters": self.parameters,
            }),
        }
    }
}

/// Check a tool or namespace name: 1–64 ASCII letters, digits, underscores or hyphens.
fn check_name(name: &str) -> std::result::Result<(), ToolError> {
    if name.is_empty()
        || name.len() > CHAT_TOOL_NAME_MAX
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(invalid(
            name,
            "name must contain 1–64 ASCII letters, digits, underscores or hyphens",
        ));
    }
    Ok(())
}

fn check_description(name: &str, description: &str) -> std::result::Result<(), ToolError> {
    if description.trim().is_empty() {
        return Err(invalid(name, "description must not be blank"));
    }
    Ok(())
}

fn check_schema(name: &str, schema: &Value) -> std::result::Result<(), ToolError> {
    if !schema.is_object() || schema.get("type").and_then(Value::as_str) != Some("object") {
        return Err(invalid(
            name,
            "parameters schema must have root type object",
        ));
    }
    Ok(())
}

fn invalid(name: &str, reason: &str) -> ToolError {
    ToolError::InvalidDefinition {
        name: name.into(),
        reason: reason.into(),
    }
}

/// Heterogeneous tools, with definitions snapshotted at registration and returned
/// in name order.
///
/// Schema map key ordering follows the underlying client model;
/// byte-for-byte serialization order is not guaranteed. Execution is explicit.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<ToolKey, RegisteredTool>,
    /// Namespace names and descriptions.
    namespaces: BTreeMap<String, String>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys())
            .field("namespaces", &self.namespaces.keys())
            .finish_non_exhaustive()
    }
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
        self.register_function_in(None, tool)
    }

    /// Register a [`RichTool`], with the same checks as [`register`](Self::register).
    pub fn register_rich<T: RichTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        self.register_rich_in(None, tool)
    }

    /// Register a [`CustomTool`]: checks its name and description as
    /// [`register`](Self::register) does. Custom tools are advertised to the Responses API
    /// ([`responses_tools`](Self::responses_tools)), not by
    /// [`tool_definitions`](Self::tool_definitions).
    pub fn register_custom<T: CustomTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        self.register_custom_in(None, tool)
    }

    /// Register a tool only known at runtime, and return the name the model sees.
    ///
    /// Unlike [`register`](Self::register), it adapts the tool instead of rejecting it:
    ///
    /// - the name (with its [`prefix`](DynamicTool::prefix), as `prefix__name`) is sanitized
    ///   ([`sanitize_tool_name`]); one that is too long or already taken gets a hash suffix
    ///   ([`ToolNamer`]); [`raw_name`](Self::raw_name) gives the original back;
    /// - the schema is sanitized ([`schema::sanitize`]), compacted above
    ///   [`DYNAMIC_SCHEMA_COMPACT_BYTES`] ([`schema::compact`]), and replaced with one that
    ///   accepts any object if it is still above [`DYNAMIC_SCHEMA_MAX_BYTES`];
    /// - a blank description becomes the raw name.
    ///
    /// It fails only for a schema whose root is not an object.
    pub fn register_dynamic(
        &mut self,
        tool: DynamicTool,
    ) -> std::result::Result<String, ToolError> {
        let mut schema = tool.schema;
        schema::sanitize(&mut schema);
        if schema.get("type").and_then(Value::as_str) != Some("object") {
            return Err(invalid(
                &tool.raw_name,
                "parameters schema must have root type object",
            ));
        }
        if schema::size(&schema) > DYNAMIC_SCHEMA_COMPACT_BYTES {
            schema::compact(&mut schema, DYNAMIC_SCHEMA_COMPACT_BYTES);
        }
        if schema::size(&schema) > DYNAMIC_SCHEMA_MAX_BYTES {
            schema = json!({"type": "object", "properties": {}, "additionalProperties": true});
        }
        let name = names::unique_name(
            tool.prefix.as_deref(),
            &tool.raw_name,
            CHAT_TOOL_NAME_MAX,
            |candidate| {
                self.tools.contains_key(&ToolKey::new(candidate))
                    || self.namespaces.contains_key(candidate)
            },
        );
        let description = if tool.description.trim().is_empty() {
            tool.raw_name.clone()
        } else {
            tool.description
        };
        let mut registered = RegisteredTool::function(
            &description,
            schema,
            Box::new(DynamicAdapter(tool.handler)),
            tool.supports_parallel,
            tool.output_budget,
        );
        registered.raw_name = Some(tool.raw_name);
        self.insert(ToolKey::new(name.clone()), registered)?;
        Ok(name)
    }

    /// Group tools under a namespace (Responses API): `name` follows the rules of tool names,
    /// `description` says what the tools are for. Register its tools with the returned
    /// [`NamespaceRegistrar`]; asking again for the same namespace adds to it.
    ///
    /// The model calls them with the namespace on the call
    /// ([`ToolInvocation::namespace`]). Namespaced tools are advertised by
    /// [`responses_tools`](Self::responses_tools) only: Chat Completions has no namespaces.
    pub fn namespace(
        &mut self,
        name: &str,
        description: &str,
    ) -> std::result::Result<NamespaceRegistrar<'_>, ToolError> {
        check_name(name)?;
        check_description(name, description)?;
        if self.tools.contains_key(&ToolKey::new(name)) {
            return Err(ToolError::Duplicate { name: name.into() });
        }
        match self.namespaces.get(name) {
            Some(existing) if existing != description => {
                return Err(invalid(
                    name,
                    "namespace already registered with another description",
                ));
            }
            Some(_) => {}
            None => {
                self.namespaces.insert(name.into(), description.into());
            }
        }
        Ok(NamespaceRegistrar {
            registry: self,
            namespace: name.into(),
        })
    }

    fn register_function_in<T: FunctionTool + 'static>(
        &mut self,
        namespace: Option<&str>,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        let name = tool.name().to_owned();
        check_name(&name)?;
        check_description(&name, tool.description())?;
        let schema = tool.parameters_schema();
        check_schema(&name, &schema)?;
        let (parallel, budget) = (
            FunctionTool::supports_parallel(&tool),
            FunctionTool::output_budget(&tool),
        );
        let description = tool.description().to_owned();
        let registered = RegisteredTool::function(
            &description,
            schema,
            Box::new(FunctionAdapter(tool)),
            parallel,
            budget,
        );
        self.insert(key(namespace, name), registered)
    }

    fn register_rich_in<T: RichTool + 'static>(
        &mut self,
        namespace: Option<&str>,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        let name = tool.name().to_owned();
        check_name(&name)?;
        check_description(&name, tool.description())?;
        let schema = tool.parameters_schema();
        check_schema(&name, &schema)?;
        let (parallel, budget) = (tool.supports_parallel(), tool.output_budget());
        let description = tool.description().to_owned();
        let registered = RegisteredTool::function(
            &description,
            schema,
            Box::new(RichAdapter(tool)),
            parallel,
            budget,
        );
        self.insert(key(namespace, name), registered)
    }

    fn register_custom_in<T: CustomTool + 'static>(
        &mut self,
        namespace: Option<&str>,
        tool: T,
    ) -> std::result::Result<(), ToolError> {
        let name = tool.name().to_owned();
        check_name(&name)?;
        check_description(&name, tool.description())?;
        let registered = RegisteredTool {
            kind: ToolKind::Custom,
            description: tool.description().to_string(),
            parameters: Value::Null,
            format: tool.format(),
            supports_parallel: tool.supports_parallel(),
            output_budget: tool.output_budget(),
            raw_name: None,
            handler: Box::new(CustomAdapter(tool)),
        };
        self.insert(key(namespace, name), registered)
    }

    fn insert(&mut self, key: ToolKey, tool: RegisteredTool) -> std::result::Result<(), ToolError> {
        if key.namespace.is_none() && self.namespaces.contains_key(&key.name) {
            return Err(ToolError::Duplicate { name: key.name });
        }
        match self.tools.entry(key) {
            Entry::Occupied(entry) => Err(ToolError::Duplicate {
                name: entry.key().name.clone(),
            }),
            Entry::Vacant(entry) => {
                entry.insert(tool);
                Ok(())
            }
        }
    }

    /// Return function definitions suitable for `ChatCompletionBuilder::tools`: the function
    /// tools outside namespaces ([`register`](Self::register),
    /// [`register_rich`](Self::register_rich), [`register_dynamic`](Self::register_dynamic)),
    /// in name order. Custom and namespaced tools are left out: see
    /// [`responses_tools`](Self::responses_tools).
    #[must_use]
    pub fn tool_definitions(&self) -> Vec<ChatCompletionTool> {
        self.tools
            .iter()
            .filter(|(key, tool)| key.namespace.is_none() && tool.kind == ToolKind::Function)
            .map(|(key, tool)| tool_function(&key.name, &tool.description, tool.parameters.clone()))
            .collect()
    }

    /// Return the tools for a Responses API request (`ResponsesBuilder::tools`): the function
    /// and custom tools outside namespaces in name order, then one `namespace` tool per
    /// namespace with its tools.
    #[must_use]
    pub fn responses_tools(&self) -> Vec<ResponsesTool> {
        let mut tools: Vec<ResponsesTool> = self
            .tools
            .iter()
            .filter(|(key, _)| key.namespace.is_none())
            .map(|(key, tool)| match tool.kind {
                ToolKind::Custom => to_tool(tool.responses_json(&key.name)),
                _ => crate::responses_api::responses_function_tool(
                    &key.name,
                    &tool.description,
                    tool.parameters.clone(),
                ),
            })
            .collect();
        for (namespace, description) in &self.namespaces {
            let members: Vec<Value> = self
                .tools
                .iter()
                .filter(|(key, _)| key.namespace.as_deref() == Some(namespace.as_str()))
                .map(|(key, tool)| tool.responses_json(&key.name))
                .collect();
            if members.is_empty() {
                continue;
            }
            tools.push(to_tool(json!({
                "type": "namespace",
                "name": namespace,
                "description": description,
                "tools": members,
            })));
        }
        tools
    }

    /// The registered tools, namespaced ones included, in order.
    pub fn keys(&self) -> impl Iterator<Item = &ToolKey> {
        self.tools.keys()
    }

    /// The number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether no tool is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// The name a dynamic tool was registered with ([`register_dynamic`](Self::register_dynamic)),
    /// for the tool advertised as `name`.
    #[must_use]
    pub fn raw_name(&self, name: &str) -> Option<&str> {
        self.tools.get(&ToolKey::new(name))?.raw_name.as_deref()
    }

    /// Whether the tool `call` names allows parallel execution
    /// ([`FunctionTool::supports_parallel`]); `false` for an unknown tool.
    #[must_use]
    pub fn supports_parallel(&self, call: &ToolInvocation) -> bool {
        self.tools
            .get(&ToolKey::of(call))
            .is_some_and(|tool| tool.supports_parallel)
    }

    /// The output budget of the tool `call` names ([`FunctionTool::output_budget`]), if it
    /// sets one.
    #[must_use]
    pub fn output_budget(&self, call: &ToolInvocation) -> Option<OutputBudget> {
        self.tools.get(&ToolKey::of(call))?.output_budget
    }

    /// Dispatch by name. Arguments must be a JSON object matching `FunctionTool::Input`.
    /// Returns JSON output; does not retry, catch panics, or enforce timeouts.
    ///
    /// Only function tools outside namespaces are found here. A rich or dynamic tool's text
    /// output is returned as a JSON string, its content items as an array.
    pub async fn execute(
        &self,
        name: &str,
        arguments: &str,
    ) -> std::result::Result<Value, ToolError> {
        let tool = self
            .tools
            .get(&ToolKey::new(name))
            .filter(|tool| tool.kind == ToolKind::Function)
            .ok_or_else(|| ToolError::Unknown { name: name.into() })?;
        tool.handler
            .call(name, arguments)
            .await
            .map(ErasedOutput::into_value)
    }

    /// Execute one approved model-issued function call and retain its identifier.
    ///
    /// On error, `ToolError::Call` retains the identifier and original error. No
    /// automatic batch execution is provided: callers choose error reporting,
    /// ordering, concurrency and authorization per call, and retain earlier results.
    /// Custom (non-function) calls and empty identifiers are rejected before execution.
    /// A rich output with images or files is rejected ([`ToolError::UnsupportedOutput`]):
    /// chat tool messages are text.
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
            let name = &function_call.function.name;
            let tool = self
                .tools
                .get(&ToolKey::new(name.as_str()))
                .filter(|tool| tool.kind == ToolKind::Function)
                .ok_or_else(|| ToolError::Unknown { name: name.clone() })?;
            let output = tool
                .handler
                .call(name, &function_call.function.arguments)
                .await?;
            Ok(ToolOutput {
                call_id: call.id().into(),
                content: output.into_body().to_chat_text()?,
            })
        }
        .await;
        result.map_err(|source| ToolError::Call {
            call_id: call.id().into(),
            source: Box::new(source),
        })
    }

    /// Answer one approved call of either API, so that every call gets an answer.
    ///
    /// Runs the tool the call names (with its namespace) and returns its output, or, when the
    /// call fails in a way the model can act on, the failure as the output with
    /// `success: false` (see [`ToolError::classify`]): bad arguments, an unknown tool, a call of
    /// the wrong kind, a handler error. Only a [`ToolFailure::Fatal`] failure, or a call without
    /// an identifier, is returned as an error ([`ToolError::Call`]).
    ///
    /// The call can be a chat call (`&ChatCompletionMessageToolCallsInner`), a Responses API
    /// call (`&FunctionToolCall`, `&CustomToolCall`) or a [`ToolInvocation`]. Send the answer
    /// with `ResponsesBuilder::call_answer` or `ChatCompletionBuilder::call_answer`. The output
    /// is not truncated: see [`CallAnswer::truncated`].
    pub async fn answer_call(
        &self,
        call: impl Into<ToolInvocation>,
    ) -> std::result::Result<CallAnswer, ToolError> {
        self.answer_call_with(call, ToolError::classify).await
    }

    /// [`answer_call`](Self::answer_call) with your own classification of failures, for
    /// example to hide handler errors from the model.
    pub async fn answer_call_with(
        &self,
        call: impl Into<ToolInvocation>,
        classify: impl FnOnce(ToolError) -> ToolFailure,
    ) -> std::result::Result<CallAnswer, ToolError> {
        let call = call.into();
        if call.call_id.is_empty() {
            return Err(ToolError::Call {
                call_id: String::new(),
                source: Box::new(ToolError::InvalidCall {
                    reason: "call identifier must not be empty".into(),
                }),
            });
        }
        match self.run(&call).await {
            Ok(output) => Ok(CallAnswer::new(&call, output, true)),
            Err(error) => match classify(error) {
                ToolFailure::Fatal(error) => Err(ToolError::Call {
                    call_id: call.call_id.clone(),
                    source: Box::new(error),
                }),
                ToolFailure::RespondToModel(message) => Ok(CallAnswer::failure(&call, message)),
            },
        }
    }

    async fn run(&self, call: &ToolInvocation) -> std::result::Result<ToolOutputBody, ToolError> {
        if let ToolKind::Other(kind) = &call.kind {
            return Err(ToolError::InvalidCall {
                reason: format!("unsupported call type {kind:?}"),
            });
        }
        let Some(tool) = self.tools.get(&ToolKey::of(call)) else {
            let name = call.namespace.as_ref().map_or_else(
                || call.name.clone(),
                |namespace| format!("{namespace}.{}", call.name),
            );
            return Err(ToolError::Unknown { name });
        };
        if tool.kind != call.kind {
            let reason = match tool.kind {
                ToolKind::Custom => format!("{} is a custom tool: send text input", call.name),
                _ => format!("{} is a function tool: send JSON arguments", call.name),
            };
            return Err(ToolError::InvalidCall { reason });
        }
        tool.handler
            .call(&call.name, &call.input)
            .await
            .map(ErasedOutput::into_body)
    }
}

impl From<&ToolInvocation> for ToolInvocation {
    fn from(call: &ToolInvocation) -> Self {
        call.clone()
    }
}

/// Registers tools in a namespace ([`ToolRegistry::namespace`]).
#[derive(Debug)]
pub struct NamespaceRegistrar<'a> {
    registry: &'a mut ToolRegistry,
    namespace: String,
}

impl NamespaceRegistrar<'_> {
    /// Register a [`FunctionTool`] in the namespace ([`ToolRegistry::register`]).
    pub fn register<T: FunctionTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<&mut Self, ToolError> {
        self.registry
            .register_function_in(Some(&self.namespace), tool)?;
        Ok(self)
    }

    /// Register a [`RichTool`] in the namespace ([`ToolRegistry::register_rich`]).
    pub fn register_rich<T: RichTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<&mut Self, ToolError> {
        self.registry
            .register_rich_in(Some(&self.namespace), tool)?;
        Ok(self)
    }

    /// Register a [`CustomTool`] in the namespace ([`ToolRegistry::register_custom`]).
    pub fn register_custom<T: CustomTool + 'static>(
        &mut self,
        tool: T,
    ) -> std::result::Result<&mut Self, ToolError> {
        self.registry
            .register_custom_in(Some(&self.namespace), tool)?;
        Ok(self)
    }
}

fn key(namespace: Option<&str>, name: String) -> ToolKey {
    ToolKey {
        namespace: namespace.map(str::to_string),
        name,
    }
}

/// A Responses API tool from its JSON definition.
fn to_tool(value: Value) -> ResponsesTool {
    serde_json::from_value(value.clone()).unwrap_or(ResponsesTool::Unknown(value))
}
