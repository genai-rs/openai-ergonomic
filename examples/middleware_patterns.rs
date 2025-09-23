//! Middleware patterns for `OpenAI` API request/response processing.
#![allow(dead_code)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unnecessary_literal_bound)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::needless_continue)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]
//!
//! This example demonstrates advanced middleware architectures including:
//! - Request/response interceptors for cross-cutting concerns
//! - Authentication and authorization middleware
//! - Rate limiting and throttling middleware
//! - Logging and observability middleware
//! - Request transformation and validation
//! - Response transformation and filtering
//! - Error handling and retry middleware
//! - Custom middleware composition and chaining
//!
//! Middleware patterns enable:
//! - Separation of concerns for cross-cutting functionality
//! - Modular and reusable request/response processing
//! - Centralized logging, monitoring, and debugging
//! - Flexible request routing and transformation
//! - Standardized error handling across the application
//!
//! Run with: `cargo run --example middleware_patterns`

use async_trait::async_trait;
use openai_ergonomic::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Core middleware trait that all middleware must implement
#[async_trait]
trait Middleware: Send + Sync + std::fmt::Debug {
    /// Process a request before it's sent to the API
    async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
        let _ = context;
        Ok(())
    }

    /// Process a response after it's received from the API
    async fn after_response(&self, context: &mut ResponseContext) -> Result<()> {
        let _ = context;
        Ok(())
    }

    /// Handle errors that occur during request processing
    async fn on_error(&self, context: &mut ErrorContext) -> Result<ErrorAction> {
        let _ = context;
        Ok(ErrorAction::Propagate)
    }

    /// Get middleware name for logging and debugging
    fn name(&self) -> &str;

    /// Get middleware priority (lower values execute first)
    fn priority(&self) -> i32 {
        100
    }
}

/// Request context containing all information about the current request
#[derive(Debug)]
struct RequestContext {
    /// HTTP method
    method: String,
    /// Request URL
    url: String,
    /// Request headers
    headers: HashMap<String, String>,
    /// Request body
    body: String,
    /// Custom metadata for middleware communication
    metadata: HashMap<String, String>,
    /// Request start time
    start_time: Instant,
    /// Request ID for tracing
    request_id: String,
}

/// Response context containing response data and processing information
#[derive(Debug)]
struct ResponseContext {
    /// Original request context
    request: RequestContext,
    /// Response status code
    status_code: u16,
    /// Response headers
    headers: HashMap<String, String>,
    /// Response body
    body: String,
    /// Response processing duration
    duration: Duration,
    /// Custom metadata
    metadata: HashMap<String, String>,
}

/// Error context for error handling middleware
#[derive(Debug)]
struct ErrorContext {
    /// Original request context
    request: RequestContext,
    /// The error that occurred
    error: Error,
    /// Number of retry attempts so far
    retry_count: i32,
    /// Custom metadata
    metadata: HashMap<String, String>,
}

/// Action to take after error handling
#[derive(Debug)]
enum ErrorAction {
    /// Propagate the error up the chain
    Propagate,
    /// Retry the request
    Retry,
    /// Return a custom response
    CustomResponse(String),
    /// Transform the error
    TransformError(Error),
}

/// Middleware manager that orchestrates the middleware chain
#[derive(Debug)]
struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add middleware to the chain
    fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        // Sort by priority to ensure correct execution order
        self.middlewares.sort_by_key(|m| m.priority());
        self
    }

    /// Process a request through the entire middleware chain
    async fn process_request(&self, mut context: RequestContext) -> Result<ResponseContext> {
        // Execute before_request for all middleware
        for middleware in &self.middlewares {
            debug!(
                "Executing before_request for middleware: {}",
                middleware.name()
            );
            if let Err(e) = middleware.before_request(&mut context).await {
                warn!(
                    "Middleware {} failed in before_request: {}",
                    middleware.name(),
                    e
                );
                return self
                    .handle_error(ErrorContext {
                        request: context,
                        error: e,
                        retry_count: 0,
                        metadata: HashMap::new(),
                    })
                    .await;
            }
        }

        // Execute the actual request (simulated here)
        let response_result = self.execute_request(&context).await;

        match response_result {
            Ok(mut response_context) => {
                // Execute after_response for all middleware (in reverse order)
                for middleware in self.middlewares.iter().rev() {
                    debug!(
                        "Executing after_response for middleware: {}",
                        middleware.name()
                    );
                    if let Err(e) = middleware.after_response(&mut response_context).await {
                        warn!(
                            "Middleware {} failed in after_response: {}",
                            middleware.name(),
                            e
                        );
                        return self
                            .handle_error(ErrorContext {
                                request: response_context.request,
                                error: e,
                                retry_count: 0,
                                metadata: HashMap::new(),
                            })
                            .await;
                    }
                }
                Ok(response_context)
            }
            Err(e) => {
                self.handle_error(ErrorContext {
                    request: context,
                    error: e,
                    retry_count: 0,
                    metadata: HashMap::new(),
                })
                .await
            }
        }
    }

    /// Handle errors through the error handling chain
    async fn handle_error(&self, mut error_context: ErrorContext) -> Result<ResponseContext> {
        for middleware in &self.middlewares {
            debug!("Executing on_error for middleware: {}", middleware.name());
            match middleware.on_error(&mut error_context).await? {
                ErrorAction::Propagate => {}
                ErrorAction::Retry => {
                    if error_context.retry_count < 3 {
                        info!("Retrying request due to middleware: {}", middleware.name());
                        error_context.retry_count += 1;
                        // Add delay before retry
                        sleep(Duration::from_millis(
                            1000 * (error_context.retry_count as u64),
                        ))
                        .await;
                        return Box::pin(self.process_request(error_context.request)).await;
                    }
                    warn!("Max retries exceeded, propagating error");
                }
                ErrorAction::CustomResponse(body) => {
                    info!(
                        "Returning custom response from middleware: {}",
                        middleware.name()
                    );
                    return Ok(ResponseContext {
                        request: error_context.request,
                        status_code: 200,
                        headers: HashMap::new(),
                        body,
                        duration: Duration::from_millis(0),
                        metadata: HashMap::new(),
                    });
                }
                ErrorAction::TransformError(new_error) => {
                    error_context.error = new_error;
                    continue;
                }
            }
        }

        Err(error_context.error)
    }

    /// Execute the actual HTTP request (simulated)
    async fn execute_request(&self, context: &RequestContext) -> Result<ResponseContext> {
        debug!("Executing request: {} {}", context.method, context.url);

        // Simulate network delay
        sleep(Duration::from_millis(100)).await;

        // Simulate successful response
        let response_body = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1_677_652_288,
            "model": "gpt-3.5-turbo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "This is a simulated response processed through middleware"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 15,
                "total_tokens": 35
            }
        })
        .to_string();

        Ok(ResponseContext {
            request: RequestContext {
                method: context.method.clone(),
                url: context.url.clone(),
                headers: context.headers.clone(),
                body: context.body.clone(),
                metadata: context.metadata.clone(),
                start_time: context.start_time,
                request_id: context.request_id.clone(),
            },
            status_code: 200,
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: response_body,
            duration: context.start_time.elapsed(),
            metadata: HashMap::new(),
        })
    }
}

/// Authentication middleware for API key management
#[derive(Debug)]
struct AuthenticationMiddleware {
    api_key: String,
    organization_id: Option<String>,
}

impl AuthenticationMiddleware {
    fn new(api_key: String, organization_id: Option<String>) -> Self {
        Self {
            api_key,
            organization_id,
        }
    }
}

#[async_trait]
impl Middleware for AuthenticationMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
        debug!("Adding authentication headers");

        // Add API key header
        context.headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );

        // Add organization header if provided
        if let Some(org_id) = &self.organization_id {
            context
                .headers
                .insert("OpenAI-Organization".to_string(), org_id.clone());
        }

        context
            .metadata
            .insert("authenticated".to_string(), "true".to_string());

        Ok(())
    }

    fn name(&self) -> &str {
        "authentication"
    }

    fn priority(&self) -> i32 {
        10 // Execute early
    }
}

/// Rate limiting middleware to prevent API quota exhaustion
#[derive(Debug)]
struct RateLimitingMiddleware {
    /// Requests per minute limit
    requests_per_minute: i32,
    /// Request timestamps for tracking
    request_timestamps: Arc<Mutex<Vec<SystemTime>>>,
}

impl RateLimitingMiddleware {
    fn new(requests_per_minute: i32) -> Self {
        Self {
            requests_per_minute,
            request_timestamps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn should_rate_limit(&self) -> bool {
        let mut timestamps = self.request_timestamps.lock().unwrap();
        let now = SystemTime::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Remove timestamps older than 1 minute
        timestamps.retain(|&timestamp| timestamp > one_minute_ago);

        // Check if we're at the limit
        timestamps.len() >= self.requests_per_minute as usize
    }

    fn record_request(&self) {
        let mut timestamps = self.request_timestamps.lock().unwrap();
        timestamps.push(SystemTime::now());
    }
}

#[async_trait]
impl Middleware for RateLimitingMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
        if self.should_rate_limit() {
            warn!("Rate limit exceeded, delaying request");

            // Calculate delay until rate limit resets
            let delay = Duration::from_secs(60) / self.requests_per_minute as u32;
            sleep(delay).await;
        }

        self.record_request();
        context
            .metadata
            .insert("rate_limited".to_string(), "checked".to_string());

        debug!("Rate limiting check passed");
        Ok(())
    }

    fn name(&self) -> &str {
        "rate_limiting"
    }

    fn priority(&self) -> i32 {
        20
    }
}

/// Logging middleware for comprehensive request/response logging
#[derive(Debug)]
struct LoggingMiddleware {
    log_requests: bool,
    log_responses: bool,
    log_headers: bool,
    log_body: bool,
}

impl LoggingMiddleware {
    fn new(log_requests: bool, log_responses: bool, log_headers: bool, log_body: bool) -> Self {
        Self {
            log_requests,
            log_responses,
            log_headers,
            log_body,
        }
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
        if self.log_requests {
            info!(
                "Request [{}]: {} {}",
                context.request_id, context.method, context.url
            );

            if self.log_headers {
                for (key, value) in &context.headers {
                    debug!(
                        "Request header [{}]: {}: {}",
                        context.request_id, key, value
                    );
                }
            }

            if self.log_body && !context.body.is_empty() {
                debug!("Request body [{}]: {}", context.request_id, context.body);
            }
        }

        Ok(())
    }

    async fn after_response(&self, context: &mut ResponseContext) -> Result<()> {
        if self.log_responses {
            info!(
                "Response [{}]: {} ({:?})",
                context.request.request_id, context.status_code, context.duration
            );

            if self.log_headers {
                for (key, value) in &context.headers {
                    debug!(
                        "Response header [{}]: {}: {}",
                        context.request.request_id, key, value
                    );
                }
            }

            if self.log_body {
                debug!(
                    "Response body [{}]: {}",
                    context.request.request_id, context.body
                );
            }
        }

        Ok(())
    }

    async fn on_error(&self, context: &mut ErrorContext) -> Result<ErrorAction> {
        error!(
            "Request error [{}]: {}",
            context.request.request_id, context.error
        );

        Ok(ErrorAction::Propagate)
    }

    fn name(&self) -> &str {
        "logging"
    }

    fn priority(&self) -> i32 {
        1000 // Execute last for before_request, first for after_response
    }
}

/// Request validation middleware
#[derive(Debug)]
struct ValidationMiddleware {
    max_request_size: usize,
    required_headers: Vec<String>,
}

impl ValidationMiddleware {
    fn new(max_request_size: usize, required_headers: Vec<String>) -> Self {
        Self {
            max_request_size,
            required_headers,
        }
    }
}

#[async_trait]
impl Middleware for ValidationMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
        // Validate request size
        if context.body.len() > self.max_request_size {
            return Err(Error::InvalidRequest(format!(
                "Request body too large: {} bytes (max: {} bytes)",
                context.body.len(),
                self.max_request_size
            )));
        }

        // Validate required headers
        for header in &self.required_headers {
            if !context.headers.contains_key(header) {
                return Err(Error::InvalidRequest(format!(
                    "Required header missing: {}",
                    header
                )));
            }
        }

        context
            .metadata
            .insert("validated".to_string(), "true".to_string());

        debug!("Request validation passed");
        Ok(())
    }

    fn name(&self) -> &str {
        "validation"
    }

    fn priority(&self) -> i32 {
        15
    }
}

/// Response transformation middleware
#[derive(Debug)]
struct ResponseTransformationMiddleware;

#[async_trait]
impl Middleware for ResponseTransformationMiddleware {
    async fn after_response(&self, context: &mut ResponseContext) -> Result<()> {
        // Add processing metadata to response
        if let Ok(mut response_json) = serde_json::from_str::<serde_json::Value>(&context.body) {
            if let Some(obj) = response_json.as_object_mut() {
                obj.insert(
                    "processing_time_ms".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(
                        context.duration.as_millis() as u64,
                    )),
                );

                obj.insert(
                    "request_id".to_string(),
                    serde_json::Value::String(context.request.request_id.clone()),
                );

                context.body = serde_json::to_string(&response_json).map_err(|e| {
                    Error::InvalidRequest(format!("Response transformation failed: {}", e))
                })?;
            }
        }

        debug!("Response transformation completed");
        Ok(())
    }

    fn name(&self) -> &str {
        "response_transformation"
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Retry middleware with exponential backoff
#[derive(Debug)]
struct RetryMiddleware {
    max_retries: i32,
    base_delay_ms: u64,
}

impl RetryMiddleware {
    fn new(max_retries: i32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
        }
    }
}

#[async_trait]
impl Middleware for RetryMiddleware {
    async fn on_error(&self, context: &mut ErrorContext) -> Result<ErrorAction> {
        match &context.error {
            Error::InvalidRequest(msg) if msg.contains("rate limit") || msg.contains("timeout") => {
                if context.retry_count < self.max_retries {
                    let delay_ms = self.base_delay_ms * 2_u64.pow(context.retry_count as u32);
                    info!(
                        "Retrying request {} (attempt {}/{}) after {}ms",
                        context.request.request_id,
                        context.retry_count + 1,
                        self.max_retries,
                        delay_ms
                    );

                    sleep(Duration::from_millis(delay_ms)).await;
                    Ok(ErrorAction::Retry)
                } else {
                    warn!(
                        "Max retries ({}) exceeded for request {}",
                        self.max_retries, context.request.request_id
                    );
                    Ok(ErrorAction::Propagate)
                }
            }
            _ => Ok(ErrorAction::Propagate),
        }
    }

    fn name(&self) -> &str {
        "retry"
    }

    fn priority(&self) -> i32 {
        900 // Execute late in error handling
    }
}

/// Metrics collection middleware
#[derive(Debug)]
struct MetricsMiddleware {
    request_count: Arc<Mutex<u64>>,
    error_count: Arc<Mutex<u64>>,
    total_duration: Arc<Mutex<Duration>>,
}

impl MetricsMiddleware {
    fn new() -> Self {
        Self {
            request_count: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
            total_duration: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    fn get_metrics(&self) -> MetricsSnapshot {
        let request_count = *self.request_count.lock().unwrap();
        let error_count = *self.error_count.lock().unwrap();
        let total_duration = *self.total_duration.lock().unwrap();

        MetricsSnapshot {
            request_count,
            error_count,
            success_count: request_count - error_count,
            average_duration: if request_count > 0 {
                total_duration / request_count as u32
            } else {
                Duration::ZERO
            },
            error_rate: if request_count > 0 {
                (error_count as f64 / request_count as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
struct MetricsSnapshot {
    request_count: u64,
    error_count: u64,
    success_count: u64,
    average_duration: Duration,
    error_rate: f64,
}

impl MetricsSnapshot {
    fn print_metrics(&self) {
        info!("=== Metrics Summary ===");
        info!("Total requests: {}", self.request_count);
        info!("Successful requests: {}", self.success_count);
        info!("Failed requests: {}", self.error_count);
        info!("Error rate: {:.2}%", self.error_rate);
        info!("Average duration: {:?}", self.average_duration);
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn before_request(&self, _context: &mut RequestContext) -> Result<()> {
        *self.request_count.lock().unwrap() += 1;
        Ok(())
    }

    async fn after_response(&self, context: &mut ResponseContext) -> Result<()> {
        *self.total_duration.lock().unwrap() += context.duration;
        Ok(())
    }

    async fn on_error(&self, _context: &mut ErrorContext) -> Result<ErrorAction> {
        *self.error_count.lock().unwrap() += 1;
        Ok(ErrorAction::Propagate)
    }

    fn name(&self) -> &str {
        "metrics"
    }

    fn priority(&self) -> i32 {
        1
    }
}

/// Enhanced client with middleware support
#[derive(Debug)]
struct MiddlewareClient {
    chain: MiddlewareChain,
    base_url: String,
}

impl MiddlewareClient {
    /// Create a new middleware-enabled client
    fn new(base_url: String) -> Self {
        Self {
            chain: MiddlewareChain::new(),
            base_url,
        }
    }

    /// Add middleware to the client
    fn with_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.chain = self.chain.add_middleware(middleware);
        self
    }

    /// Send a chat completion request through the middleware chain
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request_id = format!("req_{}", generate_request_id());

        let request_body = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": messages,
            "max_tokens": 150
        });

        let context = RequestContext {
            method: "POST".to_string(),
            url: format!("{}/v1/chat/completions", self.base_url),
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: request_body.to_string(),
            metadata: HashMap::new(),
            start_time: Instant::now(),
            request_id,
        };

        let response = self.chain.process_request(context).await?;

        // Extract content from response
        if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response.body) {
            if let Some(choices) = response_json["choices"].as_array() {
                if let Some(first_choice) = choices.first() {
                    if let Some(content) = first_choice["message"]["content"].as_str() {
                        return Ok(content.to_string());
                    }
                }
            }
        }

        Err(Error::InvalidRequest(
            "Failed to parse response".to_string(),
        ))
    }
}

/// Chat message for middleware client
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Generate a unique request ID
fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
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

    info!("Starting middleware patterns example");

    // Example 1: Basic middleware chain
    info!("=== Example 1: Basic Middleware Chain ===");

    let metrics_middleware = Arc::new(MetricsMiddleware::new());
    let metrics_ref = Arc::clone(&metrics_middleware);

    let client = MiddlewareClient::new("https://api.openai.com".to_string())
        .with_middleware(Arc::new(AuthenticationMiddleware::new(
            "test-api-key".to_string(),
            Some("org-test".to_string()),
        )))
        .with_middleware(Arc::new(ValidationMiddleware::new(
            10_000, // 10KB max request size
            vec!["Authorization".to_string()],
        )))
        .with_middleware(Arc::new(RateLimitingMiddleware::new(60))) // 60 requests per minute
        .with_middleware(Arc::new(LoggingMiddleware::new(true, true, false, false)))
        .with_middleware(Arc::new(ResponseTransformationMiddleware))
        .with_middleware(metrics_middleware);

    // Send a test request
    let messages = vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Hello, how are you?"),
    ];

    match client.chat_completion(messages).await {
        Ok(response) => {
            info!("Received response: {}", response);
        }
        Err(e) => {
            error!("Request failed: {}", e);
        }
    }

    // Print metrics
    metrics_ref.get_metrics().print_metrics();

    // Example 2: Error handling and retry middleware
    info!("\n=== Example 2: Error Handling and Retry ===");

    let error_client = MiddlewareClient::new("https://api.openai.com".to_string())
        .with_middleware(Arc::new(AuthenticationMiddleware::new(
            "test-api-key".to_string(),
            None,
        )))
        .with_middleware(Arc::new(RetryMiddleware::new(3, 1000))) // 3 retries, 1s base delay
        .with_middleware(Arc::new(LoggingMiddleware::new(true, true, false, false)));

    // This would normally trigger retry logic with real API errors
    let retry_messages = vec![ChatMessage::user("Test retry functionality")];

    match error_client.chat_completion(retry_messages).await {
        Ok(response) => {
            info!("Retry example completed: {}", response);
        }
        Err(e) => {
            warn!("Retry example failed after all attempts: {}", e);
        }
    }

    // Example 3: Custom middleware for request modification
    info!("\n=== Example 3: Custom Request Modification ===");

    #[derive(Debug)]
    struct CustomHeaderMiddleware {
        headers: HashMap<String, String>,
    }

    impl CustomHeaderMiddleware {
        fn new() -> Self {
            let mut headers = HashMap::new();
            headers.insert(
                "X-Custom-Client".to_string(),
                "openai-ergonomic".to_string(),
            );
            headers.insert(
                "X-Request-Source".to_string(),
                "middleware-example".to_string(),
            );

            Self { headers }
        }
    }

    #[async_trait]
    impl Middleware for CustomHeaderMiddleware {
        async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
            for (key, value) in &self.headers {
                context.headers.insert(key.clone(), value.clone());
            }

            // Modify request body to add custom parameters
            if let Ok(mut body_json) = serde_json::from_str::<serde_json::Value>(&context.body) {
                if let Some(obj) = body_json.as_object_mut() {
                    obj.insert(
                        "temperature".to_string(),
                        serde_json::Value::Number(serde_json::Number::from_f64(0.7).unwrap()),
                    );
                    obj.insert(
                        "user".to_string(),
                        serde_json::Value::String(format!(
                            "middleware_user_{}",
                            context.request_id
                        )),
                    );

                    context.body = serde_json::to_string(&body_json).map_err(|e| {
                        Error::InvalidRequest(format!("Body modification failed: {}", e))
                    })?;
                }
            }

            debug!("Added custom headers and modified request body");
            Ok(())
        }

        fn name(&self) -> &str {
            "custom_header"
        }

        fn priority(&self) -> i32 {
            30
        }
    }

    let custom_client = MiddlewareClient::new("https://api.openai.com".to_string())
        .with_middleware(Arc::new(AuthenticationMiddleware::new(
            "test-api-key".to_string(),
            None,
        )))
        .with_middleware(Arc::new(CustomHeaderMiddleware::new()))
        .with_middleware(Arc::new(LoggingMiddleware::new(true, true, true, true)));

    let custom_messages = vec![ChatMessage::user("Test custom middleware modifications")];

    match custom_client.chat_completion(custom_messages).await {
        Ok(response) => {
            info!("Custom middleware example completed: {}", response);
        }
        Err(e) => {
            error!("Custom middleware example failed: {}", e);
        }
    }

    // Example 4: Conditional middleware execution
    info!("\n=== Example 4: Conditional Middleware ===");

    #[derive(Debug)]
    struct ConditionalMiddleware {
        condition: fn(&RequestContext) -> bool,
        action: String,
    }

    impl ConditionalMiddleware {
        fn new(condition: fn(&RequestContext) -> bool, action: String) -> Self {
            Self { condition, action }
        }
    }

    #[async_trait]
    impl Middleware for ConditionalMiddleware {
        async fn before_request(&self, context: &mut RequestContext) -> Result<()> {
            if (self.condition)(context) {
                info!("Conditional middleware executing: {}", self.action);
                context
                    .metadata
                    .insert("conditional_action".to_string(), self.action.clone());
            } else {
                debug!("Conditional middleware skipped");
            }
            Ok(())
        }

        fn name(&self) -> &str {
            "conditional"
        }

        fn priority(&self) -> i32 {
            40
        }
    }

    let conditional_client = MiddlewareClient::new("https://api.openai.com".to_string())
        .with_middleware(Arc::new(AuthenticationMiddleware::new(
            "test-api-key".to_string(),
            None,
        )))
        .with_middleware(Arc::new(ConditionalMiddleware::new(
            |ctx| ctx.body.contains("special"),
            "Special request processing enabled".to_string(),
        )))
        .with_middleware(Arc::new(LoggingMiddleware::new(true, true, false, false)));

    // Test with normal request
    let normal_messages = vec![ChatMessage::user("Regular request")];

    info!("Sending normal request (should skip conditional middleware)");
    match conditional_client.chat_completion(normal_messages).await {
        Ok(response) => {
            info!("Normal request completed: {}", response);
        }
        Err(e) => {
            error!("Normal request failed: {}", e);
        }
    }

    // Test with special request
    let special_messages = vec![ChatMessage::user(
        "This is a special request that triggers middleware",
    )];

    info!("Sending special request (should trigger conditional middleware)");
    match conditional_client.chat_completion(special_messages).await {
        Ok(response) => {
            info!("Special request completed: {}", response);
        }
        Err(e) => {
            error!("Special request failed: {}", e);
        }
    }

    // Example 5: Performance monitoring middleware
    info!("\n=== Example 5: Performance Monitoring ===");

    #[derive(Debug)]
    struct PerformanceMiddleware {
        slow_request_threshold: Duration,
    }

    impl PerformanceMiddleware {
        fn new(threshold: Duration) -> Self {
            Self {
                slow_request_threshold: threshold,
            }
        }
    }

    #[async_trait]
    impl Middleware for PerformanceMiddleware {
        async fn after_response(&self, context: &mut ResponseContext) -> Result<()> {
            if context.duration > self.slow_request_threshold {
                warn!(
                    "Slow request detected [{}]: {:?} (threshold: {:?})",
                    context.request.request_id, context.duration, self.slow_request_threshold
                );

                // Add performance warning to response metadata
                context.metadata.insert(
                    "performance_warning".to_string(),
                    "slow_request".to_string(),
                );
            } else {
                debug!(
                    "Request performance OK [{}]: {:?}",
                    context.request.request_id, context.duration
                );
            }

            Ok(())
        }

        fn name(&self) -> &str {
            "performance"
        }

        fn priority(&self) -> i32 {
            60
        }
    }

    let perf_metrics = Arc::new(MetricsMiddleware::new());
    let perf_metrics_ref = Arc::clone(&perf_metrics);

    let perf_client = MiddlewareClient::new("https://api.openai.com".to_string())
        .with_middleware(Arc::new(AuthenticationMiddleware::new(
            "test-api-key".to_string(),
            None,
        )))
        .with_middleware(Arc::new(PerformanceMiddleware::new(Duration::from_millis(
            200,
        ))))
        .with_middleware(perf_metrics)
        .with_middleware(Arc::new(LoggingMiddleware::new(true, true, false, false)));

    // Send multiple requests to demonstrate performance monitoring
    for i in 1..=5 {
        let perf_messages = vec![ChatMessage::user(&format!(
            "Performance test request {}",
            i
        ))];

        match perf_client.chat_completion(perf_messages).await {
            Ok(response) => {
                info!("Performance test {} completed: {}", i, response);
            }
            Err(e) => {
                error!("Performance test {} failed: {}", i, e);
            }
        }

        // Small delay between requests
        sleep(Duration::from_millis(100)).await;
    }

    // Print final performance metrics
    perf_metrics_ref.get_metrics().print_metrics();

    info!("Middleware patterns example completed successfully!");
    Ok(())
}
