//! Builder pattern implementations for `OpenAI` API requests.
//!
//! This module provides ergonomic builder APIs that wrap the base `OpenAI` client
//! types with a fluent interface. The builders follow the `bon` crate pattern
//! and provide sensible defaults for common use cases.
//!
//! # Example
//!
//! ```rust,ignore
//! # use openai_ergonomic::builders::*;
//! // TODO: Add example once builders are implemented
//! ```

pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod models;
pub mod moderations;
pub mod responses;
pub mod threads;
pub mod uploads;
pub mod usage;
pub mod vector_stores;

// Re-export common builder types for convenience
pub use assistants::{
    assistant_with_code_interpreter, assistant_with_file_search, assistant_with_instructions,
    assistant_with_tools, simple_assistant, simple_run, simple_thread, streaming_run,
    temperature_run, tool_code_interpreter, tool_file_search, AssistantBuilder, AssistantTool,
    RunBuilder, ThreadBuilder,
};
pub use audio::*;
pub use batch::*;
pub use chat::{
    image_base64_part, image_base64_part_with_detail, image_url_part, image_url_part_with_detail,
    system_user, text_part, tool_web_search, user_message, ChatCompletionBuilder,
};
pub use completions::*;
pub use embeddings::*;
pub use files::*;
pub use fine_tuning::*;
pub use images::*;
pub use models::*;
pub use moderations::*;
pub use responses::*;
pub use threads::*;
pub use uploads::*;
pub use usage::*;
pub use vector_stores::*;

/// Common trait for all builders to provide consistent APIs.
pub trait Builder<T> {
    /// Build the final request type.
    fn build(self) -> crate::Result<T>;
}

/// Helper trait for builders that can be sent to the `OpenAI` API.
pub trait Sendable<R> {
    /// Send the request to the `OpenAI` API and return the response.
    async fn send(self) -> crate::Result<R>;
}
