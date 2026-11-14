//! The request builder of the Responses API.

use openai_client_base::models::{
    easy_input_message, function_call_output_item_param, function_tool, CreateResponse,
    EasyInputMessage, EasyInputMessageContent, FunctionCallOutputItemParam,
    FunctionCallOutputItemParamOutput, FunctionTool, IncludeEnum, InputItem, InputParam, Item,
    Reasoning, ReasoningEffort, ResponseTextParam, TextResponseFormatConfiguration,
    TextResponseFormatJsonSchema, Tool, ToolChoiceParam,
};
use serde_json::Value;
use std::collections::HashMap;

/// Builder for Responses API requests (`POST /responses`).
///
/// The builder is neutral: it sends only what you set, and the API's defaults apply to the rest
/// (for example, responses are stored unless [`store(false)`](Self::store) is set).
///
/// ```rust
/// use openai_ergonomic::builders::Builder;
/// use openai_ergonomic::ResponsesBuilder;
///
/// let request = ResponsesBuilder::new("gpt-4.1-mini")
///     .instructions("Answer in one sentence.")
///     .user("What is Rust?")
///     .max_output_tokens(100)
///     .build()?;
/// assert_eq!(request.model.as_deref(), Some("gpt-4.1-mini"));
/// # Ok::<(), openai_ergonomic::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct ResponsesBuilder {
    params: CreateResponse,
    reasoning_effort: Option<String>,
}

impl ResponsesBuilder {
    /// A builder for a response of `model`.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        let mut params = CreateResponse::new();
        params.model = Some(model.into());
        Self {
            params,
            reasoning_effort: None,
        }
    }

    /// Set the input to `text`, replacing any input set before.
    #[must_use]
    pub fn input_text(mut self, text: impl Into<String>) -> Self {
        self.params.input = Some(InputParam::TextInput(text.into()));
        self
    }

    /// Append an input item. Text input set with [`input_text`](Self::input_text) becomes a
    /// user message first.
    #[must_use]
    pub fn input_item(mut self, item: InputItem) -> Self {
        self.items().push(item);
        self
    }

    /// Append input items, such as the output items of an earlier response
    /// ([`ResponsesResponse::output_as_input`](super::ResponsesResponse::output_as_input)).
    #[must_use]
    pub fn input_items(mut self, items: impl IntoIterator<Item = InputItem>) -> Self {
        self.items().extend(items);
        self
    }

    /// Append a user message.
    #[must_use]
    pub fn user(self, content: impl Into<String>) -> Self {
        self.message(easy_input_message::Role::User, content.into())
    }

    /// Append a system message.
    #[must_use]
    pub fn system(self, content: impl Into<String>) -> Self {
        self.message(easy_input_message::Role::System, content.into())
    }

    /// Append a developer message.
    #[must_use]
    pub fn developer(self, content: impl Into<String>) -> Self {
        self.message(easy_input_message::Role::Developer, content.into())
    }

    /// Append an assistant message (an earlier turn of the model).
    #[must_use]
    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.message(easy_input_message::Role::Assistant, content.into())
    }

    /// Append the output of a function call: `output` answers the call `call_id`.
    #[must_use]
    pub fn function_call_output(
        self,
        call_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        let item = FunctionCallOutputItemParam {
            id: None,
            call_id: Some(Some(call_id.into())),
            r#type: function_call_output_item_param::Type::FunctionCallOutput,
            output: FunctionCallOutputItemParamOutput::Text(output.into()),
            name: None,
            namespace: None,
            caller: None,
            status: None,
        };
        self.input_item(InputItem::Item(Item::FunctionCallOutputItemParam(item)))
    }

    fn message(self, role: easy_input_message::Role, content: String) -> Self {
        let message = EasyInputMessage {
            role,
            content: EasyInputMessageContent::TextInput(content),
            phase: None,
            r#type: Some(easy_input_message::Type::Message),
        };
        self.input_item(InputItem::EasyInputMessage(message))
    }

    /// The input as a list of items, turning text input into a user message.
    fn items(&mut self) -> &mut Vec<InputItem> {
        let items = match self.params.input.take() {
            None => Vec::new(),
            Some(InputParam::InputItemList(items)) => items,
            Some(InputParam::TextInput(text)) => {
                vec![InputItem::EasyInputMessage(EasyInputMessage {
                    role: easy_input_message::Role::User,
                    content: EasyInputMessageContent::TextInput(text),
                    phase: None,
                    r#type: Some(easy_input_message::Type::Message),
                })]
            }
        };
        match self.params.input.insert(InputParam::InputItemList(items)) {
            InputParam::InputItemList(items) => items,
            InputParam::TextInput(_) => unreachable!("the input was just set to a list"),
        }
    }

    /// Set the instructions: a system (or developer) message for this response.
    #[must_use]
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.params.instructions = Some(Some(instructions.into()));
        self
    }

    /// Continue the conversation of an earlier, stored response.
    #[must_use]
    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.params.previous_response_id = Some(Some(id.into()));
        self
    }

    /// Whether the API stores the response (it does unless `false`).
    #[must_use]
    pub fn store(mut self, store: bool) -> Self {
        self.params.store = Some(Some(store));
        self
    }

    /// Additional output data to include, such as `IncludeEnum::ReasoningEncryptedContent`.
    #[must_use]
    pub fn include(mut self, include: Vec<IncludeEnum>) -> Self {
        self.params.include = Some(Some(include));
        self
    }

    /// A key that groups similar requests for prompt caching.
    #[must_use]
    pub fn prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.params.prompt_cache_key = Some(Some(key.into()));
        self
    }

    /// Whether the model may call tools in parallel.
    #[must_use]
    pub fn parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.params.parallel_tool_calls = Some(Some(parallel));
        self
    }

    /// The maximum number of built-in tool calls in the response.
    #[must_use]
    pub fn max_tool_calls(mut self, max_tool_calls: i64) -> Self {
        self.params.max_tool_calls = Some(Some(max_tool_calls));
        self
    }

    /// The maximum number of output tokens, reasoning tokens included.
    #[must_use]
    pub fn max_output_tokens(mut self, max_output_tokens: i64) -> Self {
        self.params.max_output_tokens = Some(Some(max_output_tokens));
        self
    }

    /// The sampling temperature, between 0 and 2.
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.params.temperature = Some(Some(temperature));
        self
    }

    /// The nucleus sampling probability mass, between 0 and 1.
    #[must_use]
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.params.top_p = Some(Some(top_p));
        self
    }

    /// A stable identifier of the end user, for abuse detection.
    #[must_use]
    pub fn safety_identifier(mut self, id: impl Into<String>) -> Self {
        self.params.safety_identifier = Some(Some(id.into()));
        self
    }

    /// Add a metadata key-value pair.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params
            .metadata
            .get_or_insert_with(|| Some(HashMap::new()))
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Add a tool the model may call.
    #[must_use]
    pub fn tool(mut self, tool: Tool) -> Self {
        self.params.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Add tools the model may call.
    #[must_use]
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.params.tools.get_or_insert_with(Vec::new).extend(tools);
        self
    }

    /// Add a function tool: `parameters` is its JSON Schema.
    #[must_use]
    pub fn function_tool(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        self.tool(responses_function_tool(name, description, parameters))
    }

    /// Set how the model chooses tools.
    #[must_use]
    pub fn tool_choice(mut self, tool_choice: ToolChoiceParam) -> Self {
        self.params.tool_choice = Some(tool_choice);
        self
    }

    /// Set the reasoning effort of a reasoning model: `none`, `minimal`, `low`, `medium`,
    /// `high`, `xhigh` or `max`. An unknown value fails at [`build`](crate::Builder::build).
    #[must_use]
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    /// Set the reasoning options of a reasoning model.
    #[must_use]
    pub fn reasoning(mut self, reasoning: Reasoning) -> Self {
        self.params.reasoning = Some(Some(reasoning));
        self
    }

    /// Enable JSON mode: the model answers with a JSON object (mention JSON in the
    /// instructions or input).
    #[must_use]
    pub fn json_mode(mut self) -> Self {
        use openai_client_base::models::{response_format_json_object, ResponseFormatJsonObject};

        self.params
            .text
            .get_or_insert_with(ResponseTextParam::new)
            .format = Some(TextResponseFormatConfiguration::ResponseFormatJsonObject(
            ResponseFormatJsonObject::new(response_format_json_object::Type::JsonObject),
        ));
        self
    }

    /// Ask for JSON output that matches `schema` (structured outputs, strict).
    #[must_use]
    pub fn json_schema(mut self, name: impl Into<String>, schema: Value) -> Self {
        let schema = match schema {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };
        let format = TextResponseFormatJsonSchema {
            r#type: openai_client_base::models::text_response_format_json_schema::Type::JsonSchema,
            description: None,
            name: name.into(),
            schema,
            strict: Some(Some(true)),
        };
        self.params
            .text
            .get_or_insert_with(ResponseTextParam::new)
            .format = Some(TextResponseFormatConfiguration::TextResponseFormatJsonSchema(format));
        self
    }

    /// The request built so far, to change fields the builder has no method for (or change
    /// the `CreateResponse` that [`build`](crate::Builder::build) returns).
    #[must_use]
    pub fn params_mut(&mut self) -> &mut CreateResponse {
        &mut self.params
    }
}

impl crate::builders::Builder<CreateResponse> for ResponsesBuilder {
    fn build(self) -> crate::Result<CreateResponse> {
        let mut params = self.params;
        if params
            .model
            .as_deref()
            .is_none_or(|model| model.trim().is_empty())
        {
            return Err(crate::Error::InvalidRequest(
                "Model cannot be empty".to_string(),
            ));
        }
        let has_input = match &params.input {
            None => false,
            Some(InputParam::TextInput(text)) => !text.is_empty(),
            Some(InputParam::InputItemList(items)) => !items.is_empty(),
        };
        if !has_input && params.previous_response_id.is_none() && params.prompt.is_none() {
            return Err(crate::Error::InvalidRequest(
                "A response needs input, a previous response or a prompt".to_string(),
            ));
        }
        if let Some(Some(temperature)) = params.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                return Err(crate::Error::InvalidRequest(format!(
                    "temperature must be between 0.0 and 2.0, got {temperature}"
                )));
            }
        }
        if let Some(Some(top_p)) = params.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(crate::Error::InvalidRequest(format!(
                    "top_p must be between 0.0 and 1.0, got {top_p}"
                )));
            }
        }
        if let Some(effort) = self.reasoning_effort {
            let effort = effort
                .parse::<ReasoningEffort>()
                .map_err(|error| crate::Error::InvalidRequest(error.to_string()))?;
            params
                .reasoning
                .get_or_insert_with(|| Some(Reasoning::new()))
                .get_or_insert_with(Reasoning::new)
                .effort = Some(Some(effort));
        }
        Ok(params)
    }
}

/// A function tool for the Responses API: `parameters` is its JSON Schema.
#[must_use]
pub fn responses_function_tool(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: Value,
) -> Tool {
    let parameters = match parameters {
        Value::Object(map) => Some(map.into_iter().collect()),
        _ => None,
    };
    Tool::FunctionTool(FunctionTool {
        r#type: function_tool::Type::Function,
        name: name.into(),
        r#async: None,
        description: Some(Some(description.into())),
        parameters,
        output_schema: None,
        strict: None,
        defer_loading: None,
        allowed_callers: None,
    })
}

/// A Responses API request with a user message.
#[must_use]
pub fn responses_simple(model: impl Into<String>, input: impl Into<String>) -> ResponsesBuilder {
    ResponsesBuilder::new(model).user(input)
}

/// A Responses API request with instructions and a user message.
#[must_use]
pub fn responses_system_user(
    model: impl Into<String>,
    instructions: impl Into<String>,
    input: impl Into<String>,
) -> ResponsesBuilder {
    ResponsesBuilder::new(model)
        .instructions(instructions)
        .user(input)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::builders::Builder;
    use serde_json::json;

    #[test]
    fn serializes_text_input_and_only_what_was_set() {
        let request = ResponsesBuilder::new("gpt-4.1-mini")
            .input_text("Hello")
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({"model": "gpt-4.1-mini", "input": "Hello"})
        );
    }

    #[test]
    fn serializes_items_and_parameters() {
        let request = ResponsesBuilder::new("gpt-4.1")
            .instructions("Be brief")
            .user("What is the weather in Ghent?")
            .function_call_output("call_1", r#"{"temp":12}"#)
            .function_tool(
                "get_weather",
                "Weather of a city",
                json!({"type": "object", "properties": {"city": {"type": "string"}}}),
            )
            .store(false)
            .include(vec![IncludeEnum::ReasoningEncryptedContent])
            .prompt_cache_key("conversation-1")
            .parallel_tool_calls(true)
            .max_tool_calls(4)
            .max_output_tokens(256)
            .previous_response_id("resp_0")
            .reasoning_effort("low")
            .metadata("team", "docs")
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({
                "model": "gpt-4.1",
                "instructions": "Be brief",
                "input": [
                    {"type": "message", "role": "user", "content": "What is the weather in Ghent?"},
                    {"type": "function_call_output", "call_id": "call_1", "output": "{\"temp\":12}"}
                ],
                "tools": [{
                    "type": "function",
                    "name": "get_weather",
                    "description": "Weather of a city",
                    "parameters": {"type": "object", "properties": {"city": {"type": "string"}}},
                    "strict": null
                }],
                "store": false,
                "include": ["reasoning.encrypted_content"],
                "prompt_cache_key": "conversation-1",
                "parallel_tool_calls": true,
                "max_tool_calls": 4,
                "max_output_tokens": 256,
                "previous_response_id": "resp_0",
                "reasoning": {"effort": "low"},
                "metadata": {"team": "docs"}
            })
        );
    }

    #[test]
    fn text_input_becomes_a_user_message_when_items_follow() {
        let request = ResponsesBuilder::new("gpt-4.1")
            .input_text("first")
            .assistant("second")
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap()["input"],
            json!([
                {"type": "message", "role": "user", "content": "first"},
                {"type": "message", "role": "assistant", "content": "second"}
            ])
        );
    }

    #[test]
    fn strict_is_sent_as_null_when_unset_and_as_set_otherwise() {
        // The spec marks a function tool's `strict` required and nullable.
        let request = responses_simple("gpt-4.1", "Hi")
            .function_tool("f", "A function", json!({"type": "object"}))
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap()["tools"][0]["strict"],
            Value::Null
        );

        let mut tool = responses_function_tool("f", "A function", json!({"type": "object"}));
        if let Tool::FunctionTool(function) = &mut tool {
            function.strict = Some(false);
        }
        let request = responses_simple("gpt-4.1", "Hi")
            .tool(tool)
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap()["tools"][0]["strict"],
            json!(false)
        );
    }

    #[test]
    fn json_schema_sets_the_text_format() {
        let request = responses_simple("gpt-4.1", "Name a city")
            .json_schema("city", json!({"type": "object"}))
            .build()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap()["text"],
            json!({"format": {"type": "json_schema", "name": "city", "schema": {"type": "object"}, "strict": true}})
        );
    }

    #[test]
    fn requests_round_trip() {
        let request = responses_system_user("gpt-4.1", "Be brief", "Hi")
            .build()
            .unwrap();
        let json = serde_json::to_string(&request).unwrap();
        let decoded: CreateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn rejects_invalid_requests() {
        assert!(ResponsesBuilder::new("").user("x").build().is_err());
        assert!(ResponsesBuilder::new("gpt-4.1").build().is_err());
        assert!(ResponsesBuilder::new("gpt-4.1")
            .user("x")
            .temperature(3.0)
            .build()
            .is_err());
        assert!(ResponsesBuilder::new("gpt-4.1")
            .user("x")
            .reasoning_effort("extreme")
            .build()
            .is_err());
        assert!(ResponsesBuilder::new("gpt-4.1")
            .previous_response_id("resp_1")
            .build()
            .is_ok());
    }
}
