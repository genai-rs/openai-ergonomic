//! Builder pattern implementations for `OpenAI` API requests.
//!
//! This module provides ergonomic builder APIs that wrap the base `OpenAI` client
//! types with a fluent interface. The builders follow the `bon` crate pattern
//! and provide sensible defaults for common use cases.
//!
//! # Example
//!
//! ```rust
//! # use openai_ergonomic::builders::*;
//! // TODO: Add example once builders are implemented
//! ```

pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod moderations;
pub mod responses;
pub mod threads;
pub mod uploads;
pub mod vector_stores;

// Re-export common builder types for convenience
// NOTE: Re-exports will be enabled as modules are implemented
// pub use assistants::*;
// pub use audio::*;
// pub use batch::*;
pub use chat::*; // Has implementation
                 // pub use embeddings::*;
                 // pub use files::*;
                 // pub use fine_tuning::*;
                 // pub use images::*;
                 // pub use moderations::*;
                 // pub use responses::*;
                 // pub use threads::*;
                 // pub use uploads::*;
                 // pub use vector_stores::*;

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
