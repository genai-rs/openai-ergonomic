//! Batch API builders.
//!
//! This module provides ergonomic builders for OpenAI Batch API operations,
//! which allow you to send asynchronous groups of requests with 24-hour turnaround
//! and 50% cost reduction compared to synchronous API calls.

use std::collections::HashMap;

/// Builder for creating batch jobs.
///
/// Batch jobs allow you to process multiple requests asynchronously at a lower cost
/// with longer processing time (up to 24 hours).
#[derive(Debug, Clone)]
pub struct BatchJobBuilder {
    input_file_id: String,
    endpoint: BatchEndpoint,
    completion_window: BatchCompletionWindow,
    metadata: HashMap<String, String>,
}

/// Supported endpoints for batch processing.
#[derive(Debug, Clone)]
pub enum BatchEndpoint {
    /// Chat completions endpoint
    ChatCompletions,
    /// Embeddings endpoint
    Embeddings,
    /// Completions endpoint (legacy)
    Completions,
}

/// Completion window for batch jobs.
#[derive(Debug, Clone)]
pub enum BatchCompletionWindow {
    /// Complete within 24 hours
    Hours24,
}

impl BatchJobBuilder {
    /// Create a new batch job builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::batch::{BatchJobBuilder, BatchEndpoint};
    ///
    /// let builder = BatchJobBuilder::new("file-batch-input", BatchEndpoint::ChatCompletions);
    /// ```
    #[must_use]
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        Self {
            input_file_id: input_file_id.into(),
            endpoint,
            completion_window: BatchCompletionWindow::Hours24,
            metadata: HashMap::new(),
        }
    }

    /// Set the completion window for the batch job.
    #[must_use]
    pub fn completion_window(mut self, window: BatchCompletionWindow) -> Self {
        self.completion_window = window;
        self
    }

    /// Add metadata to the batch job.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the input file ID.
    #[must_use]
    pub fn input_file_id(&self) -> &str {
        &self.input_file_id
    }

    /// Get the endpoint for this batch job.
    #[must_use]
    pub fn endpoint(&self) -> &BatchEndpoint {
        &self.endpoint
    }

    /// Get the completion window.
    #[must_use]
    pub fn completion_window_ref(&self) -> &BatchCompletionWindow {
        &self.completion_window
    }

    /// Get the metadata.
    #[must_use]
    pub fn metadata_ref(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Check if metadata is empty.
    #[must_use]
    pub fn has_metadata(&self) -> bool {
        !self.metadata.is_empty()
    }
}

/// Builder for listing batch jobs.
#[derive(Debug, Clone, Default)]
pub struct BatchJobListBuilder {
    after: Option<String>,
    limit: Option<i32>,
}

impl BatchJobListBuilder {
    /// Create a new batch job list builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cursor for pagination.
    #[must_use]
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    /// Set the maximum number of jobs to return.
    #[must_use]
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Get the pagination cursor.
    #[must_use]
    pub fn after_ref(&self) -> Option<&str> {
        self.after.as_deref()
    }

    /// Get the limit.
    #[must_use]
    pub fn limit_ref(&self) -> Option<i32> {
        self.limit
    }
}

/// Builder for retrieving batch job details.
#[derive(Debug, Clone)]
pub struct BatchJobRetrievalBuilder {
    batch_id: String,
}

impl BatchJobRetrievalBuilder {
    /// Create a new batch job retrieval builder.
    #[must_use]
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
        }
    }

    /// Get the batch ID.
    #[must_use]
    pub fn batch_id(&self) -> &str {
        &self.batch_id
    }
}

/// Builder for cancelling batch jobs.
#[derive(Debug, Clone)]
pub struct BatchJobCancelBuilder {
    batch_id: String,
}

impl BatchJobCancelBuilder {
    /// Create a new batch job cancel builder.
    #[must_use]
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
        }
    }

    /// Get the batch ID.
    #[must_use]
    pub fn batch_id(&self) -> &str {
        &self.batch_id
    }
}

/// Helper function to create a chat completions batch job.
#[must_use]
pub fn batch_chat_completions(input_file_id: impl Into<String>) -> BatchJobBuilder {
    BatchJobBuilder::new(input_file_id, BatchEndpoint::ChatCompletions)
}

/// Helper function to create an embeddings batch job.
#[must_use]
pub fn batch_embeddings(input_file_id: impl Into<String>) -> BatchJobBuilder {
    BatchJobBuilder::new(input_file_id, BatchEndpoint::Embeddings)
}

/// Helper function to create a completions batch job.
#[must_use]
pub fn batch_completions(input_file_id: impl Into<String>) -> BatchJobBuilder {
    BatchJobBuilder::new(input_file_id, BatchEndpoint::Completions)
}

/// Helper function to create a batch job with metadata.
#[must_use]
pub fn batch_job_with_metadata(
    input_file_id: impl Into<String>,
    endpoint: BatchEndpoint,
    metadata: HashMap<String, String>,
) -> BatchJobBuilder {
    let mut builder = BatchJobBuilder::new(input_file_id, endpoint);
    for (key, value) in metadata {
        builder = builder.metadata(key, value);
    }
    builder
}

/// Helper function to list batch jobs.
#[must_use]
pub fn list_batch_jobs() -> BatchJobListBuilder {
    BatchJobListBuilder::new()
}

/// Helper function to retrieve a specific batch job.
#[must_use]
pub fn get_batch_job(batch_id: impl Into<String>) -> BatchJobRetrievalBuilder {
    BatchJobRetrievalBuilder::new(batch_id)
}

/// Helper function to cancel a batch job.
#[must_use]
pub fn cancel_batch_job(batch_id: impl Into<String>) -> BatchJobCancelBuilder {
    BatchJobCancelBuilder::new(batch_id)
}

impl std::fmt::Display for BatchEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchEndpoint::ChatCompletions => write!(f, "/v1/chat/completions"),
            BatchEndpoint::Embeddings => write!(f, "/v1/embeddings"),
            BatchEndpoint::Completions => write!(f, "/v1/completions"),
        }
    }
}

impl std::fmt::Display for BatchCompletionWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchCompletionWindow::Hours24 => write!(f, "24h"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_job_builder_new() {
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
        assert_eq!(
            builder.metadata_ref().get("version"),
            Some(&"v1".to_string())
        );
    }

    #[test]
    fn test_batch_job_builder_completion_window() {
        let builder = BatchJobBuilder::new("file-input", BatchEndpoint::ChatCompletions)
            .completion_window(BatchCompletionWindow::Hours24);

        match builder.completion_window_ref() {
            BatchCompletionWindow::Hours24 => {}
        }
    }

    #[test]
    fn test_batch_job_list_builder() {
        let builder = BatchJobListBuilder::new().after("batch-123").limit(10);

        assert_eq!(builder.after_ref(), Some("batch-123"));
        assert_eq!(builder.limit_ref(), Some(10));
    }

    #[test]
    fn test_batch_job_retrieval_builder() {
        let builder = BatchJobRetrievalBuilder::new("batch-456");
        assert_eq!(builder.batch_id(), "batch-456");
    }

    #[test]
    fn test_batch_job_cancel_builder() {
        let builder = BatchJobCancelBuilder::new("batch-789");
        assert_eq!(builder.batch_id(), "batch-789");
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
    fn test_batch_job_with_metadata_helper() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let builder =
            batch_job_with_metadata("file-input", BatchEndpoint::ChatCompletions, metadata);

        assert!(builder.has_metadata());
        assert_eq!(builder.metadata_ref().len(), 2);
    }

    #[test]
    fn test_list_batch_jobs_helper() {
        let builder = list_batch_jobs();
        assert!(builder.after_ref().is_none());
        assert!(builder.limit_ref().is_none());
    }

    #[test]
    fn test_get_batch_job_helper() {
        let builder = get_batch_job("batch-123");
        assert_eq!(builder.batch_id(), "batch-123");
    }

    #[test]
    fn test_cancel_batch_job_helper() {
        let builder = cancel_batch_job("batch-456");
        assert_eq!(builder.batch_id(), "batch-456");
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
    fn test_batch_completion_window_display() {
        assert_eq!(BatchCompletionWindow::Hours24.to_string(), "24h");
    }

    #[test]
    fn test_batch_job_list_builder_default() {
        let builder = BatchJobListBuilder::default();
        assert!(builder.after_ref().is_none());
        assert!(builder.limit_ref().is_none());
    }
}
