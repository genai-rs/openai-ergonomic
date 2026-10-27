//! Audio API client.

use super::{map_api_error, Client};
use crate::builders::audio::{
    SpeechBuilder, TranscriptionBuilder, TranscriptionRequest, TranslationBuilder,
    TranslationRequest,
};
use crate::builders::Builder;
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::audio_api;
use openai_client_base::models::{CreateTranscription200Response, CreateTranslation200Response};
use std::time::Instant;

/// Client for audio API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct AudioClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(AudioClient<'_, T>);

impl<T: Default + Send + Sync> AudioClient<'_, T> {
    /// Create a speech builder for text-to-speech generation.
    #[must_use]
    pub fn speech(
        &self,
        model: impl Into<String>,
        input: impl Into<String>,
        voice: impl Into<String>,
    ) -> SpeechBuilder {
        SpeechBuilder::new(model, input, voice)
    }

    /// Submit a speech synthesis request and return binary audio data.
    pub async fn create_speech(&self, builder: SpeechBuilder) -> Result<Vec<u8>> {
        let request = builder.build()?;
        let mut state = T::default();
        let operation = operation_names::AUDIO_SPEECH;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match audio_api::create_speech()
            .configuration(&self.client.base_configuration)
            .create_speech_request(request)
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

        let bytes = response.bytes().await.map_err(Error::Http)?;
        let duration = start_time.elapsed();

        // Call after_response hook (note: no JSON response for audio)
        let response_json = format!("{{\"size\": {}}}", bytes.len());
        self.call_after_response(
            &response_json,
            operation,
            &model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(bytes.to_vec())
    }

    /// Create a transcription builder for speech-to-text workflows.
    #[must_use]
    pub fn transcription(
        &self,
        file: impl AsRef<std::path::Path>,
        model: impl Into<String>,
    ) -> TranscriptionBuilder {
        TranscriptionBuilder::new(file, model)
    }

    /// Submit a transcription request.
    pub async fn create_transcription(
        &self,
        builder: TranscriptionBuilder,
    ) -> Result<CreateTranscription200Response> {
        let request = builder.build()?;
        let model_str = request.model.clone();
        let mut state = T::default();
        let operation = operation_names::AUDIO_TRANSCRIPTION;
        // TranscriptionRequest doesn't implement Serialize, so we'll create a simple JSON representation
        let request_json = format!(r#"{{"model":"{model_str}","file":"<audio_file>"}}"#);

        // Call before_request hook
        self.call_before_request(operation, &model_str, &request_json, &mut state)
            .await?;

        let TranscriptionRequest {
            file,
            model,
            language,
            prompt,
            response_format,
            temperature,
            stream,
            chunking_strategy,
            timestamp_granularities,
            include,
        } = request;

        let timestamp_strings = timestamp_granularities.as_ref().map(|values| {
            values
                .iter()
                .map(|granularity| granularity.as_str().to_string())
                .collect::<Vec<_>>()
        });

        let start_time = Instant::now();

        // Make the API call
        let response = match audio_api::create_transcription()
            .configuration(&self.client.base_configuration)
            .file(file)
            .model(&model)
            .maybe_language(language.as_deref())
            .maybe_prompt(prompt.as_deref())
            .maybe_response_format(response_format)
            .maybe_temperature(temperature)
            .maybe_stream(stream)
            .maybe_chunking_strategy(chunking_strategy)
            .maybe_timestamp_granularities(timestamp_strings)
            .maybe_include(include)
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

    /// Create a translation builder for audio-to-English translation.
    #[must_use]
    pub fn translation(
        &self,
        file: impl AsRef<std::path::Path>,
        model: impl Into<String>,
    ) -> TranslationBuilder {
        TranslationBuilder::new(file, model)
    }

    /// Submit an audio translation request.
    pub async fn create_translation(
        &self,
        builder: TranslationBuilder,
    ) -> Result<CreateTranslation200Response> {
        let request = builder.build()?;
        let model_str = request.model.clone();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::AUDIO_TRANSLATION;
        let request_json = format!(r#"{{"model":"{model_str}","file":"<audio_file>"}}"#);

        // Call before_request hook
        self.call_before_request(operation, &model_str, &request_json, &mut state)
            .await?;

        let TranslationRequest {
            file,
            model,
            prompt,
            response_format,
            temperature,
        } = request;

        let response_format_owned = response_format.map(|format| format.to_string());

        let start_time = Instant::now();

        // Make the API call
        let response = match audio_api::create_translation()
            .configuration(&self.client.base_configuration)
            .file(file)
            .model(&model)
            .maybe_prompt(prompt.as_deref())
            .maybe_response_format(response_format_owned.as_deref())
            .maybe_temperature(temperature)
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
