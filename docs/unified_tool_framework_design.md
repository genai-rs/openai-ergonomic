# FunctionTool API design decisions

Status: **implemented** (released in 0.6.0). This page records why the tool framework looks the way it does; the [guide](tool_framework.md) describes how to use it.

PR #72 supersedes the three-trait proposal in #70. The existing `Tool` model aliases remain unchanged. One async `FunctionTool` trait covers typed and JSON tools through associated types; instances can hold application state. The registry erases types internally, captures validated metadata once and returns definitions in stable name order.

The old `tool!` macro repeated input/output types, supported only unit structs and defaulted silently to an empty schema. Ordinary Rust implementations are longer but familiar, support state without another API and produce ordinary type errors. Standard JSON Schema expressed with `json!` covers nested objects, enums and optional fields without a bespoke schema DSL. A schema generator can be used by applications without becoming a mandatory dependency here.

Registration is mutable and fallible. Duplicate tools never replace existing handlers. Basic metadata checks stop invalid names, blank descriptions and non-object schemas before the existing builder helper can silently convert a bad schema into an empty map. This is deliberately not full JSON Schema validation or strict-mode support. Serde decodes arguments, and handlers enforce business constraints.

Dispatch is per call. `execute_call` returns `ToolOutput { call_id, content }`, with contextual `ToolError::Call` failures. It accepts the actual client's function call model and rejects custom text calls. `execute` remains available for callers with names and JSON text. JSON encoding follows Serde, including string quoting.

Batch execution and implicit builder mutation were removed: the draft could execute several tools, lose their successful outputs on a later error, and encourage replay of side effects. Its example also placed tool results before the assistant call. The new complete example keeps history and assistant content, appends calls before replies, and chooses an error payload per call. Authorization, retries, timeouts and concurrency belong to applications.

Validation includes public integration tests for actual client definitions and request messages, required/optional fields, malformed arguments, duplicate registration, retained IDs, handler and encoding errors, dynamic JSON, and async state. Both examples run offline; rustdoc compiles the minimal tool. Migration details are in the [guide](tool_framework.md).

## Both APIs, and an answer for every call

The tool-framework round ([plan](plans/tool-framework-round5.md), modelled on OpenAI Codex) keeps `ToolRegistry` as the low-level primitive and `execute_call` as it was, and adds what a tool loop needs without adding the loop itself:

- **An answer for every call.** A chat request whose tool call has no tool message, or a Responses request whose function call has no output, is rejected by the API. `answer_call` therefore answers every call it can: failures the model can act on (bad arguments, an unknown tool, a call of the wrong kind, a handler error) become the call's output with `success: false`, as Codex's `RespondToModel`; only `Fatal` failures (`Error::fatal` from a handler, an output that cannot be encoded) are returned. `ToolError::classify` is the default policy and `answer_call_with` replaces it, because the message of a handler error is sent to the model.
- **One call type for both APIs.** `ToolInvocation` converts from a chat call, a Responses function or custom tool call, or an output item; `CallAnswer` becomes a tool message (`ChatCompletionBuilder::call_answer`) or a `function_call_output` / `custom_tool_call_output` item (`ResponsesBuilder::call_answer`). `tool_definitions` (chat) is unchanged; `responses_tools` adds custom and namespaced tools, which chat cannot express.
- **Rich outputs without a breaking change.** `FunctionTool::Output: Serialize` stays; tools that return images or files implement a separate `RichTool` returning a `ToolOutputBody`. `ToolOutput.content` stays a string (`ToolOutput::body()` gives a body). Outputs are text, images and files, what the Responses API accepts as a call's output; chat tool messages take text only, so an image there is an error rather than silently dropped.
- **Custom tools and namespaces** are Responses API features: `CustomTool` takes text (with an optional Lark or regex grammar) and needs no Serde; the registry key gained an optional namespace.
- **Strict typed tools, adaptive runtime tools.** `register` still rejects invalid names and schemas; `register_dynamic`, for MCP servers and plugins, sanitizes and compacts instead (Codex's name and schema sanitizers), and keeps the raw name.
- **Hints, not policy.** `supports_parallel` and `output_budget` are defaulted trait methods read at registration. The registry runs nothing concurrently and never truncates: the application (or a loop on top) decides, with `truncate_middle` and `TruncationConfig` as building blocks.

## The agent loop

The opt-in [agent loop](agent_loop.md) (`ToolLoop`, Round 5 slice 3) builds on these without changing them:

- **The history is the loop's state.** A `Conversation` of Responses API input items drives both APIs (the chat view groups calls into assistant messages and outputs into tool messages), and `normalize` repairs it before every request, retries included, so cancellation, a fatal tool error or a hand-edited history never produce a request the API rejects.
- **Stateless by default.** `store: false`, the whole history in every request (without item ids), the encrypted reasoning replayed for reasoning models, one prompt cache key per loop. The loop never chains `previous_response_id`, so a retry or a resumed run needs nothing from the server.
- **No spawning.** Calls run on the loop's task in a `FuturesUnordered`, behind a FIFO read/write gate that `supports_parallel` drives, and are recorded in call order; nothing has to be `Send` on WebAssembly and no runtime is assumed.
- **Retries of whole requests** follow `Error::retry_class` and the `retry-after` headers, which the loop reads through calls that keep their response headers (the generated `<operation>_response` functions, and the raw request for streams). Calls already started are not run again.
- **Exposure is registry metadata, loading is loop state.** `Exposure` says whether a tool is advertised; which deferred tools a search loaded belongs to the `ToolLoop`, with a client-side `tool_search` function by default and the hosted `defer_loading` search for models that have it.
