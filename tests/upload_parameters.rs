//! File parameters are `timvw_rust_runtime::Upload`s: paths are streamed from disk, bytes are
//! sent from memory (no temporary files), and a missing file is an `Error::Http` with `is_io()`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(unknown_lints, clippy::unused_async_trait_impl)]

use mockito::{Matcher, Server};
use openai_ergonomic::builders::audio::TranscriptionBuilder;
use openai_ergonomic::builders::files::{FilePurpose, FileUploadBuilder};
use openai_ergonomic::builders::images::ImageEditBuilder;
use openai_ergonomic::{Client, Config, Error, ErrorContext, Interceptor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use timvw_rust_runtime::Upload;

const MISSING: &str = "/no/such/dir/missing.wav";

fn client(server: &Server) -> Client {
    let config = Config::builder()
        .api_key("sk-test")
        .api_base(server.url())
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

fn assert_missing_file(error: &Error) {
    assert!(matches!(error, Error::Http(e) if e.is_io()), "{error:?}");
    assert!(error.is_io());
    assert_eq!(
        error.io_error().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(error.to_string().contains("missing"), "{error}");
}

struct CountErrors(Arc<AtomicUsize>);

impl Interceptor for CountErrors {
    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn a_missing_file_fails_the_upload_before_anything_is_sent() {
    let mut server = Server::new_async().await;
    let never = server
        .mock("POST", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let errors = Arc::new(AtomicUsize::new(0));
    let config = Config::builder()
        .api_key("sk-test")
        .api_base(server.url())
        .build();
    let client = Client::builder(config)
        .unwrap()
        .with_interceptor(Box::new(CountErrors(errors.clone())))
        .build();

    let error = client
        .files()
        .upload(FileUploadBuilder::from_path(
            MISSING,
            FilePurpose::Assistants,
        ))
        .await
        .unwrap_err();
    assert_missing_file(&error);

    let error = client
        .audio()
        .create_transcription(TranscriptionBuilder::new(Path::new(MISSING), "whisper-1"))
        .await
        .unwrap_err();
    assert_missing_file(&error);

    let error = client
        .images()
        .create_edit(ImageEditBuilder::new(PathBuf::from(MISSING), "edit").model("gpt-image-1"))
        .await
        .unwrap_err();
    assert_missing_file(&error);

    // The interceptors hear of each failure.
    assert_eq!(errors.load(Ordering::SeqCst), 3);
    never.assert_async().await;
}

#[tokio::test]
async fn bytes_are_uploaded_from_memory_with_their_name() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/files")
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#"name="purpose"\r\n\r\nbatch\r\n"#.into()),
            Matcher::Regex(
                "name=\"file\"; filename=\"batch.jsonl\"\r\n(Content-Type: [^\r]*\r\n)?\r\n\\{\\}\n\r\n"
                    .into(),
            ),
        ]))
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id": "file-1", "object": "file", "bytes": 3, "created_at": 1,
                "filename": "batch.jsonl", "purpose": "batch", "status": "processed"}"#,
        )
        .expect(2)
        .create_async()
        .await;
    let client = client(&server);

    let file = client
        .files()
        .upload(
            client
                .files()
                .upload_bytes("batch.jsonl", FilePurpose::Batch, b"{}\n".to_vec()),
        )
        .await
        .unwrap();
    assert_eq!(file.id, "file-1");

    let upload = Upload::bytes(b"{}\n".to_vec()).file_name("batch.jsonl");
    client
        .files()
        .create(client.files().upload_from(upload, FilePurpose::Batch))
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn a_path_is_streamed_with_its_name_and_media_type() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/audio/transcriptions")
        .match_body(Matcher::Regex(
            "name=\"file\"; filename=\"clip.wav\"\r\nContent-Type: audio/[a-z-]*wav\r\n\r\nRIFF"
                .into(),
        ))
        .with_header("content-type", "application/json")
        .with_body(r#"{"text": "hello"}"#)
        .expect(1)
        .create_async()
        .await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("clip.wav");
    std::fs::write(&path, b"RIFF....WAVE").unwrap();

    let client = client(&server);
    let response = client
        .audio()
        .create_transcription(client.audio().transcription(path, "whisper-1"))
        .await
        .unwrap();
    assert!(serde_json::to_string(&response).unwrap().contains("hello"));
    mock.assert_async().await;
}
