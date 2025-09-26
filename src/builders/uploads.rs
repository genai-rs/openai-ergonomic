//! Uploads API builders.
//!
//! This module offers high-level builders for orchestrating the uploads lifecycle:
//! creating an upload session, streaming file parts, and finalizing the upload
//! into a usable file.

use std::path::PathBuf;

use openai_client_base::models::{
    create_upload_request::Purpose as UploadPurpose,
    file_expiration_after::Anchor as ExpirationAnchor, CompleteUploadRequest, CreateUploadRequest,
    FileExpirationAfter,
};

use crate::{Builder, Error};

/// Builder for creating a new upload session.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::Builder;
/// use openai_ergonomic::builders::uploads::UploadBuilder;
/// use openai_client_base::models::{
///     create_upload_request::Purpose,
///     file_expiration_after::Anchor,
/// };
///
/// let request = UploadBuilder::new("report.csv", Purpose::Assistants, 128, "text/csv")
///     .expires_after(Anchor::CreatedAt, 3_600)
///     .build()
///     .unwrap();
///
/// assert_eq!(request.filename, "report.csv");
/// assert_eq!(request.bytes, 128);
/// assert_eq!(request.mime_type, "text/csv");
/// assert_eq!(request.expires_after.unwrap().seconds, 3_600);
/// ```
#[derive(Debug, Clone)]
pub struct UploadBuilder {
    filename: String,
    purpose: UploadPurpose,
    bytes: i32,
    mime_type: String,
    expires_after: Option<FileExpirationAfter>,
}

impl UploadBuilder {
    /// Start a builder with the required upload parameters.
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        purpose: UploadPurpose,
        bytes: i32,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            purpose,
            bytes,
            mime_type: mime_type.into(),
            expires_after: None,
        }
    }

    /// Configure the expiration policy for the resulting file.
    #[must_use]
    pub fn expires_after(mut self, anchor: ExpirationAnchor, seconds: i32) -> Self {
        self.expires_after = Some(FileExpirationAfter::new(anchor, seconds));
        self
    }

    /// Supply a pre-built expiration configuration.
    #[must_use]
    pub fn expiration(mut self, expiration: FileExpirationAfter) -> Self {
        self.expires_after = Some(expiration);
        self
    }

    /// Inspect the configured filename.
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Inspect the configured purpose.
    #[must_use]
    pub fn purpose(&self) -> UploadPurpose {
        self.purpose
    }

    /// Inspect the configured byte size.
    #[must_use]
    pub fn bytes(&self) -> i32 {
        self.bytes
    }

    /// Inspect the configured MIME type.
    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    /// Inspect the configured expiration policy.
    #[must_use]
    pub fn expires_after_ref(&self) -> Option<&FileExpirationAfter> {
        self.expires_after.as_ref()
    }

    fn validate(&self) -> crate::Result<()> {
        if self.filename.trim().is_empty() {
            return Err(Error::InvalidRequest("filename must not be empty".into()));
        }
        if self.bytes <= 0 {
            return Err(Error::InvalidRequest(
                "bytes must be a positive integer".into(),
            ));
        }
        if self.mime_type.trim().is_empty() {
            return Err(Error::InvalidRequest("mime_type must not be empty".into()));
        }
        Ok(())
    }
}

impl Builder<CreateUploadRequest> for UploadBuilder {
    fn build(self) -> crate::Result<CreateUploadRequest> {
        self.validate()?;

        let Self {
            filename,
            purpose,
            bytes,
            mime_type,
            expires_after,
        } = self;

        let mut request = CreateUploadRequest::new(filename, purpose, bytes, mime_type);
        request.expires_after = expires_after.map(Box::new);
        Ok(request)
    }
}

/// Builder for finalizing an upload.
#[derive(Debug, Clone, Default)]
pub struct CompleteUploadBuilder {
    part_ids: Vec<String>,
    md5: Option<String>,
}

impl CompleteUploadBuilder {
    /// Create a new completion builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single part identifier to the completion request.
    #[must_use]
    pub fn part_id(mut self, part_id: impl Into<String>) -> Self {
        self.part_ids.push(part_id.into());
        self
    }

    /// Add multiple part identifiers at once.
    #[must_use]
    pub fn part_ids<I, S>(mut self, part_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.part_ids.extend(part_ids.into_iter().map(Into::into));
        self
    }

    /// Attach an optional md5 checksum for integrity verification.
    #[must_use]
    pub fn md5(mut self, checksum: impl Into<String>) -> Self {
        self.md5 = Some(checksum.into());
        self
    }

    /// Access the configured part identifiers.
    #[must_use]
    pub fn part_ids_ref(&self) -> &[String] {
        &self.part_ids
    }

    /// Access the configured checksum.
    #[must_use]
    pub fn md5_ref(&self) -> Option<&str> {
        self.md5.as_deref()
    }

    fn validate(&self) -> crate::Result<()> {
        if self.part_ids.is_empty() {
            return Err(Error::InvalidRequest(
                "at least one part id is required to complete an upload".into(),
            ));
        }
        Ok(())
    }
}

impl Builder<CompleteUploadRequest> for CompleteUploadBuilder {
    fn build(self) -> crate::Result<CompleteUploadRequest> {
        self.validate()?;

        let mut request = CompleteUploadRequest::new(self.part_ids);
        request.md5 = self.md5;
        Ok(request)
    }
}

/// Representation of a staged upload part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPartSource {
    path: PathBuf,
}

impl UploadPartSource {
    /// Reference a chunk to upload by its on-disk path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Access the underlying file path.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Consume the source and return the owned path buffer.
    #[must_use]
    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client_base::models::{
        create_upload_request::Purpose, file_expiration_after::Anchor,
    };

    #[test]
    fn builds_upload_request_with_expiration() {
        let builder = UploadBuilder::new(
            "data.parquet",
            Purpose::Assistants,
            1024,
            "application/octet-stream",
        )
        .expires_after(Anchor::CreatedAt, 3600);
        let request = builder.build().expect("builder should succeed");

        assert_eq!(request.filename, "data.parquet");
        assert_eq!(request.bytes, 1024);
        assert_eq!(request.mime_type, "application/octet-stream");
        let expiration = request.expires_after.expect("expiration should be set");
        assert_eq!(expiration.seconds, 3600);
    }

    #[test]
    fn rejects_empty_filename() {
        let builder = UploadBuilder::new("   ", Purpose::Assistants, 10, "text/plain");
        let error = builder.build().expect_err("validation should fail");
        assert!(matches!(error, Error::InvalidRequest(message) if message.contains("filename")));
    }

    #[test]
    fn builds_complete_request_with_checksum() {
        let builder = CompleteUploadBuilder::new()
            .part_ids(["part_1", "part_2"])
            .md5("abc123");
        let request = builder.build().expect("builder should succeed");

        assert_eq!(request.part_ids, vec!["part_1", "part_2"]);
        assert_eq!(request.md5.as_deref(), Some("abc123"));
    }

    #[test]
    fn requires_at_least_one_part() {
        let builder = CompleteUploadBuilder::new();
        let error = builder.build().expect_err("validation should fail");
        assert!(matches!(
            error,
            Error::InvalidRequest(message) if message.contains("part id")
        ));
    }

    #[test]
    fn upload_part_source_holds_path() {
        let source = UploadPartSource::new("/tmp/chunk.bin");
        assert_eq!(source.path(), &PathBuf::from("/tmp/chunk.bin"));
        let owned = source.into_path();
        assert_eq!(owned, PathBuf::from("/tmp/chunk.bin"));
    }
}
