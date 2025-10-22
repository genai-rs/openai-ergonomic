//! Unified tool framework for `OpenAI` chat tool calling.
//!
//! This module provides a single [`Tool`] trait with flexible input/output types,
//! a [`ToolRegistry`] for managing tool definitions, and helper macros to reduce
//! boilerplate when declaring tools.

use std::collections::HashMap;

use crate::Result;
use async_trait::async_trait;
use openai_client_base::models::ChatCompletionTool;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    builders::chat::tool_function,
    errors::Error,
    responses::{chat::ToolCallExt, ChatCompletionResponseWrapper},
};

/// Trait implemented by tools that can be registered with [`ToolRegistry`].
#[async_trait]
pub trait Tool: Send + Sync {
    /// Input type deserialized from the model-provided JSON arguments.
    type Input: DeserializeOwned + Send;

    /// Output type serialized back to JSON to return to the model.
    type Output: Serialize + Send;

    /// Machine-readable name of the tool (`snake_case` recommended).
    fn name(&self) -> &str;

    /// Human-friendly description shown to the model.
    fn description(&self) -> &str;

    /// JSON schema describing the tool parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with typed input, producing a typed output.
    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;
}

#[async_trait]
trait ErasedTool: Send + Sync {
    fn definition(&self) -> ChatCompletionTool;
    async fn execute(&self, args: &str) -> Result<Value>;
}

#[async_trait]
impl<T> ErasedTool for T
where
    T: Tool + 'static,
{
    fn definition(&self) -> ChatCompletionTool {
        tool_function(self.name(), self.description(), self.parameters_schema())
    }

    async fn execute(&self, args: &str) -> Result<Value> {
        let params: T::Input = serde_json::from_str(args)?;
        let output = Tool::execute(self, params).await?;
        Ok(serde_json::to_value(output)?)
    }
}

/// Registry that holds all available tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ErasedTool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool and return the registry for chaining.
    #[must_use]
    pub fn register<T>(mut self, tool: T) -> Self
    where
        T: Tool + 'static,
    {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
        self
    }

    /// Register a tool using a mutable reference.
    pub fn register_mut<T>(&mut self, tool: T)
    where
        T: Tool + 'static,
    {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    /// Returns `OpenAI` tool definitions suitable for chat requests.
    #[must_use]
    pub fn tool_definitions(&self) -> Vec<ChatCompletionTool> {
        self.tools.values().map(|tool| tool.definition()).collect()
    }

    /// Execute a tool by name with JSON arguments.
    pub async fn execute(&self, tool_name: &str, arguments: &str) -> Result<Value> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| Error::InvalidRequest(format!("Unknown tool: {tool_name}")))?;
        tool.execute(arguments).await
    }

    /// Execute a tool and return the JSON string expected by the Chat API.
    pub async fn execute_to_string(&self, tool_name: &str, arguments: &str) -> Result<String> {
        let value = self.execute(tool_name, arguments).await?;
        Ok(value.to_string())
    }

    /// Execute every tool call present in the response.
    pub async fn process_tool_calls(
        &self,
        response: &ChatCompletionResponseWrapper,
    ) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();
        for call in response.tool_calls() {
            let tool_name = call.function_name();
            if tool_name.is_empty() {
                return Err(Error::InvalidRequest(
                    "Tool call missing function name".to_string(),
                ));
            }
            let payload = self
                .execute_to_string(tool_name, call.function_arguments())
                .await?;
            results.push((call.id().to_string(), payload));
        }
        Ok(results)
    }
}

/// Build a JSON schema object describing tool parameters.
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
        let mut required_fields = Vec::new();
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

/// Helper to resolve the input type provided to the [`tool!`] macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __openai_tool_input_type {
    (@resolve ($provided:ty) $handler:ty) => {
        $provided
    };
    (@resolve () $handler:ty) => {
        $handler
    };
}

/// Helper to resolve the output type provided to the [`tool!`] macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __openai_tool_output_type {
    (@resolve ($provided:ty) $default:ty) => {
        $provided
    };
    (@resolve () $default:ty) => {
        $default
    };
}

/// Helper to resolve the schema expression provided to the [`tool!`] macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __openai_tool_schema_expr {
    ($expr:expr) => {
        $expr
    };
    () => {
        $crate::tool_framework::tool_schema!()
    };
}

/// Macro to declare a tool that implements [`Tool`].
#[macro_export]
macro_rules! tool {
    (
        $(#[$meta:meta])*
        $vis:vis struct $struct_name:ident;

        name: $tool_name:expr;
        description: $description:expr;
        $(input_type: $input_ty:ty;)?
        $(output_type: $output_ty:ty;)?
        $(schema: $schema_expr:expr;)?

        async fn handle($arg:ident : $handler_ty:ty) -> $ret_ty:ty $body:block
    ) => {
        $(#[$meta])*
        $vis struct $struct_name;

        #[async_trait::async_trait]
        impl $crate::tool_framework::Tool for $struct_name {
            type Input = $crate::__openai_tool_input_type!(@resolve ($($input_ty)?) $handler_ty);
            type Output = $crate::__openai_tool_output_type!(@resolve ($($output_ty)?) serde_json::Value);

            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn parameters_schema(&self) -> serde_json::Value {
                $crate::__openai_tool_schema_expr!($($schema_expr)?)
            }

            async fn execute(&self, $arg: Self::Input) -> $ret_ty $body
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client_base::models::{
        chat_completion_message_tool_call::Type as ToolCallType,
        chat_completion_response_message::Role,
        create_chat_completion_response_choices_inner::FinishReason, ChatCompletionMessageToolCall,
        ChatCompletionMessageToolCallFunction, ChatCompletionMessageToolCallsInner,
        ChatCompletionResponseMessage, CreateChatCompletionResponse,
        CreateChatCompletionResponseChoicesInner,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct EchoInput {
        message: String,
    }

    tool! {
        struct EchoTool;

        name: "echo";
        description: "Echo back the message";
        input_type: EchoInput;
        schema: tool_schema!(
            message: "string", "Message to echo", required: true,
        );

        async fn handle(params: EchoInput) -> Result<serde_json::Value> {
            Ok(serde_json::json!({ "message": params.message }))
        }
    }

    #[derive(Deserialize)]
    struct AddInput {
        lhs: i64,
        rhs: i64,
    }

    #[derive(Serialize)]
    struct AddOutput {
        sum: i64,
    }

    tool! {
        struct AddTool;

        name: "add_numbers";
        description: "Add two integers";
        input_type: AddInput;
        output_type: AddOutput;
        schema: tool_schema!(
            lhs: "integer", "Left operand", required: true,
            rhs: "integer", "Right operand", required: true,
        );

        async fn handle(params: AddInput) -> Result<AddOutput> {
            Ok(AddOutput {
                sum: params.lhs + params.rhs,
            })
        }
    }

    fn sample_tool_response() -> ChatCompletionResponseWrapper {
        let tool_call = ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(
            Box::new(ChatCompletionMessageToolCall {
                id: "call_1".to_string(),
                r#type: ToolCallType::Function,
                function: Box::new(ChatCompletionMessageToolCallFunction {
                    name: "add_numbers".to_string(),
                    arguments: r#"{"lhs":2,"rhs":3}"#.to_string(),
                }),
            }),
        );

        let message = ChatCompletionResponseMessage {
            content: None,
            refusal: None,
            tool_calls: Some(vec![tool_call]),
            annotations: None,
            role: Role::Assistant,
            function_call: None,
            audio: None,
        };

        let choice = CreateChatCompletionResponseChoicesInner {
            finish_reason: FinishReason::ToolCalls,
            index: 0,
            message: Box::new(message),
            logprobs: None,
        };

        let response = CreateChatCompletionResponse {
            id: "resp_123".to_string(),
            choices: vec![choice],
            created: 0,
            model: "gpt-test".to_string(),
            service_tier: None,
            system_fingerprint: None,
            object:
                openai_client_base::models::create_chat_completion_response::Object::ChatCompletion,
            usage: None,
        };

        ChatCompletionResponseWrapper::new(response)
    }

    #[tokio::test]
    async fn executes_typed_tool() {
        let registry = ToolRegistry::new().register(EchoTool);

        let result = registry
            .execute("echo", r#"{"message":"hello"}"#)
            .await
            .unwrap();

        assert_eq!(result["message"], "hello");
    }

    #[tokio::test]
    async fn executes_typed_output_tool() {
        let registry = ToolRegistry::new().register(AddTool);

        let result = registry
            .execute("add_numbers", r#"{"lhs":5,"rhs":7}"#)
            .await
            .unwrap();

        assert_eq!(result["sum"], 12);
    }

    #[tokio::test]
    async fn processes_tool_calls() {
        let registry = ToolRegistry::new().register(AddTool);
        let response = sample_tool_response();

        let results = registry.process_tool_calls(&response).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "call_1");
        assert_eq!(results[0].1, r#"{"sum":5}"#);
    }
}
