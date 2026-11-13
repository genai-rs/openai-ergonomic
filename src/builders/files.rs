//! Files API builders.
//!
//! This module provides ergonomic builders for `OpenAI` Files API operations,
//! including uploading files, managing file metadata, and retrieving file content.
//!
//! Files are used for various purposes including:
//! - Training data for fine-tuning
//! - Documents for assistants and RAG applications
//! - Images for vision models

use bytes::Bytes;
use std::path::Path;
use timvw_rust_runtime::Upload;

/// Builder for file upload operations.
///
/// This builder provides a fluent interface for uploading files to `OpenAI`
/// with specified purposes and metadata. The content is an [`Upload`]: bytes held in memory
/// ([`FileUploadBuilder::new`], [`FileUploadBuilder::from_text`]), a file streamed from disk
/// ([`FileUploadBuilder::from_path`]) or any other upload ([`FileUploadBuilder::from_upload`]).
/// Nothing is copied to a temporary file.
#[derive(Debug, Clone)]
pub struct FileUploadBuilder {
    filename: String,
    purpose: FilePurpose,
    upload: Upload,
    /// The content, when it is held in memory.
    content: Option<Bytes>,
}

/// Purpose for which the file is being uploaded.
#[derive(Debug, Clone)]
pub enum FilePurpose {
    /// File for fine-tuning operations
    FineTune,
    /// File for assistant operations (RAG, file search)
    Assistants,
    /// File for vision model operations
    Vision,
    /// File for batch operations
    Batch,
    /// Custom purpose (for future extensions)
    Custom(String),
}

impl FileUploadBuilder {
    /// Create a new file upload builder with file content held in memory and a purpose.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::files::{FileUploadBuilder, FilePurpose};
    ///
    /// let content = b"Hello, world!";
    /// let builder = FileUploadBuilder::new("hello.txt", FilePurpose::Assistants, content.to_vec());
    /// ```
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        purpose: FilePurpose,
        content: impl Into<Bytes>,
    ) -> Self {
        let filename = filename.into();
        let content = content.into();
        Self {
            upload: Upload::bytes(content.clone()).file_name(filename.clone()),
            filename,
            purpose,
            content: Some(content),
        }
    }

    /// Create a file upload builder from anything that converts into an [`Upload`]: a path
    /// (`PathBuf`, `&Path`), bytes, or an [`Upload`] made from a stream or a reader.
    ///
    /// The file name is the upload's ([`Upload::name`]: a path's last component, or the one set
    /// with [`Upload::file_name`]), `file` when it has none.
    ///
    /// ```rust
    /// use openai_ergonomic::builders::files::{FileUploadBuilder, FilePurpose};
    /// use timvw_rust_runtime::Upload;
    ///
    /// let upload = Upload::bytes(b"{}\n".to_vec()).file_name("batch.jsonl");
    /// let builder = FileUploadBuilder::from_upload(upload, FilePurpose::Batch);
    /// assert_eq!(builder.filename(), "batch.jsonl");
    /// ```
    #[must_use]
    pub fn from_upload(upload: impl Into<Upload>, purpose: FilePurpose) -> Self {
        let upload = upload.into();
        let filename = upload.name().unwrap_or("file").to_string();
        Self {
            upload: upload.file_name(filename.clone()),
            filename,
            purpose,
            content: None,
        }
    }

    /// Create a file upload builder from a file path.
    ///
    /// The file is streamed from disk when the request is sent, not read here: a file that does
    /// not exist fails the upload with [`Error::Http`](crate::Error::Http), whose
    /// `is_io()` is `true`.
    #[must_use]
    pub fn from_path(path: impl AsRef<Path>, purpose: FilePurpose) -> Self {
        Self::from_upload(path.as_ref(), purpose)
    }

    /// Create a file upload builder from text content.
    #[must_use]
    pub fn from_text(
        filename: impl Into<String>,
        purpose: FilePurpose,
        text: impl Into<String>,
    ) -> Self {
        Self::new(filename, purpose, text.into().into_bytes())
    }

    /// Create a file upload builder from JSON content.
    pub fn from_json(
        filename: impl Into<String>,
        purpose: FilePurpose,
        json: &serde_json::Value,
    ) -> Result<Self, serde_json::Error> {
        let content = serde_json::to_vec(json)?;
        Ok(Self::new(filename, purpose, content))
    }

    /// Set the media type of the file (the `Content-Type` of its multipart part).
    #[must_use]
    pub fn mime(mut self, mime: impl Into<String>) -> Self {
        self.upload = self.upload.mime(mime.into());
        self
    }

    /// Get the filename for this upload.
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Get the purpose for this upload.
    #[must_use]
    pub fn purpose(&self) -> &FilePurpose {
        &self.purpose
    }

    /// The upload, named [`filename`](FileUploadBuilder::filename).
    #[must_use]
    pub fn upload(&self) -> &Upload {
        &self.upload
    }

    /// The content, when it is held in memory (not for a path, a stream or a reader).
    #[must_use]
    pub fn content(&self) -> Option<&[u8]> {
        self.content.as_deref()
    }

    /// The size of the content in bytes, when known: content held in memory, or an upload
    /// given its length. A path's size is only known once it is read.
    #[must_use]
    pub fn content_size(&self) -> Option<u64> {
        self.upload.content_length()
    }

    /// Whether the content is known to be empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content_size() == Some(0)
    }

    /// The content as a string, when it is held in memory and valid UTF-8.
    #[must_use]
    pub fn content_as_string(&self) -> Option<String> {
        self.content
            .as_deref()
            .and_then(|content| String::from_utf8(content.to_vec()).ok())
    }

    /// The parts of the request: the upload and the purpose.
    pub(crate) fn into_parts(self) -> (Upload, FilePurpose) {
        (self.upload, self.purpose)
    }
}

/// Builder for file retrieval operations.
#[derive(Debug, Clone)]
pub struct FileRetrievalBuilder {
    file_id: String,
}

impl FileRetrievalBuilder {
    /// Create a new file retrieval builder.
    #[must_use]
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
        }
    }

    /// Get the file ID for this retrieval.
    #[must_use]
    pub fn file_id(&self) -> &str {
        &self.file_id
    }
}

/// Builder for file listing operations.
#[derive(Debug, Clone, Default)]
pub struct FileListBuilder {
    purpose: Option<FilePurpose>,
    limit: Option<i64>,
    order: Option<FileOrder>,
}

/// Order for file listing.
#[derive(Debug, Clone)]
pub enum FileOrder {
    /// Ascending order (oldest first)
    Asc,
    /// Descending order (newest first)
    Desc,
}

impl FileListBuilder {
    /// Create a new file list builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter files by purpose.
    #[must_use]
    pub fn purpose(mut self, purpose: FilePurpose) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// Set the maximum number of files to return.
    #[must_use]
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the order for file listing.
    #[must_use]
    pub fn order(mut self, order: FileOrder) -> Self {
        self.order = Some(order);
        self
    }

    /// Get the purpose filter.
    #[must_use]
    pub fn purpose_ref(&self) -> Option<&FilePurpose> {
        self.purpose.as_ref()
    }

    /// Get the limit.
    #[must_use]
    pub fn limit_ref(&self) -> Option<i64> {
        self.limit
    }

    /// Get the order.
    #[must_use]
    pub fn order_ref(&self) -> Option<&FileOrder> {
        self.order.as_ref()
    }
}

/// Builder for file deletion operations.
#[derive(Debug, Clone)]
pub struct FileDeleteBuilder {
    file_id: String,
}

impl FileDeleteBuilder {
    /// Create a new file delete builder.
    #[must_use]
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
        }
    }

    /// Get the file ID for this deletion.
    #[must_use]
    pub fn file_id(&self) -> &str {
        &self.file_id
    }
}

/// Helper function to upload a text file for fine-tuning.
#[must_use]
pub fn upload_fine_tune_file(
    filename: impl Into<String>,
    content: impl Into<String>,
) -> FileUploadBuilder {
    FileUploadBuilder::from_text(filename, FilePurpose::FineTune, content)
}

/// Helper function to upload a text file for assistants.
#[must_use]
pub fn upload_assistants_file(
    filename: impl Into<String>,
    content: impl Into<String>,
) -> FileUploadBuilder {
    FileUploadBuilder::from_text(filename, FilePurpose::Assistants, content)
}

/// Helper function to upload a JSON file.
pub fn upload_json_file(
    filename: impl Into<String>,
    purpose: FilePurpose,
    json: &serde_json::Value,
) -> Result<FileUploadBuilder, serde_json::Error> {
    FileUploadBuilder::from_json(filename, purpose, json)
}

/// Helper function to upload a file from a path (streamed from disk when sent).
#[must_use]
pub fn upload_file_from_path(path: impl AsRef<Path>, purpose: FilePurpose) -> FileUploadBuilder {
    FileUploadBuilder::from_path(path, purpose)
}

/// Helper function to retrieve a file.
#[must_use]
pub fn retrieve_file(file_id: impl Into<String>) -> FileRetrievalBuilder {
    FileRetrievalBuilder::new(file_id)
}

/// Helper function to list all files.
#[must_use]
pub fn list_files() -> FileListBuilder {
    FileListBuilder::new()
}

/// Helper function to list files with a specific purpose.
#[must_use]
pub fn list_files_by_purpose(purpose: FilePurpose) -> FileListBuilder {
    FileListBuilder::new().purpose(purpose)
}

/// Helper function to list files with a limit.
#[must_use]
pub fn list_files_with_limit(limit: i64) -> FileListBuilder {
    FileListBuilder::new().limit(limit)
}

/// Helper function to delete a file.
#[must_use]
pub fn delete_file(file_id: impl Into<String>) -> FileDeleteBuilder {
    FileDeleteBuilder::new(file_id)
}

impl std::fmt::Display for FilePurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilePurpose::FineTune => write!(f, "fine-tune"),
            FilePurpose::Assistants => write!(f, "assistants"),
            FilePurpose::Vision => write!(f, "vision"),
            FilePurpose::Batch => write!(f, "batch"),
            FilePurpose::Custom(purpose) => write!(f, "{purpose}"),
        }
    }
}

impl std::fmt::Display for FileOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOrder::Asc => write!(f, "asc"),
            FileOrder::Desc => write!(f, "desc"),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_file_upload_builder_new() {
        let content = b"test content".to_vec();
        let builder = FileUploadBuilder::new("test.txt", FilePurpose::Assistants, content.clone());

        assert_eq!(builder.filename(), "test.txt");
        assert_eq!(builder.content(), Some(content.as_slice()));
        assert_eq!(builder.content_size(), Some(content.len() as u64));
        assert_eq!(builder.upload().name(), Some("test.txt"));
        assert!(!builder.is_empty());
        match builder.purpose() {
            FilePurpose::Assistants => {}
            _ => panic!("Expected Assistants purpose"),
        }
    }

    #[test]
    fn test_file_upload_builder_from_text() {
        let builder =
            FileUploadBuilder::from_text("hello.txt", FilePurpose::FineTune, "Hello, world!");

        assert_eq!(builder.filename(), "hello.txt");
        assert_eq!(
            builder.content_as_string(),
            Some("Hello, world!".to_string())
        );
        assert!(!builder.is_empty());
        match builder.purpose() {
            FilePurpose::FineTune => {}
            _ => panic!("Expected FineTune purpose"),
        }
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
        assert!(builder.content_size().unwrap() > 0);
        match builder.purpose() {
            FilePurpose::Batch => {}
            _ => panic!("Expected Batch purpose"),
        }
    }

    #[test]
    fn test_file_retrieval_builder() {
        let builder = FileRetrievalBuilder::new("file-123");
        assert_eq!(builder.file_id(), "file-123");
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
    fn test_file_delete_builder() {
        let builder = FileDeleteBuilder::new("file-456");
        assert_eq!(builder.file_id(), "file-456");
    }

    #[test]
    fn test_upload_fine_tune_file_helper() {
        let builder = upload_fine_tune_file("training.jsonl", "test data");
        assert_eq!(builder.filename(), "training.jsonl");
        match builder.purpose() {
            FilePurpose::FineTune => {}
            _ => panic!("Expected FineTune purpose"),
        }
    }

    #[test]
    fn test_upload_assistants_file_helper() {
        let builder = upload_assistants_file("doc.txt", "document content");
        assert_eq!(builder.filename(), "doc.txt");
        match builder.purpose() {
            FilePurpose::Assistants => {}
            _ => panic!("Expected Assistants purpose"),
        }
    }

    #[test]
    fn test_upload_json_file_helper() {
        let json = serde_json::json!({"test": true});
        let builder = upload_json_file("test.json", FilePurpose::Vision, &json).unwrap();
        assert_eq!(builder.filename(), "test.json");
        match builder.purpose() {
            FilePurpose::Vision => {}
            _ => panic!("Expected Vision purpose"),
        }
    }

    #[test]
    fn test_retrieve_file_helper() {
        let builder = retrieve_file("file-789");
        assert_eq!(builder.file_id(), "file-789");
    }

    #[test]
    fn test_list_files_helper() {
        let builder = list_files();
        assert!(builder.purpose_ref().is_none());
        assert!(builder.limit_ref().is_none());
        assert!(builder.order_ref().is_none());
    }

    #[test]
    fn test_list_files_by_purpose_helper() {
        let builder = list_files_by_purpose(FilePurpose::FineTune);
        match builder.purpose_ref() {
            Some(FilePurpose::FineTune) => {}
            _ => panic!("Expected FineTune purpose"),
        }
    }

    #[test]
    fn test_list_files_with_limit_helper() {
        let builder = list_files_with_limit(5);
        assert_eq!(builder.limit_ref(), Some(5));
    }

    #[test]
    fn test_delete_file_helper() {
        let builder = delete_file("file-delete");
        assert_eq!(builder.file_id(), "file-delete");
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
    fn test_empty_file() {
        let builder = FileUploadBuilder::new("empty.txt", FilePurpose::Assistants, vec![]);
        assert!(builder.is_empty());
        assert_eq!(builder.content_size(), Some(0));
        assert_eq!(builder.content_as_string(), Some(String::new()));
    }

    #[test]
    fn test_from_path_is_lazy_and_named_after_the_file() {
        let builder =
            FileUploadBuilder::from_path("/no/such/dir/train.jsonl", FilePurpose::FineTune);
        assert_eq!(builder.filename(), "train.jsonl");
        assert_eq!(builder.content(), None);
        assert_eq!(builder.content_size(), None);
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_from_upload_without_a_name() {
        let builder = FileUploadBuilder::from_upload(b"data".to_vec(), FilePurpose::Assistants);
        assert_eq!(builder.filename(), "file");
        assert_eq!(builder.content_size(), Some(4));
        let named = FileUploadBuilder::from_upload(
            Upload::bytes(b"x".to_vec()).file_name("x.txt"),
            FilePurpose::Assistants,
        )
        .mime("text/plain");
        assert_eq!(named.filename(), "x.txt");
        assert_eq!(named.upload().media_type(), Some("text/plain"));
    }
}
