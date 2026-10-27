//! Fine tuning API client.

use super::{map_api_error, Client};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::fine_tuning_api;
use openai_client_base::models::{
    CreateFineTuningJobRequest, FineTuningJob, ListFineTuningJobCheckpointsResponse,
    ListFineTuningJobEventsResponse, ListPaginatedFineTuningJobsResponse,
};
use std::time::Instant;

/// Client for fine-tuning API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FineTuningClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(FineTuningClient<'_, T>);

impl<T: Default + Send + Sync> FineTuningClient<'_, T> {
    /// Create a new fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::fine_tuning::FineTuningJobBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training-data");
    /// let job = client.fine_tuning().create_job(builder).await?;
    /// println!("Created job: {}", job.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_job(
        &self,
        builder: crate::builders::fine_tuning::FineTuningJobBuilder,
    ) -> Result<FineTuningJob> {
        let mut request = CreateFineTuningJobRequest::new(
            builder.model().to_string(),
            builder.training_file().to_string(),
        );

        if let Some(validation_file) = builder.validation_file_ref() {
            request.validation_file = Some(Some(validation_file.to_string()));
        }

        if let Some(suffix) = builder.suffix_ref() {
            request.suffix = Some(Some(suffix.to_string()));
        }

        // Note: Hyperparameters handling is limited due to base client API limitations
        // The generated API appears to have empty struct definitions for hyperparameters
        // For now, we skip hyperparameters configuration
        // TODO: Update when openai-client-base fixes hyperparameters types

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_CREATE;
        let model = builder.model();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::create_fine_tuning_job()
            .configuration(&self.client.base_configuration)
            .create_fine_tuning_job_request(request)
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

    /// List fine-tuning jobs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.fine_tuning().list_jobs(None, Some(20)).await?;
    /// println!("Found {} jobs", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_jobs(
        &self,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ListPaginatedFineTuningJobsResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_LIST;
        let model = "fine-tuning";
        let request_json = format!("{{\"after\":{after:?},\"limit\":{limit:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::list_paginated_fine_tuning_jobs()
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

    /// Get a specific fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let job = client.fine_tuning().get_job("ftjob-123").await?;
    /// println!("Job status: {:?}", job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_job(&self, job_id: impl Into<String>) -> Result<FineTuningJob> {
        let id = job_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_RETRIEVE;
        let model = "fine-tuning";
        let request_json = format!("{{\"job_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::retrieve_fine_tuning_job()
            .configuration(&self.client.base_configuration)
            .fine_tuning_job_id(&id)
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

    /// Cancel a fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let job = client.fine_tuning().cancel_job("ftjob-123").await?;
    /// println!("Job cancelled: {:?}", job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_job(&self, job_id: impl Into<String>) -> Result<FineTuningJob> {
        let id = job_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_CANCEL;
        let model = "fine-tuning";
        let request_json = format!("{{\"job_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::cancel_fine_tuning_job()
            .configuration(&self.client.base_configuration)
            .fine_tuning_job_id(&id)
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

    /// List events for a fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let events = client.fine_tuning().list_events("ftjob-123", None, Some(20)).await?;
    /// println!("Found {} events", events.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_events(
        &self,
        job_id: impl Into<String>,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ListFineTuningJobEventsResponse> {
        let id = job_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_LIST_EVENTS;
        let model = "fine-tuning";
        let request_json =
            format!("{{\"job_id\":\"{id}\",\"after\":{after:?},\"limit\":{limit:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::list_fine_tuning_events()
            .configuration(&self.client.base_configuration)
            .fine_tuning_job_id(&id)
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

    /// List checkpoints for a fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let checkpoints = client.fine_tuning().list_checkpoints("ftjob-123", None, Some(10)).await?;
    /// println!("Found {} checkpoints", checkpoints.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_checkpoints(
        &self,
        job_id: impl Into<String>,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ListFineTuningJobCheckpointsResponse> {
        let id = job_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FINE_TUNING_LIST_CHECKPOINTS;
        let model = "fine-tuning";
        let request_json =
            format!("{{\"job_id\":\"{id}\",\"after\":{after:?},\"limit\":{limit:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match fine_tuning_api::list_fine_tuning_job_checkpoints()
            .configuration(&self.client.base_configuration)
            .fine_tuning_job_id(&id)
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
}
