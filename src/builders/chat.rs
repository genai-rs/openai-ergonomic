//! Chat completion builders and helpers.
//!
//! This module provides ergonomic builders for chat completion requests,
//! including helpers for common message patterns and streaming responses.

use openai_client_base::models::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImage,
    ChatCompletionRequestMessageContentPartImageImageUrl,
    ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
    ChatCompletionTool, ChatCompletionToolChoiceOption, CreateChatCompletionRequest,
    CreateChatCompletionRequestAllOfTools, FunctionObject,
};
// Import the specific Role enums for each message type
use openai_client_base::models::chat_completion_request_assistant_message::Role as AssistantRole;
use openai_client_base::models::chat_completion_request_system_message::Role as SystemRole;
use openai_client_base::models::chat_completion_request_user_message::Role as UserRole;
// Import the Type enums for content parts
use openai_client_base::models::chat_completion_request_message_content_part_image::Type as ImageType;
use openai_client_base::models::chat_completion_request_message_content_part_image_image_url::Detail;
use openai_client_base::models::chat_completion_request_message_content_part_text::Type as TextType;
use serde_json::Value;

/// Builder for chat completion requests.
#[derive(Debug, Clone)]
pub struct ChatCompletionBuilder {
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i32>,
    max_completion_tokens: Option<i32>,
    stream: Option<bool>,
    tools: Option<Vec<ChatCompletionTool>>,
    tool_choice: Option<ChatCompletionToolChoiceOption>,
    response_format:
        Option<openai_client_base::models::CreateChatCompletionRequestAllOfResponseFormat>,
    n: Option<i32>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f64>,
    frequency_penalty: Option<f64>,
    top_p: Option<f64>,
    user: Option<String>,
    seed: Option<i32>,
    #[cfg(feature = "telemetry")]
    pub(crate) telemetry_context: Option<crate::telemetry::TelemetryContext>,
}

impl ChatCompletionBuilder {
    /// Create a new chat completion builder with the specified model.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            top_p: None,
            user: None,
            seed: None,
            #[cfg(feature = "telemetry")]
            telemetry_context: None,
        }
    }

    /// Set telemetry context for observability.
    ///
    /// This allows you to attach custom attributes like user ID, session ID, and tags
    /// to the OpenTelemetry spans created for this request.
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn with_telemetry_context(mut self, context: crate::telemetry::TelemetryContext) -> Self {
        self.telemetry_context = Some(context);
        self
    }

    /// Set user ID for telemetry (convenience method).
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        let ctx = self
            .telemetry_context
            .take()
            .unwrap_or_default()
            .with_user_id(user_id);
        self.telemetry_context = Some(ctx);
        self
    }

    /// Set session ID for telemetry (convenience method).
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        let ctx = self
            .telemetry_context
            .take()
            .unwrap_or_default()
            .with_session_id(session_id);
        self.telemetry_context = Some(ctx);
        self
    }

    /// Add a tag for telemetry (convenience method).
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let ctx = self
            .telemetry_context
            .take()
            .unwrap_or_default()
            .with_tag(tag);
        self.telemetry_context = Some(ctx);
        self
    }

    /// Add a system message to the conversation.
    #[must_use]
    pub fn system(mut self, content: impl Into<String>) -> Self {
        let message = ChatCompletionRequestSystemMessage {
            content: Box::new(ChatCompletionRequestSystemMessageContent::TextContent(
                content.into(),
            )),
            role: SystemRole::System,
            name: None,
        };
        self.messages.push(
            ChatCompletionRequestMessage::ChatCompletionRequestSystemMessage(Box::new(message)),
        );
        self
    }

    /// Add a user message to the conversation.
    #[must_use]
    pub fn user(mut self, content: impl Into<String>) -> Self {
        let message = ChatCompletionRequestUserMessage {
            content: Box::new(ChatCompletionRequestUserMessageContent::TextContent(
                content.into(),
            )),
            role: UserRole::User,
            name: None,
        };
        self.messages.push(
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(Box::new(message)),
        );
        self
    }

    /// Add a user message with both text and an image URL.
    #[must_use]
    pub fn user_with_image_url(
        self,
        text: impl Into<String>,
        image_url: impl Into<String>,
    ) -> Self {
        self.user_with_image_url_and_detail(text, image_url, Detail::Auto)
    }

    /// Add a user message with both text and an image URL with specified detail level.
    #[must_use]
    pub fn user_with_image_url_and_detail(
        mut self,
        text: impl Into<String>,
        image_url: impl Into<String>,
        detail: Detail,
    ) -> Self {
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

        let message = ChatCompletionRequestUserMessage {
            content: Box::new(
                ChatCompletionRequestUserMessageContent::ArrayOfContentParts(vec![
                    text_part, image_part,
                ]),
            ),
            role: UserRole::User,
            name: None,
        };

        self.messages.push(
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(Box::new(message)),
        );
        self
    }

    /// Add a user message with multiple content parts (text and/or images).
    #[must_use]
    pub fn user_with_parts(
        mut self,
        parts: Vec<ChatCompletionRequestUserMessageContentPart>,
    ) -> Self {
        let message = ChatCompletionRequestUserMessage {
            content: Box::new(ChatCompletionRequestUserMessageContent::ArrayOfContentParts(parts)),
            role: UserRole::User,
            name: None,
        };

        self.messages.push(
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(Box::new(message)),
        );
        self
    }

    /// Add an assistant message to the conversation.
    #[must_use]
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        let message = ChatCompletionRequestAssistantMessage {
            content: Some(Some(Box::new(
                ChatCompletionRequestAssistantMessageContent::TextContent(content.into()),
            ))),
            role: AssistantRole::Assistant,
            name: None,
            tool_calls: None,
            function_call: None,
            audio: None,
            refusal: None,
        };
        self.messages.push(
            ChatCompletionRequestMessage::ChatCompletionRequestAssistantMessage(Box::new(message)),
        );
        self
    }

    /// Set the temperature for the completion.
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens to generate.
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the maximum completion tokens (for newer models).
    #[must_use]
    pub fn max_completion_tokens(mut self, max_completion_tokens: i32) -> Self {
        self.max_completion_tokens = Some(max_completion_tokens);
        self
    }

    /// Enable streaming for the completion.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Add tools that the model can use.
    #[must_use]
    pub fn tools(mut self, tools: Vec<ChatCompletionTool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice option.
    #[must_use]
    pub fn tool_choice(mut self, tool_choice: ChatCompletionToolChoiceOption) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Set the response format.
    #[must_use]
    pub fn response_format(
        mut self,
        format: openai_client_base::models::CreateChatCompletionRequestAllOfResponseFormat,
    ) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the number of completions to generate.
    #[must_use]
    pub fn n(mut self, n: i32) -> Self {
        self.n = Some(n);
        self
    }

    /// Set stop sequences.
    #[must_use]
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set the presence penalty.
    #[must_use]
    pub fn presence_penalty(mut self, presence_penalty: f64) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    /// Set the frequency penalty.
    #[must_use]
    pub fn frequency_penalty(mut self, frequency_penalty: f64) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    /// Set the top-p value.
    #[must_use]
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the user identifier.
    #[must_use]
    pub fn user_id(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set the random seed for deterministic outputs.
    #[must_use]
    pub fn seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl super::Builder<CreateChatCompletionRequest> for ChatCompletionBuilder {
    #[allow(clippy::too_many_lines)]
    fn build(self) -> crate::Result<CreateChatCompletionRequest> {
        // Validate model
        if self.model.trim().is_empty() {
            return Err(crate::Error::InvalidRequest(
                "Model cannot be empty".to_string(),
            ));
        }

        // Validate messages
        if self.messages.is_empty() {
            return Err(crate::Error::InvalidRequest(
                "At least one message is required".to_string(),
            ));
        }

        // Validate message contents
        for (i, message) in self.messages.iter().enumerate() {
            match message {
                ChatCompletionRequestMessage::ChatCompletionRequestSystemMessage(msg) => {
                    if let ChatCompletionRequestSystemMessageContent::TextContent(content) =
                        msg.content.as_ref()
                    {
                        if content.trim().is_empty() {
                            return Err(crate::Error::InvalidRequest(format!(
                                "System message at index {i} cannot have empty content"
                            )));
                        }
                    }
                }
                ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
                    match msg.content.as_ref() {
                        ChatCompletionRequestUserMessageContent::TextContent(content) => {
                            if content.trim().is_empty() {
                                return Err(crate::Error::InvalidRequest(format!(
                                    "User message at index {i} cannot have empty content"
                                )));
                            }
                        }
                        ChatCompletionRequestUserMessageContent::ArrayOfContentParts(parts) => {
                            if parts.is_empty() {
                                return Err(crate::Error::InvalidRequest(format!(
                                    "User message at index {i} cannot have empty content parts"
                                )));
                            }
                        }
                    }
                }
                ChatCompletionRequestMessage::ChatCompletionRequestAssistantMessage(msg) => {
                    // Assistant messages can have content or tool calls, but not both empty
                    let has_content = msg
                        .content
                        .as_ref()
                        .and_then(|opt| opt.as_ref())
                        .is_some_and(|c| {
                            match c.as_ref() {
                                ChatCompletionRequestAssistantMessageContent::TextContent(text) => {
                                    !text.trim().is_empty()
                                }
                                _ => true, // Other content types are considered valid
                            }
                        });
                    let has_tool_calls = msg.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());

                    if !has_content && !has_tool_calls {
                        return Err(crate::Error::InvalidRequest(format!(
                            "Assistant message at index {i} must have either content or tool calls"
                        )));
                    }
                }
                _ => {
                    // Other message types (tool, function) are valid as-is
                }
            }
        }

        // Validate temperature
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(crate::Error::InvalidRequest(format!(
                    "temperature must be between 0.0 and 2.0, got {temp}"
                )));
            }
        }

        // Validate top_p
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(crate::Error::InvalidRequest(format!(
                    "top_p must be between 0.0 and 1.0, got {top_p}"
                )));
            }
        }

        // Validate frequency_penalty
        if let Some(freq) = self.frequency_penalty {
            if !(-2.0..=2.0).contains(&freq) {
                return Err(crate::Error::InvalidRequest(format!(
                    "frequency_penalty must be between -2.0 and 2.0, got {freq}"
                )));
            }
        }

        // Validate presence_penalty
        if let Some(pres) = self.presence_penalty {
            if !(-2.0..=2.0).contains(&pres) {
                return Err(crate::Error::InvalidRequest(format!(
                    "presence_penalty must be between -2.0 and 2.0, got {pres}"
                )));
            }
        }

        // Validate max_tokens
        if let Some(max_tokens) = self.max_tokens {
            if max_tokens <= 0 {
                return Err(crate::Error::InvalidRequest(format!(
                    "max_tokens must be positive, got {max_tokens}"
                )));
            }
        }

        // Validate max_completion_tokens
        if let Some(max_completion_tokens) = self.max_completion_tokens {
            if max_completion_tokens <= 0 {
                return Err(crate::Error::InvalidRequest(format!(
                    "max_completion_tokens must be positive, got {max_completion_tokens}"
                )));
            }
        }

        // Validate n
        if let Some(n) = self.n {
            if n <= 0 {
                return Err(crate::Error::InvalidRequest(format!(
                    "n must be positive, got {n}"
                )));
            }
        }

        // Validate tools
        if let Some(ref tools) = self.tools {
            for (i, tool) in tools.iter().enumerate() {
                let function = &tool.function;

                // Validate function name
                if function.name.trim().is_empty() {
                    return Err(crate::Error::InvalidRequest(format!(
                        "Tool {i} function name cannot be empty"
                    )));
                }

                // Validate function name contains only valid characters
                if !function
                    .name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    return Err(crate::Error::InvalidRequest(format!(
                        "Tool {} function name '{}' contains invalid characters",
                        i, function.name
                    )));
                }

                // Validate function description
                if let Some(ref description) = &function.description {
                    if description.trim().is_empty() {
                        return Err(crate::Error::InvalidRequest(format!(
                            "Tool {i} function description cannot be empty"
                        )));
                    }
                }
            }
        }

        let response_format = self.response_format.map(Box::new);

        Ok(CreateChatCompletionRequest {
            messages: self.messages,
            model: self.model,
            frequency_penalty: self.frequency_penalty,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            max_tokens: self.max_tokens,
            max_completion_tokens: self.max_completion_tokens,
            n: self.n,
            modalities: None,
            prediction: None,
            audio: None,
            presence_penalty: self.presence_penalty,
            response_format,
            seed: self.seed,
            service_tier: None,
            stop: self.stop.map(|s| {
                Box::new(openai_client_base::models::StopConfiguration::ArrayOfStrings(s))
            }),
            stream: self.stream,
            stream_options: None,
            temperature: self.temperature,
            top_p: self.top_p,
            tools: self.tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|tool| {
                        CreateChatCompletionRequestAllOfTools::ChatCompletionTool(Box::new(tool))
                    })
                    .collect()
            }),
            tool_choice: self.tool_choice.map(Box::new),
            parallel_tool_calls: None,
            user: self.user,
            function_call: None,
            functions: None,
            store: None,
            metadata: None,
            reasoning_effort: None,
            prompt_cache_key: None,
            safety_identifier: None,
            verbosity: None,
            web_search_options: None,
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
#[must_use]
pub fn user_message(model: impl Into<String>, content: impl Into<String>) -> ChatCompletionBuilder {
    ChatCompletionBuilder::new(model).user(content)
}

/// Helper function to create a system + user message chat completion.
#[must_use]
pub fn system_user(
    model: impl Into<String>,
    system: impl Into<String>,
    user: impl Into<String>,
) -> ChatCompletionBuilder {
    ChatCompletionBuilder::new(model).system(system).user(user)
}

/// Helper function to create a function tool.
#[must_use]
pub fn tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: Value,
) -> ChatCompletionTool {
    use std::collections::HashMap;

    // Convert Value to HashMap<String, Value>
    let params_map = if let serde_json::Value::Object(map) = parameters {
        map.into_iter().collect::<HashMap<String, Value>>()
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

/// Helper function to create a web search tool.
#[must_use]
pub fn tool_web_search() -> ChatCompletionTool {
    tool_function(
        "web_search",
        "Search the web for information",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        }),
    )
}

/// Helper function to create a text content part.
#[must_use]
pub fn text_part(content: impl Into<String>) -> ChatCompletionRequestUserMessageContentPart {
    ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartText(
        Box::new(ChatCompletionRequestMessageContentPartText {
            r#type: TextType::Text,
            text: content.into(),
        }),
    )
}

/// Helper function to create an image content part from a URL with auto detail.
#[must_use]
pub fn image_url_part(url: impl Into<String>) -> ChatCompletionRequestUserMessageContentPart {
    image_url_part_with_detail(url, Detail::Auto)
}

/// Helper function to create an image content part from a URL with specified detail level.
#[must_use]
pub fn image_url_part_with_detail(
    url: impl Into<String>,
    detail: Detail,
) -> ChatCompletionRequestUserMessageContentPart {
    ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(
        Box::new(ChatCompletionRequestMessageContentPartImage {
            r#type: ImageType::ImageUrl,
            image_url: Box::new(ChatCompletionRequestMessageContentPartImageImageUrl {
                url: url.into(),
                detail: Some(detail),
            }),
        }),
    )
}

/// Helper function to create an image content part from base64 data with auto detail.
#[must_use]
pub fn image_base64_part(
    base64_data: impl Into<String>,
    media_type: impl Into<String>,
) -> ChatCompletionRequestUserMessageContentPart {
    image_base64_part_with_detail(base64_data, media_type, Detail::Auto)
}

/// Helper function to create an image content part from base64 data with specified detail level.
#[must_use]
pub fn image_base64_part_with_detail(
    base64_data: impl Into<String>,
    media_type: impl Into<String>,
    detail: Detail,
) -> ChatCompletionRequestUserMessageContentPart {
    let data_url = format!("data:{};base64,{}", media_type.into(), base64_data.into());
    image_url_part_with_detail(data_url, detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::Builder;
    use openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOption;

    #[test]
    fn test_chat_completion_builder_new() {
        let builder = ChatCompletionBuilder::new("gpt-4");
        assert_eq!(builder.model, "gpt-4");
        assert!(builder.messages.is_empty());
        assert!(builder.temperature.is_none());
    }

    #[test]
    fn test_chat_completion_builder_system_message() {
        let builder = ChatCompletionBuilder::new("gpt-4").system("You are a helpful assistant");
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestSystemMessage(msg) => {
                match msg.content.as_ref() {
                    ChatCompletionRequestSystemMessageContent::TextContent(content) => {
                        assert_eq!(content, "You are a helpful assistant");
                    }
                    ChatCompletionRequestSystemMessageContent::ArrayOfContentParts(_) => {
                        panic!("Expected text content")
                    }
                }
            }
            _ => panic!("Expected system message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_user_message() {
        let builder = ChatCompletionBuilder::new("gpt-4").user("Hello, world!");
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
                match msg.content.as_ref() {
                    ChatCompletionRequestUserMessageContent::TextContent(content) => {
                        assert_eq!(content, "Hello, world!");
                    }
                    ChatCompletionRequestUserMessageContent::ArrayOfContentParts(_) => {
                        panic!("Expected text content")
                    }
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_assistant_message() {
        let builder = ChatCompletionBuilder::new("gpt-4").assistant("Hello! How can I help you?");
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestAssistantMessage(msg) => {
                if let Some(Some(content)) = &msg.content {
                    match content.as_ref() {
                        ChatCompletionRequestAssistantMessageContent::TextContent(text) => {
                            assert_eq!(text, "Hello! How can I help you?");
                        }
                        _ => panic!("Expected text content"),
                    }
                } else {
                    panic!("Expected content");
                }
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_user_with_image_url() {
        let builder = ChatCompletionBuilder::new("gpt-4")
            .user_with_image_url("Describe this image", "https://example.com/image.jpg");
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
                match msg.content.as_ref() {
                    ChatCompletionRequestUserMessageContent::ArrayOfContentParts(parts) => {
                        assert_eq!(parts.len(), 2);

                        // Check text part
                        match &parts[0] {
                            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartText(text_part) => {
                                assert_eq!(text_part.text, "Describe this image");
                            }
                            _ => panic!("Expected text part"),
                        }

                        // Check image part
                        match &parts[1] {
                            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                                assert_eq!(image_part.image_url.url, "https://example.com/image.jpg");
                                assert_eq!(image_part.image_url.detail, Some(Detail::Auto));
                            }
                            _ => panic!("Expected image part"),
                        }
                    }
                    ChatCompletionRequestUserMessageContent::TextContent(_) => {
                        panic!("Expected array of content parts")
                    }
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_user_with_image_url_and_detail() {
        let builder = ChatCompletionBuilder::new("gpt-4").user_with_image_url_and_detail(
            "Describe this image",
            "https://example.com/image.jpg",
            Detail::High,
        );
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
                match msg.content.as_ref() {
                    ChatCompletionRequestUserMessageContent::ArrayOfContentParts(parts) => {
                        assert_eq!(parts.len(), 2);

                        // Check image part detail
                        match &parts[1] {
                            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                                assert_eq!(image_part.image_url.detail, Some(Detail::High));
                            }
                            _ => panic!("Expected image part"),
                        }
                    }
                    ChatCompletionRequestUserMessageContent::TextContent(_) => {
                        panic!("Expected array of content parts")
                    }
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_user_with_parts() {
        let text_part = text_part("Hello");
        let image_part = image_url_part("https://example.com/image.jpg");
        let parts = vec![text_part, image_part];

        let builder = ChatCompletionBuilder::new("gpt-4").user_with_parts(parts);
        assert_eq!(builder.messages.len(), 1);

        // Verify the message structure
        match &builder.messages[0] {
            ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
                match msg.content.as_ref() {
                    ChatCompletionRequestUserMessageContent::ArrayOfContentParts(parts) => {
                        assert_eq!(parts.len(), 2);
                    }
                    ChatCompletionRequestUserMessageContent::TextContent(_) => {
                        panic!("Expected array of content parts")
                    }
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_chat_completion_builder_chaining() {
        let builder = ChatCompletionBuilder::new("gpt-4")
            .system("You are a helpful assistant")
            .user("What's the weather?")
            .temperature(0.7)
            .max_tokens(100);

        assert_eq!(builder.messages.len(), 2);
        assert_eq!(builder.temperature, Some(0.7));
        assert_eq!(builder.max_tokens, Some(100));
    }

    #[test]
    fn test_chat_completion_builder_parameters() {
        let builder = ChatCompletionBuilder::new("gpt-4")
            .temperature(0.5)
            .max_tokens(150)
            .max_completion_tokens(200)
            .stream(true)
            .n(2)
            .stop(vec!["STOP".to_string()])
            .presence_penalty(0.1)
            .frequency_penalty(0.2)
            .top_p(0.9)
            .user_id("user123");

        assert_eq!(builder.temperature, Some(0.5));
        assert_eq!(builder.max_tokens, Some(150));
        assert_eq!(builder.max_completion_tokens, Some(200));
        assert_eq!(builder.stream, Some(true));
        assert_eq!(builder.n, Some(2));
        assert_eq!(builder.stop, Some(vec!["STOP".to_string()]));
        assert_eq!(builder.presence_penalty, Some(0.1));
        assert_eq!(builder.frequency_penalty, Some(0.2));
        assert_eq!(builder.top_p, Some(0.9));
        assert_eq!(builder.user, Some("user123".to_string()));
    }

    #[test]
    fn test_chat_completion_builder_tools() {
        let tool = tool_function(
            "test_function",
            "A test function",
            serde_json::json!({"type": "object", "properties": {}}),
        );

        let builder = ChatCompletionBuilder::new("gpt-4")
            .tools(vec![tool])
            .tool_choice(ChatCompletionToolChoiceOption::Auto(
                openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOptionAutoEnum::Auto
            ));

        assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
        assert!(builder.tool_choice.is_some());
    }

    #[test]
    fn test_chat_completion_builder_build_success() {
        let builder = ChatCompletionBuilder::new("gpt-4").user("Hello");
        let request = builder.build().unwrap();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_chat_completion_builder_build_empty_messages_error() {
        let builder = ChatCompletionBuilder::new("gpt-4");
        let result = builder.build();

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, crate::Error::InvalidRequest(_)));
        }
    }

    #[test]
    fn test_user_message_helper() {
        let builder = user_message("gpt-4", "Hello, world!");
        assert_eq!(builder.model, "gpt-4");
        assert_eq!(builder.messages.len(), 1);
    }

    #[test]
    fn test_system_user_helper() {
        let builder = system_user(
            "gpt-4",
            "You are a helpful assistant",
            "What's the weather?",
        );
        assert_eq!(builder.model, "gpt-4");
        assert_eq!(builder.messages.len(), 2);
    }

    #[test]
    fn test_tool_function() {
        let tool = tool_function(
            "get_weather",
            "Get current weather",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        );

        assert_eq!(tool.function.name, "get_weather");
        assert_eq!(
            tool.function.description.as_ref().unwrap(),
            "Get current weather"
        );
        assert!(tool.function.parameters.is_some());
    }

    #[test]
    fn test_tool_web_search() {
        let tool = tool_web_search();
        assert_eq!(tool.function.name, "web_search");
        assert!(tool.function.description.is_some());
        assert!(tool.function.parameters.is_some());
    }

    #[test]
    fn test_text_part() {
        let part = text_part("Hello, world!");
        match part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartText(text_part) => {
                assert_eq!(text_part.text, "Hello, world!");
                assert_eq!(text_part.r#type, TextType::Text);
            }
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_image_url_part() {
        let part = image_url_part("https://example.com/image.jpg");
        match part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                assert_eq!(image_part.image_url.url, "https://example.com/image.jpg");
                assert_eq!(image_part.image_url.detail, Some(Detail::Auto));
                assert_eq!(image_part.r#type, ImageType::ImageUrl);
            }
            _ => panic!("Expected image part"),
        }
    }

    #[test]
    fn test_image_url_part_with_detail() {
        let part = image_url_part_with_detail("https://example.com/image.jpg", Detail::Low);
        match part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                assert_eq!(image_part.image_url.url, "https://example.com/image.jpg");
                assert_eq!(image_part.image_url.detail, Some(Detail::Low));
                assert_eq!(image_part.r#type, ImageType::ImageUrl);
            }
            _ => panic!("Expected image part"),
        }
    }

    #[test]
    fn test_image_base64_part() {
        let part = image_base64_part("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==", "image/png");
        match part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                assert!(image_part.image_url.url.starts_with("data:image/png;base64,"));
                assert_eq!(image_part.image_url.detail, Some(Detail::Auto));
                assert_eq!(image_part.r#type, ImageType::ImageUrl);
            }
            _ => panic!("Expected image part"),
        }
    }

    #[test]
    fn test_image_base64_part_with_detail() {
        let part = image_base64_part_with_detail("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==", "image/jpeg", Detail::High);
        match part {
            ChatCompletionRequestUserMessageContentPart::ChatCompletionRequestMessageContentPartImage(image_part) => {
                assert!(image_part.image_url.url.starts_with("data:image/jpeg;base64,"));
                assert_eq!(image_part.image_url.detail, Some(Detail::High));
                assert_eq!(image_part.r#type, ImageType::ImageUrl);
            }
            _ => panic!("Expected image part"),
        }
    }

    #[test]
    fn test_tool_function_with_empty_parameters() {
        let tool = tool_function(
            "simple_function",
            "A simple function",
            serde_json::json!({}),
        );

        assert_eq!(tool.function.name, "simple_function");
        assert!(tool.function.parameters.is_some());
        assert!(tool.function.parameters.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_tool_function_with_invalid_parameters() {
        let tool = tool_function(
            "function_with_string_params",
            "A function with string parameters",
            serde_json::json!("not an object"),
        );

        assert_eq!(tool.function.name, "function_with_string_params");
        assert!(tool.function.parameters.is_some());
        // Should result in empty map when parameters is not an object
        assert!(tool.function.parameters.as_ref().unwrap().is_empty());
    }
}
