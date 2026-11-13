//! Saving generated images and speech to files, and uploading or downloading files by stream.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]

use base64::Engine;
use mockito::{Matcher, Server};
use openai_client_base::models::{Image, ImagesResponse};
use openai_ergonomic::builders::files::FilePurpose;
use openai_ergonomic::{Client, Config};
use serde_json::json;
use timvw_rust_runtime::Upload;

fn client(server: &Server) -> Client {
    let config = Config::builder()
        .api_key("sk-test")
        .api_base(server.url())
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

#[tokio::test]
async fn saves_base64_and_downloaded_images() {
    let server = Server::new_async().await;
    let mut cdn = Server::new_async().await;
    let download = cdn
        .mock("GET", "/images/cat.png")
        // The provider's credentials stay with the provider.
        .match_header("authorization", Matcher::Missing)
        .with_body(b"downloaded-png")
        .expect(1)
        .create_async()
        .await;
    let client = client(&server);
    let directory = tempfile::tempdir().unwrap();

    let encoded = base64::engine::general_purpose::STANDARD.encode(b"decoded-png");
    let image = Image {
        b64_json: Some(encoded),
        ..Image::default()
    };
    let path = directory.path().join("a.png");
    assert_eq!(client.images().save(&image, &path).await.unwrap(), 11);
    assert_eq!(std::fs::read(&path).unwrap(), b"decoded-png");

    let image = Image {
        url: Some(format!("{}/images/cat.png", cdn.url())),
        ..Image::default()
    };
    let path = directory.path().join("b.png");
    assert_eq!(client.images().save(&image, &path).await.unwrap(), 14);
    assert_eq!(std::fs::read(&path).unwrap(), b"downloaded-png");
    download.assert_async().await;

    // No temporary files left behind.
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);

    let missing = Image::default();
    assert!(client.images().save(&missing, &path).await.is_err());
}

#[tokio::test]
async fn saves_every_image_of_a_response() {
    let server = Server::new_async().await;
    let client = client(&server);
    let directory = tempfile::tempdir().unwrap();
    let encode = |text: &str| base64::engine::general_purpose::STANDARD.encode(text);
    let response: ImagesResponse = serde_json::from_value(json!({
        "created": 1,
        "output_format": "webp",
        "data": [{"b64_json": encode("one")}, {"b64_json": encode("two")}]
    }))
    .unwrap();
    let paths = client
        .images()
        .save_all(&response, directory.path(), "cat")
        .await
        .unwrap();
    assert_eq!(
        paths,
        vec![
            directory.path().join("cat-0.webp"),
            directory.path().join("cat-1.webp")
        ]
    );
    assert_eq!(std::fs::read(&paths[1]).unwrap(), b"two");
}

#[tokio::test]
async fn a_failed_download_leaves_the_file_alone() {
    let mut server = Server::new_async().await;
    let _missing = server
        .mock("GET", "/gone.png")
        .with_status(404)
        .create_async()
        .await;
    let client = client(&server);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("keep.png");
    std::fs::write(&path, b"old").unwrap();
    let image = Image {
        url: Some(format!("{}/gone.png", server.url())),
        ..Image::default()
    };
    let error = client.images().save(&image, &path).await.unwrap_err();
    assert!(matches!(
        error,
        openai_ergonomic::Error::Api { status: 404, .. }
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
}

#[tokio::test]
async fn streams_speech_to_a_file() {
    let mut server = Server::new_async().await;
    let speech = server
        .mock("POST", "/audio/speech")
        .match_header("authorization", "Bearer sk-test")
        .match_body(Matcher::PartialJson(
            json!({"model": "tts-1", "input": "Hello", "voice": "alloy"}),
        ))
        .with_header("content-type", "audio/mpeg")
        .with_body(vec![7u8; 100_000])
        .expect(1)
        .create_async()
        .await;
    let client = client(&server);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hello.mp3");
    let builder = client.audio().speech("tts-1", "Hello", "alloy");
    let written = client.audio().speech_to_file(builder, &path).await.unwrap();
    assert_eq!(written, 100_000);
    assert_eq!(std::fs::read(&path).unwrap(), vec![7u8; 100_000]);
    speech.assert_async().await;
}

#[tokio::test]
async fn uploads_and_downloads_files_by_stream() {
    let mut server = Server::new_async().await;
    let file_json = json!({
        "id": "file-1",
        "object": "file",
        "bytes": 5,
        "created_at": 1,
        "filename": "notes.txt",
        "purpose": "assistants",
        "status": "processed"
    });
    let upload = server
        .mock("POST", "/files")
        .match_header("authorization", "Bearer sk-test")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex("name=\"purpose\"\r\n\r\nassistants\r\n".into()),
            Matcher::Regex(
                "name=\"file\"; filename=\"notes.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\r\n"
                    .into(),
            ),
        ]))
        .with_header("content-type", "application/json")
        .with_body(file_json.to_string())
        .expect(2)
        .create_async()
        .await;
    let content = server
        .mock("GET", "/files/file-1/content")
        .with_body("file content")
        .create_async()
        .await;
    let client = client(&server);

    let file = client
        .files()
        .upload_file(
            Upload::bytes(b"hello".to_vec())
                .file_name("notes.txt")
                .mime("text/plain"),
            FilePurpose::Assistants,
        )
        .await
        .unwrap();
    assert_eq!(file.id, "file-1");

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notes.txt");
    std::fs::write(&path, b"hello").unwrap();
    client
        .files()
        .upload_file(path, FilePurpose::Assistants)
        .await
        .unwrap();
    upload.assert_async().await;

    let target = directory.path().join("content.txt");
    assert_eq!(
        client.files().download_to("file-1", &target).await.unwrap(),
        12
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "file content");
    content.assert_async().await;
}
