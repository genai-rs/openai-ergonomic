# Examples

The [`examples/`](../examples) directory has a runnable program per area. Run one with
`cargo run --example <name>`; the module documentation at the top of each file says what it shows
and what it needs.

Most examples call the API: set `OPENAI_API_KEY` (or the `AZURE_OPENAI_*` variables, see
[configuration](configuration.md#azure-openai)). The ones marked *offline* need no key.

```bash
export OPENAI_API_KEY="sk-..."
cargo run --example quickstart
RUST_LOG=openai_ergonomic=debug cargo run --example chat_streaming   # with logs
```

## Getting started

| Example | Shows |
| --- | --- |
| [`quickstart`](../examples/quickstart.rs) | a client from the environment, chat, streaming, errors |

## Chat and the Responses API

| Example | Shows |
| --- | --- |
| [`chat_comprehensive`](../examples/chat_comprehensive.rs) | multi-turn conversations, message history, parameters, token usage |
| [`chat_streaming`](../examples/chat_streaming.rs) | `send_chat_stream`, collecting a stream, parameters, multi-turn streaming |
| [`vision_chat`](../examples/vision_chat.rs) | images by URL and base64, several images, detail levels |
| [`responses_comprehensive`](../examples/responses_comprehensive.rs) | the Responses API: instructions, function calling with `previous_response_id`, stateless conversations, JSON schemas, built-in tools |
| [`responses_streaming`](../examples/responses_streaming.rs) | `send_responses_stream`: text deltas, function calls at `response.output_item.done`, the completion handle |
| [`structured_outputs`](../examples/structured_outputs.rs) | JSON mode and JSON schemas, nested structures |
| [`completions`](../examples/completions.rs) | the legacy Completions API |

## Tools

| Example | Shows |
| --- | --- |
| [`tool_calling_simple`](../examples/tool_calling_simple.rs) | one function tool |
| [`tool_calling`](../examples/tool_calling.rs) | function tools, tool replies, tool calls while streaming |
| [`tool_calling_multiturn`](../examples/tool_calling_multiturn.rs) | the full loop with `assistant_with_tool_calls` and `tool` |
| [`tool_framework`](../examples/tool_framework.rs) | a typed `FunctionTool` run locally (*offline*) |
| [`tool_framework_typed`](../examples/tool_framework_typed.rs) | several tools, async application state, a replayed chat (*offline*) |

See [tool framework](tool_framework.md) and [tool orchestration](tool_orchestration.md).

## Audio, images and embeddings

| Example | Shows |
| --- | --- |
| [`audio_speech`](../examples/audio_speech.rs) | text to speech: voices, formats, speed, saving to a file |
| [`audio_transcription`](../examples/audio_transcription.rs) | transcription and translation, response formats, timestamps |
| [`images_comprehensive`](../examples/images_comprehensive.rs) | generation, edits with masks, variations, saving images |
| [`embeddings`](../examples/embeddings.rs) | single and batch embeddings, dimensions, similarity |

## Files, assistants and batches

| Example | Shows |
| --- | --- |
| [`files`](../examples/files.rs) | upload, list, retrieve, download and delete files |
| [`uploads`](../examples/uploads.rs) | the Uploads API for large files |
| [`vector_stores`](../examples/vector_stores.rs) | vector stores, their files, search |
| [`assistants_basic`](../examples/assistants_basic.rs) | assistants, threads, messages, runs |
| [`assistants_code_interpreter`](../examples/assistants_code_interpreter.rs) | the code interpreter tool |
| [`assistants_file_search`](../examples/assistants_file_search.rs) | the file search tool over vector stores |
| [`threads`](../examples/threads.rs) | threads with messages and metadata |
| [`batch_processing`](../examples/batch_processing.rs) | the Batch API: input files, monitoring, results |
| [`fine_tuning`](../examples/fine_tuning.rs) | fine-tuning jobs, events, checkpoints |
| [`moderations`](../examples/moderations.rs) | moderation categories and scores |
| [`models`](../examples/models.rs) | listing and choosing models |
| [`usage`](../examples/usage.rs) | the Usage API: usage and costs by period, project, model |

See [files and media](files-and-media.md) and [vector stores](vector_store_operations.md).

## Providers, HTTP and observability

| Example | Shows |
| --- | --- |
| [`azure_openai`](../examples/azure_openai.rs) | Azure `OpenAI` from the environment or in code |
| [`azure_comprehensive`](../examples/azure_comprehensive.rs) | several API areas against Azure `OpenAI` |
| [`auth_patterns`](../examples/auth_patterns.rs) | API keys, organization and project, several clients |
| [`http_middleware_retry`](../examples/http_middleware_retry.rs) | retries, timeouts and logging as tower layers (`--features tracing-http` for `TraceLayer`) |
| [`langfuse_simple`](../examples/langfuse_simple.rs) | the Langfuse interceptor, one request (`LANGFUSE_*` variables) |
| [`langfuse`](../examples/langfuse.rs) | Langfuse with sessions, tags and several scenarios |
| [`langfuse_streaming`](../examples/langfuse_streaming.rs) | Langfuse with streams, waiting for the callbacks before shutdown |

See [configuration](configuration.md), [interceptors](interceptors.md) and
[Langfuse integration](langfuse-integration.md).

## Application patterns

| Example | Shows |
| --- | --- |
| [`error_handling`](../examples/error_handling.rs) | matching on `Error`, rate limits, authentication and network errors |
| [`retry_patterns`](../examples/retry_patterns.rs) | backoff, jitter, circuit breakers, fallbacks, idempotency keys |
| [`middleware_patterns`](../examples/middleware_patterns.rs) | a middleware chain of your own: logging, rate limiting, validation (*offline*) |
| [`caching_strategies`](../examples/caching_strategies.rs) | response caches with TTLs and invalidation (*offline*) |
| [`token_counting`](../examples/token_counting.rs) | token estimates and budgets (*offline*) |
| [`testing_patterns`](../examples/testing_patterns.rs) | testing against a mockito server (*offline*) |
