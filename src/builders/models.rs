//! Models API builders.
//!
//! Provides high-level builders for working with OpenAI models, including
//! listing available models, retrieving model details, and deleting fine-tuned models.

/// Builder for retrieving a specific model by ID.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::models::ModelRetrievalBuilder;
///
/// let builder = ModelRetrievalBuilder::new("gpt-4");
/// assert_eq!(builder.model_id(), "gpt-4");
/// ```
#[derive(Debug, Clone)]
pub struct ModelRetrievalBuilder {
    model_id: String,
}

impl ModelRetrievalBuilder {
    /// Create a new model retrieval builder for the specified model ID.
    #[must_use]
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
        }
    }

    /// Get the model ID.
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Builder for deleting a fine-tuned model.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::models::ModelDeleteBuilder;
///
/// let builder = ModelDeleteBuilder::new("ft:gpt-3.5-turbo:my-org:custom-suffix:id");
/// assert_eq!(builder.model_id(), "ft:gpt-3.5-turbo:my-org:custom-suffix:id");
/// ```
#[derive(Debug, Clone)]
pub struct ModelDeleteBuilder {
    model_id: String,
}

impl ModelDeleteBuilder {
    /// Create a new model delete builder for the specified model ID.
    ///
    /// Note: You must have the Owner role in your organization to delete a model.
    #[must_use]
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
        }
    }

    /// Get the model ID.
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_retrieval_builder() {
        let builder = ModelRetrievalBuilder::new("gpt-4");
        assert_eq!(builder.model_id(), "gpt-4");
    }

    #[test]
    fn test_model_retrieval_builder_with_string() {
        let model = "gpt-3.5-turbo".to_string();
        let builder = ModelRetrievalBuilder::new(model);
        assert_eq!(builder.model_id(), "gpt-3.5-turbo");
    }

    #[test]
    fn test_model_delete_builder() {
        let builder = ModelDeleteBuilder::new("ft:gpt-3.5-turbo:my-org:custom:id");
        assert_eq!(builder.model_id(), "ft:gpt-3.5-turbo:my-org:custom:id");
    }

    #[test]
    fn test_model_delete_builder_with_string() {
        let model = "ft:gpt-4:org:suffix:123".to_string();
        let builder = ModelDeleteBuilder::new(model);
        assert_eq!(builder.model_id(), "ft:gpt-4:org:suffix:123");
    }
}
