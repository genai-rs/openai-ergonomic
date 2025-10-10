//! Comprehensive threads example demonstrating conversation management with OpenAI Assistants.
//!
//! This example showcases the OpenAI Threads API, which provides a way to manage
//! conversations between users and assistants.
//!
//! ## Features Demonstrated
//!
//! - **Thread Creation**: Create conversation threads
//! - **Thread Configuration**: Set up threads with metadata
//! - **Message Management**: Add messages to threads
//! - **Thread Persistence**: Maintain conversation state
//! - **Error Handling**: Robust error handling
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
//! cargo run --example threads
//! ```
//!
//! ## Overview
//!
//! Threads allow you to maintain conversation history and context across
//! multiple messages and runs with OpenAI assistants.

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(unused_variables)]
#![allow(missing_docs)]
#![allow(dead_code)]

use openai_ergonomic::{builders::threads::ThreadRequestBuilder, Client};

/// Thread metadata for demonstration
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    pub id: String,
    pub created_at: i64,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ThreadInfo {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn display(&self) {
        println!("  Thread ID: {}", self.id);
        println!("  Created At: {}", self.created_at);
        if !self.metadata.is_empty() {
            println!("  Metadata:");
            for (key, value) in &self.metadata {
                println!("    {}: {}", key, value);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" OpenAI Ergonomic - Comprehensive Threads Example\n");

    // Initialize client from environment variables
    println!(" Initializing OpenAI client...");
    let client = match Client::from_env() {
        Ok(c) => {
            println!(" Client initialized successfully\n");
            c.build()
        }
        Err(e) => {
            eprintln!(" Failed to initialize client: {}", e);
            eprintln!(" Make sure OPENAI_API_KEY is set");
            return Ok(());
        }
    };

    // Example 1: Create a simple thread
    println!("");
    println!(" Example 1: Create Simple Thread");
    println!("\n");

    println!("Creating new thread...");

    let builder = ThreadRequestBuilder::new();

    println!("\n Note: This would create a real thread with your API key.");
    println!("   Commented out to avoid accidental API calls.\n");

    // Uncomment to actually create thread:
    // match client.threads().create(builder).await {
    //     Ok(thread) => {
    //         println!(" Thread created successfully!");
    //         println!("  Thread ID: {}", thread.id);
    //         println!("  Created At: {}", thread.created_at);
    //     }
    //     Err(e) => {
    //         eprintln!(" Failed to create thread: {}", e);
    //     }
    // }

    // Simulate thread creation for demonstration
    let demo_thread = ThreadInfo::new("thread_demo123");
    println!(" Demo Thread Created:");
    demo_thread.display();

    // Example 2: Create thread with metadata
    println!("\n");
    println!(" Example 2: Create Thread with Metadata");
    println!("\n");

    println!("Creating thread with metadata...");

    let builder_with_metadata = ThreadRequestBuilder::new()
        .metadata("user_id", "user_12345")
        .metadata("session_id", "session_abc")
        .metadata("context", "customer_support");

    println!("  Metadata:");
    println!("    user_id: user_12345");
    println!("    session_id: session_abc");
    println!("    context: customer_support");

    // Uncomment to actually create thread:
    // match client.threads().create(builder_with_metadata).await {
    //     Ok(thread) => {
    //         println!("\n Thread with metadata created!");
    //         println!("  Thread ID: {}", thread.id);
    //     }
    //     Err(e) => {
    //         eprintln!(" Failed to create thread: {}", e);
    //     }
    // }

    let demo_thread_with_meta = ThreadInfo::new("thread_demo456")
        .with_metadata("user_id", "user_12345")
        .with_metadata("session_id", "session_abc")
        .with_metadata("context", "customer_support");

    println!("\n Demo Thread Created:");
    demo_thread_with_meta.display();

    // Example 3: Create thread with initial messages
    println!("\n");
    println!(" Example 3: Create Thread with Initial Messages");
    println!("\n");

    println!("Creating thread with initial messages...");

    // Note: The ThreadRequestBuilder supports adding messages during creation
    let builder_with_messages =
        ThreadRequestBuilder::new().metadata("conversation_type", "onboarding");

    println!("  Initial message: 'Hello, I need help getting started'");

    // In a real implementation, you would add messages like:
    // .message("user", "Hello, I need help getting started")

    println!("\n Note: Messages can be added during thread creation or afterwards");

    // Example 4: Thread lifecycle management
    println!("\n");
    println!(" Example 4: Thread Lifecycle");
    println!("\n");

    println!("Typical thread lifecycle:");
    println!("  1. Create thread (ThreadRequestBuilder)");
    println!("  2. Add messages to thread");
    println!("  3. Create runs to process messages");
    println!("  4. Retrieve assistant responses");
    println!("  5. Continue conversation");
    println!("  6. Thread persists until deleted");

    // Example 5: Thread use cases
    println!("\n");
    println!(" Example 5: Common Thread Use Cases");
    println!("\n");

    println!(" Customer Support:");
    println!("  • Create thread per customer session");
    println!("  • Metadata: customer_id, ticket_id");
    println!("  • Maintain conversation history");
    println!("  • Allow agents to review past interactions\n");

    println!(" Chatbot Conversations:");
    println!("  • One thread per user session");
    println!("  • Metadata: user_id, session_start");
    println!("  • Preserve context across messages");
    println!("  • Support multi-turn conversations\n");

    println!(" Document Q&A:");
    println!("  • Thread per document discussion");
    println!("  • Metadata: document_id, user_id");
    println!("  • Allow follow-up questions");
    println!("  • Maintain question history\n");

    println!(" Tutoring/Education:");
    println!("  • Thread per learning session");
    println!("  • Metadata: student_id, subject, lesson");
    println!("  • Track learning progression");
    println!("  • Review past explanations");

    // Example 6: Thread metadata strategies
    println!("\n");
    println!(" Example 6: Metadata Strategies");
    println!("\n");

    println!("Recommended metadata fields:\n");

    println!(" Identification:");
    println!("  user_id: Identify the user");
    println!("  session_id: Track specific sessions");
    println!("  organization_id: Multi-tenant applications\n");

    println!(" Classification:");
    println!("  category: customer_support, sales, etc.");
    println!("  priority: low, medium, high, urgent");
    println!("  language: en, es, fr, etc.\n");

    println!("⏰ Temporal:");
    println!("  session_start: When conversation began");
    println!("  last_active: Last interaction time");
    println!("  expires_at: When to auto-cleanup\n");

    println!(" Business Context:");
    println!("  product_id: Related product");
    println!("  ticket_id: Support ticket number");
    println!("  campaign_id: Marketing campaign");

    // Example 7: Thread best practices
    println!("\n");
    println!(" Example 7: Best Practices");
    println!("\n");

    println!(" Do:");
    println!("  • Create one thread per conversation");
    println!("  • Add meaningful metadata for filtering");
    println!("  • Reuse threads for ongoing conversations");
    println!("  • Clean up old/expired threads");
    println!("  • Use metadata for analytics\n");

    println!(" Don't:");
    println!("  • Create new threads for each message");
    println!("  • Store sensitive data in metadata");
    println!("  • Let threads accumulate indefinitely");
    println!("  • Use threads for one-off requests");
    println!("  • Mix different conversations in one thread");

    // Summary
    println!("\n");
    println!(" Summary");
    println!("\n");

    println!(" Threads API examples completed!");
    println!("\n Key Takeaways:");
    println!("  • Threads maintain conversation state");
    println!("  • Metadata enables organization and filtering");
    println!("  • One thread per conversation is recommended");
    println!("  • Threads persist until explicitly deleted");
    println!("  • Perfect for multi-turn conversations");

    println!("\n Integration Pattern:");
    println!("  1. Create thread at conversation start");
    println!("  2. Store thread ID in your database");
    println!("  3. Add messages as user interacts");
    println!("  4. Create runs to get assistant responses");
    println!("  5. Retrieve messages to display conversation");
    println!("  6. Reuse thread for entire conversation");

    println!("\n Related APIs:");
    println!("  • Messages API: Add/retrieve messages in threads");
    println!("  • Runs API: Process messages with assistants");
    println!("  • Assistants API: Create AI assistants");
    println!("  • Vector Stores: Add knowledge to assistants");

    println!("\n Example completed successfully!");

    Ok(())
}
