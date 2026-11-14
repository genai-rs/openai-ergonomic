# Tool orchestration

A tool call takes a round trip: send the conversation with tool definitions, run the tools the
model asks for, append the assistant's calls and one reply per call, and send the conversation
again until the model answers. The [tool framework](tool_framework.md) defines typed tools and
dispatches the calls; this page shows the loop with plain JSON tools.

## The loop

```rust,no_run
use openai_ergonomic::{tool_function, ChatCompletionBuilder, Client, ToolCallExt};
use serde_json::{json, Value};

fn get_weather(city: &str) -> String {
    // Call your weather service here
    format!("{city}: 18 °C, light rain")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let weather = tool_function(
        "get_weather",
        "Fetch the current weather for a city",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string", "description": "The city"}},
            "required": ["city"],
        }),
    );
    let mut builder = ChatCompletionBuilder::new("gpt-4o-mini")
        .system("You are a household assistant.")
        .user("Should I carry an umbrella in Brussels today?")
        .tools(vec![weather]);

    // At most a few rounds: never loop on a model that keeps calling tools.
    for _ in 0..5 {
        let response = client.send_chat(builder.clone()).await?;
        let calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
        if calls.is_empty() {
            println!("{}", response.content().unwrap_or_default());
            break;
        }

        // The assistant's calls first, then one reply per call, with the call's id.
        builder = builder
            .assistant_with_tool_calls(response.content().unwrap_or_default(), calls.clone());
        for call in &calls {
            let reply = match call.function_name() {
                "get_weather" => {
                    let arguments: Value = call.parse_arguments()?;
                    get_weather(arguments["city"].as_str().unwrap_or_default())
                }
                other => format!("unknown tool {other}"),
            };
            builder = builder.tool(call.id(), reply);
        }
    }
    Ok(())
}
```

The builder holds the whole conversation, so keep it (or rebuild it) between rounds. Decide in
your application which calls to run (authorization, confirmation), what to reply for a call
that fails or is denied, and how many rounds to allow. See
[`examples/tool_calling_multiturn.rs`](../examples/tool_calling_multiturn.rs) and, with the
tool framework, [`examples/tool_framework_typed.rs`](../examples/tool_framework_typed.rs).

## Assistants and threads

The Assistants API runs tools server side (code interpreter, file search) and keeps the
conversation in a thread. `client.threads().create(..)` creates a thread from a
`ThreadRequestBuilder`; `MessageAttachment` attaches a file to a message for a tool;
`client.assistants()` creates assistants (`AssistantBuilder`), runs (`create_run` with a
`RunBuilder`), and answers a run's function calls with `submit_tool_outputs`.

```rust,no_run
use openai_ergonomic::builders::assistants::RunBuilder;
use openai_ergonomic::{
    AttachmentTool, Client, MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let thread = ThreadRequestBuilder::new()
        .metadata("project", "alpha")
        .message_builder(
            ThreadMessageBuilder::user("Summarise the attached architecture document.")
                .attachment(
                    MessageAttachment::for_file_search("file-abc123")
                        .with_tool(AttachmentTool::CodeInterpreter),
                ),
        )?;
    let thread = client.threads().create(thread).await?;

    let run = client
        .assistants()
        .create_run(&thread.id, RunBuilder::new("asst_123"))
        .await?;
    println!("thread {}, run {} ({:?})", thread.id, run.id, run.status);
    Ok(())
}
```

Examples: [`assistants_basic.rs`](../examples/assistants_basic.rs),
[`assistants_code_interpreter.rs`](../examples/assistants_code_interpreter.rs),
[`assistants_file_search.rs`](../examples/assistants_file_search.rs),
[`threads.rs`](../examples/threads.rs).
