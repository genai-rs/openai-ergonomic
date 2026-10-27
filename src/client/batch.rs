//! Batch API client.

use super::{map_api_error, Client};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::batch_api;
use openai_client_base::models::{Batch, CreateBatchRequest, ListBatchesResponse};
use std::time::Instant;

/// Client for batch API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BatchClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(BatchClient<'_, T>);

impl<T: Default + Send + Sync> BatchClient<'_, T> {
    /// Create a new batch job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::batch::{BatchJobBuilder, BatchEndpoint};
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = BatchJobBuilder::new("file-batch-input", BatchEndpoint::ChatCompletions);
    /// let batch = client.batch().create(builder).await?;
    /// println!("Created batch: {}", batch.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: crate::builders::batch::BatchJobBuilder) -> Result<Batch> {
        use openai_client_base::models::create_batch_request::{CompletionWindow, Endpoint};

        // Map our endpoint to the base client enum
        let endpoint = match builder.endpoint() {
            crate::builders::batch::BatchEndpoint::ChatCompletions => {
                Endpoint::SlashV1SlashChatSlashCompletions
            }
            crate::builders::batch::BatchEndpoint::Embeddings => Endpoint::SlashV1SlashEmbeddings,
            crate::builders::batch::BatchEndpoint::Completions => Endpoint::SlashV1SlashCompletions,
        };

        let mut request = CreateBatchRequest::new(
            builder.input_file_id().to_string(),
            endpoint,
            CompletionWindow::Variant24h,
        );

        if builder.has_metadata() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_CREATE;
        let model = "batch";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::create_batch()
            .configuration(&self.client.base_configuration)
            .create_batch_request(request)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// List batch jobs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.batch().list(None, Some(20)).await?;
    /// println!("Found {} batches", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ListBatchesResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_LIST;
        let model = "batch";
        let request_json = format!("{{\"after\":{after:?},\"limit\":{limit:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::list_batches()
            .configuration(&self.client.base_configuration)
            .maybe_after(after)
            .maybe_limit(limit)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Get a specific batch job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let batch = client.batch().get("batch_123").await?;
    /// println!("Batch status: {:?}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, batch_id: impl Into<String>) -> Result<Batch> {
        let id = batch_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_RETRIEVE;
        let model = "batch";
        let request_json = format!("{{\"batch_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::retrieve_batch()
            .configuration(&self.client.base_configuration)
            .batch_id(&id)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Cancel a batch job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let batch = client.batch().cancel("batch_123").await?;
    /// println!("Batch cancelled: {:?}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, batch_id: impl Into<String>) -> Result<Batch> {
        let id = batch_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_CANCEL;
        let model = "batch";
        let request_json = format!("{{\"batch_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::cancel_batch()
            .configuration(&self.client.base_configuration)
            .batch_id(&id)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }
}
