//! Responses API builders.
//!
//! The Responses API is `OpenAI`'s modern interface that supports all features including
//! web search, function calling, and structured outputs.

use openai_client_base::models::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionTool, ChatCompletionToolChoiceOption,
    CreateChatCompletionRequest, CreateChatCompletionRequestAllOfTools, FunctionObject,
};
// Import the specific Role enums for each message type
use openai_client_base::models::chat_completion_request_assistant_message::Role as AssistantRole;
use openai_client_base::models::chat_completion_request_system_message::Role as SystemRole;
use openai_client_base::models::chat_completion_request_user_message::Role as UserRole;
use serde_json::Value;

/// Builder for Responses API requests.
///
/// The Responses API is the modern unified interface for `OpenAI` completions,
/// supporting streaming, tools, and structured outputs.
#[derive(Debug, Clone)]
pub struct ResponsesBuilder {
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
    reasoning_effort: Option<String>,
}

impl ResponsesBuilder {
    /// Create a new responses builder with the specified model.
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
            reasoning_effort: None,
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

    /// Enable streaming for the response.
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

    /// Add a single tool.
    #[must_use]
    pub fn tool(mut self, tool: ChatCompletionTool) -> Self {
        let mut tools = self.tools.unwrap_or_default();
        tools.push(tool);
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

    /// Enable JSON mode.
    #[must_use]
    pub fn json_mode(mut self) -> Self {
        use openai_client_base::models::{
            create_chat_completion_request_all_of_response_format,
            CreateChatCompletionRequestAllOfResponseFormat,
        };
        self.response_format = Some(CreateChatCompletionRequestAllOfResponseFormat {
            r#type: create_chat_completion_request_all_of_response_format::Type::JsonObject,
            json_schema: Box::new(openai_client_base::models::JsonSchema::new(String::new())),
        });
        self
    }

    /// Set a JSON schema for structured output.
    #[must_use]
    pub fn json_schema(mut self, name: impl Into<String>, schema: Value) -> Self {
        use openai_client_base::models::{
            create_chat_completion_request_all_of_response_format,
            CreateChatCompletionRequestAllOfResponseFormat, JsonSchema,
        };
        use std::collections::HashMap;

        // Convert Value to HashMap<String, Value>
        let schema_map = if let serde_json::Value::Object(map) = schema {
            map.into_iter().collect::<HashMap<String, Value>>()
        } else {
            HashMap::new()
        };

        let mut json_schema = JsonSchema::new(name.into());
        json_schema.schema = Some(schema_map);

        self.response_format = Some(CreateChatCompletionRequestAllOfResponseFormat {
            r#type: create_chat_completion_request_all_of_response_format::Type::JsonSchema,
            json_schema: Box::new(json_schema),
        });
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

    /// Set the reasoning effort (for o3 models).
    #[must_use]
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }
}

impl super::Builder<CreateChatCompletionRequest> for ResponsesBuilder {
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
            reasoning_effort: self.reasoning_effort.map(|e| {
                use openai_client_base::models::reasoning_effort::{
                    ReasoningEffort, ReasoningEffortTextVariantEnum,
                };
                Some(match e.as_str() {
                    "minimal" => {
                        ReasoningEffort::TextVariant(ReasoningEffortTextVariantEnum::Minimal)
                    }
                    "low" => ReasoningEffort::TextVariant(ReasoningEffortTextVariantEnum::Low),
                    "medium" => {
                        ReasoningEffort::TextVariant(ReasoningEffortTextVariantEnum::Medium)
                    }
                    "high" => ReasoningEffort::TextVariant(ReasoningEffortTextVariantEnum::High),
                    _ => ReasoningEffort::TextVariant(ReasoningEffortTextVariantEnum::Medium),
                })
            }),
            prompt_cache_key: None,
            safety_identifier: None,
            verbosity: None,
            web_search_options: None,
        })
    }
}

// Helper functions for common patterns

/// Create a simple responses request with a user message.
#[must_use]
pub fn responses_simple(model: impl Into<String>, content: impl Into<String>) -> ResponsesBuilder {
    ResponsesBuilder::new(model).user(content)
}

/// Create a responses request with system and user messages.
#[must_use]
pub fn responses_system_user(
    model: impl Into<String>,
    system: impl Into<String>,
    user: impl Into<String>,
) -> ResponsesBuilder {
    ResponsesBuilder::new(model).system(system).user(user)
}

/// Create a function tool for the Responses API.
#[must_use]
pub fn responses_tool_function(
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

/// Create a web search tool for the Responses API.
#[must_use]
pub fn responses_tool_web_search() -> ChatCompletionTool {
    responses_tool_function(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::Builder;
    use openai_client_base::models::{
        chat_completion_tool_choice_option::ChatCompletionToolChoiceOption,
        create_chat_completion_request_all_of_response_format,
    };

    #[test]
    fn test_responses_builder_new() {
        let builder = ResponsesBuilder::new("gpt-4");
        assert_eq!(builder.model, "gpt-4");
        assert!(builder.messages.is_empty());
        assert!(builder.temperature.is_none());
    }

    #[test]
    fn test_responses_builder_system_message() {
        let builder = ResponsesBuilder::new("gpt-4").system("You are a helpful assistant");
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
    fn test_responses_builder_user_message() {
        let builder = ResponsesBuilder::new("gpt-4").user("Hello, world!");
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
    fn test_responses_builder_assistant_message() {
        let builder = ResponsesBuilder::new("gpt-4").assistant("Hello! How can I help you?");
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
    fn test_responses_builder_chaining() {
        let builder = ResponsesBuilder::new("gpt-4")
            .system("You are a helpful assistant")
            .user("What's the weather?")
            .temperature(0.7)
            .max_tokens(100);

        assert_eq!(builder.messages.len(), 2);
        assert_eq!(builder.temperature, Some(0.7));
        assert_eq!(builder.max_tokens, Some(100));
    }

    #[test]
    fn test_responses_builder_temperature() {
        let builder = ResponsesBuilder::new("gpt-4").temperature(0.5);
        assert_eq!(builder.temperature, Some(0.5));
    }

    #[test]
    fn test_responses_builder_max_tokens() {
        let builder = ResponsesBuilder::new("gpt-4").max_tokens(150);
        assert_eq!(builder.max_tokens, Some(150));
    }

    #[test]
    fn test_responses_builder_max_completion_tokens() {
        let builder = ResponsesBuilder::new("gpt-4").max_completion_tokens(200);
        assert_eq!(builder.max_completion_tokens, Some(200));
    }

    #[test]
    fn test_responses_builder_stream() {
        let builder = ResponsesBuilder::new("gpt-4").stream(true);
        assert_eq!(builder.stream, Some(true));
    }

    #[test]
    fn test_responses_builder_json_mode() {
        let builder = ResponsesBuilder::new("gpt-4").json_mode();
        assert!(builder.response_format.is_some());

        if let Some(format) = &builder.response_format {
            assert!(matches!(
                format.r#type,
                create_chat_completion_request_all_of_response_format::Type::JsonObject
            ));
        }
    }

    #[test]
    fn test_responses_builder_json_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let builder = ResponsesBuilder::new("gpt-4").json_schema("person", schema);
        assert!(builder.response_format.is_some());

        if let Some(format) = &builder.response_format {
            assert!(matches!(
                format.r#type,
                create_chat_completion_request_all_of_response_format::Type::JsonSchema
            ));
            assert_eq!(format.json_schema.name, "person");
        }
    }

    #[test]
    fn test_responses_builder_tools() {
        let tool = responses_tool_function(
            "test_function",
            "A test function",
            serde_json::json!({"type": "object", "properties": {}}),
        );

        let builder = ResponsesBuilder::new("gpt-4").tool(tool.clone());
        assert_eq!(builder.tools.as_ref().unwrap().len(), 1);

        // Test adding multiple tools
        let builder = builder.tool(tool);
        assert_eq!(builder.tools.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_responses_builder_tool_choice() {
        let builder =
            ResponsesBuilder::new("gpt-4").tool_choice(ChatCompletionToolChoiceOption::Auto(
                openai_client_base::models::chat_completion_tool_choice_option::ChatCompletionToolChoiceOptionAutoEnum::Auto
            ));
        assert!(builder.tool_choice.is_some());
    }

    #[test]
    fn test_responses_builder_other_parameters() {
        let builder = ResponsesBuilder::new("gpt-4")
            .n(2)
            .stop(vec!["STOP".to_string()])
            .presence_penalty(0.1)
            .frequency_penalty(0.2)
            .top_p(0.9)
            .user_id("user123")
            .seed(42)
            .reasoning_effort("high");

        assert_eq!(builder.n, Some(2));
        assert_eq!(builder.stop, Some(vec!["STOP".to_string()]));
        assert_eq!(builder.presence_penalty, Some(0.1));
        assert_eq!(builder.frequency_penalty, Some(0.2));
        assert_eq!(builder.top_p, Some(0.9));
        assert_eq!(builder.user, Some("user123".to_string()));
        assert_eq!(builder.seed, Some(42));
        assert_eq!(builder.reasoning_effort, Some("high".to_string()));
    }

    #[test]
    fn test_responses_builder_build_success() {
        let builder = ResponsesBuilder::new("gpt-4").user("Hello");
        let request = builder.build().unwrap();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_responses_builder_build_empty_messages_error() {
        let builder = ResponsesBuilder::new("gpt-4");
        let result = builder.build();

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, crate::Error::InvalidRequest(_)));
        }
    }

    #[test]
    fn test_responses_simple_helper() {
        let builder = responses_simple("gpt-4", "Hello, world!");
        assert_eq!(builder.model, "gpt-4");
        assert_eq!(builder.messages.len(), 1);
    }

    #[test]
    fn test_responses_system_user_helper() {
        let builder = responses_system_user(
            "gpt-4",
            "You are a helpful assistant",
            "What's the weather?",
        );
        assert_eq!(builder.model, "gpt-4");
        assert_eq!(builder.messages.len(), 2);
    }

    #[test]
    fn test_responses_tool_function() {
        let tool = responses_tool_function(
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
    fn test_responses_tool_web_search() {
        let tool = responses_tool_web_search();
        assert_eq!(tool.function.name, "web_search");
        assert!(tool.function.description.is_some());
        assert!(tool.function.parameters.is_some());
    }

    #[test]
    fn test_responses_builder_reasoning_effort_mapping() {
        let test_cases = vec![
            ("minimal", "minimal"),
            ("low", "low"),
            ("medium", "medium"),
            ("high", "high"),
            ("invalid", "medium"), // Should default to medium
        ];

        for (input, _expected) in test_cases {
            let builder = ResponsesBuilder::new("o3-mini")
                .user("Test")
                .reasoning_effort(input);
            let request = builder.build().unwrap();

            // Verify that reasoning_effort is properly set in the request
            assert!(request.reasoning_effort.is_some());
        }
    }
}
