# Architecture

How `openai-ergonomic` is put together, for contributors and for users who want to know what
happens under a builder.

## Layers

```text
┌─────────────────────────┐
│  openai-ergonomic       │  ← client, builders, response helpers, streaming,
│                         │    interceptors, tools, providers, webhooks
├─────────────────────────┤
│  openai-client-base     │  ← generated from the OpenAPI spec: models + operations
├─────────────────────────┤
│  timvw-rust-runtime     │  ← tower HTTP client around reqwest: retries, timeouts,
│  (tower + reqwest)      │    provider and auth layers, uploads, server-sent events
└─────────────────────────┘
```

[`openai-client-base`](https://github.com/genai-rs/openai-client-base) is generated from
`OpenAI`'s OpenAPI spec; this crate enables the API groups it calls (see `Cargo.toml`). Both send
through a [`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime) `Client`, so the builders,
the generated operations and the raw `post_json` / `send_json` requests share the provider,
authentication, retries and layers. The same code builds natively (tokio) and for
`wasm32-unknown-unknown` (see [WebAssembly](webassembly.md)).

## Modules

```text
src/
├── lib.rs                  crate root: re-exports, the docs/ pages as doc tests
├── client/                 Client, ClientBuilder and a sub-client per API area
│   ├── mod.rs              Client<T>: chat, streaming, sub-client accessors
│   ├── responses.rs        Client<T>: the Responses API; ResponsesClient
│   ├── raw.rs              post_json / get_json / send_json / send_json_stream
│   ├── large_upload.rs     upload_large_file (Uploads API, parallel parts)
│   ├── media.rs            saving images, speech and files to disk (native only)
│   ├── headers.rs          OpenAI-Organization / OpenAI-Project headers
│   └── assistants.rs, audio.rs, batch.rs, ... one sub-client per API area
├── builders/               request builders per API area (Builder::build → generated request)
├── responses/              response wrappers (ChatCompletionResponseWrapper) and helpers
├── responses_api/          Responses API: ResponsesBuilder, ResponsesResponse, ResponsesEvent
├── config.rs               Config, ConfigBuilder, from_env
├── provider.rs             Provider presets and their tower layer
├── auth.rs                 AuthProvider, BearerTokenAuth, AuthLayer
├── errors.rs               Error, Result, RetryClass
├── streaming.rs            chunks, ChatCompletionStream, InterceptedStream, StreamCompletion
├── sse_frames.rs           server-sent event frames, `[DONE]` told apart from the end of the body
├── interceptor.rs          Interceptor<T>, its contexts, InterceptorChain
├── tool_framework/         FunctionTool, RichTool, CustomTool, DynamicTool, ToolRegistry
│   ├── output.rs           ToolInvocation, CallAnswer, ToolOutputBody, truncation
│   ├── names.rs            tool name sanitizer, ToolNamer
│   └── schema.rs           JSON Schema sanitizer and compaction
├── rate_limits.rs          RateLimitSnapshot, the layer that records x-ratelimit-* headers
├── langfuse_*.rs           Langfuse interceptor, exporter and context (feature `otel`)
├── semantic_conventions.rs OpenTelemetry GenAI attribute and operation names
├── webhooks.rs             WebhookVerifier, WebhookEvent (feature `webhook`)
├── azure_middleware.rs     deprecated Azure layer (use Provider::azure)
├── time.rs                 clocks, timers and tasks, native and WebAssembly
└── test_utils.rs           mockito helpers (feature `test-utils`, native only)
```

## The client

`Client<T = ()>` holds the `Config`, the HTTP client and the generated `Configuration`, and an
`InterceptorChain<T>`, all shared (`Arc`), so cloning a client is cheap. `Client::from_env()` or
`Client::builder(config)` gives a `ClientBuilder`; `with_interceptor` / `add_interceptor` add
interceptors and `build()` makes the client. `T` is the interceptor state passed from
`before_request` to the later hooks of one request (see [interceptors](interceptors.md)).

- `chat()` / `responses()` start a `ChatCompletionBuilder` / `ResponsesBuilder` with the default
  model; `send_chat`, `send_responses` and their `_stream` and `_stream_with_completion`
  variants send them: a `CreateChatCompletionRequest` to `/chat/completions`, a
  `CreateResponse` to `/responses`.
  `responses_api()` retrieves, cancels and deletes stored responses and lists their input items.
- `assistants()`, `audio()`, `batch()`, `completions()`, `embeddings()`, `files()`,
  `fine_tuning()`, `images()`, `models()`, `moderations()`, `threads()`, `uploads()`, `usage()`,
  `vector_stores()` return sub-clients that borrow the client.
- `http_client()` is the HTTP client, for the generated operations or requests of your own.

## A request

1. A builder collects the parameters; `Builder::build()` validates them and returns the
   generated request type (`Error::InvalidRequest` before anything is sent).
2. The client calls the interceptors' `before_request`, then the generated operation (or a raw
   request) through the HTTP client: the provider's layer adds the base URL, credentials,
   headers and paths, the `AuthProvider` its credentials, the retry layer retries.
3. An error status becomes `Error::Api` (with the `OpenAI` error's type and code) and goes to
   `on_error`; a success goes to `after_response` and comes back wrapped
   (`ChatCompletionResponseWrapper`) or as the generated response type.
4. A stream is decoded from server-sent events into `ChatCompletionChunk`s; its interceptor
   callbacks run in order in a background task (see the
   [completion contract](streaming-completion-contract.md)).

## Tests

- **Unit tests** next to the code validate builders, helpers and layers.
- **Integration tests** (`tests/*.rs`) run the client against mockito servers.
- **Doc tests** compile the rustdoc, the README and the pages under `docs/`.
- **WebAssembly tests** (`tests/wasm.rs`) run in Node.js.
- **Examples** are built by CI.

See [development](development.md).
