# Langfuse Integration Fix Needed

## Problem
Traces are not appearing in Langfuse Cloud when using the `LangfuseInterceptor`.

## Root Cause Analysis

### Current Implementation Issues
1. **Architecture Mismatch**: Our interceptor pattern splits request/response handling into separate `before_request()` and `after_response()` methods
2. **Context Loss**: OpenTelemetry `Context::current()` is task-local and doesn't automatically persist spans across these separate calls
3. **Span Lifecycle**: We attempt to create spans in `before_request` but can't easily keep them alive until `after_response`
4. **No Proper Linking**: Spans aren't properly linked as parent-child in a trace hierarchy

### Reference Implementation
The working implementation at https://github.com/timvw/reqwest-openai-tracing uses:
- **Middleware pattern**: Single method with full request/response lifecycle
- **Single span**: Creates ONE span, executes request within it, adds response attributes, then ends it
- **Proper context**: Uses `Context::current_with_span()` and `.with_context()` for propagation
- **Langfuse attributes**: Includes `langfuse.observation.type`, `langfuse.observation.input`, `langfuse.observation.output`

### Key Differences

| Aspect | Our Implementation | Reference (Working) |
|--------|-------------------|---------------------|
| Pattern | Interceptor (before/after) | Middleware (single method) |
| Spans per operation | 2 separate spans | 1 span for full operation |
| Context management | Not propagated | Properly propagated with `.with_context()` |
| Span lifecycle | Ended immediately | Lives for entire operation |
| Langfuse attrs | Partial | Complete (`langfuse.observation.*`) |

## Solutions

### Option A: Refactor to Middleware (Recommended)
Create a `reqwest-middleware` based implementation similar to the reference:
- Pros: Clean, works with OpenTelemetry patterns, proven approach
- Cons: Requires refactoring client architecture

### Option B: Use Task-Local Storage
Store span context in thread-local/task-local storage:
```rust
tokio::task_local! {
    static LANGFUSE_SPAN_CONTEXT: Arc<Mutex<Option<SpanContext>>>;
}
```
- Pros: Works with current interceptor pattern
- Cons: More complex, potential for leaks if not cleaned up properly

### Option C: Span Storage HashMap
Use a concurrent HashMap keyed by request ID to store/retrieve spans:
- Pros: Simple to implement
- Cons: Requires generating/passing request IDs, manual cleanup needed

## Recommended Next Steps

1. **Short-term**: Document the limitation clearly (DONE - see docs/langfuse-integration.md)
2. **Medium-term**: Implement Option B (task-local storage) as it works with current architecture
3. **Long-term**: Consider Option A (middleware) for cleaner architecture alignment

## Code References

- Current implementation: `src/langfuse_interceptor.rs`
- Reference implementation: https://github.com/timvw/reqwest-openai-tracing/blob/main/src/middleware.rs
- OpenTelemetry context docs: https://docs.rs/opentelemetry/latest/opentelemetry/struct.Context.html

## Test Case

To verify the fix works, traces should appear in Langfuse Cloud with:
- Proper parent-child hierarchy (root trace → generation observation)
- Complete attributes: input, output, model, tokens, duration
- Session/user IDs when configured
- Linked observations showing the full request/response flow

```bash
# Test command
cargo run --example langfuse
# Then check Langfuse Cloud dashboard for traces
```
