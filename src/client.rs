//! Client wrapper for ergonomic `OpenAI` API access.
//!
//! This module provides a high-level client that wraps the base `OpenAI` client
//! with ergonomic builders and response handling.

// Allow this lint at module level for interceptor helper methods
// that require many parameters for comprehensive context passing
#![allow(clippy::too_many_arguments)]

use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, InterceptorChain,
};
use crate::semantic_conventions::operation_names;
use crate::{
    builders::{
        assistants::{AssistantBuilder, MessageBuilder, RunBuilder},
        audio::{
            SpeechBuilder, TranscriptionBuilder, TranscriptionRequest, TranslationBuilder,
            TranslationRequest,
        },
        completions::CompletionsBuilder,
        embeddings::EmbeddingsBuilder,
        files::{FileDeleteBuilder, FileListBuilder, FileRetrievalBuilder, FileUploadBuilder},
        images::{
            ImageEditBuilder, ImageEditRequest, ImageGenerationBuilder, ImageVariationBuilder,
            ImageVariationRequest,
        },
        models::{ModelDeleteBuilder, ModelRetrievalBuilder},
        moderations::ModerationBuilder,
        threads::ThreadRequestBuilder,
        uploads::UploadBuilder,
        usage::UsageBuilder,
        Builder, ChatCompletionBuilder, ResponsesBuilder,
    },
    config::Config,
    errors::Result,
    responses::ChatCompletionResponseWrapper,
    Error, UploadPurpose,
};
use openai_client_base::apis::Error as ApiError;
use openai_client_base::{
    apis::{
        assistants_api, audio_api, batch_api, chat_api, completions_api,
        configuration::Configuration, embeddings_api, files_api, fine_tuning_api, images_api,
        models_api, moderations_api, uploads_api, usage_api, vector_stores_api,
    },
    models::{
        AssistantObject, Batch, CreateBatchRequest, CreateChatCompletionRequest,
        CreateCompletionResponse, CreateEmbeddingResponse, CreateFineTuningJobRequest,
        CreateModerationResponse, CreateTranscription200Response, CreateTranslation200Response,
        DeleteAssistantResponse, DeleteFileResponse, DeleteModelResponse,
        DeleteVectorStoreFileResponse, DeleteVectorStoreResponse, FineTuningJob, ImagesResponse,
        ListAssistantsResponse, ListBatchesResponse, ListFilesResponse,
        ListFineTuningJobCheckpointsResponse, ListFineTuningJobEventsResponse,
        ListMessagesResponse, ListModelsResponse, ListPaginatedFineTuningJobsResponse,
        ListRunStepsResponse, ListRunsResponse, ListVectorStoreFilesResponse,
        ListVectorStoresResponse, MessageObject, Model, OpenAiFile, RunObject, RunStepObject,
        SubmitToolOutputsRunRequestToolOutputsInner, ThreadObject, Upload, UsageResponse,
        VectorStoreFileObject, VectorStoreObject, VectorStoreSearchResultsPage,
    },
};
use reqwest_middleware::ClientWithMiddleware as HttpClient;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

// Helper macro to generate interceptor helper methods for sub-clients
macro_rules! impl_interceptor_helpers {
    ($client_type:ty) => {
        impl<T: Default + Send + Sync> $client_type {
            /// Helper to call `before_request` hooks
            async fn call_before_request(
                &self,
                operation: &str,
                model: &str,
                request_json: &str,
                state: &mut T,
            ) -> Result<()> {
                if !self.client.interceptors.is_empty() {
                    let mut ctx = BeforeRequestContext {
                        operation,
                        model,
                        request_json,
                        state,
                    };
                    if let Err(e) = self.client.interceptors.before_request(&mut ctx).await {
                        let error_ctx = ErrorContext {
                            operation,
                            model: Some(model),
                            request_json: Some(request_json),
                            error: &e,
                            state: Some(state),
                        };
                        self.client.interceptors.on_error(&error_ctx).await;
                        return Err(e);
                    }
                }
                Ok(())
            }

            /// Helper to handle API errors with interceptor hooks
            async fn handle_api_error<E>(
                &self,
                error: openai_client_base::apis::Error<E>,
                operation: &str,
                model: &str,
                request_json: &str,
                state: &T,
            ) -> Error {
                let error = map_api_error(error);

                if !self.client.interceptors.is_empty() {
                    let error_ctx = ErrorContext {
                        operation,
                        model: Some(model),
                        request_json: Some(request_json),
                        error: &error,
                        state: Some(state),
                    };
                    self.client.interceptors.on_error(&error_ctx).await;
                }

                error
            }

            /// Helper to call `after_response` hooks
            async fn call_after_response<R>(
                &self,
                response: &R,
                operation: &str,
                model: &str,
                request_json: &str,
                state: &T,
                duration: std::time::Duration,
                input_tokens: Option<i64>,
                output_tokens: Option<i64>,
            ) where
                R: serde::Serialize + Sync,
            {
                if !self.client.interceptors.is_empty() {
                    let response_json = serde_json::to_string(response).unwrap_or_default();
                    let ctx = AfterResponseContext {
                        operation,
                        model,
                        request_json,
                        response_json: &response_json,
                        duration,
                        input_tokens,
                        output_tokens,
                        state,
                    };
                    if let Err(e) = self.client.interceptors.after_response(&ctx).await {
                        tracing::warn!("Interceptor after_response failed: {}", e);
                    }
                }
            }
        }
    };
}

/// Builder for creating a `Client` with interceptors.
///
/// The builder pattern allows you to configure interceptors before the client
/// is created. Once built, the interceptors are immutable, eliminating the need
/// for runtime locking.
///
/// # Example
///
/// ```rust,ignore
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(my_interceptor))
///     .build();
/// ```
pub struct ClientBuilder<T = ()> {
    config: Arc<Config>,
    http: HttpClient,
    base_configuration: Configuration,
    interceptors: InterceptorChain<T>,
}

/// Main client for interacting with the `OpenAI` API.
///
/// The client provides ergonomic methods for all `OpenAI` API endpoints,
/// with built-in retry logic, rate limiting, error handling, and support
/// for middleware through interceptors.
///
/// Use `Client::from_env()` or `Client::new()` to create a builder, then call
/// `.build()` to create the client.
///
/// # Example
///
/// ```rust,ignore
/// # use openai_ergonomic::{Client, Config};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::from_env()?.build();
/// // TODO: Add usage example once builders are implemented
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client<T = ()> {
    config: Arc<Config>,
    http: HttpClient,
    base_configuration: Configuration,
    interceptors: Arc<InterceptorChain<T>>,
}

// Custom Debug implementation since InterceptorChain doesn't implement Debug
impl<T> std::fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("config", &self.config)
            .field("http", &"<HttpClient>")
            .field("base_configuration", &"<Configuration>")
            .field("interceptors", &"<InterceptorChain>")
            .finish()
    }
}

// Implementation for ClientBuilder with default state type ()
impl ClientBuilder {
    /// Create a new client builder with the given configuration.
    pub fn new(config: Config) -> Result<Self> {
        // Check if we're using Azure OpenAI
        let is_azure = config.is_azure();

        // Use custom HTTP client if provided, otherwise build a default one
        let http_client = if let Some(client) = config.http_client() {
            client.clone()
        } else {
            let reqwest_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120)) // Default timeout: 120 seconds
                .user_agent(format!("openai-ergonomic/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(Error::Http)?;

            let mut client_builder = reqwest_middleware::ClientBuilder::new(reqwest_client);

            // Add Azure authentication middleware if using Azure OpenAI
            if is_azure {
                let azure_middleware = crate::azure_middleware::AzureAuthMiddleware::new(
                    config.api_key().to_string(),
                    config.azure_api_version().map(String::from),
                    config.azure_deployment().map(String::from),
                );
                client_builder = client_builder.with(azure_middleware);
            }

            client_builder.build()
        };

        // Create openai-client-base configuration
        let mut base_configuration = Configuration::new();

        // For Azure OpenAI, we don't use bearer token (handled by middleware)
        // For standard OpenAI, use bearer token
        if !is_azure {
            base_configuration.bearer_access_token = Some(config.api_key().to_string());
        }

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
            interceptors: InterceptorChain::new(),
        })
    }

    /// Create a new client builder with default configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        Self::new(Config::from_env()?)
    }
}

// Implementation for ClientBuilder with any state type
impl<T> ClientBuilder<T> {
    /// Add an interceptor to the builder.
    ///
    /// Creates a new builder with the interceptor's state type. The interceptor provides
    /// hooks into the request/response lifecycle for observability, logging, and custom
    /// processing.
    ///
    /// Note: This method transforms the builder's type, so it can only be called once.
    /// For multiple interceptors with the same state type, use a composite interceptor
    /// or call this method multiple times (each will replace the previous chain).
    ///
    /// # Examples
    ///
    /// Simple interceptor (no state):
    /// ```rust,ignore
    /// use openai_ergonomic::{Client, Interceptor, BeforeRequestContext};
    ///
    /// struct LoggingInterceptor;
    ///
    /// #[async_trait::async_trait]
    /// impl Interceptor for LoggingInterceptor {
    ///     async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
    ///         println!("Calling {}", ctx.operation);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let client = Client::from_env()?
    ///     .with_interceptor(Box::new(LoggingInterceptor))
    ///     .build();
    /// ```
    ///
    /// Interceptor with custom state:
    /// ```rust,ignore
    /// use openai_ergonomic::{Client, LangfuseInterceptor, LangfuseState};
    ///
    /// let interceptor = LangfuseInterceptor::new(tracer, config);
    /// let client: Client<LangfuseState<_>> = Client::from_env()?
    ///     .with_interceptor(Box::new(interceptor))
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_interceptor<U>(
        self,
        interceptor: Box<dyn crate::interceptor::Interceptor<U>>,
    ) -> ClientBuilder<U> {
        let mut new_chain = InterceptorChain::new();
        new_chain.add(interceptor);

        ClientBuilder {
            config: self.config,
            http: self.http,
            base_configuration: self.base_configuration,
            interceptors: new_chain,
        }
    }

    /// Add an interceptor that uses the same state type.
    ///
    /// This allows chaining multiple interceptors with the same state type without
    /// type transformation.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = Client::from_env()?
    ///     .add_interceptor(Box::new(logger))
    ///     .add_interceptor(Box::new(metrics))
    ///     .build();
    /// ```
    #[must_use]
    pub fn add_interceptor(
        mut self,
        interceptor: Box<dyn crate::interceptor::Interceptor<T>>,
    ) -> Self {
        self.interceptors.add(interceptor);
        self
    }

    /// Build the client with the configured interceptors.
    ///
    /// After building, the interceptors are immutable, eliminating runtime locking overhead.
    #[must_use]
    pub fn build(self) -> Client<T> {
        Client {
            config: self.config,
            http: self.http,
            base_configuration: self.base_configuration,
            interceptors: Arc::new(self.interceptors),
        }
    }
}

// Implementation for Client
impl Client {
    /// Create a new client builder with the given configuration.
    pub fn builder(config: Config) -> Result<ClientBuilder> {
        ClientBuilder::new(config)
    }

    /// Create a new client builder with default configuration from environment variables.
    pub fn from_env() -> Result<ClientBuilder> {
        ClientBuilder::from_env()
    }
}

impl<T> Client<T> {
    /// Get a reference to the client configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a reference to the HTTP client.
    pub fn http_client(&self) -> &HttpClient {
        &self.http
    }
}

// Interceptor helper methods
impl<T: Default + Send + Sync> Client<T> {
    /// Helper to call `before_request` hooks
    async fn call_before_request(
        &self,
        operation: &str,
        model: &str,
        request_json: &str,
        state: &mut T,
    ) -> Result<()> {
        if !self.interceptors.is_empty() {
            let mut ctx = BeforeRequestContext {
                operation,
                model,
                request_json,
                state,
            };
            if let Err(e) = self.interceptors.before_request(&mut ctx).await {
                let error_ctx = ErrorContext {
                    operation,
                    model: Some(model),
                    request_json: Some(request_json),
                    error: &e,
                    state: Some(state),
                };
                self.interceptors.on_error(&error_ctx).await;
                return Err(e);
            }
        }
        Ok(())
    }

    /// Helper to handle API errors with interceptor hooks
    async fn handle_api_error<E>(
        &self,
        error: openai_client_base::apis::Error<E>,
        operation: &str,
        model: &str,
        request_json: &str,
        state: &T,
    ) -> Error {
        let error = map_api_error(error);

        if !self.interceptors.is_empty() {
            let error_ctx = ErrorContext {
                operation,
                model: Some(model),
                request_json: Some(request_json),
                error: &error,
                state: Some(state),
            };
            self.interceptors.on_error(&error_ctx).await;
        }

        error
    }

    /// Helper to call `after_response` hooks
    async fn call_after_response<R>(
        &self,
        response: &R,
        operation: &str,
        model: &str,
        request_json: &str,
        state: &T,
        duration: std::time::Duration,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) where
        R: serde::Serialize + Sync,
    {
        if !self.interceptors.is_empty() {
            let response_json = serde_json::to_string(response).unwrap_or_default();
            let ctx = AfterResponseContext {
                operation,
                model,
                request_json,
                response_json: &response_json,
                duration,
                input_tokens,
                output_tokens,
                state,
            };
            if let Err(e) = self.interceptors.after_response(&ctx).await {
                tracing::warn!("Interceptor after_response failed: {}", e);
            }
        }
    }
}

// Chat API methods
impl<T: Default + Send + Sync> Client<T> {
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
        let mut state = T::default();
        let operation = operation_names::CHAT;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match chat_api::create_chat_completion()
            .configuration(&self.base_configuration)
            .create_chat_completion_request(request)
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
impl<T: Default + Send + Sync> Client<T> {
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
impl<T: Default + Send + Sync> Client<T> {
    /// Get assistants client (placeholder).
    #[must_use]
    pub fn assistants(&self) -> AssistantsClient<'_, T> {
        AssistantsClient { client: self }
    }

    /// Get audio client (placeholder).
    #[must_use]
    pub fn audio(&self) -> AudioClient<'_, T> {
        AudioClient { client: self }
    }

    /// Get embeddings client (placeholder).
    #[must_use]
    pub fn embeddings(&self) -> EmbeddingsClient<'_, T> {
        EmbeddingsClient { client: self }
    }

    /// Get images client (placeholder).
    #[must_use]
    pub fn images(&self) -> ImagesClient<'_, T> {
        ImagesClient { client: self }
    }

    /// Get files client (placeholder).
    #[must_use]
    pub fn files(&self) -> FilesClient<'_, T> {
        FilesClient { client: self }
    }

    /// Get fine-tuning client (placeholder).
    #[must_use]
    pub fn fine_tuning(&self) -> FineTuningClient<'_, T> {
        FineTuningClient { client: self }
    }

    /// Get batch client (placeholder).
    #[must_use]
    pub fn batch(&self) -> BatchClient<'_, T> {
        BatchClient { client: self }
    }

    /// Get vector stores client (placeholder).
    #[must_use]
    pub fn vector_stores(&self) -> VectorStoresClient<'_, T> {
        VectorStoresClient { client: self }
    }

    /// Get moderations client (placeholder).
    #[must_use]
    pub fn moderations(&self) -> ModerationsClient<'_, T> {
        ModerationsClient { client: self }
    }

    /// Get threads client (placeholder).
    #[must_use]
    pub fn threads(&self) -> ThreadsClient<'_, T> {
        ThreadsClient { client: self }
    }

    /// Get uploads client (placeholder).
    #[must_use]
    pub fn uploads(&self) -> UploadsClient<'_, T> {
        UploadsClient { client: self }
    }

    /// Get models client.
    #[must_use]
    pub fn models(&self) -> ModelsClient<'_, T> {
        ModelsClient { client: self }
    }

    /// Get completions client.
    #[must_use]
    pub fn completions(&self) -> CompletionsClient<'_, T> {
        CompletionsClient { client: self }
    }

    /// Get usage client.
    #[must_use]
    pub fn usage(&self) -> UsageClient<'_, T> {
        UsageClient { client: self }
    }
}

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

impl<T: Default + Send + Sync> EmbeddingsClient<'_, T> {
    /// Start a builder for creating embeddings requests with the given model.
    #[must_use]
    pub fn builder(&self, model: impl Into<String>) -> EmbeddingsBuilder {
        EmbeddingsBuilder::new(model)
    }

    /// Convenience helper for embedding a single string input.
    #[must_use]
    pub fn text(&self, model: impl Into<String>, input: impl Into<String>) -> EmbeddingsBuilder {
        self.builder(model).input_text(input)
    }

    /// Convenience helper for embedding a single tokenized input.
    #[must_use]
    pub fn tokens<I>(&self, model: impl Into<String>, tokens: I) -> EmbeddingsBuilder
    where
        I: IntoIterator<Item = i32>,
    {
        self.builder(model).input_tokens(tokens)
    }

    /// Execute an embeddings request built with [`EmbeddingsBuilder`].
    pub async fn create(&self, builder: EmbeddingsBuilder) -> Result<CreateEmbeddingResponse> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::EMBEDDINGS;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match embeddings_api::create_embedding()
            .configuration(&self.client.base_configuration)
            .create_embedding_request(request)
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
            Some(i64::from(response.usage.prompt_tokens)),
            Some(i64::from(response.usage.total_tokens)),
        )
        .await;

        Ok(response)
    }
}

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
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "dall-e-2".to_string(), ToString::to_string);
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
            .as_ref()
            .map_or_else(|| "dall-e-2".to_string(), ToString::to_string);

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
        } = request;

        let start_time = Instant::now();

        // Make the API call
        let response = match images_api::create_image_edit()
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

impl<T: Default + Send + Sync> ModerationsClient<'_, T> {
    /// Create a moderation builder for checking text content.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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

impl<T: Default + Send + Sync> FilesClient<'_, T> {
    /// Upload a file to `OpenAI`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::files::FilePurpose;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = client
    ///     .files()
    ///     .upload_text("training.jsonl", FilePurpose::FineTune, "training data");
    /// let file = client.files().create(builder).await?;
    /// println!("Uploaded file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(&self, builder: FileUploadBuilder) -> Result<OpenAiFile> {
        // Write content to a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(builder.filename());
        std::fs::write(&temp_file_path, builder.content()).map_err(Error::File)?;

        // Convert FilePurpose to openai_client_base::models::FilePurpose
        let purpose = match builder.purpose().to_string().as_str() {
            "fine-tune" => openai_client_base::models::FilePurpose::FineTune,
            "vision" => openai_client_base::models::FilePurpose::Vision,
            "batch" => openai_client_base::models::FilePurpose::Batch,
            _ => openai_client_base::models::FilePurpose::Assistants, // Default for "assistants" and unknown
        };

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_UPLOAD;
        let model = "file-upload"; // No model for file operations
        let request_json = format!(
            r#"{{"filename":"{}","purpose":"{}","size":{}}}"#,
            builder.filename(),
            builder.purpose(),
            builder.content().len()
        );

        // Call before_request hook
        if let Err(e) = self
            .call_before_request(operation, model, &request_json, &mut state)
            .await
        {
            // Clean up temp file before returning
            let _ = std::fs::remove_file(&temp_file_path);
            return Err(e);
        }

        let start_time = Instant::now();

        // Make the API call
        let result = match files_api::create_file()
            .configuration(&self.client.base_configuration)
            .file(temp_file_path.clone())
            .purpose(purpose)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Clean up temp file
                let _ = std::fs::remove_file(&temp_file_path);
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        // Clean up temporary file
        let _ = std::fs::remove_file(temp_file_path);

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &result,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(result)
    }

    /// Convenience method to upload a file (alias for upload).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::files::FilePurpose;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = client
    ///     .files()
    ///     .upload_text("data.txt", FilePurpose::Assistants, "content");
    /// let file = client.files().create(builder).await?;
    /// println!("File ID: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: FileUploadBuilder) -> Result<OpenAiFile> {
        self.upload(builder).await
    }

    /// Create a file upload builder from text content.
    #[must_use]
    pub fn upload_text(
        &self,
        filename: impl Into<String>,
        purpose: crate::builders::files::FilePurpose,
        text: impl Into<String>,
    ) -> FileUploadBuilder {
        FileUploadBuilder::from_text(filename, purpose, text)
    }

    /// Create a file upload builder from bytes.
    #[must_use]
    pub fn upload_bytes(
        &self,
        filename: impl Into<String>,
        purpose: crate::builders::files::FilePurpose,
        content: Vec<u8>,
    ) -> FileUploadBuilder {
        FileUploadBuilder::new(filename, purpose, content)
    }

    /// Create a file upload builder from a file path.
    pub fn upload_from_path(
        &self,
        path: impl AsRef<std::path::Path>,
        purpose: crate::builders::files::FilePurpose,
    ) -> Result<FileUploadBuilder> {
        FileUploadBuilder::from_path(path, purpose).map_err(Error::File)
    }

    /// List files.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = client.files().list_builder();
    /// let files = client.files().list(builder).await?;
    /// println!("Found {} files", files.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, builder: FileListBuilder) -> Result<ListFilesResponse> {
        let purpose = builder.purpose_ref().map(ToString::to_string);
        let limit = builder.limit_ref();
        let order = builder.order_ref().map(ToString::to_string);

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_LIST;
        let model = "files";
        let request_json = format!(
            r#"{{"purpose":"{}","limit":{},"order":"{}"}}"#,
            purpose.as_deref().unwrap_or(""),
            limit.unwrap_or(10000),
            order.as_deref().unwrap_or("desc")
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::list_files()
            .configuration(&self.client.base_configuration)
            .maybe_purpose(purpose.as_deref())
            .maybe_limit(limit)
            .maybe_order(order.as_deref())
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

    /// Create a list files builder.
    #[must_use]
    pub fn list_builder(&self) -> FileListBuilder {
        FileListBuilder::new()
    }

    /// Retrieve information about a specific file.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let file = client.files().retrieve("file-123").await?;
    /// println!("File: {} ({})", file.filename, file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, file_id: impl Into<String>) -> Result<OpenAiFile> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_RETRIEVE;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::retrieve_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
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

    /// Retrieve information about a file using a builder.
    pub async fn get(&self, builder: FileRetrievalBuilder) -> Result<OpenAiFile> {
        self.retrieve(builder.file_id()).await
    }

    /// Download file content.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let content = client.files().download("file-123").await?;
    /// println!("Downloaded {} bytes", content.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download(&self, file_id: impl Into<String>) -> Result<String> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_DOWNLOAD;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::download_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
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
        let response_size = format!(r#"{{"size":{}}}"#, response.len());
        self.call_after_response(
            &response_size,
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

    /// Download file content as bytes.
    pub async fn download_bytes(&self, file_id: impl Into<String>) -> Result<Vec<u8>> {
        let content = self.download(file_id).await?;
        Ok(content.into_bytes())
    }

    /// Delete a file.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.files().delete("file-123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, file_id: impl Into<String>) -> Result<DeleteFileResponse> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_DELETE;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::delete_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
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

    /// Delete a file using a builder.
    pub async fn remove(&self, builder: FileDeleteBuilder) -> Result<DeleteFileResponse> {
        self.delete(builder.file_id()).await
    }
}

impl<T: Default + Send + Sync> VectorStoresClient<'_, T> {
    /// Create a new vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = VectorStoreBuilder::new()
    ///     .name("My Knowledge Base")
    ///     .add_file("file-123");
    /// let vector_store = client.vector_stores().create(builder).await?;
    /// println!("Created vector store: {}", vector_store.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        builder: crate::builders::vector_stores::VectorStoreBuilder,
    ) -> Result<VectorStoreObject> {
        use openai_client_base::models::{CreateVectorStoreRequest, VectorStoreExpirationAfter};

        let mut request = CreateVectorStoreRequest::new();
        request.name = builder.name_ref().map(String::from);
        request.file_ids = if builder.has_files() {
            Some(builder.file_ids_ref().to_vec())
        } else {
            None
        };

        if let Some(expires_after) = builder.expires_after_ref() {
            use openai_client_base::models::vector_store_expiration_after::Anchor;
            request.expires_after = Some(Box::new(VectorStoreExpirationAfter::new(
                Anchor::LastActiveAt,
                expires_after.days,
            )));
        }

        if !builder.metadata_ref().is_empty() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_CREATE;
        let model = "vector-store";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::create_vector_store()
            .configuration(&self.client.base_configuration)
            .create_vector_store_request(request)
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

    /// List vector stores.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.vector_stores().list(Some(20), None, None, None).await?;
    /// println!("Found {} vector stores", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListVectorStoresResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_LIST;
        let model = "vector-store";
        let request_json = format!(
            r#"{{"limit":{},"order":"{}"}}"#,
            limit.unwrap_or(20),
            order.unwrap_or("desc")
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::list_vector_stores()
            .configuration(&self.client.base_configuration)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
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

    /// Get a specific vector store by ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let vector_store = client.vector_stores().get("vs_123").await?;
    /// println!("Vector store: {}", vector_store.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, vector_store_id: impl Into<String>) -> Result<VectorStoreObject> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_RETRIEVE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::get_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
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

    /// Update a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = VectorStoreBuilder::new()
    ///     .name("Updated Name")
    ///     .metadata("updated", "true");
    /// let vector_store = client.vector_stores().update("vs_123", builder).await?;
    /// println!("Updated: {}", vector_store.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        vector_store_id: impl Into<String>,
        builder: crate::builders::vector_stores::VectorStoreBuilder,
    ) -> Result<VectorStoreObject> {
        use openai_client_base::models::{UpdateVectorStoreRequest, VectorStoreExpirationAfter};

        let id = vector_store_id.into();
        let mut request = UpdateVectorStoreRequest::new();
        request.name = builder.name_ref().map(String::from);

        if let Some(expires_after) = builder.expires_after_ref() {
            use openai_client_base::models::vector_store_expiration_after::Anchor;
            request.expires_after = Some(Box::new(VectorStoreExpirationAfter::new(
                Anchor::LastActiveAt,
                expires_after.days,
            )));
        }

        if !builder.metadata_ref().is_empty() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_UPDATE;
        let model = "vector-store";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::modify_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
            .update_vector_store_request(request)
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

    /// Delete a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.vector_stores().delete("vs_123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(
        &self,
        vector_store_id: impl Into<String>,
    ) -> Result<DeleteVectorStoreResponse> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_DELETE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::delete_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
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

    /// Add a file to a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let file = client.vector_stores().add_file("vs_123", "file-456").await?;
    /// println!("Added file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<VectorStoreFileObject> {
        use openai_client_base::models::CreateVectorStoreFileRequest;

        let vs_id = vector_store_id.into();
        let f_id = file_id.into();
        let request = CreateVectorStoreFileRequest::new(f_id.clone());

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_ADD;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::create_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .create_vector_store_file_request(request)
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

    /// List files in a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.vector_stores().list_files("vs_123", None, None, None, None, None).await?;
    /// println!("Found {} files", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_files(
        &self,
        vector_store_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        filter: Option<&str>,
    ) -> Result<ListVectorStoreFilesResponse> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_LIST;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::list_vector_store_files()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_filter(filter)
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

    /// Get a file from a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let file = client.vector_stores().get_file("vs_123", "file-456").await?;
    /// println!("File: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<VectorStoreFileObject> {
        let vs_id = vector_store_id.into();
        let f_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_RETRIEVE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::get_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .file_id(&f_id)
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

    /// Delete a file from a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.vector_stores().delete_file("vs_123", "file-456").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<DeleteVectorStoreFileResponse> {
        let vs_id = vector_store_id.into();
        let f_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_DELETE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::delete_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .file_id(&f_id)
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

    /// Search a vector store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreSearchBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = VectorStoreSearchBuilder::new("vs_123", "machine learning concepts");
    /// let results = client.vector_stores().search(builder).await?;
    /// println!("Found {} results", results.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        builder: crate::builders::vector_stores::VectorStoreSearchBuilder,
    ) -> Result<VectorStoreSearchResultsPage> {
        use openai_client_base::models::{VectorStoreSearchRequest, VectorStoreSearchRequestQuery};

        let query = VectorStoreSearchRequestQuery::new_text(builder.query().to_string());
        let mut request = VectorStoreSearchRequest::new(query);

        if let Some(limit) = builder.limit_ref() {
            request.max_num_results = Some(limit);
        }

        let vs_id = builder.vector_store_id().to_string();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_SEARCH;
        let model = "vector-store";
        let request_json = format!(
            r#"{{"vector_store_id":"{}","query":"{}"}}"#,
            vs_id,
            builder.query()
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::search_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .vector_store_search_request(request)
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

impl<T: Default + Send + Sync> BatchClient<'_, T> {
    /// Create a new batch job.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::batch::{BatchJobBuilder, BatchEndpoint};
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = BatchJobBuilder::new("file-batch-input", BatchEndpoint::ChatCompletions);
    /// let batch = client.batch().create(builder).await?;
    /// println!("Created batch: {}", batch.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: crate::builders::batch::BatchJobBuilder) -> Result<Batch> {
        use openai_client_base::models::create_batch_request::{CompletionWindow, Endpoint};

        // Map our endpoint to the base client enum
        let endpoint = match builder.endpoint() {
            crate::builders::batch::BatchEndpoint::ChatCompletions => {
                Endpoint::SlashV1SlashChatSlashCompletions
            }
            crate::builders::batch::BatchEndpoint::Embeddings => Endpoint::SlashV1SlashEmbeddings,
            crate::builders::batch::BatchEndpoint::Completions => Endpoint::SlashV1SlashCompletions,
        };

        let mut request = CreateBatchRequest::new(
            builder.input_file_id().to_string(),
            endpoint,
            CompletionWindow::Variant24h,
        );

        if builder.has_metadata() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_CREATE;
        let model = "batch";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::create_batch()
            .configuration(&self.client.base_configuration)
            .create_batch_request(request)
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

    /// List batch jobs.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.batch().list(None, Some(20)).await?;
    /// println!("Found {} batches", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ListBatchesResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_LIST;
        let model = "batch";
        let request_json = format!("{{\"after\":{after:?},\"limit\":{limit:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::list_batches()
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

    /// Get a specific batch job.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let batch = client.batch().get("batch_123").await?;
    /// println!("Batch status: {}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, batch_id: impl Into<String>) -> Result<Batch> {
        let id = batch_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_RETRIEVE;
        let model = "batch";
        let request_json = format!("{{\"batch_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::retrieve_batch()
            .configuration(&self.client.base_configuration)
            .batch_id(&id)
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

    /// Cancel a batch job.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let batch = client.batch().cancel("batch_123").await?;
    /// println!("Batch cancelled: {}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, batch_id: impl Into<String>) -> Result<Batch> {
        let id = batch_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::BATCH_CANCEL;
        let model = "batch";
        let request_json = format!("{{\"batch_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match batch_api::cancel_batch()
            .configuration(&self.client.base_configuration)
            .batch_id(&id)
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

impl<T: Default + Send + Sync> FineTuningClient<'_, T> {
    /// Create a new fine-tuning job.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::fine_tuning::FineTuningJobBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
            request.validation_file = Some(validation_file.to_string());
        }

        if let Some(suffix) = builder.suffix_ref() {
            request.suffix = Some(suffix.to_string());
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let job = client.fine_tuning().get_job("ftjob-123").await?;
    /// println!("Job status: {}", job.status);
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let job = client.fine_tuning().cancel_job("ftjob-123").await?;
    /// println!("Job cancelled: {}", job.status);
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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

    #[test]
    fn test_moderation_builder_creation() {
        use crate::builders::moderations::ModerationBuilder;

        let builder = ModerationBuilder::new("Test content");
        let request = builder.build().unwrap();

        assert_eq!(request.input, "Test content");
        assert!(request.model.is_none());
    }

    #[test]
    fn test_moderation_builder_with_model() {
        use crate::builders::moderations::ModerationBuilder;

        let builder = ModerationBuilder::new("Test content").model("text-moderation-stable");
        let request = builder.build().unwrap();

        assert_eq!(request.input, "Test content");
        assert_eq!(request.model, Some("text-moderation-stable".to_string()));
    }

    #[test]
    fn test_moderation_builder_array_input() {
        use crate::builders::moderations::ModerationBuilder;

        let inputs = vec!["First text".to_string(), "Second text".to_string()];
        let builder = ModerationBuilder::new_array(inputs);
        let request = builder.build().unwrap();

        // Array inputs are joined with newlines
        assert_eq!(request.input, "First text\nSecond text");
    }

    #[test]
    fn test_file_upload_builder_creation() {
        use crate::builders::files::{FilePurpose, FileUploadBuilder};

        let content = b"test content".to_vec();
        let builder = FileUploadBuilder::new("test.txt", FilePurpose::Assistants, content.clone());

        assert_eq!(builder.filename(), "test.txt");
        assert_eq!(builder.content(), content.as_slice());
        assert_eq!(builder.content_size(), content.len());
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_file_upload_builder_from_text() {
        use crate::builders::files::{FilePurpose, FileUploadBuilder};

        let builder =
            FileUploadBuilder::from_text("hello.txt", FilePurpose::FineTune, "Hello, world!");

        assert_eq!(builder.filename(), "hello.txt");
        assert_eq!(
            builder.content_as_string(),
            Some("Hello, world!".to_string())
        );
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_file_list_builder() {
        use crate::builders::files::{FileListBuilder, FileOrder, FilePurpose};

        let builder = FileListBuilder::new()
            .purpose(FilePurpose::Assistants)
            .limit(10)
            .order(FileOrder::Desc);

        assert!(builder.purpose_ref().is_some());
        assert_eq!(builder.limit_ref(), Some(10));
        assert!(builder.order_ref().is_some());
    }

    #[test]
    fn test_file_retrieval_builder() {
        use crate::builders::files::FileRetrievalBuilder;

        let builder = FileRetrievalBuilder::new("file-123");
        assert_eq!(builder.file_id(), "file-123");
    }

    #[test]
    fn test_file_delete_builder() {
        use crate::builders::files::FileDeleteBuilder;

        let builder = FileDeleteBuilder::new("file-456");
        assert_eq!(builder.file_id(), "file-456");
    }

    #[test]
    fn test_file_purpose_display() {
        use crate::builders::files::FilePurpose;

        assert_eq!(FilePurpose::FineTune.to_string(), "fine-tune");
        assert_eq!(FilePurpose::Assistants.to_string(), "assistants");
        assert_eq!(FilePurpose::Vision.to_string(), "vision");
        assert_eq!(FilePurpose::Batch.to_string(), "batch");
    }

    #[test]
    fn test_vector_store_builder_basic() {
        use crate::builders::vector_stores::VectorStoreBuilder;

        let builder = VectorStoreBuilder::new()
            .name("Test Store")
            .add_file("file-1")
            .metadata("key", "value");

        assert_eq!(builder.name_ref(), Some("Test Store"));
        assert_eq!(builder.file_count(), 1);
        assert!(builder.has_files());
        assert_eq!(builder.metadata_ref().len(), 1);
    }

    #[test]
    fn test_vector_store_builder_with_expiration() {
        use crate::builders::vector_stores::VectorStoreBuilder;

        let builder = VectorStoreBuilder::new()
            .name("Temp Store")
            .expires_after_days(30);

        assert_eq!(builder.name_ref(), Some("Temp Store"));
        assert!(builder.expires_after_ref().is_some());
        assert_eq!(builder.expires_after_ref().unwrap().days, 30);
    }

    #[test]
    fn test_vector_store_builder_multiple_files() {
        use crate::builders::vector_stores::VectorStoreBuilder;

        let files = vec!["file-1".to_string(), "file-2".to_string()];
        let builder = VectorStoreBuilder::new()
            .name("Multi-File Store")
            .file_ids(files.clone());

        assert_eq!(builder.file_ids_ref(), files.as_slice());
        assert_eq!(builder.file_count(), 2);
    }

    #[test]
    fn test_vector_store_file_builder() {
        use crate::builders::vector_stores::VectorStoreFileBuilder;

        let builder = VectorStoreFileBuilder::new("vs-123", "file-456");
        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.file_id(), "file-456");
    }

    #[test]
    fn test_vector_store_search_builder() {
        use crate::builders::vector_stores::VectorStoreSearchBuilder;

        let builder = VectorStoreSearchBuilder::new("vs-123", "test query")
            .limit(10)
            .filter("category", "docs");

        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.query(), "test query");
        assert_eq!(builder.limit_ref(), Some(10));
        assert_eq!(builder.filter_ref().len(), 1);
    }

    #[test]
    fn test_vector_store_search_builder_default() {
        use crate::builders::vector_stores::VectorStoreSearchBuilder;

        let builder = VectorStoreSearchBuilder::new("vs-123", "query");
        assert!(builder.limit_ref().is_none());
        assert!(builder.filter_ref().is_empty());
    }
}

// Placeholder client types for different API endpoints
// TODO: Implement these properly once the builders are ready

/// Client for assistants API.
#[derive(Debug, Clone, Copy)]
pub struct AssistantsClient<'a, T = ()> {
    client: &'a Client<T>,
}

impl<T: Default + Send + Sync> AssistantsClient<'_, T> {
    /// Create a new assistant.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::AssistantBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("Math Tutor")
    ///     .instructions("You are a helpful math tutor.");
    /// let assistant = client.assistants().create(builder).await?;
    /// println!("Created assistant: {}", assistant.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: AssistantBuilder) -> Result<AssistantObject> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_CREATE;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_assistant()
            .configuration(&self.client.base_configuration)
            .create_assistant_request(request)
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

    /// List assistants with pagination.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.assistants().list(Some(20), None, None, None).await?;
    /// println!("Found {} assistants", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListAssistantsResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_LIST;
        let model = "assistants";
        let request_json = format!(
            "{{\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_assistants()
            .configuration(&self.client.base_configuration)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
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

    /// Get an assistant by ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let assistant = client.assistants().get("asst_123").await?;
    /// println!("Assistant: {}", assistant.name.unwrap_or_default());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, assistant_id: impl Into<String>) -> Result<AssistantObject> {
        let id = assistant_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_RETRIEVE;
        let model = "assistants";
        let request_json = format!("{{\"assistant_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
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

    /// Update an assistant.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::AssistantBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("Updated Name")
    ///     .instructions("Updated instructions");
    /// let assistant = client.assistants().update("asst_123", builder).await?;
    /// println!("Updated: {}", assistant.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        assistant_id: impl Into<String>,
        builder: AssistantBuilder,
    ) -> Result<AssistantObject> {
        use openai_client_base::models::ModifyAssistantRequest;

        let id = assistant_id.into();
        let request_data = builder.build()?;

        // Convert CreateAssistantRequest to ModifyAssistantRequest
        let mut request = ModifyAssistantRequest::new();
        request.model = Some(request_data.model);
        // Convert Box<CreateAssistantRequestName> to Option<String> by extracting text
        request.name = request_data.name.and_then(|n| match *n {
            openai_client_base::models::CreateAssistantRequestName::Text(text) => Some(Some(text)),
            openai_client_base::models::CreateAssistantRequestName::Null => None,
        });
        request.description = request_data.description.and_then(|d| match *d {
            openai_client_base::models::CreateAssistantRequestDescription::Text(text) => {
                Some(Some(text))
            }
            openai_client_base::models::CreateAssistantRequestDescription::Null => None,
        });
        request.instructions = request_data.instructions.and_then(|i| match *i {
            openai_client_base::models::CreateAssistantRequestInstructions::Text(text) => {
                Some(Some(text))
            }
            openai_client_base::models::CreateAssistantRequestInstructions::Null => None,
        });
        request.tools = request_data.tools;
        request.metadata = request_data.metadata;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_UPDATE;
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "assistants".to_string(), Clone::clone);
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::modify_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
            .modify_assistant_request(request)
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

    /// Delete an assistant.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.assistants().delete("asst_123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, assistant_id: impl Into<String>) -> Result<DeleteAssistantResponse> {
        let id = assistant_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_DELETE;
        let model = "assistants";
        let request_json = format!("{{\"assistant_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::delete_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
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

    /// Create a run on a thread.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::RunBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = RunBuilder::new("asst_123");
    /// let run = client.assistants().create_run("thread_123", builder).await?;
    /// println!("Run created: {}", run.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_run(
        &self,
        thread_id: impl Into<String>,
        builder: RunBuilder,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_CREATE;
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "runs".to_string(), Clone::clone);
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .create_run_request(request)
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

    /// List runs on a thread.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.assistants().list_runs("thread_123", None, None, None, None).await?;
    /// println!("Found {} runs", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_runs(
        &self,
        thread_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListRunsResponse> {
        let thread_id = thread_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_LIST;
        let model = "runs";
        let request_json = format!(
            "{{\"thread_id\":\"{thread_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_runs()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
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

    /// Get a run.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let run = client.assistants().get_run("thread_123", "run_123").await?;
    /// println!("Run status: {}", run.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_run(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_RETRIEVE;
        let model = "runs";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
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

    /// Cancel a run.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let run = client.assistants().cancel_run("thread_123", "run_123").await?;
    /// println!("Run cancelled: {}", run.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_run(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_CANCEL;
        let model = "runs";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::cancel_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
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

    /// Submit tool outputs to a run.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let outputs = vec![
    ///     SubmitToolOutputsRunRequestToolOutputsInner::new("call_123", "output data")
    /// ];
    /// let run = client.assistants().submit_tool_outputs("thread_123", "run_123", outputs).await?;
    /// println!("Tool outputs submitted: {}", run.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn submit_tool_outputs(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        tool_outputs: Vec<SubmitToolOutputsRunRequestToolOutputsInner>,
    ) -> Result<RunObject> {
        use openai_client_base::models::SubmitToolOutputsRunRequest;

        let thread_id = thread_id.into();
        let run_id = run_id.into();
        let request = SubmitToolOutputsRunRequest::new(tool_outputs);

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_SUBMIT_TOOL_OUTPUTS;
        let model = "runs";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::submit_tool_ouputs_to_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .submit_tool_outputs_run_request(request)
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

    /// Create a message on a thread.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::MessageBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let builder = MessageBuilder::new("user", "Hello, assistant!");
    /// let message = client.assistants().create_message("thread_123", builder).await?;
    /// println!("Message created: {}", message.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_message(
        &self,
        thread_id: impl Into<String>,
        builder: MessageBuilder,
    ) -> Result<MessageObject> {
        let thread_id = thread_id.into();
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_CREATE;
        let model = "messages";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_message()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .create_message_request(request)
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

    /// List messages on a thread.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.assistants().list_messages("thread_123", None, None, None, None, None).await?;
    /// println!("Found {} messages", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_messages(
        &self,
        thread_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        run_id: Option<&str>,
    ) -> Result<ListMessagesResponse> {
        let thread_id = thread_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_LIST;
        let model = "messages";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?},\"run_id\":{run_id:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_messages()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_run_id(run_id)
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

    /// Get a message.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let message = client.assistants().get_message("thread_123", "msg_123").await?;
    /// println!("Message role: {}", message.role);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_message(
        &self,
        thread_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> Result<MessageObject> {
        let thread_id = thread_id.into();
        let message_id = message_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_RETRIEVE;
        let model = "messages";
        let request_json =
            format!("{{\"thread_id\":\"{thread_id}\",\"message_id\":\"{message_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_message()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .message_id(&message_id)
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

    /// List run steps.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let response = client.assistants().list_run_steps("thread_123", "run_123", None, None, None, None, None).await?;
    /// println!("Found {} run steps", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn list_run_steps(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        include: Option<Vec<String>>,
    ) -> Result<ListRunStepsResponse> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_STEP_LIST;
        let model = "run_steps";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?},\"include\":{include:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_run_steps()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_include_left_square_bracket_right_square_bracket(include)
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

    /// Get a run step.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
    /// let step = client.assistants().get_run_step("thread_123", "run_123", "step_123", None).await?;
    /// println!("Step type: {}", step.type_);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_run_step(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        step_id: impl Into<String>,
        include: Option<Vec<String>>,
    ) -> Result<RunStepObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();
        let step_id = step_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_STEP_RETRIEVE;
        let model = "run_steps";
        let request_json = format!(
            "{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\",\"step_id\":\"{step_id}\",\"include\":{include:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_run_step()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .step_id(&step_id)
            .maybe_include_left_square_bracket_right_square_bracket(include)
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

/// Client for audio API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct AudioClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for embeddings API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct EmbeddingsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for images API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ImagesClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for files API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FilesClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for fine-tuning API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FineTuningClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for batch API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BatchClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for vector stores API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct VectorStoresClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for moderations API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ModerationsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for threads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ThreadsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for uploads API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct UploadsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for models API.
#[derive(Debug, Clone, Copy)]
pub struct ModelsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for completions API.
#[derive(Debug, Clone, Copy)]
pub struct CompletionsClient<'a, T = ()> {
    client: &'a Client<T>,
}

/// Client for usage API.
#[derive(Debug, Clone, Copy)]
pub struct UsageClient<'a, T = ()> {
    client: &'a Client<T>,
}

// Apply interceptor helper methods to all sub-clients
impl_interceptor_helpers!(AssistantsClient<'_, T>);
impl_interceptor_helpers!(AudioClient<'_, T>);
impl_interceptor_helpers!(EmbeddingsClient<'_, T>);
impl_interceptor_helpers!(ImagesClient<'_, T>);
impl_interceptor_helpers!(FilesClient<'_, T>);
impl_interceptor_helpers!(FineTuningClient<'_, T>);
impl_interceptor_helpers!(BatchClient<'_, T>);
impl_interceptor_helpers!(VectorStoresClient<'_, T>);
impl_interceptor_helpers!(ModerationsClient<'_, T>);
impl_interceptor_helpers!(ThreadsClient<'_, T>);
impl_interceptor_helpers!(UploadsClient<'_, T>);
impl_interceptor_helpers!(ModelsClient<'_, T>);
impl_interceptor_helpers!(CompletionsClient<'_, T>);
impl_interceptor_helpers!(UsageClient<'_, T>);

impl<T: Default + Send + Sync> ModelsClient<'_, T> {
    /// List all available models.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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

impl<T: Default + Send + Sync> CompletionsClient<'_, T> {
    /// Create a completions builder for the specified model.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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

impl<T: Default + Send + Sync> UsageClient<'_, T> {
    /// Get usage data for audio speeches.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::usage::UsageBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?;
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
