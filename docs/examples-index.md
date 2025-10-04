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

#### [`responses_streaming.rs`](../examples/responses_streaming.rs)
**What it demonstrates:** Streaming responses

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

**Prerequisites:** `OPENAI_API_KEY`

⚠️ **Note:** This example uses the base API directly. Ergonomic wrapper not yet available.

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
| **Chat** | 4 | ✅ Complete |
| **Responses** | 3 | ✅ Complete |
| **Audio** | 2 | ✅ Complete |
| **Images** | 1 | ✅ Complete |
| **Embeddings** | 1 | ✅ Complete |
| **Assistants** | 3 | ⚠️ Partial (no client methods) |
| **Vector Stores** | 1 | ⚠️ Partial (no client methods) |
| **Batch** | 1 | ⚠️ Partial (no client methods) |
| **Moderations** | 1 | ⚠️ Partial (no client methods) |
| **Models** | 1 | ⚠️ No ergonomic API |
| **Patterns** | 7 | ✅ Complete |
| **Total** | **26** | **21% fully implemented** |

---

## Next Steps

- [API Coverage Documentation](./api-coverage.md) - See what APIs are available
- [Getting Started Guide](./getting-started.md) - Start building with openai-ergonomic
- [Architecture Overview](./architecture.md) - Understand the design
- [Contributing](../CONTRIBUTING.md) - Help add missing APIs

---

## Need Help?

- 📖 [Full Documentation](https://docs.rs/openai-ergonomic)
- 🐛 [Report Issues](https://github.com/genai-rs/openai-ergonomic/issues)
- 💬 [Discussions](https://github.com/genai-rs/openai-ergonomic/discussions)
