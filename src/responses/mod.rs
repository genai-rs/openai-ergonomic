//! Response type wrappers and ergonomic helpers.
//!
//! This module provides ergonomic wrappers around OpenAI API responses with
//! convenient methods for common operations. The responses-first approach
//! makes it easy to work with structured outputs and tool calling.
//!
//! # Example
//!
//! ```rust
//! # use openai_ergonomic::responses::*;
//! // TODO: Add example once responses are implemented
//! ```

use serde::{Deserialize, Serialize};

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
pub use assistants::*;
pub use audio::*;
pub use batch::*;
pub use chat::*;
pub use embeddings::*;
pub use files::*;
pub use fine_tuning::*;
pub use images::*;
pub use moderations::*;
pub use threads::*;
pub use uploads::*;
pub use vector_stores::*;

// TODO: Import actual types from openai-client-base once available
// use openai_client_base::responses::*;

/// Common trait for all response types to provide consistent access patterns.
pub trait Response {
    /// Get the unique identifier for this response, if available.
    fn id(&self) -> Option<&str>;

    /// Get the model that generated this response, if available.
    fn model(&self) -> Option<&str>;

    /// Get any usage information from the response, if available.
    fn usage(&self) -> Option<&Usage>;
}

/// Usage information for API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: Option<u32>,
    pub total_tokens: u32,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

/// Function definition for tool calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

/// Tool choice options for responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Function { function: ToolFunction },
}

/// Builder for responses-first structured output requests.
#[derive(Debug, Clone)]
pub struct ResponseBuilder {
    model: String,
    messages: Vec<crate::builders::ChatMessage>,
    tools: Vec<Tool>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<serde_json::Value>,
}

impl ResponseBuilder {
    /// Create a new response builder with the specified model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            response_format: None,
        }
    }

    /// Add a message to the conversation.
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        self.messages.push(crate::builders::ChatMessage {
            role: role.into(),
            content: content.into(),
        });
        self
    }

    /// Add a tool for function calling.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the tool choice strategy.
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Set the response format for structured outputs.
    pub fn response_format(mut self, format: serde_json::Value) -> Self {
        self.response_format = Some(format);
        self
    }
}

// Helper functions for common tool patterns

/// Create a web search tool definition.
pub fn tool_web_search() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "web_search".to_string(),
            description: Some("Search the web for current information".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            })),
        },
    }
}

/// Create a function tool definition with JSON schema parameters.
pub fn tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: serde_json::Value,
) -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: name.into(),
            description: Some(description.into()),
            parameters: Some(parameters),
        },
    }
}
