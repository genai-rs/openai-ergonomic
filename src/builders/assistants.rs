//! Assistants API builders.
//!
//! This module provides ergonomic builders for `OpenAI` Assistants API operations,
//! including creating assistants, managing threads, messages, and runs.
//!
//! Note: This is a simplified implementation focusing on the most commonly used features.

use serde_json::Value;
use std::collections::HashMap;

use openai_client_base::models::{
    assistant_tools_code, assistant_tools_file_search, assistant_tools_function,
    AssistantTool as BaseAssistantTool, AssistantToolsCode, AssistantToolsFileSearch,
    AssistantToolsFunction, CreateAssistantRequest, CreateAssistantRequestDescription,
    CreateAssistantRequestInstructions, CreateAssistantRequestName, CreateRunRequest,
    FunctionObject,
};

use crate::Builder;

/// Builder for creating a new assistant.
///
/// This builder provides a fluent interface for creating `OpenAI` assistants
/// with commonly used parameters including tool support.
#[derive(Debug, Clone)]
pub struct AssistantBuilder {
    model: String,
    name: Option<String>,
    description: Option<String>,
    instructions: Option<String>,
    tools: Vec<AssistantTool>,
    metadata: HashMap<String, String>,
}

impl AssistantBuilder {
    /// Create a new assistant builder with the specified model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::assistants::{AssistantBuilder, tool_code_interpreter};
    ///
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("My Assistant")
    ///     .instructions("You are a helpful coding assistant.")
    ///     .add_tool(tool_code_interpreter());
    /// ```
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            name: None,
            description: None,
            instructions: None,
            tools: Vec::new(),
            metadata: HashMap::new(),
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

    /// Add tools to the assistant.
    #[must_use]
    pub fn tools(mut self, tools: Vec<AssistantTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Add a single tool to the assistant.
    #[must_use]
    pub fn add_tool(mut self, tool: AssistantTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add metadata to the assistant.
    #[must_use]
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a single metadata key-value pair.
    #[must_use]
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
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

    /// Get the tools for this assistant.
    #[must_use]
    pub fn tools_ref(&self) -> &[AssistantTool] {
        &self.tools
    }

    /// Get the metadata for this assistant.
    #[must_use]
    pub fn metadata_ref(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl Builder<CreateAssistantRequest> for AssistantBuilder {
    fn build(self) -> crate::Result<CreateAssistantRequest> {
        let Self {
            model,
            name,
            description,
            instructions,
            tools,
            metadata,
        } = self;

        let mut request = CreateAssistantRequest::new(model);

        if let Some(name) = name {
            request.name = Some(Box::new(CreateAssistantRequestName::from(name)));
        }

        if let Some(description) = description {
            request.description = Some(Box::new(CreateAssistantRequestDescription::from(
                description,
            )));
        }

        if let Some(instructions) = instructions {
            request.instructions = Some(Box::new(CreateAssistantRequestInstructions::from(
                instructions,
            )));
        }

        if !tools.is_empty() {
            request.tools = Some(tools.into_iter().map(convert_tool).collect());
        }

        if metadata.is_empty() {
            request.metadata = None;
        } else {
            request.metadata = Some(Some(metadata));
        }

        Ok(request)
    }
}

/// Represents a tool that can be used by an assistant.
#[derive(Debug, Clone)]
pub enum AssistantTool {
    /// Code interpreter tool for executing Python code.
    CodeInterpreter,
    /// File search tool for searching through uploaded files.
    FileSearch,
    /// Function calling tool with custom function definition.
    Function {
        /// The name of the function.
        name: String,
        /// A description of what the function does.
        description: String,
        /// The JSON schema that describes the function parameters.
        parameters: Value,
    },
}

fn convert_tool(tool: AssistantTool) -> BaseAssistantTool {
    match tool {
        AssistantTool::CodeInterpreter => BaseAssistantTool::SCode(Box::new(
            AssistantToolsCode::new(assistant_tools_code::Type::CodeInterpreter),
        )),
        AssistantTool::FileSearch => BaseAssistantTool::SFileSearch(Box::new(
            AssistantToolsFileSearch::new(assistant_tools_file_search::Type::FileSearch),
        )),
        AssistantTool::Function {
            name,
            description,
            parameters,
        } => {
            use std::collections::HashMap;

            let params_map = if let serde_json::Value::Object(map) = parameters {
                map.into_iter()
                    .collect::<HashMap<String, serde_json::Value>>()
            } else {
                HashMap::new()
            };

            let mut function = FunctionObject::new(name);
            function.description = Some(description);
            if !params_map.is_empty() {
                function.parameters = Some(params_map);
            }

            BaseAssistantTool::SFunction(Box::new(AssistantToolsFunction::new(
                assistant_tools_function::Type::Function,
                function,
            )))
        }
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

impl Builder<CreateRunRequest> for RunBuilder {
    fn build(self) -> crate::Result<CreateRunRequest> {
        let Self {
            assistant_id,
            model,
            instructions,
            temperature,
            stream,
            metadata,
        } = self;

        let mut request = CreateRunRequest::new(assistant_id);
        request.model = model;
        request.instructions = instructions;
        if !metadata.is_empty() {
            request.metadata = Some(Some(metadata));
        }
        if let Some(temp) = temperature {
            request.temperature = Some(temp);
        }
        if stream {
            request.stream = Some(true);
        }

        Ok(request)
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

/// Helper function to create a code interpreter tool.
///
/// The code interpreter tool allows assistants to execute Python code,
/// perform calculations, data analysis, and generate visualizations.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::assistants::{AssistantBuilder, tool_code_interpreter};
///
/// let assistant = AssistantBuilder::new("gpt-4")
///     .name("Math Assistant")
///     .add_tool(tool_code_interpreter());
/// ```
#[must_use]
pub fn tool_code_interpreter() -> AssistantTool {
    AssistantTool::CodeInterpreter
}

/// Helper function to create a file search tool.
///
/// The file search tool allows assistants to search through uploaded files
/// and vector stores to provide relevant information from documents.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::assistants::{AssistantBuilder, tool_file_search};
///
/// let assistant = AssistantBuilder::new("gpt-4")
///     .name("Research Assistant")
///     .add_tool(tool_file_search());
/// ```
#[must_use]
pub fn tool_file_search() -> AssistantTool {
    AssistantTool::FileSearch
}

/// Helper function to create a custom function tool.
///
/// Function tools allow assistants to call custom functions that you define,
/// enabling integration with external APIs and custom business logic.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::assistants::{AssistantBuilder, tool_function};
/// use serde_json::json;
///
/// let fibonacci_tool = tool_function(
///     "calculate_fibonacci",
///     "Calculate the nth Fibonacci number",
///     json!({
///         "type": "object",
///         "properties": {
///             "n": {
///                 "type": "integer",
///                 "description": "The position in the Fibonacci sequence"
///             }
///         },
///         "required": ["n"]
///     })
/// );
///
/// let assistant = AssistantBuilder::new("gpt-4")
///     .name("Math Assistant")
///     .add_tool(fibonacci_tool);
/// ```
#[must_use]
pub fn tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: Value,
) -> AssistantTool {
    AssistantTool::Function {
        name: name.into(),
        description: description.into(),
        parameters,
    }
}

/// Helper function to create an assistant with code interpreter tool.
#[must_use]
pub fn assistant_with_code_interpreter(
    model: impl Into<String>,
    name: impl Into<String>,
) -> AssistantBuilder {
    AssistantBuilder::new(model)
        .name(name)
        .add_tool(tool_code_interpreter())
}

/// Helper function to create an assistant with file search tool.
#[must_use]
pub fn assistant_with_file_search(
    model: impl Into<String>,
    name: impl Into<String>,
) -> AssistantBuilder {
    AssistantBuilder::new(model)
        .name(name)
        .add_tool(tool_file_search())
}

/// Helper function to create an assistant with both code interpreter and file search tools.
#[must_use]
pub fn assistant_with_tools(model: impl Into<String>, name: impl Into<String>) -> AssistantBuilder {
    AssistantBuilder::new(model)
        .name(name)
        .add_tool(tool_code_interpreter())
        .add_tool(tool_file_search())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client_base::models;

    #[test]
    fn test_assistant_builder() {
        let request = AssistantBuilder::new("gpt-4")
            .name("Test Assistant")
            .description("A test assistant")
            .instructions("You are a helpful assistant")
            .add_metadata("team", "core")
            .add_tool(tool_code_interpreter())
            .build()
            .expect("assistant builder should succeed");

        assert_eq!(request.model, "gpt-4");
        assert!(request
            .name
            .as_deref()
            .is_some_and(|name| matches!(name, models::create_assistant_request_name::CreateAssistantRequestName::Text(ref value) if value == "Test Assistant")));
        assert!(request
            .description
            .as_deref()
            .is_some_and(|desc| matches!(desc, models::create_assistant_request_description::CreateAssistantRequestDescription::Text(ref value) if value == "A test assistant")));
        assert!(request.instructions.is_some());
        assert!(request
            .tools
            .as_ref()
            .is_some_and(|tools| matches!(tools.first(), Some(models::AssistantTool::SCode(_)))));
        assert!(request
            .metadata
            .as_ref()
            .is_some_and(|meta| meta.as_ref().is_some()));
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
        let request = RunBuilder::new("assistant-123")
            .model("gpt-4")
            .instructions("Follow these instructions")
            .temperature(0.7)
            .stream(true)
            .metadata("key", "value")
            .build()
            .expect("run builder should succeed");

        assert_eq!(request.assistant_id, "assistant-123");
        assert_eq!(request.model.as_deref(), Some("gpt-4"));
        assert_eq!(
            request.instructions.as_deref(),
            Some("Follow these instructions")
        );
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stream, Some(true));
        assert!(request
            .metadata
            .as_ref()
            .is_some_and(|meta| meta.as_ref().is_some()));
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

    #[test]
    fn test_assistant_builder_with_tools() {
        let builder = AssistantBuilder::new("gpt-4")
            .name("Tool Assistant")
            .add_tool(tool_code_interpreter())
            .add_tool(tool_file_search())
            .add_metadata("version", "1.0");

        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Tool Assistant"));
        assert_eq!(builder.tools_ref().len(), 2);
        assert_eq!(builder.metadata_ref().len(), 1);

        // Check tool types
        match &builder.tools_ref()[0] {
            AssistantTool::CodeInterpreter => {}
            _ => panic!("Expected CodeInterpreter tool"),
        }

        match &builder.tools_ref()[1] {
            AssistantTool::FileSearch => {}
            _ => panic!("Expected FileSearch tool"),
        }
    }

    #[test]
    fn test_tool_function() {
        use serde_json::json;

        let tool = tool_function(
            "test_function",
            "A test function",
            json!({"type": "object", "properties": {}}),
        );

        match tool {
            AssistantTool::Function {
                name,
                description,
                parameters,
            } => {
                assert_eq!(name, "test_function");
                assert_eq!(description, "A test function");
                assert!(parameters.is_object());
            }
            _ => panic!("Expected Function tool"),
        }
    }

    #[test]
    fn test_tool_helpers() {
        let code_tool = tool_code_interpreter();
        match code_tool {
            AssistantTool::CodeInterpreter => {}
            _ => panic!("Expected CodeInterpreter tool"),
        }

        let search_tool = tool_file_search();
        match search_tool {
            AssistantTool::FileSearch => {}
            _ => panic!("Expected FileSearch tool"),
        }
    }

    #[test]
    fn test_assistant_with_code_interpreter_helper() {
        let builder = assistant_with_code_interpreter("gpt-4", "Code Helper");
        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Code Helper"));
        assert_eq!(builder.tools_ref().len(), 1);

        match &builder.tools_ref()[0] {
            AssistantTool::CodeInterpreter => {}
            _ => panic!("Expected CodeInterpreter tool"),
        }
    }

    #[test]
    fn test_assistant_with_file_search_helper() {
        let builder = assistant_with_file_search("gpt-4", "Search Helper");
        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Search Helper"));
        assert_eq!(builder.tools_ref().len(), 1);

        match &builder.tools_ref()[0] {
            AssistantTool::FileSearch => {}
            _ => panic!("Expected FileSearch tool"),
        }
    }

    #[test]
    fn test_assistant_with_tools_helper() {
        let builder = assistant_with_tools("gpt-4", "Multi-Tool Helper");
        assert_eq!(builder.model(), "gpt-4");
        assert_eq!(builder.name_ref(), Some("Multi-Tool Helper"));
        assert_eq!(builder.tools_ref().len(), 2);

        match &builder.tools_ref()[0] {
            AssistantTool::CodeInterpreter => {}
            _ => panic!("Expected CodeInterpreter tool"),
        }

        match &builder.tools_ref()[1] {
            AssistantTool::FileSearch => {}
            _ => panic!("Expected FileSearch tool"),
        }
    }
}
