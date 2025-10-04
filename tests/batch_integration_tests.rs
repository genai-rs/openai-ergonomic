//! Integration tests for the Batch API.

use openai_ergonomic::builders::batch::{
    batch_chat_completions, batch_completions, batch_embeddings, BatchEndpoint,
    BatchJobBuilder, BatchJobListBuilder,
};
use std::collections::HashMap;

#[test]
fn test_batch_job_builder_basic() {
    let builder = BatchJobBuilder::new("file-input", BatchEndpoint::ChatCompletions);

    assert_eq!(builder.input_file_id(), "file-input");
    match builder.endpoint() {
        BatchEndpoint::ChatCompletions => {}
        _ => panic!("Expected ChatCompletions endpoint"),
    }
    assert!(!builder.has_metadata());
}

#[test]
fn test_batch_job_builder_with_metadata() {
    let builder = BatchJobBuilder::new("file-input", BatchEndpoint::Embeddings)
        .metadata("project", "test-project")
        .metadata("version", "v1");

    assert!(builder.has_metadata());
    assert_eq!(builder.metadata_ref().len(), 2);
    assert_eq!(
        builder.metadata_ref().get("project"),
        Some(&"test-project".to_string())
    );
}

#[test]
fn test_batch_job_builder_all_endpoints() {
    let chat_builder = BatchJobBuilder::new("file-1", BatchEndpoint::ChatCompletions);
    let embed_builder = BatchJobBuilder::new("file-2", BatchEndpoint::Embeddings);
    let complete_builder = BatchJobBuilder::new("file-3", BatchEndpoint::Completions);

    match chat_builder.endpoint() {
        BatchEndpoint::ChatCompletions => {}
        _ => panic!("Expected ChatCompletions"),
    }
    match embed_builder.endpoint() {
        BatchEndpoint::Embeddings => {}
        _ => panic!("Expected Embeddings"),
    }
    match complete_builder.endpoint() {
        BatchEndpoint::Completions => {}
        _ => panic!("Expected Completions"),
    }
}

#[test]
fn test_batch_job_list_builder() {
    let builder = BatchJobListBuilder::new().after("batch-123").limit(10);

    assert_eq!(builder.after_ref(), Some("batch-123"));
    assert_eq!(builder.limit_ref(), Some(10));
}

#[test]
fn test_batch_job_list_builder_default() {
    let builder = BatchJobListBuilder::new();

    assert!(builder.after_ref().is_none());
    assert!(builder.limit_ref().is_none());
}

#[test]
fn test_batch_chat_completions_helper() {
    let builder = batch_chat_completions("file-input");
    assert_eq!(builder.input_file_id(), "file-input");
    match builder.endpoint() {
        BatchEndpoint::ChatCompletions => {}
        _ => panic!("Expected ChatCompletions endpoint"),
    }
}

#[test]
fn test_batch_embeddings_helper() {
    let builder = batch_embeddings("file-input");
    match builder.endpoint() {
        BatchEndpoint::Embeddings => {}
        _ => panic!("Expected Embeddings endpoint"),
    }
}

#[test]
fn test_batch_completions_helper() {
    let builder = batch_completions("file-input");
    match builder.endpoint() {
        BatchEndpoint::Completions => {}
        _ => panic!("Expected Completions endpoint"),
    }
}

#[test]
fn test_batch_endpoint_display() {
    assert_eq!(
        BatchEndpoint::ChatCompletions.to_string(),
        "/v1/chat/completions"
    );
    assert_eq!(BatchEndpoint::Embeddings.to_string(), "/v1/embeddings");
    assert_eq!(BatchEndpoint::Completions.to_string(), "/v1/completions");
}

#[test]
fn test_batch_builder_metadata_multiple() {
    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());
    metadata.insert("key2".to_string(), "value2".to_string());

    let builder = BatchJobBuilder::new("file-input", BatchEndpoint::ChatCompletions)
        .metadata("key1", "value1")
        .metadata("key2", "value2");

    assert_eq!(builder.metadata_ref().len(), 2);
    assert_eq!(builder.metadata_ref().get("key1"), Some(&"value1".to_string()));
    assert_eq!(builder.metadata_ref().get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_batch_list_pagination() {
    let builder1 = BatchJobListBuilder::new().limit(20);
    let builder2 = BatchJobListBuilder::new().after("batch-last").limit(50);

    assert_eq!(builder1.limit_ref(), Some(20));
    assert!(builder1.after_ref().is_none());

    assert_eq!(builder2.limit_ref(), Some(50));
    assert_eq!(builder2.after_ref(), Some("batch-last"));
}
