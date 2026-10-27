//! Client wrapper for ergonomic `OpenAI` API access.
//!
//! This module provides a high-level client that wraps the base `OpenAI` client
//! with ergonomic builders and response handling.

// Allow this lint at module level for interceptor helper methods
// that require many parameters for comprehensive context passing
#![allow(clippy::too_many_arguments)]

use crate::builders::{Builder, ChatCompletionBuilder, ResponsesBuilder};
use crate::config::Config;
use crate::errors::Result;
use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, InterceptorChain,
};
use crate::responses::ChatCompletionResponseWrapper;
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::chat_api;
use openai_client_base::apis::configuration::Configuration;
use openai_client_base::apis::Error as ApiError;
use openai_client_base::models::CreateChatCompletionRequest;
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

mod assistants;
mod audio;
mod batch;
mod completions;
mod embeddings;
mod files;
mod fine_tuning;
mod images;
mod models;
mod moderations;
mod threads;
mod uploads;
mod usage;
mod vector_stores;

pub use assistants::AssistantsClient;
pub use audio::AudioClient;
pub use batch::BatchClient;
pub use completions::CompletionsClient;
pub use embeddings::EmbeddingsClient;
pub use files::FilesClient;
pub use fine_tuning::FineTuningClient;
pub use images::ImagesClient;
pub use models::ModelsClient;
pub use moderations::ModerationsClient;
pub use threads::ThreadsClient;
pub use uploads::UploadsClient;
pub use usage::UsageClient;
pub use vector_stores::VectorStoresClient;

/// Builder for creating a `Client` with interceptors.
///
/// The builder pattern allows you to configure interceptors before the client
/// is created. Once built, the interceptors are immutable, eliminating the need
/// for runtime locking.
///
/// # Example
///
/// ```rust,no_run
/// use openai_ergonomic::{Client, Interceptor};
///
/// struct MyInterceptor;
///
/// #[async_trait::async_trait]
/// impl Interceptor for MyInterceptor {}
///
/// # fn example() -> openai_ergonomic::Result<()> {
/// let client = Client::from_env()?
///     .with_interceptor(Box::new(MyInterceptor))
///     .build();
/// # Ok(())
/// # }
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
/// ```rust,no_run
/// # use openai_ergonomic::{Client, Config};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::from_env()?.build();
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

        // Set the custom HTTP client (with Azure middleware if configured)
        base_configuration.client = http_client.clone();

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
    /// ```rust,no_run
    /// use openai_ergonomic::{Client, Interceptor, BeforeRequestContext, Result};
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
    /// # fn example() -> Result<()> {
    /// let client = Client::from_env()?
    ///     .with_interceptor(Box::new(LoggingInterceptor))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Interceptor with custom state:
    /// ```rust,no_run
    /// # #[cfg(feature = "otel")]
    /// # mod demo {
    /// use openai_ergonomic::{Client, LangfuseConfig, LangfuseInterceptor, LangfuseState};
    /// use opentelemetry::global;
    ///
    /// # fn example() -> openai_ergonomic::Result<()> {
    /// let tracer = global::tracer("openai-ergonomic");
    /// let interceptor = LangfuseInterceptor::new(tracer, LangfuseConfig::new());
    /// let client: Client<LangfuseState<_>> = Client::from_env()?
    ///     .with_interceptor(Box::new(interceptor))
    ///     .build();
    /// # Ok(())
    /// # }
    /// # }
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
    /// ```rust,no_run
    /// use openai_ergonomic::{Client, Interceptor};
    ///
    /// struct Logger;
    /// struct Metrics;
    ///
    /// #[async_trait::async_trait]
    /// impl Interceptor for Logger {}
    /// #[async_trait::async_trait]
    /// impl Interceptor for Metrics {}
    ///
    /// # fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?
    ///     .add_interceptor(Box::new(Logger))
    ///     .add_interceptor(Box::new(Metrics))
    ///     .build();
    /// # Ok(())
    /// # }
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
impl<T: Default + Send + Sync + 'static> Client<T> {
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

    /// Send a chat completion request with streaming enabled.
    ///
    /// Returns a stream of chat completion chunks as they are generated.
    /// This allows for real-time display of the model's response.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::from_env()?.build();
    ///
    ///     let builder = client.chat().user("Tell me a story");
    ///     let mut stream = client.send_chat_stream(builder).await?;
    ///
    ///     while let Some(chunk) = stream.next().await {
    ///         let chunk = chunk?;
    ///         if let Some(content) = chunk.content() {
    ///             print!("{}", content);
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send_chat_stream(
        &self,
        builder: ChatCompletionBuilder,
    ) -> Result<crate::streaming::BoxedChatStream> {
        self.send_chat_stream_with_completion(builder)
            .await
            .map(|(stream, _completion)| stream)
    }

    /// Send a streaming chat request and return a handle for waiting on interceptor callbacks.
    ///
    /// `StreamCompletion::wait` resolves after the stream reaches EOF, yields an error, or is
    /// dropped, and every accepted callback has returned or failed. If the consumer stops early,
    /// drop the stream before waiting so the terminal outcome is cancellation.
    pub async fn send_chat_stream_with_completion(
        &self,
        mut builder: ChatCompletionBuilder,
    ) -> Result<(
        crate::streaming::BoxedChatStream,
        crate::streaming::StreamCompletion,
    )> {
        // Force streaming mode
        builder = builder.stream(true);
        let mut request = builder.build()?;
        request.stream = Some(Some(true));

        self.execute_chat_stream(request, crate::semantic_conventions::operation_names::CHAT)
            .await
    }

    /// Execute a chat completion request with streaming.
    ///
    /// This is the low-level method that performs the actual streaming request.
    async fn execute_chat_stream(
        &self,
        request: CreateChatCompletionRequest,
        operation: &str,
    ) -> Result<(
        crate::streaming::BoxedChatStream,
        crate::streaming::StreamCompletion,
    )> {
        let uri_str = format!("{}/chat/completions", self.config.api_base());

        let mut req_builder = self
            .http_client()
            .request(reqwest::Method::POST, &uri_str)
            .bearer_auth(self.config.api_key())
            .json(&request);

        // Add organization ID if present
        if let Some(org_id) = self.config.organization_id() {
            req_builder = req_builder.header("OpenAI-Organization", org_id);
        }

        // Add project ID if present
        if let Some(project_id) = self.config.project() {
            req_builder = req_builder.header("OpenAI-Project", project_id);
        }

        let req = req_builder.build()?;

        // Serialize request for interceptors
        let request_json = serde_json::to_string(&request).unwrap_or_else(|_| "{}".to_string());
        let model = request.model.clone();

        // Call before_request hook if interceptors are present
        let mut state = T::default();
        if !self.interceptors.is_empty() {
            let mut ctx = crate::interceptor::BeforeRequestContext {
                operation,
                model: &model,
                request_json: &request_json,
                state: &mut state,
            };
            self.interceptors.before_request(&mut ctx).await?;
        }

        let response = self.http_client().execute(req).await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: error_text,
                error_type: None,
                error_code: None,
            });
        }

        let stream = crate::streaming::ChatCompletionStream::new(response);

        let (intercepted, completion) = crate::streaming::InterceptedStream::new_with_completion(
            stream,
            std::sync::Arc::clone(&self.interceptors),
            operation.to_string(),
            model,
            request_json,
            state,
        )?;
        Ok((Box::pin(intercepted), completion))
    }
}

// Responses API methods
impl<T: Default + Send + Sync + 'static> Client<T> {
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

    /// Send a responses request with streaming enabled.
    ///
    /// This enables real-time streaming of responses using Server-Sent Events (SSE).
    /// The stream yields chunks as they arrive from the API.
    pub async fn send_responses_stream(
        &self,
        builder: ResponsesBuilder,
    ) -> Result<crate::streaming::BoxedChatStream> {
        self.send_responses_stream_with_completion(builder)
            .await
            .map(|(stream, _completion)| stream)
    }

    /// Send a streaming Responses request and return a handle for waiting on interceptor callbacks.
    ///
    /// `StreamCompletion::wait` resolves after the stream reaches EOF, yields an error, or is
    /// dropped, and every accepted callback has returned or failed. If the consumer stops early,
    /// drop the stream before waiting so the terminal outcome is cancellation.
    pub async fn send_responses_stream_with_completion(
        &self,
        mut builder: ResponsesBuilder,
    ) -> Result<(
        crate::streaming::BoxedChatStream,
        crate::streaming::StreamCompletion,
    )> {
        // Force streaming mode
        builder = builder.stream(true);
        let mut request = builder.build()?;
        request.stream = Some(Some(true));

        // The Responses API uses the same streaming endpoint as chat
        self.execute_chat_stream(
            request,
            crate::semantic_conventions::operation_names::RESPONSES,
        )
        .await
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

        let error = map_api_error(BaseError::ResponseError(Box::new(response)));
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
