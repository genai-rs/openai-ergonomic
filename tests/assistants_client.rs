#![allow(missing_docs)]

use mockito::{self, Matcher};
use openai_ergonomic::{
    builders::{
        assistants::{assistant_with_code_interpreter, RunBuilder},
        threads::{ThreadMessageBuilder, ThreadRequestBuilder},
    },
    Client, Config,
};
use serde_json::json;

#[tokio::test]
async fn assistants_client_create_posts_payload() {
    let mut server = mockito::Server::new_async().await;

    let expected_body = json!({
        "model": "gpt-4",
        "name": "Helper",
        "instructions": "Assist the user",
        "tools": [
            {"type": "code_interpreter"}
        ]
    });

    let mock = server
        .mock("POST", "/assistants")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(expected_body))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "asst_123",
                "object": "assistant",
                "created_at": 0,
                "name": "Helper",
                "description": null,
                "model": "gpt-4",
                "instructions": "Assist the user",
                "tools": [],
                "tool_resources": null,
                "metadata": {},
                "temperature": null,
                "top_p": null,
                "response_format": null
            }"#,
        )
        .create();

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
        .build();
    let client = Client::new(config).expect("client builds");

    let builder =
        assistant_with_code_interpreter("gpt-4", "Helper").instructions("Assist the user");
    let response = client
        .assistants()
        .create(builder)
        .await
        .expect("assistant created");

    assert_eq!(response.id, "asst_123");
    mock.assert();
    drop(server);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn assistants_client_thread_workflows() {
    let mut server = mockito::Server::new_async().await;

    let thread_mock = server
        .mock("POST", "/threads")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(
            json!({"messages": [{"role": "user", "content": "Hello"}] }),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "thread_123",
                "object": "thread",
                "created_at": 0,
                "tool_resources": null,
                "metadata": {}
            }"#,
        )
        .create();

    let message_mock = server
        .mock("POST", "/threads/thread_123/messages")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!({
            "role": "user",
            "content": "Add context"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "msg_123",
                "object": "thread.message",
                "created_at": 0,
                "thread_id": "thread_123",
                "status": "completed",
                "incomplete_details": null,
                "completed_at": 0,
                "incomplete_at": null,
                "role": "user",
                "content": [],
                "assistant_id": null,
                "run_id": null,
                "attachments": [],
                "metadata": {}
            }"#,
        )
        .create();

    let run_mock = server
        .mock("POST", "/threads/thread_123/runs")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!({
            "assistant_id": "asst_123",
            "model": "gpt-4",
            "stream": true
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "run_123",
                "object": "thread.run",
                "created_at": 0,
                "thread_id": "thread_123",
                "assistant_id": "asst_123",
                "status": "completed",
                "required_action": null,
                "last_error": null,
                "expires_at": 0,
                "started_at": 0,
                "cancelled_at": 0,
                "failed_at": 0,
                "completed_at": 0,
                "incomplete_details": null,
                "model": "gpt-4",
                "instructions": "",
                "tools": [],
                "metadata": {},
                "usage": null,
                "temperature": null,
                "top_p": null,
                "max_prompt_tokens": 0,
                "max_completion_tokens": 0,
                "truncation_strategy": {"type": "auto"},
                "tool_choice": "none",
                "parallel_tool_calls": false,
                "response_format": "auto"
            }"#,
        )
        .create();

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
        .build();
    let client = Client::new(config).expect("client builds");

    let thread_builder = ThreadRequestBuilder::new().user_message("Hello");
    let thread = client
        .threads()
        .create(thread_builder)
        .await
        .expect("thread created");
    assert_eq!(thread.id, "thread_123");

    let message_builder = ThreadMessageBuilder::user("Add context");
    let message = client
        .assistants()
        .create_message("thread_123", message_builder)
        .await
        .expect("message created");
    assert_eq!(message.id, "msg_123");

    let run_builder = RunBuilder::new("asst_123").model("gpt-4").stream(true);
    let run = client
        .assistants()
        .create_run("thread_123", run_builder)
        .await
        .expect("run created");
    assert_eq!(run.id, "run_123");

    thread_mock.assert();
    message_mock.assert();
    run_mock.assert();
    drop(server);
}
