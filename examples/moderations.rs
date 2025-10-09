#![allow(clippy::uninlined_format_args)]
//! Content moderation and filtering example.
//!
//! This example demonstrates:
//! - Content moderation API usage
//! - Category detection
//! - Threshold configuration
//! - Multi-language support
//! - Custom filtering rules
//! - Batch moderation
//! - Response filtering
//!
//! Run with: `cargo run --example moderations`

use openai_ergonomic::{Client, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct ModerationResult {
    flagged: bool,
    categories: HashMap<String, bool>,
    scores: HashMap<String, f64>,
}

#[derive(Debug)]
struct ModerationPolicy {
    thresholds: HashMap<String, f64>,
    auto_reject_categories: Vec<String>,
    require_human_review: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    use openai_ergonomic::Config;

    println!("=== Content Moderation Example ===\n");

    // Initialize client
    let client = if let Ok(c) = Client::from_env() {
        c.build()
    } else {
        println!("Note: OPENAI_API_KEY not found. Running in demo mode.");
        println!("Set OPENAI_API_KEY to test real API calls.\n");
        println!("To use the Moderations API:");
        println!("  let client = Client::from_env()?.build();");
        println!("  let builder = client.moderations().check(\"text to moderate\");");
        println!("  let response = client.moderations().create(builder).await?;");
        println!();
        println!("Running demonstration examples...\n");
        // Create a dummy client for demo purposes
        Client::builder(Config::builder().api_key("demo-key").build())?.build()
    };

    // Example 1: Basic moderation
    println!("1. Basic Moderation:");
    basic_moderation(&client);

    // Example 2: Category detection
    println!("\n2. Category Detection:");
    category_detection(&client);

    // Example 3: Custom thresholds
    println!("\n3. Custom Thresholds:");
    custom_thresholds(&client);

    // Example 4: Multi-language moderation
    println!("\n4. Multi-language Moderation:");
    multilingual_moderation(&client);

    // Example 5: Batch moderation
    println!("\n5. Batch Moderation:");
    batch_moderation(&client);

    // Example 6: Response filtering
    println!("\n6. Response Filtering:");
    response_filtering(&client).await?;

    // Example 7: Policy enforcement
    println!("\n7. Policy Enforcement:");
    policy_enforcement(&client);

    // Example 8: Moderation pipeline
    println!("\n8. Moderation Pipeline:");
    moderation_pipeline(&client).await?;

    Ok(())
}

fn basic_moderation(_client: &Client) {
    // Test various content types
    let test_inputs = vec![
        "This is a completely normal message about the weather.",
        "I really hate when people do that!",
        "Let's discuss this professional topic.",
    ];

    println!("Basic moderation demonstrates checking multiple text inputs.");
    println!("Note: To actually call the API, uncomment the async examples at the end of main().");
    println!();

    for input in test_inputs {
        println!("Input: '{}'", input);

        // You can use the client like this:
        // let builder = client.moderations().check(input);
        // let response = client.moderations().create(builder).await?;

        // For now, we'll simulate with a simple check
        let result = simulate_moderation(input);

        println!("  Flagged: {}", result.flagged);
        if result.flagged {
            println!("  Categories: {:?}", result.categories);
        }
        println!();
    }
}

fn category_detection(_client: &Client) {
    // Moderation categories
    let categories = vec![
        "harassment",
        "harassment/threatening",
        "hate",
        "hate/threatening",
        "self-harm",
        "self-harm/intent",
        "self-harm/instructions",
        "sexual",
        "sexual/minors",
        "violence",
        "violence/graphic",
    ];

    println!("Available moderation categories:");
    for category in &categories {
        println!("- {}", category);
    }

    // Test content
    let test_content = "Let's have a productive discussion about technology.";
    let result = simulate_moderation(test_content);

    println!("\nAnalyzing: '{}'", test_content);
    println!("Results:");
    for category in categories {
        let flagged = result.categories.get(category).unwrap_or(&false);
        let score = result.scores.get(category).unwrap_or(&0.0);
        println!(
            "  {:<25} Flagged: {:5} Score: {:.4}",
            category, flagged, score
        );
    }
}

fn custom_thresholds(_client: &Client) {
    // Custom thresholds for different categories
    let mut custom_thresholds = HashMap::new();
    custom_thresholds.insert("harassment".to_string(), 0.7);
    custom_thresholds.insert("violence".to_string(), 0.8);
    custom_thresholds.insert("sexual".to_string(), 0.5);

    let test_content = "This content needs moderation checking";
    let result = simulate_moderation(test_content);

    println!("Custom threshold evaluation:");
    for (category, threshold) in &custom_thresholds {
        let score = result.scores.get(category).unwrap_or(&0.0);
        let flagged = score >= threshold;

        println!(
            "Category: {:<15} Score: {:.3} Threshold: {:.1} -> {}",
            category,
            score,
            threshold,
            if flagged { "FLAGGED" } else { "OK" }
        );
    }
}

fn multilingual_moderation(_client: &Client) {
    // Test moderation in different languages
    let multilingual_tests = vec![
        ("English", "This is a test message"),
        ("Spanish", "Este es un mensaje de prueba"),
        ("French", "Ceci est un message de test"),
        ("German", "Dies ist eine Testnachricht"),
        ("Japanese", "これはテストメッセージです"),
    ];

    for (language, content) in multilingual_tests {
        println!("{}: '{}'", language, content);

        let result = simulate_moderation(content);
        println!("  Flagged: {}", result.flagged);
    }
}

fn batch_moderation(_client: &Client) {
    // Moderate multiple pieces of content efficiently
    let batch_content = [
        "First message to check",
        "Second message to check",
        "Third message to check",
        "Fourth message to check",
        "Fifth message to check",
    ];

    println!("Batch moderation of {} items:", batch_content.len());

    // In production, you might want to chunk large batches
    let chunk_size = 3;
    for (i, chunk) in batch_content.chunks(chunk_size).enumerate() {
        println!("\nChunk {} ({} items):", i + 1, chunk.len());

        for content in chunk {
            let result = simulate_moderation(content);
            println!(
                "  '{}...' -> {}",
                &content[..20.min(content.len())],
                if result.flagged { "FLAGGED" } else { "OK" }
            );
        }
    }
}

async fn response_filtering(client: &Client) -> Result<()> {
    // Filter AI responses before showing to users

    println!("Generating and moderating AI responses:");

    // Generate response
    let prompt = "Tell me about technology";
    let builder = client.chat().user(prompt).max_completion_tokens(100);
    let response = client.send_chat(builder).await?;

    if let Some(content) = response.content() {
        println!("Generated response: '{}'", content);

        // Moderate the response
        let moderation_result = simulate_moderation(content);

        if moderation_result.flagged {
            println!(
                "⚠️  Response flagged! Categories: {:?}",
                moderation_result.categories
            );
            println!("Action: Response blocked or regenerated");

            // Regenerate with more strict instructions
            let safe_builder = client
                .chat()
                .system("Provide helpful, safe, and appropriate responses only.")
                .user(prompt)
                .max_completion_tokens(100);
            let safe_response = client.send_chat(safe_builder).await?;

            if let Some(safe_content) = safe_response.content() {
                println!("Regenerated safe response: '{}'", safe_content);
            }
        } else {
            println!("✓ Response passed moderation");
        }
    }

    Ok(())
}

fn policy_enforcement(_client: &Client) {
    // Enforce content policies
    let policy = ModerationPolicy {
        thresholds: HashMap::from([
            ("harassment".to_string(), 0.5),
            ("violence".to_string(), 0.6),
            ("sexual".to_string(), 0.4),
        ]),
        auto_reject_categories: vec![
            "harassment/threatening".to_string(),
            "violence/graphic".to_string(),
        ],
        require_human_review: vec!["self-harm".to_string()],
    };

    let test_cases = vec![
        "Normal conversation about work",
        "Slightly aggressive language here",
        "Content requiring review",
    ];

    for content in test_cases {
        println!("Checking: '{}'", content);

        let result = simulate_moderation(content);
        let action = apply_policy(&result, &policy);

        match action {
            PolicyAction::Approve => println!("  ✓ Approved"),
            PolicyAction::Reject(reason) => println!("  ✗ Rejected: {}", reason),
            PolicyAction::Review(reason) => println!("  ⚠ Human review needed: {}", reason),
        }
    }
}

async fn moderation_pipeline(client: &Client) -> Result<()> {
    // Complete moderation pipeline

    type FilterFn = Box<dyn Fn(&str) -> bool + Send + Sync>;

    struct ModerationPipeline {
        pre_filters: Vec<FilterFn>,
        post_filters: Vec<FilterFn>,
    }

    let pipeline = ModerationPipeline {
        pre_filters: vec![
            Box::new(|text| text.len() < 10000), // Length check
            Box::new(|text| !text.is_empty()),   // Non-empty check
        ],
        post_filters: vec![
            Box::new(|text| !text.contains("blockedword")), // Custom word filter
        ],
    };

    println!("Running moderation pipeline:");

    let user_input = "Please help me with this technical question about Rust programming.";

    // Step 1: Pre-filters
    println!("1. Pre-filters:");
    for (i, filter) in pipeline.pre_filters.iter().enumerate() {
        if filter(user_input) {
            println!("  ✓ Pre-filter {} passed", i + 1);
        } else {
            println!("  ✗ Pre-filter {} failed", i + 1);
            return Ok(());
        }
    }

    // Step 2: API moderation
    println!("2. API moderation:");
    let moderation_result = simulate_moderation(user_input);
    if moderation_result.flagged {
        println!("  ✗ Content flagged by API");
        return Ok(());
    }
    println!("  ✓ Passed API moderation");

    // Step 3: Generate response
    println!("3. Generating response:");
    let builder = client.chat().user(user_input).max_completion_tokens(50);
    let response = client.send_chat(builder).await?;

    if let Some(content) = response.content() {
        println!("  Generated: '{}'", content);

        // Step 4: Post-filters
        println!("4. Post-filters:");
        for (i, filter) in pipeline.post_filters.iter().enumerate() {
            if filter(content) {
                println!("  ✓ Post-filter {} passed", i + 1);
            } else {
                println!("  ✗ Post-filter {} failed", i + 1);
                return Ok(());
            }
        }

        // Step 5: Response moderation
        println!("5. Response moderation:");
        let response_moderation = simulate_moderation(content);
        if response_moderation.flagged {
            println!("  ✗ Response flagged");
        } else {
            println!("  ✓ Response approved");
            println!("\nFinal output: '{}'", content);
        }
    }

    Ok(())
}

// Helper functions

fn simulate_moderation(content: &str) -> ModerationResult {
    // Simulate moderation API response
    let mut categories = HashMap::new();
    let mut scores = HashMap::new();

    // Simple heuristics for demonstration
    let harassment_score = if content.contains("hate") { 0.8 } else { 0.1 };
    let violence_score = if content.contains("aggressive") {
        0.6
    } else {
        0.05
    };

    categories.insert("harassment".to_string(), harassment_score > 0.5);
    categories.insert("violence".to_string(), violence_score > 0.5);
    categories.insert("sexual".to_string(), false);

    scores.insert("harassment".to_string(), harassment_score);
    scores.insert("violence".to_string(), violence_score);
    scores.insert("sexual".to_string(), 0.01);

    ModerationResult {
        flagged: harassment_score > 0.5 || violence_score > 0.5,
        categories,
        scores,
    }
}

enum PolicyAction {
    Approve,
    Reject(String),
    Review(String),
}

fn apply_policy(result: &ModerationResult, policy: &ModerationPolicy) -> PolicyAction {
    // Check auto-reject categories
    for category in &policy.auto_reject_categories {
        if *result.categories.get(category).unwrap_or(&false) {
            return PolicyAction::Reject(format!("Auto-rejected: {}", category));
        }
    }

    // Check human review categories
    for category in &policy.require_human_review {
        if *result.categories.get(category).unwrap_or(&false) {
            return PolicyAction::Review(format!("Review needed: {}", category));
        }
    }

    // Check custom thresholds
    for (category, threshold) in &policy.thresholds {
        if let Some(score) = result.scores.get(category) {
            if score > threshold {
                return PolicyAction::Reject(format!(
                    "{} score ({:.2}) exceeds threshold ({:.2})",
                    category, score, threshold
                ));
            }
        }
    }

    PolicyAction::Approve
}

// ========== ACTUAL API USAGE EXAMPLES ==========
// Uncomment and run these to test actual API calls
// Note: Requires OPENAI_API_KEY environment variable

/*
/// Example of actual API usage with the moderations endpoint
#[tokio::test]
async fn example_real_moderation_api() -> Result<()> {
    // Initialize client from environment
    let client = Client::from_env()?.build();

    println!("\n=== Real Moderations API Example ===\n");

    // Example 1: Simple moderation check
    println!("1. Simple moderation check:");
    let builder = client.moderations().check("Hello, this is a friendly message!");
    let response = client.moderations().create(builder).await?;

    println!("Model: {}", response.model);
    println!("Results count: {}", response.results.len());

    if let Some(result) = response.results.first() {
        println!("Flagged: {}", result.flagged);
        println!("Categories:");
        println!("  Hate: {}", result.categories.hate);
        println!("  Harassment: {}", result.categories.harassment);
        println!("  Violence: {}", result.categories.violence);
        println!("  Sexual: {}", result.categories.sexual);
        println!("  Self-harm: {}", result.categories.self_harm);

        println!("\nScores:");
        println!("  Hate: {:.6}", result.category_scores.hate);
        println!("  Harassment: {:.6}", result.category_scores.harassment);
        println!("  Violence: {:.6}", result.category_scores.violence);
    }

    // Example 2: Using specific model
    println!("\n2. With specific model:");
    let builder = client
        .moderations()
        .builder("Test content")
        .model("text-moderation-stable");

    let response = client.moderations().create(builder).await?;
    println!("Using model: {}", response.model);

    // Example 3: Using builder pattern
    println!("\n3. Using builder pattern:");
    use openai_ergonomic::builders::moderations::ModerationBuilder;

    let builder = ModerationBuilder::new("Content to moderate")
        .model("text-moderation-latest");

    let response = client.moderations().create(builder).await?;
    println!("Success! Flagged: {}", response.results[0].flagged);

    Ok(())
}
*/
