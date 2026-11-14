//! "Bring your own types": send requests and decode responses with types of your own.
//!
//! The ergonomic builders and the generated `openai-client-base` operations use the `OpenAI`
//! spec's models. These methods send any serializable request body to any path under the
//! configured API base and decode the reply into any deserializable type: a struct with only
//! the fields you need, `serde_json::Value`, or the types of an `OpenAI`-compatible provider
//! whose replies differ from the spec.
//!
//! The requests go through the same HTTP client as every other call, so its tower layers apply
//! (retries, timeouts, the provider's layer with its credentials, headers and paths, the
//! [`AuthProvider`](crate::auth::AuthProvider), layers of your own), and they carry the same
//! user agent.
//!
//! Interceptors (such as the Langfuse interceptor) are **not** called for these requests.

use super::Client;
use crate::{Error, MaybeSend, Result};
use futures::stream::{Stream, StreamExt};
use http::header::{ACCEPT, USER_AGENT};
use http::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use timvw_rust_runtime::{RequestBuilder, Response};

impl<T> Client<T> {
    /// Start a request to `{api_base}{path}` with the client's authentication and user agent.
    pub(super) fn raw_request(&self, method: Method, path: &str) -> RequestBuilder {
        let configuration = &self.base_configuration;
        let base = configuration.base_path.trim_end_matches('/');
        let url = if path.starts_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        };
        let mut request = self.http.request(method, &url);
        if let Some(user_agent) = &configuration.user_agent {
            request = request.header(USER_AGENT, user_agent.as_str());
        }
        // The provider's layer adds the credentials.
        request
    }

    /// Send a JSON request to `{api_base}{path}` and decode the JSON reply into `O`.
    ///
    /// `path` is relative to the configured API base (for `OpenAI`, `https://api.openai.com/v1`),
    /// e.g. `"/chat/completions"`. `body`, if any, is serialized as JSON. A reply with a non-2xx
    /// status becomes [`Error::Api`] with the status and the body text as message (and the
    /// `type` and `code` of an `OpenAI` `{"error": {...}}` body, when present). A reply that does
    /// not decode into `O` becomes [`Error::Http`] (whose `is_decode()` is true).
    ///
    /// The request goes through the configured HTTP client and its layers (retries, the
    /// provider's layer, the organization and project headers), with the same authentication as
    /// the other calls. Interceptors are not called.
    ///
    /// For the common cases, see [`Client::post_json`] and [`Client::get_json`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use http::Method;
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let deleted: serde_json::Value = client
    ///     .send_json(Method::DELETE, "/files/file-abc123", None::<&()>)
    ///     .await?;
    /// println!("{deleted}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_json<I, O>(
        &self,
        method: Method,
        path: &str,
        body: Option<&I>,
    ) -> impl Future<Output = Result<O>> + MaybeSend + 'static
    where
        I: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        // The request (and its JSON body) is built before the future, so that the future
        // borrows neither `self` nor `body`.
        let mut request = self
            .raw_request(method, path)
            .header(ACCEPT, "application/json");
        if let Some(body) = body {
            request = request.json(body);
        }
        async move {
            let response = check_status(request.send().await?).await?;
            Ok(response.json::<O>().await?)
        }
    }

    /// `POST` a JSON body to `{api_base}{path}` and decode the JSON reply into `O`.
    ///
    /// See [`Client::send_json`] for how requests are sent and errors are reported.
    ///
    /// # Examples
    ///
    /// Decode a chat completion into a struct that has only the fields you need:
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(Deserialize)]
    /// struct Completion {
    ///     choices: Vec<Choice>,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Choice {
    ///     message: Message,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Message {
    ///     content: Option<String>,
    /// }
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let completion: Completion = client
    ///     .post_json(
    ///         "/chat/completions",
    ///         &json!({
    ///             "model": "gpt-4o-mini",
    ///             "messages": [{"role": "user", "content": "Say hello"}]
    ///         }),
    ///     )
    ///     .await?;
    /// println!("{:?}", completion.choices[0].message.content);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Talk to an `OpenAI`-compatible provider, with `serde_json::Value` as the reply type:
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::{Client, Config};
    /// use serde_json::{json, Value};
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let config = Config::builder()
    ///     .api_key("provider-key")
    ///     .api_base("https://openai-compatible.example.com/v1")
    ///     .build();
    /// let client = Client::builder(config)?.build();
    /// let reply: Value = client
    ///     .post_json(
    ///         "/chat/completions",
    ///         &json!({"model": "some-model", "messages": [{"role": "user", "content": "Hi"}]}),
    ///     )
    ///     .await?;
    /// println!("{}", reply["choices"][0]["message"]["content"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn post_json<I, O>(
        &self,
        path: &str,
        body: &I,
    ) -> impl Future<Output = Result<O>> + MaybeSend + 'static
    where
        I: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        self.send_json(Method::POST, path, Some(body))
    }

    /// `GET` `{api_base}{path}` and decode the JSON reply into `O`.
    ///
    /// See [`Client::send_json`] for how requests are sent and errors are reported.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openai_ergonomic::Client;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Models {
    ///     data: Vec<Model>,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Model {
    ///     id: String,
    /// }
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let models: Models = client.get_json("/models").await?;
    /// for model in models.data {
    ///     println!("{}", model.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_json<O>(&self, path: &str) -> impl Future<Output = Result<O>> + MaybeSend + 'static
    where
        O: DeserializeOwned,
    {
        self.send_json(Method::GET, path, None::<&()>)
    }

    /// `POST` a JSON body to `{api_base}{path}` and decode the server-sent events of the reply
    /// (`text/event-stream`) into a stream of `O`.
    ///
    /// The body is sent as is: set the provider's streaming flag (for `OpenAI`,
    /// `"stream": true`) yourself. Each event's `data:` payload is decoded as JSON into `O`;
    /// events with an empty payload are skipped, and `data: [DONE]` or the end of the body ends
    /// the stream.
    ///
    /// A non-2xx status is returned as [`Error::Api`] before any event is read, like
    /// [`Client::send_json`]. In the stream, a payload that does not decode into `O` yields
    /// [`Error::StreamParsing`] (with the payload as `chunk`) and the stream goes on with the
    /// next event; a failure reading the body yields [`Error::StreamConnection`] and ends the
    /// stream.
    ///
    /// The request goes through the configured HTTP client and its layers. Interceptors are not
    /// called.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// use openai_ergonomic::Client;
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(Deserialize)]
    /// struct Chunk {
    ///     choices: Vec<ChunkChoice>,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct ChunkChoice {
    ///     delta: Delta,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Delta {
    ///     content: Option<String>,
    /// }
    ///
    /// # async fn example() -> openai_ergonomic::Result<()> {
    /// let client = Client::from_env()?.build();
    /// let request = json!({
    ///     "model": "gpt-4o-mini",
    ///     "stream": true,
    ///     "messages": [{"role": "user", "content": "Tell me a story"}]
    /// });
    /// let mut chunks = client
    ///     .send_json_stream::<_, Chunk>("/chat/completions", &request)
    ///     .await?;
    /// while let Some(chunk) = chunks.next().await {
    ///     for choice in chunk?.choices {
    ///         print!("{}", choice.delta.content.unwrap_or_default());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_json_stream<I, O>(
        &self,
        path: &str,
        body: &I,
    ) -> impl Future<Output = Result<impl Stream<Item = Result<O>> + MaybeSend + Unpin + 'static>>
           + MaybeSend
           + 'static
    where
        I: Serialize + ?Sized,
        O: DeserializeOwned + 'static,
    {
        let request = self
            .raw_request(Method::POST, path)
            .header(ACCEPT, "text/event-stream")
            .json(body);
        async move {
            let response = check_status(request.send().await?).await?;
            Ok(decode_events::<O>(response))
        }
    }
}

impl<T> Client<T> {
    /// Send a JSON request that answers with server-sent events, and return the response once
    /// its status is known to be 2xx.
    pub(super) fn send_event_stream_request<I: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: &I,
    ) -> impl Future<Output = Result<Response>> + MaybeSend + 'static {
        let request = self
            .raw_request(method, path)
            .header(ACCEPT, "text/event-stream")
            .json(body);
        async move { check_status(request.send().await?).await }
    }
}

/// The server-sent events of a response, each `data:` payload decoded into `O`.
fn decode_events<O: DeserializeOwned + 'static>(
    response: Response,
) -> impl Stream<Item = Result<O>> + MaybeSend + Unpin + 'static {
    Box::pin(response.events().filter_map(|event| async move {
        match event {
            Ok(event) if event.data.trim().is_empty() => None,
            Ok(event) => {
                Some(
                    serde_json::from_str::<O>(&event.data).map_err(|error| Error::StreamParsing {
                        message: format!("Failed to deserialize event: {error}"),
                        chunk: event.data,
                    }),
                )
            }
            Err(error) => Some(Err(Error::StreamConnection {
                message: format!("Stream connection error: {error}"),
            })),
        }
    }))
}

/// The response if its status is 2xx, or else an [`Error::Api`] with the status and body text.
pub(super) async fn check_status(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let message = response.text().await?;
    let (error_type, error_code) = openai_error_details(&message);
    Err(Error::Api {
        status: status.as_u16(),
        message,
        error_type,
        error_code,
    })
}

/// The `type` and `code` of an `OpenAI` error body: `{"error": {"message", "type", "code"}}`.
pub(super) fn openai_error_details(body: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (None, None);
    };
    let field = |name: &str| match &value["error"][name] {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    };
    (field("type"), field("code"))
}

#[cfg(test)]
mod tests {
    use super::openai_error_details;

    #[test]
    fn reads_openai_error_details() {
        let body = r#"{"error":{"message":"bad","type":"invalid_request_error","code":"model_not_found"}}"#;
        assert_eq!(
            openai_error_details(body),
            (
                Some("invalid_request_error".to_string()),
                Some("model_not_found".to_string())
            )
        );
        assert_eq!(
            openai_error_details(r#"{"error":{"message":"x","type":null,"code":429}}"#),
            (None, Some("429".to_string()))
        );
        assert_eq!(openai_error_details("not json"), (None, None));
    }
}
