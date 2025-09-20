# OpenAI Client Base Capabilities Analysis

## Overview
Analysis of `openai-client-base` v0.2.0 to understand capabilities and identify gaps for our ergonomic wrapper.

## Current Capabilities

### 1. Authentication ✅
- **Bearer Token**: Fully supported via `Configuration::bearer_access_token`
- **API Key**: Structure exists (`ApiKey` with prefix/key) but not actively used
- **Basic Auth**: Supported but not relevant for OpenAI
- **OAuth**: Supported but not relevant for OpenAI

**Gap**: No convenient constructors or environment variable loading for API keys.

### 2. Streaming ❌
- **Models**: Stream event types exist (100+ files with stream-related models)
- **Request Support**: `stream: Option<bool>` field in request structs
- **Response Handling**: No SSE/EventSource parsing implementation
- **Return Types**: Standard JSON responses only, no stream iterators

**Major Gap**: No actual streaming implementation despite model support. Requires:
- SSE parser/deserializer
- Stream/async iterator wrappers
- Proper error handling for partial responses

### 3. Error Handling ⚠️
- **Error Type**: Generic `Error<T>` enum with variants for:
  - Network errors (Reqwest)
  - Middleware errors (ReqwestMiddleware)
  - Serialization errors (Serde)
  - IO errors
  - Response errors with status/content
- **API Errors**: Each endpoint has typed error enums but only `UnknownValue` variant
- **Display/Debug**: Basic implementations exist

**Gap**: No structured OpenAI error parsing (error codes, messages, types).

### 4. Feature Flags ❌
- **Current State**: No feature flags defined
- **Conditional Compilation**: Not used

**Gap**: Need feature flags for:
- Optional dependencies (streaming libs, async runtimes)
- API subsets (chat, images, assistants, etc.)
- Experimental features

### 5. Configuration ⚠️
- **Base Configuration**: Simple struct with client, auth, base_path
- **Middleware Support**: Uses `reqwest_middleware`
- **Default Values**: Hardcoded OpenAI endpoint

**Gap**: Missing:
- Builder pattern for configuration
- Retry logic configuration
- Timeout configuration
- Custom headers support
- Organization ID support

### 6. HTTP Client
- **Base**: `reqwest` with `reqwest_middleware`
- **Features**: JSON, multipart, stream support in reqwest
- **Async**: Full tokio async support

### 7. Request Building
- **Builder Pattern**: Uses `bon::Builder` derive macro
- **Serialization**: Serde with special handling for optional/null
- **Multipart**: Helper module for file uploads

## Wrapper Requirements

### High Priority
1. **Streaming Implementation**
   - SSE parser using `eventsource-stream` or similar
   - Async stream wrappers returning `Stream<Item = Result<T>>`
   - Proper connection management and error recovery

2. **Ergonomic Authentication**
   - `from_env()` constructor
   - API key validation
   - Organization ID support
   - Better secret handling

3. **Structured Error Handling**
   - Parse OpenAI error responses
   - Error types matching OpenAI's error taxonomy
   - Retry-able vs non-retry-able errors
   - Rate limit handling

### Medium Priority
4. **Configuration Builder**
   - Fluent API for configuration
   - Sensible defaults
   - Environment variable support
   - Per-request overrides

5. **Response Helpers**
   - Content extraction helpers
   - Usage/cost tracking
   - Response metadata access

6. **Feature Flags**
   - Modular API surface
   - Optional dependencies
   - Minimal core vs full features

### Low Priority
7. **Observability**
   - Request/response logging hooks
   - Metrics collection points
   - Tracing integration

8. **Testing Support**
   - Mock client/server
   - Request recording/replay
   - Deterministic testing helpers

## Implementation Strategy

1. **Phase 1**: Core wrapper with auth and config builders
2. **Phase 2**: Streaming support (highest value feature)
3. **Phase 3**: Error handling improvements
4. **Phase 4**: Helper methods and ergonomic APIs
5. **Phase 5**: Testing and observability

## Dependencies to Add
- `tokio-stream` or `futures` - for stream traits
- `eventsource-stream` or `async-sse` - for SSE parsing
- `thiserror` or `anyhow` - for better error handling
- `tracing` - for observability
- `dotenv` (optional) - for env var loading