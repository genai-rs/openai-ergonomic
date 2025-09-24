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
│   ├── responses.rs         # Modern Responses API builders (✅ IMPLEMENTED)
│   ├── chat.rs              # Legacy Chat API builders (✅ IMPLEMENTED)
│   ├── assistants.rs        # Assistants API builders (✅ IMPLEMENTED)
│   ├── audio.rs             # Audio (TTS/STT) builders (🚧 PLACEHOLDER)
│   ├── images.rs            # Image generation/editing builders (🚧 PLACEHOLDER)
│   ├── embeddings.rs        # Embeddings builders (🚧 PLACEHOLDER)
│   ├── files.rs             # File management builders (✅ IMPLEMENTED)
│   ├── fine_tuning.rs       # Fine-tuning builders (✅ IMPLEMENTED)
│   ├── batch.rs             # Batch processing builders (✅ IMPLEMENTED)
│   ├── vector_stores.rs     # Vector stores builders (✅ IMPLEMENTED)
│   ├── moderations.rs       # Content moderation builders (✅ IMPLEMENTED)
│   ├── threads.rs           # Assistant threads builders (🚧 PLACEHOLDER)
│   └── uploads.rs           # File upload helpers (🚧 PLACEHOLDER)
├── responses/               # Response wrappers with ergonomic helpers
│   ├── mod.rs               # Response trait and common patterns (✅ IMPLEMENTED)
│   ├── responses.rs         # Responses API response wrappers (🚧 PLACEHOLDER)
│   ├── chat.rs              # Chat completion response wrappers (✅ IMPLEMENTED)
│   ├── assistants.rs        # Assistant response wrappers (🚧 PLACEHOLDER)
│   ├── audio.rs             # Audio response wrappers (🚧 PLACEHOLDER)
│   ├── images.rs            # Image response wrappers (🚧 PLACEHOLDER)
│   ├── embeddings.rs        # Embedding response wrappers (🚧 PLACEHOLDER)
│   ├── files.rs             # File response wrappers (🚧 PLACEHOLDER)
│   ├── fine_tuning.rs       # Fine-tuning response wrappers (🚧 PLACEHOLDER)
│   ├── batch.rs             # Batch response wrappers (🚧 PLACEHOLDER)
│   ├── vector_stores.rs     # Vector stores response wrappers (🚧 PLACEHOLDER)
│   ├── moderations.rs       # Moderation response wrappers (🚧 PLACEHOLDER)
│   ├── threads.rs           # Thread response wrappers (🚧 PLACEHOLDER)
│   └── uploads.rs           # Upload response wrappers (🚧 PLACEHOLDER)
├── streaming/               # Streaming support (🚧 NOT YET IMPLEMENTED)
│   ├── mod.rs               # Streaming traits and utilities
│   ├── responses.rs         # Responses streaming
│   ├── chat.rs              # Chat streaming (legacy)
│   └── assistants.rs        # Assistant streaming
├── constants.rs             # Model names, common values (🚧 NOT YET IMPLEMENTED)
├── types.rs                 # Type aliases for verbose names (🚧 NOT YET IMPLEMENTED)
└── test_utils.rs            # Testing utilities (feature-gated) (✅ IMPLEMENTED)
```

### Module Alignment with OpenAI Endpoints

| OpenAI Endpoint | Builder Module | Response Module | Implementation Status | Primary Use Case |
|-----------------|----------------|-----------------|---------------------|------------------|
| `/chat/completions` | `builders::responses` | `responses::responses` | ✅ Builder / 🚧 Response | Modern API (preferred) |
| `/chat/completions` | `builders::chat` | `responses::chat` | ✅ Implemented | Legacy compatibility |
| `/assistants` | `builders::assistants` | `responses::assistants` | ✅ Builder / 🚧 Response | Assistant creation/management |
| `/threads` | `builders::threads` | `responses::threads` | 🚧 Placeholder | Conversation management |
| `/audio/speech` | `builders::audio` | `responses::audio` | 🚧 Placeholder | Text-to-speech |
| `/audio/transcriptions` | `builders::audio` | `responses::audio` | 🚧 Placeholder | Speech-to-text |
| `/images/generations` | `builders::images` | `responses::images` | 🚧 Placeholder | Image generation |
| `/embeddings` | `builders::embeddings` | `responses::embeddings` | 🚧 Placeholder | Vector embeddings |
| `/files` | `builders::files` | `responses::files` | ✅ Builder / 🚧 Response | File management |
| `/fine_tuning/jobs` | `builders::fine_tuning` | `responses::fine_tuning` | ✅ Builder / 🚧 Response | Model fine-tuning |
| `/batches` | `builders::batch` | `responses::batch` | ✅ Builder / 🚧 Response | Batch processing |
| `/vector_stores` | `builders::vector_stores` | `responses::vector_stores` | ✅ Builder / 🚧 Response | Vector search |
| `/moderations` | `builders::moderations` | `responses::moderations` | ✅ Builder / 🚧 Response | Content filtering |
| `/uploads` | `builders::uploads` | `responses::uploads` | 🚧 Placeholder | Large file uploads |

**Legend:**
- ✅ **Implemented**: Full functional implementation
- 🚧 **Placeholder**: Module exists but contains only TODO comments
- ✅ Builder / 🚧 Response: Builder is implemented, response wrapper is placeholder

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
All builders implement a common `Builder` trait for consistency:

```rust
/// Common trait for all builders to provide consistent APIs.
pub trait Builder<T> {
    /// Build the final request type.
    fn build(self) -> crate::Result<T>;
}

/// Helper trait for builders that can be sent to the OpenAI API.
pub trait Sendable<R> {
    /// Send the request to the OpenAI API and return the response.
    async fn send(self) -> crate::Result<R>;
}
```

**Note**: Streaming support will be added in future versions with additional traits.

#### Builder Naming Conventions
- Builder structs end with `Builder`: `ResponsesBuilder`, `ChatCompletionBuilder`
- Constructor functions match the endpoint: `responses()`, `chat()`, `assistant()`
- Methods use descriptive names without prefixes: `.user()`, `.system()`, `.model()`
- Boolean flags use `with_` prefix: `.with_streaming()`, `.with_tools()`

#### Actual Builder Method Patterns

Based on the current implementation, here are the actual method patterns:

```rust
impl ResponsesBuilder {
    /// Create a new responses builder with the specified model
    pub fn new(model: impl Into<String>) -> Self

    // Message methods
    pub fn system(mut self, content: impl Into<String>) -> Self
    pub fn user(mut self, content: impl Into<String>) -> Self
    pub fn assistant(mut self, content: impl Into<String>) -> Self

    // Configuration methods
    pub fn temperature(mut self, temperature: f64) -> Self
    pub fn max_tokens(mut self, max_tokens: i32) -> Self
    pub fn max_completion_tokens(mut self, max_completion_tokens: i32) -> Self
    pub fn stream(mut self, stream: bool) -> Self

    // Tool methods
    pub fn tools(mut self, tools: Vec<ChatCompletionTool>) -> Self
    pub fn tool(mut self, tool: ChatCompletionTool) -> Self
    pub fn tool_choice(mut self, tool_choice: ChatCompletionToolChoiceOption) -> Self

    // Response format methods
    pub fn json_mode(mut self) -> Self
    pub fn json_schema(mut self, name: impl Into<String>, schema: Value) -> Self

    // Advanced configuration
    pub fn n(mut self, n: i32) -> Self
    pub fn stop(mut self, stop: Vec<String>) -> Self
    pub fn presence_penalty(mut self, presence_penalty: f64) -> Self
    pub fn frequency_penalty(mut self, frequency_penalty: f64) -> Self
    pub fn top_p(mut self, top_p: f64) -> Self
    pub fn user_id(mut self, user: impl Into<String>) -> Self
    pub fn seed(mut self, seed: i32) -> Self
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self // For o3 models
}

// Builder trait implementation
impl Builder<CreateChatCompletionRequest> for ResponsesBuilder {
    fn build(self) -> crate::Result<CreateChatCompletionRequest>
}
```

## Naming Conventions

### Constants and Type Aliases (Planned)

> **Status**: Not yet implemented. This section describes the planned design.

Constants and type aliases will be added as needed to improve ergonomics:

#### Model Constants (Future)
When implemented, will be organized by capability:

```rust
// Future implementation
pub mod constants {
    pub mod models {
        // Language models (latest first)
        pub const GPT_4_TURBO: &str = "gpt-4-turbo";
        pub const GPT_4: &str = "gpt-4";
        pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";
        pub const O1: &str = "o1";
        pub const O1_MINI: &str = "o1-mini";
        pub const O3_MINI: &str = "o3-mini";

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
}
```

#### Type Aliases (Future)
Currently, the crate uses full type names from `openai_client_base`. Type aliases will be added based on user feedback:

```rust
// Future implementation
pub mod types {
    use openai_client_base::models::*;

    // Request type aliases
    pub type ChatRequest = CreateChatCompletionRequest;
    pub type AssistantRequest = CreateAssistantRequest;

    // Response type aliases
    pub type ChatResponse = CreateChatCompletionResponse;
    pub type AssistantResponse = Assistant;

    // Message types
    pub type ChatMessage = ChatCompletionRequestMessage;
    pub type ToolCall = ChatCompletionMessageToolCall;
}
```

### Function Naming Conventions

#### Builder Constructor Functions
- Use endpoint name: `responses()`, `chat()`, `assistant()`
- No `create_` or `build_` prefixes
- Return builder instance ready for configuration

#### Current Helper Functions

The crate provides these helper functions:

```rust
// Simple builder creation helpers
pub fn responses_simple(model: impl Into<String>, content: impl Into<String>) -> ResponsesBuilder
pub fn responses_system_user(
    model: impl Into<String>,
    system: impl Into<String>,
    user: impl Into<String>,
) -> ResponsesBuilder

// Tool creation helpers
pub fn responses_tool_function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: Value,
) -> ChatCompletionTool

pub fn responses_tool_web_search() -> ChatCompletionTool
```

#### Helper Function Naming Conventions
- **Action + object**: `responses_tool_function()`, `responses_tool_web_search()`
- **Context when needed**: `responses_system_user()`, `responses_simple()`
- **Streaming variants**: Planned for future implementation

#### Current Response Wrapper Patterns

The crate currently implements response wrappers with extension traits:

#### Chat Completion Response Extensions

```rust
/// Extension trait for chat completion responses
pub trait ChatCompletionResponseExt {
    /// Get the content of the first choice, if available
    fn content(&self) -> Option<&str>;

    /// Get the tool calls from the first choice, if available
    fn tool_calls(&self) -> Vec<&ChatCompletionMessageToolCallsInner>;

    /// Check if the response has tool calls
    fn has_tool_calls(&self) -> bool;

    /// Get the first choice from the response
    fn first_choice(&self) -> Option<&CreateChatCompletionResponseChoicesInner>;

    /// Get the message from the first choice
    fn first_message(&self) -> Option<&ChatCompletionResponseMessage>;

    /// Check if the response was refused
    fn is_refusal(&self) -> bool;

    /// Get the refusal message if the response was refused
    fn refusal(&self) -> Option<&str>;

    /// Get the finish reason for the first choice
    fn finish_reason(&self) -> Option<String>;
}

// Implemented for CreateChatCompletionResponse
impl ChatCompletionResponseExt for CreateChatCompletionResponse { /* ... */ }
```

#### Response Method Naming Conventions
- **Property access**: `.content()`, `.usage()`, `.model()` (direct property names)
- **Boolean queries**: `.is_refusal()`, `.has_tool_calls()` (use `is_` or `has_` prefixes)
- **Collections**: `.tool_calls()`, `.choices()` (plural forms)
- **First item access**: `.first_choice()`, `.first_message()` (explicit `first_` prefix)
- **Optional access**: Most methods return `Option<T>` for safe access

## Error Handling Design

### Error Type Hierarchy

The crate uses a comprehensive error handling system built on `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid request parameters or configuration.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Authentication errors.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limiting errors.
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// HTTP client errors.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// OpenAI API errors with status code and message.
    #[error("OpenAI API error (status {status}): {message}")]
    Api {
        /// HTTP status code returned by the API
        status: u16,
        /// Error message from the API
        message: String,
        /// Type of error (if provided by API)
        error_type: Option<String>,
        /// Error code (if provided by API)
        error_code: Option<String>,
    },

    /// Streaming connection errors.
    #[error("Stream connection error: {message}")]
    StreamConnection {
        /// Error message describing the connection issue
        message: String,
    },

    /// Streaming data parsing errors.
    #[error("Stream parsing error: {message}, chunk: {chunk}")]
    StreamParsing {
        /// Error message describing the parsing issue
        message: String,
        /// The problematic chunk data
        chunk: String,
    },

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Input/Output errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Error Context Patterns

#### Builder Validation
Validation occurs in the `build()` method to provide early error detection:

```rust
impl ResponsesBuilder {
    fn build(self) -> crate::Result<CreateChatCompletionRequest> {
        // Validate model
        if self.model.trim().is_empty() {
            return Err(Error::InvalidRequest("Model cannot be empty".to_string()));
        }

        // Validate messages
        if self.messages.is_empty() {
            return Err(Error::InvalidRequest(
                "At least one message is required".to_string(),
            ));
        }

        // Validate temperature range
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(Error::InvalidRequest(format!(
                    "temperature must be between 0.0 and 2.0, got {temp}"
                )));
            }
        }

        // Build the request...
        Ok(request)
    }
}
```

#### Error Context Patterns
The error types provide rich context for different failure scenarios:

```rust
// Authentication failures
Error::Authentication("Invalid API key".to_string())

// Rate limiting with actionable info
Error::RateLimit("Rate limit exceeded, try again later".to_string())

// API errors with structured data
Error::Api {
    status: 400,
    message: "Invalid request body".to_string(),
    error_type: Some("invalid_request_error".to_string()),
    error_code: Some("missing_parameter".to_string()),
}
```

## Streaming Support (Future Implementation)

> **Status**: Not yet implemented. This section describes the planned design based on example patterns.

Streaming support will be added in future versions with the following design:

### Planned Streaming Traits

```rust
// Future implementation
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
```

### Current Streaming Examples

The crate includes comprehensive streaming examples that demonstrate the intended patterns:
- `examples/responses_streaming.rs` - Responses API streaming
- `examples/chat_comprehensive.rs` - Chat streaming patterns
- `examples/tool_calling.rs` - Streaming with function calls

### Builder Streaming Integration

Builders currently support streaming configuration:

```rust
// Current implementation
let builder = client.responses()
    .user("Tell me about Rust")
    .stream(true); // Enable streaming flag

// Future: Direct streaming method
// let stream = builder.stream().await?;
```

## Feature Flag Architecture

### Current Feature Flags

Based on the current `Cargo.toml`, the crate uses a minimal but effective feature flag system:

```toml
[features]
default = ["rustls-tls"]

# TLS backend selection (mutually exclusive)
rustls-tls = ["reqwest/rustls-tls"]
native-tls = ["reqwest/native-tls-vendored"]

# Optional functionality
test-utils = ["mockito"]
```

### Future Feature Flags (Planned)

```toml
# Streaming support (when implemented)
stream = ["futures", "tokio-stream"]

# Observability integration
observability = ["tracing", "opentelemetry"]

# Experimental features
beta-endpoints = []
realtime = ["tokio-tungstenite"]

# Platform-specific features
wasm = ["reqwest/wasm"]
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

### Response Type Safety Patterns

The response wrappers prioritize type safety and null-safe access:

```rust
// Safe content access with None handling
fn content(&self) -> Option<&str> {
    self.choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
}

// Safe tool call iteration
fn tool_calls(&self) -> Vec<&ChatCompletionMessageToolCallsInner> {
    self.choices
        .first()
        .and_then(|choice| choice.message.tool_calls.as_ref())
        .map(|calls| calls.iter().collect())
        .unwrap_or_default()
}

// Boolean convenience methods
fn has_tool_calls(&self) -> bool {
    !self.tool_calls().is_empty()
}

fn is_refusal(&self) -> bool {
    self.first_message()
        .and_then(|msg| msg.refusal.as_deref())
        .map(|refusal| !refusal.is_empty())
        .unwrap_or(false)
}
```

**Design Principles:**
- **Null safety**: All response methods handle missing fields gracefully
- **Convenience**: Boolean methods for common checks (`has_tool_calls()`, `is_refusal()`)
- **First-choice focus**: Most methods operate on the first choice for simplicity
- **Chain-safe**: Methods use Option chaining to avoid panics

## Implementation Status & Roadmap

### Current Implementation Status (v0.1.0)

#### ✅ Fully Implemented
- **Core Infrastructure**: Error handling, configuration, client wrapper
- **Builder Patterns**: ResponsesBuilder, ChatCompletionBuilder, AssistantBuilder
- **Advanced Builders**: BatchBuilder, FilesBuilder, FineTuningBuilder, ModerationsBuilder, VectorStoresBuilder
- **Response Extensions**: ChatCompletionResponseExt with ergonomic helper methods
- **Testing Infrastructure**: Comprehensive test utilities and patterns
- **Documentation**: Examples covering all major use cases

#### 🚧 Partial Implementation
- **Response Wrappers**: Only chat responses implemented, others are placeholders
- **Helper Functions**: Basic tool and builder creation helpers exist
- **Feature Flags**: Basic TLS selection, test-utils available

#### 🕳️ Planned for Future Versions
- **Streaming Support**: Dedicated streaming module and traits
- **Constants Module**: Model names and common values
- **Type Aliases**: Simplified names for verbose generated types
- **Missing Builders**: Audio, Images, Embeddings, Threads, Uploads
- **Response Wrappers**: All endpoint response wrappers
- **Advanced Features**: Observability, WebAssembly support

### Design Achievements

This API surface design provides:

1. **Clear module organization** aligned with OpenAI endpoints
2. **Tiered API design** from simple helpers to full builders
3. **Consistent naming conventions** across the crate
4. **Robust error handling** with rich context and validation
5. **Comprehensive examples** demonstrating real-world usage patterns
6. **Type safety** with compile-time validation where possible
7. **Extensible architecture** for future OpenAI API additions

### Development Priorities

Based on usage patterns from examples and user feedback:

1. **Phase 1 (v0.2.0)**: Complete response wrapper implementations
2. **Phase 2 (v0.3.0)**: Add streaming support with dedicated traits and helpers
3. **Phase 3 (v0.4.0)**: Implement missing builders (Audio, Images, Embeddings)
4. **Phase 4 (v0.5.0)**: Add constants, type aliases, and advanced features

The current implementation balances **immediate usability** with **future extensibility**, providing a solid foundation for ergonomic OpenAI API usage in Rust while maintaining compatibility with the underlying `openai-client-base` generated client.