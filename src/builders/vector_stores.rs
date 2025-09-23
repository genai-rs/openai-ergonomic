//! Vector Stores API builders.
//!
//! This module provides ergonomic builders for OpenAI Vector Stores API operations,
//! including creating and managing vector stores for RAG (Retrieval-Augmented Generation) use cases.
//!
//! Vector stores are used to store and search through documents that can be used
//! by assistants with file search capabilities.

use std::collections::HashMap;

/// Builder for creating a new vector store.
///
/// Vector stores are collections of files that can be searched through
/// using semantic similarity. They're commonly used for RAG applications.
#[derive(Debug, Clone)]
pub struct VectorStoreBuilder {
    name: Option<String>,
    file_ids: Vec<String>,
    expires_after: Option<VectorStoreExpirationPolicy>,
    metadata: HashMap<String, String>,
}

/// Expiration policy for vector stores.
#[derive(Debug, Clone)]
pub struct VectorStoreExpirationPolicy {
    /// Number of days after which the vector store expires.
    pub days: i32,
}

impl VectorStoreBuilder {
    /// Create a new vector store builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;
    ///
    /// let builder = VectorStoreBuilder::new()
    ///     .name("My Knowledge Base")
    ///     .file_ids(vec!["file-123".to_string(), "file-456".to_string()]);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: None,
            file_ids: Vec::new(),
            expires_after: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the vector store's name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the file IDs to include in the vector store.
    #[must_use]
    pub fn file_ids(mut self, file_ids: Vec<String>) -> Self {
        self.file_ids = file_ids;
        self
    }

    /// Add a single file ID to the vector store.
    #[must_use]
    pub fn add_file(mut self, file_id: impl Into<String>) -> Self {
        self.file_ids.push(file_id.into());
        self
    }

    /// Set expiration policy for the vector store.
    #[must_use]
    pub fn expires_after_days(mut self, days: i32) -> Self {
        self.expires_after = Some(VectorStoreExpirationPolicy { days });
        self
    }

    /// Add metadata to the vector store.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the name of this vector store.
    #[must_use]
    pub fn name_ref(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the file IDs for this vector store.
    #[must_use]
    pub fn file_ids_ref(&self) -> &[String] {
        &self.file_ids
    }

    /// Get the expiration policy for this vector store.
    #[must_use]
    pub fn expires_after_ref(&self) -> Option<&VectorStoreExpirationPolicy> {
        self.expires_after.as_ref()
    }

    /// Get the metadata for this vector store.
    #[must_use]
    pub fn metadata_ref(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Check if the vector store has any files.
    #[must_use]
    pub fn has_files(&self) -> bool {
        !self.file_ids.is_empty()
    }

    /// Get the number of files in the vector store.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.file_ids.len()
    }
}

impl Default for VectorStoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for vector store file operations.
#[derive(Debug, Clone)]
pub struct VectorStoreFileBuilder {
    vector_store_id: String,
    file_id: String,
}

impl VectorStoreFileBuilder {
    /// Create a new vector store file builder.
    #[must_use]
    pub fn new(vector_store_id: impl Into<String>, file_id: impl Into<String>) -> Self {
        Self {
            vector_store_id: vector_store_id.into(),
            file_id: file_id.into(),
        }
    }

    /// Get the vector store ID.
    #[must_use]
    pub fn vector_store_id(&self) -> &str {
        &self.vector_store_id
    }

    /// Get the file ID.
    #[must_use]
    pub fn file_id(&self) -> &str {
        &self.file_id
    }
}

/// Builder for searching through vector stores.
#[derive(Debug, Clone)]
pub struct VectorStoreSearchBuilder {
    vector_store_id: String,
    query: String,
    limit: Option<i32>,
    filter: HashMap<String, String>,
}

impl VectorStoreSearchBuilder {
    /// Create a new vector store search builder.
    #[must_use]
    pub fn new(vector_store_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            vector_store_id: vector_store_id.into(),
            query: query.into(),
            limit: None,
            filter: HashMap::new(),
        }
    }

    /// Set the maximum number of results to return.
    #[must_use]
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a filter to the search.
    #[must_use]
    pub fn filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filter.insert(key.into(), value.into());
        self
    }

    /// Get the vector store ID for this search.
    #[must_use]
    pub fn vector_store_id(&self) -> &str {
        &self.vector_store_id
    }

    /// Get the search query.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Get the search limit.
    #[must_use]
    pub fn limit_ref(&self) -> Option<i32> {
        self.limit
    }

    /// Get the search filters.
    #[must_use]
    pub fn filter_ref(&self) -> &HashMap<String, String> {
        &self.filter
    }
}

/// Helper function to create a simple vector store with a name.
#[must_use]
pub fn simple_vector_store(name: impl Into<String>) -> VectorStoreBuilder {
    VectorStoreBuilder::new().name(name)
}

/// Helper function to create a vector store with files.
#[must_use]
pub fn vector_store_with_files(
    name: impl Into<String>,
    file_ids: Vec<String>,
) -> VectorStoreBuilder {
    VectorStoreBuilder::new().name(name).file_ids(file_ids)
}

/// Helper function to create a temporary vector store that expires after a specified number of days.
#[must_use]
pub fn temporary_vector_store(
    name: impl Into<String>,
    expires_after_days: i32,
) -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name(name)
        .expires_after_days(expires_after_days)
}

/// Helper function to add a file to a vector store.
#[must_use]
pub fn add_file_to_vector_store(
    vector_store_id: impl Into<String>,
    file_id: impl Into<String>,
) -> VectorStoreFileBuilder {
    VectorStoreFileBuilder::new(vector_store_id, file_id)
}

/// Helper function to search through a vector store.
#[must_use]
pub fn search_vector_store(
    vector_store_id: impl Into<String>,
    query: impl Into<String>,
) -> VectorStoreSearchBuilder {
    VectorStoreSearchBuilder::new(vector_store_id, query)
}

/// Helper function to search with a limit.
#[must_use]
pub fn search_vector_store_with_limit(
    vector_store_id: impl Into<String>,
    query: impl Into<String>,
    limit: i32,
) -> VectorStoreSearchBuilder {
    VectorStoreSearchBuilder::new(vector_store_id, query).limit(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_store_builder() {
        let builder = VectorStoreBuilder::new()
            .name("Test Store")
            .add_file("file-1")
            .add_file("file-2")
            .expires_after_days(30)
            .metadata("key", "value");

        assert_eq!(builder.name_ref(), Some("Test Store"));
        assert_eq!(builder.file_count(), 2);
        assert_eq!(builder.file_ids_ref(), &["file-1", "file-2"]);
        assert!(builder.has_files());
        assert!(builder.expires_after_ref().is_some());
        assert_eq!(builder.expires_after_ref().unwrap().days, 30);
        assert_eq!(builder.metadata_ref().len(), 1);
    }

    #[test]
    fn test_vector_store_builder_with_file_ids() {
        let file_ids = vec![
            "file-1".to_string(),
            "file-2".to_string(),
            "file-3".to_string(),
        ];
        let builder = VectorStoreBuilder::new()
            .name("Bulk Files Store")
            .file_ids(file_ids.clone());

        assert_eq!(builder.name_ref(), Some("Bulk Files Store"));
        assert_eq!(builder.file_ids_ref(), file_ids.as_slice());
        assert_eq!(builder.file_count(), 3);
        assert!(builder.has_files());
    }

    #[test]
    fn test_vector_store_file_builder() {
        let builder = VectorStoreFileBuilder::new("vs-123", "file-456");
        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.file_id(), "file-456");
    }

    #[test]
    fn test_vector_store_search_builder() {
        let builder = VectorStoreSearchBuilder::new("vs-123", "search query")
            .limit(10)
            .filter("category", "documentation");

        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.query(), "search query");
        assert_eq!(builder.limit_ref(), Some(10));
        assert_eq!(builder.filter_ref().len(), 1);
        assert_eq!(
            builder.filter_ref().get("category"),
            Some(&"documentation".to_string())
        );
    }

    #[test]
    fn test_simple_vector_store_helper() {
        let builder = simple_vector_store("Simple Store");
        assert_eq!(builder.name_ref(), Some("Simple Store"));
        assert!(!builder.has_files());
    }

    #[test]
    fn test_vector_store_with_files_helper() {
        let file_ids = vec!["file-1".to_string(), "file-2".to_string()];
        let builder = vector_store_with_files("Files Store", file_ids.clone());
        assert_eq!(builder.name_ref(), Some("Files Store"));
        assert_eq!(builder.file_ids_ref(), file_ids.as_slice());
        assert!(builder.has_files());
    }

    #[test]
    fn test_temporary_vector_store_helper() {
        let builder = temporary_vector_store("Temp Store", 7);
        assert_eq!(builder.name_ref(), Some("Temp Store"));
        assert!(builder.expires_after_ref().is_some());
        assert_eq!(builder.expires_after_ref().unwrap().days, 7);
    }

    #[test]
    fn test_add_file_to_vector_store_helper() {
        let builder = add_file_to_vector_store("vs-123", "file-456");
        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.file_id(), "file-456");
    }

    #[test]
    fn test_search_vector_store_helper() {
        let builder = search_vector_store("vs-123", "test query");
        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.query(), "test query");
        assert!(builder.limit_ref().is_none());
    }

    #[test]
    fn test_search_vector_store_with_limit_helper() {
        let builder = search_vector_store_with_limit("vs-123", "test query", 5);
        assert_eq!(builder.vector_store_id(), "vs-123");
        assert_eq!(builder.query(), "test query");
        assert_eq!(builder.limit_ref(), Some(5));
    }

    #[test]
    fn test_vector_store_builder_default() {
        let builder = VectorStoreBuilder::default();
        assert!(builder.name_ref().is_none());
        assert!(!builder.has_files());
        assert!(builder.expires_after_ref().is_none());
        assert!(builder.metadata_ref().is_empty());
    }

    #[test]
    fn test_vector_store_expiration_policy() {
        let policy = VectorStoreExpirationPolicy { days: 15 };
        assert_eq!(policy.days, 15);
    }
}
