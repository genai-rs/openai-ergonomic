//! Assistants API builders.
//!
//! This module provides ergonomic builders for OpenAI Assistants API operations,
//! including creating assistants, managing threads, messages, and runs.
//!
//! Note: This is a simplified implementation focusing on the most commonly used features.

use std::collections::HashMap;

/// Builder for creating a new assistant.
///
/// This builder provides a fluent interface for creating OpenAI assistants
/// with commonly used parameters.
#[derive(Debug, Clone)]
pub struct AssistantBuilder {
    model: String,
    name: Option<String>,
    description: Option<String>,
    instructions: Option<String>,
}

impl AssistantBuilder {
    /// Create a new assistant builder with the specified model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::assistants::AssistantBuilder;
    ///
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("My Assistant")
    ///     .instructions("You are a helpful coding assistant.");
    /// ```
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            name: None,
            description: None,
            instructions: None,
        }
    }

    /// Set the assistant's name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the assistant's description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the assistant's instructions (system prompt).
    #[must_use]
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Get the model for this assistant.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the name for this assistant.
    #[must_use]
    pub fn name_ref(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the description for this assistant.
    #[must_use]
    pub fn description_ref(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the instructions for this assistant.
    #[must_use]
    pub fn instructions_ref(&self) -> Option<&str> {
        self.instructions.as_deref()
    }
}

/// Builder for creating a thread.
#[derive(Debug, Clone, Default)]
pub struct ThreadBuilder {
    metadata: HashMap<String, String>,
}

impl ThreadBuilder {
    /// Create a new thread builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata to the thread.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the metadata for this thread.
    #[must_use]
    pub fn metadata_ref(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

/// Builder for creating a run.
#[derive(Debug, Clone)]
pub struct RunBuilder {
    assistant_id: String,
    model: Option<String>,
    instructions: Option<String>,
    temperature: Option<f64>,
    stream: bool,
    metadata: HashMap<String, String>,
}

impl RunBuilder {
    /// Create a new run builder with the specified assistant ID.
    #[must_use]
    pub fn new(assistant_id: impl Into<String>) -> Self {
        Self {
            assistant_id: assistant_id.into(),
            model: None,
            instructions: None,
            temperature: None,
            stream: false,
            metadata: HashMap::new(),
        }
    }

    /// Override the assistant's model for this run.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Override the assistant's instructions for this run.
    #[must_use]
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Set the temperature for this run.
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Enable streaming for this run.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Add metadata to the run.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the assistant ID for this run.
    #[must_use]
    pub fn assistant_id(&self) -> &str {
        &self.assistant_id
    }

    /// Get the model override for this run.
    #[must_use]
    pub fn model_ref(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Get the instructions override for this run.
    #[must_use]
    pub fn instructions_ref(&self) -> Option<&str> {
        self.instructions.as_deref()
    }

    /// Get the temperature for this run.
    #[must_use]
    pub fn temperature_ref(&self) -> Option<f64> {
        self.temperature
    }

    /// Check if streaming is enabled for this run.
    #[must_use]
    pub fn is_streaming(&self) -> bool {
        self.stream
    }

    /// Get the metadata for this run.
    #[must_use]
    pub fn metadata_ref(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

/// Helper function to create a simple assistant with just a model and name.
#[must_use]
pub fn simple_assistant(model: impl Into<String>, name: impl Into<String>) -> AssistantBuilder {
    AssistantBuilder::new(model).name(name)
}

/// Helper function to create an assistant with instructions.
#[must_use]
pub fn assistant_with_instructions(
    model: impl Into<String>,
    name: impl Into<String>,
    instructions: impl Into<String>,
) -> AssistantBuilder {
    AssistantBuilder::new(model)
        .name(name)
        .instructions(instructions)
}

/// Helper function to create a new thread.
#[must_use]
pub fn simple_thread() -> ThreadBuilder {
    ThreadBuilder::new()
}

/// Helper function to create a simple run.
#[must_use]
pub fn simple_run(assistant_id: impl Into<String>) -> RunBuilder {
    RunBuilder::new(assistant_id)
}

/// Helper function to create a streaming run.
#[must_use]
pub fn streaming_run(assistant_id: impl Into<String>) -> RunBuilder {
    RunBuilder::new(assistant_id).stream(true)
}

/// Helper function to create a run with custom temperature.
#[must_use]
pub fn temperature_run(assistant_id: impl Into<String>, temperature: f64) -> RunBuilder {
    RunBuilder::new(assistant_id).temperature(temperature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_builder() {
        let builder = AssistantBuilder::new("gpt-4")
            .name("Test Assistant")
            .description("A test assistant")
            .instructions("You are a helpful assistant");

        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Test Assistant"));
        assert_eq!(builder.description_ref(), Some("A test assistant"));
        assert_eq!(
            builder.instructions_ref(),
            Some("You are a helpful assistant")
        );
    }

    #[test]
    fn test_thread_builder() {
        let builder = ThreadBuilder::new()
            .metadata("key1", "value1")
            .metadata("key2", "value2");

        assert_eq!(builder.metadata_ref().len(), 2);
        assert_eq!(
            builder.metadata_ref().get("key1"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            builder.metadata_ref().get("key2"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn test_run_builder() {
        let builder = RunBuilder::new("assistant-123")
            .model("gpt-4")
            .instructions("Follow these instructions")
            .temperature(0.7)
            .stream(true)
            .metadata("key", "value");

        assert_eq!(builder.assistant_id(), "assistant-123");
        assert_eq!(builder.model_ref(), Some("gpt-4"));
        assert_eq!(
            builder.instructions_ref(),
            Some("Follow these instructions")
        );
        assert_eq!(builder.temperature_ref(), Some(0.7));
        assert!(builder.is_streaming());
        assert_eq!(builder.metadata_ref().len(), 1);
    }

    #[test]
    fn test_simple_assistant_helper() {
        let builder = simple_assistant("gpt-4", "Helper");
        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Helper"));
    }

    #[test]
    fn test_assistant_with_instructions_helper() {
        let builder = assistant_with_instructions("gpt-4", "Helper", "Be helpful");
        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Helper"));
        assert_eq!(builder.instructions_ref(), Some("Be helpful"));
    }

    #[test]
    fn test_simple_thread_helper() {
        let builder = simple_thread();
        assert!(builder.metadata_ref().is_empty());
    }

    #[test]
    fn test_simple_run_helper() {
        let builder = simple_run("assistant-123");
        assert_eq!(builder.assistant_id(), "assistant-123");
        assert!(!builder.is_streaming());
    }

    #[test]
    fn test_streaming_run_helper() {
        let builder = streaming_run("assistant-123");
        assert_eq!(builder.assistant_id(), "assistant-123");
        assert!(builder.is_streaming());
    }

    #[test]
    fn test_temperature_run_helper() {
        let builder = temperature_run("assistant-123", 0.8);
        assert_eq!(builder.assistant_id(), "assistant-123");
        assert_eq!(builder.temperature_ref(), Some(0.8));
    }
}
