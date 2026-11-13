//! Saving generated media to files, and uploading files without a temporary copy.

use super::raw::check_status;
use super::{AudioClient, FilesClient, ImagesClient};
use crate::builders::audio::SpeechBuilder;
use crate::builders::files::FilePurpose;
use crate::builders::Builder;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::{Error, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use http::Method;
use openai_client_base::models::{Image, ImagesResponse, OpenAiFile};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use timvw_rust_runtime::multipart::{Form, Part};
use timvw_rust_runtime::Upload;

impl<T: Default + Send + Sync> ImagesClient<'_, T> {
    /// Saves a generated image to `path` and returns the number of bytes written: decodes its
    /// `b64_json`, or downloads its `url`.
    ///
    /// A download goes through the client's HTTP stack (retries, timeouts), streamed to a
    /// temporary file that replaces `path` once complete. The provider's credentials are only
    /// sent to the provider's base URL, so not to the image host.
    pub async fn save(&self, image: &Image, path: impl AsRef<Path>) -> Result<u64> {
        let path = path.as_ref();
        if let Some(encoded) = image.b64_json.as_deref() {
            let bytes = BASE64
                .decode(encoded.trim())
                .map_err(|e| Error::InvalidRequest(format!("the image is not base64: {e}")))?;
            write_atomically(path, &bytes).await?;
            return Ok(bytes.len() as u64);
        }
        let Some(url) = image.url.as_deref() else {
            return Err(Error::InvalidRequest(
                "the image has neither `b64_json` nor `url`".to_string(),
            ));
        };
        let response = self.client.http.request(Method::GET, url).send().await?;
        Ok(check_status(response).await?.save_to(path).await?)
    }

    /// Saves every image of a response to `directory`, as `{stem}-{index}.{extension}` (the
    /// extension of the response's `output_format`, `png` by default), and returns their paths.
    pub async fn save_all(
        &self,
        response: &ImagesResponse,
        directory: impl AsRef<Path>,
        stem: &str,
    ) -> Result<Vec<PathBuf>> {
        let extension = response
            .output_format
            .and_then(|format| serde_json::to_value(format).ok())
            .and_then(|value| value.as_str().map(str::to_string))
            .filter(|format| format != "unknown")
            .unwrap_or_else(|| "png".to_string());
        let mut paths = Vec::new();
        for (index, image) in response.data.iter().flatten().enumerate() {
            let path = directory
                .as_ref()
                .join(format!("{stem}-{index}.{extension}"));
            self.save(image, &path).await?;
            paths.push(path);
        }
        Ok(paths)
    }
}

impl<T: Default + Send + Sync> AudioClient<'_, T> {
    /// Synthesizes speech and streams the audio to `path` as it arrives, without holding it in
    /// memory; returns the number of bytes written.
    ///
    /// The audio goes to a temporary file that replaces `path` once complete. Interceptors are
    /// called as for [`AudioClient::create_speech`].
    pub async fn speech_to_file(
        &self,
        builder: SpeechBuilder,
        path: impl AsRef<Path>,
    ) -> Result<u64> {
        let request = builder.build()?;
        let mut state = T::default();
        let operation = operation_names::AUDIO_SPEECH;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();
        let interceptors = &self.client.interceptors;
        if !interceptors.is_empty() {
            let mut ctx = BeforeRequestContext {
                operation,
                model: &model,
                request_json: &request_json,
                state: &mut state,
            };
            interceptors.before_request(&mut ctx).await?;
        }
        let start_time = Instant::now();
        let written = async {
            let response = self
                .client
                .raw_request(Method::POST, "/audio/speech")
                .json(&request)
                .send()
                .await?;
            Ok::<_, Error>(check_status(response).await?.save_to(path).await?)
        }
        .await;
        let written = match written {
            Ok(written) => written,
            Err(error) => {
                if !interceptors.is_empty() {
                    let ctx = ErrorContext {
                        operation,
                        model: Some(&model),
                        request_json: Some(&request_json),
                        error: &error,
                        state: Some(&state),
                    };
                    interceptors.on_error(&ctx).await;
                }
                return Err(error);
            }
        };
        if !interceptors.is_empty() {
            let response_json = format!("{{\"size\": {written}}}");
            let ctx = AfterResponseContext {
                operation,
                model: &model,
                request_json: &request_json,
                response_json: &response_json,
                duration: start_time.elapsed(),
                input_tokens: None,
                output_tokens: None,
                state: &state,
            };
            if let Err(error) = interceptors.after_response(&ctx).await {
                tracing::warn!("Interceptor after_response failed: {}", error);
            }
        }
        Ok(written)
    }
}

impl<T: Default + Send + Sync> FilesClient<'_, T> {
    /// Uploads a file from anything that converts into an [`Upload`]: a path (streamed from
    /// disk, named after its last component), bytes, a reader or a stream. Name an upload of
    /// bytes with [`Upload::file_name`]: the API uses the extension.
    ///
    /// Unlike [`FilesClient::upload`] and [`FilesClient::create`], nothing is copied to a temporary file. Interceptors are
    /// not called. For files over 512 MB, use [`Client::upload_large_file`](crate::Client::upload_large_file).
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::builders::files::FilePurpose;
    /// use openai_ergonomic::Client;
    /// use std::path::PathBuf;
    /// use timvw_rust_runtime::Upload;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let file = client
    ///     .files()
    ///     .upload_file(PathBuf::from("data/batch.jsonl"), FilePurpose::Batch)
    ///     .await?;
    /// let notes = Upload::bytes(b"hello".to_vec()).file_name("notes.txt").mime("text/plain");
    /// let other = client.files().upload_file(notes, FilePurpose::Assistants).await?;
    /// # let _ = (file, other);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file(
        &self,
        upload: impl Into<Upload>,
        purpose: FilePurpose,
    ) -> Result<OpenAiFile> {
        let upload = upload.into().resolve().await?;
        let form = Form::new()
            .text("purpose", purpose.to_string())
            .part("file", Part::upload(upload));
        let response = self
            .client
            .raw_request(Method::POST, "/files")
            .multipart(form)
            .send()
            .await?;
        Ok(check_status(response).await?.json().await?)
    }

    /// Streams the content of a file to `path`, without holding it in memory; returns the
    /// number of bytes written. The content goes to a temporary file that replaces `path` once
    /// complete. Interceptors are not called.
    pub async fn download_to(&self, file_id: &str, path: impl AsRef<Path>) -> Result<u64> {
        let response = self
            .client
            .raw_request(Method::GET, &format!("/files/{file_id}/content"))
            .send()
            .await?;
        Ok(check_status(response).await?.save_to(path).await?)
    }
}

/// Writes `bytes` to a temporary file next to `path` and renames it to `path`.
async fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .map_or_else(|| "image".into(), |name| name.to_string_lossy());
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_file_name(format!(".{name}.{}-{unique}.part", std::process::id()));
    if let Err(error) = tokio::fs::write(&temporary, bytes).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(Error::File(error));
    }
    if let Err(error) = tokio::fs::rename(&temporary, path).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(Error::File(error));
    }
    Ok(())
}
