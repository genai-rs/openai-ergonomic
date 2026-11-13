//! `Client::upload_large_file`: create, parts in parallel (each retried on its own), complete.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::significant_drop_tightening
)]

use bytes::Bytes;
use mockito::{Matcher, Mock, Server};
use openai_ergonomic::{Client, Config, Error, UploadPurpose};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use timvw_rust_runtime::{Body, BoxError, Upload};

const CONTENT: &[u8] = b"abcdefghij";

fn upload_object(status: &str) -> serde_json::Value {
    json!({
        "id": "upload_1",
        "object": "upload",
        "created_at": 1,
        "expires_at": 2,
        "filename": "data.jsonl",
        "purpose": "fine-tune",
        "bytes": 10,
        "status": status,
        "file": null
    })
}

fn client(server: &Server) -> Client {
    let config = Config::builder()
        .api_key("sk-test")
        .api_base(server.url())
        .max_retries(0)
        .build();
    Client::builder(config).unwrap().build()
}

/// Mocks for an upload of `abcdefghij` in parts of 4 bytes: `abcd`, `efgh`, `ij`.
async fn mocks(server: &mut Server) -> Vec<Mock> {
    let create = server
        .mock("POST", "/uploads")
        .match_header("authorization", "Bearer sk-test")
        .match_body(Matcher::Json(json!({
            "filename": "data.jsonl",
            "purpose": "fine-tune",
            "bytes": 10,
            "mime_type": "application/jsonl"
        })))
        .with_header("content-type", "application/json")
        .with_body(upload_object("pending").to_string())
        .expect(1)
        .create_async()
        .await;
    let mut mocks = vec![create];
    for (index, data) in ["abcd", "efgh", "ij"].iter().enumerate() {
        mocks.push(
            server
                .mock("POST", "/uploads/upload_1/parts")
                .match_header("authorization", "Bearer sk-test")
                .match_header(
                    "content-type",
                    Matcher::Regex("^multipart/form-data; boundary=".into()),
                )
                .match_body(Matcher::Regex(format!(
                    "name=\"data\"; filename=\"[^\"]*\"\r\n(Content-Type: [^\r]*\r\n)?\r\n{data}\r\n--"
                )))
                .with_header("content-type", "application/json")
                .with_body(
                    json!({
                        "id": format!("part_{index}"),
                        "object": "upload.part",
                        "created_at": 1,
                        "upload_id": "upload_1"
                    })
                    .to_string(),
                )
                .expect(1)
                .create_async()
                .await,
        );
    }
    let mut completed = upload_object("completed");
    completed["file"] = json!({
        "id": "file-1",
        "object": "file",
        "bytes": 10,
        "created_at": 1,
        "filename": "data.jsonl",
        "purpose": "fine-tune",
        "status": "processed"
    });
    mocks.push(
        server
            .mock("POST", "/uploads/upload_1/complete")
            .match_body(Matcher::Json(
                json!({"part_ids": ["part_0", "part_1", "part_2"]}),
            ))
            .with_header("content-type", "application/json")
            .with_body(completed.to_string())
            .expect(1)
            .create_async()
            .await,
    );
    mocks
}

async fn upload_and_check(upload: Upload) {
    let mut server = Server::new_async().await;
    let mocks = mocks(&mut server).await;
    let client = client(&server);
    let result = client
        .upload_large_file(upload.file_name("data.jsonl"), UploadPurpose::FineTune)
        .mime_type("application/jsonl")
        .part_size(4)
        .concurrency(3)
        .await
        .unwrap();
    assert_eq!(result.id, "upload_1");
    assert_eq!(result.file.unwrap().unwrap().id, "file-1");
    for mock in mocks {
        mock.assert_async().await;
    }
}

#[tokio::test]
async fn uploads_bytes_in_parts() {
    upload_and_check(Upload::bytes(CONTENT)).await;
}

#[tokio::test]
async fn uploads_a_file_in_slices() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.jsonl");
    std::fs::write(&path, CONTENT).unwrap();
    upload_and_check(Upload::path(path)).await;
}

#[tokio::test]
async fn uploads_a_stream_in_chunks() {
    let chunks =
        ["abc", "defgh", "i", "j"].map(|chunk| Ok::<_, std::io::Error>(Bytes::from(chunk)));
    upload_and_check(Upload::stream(futures::stream::iter(chunks)).len(10)).await;
}

#[tokio::test]
async fn a_stream_without_a_length_is_rejected() {
    let server = Server::new_async().await;
    let chunks = [Ok::<_, std::io::Error>(Bytes::from_static(b"abc"))];
    let error = client(&server)
        .upload_large_file(
            Upload::stream(futures::stream::iter(chunks)).file_name("x"),
            UploadPurpose::Batch,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRequest(_)), "{error:?}");
}

/// A server of its own, to fail parts and count the requests in flight.
struct FakeUploads {
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    /// Failures still to answer, by part content.
    failures: Mutex<HashMap<String, usize>>,
    attempts: Mutex<HashMap<String, usize>>,
    cancelled: AtomicUsize,
    completed_with: Mutex<Option<serde_json::Value>>,
}

impl FakeUploads {
    fn new(failures: &[(&str, usize)]) -> Arc<Self> {
        Arc::new(Self {
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            failures: Mutex::new(
                failures
                    .iter()
                    .map(|(part, count)| ((*part).to_string(), *count))
                    .collect(),
            ),
            attempts: Mutex::new(HashMap::new()),
            cancelled: AtomicUsize::new(0),
            completed_with: Mutex::new(None),
        })
    }

    fn client(self: &Arc<Self>) -> Client {
        let fake = self.clone();
        let http = timvw_rust_runtime::Client::from_service(tower::service_fn(
            move |request: http::Request<Body>| {
                let fake = fake.clone();
                async move { fake.handle(request).await }
            },
        ));
        let config = Config::builder()
            .api_key("sk-test")
            .api_base("https://api.example/v1")
            .http_client(http)
            .build();
        Client::builder(config).unwrap().build()
    }

    async fn handle(&self, request: http::Request<Body>) -> Result<http::Response<Body>, BoxError> {
        let path = request.uri().path().to_string();
        let body = request.into_body().collect_bytes().await?;
        let reply = |status: u16, value: serde_json::Value| {
            let mut response = http::Response::new(Body::from(value.to_string()));
            *response.status_mut() = http::StatusCode::from_u16(status).unwrap();
            Ok(response)
        };
        match path.as_str() {
            "/v1/uploads" => reply(200, upload_object("pending")),
            "/v1/uploads/upload_1/parts" => {
                let text = String::from_utf8_lossy(&body).to_string();
                let data = text
                    .split("\r\n\r\n")
                    .nth(1)
                    .and_then(|rest| rest.split("\r\n--").next())
                    .unwrap()
                    .to_string();
                *self
                    .attempts
                    .lock()
                    .unwrap()
                    .entry(data.clone())
                    .or_default() += 1;
                let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_in_flight.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                let fail = {
                    let mut failures = self.failures.lock().unwrap();
                    match failures.get_mut(&data) {
                        Some(left) if *left > 0 => {
                            *left -= 1;
                            true
                        }
                        _ => false,
                    }
                };
                if fail {
                    return reply(
                        503,
                        json!({"error": {"message": "busy", "type": "server_error"}}),
                    );
                }
                reply(
                    200,
                    json!({"id": format!("part_{data}"), "object": "upload.part", "created_at": 1, "upload_id": "upload_1"}),
                )
            }
            "/v1/uploads/upload_1/complete" => {
                *self.completed_with.lock().unwrap() = Some(serde_json::from_slice(&body).unwrap());
                reply(200, upload_object("completed"))
            }
            "/v1/uploads/upload_1/cancel" => {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
                reply(200, upload_object("cancelled"))
            }
            other => reply(404, json!({"error": {"message": other}})),
        }
    }
}

#[tokio::test]
async fn parts_are_sent_in_parallel_within_the_bound_and_retried_on_their_own() {
    let fake = FakeUploads::new(&[("cd", 2), ("gh", 1)]);
    let client = fake.client();
    let upload = client
        .upload_large_file(
            Upload::bytes(b"abcdefghijkl".to_vec()),
            UploadPurpose::FineTune,
        )
        .filename("data.jsonl")
        .mime_type("application/jsonl")
        .part_size(2)
        .concurrency(2)
        .retry_backoff(Duration::from_millis(1))
        .await
        .unwrap();
    assert_eq!(upload.status.to_string(), "completed");
    let max = fake.max_in_flight.load(Ordering::SeqCst);
    assert!((1..=2).contains(&max), "max in flight {max}");
    let attempts = fake.attempts.lock().unwrap().clone();
    assert_eq!(attempts["cd"], 3);
    assert_eq!(attempts["gh"], 2);
    assert_eq!(attempts["ab"], 1);
    assert_eq!(attempts.len(), 6);
    // The part IDs in file order, whatever order the parts finished in.
    assert_eq!(
        fake.completed_with.lock().unwrap().clone().unwrap(),
        json!({"part_ids": ["part_ab", "part_cd", "part_ef", "part_gh", "part_ij", "part_kl"]})
    );
    assert_eq!(fake.cancelled.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_part_that_keeps_failing_cancels_the_upload() {
    let fake = FakeUploads::new(&[("ef", 10)]);
    let client = fake.client();
    let error = client
        .upload_large_file(Upload::bytes(b"abcdefgh".to_vec()), UploadPurpose::FineTune)
        .filename("data.jsonl")
        .mime_type("application/jsonl")
        .part_size(2)
        .part_retries(1)
        .retry_backoff(Duration::from_millis(1))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Api { status: 503, .. }), "{error:?}");
    assert_eq!(fake.attempts.lock().unwrap()["ef"], 2);
    assert_eq!(fake.cancelled.load(Ordering::SeqCst), 1);
    assert!(fake.completed_with.lock().unwrap().is_none());
}

#[tokio::test]
async fn the_file_name_and_media_type_default_to_the_uploads() {
    let mut server = Server::new_async().await;
    let create = server
        .mock("POST", "/uploads")
        .match_body(Matcher::Json(json!({
            "filename": "notes.txt",
            "purpose": "assistants",
            "bytes": 2,
            "mime_type": "text/plain"
        })))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": {"message": "stop here", "type": "invalid_request_error"}}"#)
        .expect(1)
        .create_async()
        .await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notes.txt");
    std::fs::write(&path, b"hi").unwrap();
    let error = client(&server)
        .upload_large_file(path, UploadPurpose::Assistants)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Api { status: 400, .. }), "{error:?}");
    create.assert_async().await;
}

#[tokio::test]
async fn an_upload_without_a_file_name_is_rejected() {
    let server = Server::new_async().await;
    let error = client(&server)
        .upload_large_file(Upload::bytes(CONTENT), UploadPurpose::Batch)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidRequest(message) if message.contains("file name")),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_missing_file_is_an_io_error() {
    let server = Server::new_async().await;
    let error = client(&server)
        .upload_large_file(
            std::path::PathBuf::from("/no/such/dir/data.jsonl"),
            UploadPurpose::FineTune,
        )
        .await
        .unwrap_err();
    assert!(matches!(&error, Error::Http(e) if e.is_io()), "{error:?}");
}
