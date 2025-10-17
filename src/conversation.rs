//! Conversation utilities for managing multi-turn chat state.
//!
//! `ConversationState` offers a thin layer over [`ChatCompletionBuilder`]
//! that keeps the chat history in place, avoids repeated cloning when adding
//! messages, and caches tool payloads so JSON serialization only happens once.
//! It is designed to integrate directly with [`Client::execute_chat`] and
//! tooling workflows.
//!
//! # Example
//! ```rust,ignore
//! use openai_ergonomic::{Client, ConversationState, tool_function};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() -> openai_ergonomic::Result<()> {
//! let client = Client::from_env()?.build();
//! let mut state = ConversationState::new("gpt-4o-mini")
//!     .with_system("You are a helpful assistant.");
//!
//! state.push_user("What is on my calendar today?");
//! let request = state.build_request()?;
//! let response = client.execute_chat(request).await?;
//! state.apply_response(&response);
//!
//! if !response.tool_calls().is_empty() {
//!     // Call tools and push results back into the conversation
//!     let tool_result = ConversationState::tool_result(json!({
//!         "events": ["Project sync", "1:1"]
//!     }))?;
//!     state.push_tool_result("call_0", tool_result);
//!     let follow_up = state.build_request()?;
//!     let reply = client.execute_chat(follow_up).await?;
//!     state.apply_response(&reply);
//! }
//! # Ok(())
//! # }
//! ```

use std::{fmt, sync::Arc};

use crate::{
    builders::{chat::ChatCompletionBuilder, Builder},
    responses::ChatCompletionResponseWrapper,
    Error, Result,
};
use openai_client_base::models::{
    chat_completion_request_message_content_part_image::Type as ImageType,
    chat_completion_request_message_content_part_text::Type as TextType,
    ChatCompletionMessageToolCallsInner, ChatCompletionRequestMessageContentPartImage,
    ChatCompletionRequestMessageContentPartImageImageUrl,
    ChatCompletionRequestMessageContentPartText, ChatCompletionRequestUserMessageContentPart,
    ChatCompletionTool, ChatCompletionToolChoiceOption, CreateChatCompletionRequest,
    CreateChatCompletionRequestAllOfResponseFormat,
};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
struct ConversationConfig {
    temperature: Option<f64>,
    max_tokens: Option<i32>,
    max_completion_tokens: Option<i32>,
    stream: Option<bool>,
    tool_choice: Option<ChatCompletionToolChoiceOption>,
    response_format: Option<CreateChatCompletionRequestAllOfResponseFormat>,
    n: Option<i32>,
    stop: Option<Arc<Vec<String>>>,
    presence_penalty: Option<f64>,
    frequency_penalty: Option<f64>,
    top_p: Option<f64>,
    user: Option<String>,
    seed: Option<i32>,
    tools: Option<Arc<Vec<ChatCompletionTool>>>,
}

/// Represents the cached JSON payload returned by a tool invocation.
#[derive(Clone)]
pub struct ToolResult {
    value: Value,
    compact: Arc<String>,
}

impl ToolResult {
    /// Create a new `ToolResult` from a JSON value. The JSON is serialized
    /// exactly once and cached for reuse.
    pub fn new(value: Value) -> Result<Self> {
        let compact = serde_json::to_string(&value)?;
        Ok(Self {
            value,
            compact: Arc::new(compact),
        })
    }

    /// Create a `ToolResult` from an already serialized JSON string.
    #[must_use]
    pub fn from_serialized(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            value: Value::String(content.clone()),
            compact: Arc::new(content),
        }
    }

    /// Borrow the structured JSON value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Borrow the cached compact JSON string.
    pub fn as_str(&self) -> &str {
        self.compact.as_str()
    }
}

impl fmt::Debug for ToolResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolResult")
            .field("value", &self.value)
            .field("compact", &self.compact)
            .finish()
    }
}

#[derive(Debug, Clone)]
enum ConversationMessage {
    System(String),
    UserText(String),
    UserParts(Vec<ChatCompletionRequestUserMessageContentPart>),
    AssistantText(String),
    AssistantToolCalls {
        content: Option<String>,
        tool_calls: Vec<ChatCompletionMessageToolCallsInner>,
    },
    Tool {
        tool_call_id: String,
        result: ToolResult,
    },
}

/// Stateful builder for multi-turn chat conversations.
///
/// This type keeps the history of a conversation, offers ergonomic helpers for
/// mutating messages without cloning the entire request, and can generate
/// [`CreateChatCompletionRequest`] values on demand.
#[derive(Debug, Clone)]
pub struct ConversationState {
    model: String,
    messages: Vec<ConversationMessage>,
    config: ConversationConfig,
}

impl ConversationState {
    /// Create a new `ConversationState` for the given model.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            config: ConversationConfig::default(),
        }
    }

    /// Convenience constructor that sets the first system message.
    #[must_use]
    pub fn with_system(mut self, content: impl Into<String>) -> Self {
        self.push_system(content);
        self
    }

    /// Get the total number of messages in the conversation.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check whether the conversation currently has no messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Append a system message.
    pub fn push_system(&mut self, content: impl Into<String>) {
        self.messages
            .push(ConversationMessage::System(content.into()));
    }

    /// Append a user message with plain text content.
    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages
            .push(ConversationMessage::UserText(content.into()));
    }

    /// Append a user message constructed from content parts.
    pub fn push_user_with_parts(
        &mut self,
        parts: Vec<ChatCompletionRequestUserMessageContentPart>,
    ) -> Result<()> {
        if parts.is_empty() {
            return Err(Error::Builder(
                "User message parts cannot be empty".to_string(),
            ));
        }
        self.messages.push(ConversationMessage::UserParts(parts));
        Ok(())
    }

    /// Append a user message containing plain text and a single image URL.
    pub fn push_user_with_image_url(
        &mut self,
        text: impl Into<String>,
        image_url: impl Into<String>,
        detail: crate::Detail,
    ) {
        let text_part = ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartText(
            Box::new(ChatCompletionRequestMessageContentPartText {
                r#type: TextType::Text,
                text: text.into(),
            }),
        );

        let image_part = ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(
            Box::new(ChatCompletionRequestMessageContentPartImage {
                r#type: ImageType::ImageUrl,
                image_url: Box::new(ChatCompletionRequestMessageContentPartImageImageUrl {
                    url: image_url.into(),
                    detail: Some(detail),
                }),
            }),
        );

        // Safe unwrap: `push_user_with_parts` validates non-empty input.
        self.push_user_with_parts(vec![text_part, image_part])
            .expect("non-empty message parts must succeed");
    }

    /// Append an assistant message.
    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages
            .push(ConversationMessage::AssistantText(content.into()));
    }

    /// Append an assistant message that includes tool call directives.
    pub fn push_assistant_tool_calls(
        &mut self,
        content: Option<String>,
        tool_calls: Vec<ChatCompletionMessageToolCallsInner>,
    ) -> Result<()> {
        if content.as_ref().is_none_or(|c| c.trim().is_empty()) && tool_calls.is_empty() {
            return Err(Error::Builder(
                "Assistant tool call message requires content or tool calls".to_string(),
            ));
        }

        self.messages.push(ConversationMessage::AssistantToolCalls {
            content,
            tool_calls,
        });
        Ok(())
    }

    /// Append an assistant message with tool calls using text content.
    pub fn push_assistant_tool_calls_text(
        &mut self,
        content: impl Into<String>,
        tool_calls: Vec<ChatCompletionMessageToolCallsInner>,
    ) -> Result<()> {
        self.push_assistant_tool_calls(Some(content.into()), tool_calls)
    }

    /// Append the result of a tool invocation using cached JSON.
    pub fn push_tool_result(&mut self, tool_call_id: impl Into<String>, result: ToolResult) {
        self.messages.push(ConversationMessage::Tool {
            tool_call_id: tool_call_id.into(),
            result,
        });
    }

    /// Append the result of a tool invocation from an already serialized string.
    ///
    /// The provided string should already be valid JSON. It is reused without
    /// re-serializing to avoid additional allocations.
    pub fn push_tool_result_raw(
        &mut self,
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
    ) {
        let content = content.into();
        let result = ToolResult {
            value: Value::String(content.clone()),
            compact: Arc::new(content),
        };
        self.messages.push(ConversationMessage::Tool {
            tool_call_id: tool_call_id.into(),
            result,
        });
    }

    /// Convenience helper to create a [`ToolResult`] from a JSON value.
    pub fn tool_result(value: Value) -> Result<ToolResult> {
        ToolResult::new(value)
    }

    /// Apply the first choice from a response to the conversation history.
    pub fn apply_response(&mut self, response: &ChatCompletionResponseWrapper) {
        if let Some(choice) = response.choices().first() {
            let content = choice.message.content.clone();
            let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();

            if !tool_calls.is_empty() {
                let _ = self.push_assistant_tool_calls(content, tool_calls);
            } else if let Some(content) = content {
                if !content.trim().is_empty() {
                    self.push_assistant(content);
                }
            }
        }
    }

    /// Set the tools available to the model.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ChatCompletionTool>) -> Self {
        self.config.tools = Some(Arc::new(tools));
        self
    }

    /// Update the tools for the conversation.
    pub fn set_tools(&mut self, tools: Vec<ChatCompletionTool>) {
        self.config.tools = Some(Arc::new(tools));
    }

    /// Clear any configured tools.
    pub fn clear_tools(&mut self) {
        self.config.tools = None;
    }

    /// Set the temperature parameter.
    pub fn set_temperature(&mut self, temperature: f64) {
        self.config.temperature = Some(temperature);
    }

    /// Set the maximum number of tokens in the response.
    pub fn set_max_tokens(&mut self, max_tokens: i32) {
        self.config.max_tokens = Some(max_tokens);
    }

    /// Set the maximum completion tokens.
    pub fn set_max_completion_tokens(&mut self, max_completion_tokens: i32) {
        self.config.max_completion_tokens = Some(max_completion_tokens);
    }

    /// Enable or disable streaming mode.
    pub fn set_stream(&mut self, stream: bool) {
        self.config.stream = Some(stream);
    }

    /// Override the tool choice behaviour.
    pub fn set_tool_choice(&mut self, tool_choice: ChatCompletionToolChoiceOption) {
        self.config.tool_choice = Some(tool_choice);
    }

    /// Set the response format configuration.
    pub fn set_response_format(
        &mut self,
        response_format: CreateChatCompletionRequestAllOfResponseFormat,
    ) {
        self.config.response_format = Some(response_format);
    }

    /// Set the number of completions to generate.
    pub fn set_n(&mut self, n: i32) {
        self.config.n = Some(n);
    }

    /// Configure stop sequences.
    pub fn set_stop_sequences(&mut self, stop: Vec<String>) {
        self.config.stop = Some(Arc::new(stop));
    }

    /// Clear configured stop sequences.
    pub fn clear_stop_sequences(&mut self) {
        self.config.stop = None;
    }

    /// Set the presence penalty.
    pub fn set_presence_penalty(&mut self, presence_penalty: f64) {
        self.config.presence_penalty = Some(presence_penalty);
    }

    /// Set the frequency penalty.
    pub fn set_frequency_penalty(&mut self, frequency_penalty: f64) {
        self.config.frequency_penalty = Some(frequency_penalty);
    }

    /// Set the top-p sampling value.
    pub fn set_top_p(&mut self, top_p: f64) {
        self.config.top_p = Some(top_p);
    }

    /// Set the user identifier metadata.
    pub fn set_user(&mut self, user: impl Into<String>) {
        self.config.user = Some(user.into());
    }

    /// Set the sampling seed.
    pub fn set_seed(&mut self, seed: i32) {
        self.config.seed = Some(seed);
    }

    /// Build a [`CreateChatCompletionRequest`] for the current conversation.
    pub fn build_request(&self) -> Result<CreateChatCompletionRequest> {
        self.to_builder().build()
    }

    /// Convert the current conversation into a [`ChatCompletionBuilder`].
    pub fn to_builder(&self) -> ChatCompletionBuilder {
        let mut builder = ChatCompletionBuilder::new(self.model.clone());

        if let Some(temperature) = self.config.temperature {
            builder = builder.temperature(temperature);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            builder = builder.max_tokens(max_tokens);
        }
        if let Some(max_completion_tokens) = self.config.max_completion_tokens {
            builder = builder.max_completion_tokens(max_completion_tokens);
        }
        if let Some(stream) = self.config.stream {
            builder = builder.stream(stream);
        }
        if let Some(ref tools) = self.config.tools {
            builder = builder.tools((**tools).clone());
        }
        if let Some(ref tool_choice) = self.config.tool_choice {
            builder = builder.tool_choice(tool_choice.clone());
        }
        if let Some(ref response_format) = self.config.response_format {
            builder = builder.response_format(response_format.clone());
        }
        if let Some(n) = self.config.n {
            builder = builder.n(n);
        }
        if let Some(ref stop) = self.config.stop {
            builder = builder.stop((**stop).clone());
        }
        if let Some(presence_penalty) = self.config.presence_penalty {
            builder = builder.presence_penalty(presence_penalty);
        }
        if let Some(frequency_penalty) = self.config.frequency_penalty {
            builder = builder.frequency_penalty(frequency_penalty);
        }
        if let Some(top_p) = self.config.top_p {
            builder = builder.top_p(top_p);
        }
        if let Some(ref user) = self.config.user {
            builder = builder.user_id(user.clone());
        }
        if let Some(seed) = self.config.seed {
            builder = builder.seed(seed);
        }

        for message in &self.messages {
            builder = match message {
                ConversationMessage::System(content) => builder.system(content.clone()),
                ConversationMessage::UserText(content) => builder.user(content.clone()),
                ConversationMessage::UserParts(parts) => builder.user_with_parts(parts.clone()),
                ConversationMessage::AssistantText(content) => builder.assistant(content.clone()),
                ConversationMessage::AssistantToolCalls {
                    content,
                    tool_calls,
                } => builder.assistant_with_tool_calls(
                    content.clone().unwrap_or_default(),
                    tool_calls.clone(),
                ),
                ConversationMessage::Tool {
                    tool_call_id,
                    result,
                } => builder.tool(tool_call_id.clone(), result.as_str().to_string()),
            };
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client_base::models::{
        chat_completion_response_message::Role as ResponseRole,
        create_chat_completion_response::Object as ResponseObject,
        create_chat_completion_response_choices_inner::FinishReason as ChoiceFinishReason,
        ChatCompletionResponseMessage, CreateChatCompletionResponse,
        CreateChatCompletionResponseChoicesInner,
    };
    use serde_json::json;

    #[test]
    fn tool_result_caches_json() {
        let value = json!({ "key": "value", "nested": { "x": 1 } });
        let result = ToolResult::new(value.clone()).unwrap();
        assert_eq!(result.value(), &value);
        assert_eq!(result.as_str(), r#"{"key":"value","nested":{"x":1}}"#);
    }

    #[test]
    fn conversation_tracks_messages() {
        let mut state = ConversationState::new("gpt-4");
        state.push_system("You are helpful.");
        state.push_user("Hello!");
        state.push_assistant("Hi there");

        assert_eq!(state.len(), 3);
        assert!(!state.is_empty());
    }

    #[test]
    fn conversation_builds_request() {
        let mut state = ConversationState::new("gpt-4");
        state.set_temperature(0.4);
        state.push_system("System");
        state.push_user("User");
        state.push_assistant("Assistant");

        let request = state.build_request().unwrap();
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.temperature, Some(0.4));
    }

    #[test]
    fn conversation_handles_tool_results() {
        let mut state = ConversationState::new("gpt-4");
        let result = ConversationState::tool_result(json!({"value": 42})).unwrap();
        state.push_tool_result("call_123", result);
        assert!(state.push_assistant_tool_calls(None, Vec::new()).is_err());
    }

    #[test]
    fn apply_response_adds_assistant_content() {
        let mut state = ConversationState::new("gpt-4");
        let message = ChatCompletionResponseMessage::new(
            Some("Hello!".to_string()),
            None,
            ResponseRole::Assistant,
        );
        let choice = CreateChatCompletionResponseChoicesInner::new(
            ChoiceFinishReason::Stop,
            0,
            message,
            None,
        );
        let response = CreateChatCompletionResponse::new(
            "id".to_string(),
            vec![choice],
            0,
            "gpt-4".to_string(),
            ResponseObject::ChatCompletion,
        );
        let response_wrapper = ChatCompletionResponseWrapper::new(response);

        state.apply_response(&response_wrapper);
        assert_eq!(state.len(), 1);
    }
}
