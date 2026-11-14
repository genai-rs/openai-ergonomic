//! Responses API (`/responses`) methods.

use super::Client;
use crate::builders::Builder;
use crate::errors::Result;
use crate::responses_api::{
    BoxedResponsesStream, ResponsesBuilder, ResponsesResponse, ResponsesStream,
};
use crate::semantic_conventions::operation_names;
use crate::streaming::{InterceptedStream, StreamCompletion};
use crate::time::Instant;
use openai_client_base::apis::responses_api;
use openai_client_base::models::{CreateResponse, ResponseItemList};
use std::sync::Arc;

impl<T: Default + crate::MaybeSend + crate::MaybeSync + 'static> Client<T> {
    /// A Responses API request builder for the configured default model, as for `client.chat()`
    /// (`ConfigBuilder::default_model`, `OPENAI_DEFAULT_MODEL`, else `gpt-4`).
    pub fn responses(&self) -> ResponsesBuilder {
        let model = self
            .config
            .default_model()
            .unwrap_or(crate::config::DEFAULT_MODEL);
        ResponsesBuilder::new(model)
    }

    /// A Responses API request builder with a user message.
    pub fn responses_simple(&self, input: impl Into<String>) -> ResponsesBuilder {
        self.responses().user(input)
    }

    /// The Responses API client: create, retrieve, cancel and delete responses.
    #[must_use]
    pub fn responses_api(&self) -> ResponsesClient<'_, T> {
        ResponsesClient { client: self }
    }

    /// Create a response (`POST /responses`), with the generated `create_response` operation.
    ///
    /// A non-2xx status is returned as [`Error::Api`](crate::Error::Api) with its status and the `OpenAI` error's
    /// type and code (429 for rate limits: [`Error::is_rate_limit`](crate::Error::is_rate_limit)). A response that failed or
    /// stopped early on the model's side is a successful call: check
    /// [`ResponsesResponse::status`].
    pub async fn execute_responses(&self, request: CreateResponse) -> Result<ResponsesResponse> {
        let operation = operation_names::RESPONSES;
        let model = request.model.clone().unwrap_or_default();
        let request_json = serde_json::to_string(&request).unwrap_or_default();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let start_time = Instant::now();
        let response = match responses_api::create_response()
            .configuration(&self.base_configuration)
            .create_response(&request)
            .call()
            .await
        {
            Ok(response) => ResponsesResponse::new(response),
            Err(error) => {
                return Err(self
                    .handle_api_error(error, operation, &model, &request_json, &state)
                    .await);
            }
        };

        let usage = response.usage();
        self.call_after_response(
            &response,
            operation,
            &model,
            &request_json,
            &state,
            start_time.elapsed(),
            usage.map(|usage| usage.input_tokens),
            usage.map(|usage| usage.output_tokens),
        )
        .await;
        Ok(response)
    }

    /// Build and create a response (`POST /responses`). See [`Client::execute_responses`].
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let response = client
    ///     .send_responses(client.responses().instructions("Be brief").user("What is Rust?"))
    ///     .await?;
    /// println!("{}", response.output_text());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_responses(&self, builder: ResponsesBuilder) -> Result<ResponsesResponse> {
        self.execute_responses(builder.build()?).await
    }

    /// Create a response and stream its events (`POST /responses` with `stream: true`).
    ///
    /// The stream yields every event and ends after the terminal one (`response.completed`,
    /// `response.incomplete`, `response.failed` or `error`); see [`ResponsesStream`].
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// use openai_ergonomic::Client;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let mut events = client
    ///     .send_responses_stream(client.responses().user("Tell me a story"))
    ///     .await?;
    /// while let Some(event) = events.next().await {
    ///     if let Some(text) = event?.text_delta() {
    ///         print!("{text}");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_responses_stream(
        &self,
        builder: ResponsesBuilder,
    ) -> Result<BoxedResponsesStream> {
        self.send_responses_stream_with_completion(builder)
            .await
            .map(|(stream, _completion)| stream)
    }

    /// Stream a response and return a handle for waiting on interceptor callbacks.
    ///
    /// `StreamCompletion::wait` resolves after the stream ends, yields an error, or is dropped,
    /// and every accepted callback has returned or failed. If the consumer stops early, drop
    /// the stream before waiting so the terminal outcome is cancellation.
    pub async fn send_responses_stream_with_completion(
        &self,
        builder: ResponsesBuilder,
    ) -> Result<(BoxedResponsesStream, StreamCompletion)> {
        self.execute_responses_stream(builder.build()?).await
    }

    /// Stream a response from a request, with the generated `create_response_stream` operation.
    /// See [`Client::send_responses_stream_with_completion`].
    pub async fn execute_responses_stream(
        &self,
        mut request: CreateResponse,
    ) -> Result<(BoxedResponsesStream, StreamCompletion)> {
        let operation = operation_names::RESPONSES;
        request.stream = Some(Some(true));
        let model = request.model.clone().unwrap_or_default();
        let request_json = serde_json::to_string(&request).unwrap_or_default();
        let mut state = T::default();
        self.call_before_request(operation, &model, &request_json, &mut state)
            .await?;

        let events = match responses_api::create_response_stream()
            .configuration(&self.base_configuration)
            .create_response(&request)
            .call()
            .await
        {
            Ok(events) => events.into_events(),
            Err(error) => {
                return Err(self
                    .handle_api_error(error, operation, &model, &request_json, &state)
                    .await);
            }
        };
        let (stream, completion) = InterceptedStream::new_with_completion(
            crate::time::idle_timeout(
                ResponsesStream::from_events(events),
                self.config.stream_idle_timeout(),
            ),
            Arc::clone(&self.interceptors),
            operation.to_string(),
            model,
            request_json,
            state,
        )?;
        Ok((Box::pin(stream), completion))
    }
}

/// The Responses API: create, retrieve, cancel and delete responses, and list their input items.
///
/// Obtained with [`Client::responses_api`]. Creating a response is also available on the
/// client ([`Client::send_responses`], [`Client::send_responses_stream`]).
#[derive(Debug, Clone, Copy)]
pub struct ResponsesClient<'a, T = ()> {
    client: &'a Client<T>,
}

impl<T: Default + crate::MaybeSend + crate::MaybeSync + 'static> ResponsesClient<'_, T> {
    /// Create a response. See [`Client::send_responses`].
    pub async fn create(&self, builder: ResponsesBuilder) -> Result<ResponsesResponse> {
        self.client.send_responses(builder).await
    }

    /// Create a response and stream its events. See [`Client::send_responses_stream`].
    pub async fn create_stream(&self, builder: ResponsesBuilder) -> Result<BoxedResponsesStream> {
        self.client.send_responses_stream(builder).await
    }

    /// Retrieve a stored response (`GET /responses/{id}`).
    pub async fn retrieve(&self, response_id: &str) -> Result<ResponsesResponse> {
        responses_api::get_response()
            .configuration(&self.client.base_configuration)
            .response_id(response_id)
            .call()
            .await
            .map(ResponsesResponse::new)
            .map_err(super::map_api_error)
    }

    /// Cancel a background response (`POST /responses/{id}/cancel`).
    pub async fn cancel(&self, response_id: &str) -> Result<ResponsesResponse> {
        responses_api::cancel_response()
            .configuration(&self.client.base_configuration)
            .response_id(response_id)
            .call()
            .await
            .map(ResponsesResponse::new)
            .map_err(super::map_api_error)
    }

    /// The input items of a stored response (`GET /responses/{id}/input_items`): the first
    /// page (the API's default size), oldest first. See
    /// [`list_input_items_page`](Self::list_input_items_page).
    pub async fn list_input_items(&self, response_id: &str) -> Result<ResponseItemList> {
        self.list_input_items_page(response_id, None, None).await
    }

    /// A page of the input items of a stored response: at most `limit` items (1 to 100), after
    /// the item `after` (the `last_id` of the previous page), oldest first. `has_more` says
    /// whether another page follows.
    pub async fn list_input_items_page(
        &self,
        response_id: &str,
        limit: Option<u32>,
        after: Option<&str>,
    ) -> Result<ResponseItemList> {
        responses_api::list_input_items()
            .configuration(&self.client.base_configuration)
            .response_id(response_id)
            .maybe_limit(limit.map(i64::from))
            .order("asc")
            .maybe_after(after)
            .call()
            .await
            .map_err(super::map_api_error)
    }

    /// Delete a stored response (`DELETE /responses/{id}`); returns whether it was deleted.
    pub async fn delete(&self, response_id: &str) -> Result<bool> {
        responses_api::delete_response()
            .configuration(&self.client.base_configuration)
            .response_id(response_id)
            .call()
            .await
            .map(|deleted| deleted.deleted)
            .map_err(super::map_api_error)
    }
}
