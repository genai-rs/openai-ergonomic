#![allow(clippy::uninlined_format_args)]
//! Model listing and selection example.
//!
//! This example demonstrates:
//! - Listing available models
//! - Model capabilities and properties
//! - Model selection strategies
//! - Cost optimization
//! - Performance vs quality tradeoffs
//! - Model deprecation handling
//!
//! Run with: `cargo run --example models`

use openai_ergonomic::{Client, Response, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    name: String,
    context_window: usize,
    max_output_tokens: usize,
    supports_vision: bool,
    supports_function_calling: bool,
    cost_per_1k_input: f64,
    cost_per_1k_output: f64,
    deprecated: bool,
    replacement: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Model Listing and Selection ===\n");

    // Initialize client
    let client = Client::from_env()?;

    // Example 1: List available models
    println!("1. Available Models:");
    list_models(&client)?;

    // Example 2: Model capabilities
    println!("\n2. Model Capabilities:");
    model_capabilities();

    // Example 3: Model selection by task
    println!("\n3. Model Selection by Task:");
    model_selection_by_task(&client).await?;

    // Example 4: Cost optimization
    println!("\n4. Cost Optimization:");
    cost_optimization(&client).await?;

    // Example 5: Performance testing
    println!("\n5. Performance Testing:");
    performance_testing(&client).await?;

    // Example 6: Model migration
    println!("\n6. Model Migration:");
    model_migration(&client).await?;

    // Example 7: Dynamic model selection
    println!("\n7. Dynamic Model Selection:");
    dynamic_model_selection(&client).await?;

    Ok(())
}

fn list_models(_client: &Client) -> Result<()> {
    // In a real implementation, you'd call the models API endpoint
    // For now, we'll use a hardcoded list of current models

    let models = get_model_registry();

    println!("Currently available models:");
    println!(
        "{:<20} {:>10} {:>10} {:>10}",
        "Model", "Context", "Output", "Cost/1K"
    );
    println!("{:-<50}", "");

    for (name, info) in &models {
        if !info.deprecated {
            println!(
                "{:<20} {:>10} {:>10} ${:>9.4}",
                name,
                format_tokens(info.context_window),
                format_tokens(info.max_output_tokens),
                info.cost_per_1k_input
            );
        }
    }

    println!("\nDeprecated models:");
    for (name, info) in &models {
        if info.deprecated {
            println!(
                "- {} (use {} instead)",
                name,
                info.replacement.as_deref().unwrap_or("newer model")
            );
        }
    }

    Ok(())
}

fn model_capabilities() {
    let models = get_model_registry();

    println!("Model Capabilities Matrix:");
    println!(
        "{:<20} {:>8} {:>8} {:>10} {:>10}",
        "Model", "Vision", "Tools", "Streaming", "JSON Mode"
    );
    println!("{:-<60}", "");

    for (name, info) in &models {
        if !info.deprecated {
            println!(
                "{:<20} {:>8} {:>8} {:>10} {:>10}",
                name,
                if info.supports_vision { "✓" } else { "✗" },
                if info.supports_function_calling {
                    "✓"
                } else {
                    "✗"
                },
                "✓", // All support streaming
                "✓", // All support JSON mode
            );
        }
    }
}

async fn model_selection_by_task(client: &Client) -> Result<()> {
    // Task-specific model recommendations
    let task_models = vec![
        ("Simple Q&A", "gpt-3.5-turbo", "Fast and cost-effective"),
        ("Complex reasoning", "gpt-4o", "Best reasoning capabilities"),
        ("Code generation", "gpt-4o", "Excellent code understanding"),
        ("Vision tasks", "gpt-4o", "Native vision support"),
        (
            "Quick responses",
            "gpt-4o-mini",
            "Low latency, good quality",
        ),
        (
            "Bulk processing",
            "gpt-3.5-turbo",
            "Best cost/performance ratio",
        ),
    ];

    for (task, model, reason) in task_models {
        println!("Task: {}", task);
        println!("  Recommended: {}", model);
        println!("  Reason: {}", reason);

        // Demo the model
        let builder = client
            .chat()
            .user(&format!("Say 'Hello from {}'", model))
            .max_completion_tokens(10);
        let response = client.send_chat(builder).await?;

        if let Some(content) = response.content() {
            println!("  Response: {}\n", content);
        }
    }

    Ok(())
}

async fn cost_optimization(client: &Client) -> Result<()> {
    let models = get_model_registry();
    let test_prompt = "Explain the theory of relativity in one sentence";
    let estimated_input_tokens = 15;
    let estimated_output_tokens = 50;

    println!("Cost comparison for same task:");
    println!("Prompt: '{}'\n", test_prompt);

    let mut costs = Vec::new();

    for (name, info) in &models {
        if !info.deprecated {
            let input_cost = (estimated_input_tokens as f64 / 1000.0) * info.cost_per_1k_input;
            let output_cost = (estimated_output_tokens as f64 / 1000.0) * info.cost_per_1k_output;
            let total_cost = input_cost + output_cost;

            costs.push((name.clone(), total_cost));
        }
    }

    costs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("{:<20} {:>15}", "Model", "Estimated Cost");
    println!("{:-<35}", "");
    for (model, cost) in costs {
        println!("{:<20} ${:>14.6}", model, cost);
    }

    // Demonstrate cheapest vs best
    println!("\nRunning with cheapest model (gpt-3.5-turbo):");
    let builder = client.chat().user(test_prompt);
    let cheap_response = client.send_chat(builder).await?;

    if let Some(content) = cheap_response.content() {
        println!("Response: {}", content);
    }

    Ok(())
}

async fn performance_testing(client: &Client) -> Result<()> {
    use std::time::Instant;

    let models_to_test = vec!["gpt-4o-mini", "gpt-3.5-turbo"];
    let test_prompt = "Write a haiku about programming";

    println!("Performance comparison:");
    println!("{:<20} {:>10} {:>15}", "Model", "Latency", "Tokens/sec");
    println!("{:-<45}", "");

    for model in models_to_test {
        let start = Instant::now();

        let builder = client.chat().user(test_prompt);
        let response = client.send_chat(builder).await?;

        let elapsed = start.elapsed();

        if let Some(usage) = response.usage() {
            let total_tokens = usage.total_tokens as f64;
            let tokens_per_sec = total_tokens / elapsed.as_secs_f64();

            println!("{:<20} {:>10.2?} {:>15.1}", model, elapsed, tokens_per_sec);
        }
    }

    Ok(())
}

async fn model_migration(client: &Client) -> Result<()> {
    // Handle deprecated model migration
    let deprecated_mappings = HashMap::from([
        ("text-davinci-003", "gpt-3.5-turbo"),
        ("gpt-4-32k", "gpt-4o"),
        ("gpt-4-vision-preview", "gpt-4o"),
    ]);

    let requested_model = "text-davinci-003"; // Deprecated model

    if let Some(replacement) = deprecated_mappings.get(requested_model) {
        println!(
            "Warning: {} is deprecated. Using {} instead.",
            requested_model, replacement
        );

        let builder = client.chat().user("Hello from migrated model");
        let response = client.send_chat(builder).await?;

        if let Some(content) = response.content() {
            println!("Response from {}: {}", replacement, content);
        }
    }

    Ok(())
}

async fn dynamic_model_selection(client: &Client) -> Result<()> {
    // Select model based on runtime conditions

    #[derive(Debug)]
    struct RequestContext {
        urgency: Urgency,
        complexity: Complexity,
        budget: Budget,
        needs_vision: bool,
    }

    #[derive(Debug)]
    enum Urgency {
        Low,
        Medium,
        High,
    }

    #[derive(Debug)]
    enum Complexity {
        Simple,
        Moderate,
        Complex,
    }

    #[derive(Debug)]
    enum Budget {
        Tight,
        Normal,
        Flexible,
    }

    fn select_model(ctx: &RequestContext) -> &'static str {
        match (&ctx.urgency, &ctx.complexity, &ctx.budget) {
            // High urgency + simple = fast cheap model
            (Urgency::High, Complexity::Simple, _) => "gpt-3.5-turbo",

            // Complex + flexible budget = best model
            (_, Complexity::Complex, Budget::Flexible) => "gpt-4o",

            // Tight budget = cheapest
            (_, _, Budget::Tight) => "gpt-3.5-turbo",

            // Vision required
            _ if ctx.needs_vision => "gpt-4o",

            // Default balanced choice
            _ => "gpt-4o-mini",
        }
    }

    // Example contexts
    let contexts = vec![
        RequestContext {
            urgency: Urgency::High,
            complexity: Complexity::Simple,
            budget: Budget::Tight,
            needs_vision: false,
        },
        RequestContext {
            urgency: Urgency::Low,
            complexity: Complexity::Complex,
            budget: Budget::Flexible,
            needs_vision: false,
        },
        RequestContext {
            urgency: Urgency::Medium,
            complexity: Complexity::Moderate,
            budget: Budget::Normal,
            needs_vision: true,
        },
    ];

    for (i, ctx) in contexts.iter().enumerate() {
        let model = select_model(&ctx);
        println!("Context {}: {:?}", i + 1, ctx);
        println!("  Selected model: {}", model);

        let builder = client
            .chat()
            .user(&format!("Hello from dynamically selected {}", model))
            .max_completion_tokens(20);
        let response = client.send_chat(builder).await?;

        if let Some(content) = response.content() {
            println!("  Response: {}\n", content);
        }
    }

    Ok(())
}

fn get_model_registry() -> HashMap<String, ModelInfo> {
    HashMap::from([
        (
            "gpt-4o".to_string(),
            ModelInfo {
                name: "gpt-4o".to_string(),
                context_window: 128_000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_function_calling: true,
                cost_per_1k_input: 0.0025,
                cost_per_1k_output: 0.01,
                deprecated: false,
                replacement: None,
            },
        ),
        (
            "gpt-4o-mini".to_string(),
            ModelInfo {
                name: "gpt-4o-mini".to_string(),
                context_window: 128_000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_function_calling: true,
                cost_per_1k_input: 0.00015,
                cost_per_1k_output: 0.0006,
                deprecated: false,
                replacement: None,
            },
        ),
        (
            "gpt-3.5-turbo".to_string(),
            ModelInfo {
                name: "gpt-3.5-turbo".to_string(),
                context_window: 16385,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_function_calling: true,
                cost_per_1k_input: 0.0003,
                cost_per_1k_output: 0.0006,
                deprecated: false,
                replacement: None,
            },
        ),
        (
            "text-davinci-003".to_string(),
            ModelInfo {
                name: "text-davinci-003".to_string(),
                context_window: 4097,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_function_calling: false,
                cost_per_1k_input: 0.02,
                cost_per_1k_output: 0.02,
                deprecated: true,
                replacement: Some("gpt-3.5-turbo".to_string()),
            },
        ),
    ])
}

fn format_tokens(tokens: usize) -> String {
    if tokens >= 1000 {
        format!("{}K", tokens / 1000)
    } else {
        tokens.to_string()
    }
}
