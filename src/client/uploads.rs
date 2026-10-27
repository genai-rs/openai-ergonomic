//! Uploads API client.

use super::{map_api_error, Client};
use crate::builders::uploads::UploadBuilder;
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::{Error, UploadPurpose};
use openai_client_base::apis::uploads_api;
use openai_client_base::models::Upload;
use std::time::Instant;

/// Client for uploads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct UploadsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(UploadsClient<'_, T>);

impl<T: Default + Send + Sync> UploadsClient<'_, T> {
    /// Create a new upload builder for the given file metadata.
    #[must_use]
    pub fn builder(
        &self,
        filename: impl Into<String>,
        purpose: UploadPurpose,
        bytes: i32,
        mime_type: impl Into<String>,
    ) -> UploadBuilder {
        UploadBuilder::new(filename, purpose, bytes, mime_type)
    }

    /// Create an upload session.
    pub async fn create(&self, builder: UploadBuilder) -> Result<Upload> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::UPLOAD_CREATE;
        let model = "upload"; // No model for upload operations
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match uploads_api::create_upload()
            .configuration(&self.client.base_configuration)
            .create_upload_request(request)
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
