//! Client wrapper for ergonomic `OpenAI` API access.
//!
//! This module provides a high-level client that wraps the base `OpenAI` client
//! with ergonomic builders and response handling.

use crate::{config::Config, errors::Result, Error};
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
/// ```rust
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
    http_client: HttpClient,
    // TODO: Add openai-client-base client once available
    // base_client: openai_client_base::Client,
}

impl Client {
    /// Create a new client with the given configuration.
    pub fn new(config: Config) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(config.timeout_seconds()))
            .user_agent(format!("openai-ergonomic/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        Ok(Self {
            config: Arc::new(config),
            http_client,
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
        &self.http_client
    }
}

// Chat API methods
impl Client {
    /// Create a chat completion builder.
    pub fn chat(&self) -> crate::builders::ChatCompletionBuilder {
        // TODO: Use default model from config
        crate::builders::ChatCompletionBuilder::new("gpt-4")
    }

    /// Create a chat completion with a simple user message.
    pub fn chat_simple(
        &self,
        message: impl Into<String>,
    ) -> crate::builders::ChatCompletionBuilder {
        self.chat().user(message)
    }

    /// Create a chat completion with system and user messages.
    pub fn chat_with_system(
        &self,
        system: impl Into<String>,
        user: impl Into<String>,
    ) -> crate::builders::ChatCompletionBuilder {
        self.chat().system(system).user(user)
    }
}

// Responses API methods
impl Client {
    /// Create a responses builder for structured outputs.
    pub fn responses(&self) -> crate::responses::ResponseBuilder {
        // TODO: Use default model from config
        crate::responses::ResponseBuilder::new("gpt-4")
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
