//! Embeddings API builders.
//!
//! Provides high-level builders for creating `OpenAI` embeddings requests
//! covering text inputs, tokenized inputs, and configuration options such as
//! encoding format and dimensionality.

use openai_client_base::models::{
    create_embedding_request::EncodingFormat, CreateEmbeddingRequest, CreateEmbeddingRequestInput,
};

use crate::{Builder, Error, Result};

/// Types of input supported by the embeddings endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingInput {
    /// A single string to embed.
    Text(String),
    /// Multiple strings to embed in one request.
    TextArray(Vec<String>),
    /// A single tokenized input represented as integers.
    Tokens(Vec<i32>),
    /// Multiple tokenized inputs.
    TokensBatch(Vec<Vec<i32>>),
}

impl EmbeddingInput {
    fn into_request_input(self) -> CreateEmbeddingRequestInput {
        match self {
            Self::Text(value) => CreateEmbeddingRequestInput::new_text(value),
            Self::TextArray(values) => CreateEmbeddingRequestInput::new_arrayofstrings(values),
            Self::Tokens(values) => CreateEmbeddingRequestInput::new_arrayofintegers(values),
            Self::TokensBatch(values) => {
                CreateEmbeddingRequestInput::new_arrayofintegerarrays(values)
            }
        }
    }
}

/// Builder for creating embedding requests.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::{Builder, EmbeddingsBuilder};
///
/// let request = EmbeddingsBuilder::new("text-embedding-3-small")
///     .input_text("hello world")
///     .dimensions(256)
///     .build()
///     .unwrap();
///
/// assert_eq!(request.model, "text-embedding-3-small");
/// assert_eq!(request.dimensions, Some(256));
/// ```
#[derive(Debug, Clone)]
pub struct EmbeddingsBuilder {
    model: String,
    input: Option<EmbeddingInput>,
    encoding_format: Option<EncodingFormat>,
    dimensions: Option<i32>,
    user: Option<String>,
}

impl EmbeddingsBuilder {
    /// Create a new embeddings builder for the specified model.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            input: None,
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    /// Provide the request input explicitly.
    #[must_use]
    pub fn input(mut self, input: EmbeddingInput) -> Self {
        self.input = Some(input);
        self
    }

    /// Embed a single string input.
    #[must_use]
    pub fn input_text(mut self, text: impl Into<String>) -> Self {
        self.input = Some(EmbeddingInput::Text(text.into()));
        self
    }

    /// Embed multiple string inputs in one request.
    #[must_use]
    pub fn input_texts<I, S>(mut self, texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let collected = texts.into_iter().map(Into::into).collect();
        self.input = Some(EmbeddingInput::TextArray(collected));
        self
    }

    /// Embed a single tokenized input.
    #[must_use]
    pub fn input_tokens<I>(mut self, tokens: I) -> Self
    where
        I: IntoIterator<Item = i32>,
    {
        self.input = Some(EmbeddingInput::Tokens(tokens.into_iter().collect()));
        self
    }

    /// Embed multiple tokenized inputs.
    #[must_use]
    pub fn input_token_batches<I, J>(mut self, batches: I) -> Self
    where
        I: IntoIterator<Item = J>,
        J: IntoIterator<Item = i32>,
    {
        let collected = batches
            .into_iter()
            .map(|batch| batch.into_iter().collect())
            .collect();
        self.input = Some(EmbeddingInput::TokensBatch(collected));
        self
    }

    /// Set the encoding format for the embeddings response.
    #[must_use]
    pub fn encoding_format(mut self, format: EncodingFormat) -> Self {
        self.encoding_format = Some(format);
        self
    }

    /// Set the output dimensions for supported models.
    #[must_use]
    pub fn dimensions(mut self, dimensions: i32) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Associate a user identifier with the request.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Access the configured model name.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Access the configured input, if set.
    #[must_use]
    pub fn input_ref(&self) -> Option<&EmbeddingInput> {
        self.input.as_ref()
    }

    /// Access the configured encoding format, if set.
    #[must_use]
    pub fn encoding_format_ref(&self) -> Option<EncodingFormat> {
        self.encoding_format
    }

    /// Access the configured dimensions, if set.
    #[must_use]
    pub fn dimensions_ref(&self) -> Option<i32> {
        self.dimensions
    }

    /// Access the configured user identifier, if set.
    #[must_use]
    pub fn user_ref(&self) -> Option<&str> {
        self.user.as_deref()
    }

    fn validate(&self) -> Result<()> {
        if let Some(dimensions) = self.dimensions {
            if dimensions <= 0 {
                return Err(Error::InvalidRequest(
                    "Embedding dimensions must be positive".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Builder<CreateEmbeddingRequest> for EmbeddingsBuilder {
    fn build(self) -> Result<CreateEmbeddingRequest> {
        self.validate()?;

        let Self {
            model,
            input,
            encoding_format,
            dimensions,
            user,
        } = self;

        let request_input = input
            .ok_or_else(|| Error::InvalidRequest("Embeddings input is required".to_string()))?
            .into_request_input();

        let mut request = CreateEmbeddingRequest::new(request_input, model);
        request.encoding_format = encoding_format;
        request.dimensions = dimensions;
        request.user = user;

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_text_input_request() {
        let builder = EmbeddingsBuilder::new("text-embedding-3-small").input_text("hello world");
        let request = builder.build().expect("builder should succeed");

        assert_eq!(request.model, "text-embedding-3-small");
        assert!(matches!(
            *request.input,
            CreateEmbeddingRequestInput::String(ref value) if value == "hello world"
        ));
    }

    #[test]
    fn builds_multiple_texts_request() {
        let builder =
            EmbeddingsBuilder::new("text-embedding-3-large").input_texts(["foo", "bar", "baz"]);
        let request = builder.build().expect("builder should succeed");

        match *request.input {
            CreateEmbeddingRequestInput::ArrayOfStrings(values) => {
                assert_eq!(values, vec!["foo", "bar", "baz"]);
            }
            other => panic!("unexpected input variant: {other:?}"),
        }
    }

    #[test]
    fn builds_token_batch_request() {
        let builder = EmbeddingsBuilder::new("text-embedding-3-small")
            .input_token_batches([vec![1, 2, 3], vec![4, 5, 6]]);
        let request = builder.build().expect("builder should succeed");

        match *request.input {
            CreateEmbeddingRequestInput::ArrayOfIntegerArrays(values) => {
                assert_eq!(values, vec![vec![1, 2, 3], vec![4, 5, 6]]);
            }
            other => panic!("unexpected input variant: {other:?}"),
        }
    }

    #[test]
    fn validates_dimensions_positive() {
        let builder = EmbeddingsBuilder::new("text-embedding-3-small")
            .input_text("test")
            .dimensions(0);
        let error = builder.build().expect_err("dimensions should be validated");
        assert!(matches!(error, Error::InvalidRequest(message) if message.contains("positive")));
    }

    #[test]
    fn requires_input() {
        let builder = EmbeddingsBuilder::new("text-embedding-3-small");
        let error = builder.build().expect_err("input is required");
        assert!(matches!(error, Error::InvalidRequest(message) if message.contains("input")));
    }

    #[test]
    fn propagates_encoding_and_user() {
        let builder = EmbeddingsBuilder::new("text-embedding-3-small")
            .input_text("hello")
            .encoding_format(EncodingFormat::Base64)
            .dimensions(512)
            .user("user-123");
        let request = builder.build().expect("builder should succeed");

        assert_eq!(request.encoding_format, Some(EncodingFormat::Base64));
        assert_eq!(request.dimensions, Some(512));
        assert_eq!(request.user.as_deref(), Some("user-123"));
    }
}
