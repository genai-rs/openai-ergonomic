//! Demonstrates strongly typed inputs and outputs with the tool framework.

use openai_client_base::models::{
    chat_completion_message_tool_call::Type as ToolCallType,
    chat_completion_response_message::Role,
    create_chat_completion_response_choices_inner::FinishReason, ChatCompletionMessageToolCall,
    ChatCompletionMessageToolCallFunction, ChatCompletionMessageToolCallsInner,
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, CreateChatCompletionResponse,
    CreateChatCompletionResponseChoicesInner,
};
use openai_ergonomic::{
    builders::chat::ChatCompletionBuilder, builders::Builder,
    responses::ChatCompletionResponseWrapper, tool, tool_framework::ToolRegistry, tool_schema_from,
    Result,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
/// Input payload for the typed addition tool.
pub struct AddParams {
    lhs: i64,
    rhs: i64,
}

#[derive(Debug, Serialize)]
/// Typed output returned by the addition tool.
pub struct AddResult {
    sum: i64,
}

tool! {
    /// Simple strongly-typed addition tool used in the example.
    pub struct AddTool;

    name: "add_numbers";
    description: "Add two integers";
    input_type: AddParams;
    output_type: AddResult;
    schema: tool_schema_from!(AddParams);

    async fn handle(params: AddParams) -> Result<AddResult> {
        Ok(AddResult {
            sum: params.lhs + params.rhs,
        })
    }
}

fn sample_response() -> ChatCompletionResponseWrapper {
    let tool_call = ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(Box::new(
        ChatCompletionMessageToolCall {
            id: "call_1".into(),
            r#type: ToolCallType::Function,
            function: Box::new(ChatCompletionMessageToolCallFunction {
                name: "add_numbers".into(),
                arguments: r#"{"lhs":2,"rhs":3}"#.into(),
            }),
        },
    ));

    let message = ChatCompletionResponseMessage {
        content: None,
        refusal: None,
        tool_calls: Some(vec![tool_call]),
        annotations: None,
        role: Role::Assistant,
        function_call: None,
        audio: None,
    };

    let choice = CreateChatCompletionResponseChoicesInner {
        finish_reason: FinishReason::ToolCalls,
        index: 0,
        message: Box::new(message),
        logprobs: None,
    };

    let response = CreateChatCompletionResponse {
        id: "resp_001".into(),
        choices: vec![choice],
        created: 0,
        model: "gpt-test".into(),
        service_tier: None,
        system_fingerprint: None,
        object: openai_client_base::models::create_chat_completion_response::Object::ChatCompletion,
        usage: None,
    };

    ChatCompletionResponseWrapper::new(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    let registry = ToolRegistry::new().register(AddTool);

    let response = sample_response();
    let builder = ChatCompletionBuilder::new("gpt-test");
    let builder = registry
        .process_tool_calls_into_builder(&response, builder)
        .await?;
    let request = builder.build()?;

    for message in request.messages {
        if let ChatCompletionRequestMessage::ChatCompletionRequestToolMessage(msg) = message {
            let call_id = &msg.tool_call_id;
            let content = &msg.content;
            println!("Tool call {call_id} returned {content}");
        }
    }

    Ok(())
}
