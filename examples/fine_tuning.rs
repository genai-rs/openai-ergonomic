//! Comprehensive fine-tuning example demonstrating model customization with OpenAI.
//!
//! This example showcases the OpenAI Fine-tuning API, including:
//! - Creating fine-tuning jobs with training data
//! - Listing fine-tuning jobs with pagination
//! - Retrieving job status and monitoring progress
//! - Listing events for jobs to track training progress
//! - Listing checkpoints for jobs
//! - Cancelling running jobs
//!
//! ## Features Demonstrated
//!
//! - **Job Creation**: Create fine-tuning jobs with various configurations
//! - **Job Listing**: List all fine-tuning jobs with filtering
//! - **Job Monitoring**: Track job progress and view events
//! - **Checkpoint Management**: View and manage training checkpoints
//! - **Job Cancellation**: Cancel running jobs when needed
//! - **Error Handling**: Robust error handling for various scenarios
//!
//! ## Prerequisites
//!
//! Set your OpenAI API key:
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example fine_tuning
//! ```
//!
//! Note: This example demonstrates the API structure. Fine-tuning requires
//! properly formatted training data files uploaded to OpenAI.

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(dead_code)]

use openai_ergonomic::{builders::fine_tuning::FineTuningJobBuilder, Client};

/// Fine-tuning job metadata for demonstration
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: String,
    pub model: String,
    pub status: String,
    pub training_file: String,
    pub created_at: i64,
}

impl JobInfo {
    pub fn new(
        id: impl Into<String>,
        model: impl Into<String>,
        status: impl Into<String>,
        training_file: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            status: status.into(),
            training_file: training_file.into(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    pub fn display(&self) {
        println!("  ID: {}", self.id);
        println!("  Model: {}", self.model);
        println!("  Status: {}", self.status);
        println!("  Training File: {}", self.training_file);
        println!("  Created At: {}", self.created_at);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 OpenAI Ergonomic - Comprehensive Fine-tuning Example\n");

    // Initialize client from environment variables
    println!("📝 Initializing OpenAI client...");
    let client = match Client::from_env() {
        Ok(c) => {
            println!("✅ Client initialized successfully\n");
            c
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {}", e);
            eprintln!("💡 Make sure OPENAI_API_KEY is set");
            return Ok(());
        }
    };

    // Example 1: Create a fine-tuning job
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 1: Create Fine-tuning Job");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Note: You need to upload a training file first
    // For demonstration purposes, we'll use a placeholder file ID
    let training_file_id = "file-training-data";

    println!("Creating fine-tuning job...");
    println!("  Base Model: gpt-3.5-turbo");
    println!("  Training File: {}", training_file_id);
    println!("  Suffix: my-custom-model");

    let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", training_file_id)
        .suffix("my-custom-model")
        .epochs(3);

    println!("\n💡 Note: This would create a real fine-tuning job with your API key.");
    println!("   Commented out to avoid accidental charges.\n");

    // Uncomment to actually create the job:
    // match client.fine_tuning().create_job(builder).await {
    //     Ok(job) => {
    //         println!("✅ Fine-tuning job created successfully!");
    //         println!("  Job ID: {}", job.id);
    //         println!("  Status: {}", job.status);
    //         println!("  Model: {}", job.model);
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to create fine-tuning job: {}", e);
    //     }
    // }

    // Simulate job creation for demonstration
    let demo_job = JobInfo::new(
        "ftjob-demo123",
        "gpt-3.5-turbo",
        "validating",
        training_file_id,
    );
    println!("📊 Demo Job Created:");
    demo_job.display();

    // Example 2: List fine-tuning jobs
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 2: List Fine-tuning Jobs");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Listing fine-tuning jobs (limit: 5)...\n");

    // Uncomment to actually list jobs:
    // match client.fine_tuning().list_jobs(None, Some(5)).await {
    //     Ok(response) => {
    //         println!("✅ Found {} fine-tuning jobs", response.data.len());
    //         for (i, job) in response.data.iter().enumerate() {
    //             println!("\n📝 Job {}:", i + 1);
    //             println!("  ID: {}", job.id);
    //             println!("  Model: {}", job.model);
    //             println!("  Status: {}", job.status);
    //             println!("  Created At: {}", job.created_at);
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to list fine-tuning jobs: {}", e);
    //     }
    // }

    println!("💡 Demo: Would list your fine-tuning jobs here");

    // Example 3: Get specific fine-tuning job
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 3: Get Fine-tuning Job Details");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let job_id = "ftjob-demo123";
    println!("Retrieving job: {}\n", job_id);

    // Uncomment to actually get job:
    // match client.fine_tuning().get_job(job_id).await {
    //     Ok(job) => {
    //         println!("✅ Job retrieved successfully!");
    //         println!("  ID: {}", job.id);
    //         println!("  Model: {}", job.model);
    //         println!("  Status: {}", job.status);
    //         println!("  Created At: {}", job.created_at);
    //         if let Some(finished_at) = job.finished_at {
    //             println!("  Finished At: {}", finished_at);
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to get fine-tuning job: {}", e);
    //     }
    // }

    println!("💡 Demo: Would show detailed job information");

    // Example 4: List job events
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 4: List Fine-tuning Job Events");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Listing events for job: {}\n", job_id);

    // Uncomment to actually list events:
    // match client.fine_tuning().list_events(job_id, None, Some(10)).await {
    //     Ok(response) => {
    //         println!("✅ Found {} events", response.data.len());
    //         for (i, event) in response.data.iter().enumerate() {
    //             println!("\n📋 Event {}:", i + 1);
    //             println!("  Message: {}", event.message);
    //             println!("  Created At: {}", event.created_at);
    //             if let Some(level) = &event.level {
    //                 println!("  Level: {}", level);
    //             }
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to list events: {}", e);
    //     }
    // }

    println!("💡 Demo: Would show training events like:");
    println!("  - Job started");
    println!("  - Training step 1/100 complete");
    println!("  - Validation loss: 0.452");
    println!("  - Training complete");

    // Example 5: List job checkpoints
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 5: List Fine-tuning Job Checkpoints");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Listing checkpoints for job: {}\n", job_id);

    // Uncomment to actually list checkpoints:
    // match client.fine_tuning().list_checkpoints(job_id, None, Some(5)).await {
    //     Ok(response) => {
    //         println!("✅ Found {} checkpoints", response.data.len());
    //         for (i, checkpoint) in response.data.iter().enumerate() {
    //             println!("\n💾 Checkpoint {}:", i + 1);
    //             println!("  ID: {}", checkpoint.id);
    //             println!("  Created At: {}", checkpoint.created_at);
    //             println!("  Step Number: {}", checkpoint.step_number);
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to list checkpoints: {}", e);
    //     }
    // }

    println!("💡 Demo: Would show model checkpoints from training");

    // Example 6: Cancel fine-tuning job
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 6: Cancel Fine-tuning Job");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Cancelling job: {}\n", job_id);

    // Uncomment to actually cancel job:
    // match client.fine_tuning().cancel_job(job_id).await {
    //     Ok(job) => {
    //         println!("✅ Job cancelled successfully!");
    //         println!("  Job ID: {}", job.id);
    //         println!("  Status: {}", job.status);
    //     }
    //     Err(e) => {
    //         eprintln!("❌ Failed to cancel job: {}", e);
    //     }
    // }

    println!("💡 Demo: Would cancel the running fine-tuning job");

    // Example 7: Create job with validation file
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 7: Create Job with Validation File");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let validation_file_id = "file-validation-data";

    println!("Creating fine-tuning job with validation...");
    println!("  Base Model: gpt-3.5-turbo");
    println!("  Training File: {}", training_file_id);
    println!("  Validation File: {}", validation_file_id);
    println!("  Epochs: 5");
    println!("  Learning Rate Multiplier: 0.1");

    let builder_with_validation = FineTuningJobBuilder::new("gpt-3.5-turbo", training_file_id)
        .validation_file(validation_file_id)
        .epochs(5)
        .learning_rate_multiplier(0.1);

    println!("\n💡 Note: Validation files help monitor overfitting during training");

    // Example 8: Create job with Weights & Biases integration
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 Example 8: Create Job with W&B Integration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Creating fine-tuning job with W&B...");
    println!("  Base Model: gpt-3.5-turbo");
    println!("  Training File: {}", training_file_id);
    println!("  W&B Project: my-finetuning-project");

    let builder_with_wandb = FineTuningJobBuilder::new("gpt-3.5-turbo", training_file_id)
        .with_wandb("my-finetuning-project");

    println!("\n💡 Note: W&B integration provides detailed training metrics visualization");

    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("✅ Fine-tuning API examples completed!");
    println!("\n📚 Key Takeaways:");
    println!("  • Fine-tuning allows customizing models for specific tasks");
    println!("  • Jobs can be created with various hyperparameters");
    println!("  • Progress can be monitored through events and checkpoints");
    println!("  • Validation files help prevent overfitting");
    println!("  • Integrations like W&B provide detailed metrics");
    println!("  • Jobs can be cancelled if needed");

    println!("\n💡 Next Steps:");
    println!("  1. Prepare your training data in JSONL format");
    println!("  2. Upload training data using the Files API");
    println!("  3. Create a fine-tuning job with appropriate parameters");
    println!("  4. Monitor progress through events");
    println!("  5. Use the fine-tuned model in your applications");

    println!("\n🎉 Example completed successfully!");

    Ok(())
}
