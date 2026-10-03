//! Regression tests for raw file responses from the generated client.

use mockito::Matcher;
use openai_ergonomic::{Client, Config};

#[tokio::test]
async fn download_bytes_preserves_non_utf8_content() {
    let mut server = mockito::Server::new_async().await;
    let content = vec![0, 255, 128, b'a', b'\n'];
    let mock = server
        .mock("GET", "/files/file-binary/content")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(content.clone())
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .build();
    let client = Client::builder(config).unwrap().build();

    assert_eq!(
        client.files().download_bytes("file-binary").await.unwrap(),
        content
    );
    mock.assert_async().await;
    drop(server); // Keep the server alive until the request and assertion finish.
}

#[tokio::test]
async fn download_decodes_utf8_text() {
    let mut server = mockito::Server::new_async().await;
    let content = "File content: 你好世界\n";
    let mock = server
        .mock("GET", "/files/file-text/content")
        .with_status(200)
        .with_header("content-type", "text/plain; charset=utf-8")
        .with_body(content)
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .build();
    let client = Client::builder(config).unwrap().build();

    assert_eq!(client.files().download("file-text").await.unwrap(), content);
    mock.assert_async().await;
    drop(server); // Keep the server alive until the request and assertion finish.
}

#[tokio::test]
async fn download_preserves_api_error_status_and_body() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/files/file-missing/content")
        .match_header("authorization", Matcher::Any)
        .with_status(404)
        .with_body("missing file")
        .create_async()
        .await;
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .build();
    let client = Client::builder(config).unwrap().build();

    assert!(
        matches!(client.files().download_bytes("file-missing").await,
        Err(openai_ergonomic::Error::Api { status: 404, message, .. }) if message == "missing file")
    );
    mock.assert_async().await;
    drop(server); // Keep the server alive until the request and assertion finish.
}
