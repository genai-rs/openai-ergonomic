# Multi-turn Tool UX Design

**Status**: Draft  
**Context**: Replaces the ad-hoc loops prototyped in PRs [#70](https://github.com/genai-rs/openai-ergonomic/pull/70) and [#72](https://github.com/genai-rs/openai-ergonomic/pull/72)

## Motivation

`ToolRegistry::process_tool_calls_into_builder` proved that we can wire tool responses back into the `ChatCompletionBuilder`, but the ergonomics are still rough:

- Every integration re-implements the same while-loop: send request → inspect tool calls → execute tools → append messages → repeat.
- The loop is fragile; forgetting to include `tool_definitions`, mishandling `tool_call_id`, or failing to cap retries results in confusing API errors.
- Instrumentation, streaming support, and early exits (e.g. human-in-the-loop) are hard to layer on top because there is no central orchestration point.

We want a first-class UX for multi-turn conversations with tool usage that is easy to adopt, composable, and opinionated about safety.

## Goals

1. **Single entry point** for multi-turn chat that handles the send/execute/append loop.
2. **Typed tools** should plug in through the existing `tool!` macro and `ToolRegistry` without extra code.
3. **Custom hooks** (telemetry, logging, human approval) should be easy to inject.
4. **Safety controls**: max iterations, timeouts, and optional filters for which tools can be auto-run.
5. **Streaming support** should be opt-in but ergonomic.

Non-goals: building a workflow engine, handling Assistants API (we focus on Chat Completions first), or orchestrating external state machines.

## Proposed API Surface

```rust
use openai_ergonomic::multi_turn::{ConversationBuilder, ConversationRunner, LoopControl, StepContext};

let runner = ConversationBuilder::new(client.clone(), model)
    .system("You are a helpful assistant")
    .tools(registry)
    .max_turns(6)
    .auto_execute_tools(true)
    .on_tool_executed(|ctx| {
        tracing::info!(tool = ctx.tool_name, "executed tool");
        LoopControl::Continue
    })
    .on_response(|ctx| {
        if ctx.response.contains("approval required") {
            LoopControl::Stop
        } else {
            LoopControl::Continue
        }
    })
    .build();

let transcript = runner.run("Help me book a meeting").await?;
println!("Final reply: {}", transcript.final_message());
```

### Key Types

- `ConversationBuilder`: configures the loop (model, initial messages, registry, limits, hooks).
- `ConversationRunner`: executes the conversation; owns the `ToolRegistry` and keeps track of pending messages.
- `Transcript`: immutable record returned at the end; exposes helper methods (`final_message`, `tool_invocations`, `iterations`).
- `LoopControl`: enum returned by hooks to continue, stop with a reason, or override the assistant response.
- `StepContext`: snapshot passed into hooks containing the last response, executed tool results, iteration count, and references to the client/registry for advanced scenarios.

### Execution Flow (per iteration)

1. Build a request from accumulated messages + tool definitions.
2. Call `client.create_chat_completion` (streaming or standard depending on configuration).
3. Inspect the response:
   - If no tool calls, run the `on_response` hook; decide to continue or stop.
   - If tool calls exist:
     - For each call, check against allow/deny list, execute via the registry (unless `auto_execute_tools` is false), run the `on_tool_executed` hook.
     - Append assistant tool-call message and tool result messages to the transcript.
4. Repeat until `LoopControl::Stop`, hitting `max_turns`, exceeding timeout, or encountering an error.

Errors (tool failure, validation error, hook cancellation) bubble up as `LoopError` with rich context; the transcript is returned alongside for post-mortem.

## Configuration Matrix

| Option | Default | Notes |
|--------|---------|-------|
| `max_turns` | 4 | protects against infinite recursion |
| `max_tool_calls_per_turn` | 4 | additional safety valve |
| `auto_execute_tools` | `true` | when false, hooks must enqueue tool results manually |
| `tool_allow_list` | `None` | restrict automatic execution |
| `timeout` | `Duration::from_secs(30)` | total wall time |
| `streaming` | `Disabled` | optional streaming callback fed with incremental deltas |

## Extending Telemetry & Observability

Hooks receive a `StepContext` that contains:

```rust
pub struct StepContext<'a> {
    pub iteration: usize,
    pub request: &'a CreateChatCompletionRequest,
    pub response: &'a ChatCompletionResponseWrapper,
    pub tool_results: &'a [ToolExecution],
    pub transcript: &'a Transcript,
    pub client: &'a Client,
}
```

This makes it trivial to emit Langfuse spans, send metrics, or gate progression.

## Compatibility

- Reuses `Tool`, `ToolRegistry`, and `tool!`; no breaking changes for existing users who prefer manual control.
- The new module lives under `src/multi_turn/mod.rs` and re-exports a `multi_turn` namespace from `lib.rs`.
- Examples `examples/tool_framework.rs` and `examples/tool_framework_typed.rs` gain a sibling `examples/tool_loop.rs` showcasing the new UX.

## Open Questions

1. **Streaming state**: should the transcript store intermediate deltas or only the final message per turn?
2. **Parallel tool calls**: OpenAI can request multiple calls at once; initial version will execute sequentially, but we may want an async `join_all` for independent tools.
3. **Azure compatibility**: ensure that loop parameters (like reasoning tokens) round-trip through the existing builder when using Azure deployments.
4. **Assistants API**: future work; architecture should allow us to share the orchestration logic once we wrap Assistants.

## Next Steps

1. Prototype `ConversationRunner` in a draft PR, focusing on the non-streaming path and basic hooks.
2. Add integration tests covering:
   - happy-path multi-turn with one tool
   - failure when a tool returns an error
   - enforcing `max_turns`
3. Update documentation and examples, deprecating the manual loop in `ToolRegistry::process_tool_calls_into_builder` once the runner is stable.
4. Collect feedback from early adopters (PR #70/#72 reviewers) before locking the API.
