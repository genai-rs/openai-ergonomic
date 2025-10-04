//! Integration tests for the Models API.
//!
//! These tests verify the ergonomic wrappers around the `OpenAI` Models API.
#![allow(clippy::doc_markdown)]
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::useless_vec)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use openai_ergonomic::{builders::models::*, Client, Result};

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_list_models() -> Result<()> {
    let client = Client::from_env()?;
    let response = client.models().list().await?;

    // Verify we got models
    assert!(!response.data.is_empty(), "Should have at least one model");

    // Verify structure of first model
    let first_model = &response.data[0];
    assert!(!first_model.id.is_empty());
    assert!(!first_model.owned_by.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_get_model() -> Result<()> {
    let client = Client::from_env()?;

    // Get a known model
    let model = client.models().get("gpt-3.5-turbo").await?;

    assert_eq!(model.id, "gpt-3.5-turbo");
    assert!(!model.owned_by.is_empty());
    assert!(model.created > 0);

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_get_model_with_builder() -> Result<()> {
    let client = Client::from_env()?;

    let builder = ModelRetrievalBuilder::new("gpt-4");
    let model = client.models().retrieve(builder).await?;

    assert_eq!(model.id, "gpt-4");

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default - requires a fine-tuned model to delete
async fn test_delete_model() -> Result<()> {
    // Note: This test requires a fine-tuned model that can be deleted.
    // In practice, you should create a model first before testing deletion.
    let client = Client::from_env()?;

    // This would fail if you don't have a model to delete
    // let model_id = "ft:gpt-3.5-turbo:my-org:custom:id";
    // let response = client.models().delete(model_id).await?;
    // assert_eq!(response.object, "model");
    // assert!(response.deleted);

    // For now, just verify the builder works
    let builder = ModelDeleteBuilder::new("ft:gpt-3.5-turbo:test:test:test");
    assert_eq!(builder.model_id(), "ft:gpt-3.5-turbo:test:test:test");

    Ok(())
}

#[test]
fn test_model_retrieval_builder() {
    let builder = ModelRetrievalBuilder::new("gpt-4");
    assert_eq!(builder.model_id(), "gpt-4");
}

#[test]
fn test_model_retrieval_builder_with_string() {
    let model = "gpt-3.5-turbo".to_string();
    let builder = ModelRetrievalBuilder::new(model);
    assert_eq!(builder.model_id(), "gpt-3.5-turbo");
}

#[test]
fn test_model_delete_builder() {
    let builder = ModelDeleteBuilder::new("ft:gpt-3.5-turbo:org:suffix:id");
    assert_eq!(builder.model_id(), "ft:gpt-3.5-turbo:org:suffix:id");
}

#[test]
fn test_model_delete_builder_with_string() {
    let model = "ft:gpt-4:test:test:123".to_string();
    let builder = ModelDeleteBuilder::new(model);
    assert_eq!(builder.model_id(), "ft:gpt-4:test:test:123");
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_list_models_contains_common_models() -> Result<()> {
    let client = Client::from_env()?;
    let response = client.models().list().await?;

    let model_ids: Vec<String> = response.data.iter().map(|m| m.id.clone()).collect();

    // Check for some common models (at least one should exist)
    let common_models = vec![
        "gpt-4",
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-3.5-turbo",
        "text-embedding-3-small",
        "whisper-1",
    ];

    let found_common = common_models
        .iter()
        .any(|model| model_ids.iter().any(|id| id.contains(model)));

    assert!(
        found_common,
        "Should have at least one common model in the list"
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_get_embedding_model() -> Result<()> {
    let client = Client::from_env()?;

    let model = client.models().get("text-embedding-3-small").await?;

    assert!(model.id.contains("text-embedding"));

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_get_whisper_model() -> Result<()> {
    let client = Client::from_env()?;

    let model = client.models().get("whisper-1").await?;

    assert!(model.id.contains("whisper"));

    Ok(())
}
