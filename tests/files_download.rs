//! Regression tests for raw file responses from the generated client, against openai-mock-server.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openai_ergonomic::{Client, Config};
use openai_mock_server::server::files_api::DownloadFileRequest;
use openai_mock_server::{op, MockServer, Reply};

fn client_for(mock: &MockServer) -> Client {
    let config = Config::builder()
        .api_key("test-key")
        .api_base(mock.base_url())
        .build();
    Client::builder(config).unwrap().build()
}

#[tokio::test]
async fn download_bytes_preserves_non_utf8_content() {
    let mock = MockServer::start().await;
    let content = vec![0, 255, 128, b'a', b'\n'];
    mock.on(op::DOWNLOAD_FILE)
        .reply(Reply::bytes("application/octet-stream", content.clone()));

    assert_eq!(
        client_for(&mock)
            .files()
            .download_bytes("file-binary")
            .await
            .unwrap(),
        content
    );
    let requests = mock.requests::<DownloadFileRequest>();
    assert_eq!(requests[0].file_id, "file-binary");
    assert_eq!(requests[0].headers["authorization"], "Bearer test-key");
}

#[tokio::test]
async fn download_decodes_utf8_text() {
    let mock = MockServer::start().await;
    let content = "File content: 你好世界\n";
    mock.on(op::DOWNLOAD_FILE)
        .reply(Reply::bytes("text/plain; charset=utf-8", content));

    assert_eq!(
        client_for(&mock)
            .files()
            .download("file-text")
            .await
            .unwrap(),
        content
    );
    assert_eq!(
        mock.requests::<DownloadFileRequest>()[0].file_id,
        "file-text"
    );
}

#[tokio::test]
async fn download_preserves_api_error_status_and_body() {
    let mock = MockServer::start().await;
    mock.on(op::DOWNLOAD_FILE)
        .reply(Reply::text("missing file").status(404));

    assert!(
        matches!(client_for(&mock).files().download_bytes("file-missing").await,
        Err(openai_ergonomic::Error::Api { status: 404, message, .. }) if message == "missing file")
    );
    assert_eq!(mock.calls(op::DOWNLOAD_FILE), 1);
}
