//! Client wrapper for ergonomic `OpenAI` API access.
//!
//! This module provides a high-level client that wraps the base `OpenAI` client
//! with ergonomic builders and response handling.

// Allow this lint at module level for interceptor helper methods
// that require many parameters for comprehensive context passing
#![allow(clippy::too_many_arguments)]

use crate::builders::{Builder, ChatCompletionBuilder};
use crate::config::Config;
use crate::errors::Result;
use crate::interceptor::{
    AfterResponseContext, BeforeRequestContext, ErrorContext, InterceptorChain,
};
use crate::rate_limits::LatestRateLimits;
use crate::responses::ChatCompletionResponseWrapper;
use crate::semantic_conventions::operation_names;
use crate::time::Instant;
use crate::Error;
use openai_client_base::apis::chat_api;
use openai_client_base::apis::configuration::Configuration;
use openai_client_base::apis::Error as ApiError;
use openai_client_base::models::CreateChatCompletionRequest;
use std::sync::Arc;
#[cfg(not(target_family = "wasm"))]
use std::time::Duration;
use timvw_rust_runtime::retry::{openai_insufficient_quota, RetryLayer, RetryPolicy};
use timvw_rust_runtime::{Client as HttpClient, ReqwestService};

// Helper macro to generate interceptor helper methods for sub-clients
macro_rules! impl_interceptor_helpers {
    ($client_type:ty) => {
        impl<T: Default + crate::MaybeSend + crate::MaybeSync> $client_type {
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
mod headers;
mod images;
mod large_upload;
mod media;
mod models;
mod moderations;
mod raw;
mod responses;
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
pub use large_upload::{LargeUpload, DEFAULT_UPLOAD_CONCURRENCY, MAX_UPLOAD_PART_SIZE};
pub use models::ModelsClient;
pub use moderations::ModerationsClient;
pub use responses::ResponsesClient;
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
/// impl Interceptor for MyInterceptor {}
///
/// # fn example() -> openai_ergonomic::Result<()> {
/// let client = Client::from_env()?
///     .with_interceptor(MyInterceptor)
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct ClientBuilder<T = ()> {
    config: Arc<Config>,
    http: HttpClient,
    base_configuration: Configuration,
    interceptors: InterceptorChain<T>,
    rate_limits: LatestRateLimits,
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
    rate_limits: LatestRateLimits,
}

// Custom Debug implementation since InterceptorChain doesn't implement Debug
impl<T> std::fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("config", &self.config)
            .field("http", &"<HttpClient>")
            .field("base_configuration", &"<Configuration>")
            .field("interceptors", &"<InterceptorChain>")
            .field("rate_limits", &self.rate_limits)
            .finish()
    }
}

// Implementation for ClientBuilder with default state type ()
impl ClientBuilder {
    /// Create a new client builder with the given configuration.
    ///
    /// The HTTP client is the configured one, or a reqwest client with a 120 second timeout and
    /// retries ([`Config::max_retries`]); on WebAssembly a reqwest client with retries (the
    /// host's `fetch` owns timeouts). Around it go, from the inside out: the
    /// [`AuthProvider`](crate::auth::AuthProvider)'s layer when one is set, then the
    /// [provider](Config::provider)'s layer (base URL, credentials, headers, query parameters,
    /// paths) with the organization and project headers.
    pub fn new(config: Config) -> Result<Self> {
        // Use custom HTTP client if provided, otherwise build a default one
        #[cfg_attr(target_family = "wasm", allow(clippy::option_if_let_else))]
        let mut http_client = if let Some(client) = config.http_client() {
            client.clone()
        } else {
            // On WebAssembly reqwest sends with the host's `fetch`, which owns timeouts and the
            // user agent: a plain client.
            #[cfg(target_family = "wasm")]
            let reqwest_client = reqwest::Client::new();
            #[cfg(not(target_family = "wasm"))]
            let reqwest_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120)) // Default timeout: 120 seconds
                .user_agent(format!("openai-ergonomic/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| Error::Config(format!("failed to build the HTTP client: {e}")))?;

            // Retry transient failures (429, 408, 5xx, failed connections); 0 disables retries. A 429
            // for an exhausted quota (`insufficient_quota`) is permanent: it is returned, not retried.
            let max_retries = usize::try_from(config.max_retries()).unwrap_or(usize::MAX);
            HttpClient::from_service(ReqwestService::new(reqwest_client)).layer(
                RetryLayer::new(RetryPolicy::new(max_retries))
                    .permanent_429(openai_insufficient_quota),
            )
        };

        let provider = config.provider();

        // Per-request credentials, refreshed after a 401; only for requests to the provider.
        if let Some(auth) = config.auth_provider() {
            http_client = http_client.layer(
                crate::auth::AuthLayer::from_shared(auth.clone()).scope(provider.api_base())?,
            );
        }

        // Send every request under the provider's base URL there, with its credentials,
        // headers, query parameters and paths, and the organization and project headers. The
        // generated openai-client-base operations go through it too.
        http_client =
            http_client.layer(provider.layer_with(headers::organization_headers(&config)?)?);

        // Record the rate limits of every response (`Client::rate_limits`).
        let rate_limits = LatestRateLimits::default();
        http_client = http_client.layer(rate_limits.layer());

        // Create openai-client-base configuration
        let mut base_configuration = Configuration::new();
        base_configuration.client = http_client.clone();
        base_configuration.user_agent =
            Some(format!("openai-ergonomic/{}", env!("CARGO_PKG_VERSION")));
        // The provider's layer authenticates the requests.
        base_configuration.bearer_access_token = None;
        base_configuration.base_path = provider.api_base().to_string();

        Ok(Self {
            config: Arc::new(config),
            http: http_client,
            base_configuration,
            interceptors: InterceptorChain::new(),
            rate_limits,
        })
    }

    /// Create a new client builder with default configuration from environment variables.
    ///
    /// Not available on WebAssembly ([`Config::from_env`]).
    #[cfg(not(target_family = "wasm"))]
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
    /// The interceptor is taken by value; `Box<I>` and `Arc<I>` are accepted too.
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
    /// impl Interceptor for LoggingInterceptor {
    ///     async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> {
    ///         println!("Calling {}", ctx.operation);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # fn example() -> Result<()> {
    /// let client = Client::from_env()?
    ///     .with_interceptor(LoggingInterceptor)
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
    ///     .with_interceptor(interceptor)
    ///     .build();
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[must_use]
    pub fn with_interceptor<U, I>(self, interceptor: I) -> ClientBuilder<U>
    where
        I: crate::interceptor::Interceptor<U> + 'static,
    {
        let mut new_chain = InterceptorChain::new();
        new_chain.add(interceptor);

        ClientBuilder {
            config: self.config,
            http: self.http,
            base_configuration: self.base_configuration,
            interceptors: new_chain,
            rate_limits: self.rate_limits,
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
    /// impl Interceptor for Logger {}
    /// impl Interceptor for Metrics {}
    ///
    /// # fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?
    ///     .add_interceptor(Logger)
    ///     .add_interceptor(Metrics)
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn add_interceptor<I>(mut self, interceptor: I) -> Self
    where
        I: crate::interceptor::Interceptor<T> + 'static,
    {
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
            rate_limits: self.rate_limits,
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
    ///
    /// Not available on WebAssembly ([`Config::from_env`]).
    #[cfg(not(target_family = "wasm"))]
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

    /// The rate limits of the latest response that reported them
    /// (`x-ratelimit-{limit,remaining,reset}-{requests,tokens}`), for any request of this
    /// client or its clones, streaming ones included. `None` before the first such response
    /// (some providers, such as Azure, send other headers).
    #[must_use]
    pub fn rate_limits(&self) -> Option<crate::RateLimitSnapshot> {
        self.rate_limits.get()
    }
}

// Interceptor helper methods
impl<T: Default + crate::MaybeSend + crate::MaybeSync> Client<T> {
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
impl<T: Default + crate::MaybeSend + crate::MaybeSync + 'static> Client<T> {
    /// Create a chat completion builder.
    pub fn chat(&self) -> ChatCompletionBuilder {
        let model = self
            .config
            .default_model()
            .unwrap_or(crate::config::DEFAULT_MODEL);
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
            .create_chat_completion_request(&request)
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
            response.usage.as_ref().map(|u| u.prompt_tokens),
            response.usage.as_ref().map(|u| u.completion_tokens),
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

        // Sent as a raw request and decoded here rather than through the generated stream, so
        // that a parse error carries the offending chunk.
        let response = self
            .send_event_stream_request(http::Method::POST, "/chat/completions", &request)
            .await?;
        let stream = crate::time::idle_timeout(
            crate::streaming::ChatCompletionStream::new(response),
            self.config.stream_idle_timeout(),
        );

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

// TODO: Add methods for other API endpoints
impl<T: Default + crate::MaybeSend + crate::MaybeSync> Client<T> {
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
        ApiError::Http(err) => Error::Http(err),
        ApiError::Serde(err) => Error::Json(err),
        ApiError::ResponseError(response) => {
            // The `type` and `code` of an `OpenAI` error body, as for raw requests.
            let (error_type, error_code) = raw::openai_error_details(&response.content);
            Error::Api {
                status: response.status.as_u16(),
                message: response.content,
                error_type,
                error_code,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use openai_client_base::apis::{Error as BaseError, ResponseContent};

    #[test]
    fn map_api_error_keeps_the_openai_error_type_and_code() {
        let response = ResponseContent {
            status: http::StatusCode::TOO_MANY_REQUESTS,
            content: r#"{"error":{"message":"slow down","type":"requests","code":"rate_limit_exceeded"}}"#
                .to_string(),
            headers: http::HeaderMap::new(),
            entity: Option::<()>::None,
        };
        let error = map_api_error(BaseError::ResponseError(Box::new(response)));
        assert!(error.is_rate_limit());
        assert!(matches!(
            error,
            Error::Api { error_type: Some(t), error_code: Some(c), .. }
                if t == "requests" && c == "rate_limit_exceeded"
        ));
    }

    #[test]
    fn map_api_error_converts_response() {
        let response = ResponseContent {
            status: http::StatusCode::BAD_REQUEST,
            content: "bad request".to_string(),
            headers: http::HeaderMap::new(),
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

        assert_eq!(request.input, "Test content".into());
        assert!(request.model.is_none());
    }

    #[test]
    fn test_moderation_builder_with_model() {
        use crate::builders::moderations::ModerationBuilder;

        let builder = ModerationBuilder::new("Test content").model("text-moderation-stable");
        let request = builder.build().unwrap();

        assert_eq!(request.input, "Test content".into());
        assert_eq!(request.model, Some("text-moderation-stable".to_string()));
    }

    #[test]
    fn test_moderation_builder_array_input() {
        use crate::builders::moderations::ModerationBuilder;

        let inputs = vec!["First text".to_string(), "Second text".to_string()];
        let builder = ModerationBuilder::new_array(inputs);
        let request = builder.build().unwrap();

        // Each text is its own input
        assert_eq!(
            serde_json::to_value(&request.input).unwrap(),
            serde_json::json!(["First text", "Second text"])
        );
    }

    #[test]
    fn test_file_upload_builder_creation() {
        use crate::builders::files::{FilePurpose, FileUploadBuilder};

        let content = b"test content".to_vec();
        let builder = FileUploadBuilder::new("test.txt", FilePurpose::Assistants, content.clone());

        assert_eq!(builder.filename(), "test.txt");
        assert_eq!(builder.content(), Some(content.as_slice()));
        assert_eq!(builder.content_size(), Some(content.len() as u64));
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
