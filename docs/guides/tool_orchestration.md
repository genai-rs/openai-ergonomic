# Tool Orchestration with Assistants

`openai-ergonomic` layers ergonomic builders on top of the Assistants API so you can assemble assistants, threads, messages, and runs without hand-writing request JSON. This guide shows how the pieces fit together for multi-tool workflows.

## Choosing and Registering Tools

Start with [`AssistantBuilder`](../../src/builders/assistants.rs). The crate provides convenience helpers for the built-in tools, and a flexible `tool_function` helper for custom functions.

```rust,no_run
use openai_ergonomic::builders::assistants::{
    assistant_with_tools,
    tool_code_interpreter,
    tool_file_search,
};

let assistant = assistant_with_tools(
    "gpt-4.1",
    "Code Tutor",
    vec![tool_code_interpreter(), tool_file_search()],
)
.instructions("Help users understand and refactor code.");
```

For bespoke functions, define JSON schemas with `serde_json::json!` and pass them to [`tool_function`](../../src/responses/mod.rs) (re-exported at the crate root). Function tools work for both Responses API and Assistants runs; they share the same schema format.

```rust,no_run
use openai_ergonomic::responses::tool_function;

let summarize_tool = tool_function(
    "summarize_log",
    "Summarise structured log lines into a short paragraph",
    serde_json::json!({
        "type": "object",
        "properties": {
            "entries": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["entries"]
    }),
);
```

Attach custom tools via `AssistantBuilder::add_tool` and mix them with first-party helpers.

## Preparing Threads and Messages

Threads are built with [`ThreadRequestBuilder`](../../src/builders/threads.rs). Seed the conversation with user or assistant messages, and attach files to specific tools if you need code interpreter or file search context.

```rust,no_run
use openai_ergonomic::builders::threads::{
    AttachmentTool,
    MessageAttachment,
    ThreadMessageBuilder,
    ThreadRequestBuilder,
};
use openai_client_base::models::CreateThreadRequest;

fn build_thread() -> openai_ergonomic::Result<CreateThreadRequest> {
    ThreadRequestBuilder::new()
        .user_message("We uploaded benchmark results. Analyse bottlenecks.")
        .message_builder(
            ThreadMessageBuilder::assistant("Great, I will need the CSV to inspect.")
                .attachment(
                    MessageAttachment::for_code_interpreter("file-uploaded-csv")
                        .with_tool(AttachmentTool::FileSearch)
                )
        )?
        .metadata("project", "alpha")
        .build()
}
```

> Tip: `MessageAttachment::for_code_interpreter` and `MessageAttachment::for_file_search` ensure the tool type matches the API contract. Use `with_tool` to share the same file with multiple tools.

## Creating Runs

[`RunBuilder`](../../src/builders/assistants.rs) encapsulates run options such as tool choice, parallel tool calls, truncation strategy, and streaming.

```rust,no_run
use openai_ergonomic::builders::assistants::RunBuilder;

let run = RunBuilder::new("asst_code_tutor")
    .model("gpt-4.1")
    .tool_choice_auto()
    .parallel_tool_calls(true)
    .stream(true)
    .temperature(0.2);
```

The ergonomic client connects the dots:

```rust,no_run
use openai_ergonomic::{Client, Config};

let client = Client::new(config)?;
let assistant = client.assistants().create(assistant).await?;
let thread = client.threads().create(thread).await?;
let run = client
    .assistants()
    .create_run(thread.id.clone(), run)
    .await?;
```

## Handling Tool Calls

When runs trigger tools you can inspect the output via `RunObject::required_action`. The helper `ChatCompletionResponseWrapper::tool_calls()` (for Responses API) and the raw `RunObject` payload (for Assistants API) both expose the schema you provided earlier. Feed the arguments through your own function dispatcher, then append the results as new thread messages using `ThreadMessageBuilder::assistant` or `ThreadMessageBuilder::user` as appropriate.

```rust,no_run
if let Some(action) = run.required_action {
    for call in action.submit_tool_outputs.tool_calls {
        if call.function.name == "summarize_log" {
            let output = summarize_entries(call.function.arguments)?;
            client
                .assistants()
                .create_message(
                    &thread.id,
                    ThreadMessageBuilder::assistant(output),
                )
                .await?;
        }
    }
}
```

## Checklist

1. Create an assistant with the tools it needs (`tool_code_interpreter`, `tool_file_search`, or custom `tool_function`).
2. Seed threads with messages and attach files to the relevant tools via `MessageAttachment`.
3. Configure `RunBuilder` with the desired model, tool choice strategy, and streaming settings.
4. Process tool calls and feed results back into the thread using the thread builder helpers.

Following this pattern keeps the full tool orchestration workflow inside the ergonomic layer—no raw HTTP payloads required.
