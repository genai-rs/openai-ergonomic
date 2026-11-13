//! Large files through the Uploads API: create an upload, add its parts in parallel, complete
//! it.

use super::raw::check_status;
use super::Client;
use crate::{BoxFuture, Error, Result, UploadPurpose};
use bytes::{Bytes, BytesMut};
use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};
use http::Method;
use openai_client_base::models::{
    CompleteUploadRequest, CreateUploadRequest, Upload as UploadObject, UploadPart,
};
use std::future::IntoFuture;
use std::time::Duration;
use timvw_rust_runtime::multipart::{Form, Part};
use timvw_rust_runtime::{Body, Upload};

/// The largest part the Uploads API accepts: 64 MB.
pub const MAX_UPLOAD_PART_SIZE: u64 = 64_000_000;
/// How many parts are sent at the same time, by default.
pub const DEFAULT_UPLOAD_CONCURRENCY: usize = 4;

/// A file sent through the Uploads API in parts; see [`Client::upload_large_file`].
///
/// Await it (or call [`LargeUpload::send`]) to upload.
#[must_use = "a LargeUpload does nothing until awaited"]
pub struct LargeUpload<'a, T = ()> {
    client: &'a Client<T>,
    upload: Upload,
    filename: String,
    purpose: UploadPurpose,
    mime_type: String,
    part_size: u64,
    concurrency: usize,
    part_retries: usize,
    retry_backoff: Duration,
    md5: Option<String>,
}

impl<T> std::fmt::Debug for LargeUpload<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LargeUpload")
            .field("upload", &self.upload)
            .field("filename", &self.filename)
            .field("purpose", &self.purpose)
            .field("mime_type", &self.mime_type)
            .field("part_size", &self.part_size)
            .field("concurrency", &self.concurrency)
            .field("part_retries", &self.part_retries)
            .finish_non_exhaustive()
    }
}

impl<T> Client<T> {
    /// Uploads a large file (up to 8 GB) through the Uploads API: creates an upload, adds the
    /// file in parts of at most 64 MB, several at a time, and completes it. The result is the
    /// completed upload, whose `file` is the `File` object to use like any uploaded file.
    ///
    /// `upload` is anything that converts into a [`Upload`]: a path (`PathBuf`, `&Path`), bytes
    /// (`Vec<u8>`, `Bytes`), or an `Upload` made from a reader or a stream (give it its length
    /// with [`Upload::len`]: the Uploads API needs the size up front).
    ///
    /// - A file is read part by part, each part straight from its offset in the file
    ///   ([`Upload::slice`]), and bytes are sliced without a copy. A stream or a reader is read
    ///   once, in order, holding at most [`concurrency`](LargeUpload::concurrency) + 1 parts in
    ///   memory.
    /// - Each part is its own request, so the client's retry layer retries it on its own; on top
    ///   of that a part whose request fails with a transport error, a `408`, a `429` or a `5xx`
    ///   is sent again up to [`part_retries`](LargeUpload::part_retries) times.
    /// - When a part fails for good, the upload is cancelled and the error returned.
    ///
    /// Interceptors are not called for these requests.
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::{Client, UploadPurpose};
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let upload = client
    ///     .upload_large_file(
    ///         PathBuf::from("training.jsonl"),
    ///         "training.jsonl",
    ///         UploadPurpose::FineTune,
    ///         "application/jsonl",
    ///     )
    ///     .concurrency(8)
    ///     .await?;
    /// println!("file: {:?}", upload.file);
    /// # Ok(())
    /// # }
    /// ```
    pub fn upload_large_file(
        &self,
        upload: impl Into<Upload>,
        filename: impl Into<String>,
        purpose: UploadPurpose,
        mime_type: impl Into<String>,
    ) -> LargeUpload<'_, T> {
        LargeUpload {
            client: self,
            upload: upload.into(),
            filename: filename.into(),
            purpose,
            mime_type: mime_type.into(),
            part_size: MAX_UPLOAD_PART_SIZE,
            concurrency: DEFAULT_UPLOAD_CONCURRENCY,
            part_retries: 2,
            retry_backoff: Duration::from_millis(500),
            md5: None,
        }
    }
}

impl<T> LargeUpload<'_, T> {
    /// The size of each part but the last (default and maximum [`MAX_UPLOAD_PART_SIZE`]).
    pub fn part_size(mut self, bytes: u64) -> Self {
        self.part_size = bytes;
        self
    }

    /// How many parts are sent at the same time (default [`DEFAULT_UPLOAD_CONCURRENCY`]).
    pub fn concurrency(mut self, parts: usize) -> Self {
        self.concurrency = parts.max(1);
        self
    }

    /// How many more times a failed part is sent (default 2).
    pub fn part_retries(mut self, retries: usize) -> Self {
        self.part_retries = retries;
        self
    }

    /// The wait before a part is sent again (default 500 ms), doubled on each retry.
    pub fn retry_backoff(mut self, backoff: Duration) -> Self {
        self.retry_backoff = backoff;
        self
    }

    /// The MD5 checksum of the whole file, which the API checks when completing the upload.
    pub fn md5(mut self, md5: impl Into<String>) -> Self {
        self.md5 = Some(md5.into());
        self
    }

    /// Uploads the file; see [`Client::upload_large_file`].
    pub async fn send(self) -> Result<UploadObject> {
        if self.part_size == 0 || self.part_size > MAX_UPLOAD_PART_SIZE {
            return Err(Error::InvalidRequest(format!(
                "the part size must be between 1 and {MAX_UPLOAD_PART_SIZE} bytes, not {}",
                self.part_size
            )));
        }
        let upload = self.upload.clone().resolve().await?;
        let length = upload.content_length().ok_or_else(|| {
            Error::InvalidRequest(
                "the length of the upload is unknown: set it with `Upload::len`".to_string(),
            )
        })?;
        let request = CreateUploadRequest {
            filename: self.filename.clone(),
            purpose: self.purpose,
            bytes: i64::try_from(length)
                .map_err(|_| Error::InvalidRequest("the upload is too large".to_string()))?,
            mime_type: self.mime_type.clone(),
            expires_after: None,
        };
        let created: UploadObject = self.client.post_json("/uploads", &request).await?;

        match self.send_parts(&created.id, upload, length).await {
            Ok(part_ids) => {
                let complete = CompleteUploadRequest {
                    part_ids,
                    md5: self.md5.clone(),
                };
                self.client
                    .post_json(&format!("/uploads/{}/complete", created.id), &complete)
                    .await
            }
            Err(error) => {
                // Best effort: the upload expires anyway.
                let cancelled: Result<serde_json::Value> = self
                    .client
                    .post_json(&format!("/uploads/{}/cancel", created.id), &())
                    .await;
                if let Err(cancel_error) = cancelled {
                    tracing::debug!("failed to cancel upload {}: {cancel_error}", created.id);
                }
                Err(error)
            }
        }
    }

    /// Sends the parts, `concurrency` at a time, and returns their IDs in order.
    async fn send_parts(
        &self,
        upload_id: &str,
        upload: Upload,
        length: u64,
    ) -> Result<Vec<String>> {
        let parts = split(upload, length, self.part_size);
        parts
            .map_ok(|part| self.send_part(upload_id, part))
            .try_buffered(self.concurrency)
            .try_collect()
            .await
    }

    /// Sends one part, again after a transient failure.
    async fn send_part(&self, upload_id: &str, part: Upload) -> Result<String> {
        let path = format!("/uploads/{upload_id}/parts");
        let mut attempt = 0;
        loop {
            let form = Form::new().part("data", Part::upload(part.clone()));
            let sent = async {
                let response = self
                    .client
                    .raw_request(Method::POST, &path)
                    .multipart(form)
                    .send()
                    .await?;
                let part: UploadPart = check_status(response).await?.json().await?;
                Ok::<_, Error>(part.id)
            }
            .await;
            match sent {
                Ok(id) => return Ok(id),
                Err(error) if attempt < self.part_retries && is_transient(&error) => {
                    let backoff = self
                        .retry_backoff
                        .saturating_mul(1 << attempt.min(16))
                        .min(Duration::from_secs(30));
                    tracing::debug!(
                        "retrying a part of upload {upload_id} in {backoff:?}: {error}"
                    );
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl<'a, T: Send + Sync> IntoFuture for LargeUpload<'a, T> {
    type Output = Result<UploadObject>;
    type IntoFuture = BoxFuture<'a, Result<UploadObject>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// Whether sending a part again may succeed.
fn is_transient(error: &Error) -> bool {
    match error {
        Error::Http(error) => !error.is_builder(),
        Error::Api {
            status, error_code, ..
        } => {
            (*status == 408 || *status == 429 || *status >= 500)
                && error_code.as_deref() != Some("insufficient_quota")
        }
        _ => false,
    }
}

/// The parts of an upload of `length` bytes: slices of a source that can be read again (a file,
/// bytes), or chunks read in order from a stream. An empty upload has one empty part.
fn split(upload: Upload, length: u64, part_size: u64) -> BoxStream<'static, Result<Upload>> {
    if length == 0 {
        return stream::once(async { Ok(Upload::bytes(Bytes::new())) }).boxed();
    }
    if upload.is_replayable() {
        let count = length.div_ceil(part_size);
        return stream::iter((0..count).map(move |index| {
            let start = index * part_size;
            let end = (start + part_size).min(length);
            Ok(upload.clone().slice(start..end))
        }))
        .boxed();
    }
    let response = timvw_rust_runtime::Response::from(http::Response::new(Body::from(upload)));
    let part_size = usize::try_from(part_size).unwrap_or(usize::MAX);
    stream::try_unfold(
        (
            response.bytes_stream().boxed(),
            BytesMut::new(),
            0u64,
            false,
        ),
        move |(mut chunks, mut buffer, mut read, mut done)| async move {
            while !done && buffer.len() < part_size {
                match chunks.next().await {
                    Some(chunk) => {
                        let chunk = chunk?;
                        read += chunk.len() as u64;
                        buffer.extend_from_slice(&chunk);
                    }
                    None => done = true,
                }
            }
            if read > length || (done && read < length) {
                return Err(Error::InvalidRequest(format!(
                    "the upload has {read} bytes{}, not the {length} it was declared with",
                    if done { "" } else { " or more" }
                )));
            }
            if buffer.is_empty() {
                return Ok(None);
            }
            let part = buffer.split_to(buffer.len().min(part_size)).freeze();
            Ok(Some((Upload::bytes(part), (chunks, buffer, read, done))))
        },
    )
    .boxed()
}
