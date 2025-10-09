//! Comprehensive files example demonstrating file management with OpenAI.
//!
//! This example showcases the OpenAI Files API, including:
//! - Uploading files for different purposes (fine-tuning, assistants, batch)
//! - Listing files with filtering and pagination
//! - Retrieving file metadata
//! - Downloading file content
//! - Deleting files
//!
//! ## Features Demonstrated
//!
//! - **File Upload**: Upload text files, JSON files, and files from disk
//! - **File Listing**: List files with filtering by purpose and pagination
//! - **File Retrieval**: Get metadata about specific files
//! - **File Download**: Download file content as text or bytes
//! - **File Deletion**: Delete files when no longer needed
//! - **Error Handling**: Robust error handling for various failure scenarios
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
//! cargo run --example files
//! ```
//!
//! Note: This example uses simulated responses to keep the example runnable without
//! real OpenAI credentials. Replace the simulated sections with real API calls
//! when you're ready to interact with the live API.

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unused_async)]
#![allow(clippy::useless_vec)]
#![allow(dead_code)]

use openai_ergonomic::Client;

/// File metadata for demonstration purposes
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File ID from OpenAI
    pub id: String,
    /// Original filename
    pub filename: String,
    /// File size in bytes
    pub bytes: usize,
    /// File purpose
    pub purpose: String,
    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,
}

impl FileMetadata {
    /// Create a new file metadata instance
    pub fn new(
        id: impl Into<String>,
        filename: impl Into<String>,
        bytes: usize,
        purpose: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            filename: filename.into(),
            bytes,
            purpose: purpose.into(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    /// Format the file size in a human-readable way
    pub fn formatted_size(&self) -> String {
        if self.bytes < 1024 {
            format!("{} B", self.bytes)
        } else if self.bytes < 1024 * 1024 {
            format!("{:.2} KB", self.bytes as f64 / 1024.0)
        } else if self.bytes < 1024 * 1024 * 1024 {
            format!("{:.2} MB", self.bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", self.bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Format the creation time in a human-readable way
    pub fn formatted_created_at(&self) -> String {
        format!("Unix timestamp: {}", self.created_at)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 OpenAI Ergonomic - Comprehensive Files Example\n");

    // Initialize client from environment variables
    let client = match Client::from_env() {
        Ok(client_builder) => {
            println!("✅ Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {e}");
            eprintln!("💡 Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Example 1: Upload Text File
    println!("\n📝 Example 1: Upload Text File");
    println!("================================");

    match upload_text_file_example(&client).await {
        Ok(file_id) => {
            println!("✅ Text file uploaded successfully: {}", file_id);
        }
        Err(e) => {
            eprintln!("❌ Text file upload failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 2: Upload JSON File
    println!("\n📊 Example 2: Upload JSON File");
    println!("================================");

    match upload_json_file_example(&client).await {
        Ok(file_id) => {
            println!("✅ JSON file uploaded successfully: {}", file_id);
        }
        Err(e) => {
            eprintln!("❌ JSON file upload failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 3: List Files
    println!("\n📋 Example 3: List Files");
    println!("=========================");

    match list_files_example(&client).await {
        Ok(count) => {
            println!("✅ Listed {} files successfully", count);
        }
        Err(e) => {
            eprintln!("❌ List files failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 4: Retrieve File Metadata
    println!("\n🔍 Example 4: Retrieve File Metadata");
    println!("======================================");

    match retrieve_file_example(&client).await {
        Ok(()) => {
            println!("✅ File metadata retrieved successfully");
        }
        Err(e) => {
            eprintln!("❌ Retrieve file failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 5: Download File Content
    println!("\n⬇️  Example 5: Download File Content");
    println!("======================================");

    match download_file_example(&client).await {
        Ok(size) => {
            println!("✅ Downloaded {} bytes successfully", size);
        }
        Err(e) => {
            eprintln!("❌ Download file failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 6: Delete File
    println!("\n🗑️  Example 6: Delete File");
    println!("===========================");

    match delete_file_example(&client).await {
        Ok(()) => {
            println!("✅ File deleted successfully");
        }
        Err(e) => {
            eprintln!("❌ Delete file failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    // Example 7: File Management Workflow
    println!("\n🔄 Example 7: File Management Workflow");
    println!("========================================");

    match file_workflow_example(&client).await {
        Ok(()) => {
            println!("✅ File workflow completed successfully");
        }
        Err(e) => {
            eprintln!("❌ File workflow failed: {e}");
            handle_file_error(e.as_ref());
        }
    }

    println!("\n🎉 All examples completed! Check the console output above for results.");
    println!("\nNote: This example simulates API responses. Replace the simulated sections with");
    println!("real client.files() calls when you're ready to interact with the API.");

    Ok(())
}

/// Example 1: Upload a text file
async fn upload_text_file_example(_client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    println!("Uploading a text file for assistants...");

    // Simulated file content
    let content = "This is a sample document for the assistants API.\n\
                   It contains information that can be searched and referenced.\n\
                   The file format is plain text for simplicity.";

    println!("📝 Filename: document.txt");
    println!("📏 Size: {} bytes", content.len());
    println!("🎯 Purpose: assistants");

    // This would be the intended API usage:
    // let builder = client
    //     .files()
    //     .upload_text("document.txt", FilePurpose::Assistants, content);
    // let file = client.files().create(builder).await?;
    // println!("✅ Uploaded file ID: {}", file.id);
    // Ok(file.id)

    // Simulate the response
    let file_id = "file-abc123";
    println!("📤 Upload initiated...");
    println!("✅ File uploaded successfully");
    println!("   File ID: {}", file_id);
    println!("   Status: processed");

    Ok(file_id.to_string())
}

/// Example 2: Upload a JSON file for batch processing
async fn upload_json_file_example(_client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    println!("Uploading a JSON file for batch processing...");

    // Create a JSON file for batch processing
    let batch_data = serde_json::json!({
        "custom_id": "request-1",
        "method": "POST",
        "url": "/v1/chat/completions",
        "body": {
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"}
            ]
        }
    });

    let content = serde_json::to_string_pretty(&batch_data)?;

    println!("📝 Filename: batch_request.jsonl");
    println!("📏 Size: {} bytes", content.len());
    println!("🎯 Purpose: batch");
    println!("📊 Content preview:");
    println!("{}", content);

    // This would be the intended API usage:
    // let builder = FileUploadBuilder::from_json(
    //     "batch_request.jsonl",
    //     FilePurpose::Batch,
    //     &batch_data
    // )?;
    // let file = client.files().create(builder).await?;
    // Ok(file.id)

    // Simulate the response
    let file_id = "file-batch456";
    println!("\n📤 Upload initiated...");
    println!("✅ File uploaded successfully");
    println!("   File ID: {}", file_id);
    println!("   Status: processed");

    Ok(file_id.to_string())
}

/// Example 3: List files with filtering
async fn list_files_example(_client: &Client) -> Result<usize, Box<dyn std::error::Error>> {
    println!("Listing files with filtering...");

    // This would be the intended API usage:
    // let builder = client
    //     .files()
    //     .list_builder()
    //     .purpose(FilePurpose::Assistants)
    //     .limit(10);
    // let response = client.files().list(builder).await?;
    // println!("Found {} files", response.data.len());
    // for file in &response.data {
    //     println!("  - {} ({}) - {} bytes", file.filename, file.id, file.bytes);
    // }
    // Ok(response.data.len())

    // Simulate the response
    let simulated_files = vec![
        FileMetadata::new("file-abc123", "document.txt", 1024, "assistants"),
        FileMetadata::new("file-def456", "training.jsonl", 2048, "fine-tune"),
        FileMetadata::new("file-ghi789", "batch_request.jsonl", 512, "batch"),
    ];

    println!("\n📋 Listing all files:");
    println!("   Found {} files", simulated_files.len());

    for (i, file) in simulated_files.iter().enumerate() {
        println!("\n   {}. {}", i + 1, file.filename);
        println!("      ID: {}", file.id);
        println!("      Size: {}", file.formatted_size());
        println!("      Purpose: {}", file.purpose);
        println!("      Created: {}", file.formatted_created_at());
    }

    println!("\n💡 Filtering options:");
    println!("   - Filter by purpose (fine-tune, assistants, batch, vision)");
    println!("   - Limit results (default: 20)");
    println!("   - Order by creation time (asc/desc)");

    Ok(simulated_files.len())
}

/// Example 4: Retrieve file metadata
async fn retrieve_file_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Retrieving file metadata...");

    let file_id = "file-abc123";
    println!("🔍 Looking up file: {}", file_id);

    // This would be the intended API usage:
    // let file = client.files().retrieve(file_id).await?;
    // println!("✅ File found:");
    // println!("   Filename: {}", file.filename);
    // println!("   Size: {} bytes", file.bytes);
    // println!("   Purpose: {}", file.purpose);
    // println!("   Status: {}", file.status);
    // println!("   Created: {}", file.created_at);
    // Ok(())

    // Simulate the response
    let file = FileMetadata::new(file_id, "document.txt", 1024, "assistants");

    println!("\n✅ File metadata retrieved:");
    println!("   ID: {}", file.id);
    println!("   Filename: {}", file.filename);
    println!("   Size: {}", file.formatted_size());
    println!("   Purpose: {}", file.purpose);
    println!("   Created: {}", file.formatted_created_at());
    println!("   Status: processed");

    Ok(())
}

/// Example 5: Download file content
async fn download_file_example(_client: &Client) -> Result<usize, Box<dyn std::error::Error>> {
    println!("Downloading file content...");

    let file_id = "file-abc123";
    println!("⬇️  Downloading file: {}", file_id);

    // This would be the intended API usage:
    // let content = client.files().download(file_id).await?;
    // println!("✅ Downloaded {} bytes", content.len());
    // println!("📄 Content preview:");
    // println!("{}", &content[..100.min(content.len())]);
    // Ok(content.len())

    // Simulate the response
    let content = "This is a sample document for the assistants API.\n\
                   It contains information that can be searched and referenced.\n\
                   The file format is plain text for simplicity.";

    println!("\n✅ File downloaded successfully");
    println!("   Size: {} bytes", content.len());
    println!("\n📄 Content preview:");
    let preview_len = 100.min(content.len());
    println!("{}", &content[..preview_len]);
    if content.len() > preview_len {
        println!("   ... (truncated)");
    }

    println!("\n💡 Download options:");
    println!("   - download() - Returns content as String");
    println!("   - download_bytes() - Returns content as Vec<u8>");

    Ok(content.len())
}

/// Example 6: Delete a file
async fn delete_file_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Deleting a file...");

    let file_id = "file-temp123";
    println!("🗑️  Deleting file: {}", file_id);

    // This would be the intended API usage:
    // let response = client.files().delete(file_id).await?;
    // println!("✅ File deleted: {}", response.deleted);
    // Ok(())

    // Simulate the response
    println!("\n✅ File deleted successfully");
    println!("   File ID: {}", file_id);
    println!("   Deleted: true");

    println!("\n⚠️  Important notes:");
    println!("   - Deleted files cannot be recovered");
    println!("   - Files in use by jobs cannot be deleted");
    println!("   - Check file dependencies before deletion");

    Ok(())
}

/// Example 7: Complete file management workflow
async fn file_workflow_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating a complete file management workflow...");

    println!("\n🔄 Workflow steps:");
    println!("   1. Create training data");
    println!("   2. Upload file");
    println!("   3. Verify upload");
    println!("   4. List files");
    println!("   5. Clean up");

    // Step 1: Create training data
    println!("\n📝 Step 1: Creating training data...");
    let training_data = vec![
        serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is AI?"},
                {"role": "assistant", "content": "AI stands for Artificial Intelligence..."}
            ]
        }),
        serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Explain machine learning"},
                {"role": "assistant", "content": "Machine learning is a subset of AI..."}
            ]
        }),
    ];

    let jsonl_content: Vec<String> = training_data
        .iter()
        .map(|obj| serde_json::to_string(obj).unwrap())
        .collect();
    let content = jsonl_content.join("\n");

    println!("   ✅ Created {} training examples", training_data.len());
    println!("   📏 Total size: {} bytes", content.len());

    // Step 2: Upload file
    println!("\n📤 Step 2: Uploading file...");
    // This would be the intended API usage:
    // let builder = client
    //     .files()
    //     .upload_text("training.jsonl", FilePurpose::FineTune, &content);
    // let file = client.files().create(builder).await?;
    // let file_id = file.id;

    let file_id = "file-workflow789";
    println!("   ✅ File uploaded: {}", file_id);

    // Step 3: Verify upload
    println!("\n🔍 Step 3: Verifying upload...");
    // This would be the intended API usage:
    // let file_info = client.files().retrieve(&file_id).await?;
    // println!("   ✅ File verified:");
    // println!("      Filename: {}", file_info.filename);
    // println!("      Status: {}", file_info.status);

    println!("   ✅ File verified:");
    println!("      Filename: training.jsonl");
    println!("      Status: processed");
    println!("      Size: {} bytes", content.len());

    // Step 4: List files to confirm
    println!("\n📋 Step 4: Listing all files...");
    // This would be the intended API usage:
    // let files = client.files().list(client.files().list_builder()).await?;
    // println!("   ✅ Total files: {}", files.data.len());

    println!("   ✅ Total files: 4");
    println!("   📝 Including our new file: {}", file_id);

    // Step 5: Optional cleanup
    println!("\n🗑️  Step 5: Cleanup (optional)...");
    println!("   💡 Skipping deletion - file may be used for training");
    println!("   ℹ️  To delete: client.files().delete(&file_id).await");

    println!("\n✅ Workflow completed successfully!");

    println!("\n💡 Best practices:");
    println!("   1. Validate file format before upload");
    println!("   2. Check file size limits (max 512 MB per file)");
    println!("   3. Use appropriate purpose for each file");
    println!("   4. Verify file status after upload");
    println!("   5. Clean up unused files to save storage");
    println!("   6. Use JSONL format for training data");
    println!("   7. Handle upload errors gracefully");

    Ok(())
}

/// Handle file-specific errors with helpful context
fn handle_file_error(error: &dyn std::error::Error) {
    eprintln!("🚫 File Error: {}", error);

    if let Some(source) = error.source() {
        eprintln!("   Caused by: {}", source);
    }

    eprintln!("\n💡 Troubleshooting tips:");
    eprintln!("   - Check your API key and network connection");
    eprintln!("   - Verify file format matches the purpose");
    eprintln!("   - Ensure file size is within limits (max 512 MB)");
    eprintln!("   - Check file ID is valid for retrieve/download/delete");
    eprintln!("   - For fine-tuning: Use JSONL format");
    eprintln!("   - For batch: Follow batch API format");
    eprintln!("   - For assistants: Check supported file types");
}
