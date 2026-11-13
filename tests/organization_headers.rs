//! The `OpenAI-Organization` and `OpenAI-Project` headers go with every request, streamed or not.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use futures::StreamExt;
use mockito::{Mock, Request, Server};
use openai_ergonomic::{Client, Config};
use serde_json::json;

const ORGANIZATION: &str = "openai-organization";
const PROJECT: &str = "openai-project";

fn config(server: &Server) -> openai_ergonomic::config::ConfigBuilder {
    Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
}

fn single(request: &Request, name: &str, value: &str) -> bool {
    let values = request.header(name);
    values.len() == 1 && values[0] == value
}

fn has_both(request: &Request) -> bool {
    single(request, ORGANIZATION, "org-123") && single(request, PROJECT, "proj-456")
}

fn has_neither(request: &Request) -> bool {
    !request.has_header(ORGANIZATION) && !request.has_header(PROJECT)
}

fn completion_body() -> String {
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello", "refusal": null},
            "logprobs": null,
            "finish_reason": "stop"
        }]
    })
    .to_string()
}

fn stream_body() -> String {
    "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\
     \"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\
     \"finish_reason\":null}]}\n\ndata: [DONE]\n\n"
        .to_string()
}

async fn completion_mock(
    server: &mut Server,
    matcher: impl Fn(&Request) -> bool + Send + Sync + 'static,
) -> Mock {
    server
        .mock("POST", "/chat/completions")
        .match_request(matcher)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(completion_body())
        .create_async()
        .await
}

async fn stream_mock(
    server: &mut Server,
    matcher: impl Fn(&Request) -> bool + Send + Sync + 'static,
) -> Mock {
    server
        .mock("POST", "/chat/completions")
        .match_request(matcher)
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(stream_body())
        .create_async()
        .await
}

async fn send_normal(client: &Client) {
    let response = client.send_chat(client.chat().user("hello")).await.unwrap();
    assert_eq!(response.content(), Some("Hello"));
}

async fn send_stream(client: &Client) {
    let stream = client
        .send_chat_stream(client.chat().user("hello"))
        .await
        .unwrap();
    let chunks: Vec<_> = stream.collect().await;
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap().content(), Some("Hello"));
}

#[tokio::test]
async fn configured_headers_go_with_normal_and_streaming_requests() {
    let mut server = Server::new_async().await;
    let normal = completion_mock(&mut server, has_both).await;
    let client = Client::builder(
        config(&server)
            .organization("org-123")
            .project("proj-456")
            .build(),
    )
    .unwrap()
    .build();

    send_normal(&client).await;
    normal.assert_async().await;
    normal.remove_async().await;

    let streamed = stream_mock(&mut server, has_both).await;
    send_stream(&client).await;
    streamed.assert_async().await;
    drop(server);
}

#[tokio::test]
async fn no_headers_without_organization_or_project() {
    let mut server = Server::new_async().await;
    let normal = completion_mock(&mut server, has_neither).await;
    let client = Client::builder(config(&server).build()).unwrap().build();

    send_normal(&client).await;
    normal.assert_async().await;
    normal.remove_async().await;

    let streamed = stream_mock(&mut server, has_neither).await;
    send_stream(&client).await;
    streamed.assert_async().await;
    drop(server);
}

#[tokio::test]
async fn only_the_configured_header_is_sent() {
    let mut server = Server::new_async().await;
    let mock = completion_mock(&mut server, |request| {
        single(request, PROJECT, "proj-456") && !request.has_header(ORGANIZATION)
    })
    .await;
    let client = Client::builder(config(&server).project("proj-456").build())
        .unwrap()
        .build();

    send_normal(&client).await;
    mock.assert_async().await;
    drop(server);
}

#[tokio::test]
async fn headers_go_through_a_user_supplied_client_once() {
    let mut server = Server::new_async().await;
    // The user's own client already sends an organization header by default: it is replaced,
    // not repeated.
    let mut defaults = reqwest::header::HeaderMap::new();
    defaults.insert(ORGANIZATION, "org-default".parse().unwrap());
    let http_client = timvw_rust_runtime::Client::from_reqwest(
        reqwest::Client::builder()
            .default_headers(defaults)
            .build()
            .unwrap(),
    );
    let client = Client::builder(
        config(&server)
            .organization("org-123")
            .project("proj-456")
            .http_client(http_client)
            .build(),
    )
    .unwrap()
    .build();

    let normal = completion_mock(&mut server, has_both).await;
    send_normal(&client).await;
    normal.assert_async().await;
    normal.remove_async().await;

    let streamed = stream_mock(&mut server, has_both).await;
    send_stream(&client).await;
    streamed.assert_async().await;
    drop(server);
}

#[test]
fn an_invalid_header_value_is_a_config_error() {
    let config = Config::builder()
        .api_key("test-key")
        .organization("org\n123")
        .build();
    assert!(matches!(
        Client::builder(config),
        Err(openai_ergonomic::Error::Config(_))
    ));
}

#[tokio::test]
async fn generated_stream_keeps_error_mapping() {
    let mut server = Server::new_async().await;
    // Without retries, so the 429 is returned at once.
    let client = Client::builder(config(&server).max_retries(0).build())
        .unwrap()
        .build();

    let rejected = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":{"message":"slow down"}}"#)
        .create_async()
        .await;
    let Err(error) = client.send_chat_stream(client.chat().user("hello")).await else {
        panic!("a 429 should fail the request");
    };
    match error {
        openai_ergonomic::Error::Api {
            status, message, ..
        } => {
            assert_eq!(status, 429);
            assert!(message.contains("slow down"));
        }
        other => panic!("expected an API error, got {other:?}"),
    }
    rejected.assert_async().await;
    rejected.remove_async().await;

    let malformed = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(format!("data: {{not-json}}\n\n{}", stream_body()))
        .create_async()
        .await;
    let chunks: Vec<_> = client
        .send_chat_stream(client.chat().user("hello"))
        .await
        .unwrap()
        .collect()
        .await;
    match &chunks[0] {
        Err(openai_ergonomic::Error::StreamParsing { chunk, .. }) => {
            assert!(chunk.contains("{not-json}"));
        }
        other => panic!("expected a parse error carrying the chunk, got {other:?}"),
    }
    malformed.assert_async().await;
    drop(server);
}
