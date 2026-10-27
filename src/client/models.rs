//! Models API client.

use super::{map_api_error, Client};
use crate::builders::models::{ModelDeleteBuilder, ModelRetrievalBuilder};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::models_api;
use openai_client_base::models::{DeleteModelResponse, ListModelsResponse, Model};
use std::time::Instant;

/// Client for models API.
#[derive(Debug, Clone, Copy)]
pub struct ModelsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(ModelsClient<'_, T>);

impl<T: Default + Send + Sync> ModelsClient<'_, T> {
    /// List all available models.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let models = client.models().list().await?;
    /// println!("Available models: {}", models.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Result<ListModelsResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MODEL_LIST;
        let model = "models";
        let request_json = "{}".to_string();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match models_api::list_models()
            .configuration(&self.client.base_configuration)
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

    /// Retrieve information about a specific model.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let model = client.models().get("gpt-4").await?;
    /// println!("Model ID: {}", model.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, model_id: impl Into<String>) -> Result<Model> {
        let id = model_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MODEL_RETRIEVE;
        let model = "models";
        let request_json = format!("{{\"model_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match models_api::retrieve_model()
            .configuration(&self.client.base_configuration)
            .model(&id)
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

    /// Retrieve information about a model using a builder.
    pub async fn retrieve(&self, builder: ModelRetrievalBuilder) -> Result<Model> {
        self.get(builder.model_id()).await
    }

    /// Delete a fine-tuned model.
    ///
    /// You must have the Owner role in your organization to delete a model.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.models().delete("ft:gpt-3.5-turbo:my-org:custom:id").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, model_id: impl Into<String>) -> Result<DeleteModelResponse> {
        let id = model_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MODEL_DELETE;
        let model = "models";
        let request_json = format!("{{\"model_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match models_api::delete_model()
            .configuration(&self.client.base_configuration)
            .model(&id)
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

    /// Delete a fine-tuned model using a builder.
    pub async fn remove(&self, builder: ModelDeleteBuilder) -> Result<DeleteModelResponse> {
        self.delete(builder.model_id()).await
    }
}
