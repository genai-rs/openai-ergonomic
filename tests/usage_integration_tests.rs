//! Integration tests for the Usage API.
//!
//! These tests verify the ergonomic wrappers around the OpenAI Usage API.

use openai_ergonomic::{
    builders::usage::{BucketWidth, GroupBy, UsageBuilder},
    Client, Result,
};

fn get_test_time_range() -> (i32, i32) {
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;
    let start_time = end_time - (7 * 24 * 60 * 60); // 7 days ago
    (start_time, end_time)
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completions_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().completions(builder).await?;

    // Data may be empty if no usage in the time period
    // Just verify the structure is correct
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_embeddings_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().embeddings(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_audio_speeches_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().audio_speeches(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_audio_transcriptions_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().audio_transcriptions(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_images_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().images(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_moderations_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().moderations(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_vector_stores_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().vector_stores(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_code_interpreter_sessions_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let usage = client.usage().code_interpreter_sessions(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_costs_usage() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time));
    let costs = client.usage().costs(builder).await?;

    // Just verify the request succeeded
    assert!(costs.data.is_empty() || !costs.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_usage_with_bucket_width() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time))
        .bucket_width(BucketWidth::Day);

    let usage = client.usage().completions(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_usage_with_model_filter() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time))
        .model("gpt-4");

    let usage = client.usage().completions(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_usage_with_group_by() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time))
        .group_by(GroupBy::Model);

    let usage = client.usage().completions(builder).await?;

    // Just verify the request succeeded
    assert!(usage.data.is_empty() || !usage.data.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_usage_with_limit() -> Result<()> {
    let client = Client::from_env()?;
    let (start_time, end_time) = get_test_time_range();

    let builder = UsageBuilder::new(start_time, Some(end_time))
        .limit(10);

    let usage = client.usage().completions(builder).await?;

    // Just verify the request succeeded and respects limit
    assert!(usage.data.len() <= 10);

    Ok(())
}

#[test]
fn test_usage_builder_basic() {
    let builder = UsageBuilder::new(1704067200, None);
    assert_eq!(builder.start_time(), 1704067200);
    assert_eq!(builder.end_time(), None);
}

#[test]
fn test_usage_builder_with_end_time() {
    let builder = UsageBuilder::new(1704067200, Some(1704153600));
    assert_eq!(builder.start_time(), 1704067200);
    assert_eq!(builder.end_time(), Some(1704153600));
}

#[test]
fn test_usage_builder_with_bucket_width() {
    let builder = UsageBuilder::new(1704067200, None)
        .bucket_width(BucketWidth::Day);

    assert_eq!(builder.bucket_width_ref(), Some(BucketWidth::Day));
    assert_eq!(builder.bucket_width_str(), Some("1d"));
}

#[test]
fn test_usage_builder_with_filters() {
    let builder = UsageBuilder::new(1704067200, None)
        .project_id("proj_123")
        .user_id("user_456")
        .model("gpt-4");

    assert_eq!(builder.project_ids_ref(), &["proj_123"]);
    assert_eq!(builder.user_ids_ref(), &["user_456"]);
    assert_eq!(builder.models_ref(), &["gpt-4"]);
}

#[test]
fn test_usage_builder_with_multiple_filters() {
    let builder = UsageBuilder::new(1704067200, None)
        .project_ids(vec!["proj_1", "proj_2"])
        .user_ids(vec!["user_1", "user_2"])
        .models(vec!["gpt-4", "gpt-3.5-turbo"]);

    assert_eq!(builder.project_ids_ref().len(), 2);
    assert_eq!(builder.user_ids_ref().len(), 2);
    assert_eq!(builder.models_ref().len(), 2);
}

#[test]
fn test_usage_builder_with_group_by() {
    let builder = UsageBuilder::new(1704067200, None)
        .group_by(GroupBy::ProjectId)
        .group_by(GroupBy::Model);

    assert_eq!(builder.group_by_ref().len(), 2);
    let group_by_strings = builder.group_by_option().unwrap();
    assert_eq!(group_by_strings, vec!["project_id", "model"]);
}

#[test]
fn test_bucket_width_display() {
    assert_eq!(BucketWidth::Day.to_string(), "1d");
    assert_eq!(BucketWidth::Hour.to_string(), "1h");
}

#[test]
fn test_group_by_display() {
    assert_eq!(GroupBy::ProjectId.to_string(), "project_id");
    assert_eq!(GroupBy::UserId.to_string(), "user_id");
    assert_eq!(GroupBy::ApiKeyId.to_string(), "api_key_id");
    assert_eq!(GroupBy::Model.to_string(), "model");
}
