//! Completions API builders.
//!
//! Provides high-level builders for creating text completion requests using
//! the legacy Completions API. For chat-based completions, use the Chat API instead.

use openai_client_base::models::{
    ChatCompletionStreamOptions, CreateCompletionRequest, StopConfiguration,
};

use crate::{Builder, Error, Result};

/// Builder for creating text completion requests.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::completions::CompletionsBuilder;
/// use openai_ergonomic::Builder;
///
/// let request = CompletionsBuilder::new("gpt-3.5-turbo-instruct")
///     .prompt("Once upon a time")
///     .max_tokens(50)
///     .temperature(0.7)
///     .build()
///     .unwrap();
///
/// assert_eq!(request.model, "gpt-3.5-turbo-instruct");
/// assert_eq!(request.max_tokens, Some(50));
/// ```
#[derive(Debug, Clone)]
pub struct CompletionsBuilder {
    model: String,
    prompt: Option<String>,
    best_of: Option<i32>,
    echo: Option<bool>,
    frequency_penalty: Option<f64>,
    logit_bias: Option<std::collections::HashMap<String, i32>>,
    logprobs: Option<i32>,
    max_tokens: Option<i32>,
    n: Option<i32>,
    presence_penalty: Option<f64>,
    seed: Option<i64>,
    stop: Option<Vec<String>>,
    stream: Option<bool>,
    stream_options: Option<ChatCompletionStreamOptions>,
    suffix: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    user: Option<String>,
}

impl CompletionsBuilder {
    /// Create a new completions builder for the specified model.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: None,
            best_of: None,
            echo: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            max_tokens: None,
            n: None,
            presence_penalty: None,
            seed: None,
            stop: None,
            stream: None,
            stream_options: None,
            suffix: None,
            temperature: None,
            top_p: None,
            user: None,
        }
    }

    /// Set the prompt for the completion.
    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Generates `best_of` completions server-side and returns the "best".
    ///
    /// # Note
    ///
    /// Because this parameter generates many completions, it can quickly consume your token quota.
    /// Use carefully and ensure that you have reasonable settings for `max_tokens` and `stop`.
    #[must_use]
    pub fn best_of(mut self, best_of: i32) -> Self {
        self.best_of = Some(best_of);
        self
    }

    /// Echo back the prompt in addition to the completion.
    #[must_use]
    pub fn echo(mut self, echo: bool) -> Self {
        self.echo = Some(echo);
        self
    }

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their
    /// existing frequency in the text so far.
    #[must_use]
    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Modify the likelihood of specified tokens appearing in the completion.
    #[must_use]
    pub fn logit_bias(mut self, bias: std::collections::HashMap<String, i32>) -> Self {
        self.logit_bias = Some(bias);
        self
    }

    /// Include the log probabilities on the `logprobs` most likely output tokens.
    /// An integer between 0 and 5.
    #[must_use]
    pub fn logprobs(mut self, logprobs: i32) -> Self {
        self.logprobs = Some(logprobs);
        self
    }

    /// The maximum number of tokens that can be generated in the completion.
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// How many completions to generate for each prompt.
    #[must_use]
    pub fn n(mut self, n: i32) -> Self {
        self.n = Some(n);
        self
    }

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether
    /// they appear in the text so far.
    #[must_use]
    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// If specified, the system will make a best effort to sample deterministically.
    #[must_use]
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Up to 4 sequences where the API will stop generating further tokens.
    #[must_use]
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Add a single stop sequence.
    #[must_use]
    pub fn add_stop(mut self, stop: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(stop.into());
        self
    }

    /// Whether to stream back partial progress.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Options for streaming response.
    #[must_use]
    pub fn stream_options(mut self, options: ChatCompletionStreamOptions) -> Self {
        self.stream_options = Some(options);
        self
    }

    /// The suffix that comes after a completion of inserted text.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// What sampling temperature to use, between 0 and 2.
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// An alternative to sampling with temperature, called nucleus sampling.
    #[must_use]
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// A unique identifier representing your end-user.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

impl Builder<CreateCompletionRequest> for CompletionsBuilder {
    fn build(self) -> Result<CreateCompletionRequest> {
        let prompt = self
            .prompt
            .ok_or_else(|| Error::Builder("prompt is required".to_string()))?;

        let mut request = CreateCompletionRequest::new(self.model, prompt);

        request.best_of = self.best_of;
        request.echo = self.echo;
        request.frequency_penalty = self.frequency_penalty;
        request.logit_bias = self.logit_bias;
        request.logprobs = self.logprobs;
        request.max_tokens = self.max_tokens;
        request.n = self.n;
        request.presence_penalty = self.presence_penalty;
        request.seed = self.seed;
        request.stop = self.stop.map(|stops| {
            if stops.len() == 1 {
                Box::new(StopConfiguration::new_text(
                    stops.into_iter().next().unwrap(),
                ))
            } else {
                Box::new(StopConfiguration::new_arrayofstrings(stops))
            }
        });
        request.stream = self.stream;
        request.stream_options = self.stream_options.map(|opts| Some(Box::new(opts)));
        request.suffix = self.suffix;
        request.temperature = self.temperature;
        request.top_p = self.top_p;
        request.user = self.user;

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions_builder_basic() {
        let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct").prompt("Hello, world!");

        let request = builder.build().unwrap();
        assert_eq!(request.model, "gpt-3.5-turbo-instruct");
    }

    #[test]
    fn test_completions_builder_with_options() {
        let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct")
            .prompt("Test prompt")
            .max_tokens(100)
            .temperature(0.8)
            .top_p(0.9)
            .n(3);

        let request = builder.build().unwrap();
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.8));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.n, Some(3));
    }

    #[test]
    fn test_completions_builder_with_stop_sequences() {
        let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct")
            .prompt("Test")
            .add_stop("\n")
            .add_stop("END");

        let request = builder.build().unwrap();
        assert!(request.stop.is_some());
    }

    #[test]
    fn test_completions_builder_missing_prompt() {
        let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct");
        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_completions_builder_with_single_prompt() {
        let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct").prompt("Hello");

        let request = builder.build().unwrap();
        assert_eq!(request.model, "gpt-3.5-turbo-instruct");
        assert_eq!(request.prompt, "Hello");
    }
}
