//! Integration tests for the Vector Stores API.

use openai_ergonomic::builders::vector_stores::{
    add_file_to_vector_store, search_vector_store, search_vector_store_with_limit,
    simple_vector_store, temporary_vector_store, vector_store_with_files, VectorStoreBuilder,
    VectorStoreFileBuilder, VectorStoreSearchBuilder,
};

#[test]
fn test_vector_store_builder_basic() {
    let builder = VectorStoreBuilder::new()
        .name("Test Store")
        .add_file("file-1")
        .metadata("key", "value");

    assert_eq!(builder.name_ref(), Some("Test Store"));
    assert_eq!(builder.file_count(), 1);
    assert!(builder.has_files());
    assert_eq!(builder.metadata_ref().len(), 1);
}

#[test]
fn test_vector_store_builder_with_expiration() {
    let builder = VectorStoreBuilder::new()
        .name("Temp Store")
        .expires_after_days(30);

    assert_eq!(builder.name_ref(), Some("Temp Store"));
    assert!(builder.expires_after_ref().is_some());
    assert_eq!(builder.expires_after_ref().unwrap().days, 30);
}

#[test]
fn test_vector_store_builder_multiple_files() {
    let files = vec!["file-1".to_string(), "file-2".to_string()];
    let builder = VectorStoreBuilder::new()
        .name("Multi-File Store")
        .file_ids(files.clone());

    assert_eq!(builder.file_ids_ref(), files.as_slice());
    assert_eq!(builder.file_count(), 2);
}

#[test]
fn test_vector_store_builder_clear_files() {
    let builder = VectorStoreBuilder::new()
        .add_file("file-1")
        .add_file("file-2")
        .clear_files()
        .add_file("file-3");

    assert_eq!(builder.file_count(), 1);
    assert_eq!(builder.file_ids_ref(), &["file-3"]);
}

#[test]
fn test_vector_store_builder_add_files() {
    let builder = VectorStoreBuilder::new()
        .name("Batch Store")
        .add_file("file-1")
        .add_files(vec!["file-2", "file-3"])
        .add_file("file-4");

    assert_eq!(builder.file_count(), 4);
    assert_eq!(
        builder.file_ids_ref(),
        &["file-1", "file-2", "file-3", "file-4"]
    );
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
fn test_vector_store_file_builder() {
    let builder = VectorStoreFileBuilder::new("vs-123", "file-456");
    assert_eq!(builder.vector_store_id(), "vs-123");
    assert_eq!(builder.file_id(), "file-456");
}

#[test]
fn test_vector_store_search_builder() {
    let builder = VectorStoreSearchBuilder::new("vs-123", "test query")
        .limit(10)
        .filter("category", "docs")
        .filter("type", "reference");

    assert_eq!(builder.vector_store_id(), "vs-123");
    assert_eq!(builder.query(), "test query");
    assert_eq!(builder.limit_ref(), Some(10));
    assert_eq!(builder.filter_ref().len(), 2);
    assert_eq!(
        builder.filter_ref().get("category"),
        Some(&"docs".to_string())
    );
}

#[test]
fn test_vector_store_search_builder_default() {
    let builder = VectorStoreSearchBuilder::new("vs-123", "query");
    assert!(builder.limit_ref().is_none());
    assert!(builder.filter_ref().is_empty());
}

#[test]
fn test_simple_vector_store_helper() {
    let builder = simple_vector_store("Simple Store")
        .metadata("type", "simple")
        .add_file("file-1");

    assert_eq!(builder.name_ref(), Some("Simple Store"));
    assert!(builder.has_files());
    assert_eq!(
        builder.metadata_ref().get("type"),
        Some(&"simple".to_string())
    );
}

#[test]
fn test_vector_store_with_files_helper() {
    let files = vec!["file-1".to_string(), "file-2".to_string()];
    let builder = vector_store_with_files("Files Store", files.clone());

    assert_eq!(builder.name_ref(), Some("Files Store"));
    assert_eq!(builder.file_ids_ref(), files.as_slice());
    assert_eq!(builder.file_count(), 2);
}

#[test]
fn test_temporary_vector_store_helper() {
    let builder = temporary_vector_store("Temp Store", 7)
        .add_file("file-session-1")
        .metadata("session", "active");

    assert_eq!(builder.name_ref(), Some("Temp Store"));
    assert!(builder.expires_after_ref().is_some());
    assert_eq!(builder.expires_after_ref().unwrap().days, 7);
    assert!(builder.has_files());
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
    let builder = search_vector_store_with_limit("vs-123", "limited query", 5);
    assert_eq!(builder.vector_store_id(), "vs-123");
    assert_eq!(builder.query(), "limited query");
    assert_eq!(builder.limit_ref(), Some(5));
}

#[test]
fn test_vector_store_builder_metadata() {
    let builder = VectorStoreBuilder::new()
        .name("Metadata Store")
        .metadata("env", "production")
        .metadata("version", "1.0")
        .metadata("owner", "team-a");

    assert_eq!(builder.metadata_ref().len(), 3);
    assert_eq!(
        builder.metadata_ref().get("env"),
        Some(&"production".to_string())
    );
    assert_eq!(
        builder.metadata_ref().get("version"),
        Some(&"1.0".to_string())
    );
}

#[test]
fn test_vector_store_expiration_policy() {
    use openai_ergonomic::builders::vector_stores::VectorStoreExpirationPolicy;

    let policy = VectorStoreExpirationPolicy { days: 90 };
    assert_eq!(policy.days, 90);
}

#[test]
fn test_vector_store_search_builder_filters() {
    let builder = VectorStoreSearchBuilder::new("vs-789", "search term")
        .limit(20)
        .filter("status", "active")
        .filter("priority", "high")
        .filter("category", "documentation");

    assert_eq!(builder.filter_ref().len(), 3);
    assert_eq!(
        builder.filter_ref().get("status"),
        Some(&"active".to_string())
    );
    assert_eq!(
        builder.filter_ref().get("priority"),
        Some(&"high".to_string())
    );
}

#[test]
fn test_vector_store_builder_complex() {
    let builder = VectorStoreBuilder::new()
        .name("Complex Store")
        .add_file("file-1")
        .add_files(vec!["file-2", "file-3"])
        .expires_after_days(180)
        .metadata("department", "engineering")
        .metadata("project", "vector-db")
        .metadata("criticality", "high");

    assert_eq!(builder.name_ref(), Some("Complex Store"));
    assert_eq!(builder.file_count(), 3);
    assert!(builder.expires_after_ref().is_some());
    assert_eq!(builder.expires_after_ref().unwrap().days, 180);
    assert_eq!(builder.metadata_ref().len(), 3);
}

#[test]
fn test_vector_store_builder_edge_cases() {
    // Test with no name
    let builder_no_name = VectorStoreBuilder::new().add_file("file-1");
    assert!(builder_no_name.name_ref().is_none());
    assert!(builder_no_name.has_files());

    // Test with no files
    let builder_no_files = VectorStoreBuilder::new().name("Empty Store");
    assert_eq!(builder_no_files.name_ref(), Some("Empty Store"));
    assert!(!builder_no_files.has_files());
    assert_eq!(builder_no_files.file_count(), 0);

    // Test with empty metadata
    let builder_empty = VectorStoreBuilder::new();
    assert!(builder_empty.metadata_ref().is_empty());
}

#[test]
fn test_search_builder_edge_cases() {
    // Test with minimum parameters
    let builder_min = VectorStoreSearchBuilder::new("vs-1", "q");
    assert_eq!(builder_min.vector_store_id(), "vs-1");
    assert_eq!(builder_min.query(), "q");
    assert!(builder_min.limit_ref().is_none());
    assert!(builder_min.filter_ref().is_empty());

    // Test with max parameters
    let builder_max = VectorStoreSearchBuilder::new("vs-2", "complex query here")
        .limit(50)
        .filter("a", "1")
        .filter("b", "2")
        .filter("c", "3");
    assert_eq!(builder_max.limit_ref(), Some(50));
    assert_eq!(builder_max.filter_ref().len(), 3);
}
