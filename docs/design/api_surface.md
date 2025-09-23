# API Surface Design

> Comprehensive specification for the openai-ergonomic crate's API design, module organization, and builder patterns.

## Table of Contents

- [Module Map](#module-map)
- [Builder API Specifications](#builder-api-specifications)
- [Naming Conventions](#naming-conventions)
- [Error Handling Design](#error-handling-design)
- [Streaming Helper Specifications](#streaming-helper-specifications)
- [Feature Flag Architecture](#feature-flag-architecture)
- [Type Safety Patterns](#type-safety-patterns)

## Module Map

The crate is organized around OpenAI endpoint groups with corresponding builder and response modules:

### Core API Modules

```
src/
├── lib.rs                    # Public API re-exports and crate documentation
├── client.rs                 # Main Client struct with endpoint methods
├── config.rs                 # Configuration management and builder
├── errors.rs                 # Error types and handling
├── builders/                 # Builder patterns for requests
│   ├── mod.rs               # Builder trait and common patterns
│   ├── responses.rs         # Modern Responses API builders (primary)
│   ├── chat.rs              # Legacy Chat API builders (compatibility)
│   ├── assistants.rs        # Assistants API builders
│   ├── audio.rs             # Audio (TTS/STT) builders
│   ├── images.rs            # Image generation/editing builders
│   ├── embeddings.rs        # Embeddings builders
│   ├── files.rs             # File management builders
│   ├── fine_tuning.rs       # Fine-tuning builders
│   ├── batch.rs             # Batch processing builders
│   ├── vector_stores.rs     # Vector stores builders
│   ├── moderations.rs       # Content moderation builders
│   ├── threads.rs           # Assistant threads builders
│   └── uploads.rs           # File upload helpers
├── responses/               # Response wrappers with ergonomic helpers
│   ├── mod.rs               # Response trait and common patterns
│   ├── responses.rs         # Responses API response wrappers
│   ├── chat.rs              # Chat completion response wrappers
│   ├── assistants.rs        # Assistant response wrappers
│   ├── audio.rs             # Audio response wrappers
│   ├── images.rs            # Image response wrappers
│   ├── embeddings.rs        # Embedding response wrappers
│   ├── files.rs             # File response wrappers
│   ├── fine_tuning.rs       # Fine-tuning response wrappers
│   ├── batch.rs             # Batch response wrappers
│   ├── vector_stores.rs     # Vector stores response wrappers
│   ├── moderations.rs       # Moderation response wrappers
│   ├── threads.rs           # Thread response wrappers
│   └── uploads.rs           # Upload response wrappers
├── streaming/               # Streaming support
│   ├── mod.rs               # Streaming traits and utilities
│   ├── responses.rs         # Responses streaming
│   ├── chat.rs              # Chat streaming (legacy)
│   └── assistants.rs        # Assistant streaming
├── constants.rs             # Model names, common values
├── types.rs                 # Type aliases for verbose names
└── test_utils.rs            # Testing utilities (feature-gated)
```

### Module Alignment with OpenAI Endpoints

| OpenAI Endpoint | Builder Module | Response Module | Primary Use Case |
|-----------------|----------------|-----------------|------------------|
| `/chat/completions` | `builders::responses` | `responses::responses` | Modern API (preferred) |
| `/chat/completions` | `builders::chat` | `responses::chat` | Legacy compatibility |
| `/assistants` | `builders::assistants` | `responses::assistants` | Assistant creation/management |
| `/threads` | `builders::threads` | `responses::threads` | Conversation management |
| `/audio/speech` | `builders::audio` | `responses::audio` | Text-to-speech |
| `/audio/transcriptions` | `builders::audio` | `responses::audio` | Speech-to-text |
| `/images/generations` | `builders::images` | `responses::images` | Image generation |
| `/embeddings` | `builders::embeddings` | `responses::embeddings` | Vector embeddings |
| `/files` | `builders::files` | `responses::files` | File management |
| `/fine_tuning/jobs` | `builders::fine_tuning` | `responses::fine_tuning` | Model fine-tuning |
| `/batches` | `builders::batch` | `responses::batch` | Batch processing |
| `/vector_stores` | `builders::vector_stores` | `responses::vector_stores` | Vector search |
| `/moderations` | `builders::moderations` | `responses::moderations` | Content filtering |
| `/uploads` | `builders::uploads` | `responses::uploads` | Large file uploads |

## Builder API Specifications

### When to Use `bon::builder` vs Convenience Helpers

The crate follows a tiered approach to API design:

#### Tier 1: Convenience Helpers (Preferred)
For common use cases, provide simple function-based APIs:

```rust
// Simple chat with string input/output
pub async fn chat_simple(prompt: &str) -> Result<String>

// Streaming chat with ergonomic types
pub async fn chat_stream(prompt: &str) -> Result<ChatStream>

// Common responses patterns
pub async fn responses_simple(prompt: &str) -> Result<String>
pub async fn responses_system_user(system: &str, user: &str) -> Result<String>
```

#### Tier 2: Builder APIs (Flexible)
For complex scenarios requiring configuration:

```rust
// Builder pattern with compile-time validation
pub fn chat() -> ChatCompletionBuilder
pub fn responses() -> ResponsesBuilder
pub fn assistant() -> AssistantBuilder

// Usage:
client.responses()
    .system("You are a helpful assistant")
    .user("What is Rust?")
    .model("gpt-4")
    .temperature(0.7)
    .send()
    .await?;
```

#### Tier 3: Raw `bon::builder` (Advanced)
Direct access to generated OpenAPI builders for maximum flexibility:

```rust
// Direct bon::builder access
pub fn create_chat_completion() -> CreateChatCompletionRequestBuilder

// Usage for advanced scenarios:
client.create_chat_completion()
    .messages(messages)
    .model("gpt-4")
    .temperature(Some(0.7))
    .max_tokens(Some(1000))
    .send()
    .await?;
```

### Builder Pattern Specifications

#### Builder Trait
All builders implement a common `Builder` trait:

```rust
pub trait Builder<T> {
    type Output;

    /// Execute the request and return the response
    async fn send(self) -> Result<Self::Output>;

    /// Execute the request and return a stream (if supported)
    async fn stream(self) -> Result<impl Stream<Item = Result<T>>>;
}

pub trait Sendable {
    type Response;
    async fn send(self) -> Result<Self::Response>;
}
```

#### Builder Naming Conventions
- Builder structs end with `Builder`: `ResponsesBuilder`, `ChatCompletionBuilder`
- Constructor functions match the endpoint: `responses()`, `chat()`, `assistant()`
- Methods use descriptive names without prefixes: `.user()`, `.system()`, `.model()`
- Boolean flags use `with_` prefix: `.with_streaming()`, `.with_tools()`

#### Builder Method Patterns

```rust
impl ResponsesBuilder {
    // Message methods
    pub fn system(mut self, content: impl Into<String>) -> Self
    pub fn user(mut self, content: impl Into<String>) -> Self
    pub fn assistant(mut self, content: impl Into<String>) -> Self

    // Configuration methods
    pub fn model(mut self, model: impl Into<String>) -> Self
    pub fn temperature(mut self, temp: f64) -> Self
    pub fn max_tokens(mut self, tokens: u32) -> Self

    // Tool methods
    pub fn tool(mut self, tool: Tool) -> Self
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self

    // Execution methods
    pub async fn send(self) -> Result<ResponsesResponse>
    pub async fn stream(self) -> Result<ResponsesStream>
}
```

## Naming Conventions

### Constants and Type Aliases

#### Model Constants
Organized by capability and release date:

```rust
pub mod constants {
    pub mod models {
        // Language models (latest first)
        pub const GPT_4_TURBO: &str = "gpt-4-turbo";
        pub const GPT_4: &str = "gpt-4";
        pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";

        // Embedding models
        pub const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
        pub const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";

        // Audio models
        pub const TTS_1: &str = "tts-1";
        pub const TTS_1_HD: &str = "tts-1-hd";
        pub const WHISPER_1: &str = "whisper-1";

        // Image models
        pub const DALL_E_3: &str = "dall-e-3";
        pub const DALL_E_2: &str = "dall-e-2";
    }

    pub mod audio {
        pub const VOICES: &[&str] = &["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
        pub const FORMATS: &[&str] = &["mp3", "opus", "aac", "flac"];
    }

    pub mod images {
        pub const SIZES_DALL_E_2: &[&str] = &["256x256", "512x512", "1024x1024"];
        pub const SIZES_DALL_E_3: &[&str] = &["1024x1024", "1792x1024", "1024x1792"];
    }
}
```

#### Type Aliases
Simplify verbose generated type names:

```rust
pub mod types {
    use openai_client_base::models::*;

    // Request type aliases
    pub type ChatRequest = CreateChatCompletionRequest;
    pub type ResponsesRequest = CreateResponsesRequest;
    pub type AssistantRequest = CreateAssistantRequest;

    // Response type aliases
    pub type ChatResponse = CreateChatCompletionResponse;
    pub type ResponsesResponse = CreateResponsesResponse;
    pub type AssistantResponse = Assistant;

    // Message types
    pub type ChatMessage = ChatCompletionRequestMessage;
    pub type ResponsesMessage = ResponsesRequestMessage;

    // Tool types
    pub type FunctionTool = ChatCompletionTool;
    pub type ToolCall = ChatCompletionMessageToolCall;
}
```

### Function Naming Conventions

#### Builder Constructor Functions
- Use endpoint name: `responses()`, `chat()`, `assistant()`
- No `create_` or `build_` prefixes
- Return builder instance ready for configuration

#### Helper Function Naming
- Action + object: `tool_function()`, `tool_web_search()`
- Context when needed: `system_user()`, `image_base64_part()`
- Streaming variants: `chat_stream()`, `responses_stream()`

#### Response Method Naming
- Use property names directly: `.content()`, `.usage()`, `.model()`
- Boolean queries with `is_` or `has_`: `.is_finished()`, `.has_tools()`
- Collections use plural: `.choices()`, `.messages()`, `.tools()`

## Error Handling Design

### Error Type Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// API errors from OpenAI
    #[error("OpenAI API error: {0}")]
    Api(#[from] openai_client_base::Error),

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<std::time::Duration>,
    },

    /// Authentication errors
    #[error("Authentication failed: {message}")]
    Auth { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Validation errors for request parameters
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    /// Network/transport errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON parsing errors
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Streaming errors
    #[error("Streaming error: {message}")]
    Stream { message: String },

    /// Timeout errors
    #[error("Request timeout after {duration:?}")]
    Timeout { duration: std::time::Duration },
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Error Context Patterns

#### Builder Validation
Validate parameters at build time when possible:

```rust
impl ResponsesBuilder {
    pub fn temperature(mut self, temp: f64) -> Result<Self> {
        if temp < 0.0 || temp > 2.0 {
            return Err(Error::Validation {
                field: "temperature".to_string(),
                message: "must be between 0.0 and 2.0".to_string(),
            });
        }
        self.temperature = Some(temp);
        Ok(self)
    }
}
```

#### Response Error Handling
Provide context for API errors:

```rust
impl From<openai_client_base::Error> for Error {
    fn from(err: openai_client_base::Error) -> Self {
        match err {
            openai_client_base::Error::Http { status: 429, .. } => {
                Error::RateLimit {
                    message: "API rate limit exceeded".to_string(),
                    retry_after: None, // Parse from headers if available
                }
            }
            openai_client_base::Error::Http { status: 401, .. } => {
                Error::Auth {
                    message: "Invalid API key or authentication failed".to_string(),
                }
            }
            _ => Error::Api(err),
        }
    }
}
```

## Streaming Helper Specifications

### Streaming Traits

```rust
pub trait StreamExt {
    type Item;

    /// Collect all chunks into a single response
    async fn collect_content(self) -> Result<String>;

    /// Collect with usage information
    async fn collect_with_usage(self) -> Result<(String, Usage)>;

    /// Apply function to each chunk
    fn map_chunks<F, T>(self, f: F) -> impl Stream<Item = Result<T>>
    where
        F: Fn(Self::Item) -> T;
}

pub trait ResponseStream: Stream<Item = Result<Self::Chunk>> {
    type Chunk;
    type FinalResponse;

    /// Get the accumulated response so far
    fn current_response(&self) -> &Self::FinalResponse;

    /// Check if the stream is complete
    fn is_finished(&self) -> bool;
}
```

### Streaming Response Types

```rust
/// Streaming chat completion chunk
#[derive(Debug, Clone)]
pub struct ChatChunk {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
    pub model: String,
}

impl ChatChunk {
    /// Get the content from the first choice
    pub fn content(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }

    /// Get the finish reason if present
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices.first()?.finish_reason.as_deref()
    }
}

/// Streaming responses chunk
pub type ResponsesChunk = ChatChunk; // Same structure

/// Assistant run stream chunk
#[derive(Debug, Clone)]
pub struct RunChunk {
    pub event: String,
    pub data: serde_json::Value,
}
```

### Streaming Helper Functions

```rust
/// Create a streaming responses request
pub async fn responses_stream(
    client: &Client,
    prompt: impl Into<String>,
) -> Result<ResponsesStream> {
    client.responses()
        .user(prompt)
        .with_streaming()
        .stream()
        .await
}

/// Stream with callback for each chunk
pub async fn responses_stream_with_callback<F>(
    client: &Client,
    prompt: impl Into<String>,
    mut callback: F,
) -> Result<String>
where
    F: FnMut(&ResponsesChunk) -> Result<()>,
{
    let mut stream = responses_stream(client, prompt).await?;
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        callback(&chunk)?;

        if let Some(text) = chunk.content() {
            content.push_str(text);
        }
    }

    Ok(content)
}
```

## Feature Flag Architecture

### Feature Flags

```toml
[features]
default = ["native-tls"]

# TLS backend selection (mutually exclusive)
native-tls = ["openai-client-base/native-tls"]
rustls = ["openai-client-base/rustls"]

# Optional functionality
stream = ["futures", "tokio-stream"]
test-utils = ["mockito", "tokio-test"]
observability = ["tracing", "opentelemetry"]

# Experimental features
realtime = ["tokio-tungstenite", "serde_json"]
beta-endpoints = []

# Platform-specific features
wasm = ["openai-client-base/wasm"]
```

### Feature-Gated Code Patterns

```rust
// Streaming support
#[cfg(feature = "stream")]
pub mod streaming;

#[cfg(feature = "stream")]
impl ResponsesBuilder {
    pub async fn stream(self) -> Result<ResponsesStream> {
        // Streaming implementation
    }
}

// Test utilities
#[cfg(feature = "test-utils")]
pub mod test_utils;

// Observability
#[cfg(feature = "observability")]
impl Client {
    pub fn with_tracing(mut self) -> Self {
        // Add tracing middleware
        self
    }
}

// Experimental endpoints
#[cfg(feature = "beta-endpoints")]
pub mod beta {
    // Beta API implementations
}
```

## Type Safety Patterns

### Builder State Types
Use phantom types to enforce builder state:

```rust
pub struct ResponsesBuilder<State = Buildable> {
    inner: CreateResponsesRequest,
    _state: PhantomData<State>,
}

pub struct Buildable;
pub struct Executable;

impl ResponsesBuilder<Buildable> {
    pub fn user(mut self, content: impl Into<String>) -> ResponsesBuilder<Executable> {
        // Add user message, transition to executable state
        ResponsesBuilder {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl ResponsesBuilder<Executable> {
    pub async fn send(self) -> Result<ResponsesResponse> {
        // Only executable builders can be sent
    }
}
```

### Compile-Time Validation
Use const generics and type-level programming where beneficial:

```rust
/// Temperature value validated at compile time
#[derive(Debug, Clone, Copy)]
pub struct Temperature<const VALID: bool = true>(f64);

impl Temperature<true> {
    pub const fn new(value: f64) -> Option<Self> {
        if value >= 0.0 && value <= 2.0 {
            Some(Self(value))
        } else {
            None
        }
    }
}

// Usage:
const TEMP: Temperature = Temperature::new(0.7).unwrap();
builder.temperature(TEMP);
```

### Response Type Safety
Ensure response methods match the actual API response:

```rust
pub trait ResponseExt {
    fn usage(&self) -> Option<&Usage>;
    fn model(&self) -> &str;
    fn created(&self) -> u64;
}

impl ResponseExt for ResponsesResponse {
    fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn created(&self) -> u64 {
        self.created
    }
}
```

This API surface design provides:

1. **Clear module organization** aligned with OpenAI endpoints
2. **Tiered API design** from simple helpers to full builders
3. **Consistent naming conventions** across the crate
4. **Robust error handling** with rich context
5. **First-class streaming support** with ergonomic helpers
6. **Flexible feature flags** for optional functionality
7. **Type safety patterns** to prevent runtime errors

The design balances ergonomics for common use cases with flexibility for advanced scenarios, following Rust idioms and providing a pleasant developer experience.