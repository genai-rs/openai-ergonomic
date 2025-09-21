//! Chat completion response types and helpers.

use serde::{Deserialize, Serialize};

/// Placeholder for chat completion response until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<super::Usage>,
}

/// Placeholder for chat choice until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Placeholder for chat message until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Placeholder for tool call until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

/// Placeholder for function call until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
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
