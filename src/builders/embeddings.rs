//! Embeddings API builders.
//!
//! Provides a fluent interface for constructing embedding requests with sensible
//! validation and convenience helpers for common input patterns.

use openai_client_base::models::{
    create_embedding_request::EncodingFormat, CreateEmbeddingRequest, CreateEmbeddingRequestInput,
};

use crate::{Builder, Error, Result};

/// Builder for embedding requests.
#[derive(Debug, Clone)]
pub struct EmbeddingBuilder {
    model: String,
    input: CreateEmbeddingRequestInput,
    encoding_format: Option<EncodingFormat>,
    dimensions: Option<i32>,
    user: Option<String>,
}

impl EmbeddingBuilder {
    /// Create a new builder with the required model identifier and input payload.
    #[must_use]
    pub fn new(model: impl Into<String>, input: impl Into<CreateEmbeddingRequestInput>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    /// Convenience constructor for a single text input.
    #[must_use]
    pub fn from_text(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(model, CreateEmbeddingRequestInput::new_text(text.into()))
    }

    /// Convenience constructor for batched text inputs.
    #[must_use]
    pub fn from_texts<I, S>(model: impl Into<String>, texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let items = texts.into_iter().map(Into::into).collect();
        Self::new(
            model,
            CreateEmbeddingRequestInput::new_arrayofstrings(items),
        )
    }

    /// Replace the embedding input with a single text string.
    #[must_use]
    pub fn input_text(mut self, text: impl Into<String>) -> Self {
        self.input = CreateEmbeddingRequestInput::new_text(text.into());
        self
    }

    /// Replace the input with multiple text strings.
    #[must_use]
    pub fn input_texts<I, S>(mut self, texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.input = CreateEmbeddingRequestInput::new_arrayofstrings(
            texts.into_iter().map(Into::into).collect(),
        );
        self
    }

    /// Replace the input with a single token sequence.
    #[must_use]
    pub fn input_tokens<I>(mut self, tokens: I) -> Self
    where
        I: IntoIterator<Item = i32>,
    {
        self.input = CreateEmbeddingRequestInput::new_arrayofintegers(tokens.into_iter().collect());
        self
    }

    /// Replace the input with batched token sequences.
    #[must_use]
    pub fn input_token_batches<I, J>(mut self, batches: I) -> Self
    where
        I: IntoIterator<Item = J>,
        J: IntoIterator<Item = i32>,
    {
        let items = batches
            .into_iter()
            .map(|batch| batch.into_iter().collect::<Vec<_>>())
            .collect();
        self.input = CreateEmbeddingRequestInput::new_arrayofintegerarrays(items);
        self
    }

    /// Specify the encoding format (`float` or `base64`).
    #[must_use]
    pub fn encoding_format(mut self, format: EncodingFormat) -> Self {
        self.encoding_format = Some(format);
        self
    }

    /// Override the target dimensionality (must be positive).
    #[must_use]
    pub fn dimensions(mut self, dimensions: i32) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Attach an end-user identifier for abuse monitoring.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Borrow the configured model.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Borrow the current input payload.
    #[must_use]
    pub fn input(&self) -> &CreateEmbeddingRequestInput {
        &self.input
    }
}

impl Builder<CreateEmbeddingRequest> for EmbeddingBuilder {
    fn build(self) -> Result<CreateEmbeddingRequest> {
        if let Some(dimensions) = self.dimensions {
            if dimensions <= 0 {
                return Err(Error::InvalidRequest(format!(
                    "Embedding dimensions must be positive (got {dimensions})"
                )));
            }
        }

        Ok(CreateEmbeddingRequest {
            input: Box::new(self.input),
            model: self.model,
            encoding_format: self.encoding_format,
            dimensions: self.dimensions,
            user: self.user,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_embedding_request_from_text() {
        let request = EmbeddingBuilder::from_text("text-embedding-3-small", "hello world")
            .encoding_format(EncodingFormat::Float)
            .build()
            .expect("valid embedding builder");

        assert_eq!(request.model, "text-embedding-3-small");
        matches!(
            *request.input,
            CreateEmbeddingRequestInput::String(ref text) if text == "hello world"
        );
        assert_eq!(request.encoding_format, Some(EncodingFormat::Float));
    }

    #[test]
    fn replaces_input_with_tokens() {
        let request = EmbeddingBuilder::new(
            "text-embedding-3-large",
            CreateEmbeddingRequestInput::new_text("ignored".to_string()),
        )
        .input_tokens([1, 2, 3])
        .dimensions(512)
        .build()
        .expect("valid embedding builder");

        assert_eq!(request.dimensions, Some(512));
        matches!(
            *request.input,
            CreateEmbeddingRequestInput::ArrayOfIntegers(ref values) if values == &[1, 2, 3]
        );
    }

    #[test]
    fn validates_dimensions() {
        let err = EmbeddingBuilder::from_text("text-embedding-3-small", "bad")
            .dimensions(0)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }
}
