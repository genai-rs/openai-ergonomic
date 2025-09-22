//! Chat completion response types and helpers.

use openai_client_base::models::{
    ChatCompletionMessageToolCallsInner, ChatCompletionResponseMessage,
    CreateChatCompletionResponse, CreateChatCompletionResponseChoicesInner,
};

/// Extension trait for chat completion responses.
pub trait ChatCompletionResponseExt {
    /// Get the content of the first choice, if available.
    fn content(&self) -> Option<&str>;

    /// Get the tool calls from the first choice, if available.
    fn tool_calls(&self) -> Vec<&ChatCompletionMessageToolCallsInner>;

    /// Check if the response has tool calls.
    fn has_tool_calls(&self) -> bool;

    /// Get the first choice from the response.
    fn first_choice(&self) -> Option<&CreateChatCompletionResponseChoicesInner>;

    /// Get the message from the first choice.
    fn first_message(&self) -> Option<&ChatCompletionResponseMessage>;

    /// Check if the response was refused.
    fn is_refusal(&self) -> bool;

    /// Get the refusal message if the response was refused.
    fn refusal(&self) -> Option<&str>;

    /// Get the finish reason for the first choice.
    fn finish_reason(&self) -> Option<&str>;
}

impl ChatCompletionResponseExt for CreateChatCompletionResponse {
    fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|choice| choice.message.content.as_deref())
    }

    fn tool_calls(&self) -> Vec<&ChatCompletionMessageToolCallsInner> {
        self.choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_ref())
            .map(|calls| calls.iter().collect())
            .unwrap_or_default()
    }

    fn has_tool_calls(&self) -> bool {
        !self.tool_calls().is_empty()
    }

    fn first_choice(&self) -> Option<&CreateChatCompletionResponseChoicesInner> {
        self.choices.first()
    }

    fn first_message(&self) -> Option<&ChatCompletionResponseMessage> {
        self.first_choice().map(|choice| &choice.message)
    }

    fn is_refusal(&self) -> bool {
        self.first_message()
            .and_then(|msg| msg.refusal.as_ref())
            .is_some()
    }

    fn refusal(&self) -> Option<&str> {
        self.first_message()
            .and_then(|msg| msg.refusal.as_ref())
            .and_then(|r| r.as_ref())
            .map(|s| s.as_str())
    }

    fn finish_reason(&self) -> Option<String> {
        use openai_client_base::models::create_chat_completion_response_choices_inner::FinishReason;
        self.first_choice()
            .map(|choice| match &choice.finish_reason {
                FinishReason::Stop => "stop".to_string(),
                FinishReason::Length => "length".to_string(),
                FinishReason::ToolCalls => "tool_calls".to_string(),
                FinishReason::ContentFilter => "content_filter".to_string(),
                FinishReason::FunctionCall => "function_call".to_string(),
            })
    }
}

/// Extension trait for tool calls.
pub trait ToolCallExt {
    /// Get the function name from the tool call.
    fn function_name(&self) -> &str;

    /// Get the function arguments as a string.
    fn function_arguments(&self) -> &str;

    /// Parse the function arguments as JSON.
    fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error>;
}

impl ToolCallExt for ChatCompletionMessageToolCallsInner {
    fn function_name(&self) -> &str {
        &self.function.name
    }

    fn function_arguments(&self) -> &str {
        &self.function.arguments
    }

    fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.function.arguments)
    }
}

// Re-export types for convenience
pub use openai_client_base::models::{
    ChatCompletionResponseMessage as ChatMessage,
    CreateChatCompletionResponse as ChatCompletionResponse,
    CreateChatCompletionResponseChoicesInner as ChatChoice,
};

// Re-export the FunctionCall type with a more ergonomic alias
pub use openai_client_base::models::{
    ChatCompletionMessageToolCallFunction as FunctionCall,
    ChatCompletionResponseMessageFunctionCall,
};

// Re-export ToolCall with an alias
pub use openai_client_base::models::ChatCompletionMessageToolCallsInner as ToolCall;
