# Architecture Overview

This document provides an overview of the `openai-ergonomic` crate architecture, design principles, and module organization.

## Design Principles

### 1. Ergonomic API Design
- **Builder Pattern**: Use type-safe builders for complex request construction
- **Sensible Defaults**: Provide reasonable defaults while allowing customization
- **Fluent Interface**: Enable method chaining for natural API usage
- **Type Safety**: Leverage Rust's type system to prevent invalid configurations

### 2. Async-First
- **Tokio Integration**: Built on top of the Tokio async runtime
- **Stream Support**: First-class support for streaming responses
- **Non-blocking**: All network operations are non-blocking
- **Future-ready**: Designed for async/await patterns

### 3. Layered Architecture
```
┌─────────────────────────┐
│   High-level Builders   │  ← Ergonomic API (openai-ergonomic)
├─────────────────────────┤
│   Generated Client      │  ← Low-level API (openai-client-base)
├─────────────────────────┤
│   HTTP Client (reqwest) │  ← Transport layer
└─────────────────────────┘
```

## Module Structure

### Core Modules

#### `client`
The main client module providing the entry point for all API interactions.

```rust
pub struct OpenAIClient {
    // Internal HTTP client and configuration
}

impl OpenAIClient {
    pub fn new() -> ClientBuilder { /* ... */ }
    pub fn chat_completions(&self) -> ChatCompletionsBuilder { /* ... */ }
    pub fn embeddings(&self) -> EmbeddingsBuilder { /* ... */ }
    // ... other API endpoints
}
```

#### `builders`
Type-safe builder patterns for each OpenAI API endpoint.

```
builders/
├── chat/              # Chat completions
│   ├── mod.rs
│   ├── builder.rs     # ChatCompletionsBuilder
│   └── streaming.rs   # Streaming support
├── embeddings/        # Embeddings API
├── images/            # Image generation
├── audio/             # Speech and transcription
├── assistants/        # Assistants API
└── files/             # File operations
```

#### `types`
Shared types, enums, and data structures.

```rust
pub mod models;        // Model identifiers and metadata
pub mod errors;        // Error types and handling
pub mod responses;     // Response types
pub mod common;        // Shared types and utilities
```

#### `config`
Configuration management and client setup.

```rust
pub struct ClientConfig {
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
    // ... other configuration options
}
```

### Builder Pattern Implementation

Each API endpoint follows a consistent builder pattern:

```rust
pub struct ChatCompletionsBuilder<'a> {
    client: &'a OpenAIClient,
    model: Option<String>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    // ... other parameters
}

impl<'a> ChatCompletionsBuilder<'a> {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn message(mut self, role: &str, content: impl Into<String>) -> Self {
        self.messages.push(Message::new(role, content));
        self
    }

    pub async fn send(self) -> Result<ChatCompletion, OpenAIError> {
        // Validation and API call
    }

    pub async fn stream(self) -> Result<impl Stream<Item = Result<ChatCompletionChunk, OpenAIError>>, OpenAIError> {
        // Streaming implementation
    }
}
```

## Key Design Decisions

### 1. Builder Validation
- **Compile-time**: Use type states where possible to catch errors at compile time
- **Runtime**: Validate required parameters before making API calls
- **Clear Errors**: Provide meaningful error messages for invalid configurations

### 2. Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum OpenAIError {
    #[error("API error: {message}")]
    Api { message: String, status: u16 },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },
}
```

### 3. Streaming Support
- **Async Streams**: Use `futures::Stream` for streaming responses
- **Backpressure**: Handle backpressure naturally through the stream interface
- **Error Handling**: Propagate errors through the stream

### 4. Testing Strategy
```
tests/
├── unit/              # Unit tests for individual components
├── integration/       # End-to-end API tests
├── mocks/             # Mock server tests
└── examples/          # Example validation tests
```

## Extension Points

### 1. Custom Models
Support for custom model identifiers and parameters:

```rust
impl ChatCompletionsBuilder<'_> {
    pub fn custom_model(mut self, model: CustomModel) -> Self {
        // Handle custom model configuration
    }
}
```

### 2. Middleware Support
Hook points for request/response middleware:

```rust
pub trait Middleware {
    async fn process_request(&self, request: &mut Request) -> Result<(), Error>;
    async fn process_response(&self, response: &mut Response) -> Result<(), Error>;
}
```

### 3. Custom Serialization
Support for custom serialization formats and transformations.

## Performance Considerations

### 1. Connection Pooling
- Reuse HTTP connections through `reqwest::Client`
- Configure connection pool size based on usage patterns

### 2. Memory Management
- Stream large responses to avoid memory pressure
- Use efficient serialization for large payloads

### 3. Rate Limiting
- Built-in rate limiting and retry logic
- Configurable backoff strategies

## Future Architecture Considerations

### 1. Plugin System
Extensible plugin architecture for custom functionality:
- Custom authentication methods
- Response transformations
- Caching strategies

### 2. Multi-client Support
Support for multiple OpenAI API keys and configurations within a single application.

### 3. Offline Capabilities
Local model integration and offline fallback strategies.

## Dependencies

### Core Dependencies
- `tokio`: Async runtime
- `reqwest`: HTTP client
- `serde`: Serialization framework
- `bon`: Builder pattern macros
- `thiserror`: Error handling

### Optional Dependencies
- `tracing`: Logging and instrumentation
- `wiremock`: Testing utilities
- `futures`: Stream utilities

## Migration Path

For users migrating from other OpenAI Rust clients:

1. **From `async-openai`**: Migration guide for common patterns
2. **From `openai-api-rs`**: Mapping between API surface differences
3. **From raw HTTP clients**: Benefits of type-safe builders

## Documentation Standards

### Code Documentation
- All public APIs must have rustdoc comments
- Include examples in documentation
- Document error conditions and edge cases

### Architecture Documentation
- Keep this document updated with significant changes
- Document design decisions and rationales
- Provide migration guides for breaking changes

This architecture provides a solid foundation for building an ergonomic, type-safe, and performant OpenAI API client while maintaining flexibility for future enhancements.