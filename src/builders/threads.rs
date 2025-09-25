//! Threads API builders.
//!
//! Provides ergonomic builders for creating assistant threads and messages with
//! attachments and metadata support.

use std::collections::HashMap;

use openai_client_base::models::create_message_request::Role as MessageRole;
use openai_client_base::models::{
    AssistantToolsCode, AssistantToolsFileSearchTypeOnly, CreateMessageRequest,
    CreateMessageRequestAttachmentsInner, CreateMessageRequestAttachmentsInnerToolsInner,
    CreateThreadRequest,
};
use serde_json::Value;

use crate::Builder;

/// Attachment tools that can be associated with a message file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentTool {
    /// Make the attachment available to the code interpreter tool.
    CodeInterpreter,
    /// Make the attachment available to the file search tool.
    FileSearch,
}

impl AttachmentTool {
    fn to_api(self) -> CreateMessageRequestAttachmentsInnerToolsInner {
        match self {
            Self::CodeInterpreter =>
                CreateMessageRequestAttachmentsInnerToolsInner::AssistantToolsCode(
                    Box::new(AssistantToolsCode::new(
                        openai_client_base::models::assistant_tools_code::Type::CodeInterpreter,
                    )),
                ),
            Self::FileSearch =>
                CreateMessageRequestAttachmentsInnerToolsInner::AssistantToolsFileSearchTypeOnly(
                    Box::new(AssistantToolsFileSearchTypeOnly::new(
                        openai_client_base::models::assistant_tools_file_search_type_only::Type::FileSearch,
                    )),
                ),
        }
    }
}

/// Attachment to include with a thread message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAttachment {
    file_id: String,
    tools: Vec<AttachmentTool>,
}

impl MessageAttachment {
    /// Attach a file for the code interpreter tool.
    #[must_use]
    pub fn for_code_interpreter(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            tools: vec![AttachmentTool::CodeInterpreter],
        }
    }

    /// Attach a file for the file search tool.
    #[must_use]
    pub fn for_file_search(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            tools: vec![AttachmentTool::FileSearch],
        }
    }

    /// Add an additional tool that should receive this attachment.
    #[must_use]
    pub fn with_tool(mut self, tool: AttachmentTool) -> Self {
        if !self.tools.contains(&tool) {
            self.tools.push(tool);
        }
        self
    }

    fn into_api(self) -> CreateMessageRequestAttachmentsInner {
        let mut inner = CreateMessageRequestAttachmentsInner::new();
        inner.file_id = Some(self.file_id);
        if !self.tools.is_empty() {
            let tools = self.tools.into_iter().map(AttachmentTool::to_api).collect();
            inner.tools = Some(tools);
        }
        inner
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum MetadataState {
    #[default]
    Unset,
    Present(HashMap<String, String>),
    ExplicitNull,
}

impl MetadataState {
    fn upsert(&mut self, key: String, value: String) {
        match self {
            MetadataState::Unset | MetadataState::ExplicitNull => {
                let mut map = HashMap::new();
                map.insert(key, value);
                *self = MetadataState::Present(map);
            }
            MetadataState::Present(map) => {
                map.insert(key, value);
            }
        }
    }

    fn replace(&mut self, metadata: HashMap<String, String>) {
        *self = MetadataState::Present(metadata);
    }

    fn clear(&mut self) {
        *self = MetadataState::ExplicitNull;
    }

    #[allow(clippy::option_option)]
    fn into_option(self) -> Option<Option<HashMap<String, String>>> {
        match self {
            MetadataState::Unset => None,
            MetadataState::Present(map) if map.is_empty() => None,
            MetadataState::Present(map) => Some(Some(map)),
            MetadataState::ExplicitNull => Some(None),
        }
    }
}

/// Builder for messages that seed a thread.
#[derive(Debug, Clone, Default)]
pub struct ThreadMessageBuilder {
    role: MessageRole,
    content: String,
    attachments: Vec<MessageAttachment>,
    metadata: MetadataState,
}

impl ThreadMessageBuilder {
    /// Create a user message with the provided text content.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            attachments: Vec::new(),
            metadata: MetadataState::Unset,
        }
    }

    /// Create an assistant-authored message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            attachments: Vec::new(),
            metadata: MetadataState::Unset,
        }
    }

    /// Set the message content explicitly.
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Attach a file to the message.
    #[must_use]
    pub fn attachment(mut self, attachment: MessageAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Attach multiple files to the message.
    #[must_use]
    pub fn attachments<I>(mut self, attachments: I) -> Self
    where
        I: IntoIterator<Item = MessageAttachment>,
    {
        self.attachments.extend(attachments);
        self
    }

    /// Set metadata for the message.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.upsert(key.into(), value.into());
        self
    }

    /// Replace metadata with a full map.
    #[must_use]
    pub fn metadata_map(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata.replace(metadata);
        self
    }

    /// Remove metadata by sending an explicit null.
    #[must_use]
    pub fn clear_metadata(mut self) -> Self {
        self.metadata.clear();
        self
    }
}

impl Builder<CreateMessageRequest> for ThreadMessageBuilder {
    fn build(self) -> crate::Result<CreateMessageRequest> {
        let mut request = CreateMessageRequest::new(self.role, Value::String(self.content));
        if !self.attachments.is_empty() {
            let attachments = self
                .attachments
                .into_iter()
                .map(MessageAttachment::into_api)
                .collect();
            request.attachments = Some(Some(attachments));
        }
        request.metadata = self.metadata.into_option();
        Ok(request)
    }
}

impl ThreadMessageBuilder {
    /// Build the message, panicking only if serialization fails (not expected).
    #[must_use]
    pub fn finish(self) -> CreateMessageRequest {
        self.build()
            .expect("thread message builder should be infallible")
    }
}

/// Builder for creating a thread with initial messages and metadata.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::Builder;
/// use openai_ergonomic::builders::threads::{ThreadMessageBuilder, ThreadRequestBuilder};
///
/// let thread = ThreadRequestBuilder::new()
///     .user_message("Hello there")
///     .message_builder(ThreadMessageBuilder::assistant("How can I help?"))
///     .unwrap()
///     .metadata("topic", "support")
///     .build()
///     .unwrap();
///
/// assert_eq!(thread.metadata.unwrap().unwrap().get("topic"), Some(&"support".to_string()));
/// assert_eq!(thread.messages.as_ref().unwrap().len(), 2);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ThreadRequestBuilder {
    messages: Vec<CreateMessageRequest>,
    metadata: MetadataState,
}

impl ThreadRequestBuilder {
    /// Create a new empty thread builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the thread with an initial user message.
    #[must_use]
    pub fn user_message(mut self, content: impl Into<String>) -> Self {
        self.messages
            .push(ThreadMessageBuilder::user(content).finish());
        self
    }

    /// Seed the thread with an assistant message.
    #[must_use]
    pub fn assistant_message(mut self, content: impl Into<String>) -> Self {
        self.messages
            .push(ThreadMessageBuilder::assistant(content).finish());
        self
    }

    /// Add a fully configured message request.
    #[must_use]
    pub fn message_request(mut self, message: CreateMessageRequest) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a thread message builder.
    pub fn message_builder(mut self, builder: ThreadMessageBuilder) -> crate::Result<Self> {
        self.messages.push(builder.build()?);
        Ok(self)
    }

    /// Add metadata to the thread.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.upsert(key.into(), value.into());
        self
    }

    /// Replace thread metadata with a full map.
    #[must_use]
    pub fn metadata_map(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata.replace(metadata);
        self
    }

    /// Remove metadata by sending an explicit null.
    #[must_use]
    pub fn clear_metadata(mut self) -> Self {
        self.metadata.clear();
        self
    }

    /// Access the configured messages.
    #[must_use]
    pub fn messages(&self) -> &[CreateMessageRequest] {
        &self.messages
    }
}

impl Builder<CreateThreadRequest> for ThreadRequestBuilder {
    fn build(self) -> crate::Result<CreateThreadRequest> {
        let mut request = CreateThreadRequest::new();
        if !self.messages.is_empty() {
            request.messages = Some(self.messages);
        }
        request.metadata = self.metadata.into_option();
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_basic_user_message() {
        let builder = ThreadMessageBuilder::user("Hello");
        let message = builder.build().expect("builder should succeed");

        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, Value::String("Hello".to_string()));
        assert!(message.attachments.is_none());
        assert!(message.metadata.is_none());
    }

    #[test]
    fn builds_message_with_attachment() {
        let attachment = MessageAttachment::for_code_interpreter("file-123");
        let message = ThreadMessageBuilder::user("process this")
            .attachment(attachment)
            .build()
            .expect("builder should succeed");

        let attachments = message.attachments.unwrap().unwrap();
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].file_id.as_deref(), Some("file-123"));
        assert!(attachments[0].tools.as_ref().is_some());
    }

    #[test]
    fn builds_thread_with_metadata() {
        let thread = ThreadRequestBuilder::new()
            .user_message("Hi there")
            .metadata("topic", "support")
            .build()
            .expect("builder should succeed");

        assert!(thread.messages.is_some());
        let metadata = thread.metadata.unwrap().unwrap();
        assert_eq!(metadata.get("topic"), Some(&"support".to_string()));
    }

    #[test]
    fn can_explicitly_clear_metadata() {
        let thread = ThreadRequestBuilder::new()
            .metadata("foo", "bar")
            .clear_metadata()
            .build()
            .expect("builder should succeed");

        assert!(thread.metadata.is_some());
        assert!(thread.metadata.unwrap().is_none());
    }

    #[test]
    fn accepts_custom_message_builder() {
        let message_builder = ThreadMessageBuilder::assistant("Hello").metadata("tone", "friendly");
        let thread = ThreadRequestBuilder::new()
            .message_builder(message_builder)
            .expect("builder should succeed")
            .build()
            .expect("thread build should succeed");

        let message = thread.messages.unwrap();
        assert_eq!(message.len(), 1);
        assert_eq!(message[0].role, MessageRole::Assistant);
        let metadata = message[0].metadata.clone().unwrap().unwrap();
        assert_eq!(metadata.get("tone"), Some(&"friendly".to_string()));
    }
}
