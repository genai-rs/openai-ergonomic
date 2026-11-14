# Migration

Each release that changes the public API bumps the minor version. Generated request and response
types follow the matching `openai-client-base` release, so also check its
[upgrading notes](https://github.com/genai-rs/openai-client-base/blob/main/guides/upgrading.md)
and the [changelog](../CHANGELOG.md) when you upgrade across a minor version.

## Migrating to 0.22

Version 0.22 moves to `openai-client-base` 0.34, gives the Responses API its own client and
makes the error enums open for new variants. It bundles every breaking change planned for the
tool-framework round, so later releases of that round only add API.

- **`openai-client-base` 0.34.** Code that uses its types directly moves to 0.34: among others
  `CreateResponse.input` is an `Option<InputParam>`, `Response.instructions` an
  `Option<ResponseAllOfInstructions>`, `CreateCompletionRequest.prompt` an
  `Option<CreateCompletionRequestPrompt>` and `CreateModerationRequest.input` a
  `CreateModerationRequestInput`; `logprobs`, `cache_write_tokens` and `access_programs` are
  optional; item enums have an `Unknown` variant. See its
  [0.33.0 to 0.34.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.33.0-to-0.34.0.md).

- **`ResponsesBuilder` is the real Responses API.** It used to build a chat completion and send
  it to `/chat/completions`. Now `client.responses()` returns a `ResponsesBuilder` for
  `POST /responses`, `send_responses` returns a `ResponsesResponse`, and `send_responses_stream`
  yields `ResponsesEvent`s (see [Responses workflows](responses_workflows.md)). Code that wanted
  a chat completion moves to `ChatCompletionBuilder`, which gained the old builder's
  `json_mode()`, `json_schema(..)`, `reasoning_effort(..)` and a single-tool `add_tool(..)`
  (`tool(..)` on the chat builder appends a tool *message*):

  ```rust,ignore
  // Before: a chat completion, despite the name
  let builder = client.responses().system("Be brief").user("Hi").max_completion_tokens(100);
  let response = client.send_responses(builder).await?;
  println!("{}", response.content().unwrap_or_default());
  ```

  ```rust,no_run
  # async fn run(client: openai_ergonomic::Client) -> openai_ergonomic::Result<()> {
  // After, staying on Chat Completions
  let builder = client.chat().system("Be brief").user("Hi").max_completion_tokens(100);
  let response = client.send_chat(builder).await?;
  println!("{}", response.content().unwrap_or_default());

  // After, on the Responses API
  let builder = client.responses().instructions("Be brief").user("Hi").max_output_tokens(100);
  let response = client.send_responses(builder).await?;
  println!("{}", response.output_text());
  # Ok(())
  # }
  ```

  | Before | After (chat) | After (Responses API) |
  | --- | --- | --- |
  | `client.responses()`, `ResponsesBuilder::new(m)` | `client.chat()`, `ChatCompletionBuilder::new(m)` | unchanged names, new builder |
  | `responses_simple(m, text)` | `user_message(m, text)` | `responses_simple(m, text)` |
  | `responses_system_user(m, s, u)` | `system_user(m, s, u)` | `responses_system_user(m, instructions, u)` |
  | `responses_tool_function(..)` | `tool_function(..)` | `responses_function_tool(..)` or `.function_tool(..)` |
  | `responses_tool_web_search()` | `tool_web_search()` | a built-in `web_search` tool |
  | `.tool(t)` | `.add_tool(t)` | `.tool(t)` (a generated `Tool`) |
  | `.max_completion_tokens(n)` | unchanged | `.max_output_tokens(n)` |
  | `send_responses(b)` → `ChatCompletionResponseWrapper` | `send_chat(b)` | `send_responses(b)` → `ResponsesResponse` |
  | `send_responses_stream(b)` → `BoxedChatStream` | `send_chat_stream(b)` | `send_responses_stream(b)` → `BoxedResponsesStream` |
  | `execute_responses(CreateChatCompletionRequest)` | `execute_chat(..)` | `execute_responses(CreateResponse)` |
  | `response.content()` | unchanged | `response.output_text()` |
  | `response.tool_calls()` | unchanged | `response.function_calls()` |

  The `responses` operation name of interceptors and Langfuse now marks real Responses calls.
  `ResponsesBuilder::build()` returns the generated `CreateResponse`, and
  `test_utils::helpers::{minimal_responses_request, complex_responses_request}` return one.

- **Completions prompts and moderation inputs take the API's unions.**
  `CompletionsBuilder::prompt(..)` takes `impl Into<CreateCompletionRequestPrompt>` (a `&str` or
  `String` still works) and the new `prompts(..)` sends several prompts.
  `ModerationBuilder::new(..)`, `moderate_text(..)`, `moderate_text_with_model(..)` and
  `client.moderations().builder(..)` / `check(..)` take `impl Into<ModerationInput>`: a text, a
  `Vec<String>`, or multimodal `CreateModerationRequestInputOneOfInner` items (the new
  `ModerationInput::Items`; `ModerationInput` is `#[non_exhaustive]`). Several texts used to be
  joined with newlines into one input; they are now sent as an array, one result per text.

- **`Error` and `ToolError` are `#[non_exhaustive]`**, so are `StreamOutcome` and
  `StreamFailureKind`: a `match` on them needs a wildcard arm. Prefer the classification
  methods (`is_rate_limit()`, `is_retryable()`, `is_client_error()`, ...).

  ```rust
  use openai_ergonomic::Error;

  fn describe(error: &Error) -> String {
      match error {
          Error::Api { status, .. } => format!("API error {status}"),
          Error::Http(error) => format!("transport: {error}"),
          // New variants may come in minor releases.
          other => other.to_string(),
      }
  }
  # assert_eq!(describe(&Error::api(429, "slow down")), "API error 429");
  ```

- **A cut-off chat stream is an error.** A chat stream completes at `data: [DONE]`, or when the
  body ends after a chunk with a `finish_reason`. A body that ends before either used to end the
  stream as if it had completed; it now yields `Error::StreamConnection` as the last item, and
  interceptors see `StreamOutcome::StreamFailed` (kind `Connection`) instead of `Completed`.
  `ChatCompletionStream::from_events`, which cannot see `[DONE]`, requires a `finish_reason`.

- **`InterceptedStream` is generic over its items.** `InterceptedStream<T, C = ChatCompletionChunk>`
  takes any stream of `C: InterceptedItem` (`ChatCompletionChunk` or `ResponsesEvent`); code that
  names `InterceptedStream<T>` and passes a `ChatCompletionStream` keeps compiling.

- **Vector store search filters are sent.** `VectorStoreSearchBuilder::filter(key, value)` was
  ignored by `client.vector_stores().search(..)`; it is now sent as an `eq` filter (several are
  combined with `and`), so searches with filters return fewer results than before.

## Upgrading to 0.21

Version 0.21 moves to `openai-client-base` 0.33 (generated with timvw-rust 0.29) and
[`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime) 0.4. The builders do not change.

- **Runtime 0.4.** Types named from `timvw_rust_runtime` (`Config::builder().http_client(..)`,
  `Client::http_client()`, `Error::Http`, `Upload`, ...) are 0.4's: depend on
  `timvw-rust-runtime = "0.4"` if you name them.
- **Per-call options win over the provider.** When you call the generated operations with
  `.options(&RequestOptions)`, a header or query parameter the options set is sent instead of the
  provider's (once), where the provider's `ProviderLayer` used to replace it: an `api-version` set
  for one call now reaches Azure. The new `RequestOptions::base_url(..)` moves one call to another
  base URL: under the provider's base URL it is still mapped and authenticated, anywhere else it
  gets nothing from the provider and no `AuthProvider` credentials. See
  [per-call options](configuration.md#per-call-options).
- **`openai-client-base` 0.33.** Code that uses its types directly moves to 0.33: the multipart
  unions with a file variant hold `Upload`s (build them with `file(..)`/`files(..)`, or pass a
  `PathBuf`), `CreateContainerFileBody.file` is an `Option<Upload>`, `apis::Error::Io` is gone (a
  file error is an `apis::Error::Http` whose `is_io()` is true, as the crate's own `Error::Http`
  already was), and the generated calls declare their base URL. See its
  [0.32.0 to 0.33.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.32.0-to-0.33.0.md).
- **WebAssembly: interceptor state.** `Client<T>` and its sub-clients bound the interceptor state
  `T` by `Default + MaybeSend + MaybeSync` instead of `Default + Send + Sync`: unchanged natively;
  on WebAssembly the state may now hold an `Rc` (see [interceptors](interceptors.md#state-across-hooks)).
- **Documentation.** The README is a short landing page; its sections moved to the pages under
  [`docs/`](.).

## Upgrading to 0.20

Version 0.20 moves to [`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime) 0.3 and
`openai-client-base` 0.32 (generated on runtime 0.3, so the generated operations send through
the same HTTP client and layers as the rest of the crate).

- **Runtime 0.3.** `Config::builder().http_client(..)`, `Client::http_client()`, `Error::Http`,
  `ChatCompletionStream::new` and `ChatCompletionStream::from_events` use
  `timvw_rust_runtime` 0.3 types (`Client`, `Error`, `Response`, `sse::Events`): depend on
  `timvw-rust-runtime = "0.3"` if you name them.
- **File parameters are `Upload`s.** `TranscriptionBuilder::new`, `TranslationBuilder::new`,
  `ImageEditBuilder::new`/`mask`, `ImageVariationBuilder::new` and the `audio()`/`images()`
  shortcuts take `impl Into<timvw_rust_runtime::Upload>`: a `PathBuf` or `&Path` (streamed from
  disk) still works, and so do bytes (`Vec<u8>`, `Bytes`; name them with `Upload::file_name`)
  and streams. A `&str` is no longer taken as a path: write `Path::new("audio.wav")`. Their
  `file()`/`image()` accessors and the request structs' fields are `Upload`s.
- **No temporary files.** `FilesClient::upload`/`create` send the `FileUploadBuilder`'s content
  as it is (bytes from memory, a path streamed from disk). `FileUploadBuilder::from_path` (and
  `upload_file_from_path`, `FilesClient::upload_from_path`) no longer read the file and no
  longer return a `Result`; `FileUploadBuilder::from_upload` and `FilesClient::upload_from`
  take any `Upload`. `content()`, `content_size()` return `Option`s (`None` for a path).
- **A missing file** to upload is an `Error::Http` whose `is_io()` is `true`, raised before
  anything is sent (and reported to the interceptors' `on_error`); `Error::is_io()` and
  `Error::io_error()` (with its `kind()`, `NotFound`) read it, for `Error::File` as well.
- **`upload_large_file(upload, purpose)`.** The file name defaults to the upload's
  (`Upload::name`, a path's last component) and the media type to its own
  (`Upload::media_type`, guessed from a path's extension); set them with
  `.filename(..)`/`.mime_type(..)`.
- **WebAssembly.** The crate builds for `wasm32-unknown-unknown` (see
  [WebAssembly](webassembly.md)). The bounds of `FunctionTool`, `Interceptor` and `AuthProvider`
  (and of their futures) are `MaybeSend`/`MaybeSync` (re-exported from the runtime): exactly
  `Send`/`Sync` natively, nothing on WebAssembly. Natively nothing changes for implementers.
- **Native async traits.** `FunctionTool` and `Interceptor` no longer use `async-trait` (see
  [below](#native-async-traits-020)).
- **Secret credentials.** The credentials of the generated `Configuration`
  (`bearer_access_token`, `api_key`, ...) are `SecretString`s: code that drives
  `openai-client-base` itself sets them with `Some("sk-...".into())` and reads them with
  `ExposeSecret::expose_secret`. Their `Debug` output is redacted.
- **Quota errors are not retried.** A `429` whose error code is `insufficient_quota` is
  returned at once as `Error::Api` instead of being retried: waiting does not refill a quota.
- **Providers.** `Config::builder().provider(Provider::...)` sets the base URL, credentials,
  headers, query parameters and paths in one go (see [Providers](configuration.md#providers)). The existing
  settings keep working: `api_key`/`api_base` describe an `OpenAI` provider, and
  `azure_deployment` (or an `*.openai.azure.com` base) an Azure one.
- **Credentials only go to the provider.** The `Authorization` (or Azure `api-key`) header and
  the organization and project headers are added by the provider's layer, and only to requests
  under the provider's base URL. The provider's layer now also wraps a client set with
  `http_client(..)`, so a custom client no longer needs `AzureAuthMiddleware`, which is
  deprecated in favour of `Provider::azure` (adding it anyway does no harm).
- **Streaming errors.** Chat and Responses streams are sent as raw requests: an error status
  before the first event is an `Error::Api` with the `type` and `code` of an `OpenAI` error body.
- **New:** `AuthProvider`/`BearerTokenAuth` ([Authentication](configuration.md#authentication)), webhook
  verification ([Webhooks](webhooks.md)), `Client::upload_large_file`, `ImagesClient::save`,
  `AudioClient::speech_to_file`, `FilesClient::upload_file` and `FilesClient::download_to`
  ([Large Files and Media](files-and-media.md)).

### Native async traits (0.20)

`FunctionTool` and `Interceptor` are now native async traits; the crate no longer
depends on `async-trait`. Drop `#[async_trait]` from your `impl FunctionTool` /
`impl Interceptor` blocks and keep the methods as plain `async fn` (their futures
must be `Send`). `with_interceptor`, `add_interceptor` and `InterceptorChain::add`
take the interceptor by value; `Box<I>` and `Arc<I>` still work because they
implement `Interceptor` too. `dyn Interceptor` is no longer a valid type: collect
heterogeneous interceptors in an `InterceptorChain` instead.

```rust,ignore
// Before
#[async_trait::async_trait]
impl Interceptor for Logger {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> { Ok(()) }
}
let client = Client::from_env()?.with_interceptor(Box::new(Logger)).build();

// After
impl Interceptor for Logger {
    async fn before_request(&self, ctx: &mut BeforeRequestContext<'_>) -> Result<()> { Ok(()) }
}
let client = Client::from_env()?.with_interceptor(Logger).build();
```

## Upgrading to 0.19

Version 0.19 moves the HTTP stack from `reqwest-middleware` to
[`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime), a [tower](https://docs.rs/tower)-based
client (with `openai-client-base` 0.30; see its
[0.29.0 to 0.30.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.29.0-to-0.30.0.md)). `Config::builder().http_client(..)` and
`Client::http_client()` now take and return a `timvw_rust_runtime::Client` instead of a
`reqwest_middleware::ClientWithMiddleware`: build one with
`timvw_rust_runtime::Client::from_reqwest(reqwest_client)` (with default retries) or
`Client::from_service(ReqwestService::new(reqwest_client))` and add tower layers with `.layer(..)`
(see [Retries, Timeouts and Custom HTTP Clients](configuration.md#retries-timeouts-and-custom-http-clients)).
`Error::Http` wraps a `timvw_rust_runtime::Error` (with `is_timeout()`, `is_connect()`, ...), and
`Error::HttpMiddleware` is gone. `AzureAuthMiddleware` is now a tower layer, and
`ChatCompletionStream::new` takes a `timvw_rust_runtime::Response`. The default client now honours
`Config::max_retries` (default 3): set it to `0` to turn retries off.

## Upgrading to 0.18 and earlier

Version 0.18 uses `openai-client-base` 0.29 and requires Rust 1.88 or newer.
Re-exported client types such as `Detail`, `UploadPurpose` and `CreateImageRequest`
now come from `openai-client-base` 0.29, so code that names them directly from
`openai-client-base` must move to 0.29 as well. The re-exported image `OutputFormat`
is now `openai-client-base`'s `ImagesApiOutputFormat`, which adds `Gif`, `Svg` and
`Unknown`, so an exhaustive `match` on it must handle those. `ImageEditBuilder::output_format`
still takes a string; a value that is not a known format is rejected with
`Error::InvalidRequest` before the request is sent. See the
[0.28.0 to 0.29.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.28.0-to-0.29.0.md)
for the other generated changes. 0.28 changed two generated items:
the `bucket_width` parameter of `organization_decisionsusage` is now
`Option<models::DecisionsUsageBucketWidth>` instead of `Option<&str>`, and the values
of `ChatkitWorkflow.state_variables` are now `Option<..>`, so a `null` value decodes.
See the
[0.27.0 to 0.28.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.27.0-to-0.28.0.md).
Code that calls the generated
`openai-client-base` functions itself passes request bodies by reference (`&request`)
from 0.27 on, and every generated model gains a `validate()` method
(`openai_client_base::models::validation::Validate`) that checks the spec's limits;
nothing calls it for you. Error responses changed too: `apis::ResponseContent` gained
a `headers` field, so code that builds one by hand adds
`headers: reqwest::header::HeaderMap::new()`, and the variant in `entity` now follows
the response status (exact code, then range, then the default response) instead of
the first variant whose type accepts the body. See the
[0.26.0 to 0.27.0 migration notes](https://github.com/genai-rs/openai-client-base/blob/main/migration/api-diff-0.26.0-to-0.27.0.md)
for the full list. Since 0.26, optional nullable fields
such as `temperature`, `top_p`, `user` and `metadata` are `Option<Option<T>>`, and
`CreateMessageRequest.content` and `PredictionContent.content` are typed unions
instead of `serde_json::Value`. The builders still take plain values.
Generated request and response types follow their matching client release, so
check the [changelog](../CHANGELOG.md) when you upgrade across a minor version. Image
generation and editing require an explicit `.model(...)`; choose the model before
building or sending those requests.

## Upgrading to 0.7

Version 0.7 moved to `openai-client-base` 0.16 and required Rust 1.88 or newer. Most builder
method signatures stayed the same. The generated request structs that `Builder::build()` returns
use nested options (`Option<Option<T>>`) for optional fields whose wire format distinguishes an
omitted JSON property from an explicit `null`: changed fields in chat and legacy completion
requests, image generation requests, run and fine-tuning requests, and vector store updates.
Set a value with two `Some`s, send `null` with `Some(None)`, and leave a field at `None` to omit
it. Read values with `.flatten()`.

```rust
use openai_client_base::models::CreateCompletionRequest;

let mut request = CreateCompletionRequest::new("gpt-4".into(), Some("Hello".into()));
request.stream = Some(Some(false)); // sends `"stream": false`
request.stop = Some(None);          // sends `"stop": null`
request.n = None;                   // omits `n`

let max_tokens: Option<i64> = request.max_tokens.flatten();
let json = serde_json::to_value(request).unwrap();
assert_eq!(json["stream"], false);
assert!(json["stop"].is_null());
assert!(json.get("n").is_none());
```

The ergonomic builders omit optional fields you leave unset and keep the values you set,
including empty strings, `false` and zero: `.stream(false)` sends `"stream": false`. Some
builders have deliberate defaults: `RunBuilder` sends `"stream": false` unless streaming is
enabled. Streamed chunks keep the generated distinction between a missing and a `null`
`finish_reason`; the `finish_reason()` helper returns `None` for both.
