//! Assistants API client.

use super::{map_api_error, Client};
use crate::builders::assistants::{AssistantBuilder, MessageBuilder, RunBuilder};
use crate::errors::Result;
use crate::interceptor::{AfterResponseContext, BeforeRequestContext, ErrorContext};
use crate::semantic_conventions::operation_names;
use crate::Error;
use openai_client_base::apis::assistants_api;
use openai_client_base::models::{
    AssistantObject, DeleteAssistantResponse, ListAssistantsResponse, ListMessagesResponse,
    ListRunStepsResponse, ListRunsResponse, MessageObject, RunObject, RunStepObject,
    SubmitToolOutputsRunRequestToolOutputsInner,
};
use std::time::Instant;

/// Client for assistants API.
#[derive(Debug, Clone, Copy)]
pub struct AssistantsClient<'a, T = ()> {
    pub(super) client: &'a Client<T>,
}

impl_interceptor_helpers!(AssistantsClient<'_, T>);

impl<T: Default + Send + Sync> AssistantsClient<'_, T> {
    /// Create a new assistant.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::AssistantBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("Math Tutor")
    ///     .instructions("You are a helpful math tutor.");
    /// let assistant = client.assistants().create(builder).await?;
    /// println!("Created assistant: {}", assistant.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, builder: AssistantBuilder) -> Result<AssistantObject> {
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_CREATE;
        let model = request.model.clone();
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_assistant()
            .configuration(&self.client.base_configuration)
            .create_assistant_request(request)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// List assistants with pagination.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.assistants().list(Some(20), None, None, None).await?;
    /// println!("Found {} assistants", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListAssistantsResponse> {
        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_LIST;
        let model = "assistants";
        let request_json = format!(
            "{{\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_assistants()
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

    /// Get an assistant by ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let assistant = client.assistants().get("asst_123").await?;
    /// println!("Assistant: {}", assistant.name.unwrap_or_default());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, assistant_id: impl Into<String>) -> Result<AssistantObject> {
        let id = assistant_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_RETRIEVE;
        let model = "assistants";
        let request_json = format!("{{\"assistant_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
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

    /// Update an assistant.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::AssistantBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = AssistantBuilder::new("gpt-4")
    ///     .name("Updated Name")
    ///     .instructions("Updated instructions");
    /// let assistant = client.assistants().update("asst_123", builder).await?;
    /// println!("Updated: {}", assistant.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        assistant_id: impl Into<String>,
        builder: AssistantBuilder,
    ) -> Result<AssistantObject> {
        use openai_client_base::models::ModifyAssistantRequest;

        let id = assistant_id.into();
        let request_data = builder.build()?;

        // Convert CreateAssistantRequest to ModifyAssistantRequest
        let mut request = ModifyAssistantRequest::new();
        request.model = Some(request_data.model);
        // Convert Box<CreateAssistantRequestName> to Option<String> by extracting text
        request.name = request_data.name.and_then(|n| match *n {
            openai_client_base::models::CreateAssistantRequestName::Text(text) => Some(Some(text)),
            openai_client_base::models::CreateAssistantRequestName::Null => None,
        });
        request.description = request_data.description.and_then(|d| match *d {
            openai_client_base::models::CreateAssistantRequestDescription::Text(text) => {
                Some(Some(text))
            }
            openai_client_base::models::CreateAssistantRequestDescription::Null => None,
        });
        request.instructions = request_data.instructions.and_then(|i| match *i {
            openai_client_base::models::CreateAssistantRequestInstructions::Text(text) => {
                Some(Some(text))
            }
            openai_client_base::models::CreateAssistantRequestInstructions::Null => None,
        });
        request.tools = request_data.tools;
        request.metadata = request_data.metadata;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_UPDATE;
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "assistants".to_string(), Clone::clone);
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::modify_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
            .modify_assistant_request(request)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// Delete an assistant.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.assistants().delete("asst_123").await?;
    /// println!("Deleted: {}", response.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, assistant_id: impl Into<String>) -> Result<DeleteAssistantResponse> {
        let id = assistant_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::ASSISTANT_DELETE;
        let model = "assistants";
        let request_json = format!("{{\"assistant_id\":\"{id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::delete_assistant()
            .configuration(&self.client.base_configuration)
            .assistant_id(&id)
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

    /// Create a run on a thread.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::RunBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = RunBuilder::new("asst_123");
    /// let run = client.assistants().create_run("thread_123", builder).await?;
    /// println!("Run created: {}", run.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_run(
        &self,
        thread_id: impl Into<String>,
        builder: RunBuilder,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_CREATE;
        let model = request
            .model
            .as_ref()
            .map_or_else(|| "runs".to_string(), Clone::clone);
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .create_run_request(request)
            .call()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error = self
                    .handle_api_error(e, operation, &model, &request_json, &state)
                    .await;
                return Err(error);
            }
        };

        let duration = start_time.elapsed();

        // Call after_response hook
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            duration,
            None,
            None,
        )
        .await;

        Ok(response)
    }

    /// List runs on a thread.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.assistants().list_runs("thread_123", None, None, None, None).await?;
    /// println!("Found {} runs", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_runs(
        &self,
        thread_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
    ) -> Result<ListRunsResponse> {
        let thread_id = thread_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_LIST;
        let model = "runs";
        let request_json = format!(
            "{{\"thread_id\":\"{thread_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_runs()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
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

    /// Get a run.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let run = client.assistants().get_run("thread_123", "run_123").await?;
    /// println!("Run status: {:?}", run.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_run(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_RETRIEVE;
        let model = "runs";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
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

    /// Cancel a run.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let run = client.assistants().cancel_run("thread_123", "run_123").await?;
    /// println!("Run cancelled: {:?}", run.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_run(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> Result<RunObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_CANCEL;
        let model = "runs";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::cancel_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
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

    /// Submit tool outputs to a run.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_client_base::models::SubmitToolOutputsRunRequestToolOutputsInner;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let outputs = vec![
    ///     SubmitToolOutputsRunRequestToolOutputsInner {
    ///         tool_call_id: Some("call_123".to_string()),
    ///         output: Some("output data".to_string()),
    ///     }
    /// ];
    /// let run = client.assistants().submit_tool_outputs("thread_123", "run_123", outputs).await?;
    /// println!("Tool outputs submitted: {}", run.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn submit_tool_outputs(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        tool_outputs: Vec<SubmitToolOutputsRunRequestToolOutputsInner>,
    ) -> Result<RunObject> {
        use openai_client_base::models::SubmitToolOutputsRunRequest;

        let thread_id = thread_id.into();
        let run_id = run_id.into();
        let request = SubmitToolOutputsRunRequest::new(tool_outputs);

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_SUBMIT_TOOL_OUTPUTS;
        let model = "runs";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::submit_tool_ouputs_to_run()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .submit_tool_outputs_run_request(request)
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

    /// Create a message on a thread.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use openai_ergonomic::builders::assistants::MessageBuilder;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let builder = MessageBuilder::new("user", "Hello, assistant!");
    /// let message = client.assistants().create_message("thread_123", builder).await?;
    /// println!("Message created: {}", message.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_message(
        &self,
        thread_id: impl Into<String>,
        builder: MessageBuilder,
    ) -> Result<MessageObject> {
        let thread_id = thread_id.into();
        let request = builder.build()?;

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_CREATE;
        let model = "messages";
        let request_json = serde_json::to_string(&request).unwrap_or_default();

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::create_message()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .create_message_request(request)
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

    /// List messages on a thread.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.assistants().list_messages("thread_123", None, None, None, None, None).await?;
    /// println!("Found {} messages", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_messages(
        &self,
        thread_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        run_id: Option<&str>,
    ) -> Result<ListMessagesResponse> {
        let thread_id = thread_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_LIST;
        let model = "messages";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?},\"run_id\":{run_id:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_messages()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_run_id(run_id)
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

    /// Get a message.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let message = client.assistants().get_message("thread_123", "msg_123").await?;
    /// println!("Message role: {:?}", message.role);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_message(
        &self,
        thread_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> Result<MessageObject> {
        let thread_id = thread_id.into();
        let message_id = message_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::MESSAGE_RETRIEVE;
        let model = "messages";
        let request_json =
            format!("{{\"thread_id\":\"{thread_id}\",\"message_id\":\"{message_id}\"}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_message()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .message_id(&message_id)
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

    /// List run steps.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client.assistants().list_run_steps("thread_123", "run_123", None, None, None, None, None).await?;
    /// println!("Found {} run steps", response.data.len());
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn list_run_steps(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        limit: Option<i32>,
        order: Option<&str>,
        after: Option<&str>,
        before: Option<&str>,
        include: Option<Vec<String>>,
    ) -> Result<ListRunStepsResponse> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_STEP_LIST;
        let model = "run_steps";
        let request_json = format!("{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\",\"limit\":{limit:?},\"order\":{order:?},\"after\":{after:?},\"before\":{before:?},\"include\":{include:?}}}");

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::list_run_steps()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .maybe_limit(limit)
            .maybe_order(order)
            .maybe_after(after)
            .maybe_before(before)
            .maybe_include_left_square_bracket_right_square_bracket(include)
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

    /// Get a run step.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let step = client.assistants().get_run_step("thread_123", "run_123", "step_123", None).await?;
    /// println!("Step type: {:?}", step.r#type);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_run_step(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        step_id: impl Into<String>,
        include: Option<Vec<String>>,
    ) -> Result<RunStepObject> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();
        let step_id = step_id.into();

        // Prepare interceptor context
        let mut state = T::default();
        let operation = operation_names::RUN_STEP_RETRIEVE;
        let model = "run_steps";
        let request_json = format!(
            "{{\"thread_id\":\"{thread_id}\",\"run_id\":\"{run_id}\",\"step_id\":\"{step_id}\",\"include\":{include:?}}}"
        );

        // Call before_request hook
        self.call_before_request(operation, model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();

        // Make the API call
        let response = match assistants_api::get_run_step()
            .configuration(&self.client.base_configuration)
            .thread_id(&thread_id)
            .run_id(&run_id)
            .step_id(&step_id)
            .maybe_include_left_square_bracket_right_square_bracket(include)
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
