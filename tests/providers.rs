//! Provider presets: the generated operations and the raw requests both go to the provider, with
//! its credentials, headers, query parameters and paths.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]

use mockito::{Matcher, Mock, Server};
use openai_ergonomic::{Client, Config, Provider};
use serde_json::{json, Value};

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

fn client(provider: Provider) -> Client {
    let config = Config::builder()
        .provider(provider)
        .default_model("m")
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

/// A chat completion through the generated operation, and a raw `GET /models`.
async fn exercise(client: &Client) {
    let response = client.send_chat(client.chat_simple("Hi")).await.unwrap();
    assert_eq!(response.content(), Some("Hello"));
    let models: Value = client.get_json("/models").await.unwrap();
    assert_eq!(models["data"][0]["id"], "m");
}

async fn mocks(
    server: &mut Server,
    prefix: &str,
    query: Matcher,
    headers: &[(&'static str, Matcher)],
) -> (Mock, Mock) {
    let mut chat = server
        .mock("POST", format!("{prefix}/chat/completions").as_str())
        .match_query(query.clone())
        .with_header("content-type", "application/json")
        .with_body(completion_body());
    let mut models = server
        .mock("GET", format!("{prefix}/models").as_str())
        .match_query(query)
        .with_header("content-type", "application/json")
        .with_body(json!({"object": "list", "data": [{"id": "m"}]}).to_string());
    for (name, matcher) in headers {
        chat = chat.match_header(*name, matcher.clone());
        models = models.match_header(*name, matcher.clone());
    }
    (chat.create_async().await, models.create_async().await)
}

#[tokio::test]
async fn openai() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/v1",
        Matcher::Any,
        &[("authorization", Matcher::Exact("Bearer sk-test".into()))],
    )
    .await;
    exercise(&client(
        Provider::openai("sk-test").base_url(format!("{}/v1", server.url())),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn openai_from_the_legacy_settings() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "",
        Matcher::Any,
        &[
            ("authorization", Matcher::Exact("Bearer sk-test".into())),
            ("openai-organization", Matcher::Exact("org-1".into())),
        ],
    )
    .await;
    let config = Config::builder()
        .api_key("sk-test")
        .api_base(server.url())
        .organization("org-1")
        .default_model("m")
        .build();
    exercise(&Client::builder(config).unwrap().build()).await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn azure() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/openai/deployments/gpt-4o",
        Matcher::UrlEncoded("api-version".into(), "2024-10-21".into()),
        &[
            ("api-key", Matcher::Exact("azure-key".into())),
            ("authorization", Matcher::Missing),
        ],
    )
    .await;
    exercise(&client(
        Provider::azure(server.url(), "azure-key", "gpt-4o").api_version("2024-10-21"),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn azure_from_the_legacy_settings() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/openai/deployments/dep",
        Matcher::UrlEncoded("api-version".into(), "2024-02-01".into()),
        &[("api-key", Matcher::Exact("azure-key".into()))],
    )
    .await;
    let config = Config::builder()
        .api_key("azure-key")
        .api_base(server.url())
        .azure_deployment("dep")
        .default_model("m")
        .build();
    assert!(config.is_azure());
    exercise(&Client::builder(config).unwrap().build()).await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn openrouter() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/api/v1",
        Matcher::Any,
        &[
            ("authorization", Matcher::Exact("Bearer sk-or".into())),
            ("http-referer", Matcher::Exact("https://app.example".into())),
            ("x-title", Matcher::Exact("My App".into())),
        ],
    )
    .await;
    exercise(&client(
        Provider::openrouter("sk-or")
            .base_url(format!("{}/api/v1", server.url()))
            .http_referer("https://app.example")
            .title("My App"),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn groq() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/openai/v1",
        Matcher::Any,
        &[("authorization", Matcher::Exact("Bearer gsk".into()))],
    )
    .await;
    exercise(&client(
        Provider::groq("gsk").base_url(format!("{}/openai/v1", server.url())),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn ollama() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/v1",
        Matcher::Any,
        &[
            ("authorization", Matcher::Missing),
            ("api-key", Matcher::Missing),
        ],
    )
    .await;
    exercise(&client(
        Provider::ollama().base_url(format!("{}/v1", server.url())),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn custom() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/gateway/v1",
        Matcher::UrlEncoded("tenant".into(), "acme".into()),
        &[
            ("x-api-key", Matcher::Exact("gw-key".into())),
            ("x-team", Matcher::Exact("search".into())),
        ],
    )
    .await;
    exercise(&client(
        Provider::custom(format!("{}/gateway/v1/", server.url()))
            .api_key_header("x-api-key", "gw-key")
            .header("x-team", "search")
            .query("tenant", "acme"),
    ))
    .await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[tokio::test]
async fn the_provider_layer_wraps_a_custom_http_client() {
    let mut server = Server::new_async().await;
    let (chat, models) = mocks(
        &mut server,
        "/openai/deployments/dep",
        Matcher::UrlEncoded("api-version".into(), "2024-02-01".into()),
        &[("api-key", Matcher::Exact("k".into()))],
    )
    .await;
    let config = Config::builder()
        .provider(Provider::azure(server.url(), "k", "dep"))
        .http_client(timvw_rust_runtime::Client::default())
        .default_model("m")
        .build();
    exercise(&Client::builder(config).unwrap().build()).await;
    chat.assert_async().await;
    models.assert_async().await;
}

#[test]
fn an_invalid_provider_is_a_config_error() {
    let config = Config::builder()
        .provider(Provider::custom("not a url"))
        .build();
    assert!(matches!(
        Client::builder(config),
        Err(openai_ergonomic::Error::Config(_))
    ));
}
