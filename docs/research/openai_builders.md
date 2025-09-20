# openai-builders Audit Notes

## Module Coverage
`openai-experiment/crates/openai-builders` exposes a wide set of modules under `src/`:
- Core surface: `responses`, `chat`, `assistants`, `audio`, `embeddings`, `images`, `files`, `fine_tuning`, `batch`, `vector_stores`, `moderations`, `threads`, `uploads`, `usage`.
- Support utilities: `constants`, `types` (type aliases for verbose generated names), `binary`, `streaming`, `sse_client`, `client`, `config`, `middleware`, `observability` helpers (feature-gated).
- Additional admin endpoints: `projects`, `users`, `invites`, `certificates`, `audit_logs`, `admin`.
- Streaming helpers (`streaming.rs`, `streaming_instrumentation.rs`, `test_streaming.rs`) leverage `futures` streams and SSE wrappers.

Each module typically exposes:
- `bon::builder` functions returning OpenAPI structs while smoothing option handling.
- Convenience helpers (e.g., `responses::tool_web_search`, `chat::sys/user/part_*` builders).
- Type aliases to hide nested Option<Option<T>> or verbose names.

### Known TODOs / Workarounds
- `responses.rs`: `tools` and `tool_choice` fields temporarily set to `None` due to spec mismatch; comments call out required generator fixes.
- `responses` tool-choice helpers are commented out because `ResponsePropertiesToolChoice` is generated as an empty enum.
- Various builders construct structs manually (`CreateAssistantRequest`, etc.) to compensate for OpenAPI generator limitations.
- Observability features gated behind `observability` feature flag and rely on optional crates (OpenTelemetry, tracing, etc.).
- Minimal error abstractions beyond wrappers around `openai-client-base` errors.

## Example Coverage
`openai-experiment/examples/` offers reference implementations for nearly every OpenAI capability:
- Responses API: `responses/`, `responses-basic`, `responses-stream`, `responses-function-call`, `responses-web-search`.
- Chat legacy: `chat/`, `chat-stream`, `chat-store`, `completions` variants.
- Assistants: standard/tooling/streaming examples, file search, code interpreter.
- Audio: speech synthesis, transcription, translation (sync + streaming).
- Images: generation, edits, variations, base64 responses.
- Moderation, embeddings, vector store retrieval, structured outputs, middleware demos.
- Azure compatibility, Gemini compatibility, BYO types, instrumentation/observability demos.
- `README.md` in examples directory documents status, TODOs, and testing notes.

These examples provide patterns for ergonomic helper APIs and should inform which high-level helpers we port (e.g., `tool_function`, binary helpers, streaming instrumentation).

## Implications for openai-ergonomic
- Preserve module boundaries and naming to ease migration (e.g., `responses::*`, `constants::models` constants, `types::*` aliases).
- Plan to address current TODOs (tool choice handling, generator fixes) or document them clearly if deferring.
- Build example suite mirroring the coverage above, but streamlined for ergonomic crate usage (likely fewer but higher-level examples).
- Consider feature flags (`observability`, `realtime`, etc.) to keep optional dependencies off by default.
- Capture these notes in design specs (`docs/design/api_surface.md`) and research docs to guide implementation.
