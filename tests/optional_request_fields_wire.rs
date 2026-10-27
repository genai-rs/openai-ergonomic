//! Regression tests for omitted and explicitly empty optional request fields.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use mockito::{Matcher, Server};
use openai_ergonomic::{
    builders::fine_tuning::FineTuningJobBuilder, builders::vector_stores::VectorStoreBuilder,
    Client, Config,
};
use serde_json::json;

fn client_for(server: &Server) -> Client {
    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-4")
        .build();
    Client::builder(config).expect("client builds").build()
}

#[tokio::test]
async fn vector_store_update_omits_unset_name_and_sends_empty_name() {
    let mut server = Server::new_async().await;
    let omitted = server
        .mock("POST", "/vector_stores/vs-123")
        .match_body(Matcher::Json(json!({})))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(vector_store_response())
        .create_async()
        .await;

    client_for(&server)
        .vector_stores()
        .update("vs-123", VectorStoreBuilder::new())
        .await
        .expect("empty update succeeds");
    omitted.assert_async().await;

    let explicit_empty = server
        .mock("POST", "/vector_stores/vs-123")
        .match_body(Matcher::Json(json!({ "name": "" })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(vector_store_response())
        .create_async()
        .await;

    client_for(&server)
        .vector_stores()
        .update("vs-123", VectorStoreBuilder::new().name(""))
        .await
        .expect("empty-name update succeeds");
    explicit_empty.assert_async().await;
    drop(server);
}

#[tokio::test]
async fn fine_tuning_create_job_omits_unset_fields_and_sends_empty_values() {
    let mut server = Server::new_async().await;
    let omitted = server
        .mock("POST", "/fine_tuning/jobs")
        .match_body(Matcher::Json(json!({
            "model": "gpt-4",
            "training_file": "file-training"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(fine_tuning_job_response())
        .create_async()
        .await;

    client_for(&server)
        .fine_tuning()
        .create_job(FineTuningJobBuilder::new("gpt-4", "file-training"))
        .await
        .expect("default fine-tuning job succeeds");
    omitted.assert_async().await;

    let explicit_empty = server
        .mock("POST", "/fine_tuning/jobs")
        .match_body(Matcher::Json(json!({
            "model": "gpt-4",
            "training_file": "file-training",
            "validation_file": "",
            "suffix": ""
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(fine_tuning_job_response())
        .create_async()
        .await;

    client_for(&server)
        .fine_tuning()
        .create_job(
            FineTuningJobBuilder::new("gpt-4", "file-training")
                .validation_file("")
                .suffix(""),
        )
        .await
        .expect("explicit-empty fine-tuning job succeeds");
    explicit_empty.assert_async().await;
    drop(server);
}

fn vector_store_response() -> &'static str {
    r#"{
        "id": "vs-123",
        "object": "vector_store",
        "created_at": 1,
        "name": "store",
        "usage_bytes": 0,
        "file_counts": {"in_progress": 0, "completed": 0, "failed": 0, "cancelled": 0, "total": 0},
        "status": "completed",
        "expires_at": null,
        "last_active_at": null,
        "metadata": null
    }"#
}

fn fine_tuning_job_response() -> &'static str {
    r#"{
        "id": "ftjob-123",
        "created_at": 1,
        "error": null,
        "fine_tuned_model": null,
        "finished_at": null,
        "model": "gpt-4",
        "object": "fine_tuning.job",
        "organization_id": "org-123",
        "result_files": [],
        "status": "queued",
        "trained_tokens": null,
        "training_file": "file-training",
        "validation_file": null,
        "seed": null
    }"#
}
