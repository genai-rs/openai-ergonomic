# Builder Patterns

> Comprehensive documentation of builder patterns, helper functions, type safety, and async patterns used throughout the openai-ergonomic crate.

## Table of Contents

- [Core Builder Patterns](#core-builder-patterns)
- [Helper Function Conventions](#helper-function-conventions)
- [Type Safety Patterns](#type-safety-patterns)
- [Async/Await Patterns](#asyncawait-patterns)
- [Configuration Patterns](#configuration-patterns)
- [Extension Trait Patterns](#extension-trait-patterns)

## Core Builder Patterns

### The Builder Trait

All builders in the crate implement a common `Builder` trait that provides consistent behavior:

```rust
/// Core trait for request builders
pub trait Builder<T> {
    type Output;

    /// Execute the request and return the response
    async fn send(self) -> Result<Self::Output>;
}

/// Trait for builders that support streaming
pub trait StreamableBuilder<T>: Builder<T> {
    type Stream: Stream<Item = Result<T>>;

    /// Execute the request and return a stream
    async fn stream(self) -> Result<Self::Stream>;
}

/// Simplified trait for basic sendable requests
pub trait Sendable {
    type Response;

    async fn send(self) -> Result<Self::Response>;
}
```

### Fluent Interface Pattern

All builders use the fluent interface pattern for method chaining:

```rust
pub struct ResponsesBuilder {
    request: CreateResponsesRequest,
    client: Arc<Client>,
}

impl ResponsesBuilder {
    /// Add a system message (consumes and returns self)
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(ResponsesMessage::system(content.into()));
        self
    }

    /// Add a user message
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(ResponsesMessage::user(content.into()));
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.request.model = model.into();
        self
    }

    /// Set temperature with validation
    pub fn temperature(mut self, temp: f64) -> Result<Self> {
        if temp < 0.0 || temp > 2.0 {
            return Err(Error::Validation {
                field: "temperature".to_string(),
                message: "must be between 0.0 and 2.0".to_string(),
            });
        }
        self.request.temperature = Some(temp);
        Ok(self)
    }
}

// Usage example:
let response = client
    .responses()
    .system("You are a helpful assistant")
    .user("What is Rust?")
    .model(constants::models::GPT_4_TURBO)
    .temperature(0.7)?
    .send()
    .await?;
```

### Builder State Pattern

Use phantom types to enforce correct builder usage at compile time:

```rust
use std::marker::PhantomData;

/// Marker traits for builder states
pub mod states {
    pub trait BuilderState {}

    /// Builder has no messages yet
    pub struct Empty;
    impl BuilderState for Empty {}

    /// Builder has at least one message
    pub struct WithMessages;
    impl BuilderState for WithMessages {}

    /// Builder is ready to execute
    pub struct Ready;
    impl BuilderState for Ready {}
}

pub struct ResponsesBuilder<State = states::Empty> {
    request: CreateResponsesRequest,
    client: Arc<Client>,
    _state: PhantomData<State>,
}

impl ResponsesBuilder<states::Empty> {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            request: CreateResponsesRequest::default(),
            client,
            _state: PhantomData,
        }
    }

    /// Adding first message transitions to WithMessages state
    pub fn system(mut self, content: impl Into<String>) -> ResponsesBuilder<states::WithMessages> {
        self.request.messages.push(ResponsesMessage::system(content.into()));
        ResponsesBuilder {
            request: self.request,
            client: self.client,
            _state: PhantomData,
        }
    }
}

impl ResponsesBuilder<states::WithMessages> {
    /// Can add more messages
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(ResponsesMessage::user(content.into()));
        self
    }

    /// Set model and transition to Ready state
    pub fn model(mut self, model: impl Into<String>) -> ResponsesBuilder<states::Ready> {
        self.request.model = model.into();
        ResponsesBuilder {
            request: self.request,
            client: self.client,
            _state: PhantomData,
        }
    }
}

// Only Ready builders can be executed
impl ResponsesBuilder<states::Ready> {
    pub async fn send(self) -> Result<ResponsesResponse> {
        self.client.create_responses(self.request).await
    }
}
```

### Optional Configuration Pattern

Handle optional parameters gracefully:

```rust
impl ResponsesBuilder {
    /// Optional parameters use Option internally but provide ergonomic API
    pub fn max_tokens(mut self, tokens: impl Into<Option<u32>>) -> Self {
        self.request.max_tokens = tokens.into();
        self
    }

    /// Provide both Some() and None() convenience methods
    pub fn with_max_tokens(self, tokens: u32) -> Self {
        self.max_tokens(Some(tokens))
    }

    pub fn without_max_tokens(self) -> Self {
        self.max_tokens(None)
    }

    /// Boolean flags as methods
    pub fn enable_streaming(mut self) -> Self {
        self.request.stream = Some(true);
        self
    }

    pub fn disable_streaming(mut self) -> Self {
        self.request.stream = Some(false);
        self
    }
}
```

## Helper Function Conventions

### Function Naming

Helper functions follow consistent naming patterns:

```rust
// Action + Object pattern
pub fn tool_function(name: &str, description: &str) -> Tool
pub fn tool_web_search(query: &str) -> Tool
pub fn system_message(content: &str) -> ResponsesMessage
pub fn user_message(content: &str) -> ResponsesMessage

// Context-specific helpers
pub fn image_base64_part(base64_data: &str) -> MessageContentPart
pub fn image_url_part(url: &str) -> MessageContentPart
pub fn text_part(content: &str) -> MessageContentPart

// Composite helpers
pub fn system_user(system: &str, user: &str) -> Vec<ResponsesMessage>
pub fn responses_simple(prompt: &str) -> ResponsesBuilder
```

### Helper Implementation Patterns

#### Simple Value Constructors

```rust
/// Create a function tool with validation
pub fn tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
) -> Result<Tool> {
    let name = name.into();

    // Validate function name
    if name.is_empty() {
        return Err(Error::Validation {
            field: "name".to_string(),
            message: "function name cannot be empty".to_string(),
        });
    }

    Ok(Tool {
        type_: "function".to_string(),
        function: Some(FunctionDefinition {
            name,
            description: Some(description.into()),
            parameters: None,
            strict: None,
        }),
    })
}

/// Create a web search tool
pub fn tool_web_search(query: impl Into<String>) -> Tool {
    Tool {
        type_: "web_search".to_string(),
        web_search: Some(WebSearchDefinition {
            query: query.into(),
        }),
    }
}
```

#### Message Construction Helpers

```rust
/// Create message content parts for multimodal inputs
pub fn text_part(content: impl Into<String>) -> MessageContentPart {
    MessageContentPart {
        type_: "text".to_string(),
        text: Some(content.into()),
        image_url: None,
    }
}

pub fn image_url_part(url: impl Into<String>) -> MessageContentPart {
    MessageContentPart {
        type_: "image_url".to_string(),
        text: None,
        image_url: Some(ImageUrl {
            url: url.into(),
            detail: None,
        }),
    }
}

pub fn image_url_part_with_detail(
    url: impl Into<String>,
    detail: Detail,
) -> MessageContentPart {
    MessageContentPart {
        type_: "image_url".to_string(),
        text: None,
        image_url: Some(ImageUrl {
            url: url.into(),
            detail: Some(detail),
        }),
    }
}

/// Composite helpers for common patterns
pub fn user_message_with_image(
    text: impl Into<String>,
    image_url: impl Into<String>,
) -> ResponsesMessage {
    ResponsesMessage {
        role: "user".to_string(),
        content: Some(vec![
            text_part(text),
            image_url_part(image_url),
        ]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}
```

#### Builder Initialization Helpers

```rust
/// Quick builder initialization for common patterns
pub fn responses_simple(prompt: impl Into<String>) -> ResponsesBuilder {
    ResponsesBuilder::new()
        .user(prompt)
        .model(constants::models::GPT_4_TURBO)
}

pub fn responses_system_user(
    system: impl Into<String>,
    user: impl Into<String>,
) -> ResponsesBuilder {
    ResponsesBuilder::new()
        .system(system)
        .user(user)
        .model(constants::models::GPT_4_TURBO)
}

pub fn chat_with_history(messages: Vec<ChatMessage>) -> ChatCompletionBuilder {
    let mut builder = ChatCompletionBuilder::new();
    for message in messages {
        builder = builder.message(message);
    }
    builder.model(constants::models::GPT_4_TURBO)
}
```

## Type Safety Patterns

### Newtype Pattern for Validation

Use newtype wrappers to enforce validation at the type level:

```rust
/// Temperature value that's validated at construction
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature(f64);

impl Temperature {
    pub fn new(value: f64) -> Result<Self> {
        if value < 0.0 || value > 2.0 {
            Err(Error::Validation {
                field: "temperature".to_string(),
                message: format!("value {} must be between 0.0 and 2.0", value),
            })
        } else {
            Ok(Self(value))
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

// Convenient constants
impl Temperature {
    pub const CREATIVE: Temperature = Temperature(0.9);
    pub const BALANCED: Temperature = Temperature(0.7);
    pub const FOCUSED: Temperature = Temperature(0.3);
    pub const DETERMINISTIC: Temperature = Temperature(0.0);
}

impl From<Temperature> for f64 {
    fn from(temp: Temperature) -> f64 {
        temp.0
    }
}

// Usage in builders
impl ResponsesBuilder {
    pub fn temperature(mut self, temp: Temperature) -> Self {
        self.request.temperature = Some(temp.into());
        self
    }
}
```

### Compile-Time Constants

Use const functions and const generics where possible:

```rust
/// Model identifier with compile-time validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelId(&'static str);

impl ModelId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }
}

// Model constants
pub mod models {
    use super::ModelId;

    pub const GPT_4_TURBO: ModelId = ModelId::new("gpt-4-turbo");
    pub const GPT_4: ModelId = ModelId::new("gpt-4");
    pub const GPT_3_5_TURBO: ModelId = ModelId::new("gpt-3.5-turbo");
}

// Generic validation with const parameters
pub struct MaxTokens<const MAX: u32>(u32);

impl<const MAX: u32> MaxTokens<MAX> {
    pub fn new(value: u32) -> Result<Self> {
        if value > MAX {
            Err(Error::Validation {
                field: "max_tokens".to_string(),
                message: format!("value {} exceeds maximum {}", value, MAX),
            })
        } else {
            Ok(Self(value))
        }
    }
}

// Usage:
type GPT4MaxTokens = MaxTokens<128000>;
let tokens = GPT4MaxTokens::new(1000)?;
```

### Phantom Types for Request State

Track request completeness with phantom types:

```rust
pub mod markers {
    pub struct NoMessages;
    pub struct HasMessages;
    pub struct NoModel;
    pub struct HasModel;
}

pub struct ResponsesBuilder<M = markers::NoMessages, O = markers::NoModel> {
    request: CreateResponsesRequest,
    client: Arc<Client>,
    _messages: PhantomData<M>,
    _model: PhantomData<O>,
}

impl ResponsesBuilder<markers::NoMessages, markers::NoModel> {
    pub fn system(
        mut self,
        content: impl Into<String>,
    ) -> ResponsesBuilder<markers::HasMessages, markers::NoModel> {
        self.request.messages.push(ResponsesMessage::system(content.into()));
        ResponsesBuilder {
            request: self.request,
            client: self.client,
            _messages: PhantomData,
            _model: PhantomData,
        }
    }
}

impl<M> ResponsesBuilder<M, markers::NoModel> {
    pub fn model(
        mut self,
        model: impl Into<String>,
    ) -> ResponsesBuilder<M, markers::HasModel> {
        self.request.model = model.into();
        ResponsesBuilder {
            request: self.request,
            client: self.client,
            _messages: PhantomData,
            _model: PhantomData,
        }
    }
}

// Only complete builders can be sent
impl ResponsesBuilder<markers::HasMessages, markers::HasModel> {
    pub async fn send(self) -> Result<ResponsesResponse> {
        self.client.create_responses(self.request).await
    }
}
```

## Async/Await Patterns

### Async Builder Methods

Builders provide both sync configuration and async execution:

```rust
impl ResponsesBuilder {
    // Sync configuration methods return Self
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(ResponsesMessage::user(content.into()));
        self
    }

    // Async execution methods consume self and return Future
    pub async fn send(self) -> Result<ResponsesResponse> {
        self.client
            .execute_request(self.request)
            .await
    }

    pub async fn stream(self) -> Result<ResponsesStream> {
        let mut request = self.request;
        request.stream = Some(true);

        self.client
            .execute_streaming_request(request)
            .await
    }
}
```

### Future Combinators

Provide helpers for common async patterns:

```rust
impl ResponsesBuilder {
    /// Execute multiple requests concurrently
    pub async fn send_batch(builders: Vec<Self>) -> Result<Vec<ResponsesResponse>> {
        let futures = builders.into_iter().map(|b| b.send());
        futures::future::try_join_all(futures).await
    }

    /// Execute with timeout
    pub async fn send_with_timeout(
        self,
        timeout: Duration,
    ) -> Result<ResponsesResponse> {
        tokio::time::timeout(timeout, self.send())
            .await
            .map_err(|_| Error::Timeout { duration: timeout })?
    }

    /// Execute with retries
    pub async fn send_with_retries(
        self,
        max_retries: u32,
    ) -> Result<ResponsesResponse> {
        let mut retries = 0;
        loop {
            match self.clone().send().await {
                Ok(response) => return Ok(response),
                Err(Error::RateLimit { retry_after, .. }) if retries < max_retries => {
                    retries += 1;
                    if let Some(delay) = retry_after {
                        tokio::time::sleep(delay).await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(1 << retries)).await;
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### Stream Processing Patterns

Provide ergonomic helpers for stream processing:

```rust
use futures::Stream;
use tokio_stream::StreamExt;

pub trait ResponseStreamExt: Stream {
    /// Collect all chunks into content
    async fn collect_content(self) -> Result<String>
    where
        Self: Stream<Item = Result<ResponsesChunk>> + Sized,
    {
        let mut content = String::new();
        let mut stream = Box::pin(self);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if let Some(text) = chunk.content() {
                content.push_str(text);
            }
        }

        Ok(content)
    }

    /// Process chunks with callback
    async fn for_each_chunk<F, Fut>(self, mut f: F) -> Result<()>
    where
        Self: Stream<Item = Result<ResponsesChunk>> + Sized,
        F: FnMut(ResponsesChunk) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let mut stream = Box::pin(self);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            f(chunk).await?;
        }

        Ok(())
    }

    /// Collect with usage information
    async fn collect_with_usage(self) -> Result<(String, Option<Usage>)>
    where
        Self: Stream<Item = Result<ResponsesChunk>> + Sized,
    {
        let mut content = String::new();
        let mut usage = None;
        let mut stream = Box::pin(self);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

            if let Some(text) = chunk.content() {
                content.push_str(text);
            }

            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
        }

        Ok((content, usage))
    }
}

// Implement for all compatible streams
impl<S> ResponseStreamExt for S
where
    S: Stream<Item = Result<ResponsesChunk>>,
{}
```

## Configuration Patterns

### Builder Configuration

Use builder pattern for configuration objects:

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub organization_id: Option<String>,
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub default_model: String,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    pub fn from_env() -> Result<Self> {
        ConfigBuilder::default()
            .api_key_from_env()
            .organization_id_from_env()
            .build()
    }
}

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    api_key: Option<String>,
    organization_id: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    default_model: Option<String>,
}

impl ConfigBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn api_key_from_env(mut self) -> Self {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.api_key = Some(key);
        }
        self
    }

    pub fn organization_id(mut self, id: impl Into<String>) -> Self {
        self.organization_id = Some(id.into());
        self
    }

    pub fn build(self) -> Result<Config> {
        Ok(Config {
            api_key: self.api_key.ok_or_else(|| Error::Config {
                message: "API key is required".to_string(),
            })?,
            organization_id: self.organization_id,
            base_url: self.base_url.unwrap_or_else(||
                "https://api.openai.com/v1".to_string()
            ),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            max_retries: self.max_retries.unwrap_or(3),
            default_model: self.default_model.unwrap_or_else(||
                constants::models::GPT_4_TURBO.to_string()
            ),
        })
    }
}
```

## Extension Trait Patterns

### Response Extension Traits

Add ergonomic methods to response types:

```rust
/// Extension trait for chat completion responses
pub trait ChatCompletionResponseExt {
    /// Get the content from the first choice
    fn content(&self) -> Option<&str>;

    /// Get all choice contents
    fn all_content(&self) -> Vec<&str>;

    /// Check if response was truncated due to length
    fn is_truncated(&self) -> bool;

    /// Get finish reason for first choice
    fn finish_reason(&self) -> Option<&str>;

    /// Extract tool calls if present
    fn tool_calls(&self) -> Vec<&ToolCall>;
}

impl ChatCompletionResponseExt for ChatCompletionResponse {
    fn content(&self) -> Option<&str> {
        self.choices
            .first()?
            .message
            .content
            .as_deref()
    }

    fn all_content(&self) -> Vec<&str> {
        self.choices
            .iter()
            .filter_map(|choice| choice.message.content.as_deref())
            .collect()
    }

    fn is_truncated(&self) -> bool {
        self.choices
            .iter()
            .any(|choice| choice.finish_reason.as_deref() == Some("length"))
    }

    fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()?
            .finish_reason
            .as_deref()
    }

    fn tool_calls(&self) -> Vec<&ToolCall> {
        self.choices
            .iter()
            .filter_map(|choice| choice.message.tool_calls.as_ref())
            .flatten()
            .collect()
    }
}
```

These builder patterns provide:

1. **Consistent fluent interfaces** across all builders
2. **Type safety** through phantom types and validation
3. **Ergonomic helpers** for common use cases
4. **Async-first design** with proper Future handling
5. **Stream processing utilities** for real-time data
6. **Extension traits** to enrich existing types
7. **Configuration builders** for flexible setup

The patterns emphasize developer ergonomics while maintaining type safety and preventing common mistakes through compile-time guarantees.