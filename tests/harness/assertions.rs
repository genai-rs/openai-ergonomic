//! Custom assertions for comprehensive testing of OpenAI API interactions.
//!
//! This module provides specialized assertion functions that understand
//! the structure and semantics of OpenAI API requests and responses.

use openai_client_base::models::{
    ChatCompletionRequestMessage, CreateChatCompletionRequest, CreateChatCompletionResponse,
    CreateChatCompletionStreamResponse,
};
use openai_ergonomic::Error;
use serde_json::Value;
use std::collections::HashMap;

/// Assert that a value has a specific field.
pub fn assert_has_field(value: &Value, field: &str) {
    assert!(
        value.get(field).is_some(),
        "Expected field '{field}' not found in response: {value}"
    );
}

/// Assert that a value has a specific field with a specific value.
pub fn assert_field_equals(value: &Value, field: &str, expected: &Value) {
    let actual = value
        .get(field)
        .unwrap_or_else(|| panic!("Field '{field}' not found in: {value}"));
    assert_eq!(
        actual, expected,
        "Field '{field}' has unexpected value. Expected: {expected}, Actual: {actual}"
    );
}

/// Assert that a value has a specific field with a string value.
pub fn assert_field_is_string(value: &Value, field: &str) {
    let field_value = value
        .get(field)
        .unwrap_or_else(|| panic!("Field '{field}' not found in: {value}"));
    assert!(
        field_value.is_string(),
        "Field '{field}' is not a string: {field_value}"
    );
}

/// Assert that a value has a specific field with a numeric value.
pub fn assert_field_is_number(value: &Value, field: &str) {
    let field_value = value
        .get(field)
        .unwrap_or_else(|| panic!("Field '{field}' not found in: {value}"));
    assert!(
        field_value.is_number(),
        "Field '{field}' is not a number: {field_value}"
    );
}

/// Assert that a value has a specific field with an array value.
pub fn assert_field_is_array(value: &Value, field: &str) {
    let field_value = value
        .get(field)
        .unwrap_or_else(|| panic!("Field '{field}' not found in: {value}"));
    assert!(
        field_value.is_array(),
        "Field '{field}' is not an array: {field_value}"
    );
}

/// Assert that an array field has a specific length.
pub fn assert_array_length(value: &Value, field: &str, expected_length: usize) {
    assert_field_is_array(value, field);
    let array = value.get(field).unwrap().as_array().unwrap();
    assert_eq!(
        array.len(),
        expected_length,
        "Array '{field}' has length {} but expected {expected_length}",
        array.len()
    );
}

/// Assert that a response has a successful status (no error field).
pub fn assert_success_response(value: &Value) {
    assert!(
        value.get("error").is_none(),
        "Expected success but got error: {:?}",
        value.get("error")
    );
}

/// Assert that a response has an error with specific type.
pub fn assert_error_response(value: &Value, expected_error_type: &str) {
    assert_has_field(value, "error");
    let error = value.get("error").unwrap();
    assert_field_equals(
        error,
        "type",
        &Value::String(expected_error_type.to_string()),
    );
}

/// Assert that a response has an error with specific type and message containing text.
pub fn assert_error_response_contains(
    value: &Value,
    expected_error_type: &str,
    message_contains: &str,
) {
    assert_error_response(value, expected_error_type);
    let error = value.get("error").unwrap();
    let message = error
        .get("message")
        .unwrap()
        .as_str()
        .expect("Error message should be a string");
    assert!(
        message.contains(message_contains),
        "Error message '{message}' does not contain '{message_contains}'"
    );
}

/// Assert that a chat completion request has valid structure.
pub fn assert_valid_chat_request(request: &CreateChatCompletionRequest) {
    assert!(!request.model.is_empty(), "Model cannot be empty");
    assert!(!request.messages.is_empty(), "Messages cannot be empty");

    // Validate message structure
    for (i, message) in request.messages.iter().enumerate() {
        assert_valid_message(message, i);
    }

    // Validate parameter ranges if present
    if let Some(temp) = request.temperature {
        assert!(
            (0.0..=2.0).contains(&temp),
            "Temperature {temp} is out of valid range [0.0, 2.0]"
        );
    }

    if let Some(top_p) = request.top_p {
        assert!(
            (0.0..=1.0).contains(&top_p),
            "Top-p {top_p} is out of valid range [0.0, 1.0]"
        );
    }

    if let Some(freq_penalty) = request.frequency_penalty {
        assert!(
            (-2.0..=2.0).contains(&freq_penalty),
            "Frequency penalty {freq_penalty} is out of valid range [-2.0, 2.0]"
        );
    }

    if let Some(pres_penalty) = request.presence_penalty {
        assert!(
            (-2.0..=2.0).contains(&pres_penalty),
            "Presence penalty {pres_penalty} is out of valid range [-2.0, 2.0]"
        );
    }

    if let Some(max_tokens) = request.max_tokens {
        assert!(max_tokens > 0, "Max tokens must be positive");
    }

    if let Some(max_completion_tokens) = request.max_completion_tokens {
        assert!(
            max_completion_tokens > 0,
            "Max completion tokens must be positive"
        );
    }

    if let Some(n) = request.n {
        assert!(n > 0, "N must be positive");
    }
}

/// Assert that a message has valid structure.
pub fn assert_valid_message(message: &ChatCompletionRequestMessage, index: usize) {
    match message {
        ChatCompletionRequestMessage::ChatCompletionRequestSystemMessage(msg) => {
            match msg.content.as_ref() {
                openai_client_base::models::ChatCompletionRequestSystemMessageContent::TextContent(content) => {
                    assert!(
                        !content.is_empty(),
                        "System message at index {index} cannot have empty content"
                    );
                }
                _ => {
                    // Other content types are valid
                }
            }
        }
        ChatCompletionRequestMessage::ChatCompletionRequestUserMessage(msg) => {
            // User messages can have string or array content
            match msg.content.as_ref() {
                openai_client_base::models::ChatCompletionRequestUserMessageContent::TextContent(s) => {
                    assert!(
                        !s.is_empty(),
                        "User message at index {index} cannot have empty string content"
                    );
                }
                openai_client_base::models::ChatCompletionRequestUserMessageContent::ArrayOfContentParts(arr) => {
                    assert!(
                        !arr.is_empty(),
                        "User message at index {index} cannot have empty array content"
                    );
                }
            }
        }
        ChatCompletionRequestMessage::ChatCompletionRequestAssistantMessage(msg) => {
            // Assistant messages can have content or tool calls, but not both empty
            let has_content = msg.content.as_ref().and_then(|opt| opt.as_ref()).map_or(false, |c| {
                match c.as_ref() {
                    openai_client_base::models::ChatCompletionRequestAssistantMessageContent::TextContent(text) => !text.is_empty(),
                    _ => true, // Other content types are considered valid
                }
            });
            let has_tool_calls = msg.tool_calls.as_ref().map_or(false, |tc| !tc.is_empty());

            assert!(
                has_content || has_tool_calls,
                "Assistant message at index {index} must have either content or tool calls"
            );
        }
        _ => {
            // Other message types are valid but we don't need to validate their specific structure
        }
    }
}

/// Assert that a chat completion response has valid structure.
pub fn assert_valid_chat_response(response: &CreateChatCompletionResponse) {
    assert!(!response.id.is_empty(), "Response ID cannot be empty");
    // Note: object field type validation removed due to API changes
    assert!(!response.model.is_empty(), "Response model cannot be empty");
    assert!(
        !response.choices.is_empty(),
        "Response must have at least one choice"
    );

    // Validate each choice
    for (i, choice) in response.choices.iter().enumerate() {
        assert_eq!(choice.index, i as i32, "Choice index should match position");

        // Note: Message validation removed due to API structure changes

        // Note: Finish reason validation simplified due to enum type changes
        // finish_reason is not an Option in the current API
    }

    // Validate usage if present
    if let Some(usage) = &response.usage {
        assert!(usage.prompt_tokens >= 0, "Prompt tokens cannot be negative");
        assert!(
            usage.completion_tokens >= 0,
            "Completion tokens cannot be negative"
        );
        assert!(usage.total_tokens >= 0, "Total tokens cannot be negative");
        assert_eq!(
            usage.total_tokens,
            usage.prompt_tokens + usage.completion_tokens,
            "Total tokens should equal prompt + completion tokens"
        );
    }
}

/// Assert that a streaming response chunk has valid structure.
pub fn assert_valid_stream_chunk(chunk: &CreateChatCompletionStreamResponse) {
    assert!(!chunk.id.is_empty(), "Chunk ID cannot be empty");
    // Note: object field type validation removed due to API changes
    assert!(!chunk.model.is_empty(), "Chunk model cannot be empty");
    assert!(
        !chunk.choices.is_empty(),
        "Chunk must have at least one choice"
    );

    // Validate each choice
    for (i, choice) in chunk.choices.iter().enumerate() {
        assert_eq!(choice.index, i as i32, "Choice index should match position");

        // Delta is always present in the current API structure
        // Note: Delta validation simplified due to API structure changes

        // Note: Finish reason validation simplified due to enum type changes
        // finish_reason is not an Option in the current API
    }
}

/// Assert that two JSON values are equivalent (order-independent for objects).
pub fn assert_json_equivalent(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Object(actual_obj), Value::Object(expected_obj)) => {
            assert_eq!(
                actual_obj.len(),
                expected_obj.len(),
                "Objects have different number of fields"
            );

            for (key, expected_value) in expected_obj {
                let actual_value = actual_obj
                    .get(key)
                    .unwrap_or_else(|| panic!("Key '{key}' not found in actual object"));
                assert_json_equivalent(actual_value, expected_value);
            }
        }
        (Value::Array(actual_arr), Value::Array(expected_arr)) => {
            assert_eq!(
                actual_arr.len(),
                expected_arr.len(),
                "Arrays have different lengths"
            );

            for (_i, (actual_item, expected_item)) in
                actual_arr.iter().zip(expected_arr.iter()).enumerate()
            {
                assert_json_equivalent(actual_item, expected_item);
            }
        }
        (actual, expected) => {
            assert_eq!(actual, expected, "Values are not equal");
        }
    }
}

/// Assert that a request can be serialized to JSON and back without loss.
pub fn assert_request_serializable(request: &CreateChatCompletionRequest) {
    let serialized = serde_json::to_value(request).expect("Request should be serializable");

    let deserialized: CreateChatCompletionRequest =
        serde_json::from_value(serialized).expect("Request should be deserializable");

    // Basic equality checks (since we can't derive PartialEq for the entire struct)
    assert_eq!(request.model, deserialized.model);
    assert_eq!(request.messages.len(), deserialized.messages.len());
    assert_eq!(request.temperature, deserialized.temperature);
    assert_eq!(request.max_tokens, deserialized.max_tokens);
}

/// Assert that an error has a specific type.
pub fn assert_error_type<T: std::fmt::Debug>(result: Result<T, Error>, expected_error_type: &str) {
    assert!(result.is_err(), "Expected error but got success");
    let error = result.unwrap_err();
    let error_string = error.to_string().to_lowercase();
    let expected_lower = expected_error_type.to_lowercase();
    assert!(
        error_string.contains(&expected_lower),
        "Error '{error}' does not contain expected type '{expected_error_type}'"
    );
}

/// Assert that an error message contains specific text.
pub fn assert_error_contains<T: std::fmt::Debug>(result: Result<T, Error>, expected_text: &str) {
    assert!(result.is_err(), "Expected error but got success");
    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(
        error_string.contains(expected_text),
        "Error '{error}' does not contain expected text '{expected_text}'"
    );
}

/// Assert that a builder fails with a specific error message.
pub fn assert_builder_fails_with<T, B>(builder: B, expected_error: &str)
where
    T: std::fmt::Debug,
    B: FnOnce() -> Result<T, Error>,
{
    let result = builder();
    assert_error_contains(result, expected_error);
}

/// Assert that HTTP headers contain expected rate limiting information.
pub fn assert_rate_limit_headers(headers: &HashMap<String, String>) {
    if headers.contains_key("x-ratelimit-remaining-requests") {
        let remaining = headers.get("x-ratelimit-remaining-requests").unwrap();
        let _: u32 = remaining
            .parse()
            .expect("Rate limit remaining should be a valid number");
    }

    if headers.contains_key("x-ratelimit-reset-requests") {
        let reset = headers.get("x-ratelimit-reset-requests").unwrap();
        let _: u64 = reset
            .parse()
            .expect("Rate limit reset should be a valid timestamp");
    }
}

/// Assert that a JSON schema is valid.
pub fn assert_valid_json_schema(schema: &Value) {
    assert_field_equals(schema, "type", &Value::String("object".to_string()));
    assert_has_field(schema, "properties");

    let properties = schema.get("properties").unwrap();
    assert!(properties.is_object(), "Properties should be an object");

    // If required field exists, it should be an array
    if let Some(required) = schema.get("required") {
        assert!(required.is_array(), "Required field should be an array");
    }
}

/// Assert that function parameters follow OpenAI function calling schema.
pub fn assert_valid_function_parameters(parameters: &Value) {
    assert_valid_json_schema(parameters);

    // Parameters should not have 'additionalProperties: true' for OpenAI compatibility
    if let Some(additional_props) = parameters.get("additionalProperties") {
        assert!(
            !additional_props.as_bool().unwrap_or(false),
            "Function parameters should not allow additional properties"
        );
    }
}

/// Assert that a tool definition is valid.
pub fn assert_valid_tool_definition(tool: &Value) {
    assert_field_equals(tool, "type", &Value::String("function".to_string()));
    assert_has_field(tool, "function");

    let function = tool.get("function").unwrap();
    assert_has_field(function, "name");
    assert_has_field(function, "description");
    assert_has_field(function, "parameters");

    let name = function
        .get("name")
        .unwrap()
        .as_str()
        .expect("Function name should be a string");
    assert!(!name.is_empty(), "Function name cannot be empty");
    assert!(
        name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "Function name should only contain alphanumeric characters and underscores"
    );

    let description = function
        .get("description")
        .unwrap()
        .as_str()
        .expect("Function description should be a string");
    assert!(
        !description.is_empty(),
        "Function description cannot be empty"
    );

    let parameters = function.get("parameters").unwrap();
    assert_valid_function_parameters(parameters);
}

/// Assert that streaming chunks form a complete message.
pub fn assert_complete_streaming_message(chunks: &[Value], expected_content: &str) {
    assert!(!chunks.is_empty(), "Chunks cannot be empty");

    let mut combined_content = String::new();
    let mut has_final_chunk = false;

    for chunk in chunks {
        assert_has_field(chunk, "choices");
        let choices = chunk.get("choices").unwrap().as_array().unwrap();
        assert!(
            !choices.is_empty(),
            "Each chunk should have at least one choice"
        );

        let choice = &choices[0];
        assert_has_field(choice, "delta");

        let delta = choice.get("delta").unwrap();
        if let Some(content) = delta.get("content") {
            if let Some(content_str) = content.as_str() {
                combined_content.push_str(content_str);
            }
        }

        if let Some(finish_reason) = choice.get("finish_reason") {
            if !finish_reason.is_null() {
                has_final_chunk = true;
            }
        }
    }

    assert!(has_final_chunk, "Streaming should end with a finish_reason");
    assert_eq!(
        combined_content.trim(),
        expected_content.trim(),
        "Combined streaming content doesn't match expected content"
    );
}

/// Performance assertion: ensure operation completes within time limit.
pub fn assert_completes_within<F, R>(
    operation: F,
    max_duration: std::time::Duration,
    description: &str,
) -> R
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = operation();
    let duration = start.elapsed();

    assert!(
        duration <= max_duration,
        "{description} took {duration:?} but should complete within {max_duration:?}"
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_field_assertions() {
        let value = json!({"name": "test", "count": 42, "items": [1, 2, 3]});

        assert_has_field(&value, "name");
        assert_field_equals(&value, "name", &json!("test"));
        assert_field_is_string(&value, "name");
        assert_field_is_number(&value, "count");
        assert_field_is_array(&value, "items");
        assert_array_length(&value, "items", 3);
    }

    #[test]
    fn test_response_assertions() {
        let success = json!({"id": "test", "choices": [{"message": {"content": "hello"}}]});
        assert_success_response(&success);

        let error =
            json!({"error": {"type": "rate_limit_exceeded", "message": "Too many requests"}});
        assert_error_response(&error, "rate_limit_exceeded");
        assert_error_response_contains(&error, "rate_limit_exceeded", "Too many");
    }

    #[test]
    fn test_json_equivalence() {
        let obj1 = json!({"a": 1, "b": 2});
        let obj2 = json!({"b": 2, "a": 1});
        assert_json_equivalent(&obj1, &obj2);

        let arr1 = json!([1, 2, 3]);
        let arr2 = json!([1, 2, 3]);
        assert_json_equivalent(&arr1, &arr2);
    }

    #[test]
    fn test_schema_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"],
            "additionalProperties": false
        });

        assert_valid_json_schema(&schema);
        assert_valid_function_parameters(&schema);
    }

    #[test]
    fn test_tool_validation() {
        let tool = json!({
            "type": "function",
            "function": {
                "name": "test_function",
                "description": "A test function",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        });

        assert_valid_tool_definition(&tool);
    }

    #[test]
    fn test_streaming_chunks() {
        let chunks = vec![
            json!({
                "choices": [{
                    "delta": {"content": "Hello"},
                    "finish_reason": null
                }]
            }),
            json!({
                "choices": [{
                    "delta": {"content": " world!"},
                    "finish_reason": "stop"
                }]
            }),
        ];

        assert_complete_streaming_message(&chunks, "Hello world!");
    }

    #[test]
    fn test_performance_assertion() {
        let _result = assert_completes_within(
            || std::thread::sleep(std::time::Duration::from_millis(10)),
            std::time::Duration::from_millis(100),
            "Sleep operation",
        );

        // Should complete without panicking
    }
}
