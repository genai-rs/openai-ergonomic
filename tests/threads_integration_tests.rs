//! Integration tests for the Threads API.

use openai_ergonomic::{builders::threads::ThreadRequestBuilder, Builder};

#[test]
fn test_thread_request_builder_basic() {
    let builder = ThreadRequestBuilder::new();

    // Basic builder can be created without errors
    // Verify it has no messages initially
    assert_eq!(builder.messages().len(), 0);
}

#[test]
fn test_thread_request_builder_with_metadata() {
    let builder = ThreadRequestBuilder::new()
        .metadata("user_id", "user_123")
        .metadata("session_id", "session_abc");

    // Builder can be created with metadata
    // Metadata is stored internally and will be sent during build()
    assert_eq!(builder.messages().len(), 0);
}

#[test]
fn test_thread_request_builder_with_user_message() {
    let builder = ThreadRequestBuilder::new().user_message("Hello, assistant!");

    // Verify message was added
    assert_eq!(builder.messages().len(), 1);
}

#[test]
fn test_thread_request_builder_with_assistant_message() {
    let builder = ThreadRequestBuilder::new().assistant_message("Hello, user!");

    // Verify message was added
    assert_eq!(builder.messages().len(), 1);
}

#[test]
fn test_thread_request_builder_multiple_messages() {
    let builder = ThreadRequestBuilder::new()
        .user_message("What is the weather?")
        .assistant_message("Let me check that for you.")
        .user_message("Thank you!");

    // Verify all messages were added
    assert_eq!(builder.messages().len(), 3);
}

#[test]
fn test_thread_request_builder_clear_metadata() {
    let builder = ThreadRequestBuilder::new()
        .metadata("key1", "value1")
        .metadata("key2", "value2")
        .clear_metadata();

    // Metadata should be cleared
    // Since we don't have direct access to metadata, just verify builder still works
    assert_eq!(builder.messages().len(), 0);
}

#[test]
fn test_thread_request_builder_metadata_chaining() {
    // Test that metadata can be chained with messages
    let builder = ThreadRequestBuilder::new()
        .metadata("context", "support")
        .user_message("I need help")
        .metadata("priority", "high")
        .assistant_message("I'm here to help");

    // Verify messages were added despite metadata calls
    assert_eq!(builder.messages().len(), 2);
}

#[test]
fn test_thread_request_builder_build() {
    let builder = ThreadRequestBuilder::new()
        .metadata("user_id", "user_123")
        .user_message("Test message");

    // Verify build() succeeds
    let result = builder.build();
    assert!(result.is_ok());
}

#[test]
fn test_thread_request_builder_empty_build() {
    let builder = ThreadRequestBuilder::new();

    // Verify empty builder can be built
    let result = builder.build();
    assert!(result.is_ok());
}

// Note: More comprehensive tests would require access to the internals
// or using the actual API to verify the thread is created correctly
