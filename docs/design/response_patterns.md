# Response Patterns

> Documentation of response wrapper design, extension trait patterns, and error transformation patterns for ergonomic response handling.

## Table of Contents

- [Response Wrapper Design](#response-wrapper-design)
- [Extension Trait Patterns](#extension-trait-patterns)
- [Error Transformation Patterns](#error-transformation-patterns)
- [Streaming Response Patterns](#streaming-response-patterns)
- [Response Type Hierarchy](#response-type-hierarchy)
- [Usage Tracking Patterns](#usage-tracking-patterns)

## Response Wrapper Design

### Core Response Wrapper Trait

All response wrappers implement a common `Response` trait for consistent behavior:

```rust
/// Core trait for all API response wrappers
pub trait Response {
    /// Get the underlying response object
    fn inner(&self) -> &dyn std::any::Any;

    /// Get usage information if available
    fn usage(&self) -> Option<&Usage>;

    /// Get the model used for this response
    fn model(&self) -> Option<&str>;

    /// Get the response ID
    fn id(&self) -> Option<&str>;

    /// Get the creation timestamp
    fn created(&self) -> Option<u64>;

    /// Check if the response indicates an error state
    fn has_error(&self) -> bool { false }
}

/// Trait for responses that can provide text content
pub trait TextResponse: Response {
    /// Get the primary text content
    fn content(&self) -> Option<&str>;

    /// Get all text content as a concatenated string
    fn all_content(&self) -> String;

    /// Check if content was truncated
    fn is_truncated(&self) -> bool;
}

/// Trait for responses with multiple choices
pub trait MultiChoiceResponse: Response {
    type Choice;

    /// Get all choices
    fn choices(&self) -> &[Self::Choice];

    /// Get the first choice
    fn first_choice(&self) -> Option<&Self::Choice>;

    /// Get the best choice (usually first, but can be overridden)
    fn best_choice(&self) -> Option<&Self::Choice> {
        self.first_choice()
    }
}
```

### Response Wrapper Implementation Pattern

Response wrappers add ergonomic methods while preserving access to the underlying response:

```rust
/// Wrapper for chat completion responses with ergonomic methods
#[derive(Debug, Clone)]
pub struct ChatCompletionResponseWrapper {
    inner: ChatCompletionResponse,
}

impl ChatCompletionResponseWrapper {
    pub fn new(response: ChatCompletionResponse) -> Self {
        Self { inner: response }
    }

    /// Get the underlying response
    pub fn into_inner(self) -> ChatCompletionResponse {
        self.inner
    }

    /// Get a reference to the underlying response
    pub fn inner(&self) -> &ChatCompletionResponse {
        &self.inner
    }
}

impl Response for ChatCompletionResponseWrapper {
    fn inner(&self) -> &dyn std::any::Any {
        &self.inner
    }

    fn usage(&self) -> Option<&Usage> {
        self.inner.usage.as_ref()
    }

    fn model(&self) -> Option<&str> {
        Some(&self.inner.model)
    }

    fn id(&self) -> Option<&str> {
        Some(&self.inner.id)
    }

    fn created(&self) -> Option<u64> {
        Some(self.inner.created)
    }
}

impl TextResponse for ChatCompletionResponseWrapper {
    fn content(&self) -> Option<&str> {
        self.inner
            .choices
            .first()?
            .message
            .content
            .as_deref()
    }

    fn all_content(&self) -> String {
        self.inner
            .choices
            .iter()
            .filter_map(|choice| choice.message.content.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn is_truncated(&self) -> bool {
        self.inner
            .choices
            .iter()
            .any(|choice| choice.finish_reason.as_deref() == Some("length"))
    }
}

impl MultiChoiceResponse for ChatCompletionResponseWrapper {
    type Choice = ChatChoice;

    fn choices(&self) -> &[Self::Choice] {
        &self.inner.choices
    }

    fn first_choice(&self) -> Option<&Self::Choice> {
        self.inner.choices.first()
    }
}
```

### Domain-Specific Response Wrappers

Create specialized wrappers for different response types:

```rust
/// Wrapper for responses API responses
#[derive(Debug, Clone)]
pub struct ResponsesResponseWrapper {
    inner: ResponsesResponse,
}

impl ResponsesResponseWrapper {
    pub fn new(response: ResponsesResponse) -> Self {
        Self { inner: response }
    }

    /// Check if the response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.inner
            .choices
            .iter()
            .any(|choice| choice.message.tool_calls.is_some())
    }

    /// Get all tool calls from all choices
    pub fn tool_calls(&self) -> Vec<&ToolCall> {
        self.inner
            .choices
            .iter()
            .filter_map(|choice| choice.message.tool_calls.as_ref())
            .flatten()
            .collect()
    }

    /// Extract function calls specifically
    pub fn function_calls(&self) -> Vec<&FunctionCall> {
        self.tool_calls()
            .iter()
            .filter_map(|tool_call| {
                if tool_call.type_ == "function" {
                    tool_call.function.as_ref()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if response indicates web search was used
    pub fn used_web_search(&self) -> bool {
        self.tool_calls()
            .iter()
            .any(|tool_call| tool_call.type_ == "web_search")
    }
}

/// Wrapper for embedding responses
#[derive(Debug, Clone)]
pub struct EmbeddingResponseWrapper {
    inner: CreateEmbeddingResponse,
}

impl EmbeddingResponseWrapper {
    pub fn new(response: CreateEmbeddingResponse) -> Self {
        Self { inner: response }
    }

    /// Get embeddings as vectors
    pub fn embeddings(&self) -> Vec<&[f64]> {
        self.inner
            .data
            .iter()
            .map(|item| item.embedding.as_slice())
            .collect()
    }

    /// Get the first embedding (for single text input)
    pub fn embedding(&self) -> Option<&[f64]> {
        self.inner
            .data
            .first()
            .map(|item| item.embedding.as_slice())
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Option<f64> {
        if a.len() != b.len() {
            return None;
        }

        let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            Some(0.0)
        } else {
            Some(dot_product / (norm_a * norm_b))
        }
    }

    /// Get dimensions of embeddings
    pub fn dimensions(&self) -> Option<usize> {
        self.inner
            .data
            .first()
            .map(|item| item.embedding.len())
    }
}

/// Wrapper for image responses
#[derive(Debug, Clone)]
pub struct ImageResponseWrapper {
    inner: ImagesResponse,
}

impl ImageResponseWrapper {
    pub fn new(response: ImagesResponse) -> Self {
        Self { inner: response }
    }

    /// Get image URLs
    pub fn urls(&self) -> Vec<Option<&str>> {
        self.inner
            .data
            .iter()
            .map(|item| item.url.as_deref())
            .collect()
    }

    /// Get base64 encoded images
    pub fn base64_images(&self) -> Vec<Option<&str>> {
        self.inner
            .data
            .iter()
            .map(|item| item.b64_json.as_deref())
            .collect()
    }

    /// Get revised prompts (for DALL-E 3)
    pub fn revised_prompts(&self) -> Vec<Option<&str>> {
        self.inner
            .data
            .iter()
            .map(|item| item.revised_prompt.as_deref())
            .collect()
    }

    /// Download image from URL
    pub async fn download_image(&self, index: usize) -> Result<Vec<u8>> {
        let url = self.urls()
            .get(index)
            .and_then(|&url| url)
            .ok_or_else(|| Error::Validation {
                field: "index".to_string(),
                message: format!("no image at index {}", index),
            })?;

        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Decode base64 image
    pub fn decode_base64_image(&self, index: usize) -> Result<Vec<u8>> {
        let b64_data = self.base64_images()
            .get(index)
            .and_then(|&data| data)
            .ok_or_else(|| Error::Validation {
                field: "index".to_string(),
                message: format!("no base64 image at index {}", index),
            })?;

        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD
            .decode(b64_data)
            .map_err(|e| Error::Validation {
                field: "base64_data".to_string(),
                message: format!("invalid base64: {}", e),
            })
    }
}
```

## Extension Trait Patterns

### Response Extension Traits

Extension traits add ergonomic methods to existing response types without wrapper overhead:

```rust
/// Extension methods for chat completion responses
pub trait ChatCompletionResponseExt {
    /// Get the content from the first choice
    fn content(&self) -> Option<&str>;

    /// Get content from a specific choice
    fn content_at(&self, index: usize) -> Option<&str>;

    /// Get all choice contents
    fn all_content(&self) -> Vec<&str>;

    /// Check if any choice was truncated
    fn is_truncated(&self) -> bool;

    /// Get finish reason for first choice
    fn finish_reason(&self) -> Option<&str>;

    /// Get all finish reasons
    fn finish_reasons(&self) -> Vec<Option<&str>>;

    /// Extract tool calls from first choice
    fn tool_calls(&self) -> Vec<&ToolCall>;

    /// Extract all tool calls from all choices
    fn all_tool_calls(&self) -> Vec<&ToolCall>;

    /// Check if response contains function calls
    fn has_function_calls(&self) -> bool;

    /// Get function calls specifically
    fn function_calls(&self) -> Vec<&FunctionCall>;

    /// Calculate total tokens used
    fn total_tokens(&self) -> Option<u32>;

    /// Get prompt tokens used
    fn prompt_tokens(&self) -> Option<u32>;

    /// Get completion tokens generated
    fn completion_tokens(&self) -> Option<u32>;
}

impl ChatCompletionResponseExt for ChatCompletionResponse {
    fn content(&self) -> Option<&str> {
        self.choices
            .first()?
            .message
            .content
            .as_deref()
    }

    fn content_at(&self, index: usize) -> Option<&str> {
        self.choices
            .get(index)?
            .message
            .content
            .as_deref()
    }

    fn all_content(&self) -> Vec<&str> {
        self.choices
            .iter()
            .filter_map(|choice| choice.message.content.as_deref())
            .collect()
    }

    fn is_truncated(&self) -> bool {
        self.choices
            .iter()
            .any(|choice| choice.finish_reason.as_deref() == Some("length"))
    }

    fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()?
            .finish_reason
            .as_deref()
    }

    fn finish_reasons(&self) -> Vec<Option<&str>> {
        self.choices
            .iter()
            .map(|choice| choice.finish_reason.as_deref())
            .collect()
    }

    fn tool_calls(&self) -> Vec<&ToolCall> {
        self.choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_ref())
            .map(|calls| calls.iter().collect())
            .unwrap_or_default()
    }

    fn all_tool_calls(&self) -> Vec<&ToolCall> {
        self.choices
            .iter()
            .filter_map(|choice| choice.message.tool_calls.as_ref())
            .flatten()
            .collect()
    }

    fn has_function_calls(&self) -> bool {
        self.all_tool_calls()
            .iter()
            .any(|tool_call| tool_call.type_ == "function")
    }

    fn function_calls(&self) -> Vec<&FunctionCall> {
        self.all_tool_calls()
            .iter()
            .filter_map(|tool_call| {
                if tool_call.type_ == "function" {
                    tool_call.function.as_ref()
                } else {
                    None
                }
            })
            .collect()
    }

    fn total_tokens(&self) -> Option<u32> {
        self.usage.as_ref().map(|u| u.total_tokens)
    }

    fn prompt_tokens(&self) -> Option<u32> {
        self.usage.as_ref().map(|u| u.prompt_tokens)
    }

    fn completion_tokens(&self) -> Option<u32> {
        self.usage.as_ref().map(|u| u.completion_tokens)
    }
}

/// Extension methods for tool calls
pub trait ToolCallExt {
    /// Parse function arguments as JSON
    fn parse_arguments<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned;

    /// Get function name
    fn function_name(&self) -> Option<&str>;

    /// Get raw arguments string
    fn arguments_str(&self) -> Option<&str>;

    /// Check if this is a specific function call
    fn is_function(&self, name: &str) -> bool;
}

impl ToolCallExt for ToolCall {
    fn parse_arguments<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let args_str = self.arguments_str().ok_or_else(|| Error::Validation {
            field: "arguments".to_string(),
            message: "no arguments found in tool call".to_string(),
        })?;

        serde_json::from_str(args_str).map_err(|e| Error::Json(e))
    }

    fn function_name(&self) -> Option<&str> {
        if self.type_ == "function" {
            self.function.as_ref().map(|f| f.name.as_str())
        } else {
            None
        }
    }

    fn arguments_str(&self) -> Option<&str> {
        if self.type_ == "function" {
            self.function.as_ref().map(|f| f.arguments.as_str())
        } else {
            None
        }
    }

    fn is_function(&self, name: &str) -> bool {
        self.function_name() == Some(name)
    }
}
```

### Generic Extension Patterns

Create generic extension traits for common patterns:

```rust
/// Extension trait for responses with usage information
pub trait UsageResponseExt {
    fn usage(&self) -> Option<&Usage>;

    /// Get token cost estimation (requires pricing data)
    fn estimated_cost(&self, model: &str) -> Option<f64> {
        let usage = self.usage()?;
        calculate_cost(model, usage)
    }

    /// Check if usage is close to context limit
    fn is_near_context_limit(&self, model: &str) -> bool {
        let usage = self.usage()?;
        let limit = get_context_limit(model)?;
        usage.total_tokens as f64 / limit as f64 > 0.9
    }
}

impl<T> UsageResponseExt for T
where
    T: Response,
{
    fn usage(&self) -> Option<&Usage> {
        Response::usage(self)
    }
}

/// Extension trait for responses with choices
pub trait ChoiceResponseExt<C> {
    fn choices(&self) -> &[C];

    /// Get choice by index with bounds checking
    fn choice_at(&self, index: usize) -> Option<&C> {
        self.choices().get(index)
    }

    /// Find choice by predicate
    fn find_choice<P>(&self, predicate: P) -> Option<&C>
    where
        P: Fn(&C) -> bool,
    {
        self.choices().iter().find(|&choice| predicate(choice))
    }

    /// Get number of choices
    fn choice_count(&self) -> usize {
        self.choices().len()
    }
}

impl ChoiceResponseExt<ChatChoice> for ChatCompletionResponse {
    fn choices(&self) -> &[ChatChoice] {
        &self.choices
    }
}
```

## Error Transformation Patterns

### Response Error Handling

Transform API errors into domain-specific errors:

```rust
/// Transform response into Result based on content
pub trait ResponseResultExt {
    type Success;
    type Error;

    /// Convert response to Result based on success criteria
    fn into_result(self) -> Result<Self::Success, Self::Error>;
}

impl ResponseResultExt for ChatCompletionResponse {
    type Success = String;
    type Error = Error;

    fn into_result(self) -> Result<Self::Success, Self::Error> {
        // Check for API-level errors first
        if let Some(error) = self.error {
            return Err(Error::Api(error.into()));
        }

        // Check for content policy violations
        if self.choices.iter().any(|choice| {
            choice.finish_reason.as_deref() == Some("content_filter")
        }) {
            return Err(Error::ContentPolicy {
                message: "Content was filtered due to policy violations".to_string(),
            });
        }

        // Extract content or return error
        self.content()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Response {
                message: "No content in response".to_string(),
            })
    }
}

/// Validate response completeness
pub trait ResponseValidationExt {
    /// Check if response is complete and valid
    fn validate(&self) -> Result<()>;

    /// Check specific validation criteria
    fn is_complete(&self) -> bool;
    fn has_content(&self) -> bool;
    fn is_successful(&self) -> bool;
}

impl ResponseValidationExt for ChatCompletionResponse {
    fn validate(&self) -> Result<()> {
        if !self.is_complete() {
            return Err(Error::Response {
                message: "Response is incomplete".to_string(),
            });
        }

        if !self.has_content() {
            return Err(Error::Response {
                message: "Response has no content".to_string(),
            });
        }

        if !self.is_successful() {
            return Err(Error::Response {
                message: "Response indicates failure".to_string(),
            });
        }

        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.choices.iter().any(|choice| {
            matches!(
                choice.finish_reason.as_deref(),
                Some("stop") | Some("tool_calls")
            )
        })
    }

    fn has_content(&self) -> bool {
        self.choices.iter().any(|choice| {
            choice.message.content.is_some() || choice.message.tool_calls.is_some()
        })
    }

    fn is_successful(&self) -> bool {
        !self.choices.iter().any(|choice| {
            matches!(
                choice.finish_reason.as_deref(),
                Some("content_filter") | Some("length")
            )
        })
    }
}
```

## Streaming Response Patterns

### Streaming Response Wrappers

Handle streaming responses with accumulation and state tracking:

```rust
/// Wrapper for streaming chat responses
#[derive(Debug)]
pub struct ChatStreamResponse {
    accumulated: ChatCompletionResponse,
    is_complete: bool,
}

impl ChatStreamResponse {
    pub fn new() -> Self {
        Self {
            accumulated: ChatCompletionResponse::default(),
            is_complete: false,
        }
    }

    /// Update with a new chunk
    pub fn update(&mut self, chunk: &ChatCompletionChunk) -> Result<()> {
        // Update accumulated response
        if self.accumulated.id.is_empty() {
            self.accumulated.id = chunk.id.clone();
            self.accumulated.model = chunk.model.clone();
            self.accumulated.created = chunk.created;
        }

        // Process choices
        for (i, chunk_choice) in chunk.choices.iter().enumerate() {
            // Ensure we have enough choices in accumulated response
            while self.accumulated.choices.len() <= i {
                self.accumulated.choices.push(ChatChoice::default());
            }

            let acc_choice = &mut self.accumulated.choices[i];

            // Accumulate content
            if let Some(delta_content) = &chunk_choice.delta.content {
                if acc_choice.message.content.is_none() {
                    acc_choice.message.content = Some(String::new());
                }
                if let Some(content) = &mut acc_choice.message.content {
                    content.push_str(delta_content);
                }
            }

            // Handle tool calls
            if let Some(delta_tool_calls) = &chunk_choice.delta.tool_calls {
                if acc_choice.message.tool_calls.is_none() {
                    acc_choice.message.tool_calls = Some(Vec::new());
                }
                // Accumulate tool call data...
            }

            // Update finish reason
            if let Some(finish_reason) = &chunk_choice.finish_reason {
                acc_choice.finish_reason = Some(finish_reason.clone());
                if finish_reason != "null" {
                    self.is_complete = true;
                }
            }
        }

        // Update usage if present
        if let Some(usage) = &chunk.usage {
            self.accumulated.usage = Some(usage.clone());
        }

        Ok(())
    }

    /// Get the accumulated response
    pub fn response(&self) -> &ChatCompletionResponse {
        &self.accumulated
    }

    /// Check if streaming is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get current content
    pub fn current_content(&self) -> Option<&str> {
        self.accumulated.content()
    }

    /// Consume and return the final response
    pub fn into_response(self) -> ChatCompletionResponse {
        self.accumulated
    }
}

/// Stream processor for chat completions
pub struct ChatStream {
    stream: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
    accumulated: ChatStreamResponse,
}

impl ChatStream {
    pub fn new(
        stream: impl Stream<Item = Result<ChatCompletionChunk>> + Send + 'static,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            accumulated: ChatStreamResponse::new(),
        }
    }

    /// Process stream and call callback for each chunk
    pub async fn process_with_callback<F>(
        mut self,
        mut callback: F,
    ) -> Result<ChatCompletionResponse>
    where
        F: FnMut(&ChatCompletionChunk, &ChatCompletionResponse) -> Result<()>,
    {
        while let Some(chunk) = self.stream.next().await {
            let chunk = chunk?;
            self.accumulated.update(&chunk)?;
            callback(&chunk, self.accumulated.response())?;

            if self.accumulated.is_complete() {
                break;
            }
        }

        Ok(self.accumulated.into_response())
    }

    /// Collect stream into final response
    pub async fn collect(mut self) -> Result<ChatCompletionResponse> {
        while let Some(chunk) = self.stream.next().await {
            let chunk = chunk?;
            self.accumulated.update(&chunk)?;

            if self.accumulated.is_complete() {
                break;
            }
        }

        Ok(self.accumulated.into_response())
    }

    /// Get current accumulated state
    pub fn current_response(&self) -> &ChatCompletionResponse {
        self.accumulated.response()
    }
}

impl Stream for ChatStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}
```

## Response Type Hierarchy

### Type Organization

Organize response types in a hierarchy that reflects usage patterns:

```rust
/// Base response types
pub mod base {
    pub use openai_client_base::models::{
        ChatCompletionResponse,
        CreateEmbeddingResponse,
        ImagesResponse,
        // ... other base types
    };
}

/// Wrapped response types with ergonomic methods
pub mod wrapped {
    pub use super::{
        ChatCompletionResponseWrapper,
        EmbeddingResponseWrapper,
        ImageResponseWrapper,
        // ... other wrapped types
    };
}

/// Streaming response types
pub mod streaming {
    pub use super::{
        ChatStream,
        ChatStreamResponse,
        ResponsesStream,
        // ... other streaming types
    };
}

/// Extension traits for all response types
pub mod ext {
    pub use super::{
        ChatCompletionResponseExt,
        ToolCallExt,
        UsageResponseExt,
        ResponseValidationExt,
        // ... other extension traits
    };
}

/// Re-export for convenience
pub use base::*;
pub use ext::*;
```

## Usage Tracking Patterns

### Usage Information Helpers

Provide utilities for tracking and analyzing usage across responses:

```rust
/// Usage tracking and analysis utilities
pub struct UsageTracker {
    total_requests: u64,
    total_tokens: u64,
    total_cost: f64,
    by_model: std::collections::HashMap<String, ModelUsage>,
}

#[derive(Debug, Default)]
pub struct ModelUsage {
    pub requests: u64,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub estimated_cost: f64,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            total_tokens: 0,
            total_cost: 0.0,
            by_model: std::collections::HashMap::new(),
        }
    }

    /// Track usage from a response
    pub fn track<R: Response + UsageResponseExt>(&mut self, response: &R) {
        self.total_requests += 1;

        if let (Some(usage), Some(model)) = (response.usage(), response.model()) {
            self.total_tokens += usage.total_tokens as u64;

            let model_usage = self.by_model.entry(model.to_string()).or_default();
            model_usage.requests += 1;
            model_usage.total_tokens += usage.total_tokens as u64;
            model_usage.prompt_tokens += usage.prompt_tokens as u64;
            model_usage.completion_tokens += usage.completion_tokens as u64;

            if let Some(cost) = response.estimated_cost(model) {
                self.total_cost += cost;
                model_usage.estimated_cost += cost;
            }
        }
    }

    /// Get usage summary
    pub fn summary(&self) -> UsageSummary {
        UsageSummary {
            total_requests: self.total_requests,
            total_tokens: self.total_tokens,
            total_cost: self.total_cost,
            models: self.by_model.len(),
            most_used_model: self.most_used_model(),
        }
    }

    fn most_used_model(&self) -> Option<String> {
        self.by_model
            .iter()
            .max_by_key(|(_, usage)| usage.requests)
            .map(|(model, _)| model.clone())
    }
}

#[derive(Debug)]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub models: usize,
    pub most_used_model: Option<String>,
}

/// Extension trait to add usage tracking to responses
pub trait TrackableResponse: Response + UsageResponseExt {
    /// Track this response in a usage tracker
    fn track_usage(&self, tracker: &mut UsageTracker) {
        tracker.track(self);
    }
}

impl<T> TrackableResponse for T
where
    T: Response + UsageResponseExt,
{}
```

These response patterns provide:

1. **Consistent wrapper design** with access to underlying data
2. **Rich extension traits** for ergonomic method access
3. **Error transformation** patterns for robust handling
4. **Streaming response** accumulation and processing
5. **Type hierarchy organization** for clear structure
6. **Usage tracking utilities** for monitoring and analysis

The patterns emphasize both ergonomics and performance, allowing developers to choose between lightweight extension traits and feature-rich wrapper types based on their needs.