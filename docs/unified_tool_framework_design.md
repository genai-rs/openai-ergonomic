# FunctionTool API design decisions

PR #72 supersedes the three-trait proposal in #70. The existing `Tool` model aliases remain unchanged. One async `FunctionTool` trait covers typed and JSON tools through associated types; instances can hold application state. The registry erases types internally, captures validated metadata once and returns definitions in stable name order.

The old `tool!` macro repeated input/output types, supported only unit structs and defaulted silently to an empty schema. Ordinary Rust implementations are longer but familiar, support state without another API and produce ordinary type errors. Standard JSON Schema expressed with `json!` covers nested objects, enums and optional fields without a bespoke schema DSL. A schema generator can be used by applications without becoming a mandatory dependency here.

Registration is mutable and fallible. Duplicate tools never replace existing handlers. Basic metadata checks stop invalid names, blank descriptions and non-object schemas before the existing builder helper can silently convert a bad schema into an empty map. This is deliberately not full JSON Schema validation or strict-mode support. Serde decodes arguments, and handlers enforce business constraints.

Dispatch is per call. `execute_call` returns `ToolOutput { call_id, content }`, with contextual `ToolError::Call` failures. It accepts the actual client's function call model and rejects custom text calls. `execute` remains available for callers with names and JSON text. JSON encoding follows Serde, including string quoting.

Batch execution and implicit builder mutation were removed: the draft could execute several tools, lose their successful outputs on a later error, and encourage replay of side effects. Its example also placed tool results before the assistant call. The new complete example keeps history and assistant content, appends calls before replies, and chooses an error payload per call. Authorization, retries, timeouts and concurrency belong to applications.

Validation includes public integration tests for actual client definitions and request messages, required/optional fields, malformed arguments, duplicate registration, retained IDs, handler and encoding errors, dynamic JSON, and async state. Both examples run offline; rustdoc compiles the minimal tool. Migration details are in the [guide](tool_framework.md).
