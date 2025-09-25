# Migrating from `openai-builders`

If you currently rely on the experimental `openai-builders` crate, you can move to `openai-ergonomic` without rewriting every payload by hand. This guide highlights the mechanical changes and how to take advantage of the extra ergonomics available here.

## 1. Update Dependencies

```toml
[dependencies]
openai-ergonomic = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Remove the `openai-builders` entry (and the direct `bon` dependency if you only used it transitively).

## 2. Replace Module Imports

`openai-ergonomic` re-exports the same families of builders under `openai_ergonomic::builders::*`. Swap import paths according to the table below.

| `openai-builders` path | `openai-ergonomic` replacement |
| --- | --- |
| `openai_builders::chat::ChatCompletionBuilder` | `openai_ergonomic::builders::chat::ChatCompletionBuilder` |
| `openai_builders::responses::ResponsesBuilder` | `openai_ergonomic::builders::responses::ResponsesBuilder` |
| `openai_builders::assistants::RunBuilder` | `openai_ergonomic::builders::assistants::RunBuilder` |
| `openai_builders::images::ImageGenerationBuilder` | `openai_ergonomic::builders::images::ImageGenerationBuilder` |
| `openai_builders::audio::TranscriptionBuilder` | `openai_ergonomic::builders::audio::TranscriptionBuilder` |

Most helper functions (for example `tool_function`, `responses_simple`, or `simple_assistant`) are re-exported at the crate root, so you can often replace `openai_builders::responses::tool_function` with `openai_ergonomic::tool_function`.

## 3. Adopt the Ergonomic Client

Unlike `openai-builders`, this crate ships a `Client` wrapper around `openai-client-base`. Instead of manually instantiating the generated API, configure once and call the convenience methods.

```rust,no_run
use openai_ergonomic::{Client, Config};

let config = Config::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .default_model("gpt-4.1-mini")
    .build();

let client = Client::new(config)?;
let response = client
    .chat()
    .system("You are concise.")
    .user("Summarise the latest release notes.")
    .temperature(0.4);

let result = client.send_chat(response).await?;
```

You can still obtain the raw builder output (`CreateChatCompletionRequest`, etc.) with `.build()` when you need to call the generated API directly.

## 4. Responses API Enhancements

`openai-ergonomic` focuses on the Responses endpoint. Builders now include:

- `.json_mode()` and `.json_schema(name, schema)` helpers.
- Rich tool composition via `tool_function` and `tool_web_search`.
- A `ChatCompletionResponseWrapper` that exposes `primary_text()`, `tool_calls()`, streaming utilities, and JSON extraction helpers.

Porting code that previously inspected `ChatCompletionResponse` manually now becomes:

```rust,no_run
let wrapper = client.send_responses(builder).await?;
for call in wrapper.tool_calls() {
    println!("tool call: {}", call.function().name());
}
```

## 5. Assistants and Threads

Thread and attachment handling gained dedicated builders. Replace ad-hoc payload construction with:

```rust,no_run
use openai_client_base::models::CreateThreadRequest;
use openai_ergonomic::builders::threads::{
    AttachmentTool,
    MessageAttachment,
    ThreadMessageBuilder,
    ThreadRequestBuilder,
};

fn build_thread() -> openai_ergonomic::Result<CreateThreadRequest> {
    ThreadRequestBuilder::new()
        .user_message("Investigate ticket #42")
        .message_builder(
            ThreadMessageBuilder::assistant("Send me the log extract.")
                .attachment(
                    MessageAttachment::for_code_interpreter("file-log")
                        .with_tool(AttachmentTool::FileSearch)
                )
        )?
        .build()
}
```

Run creation relies on the same `RunBuilder` you already used.

## 6. Clean Up Utility Code

Many helper functions (`simple_assistant`, `responses_simple`, `vector_store_with_files`, etc.) live alongside the builders. Search for utility functions you copied from `openai-builders` and replace them with the built-in equivalents to reduce boilerplate.

## 7. Testing & Mocking

The repository includes mock-driven integration tests under `tests/`. Use them as templates for migrating your own tests—the `mockito` patterns stay identical, and the ergonomic builders drop directly into request bodies.

## Summary Checklist

- [ ] Update `Cargo.toml` dependencies.
- [ ] Swap import paths to `openai_ergonomic::builders::*` (use the compatibility table as a starting point).
- [ ] Instantiate the new `Client` instead of calling `openai-client-base` directly.
- [ ] Replace manual JSON with the richer builders (responses, assistants, threads, uploads).
- [ ] Leverage the provided helpers and wrappers to reduce custom glue code.

Following these steps should make the migration largely mechanical while unlocking the newer ergonomics around the Responses and Assistants APIs.
