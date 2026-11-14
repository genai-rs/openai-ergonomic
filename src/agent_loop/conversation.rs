//! The history of a [`ToolLoop`](super::ToolLoop): Responses API input items, repaired before
//! every request.

use std::collections::BTreeSet;

use openai_client_base::models::{ChatCompletionRequestMessage, InputItem, OutputItem};
use serde_json::{json, Map, Value};

use super::config::ModelCaps;
use crate::tool_framework::{CallAnswer, ToolInvocation, ToolKind, ABORTED_OUTPUT};

/// The text that replaces an image when the model takes none ([`ModelCaps::images`]).
pub const IMAGE_OMITTED: &str = "[image omitted: the model does not accept images]";

/// A conversation: the input items of the Responses API (messages, the model's output items,
/// tool calls and their outputs), in order.
///
/// A [`ToolLoop`](super::ToolLoop) appends the model's output items and the answers to its tool
/// calls, and [repairs](Self::normalize) the history before every request. The same history
/// drives the Chat Completions API: messages, calls and outputs become chat messages; reasoning
/// and built-in tool items are left out there.
///
/// ```
/// use openai_ergonomic::{CallAnswer, Conversation, ModelCaps, ToolInvocation};
///
/// let mut conversation = Conversation::new();
/// conversation.push_user("What is the weather in Ghent?");
/// let call = ToolInvocation::function("call_1", "get_weather", r#"{"city":"Ghent"}"#);
/// conversation.push_call(&call);
/// // The call was never answered (say, the application stopped): repair the history.
/// let report = conversation.normalize(ModelCaps::default());
/// assert_eq!(report.aborted, ["call_1"]);
/// assert!(conversation.unanswered_calls().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conversation {
    items: Vec<InputItem>,
}

/// What [`Conversation::normalize`] changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NormalizeReport {
    /// The calls that had no output and were answered with `"aborted"`, by call id.
    pub aborted: Vec<String>,
    /// The outputs dropped because no earlier call has their call id, by call id.
    pub orphans: Vec<String>,
    /// The images replaced with a text because the model takes none.
    pub images: usize,
}

impl NormalizeReport {
    /// Whether nothing changed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.aborted.is_empty() && self.orphans.is_empty() && self.images == 0
    }
}

/// How an item relates to tool calls.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Link {
    Call(String),
    Output(String),
    None,
}

impl Conversation {
    /// An empty conversation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A conversation that starts with a user message.
    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        let mut conversation = Self::new();
        conversation.push_user(text);
        conversation
    }

    /// A conversation of these items.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = InputItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// The items, in order.
    #[must_use]
    pub fn items(&self) -> &[InputItem] {
        &self.items
    }

    /// The items, by value.
    #[must_use]
    pub fn into_items(self) -> Vec<InputItem> {
        self.items
    }

    /// The number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there is no item.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Append an item.
    pub fn push(&mut self, item: impl Into<InputItem>) {
        self.items.push(item.into());
    }

    /// Append a user message.
    pub fn push_user(&mut self, text: impl Into<String>) {
        self.push_message("user", &text.into());
    }

    /// Append a developer message (instructions within the conversation).
    pub fn push_developer(&mut self, text: impl Into<String>) {
        self.push_message("developer", &text.into());
    }

    /// Append a system message.
    pub fn push_system(&mut self, text: impl Into<String>) {
        self.push_message("system", &text.into());
    }

    /// Append an assistant message (an earlier answer of the model).
    pub fn push_assistant(&mut self, text: impl Into<String>) {
        self.push_message("assistant", &text.into());
    }

    fn push_message(&mut self, role: &str, text: &str) {
        self.items.push(to_item(
            json!({"type": "message", "role": role, "content": text}),
        ));
    }

    /// Append a tool call of the model: a `function_call` or a `custom_tool_call` item.
    pub fn push_call(&mut self, call: &ToolInvocation) {
        let mut value = match call.kind {
            ToolKind::Custom => json!({
                "type": "custom_tool_call",
                "call_id": call.call_id,
                "name": call.name,
                "input": call.input,
            }),
            _ => json!({
                "type": "function_call",
                "call_id": call.call_id,
                "name": call.name,
                "arguments": call.input,
            }),
        };
        if let Some(namespace) = &call.namespace {
            value["namespace"] = Value::String(namespace.clone());
        }
        self.items.push(to_item(value));
    }

    /// Append the answer to a call: a `function_call_output` or a `custom_tool_call_output`.
    pub fn push_answer(&mut self, answer: &CallAnswer) {
        self.items.push(answer.to_input_item());
    }

    /// Append the output items of a response (messages, reasoning, tool calls), as input items.
    pub fn record_output(&mut self, output: &[OutputItem]) {
        for item in output {
            self.record_output_item(item);
        }
    }

    /// Append one output item of a response.
    pub fn record_output_item(&mut self, item: &OutputItem) {
        let value = serde_json::to_value(item).unwrap_or(Value::Null);
        if value.is_object() {
            self.items.push(to_item(value));
        }
    }

    /// The calls that no later item answers, in order.
    #[must_use]
    pub fn unanswered_calls(&self) -> Vec<ToolInvocation> {
        let answered: BTreeSet<String> = self
            .items
            .iter()
            .filter_map(|item| match link(item) {
                Link::Output(id) => Some(id),
                _ => None,
            })
            .collect();
        self.items
            .iter()
            .filter(|item| matches!(link(item), Link::Call(id) if !answered.contains(&id)))
            .filter_map(invocation)
            .collect()
    }

    /// The tool calls of the conversation, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<ToolInvocation> {
        self.items.iter().filter_map(invocation).collect()
    }

    /// The text of the last assistant message, if any.
    #[must_use]
    pub fn last_assistant_text(&self) -> Option<String> {
        self.items.iter().rev().find_map(|item| {
            let value = view(item);
            (value.get("role").and_then(Value::as_str) == Some("assistant"))
                .then(|| content_text(&value["content"]))
        })
    }

    /// Remove the item at `index`, and its partner: the outputs of a call, or the call of an
    /// output (and that call's other outputs). Returns the removed items in order; nothing if
    /// `index` is out of range.
    pub fn remove(&mut self, index: usize) -> Vec<InputItem> {
        let Some(item) = self.items.get(index) else {
            return Vec::new();
        };
        let id = match link(item) {
            Link::Call(id) | Link::Output(id) => Some(id),
            Link::None => None,
        };
        let mut removed = Vec::new();
        let mut position = 0;
        self.items.retain(|item| {
            let here = position;
            position += 1;
            let partner = id.as_ref().is_some_and(|id| match link(item) {
                Link::Call(other) | Link::Output(other) => &other == id,
                Link::None => false,
            });
            if here == index || partner {
                removed.push(item.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Repair the history so that the API accepts it; idempotent:
    ///
    /// - a call without an output gets one, `"aborted"`, after the call and the calls and outputs
    ///   that directly follow it (so that a chat assistant message keeps its tool messages
    ///   together);
    /// - an output whose call is not earlier in the conversation is dropped;
    /// - when the model takes no images (`caps.images` is false), the images of messages and
    ///   tool outputs are replaced with a short text ([`IMAGE_OMITTED`]).
    ///
    /// The synthetic outputs carry no id, so the repaired history is the same every time: the
    /// prompt cache still applies.
    pub fn normalize(&mut self, caps: ModelCaps) -> NormalizeReport {
        let mut report = NormalizeReport::default();

        // Orphan outputs.
        let mut calls = BTreeSet::new();
        self.items.retain(|item| match link(item) {
            Link::Call(id) => {
                calls.insert(id);
                true
            }
            Link::Output(id) if calls.contains(&id) => true,
            Link::Output(id) => {
                report.orphans.push(id);
                false
            }
            Link::None => true,
        });

        // Missing outputs.
        let answered: BTreeSet<String> = self
            .items
            .iter()
            .filter_map(|item| match link(item) {
                Link::Output(id) => Some(id),
                _ => None,
            })
            .collect();
        let mut index = 0;
        while index < self.items.len() {
            if let Link::Call(id) = link(&self.items[index]) {
                if !answered.contains(&id) {
                    let call = invocation(&self.items[index]);
                    let mut at = index + 1;
                    while at < self.items.len() && link(&self.items[at]) != Link::None {
                        at += 1;
                    }
                    let answer = call.map_or_else(
                        || {
                            to_item(json!({
                                "type": "function_call_output",
                                "call_id": id,
                                "output": ABORTED_OUTPUT,
                            }))
                        },
                        |call| CallAnswer::aborted(&call).to_input_item(),
                    );
                    self.items.insert(at, answer);
                    report.aborted.push(id);
                }
            }
            index += 1;
        }

        if !caps.images {
            for item in &mut self.items {
                let mut value = view(item);
                let mut count = 0;
                for key in ["content", "output"] {
                    if let Some(Value::Array(parts)) = value.get_mut(key) {
                        for part in parts {
                            if part.get("type").and_then(Value::as_str) == Some("input_image") {
                                *part = json!({"type": "input_text", "text": IMAGE_OMITTED});
                                count += 1;
                            }
                        }
                    }
                }
                if count > 0 {
                    *item = to_item(value);
                    report.images += count;
                }
            }
        }

        debug_assert!(
            self.clone().normalize_again(caps).is_clean(),
            "normalize is idempotent"
        );
        report
    }

    fn normalize_again(mut self, caps: ModelCaps) -> NormalizeReport {
        // Without the debug assertion, so that it does not recurse.
        let mut calls = BTreeSet::new();
        let mut report = NormalizeReport::default();
        for item in &self.items {
            match link(item) {
                Link::Call(id) => {
                    calls.insert(id);
                }
                Link::Output(id) if !calls.contains(&id) => report.orphans.push(id),
                _ => {}
            }
        }
        report.aborted = self
            .unanswered_calls()
            .into_iter()
            .map(|call| call.call_id)
            .collect();
        if !caps.images {
            self.items.retain(|item| {
                let value = view(item);
                ["content", "output"].iter().any(|key| {
                    value[*key].as_array().is_some_and(|parts| {
                        parts
                            .iter()
                            .any(|part| part["type"].as_str() == Some("input_image"))
                    })
                })
            });
            report.images = self.items.len();
        }
        report
    }

    /// The input of a stateless request: the items without their ids (the server kept none), and
    /// without the reasoning it cannot read back (reasoning items without encrypted content).
    /// Item references are kept.
    pub(crate) fn stateless_input(&self) -> Vec<InputItem> {
        self.items
            .iter()
            .filter_map(|item| {
                let mut value = view(item);
                let kind = value
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if kind == "item_reference" {
                    return Some(item.clone());
                }
                if kind == "reasoning"
                    && value
                        .get("encrypted_content")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    return None;
                }
                let map = value.as_object_mut()?;
                if map.remove("id").is_none() {
                    return Some(item.clone());
                }
                Some(InputItem::Unknown(value))
            })
            .collect()
    }

    /// The conversation as chat messages: messages by role; consecutive calls (with the
    /// assistant text before them) as one assistant message with tool calls; outputs as tool
    /// messages (images and files as a short text). Reasoning and other items are left out.
    pub(crate) fn to_chat_messages(&self) -> Vec<ChatCompletionRequestMessage> {
        let mut messages = Vec::new();
        let mut assistant: Option<(Option<String>, Vec<Value>)> = None;
        let flush = |assistant: &mut Option<(Option<String>, Vec<Value>)>,
                     messages: &mut Vec<Value>| {
            if let Some((content, calls)) = assistant.take() {
                let mut message = Map::new();
                message.insert("role".into(), "assistant".into());
                message.insert("content".into(), content.map_or(Value::Null, Value::String));
                if !calls.is_empty() {
                    message.insert("tool_calls".into(), Value::Array(calls));
                }
                messages.push(Value::Object(message));
            }
        };
        for item in &self.items {
            let value = view(item);
            let kind = value.get("type").and_then(Value::as_str);
            let role = value.get("role").and_then(Value::as_str);
            match (kind, role) {
                (Some("message") | None, Some("assistant")) => {
                    flush(&mut assistant, &mut messages);
                    let text = content_text(&value["content"]);
                    assistant = Some(((!text.is_empty()).then_some(text), Vec::new()));
                }
                (Some("message") | None, Some(role)) => {
                    flush(&mut assistant, &mut messages);
                    let role = if role == "developer" {
                        "developer"
                    } else {
                        role
                    };
                    messages.push(
                        json!({"role": role, "content": chat_content(&value["content"], role)}),
                    );
                }
                (Some("function_call"), _) => {
                    let call = json!({
                        "id": value["call_id"],
                        "type": "function",
                        "function": {"name": value["name"], "arguments": value["arguments"]},
                    });
                    assistant
                        .get_or_insert_with(|| (None, Vec::new()))
                        .1
                        .push(call);
                }
                (Some("custom_tool_call"), _) => {
                    let call = json!({
                        "id": value["call_id"],
                        "type": "custom",
                        "custom": {"name": value["name"], "input": value["input"]},
                    });
                    assistant
                        .get_or_insert_with(|| (None, Vec::new()))
                        .1
                        .push(call);
                }
                (Some("function_call_output" | "custom_tool_call_output"), _) => {
                    flush(&mut assistant, &mut messages);
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": value["call_id"],
                        "content": output_text(&value["output"]),
                    }));
                }
                _ => {}
            }
        }
        flush(&mut assistant, &mut messages);
        messages
            .into_iter()
            .filter_map(|message| match serde_json::from_value(message.clone()) {
                Ok(message) => Some(message),
                Err(error) => {
                    tracing::warn!(%error, %message, "history item left out of the chat request");
                    None
                }
            })
            .collect()
    }

    /// Append the assistant message of a chat completion: its text, then its tool calls as
    /// `function_call` (or `custom_tool_call`) items. Returns the calls.
    pub(crate) fn record_chat_message(&mut self, message: &Value) -> Vec<ToolInvocation> {
        let text = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !text.is_empty() {
            self.push_assistant(text);
        }
        let mut calls = Vec::new();
        for call in message
            .get("tool_calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = call["id"].as_str().unwrap_or_default();
            let invocation = match call["type"].as_str() {
                Some("custom") => ToolInvocation::custom(
                    id,
                    call["custom"]["name"].as_str().unwrap_or_default(),
                    call["custom"]["input"].as_str().unwrap_or_default(),
                ),
                _ => ToolInvocation::function(
                    id,
                    call["function"]["name"].as_str().unwrap_or_default(),
                    call["function"]["arguments"].as_str().unwrap_or_default(),
                ),
            };
            self.push_call(&invocation);
            calls.push(invocation);
        }
        calls
    }
}

impl From<Vec<InputItem>> for Conversation {
    fn from(items: Vec<InputItem>) -> Self {
        Self { items }
    }
}

impl Extend<InputItem> for Conversation {
    fn extend<I: IntoIterator<Item = InputItem>>(&mut self, items: I) {
        self.items.extend(items);
    }
}

/// An item as JSON.
fn view(item: &InputItem) -> Value {
    serde_json::to_value(item).unwrap_or(Value::Null)
}

/// An item from JSON: the generated type when it decodes, else the JSON as it is.
fn to_item(value: Value) -> InputItem {
    serde_json::from_value(value.clone()).unwrap_or(InputItem::Unknown(value))
}

fn link(item: &InputItem) -> Link {
    let value = view(item);
    let id = value.get("call_id").and_then(Value::as_str);
    match (value.get("type").and_then(Value::as_str), id) {
        (Some("function_call" | "custom_tool_call"), Some(id)) => Link::Call(id.to_string()),
        (Some("function_call_output" | "custom_tool_call_output"), Some(id)) => {
            Link::Output(id.to_string())
        }
        _ => Link::None,
    }
}

/// The call of a `function_call` or `custom_tool_call` item.
fn invocation(item: &InputItem) -> Option<ToolInvocation> {
    let value = view(item);
    let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default();
    let call = match value.get("type").and_then(Value::as_str)? {
        "function_call" => {
            ToolInvocation::function(text("call_id"), text("name"), text("arguments"))
        }
        "custom_tool_call" => ToolInvocation::custom(text("call_id"), text("name"), text("input")),
        _ => return None,
    };
    Some(match value.get("namespace").and_then(Value::as_str) {
        Some(namespace) => call.in_namespace(namespace),
        None => call,
    })
}

/// The text of a message's content: the string, or its text parts.
fn content_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    }
}

/// The text of a tool output: the string, or its text items (other items as a short text).
fn output_text(output: &Value) -> String {
    match output {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .map(|part| {
                part.get("text").and_then(Value::as_str).map_or_else(
                    || {
                        format!(
                            "[{} omitted: chat tool messages are text]",
                            part.get("type").and_then(Value::as_str).unwrap_or("item")
                        )
                    },
                    str::to_string,
                )
            })
            .collect(),
        _ => String::new(),
    }
}

/// The chat content of a message: text, or (for a user message) text and image parts.
fn chat_content(content: &Value, role: &str) -> Value {
    let Value::Array(parts) = content else {
        return Value::String(content_text(content));
    };
    if role != "user" {
        return Value::String(content_text(content));
    }
    let parts: Vec<Value> = parts
        .iter()
        .filter_map(|part| match part.get("type").and_then(Value::as_str) {
            Some("input_text" | "output_text" | "text") => {
                Some(json!({"type": "text", "text": part["text"]}))
            }
            Some("input_image") => part.get("image_url").and_then(Value::as_str).map(|url| {
                let mut image = json!({"url": url});
                if let Some(detail) = part.get("detail").filter(|detail| detail.is_string()) {
                    image["detail"] = detail.clone();
                }
                json!({"type": "image_url", "image_url": image})
            }),
            _ => None,
        })
        .collect();
    Value::Array(parts)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::tool_framework::OutputContent;

    fn types(conversation: &Conversation) -> Vec<String> {
        conversation
            .items()
            .iter()
            .map(|item| {
                let value = view(item);
                let kind = value["type"].as_str().unwrap_or("?").to_string();
                match value["call_id"].as_str() {
                    Some(id) => format!("{kind}:{id}"),
                    None => kind,
                }
            })
            .collect()
    }

    #[test]
    fn answers_unanswered_calls_after_their_group() {
        let mut conversation = Conversation::user("hi");
        let a = ToolInvocation::function("a", "f", "{}");
        let b = ToolInvocation::function("b", "f", "{}");
        conversation.push_call(&a);
        conversation.push_call(&b);
        conversation.push_answer(&CallAnswer::new(&b, "2", true));
        conversation.push_user("next");
        let report = conversation.normalize(ModelCaps::default());
        assert_eq!(report.aborted, ["a"]);
        assert_eq!(
            types(&conversation),
            [
                "message",
                "function_call:a",
                "function_call:b",
                "function_call_output:b",
                "function_call_output:a",
                "message"
            ]
        );
        let aborted = view(&conversation.items()[4]);
        assert_eq!(aborted["output"], ABORTED_OUTPUT);
        // Idempotent.
        assert!(conversation.normalize(ModelCaps::default()).is_clean());
    }

    #[test]
    fn custom_calls_get_custom_outputs() {
        let mut conversation = Conversation::new();
        conversation
            .push_call(&ToolInvocation::custom("c", "patch", "*** Begin").in_namespace("fs"));
        conversation.normalize(ModelCaps::default());
        let output = view(&conversation.items()[1]);
        assert_eq!(output["type"], "custom_tool_call_output");
        assert_eq!(output["call_id"], "c");
        assert_eq!(conversation.calls()[0].namespace.as_deref(), Some("fs"));
    }

    #[test]
    fn drops_orphan_outputs() {
        let mut conversation = Conversation::user("hi");
        let a = ToolInvocation::function("a", "f", "{}");
        conversation.push_answer(&CallAnswer::new(&a, "early", true));
        conversation.push_call(&a);
        conversation.push_answer(&CallAnswer::new(&a, "1", true));
        let ghost = ToolInvocation::function("ghost", "f", "{}");
        conversation.push_answer(&CallAnswer::new(&ghost, "?", true));
        let report = conversation.normalize(ModelCaps::default());
        assert_eq!(report.orphans, ["a", "ghost"]);
        assert!(report.aborted.is_empty());
        assert_eq!(
            types(&conversation),
            ["message", "function_call:a", "function_call_output:a"]
        );
    }

    #[test]
    fn strips_images_when_the_model_takes_none() {
        let mut conversation = Conversation::new();
        let call = ToolInvocation::function("a", "screenshot", "{}");
        conversation.push_call(&call);
        conversation.push_answer(&CallAnswer::new(
            &call,
            vec![
                OutputContent::text("here"),
                OutputContent::image_url("https://example.com/a.png"),
            ],
            true,
        ));
        assert!(conversation.normalize(ModelCaps::default()).is_clean());
        let report = conversation.normalize(ModelCaps::new().with_images(false));
        assert_eq!(report.images, 1);
        let output = view(&conversation.items()[1]);
        assert_eq!(output["output"][1]["text"], IMAGE_OMITTED);
        assert!(conversation
            .normalize(ModelCaps::new().with_images(false))
            .is_clean());
    }

    #[test]
    fn remove_takes_the_partner_along() {
        let mut conversation = Conversation::user("hi");
        let a = ToolInvocation::function("a", "f", "{}");
        conversation.push_call(&a);
        conversation.push_answer(&CallAnswer::new(&a, "1", true));
        let removed = conversation.remove(2);
        assert_eq!(removed.len(), 2);
        assert_eq!(types(&conversation), ["message"]);
        assert!(conversation.remove(5).is_empty());
    }

    #[test]
    fn stateless_input_drops_ids_and_unreadable_reasoning() {
        let conversation = Conversation::from_items([
            to_item(json!({"type": "reasoning", "id": "rs_1", "summary": []})),
            to_item(
                json!({"type": "reasoning", "id": "rs_2", "summary": [], "encrypted_content": "gAAA"}),
            ),
            to_item(
                json!({"type": "function_call", "id": "fc_1", "call_id": "a", "name": "f", "arguments": "{}"}),
            ),
            to_item(json!({"type": "item_reference", "id": "msg_1"})),
        ]);
        let input: Vec<Value> = conversation
            .stateless_input()
            .iter()
            .map(|item| serde_json::to_value(item).unwrap())
            .collect();
        assert_eq!(input.len(), 3);
        assert_eq!(input[0]["encrypted_content"], "gAAA");
        assert!(input[0].get("id").is_none());
        assert!(input[1].get("id").is_none());
        assert_eq!(input[2]["id"], "msg_1");
    }

    #[test]
    fn becomes_chat_messages() {
        let mut conversation = Conversation::new();
        conversation.push_system("Be brief");
        conversation.push_user("Weather in Ghent and Paris?");
        conversation.push_assistant("Let me check.");
        let a = ToolInvocation::function("a", "get_weather", r#"{"city":"Ghent"}"#);
        let b = ToolInvocation::function("b", "get_weather", r#"{"city":"Paris"}"#);
        conversation.push_call(&a);
        conversation.push_call(&b);
        conversation.push_answer(&CallAnswer::new(&a, "12", true));
        conversation.push_answer(&CallAnswer::new(
            &b,
            vec![
                OutputContent::text("14"),
                OutputContent::image_url("https://x/y.png"),
            ],
            true,
        ));
        conversation.push(to_item(
            json!({"type": "reasoning", "id": "rs", "summary": []}),
        ));
        let messages: Vec<Value> = conversation
            .to_chat_messages()
            .iter()
            .map(|message| serde_json::to_value(message).unwrap())
            .collect();
        assert_eq!(messages.len(), 5);
        assert_eq!(
            messages[0],
            json!({"role": "system", "content": "Be brief"})
        );
        assert_eq!(messages[2]["content"], "Let me check.");
        assert_eq!(messages[2]["tool_calls"][1]["id"], "b");
        assert_eq!(
            messages[2]["tool_calls"][1]["function"]["name"],
            "get_weather"
        );
        assert_eq!(
            messages[3],
            json!({"role": "tool", "tool_call_id": "a", "content": "12"})
        );
        assert_eq!(
            messages[4]["content"],
            "14[input_image omitted: chat tool messages are text]"
        );
    }

    #[test]
    fn records_chat_and_responses_output() {
        let mut conversation = Conversation::new();
        let calls = conversation.record_chat_message(&json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{"id": "a", "type": "function", "function": {"name": "f", "arguments": "{}"}}]
        }));
        assert_eq!(calls, [ToolInvocation::function("a", "f", "{}")]);
        assert_eq!(conversation.unanswered_calls(), calls);
        let output: Vec<OutputItem> = serde_json::from_value(json!([
            {"type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
             "content": [{"type": "output_text", "text": "Done", "annotations": []}]}
        ]))
        .unwrap();
        conversation.record_output(&output);
        assert_eq!(conversation.last_assistant_text().as_deref(), Some("Done"));
    }
}
