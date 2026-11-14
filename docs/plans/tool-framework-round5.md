# Tool framework, round 5: gap analysis against Codex

Status: slice 1 is implemented (step 1 of the order below, with every breaking change of the
round): the Responses API client (`src/responses_api/`, `client.responses()`,
`client.responses_api()`), the chat-backed `ResponsesBuilder` folded into
`ChatCompletionBuilder`, `#[non_exhaustive]` on `Error`, `ToolError`, `StreamOutcome` and
`StreamFailureKind`, and the chat stream EOF rule of item 12. The Responses stream already
skips malformed events and fails a stream cut off before its terminal event. The rest is
planned: later slices only add API.
Baseline: branch `refactor/no-async-trait` (native async `FunctionTool`/`Interceptor`).
Reference: OpenAI Codex `codex-rs` at `69231e119b`. Codex paths are relative to `codex-rs/`.

## Where we stand

Two facts shape every item below.

1. **We have no real Responses API path.** `ResponsesBuilder`
   (`src/builders/responses.rs:24`) holds chat messages and `ChatCompletionTool`s and
   builds a `CreateChatCompletionRequest` (`src/builders/responses.rs:370-411`).
   `Client::execute_responses` posts it to `/chat/completions`
   (`src/client/mod.rs:728-734`: "The Responses API uses the same underlying endpoint as
   chat"), and the streaming variant reuses the chat SSE decoder
   (`src/client/mod.rs:763-783`, `src/streaming.rs:965-1003`). The dependency already has
   everything needed for the real API: `openai-client-base` 0.30 exposes
   `responses_api::{create_response, create_response_stream}` and `CreateResponse` with
   `store`, `include`, `prompt_cache_key`, `parallel_tool_calls`, `previous_response_id`,
   `max_tool_calls` and `truncation`. It also has typed `FunctionToolParam`,
   `CustomToolParam` (grammar format), `NamespaceToolParam` and
   `FunctionCallOutputItemParam`, plus stream events such as `ResponseOutputItemDoneEvent`,
   `ResponseIncompleteEvent` and `ResponseFailedEvent`.
2. **We have no loop.** `ToolRegistry` (`src/tool_framework.rs:216`) only registers tools,
   lists their definitions and runs one call at a time (`execute`: `:293`,
   `execute_call`: `:311`). By design the application owns the loop
   (`docs/unified_tool_framework_design.md`). Codex's guarantees (every call answered,
   ordered outputs, history repair, retries) all live in its turn loop
   (`core/src/session/turn.rs:545`, `:1938`).

The proposal therefore keeps `ToolRegistry` as the low-level primitive and adds two layers:

- a typed Responses client (`client.responses_api()`, new module `src/responses_api/`);
- an opt-in **`ToolLoop`**: a runner that owns a `Conversation` (history), a registry and a
  `LoopConfig`, and drives request → stream → dispatch → outputs → re-request.

Applications that want manual control keep using `execute_call`.

Breaking-change legend: **B** = breaking for existing users; **A** = additive;
**B-later** = additive now, but replaces something we would deprecate.

---

## 1. RespondToModel vs Fatal; parse errors go back to the model; every call_id answered

**Today**
- `ToolError` has 9 variants, all local to the application (`src/tool_framework.rs:84-150`).
  Argument decode failures become `ToolError::Arguments` (`:181-195`), and handler errors
  become `Execution` (`:196-202`).
- `execute_call` returns `Err(ToolError::Call{..})` and leaves it to the caller to decide
  whether the model sees anything (`:304-345`). The docs say "Errors remain local unless
  the application chooses to send them to a model" (`:72-74`).
- `FunctionTool::execute` returns `crate::Result`, so a tool cannot say "tell the model"
  vs "abort".
- Nothing guarantees an output per `call_id`. A chat request with a missing tool message
  is rejected by the API with a 400.

**Codex**
- `enum FunctionCallError { RespondToModel(String), Fatal(String) }`
  (`tools/src/function_call_error.rs:5-10`).
- Serde failures become `RespondToModel("failed to parse function arguments: {err}")`
  (`core/src/tools/handlers/mod.rs:90-97`). An unknown tool becomes
  `"unsupported call: {name}"` (`core/src/tools/registry.rs:1176-1181`).
- `Fatal` aborts the turn. Every other error becomes a `FunctionCallOutput` with the error
  text and `success: false` (`core/src/tools/parallel.rs:131-144, 460-485`).
- Cancelled or aborted calls get `"aborted before dispatch"` / `"aborted by user after
  Ns"` (`parallel.rs:487-509`). History normalisation inserts `"aborted"` as a backstop
  (item 4).

**Proposal**
```rust
/// What the loop does with a failed call.
#[non_exhaustive]
pub enum ToolFailure {
    /// Sent to the model as the call's output (`success=false` for logging).
    RespondToModel(String),
    /// Ends the loop with `Error::Tool`; outputs already gathered are kept in history.
    Fatal(crate::Error),
}

impl ToolError {
    /// Default classification: Arguments/InvalidArguments/Unknown/InvalidCall
    /// -> RespondToModel (Serde message, tool name); Execution -> RespondToModel(err
    /// Display) unless the source is `Error::Fatal`; Output -> Fatal.
    pub fn classify(&self) -> ToolFailure;
}

impl crate::Error {
    /// Opt-in escape hatch for handlers: `Err(Error::fatal(e))` ends the loop.
    pub fn fatal(e: impl std::error::Error + Send + Sync + 'static) -> Self;
}

impl ToolRegistry {
    /// Like `execute_call`, but never fails for RespondToModel: returns the
    /// output to send (error text included) and whether it succeeded.
    pub async fn answer_call(&self, call: &ToolCall) -> Result<CallAnswer, ToolError /*Fatal*/>;
}
pub struct CallAnswer { pub call_id: String, pub output: ToolOutputBody, pub success: bool }
```
`ToolLoop` uses `answer_call` and builds every call's answer before the next request. In
addition, item 4's repair inserts `"aborted"` for any call still unanswered (for example
after cancellation or a `Fatal`).

**Breaking:** **A** for `classify`, `answer_call` and `ToolFailure`. A new `Error::Fatal`
variant is **B**: `Error` (`src/errors.rs:13`) is not `#[non_exhaustive]`. Add
`#[non_exhaustive]` in the same breaking release, so later variants (items 12) become
additive. `execute_call` is unchanged.

## 2. Dispatch on `response.output_item.done` while streaming

**Today**
- There is no Responses stream. The chat stream yields `ChatCompletionChunk`s
  (`src/streaming.rs:216-320`) with tool-call deltas (`tool_calls()`: `:273`).
- Assembling the deltas into calls is left to the user. Nothing runs tools before the
  stream ends.

**Codex**
- The SSE parser maps `response.output_item.done` to `ResponseEvent::OutputItemDone`
  (`codex-api/src/sse/responses.rs:411-418`).
- The turn loop calls `handle_output_item_done` (`core/src/session/turn.rs:3210, 3333`).
  That records the call in history and spawns the handler at once
  (`core/src/stream_events_utils.rs:351-405`, `parallel.rs:278`
  `AbortOnDropHandle::new(tokio::spawn(..))`).
- The future goes into a `FuturesOrdered` (`turn.rs:3045, 3346`), which is drained after
  `response.completed` (`turn.rs:2858-2884, 3807-3818`).

**Proposal**
- `ResponsesStream` (new, `src/responses_api/stream.rs`) yields a typed
  `ResponsesEvent` enum:
  `OutputItemAdded`, `OutputItemDone(OutputItem)`, `TextDelta`,
  `FunctionCallArgumentsDelta`, `Completed(Response)`, `Incomplete{..}`,
  `Failed{..}`, `RateLimits(..)` and `Other(serde_json::Value)`.
- `ToolLoop` matches `OutputItemDone(OutputItem::FunctionCall | CustomToolCall)` and
  hands the call to the dispatcher of item 3. It does not wait for `Completed`.
- Text deltas are forwarded to an optional `on_event` callback / output stream, so UIs
  still stream text.

```rust
let mut run = tool_loop.run_streamed(input).await?;          // impl Stream<Item = LoopEvent>
while let Some(ev) = run.next().await { /* LoopEvent::Model(ResponsesEvent) | ToolStarted | ToolFinished | TurnDone */ }
```

**Breaking:** **A** (new API). Chat-completions streaming stays as it is; doing early
dispatch there would mean assembling deltas ourselves, which we will not do in this round.

## 3. Per-tool `supports_parallel`, RwLock gate, outputs in call order

**Today**
- There is no concurrency support. `execute_call` documents that callers choose ordering
  and concurrency (`src/tool_framework.rs:300-305`).
- Requests leave `parallel_tool_calls: None` (`src/builders/responses.rs:388`).

**Codex**
- `ToolExecutor::supports_parallel_tool_calls()` defaults to `false`
  (`tools/src/tool_executor.rs:122-124`).
- Each runtime has one `Arc<tokio::sync::RwLock<()>>`. Parallel-safe tools take `read()`;
  the rest take `write()` (`parallel.rs:54, 93, 292-307, 340`).
- `FuturesOrdered` keeps the outputs in call order (`turn.rs:3045, 2858-2884`). MCP sets
  the flag per server (`codex-mcp/src/tools.rs:31-33`).

**Proposal**
```rust
pub trait FunctionTool: Send + Sync {
    // ...existing...
    /// May this tool run concurrently with other parallel-safe tools? Default false.
    fn supports_parallel(&self) -> bool { false }
}
```
- The registry snapshots the flag at `register`. `ToolLoop` keeps one
  `Arc<tokio::sync::RwLock<()>>` per loop: the tools that may run in parallel take `read`
  and the others take `write`.
- The futures are spawned (via a `Spawn` hook, so we stay runtime-agnostic and ready for
  WASM; the default is `tokio::spawn`) and collected in a `FuturesOrdered`. History gets
  each call's output in the order the calls were issued.
- `LoopConfig::parallel_tool_calls: bool` (default `true`) is sent as
  `parallel_tool_calls`.

**Breaking:** **A**. A defaulted trait method is source-compatible.

## 4. History repair before each request

**Today**
- There is no history type. Users append messages to `ChatCompletionBuilder`
  (`assistant_with_tool_calls`: `src/builders/chat.rs:191`, `tool`: `:221`).
- Nothing repairs or validates pairing.

**Codex**
- `ContextManager::for_prompt` → `normalize_history` (`core/src/context_manager/history.rs:720-750, 1188-1206`) runs `ensure_call_outputs_present`, `remove_orphan_outputs` and the image/audio strip.
- Missing outputs get `"aborted"` directly after the call. The synthetic IDs are UUIDv5,
  so they stay stable and the prompt cache keeps working
  (`core/src/context_manager/normalize.rs:18-19, 21-169`).
- Orphan outputs are dropped (`normalize.rs:171-241`). Removing an item removes its
  partner too (`normalize.rs:243+`).

**Proposal**
```rust
pub struct Conversation { items: Vec<InputItem> /* Responses input items */ }
impl Conversation {
    pub fn push(&mut self, item: impl Into<InputItem>);
    pub fn record_response(&mut self, output: &[OutputItem]);   // calls, reasoning, messages
    pub fn record_output(&mut self, answer: CallAnswer);
    /// Idempotent: insert "aborted" after unanswered calls, drop orphan outputs,
    /// strip images if `!caps.images`. Returns what changed for logging.
    pub fn normalize(&mut self, caps: ModelCaps) -> NormalizeReport;
    pub fn remove(&mut self, index: usize);                       // removes partner too
}
```
- `ToolLoop` calls `normalize` before every request. That includes retries (item 12),
  because a mid-stream retry re-sends history that may contain calls that were dispatched
  but never answered.
- Synthetic outputs need no ids (`function_call_output` takes `call_id` only), so the
  output is deterministic and cache-friendly by construction.
- `debug_assert!` on repairs, like Codex's `error_or_panic`.

**Breaking:** **A**.

## 5. Tool-output truncation (head+tail, header, per-tool budget)

**Today**
- There is none. `ToolOutput.content` is the full JSON string
  (`src/tool_framework.rs:157-163`).
- The only truncation in the crate caps callback diagnostics
  (`src/streaming.rs:926-933`).

**Codex**
- `TruncationPolicy::{Bytes(n), Tokens(n)}` (`protocol/src/protocol.rs:3826-3874`), with
  4 bytes per token (`utils/string/src/truncate.rs:6`).
- The text is split half head and half tail (`truncate.rs:149-152`). The middle is
  replaced with `"…N tokens truncated…"` / `"…N chars truncated…"` (`truncate.rs:28-54`),
  cutting on UTF-8 boundaries.
- Optional header: `"Warning: truncated output (original token count: N)\nTotal output
  lines: L\n\n"` (`utils/output-truncation/src/lib.rs:26-51`).
- Content items: text and audio count against the budget; images are kept (`lib.rs:138-173`).
- Budget sources:
  - the model default is `bytes(10_000)` (`models-manager/src/model_info.rs:127`);
  - config overrides it (`:32-44`);
  - each tool can override it (`tools/src/tool_output.rs:32-36`);
  - a 1.2× allowance for serialisation (`lib.rs:22-24`).

**Proposal**
```rust
#[derive(Clone, Copy)] pub enum OutputBudget { Bytes(usize), Tokens(usize) /* 4 B/token */, Unlimited }
pub struct TruncationConfig { pub default: OutputBudget, pub header: bool }   // default Bytes(10_000), header true
pub trait FunctionTool { fn output_budget(&self) -> Option<OutputBudget> { None } }
pub fn truncate_middle(text: &str, budget: OutputBudget) -> Cow<'_, str>;      // pub, reusable
```
- The truncation is applied when `ToolLoop` records the output, never inside `execute`.
- Header format: `"[truncated: original {n} bytes/~{t} tokens, {lines} lines]\n"`.
- Marker: `"…{removed} chars truncated…"`.
- Images are never truncated.

**Breaking:** **A**. Opt-out via `Unlimited`. The default does change what the model sees
in `ToolLoop`, which is a new API, so nothing existing changes.

## 6. Outputs as string or content items (text / image / audio)

**Today**
- `ToolOutput { call_id, content: String }` with a JSON-encoded string
  (`src/tool_framework.rs:157-163`).
- `FunctionTool::Output: Serialize` is always turned into JSON text (`:196-205`).

**Codex**
- `FunctionCallOutputBody` is `#[serde(untagged)]`: either `Text(String)` or
  `ContentItems(Vec<FunctionCallOutputContentItem>)`. The items are `InputText`,
  `InputImage{image_url|file_id, detail}`, `InputAudio` and `EncryptedContent`
  (`protocol/src/models.rs:2394-2414, 2500-2596`).
- `success` is internal and never serialised.

**Proposal**
```rust
#[non_exhaustive]
pub enum ToolOutputBody { Text(String), Items(Vec<OutputContent>) }
#[non_exhaustive]
pub enum OutputContent { Text(String), Image { url_or_file: ImageRef, detail: Option<Detail> }, Audio { data: String, format: String } }

/// Conversion from a tool's `Output` into what the model sees.
pub trait IntoToolOutput { fn into_tool_output(self) -> Result<ToolOutputBody>; }
// blanket: T: Serialize -> Text(json)    (keeps today's behaviour)
// impls:  ToolOutputBody, Vec<OutputContent>, OutputContent, Image helpers
```
- Changing the bound `type Output: Serialize` to `type Output: IntoToolOutput` would be
  breaking. Instead, keep `Serialize` and add an `async fn execute_rich` default method?
  That is messy.
- **Preferred:** a sealed adapter. `register_rich<T: RichTool>` for tools that return
  `ToolOutputBody` directly, while `FunctionTool` stays JSON-only.
- `ToolOutput.content: String` becomes `body: ToolOutputBody` in 1.0 (**B-later**).
  Until then, `ToolOutput::body()` is an accessor.
- Chat Completions only accepts text and image parts in tool messages. The chat bridge
  rejects audio with `ToolFailure::RespondToModel`.

**Breaking:** **A** now (new trait/registration). **B** if we later fold it into
`ToolOutput`.

## 7. Custom/freeform tools with Lark grammar, and namespaces

**Today**
- Only function tools. `execute_call` rejects custom calls with "only function tool calls
  are supported" (`src/tool_framework.rs:322-327`).
- Names are restricted to `[A-Za-z0-9_-]{1,64}` (`:246-255`).
- There are no namespaces.

**Codex**
- `FreeformTool{name, description, defer_loading, format: {type:"grammar", syntax:"lark",
  definition}}` serialises as `type:"custom"` (`tools/src/responses_api.rs:16-30`). It is
  used for apply_patch (`core/src/tools/handlers/apply_patch_spec.rs:5-27`).
- `ToolSpec::Namespace{name, description, tools}` uses `type:"namespace"`
  (`tool_spec.rs:22-26`, `responses_api.rs:58-119`).
- Call items carry `namespace`. The router normalises names with
  `ToolName::with_default_namespace()` (`protocol/src/tool_name.rs:7-61`,
  `core/src/tools/router.rs:278-347`).

**Proposal**
```rust
pub trait CustomTool: Send + Sync {             // freeform text input, no Serde
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn format(&self) -> CustomFormat { CustomFormat::Text }   // | Grammar { syntax: Lark|Regex, definition }
    fn execute(&self, input: String) -> impl Future<Output = Result<ToolOutputBody>> + Send;
    fn supports_parallel(&self) -> bool { false }
}
impl ToolRegistry {
    pub fn register_custom<T: CustomTool + 'static>(&mut self, tool: T) -> Result<(), ToolError>;
    pub fn namespace(&mut self, name: &str, description: &str) -> NamespaceRegistrar<'_>; // .register(..)
}
```
- The registry key becomes `ToolKey { namespace: Option<String>, name }`.
- `responses_tools()` emits `Function`, `Custom` and `Namespace` params. `tool_definitions()`
  (chat) keeps emitting functions only and errors if there are custom tools: Chat
  Completions has `custom` tools too, but namespaces are Responses-only.
- `ErasedTool` gets an input enum (`Arguments(&str) | Input(&str)`).

**Breaking:** **A**. The internal `ErasedTool` change is private.

## 8. MCP name sanitizer and schema sanitizer/compaction

**Today**
- Registration rejects invalid names and requires an object root schema
  (`src/tool_framework.rs:240-265`). There is no sanitising, schema rewriting or MCP
  support.

**Codex**
- `sanitize_responses_api_tool_name` maps `[^A-Za-z0-9_]` to `_` and turns an empty
  result into `"_"` (`codex-mcp/src/mcp/mod.rs:635-650`).
- Names are `server__tool`, at most 128 characters, with a 12-hex SHA1 suffix on
  collision; the raw name is kept for the call (`codex-mcp/src/tools.rs:24, 134-267`).
- `sanitize_json_schema`:
  - a `true`/`false` schema becomes `{"type":"string"}`;
  - `const` becomes `enum`;
  - the type is inferred when missing;
  - unreachable `$defs` are pruned.

  See `tools/src/json_schema.rs:65-175`.
- Compaction over 5 000 bytes, applied in order:
  1. strip descriptions,
  2. drop definitions,
  3. collapse below depth 3,
  4. prune compositions.

  See `tools/src/json_schema/compaction.rs:15-38`.
- MCP tools larger than 8 000 bytes fall back to a permissive schema
  (`responses_api.rs:14, 136-151`).

**Proposal**
- `pub mod schema` with `sanitize(&mut Value) -> SanitizeReport` and
  `compact(&mut Value, max_bytes) -> CompactReport`. Both are pure, public and
  well-tested.
- `pub fn sanitize_tool_name(raw: &str) -> String` and
  `pub struct ToolNamer { max_len: usize /* 64 for chat, 128 for responses */ }` with
  `fn unique(&mut self, namespace, name) -> String` (hash suffix on collision).
- `ToolRegistry::register_dynamic(name, description, schema, handler)` for tools only
  known at runtime (MCP, plugins). It applies the namer and the sanitiser, and keeps a
  `display_name -> raw_name` map for dispatch.
- No MCP client in this crate. An `openai-ergonomic-mcp` companion can sit on top of
  `register_dynamic`.

**Breaking:** **A**. The sanitiser is only applied to `register_dynamic`. Typed tools
keep the strict validation.

## 9. Stateless defaults (store:false, encrypted reasoning, replay, stable cache key)

**Today**
- `store: None`, `prompt_cache_key: None`, `parallel_tool_calls: None`
  (`src/builders/responses.rs:388-401`, `src/builders/chat.rs:553`).
- There is no reasoning replay because there is no Responses history.

**Codex**
- `store:false`, `stream:true`, `tool_choice:"auto"`,
  `include:["reasoning.encrypted_content"]` and a session-derived `prompt_cache_key`
  (`core/src/client.rs:1258-1310`).
- `enum PromptCacheKey { SessionDerived, Custom, Omit }`
  (`core/src/thread_manager.rs:335-343`, `client.rs:712-733`).
- Reasoning items stay in history and are re-sent. `encrypted_content` is stripped for
  non-OpenAI providers (`client.rs:1231-1250`).

**Proposal**
```rust
pub struct LoopConfig {
    pub store: bool,                         // default false
    pub include_encrypted_reasoning: bool,   // default true (only sent when store == false)
    pub prompt_cache_key: PromptCacheKey,    // default PerLoop (random UUID per ToolLoop, stable across its requests)
    pub parallel_tool_calls: bool,           // default true
    pub max_iterations: Option<NonZeroU32>,  // item 11
    pub truncation: TruncationConfig,        // item 5
    pub retry: StreamRetryConfig,            // item 12
}
pub enum PromptCacheKey { PerLoop, Custom(String), Omit }
```
- `Conversation::record_response` keeps `reasoning` items verbatim, `encrypted_content`
  included, and they are replayed on the next request.
- For Azure, or a base URL that is not OpenAI, `include` is dropped and reasoning items
  are stripped. That avoids 400s; Azure detection already exists in `azure_middleware`.
- `ResponsesBuilder` (the real one) stays neutral: the defaults apply only in `ToolLoop`.

**Breaking:** **A** for `ToolLoop`. Renaming today's chat-backed `ResponsesBuilder` is
**B** (see order, step 1).

## 10. Exposure enum, deferred tools, tool_search

**Today**
- Every registered tool is always exposed (`tool_definitions`:
  `src/tool_framework.rs:284-290`).

**Codex**
- `enum ToolExposure { Direct, Deferred, DeferredModelOnly, DirectModelOnly,
  CodeModeOnly, Hidden }` (`tools/src/tool_executor.rs:50-99`).
- Deferred tools are sent with `defer_loading: true` (`responses_api.rs:153-173`).
- A client-executed `tool_search` tool (`type:"tool_search"`, `execution:"client"`) runs
  BM25 over deferred tool metadata. It returns `ToolSearchOutput{tools}` with a default
  limit of 8 (`core/src/tools/handlers/tool_search.rs:14-17, 237-337`,
  `tool_search_spec.rs:97-141`).

**Proposal**
```rust
#[non_exhaustive] pub enum Exposure { Direct, Deferred, Hidden }   // Hidden = dispatchable, never advertised
pub trait FunctionTool { fn exposure(&self) -> Exposure { Exposure::Direct } }
impl ToolRegistry {
    pub fn set_exposure(&mut self, key: &ToolKey, e: Exposure);
    /// Client-side search; pluggable ranking (default: simple token overlap; BM25 behind a feature).
    pub fn enable_tool_search(&mut self, cfg: ToolSearchConfig);
}
```
- Only `Direct` tools are sent. When deferred tools exist and search is enabled, the
  `tool_search` tool is added.
- Search results are loaded for the next request (`LoadedTools` state on `ToolLoop`).
- Server-side `defer_loading` is used only when the model supports it (feature flag in
  `LoopConfig`).
- `CodeModeOnly`/`ModelOnly` are not needed.

**Breaking:** **A**.

## 11. Configurable max iterations and cancellation

**Today**
- There is no loop, so no limit. Examples hand-roll loops
  (`examples/tool_calling_multiturn.rs`).
- Cancellation means dropping the stream. `InterceptedStream` reports
  `StreamOutcome::Cancelled` (`src/streaming.rs:105-113`).

**Codex**
- No max-iteration constant. The loop ends when no follow-up is needed
  (`turn.rs:545, 693, 808`).
- A `CancellationToken` flows through. Stream reads give `TurnAborted`
  (`turn.rs:3103-3147`).
- In-flight tools are drained and get "aborted by user after Ns"
  (`turn.rs:3807-3829`, `parallel.rs:374-435`).

**Proposal**
- `LoopConfig::max_iterations: Option<NonZeroU32>`, default `Some(16)`. When exceeded,
  the loop returns `LoopOutcome::MaxIterations { conversation }` (not an error), so
  callers can continue.
- `ToolLoop::run(input, cancel: CancellationSignal)`. `CancellationSignal` is a thin
  wrapper over `tokio_util::sync::CancellationToken`, or our own
  `Arc<Notify + AtomicBool>` to avoid the new dependency.
- On cancel:
  1. the stream is dropped;
  2. in-flight tool futures get `select!` against the signal;
  3. unfinished calls are answered with `"aborted by user after {s:.1}s"`;
  4. the `Conversation` is returned in `LoopOutcome::Cancelled`, already normalised, so
     it can be resumed.
- `FunctionTool` gets `fn finishes_on_cancellation(&self) -> bool { false }` only if
  needed. Skip it initially.

**Breaking:** **A**.

## 12. Responses stream robustness

**Today**
- Our chat SSE decoder ends on `[DONE]` *or EOF* without distinguishing them
  (`src/streaming.rs:948-1003`). A truncated stream therefore counts as
  `StreamOutcome::Completed` (`:105-113`).
- A parse error ends the stream (`:983-988`). Codex skips the bad event; we fail.
- Retries happen only at the HTTP layer, before the body
  (`RetryLayer::new(RetryPolicy::new(max_retries))`, `src/client/mod.rs:252-254`).
- `Error::is_retryable` = rate limit or 5xx (`src/errors.rs:165-167`).
- Rate-limit headers are not exposed.

**Codex**
- Stream errors:
  - EOF without `response.completed` → `"stream closed before response.completed"`
    (`codex-api/src/sse/responses.rs:643-649`);
  - idle timeout 300 s (`:650-655`);
  - malformed events are skipped (`:660-672`).
- `response.incomplete` (`responses.rs:476-546`):
  - `content_filter` → ContentFilter;
  - `interrupted` → completed with `end_turn=false`;
  - other reasons → `ResponseIncomplete{partial_output, reason}`.
- `response.failed` is mapped by code: context_length_exceeded, insufficient_quota,
  rate_limit_exceeded (with retry-after parsed from headers or message),
  server_is_overloaded, … (`codex-api/src/sse/responses_error.rs:26-110`,
  `codex-api/src/error.rs:92+`).
- `enum RetryPolicy { Never, ServerDirected, Backoff, FallBackToHttp }`
  (`protocol/src/error.rs:85-88, 470-535`). Backoff is 200 ms·2^(n-1) ±10 %
  (`async-utils/src/backoff.rs:7-17`). There are 5 stream retries by default
  (`model-provider-info/src/lib.rs:74-80`).
- On a retry the whole request is re-issued from history (`core/src/responses_retry.rs:58-181`).
- `x-codex-*` rate-limit headers become `ResponseEvent::RateLimits` before the body
  (`responses.rs:47, 81-83`, `codex-api/src/rate_limits.rs:28-230`).

**Proposal**
```rust
pub enum ResponsesEvent { /* item 2 */ RateLimits(RateLimitSnapshot), Incomplete(IncompleteReason, Response), Failed(ResponseFailure) , .. }
pub struct RateLimitSnapshot {               // from x-ratelimit-{limit,remaining,reset}-{requests,tokens}
    pub requests: Option<Window>, pub tokens: Option<Window>,
}
#[non_exhaustive] pub enum RetryClass { Never, ServerDirected { after: Duration }, Backoff }
impl crate::Error { pub fn retry_class(&self) -> RetryClass; }   // supersedes is_retryable (keep it, delegating)
pub struct StreamRetryConfig { pub max_retries: u32 /* 5 */, pub idle_timeout: Duration /* 300s */, pub base: Duration /* 200ms */ }
```
- Decoder:
  - EOF before `response.completed` / `.incomplete` / `.failed` →
    `Error::StreamConnection{"closed before response.completed"}`;
  - a malformed *event* is skipped with a `tracing::debug!` (configurable strict mode);
  - idle timeout via `tokio::time::timeout` per read.
- `incomplete`:
  - `max_output_tokens` → `LoopOutcome::Incomplete` (with partial output, never retried);
  - `content_filter` → `Error::ContentFilter`;
  - `interrupted` → continue the loop.
- `failed` → typed `Error` per code:
  - `context_length_exceeded` → `Error::ContextLengthExceeded`, Never;
  - `insufficient_quota` → Never (aligned with the other session's retry rule);
  - `rate_limit_exceeded` → ServerDirected if a delay is known, else Backoff;
  - `server_is_overloaded` → ServerDirected;
  - anything else → Backoff.
- `ToolLoop` re-issues the whole request from the normalised `Conversation` after a
  mid-stream failure, up to `max_retries`. Before re-issuing it waits as `retry_class`
  says.
- Tools already dispatched in the failed attempt are **not** re-run. Their outputs are
  kept, and the partial output items are recorded only if they came with `output_item.done`.
  This mirrors Codex, which records calls at dispatch time.
- Chat streams get the EOF-vs-`[DONE]` distinction too: `StreamOutcome::Completed`
  requires `[DONE]` or a `finish_reason`. That changes the outcome reported to
  interceptors and is **B** (behaviour).

**Breaking:**
- **A** for the Responses path.
- **B** (behavioural) for the chat EOF rule and for skipping malformed events in the
  Responses decoder. Ship it in the same minor release, with a CHANGELOG note.

---

## Proposed implementation order

Each step is one PR and leaves the crate releasable.

1. **Real Responses client** (prerequisite for 2, 7, 9, 10, 12; **B**).
   - Rename today's chat-backed `ResponsesBuilder`/`send_responses*` to the chat API, or
     deprecate them.
   - Add `client.responses_api()` built on `openai_client_base::apis::responses_api`, with
     typed `InputItem`/`OutputItem` wrappers and a builder that is neutral (no defaults
     forced).
2. **`ResponsesStream` + robustness** (item 12 decoder part, item 2 event enum, rate-limit
   headers). This is testable with mockito SSE fixtures. It includes the `retry_class`
   taxonomy, which must coordinate with the `insufficient_quota` rule from the other
   branch.
3. **`Conversation` + `normalize`** (item 4) and **stateless defaults** (item 9). Both are
   pure data and heavily unit-testable.
4. **Error classification** (item 1): `ToolFailure`, `classify`, `answer_call`,
   `Error::fatal`. It also benefits manual users of the chat API.
5. **`ToolLoop` MVP**:
   - sequential dispatch, then `OutputItemDone` early dispatch (item 2);
   - max iterations and cancellation (item 11);
   - whole-request re-issue on stream failure (item 12, retry part).
6. **Parallelism** (item 3): `supports_parallel`, RwLock gate, `FuturesOrdered`, and the
   `Spawn` hook.
7. **Truncation** (item 5) and **rich outputs** (item 6). Truncation must understand
   content items, so they go together.
8. **Custom/freeform tools + namespaces** (item 7). This changes the registry key.
9. **Dynamic tools, name and schema sanitiser/compaction** (item 8), then **exposure,
   deferred loading and `tool_search`** (item 10), which builds on dynamic tools.

Steps 1 and 12's chat EOF rule are the only breaking changes. Bundle them into the same
minor release as the native-async-trait change, so users migrate once.
