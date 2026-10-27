//! Threads API client.

use super::{map_api_error, Client};
use crate::builders::threads::ThreadRequestBuilder;
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::assistants_api;
use openai_client_base::models::ThreadObject;
use std::time::Instant;

/// Client for threads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ThreadsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(ThreadsClient<'_, T>);

impl<T: Default + Send + Sync> ThreadsClient<'_, T> {
    /// Start building a new thread request.
    #[must_use]
    pub fn builder(&self) -> ThreadRequestBuilder {
        ThreadRequestBuilder::new()
    }

    /// Create a thread using the provided builder.
    pub async fn create(&self, builder: ThreadRequestBuilder) -> Result<ThreadObject> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::THREAD_CREATE;
        let model = "thread"; // No model for thread operations
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_thread()
            .configuration(&self.client.base_configuration)
            .maybe_create_thread_request(Some(request))
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
