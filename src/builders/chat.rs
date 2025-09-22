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
        }
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
}

impl super::Builder<CreateChatCompletionRequest> for ChatCompletionBuilder {
    fn build(self) -> crate::Result<CreateChatCompletionRequest> {
        if self.messages.is_empty() {
            return Err(crate::Error::InvalidRequest(
                "At least one message is required".to_string(),
            ));
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
            seed: None,
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
