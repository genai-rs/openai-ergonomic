//! A response of the Responses API, with ergonomic accessors.

use openai_client_base::models::{
    self, response::Status, FunctionToolCall, InputItem, OutputItem, OutputMessageContent,
    ResponseError, ResponseUsage,
};
use serde::{Deserialize, Serialize};

use crate::tool_framework::ToolInvocation;

/// A model response (`POST /responses`, `GET /responses/{id}`), wrapping the generated
/// [`models::Response`]. It serializes and deserializes as the generated response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResponsesResponse {
    inner: models::Response,
}

impl ResponsesResponse {
    /// Wrap a generated response.
    #[must_use]
    pub fn new(inner: models::Response) -> Self {
        Self { inner }
    }

    /// The response ID, to continue the conversation with
    /// [`previous_response_id`](super::ResponsesBuilder::previous_response_id) or to retrieve it.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.inner.id
    }

    /// The model that generated the response.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// The status: `Completed`, `Failed`, `InProgress`, `Cancelled`, `Queued` or `Incomplete`.
    #[must_use]
    pub fn status(&self) -> Option<Status> {
        self.inner.status
    }

    /// The error of a failed response.
    #[must_use]
    pub fn error(&self) -> Option<&ResponseError> {
        self.inner.error.as_ref()
    }

    /// Why an incomplete response stopped, such as `"max_output_tokens"`.
    #[must_use]
    pub fn incomplete_reason(&self) -> Option<String> {
        self.inner
            .incomplete_details
            .as_ref()
            .and_then(|details| details.reason)
            .map(|reason| reason.to_string())
    }

    /// The output items: messages, function calls, reasoning, built-in tool calls.
    #[must_use]
    pub fn output(&self) -> &[OutputItem] {
        &self.inner.output
    }

    /// The text of the output messages, concatenated (refusals excluded).
    #[must_use]
    pub fn output_text(&self) -> String {
        let mut text = String::new();
        for item in &self.inner.output {
            if let OutputItem::OutputMessage(message) = item {
                for part in &message.content {
                    if let OutputMessageContent::OutputTextContent(part) = part {
                        text.push_str(&part.text);
                    }
                }
            }
        }
        text
    }

    /// The refusal of the model, if it refused.
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.inner.output.iter().find_map(|item| match item {
            OutputItem::OutputMessage(message) => {
                message.content.iter().find_map(|part| match part {
                    OutputMessageContent::RefusalContent(refusal) => Some(refusal.refusal.as_str()),
                    _ => None,
                })
            }
            _ => None,
        })
    }

    /// The function calls the model made, in output order. Answer each with
    /// [`function_call_output`](super::ResponsesBuilder::function_call_output) and its
    /// `call_id`.
    #[must_use]
    pub fn function_calls(&self) -> Vec<&FunctionToolCall> {
        self.inner
            .output
            .iter()
            .filter_map(|item| match item {
                OutputItem::FunctionToolCall(call) => Some(call),
                _ => None,
            })
            .collect()
    }

    /// The tool calls the model made (function and custom tool calls), in output order, ready
    /// for [`ToolRegistry::answer_call`](crate::ToolRegistry::answer_call).
    #[must_use]
    pub fn tool_invocations(&self) -> Vec<ToolInvocation> {
        self.inner
            .output
            .iter()
            .filter_map(ToolInvocation::from_output_item)
            .collect()
    }

    /// The error of a failed response ([`Error::ResponseFailed`](crate::Error::ResponseFailed)),
    /// whose [`retry_class`](crate::Error::retry_class) says whether to try again. `None`
    /// unless the status is `Failed`.
    #[must_use]
    pub fn failure(&self) -> Option<crate::Error> {
        (self.inner.status == Some(Status::Failed))
            .then(|| super::stream::response_failure(&self.inner))
    }

    /// The output items as input items, to send them back in the next request when the
    /// conversation is not stored (`store(false)`): the function calls, reasoning and
    /// messages of this response, before the function call outputs that answer it.
    pub fn output_as_input(&self) -> crate::Result<Vec<InputItem>> {
        self.inner
            .output
            .iter()
            .map(|item| Ok(serde_json::from_value(serde_json::to_value(item)?)?))
            .collect()
    }

    /// The token usage.
    #[must_use]
    pub fn usage(&self) -> Option<&ResponseUsage> {
        self.inner.usage.as_ref().and_then(Option::as_ref)
    }

    /// The generated response.
    #[must_use]
    pub fn inner(&self) -> &models::Response {
        &self.inner
    }

    /// The generated response, by value.
    #[must_use]
    pub fn into_inner(self) -> models::Response {
        self.inner
    }
}

impl From<models::Response> for ResponsesResponse {
    fn from(inner: models::Response) -> Self {
        Self::new(inner)
    }
}
