//! Client wrapper for ergonomic `OpenAI` API access.
//!
//! This module provides a high-level client that wraps the base `OpenAI` client
//! with ergonomic builders and response handling.

use crate::{
    builders::{
        audio::{
            SpeechBuilder, TranscriptionBuilder, TranscriptionRequest, TranslationBuilder,
            TranslationRequest,
        },
        images::{
            ImageEditBuilder, ImageEditRequest, ImageGenerationBuilder, ImageVariationBuilder,
            ImageVariationRequest,
        },
        Builder, ChatCompletionBuilder, ResponsesBuilder,
    },
    config::Config,
    errors::Result,
    responses::ChatCompletionResponseWrapper,
    Error,
};
use openai_client_base::apis::Error as ApiError;
use openai_client_base::{
    apis::{audio_api, chat_api, configuration::Configuration, images_api},
    models::{
        CreateChatCompletionRequest, CreateTranscription200Response, CreateTranslation200Response,
        ImagesResponse,
    },
};
use reqwest::Client as HttpClient;
use std::sync::Arc;
use tokio::time::Duration;

/// Main client for interacting with the `OpenAI` API.
///
/// The client provides ergonomic methods for all `OpenAI` API endpoints,
/// with built-in retry logic, rate limiting, and error handling.
///
/// # Example
///
/// ```rust,ignore
/// # use openai_ergonomic::{Client, Config};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new(Config::default())?;
/// // TODO: Add usage example once builders are implemented
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    config: Arc<Config>,
    http: HttpClient,
    base_configuration: Configuration,
}

impl Client {
    /// Create a new client with the given configuration.
    pub fn new(config: Config) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(config.timeout_seconds()))
            .user_agent(format!("openai-ergonomic/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        // Create openai-client-base configuration
        let mut base_configuration = Configuration::new();
        base_configuration.bearer_access_token = Some(config.api_key().to_string());
        if let Some(base_url) = config.base_url() {
            base_configuration.base_path = base_url.to_string();
        }
        if let Some(org_id) = config.organization_id() {
            base_configuration.user_agent = Some(format!(
                "openai-ergonomic/{} org/{}",
                env!("CARGO_PKG_VERSION"),
                org_id
            ));
        }

        Ok(Self {
            config: Arc::new(config),
            http: http_client,
            base_configuration,
        })
    }

    /// Create a new client with default configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        Self::new(Config::from_env()?)
    }

    /// Get a reference to the client configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a reference to the HTTP client.
    pub fn http_client(&self) -> &HttpClient {
        &self.http
    }
}

// Chat API methods
impl Client {
    /// Create a chat completion builder.
    pub fn chat(&self) -> ChatCompletionBuilder {
        let model = self.config.default_model().unwrap_or("gpt-4");
        ChatCompletionBuilder::new(model)
    }

    /// Create a chat completion with a simple user message.
    pub fn chat_simple(&self, message: impl Into<String>) -> ChatCompletionBuilder {
        self.chat().user(message)
    }

    /// Create a chat completion with system and user messages.
    pub fn chat_with_system(
        &self,
        system: impl Into<String>,
        user: impl Into<String>,
    ) -> ChatCompletionBuilder {
        self.chat().system(system).user(user)
    }

    /// Execute a chat completion request.
    pub async fn execute_chat(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<ChatCompletionResponseWrapper> {
        let response = chat_api::create_chat_completion()
            .configuration(&self.base_configuration)
            .create_chat_completion_request(request)
            .call()
            .await
            .map_err(|e| Error::Api {
                status: 0,
                message: e.to_string(),
                error_type: None,
                error_code: None,
            })?;

        Ok(ChatCompletionResponseWrapper::new(response))
    }

    /// Execute a chat completion builder.
    pub async fn send_chat(
        &self,
        builder: ChatCompletionBuilder,
    ) -> Result<ChatCompletionResponseWrapper> {
        let request = builder.build()?;
        self.execute_chat(request).await
    }
}

// Responses API methods
impl Client {
    /// Create a responses builder for structured outputs.
    pub fn responses(&self) -> ResponsesBuilder {
        let model = self.config.default_model().unwrap_or("gpt-4");
        ResponsesBuilder::new(model)
    }

    /// Create a simple responses request with a user message.
    pub fn responses_simple(&self, message: impl Into<String>) -> ResponsesBuilder {
        self.responses().user(message)
    }

    /// Execute a responses request.
    pub async fn execute_responses(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<ChatCompletionResponseWrapper> {
        // The Responses API uses the same underlying endpoint as chat
        self.execute_chat(request).await
    }

    /// Execute a responses builder.
    pub async fn send_responses(
        &self,
        builder: ResponsesBuilder,
    ) -> Result<ChatCompletionResponseWrapper> {
        let request = builder.build()?;
        self.execute_responses(request).await
    }
}

// TODO: Add methods for other API endpoints
impl Client {
    /// Get assistants client (placeholder).
    #[must_use]
    pub fn assistants(&self) -> AssistantsClient<'_> {
        AssistantsClient { client: self }
    }

    /// Get audio client (placeholder).
    #[must_use]
    pub fn audio(&self) -> AudioClient<'_> {
        AudioClient { client: self }
    }

    /// Get embeddings client (placeholder).
    #[must_use]
    pub fn embeddings(&self) -> EmbeddingsClient<'_> {
        EmbeddingsClient { client: self }
    }

    /// Get images client (placeholder).
    #[must_use]
    pub fn images(&self) -> ImagesClient<'_> {
        ImagesClient { client: self }
    }

    /// Get files client (placeholder).
    #[must_use]
    pub fn files(&self) -> FilesClient<'_> {
        FilesClient { client: self }
    }

    /// Get fine-tuning client (placeholder).
    #[must_use]
    pub fn fine_tuning(&self) -> FineTuningClient<'_> {
        FineTuningClient { client: self }
    }

    /// Get batch client (placeholder).
    #[must_use]
    pub fn batch(&self) -> BatchClient<'_> {
        BatchClient { client: self }
    }

    /// Get vector stores client (placeholder).
    #[must_use]
    pub fn vector_stores(&self) -> VectorStoresClient<'_> {
        VectorStoresClient { client: self }
    }

    /// Get moderations client (placeholder).
    #[must_use]
    pub fn moderations(&self) -> ModerationsClient<'_> {
        ModerationsClient { client: self }
    }

    /// Get threads client (placeholder).
    #[must_use]
    pub fn threads(&self) -> ThreadsClient<'_> {
        ThreadsClient { client: self }
    }

    /// Get uploads client (placeholder).
    #[must_use]
    pub fn uploads(&self) -> UploadsClient<'_> {
        UploadsClient { client: self }
    }
}

impl AudioClient<'_> {
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
        let response = audio_api::create_speech()
            .configuration(&self.client.base_configuration)
            .create_speech_request(request)
            .call()
            .await
            .map_err(map_api_error)?;
        let bytes = response.bytes().await.map_err(Error::Http)?;
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
        } = builder.build()?;

        let timestamp_strings = timestamp_granularities.as_ref().map(|values| {
            values
                .iter()
                .map(|granularity| granularity.as_str().to_string())
                .collect::<Vec<_>>()
        });

        let request_builder = audio_api::create_transcription()
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
            .maybe_include(include);

        request_builder.call().await.map_err(map_api_error)
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
        let TranslationRequest {
            file,
            model,
            prompt,
            response_format,
            temperature,
        } = builder.build()?;

        let response_format_owned = response_format.map(|format| format.to_string());

        let request_builder = audio_api::create_translation()
            .configuration(&self.client.base_configuration)
            .file(file)
            .model(&model)
            .maybe_prompt(prompt.as_deref())
            .maybe_response_format(response_format_owned.as_deref())
            .maybe_temperature(temperature);

        request_builder.call().await.map_err(map_api_error)
    }
}

impl ImagesClient<'_> {
    /// Create a builder for image generation requests.
    #[must_use]
    pub fn generate(&self, prompt: impl Into<String>) -> ImageGenerationBuilder {
        ImageGenerationBuilder::new(prompt)
    }

    /// Execute an image generation request.
    pub async fn create(&self, builder: ImageGenerationBuilder) -> Result<ImagesResponse> {
        let request = builder.build()?;
        images_api::create_image()
            .configuration(&self.client.base_configuration)
            .create_image_request(request)
            .call()
            .await
            .map_err(map_api_error)
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
        let ImageEditRequest {
            image,
            prompt,
            mask,
            background,
            model,
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
        } = builder.build()?;

        images_api::create_image_edit()
            .configuration(&self.client.base_configuration)
            .image(image)
            .prompt(&prompt)
            .maybe_mask(mask)
            .maybe_background(background.as_deref())
            .maybe_model(model.as_deref())
            .maybe_n(n)
            .maybe_size(size.as_deref())
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
            .map_err(map_api_error)
    }

    /// Create an image variation builder.
    #[must_use]
    pub fn variation(&self, image: impl AsRef<std::path::Path>) -> ImageVariationBuilder {
        ImageVariationBuilder::new(image)
    }

    /// Execute an image variation request.
    pub async fn create_variation(&self, builder: ImageVariationBuilder) -> Result<ImagesResponse> {
        let ImageVariationRequest {
            image,
            model,
            n,
            response_format,
            size,
            user,
        } = builder.build()?;

        images_api::create_image_variation()
            .configuration(&self.client.base_configuration)
            .image(image)
            .maybe_model(model.as_deref())
            .maybe_n(n)
            .maybe_response_format(response_format.as_deref())
            .maybe_size(size.as_deref())
            .maybe_user(user.as_deref())
            .call()
            .await
            .map_err(map_api_error)
    }
}

fn map_api_error<T>(error: ApiError<T>) -> Error {
    match error {
        ApiError::Reqwest(err) => Error::Http(err),
        ApiError::ReqwestMiddleware(err) => {
            Error::Internal(format!("reqwest middleware error: {err}"))
        }
        ApiError::Serde(err) => Error::Json(err),
        ApiError::Io(err) => Error::File(err),
        ApiError::ResponseError(response) => Error::Api {
            status: response.status.as_u16(),
            message: response.content,
            error_type: None,
            error_code: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client_base::apis::{Error as BaseError, ResponseContent};

    #[test]
    fn map_api_error_converts_response() {
        let response = ResponseContent {
            status: reqwest::StatusCode::BAD_REQUEST,
            content: "bad request".to_string(),
            entity: Option::<()>::None,
        };

        let error = map_api_error(BaseError::ResponseError(response));
        match error {
            Error::Api {
                status, message, ..
            } => {
                assert_eq!(status, 400);
                assert!(message.contains("bad request"));
            }
            other => panic!("expected API error, got {other:?}"),
        }
    }
}

// Placeholder client types for different API endpoints
// TODO: Implement these properly once the builders are ready

/// Client for assistants API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct AssistantsClient<'a> {
    client: &'a Client,
}

/// Client for audio API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct AudioClient<'a> {
    client: &'a Client,
}

/// Client for embeddings API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct EmbeddingsClient<'a> {
    client: &'a Client,
}

/// Client for images API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ImagesClient<'a> {
    client: &'a Client,
}

/// Client for files API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FilesClient<'a> {
    client: &'a Client,
}

/// Client for fine-tuning API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FineTuningClient<'a> {
    client: &'a Client,
}

/// Client for batch API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BatchClient<'a> {
    client: &'a Client,
}

/// Client for vector stores API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct VectorStoresClient<'a> {
    client: &'a Client,
}

/// Client for moderations API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ModerationsClient<'a> {
    client: &'a Client,
}

/// Client for threads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ThreadsClient<'a> {
    client: &'a Client,
}

/// Client for uploads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct UploadsClient<'a> {
    client: &'a Client,
}
