//! The Responses API (`/responses`): request builder, response, streaming events.
//!
//! [`ResponsesBuilder`] builds the generated `openai-client-base` [`CreateResponse`];
//! [`Client::send_responses`](crate::Client::send_responses) sends it to `POST /responses` and
//! returns a [`ResponsesResponse`], and
//! [`Client::send_responses_stream`](crate::Client::send_responses_stream) streams its
//! [`ResponsesEvent`]s. [`Client::responses_api`](crate::Client::responses_api) retrieves,
//! cancels and deletes stored responses.
//!
//! ```rust,no_run
//! use openai_ergonomic::Client;
//!
//! # async fn example() -> openai_ergonomic::Result<()> {
//! let client = Client::from_env()?.build();
//! let response = client
//!     .send_responses(client.responses().user("Say hello"))
//!     .await?;
//! println!("{}", response.output_text());
//! # Ok(())
//! # }
//! ```
//!
//! Chat completions (`/chat/completions`) have their own builder,
//! [`ChatCompletionBuilder`](crate::ChatCompletionBuilder).

mod builder;
mod response;
mod stream;

pub use builder::{
    responses_function_tool, responses_simple, responses_system_user, ResponsesBuilder,
};
pub use response::ResponsesResponse;
pub use stream::{
    collect_text, BoxedResponsesStream, ResponsesEvent, ResponsesEventKind, ResponsesStream,
};

/// The generated models of the Responses API that its builder and accessors use.
pub use openai_client_base::models::{
    CreateResponse, IncludeEnum, InputItem, InputParam, OutputItem, Response as ResponseObject,
    ResponseItemList, ResponseStreamEvent, Tool as ResponsesTool, ToolChoiceParam,
};
