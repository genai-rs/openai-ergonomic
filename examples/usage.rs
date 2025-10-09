#![allow(clippy::uninlined_format_args)]
//! Usage API example.
//!
//! This example demonstrates:
//! - Querying usage data for different API endpoints
//! - Filtering usage by time range, projects, users, and models
//! - Aggregating usage data
//! - Retrieving cost data
//!
//! Run with: `cargo run --example usage`

use openai_ergonomic::{
    builders::usage::{BucketWidth, GroupBy, UsageBuilder},
    Client, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Usage API Examples ===\n");

    // Initialize client
    let client = Client::from_env()?.build();

    // Get current time and time 30 days ago (in Unix timestamp seconds)
    // Using hardcoded timestamps for demonstration
    let end_time = 1_700_000_000_i32; // Example: Nov 2023
    let start_time = end_time - (30 * 24 * 60 * 60); // 30 days ago

    println!(
        "Querying usage data (timestamps: {} to {})",
        start_time, end_time
    );
    println!();

    // Example 1: Basic usage query
    println!("1. Basic Usage Query (Completions):");
    basic_usage_query(&client, start_time, end_time).await?;

    // Example 2: Usage with aggregation
    println!("\n2. Usage with Daily Aggregation:");
    usage_with_aggregation(&client, start_time, end_time).await?;

    // Example 3: Usage filtered by model
    println!("\n3. Usage Filtered by Model:");
    usage_by_model(&client, start_time, end_time).await?;

    // Example 4: Usage grouped by project
    println!("\n4. Usage Grouped by Project:");
    usage_grouped_by_project(&client, start_time, end_time).await?;

    // Example 5: Cost data
    println!("\n5. Cost Data:");
    cost_data(&client, start_time, end_time).await?;

    // Example 6: Audio usage
    println!("\n6. Audio Usage:");
    audio_usage(&client, start_time, end_time).await?;

    // Example 7: Image usage
    println!("\n7. Image Usage:");
    image_usage(&client, start_time, end_time).await?;

    // Example 8: Embeddings usage
    println!("\n8. Embeddings Usage:");
    embeddings_usage(&client, start_time, end_time).await?;

    println!("\n=== All examples completed successfully ===");

    Ok(())
}

async fn basic_usage_query(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time));

    let usage = client.usage().completions(builder).await?;

    println!("Completions usage:");
    println!("  Data points: {}", usage.data.len());

    if usage.has_more {
        println!("  Has more: yes");
    }

    Ok(())
}

async fn usage_with_aggregation(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .bucket_width(BucketWidth::Day)
        .limit(10);

    let usage = client.usage().completions(builder).await?;

    println!("Daily aggregated completions usage:");
    println!("  Bucket width: 1 day");
    println!("  Data points: {}", usage.data.len());

    Ok(())
}

async fn usage_by_model(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .model("gpt-4")
        .limit(100);

    let usage = client.usage().completions(builder).await?;

    println!("Completions usage for gpt-4:");
    println!("  Data points: {}", usage.data.len());

    Ok(())
}

async fn usage_grouped_by_project(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .group_by(GroupBy::ProjectId)
        .group_by(GroupBy::Model)
        .limit(50);

    let usage = client.usage().completions(builder).await?;

    println!("Completions usage grouped by project and model:");
    println!("  Data points: {}", usage.data.len());

    Ok(())
}

async fn cost_data(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .bucket_width(BucketWidth::Day)
        .limit(10);

    let costs = client.usage().costs(builder).await?;

    println!("Cost data:");
    println!("  Data points: {}", costs.data.len());

    Ok(())
}

async fn audio_usage(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time)).limit(10);

    // Audio speeches (text-to-speech)
    let speeches = client.usage().audio_speeches(builder.clone()).await?;
    println!("Audio speeches usage: {} data points", speeches.data.len());

    // Audio transcriptions
    let transcriptions = client.usage().audio_transcriptions(builder).await?;
    println!(
        "Audio transcriptions usage: {} data points",
        transcriptions.data.len()
    );

    Ok(())
}

async fn image_usage(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .bucket_width(BucketWidth::Day)
        .limit(10);

    let usage = client.usage().images(builder).await?;

    println!("Image generation usage:");
    println!("  Data points: {}", usage.data.len());

    Ok(())
}

async fn embeddings_usage(client: &Client, start_time: i32, end_time: i32) -> Result<()> {
    let builder = UsageBuilder::new(start_time, Some(end_time))
        .model("text-embedding-3-small")
        .limit(100);

    let usage = client.usage().embeddings(builder).await?;

    println!("Embeddings usage for text-embedding-3-small:");
    println!("  Data points: {}", usage.data.len());

    Ok(())
}
