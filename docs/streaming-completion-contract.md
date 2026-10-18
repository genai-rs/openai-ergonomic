# Streaming interceptor completion (OE05)

Status: **accepted contract; OE06 implementation in progress**.
Accepted by the independent supervisor decision recorded against
`genai-rs-openai-ergonomic#dcbr`, reviewing head
`5a1169f09650d43ade25ad8ce7034d909dfe6450` and report
`work/reviews/oe05.json` in the supervisor workspace.
The accepted implementation belongs to OE06 (`genai-rs-openai-ergonomic#v5hz`).
OE04 (`genai-rs-openai-ergonomic#cqg2`) has since landed; preserve its parser
behavior and do not edit parser code in OE06.

## Evidence and scope

Reviewed baseline: `8717a622373d565d3d9511ba7150dca245b4fcfa` (17 October 2026).

- [`src/streaming.rs`](../src/streaming.rs): `InterceptedStream::poll_next`
  spawns an independent task per chunk and EOF, discards callback results, has
  no terminal latch, and has no drop notification. An error item is forwarded
  without invoking an error hook.
- [`src/interceptor.rs`](../src/interceptor.rs): each chain call visits
  interceptors in registration order, but separate spawned calls can overlap.
  The existing result-returning chain methods stop at their first error.
  `on_stream_end` is documented as successful completion only.
- [`docs/langfuse-integration.md`](langfuse-integration.md): the shutdown advice
  relies on a small delay for detached callback tasks. No finite delay guarantees
  completion of an arbitrarily slow callback.
- [`src/client.rs`](../src/client.rs): streaming methods erase the concrete
  stream into `BoxedChatStream`. Adding a method only to `InterceptedStream`
  would leave normal client callers unable to use it.

This contract concerns the chat streaming wrapper, including client entry points
that delegate to it. It does not redefine SSE parsing, repair Responses wire
compatibility, add telemetry providers, or promise remote delivery of traces.

## Additive API

Keep existing `BoxedChatStream`, `send_chat_stream`, `send_responses_stream`, and
the public `InterceptedStream::new` signature source-compatible. Add opt-in
entry points returning `(BoxedChatStream, StreamCompletion)`, named
`send_chat_stream_with_completion` and `send_responses_stream_with_completion`,
plus `InterceptedStream::new_with_completion` for direct construction.
These names and types are additive to the existing streaming API.

`StreamCompletion` is a cloneable per-stream handle. Its async
`wait(&self) -> Result<Arc<StreamCompletionReport>, CompletionUnavailable>` waits
for the terminal condition **and** callback drain; it does not consume or poll
the stream. The immutable report is cached, so concurrent/repeated waits return
the same report without invoking hooks again. Dropping a waiter or timing out
a wait does not cancel the worker or lose its result. This is per-stream
completion, not a client-global barrier that can race with new requests.

The report contains:

- `outcome`: `Completed`, `StreamFailed(StreamFailure)`, or `Cancelled`;
- number of accepted chunks (including usage-only chunks), stream duration and
  last observed usage, with missing usage remaining unknown;
- callback attempts, successes, and failures, with each failure identifying the
  hook, registration index, optional chunk index, and failure kind (returned
  error or caught panic). Bound diagnostic text and retained failure details;
  keep an exact total failure count and an explicit truncated-details count.

`StreamFailure` is an owned classification/summary, not a borrowed `Error` or a
second ownership claim on the original error. The original error is yielded to
the stream caller. Do not copy raw prompts/chunks into diagnostics. A callback
failure is separate from stream outcome: a completed response can have failed
telemetry. `CompletionUnavailable` means the dispatcher ended without a sealed
report (for example an unexpected task failure); it must never masquerade as
successful drain.

## Ordering and terminal states

For each stream, `before_request` finishes before any chunk callback. Preserve
its existing fail-fast behavior: setup errors return from stream creation, so
there is no completion handle for a stream that was never created.

After successful creation, callbacks execute serially: chunk 0 across registered
interceptors in registration order, then chunk 1, and so on, then one terminal
event across the same interceptors. No chunk callback overlaps another callback
for that stream. Different streams may still call a shared interceptor
concurrently. Chunks may reach the caller before their callbacks finish;
`wait` supplies the barrier. Slow callbacks exert bounded backpressure rather
than creating an unbounded number of detached tasks.

An accepted chunk is a successfully decoded item admitted to the callback queue
before it is yielded to the caller. Do not count an unread chunk or an error
item. Retain queue order and finish every admitted callback on early drop.

| Trigger | Latched outcome | Stream behavior | Terminal callback |
| --- | --- | --- | --- |
| Underlying parser returns EOF | `Completed` | Return `None`; further polls return `None` | After all accepted chunks |
| First stream error item | `StreamFailed` | Yield original error once; release transport; later polls return `None` | After all accepted chunks |
| Drop before either terminal trigger | `Cancelled` | Release transport; read no more items | After all accepted chunks |
| Drop after a terminal trigger | Unchanged | No further work admitted | No duplicate event |

Use one atomic/locked terminal transition. The first observed trigger wins; a
drop cannot overwrite a recorded stream failure. A finish-reason chunk alone
does not terminate the stream, since later usage chunks may follow. `Completed`
means the parser's normal EOF, not independent validation that an upstream
protocol completion marker was present. Parser-level termination belongs to
OE04 (`genai-rs-openai-ergonomic#cqg2`); do not silently expand that definition
here.

## Terminal notifications and callback failures

Add a default `Interceptor::on_stream_terminal` hook taking a new context with
the existing end summary and the owned terminal outcome. Its default forwards
`Completed` to the existing `on_stream_end`; failure/cancellation default to
no-op. This avoids adding mandatory fields to externally constructed
`StreamEndContext` values and keeps existing interceptor implementations
compiling. Overrides must explicitly forward success if they also need their
old `on_stream_end` implementation. Document that rule in OE06.

Do not call `on_stream_end` on failure or cancellation. The new terminal hook is
the streaming failure/cancellation notification; do not also synthesize
`on_error` from a lossy reconstructed error. Existing pre-stream and
non-streaming error-hook behavior remains outside this change. Built-in
telemetry integration must implement the terminal hook so cancellation and
failure finish its spans rather than leaving them open.

Update the `Interceptor::on_error` documentation: after a stream is
successfully created, streaming errors are reported through
`on_stream_terminal(StreamFailed)` and are not automatically sent to
`on_error`. Keep its documented behavior for other error paths. This avoids
claiming that `on_error` receives errors from every streaming stage.

For this managed streaming dispatcher, attempt each registered interceptor even
if an earlier one returns an error. Record the failure and continue with later
interceptors and later events, including the terminal event. Catch unwinding
panics at each callback boundary and record them as failures; process aborts
are outside the guarantee. Do not recursively invoke `on_error` for a callback
failure. Do not change existing public chain methods' fail-fast behavior;
provide internal streaming dispatch support instead.

The report is sealed only after every accepted chunk callback and every
terminal callback has returned or failed with a recorded result. Therefore a
resolved report, even one containing failures, means no managed callback for
that stream remains running. Child tasks spawned privately by an interceptor
are excluded: an interceptor must await its own work if it wants that work
covered by the barrier.

## Ownership, cancellation, and shutdown

Use one owned dispatcher per intercepted stream, a bounded chunk queue, a
separate terminal signal, and shared completion state. Reserve queue capacity
before polling another inner item, so there is no unbounded pending-payload
buffer. A one-slot queue is a reasonable initial policy; capacity is an
internal tuning choice.

Dispatcher startup needs an active Tokio runtime. Document this requirement on
the new direct `InterceptedStream::new_with_completion` constructor and validate
it without introducing an undocumented panic during public construction. The
additive direct constructor should report the missing runtime explicitly; the
async client entry points already run inside their caller's runtime. Preserve
and document the existing constructor's runtime/polling requirements. OE06
must test construction outside a runtime and drop before the first poll, so
cancellation of an unstarted producer remains well-defined.

`Drop` only signals cancellation and releases the transport; it must not block,
spawn a new task, or depend on queue capacity to record the terminal event.
The existing worker drains admitted chunks, then terminal hooks, then seals
the report. The completion handle keeps the report available but must not keep
the chunk producer alive. Dropping all handles does not abort pending
callbacks. An empty interceptor chain still produces a correct terminal report
with zero callbacks, without requiring a callback worker.

The existing client methods may use the same lifecycle internally and discard
the handle. This strengthens ordering and fixes duplicate EOF notifications,
but adds observable backpressure and failure/cancellation semantics. Review
these behavioral changes explicitly even though the return types are stable.

Accepted caller flow (pseudocode; exact names follow the accepted API):

```text
(stream, completion) = await client.send_chat_stream_with_completion(request)
consume stream, recording any error or deciding to stop early
drop stream                         // also required after break/early return
report = await timeout(deadline, completion.wait())
inspect report.outcome and report.callback_failures
await exporter flush/shutdown       // use the exporter's supported API
exit runtime
```

Awaiting `wait` while holding an unconsumed stream waits for a terminal trigger;
it must not implicitly cancel or consume that stream. A caller that stops early
must drop it first. Once EOF/error is observed, dropping is harmless and cannot
change the terminal outcome. A wait timeout means **incomplete/unknown drain**,
not success: retain the handle and retry or report an incomplete shutdown.
There is no internal timing sleep. Callbacks that never resolve can prevent
completion forever; callers choose their shutdown deadline. Runtime shutdown
or process exit before drain provides no delivery guarantee. Exporter flush
is a separate operation after callback drain, because callbacks can enqueue
new spans until they finish.

## Deterministic regression tests

[`tests/streaming_completion.rs`](../tests/streaming_completion.rs)
uses the actual public `ChatCompletionStream` and `InterceptedStream`, a local
mock HTTP response, and a mock interceptor delayed by semaphore gates. The
response is buffered to remove transport scheduling from the callback proof.
Additional tests keep a malformed SSE body's transport pending to prove
terminal parser results release it immediately, and use an application error
whose `Display` panics to verify the callback dispatcher still drains.
Every case has a five-second watchdog; no sleep or elapsed-time threshold is
used to infer completion. Channel closure proves that no hook remains alive
in the drop/error cases.

Run from the repository root:

```sh
rtk proxy env CARGO_BUILD_JOBS=2 cargo test --test streaming_completion
```

The accepted OE05 prototype characterized the baseline defects before
implementation. Its immutable source is available at
[`tests/streaming_completion_prototype.rs` in merged PR #120](https://github.com/genai-rs/openai-ergonomic/blob/15ff29decba354b9f2a44c00577adf28a748770f/tests/streaming_completion_prototype.rs).
OE06 replaces those assertions with the desired contract while keeping these
observations here as diagnostic history:

| Test case | Historical baseline evidence | OE06 assertion |
| --- | --- | --- |
| EOF with two closed gates | End(2), then deliberately released Chunk(1), Chunk(0) | Wait stays pending; Chunk(0), Chunk(1), Terminal; count 2 |
| Repeated EOF | Two End(0) callbacks | One terminal hook; cached repeat/concurrent waits |
| Drop after one of two chunks | Chunk(0), then channel closes without terminal hook | One accepted chunk, then `Cancelled`; drain completes |
| Malformed stream item then EOF | Parsing error, then success End(0); no error hook | `StreamFailed`, no success end; original error yielded once |
| Callback returns an error | Caller sees successful chunk and EOF only | Report records hook failure; later callbacks still attempted |

## OE06 boundary and acceptance checklist

The focused OE06 change adds the shared lifecycle/dispatcher, additive client
and terminal-hook APIs, built-in terminal handling, deterministic tests, and
replaces the shutdown sleep in the affected streaming example/docs.

OE06 must additionally test: a delayed terminal hook; two interceptors with
the first returning an error or panicking; zero interceptors; empty streams;
drop before first poll; drop while a chunk callback is blocked; timeout and
retry of `wait`; concurrent/repeated waits; concurrent streams; a full queue
at drop; and unexpected worker termination producing `CompletionUnavailable`.
Assert exact attempts/order/outcomes and that no callback runs after report
resolution. Tests must use mocks and bounded watchdogs, never an external
telemetry service or a sleep as proof of completion.

The supervisor accepted the additive `(BoxedChatStream, StreamCompletion)`
entry points, serial bounded ordering/backpressure including existing entry
points, the default success adapter from `on_stream_terminal` to
`on_stream_end`, continued dispatch after returned callback errors or panics,
and nonblocking first-terminal-wins cancellation followed by admitted-callback
drain. `wait` does not poll or cancel the producer, and exporter flush follows
the completion barrier. Preserve public context construction and existing
public chain fail-fast behavior.

OE06 must also cover the runtime/constructor and `on_error` documentation
requirements above, replace baseline defect assertions with desired contract
assertions while preserving their diagnostic evidence, and implement no parser
change (OE04 owns that work). Run callback failure/panic order, full-queue drop,
zero-interceptor, repeated/concurrent waits, timeout/retry, delayed terminal,
and unavailable-worker tests before implementation acceptance. If preserving
the existing constructor behavior needs broader API changes, stop for review;
do not silently add a panic path. If implementation needs a breaking public
context change, a second telemetry stack, or an MSRV increase, split it for
review instead of expanding OE06. No additional implementation split is
currently justified.
