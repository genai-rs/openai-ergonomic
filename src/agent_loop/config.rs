//! The configuration of a [`ToolLoop`](super::ToolLoop).

use std::num::NonZeroU32;
use std::time::Duration;

use crate::TruncationConfig;

/// What the model can take, which changes what the loop sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModelCaps {
    /// Whether the model accepts images. When it does not,
    /// [`Conversation::normalize`](super::Conversation::normalize) replaces the images of the
    /// history (in messages and tool outputs) with a short text. Default `true`.
    pub images: bool,
    /// Whether the model searches deferred tools itself (the Responses API's hosted
    /// `tool_search` with `defer_loading` tools). When it does, the loop advertises the
    /// [deferred](crate::tool_framework::Exposure::Deferred) tools with `defer_loading: true` and
    /// a `tool_search` tool, and records the tools the model loaded; when it does not (and on the
    /// Chat Completions API), the loop offers a `tool_search` function it answers itself.
    /// Default `false`.
    pub tool_search: bool,
    /// Whether the model reasons, so that its reasoning items are worth keeping: with `store`
    /// off, the loop then asks for their encrypted content
    /// ([`LoopConfig::include_encrypted_reasoning`]). Also assumed when the request template
    /// sets reasoning options. Default `false` (models without reasoning reject the request).
    pub reasoning: bool,
}

impl Default for ModelCaps {
    fn default() -> Self {
        Self {
            images: true,
            tool_search: false,
            reasoning: false,
        }
    }
}

impl ModelCaps {
    /// The defaults: images, no hosted tool search.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the model accepts images.
    #[must_use]
    pub fn with_images(mut self, images: bool) -> Self {
        self.images = images;
        self
    }

    /// Whether the model searches deferred tools itself.
    #[must_use]
    pub fn with_tool_search(mut self, tool_search: bool) -> Self {
        self.tool_search = tool_search;
        self
    }

    /// Whether the model reasons.
    #[must_use]
    pub fn with_reasoning(mut self, reasoning: bool) -> Self {
        self.reasoning = reasoning;
        self
    }
}

/// The `prompt_cache_key` the loop sends with every request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum PromptCacheKey {
    /// A key made for each [`ToolLoop`](super::ToolLoop), the same for all its requests, so that
    /// they share the prompt cache.
    #[default]
    PerLoop,
    /// This key.
    Custom(String),
    /// No key.
    Omit,
}

/// How the loop retries a request that failed, as a whole.
///
/// A connection that failed, a stream cut off or failed midway, a rate limit, a server error:
/// only errors whose
/// [`retry_class`](crate::Error::retry_class) allows it are retried.
///
/// The delay is the one the server asked for (a `retry-after-ms` or `retry-after` header, or a
/// "try again in" message), else `base_delay · 2^(attempt − 1)` with ±10 % jitter, at most
/// `max_delay`. The client's own HTTP retries ([`Config::max_retries`](crate::Config::max_retries))
/// come first, for failures before the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RetryConfig {
    /// Retries of one request after its first attempt. Default 5; 0 disables them.
    pub max_retries: u32,
    /// The first backoff delay. Default 200 ms.
    pub base_delay: Duration,
    /// The longest delay, server-directed ones included. Default 60 s.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(60),
        }
    }
}

impl RetryConfig {
    /// At most `max_retries` retries, with the default delays.
    #[must_use]
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }

    /// No retries.
    #[must_use]
    pub fn none() -> Self {
        Self::new(0)
    }

    /// The first backoff delay.
    #[must_use]
    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    /// The longest delay.
    #[must_use]
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// The backoff delay before retry `attempt` (1 for the first retry), with `jitter` in
    /// `[-1, 1]` scaling ±10 %.
    pub(crate) fn backoff(&self, attempt: u32, jitter: f64) -> Duration {
        let exponent = attempt.saturating_sub(1).min(30);
        let delay = self.base_delay.saturating_mul(1_u32 << exponent);
        let factor = jitter.clamp(-1.0, 1.0).mul_add(0.1, 1.0);
        delay.mul_f64(factor).min(self.max_delay)
    }
}

/// The client-side tool search of the loop.
///
/// A `tool_search` function the model calls to find [deferred](crate::tool_framework::Exposure::Deferred) tools, answered with
/// [`ToolRegistry::search_deferred`](crate::ToolRegistry::search_deferred). The tools found are
/// advertised from the next request on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ToolSearchConfig {
    /// The most tools one search loads. Default 8.
    pub limit: usize,
    /// The description of the `tool_search` tool the model sees.
    pub description: String,
}

impl Default for ToolSearchConfig {
    fn default() -> Self {
        Self {
            limit: 8,
            description: "Search for more tools by what they do. The tools found can be called \
                          from the next step on."
                .to_string(),
        }
    }
}

impl ToolSearchConfig {
    /// The default search, with at most `limit` tools per search.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            ..Self::default()
        }
    }

    /// The description of the `tool_search` tool.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// The configuration of a [`ToolLoop`](super::ToolLoop).
///
/// The defaults are those of a stateless agent: nothing stored on the server, the encrypted reasoning replayed, one prompt cache key per
/// loop, parallel tool calls, at most 16 requests per run, streamed responses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[allow(clippy::struct_excessive_bools)] // independent switches of the requests
pub struct LoopConfig {
    /// The most model requests of one run (retries not counted); `None` for no limit. When the
    /// model still calls tools after the last one, the run ends with
    /// [`LoopStop::MaxIterations`](super::LoopStop::MaxIterations). Default 16.
    pub max_iterations: Option<NonZeroU32>,
    /// Responses API `store`: whether the server keeps the responses. The loop sends the whole
    /// conversation with every request either way, and never chains requests with
    /// `previous_response_id`. Default `false`.
    pub store: bool,
    /// Ask for the reasoning items' encrypted content (`include: ["reasoning.encrypted_content"]`)
    /// when `store` is `false` and the model reasons ([`ModelCaps::reasoning`]), and send it
    /// back, so that a reasoning model keeps its reasoning across requests. Default `true`.
    pub include_encrypted_reasoning: bool,
    /// The `prompt_cache_key` of the requests. Default [`PromptCacheKey::PerLoop`].
    pub prompt_cache_key: PromptCacheKey,
    /// `parallel_tool_calls`: whether the model may call several tools at once. The loop runs
    /// the calls of tools that allow it ([`FunctionTool::supports_parallel`](crate::FunctionTool::supports_parallel))
    /// at the same time and the others alone, and answers in call order. Default `true`.
    pub parallel_tool_calls: bool,
    /// How much of a tool's output the model sees. Default 10 000 bytes, with a header.
    pub truncation: TruncationConfig,
    /// The retries of a failed request. Default 5 retries from 200 ms.
    pub retry: RetryConfig,
    /// Stream Responses API responses, to run each tool call as soon as the model finished it
    /// (`response.output_item.done`), while the response goes on. Chat Completions requests are
    /// not streamed. Default `true`.
    pub stream: bool,
    /// What the model can take.
    pub caps: ModelCaps,
    /// The client-side tool search over deferred tools; `None` to offer none (deferred tools are
    /// then only advertised once [loaded](super::ToolLoop::load_tool)). Default on.
    pub tool_search: Option<ToolSearchConfig>,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: NonZeroU32::new(16),
            store: false,
            include_encrypted_reasoning: true,
            prompt_cache_key: PromptCacheKey::PerLoop,
            parallel_tool_calls: true,
            truncation: TruncationConfig::default(),
            retry: RetryConfig::default(),
            stream: true,
            caps: ModelCaps::default(),
            tool_search: Some(ToolSearchConfig::default()),
        }
    }
}

impl LoopConfig {
    /// The defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// At most `max` model requests per run; 0 for no limit.
    #[must_use]
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = NonZeroU32::new(max);
        self
    }

    /// Whether the server stores the responses.
    #[must_use]
    pub fn with_store(mut self, store: bool) -> Self {
        self.store = store;
        self
    }

    /// Whether to ask for and replay the encrypted reasoning.
    #[must_use]
    pub fn with_encrypted_reasoning(mut self, include: bool) -> Self {
        self.include_encrypted_reasoning = include;
        self
    }

    /// The prompt cache key.
    #[must_use]
    pub fn with_prompt_cache_key(mut self, key: PromptCacheKey) -> Self {
        self.prompt_cache_key = key;
        self
    }

    /// Whether the model may call several tools at once.
    #[must_use]
    pub fn with_parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.parallel_tool_calls = parallel;
        self
    }

    /// The truncation of tool outputs.
    #[must_use]
    pub fn with_truncation(mut self, truncation: TruncationConfig) -> Self {
        self.truncation = truncation;
        self
    }

    /// The retries of failed requests.
    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Whether to stream Responses API responses.
    #[must_use]
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// What the model can take.
    #[must_use]
    pub fn with_caps(mut self, caps: ModelCaps) -> Self {
        self.caps = caps;
        self
    }

    /// The client-side tool search; `None` to offer none.
    #[must_use]
    pub fn with_tool_search(mut self, search: Option<ToolSearchConfig>) -> Self {
        self.tool_search = search;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_is_capped() {
        let retry = RetryConfig::default().with_max_delay(Duration::from_secs(1));
        assert_eq!(retry.backoff(1, 0.0), Duration::from_millis(200));
        assert_eq!(retry.backoff(2, 0.0), Duration::from_millis(400));
        assert_eq!(retry.backoff(3, 1.0), Duration::from_millis(880));
        assert_eq!(retry.backoff(4, 0.0), Duration::from_secs(1));
        assert_eq!(retry.backoff(100, -1.0), Duration::from_secs(1));
    }
}
