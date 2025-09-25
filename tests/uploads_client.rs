#![allow(missing_docs)]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use mockito::{self, Matcher};
use openai_client_base::models::create_upload_request::Purpose;
use openai_client_base::models::upload::Status as UploadStatus;
use openai_ergonomic::{
    builders::uploads::{CompleteUploadBuilder, UploadBuilder, UploadPartSource},
    Client, Config,
};
use serde_json::json;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn uploads_client_lifecycle_requests() {
    let mut server = mockito::Server::new_async().await;

    let create_mock = server
        .mock("POST", "/uploads")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!({
            "filename": "data.csv",
            "purpose": "assistants",
            "bytes": 128,
            "mime_type": "text/csv"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "upload_123",
                "created_at": 0,
                "filename": "data.csv",
                "bytes": 128,
                "purpose": "assistants",
                "status": "pending",
                "expires_at": 0,
                "object": "upload",
                "file": null
            }"#,
        )
        .create();

    let part_mock = server
        .mock("POST", "/uploads/upload_123/parts")
        .match_header("authorization", "Bearer test-key")
        .match_header(
            "content-type",
            Matcher::Regex("multipart/form-data; boundary=.*".into()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "part_1",
                "created_at": 0,
                "upload_id": "upload_123",
                "object": "upload.part"
            }"#,
        )
        .create();

    let complete_mock = server
        .mock("POST", "/uploads/upload_123/complete")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!({
            "part_ids": ["part_1"]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "upload_123",
                "created_at": 0,
                "filename": "data.csv",
                "bytes": 128,
                "purpose": "assistants",
                "status": "completed",
                "expires_at": 0,
                "object": "upload",
                "file": null
            }"#,
        )
        .create();

    let cancel_mock = server
        .mock("POST", "/uploads/upload_123/cancel")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "upload_123",
                "created_at": 0,
                "filename": "data.csv",
                "bytes": 128,
                "purpose": "assistants",
                "status": "cancelled",
                "expires_at": 0,
                "object": "upload",
                "file": null
            }"#,
        )
        .create();

    let upload_part_path = create_temp_file(b"chunk data");

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
        .build();
    let client = Client::new(config).expect("client builds");

    let upload = client
        .uploads()
        .create(UploadBuilder::new(
            "data.csv",
            Purpose::Assistants,
            128,
            "text/csv",
        ))
        .await
        .expect("create upload succeeds");
    assert_eq!(upload.id, "upload_123");

    let part = client
        .uploads()
        .add_part(
            "upload_123",
            UploadPartSource::new(upload_part_path.clone()),
        )
        .await
        .expect("add part succeeds");
    assert_eq!(part.id, "part_1");

    let completed = client
        .uploads()
        .complete("upload_123", CompleteUploadBuilder::new().part_id("part_1"))
        .await
        .expect("complete upload succeeds");
    assert_eq!(completed.status, UploadStatus::Completed);

    let cancelled = client
        .uploads()
        .cancel("upload_123")
        .await
        .expect("cancel upload succeeds");
    assert_eq!(cancelled.status, UploadStatus::Cancelled);

    create_mock.assert();
    part_mock.assert();
    complete_mock.assert();
    cancel_mock.assert();

    cleanup_temp_file(upload_part_path);
    drop(server);
}

fn create_temp_file(data: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let unique = format!("openai_ergonomic_upload_{}_{}", std::process::id(), nanos);
    path.push(unique);
    fs::write(&path, data).expect("write temp file");
    path
}

fn cleanup_temp_file(path: PathBuf) {
    let _ = fs::remove_file(path);
}
