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
        // Write content to a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(builder.filename());
        std::fs::write(&temp_file_path, builder.content()).map_err(Error::File)?;

        // Convert FilePurpose to a string for the API
        let purpose = builder.purpose().to_string();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::FILE_UPLOAD;
        let model = "file-upload"; // No model for file operations
        let request_json = format!(
            r#"{{"filename":"{}","purpose":"{}","size":{}}}"#,
            builder.filename(),
            builder.purpose(),
            builder.content().len()
        );

        // Call before_request hook
        if let Err(e) = self
            .call_before_request(operation, model, &request_json, &mut state)
            .await
        {
            // Clean up temp file before returning
            let _ = std::fs::remove_file(&temp_file_path);
            return Err(e);
        }

        let start_time = Instant::now();

        // Make the API call
        let result = match files_api::create_file()
            .configuration(&self.client.base_configuration)
            .file(temp_file_path.clone())
            .purpose(&purpose)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Clean up temp file
                let _ = std::fs::remove_file(&temp_file_path);
                let error = self
                    .handle_api_error(e, operation, model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        // Clean up temporary file
        let _ = std::fs::remove_file(temp_file_path);

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
        content: Vec<u8>,
    ) -> FileUploadBuilder {
        FileUploadBuilder::new(filename, purpose, content)
    }

    /// Create a file upload builder from a file path.
    pub fn upload_from_path(
        &self,
        path: impl AsRef<std::path::Path>,
        purpose: crate::builders::files::FilePurpose,
    ) -> Result<FileUploadBuilder> {
        FileUploadBuilder::from_path(path, purpose).map_err(Error::File)
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
