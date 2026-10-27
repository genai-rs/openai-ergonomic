//! Embeddings API client.

use super::{map_api_error, Client};
use crate::builders::embeddings::EmbeddingsBuilder;
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::embeddings_api;
use openai_client_base::models::CreateEmbeddingResponse;
use std::time::Instant;

/// Client for embeddings API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct EmbeddingsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(EmbeddingsClient<'_, T>);

impl<T: Default + Send + Sync> EmbeddingsClient<'_, T> {
    /// Start a builder for creating embeddings requests with the given model.
    #[must_use]
    pub fn builder(&self, model: impl Into<String>) -> EmbeddingsBuilder {
        EmbeddingsBuilder::new(model)
    }

    /// Convenience helper for embedding a single string input.
    #[must_use]
    pub fn text(&self, model: impl Into<String>, input: impl Into<String>) -> EmbeddingsBuilder {
        self.builder(model).input_text(input)
    }

    /// Convenience helper for embedding a single tokenized input.
    #[must_use]
    pub fn tokens<I>(&self, model: impl Into<String>, tokens: I) -> EmbeddingsBuilder
    where
        I: IntoIterator<Item = i32>,
    {
        self.builder(model).input_tokens(tokens)
    }

    /// Execute an embeddings request built with [`EmbeddingsBuilder`].
    pub async fn create(&self, builder: EmbeddingsBuilder) -> Result<CreateEmbeddingResponse> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::EMBEDDINGS;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match embeddings_api::create_embedding()
            .configuration(&self.client.base_configuration)
            .create_embedding_request(request)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            duration,
            Some(i64::from(response.usage.prompt_tokens)),
            Some(i64::from(response.usage.total_tokens)),
        )
        .await;

        Ok(response)
    }
}
