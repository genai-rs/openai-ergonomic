//! Comprehensive uploads example demonstrating large file uploads with OpenAI.
//!
//! This example showcases the OpenAI Uploads API, which allows uploading large files
//! in multiple parts for improved reliability and performance.
//!
//! ## Features Demonstrated
//!
//! - **Upload Creation**: Initialize multipart uploads for large files
//! - **Part Upload**: Upload file parts in chunks
//! - **Upload Completion**: Finalize uploads after all parts are uploaded
//! - **Error Handling**: Robust error handling for upload failures
//! - **Progress Tracking**: Monitor upload progress
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
//! cargo run --example uploads
//! ```
//!
//! ## When to Use Uploads API
//!
//! The Uploads API is designed for large files (>512 MB). For smaller files,
//! use the standard Files API instead.

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::unused_async)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_sign_loss)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use openai_ergonomic::{builders::uploads::UploadBuilder, Client, UploadPurpose};

/// Upload metadata for demonstration
#[derive(Debug, Clone)]
pub struct UploadInfo {
    pub id: String,
    pub filename: String,
    pub bytes: i32,
    pub purpose: String,
    pub status: String,
}

impl UploadInfo {
    pub fn new(
        id: impl Into<String>,
        filename: impl Into<String>,
        bytes: i32,
        purpose: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            filename: filename.into(),
            bytes,
            purpose: purpose.into(),
            status: status.into(),
        }
    }

    pub fn display(&self) {
        println!("  Upload ID: {}", self.id);
        println!("  Filename: {}", self.filename);
        println!(
            "  Size: {} bytes ({:.2} MB)",
            self.bytes,
            self.bytes as f64 / (1024.0 * 1024.0)
        );
        println!("  Purpose: {}", self.purpose);
        println!("  Status: {}", self.status);
    }

    pub fn formatted_size(&self) -> String {
        let bytes = self.bytes as f64;
        if bytes < 1024.0 {
            format!("{:.0} B", bytes)
        } else if bytes < 1024.0 * 1024.0 {
            format!("{:.2} KB", bytes / 1024.0)
        } else if bytes < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} MB", bytes / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" OpenAI Ergonomic - Comprehensive Uploads Example\n");

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

    // Example 1: Create multipart upload for a large file
    println!("");
    println!(" Example 1: Create Multipart Upload");
    println!("\n");

    // Simulate a large file
    let filename = "large_training_dataset.jsonl";
    let file_size_mb = 750; // 750 MB
    let file_size_bytes = file_size_mb * 1024 * 1024;
    let mime_type = "application/jsonl";

    println!("Creating multipart upload...");
    println!("  Filename: {}", filename);
    println!("  Size: {} MB ({} bytes)", file_size_mb, file_size_bytes);
    println!("  Purpose: fine-tune");
    println!("  MIME Type: {}", mime_type);

    let builder = client.uploads().builder(
        filename,
        UploadPurpose::FineTune,
        file_size_bytes,
        mime_type,
    );

    println!("\n Note: This would create a real multipart upload session.");
    println!("   Commented out to avoid accidental API calls.\n");

    // Uncomment to actually create upload:
    // match client.uploads().create(builder).await {
    //     Ok(upload) => {
    //         println!(" Upload session created successfully!");
    //         println!("  Upload ID: {}", upload.id);
    //         println!("  Status: {}", upload.status);
    //         println!("  Expires At: {}", upload.expires_at);
    //     }
    //     Err(e) => {
    //         eprintln!(" Failed to create upload: {}", e);
    //     }
    // }

    // Simulate upload creation for demonstration
    let demo_upload = UploadInfo::new(
        "upload-demo123",
        filename,
        file_size_bytes,
        "fine-tune",
        "pending",
    );
    println!(" Demo Upload Created:");
    demo_upload.display();

    // Example 2: Upload file parts
    println!("\n");
    println!(" Example 2: Upload File Parts");
    println!("\n");

    let upload_id = "upload-demo123";
    let part_size_mb = 64; // Upload in 64 MB chunks
    let total_parts = (file_size_mb + part_size_mb - 1) / part_size_mb; // Ceiling division

    println!(
        "Uploading {} parts ({} MB each)...\n",
        total_parts, part_size_mb
    );

    for part_num in 1..=total_parts {
        let progress_percent = (part_num as f64 / total_parts as f64) * 100.0;

        println!(
            " Uploading part {}/{} ({:.1}% complete)",
            part_num, total_parts, progress_percent
        );

        // In a real implementation, you would:
        // 1. Read the file chunk from disk
        // 2. Upload it to the part URL provided by OpenAI
        // 3. Track the part ID for completion

        // Uncomment to actually upload parts:
        // let part_data = read_file_chunk(filename, part_num, part_size_mb)?;
        // match upload_part(upload_id, part_num, &part_data).await {
        //     Ok(part_id) => {
        //         println!("   Part {} uploaded (ID: {})", part_num, part_id);
        //     }
        //     Err(e) => {
        //         eprintln!("   Failed to upload part {}: {}", part_num, e);
        //         break;
        //     }
        // }
    }

    println!("\n All {} parts uploaded successfully", total_parts);

    // Example 3: Complete the upload
    println!("\n");
    println!(" Example 3: Complete Upload");
    println!("\n");

    println!("Completing upload: {}\n", upload_id);

    // Uncomment to actually complete upload:
    // match complete_upload(upload_id, part_ids).await {
    //     Ok(file) => {
    //         println!(" Upload completed successfully!");
    //         println!("  File ID: {}", file.id);
    //         println!("  Filename: {}", file.filename);
    //         println!("  Status: ready");
    //         println!("  Purpose: {}", file.purpose);
    //     }
    //     Err(e) => {
    //         eprintln!(" Failed to complete upload: {}", e);
    //     }
    // }

    println!(" Demo: Would finalize the upload and create a file object");
    println!("  File ID: file-abc123");
    println!("  Filename: {}", filename);
    println!("  Status: ready");

    // Example 4: Upload smaller file (alternative approach)
    println!("\n");
    println!(" Example 4: Upload Smaller File");
    println!("\n");

    let small_filename = "training_data.jsonl";
    let small_size_mb = 10;
    let small_size_bytes = small_size_mb * 1024 * 1024;

    println!("Creating upload for smaller file...");
    println!("  Filename: {}", small_filename);
    println!("  Size: {} MB", small_size_mb);
    println!("  Purpose: assistants");

    let small_builder = client.uploads().builder(
        small_filename,
        UploadPurpose::Assistants,
        small_size_bytes,
        "application/jsonl",
    );

    println!("\n Note: For files < 512 MB, consider using the regular Files API");
    println!("   The Uploads API is optimized for large files.");

    // Example 5: Error handling and retry
    println!("\n");
    println!(" Example 5: Error Handling & Retry");
    println!("\n");

    println!("Demonstrating retry logic for failed part uploads...\n");

    let max_retries = 3;
    let failed_part = 5;

    for retry in 1..=max_retries {
        println!(" Attempt {} to upload part {}", retry, failed_part);

        // Simulate upload attempt
        // In a real implementation:
        // match upload_part(upload_id, failed_part, &part_data).await {
        //     Ok(part_id) => {
        //         println!("   Upload succeeded");
        //         break;
        //     }
        //     Err(e) => {
        //         if retry < max_retries {
        //             println!("    Upload failed, retrying... ({})", e);
        //             tokio::time::sleep(Duration::from_secs(2_u64.pow(retry))).await;
        //         } else {
        //             eprintln!("   Upload failed after {} attempts: {}", max_retries, e);
        //         }
        //     }
        // }
    }

    println!("\n Tip: Implement exponential backoff for retry logic");

    // Example 6: Upload progress tracking
    println!("\n");
    println!(" Example 6: Progress Tracking");
    println!("\n");

    struct UploadProgress {
        total_bytes: i32,
        uploaded_bytes: i32,
        total_parts: i32,
        uploaded_parts: i32,
    }

    impl UploadProgress {
        fn percentage(&self) -> f64 {
            (self.uploaded_bytes as f64 / self.total_bytes as f64) * 100.0
        }

        fn eta_seconds(&self, bytes_per_second: f64) -> i32 {
            let remaining_bytes = self.total_bytes - self.uploaded_bytes;
            (remaining_bytes as f64 / bytes_per_second) as i32
        }

        fn display(&self, bytes_per_second: f64) {
            let progress_bar_width = 40;
            let filled = ((self.percentage() / 100.0) * progress_bar_width as f64) as usize;
            let empty = progress_bar_width - filled;

            print!("  [");
            print!("{}", "".repeat(filled));
            print!("{}", "".repeat(empty));
            print!("] ");

            println!(
                "{:.1}% ({}/{} parts, {} MB/s, ETA: {}s)",
                self.percentage(),
                self.uploaded_parts,
                self.total_parts,
                bytes_per_second / (1024.0 * 1024.0),
                self.eta_seconds(bytes_per_second)
            );
        }
    }

    let progress = UploadProgress {
        total_bytes: file_size_bytes,
        uploaded_bytes: (file_size_bytes as f64 * 0.65) as i32,
        total_parts,
        uploaded_parts: (total_parts as f64 * 0.65) as i32,
    };

    println!("Current upload progress:");
    progress.display(10.0 * 1024.0 * 1024.0); // 10 MB/s

    // Summary
    println!("\n");
    println!(" Summary");
    println!("\n");

    println!(" Uploads API examples completed!");
    println!("\n Key Takeaways:");
    println!("  • Uploads API is designed for large files (>512 MB)");
    println!("  • Files are uploaded in parts for reliability");
    println!("  • Each part can be retried independently");
    println!("  • Progress can be tracked during upload");
    println!("  • Upload must be completed after all parts are uploaded");

    println!("\n Best Practices:");
    println!("  1. Use appropriate part sizes (typically 64 MB)");
    println!("  2. Implement retry logic with exponential backoff");
    println!("  3. Track progress and provide user feedback");
    println!("  4. Handle upload cancellation gracefully");
    println!("  5. Verify file integrity after upload");

    println!("\n When to Use:");
    println!("  • Large training datasets for fine-tuning");
    println!("  • Big files for assistants (>512 MB)");
    println!("  • Batch processing input files");
    println!("  • Any file where reliability is critical");

    println!("\n Example completed successfully!");

    Ok(())
}
