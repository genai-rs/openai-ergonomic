# API coverage

Which `OpenAI` APIs have ergonomic builders and a sub-client, and how to reach the others.

## With builders

| API | Client | Builders | Examples |
|-----|--------|----------|----------|
| Chat completions | `client.chat()`, `send_chat` | [`chat.rs`](../src/builders/chat.rs) | [`chat_comprehensive.rs`](../examples/chat_comprehensive.rs), [`vision_chat.rs`](../examples/vision_chat.rs), [`quickstart.rs`](../examples/quickstart.rs) |
| Responses | `client.responses()`, `send_responses`, `send_responses_stream`, `client.responses_api()` | [`responses_api/`](../src/responses_api) | [`responses_comprehensive.rs`](../examples/responses_comprehensive.rs), [`responses_streaming.rs`](../examples/responses_streaming.rs), [`structured_outputs.rs`](../examples/structured_outputs.rs) |
| Audio | `client.audio()` | [`audio.rs`](../src/builders/audio.rs) | [`audio_speech.rs`](../examples/audio_speech.rs), [`audio_transcription.rs`](../examples/audio_transcription.rs) |
| Embeddings | `client.embeddings()` | [`embeddings.rs`](../src/builders/embeddings.rs) | [`embeddings.rs`](../examples/embeddings.rs) |
| Images | `client.images()` | [`images.rs`](../src/builders/images.rs) | [`images_comprehensive.rs`](../examples/images_comprehensive.rs) |
| Assistants | `client.assistants()` | [`assistants.rs`](../src/builders/assistants.rs) | [`assistants_basic.rs`](../examples/assistants_basic.rs), [`assistants_code_interpreter.rs`](../examples/assistants_code_interpreter.rs), [`assistants_file_search.rs`](../examples/assistants_file_search.rs) |
| Threads | `client.threads()` | [`threads.rs`](../src/builders/threads.rs) | [`threads.rs`](../examples/threads.rs) |
| Files | `client.files()` | [`files.rs`](../src/builders/files.rs) | [`files.rs`](../examples/files.rs) |
| Uploads | `client.uploads()`, `upload_large_file` | [`uploads.rs`](../src/builders/uploads.rs) | [`uploads.rs`](../examples/uploads.rs) |
| Vector stores | `client.vector_stores()` | [`vector_stores.rs`](../src/builders/vector_stores.rs) | [`vector_stores.rs`](../examples/vector_stores.rs) |
| Batch | `client.batch()` | [`batch.rs`](../src/builders/batch.rs) | [`batch_processing.rs`](../examples/batch_processing.rs) |
| Fine-tuning | `client.fine_tuning()` | [`fine_tuning.rs`](../src/builders/fine_tuning.rs) | [`fine_tuning.rs`](../examples/fine_tuning.rs) |
| Moderations | `client.moderations()` | [`moderations.rs`](../src/builders/moderations.rs) | [`moderations.rs`](../examples/moderations.rs) |
| Models | `client.models()` | [`models.rs`](../src/builders/models.rs) | [`models.rs`](../examples/models.rs) |
| Usage | `client.usage()` | [`usage.rs`](../src/builders/usage.rs) | [`usage.rs`](../examples/usage.rs) |
| Completions (legacy) | `client.completions()` | [`completions.rs`](../src/builders/completions.rs) | [`completions.rs`](../examples/completions.rs) |

Each has integration tests against mock servers under [`tests/`](../tests). Webhook deliveries
(not an API call) are covered by [webhooks](webhooks.md).

## Without builders

- **The other operations of the enabled API groups** (`Cargo.toml` lists them) are available as
  the generated `openai-client-base` functions; send them through the client's HTTP stack (see
  [the generated operations](chat-and-responses.md#the-generated-operations)).
- **Any other endpoint** (the Responses API's input items and token counts, Realtime client secrets, Evals,
  Conversations, the administration APIs, ...): `post_json`, `get_json`, `send_json` and
  `send_json_stream` send a request and decode the reply with types of your own, with the
  client's provider, authentication and retries (see
  [bring your own types](chat-and-responses.md#bring-your-own-types)). Realtime sessions over
  WebRTC or WebSocket are not covered.
