//! Vector stores API client.

use super::{map_api_error, Client};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::vector_stores_api;
use openai_client_base::models::{
    DeleteVectorStoreFileResponse, DeleteVectorStoreResponse, ListVectorStoreFilesResponse,
    ListVectorStoresResponse, VectorStoreFileObject, VectorStoreObject,
    VectorStoreSearchResultsPage,
};
use std::time::Instant;

/// Client for vector stores API.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct VectorStoresClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(VectorStoresClient<'_, T>);

impl<T: Default + Send + Sync> VectorStoresClient<'_, T> {
    /// Create a new vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = VectorStoreBuilder::new()
    ///     .name("My Knowledge Base")
    ///     .add_file("file-123");
    /// let vector_store = client.vector_stores().create(builder).await?;
    /// println!("Created vector store: {}", vector_store.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        builder: crate::builders::vector_stores::VectorStoreBuilder,
    ) -> Result<VectorStoreObject> {
        use openai_client_base::models::{CreateVectorStoreRequest, VectorStoreExpirationAfter};

        let mut request = CreateVectorStoreRequest::new();
        request.name = builder.name_ref().map(String::from);
        request.file_ids = if builder.has_files() {
            Some(builder.file_ids_ref().to_vec())
        } else {
            None
        };

        if let Some(expires_after) = builder.expires_after_ref() {
            use openai_client_base::models::vector_store_expiration_after::Anchor;
            request.expires_after = Some(Box::new(VectorStoreExpirationAfter::new(
                Anchor::LastActiveAt,
                expires_after.days,
            )));
        }

        if !builder.metadata_ref().is_empty() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_CREATE;
        let model = "vector-store";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::create_vector_store()
            .configuration(&self.client.base_configuration)
            .create_vector_store_request(request)
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

    /// List vector stores.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.vector_stores().list(Some(20), None, None, None).await?;
    /// println!("Found {} vector stores", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListVectorStoresResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_LIST;
        let model = "vector-store";
        let request_json = format!(
            r#"{{"limit":{},"order":"{}"}}"#,
            limit.unwrap_or(20),
            order.unwrap_or("desc")
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::list_vector_stores()
            .configuration(&self.client.base_configuration)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
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

    /// Get a specific vector store by ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let vector_store = client.vector_stores().get("vs_123").await?;
    /// println!("Vector store: {}", vector_store.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, vector_store_id: impl Into<String>) -> Result<VectorStoreObject> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_RETRIEVE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::get_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
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

    /// Update a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = VectorStoreBuilder::new()
    ///     .name("Updated Name")
    ///     .metadata("updated", "true");
    /// let vector_store = client.vector_stores().update("vs_123", builder).await?;
    /// println!("Updated: {}", vector_store.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        vector_store_id: impl Into<String>,
        builder: crate::builders::vector_stores::VectorStoreBuilder,
    ) -> Result<VectorStoreObject> {
        use openai_client_base::models::{UpdateVectorStoreRequest, VectorStoreExpirationAfter};

        let id = vector_store_id.into();
        let mut request = UpdateVectorStoreRequest::new();
        request.name = builder.name_ref().map(|name| Some(String::from(name)));

        if let Some(expires_after) = builder.expires_after_ref() {
            use openai_client_base::models::vector_store_expiration_after::Anchor;
            request.expires_after = Some(Box::new(VectorStoreExpirationAfter::new(
                Anchor::LastActiveAt,
                expires_after.days,
            )));
        }

        if !builder.metadata_ref().is_empty() {
            request.metadata = Some(Some(builder.metadata_ref().clone()));
        }

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_UPDATE;
        let model = "vector-store";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::modify_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
            .update_vector_store_request(request)
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

    /// Delete a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.vector_stores().delete("vs_123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(
        &self,
        vector_store_id: impl Into<String>,
    ) -> Result<DeleteVectorStoreResponse> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_DELETE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::delete_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
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

    /// Add a file to a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let file = client.vector_stores().add_file("vs_123", "file-456").await?;
    /// println!("Added file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<VectorStoreFileObject> {
        use openai_client_base::models::CreateVectorStoreFileRequest;

        let vs_id = vector_store_id.into();
        let f_id = file_id.into();
        let request = CreateVectorStoreFileRequest::new(f_id.clone());

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_ADD;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::create_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .create_vector_store_file_request(request)
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

    /// List files in a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.vector_stores().list_files("vs_123", None, None, None, None, None).await?;
    /// println!("Found {} files", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_files(
        &self,
        vector_store_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        filter: Option<&str>,
    ) -> Result<ListVectorStoreFilesResponse> {
        let id = vector_store_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_LIST;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::list_vector_store_files()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_filter(filter)
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

    /// Get a file from a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let file = client.vector_stores().get_file("vs_123", "file-456").await?;
    /// println!("File: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<VectorStoreFileObject> {
        let vs_id = vector_store_id.into();
        let f_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_RETRIEVE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::get_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .file_id(&f_id)
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

    /// Delete a file from a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.vector_stores().delete_file("vs_123", "file-456").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_file(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Result<DeleteVectorStoreFileResponse> {
        let vs_id = vector_store_id.into();
        let f_id = file_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_FILE_DELETE;
        let model = "vector-store";
        let request_json = format!(r#"{{"vector_store_id":"{vs_id}","file_id":"{f_id}"}}"#);

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::delete_vector_store_file()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .file_id(&f_id)
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

    /// Search a vector store.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::vector_stores::VectorStoreSearchBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = VectorStoreSearchBuilder::new("vs_123", "machine learning concepts");
    /// let results = client.vector_stores().search(builder).await?;
    /// println!("Found {} results", results.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        builder: crate::builders::vector_stores::VectorStoreSearchBuilder,
    ) -> Result<VectorStoreSearchResultsPage> {
        use openai_client_base::models::{VectorStoreSearchRequest, VectorStoreSearchRequestQuery};

        let query = VectorStoreSearchRequestQuery::new_text(builder.query().to_string());
        let mut request = VectorStoreSearchRequest::new(query);

        if let Some(limit) = builder.limit_ref() {
            request.max_num_results = Some(limit);
        }

        let vs_id = builder.vector_store_id().to_string();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::VECTOR_STORE_SEARCH;
        let model = "vector-store";
        let request_json = format!(
            r#"{{"vector_store_id":"{}","query":"{}"}}"#,
            vs_id,
            builder.query()
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match vector_stores_api::search_vector_store()
            .configuration(&self.client.base_configuration)
            .vector_store_id(&vs_id)
            .vector_store_search_request(request)
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
}
