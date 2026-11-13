//! Files API client.

use super::{map_api_error, Client};
use crate::builders::files::{
    FileDeleteBuilder, FileListBuilder, FileRetrievalBuilder, FileUploadBuilder,
};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::files_api;
use openai_client_base::models::{DeleteFileResponse, ListFilesResponse, OpenAiFile};
use std::time::Instant;

/// Client for files API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FilesClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(FilesClient<'_, T>);

impl<T: Default + Send + Sync> FilesClient<'_, T> {
    /// Upload a file to `OpenAI`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::files::FilePurpose;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client
    ///     .files()
    ///     .upload_text("training.jsonl", FilePurpose::FineTune, "training data");
    /// let file = client.files().create(builder).await?;
    /// println!("Uploaded file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(&self, builder: FileUploadBuilder) -> Result<OpenAiFile> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_UPLOAD;
        let model = "file-upload"; // No model for file operations
        let request_json = serde_json::json!({
            "filename": builder.filename(),
            "purpose": builder.purpose().to_string(),
            "size": builder.content_size(),
        })
        .to_string();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let (upload, purpose) = builder.into_parts();
        let purpose = purpose.to_string();
        let start_time = Instant::now();

        // Make the API call: the upload is streamed as it is (a path from disk), no temporary
        // copy. A missing file is an `Error::Http` whose `is_io()` is true.
        let result = match files_api::create_file()
            .configuration(&self.client.base_configuration)
            .file(upload)
            .purpose(&purpose)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &result,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(result)
    }

    /// Convenience method to upload a file (alias for upload).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::files::FilePurpose;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client
    ///     .files()
    ///     .upload_text("data.txt", FilePurpose::Assistants, "content");
    /// let file = client.files().create(builder).await?;
    /// println!("File ID: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: FileUploadBuilder) -> Result<OpenAiFile> {
        self.upload(builder).await
    }

    /// Create a file upload builder from text content.
    #[must_use]
    pub fn upload_text(
        &self,
        filename: impl Into<String>,
        purpose: crate::builders::files::FilePurpose,
        text: impl Into<String>,
    ) -> FileUploadBuilder {
        FileUploadBuilder::from_text(filename, purpose, text)
    }

    /// Create a file upload builder from bytes.
    #[must_use]
    pub fn upload_bytes(
        &self,
        filename: impl Into<String>,
        purpose: crate::builders::files::FilePurpose,
        content: impl Into<bytes::Bytes>,
    ) -> FileUploadBuilder {
        FileUploadBuilder::new(filename, purpose, content)
    }

    /// Create a file upload builder from a file path. The file is streamed from disk when the
    /// upload is sent; a missing file fails it with [`Error::Http`] (`is_io()`).
    #[must_use]
    pub fn upload_from_path(
        &self,
        path: impl AsRef<std::path::Path>,
        purpose: crate::builders::files::FilePurpose,
    ) -> FileUploadBuilder {
        FileUploadBuilder::from_path(path, purpose)
    }

    /// Create a file upload builder from anything that converts into an
    /// [`Upload`](timvw_rust_runtime::Upload): a path, bytes, or an upload made from a stream.
    #[must_use]
    pub fn upload_from(
        &self,
        upload: impl Into<timvw_rust_runtime::Upload>,
        purpose: crate::builders::files::FilePurpose,
    ) -> FileUploadBuilder {
        FileUploadBuilder::from_upload(upload, purpose)
    }

    /// List files.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = client.files().list_builder();
    /// let files = client.files().list(builder).await?;
    /// println!("Found {} files", files.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, builder: FileListBuilder) -> Result<ListFilesResponse> {
        let purpose = builder.purpose_ref().map(ToString::to_string);
        let limit = builder.limit_ref();
        let order = builder.order_ref().map(ToString::to_string);

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_LIST;
        let model = "files";
        let request_json = format!(
            r#"{{"purpose":"{}","limit":{},"order":"{}"}}"#,
            purpose.as_deref().unwrap_or(""),
            limit.unwrap_or(10000),
            order.as_deref().unwrap_or("desc")
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::list_files()
            .configuration(&self.client.base_configuration)
            .maybe_purpose(purpose.as_deref())
            .maybe_limit(limit)
            .maybe_order(order.as_deref())
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Create a list files builder.
    #[must_use]
    pub fn list_builder(&self) -> FileListBuilder {
        FileListBuilder::new()
    }

    /// Retrieve information about a specific file.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let file = client.files().retrieve("file-123").await?;
    /// println!("File: {} ({})", file.filename, file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, file_id: impl Into<String>) -> Result<OpenAiFile> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_RETRIEVE;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::retrieve_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Retrieve information about a file using a builder.
    pub async fn get(&self, builder: FileRetrievalBuilder) -> Result<OpenAiFile> {
        self.retrieve(builder.file_id()).await
    }

    /// Download file content.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let content = client.files().download("file-123").await?;
    /// println!("Downloaded {} bytes", content.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download(&self, file_id: impl Into<String>) -> Result<String> {
        let content = self.download_bytes(file_id).await?;
        Ok(String::from_utf8_lossy(&content).into_owned())
    }

    /// Download file content as bytes without decoding the response body.
    pub async fn download_bytes(&self, file_id: impl Into<String>) -> Result<Vec<u8>> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_DOWNLOAD;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::download_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let response = response.bytes().await?.to_vec();
        let duration = start_time.elapsed();

        // Call after_response hook
        let response_size = format!(r#"{{"size":{}}}"#, response.len());
        self.call_after_response(
            &response_size,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Delete a file.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.files().delete("file-123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, file_id: impl Into<String>) -> Result<DeleteFileResponse> {
        let file_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_DELETE;
        let model = "files";
        let request_json = format!(r#"{{"file_id":"{file_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match files_api::delete_file()
            .configuration(&self.client.base_configuration)
            .file_id(&file_id)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Delete a file using a builder.
    pub async fn remove(&self, builder: FileDeleteBuilder) -> Result<DeleteFileResponse> {
        self.delete(builder.file_id()).await
    }
}
