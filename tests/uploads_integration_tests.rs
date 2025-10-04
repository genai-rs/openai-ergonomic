//! Integration tests for the Uploads API.
#![allow(clippy::doc_markdown)]
#![allow(clippy::no_effect_underscore_binding)]

use openai_ergonomic::UploadPurpose;

#[test]
fn test_upload_purpose_variants() {
    // Test that all purpose variants can be created
    let _assistants = UploadPurpose::Assistants;
    let _batch = UploadPurpose::Batch;
    let _fine_tune = UploadPurpose::FineTune;
    let _vision = UploadPurpose::Vision;
}

#[test]
fn test_upload_purpose_equality() {
    // Test that same purposes are equal
    assert_eq!(UploadPurpose::Assistants, UploadPurpose::Assistants);
    assert_eq!(UploadPurpose::Batch, UploadPurpose::Batch);
    assert_eq!(UploadPurpose::FineTune, UploadPurpose::FineTune);
    assert_eq!(UploadPurpose::Vision, UploadPurpose::Vision);

    // Test that different purposes are not equal
    assert_ne!(UploadPurpose::Assistants, UploadPurpose::Batch);
    assert_ne!(UploadPurpose::FineTune, UploadPurpose::Vision);
}

// Note: UploadBuilder tests would go here if we had access to the builder
// Currently the builder is created via client.uploads().builder()
// Integration tests with actual API calls would be in a separate test module
// that requires OPENAI_API_KEY
