# Examples Index

Complete guide to all examples in the `openai-ergonomic` crate.

## Quick Navigation

- [Getting Started](#getting-started)
- [Core AI Features](#core-ai-features)
- [Audio](#audio)
- [Images](#images)
- [Embeddings](#embeddings)
- [Assistants & Tools](#assistants--tools)
- [Batch Processing](#batch-processing)
- [Advanced Patterns](#advanced-patterns)

---

## Getting Started

### [`quickstart.rs`](../examples/quickstart.rs)
**What it demonstrates:** Minimal example to get started with chat completions

```bash
cargo run --example quickstart
```

**Key concepts:**
- Client initialization from environment
- Simple chat completion
- Basic error handling

**Prerequisites:** `OPENAI_API_KEY` environment variable

---

## Core AI Features

### Chat & Completions

#### [`chat_comprehensive.rs`](../examples/chat_comprehensive.rs)
**What it demonstrates:** Comprehensive chat API features

```bash
cargo run --example chat_comprehensive
```

**Key concepts:**
- Multiple message types (system, user, assistant)
- Temperature and model selection
- Streaming responses
- Token usage tracking
- Stop sequences
- JSON mode
- Seed for reproducibility

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`vision_chat.rs`](../examples/vision_chat.rs)
**What it demonstrates:** Vision capabilities with GPT-4 Vision

```bash
cargo run --example vision_chat
```

**Key concepts:**
- Image URL input
- Base64 image encoding
- Image detail levels (low, high, auto)
- Multi-modal prompts

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`tool_calling.rs`](../examples/tool_calling.rs)
**What it demonstrates:** Advanced function/tool calling

```bash
cargo run --example tool_calling
```

**Key concepts:**
- Function definitions
- Tool choice strategies
- Parallel function calling
- Function result integration
- Multi-turn conversations with tools

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`tool_calling_simple.rs`](../examples/tool_calling_simple.rs)
**What it demonstrates:** Simple tool calling example

```bash
cargo run --example tool_calling_simple
```

**Key concepts:**
- Basic function definition
- Single tool call
- Simple integration

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`tool_framework.rs`](../examples/tool_framework.rs)
**What it demonstrates:** Registering tools with typed inputs using the unified tool framework

```bash
cargo run --example tool_framework
```

**Key concepts:**
- Defining tools with the `tool!` macro
- Typed input parameters with automatic schema generation
- Executing tools via `ToolRegistry`
- Returning JSON payloads to the model

**Prerequisites:** None (offline example)

---

#### [`tool_framework_typed.rs`](../examples/tool_framework_typed.rs)
**What it demonstrates:** Strongly typed inputs and outputs plus processing tool calls

```bash
cargo run --example tool_framework_typed
```

**Key concepts:**
- Typed tool outputs serialized automatically
- Using `ToolRegistry::process_tool_calls`
- Converting tool call results into chat tool messages
- Offline simulation of OpenAI tool calls

**Prerequisites:** None (offline example)

---

#### [`tool_calling_multiturn.rs`](../examples/tool_calling_multiturn.rs)
**What it demonstrates:** Multi-turn tool calling with proper conversation history

```bash
cargo run --example tool_calling_multiturn
```

**Key concepts:**
- Multi-turn tool calling loops
- Proper use of `assistant_with_tool_calls()` for message history
- Sequential and parallel tool execution
- Real-world tools (calculator, memory storage)
- Iteration limits and error handling

**Prerequisites:** `OPENAI_API_KEY`

---

### Responses API (Structured Outputs)

#### [`responses_comprehensive.rs`](../examples/responses_comprehensive.rs)
**What it demonstrates:** Comprehensive responses API features

```bash
cargo run --example responses_comprehensive
```

**Key concepts:**
- Background response processing
- Structured output schemas (JSON Schema)
- Response retrieval
- Cancellation
- Input item management

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`chat_streaming.rs`](../examples/chat_streaming.rs)
**What it demonstrates:** Real-time chat completion streaming with Server-Sent Events (SSE)

```bash
cargo run --example chat_streaming
```

**Key concepts:**
- Basic streaming with `send_chat_stream()`
- Processing chunks in real-time
- Streaming with parameters (temperature, max_tokens)
- Collecting full content from stream
- Streaming with system messages
- Multi-turn conversations with streaming

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`responses_streaming.rs`](../examples/responses_streaming.rs)
**What it demonstrates:** Streaming responses API

```bash
cargo run --example responses_streaming
```

**Key concepts:**
- Real-time streaming
- Chunk processing
- Event handling
- Stream error handling

**Prerequisites:** `OPENAI_API_KEY`

---

#### [`langfuse_streaming.rs`](../examples/langfuse_streaming.rs)
**What it demonstrates:** Streaming with Langfuse observability and tracing

```bash
cargo run --example langfuse_streaming
```

**Key concepts:**
- Interceptor integration with streaming
- Real-time chunk-level tracing
- Token usage tracking in streams
- Streaming observability patterns
- Proper shutdown with spawned tasks

**Prerequisites:** `OPENAI_API_KEY`, `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`

---

#### [`structured_outputs.rs`](../examples/structured_outputs.rs)
**What it demonstrates:** JSON Schema-based structured outputs

```bash
cargo run --example structured_outputs
```

**Key concepts:**
- JSON Schema definition
- Type-safe responses
- Schema validation
- Nested structures
- Enums and required fields

**Prerequisites:** `OPENAI_API_KEY`

---

## Audio

### [`audio_speech.rs`](../examples/audio_speech.rs)
**What it demonstrates:** Text-to-speech conversion

```bash
cargo run --example audio_speech
```

**Key concepts:**
- Voice selection (alloy, echo, fable, onyx, nova, shimmer)
- Audio formats (mp3, opus, aac, flac, wav, pcm)
- Speed control
- Saving audio files

**Prerequisites:** `OPENAI_API_KEY`

---

### [`audio_transcription.rs`](../examples/audio_transcription.rs)
**What it demonstrates:** Audio transcription and translation

```bash
cargo run --example audio_transcription
```

**Key concepts:**
- Audio file transcription
- Language detection
- Translation to English
- Response formats (json, text, srt, vtt)
- Timestamp granularities
- Temperature control

**Prerequisites:**
- `OPENAI_API_KEY`
- Audio file for transcription

---

## Images

### [`images_comprehensive.rs`](../examples/images_comprehensive.rs)
**What it demonstrates:** Complete image API features

```bash
cargo run --example images_comprehensive
```

**Key concepts:**
- Image generation from prompts
- Image editing with masks
- Image variations
- Size options (256x256, 512x512, 1024x1024, 1792x1024, 1024x1792)
- Quality settings (standard, hd)
- Style options (vivid, natural)
- Response formats (URL, base64)
- DALL-E 2 and DALL-E 3

**Prerequisites:**
- `OPENAI_API_KEY`
- Image files for editing/variations

---

## Embeddings

### [`embeddings.rs`](../examples/embeddings.rs)
**What it demonstrates:** Text embeddings generation

```bash
cargo run --example embeddings
```

**Key concepts:**
- Single text embedding
- Batch embeddings
- Dimensionality control
- Cosine similarity calculation
- Vector operations

**Prerequisites:** `OPENAI_API_KEY`

---

## Assistants & Tools

### [`assistants_basic.rs`](../examples/assistants_basic.rs)
**What it demonstrates:** Basic assistant usage

```bash
cargo run --example assistants_basic
```

**Key concepts:**
- Assistant creation
- Thread creation
- Message posting
- Run execution
- Response retrieval
- Cleanup

**Prerequisites:** `OPENAI_API_KEY`

---

### [`assistants_code_interpreter.rs`](../examples/assistants_code_interpreter.rs)
**What it demonstrates:** Code Interpreter tool

```bash
cargo run --example assistants_code_interpreter
```

**Key concepts:**
- Code execution
- Data analysis
- Chart generation
- File output
- Math calculations

**Prerequisites:** `OPENAI_API_KEY`

---

### [`assistants_file_search.rs`](../examples/assistants_file_search.rs)
**What it demonstrates:** File Search tool (RAG)

```bash
cargo run --example assistants_file_search
```

**Key concepts:**
- Vector store creation
- File upload
- Document search
- Knowledge retrieval
- Citations

**Prerequisites:**
- `OPENAI_API_KEY`
- Document files

---

### [`vector_stores.rs`](../examples/vector_stores.rs)
**What it demonstrates:** Vector store operations

```bash
cargo run --example vector_stores
```

**Key concepts:**
- Vector store creation
- File management
- Search operations
- Expiration policies

**Prerequisites:**
- `OPENAI_API_KEY`
- Document files

---

## Batch Processing

### [`batch_processing.rs`](../examples/batch_processing.rs)
**What it demonstrates:** Batch API for async processing

```bash
cargo run --example batch_processing
```

**Key concepts:**
- Batch job creation
- JSONL file format
- Job status polling
- Result retrieval
- Cost optimization (50% discount)
- Completion windows (24h)

**Prerequisites:**
- `OPENAI_API_KEY`
- JSONL batch file

---

## Content Moderation

### [`moderations.rs`](../examples/moderations.rs)
**What it demonstrates:** Content moderation API

```bash
cargo run --example moderations
```

**Key concepts:**
- Text classification
- Category detection (hate, violence, sexual, etc.)
- Confidence scores
- Flagged content detection

**Prerequisites:** `OPENAI_API_KEY`

---

## Models

### [`models.rs`](../examples/models.rs)
**What it demonstrates:** List and retrieve model information

```bash
cargo run --example models
```

**Key concepts:**
- Listing available models
- Model details retrieval
- Model capabilities
- Ergonomic API available

**Prerequisites:** `OPENAI_API_KEY`

---

## Fine-tuning

### [`fine_tuning.rs`](../examples/fine_tuning.rs)
**What it demonstrates:** Comprehensive fine-tuning API features

```bash
cargo run --example fine_tuning
```

**Key concepts:**
- Creating fine-tuning jobs
- Listing and monitoring jobs
- Viewing training events
- Managing checkpoints
- Job cancellation
- Weights & Biases integration

**Prerequisites:**
- `OPENAI_API_KEY`
- Training data file (JSONL format)

---

## Uploads

### [`uploads.rs`](../examples/uploads.rs)
**What it demonstrates:** Large file uploads with multipart support

```bash
cargo run --example uploads
```

**Key concepts:**
- Multipart upload creation
- Uploading file parts in chunks
- Upload completion
- Progress tracking
- Error handling and retry logic
- Best for files >512 MB

**Prerequisites:**
- `OPENAI_API_KEY`
- Large file to upload

---

## Threads

### [`threads.rs`](../examples/threads.rs)
**What it demonstrates:** Conversation thread management

```bash
cargo run --example threads
```

**Key concepts:**
- Creating conversation threads
- Thread metadata
- Persistent conversation state
- Thread lifecycle management
- Use cases for threads

**Prerequisites:** `OPENAI_API_KEY`

---

## Completions (Legacy)

### [`completions.rs`](../examples/completions.rs)
**What it demonstrates:** Legacy completions API

```bash
cargo run --example completions
```

**Key concepts:**
- Text completions
- Temperature and parameters
- Stop sequences
- Multiple choices (n parameter)
- Echo mode
- Insert mode with suffix
- Logprobs

**Prerequisites:** `OPENAI_API_KEY`

**Note:** Legacy API. Use Chat Completions for new applications.

---

## Usage Tracking

### [`usage.rs`](../examples/usage.rs)
**What it demonstrates:** API usage and cost tracking

```bash
cargo run --example usage
```

**Key concepts:**
- Querying usage data by time range
- Filtering by model, project, user
- Daily/hourly aggregation
- Cost data retrieval
- Usage for different endpoints (completions, embeddings, images, audio)

**Prerequisites:** `OPENAI_API_KEY`

---

## Files

### [`files.rs`](../examples/files.rs)
**What it demonstrates:** File upload and management

```bash
cargo run --example files
```

**Key concepts:**
- Uploading files for assistants/fine-tuning
- Listing uploaded files
- Retrieving file content
- Deleting files
- File purposes (assistants, fine-tune, batch)

**Prerequisites:**
- `OPENAI_API_KEY`
- Files to upload

---

## Advanced Patterns

### [`auth_patterns.rs`](../examples/auth_patterns.rs)
**What it demonstrates:** Authentication patterns

```bash
cargo run --example auth_patterns
```

**Key concepts:**
- Environment-based auth
- Explicit configuration
- Organization IDs
- Project IDs
- Custom headers

---

### [`caching_strategies.rs`](../examples/caching_strategies.rs)
**What it demonstrates:** Response caching strategies

```bash
cargo run --example caching_strategies
```

**Key concepts:**
- In-memory caching
- Redis caching
- Cache invalidation
- TTL management
- Cache key generation

**Prerequisites:**
- `OPENAI_API_KEY`
- Redis (for Redis examples)

---

### [`error_handling.rs`](../examples/error_handling.rs)
**What it demonstrates:** Comprehensive error handling

```bash
cargo run --example error_handling
```

**Key concepts:**
- Error types (API, network, validation)
- Status code handling
- Retry logic
- Graceful degradation
- Error recovery

**Prerequisites:** `OPENAI_API_KEY`

---

### [`middleware_patterns.rs`](../examples/middleware_patterns.rs)
**What it demonstrates:** Request/response middleware

```bash
cargo run --example middleware_patterns
```

**Key concepts:**
- Logging middleware
- Metrics collection
- Request transformation
- Response transformation
- Middleware chaining

**Prerequisites:** `OPENAI_API_KEY`

---

### [`retry_patterns.rs`](../examples/retry_patterns.rs)
**What it demonstrates:** Retry strategies

```bash
cargo run --example retry_patterns
```

**Key concepts:**
- Exponential backoff
- Jitter
- Max retries
- Retry conditions
- Circuit breaker pattern

**Prerequisites:** `OPENAI_API_KEY`

---

### [`testing_patterns.rs`](../examples/testing_patterns.rs)
**What it demonstrates:** Testing strategies

```bash
cargo run --example testing_patterns
```

**Key concepts:**
- Mock server setup
- Fixture management
- Async testing
- Integration testing
- Test utilities

---

### [`token_counting.rs`](../examples/token_counting.rs)
**What it demonstrates:** Token usage and estimation

```bash
cargo run --example token_counting
```

**Key concepts:**
- Token estimation
- Cost calculation
- Usage tracking
- Model-specific pricing
- Optimization strategies

**Prerequisites:** `OPENAI_API_KEY`

---

## Running Examples

### Basic Usage

```bash
# Run a specific example
cargo run --example quickstart

# Run with environment file
OPENAI_API_KEY=your-key cargo run --example chat_comprehensive

# Run with .env file
cp .env.example .env  # Edit with your API key
cargo run --example quickstart
```

### With Logging

```bash
# Enable debug logs
RUST_LOG=debug cargo run --example chat_comprehensive

# Enable trace logs
RUST_LOG=trace cargo run --example streaming_responses
```

### Environment Setup

Create a `.env` file in the project root:

```env
OPENAI_API_KEY=sk-...
OPENAI_ORG_ID=org-...  # Optional
```

---

## Example Categories Summary

| Category | Examples | Status |
|----------|----------|--------|
| **Chat** | 5 | Complete |
| **Responses** | 3 | Complete |
| **Audio** | 2 | Complete |
| **Images** | 1 | Complete |
| **Embeddings** | 1 | Complete |
| **Assistants** | 3 | Complete |
| **Vector Stores** | 1 | Complete |
| **Batch** | 1 | Complete |
| **Moderations** | 1 | Complete |
| **Models** | 1 | Complete |
| **Fine-tuning** | 1 | Complete |
| **Uploads** | 1 | Complete |
| **Threads** | 1 | Complete |
| **Completions** | 1 | Complete |
| **Usage** | 1 | Complete |
| **Files** | 1 | Complete |
| **Patterns** | 7 | Complete |
| **Total** | **33** | **100% fully implemented** |

---

## Next Steps

- [API Coverage Documentation](./api-coverage.md) - See what APIs are available
- [Getting Started Guide](./getting-started.md) - Start building with openai-ergonomic
- [Architecture Overview](./architecture.md) - Understand the design
- [Contributing](../CONTRIBUTING.md) - Help add missing APIs

---

## Need Help?

- [Full Documentation](https://docs.rs/openai-ergonomic)
- [Report Issues](https://github.com/genai-rs/openai-ergonomic/issues)
- [Discussions](https://github.com/genai-rs/openai-ergonomic/discussions)
