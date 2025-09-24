//! Uploads API builders.
//!
//! Provides a simple builder for preparing multipart upload sessions with
//! validation on file size and expiration settings.

use openai_client_base::models::create_upload_request::Purpose;
use openai_client_base::models::{file_expiration_after, CreateUploadRequest, FileExpirationAfter};

use crate::{Builder, Error, Result};

/// Builder for creating upload sessions.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::{Builder, UploadBuilder, UploadPurpose};
///
/// let request = UploadBuilder::new(
///         "dataset.jsonl",
///         UploadPurpose::Assistants,
///         2048,
///         "application/json",
///     )
///     .expires_after_seconds(3600)
///     .build()
///     .unwrap();
///
/// assert_eq!(request.filename, "dataset.jsonl");
/// assert_eq!(request.purpose, UploadPurpose::Assistants);
/// ```
#[derive(Debug, Clone)]
pub struct UploadBuilder {
    filename: String,
    purpose: Purpose,
    bytes: i32,
    mime_type: String,
    expires_after: Option<FileExpirationAfter>,
}

impl UploadBuilder {
    /// Create a new upload builder.
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        purpose: Purpose,
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

    /// Override the filename before building the request.
    #[must_use]
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = filename.into();
        self
    }

    /// Override the number of bytes expected for the upload.
    #[must_use]
    pub fn bytes(mut self, bytes: i32) -> Self {
        self.bytes = bytes;
        self
    }

    /// Override the MIME type.
    #[must_use]
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = mime_type.into();
        self
    }

    /// Override the purpose for this upload.
    #[must_use]
    pub fn purpose(mut self, purpose: Purpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// Set the expiration policy directly.
    #[must_use]
    pub fn expires_after(mut self, expiration: FileExpirationAfter) -> Self {
        self.expires_after = Some(expiration);
        self
    }

    /// Set the expiration in seconds using the default `created_at` anchor.
    #[must_use]
    pub fn expires_after_seconds(mut self, seconds: i32) -> Self {
        let expiration =
            FileExpirationAfter::new(file_expiration_after::Anchor::CreatedAt, seconds);
        self.expires_after = Some(expiration);
        self
    }

    /// Access the configured purpose.
    #[must_use]
    pub fn purpose_ref(&self) -> Purpose {
        self.purpose
    }

    /// Access the configured expiration policy.
    #[must_use]
    pub fn expires_after_ref(&self) -> Option<&FileExpirationAfter> {
        self.expires_after.as_ref()
    }

    fn validate(&self) -> Result<()> {
        if self.bytes <= 0 {
            return Err(Error::InvalidRequest(
                "Upload byte size must be positive".to_string(),
            ));
        }

        if let Some(expiration) = &self.expires_after {
            if !(3600..=2_592_000).contains(&expiration.seconds) {
                return Err(Error::InvalidRequest(format!(
                    "Expiration seconds must be between 3600 and 2592000 (got {})",
                    expiration.seconds
                )));
            }
        }

        Ok(())
    }
}

impl Builder<CreateUploadRequest> for UploadBuilder {
    fn build(self) -> Result<CreateUploadRequest> {
        self.validate()?;
        let mut request =
            CreateUploadRequest::new(self.filename, self.purpose, self.bytes, self.mime_type);
        request.expires_after = self.expires_after.map(Box::new);
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_request() {
        let builder = UploadBuilder::new(
            "transcript.zip",
            Purpose::Assistants,
            1024,
            "application/zip",
        )
        .expires_after_seconds(7200);

        let request = builder.build().expect("builder should succeed");
        assert_eq!(request.filename, "transcript.zip");
        assert_eq!(request.bytes, 1024);
        assert_eq!(request.mime_type, "application/zip");
        assert!(request.expires_after.is_some());
    }

    #[test]
    fn enforces_positive_bytes() {
        let builder = UploadBuilder::new("file", Purpose::Assistants, 0, "text/plain");
        let error = builder.build().expect_err("should fail validation");
        assert!(matches!(error, Error::InvalidRequest(message) if message.contains("positive")));
    }

    #[test]
    fn validates_expiration_range() {
        let builder = UploadBuilder::new("file", Purpose::Assistants, 1_024, "text/plain")
            .expires_after_seconds(10);
        let error = builder.build().expect_err("should enforce range");
        assert!(matches!(
            error,
            Error::InvalidRequest(message) if message.contains("3600")
        ));
    }
}
