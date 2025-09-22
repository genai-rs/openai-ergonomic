//! Response type wrappers and ergonomic helpers.
//!
//! This module provides ergonomic wrappers around `OpenAI` API responses with
//! convenient methods for common operations. The responses-first approach
//! makes it easy to work with structured outputs and tool calling.
//!
//! # Example
//!
//! ```rust,ignore
//! # use openai_ergonomic::responses::*;
//! let response = client.responses()
//!     .model("gpt-4")
//!     .user("What is the weather?")
//!     .tool(tool_web_search())
//!     .send()
//!     .await?;
//!
//! // Access response content
//! if let Some(content) = response.content() {
//!     println!("{}", content);
//! }
//!
//! // Handle tool calls
//! for tool_call in response.tool_calls() {
//!     println!("Tool: {} Args: {}", tool_call.name(), tool_call.arguments());
//! }
//! ```

use openai_client_base::models::{
    AssistantsNamedToolChoiceFunction, ChatCompletionTool, ChatCompletionToolChoiceOption,
    CompletionUsage, CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
    FunctionObject,
};

pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod moderations;
pub mod threads;
pub mod uploads;
pub mod vector_stores;

// Re-export response types for convenience
// NOTE: Re-exports will be enabled as modules are implemented
// pub use assistants::*;
// pub use audio::*;
// pub use batch::*;
pub use chat::*; // Has implementation
                 // pub use embeddings::*;
                 // pub use files::*;
                 // pub use fine_tuning::*;
                 // pub use images::*;
                 // pub use moderations::*;
                 // pub use threads::*;
                 // pub use uploads::*;
                 // pub use vector_stores::*;

/// Common trait for all response types to provide consistent access patterns.
pub trait Response {
    /// Get the unique identifier for this response, if available.
    fn id(&self) -> Option<&str>;

    /// Get the model that generated this response, if available.
    fn model(&self) -> Option<&str>;

    /// Get any usage information from the response, if available.
    fn usage(&self) -> Option<&CompletionUsage>;
}

/// Wrapper for chat completion responses with ergonomic helpers.
#[derive(Debug, Clone)]
pub struct ChatCompletionResponseWrapper {
    inner: CreateChatCompletionResponse,
    base_url: Option<String>,
}

impl ChatCompletionResponseWrapper {
    /// Create a new response wrapper.
    pub fn new(response: CreateChatCompletionResponse) -> Self {
        Self {
            inner: response,
            base_url: None,
        }
    }

    /// Create a response wrapper with a base URL for generating links.
    pub fn with_base_url(response: CreateChatCompletionResponse, base_url: String) -> Self {
        Self {
            inner: response,
            base_url: Some(base_url),
        }
    }

    /// Get the first message content from the response.
    pub fn content(&self) -> Option<&str> {
        self.inner.choices.first()?.message.content.as_deref()
    }

    /// Get all choices from the response.
    pub fn choices(
        &self,
    ) -> &[openai_client_base::models::CreateChatCompletionResponseChoicesInner] {
        &self.inner.choices
    }

    /// Get tool calls from the first choice, if any.
    pub fn tool_calls(
        &self,
    ) -> Vec<&openai_client_base::models::ChatCompletionMessageToolCallsInner> {
        self.inner
            .choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|calls| calls.iter().collect())
            .unwrap_or_default()
    }

    /// Check if the response was refused.
    pub fn is_refusal(&self) -> bool {
        self.inner
            .choices
            .first()
            .and_then(|c| c.message.refusal.as_ref())
            .is_some()
    }

    /// Get the refusal message if the response was refused.
    pub fn refusal(&self) -> Option<&str> {
        self.inner
            .choices
            .first()
            .and_then(|c| c.message.refusal.as_ref())
            .map(std::string::String::as_str)
    }

    /// Get the finish reason for the first choice.
    pub fn finish_reason(&self) -> Option<String> {
        use openai_client_base::models::create_chat_completion_response_choices_inner::FinishReason;
        self.inner.choices.first().map(|c| match &c.finish_reason {
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "length".to_string(),
            FinishReason::ToolCalls => "tool_calls".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::FunctionCall => "function_call".to_string(),
        })
    }

    /// Generate a URL for this response if `base_url` was provided.
    pub fn url(&self) -> Option<String> {
        self.base_url
            .as_ref()
            .map(|base| format!("{}/chat/{}", base, self.inner.id))
    }

    /// Get the inner response object.
    pub fn inner(&self) -> &CreateChatCompletionResponse {
        &self.inner
    }
}

impl Response for ChatCompletionResponseWrapper {
    fn id(&self) -> Option<&str> {
        Some(&self.inner.id)
    }

    fn model(&self) -> Option<&str> {
        Some(&self.inner.model)
    }

    fn usage(&self) -> Option<&CompletionUsage> {
        self.inner.usage.as_deref()
    }
}

/// Wrapper for streaming chat completion responses.
#[derive(Debug, Clone)]
pub struct ChatCompletionStreamResponseWrapper {
    inner: CreateChatCompletionStreamResponse,
}

impl ChatCompletionStreamResponseWrapper {
    /// Create a new stream response wrapper.
    pub fn new(response: CreateChatCompletionStreamResponse) -> Self {
        Self { inner: response }
    }

    /// Get the delta content from this chunk.
    pub fn delta_content(&self) -> Option<&str> {
        self.inner
            .choices
            .first()
            .and_then(|c| c.delta.content.as_ref())
            .and_then(|c| c.as_ref())
            .map(std::string::String::as_str)
    }

    /// Get tool call deltas from this chunk.
    pub fn delta_tool_calls(
        &self,
    ) -> Vec<&openai_client_base::models::ChatCompletionMessageToolCallChunk> {
        self.inner
            .choices
            .first()
            .and_then(|c| c.delta.tool_calls.as_ref())
            .map(|calls| calls.iter().collect())
            .unwrap_or_default()
    }

    /// Check if this is the final chunk.
    pub fn is_finished(&self) -> bool {
        use openai_client_base::models::create_chat_completion_stream_response_choices_inner::FinishReason;
        self.inner.choices.first().is_none_or(|c| {
            !matches!(
                c.finish_reason,
                FinishReason::Stop
                    | FinishReason::Length
                    | FinishReason::ToolCalls
                    | FinishReason::ContentFilter
                    | FinishReason::FunctionCall
            )
        })
    }

    /// Get the inner stream response object.
    pub fn inner(&self) -> &CreateChatCompletionStreamResponse {
        &self.inner
    }
}

// Helper functions for creating tools

/// Create a function tool definition.
#[must_use]
pub fn tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: serde_json::Value,
) -> ChatCompletionTool {
    use std::collections::HashMap;

    // Convert Value to HashMap<String, Value>
    let params_map = if let serde_json::Value::Object(map) = parameters {
        map.into_iter()
            .collect::<HashMap<String, serde_json::Value>>()
    } else {
        HashMap::new()
    };

    ChatCompletionTool {
        r#type: openai_client_base::models::chat_completion_tool::Type::Function,
        function: Box::new(FunctionObject {
            name: name.into(),
            description: Some(description.into()),
            parameters: Some(params_map),
            strict: None,
        }),
    }
}

/// Create a web search tool definition.
#[must_use]
pub fn tool_web_search() -> ChatCompletionTool {
    tool_function(
        "web_search",
        "Search the web for current information",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
    )
}

/// Helper for creating tool choice options.
pub struct ToolChoiceHelper;

impl ToolChoiceHelper {
    /// Let the model automatically decide whether to use tools.
    pub fn auto() -> ChatCompletionToolChoiceOption {
        use openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOptionAutoEnum;
        ChatCompletionToolChoiceOption::Auto(ChatCompletionToolChoiceOptionAutoEnum::Auto)
    }

    /// Prevent the model from using any tools.
    pub fn none() -> ChatCompletionToolChoiceOption {
        use openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOptionAutoEnum;
        ChatCompletionToolChoiceOption::Auto(ChatCompletionToolChoiceOptionAutoEnum::None)
    }

    /// Require the model to use a tool.
    pub fn required() -> ChatCompletionToolChoiceOption {
        use openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOptionAutoEnum;
        ChatCompletionToolChoiceOption::Auto(ChatCompletionToolChoiceOptionAutoEnum::Required)
    }

    /// Require the model to use a specific tool.
    pub fn specific(name: impl Into<String>) -> ChatCompletionToolChoiceOption {
        ChatCompletionToolChoiceOption::Chatcompletionnamedtoolchoice(
            openai_client_base::models::ChatCompletionNamedToolChoice {
                r#type:
                    openai_client_base::models::chat_completion_named_tool_choice::Type::Function,
                function: Box::new(AssistantsNamedToolChoiceFunction { name: name.into() }),
            },
        )
    }
}

/// Re-export commonly used types from openai-client-base for convenience
pub use openai_client_base::models::{
    ChatCompletionMessageToolCall as ToolCall,
    ChatCompletionResponseMessageFunctionCall as FunctionCall, ChatCompletionTool as Tool,
    ChatCompletionToolChoiceOption as ToolChoice, CompletionUsage as Usage,
    CreateChatCompletionResponse as ChatResponse,
    CreateChatCompletionStreamResponse as StreamResponse,
};

/// Placeholder for the `ResponseBuilder` until client is ready
#[derive(Debug, Clone)]
pub struct ResponseBuilder;

/// Placeholder for the Response struct
#[derive(Debug, Clone)]
pub struct ResponsePlaceholder;
