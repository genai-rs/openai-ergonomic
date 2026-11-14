//! Responses API builders.
//!
//! The builder of the Responses API (`POST /responses`) lives in
//! [`responses_api`](crate::responses_api); it is re-exported here with its helpers.
//! Chat completions (`POST /chat/completions`) use
//! [`ChatCompletionBuilder`](super::ChatCompletionBuilder).

pub use crate::responses_api::{
    responses_function_tool, responses_simple, responses_system_user, ResponsesBuilder,
};
