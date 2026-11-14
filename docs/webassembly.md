# WebAssembly

The crate builds for `wasm32-unknown-unknown` in a JavaScript host (a browser, a web worker,
Node.js, Cloudflare Workers): the client, the builders, streaming, interceptors, the tool
framework, providers and authentication, large uploads and webhooks work as natively. Build it
without the default features: the TLS features add nothing there (the host's `fetch` does TLS)
and `otel` (Langfuse/OpenTelemetry export) is native only.

```bash
cargo add openai-ergonomic --no-default-features
# optionally: --features webhook,tracing-http
cargo build --target wasm32-unknown-unknown --no-default-features --features webhook
```

## What differs

Behind `#[cfg(target_family = "wasm")]`, following
[`timvw-rust-runtime`'s WebAssembly support](https://docs.rs/timvw-rust-runtime):

- **Configuration.** There is no process environment: `Config::from_env`, `Client::from_env`
  and `ClientBuilder::from_env` are not available. Use `Config::builder()` (for example with a
  Worker's `env` bindings).
- **HTTP.** reqwest sends with the host's `fetch`, which owns connections, timeouts and the user
  agent: the default client has no 120 second timeout there (retries remain; add a
  `timvw_rust_runtime::timeout::TimeoutLayer` if you need one, tower's does not work there).
  `Config::builder().http_client(..)` takes a `timvw_rust_runtime::Client` as natively; its
  service need not be `Send`.
- **`Send` and `Sync`.** The host's futures cannot leave its thread, so on WebAssembly nothing
  needs to be `Send` or `Sync`: `FunctionTool`, `Interceptor`, `AuthProvider`, their futures,
  the interceptor state `T` of a `Client<T>`, `BearerTokenAuth`'s fetch function and
  `BoxedChatStream` are bound by `MaybeSend`/`MaybeSync` (`Send`/`Sync` natively, nothing on
  WebAssembly). A tool, an interceptor or an interceptor's state may hold an `Rc`.
- **Time and tasks.** Streaming interceptor callbacks run with
  `wasm_bindgen_futures::spawn_local` instead of a tokio task (no tokio runtime is needed);
  retry backoff (the runtime's retry layer and `upload_large_file`'s parts) waits with the
  host's `setTimeout`; clocks are `performance.now()`/`Date.now()` (`web-time`), as
  `std::time::Instant::now()` panics there. `AuthToken::expires_at` and
  `WebhookVerifier::verify_at` take `web_time::Instant`/`web_time::SystemTime` there.
- **No files.** `ImagesClient::save`/`save_all`, `AudioClient::speech_to_file` and
  `FilesClient::download_to` are native only. Upload bytes or streams
  (`Upload::bytes(..).file_name(..)`); a path upload fails with an `Error::Http` whose
  `io_error()` is of kind `Unsupported`. `upload_large_file` sends its parts concurrently on the
  host's event loop. `test-utils` (mockito) is native only.

## Tests

`tests/wasm.rs` runs a chat completion, a streamed one with an interceptor, a streamed Responses
API response, an interceptor whose state holds an `Rc`, a tool call through the tool framework, a
webhook verification, and retries with backoff in Node.js against a scripted, non-`Send` tower service, with
[`wasm-bindgen-test-runner`](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/index.html)
(install `wasm-bindgen-cli` of the `wasm-bindgen` version in `Cargo.lock`):

```bash
rustup target add wasm32-unknown-unknown
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test --target wasm32-unknown-unknown --no-default-features --features webhook --test wasm
cargo clippy --target wasm32-unknown-unknown --no-default-features --features webhook,tracing-http \
  --lib --test wasm -- -D warnings
```

CI runs both (the `wasm-clippy-args` / `wasm-test-args` of `.github/workflows/ci.yml`). The other
tests and the examples are native only.
