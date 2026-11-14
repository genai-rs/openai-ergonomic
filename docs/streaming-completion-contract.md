# Streaming completion contract

What the client guarantees about [interceptor](interceptors.md) callbacks of a streamed chat
completion or Responses request, and how to wait for them before shutting down (for example an
OpenTelemetry exporter). [Streaming](streaming.md) has the everyday usage.

## Entry points

`send_chat_stream_with_completion` and `send_responses_stream_with_completion` return a
`(BoxedChatStream, StreamCompletion)`; `InterceptedStream::new_with_completion` does the same for
a stream you build yourself. `send_chat_stream` and `send_responses_stream` run the same
lifecycle and drop the handle.

`StreamCompletion` is a cloneable handle per stream. `wait()` resolves to an
`Arc<StreamCompletionReport>` once the stream has reached its terminal state **and** every
accepted callback has returned or failed. It does not poll, consume or cancel the stream.
Repeated and concurrent waits return the same report without calling any hook again; dropping a
waiter, or a timeout around it, cancels nothing. `wait()` returns `CompletionUnavailable` when
the dispatcher ended without a report (an unexpected task failure): that is never a successful
drain.

## The report

`StreamCompletionReport` has:

- `outcome`: `StreamOutcome::Completed`, `StreamFailed(StreamFailure)` or `Cancelled`;
- `total_chunks` (accepted chunks, usage-only ones included), `duration`, and the last
  `input_tokens` / `output_tokens` seen (`None` when the stream reported no usage);
- `callback_attempts`, `callback_successes`, `callback_failure_count` (exact), and
  `callback_failures`: each a `StreamCallbackFailure` with its `hook` (`Chunk` or `Terminal`),
  `interceptor_index`, `chunk_index`, `kind` (`ReturnedError` or `Panicked`) and a bounded
  `diagnostic`. At most 64 failures are kept, with diagnostics of at most 512 characters;
  `truncated_failure_details` counts the failures that were not kept.

`StreamFailure` is an owned summary (`kind`: connection, parsing, buffer or other); the original
error is yielded to the stream's consumer. Diagnostics hold no prompts or chunks. A callback
failure is separate from the outcome: a completed stream can have failed telemetry.

## Ordering

For each stream:

- `before_request` finishes before any chunk callback. If it fails, creating the stream fails
  and there is no completion handle.
- Callbacks run one at a time: chunk 0 through every interceptor in registration order, then
  chunk 1, and so on, then one terminal event through every interceptor. Different streams may
  call a shared interceptor concurrently.
- A chunk may reach the consumer before its callbacks have finished; `wait()` is the barrier.
  The callback queue is bounded (one chunk), so slow callbacks slow the stream down instead of
  piling up detached tasks.
- An accepted chunk is a decoded chunk admitted to the callback queue before it is yielded. An
  error item or a chunk never read is not counted.

## Terminal states

| Trigger | Outcome | Stream | Terminal callback |
| --- | --- | --- | --- |
| End of the stream | `Completed` | `None`, and `None` after that | after all accepted chunks |
| First error item | `StreamFailed` | the original error once, then `None`; the connection is released | after all accepted chunks |
| Dropped before either | `Cancelled` | the connection is released, nothing more is read | after all accepted chunks |
| Dropped after either | unchanged | nothing more is admitted | no second event |

The first trigger wins; a drop never overwrites a failure. A chunk with a finish reason does not
end the stream (usage chunks may follow). `Completed` means the parser reached the end of the
event stream, not that the server sent a completion marker.

## Hooks

`Interceptor::on_stream_terminal` is called once per established stream, with the end summary
and the outcome. Its default forwards `Completed` to `on_stream_end` and does nothing for a
failure or a cancellation; an override that still wants `on_stream_end` on success must call it.
After the stream is created, stream failures go to `on_stream_terminal`, not to `on_error`. The
built-in `LangfuseInterceptor` implements `on_stream_terminal`, so failed and cancelled streams
end their spans too.

Every interceptor is called for every event even when an earlier one returns an error or panics:
the failure is recorded in the report (panics that unwind are caught at each callback; an abort
is not), and `on_error` is not called for it. The non-streaming interceptor chain keeps stopping
at the first error.

The report is sealed only after every accepted chunk callback and every terminal callback has
finished, so a resolved report (even with failures) means no managed callback of that stream is
still running. Tasks an interceptor spawns on its own are not covered: await that work inside the
hook if the barrier should include it.

## Tasks, cancellation and shutdown

Each stream has one dispatcher, a bounded chunk queue, a terminal signal and shared completion
state. Natively the dispatcher is a tokio task: `InterceptedStream::new` needs no runtime to be
constructed, but its first poll must be inside a tokio runtime, and dropping it before the first
poll starts nothing; `InterceptedStream::new_with_completion` returns an error instead of
panicking outside a runtime. On WebAssembly the dispatcher runs with
`wasm_bindgen_futures::spawn_local` and needs no runtime (see [WebAssembly](webassembly.md)).

Dropping the stream only signals cancellation and releases the connection: it does not block or
spawn. The dispatcher drains the admitted chunks, runs the terminal callbacks and seals the
report. Dropping every `StreamCompletion` handle does not abort pending callbacks. With no
interceptors, the report is still correct, with zero callbacks.

A shutdown then looks like this:

```text
(stream, completion) = client.send_chat_stream_with_completion(builder)
consume the stream, keeping any error, or stop early
drop(stream)                                 // required after stopping early
report = timeout(deadline, completion.wait())
inspect report.outcome and report.callback_failures
flush / shut down the exporter
```

`wait()` on a stream that is still held and not consumed waits for it to end; it does not
cancel it. A timeout means the drain is incomplete or unknown, not that it succeeded: keep the
handle and wait again, or report the incomplete shutdown. Callbacks that never return keep the
report from resolving, so choose your deadline. Flushing the exporter is a separate step after
the drain, because callbacks can add spans until they finish; a runtime shutdown or process exit
before the drain gives no delivery guarantee. [Langfuse integration](langfuse-integration.md#streaming)
has a complete example.

## Tests

[`tests/streaming_completion.rs`](../tests/streaming_completion.rs) checks this contract against
mock HTTP responses and interceptors held by semaphore gates, with a watchdog on every case and
no sleeps: callback order (usage-only chunks included), a slow terminal callback and repeated
waits, a drop with a full queue, a drop before the first poll, a stream error, callback errors
and panics, bounded failure details, zero interceptors, concurrent streams sharing interceptors,
the client's entry points, and the constructor outside a runtime.

```bash
cargo test --test streaming_completion
```
