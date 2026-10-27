//! Usage API client.

use super::{map_api_error, Client};
use crate::builders::usage::UsageBuilder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::usage_api;
use openai_client_base::models::UsageResponse;
use std::time::Instant;

/// Client for usage API.
#[derive(Debug, Clone, Copy)]
pub struct UsageClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(UsageClient<'_, T>);

impl<T: Default + Send + Sync> UsageClient<'_, T> {
    /// Get usage data for audio speeches.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::usage::UsageBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = UsageBuilder::new(1704067200, None);
    /// let usage = client.usage().audio_speeches(builder).await?;
    /// println!("Usage: {:?}", usage);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn audio_speeches(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_AUDIO_SPEECHES;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_audio_speeches()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for audio transcriptions.
    pub async fn audio_transcriptions(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_AUDIO_TRANSCRIPTIONS;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_audio_transcriptions()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for code interpreter sessions.
    pub async fn code_interpreter_sessions(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_CODE_INTERPRETER;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_code_interpreter_sessions()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for completions.
    pub async fn completions(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_COMPLETIONS;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_completions()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for embeddings.
    pub async fn embeddings(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_EMBEDDINGS;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_embeddings()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for images.
    pub async fn images(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_IMAGES;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_images()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for moderations.
    pub async fn moderations(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_MODERATIONS;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_moderations()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_user_ids(builder.user_ids_option())
            .maybe_api_key_ids(builder.api_key_ids_option())
            .maybe_models(builder.models_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get usage data for vector stores.
    pub async fn vector_stores(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_VECTOR_STORES;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_vector_stores()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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

    /// Get cost data.
    pub async fn costs(&self, builder: UsageBuilder) -> Result<UsageResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::USAGE_COSTS;
        let model = "usage";
        let start_time = builder.start_time();
        let request_json = format!("{{\"start_time\":{start_time}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match usage_api::usage_costs()
            .configuration(&self.client.base_configuration)
            .start_time(builder.start_time())
            .maybe_end_time(builder.end_time())
            .maybe_bucket_width(builder.bucket_width_str())
            .maybe_project_ids(builder.project_ids_option())
            .maybe_group_by(builder.group_by_option())
            .maybe_limit(builder.limit_ref())
            .maybe_page(builder.page_ref())
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
