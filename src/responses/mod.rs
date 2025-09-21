//! Response type wrappers and ergonomic helpers.
//!
//! This module provides ergonomic wrappers around `OpenAI` API responses with
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
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion (if applicable)
    pub completion_tokens: Option<u32>,
    /// Total number of tokens used
    pub total_tokens: u32,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Type of tool (usually "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition for the tool
    pub function: ToolFunction,
}

/// Function definition for tool calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    /// Name of the function
    pub name: String,
    /// Description of what the function does
    pub description: Option<String>,
    /// JSON Schema defining the function parameters
    pub parameters: Option<serde_json::Value>,
}

/// Tool choice options for responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Let the model decide whether to call tools
    Auto,
    /// Disable tool calling
    None,
    /// Force the model to call a tool
    Required,
    /// Force the model to call a specific function
    Function {
        /// The function to call
        function: ToolFunction,
    },
}

/// Builder for responses-first structured output requests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResponseBuilder {
    model: String,
    messages: Vec<crate::builders::ChatMessage>,
    tools: Vec<Tool>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<serde_json::Value>,
}

impl ResponseBuilder {
    /// Create a new response builder with the specified model.
    #[must_use]
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
    #[must_use]
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        self.messages.push(crate::builders::ChatMessage {
            role: role.into(),
            content: content.into(),
        });
        self
    }

    /// Add a tool for function calling.
    #[must_use]
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the tool choice strategy.
    #[must_use]
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Set the response format for structured outputs.
    #[must_use]
    pub fn response_format(mut self, format: serde_json::Value) -> Self {
        self.response_format = Some(format);
        self
    }
}

// Helper functions for common tool patterns

/// Create a web search tool definition.
#[must_use]
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
#[must_use]
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
