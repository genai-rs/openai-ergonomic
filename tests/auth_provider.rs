//! An `AuthProvider` authenticates the generated operations and the raw requests, and a `401`
//! refreshes the credentials once and sends the request once more.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]

use mockito::{Matcher, Server};
use openai_ergonomic::auth::{AuthToken, BearerTokenAuth};
use openai_ergonomic::{Client, Config, Error, Provider};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn completion_body() -> String {
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 1,
        "model": "m",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello", "refusal": null},
            "logprobs": null,
            "finish_reason": "stop"
        }]
    })
    .to_string()
}

fn client(server: &Server, fetches: Arc<AtomicUsize>) -> Client {
    let auth = BearerTokenAuth::new(move || {
        let n = fetches.fetch_add(1, Ordering::SeqCst) + 1;
        async move { Ok(AuthToken::new(format!("token-{n}"))) }
    });
    let config = Config::builder()
        .provider(Provider::custom(server.url()))
        .auth_provider(auth)
        .default_model("m")
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

#[tokio::test]
async fn a_rejected_token_is_refreshed_and_the_request_sent_again() {
    let mut server = Server::new_async().await;
    let rejected_chat = server
        .mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer token-1")
        .with_status(401)
        .with_body(r#"{"error":{"message":"expired","type":"invalid_request_error"}}"#)
        .expect(1)
        .create_async()
        .await;
    let chat = server
        .mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer token-2")
        .match_body(Matcher::PartialJson(json!({"model": "m"})))
        .with_header("content-type", "application/json")
        .with_body(completion_body())
        .expect(1)
        .create_async()
        .await;
    let models = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer token-2")
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[{"id":"m"}]}"#)
        .create_async()
        .await;

    let fetches = Arc::new(AtomicUsize::new(0));
    let client = client(&server, fetches.clone());

    // The generated operation: 401 with token-1, refresh, sent again with token-2.
    let response = client.send_chat(client.chat_simple("Hi")).await.unwrap();
    assert_eq!(response.content(), Some("Hello"));
    // A raw request reuses token-2.
    let models_reply: Value = client.get_json("/models").await.unwrap();
    assert_eq!(models_reply["data"][0]["id"], "m");

    assert_eq!(fetches.load(Ordering::SeqCst), 2);
    rejected_chat.assert_async().await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn a_second_401_is_returned() {
    let mut server = Server::new_async().await;
    let rejected = server
        .mock("GET", "/models")
        .with_status(401)
        .with_body(
            r#"{"error":{"message":"no","type":"invalid_request_error","code":"invalid_api_key"}}"#,
        )
        .expect(2)
        .create_async()
        .await;
    let fetches = Arc::new(AtomicUsize::new(0));
    let client = client(&server, fetches.clone());
    let error = client.get_json::<Value>("/models").await.unwrap_err();
    match error {
        Error::Api {
            status, error_code, ..
        } => {
            assert_eq!(status, 401);
            assert_eq!(error_code.as_deref(), Some("invalid_api_key"));
        }
        other => panic!("expected a 401, got {other:?}"),
    }
    assert_eq!(fetches.load(Ordering::SeqCst), 2);
    rejected.assert_async().await;
}

#[tokio::test]
async fn concurrent_401s_refresh_once() {
    let mut server = Server::new_async().await;
    let _rejected = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer token-1")
        .with_status(401)
        .create_async()
        .await;
    let accepted = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer token-2")
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[]}"#)
        .expect(10)
        .create_async()
        .await;
    let fetches = Arc::new(AtomicUsize::new(0));
    let client = client(&server, fetches.clone());
    // Warm the cache with token-1 so that all ten requests are sent with it.
    let warm = client.get_json::<Value>("/models").await;
    assert!(warm.is_ok());
    assert_eq!(fetches.load(Ordering::SeqCst), 2);

    // Now the server rotates again: token-2 is rejected, token-3 accepted.
    server.reset();
    let _rejected = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer token-2")
        .with_status(401)
        .create_async()
        .await;
    let accepted_again = server
        .mock("GET", "/models")
        .match_header("authorization", "Bearer token-3")
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[]}"#)
        .expect(10)
        .create_async()
        .await;
    let requests = (0..10).map(|_| client.get_json::<Value>("/models"));
    for result in futures::future::join_all(requests).await {
        result.unwrap();
    }
    // One more fetch for all ten 401s.
    assert_eq!(fetches.load(Ordering::SeqCst), 3);
    accepted_again.assert_async().await;
    drop(accepted);
}

/// A call sent elsewhere with a per-call base URL (`RequestOptions::base_url`) is not
/// authenticated: the `AuthProvider` only sees requests under the provider's base URL.
#[tokio::test]
async fn a_per_call_base_url_elsewhere_is_not_authenticated() {
    let server = Server::new_async().await;
    let mut other = Server::new_async().await;
    let models = other
        .mock("GET", "/v1/models")
        .match_header("authorization", Matcher::Missing)
        .with_header("content-type", "application/json")
        .with_body(json!({"object": "list", "data": []}).to_string())
        .create_async()
        .await;
    let fetches = Arc::new(AtomicUsize::new(0));
    let client = client(&server, fetches.clone());
    let mut configuration = openai_client_base::apis::configuration::Configuration::new();
    configuration.client = client.http_client().clone();
    configuration.base_path = server.url();
    let options = timvw_rust_runtime::RequestOptions::new()
        .base_url(&format!("{}/v1", other.url()))
        .unwrap();
    openai_client_base::apis::models_api::list_models()
        .configuration(&configuration)
        .options(&options)
        .call()
        .await
        .unwrap();
    models.assert_async().await;
    assert_eq!(fetches.load(Ordering::SeqCst), 0);
}
