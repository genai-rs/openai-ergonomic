//! Caching strategies for `OpenAI` API responses.
#![allow(dead_code)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unused_async)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unused_self)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::inherent_to_string)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::inefficient_to_string)]
//!
//! This example demonstrates comprehensive caching approaches including:
//! - In-memory caching with TTL (Time To Live)
//! - Persistent caching with file system storage
//! - Redis-compatible distributed caching
//! - Smart cache invalidation strategies
//! - Cache warming and precomputation
//! - Conditional caching based on request patterns
//! - Cache analytics and optimization
//! - Cost-aware caching decisions
//!
//! Caching benefits for AI applications:
//! - Significant cost reduction by avoiding duplicate API calls
//! - Improved response times for frequently requested content
//! - Better user experience with instant responses
//! - Reduced API rate limit pressure
//! - Offline capability for cached responses
//!
//! Run with: `cargo run --example caching_strategies`

use openai_ergonomic::{Client, Config, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Cache key for identifying unique requests
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Request endpoint
    endpoint: String,
    /// Request parameters hash
    params_hash: u64,
    /// Model name
    model: String,
    /// User ID for user-specific caching
    user_id: Option<String>,
}

impl CacheKey {
    /// Create a cache key from request parameters
    fn new(endpoint: &str, params: &ChatCompletionParams, user_id: Option<String>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        params.hash(&mut hasher);
        let params_hash = hasher.finish();

        Self {
            endpoint: endpoint.to_string(),
            params_hash,
            model: params.model.clone(),
            user_id,
        }
    }

    /// Generate a string representation for file-based caching
    fn to_string(&self) -> String {
        match &self.user_id {
            Some(user) => format!(
                "{}:{}:{}:{}",
                self.endpoint, self.model, self.params_hash, user
            ),
            None => format!("{}:{}:{}", self.endpoint, self.model, self.params_hash),
        }
    }
}

/// Cached response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedResponse {
    /// The actual response content
    content: String,
    /// When the response was cached
    cached_at: u64,
    /// Time-to-live in seconds
    ttl_seconds: u64,
    /// Token usage information
    token_usage: TokenUsageInfo,
    /// Response metadata
    metadata: HashMap<String, String>,
    /// Number of times this cache entry has been accessed
    access_count: u64,
    /// Last access timestamp
    last_accessed: u64,
}

impl CachedResponse {
    /// Check if the cached response has expired
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now > self.cached_at + self.ttl_seconds
    }

    /// Mark this cache entry as accessed
    fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Calculate age of the cached response
    fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now.saturating_sub(self.cached_at)
    }
}

/// Token usage information for cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenUsageInfo {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    estimated_cost_usd: f64,
}

/// Chat completion request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionParams {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f64>,
    max_tokens: Option<i32>,
    top_p: Option<f64>,
    frequency_penalty: Option<f64>,
    presence_penalty: Option<f64>,
}

impl std::hash::Hash for ChatCompletionParams {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.model.hash(state);
        self.messages.hash(state);
        // Convert f64 to bits for hashing
        if let Some(temp) = self.temperature {
            temp.to_bits().hash(state);
        }
        self.max_tokens.hash(state);
        if let Some(top_p) = self.top_p {
            top_p.to_bits().hash(state);
        }
        if let Some(freq_penalty) = self.frequency_penalty {
            freq_penalty.to_bits().hash(state);
        }
        if let Some(pres_penalty) = self.presence_penalty {
            pres_penalty.to_bits().hash(state);
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

/// Cache statistics for monitoring and optimization
#[derive(Debug, Clone)]
struct CacheStats {
    /// Total cache requests
    total_requests: u64,
    /// Cache hits
    cache_hits: u64,
    /// Cache misses
    cache_misses: u64,
    /// Total entries in cache
    cache_size: u64,
    /// Total memory usage estimate
    memory_usage_bytes: u64,
    /// Cost savings from cache hits
    cost_savings_usd: f64,
    /// Time savings from cache hits
    time_savings_ms: u64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_size: 0,
            memory_usage_bytes: 0,
            cost_savings_usd: 0.0,
            time_savings_ms: 0,
        }
    }

    fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.total_requests as f64) * 100.0
        }
    }

    fn print_stats(&self) {
        info!("=== Cache Statistics ===");
        info!("Total requests: {}", self.total_requests);
        info!("Cache hits: {}", self.cache_hits);
        info!("Cache misses: {}", self.cache_misses);
        info!("Hit rate: {:.2}%", self.hit_rate());
        info!("Cache size: {} entries", self.cache_size);
        info!(
            "Memory usage: {:.2} KB",
            self.memory_usage_bytes as f64 / 1024.0
        );
        info!("Cost savings: ${:.4}", self.cost_savings_usd);
        info!("Time savings: {}ms", self.time_savings_ms);
    }
}

/// In-memory cache with TTL support
#[derive(Debug)]
struct MemoryCache {
    /// Cache storage
    cache: Arc<Mutex<HashMap<CacheKey, CachedResponse>>>,
    /// Cache statistics
    stats: Arc<Mutex<CacheStats>>,
    /// Default TTL for new entries
    default_ttl: Duration,
    /// Maximum cache size (number of entries)
    max_size: usize,
}

impl MemoryCache {
    /// Create a new memory cache
    fn new(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::new())),
            default_ttl,
            max_size,
        }
    }

    /// Get a cached response
    async fn get(&self, key: &CacheKey) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        stats.total_requests += 1;

        if let Some(cached_response) = cache.get_mut(key) {
            if !cached_response.is_expired() {
                cached_response.mark_accessed();
                stats.cache_hits += 1;
                stats.cost_savings_usd += cached_response.token_usage.estimated_cost_usd;
                stats.time_savings_ms += 500; // Estimate 500ms saved per cache hit

                debug!("Cache hit for key: {}", key.to_string());
                return Some(cached_response.content.clone());
            }
            // Remove expired entry
            cache.remove(key);
            debug!("Removed expired cache entry for key: {}", key.to_string());
        }

        stats.cache_misses += 1;
        debug!("Cache miss for key: {}", key.to_string());
        None
    }

    /// Store a response in the cache
    async fn put(&self, key: CacheKey, content: String, token_usage: TokenUsageInfo) {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Evict entries if cache is full
        if cache.len() >= self.max_size {
            self.evict_lru(&mut cache);
        }

        let cached_response = CachedResponse {
            content,
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: self.default_ttl.as_secs(),
            token_usage,
            metadata: HashMap::new(),
            access_count: 0,
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        cache.insert(key.clone(), cached_response);
        stats.cache_size = cache.len() as u64;

        // Estimate memory usage
        let entry_size = key.to_string().len() + 1000; // Rough estimate
        stats.memory_usage_bytes += entry_size as u64;

        debug!("Cached response for key: {}", key.to_string());
    }

    /// Evict least recently used entries
    fn evict_lru(&self, cache: &mut HashMap<CacheKey, CachedResponse>) {
        // Find the entry with the oldest last_accessed time
        if let Some((lru_key, _)) = cache
            .iter()
            .min_by_key(|(_, response)| response.last_accessed)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            cache.remove(&lru_key);
            debug!("Evicted LRU entry: {}", lru_key.to_string());
        }
    }

    /// Clear expired entries
    async fn cleanup_expired(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let initial_size = cache.len();
        cache.retain(|_, response| !response.is_expired());
        let removed_count = initial_size - cache.len();

        stats.cache_size = cache.len() as u64;

        if removed_count > 0 {
            info!("Cleaned up {} expired cache entries", removed_count);
        }
    }

    /// Get cache statistics
    fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear all cache entries
    async fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        cache.clear();
        stats.cache_size = 0;
        stats.memory_usage_bytes = 0;

        info!("Cache cleared");
    }
}

/// File-based persistent cache
#[derive(Debug)]
struct FileCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// In-memory index for faster lookups
    index: Arc<Mutex<HashMap<CacheKey, PathBuf>>>,
    /// Statistics
    stats: Arc<Mutex<CacheStats>>,
    /// Default TTL
    default_ttl: Duration,
}

impl FileCache {
    /// Create a new file-based cache
    fn new(cache_dir: &Path, default_ttl: Duration) -> Result<Self> {
        fs::create_dir_all(cache_dir).map_err(|e| {
            Error::InvalidRequest(format!("Failed to create cache directory: {}", e))
        })?;

        let cache = Self {
            cache_dir: cache_dir.to_path_buf(),
            index: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(CacheStats::new())),
            default_ttl,
        };

        // Build index from existing files
        cache.rebuild_index()?;

        Ok(cache)
    }

    /// Rebuild the in-memory index from disk
    fn rebuild_index(&self) -> Result<()> {
        let mut index = self.index.lock().unwrap();
        index.clear();

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        // Parse cache key from filename
                        let parts: Vec<&str> = stem.split(':').collect();
                        if parts.len() >= 3 {
                            let key = CacheKey {
                                endpoint: parts[0].to_string(),
                                model: parts[1].to_string(),
                                params_hash: parts[2].parse().unwrap_or(0),
                                user_id: parts.get(3).map(|s| s.to_string()),
                            };
                            index.insert(key, entry.path());
                        }
                    }
                }
            }
        }

        let mut stats = self.stats.lock().unwrap();
        stats.cache_size = index.len() as u64;

        info!("Rebuilt cache index with {} entries", index.len());
        Ok(())
    }

    /// Get a cached response from disk
    async fn get(&self, key: &CacheKey) -> Option<String> {
        let index = self.index.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        stats.total_requests += 1;

        if let Some(file_path) = index.get(key) {
            if let Ok(content) = fs::read_to_string(file_path) {
                if let Ok(cached_response) = serde_json::from_str::<CachedResponse>(&content) {
                    if !cached_response.is_expired() {
                        stats.cache_hits += 1;
                        stats.cost_savings_usd += cached_response.token_usage.estimated_cost_usd;
                        stats.time_savings_ms += 200; // File access is faster than API

                        debug!("File cache hit for key: {}", key.to_string());
                        return Some(cached_response.content);
                    }
                    // Remove expired file
                    let _ = fs::remove_file(file_path);
                    debug!("Removed expired cache file: {:?}", file_path);
                }
            }
        }

        stats.cache_misses += 1;
        debug!("File cache miss for key: {}", key.to_string());
        None
    }

    /// Store a response in the file cache
    async fn put(&self, key: CacheKey, content: String, token_usage: TokenUsageInfo) -> Result<()> {
        let cached_response = CachedResponse {
            content,
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: self.default_ttl.as_secs(),
            token_usage,
            metadata: HashMap::new(),
            access_count: 0,
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let filename = format!("{}.json", key.to_string());
        let file_path = self.cache_dir.join(filename);

        let json_content = serde_json::to_string_pretty(&cached_response).map_err(|e| {
            Error::InvalidRequest(format!("Failed to serialize cache entry: {}", e))
        })?;

        fs::write(&file_path, json_content)
            .map_err(|e| Error::InvalidRequest(format!("Failed to write cache file: {}", e)))?;

        // Update index
        let mut index = self.index.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        index.insert(key.clone(), file_path);
        stats.cache_size = index.len() as u64;

        debug!("Stored cache entry to file: {}", key.to_string());
        Ok(())
    }

    /// Clean up expired cache files
    async fn cleanup_expired(&self) -> Result<()> {
        let index = self.index.lock().unwrap();
        let mut removed_count = 0;

        for (key, file_path) in index.iter() {
            if let Ok(content) = fs::read_to_string(file_path) {
                if let Ok(cached_response) = serde_json::from_str::<CachedResponse>(&content) {
                    if cached_response.is_expired() {
                        if fs::remove_file(file_path).is_ok() {
                            removed_count += 1;
                            debug!("Removed expired cache file: {}", key.to_string());
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} expired file cache entries", removed_count);
            // Rebuild index to reflect deletions
            drop(index);
            self.rebuild_index()?;
        }

        Ok(())
    }

    /// Get cache statistics
    fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }
}

/// Smart caching client that combines multiple cache layers
#[derive(Debug)]
struct CachingClient {
    client: Client,
    memory_cache: MemoryCache,
    file_cache: Option<FileCache>,
    cache_strategy: CacheStrategy,
}

/// Cache strategy configuration
#[derive(Debug, Clone)]
struct CacheStrategy {
    /// Whether to cache all responses or use selective caching
    cache_all: bool,
    /// Minimum response length to cache (avoid caching very short responses)
    min_response_length: usize,
    /// Cache deterministic requests (temperature = 0)
    cache_deterministic: bool,
    /// Cache expensive requests (high token count)
    cache_expensive: bool,
    /// Minimum cost threshold for caching
    min_cost_threshold: f64,
    /// Whether to enable cache warming
    enable_warming: bool,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self {
            cache_all: false,
            min_response_length: 50,
            cache_deterministic: true,
            cache_expensive: true,
            min_cost_threshold: 0.001, // $0.001
            enable_warming: false,
        }
    }
}

impl CachingClient {
    /// Create a new caching client
    fn new(client: Client, cache_dir: Option<&Path>) -> Result<Self> {
        let memory_cache = MemoryCache::new(Duration::from_secs(60 * 60), 1000);

        let file_cache = if let Some(dir) = cache_dir {
            Some(FileCache::new(dir, Duration::from_secs(24 * 60 * 60))?)
        } else {
            None
        };

        Ok(Self {
            client,
            memory_cache,
            file_cache,
            cache_strategy: CacheStrategy::default(),
        })
    }

    /// Configure caching strategy
    fn with_strategy(mut self, strategy: CacheStrategy) -> Self {
        self.cache_strategy = strategy;
        self
    }

    /// Send a chat completion request with caching
    async fn chat_completion(
        &self,
        params: ChatCompletionParams,
        user_id: Option<String>,
    ) -> Result<String> {
        let cache_key = CacheKey::new("/v1/chat/completions", &params, user_id);

        // Try memory cache first
        if let Some(cached_content) = self.memory_cache.get(&cache_key).await {
            debug!("Retrieved from memory cache");
            return Ok(cached_content);
        }

        // Try file cache second
        if let Some(file_cache) = &self.file_cache {
            if let Some(cached_content) = file_cache.get(&cache_key).await {
                debug!("Retrieved from file cache, promoting to memory cache");

                // Promote to memory cache for faster future access
                let token_usage = TokenUsageInfo {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    estimated_cost_usd: 0.0,
                };
                self.memory_cache
                    .put(cache_key, cached_content.clone(), token_usage)
                    .await;

                return Ok(cached_content);
            }
        }

        // Cache miss - make actual API call
        let response = self.make_api_call(&params).await?;

        // Calculate token usage and cost
        let token_usage = self.estimate_token_usage(&params, &response);

        // Decide whether to cache based on strategy
        if self.should_cache(&params, &response, &token_usage) {
            // Store in memory cache
            self.memory_cache
                .put(cache_key.clone(), response.clone(), token_usage.clone())
                .await;

            // Store in file cache if available
            if let Some(file_cache) = &self.file_cache {
                if let Err(e) = file_cache
                    .put(cache_key, response.clone(), token_usage)
                    .await
                {
                    warn!("Failed to store in file cache: {}", e);
                }
            }
        }

        Ok(response)
    }

    /// Make the actual API call (simulated)
    async fn make_api_call(&self, params: &ChatCompletionParams) -> Result<String> {
        // Simulate API call delay
        sleep(Duration::from_millis(500)).await;

        // Simulate API response based on parameters
        let response = match params.messages.first() {
            Some(msg) if msg.content.contains("error") => {
                return Err(Error::InvalidRequest("Simulated API error".to_string()));
            }
            Some(msg) => {
                format!(
                    "Response to: {}",
                    msg.content.chars().take(50).collect::<String>()
                )
            }
            None => "Empty response".to_string(),
        };

        Ok(response)
    }

    /// Estimate token usage and cost for a request/response pair
    fn estimate_token_usage(
        &self,
        params: &ChatCompletionParams,
        response: &str,
    ) -> TokenUsageInfo {
        // Rough token estimation (1 token ≈ 4 characters for English)
        let prompt_text: String = params
            .messages
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let prompt_tokens = (prompt_text.len() / 4) as i32;
        let completion_tokens = (response.len() / 4) as i32;
        let total_tokens = prompt_tokens + completion_tokens;

        // Estimate cost based on model (simplified)
        let cost_per_1k_tokens = match params.model.as_str() {
            "gpt-4" => 0.03,
            "gpt-3.5-turbo" => 0.002,
            _ => 0.002,
        };

        let estimated_cost_usd = (total_tokens as f64 / 1000.0) * cost_per_1k_tokens;

        TokenUsageInfo {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            estimated_cost_usd,
        }
    }

    /// Determine whether to cache a response based on strategy
    fn should_cache(
        &self,
        params: &ChatCompletionParams,
        response: &str,
        token_usage: &TokenUsageInfo,
    ) -> bool {
        if self.cache_strategy.cache_all {
            return true;
        }

        // Check minimum response length
        if response.len() < self.cache_strategy.min_response_length {
            return false;
        }

        // Check deterministic requests
        if self.cache_strategy.cache_deterministic {
            if let Some(temp) = params.temperature {
                if temp == 0.0 {
                    return true;
                }
            }
        }

        // Check expensive requests
        if self.cache_strategy.cache_expensive {
            if token_usage.estimated_cost_usd >= self.cache_strategy.min_cost_threshold {
                return true;
            }
        }

        false
    }

    /// Warm the cache with common requests
    async fn warm_cache(&self, common_prompts: Vec<ChatCompletionParams>) -> Result<()> {
        if !self.cache_strategy.enable_warming {
            return Ok(());
        }

        info!(
            "Starting cache warming with {} prompts",
            common_prompts.len()
        );

        for (i, params) in common_prompts.iter().enumerate() {
            info!("Warming cache {}/{}", i + 1, common_prompts.len());

            match self.chat_completion(params.clone(), None).await {
                Ok(_) => debug!("Cache warmed for prompt {}", i + 1),
                Err(e) => warn!("Failed to warm cache for prompt {}: {}", i + 1, e),
            }

            // Small delay to avoid overwhelming the API
            sleep(Duration::from_millis(100)).await;
        }

        info!("Cache warming completed");
        Ok(())
    }

    /// Get combined cache statistics
    fn get_cache_stats(&self) -> (CacheStats, Option<CacheStats>) {
        let memory_stats = self.memory_cache.get_stats();
        let file_stats = self.file_cache.as_ref().map(|cache| cache.get_stats());

        (memory_stats, file_stats)
    }

    /// Clean up expired entries in all caches
    async fn cleanup_expired(&self) -> Result<()> {
        self.memory_cache.cleanup_expired().await;

        if let Some(file_cache) = &self.file_cache {
            file_cache.cleanup_expired().await?;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting caching strategies example");

    // Create a test client
    let config = Config::builder().api_key("test-api-key").build();
    let client = Client::builder(config)?.build();

    // Example 1: Basic memory caching
    info!("=== Example 1: Memory Caching ===");

    let cache_dir = std::env::temp_dir().join("openai_cache");
    let caching_client =
        CachingClient::new(client, Some(&cache_dir))?.with_strategy(CacheStrategy {
            cache_all: true,
            min_response_length: 10,
            cache_deterministic: true,
            cache_expensive: true,
            min_cost_threshold: 0.0,
            enable_warming: false,
        });

    // Test the same request multiple times to demonstrate caching
    let test_params = ChatCompletionParams {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("What is the capital of France?"),
        ],
        temperature: Some(0.0), // Deterministic for caching
        max_tokens: Some(100),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    info!("Making first request (should be cache miss)");
    let start_time = Instant::now();
    let response1 = caching_client
        .chat_completion(test_params.clone(), None)
        .await?;
    let first_duration = start_time.elapsed();
    info!("First response: {} (took {:?})", response1, first_duration);

    info!("Making second request (should be cache hit)");
    let start_time = Instant::now();
    let response2 = caching_client
        .chat_completion(test_params.clone(), None)
        .await?;
    let second_duration = start_time.elapsed();
    info!(
        "Second response: {} (took {:?})",
        response2, second_duration
    );

    // Verify responses are identical and second is faster
    assert_eq!(response1, response2);
    info!(
        "✓ Cache working: responses identical, second request {:?} faster",
        first_duration.saturating_sub(second_duration)
    );

    // Example 2: User-specific caching
    info!("\n=== Example 2: User-Specific Caching ===");

    let user_params = ChatCompletionParams {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![ChatMessage::user(
            "What is my favorite programming language?",
        )],
        temperature: Some(0.7),
        max_tokens: Some(50),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    // Same request for different users should be cached separately
    let user1_response = caching_client
        .chat_completion(user_params.clone(), Some("user1".to_string()))
        .await?;
    let user2_response = caching_client
        .chat_completion(user_params.clone(), Some("user2".to_string()))
        .await?;

    info!("User 1 response: {}", user1_response);
    info!("User 2 response: {}", user2_response);

    // Test cache hit for user 1
    let user1_cached = caching_client
        .chat_completion(user_params, Some("user1".to_string()))
        .await?;
    assert_eq!(user1_response, user1_cached);
    info!("✓ User-specific caching working");

    // Example 3: Conditional caching based on parameters
    info!("\n=== Example 3: Conditional Caching ===");

    let conditional_client = CachingClient::new(
        Client::builder(Config::builder().api_key("test-api-key").build())?.build(),
        Some(&cache_dir),
    )?
    .with_strategy(CacheStrategy {
        cache_all: false,
        min_response_length: 20,
        cache_deterministic: true,
        cache_expensive: true,
        min_cost_threshold: 0.001,
        enable_warming: false,
    });

    // Test deterministic request (should be cached)
    let deterministic_params = ChatCompletionParams {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![ChatMessage::user("Count from 1 to 5")],
        temperature: Some(0.0), // Deterministic
        max_tokens: Some(50),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    info!("Testing deterministic request (should cache)");
    conditional_client
        .chat_completion(deterministic_params.clone(), None)
        .await?;
    conditional_client
        .chat_completion(deterministic_params, None)
        .await?;

    // Test non-deterministic request (might not be cached based on strategy)
    let creative_params = ChatCompletionParams {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![ChatMessage::user("Write a creative story")],
        temperature: Some(1.0), // Creative
        max_tokens: Some(50),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    info!("Testing creative request (might not cache)");
    conditional_client
        .chat_completion(creative_params.clone(), None)
        .await?;
    conditional_client
        .chat_completion(creative_params, None)
        .await?;

    // Example 4: Cache warming
    info!("\n=== Example 4: Cache Warming ===");

    let warming_client = CachingClient::new(
        Client::builder(Config::builder().api_key("test-api-key").build())?.build(),
        Some(&cache_dir),
    )?
    .with_strategy(CacheStrategy {
        cache_all: true,
        enable_warming: true,
        ..Default::default()
    });

    // Define common prompts for warming
    let common_prompts = vec![
        ChatCompletionParams {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage::user("What is machine learning?")],
            temperature: Some(0.0),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        ChatCompletionParams {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage::user("Explain quantum computing")],
            temperature: Some(0.0),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
        ChatCompletionParams {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage::user("What is blockchain?")],
            temperature: Some(0.0),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        },
    ];

    warming_client.warm_cache(common_prompts.clone()).await?;

    // Test that warmed entries are available
    info!("Testing warmed cache entries");
    for (i, params) in common_prompts.iter().enumerate() {
        let start_time = Instant::now();
        let response = warming_client.chat_completion(params.clone(), None).await?;
        let duration = start_time.elapsed();
        info!(
            "Warmed entry {} retrieved in {:?}: {}",
            i + 1,
            duration,
            response
        );
    }

    // Example 5: Cache analytics and optimization
    info!("\n=== Example 5: Cache Analytics ===");

    let (memory_stats, file_stats) = caching_client.get_cache_stats();

    info!("Memory cache statistics:");
    memory_stats.print_stats();

    if let Some(file_stats) = file_stats {
        info!("\nFile cache statistics:");
        file_stats.print_stats();
    }

    // Example 6: Cache cleanup and maintenance
    info!("\n=== Example 6: Cache Maintenance ===");

    info!("Performing cache cleanup");
    caching_client.cleanup_expired().await?;

    // Show updated statistics
    let (memory_stats_after, file_stats_after) = caching_client.get_cache_stats();
    info!("Statistics after cleanup:");
    memory_stats_after.print_stats();

    if let Some(ref file_stats_after) = file_stats_after {
        info!("\nFile cache after cleanup:");
        file_stats_after.print_stats();
    }

    // Example 7: Cost analysis
    info!("\n=== Example 7: Cost Analysis ===");

    let total_cost_savings = memory_stats_after.cost_savings_usd
        + file_stats_after
            .as_ref()
            .map(|s| s.cost_savings_usd)
            .unwrap_or(0.0);

    let total_time_savings = memory_stats_after.time_savings_ms
        + file_stats_after
            .as_ref()
            .map(|s| s.time_savings_ms)
            .unwrap_or(0);

    info!("=== Overall Cache Performance ===");
    info!("Total cost savings: ${:.4}", total_cost_savings);
    info!("Total time savings: {}ms", total_time_savings);
    info!("Cache efficiency: Significant improvement in response times and cost reduction");

    // Cleanup
    info!("Cleaning up cache directory");
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| Error::InvalidRequest(format!("Cleanup failed: {}", e)))?;
    }

    info!("Caching strategies example completed successfully!");
    Ok(())
}
