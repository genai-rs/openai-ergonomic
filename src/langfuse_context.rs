//! Langfuse trace attributes shared by an interceptor and its callers.

use opentelemetry::KeyValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Attribute names recognized by Langfuse.
pub mod attributes {
    /// Session identifier.
    pub const TRACE_SESSION_ID: &str = "langfuse.session.id";
    /// User identifier.
    pub const TRACE_USER_ID: &str = "langfuse.user.id";
    /// JSON array of tags.
    pub const TRACE_TAGS: &str = "langfuse.trace.tags";
    /// JSON object containing metadata.
    pub const TRACE_METADATA: &str = "langfuse.trace.metadata";
    /// Trace name.
    pub const TRACE_NAME: &str = "langfuse.trace.name";
}

/// Thread-safe Langfuse attributes added to each span made by an interceptor.
#[derive(Clone, Default)]
pub struct LangfuseContext {
    attributes: Arc<RwLock<HashMap<String, String>>>,
}

impl LangfuseContext {
    /// Create an empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session identifier.
    pub fn set_session_id(&self, session_id: impl Into<String>) -> &Self {
        self.set_attribute(attributes::TRACE_SESSION_ID, session_id)
    }

    /// Set the user identifier.
    pub fn set_user_id(&self, user_id: impl Into<String>) -> &Self {
        self.set_attribute(attributes::TRACE_USER_ID, user_id)
    }

    /// Replace the tags on future spans.
    pub fn add_tags(&self, tags: impl IntoIterator<Item = String>) -> &Self {
        let tags: Vec<String> = tags.into_iter().collect();
        self.set_attribute(
            attributes::TRACE_TAGS,
            serde_json::to_string(&tags).expect("serializing strings cannot fail"),
        )
    }

    /// Add one tag to future spans.
    pub fn add_tag(&self, tag: impl Into<String>) -> &Self {
        let mut attributes = self.attributes.write().unwrap();
        let mut tags = attributes
            .get(attributes::TRACE_TAGS)
            .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
            .unwrap_or_default();
        tags.push(tag.into());
        attributes.insert(
            attributes::TRACE_TAGS.to_string(),
            serde_json::to_string(&tags).expect("serializing strings cannot fail"),
        );
        drop(attributes);
        self
    }

    /// Set the metadata JSON object on future spans.
    pub fn set_metadata(&self, metadata: impl Into<serde_json::Value>) -> &Self {
        self.set_attribute(attributes::TRACE_METADATA, metadata.into().to_string())
    }

    /// Set an arbitrary string attribute on future spans.
    pub fn set_attribute(&self, key: impl Into<String>, value: impl Into<String>) -> &Self {
        self.attributes
            .write()
            .unwrap()
            .insert(key.into(), value.into());
        self
    }

    /// Set the trace name on future spans.
    pub fn set_trace_name(&self, name: impl Into<String>) -> &Self {
        self.set_attribute(attributes::TRACE_NAME, name)
    }

    /// Clear all attributes on future spans.
    pub fn clear(&self) {
        self.attributes.write().unwrap().clear();
    }

    /// Return the current attributes as OpenTelemetry key-value pairs.
    #[must_use]
    pub fn get_attributes(&self) -> Vec<KeyValue> {
        self.attributes
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| KeyValue::new(key.clone(), value.clone()))
            .collect()
    }

    /// Check whether an attribute is set.
    #[must_use]
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.read().unwrap().contains_key(key)
    }

    /// Return an attribute value, if present.
    #[must_use]
    pub fn get_attribute(&self, key: &str) -> Option<String> {
        self.attributes.read().unwrap().get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::{attributes, LangfuseContext};

    #[test]
    fn attributes_survive_clone_and_clear() {
        let context = LangfuseContext::new();
        context.set_session_id("session-1").add_tag("first");
        let cloned = context.clone();
        cloned.add_tag("second");

        assert_eq!(
            context.get_attribute(attributes::TRACE_SESSION_ID),
            Some("session-1".to_string())
        );
        assert_eq!(
            context.get_attribute(attributes::TRACE_TAGS),
            Some(r#"["first","second"]"#.to_string())
        );
        assert_eq!(context.get_attributes().len(), 2);

        cloned.clear();
        assert!(context.get_attributes().is_empty());
    }
}
