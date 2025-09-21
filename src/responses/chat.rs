//! Chat completion response types and helpers.

use serde::{Deserialize, Serialize};

/// Placeholder for chat completion response until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the completion
    pub id: String,
    /// Model used for the completion
    pub model: String,
    /// Array of completion choices
    pub choices: Vec<ChatChoice>,
    /// Token usage information
    pub usage: Option<super::Usage>,
}

/// Placeholder for chat choice until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// Index of this choice in the array
    pub index: u32,
    /// Message content for this choice
    pub message: ChatMessage,
    /// Reason the model stopped generating
    pub finish_reason: Option<String>,
}

/// Placeholder for chat message until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender (system, user, assistant, etc.)
    pub role: String,
    /// Text content of the message
    pub content: Option<String>,
    /// Tool calls made by the assistant
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Placeholder for tool call until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for the tool call
    pub id: String,
    /// Type of tool (usually "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function call details
    pub function: FunctionCall,
}

/// Placeholder for function call until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call
    pub name: String,
    /// JSON-encoded arguments for the function
    pub arguments: String,
}

impl super::Response for ChatCompletionResponse {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }

    fn model(&self) -> Option<&str> {
        Some(&self.model)
    }

    fn usage(&self) -> Option<&super::Usage> {
        self.usage.as_ref()
    }
}

impl ChatCompletionResponse {
    /// Get the content of the first choice, if available.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|choice| choice.message.content.as_deref())
    }

    /// Get the tool calls from the first choice, if available.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_deref())
    }

    /// Check if the response has tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls().is_some_and(|calls| !calls.is_empty())
    }
}
