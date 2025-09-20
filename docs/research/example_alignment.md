# OpenAI Example Alignment Mapping

> Comprehensive analysis of openai-experiment examples and recommendations for the ergonomic crate

## Executive Summary

Analyzed 49 examples from `openai-experiment/examples/*` to determine which should be ported, merged, or dropped for the ergonomic crate. The analysis reveals comprehensive API coverage with opportunities for consolidation and improved ergonomics.

## Current Coverage Analysis

### 1. Responses API (5 examples)
- **responses**: Basic usage with Responses builder
- **responses-basic**: Simplified version
- **responses-function-call**: Function calling with Responses
- **responses-stream**: Streaming responses
- **responses-web-search**: Web search integration

**Decision**: MERGE into 2 comprehensive examples
- `responses_comprehensive.rs` - Combines basic, function calling, and web search
- `responses_streaming.rs` - Dedicated streaming example

### 2. Chat API (3 examples)
- **chat**: Basic chat completions
- **chat-store**: Chat with message history
- **chat-stream**: Streaming chat

**Decision**: MERGE into 1 comprehensive example
- `chat_comprehensive.rs` - Shows basic, history management, and streaming

### 3. Assistants API (4 examples)
- **assistants**: Basic assistant usage
- **assistants-code-interpreter**: Code interpreter tool
- **assistants-file-search**: File search capabilities
- **assistants-func-call-stream**: Streaming function calls

**Decision**: PORT ALL as separate examples
- Each demonstrates distinct capabilities worth preserving

### 4. Audio API (4 examples)
- **audio-speech**: Text-to-speech
- **audio-speech-stream**: Streaming TTS
- **audio-transcribe**: Speech-to-text
- **audio-translate**: Audio translation

**Decision**: MERGE into 2 examples
- `audio_speech.rs` - Combines TTS and streaming
- `audio_transcription.rs` - Combines transcribe and translate

### 5. Images API (4 examples)
- **create-image**: Basic generation
- **create-image-b64-json**: Base64 output
- **create-image-edit**: Image editing
- **create-image-variation**: Variations

**Decision**: MERGE into 1 comprehensive example
- `images_comprehensive.rs` - Shows all image operations

### 6. Embeddings (2 examples)
- **embeddings**: Basic usage
- **embeddings-test**: Testing patterns

**Decision**: MERGE into 1 example
- `embeddings.rs` - Includes testing patterns

### 7. Function/Tool Calling (4 examples)
- **function-call**: Legacy function calling
- **function-call-stream**: Streaming functions
- **tool-call**: Modern tool calling
- **tool-call-stream**: Streaming tools

**Decision**: PORT modern tool calling only
- `tool_calling.rs` - Modern approach with streaming

### 8. Structured Outputs (2 examples)
- **structured-outputs**: Using response_format
- **structured-outputs-schemars**: With schemars integration

**Decision**: MERGE into 1 example
- `structured_outputs.rs` - Shows both approaches

### 9. Other Core APIs
- **completions** (3 examples): DROP - Legacy API
- **moderations**: PORT as-is
- **models**: PORT as-is
- **vision-chat**: PORT as-is
- **vector-store-retrieval**: PORT as-is
- **realtime** (2 examples): DEFER - API not ready

### 10. Platform/Integration Examples
- **azure-openai-service**: DROP - Platform-specific
- **gemini-openai-compatibility**: DROP - Third-party
- **ollama-chat**: DROP - Third-party
- **observability-langfuse**: DROP - Separate concern
- **middleware-demo**: ADAPT for ergonomic patterns
- **mock-mode**: ADAPT for testing patterns

## Recommended Example Structure

### Phase 1: Core Examples (MUST HAVE)
1. `responses_comprehensive.rs` - Primary modern API
2. `responses_streaming.rs` - Streaming patterns
3. `chat_comprehensive.rs` - Chat completions fallback
4. `assistants_basic.rs` - Assistant API intro
5. `audio_speech.rs` - Text-to-speech
6. `audio_transcription.rs` - Speech-to-text
7. `images_comprehensive.rs` - All image operations
8. `embeddings.rs` - Vector embeddings
9. `structured_outputs.rs` - JSON mode and schemas
10. `vision_chat.rs` - Image understanding
11. `quickstart.rs` - 5-minute getting started

### Phase 2: Extended Examples (SHOULD HAVE)
12. `tool_calling.rs` - Function/tool calling
13. `assistants_code_interpreter.rs` - Code execution
14. `assistants_file_search.rs` - RAG patterns
15. `vector_store_retrieval.rs` - Vector search
16. `moderations.rs` - Content filtering
17. `models.rs` - Model listing
18. `error_handling.rs` - Comprehensive error patterns
19. `retry_patterns.rs` - Resilience strategies
20. `responses_web_search.rs` - Web-grounded generation

### Phase 3: Advanced Examples (NICE TO HAVE)
21. `batch_processing.rs` - Batch API usage
22. `fine_tuning.rs` - Model fine-tuning
23. `testing_patterns.rs` - Mock and test strategies
24. `middleware_custom.rs` - Request/response interceptors
25. `streaming_advanced.rs` - SSE parsing details

### Phase 4: Future Examples (WHEN READY)
26. `realtime_chat.rs` - WebSocket realtime API
27. `multi_modal.rs` - Combined vision/audio
28. `agent_patterns.rs` - Complex agent workflows

## Gaps Identified

### Missing Core Patterns
1. **Authentication varieties** - API key, OAuth, Azure AD
2. **Retry with exponential backoff** - Production resilience
3. **Rate limit handling** - 429 response management
4. **Batch operations** - Efficient bulk processing
5. **Caching strategies** - Response caching patterns

### Missing Ergonomic Helpers
1. **Quick one-liners** - `openai.quick_response("prompt")`
2. **Common presets** - `ChatPresets::customer_support()`
3. **Validation helpers** - Token counting, prompt validation
4. **Response parsers** - Extract code blocks, JSON, etc.
5. **Conversation managers** - Thread and context handling

## Implementation Issues Found

### API Generation Problems
1. Empty enum variants in `openai-client-base`:
   - `ChatCompletionMessageToolCallsInner`
   - `MessageContentInner`
   - Causes compilation failures in multiple examples

2. Missing implementations:
   - `RealtimeClient` not generated
   - Some streaming helpers incomplete

3. Type mismatches:
   - `response_format` handling inconsistent
   - Tool/function call types need wrapper

### Ergonomic Improvements Needed
1. **Builder complexity** - Some builders require too much boilerplate
2. **Error messages** - Need more context and suggestions
3. **Default values** - Should have sensible defaults
4. **Type inference** - Reduce explicit type annotations
5. **Async patterns** - Cleaner async/await usage

## Migration Guide Outline

For users coming from:
1. **openai-experiment** - Direct mapping table
2. **async-openai** - API comparison
3. **OpenAI Python** - Rust equivalents
4. **Raw HTTP** - Builder advantages

## Testing Strategy

1. **Doc tests** - Every example snippet must compile
2. **Integration tests** - Against mock server
3. **Smoke tests** - Optional real API validation
4. **Example CI** - All examples must build

## Next Steps

1. Fix API generation issues in `openai-client-base`
2. Create `examples/` directory structure
3. Implement Phase 1 examples with ergonomic builders
4. Write migration guide from existing examples
5. Add missing helper patterns identified

## Appendix: Detailed Example Analysis

[The detailed analysis from the agent's research of all 49 examples would go here, but omitted for brevity. It includes specific findings about each example's purpose, implementation issues, and porting recommendations.]