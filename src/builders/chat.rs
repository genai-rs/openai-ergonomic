//! Chat completion builders and helpers.
//!
//! This module provides ergonomic builders for chat completion requests,
//! including helpers for common message patterns and streaming responses.

use serde::{Deserialize, Serialize};

// TODO: Import actual types from openai-client-base once available
// use openai_client_base::types::*;

/// Placeholder for chat completion message until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Placeholder for chat completion request until openai-client-base is integrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

/// Builder for chat completion requests.
#[derive(Debug, Clone)]
pub struct ChatCompletionBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
}

impl ChatCompletionBuilder {
    /// Create a new chat completion builder with the specified model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: None,
        }
    }

    /// Add a system message to the conversation.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content: content.into(),
        });
        self
    }

    /// Add a user message to the conversation.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.into(),
        });
        self
    }

    /// Add an assistant message to the conversation.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: content.into(),
        });
        self
    }

    /// Set the temperature for the completion.
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens to generate.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming for the completion.
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

impl super::Builder<ChatCompletionRequest> for ChatCompletionBuilder {
    fn build(self) -> crate::Result<ChatCompletionRequest> {
        if self.messages.is_empty() {
            return Err(crate::Error::InvalidRequest(
                "At least one message is required".to_string(),
            ));
        }

        Ok(ChatCompletionRequest {
            model: self.model,
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
        })
    }
}

// TODO: Implement Sendable trait once client is available
// impl super::Sendable<ChatCompletionResponse> for ChatCompletionBuilder {
//     async fn send(self) -> crate::Result<ChatCompletionResponse> {
//         // Implementation will use the client wrapper
//         todo!("Implement once client wrapper is available")
//     }
// }

/// Helper function to create a simple user message chat completion.
pub fn user_message(model: impl Into<String>, content: impl Into<String>) -> ChatCompletionBuilder {
    ChatCompletionBuilder::new(model).user(content)
}

/// Helper function to create a system + user message chat completion.
pub fn system_user(
    model: impl Into<String>,
    system: impl Into<String>,
    user: impl Into<String>,
) -> ChatCompletionBuilder {
    ChatCompletionBuilder::new(model).system(system).user(user)
}
