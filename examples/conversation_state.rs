//! Demonstrates managing multi-turn chat with `ConversationState`.

use openai_ergonomic::{tool_function, Client, ConversationState, ToolCallExt, ToolResult};
use serde_json::json;

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    // Initialize the ergonomic client from environment variables.
    let client = Client::from_env()?.build();

    // Build a conversation state with a helpful system message and tools.
    let mut state = ConversationState::new("gpt-4o-mini")
        .with_system("You are a helpful assistant that can fetch calendar entries.");

    state.set_tools(vec![tool_function(
        "get_calendar",
        "Fetch mocked calendar entries for a given date.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "date": { "type": "string", "description": "ISO date" }
            },
            "required": ["date"]
        }),
    )]);

    // Kick off the conversation.
    state.push_user("What's on my calendar today?");
    let mut response = client.execute_chat(state.build_request()?).await?;
    state.apply_response(&response);

    // Loop while the model keeps asking for tool calls.
    while !response.tool_calls().is_empty() {
        for tool_call in response.tool_calls() {
            // Parse the arguments once and create a cached tool result.
            let args: serde_json::Value = serde_json::from_str(tool_call.function_arguments())?;

            let date = args["date"].as_str().unwrap_or("2024-01-01").to_string();

            let tool_result = ToolResult::new(json!({
                "date": date,
                "events": [
                    { "time": "09:00", "title": "Team standup" },
                    { "time": "14:00", "title": "Roadmap review" }
                ]
            }))?;

            state.push_tool_result(tool_call.id(), tool_result);
        }

        response = client.execute_chat(state.build_request()?).await?;
        state.apply_response(&response);
    }

    if let Some(content) = response.content() {
        println!("Assistant: {content}");
    }

    Ok(())
}
