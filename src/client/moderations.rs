//! Moderations API client.

use super::{map_api_error, Client};
use crate::builders::moderations::ModerationBuilder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::moderations_api;
use openai_client_base::models::CreateModerationResponse;
use std::time::Instant;

/// Client for moderations API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ModerationsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(ModerationsClient<'_, T>);

impl<T: Default + Send + Sync> ModerationsClient<'_, T> {
    /// Create a moderation builder for checking text content.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client.moderations().builder("Text to check");
    /// let response = client.moderations().create(builder).await?;
    /// println!("Flagged: {}", response.results[0].flagged);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn builder(&self, input: impl Into<String>) -> ModerationBuilder {
        ModerationBuilder::new(input)
    }

    /// Convenience method for moderating a single text input.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client.moderations().check("Hello world");
    /// let response = client.moderations().create(builder).await?;
    ///
    /// if response.results[0].flagged {
    ///     println!("Content was flagged for moderation");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn check(&self, input: impl Into<String>) -> ModerationBuilder {
        ModerationBuilder::new(input)
    }

    /// Execute a moderation request built with [`ModerationBuilder`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    ///
    /// let builder = client
    ///     .moderations()
    ///     .check("Is this content appropriate?")
    ///     .model("text-moderation-latest");
    ///
    /// let response = client.moderations().create(builder).await?;
    ///
    /// println!("Model: {}", response.model);
    /// for result in response.results {
    ///     println!("Flagged: {}", result.flagged);
    ///     println!("Hate: {}", result.categories.hate);
    ///     println!("Violence: {}", result.categories.violence);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response cannot be parsed.
    pub async fn create(&self, builder: ModerationBuilder) -> Result<CreateModerationResponse> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MODERATION;
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "text-moderation-latest".to_string(), ToString::to_string);
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match moderations_api::create_moderation()
            .configuration(&self.client.base_configuration)
            .create_moderation_request(request)
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
            None,
            None,
        )
        .await;

        Ok(response)
    }
}
