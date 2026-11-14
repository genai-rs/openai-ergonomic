//! Tool calls, their answers and what the model sees of a tool's output, for both the Chat
//! Completions and the Responses API.

use std::borrow::Cow;

use openai_client_base::models::{
    ChatCompletionMessageToolCallsInner, CustomToolCall, DetailEnum, FunctionToolCall, InputItem,
    OutputItem,
};
use serde::Serialize;
use serde_json::{json, Value};

use super::ToolError;

/// The kind of a tool, or of a call the model made.
///
/// New kinds may be added in minor releases: match with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolKind {
    /// A function tool: JSON arguments, answered with a `function_call_output` (Responses API)
    /// or a tool message (Chat Completions).
    Function,
    /// A custom (freeform) tool: text input, answered with a `custom_tool_call_output`.
    Custom,
    /// A call of a type this crate does not dispatch, with the type the model sent.
    Other(String),
}

/// One tool call the model made, from either API, ready for
/// [`ToolRegistry::answer_call`](super::ToolRegistry::answer_call).
///
/// Convert a call with `From`: a chat call (`&ChatCompletionMessageToolCallsInner`), a Responses
/// API call (`&FunctionToolCall`, `&CustomToolCall`), or an output item with
/// [`from_output_item`](Self::from_output_item).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ToolInvocation {
    /// The identifier the answer must carry.
    pub call_id: String,
    /// The namespace of the tool (Responses API), if any.
    pub namespace: Option<String>,
    /// The tool name.
    pub name: String,
    /// The kind of call.
    pub kind: ToolKind,
    /// The JSON arguments of a function call, or the text input of a custom call.
    pub input: String,
}

impl ToolInvocation {
    /// A function call with JSON `arguments`.
    #[must_use]
    pub fn function(
        call_id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            namespace: None,
            name: name.into(),
            kind: ToolKind::Function,
            input: arguments.into(),
        }
    }

    /// A custom (freeform) tool call with its text `input`.
    #[must_use]
    pub fn custom(
        call_id: impl Into<String>,
        name: impl Into<String>,
        input: impl Into<String>,
    ) -> Self {
        Self {
            kind: ToolKind::Custom,
            ..Self::function(call_id, name, input)
        }
    }

    /// The same call, of a tool in `namespace`.
    #[must_use]
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// The call of a Responses API output item: a function call or a custom tool call. Other
    /// items (messages, reasoning, built-in tool calls) give `None`.
    #[must_use]
    pub fn from_output_item(item: &OutputItem) -> Option<Self> {
        match item {
            OutputItem::FunctionToolCall(call) => Some(call.into()),
            OutputItem::CustomToolCall(call) => Some(call.into()),
            _ => None,
        }
    }
}

impl From<&FunctionToolCall> for ToolInvocation {
    fn from(call: &FunctionToolCall) -> Self {
        Self {
            namespace: call.namespace.clone(),
            ..Self::function(&call.call_id, &call.name, &call.arguments)
        }
    }
}

impl From<&CustomToolCall> for ToolInvocation {
    fn from(call: &CustomToolCall) -> Self {
        Self {
            namespace: call.namespace.clone(),
            ..Self::custom(&call.call_id, &call.name, &call.input)
        }
    }
}

impl From<&ChatCompletionMessageToolCallsInner> for ToolInvocation {
    fn from(call: &ChatCompletionMessageToolCallsInner) -> Self {
        match call {
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageToolCall(call) => {
                Self::function(&call.id, &call.function.name, &call.function.arguments)
            }
            ChatCompletionMessageToolCallsInner::ChatCompletionMessageCustomToolCall(call) => {
                Self::custom(&call.id, &call.custom.name, &call.custom.input)
            }
            ChatCompletionMessageToolCallsInner::Unknown(value) => {
                let text = |key: &str| {
                    value
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                Self {
                    call_id: text("id"),
                    namespace: None,
                    name: String::new(),
                    kind: ToolKind::Other(text("type")),
                    input: String::new(),
                }
            }
        }
    }
}

/// Image detail of an [`OutputContent::Image`]: the generated `DetailEnum` (`Low`, `High`,
/// `Auto`, `Original`).
pub type ImageDetail = DetailEnum;

/// One content item of a tool output that is more than text.
///
/// The Responses API accepts text, images and files as the output of a function or custom tool
/// call; Chat Completions tool messages accept text only. New kinds may be added in minor
/// releases: build items with the constructors and match with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputContent {
    /// Text.
    Text(String),
    /// An image, by URL (or data URL) or by uploaded file ID.
    #[non_exhaustive]
    Image {
        /// The image URL, or a `data:` URL with the base64 image.
        image_url: Option<String>,
        /// The ID of an uploaded file.
        file_id: Option<String>,
        /// The detail level the model sees the image at.
        detail: Option<ImageDetail>,
    },
    /// A file, by uploaded file ID, URL or base64 data.
    #[non_exhaustive]
    File {
        /// The ID of an uploaded file.
        file_id: Option<String>,
        /// The URL of the file.
        file_url: Option<String>,
        /// The base64 content of the file.
        file_data: Option<String>,
        /// The file name the model sees.
        filename: Option<String>,
    },
}

impl OutputContent {
    /// A text item.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// An image at a URL (or a `data:` URL).
    #[must_use]
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            image_url: Some(url.into()),
            file_id: None,
            detail: None,
        }
    }

    /// An image uploaded as a file.
    #[must_use]
    pub fn image_file(file_id: impl Into<String>) -> Self {
        Self::Image {
            image_url: None,
            file_id: Some(file_id.into()),
            detail: None,
        }
    }

    /// A file uploaded with the Files API.
    #[must_use]
    pub fn file_id(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: Some(file_id.into()),
            file_url: None,
            file_data: None,
            filename: None,
        }
    }

    /// A file at a URL.
    #[must_use]
    pub fn file_url(url: impl Into<String>) -> Self {
        Self::File {
            file_id: None,
            file_url: Some(url.into()),
            file_data: None,
            filename: None,
        }
    }

    /// A file sent inline: its name and its base64 content.
    #[must_use]
    pub fn file_data(filename: impl Into<String>, base64: impl Into<String>) -> Self {
        Self::File {
            file_id: None,
            file_url: None,
            file_data: Some(base64.into()),
            filename: Some(filename.into()),
        }
    }

    /// The same image with a detail level; other items are returned unchanged.
    #[must_use]
    pub fn with_detail(mut self, level: ImageDetail) -> Self {
        if let Self::Image { detail, .. } = &mut self {
            *detail = Some(level);
        }
        self
    }

    /// The text of a text item.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The item as the Responses API sends it (`input_text`, `input_image`, `input_file`).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut value = match self {
            Self::Text(text) => json!({"type": "input_text", "text": text}),
            Self::Image {
                image_url,
                file_id,
                detail,
            } => json!({
                "type": "input_image",
                "image_url": image_url,
                "file_id": file_id,
                "detail": detail.map(|detail| detail.to_string()),
            }),
            Self::File {
                file_id,
                file_url,
                file_data,
                filename,
            } => json!({
                "type": "input_file",
                "file_id": file_id,
                "file_url": file_url,
                "file_data": file_data,
                "filename": filename,
            }),
        };
        if let Some(map) = value.as_object_mut() {
            map.retain(|_, value| !value.is_null());
        }
        value
    }
}

/// What the model sees of a tool's output: text, or content items (text, images, files).
///
/// [`FunctionTool`](super::FunctionTool)s produce JSON text; [`RichTool`](super::RichTool)s,
/// [`CustomTool`](super::CustomTool)s and dynamic tools return a body directly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolOutputBody {
    /// Text, such as the JSON encoding of a function tool's output.
    Text(String),
    /// Content items. Chat Completions accepts only text items.
    Items(Vec<OutputContent>),
}

impl ToolOutputBody {
    /// A text body.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// A text body with the JSON encoding of `value` (strings are quoted, as for function tools).
    pub fn json<T: Serialize + ?Sized>(value: &T) -> crate::Result<Self> {
        Ok(Self::Text(serde_json::to_string(value)?))
    }

    /// The text of a text body.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Items(_) => None,
        }
    }

    /// The body as the content of a Chat Completions tool message, which takes text only: the
    /// text, or the text items concatenated. An image or a file is an error
    /// ([`ToolError::UnsupportedOutput`]).
    pub fn to_chat_text(&self) -> Result<String, ToolError> {
        match self {
            Self::Text(text) => Ok(text.clone()),
            Self::Items(items) => {
                let mut text = String::new();
                for item in items {
                    let Some(part) = item.as_text() else {
                        return Err(ToolError::UnsupportedOutput {
                            reason: "Chat Completions tool messages accept text only".into(),
                        });
                    };
                    text.push_str(part);
                }
                Ok(text)
            }
        }
    }

    /// The `output` of a `function_call_output` or `custom_tool_call_output` item: a string, or
    /// an array of content items.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::Text(text) => Value::String(text.clone()),
            Self::Items(items) => Value::Array(items.iter().map(OutputContent::to_json).collect()),
        }
    }

    /// The body cut to `budget`, keeping its head and its tail (see [`truncate_middle`]).
    ///
    /// The text of all text items counts against the budget, in order: the item that crosses it
    /// is cut in the middle and later text items are left out (an item says how many). Images
    /// and files are always kept. With `header`, a first line says how long the text was.
    #[must_use]
    pub fn truncated(&self, budget: OutputBudget, header: bool) -> Self {
        let Some(max) = budget.max_bytes() else {
            return self.clone();
        };
        match self {
            Self::Text(text) if text.len() <= max => self.clone(),
            Self::Text(text) => {
                let mut out = if header {
                    truncation_header(text.len(), text.lines().count())
                } else {
                    String::new()
                };
                out.push_str(&truncate_middle(text, budget));
                Self::Text(out)
            }
            Self::Items(items) => {
                let total: usize = items
                    .iter()
                    .filter_map(OutputContent::as_text)
                    .map(str::len)
                    .sum();
                if total <= max {
                    return self.clone();
                }
                let lines = items
                    .iter()
                    .filter_map(OutputContent::as_text)
                    .map(|text| text.lines().count())
                    .sum();
                let mut out = Vec::with_capacity(items.len() + 2);
                if header {
                    out.push(OutputContent::Text(truncation_header(total, lines)));
                }
                let mut left = max;
                let mut omitted = 0_usize;
                for item in items {
                    match item.as_text() {
                        None => out.push(item.clone()),
                        Some(_) if left == 0 => omitted += 1,
                        Some(text) if text.len() <= left => {
                            left -= text.len();
                            out.push(item.clone());
                        }
                        Some(text) => {
                            let budget = budget.with_bytes(left);
                            out.push(OutputContent::Text(
                                truncate_middle(text, budget).into_owned(),
                            ));
                            left = 0;
                        }
                    }
                }
                if omitted > 0 {
                    out.push(OutputContent::Text(format!(
                        "…{omitted} more text items omitted…"
                    )));
                }
                Self::Items(out)
            }
        }
    }
}

impl From<String> for ToolOutputBody {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for ToolOutputBody {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<Vec<OutputContent>> for ToolOutputBody {
    fn from(items: Vec<OutputContent>) -> Self {
        Self::Items(items)
    }
}

impl From<OutputContent> for ToolOutputBody {
    fn from(item: OutputContent) -> Self {
        Self::Items(vec![item])
    }
}

/// How much of a tool's output the model sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputBudget {
    /// At most this many bytes of text (UTF-8), cut on character boundaries.
    Bytes(usize),
    /// About this many tokens, counted as 4 bytes each.
    Tokens(usize),
    /// No limit.
    Unlimited,
}

impl OutputBudget {
    /// The budget in bytes; `None` when unlimited.
    #[must_use]
    pub fn max_bytes(self) -> Option<usize> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Tokens(tokens) => Some(tokens.saturating_mul(BYTES_PER_TOKEN)),
            Self::Unlimited => None,
        }
    }

    /// A budget of the same unit for `bytes` bytes.
    fn with_bytes(self, bytes: usize) -> Self {
        match self {
            Self::Tokens(_) => Self::Tokens(bytes / BYTES_PER_TOKEN),
            _ => Self::Bytes(bytes),
        }
    }
}

const BYTES_PER_TOKEN: usize = 4;

/// The truncation of tool outputs before they are sent to the model.
///
/// The default budget is 10 000 bytes, with a header line; a tool can override the budget
/// ([`FunctionTool::output_budget`](super::FunctionTool::output_budget)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TruncationConfig {
    /// The budget of tools that do not set one.
    pub default: OutputBudget,
    /// Whether a truncated output starts with a line that says how long it was.
    pub header: bool,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            default: OutputBudget::Bytes(10_000),
            header: true,
        }
    }
}

impl TruncationConfig {
    /// A configuration with this default budget and the header line.
    #[must_use]
    pub fn new(default: OutputBudget) -> Self {
        Self {
            default,
            header: true,
        }
    }

    /// The same configuration with or without the header line.
    #[must_use]
    pub fn with_header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    /// The budget of a tool: its own, else the default.
    #[must_use]
    pub fn budget_for(&self, tool_budget: Option<OutputBudget>) -> OutputBudget {
        tool_budget.unwrap_or(self.default)
    }
}

fn truncation_header(bytes: usize, lines: usize) -> String {
    format!(
        "[truncated: original {bytes} bytes/~{} tokens, {lines} lines]\n",
        bytes.div_ceil(BYTES_PER_TOKEN)
    )
}

/// Cut `text` to `budget`, keeping its first and its last half.
///
/// The middle is replaced with `…N chars truncated…` (`…N tokens truncated…` for a token
/// budget). Cuts fall on character boundaries; text within the budget is returned as is.
///
/// ```
/// use openai_ergonomic::tool_framework::{truncate_middle, OutputBudget};
///
/// let text = "a".repeat(10) + &"b".repeat(10);
/// assert_eq!(truncate_middle(&text, OutputBudget::Bytes(8)), "aaaa…12 chars truncated…bbbb");
/// assert_eq!(truncate_middle("short", OutputBudget::Bytes(8)), "short");
/// ```
#[must_use]
pub fn truncate_middle(text: &str, budget: OutputBudget) -> Cow<'_, str> {
    let Some(max) = budget.max_bytes() else {
        return Cow::Borrowed(text);
    };
    if text.len() <= max {
        return Cow::Borrowed(text);
    }
    let mut head_end = max / 2;
    while !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = text.len() - (max - max / 2);
    while !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let removed = &text[head_end..tail_start];
    let marker = match budget {
        OutputBudget::Tokens(_) => {
            format!(
                "…{} tokens truncated…",
                removed.len().div_ceil(BYTES_PER_TOKEN)
            )
        }
        _ => format!("…{} chars truncated…", removed.chars().count()),
    };
    Cow::Owned(format!(
        "{}{marker}{}",
        &text[..head_end],
        &text[tail_start..]
    ))
}

/// The answer to one tool call, to send back to the model.
///
/// [`ToolRegistry::answer_call`](super::ToolRegistry::answer_call) gives one for every call
/// it can answer, failed calls included (`success: false`, with the error text as output).
/// Send it with [`ResponsesBuilder::call_answer`](crate::ResponsesBuilder::call_answer) or
/// [`ChatCompletionBuilder::call_answer`](crate::ChatCompletionBuilder::call_answer).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallAnswer {
    /// The identifier of the call this answers.
    pub call_id: String,
    /// The kind of call: a function call is answered with a `function_call_output`, a custom
    /// call with a `custom_tool_call_output`.
    pub kind: ToolKind,
    /// What the model sees.
    pub output: ToolOutputBody,
    /// Whether the tool succeeded. Not sent to the model; for logging and policies.
    pub success: bool,
}

/// The output of a call that was never answered, as history repair inserts it.
pub const ABORTED_OUTPUT: &str = "aborted";

impl CallAnswer {
    /// An answer to `call`.
    #[must_use]
    pub fn new(call: &ToolInvocation, output: impl Into<ToolOutputBody>, success: bool) -> Self {
        Self {
            call_id: call.call_id.clone(),
            kind: call.kind.clone(),
            output: output.into(),
            success,
        }
    }

    /// A failed answer to `call`, with the text the model sees.
    #[must_use]
    pub fn failure(call: &ToolInvocation, message: impl Into<String>) -> Self {
        Self::new(call, ToolOutputBody::Text(message.into()), false)
    }

    /// The answer to a call that was not run, such as a denied or cancelled one: `"aborted"`.
    #[must_use]
    pub fn aborted(call: &ToolInvocation) -> Self {
        Self::failure(call, ABORTED_OUTPUT)
    }

    /// The same answer, its output cut to `budget` ([`ToolOutputBody::truncated`]).
    #[must_use]
    pub fn truncated(mut self, budget: OutputBudget, header: bool) -> Self {
        self.output = self.output.truncated(budget, header);
        self
    }

    /// The answer as a Responses API input item: a `function_call_output`, or a
    /// `custom_tool_call_output` for a custom call.
    #[must_use]
    pub fn to_input_item(&self) -> InputItem {
        let r#type = match self.kind {
            ToolKind::Custom => "custom_tool_call_output",
            _ => "function_call_output",
        };
        let value = json!({
            "type": r#type,
            "call_id": self.call_id,
            "output": self.output.to_json(),
        });
        serde_json::from_value(value.clone()).unwrap_or(InputItem::Unknown(value))
    }
}

impl From<CallAnswer> for InputItem {
    fn from(answer: CallAnswer) -> Self {
        answer.to_input_item()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn truncate_middle_keeps_char_boundaries() {
        let text = "é".repeat(10); // 20 bytes
        let cut = truncate_middle(&text, OutputBudget::Bytes(7));
        assert!(cut.starts_with("é"));
        assert!(cut.ends_with("é"));
        assert!(cut.contains("chars truncated"));
        let long = "x".repeat(100);
        let cut = truncate_middle(&long, OutputBudget::Tokens(2));
        assert_eq!(cut, "xxxx…23 tokens truncated…xxxx");
        assert_eq!(truncate_middle("abc", OutputBudget::Unlimited), "abc");
    }

    #[test]
    fn truncated_text_has_a_header() {
        let body = ToolOutputBody::text("line\n".repeat(10));
        let cut = body.truncated(OutputBudget::Bytes(10), true);
        let text = cut.as_text().unwrap();
        assert!(text.starts_with("[truncated: original 50 bytes/~13 tokens, 10 lines]\n"));
        assert_eq!(body.truncated(OutputBudget::Bytes(100), true), body);
        let cut = body.truncated(OutputBudget::Bytes(10), false);
        assert!(cut.as_text().unwrap().starts_with("line\n"));
    }

    #[test]
    fn truncated_items_keep_images() {
        let body = ToolOutputBody::Items(vec![
            OutputContent::text("a".repeat(6)),
            OutputContent::image_url("https://example.com/cat.png"),
            OutputContent::text("b".repeat(10)),
            OutputContent::text("c"),
        ]);
        let ToolOutputBody::Items(items) = body.truncated(OutputBudget::Bytes(10), false) else {
            panic!("items expected");
        };
        assert_eq!(items[0], OutputContent::text("aaaaaa"));
        assert!(matches!(items[1], OutputContent::Image { .. }));
        assert_eq!(items[2], OutputContent::text("bb…6 chars truncated…bb"));
        assert_eq!(items[3], OutputContent::text("…1 more text items omitted…"));
    }

    #[test]
    fn chat_text_rejects_images() {
        let body = ToolOutputBody::Items(vec![OutputContent::text("a"), OutputContent::text("b")]);
        assert_eq!(body.to_chat_text().unwrap(), "ab");
        let body: ToolOutputBody = OutputContent::image_file("file_1").into();
        assert!(matches!(
            body.to_chat_text(),
            Err(ToolError::UnsupportedOutput { .. })
        ));
    }

    #[test]
    fn answers_become_input_items() {
        let call = ToolInvocation::function("call_1", "f", "{}");
        let item = CallAnswer::new(&call, "42", true).to_input_item();
        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            json!({"type": "function_call_output", "call_id": "call_1", "output": "42"})
        );
        let call = ToolInvocation::custom("call_2", "patch", "*** Begin");
        let answer = CallAnswer::new(
            &call,
            vec![
                OutputContent::text("done"),
                OutputContent::image_url("https://example.com/a.png").with_detail(DetailEnum::Low),
            ],
            true,
        );
        let value = serde_json::to_value(answer.to_input_item()).unwrap();
        assert_eq!(value["type"], "custom_tool_call_output");
        assert_eq!(value["call_id"], "call_2");
        assert_eq!(
            value["output"],
            json!([
                {"type": "input_text", "text": "done"},
                {"type": "input_image", "image_url": "https://example.com/a.png", "detail": "low"}
            ])
        );
    }
}
