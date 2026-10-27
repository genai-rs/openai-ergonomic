//! Images API client.

use super::{map_api_error, Client};
use crate::builders::images::{
    ImageEditBuilder, ImageEditRequest, ImageGenerationBuilder, ImageVariationBuilder,
    ImageVariationRequest,
};
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::images_api;
use openai_client_base::models::ImagesResponse;
use std::time::Instant;

/// Client for images API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ImagesClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(ImagesClient<'_, T>);

impl<T: Default + Send + Sync> ImagesClient<'_, T> {
    /// Create a builder for image generation requests.
    #[must_use]
    pub fn generate(&self, prompt: impl Into<String>) -> ImageGenerationBuilder {
        ImageGenerationBuilder::new(prompt)
    }

    /// Execute an image generation request.
    pub async fn create(&self, builder: ImageGenerationBuilder) -> Result<ImagesResponse> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::IMAGE_GENERATION;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match images_api::create_image()
            .configuration(&self.client.base_configuration)
            .create_image_request(request)
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

    /// Create an image edit builder using a base image and prompt.
    #[must_use]
    pub fn edit(
        &self,
        image: impl AsRef<std::path::Path>,
        prompt: impl Into<String>,
    ) -> ImageEditBuilder {
        ImageEditBuilder::new(image, prompt)
    }

    /// Execute an image edit request.
    pub async fn create_edit(&self, builder: ImageEditBuilder) -> Result<ImagesResponse> {
        let request = builder.build()?;
        let model_str = request
            .model
            .as_deref()
            .ok_or_else(|| Error::InvalidRequest("Image editing requires a model".to_string()))?
            .to_string();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::IMAGE_EDIT;
        let request_json = format!(
            r#"{{"prompt":"{}","model":"{}"}}"#,
            request.prompt, model_str
        );

        // Call before_request hook
        self.call_before_request(operation, &model_str, &request_json, &mut state)
            .await?;

        let ImageEditRequest {
            image,
            prompt,
            mask,
            background,
            model: _,
            n,
            size,
            response_format,
            output_format,
            output_compression,
            user,
            input_fidelity,
            stream,
            partial_images,
            quality,
        } = request;

        let start_time = Instant::now();

        // Make the API call
        let response = match images_api::create_image_edit()
            .configuration(&self.client.base_configuration)
            .image(image)
            .prompt(&prompt)
            .maybe_mask(mask)
            .maybe_background(background.as_deref())
            .model(&model_str)
            .maybe_n(n)
            .maybe_size(size.map(Into::into))
            .maybe_response_format(response_format.as_deref())
            .maybe_output_format(output_format.as_deref())
            .maybe_output_compression(output_compression)
            .maybe_user(user.as_deref())
            .maybe_input_fidelity(input_fidelity)
            .maybe_stream(stream)
            .maybe_partial_images(partial_images)
            .maybe_quality(quality.as_deref())
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model_str, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model_str,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Create an image variation builder.
    #[must_use]
    pub fn variation(&self, image: impl AsRef<std::path::Path>) -> ImageVariationBuilder {
        ImageVariationBuilder::new(image)
    }

    /// Execute an image variation request.
    pub async fn create_variation(&self, builder: ImageVariationBuilder) -> Result<ImagesResponse> {
        let request = builder.build()?;
        let model_str = request
            .model
            .as_ref()
            .map_or_else(|| "dall-e-2".to_string(), ToString::to_string);

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::IMAGE_VARIATION;
        let request_json = format!(r#"{{"model":"{model_str}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, &model_str, &request_json, &mut state)
            .await?;

        let ImageVariationRequest {
            image,
            model,
            n,
            response_format,
            size,
            user,
        } = request;

        let start_time = Instant::now();

        // Make the API call
        let response = match images_api::create_image_variation()
            .configuration(&self.client.base_configuration)
            .image(image)
            .maybe_model(model.as_deref())
            .maybe_n(n)
            .maybe_response_format(response_format.as_deref())
            .maybe_size(size.as_deref())
            .maybe_user(user.as_deref())
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model_str, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model_str,
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
