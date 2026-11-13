//! "Bring your own types": `send_json`, `post_json`, `get_json` and `send_json_stream` send and
//! decode types of the caller's own, through the configured client.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]

use futures::StreamExt;
use http::Method;
use mockito::{Matcher, Server};
use openai_ergonomic::{Client, Config, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn client(server: &Server) -> Client {
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

#[derive(Serialize)]
struct MyRequest<'a> {
    model: &'a str,
    messages: Vec<MyMessage<'a>>,
    // A field the OpenAI spec does not know.
    provider_option: bool,
}

#[derive(Serialize)]
struct MyMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct MyCompletion {
    choices: Vec<MyChoice>,
}

#[derive(Debug, Deserialize)]
struct MyChoice {
    message: MyReply,
}

#[derive(Debug, Deserialize)]
struct MyReply {
    content: String,
}

#[tokio::test]
async fn custom_request_and_response_types() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_header("user-agent", Matcher::Regex("^openai-ergonomic/".into()))
        .match_body(Matcher::Json(json!({
            "model": "my-model",
            "messages": [{"role": "user", "content": "Hi"}],
            "provider_option": true
        })))
        .with_header("content-type", "application/json")
        // Only part of a completion, with a field the spec does not have.
        .with_body(r#"{"choices":[{"message":{"content":"Hello!","extra":1}}],"vendor":"x"}"#)
        .create_async()
        .await;

    let request = MyRequest {
        model: "my-model",
        messages: vec![MyMessage {
            role: "user",
            content: "Hi",
        }],
        provider_option: true,
    };
    let completion: MyCompletion = client(&server)
        .post_json("/chat/completions", &request)
        .await
        .unwrap();

    assert_eq!(completion.choices[0].message.content, "Hello!");
    mock.assert_async().await;
}

#[tokio::test]
async fn get_and_send_json_with_values() {
    let mut server = Server::new_async().await;
    let list = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer test-key")
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[{"id":"m1"},{"id":"m2"}]}"#)
        .create_async()
        .await;
    let delete = server
        .mock("DELETE", "/files/file-1")
        .match_header("authorization", "Bearer test-key")
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"file-1","deleted":true}"#)
        .create_async()
        .await;

    let client = client(&server);
    let models: Value = client.get_json("/models").await.unwrap();
    assert_eq!(models["data"][1]["id"], "m2");
    // A path without a leading slash is joined with one.
    let deleted: Value = client
        .send_json(Method::DELETE, "files/file-1", None::<&()>)
        .await
        .unwrap();
    assert_eq!(deleted["deleted"], true);

    list.assert_async().await;
    delete.assert_async().await;
}

#[tokio::test]
async fn error_status_maps_to_api_error() {
    let mut server = Server::new_async().await;
    let body = r#"{"error":{"message":"The model does not exist","type":"invalid_request_error","code":"model_not_found"}}"#;
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let error = client(&server)
        .post_json::<_, Value>("/chat/completions", &json!({"model": "nope"}))
        .await
        .unwrap_err();

    match error {
        Error::Api {
            status,
            message,
            error_type,
            error_code,
        } => {
            assert_eq!(status, 404);
            assert_eq!(message, body);
            assert_eq!(error_type.as_deref(), Some("invalid_request_error"));
            assert_eq!(error_code.as_deref(), Some("model_not_found"));
        }
        other => panic!("expected an API error, got {other:?}"),
    }
    mock.assert_async().await;
}

#[tokio::test]
async fn error_status_with_a_plain_body() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/models")
        .with_status(500)
        .with_body("upstream exploded")
        .create_async()
        .await;

    let error = client(&server)
        .get_json::<Value>("/models")
        .await
        .unwrap_err();
    assert!(error.is_server_error());
    assert!(
        matches!(error, Error::Api { status: 500, ref message, error_type: None, error_code: None } if message == "upstream exploded")
    );
    mock.assert_async().await;
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Models {
    data: Vec<Value>,
}

#[tokio::test]
async fn undecodable_reply_is_an_http_decode_error() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/models")
        .with_header("content-type", "application/json")
        .with_body(r#"{"unexpected": true}"#)
        .create_async()
        .await;

    let error = client(&server)
        .get_json::<Models>("/models")
        .await
        .unwrap_err();
    assert!(
        matches!(error, Error::Http(ref e) if e.is_decode()),
        "{error:?}"
    );
    mock.assert_async().await;
}

#[derive(Debug, Deserialize)]
struct MyChunk {
    choices: Vec<MyChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct MyChunkChoice {
    delta: MyDelta,
}

#[derive(Debug, Deserialize)]
struct MyDelta {
    content: Option<String>,
}

#[tokio::test]
async fn streaming_into_a_custom_type() {
    let mut server = Server::new_async().await;
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n\
                : a comment\n\n\
                data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}],\"vendor\":1}\n\n\
                data: {not json}\n\n\
                data: {\"choices\":[{\"delta\":{}}]}\n\n\
                data: [DONE]\n\n\
                data: {\"choices\":[{\"delta\":{\"content\":\"after done\"}}]}\n\n";
    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer test-key")
        .match_header("accept", "text/event-stream")
        .match_body(Matcher::PartialJson(json!({"stream": true})))
        .with_header("content-type", "text/event-stream")
        .with_body(body)
        .create_async()
        .await;

    let stream = client(&server)
        .send_json_stream::<_, MyChunk>(
            "/chat/completions",
            &json!({"model": "my-model", "stream": true, "messages": []}),
        )
        .await
        .unwrap();
    let items: Vec<_> = stream.collect().await;

    assert_eq!(items.len(), 4, "{items:?}");
    let content = |item: &openai_ergonomic::Result<MyChunk>| {
        item.as_ref().unwrap().choices[0].delta.content.clone()
    };
    assert_eq!(content(&items[0]).as_deref(), Some("Hel"));
    assert_eq!(content(&items[1]).as_deref(), Some("lo"));
    // A payload that does not decode is an error, and the stream goes on.
    assert!(
        matches!(&items[2], Err(Error::StreamParsing { chunk, .. }) if chunk == "{not json}"),
        "{:?}",
        items[2]
    );
    assert_eq!(content(&items[3]), None);
    mock.assert_async().await;
}

#[tokio::test]
async fn streaming_error_status_is_returned_before_the_stream() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(401)
        .with_body(r#"{"error":{"message":"bad key","type":"invalid_request_error","code":"invalid_api_key"}}"#)
        .create_async()
        .await;

    let result = client(&server)
        .send_json_stream::<_, Value>("/chat/completions", &json!({"stream": true}))
        .await;
    let Err(error) = result else {
        panic!("expected an error")
    };
    assert!(error.is_auth_error());
    assert!(
        matches!(error, Error::Api { error_code: Some(ref code), .. } if code == "invalid_api_key")
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn raw_calls_go_through_the_client_layers() {
    let mut server = Server::new_async().await;
    // The organization and project headers are a layer of the client.
    let mock = server
        .mock("POST", "/embeddings")
        .match_header("openai-organization", "org-123")
        .match_header("openai-project", "proj-456")
        .match_header("authorization", "Bearer test-key")
        .with_body(r#"{"ok":true}"#)
        .create_async()
        .await;

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .organization("org-123")
        .project("proj-456")
        .build();
    let client = Client::builder(config).unwrap().build();
    let reply: Value = client
        .post_json("/embeddings", &json!({"input": "x"}))
        .await
        .unwrap();
    assert_eq!(reply["ok"], true);
    mock.assert_async().await;
}

#[tokio::test]
async fn default_client_retries_transient_errors() {
    let mut server = Server::new_async().await;
    let failing = server
        .mock("GET", "/models")
        .with_status(503)
        .with_header("retry-after", "0")
        .expect(2)
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .max_retries(1)
        .build();
    let client = Client::builder(config).unwrap().build();

    let error = client.get_json::<Value>("/models").await.unwrap_err();
    assert!(matches!(error, Error::Api { status: 503, .. }));
    // One attempt and one retry.
    failing.assert_async().await;
}

#[tokio::test]
async fn azure_auth_and_paths_apply_to_raw_calls() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/openai/deployments/my-deployment/chat/completions")
        .match_query(Matcher::UrlEncoded(
            "api-version".into(),
            "2024-02-01".into(),
        ))
        .match_header("api-key", "azure-key")
        .match_header("authorization", Matcher::Missing)
        .with_body(r#"{"choices":[{"message":{"content":"from azure"}}]}"#)
        .create_async()
        .await;

    let config = Config::builder()
        .api_key("azure-key")
        .api_base(server.url())
        .azure_deployment("my-deployment")
        .build();
    let client = Client::builder(config).unwrap().build();
    let completion: MyCompletion = client
        .post_json("/chat/completions", &json!({"messages": []}))
        .await
        .unwrap();
    assert_eq!(completion.choices[0].message.content, "from azure");
    mock.assert_async().await;
}
