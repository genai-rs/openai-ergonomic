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
    ///
    /// `None` means the response has no choices. A reason this crate does not know is
    /// `Some("unknown")`, since the choice did finish, and the API's own text is not kept.
    fn finish_reason(&self) -> Option<String>;
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
            .and_then(|choice| choice.message.tool_calls.as_ref()?.as_ref())
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
            .and_then(|msg| msg.refusal.as_ref()?.as_ref())
            .is_some()
    }

    fn refusal(&self) -> Option<&str> {
        self.first_message()
            .and_then(|msg| msg.refusal.as_ref()?.as_ref())
            .map(std::string::String::as_str)
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
                FinishReason::Unknown => "unknown".to_string(),
            })
    }
}

/// Extension trait for tool calls.
pub trait ToolCallExt {
    /// Get the tool call ID.
    ///
    /// For a tool call type this crate does not know, this is the `id` field of the JSON that
    /// came, or an empty string if it has none.
    fn id(&self) -> &str;

    /// Get the function name from the tool call.
    ///
    /// Empty for a call that is not a function call: a custom one, or a type this crate does not
    /// know. Match on the variant to tell these apart.
    fn function_name(&self) -> &str;

    /// Get the function arguments as a string.
    ///
    /// Empty for a call that is not a function call: a custom one, or a type this crate does not
    /// know. Match on the variant to tell these apart.
    fn function_arguments(&self) -> &str;

    /// Parse the function arguments as JSON.
    ///
    /// Fails for a call that is not a function call, whose arguments are empty.
    fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error>;
}

impl ToolCallExt for ChatCompletionMessageToolCallsInner {
    fn id(&self) -> &str {
        match self {
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(tool_call) => {
                &tool_call.id
            }
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageCustomToolCall(tool_call) => {
                &tool_call.id
            }
            ChatCompletionMessageToolCallsInner::Unknown(value) => value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        }
    }

    fn function_name(&self) -> &str {
        match self {
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(tool_call) => {
                &tool_call.function.name
            }
            // Custom tool calls don't have function names in the same way, and nothing says a
            // call of an unknown type has a function: its JSON is in the variant.
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageCustomToolCall(_)
            | ChatCompletionMessageToolCallsInner::Unknown(_) => "",
        }
    }

    fn function_arguments(&self) -> &str {
        match self {
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(tool_call) => {
                &tool_call.function.arguments
            }
            // Custom tool calls don't have function arguments in the same way, and nothing says a
            // call of an unknown type has a function: its JSON is in the variant.
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageCustomToolCall(_)
            | ChatCompletionMessageToolCallsInner::Unknown(_) => "",
        }
    }

    fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(self.function_arguments())
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use serde_json::json;

    fn completion(finish_reason: &str) -> CreateChatCompletionResponse {
        serde_json::from_value(json!({
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-test",
            "choices": [{
                "index": 0,
                "finish_reason": finish_reason,
                "logprobs": null,
                "message": {"role": "assistant", "content": "hi", "refusal": null}
            }]
        }))
        .unwrap()
    }

    #[test]
    fn a_new_finish_reason_is_some_unknown() {
        // The choice finished, so this is not `None`, and it is not passed off as a listed reason.
        let response = completion("brand_new_reason");
        assert_eq!(response.finish_reason().as_deref(), Some("unknown"));
        assert_eq!(response.content(), Some("hi"));

        assert_eq!(completion("stop").finish_reason().as_deref(), Some("stop"));

        let mut empty = completion("stop");
        empty.choices.clear();
        assert_eq!(empty.finish_reason(), None);
    }

    #[test]
    fn a_tool_call_of_a_new_type_keeps_its_id_and_has_no_function() {
        let call: ChatCompletionMessageToolCallsInner = serde_json::from_value(json!({
            "type": "brand_new_tool",
            "id": "call_1",
            "function": {"name": "not_read", "arguments": "{}"}
        }))
        .unwrap();

        assert_eq!(call.id(), "call_1");
        assert_eq!(call.function_name(), "");
        assert_eq!(call.function_arguments(), "");
        assert!(call.parse_arguments::<serde_json::Value>().is_err());
    }

    #[test]
    fn a_tool_call_of_a_new_type_without_an_id_has_an_empty_one() {
        for id in [json!(null), json!(3)] {
            let call: ChatCompletionMessageToolCallsInner =
                serde_json::from_value(json!({"type": "brand_new_tool", "id": id})).unwrap();
            assert_eq!(call.id(), "");
        }
        let call: ChatCompletionMessageToolCallsInner =
            serde_json::from_value(json!({"type": "brand_new_tool"})).unwrap();
        assert_eq!(call.id(), "");
    }
}
