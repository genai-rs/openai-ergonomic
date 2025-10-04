//! Integration tests for the Files API.

use openai_ergonomic::builders::files::{
    FileDeleteBuilder, FileListBuilder, FileOrder, FilePurpose, FileRetrievalBuilder,
    FileUploadBuilder,
};

#[test]
fn test_file_upload_builder_basic() {
    let content = b"test content".to_vec();
    let builder = FileUploadBuilder::new("test.txt", FilePurpose::Assistants, content.clone());

    assert_eq!(builder.filename(), "test.txt");
    assert_eq!(builder.content(), content.as_slice());
    assert_eq!(builder.content_size(), content.len());
    assert!(!builder.is_empty());
}

#[test]
fn test_file_upload_builder_from_text() {
    let builder = FileUploadBuilder::from_text("hello.txt", FilePurpose::FineTune, "Hello, world!");

    assert_eq!(builder.filename(), "hello.txt");
    assert_eq!(
        builder.content_as_string(),
        Some("Hello, world!".to_string())
    );
    assert!(!builder.is_empty());
}

#[test]
fn test_file_upload_builder_from_json() {
    let json = serde_json::json!({
        "name": "test",
        "value": 42
    });

    let builder = FileUploadBuilder::from_json("data.json", FilePurpose::Batch, &json).unwrap();

    assert_eq!(builder.filename(), "data.json");
    assert!(!builder.is_empty());
    assert!(builder.content_size() > 0);
}

#[test]
fn test_file_upload_builder_empty() {
    let builder = FileUploadBuilder::new("empty.txt", FilePurpose::Assistants, vec![]);

    assert!(builder.is_empty());
    assert_eq!(builder.content_size(), 0);
    assert_eq!(builder.content_as_string(), Some(String::new()));
}

#[test]
fn test_file_list_builder() {
    let builder = FileListBuilder::new()
        .purpose(FilePurpose::Assistants)
        .limit(10)
        .order(FileOrder::Desc);

    match builder.purpose_ref() {
        Some(FilePurpose::Assistants) => {}
        _ => panic!("Expected Assistants purpose"),
    }
    assert_eq!(builder.limit_ref(), Some(10));
    match builder.order_ref() {
        Some(FileOrder::Desc) => {}
        _ => panic!("Expected Desc order"),
    }
}

#[test]
fn test_file_list_builder_default() {
    let builder = FileListBuilder::new();

    assert!(builder.purpose_ref().is_none());
    assert!(builder.limit_ref().is_none());
    assert!(builder.order_ref().is_none());
}

#[test]
fn test_file_retrieval_builder() {
    let builder = FileRetrievalBuilder::new("file-123");
    assert_eq!(builder.file_id(), "file-123");
}

#[test]
fn test_file_delete_builder() {
    let builder = FileDeleteBuilder::new("file-456");
    assert_eq!(builder.file_id(), "file-456");
}

#[test]
fn test_file_purpose_display() {
    assert_eq!(FilePurpose::FineTune.to_string(), "fine-tune");
    assert_eq!(FilePurpose::Assistants.to_string(), "assistants");
    assert_eq!(FilePurpose::Vision.to_string(), "vision");
    assert_eq!(FilePurpose::Batch.to_string(), "batch");
    assert_eq!(
        FilePurpose::Custom("custom".to_string()).to_string(),
        "custom"
    );
}

#[test]
fn test_file_order_display() {
    assert_eq!(FileOrder::Asc.to_string(), "asc");
    assert_eq!(FileOrder::Desc.to_string(), "desc");
}

#[test]
fn test_file_upload_builder_helpers() {
    use openai_ergonomic::builders::files::{
        upload_assistants_file, upload_fine_tune_file, upload_json_file,
    };

    // Test upload_fine_tune_file
    let builder = upload_fine_tune_file("training.jsonl", "test data");
    assert_eq!(builder.filename(), "training.jsonl");
    match builder.purpose() {
        FilePurpose::FineTune => {}
        _ => panic!("Expected FineTune purpose"),
    }

    // Test upload_assistants_file
    let builder = upload_assistants_file("doc.txt", "document content");
    assert_eq!(builder.filename(), "doc.txt");
    match builder.purpose() {
        FilePurpose::Assistants => {}
        _ => panic!("Expected Assistants purpose"),
    }

    // Test upload_json_file
    let json = serde_json::json!({"test": true});
    let builder = upload_json_file("test.json", FilePurpose::Vision, &json).unwrap();
    assert_eq!(builder.filename(), "test.json");
    match builder.purpose() {
        FilePurpose::Vision => {}
        _ => panic!("Expected Vision purpose"),
    }
}

#[test]
fn test_file_list_builder_helpers() {
    use openai_ergonomic::builders::files::{
        list_files, list_files_by_purpose, list_files_with_limit,
    };

    // Test list_files
    let builder = list_files();
    assert!(builder.purpose_ref().is_none());
    assert!(builder.limit_ref().is_none());
    assert!(builder.order_ref().is_none());

    // Test list_files_by_purpose
    let builder = list_files_by_purpose(FilePurpose::FineTune);
    match builder.purpose_ref() {
        Some(FilePurpose::FineTune) => {}
        _ => panic!("Expected FineTune purpose"),
    }

    // Test list_files_with_limit
    let builder = list_files_with_limit(5);
    assert_eq!(builder.limit_ref(), Some(5));
}

#[test]
fn test_file_retrieval_builder_helper() {
    use openai_ergonomic::builders::files::retrieve_file;

    let builder = retrieve_file("file-789");
    assert_eq!(builder.file_id(), "file-789");
}

#[test]
fn test_file_delete_builder_helper() {
    use openai_ergonomic::builders::files::delete_file;

    let builder = delete_file("file-delete");
    assert_eq!(builder.file_id(), "file-delete");
}

#[test]
fn test_file_upload_builder_content_methods() {
    let content = "Test content with UTF-8: 你好世界";
    let builder = FileUploadBuilder::from_text("test.txt", FilePurpose::Assistants, content);

    assert_eq!(builder.content_as_string(), Some(content.to_string()));
    assert_eq!(builder.content(), content.as_bytes());
    assert_eq!(builder.content_size(), content.as_bytes().len());
}

// Note: These tests require API credentials and should be run with care
// They are commented out by default to avoid consuming API credits during regular testing

/*
#[tokio::test]
async fn test_files_upload_and_delete() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    // Upload a test file
    let content = "This is a test file for the Files API integration test.";
    let builder = client
        .files()
        .upload_text("test_file.txt", FilePurpose::Assistants, content);

    let file = client.files().create(builder).await?;

    assert!(!file.id.is_empty());
    assert_eq!(file.filename, "test_file.txt");
    assert_eq!(file.purpose, "assistants");
    assert!(file.bytes > 0);

    // Clean up - delete the file
    let delete_response = client.files().delete(&file.id).await?;
    assert!(delete_response.deleted);

    Ok(())
}

#[tokio::test]
async fn test_files_list() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    let builder = client.files().list_builder().limit(10);
    let response = client.files().list(builder).await?;

    assert_eq!(response.object, "list");
    // response.data can be empty if no files exist
    assert!(response.data.len() <= 10);

    Ok(())
}

#[tokio::test]
async fn test_files_list_with_purpose() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    let builder = client
        .files()
        .list_builder()
        .purpose(FilePurpose::Assistants)
        .limit(5);

    let response = client.files().list(builder).await?;

    assert_eq!(response.object, "list");
    assert!(response.data.len() <= 5);

    // Verify all files have the assistants purpose
    for file in &response.data {
        assert_eq!(file.purpose, "assistants");
    }

    Ok(())
}

#[tokio::test]
async fn test_files_retrieve() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    // First, upload a file to retrieve
    let content = "Test content for retrieval";
    let builder = client
        .files()
        .upload_text("retrieve_test.txt", FilePurpose::Assistants, content);

    let uploaded_file = client.files().create(builder).await?;
    let file_id = uploaded_file.id.clone();

    // Now retrieve it
    let retrieved_file = client.files().retrieve(&file_id).await?;

    assert_eq!(retrieved_file.id, file_id);
    assert_eq!(retrieved_file.filename, "retrieve_test.txt");
    assert_eq!(retrieved_file.purpose, "assistants");

    // Clean up
    client.files().delete(&file_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_files_download() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    // Upload a file to download
    let original_content = "This is the content we will download back.";
    let builder = client
        .files()
        .upload_text("download_test.txt", FilePurpose::Assistants, original_content);

    let file = client.files().create(builder).await?;
    let file_id = file.id.clone();

    // Download the file content
    let downloaded_content = client.files().download(&file_id).await?;

    assert_eq!(downloaded_content, original_content);

    // Also test download_bytes
    let downloaded_bytes = client.files().download_bytes(&file_id).await?;
    assert_eq!(downloaded_bytes, original_content.as_bytes());

    // Clean up
    client.files().delete(&file_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_files_upload_json() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    // Create JSON content
    let json_data = serde_json::json!({
        "test": "data",
        "number": 42,
        "nested": {
            "key": "value"
        }
    });

    let builder = FileUploadBuilder::from_json(
        "test_data.json",
        FilePurpose::Assistants,
        &json_data,
    )?;

    let file = client.files().create(builder).await?;

    assert!(!file.id.is_empty());
    assert_eq!(file.filename, "test_data.json");

    // Clean up
    client.files().delete(&file.id).await?;

    Ok(())
}

#[tokio::test]
async fn test_files_complete_workflow() -> Result<(), Box<dyn std::error::Error>> {
    use openai_ergonomic::Client;

    let client = Client::from_env()?;

    // 1. Upload a file
    let content = "Complete workflow test file.";
    let builder = client
        .files()
        .upload_text("workflow_test.txt", FilePurpose::Assistants, content);

    let file = client.files().create(builder).await?;
    let file_id = file.id.clone();

    // 2. Verify it appears in the list
    let list_builder = client.files().list_builder().limit(100);
    let list_response = client.files().list(list_builder).await?;

    let found = list_response.data.iter().any(|f| f.id == file_id);
    assert!(found, "Uploaded file should appear in the list");

    // 3. Retrieve file metadata
    let retrieved = client.files().retrieve(&file_id).await?;
    assert_eq!(retrieved.id, file_id);

    // 4. Download file content
    let downloaded = client.files().download(&file_id).await?;
    assert_eq!(downloaded, content);

    // 5. Delete the file
    let delete_response = client.files().delete(&file_id).await?;
    assert!(delete_response.deleted);

    // 6. Verify it's deleted (should return an error)
    let retrieve_result = client.files().retrieve(&file_id).await;
    assert!(retrieve_result.is_err(), "Deleted file should not be retrievable");

    Ok(())
}
*/
