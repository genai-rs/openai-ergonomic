//! Completions API client.

use super::{map_api_error, Client};
use crate::builders::completions::CompletionsBuilder;
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::completions_api;
use openai_client_base::models::CreateCompletionResponse;
use std::time::Instant;

/// Client for completions API.
#[derive(Debug, Clone, Copy)]
pub struct CompletionsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(CompletionsClient<'_, T>);

impl<T: Default + Send + Sync> CompletionsClient<'_, T> {
    /// Create a completions builder for the specified model.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client.completions().builder("gpt-3.5-turbo-instruct");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn builder(&self, model: impl Into<String>) -> CompletionsBuilder {
        CompletionsBuilder::new(model)
    }

    /// Execute a completion request.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client.completions()
    ///     .builder("gpt-3.5-turbo-instruct")
    ///     .prompt("Once upon a time")
    ///     .max_tokens(50);
    /// let response = client.completions().create(builder).await?;
    /// println!("Completion: {:?}", response.choices);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: CompletionsBuilder) -> Result<CreateCompletionResponse> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::TEXT_COMPLETION;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match completions_api::create_completion()
            .configuration(&self.client.base_configuration)
            .create_completion_request(request)
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
            response.usage.as_ref().map(|u| i64::from(u.prompt_tokens)),
            response
                .usage
                .as_ref()
                .map(|u| i64::from(u.completion_tokens)),
        )
        .await;

        Ok(response)
    }
}
