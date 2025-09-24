# Migrating from `openai-builders`

The legacy `openai-builders` crate lives inside the `openai-experiment` monorepo and mirrors the generated REST client closely. `openai-ergonomic` refines those builders into a standalone crate with stronger ergonomics, richer helper methods, and a focus on the modern Responses + Assistants surfaces. This guide highlights the main differences and shows how to adapt existing code.

## Key Differences

| Surface | `openai-builders` | `openai-ergonomic` | Notes |
|--------|-------------------|--------------------|-------|
| Chat / Responses | `openai_builders::chat::ChatCompletion::builder()` | `openai_ergonomic::ResponsesBuilder::new(model)` or `client.responses()` | Unified Responses-first API, helper methods for system/user/tool messages. |
| Embeddings | Manual `CreateEmbeddingRequest::builder()` with `bon` optional fields | `EmbeddingsBuilder::new(model)` with dedicated `input_text`, `input_tokens`, `dimensions`, `encoding_format` helpers | Automatic validation (e.g. positive dimensions) and type-safe input variants. |
| Threads & Attachments | Compose JSON maps manually | `ThreadRequestBuilder`, `ThreadMessageBuilder`, `MessageAttachment::for_file_search` | Handles attachment tooling, metadata double-option encoding, and request assembly. |
| Uploads | Populate `CreateUploadRequest` struct by hand | `UploadBuilder::new(filename, purpose, bytes, mime_type)` | Built-in validation for byte counts and expiration window. |
| Convenience Accessors | Access raw structs | Response wrappers (`ChatCompletionResponseWrapper`) expose `content()`, `tool_calls()`, `usage()` etc. | Less boilerplate when reading responses. |

## Example: Embeddings Request

**Before**

```rust
use openai_builders::embeddings::CreateEmbeddingRequestBuilder;

let request = CreateEmbeddingRequestBuilder::default()
    .model("text-embedding-3-small")
    .input("The quick brown fox")
    .dimensions(Some(256))
    .build()?;
```

**After**

```rust
use openai_ergonomic::EmbeddingsBuilder;

let request = EmbeddingsBuilder::new("text-embedding-3-small")
    .input_text("The quick brown fox")
    .dimensions(256)
    .build()?;
```

All optional fields are now regular methods—no need to reach into generated structs.

## Example: Thread Attachments

**Before**

```rust
use serde_json::json;

let request = json!({
    "messages": [{
        "role": "user",
        "content": "Summarise the docs",
        "attachments": [{
            "file_id": "file-123",
            "tools": [
                {"type": "file_search"},
                {"type": "code_interpreter"}
            ]
        }]
    }]
});
```

**After**

```rust
use openai_ergonomic::builders::threads::{
    AttachmentTool, MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};

let thread_request = ThreadRequestBuilder::new()
    .message_builder(
        ThreadMessageBuilder::user("Summarise the docs")
            .attachment(
                MessageAttachment::for_file_search("file-123")
                    .with_tool(AttachmentTool::CodeInterpreter),
            ),
    )?
    .build()?;
```

The new builders ensure the JSON schema is correct (tool enums, double-option metadata) and remain extendable as OpenAI evolves the API.

## Migration Tips

1. **Start with responses-first helpers.** Swap `ChatCompletion::builder()` / `ResponsesRequest::builder()` for `ResponsesBuilder` and the `client.send_responses` helper. This collapses multiple request structs into a single fluent surface.
2. **Leverage strong input types.** `EmbeddingInput` and `AttachmentTool` reduce the chance of malformed payloads and provide compiler guidance when new variants are introduced.
3. **Keep planning artefacts updated.** The crate emphasises `PLAN.md` / `TODO.md`; align your migration milestones there so parallel agents stay in sync.
4. **Mix and match during migration.** The underlying `openai-client-base` types remain accessible. You can wrap existing `Create*Request` values inside the new builders (or vice versa) while you migrate module by module.

With these patterns in place you can iteratively port codebases off `openai-builders`, gaining documentation, validation, and higher-level helpers along the way.
