//! Batch processing example for high-volume OpenAI API usage.
//!
//! This example demonstrates advanced batch processing patterns including:
//! - Batch API endpoint usage for cost-effective processing
//! - File upload and management for batch inputs
//! - Asynchronous batch monitoring and result retrieval
//! - Error handling and retry strategies for batch operations
//! - Performance optimization techniques for high-volume processing
//! - Cost tracking and optimization strategies
//!
//! The Batch API allows processing up to 50,000 requests per batch with:
//! - 50% cost reduction compared to synchronous API calls
//! - 24-hour processing window
//! - Built-in retry logic for failed requests
//!
//! Run with: `cargo run --example batch_processing`

use openai_ergonomic::{Client, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Represents a single batch request in JSONL format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchRequest {
    /// Unique identifier for this request
    custom_id: String,
    /// HTTP method (always "POST" for OpenAI batch API)
    method: String,
    /// API endpoint path
    url: String,
    /// Request body containing the API parameters
    body: BatchRequestBody,
}

/// Request body for a batch API call
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchRequestBody {
    /// Model to use for the request
    model: String,
    /// Messages for chat completion
    messages: Vec<ChatMessage>,
    /// Maximum tokens to generate
    max_tokens: Option<i32>,
    /// Temperature for response randomness (0.0 to 2.0)
    temperature: Option<f64>,
}

/// Chat message for batch requests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    /// Role of the message sender
    role: String,
    /// Content of the message
    content: String,
}

/// Represents a batch job status and metadata
#[derive(Debug, Clone)]
struct BatchJob {
    /// Batch job identifier
    id: String,
    /// Current status of the batch
    status: String,
    /// Input file ID
    input_file_id: String,
    /// Output file ID (when completed)
    output_file_id: Option<String>,
    /// Error file ID (if errors occurred)
    error_file_id: Option<String>,
    /// When the batch was created
    created_at: i64,
    /// When the batch completed (if applicable)
    completed_at: Option<i64>,
    /// Number of requests in the batch
    request_counts: BatchRequestCounts,
}

/// Request counts for batch processing
#[derive(Debug, Clone)]
struct BatchRequestCounts {
    /// Total requests submitted
    total: i32,
    /// Requests completed successfully
    completed: i32,
    /// Requests that failed
    failed: i32,
}

/// Batch processing manager for handling large-scale API operations
#[derive(Debug)]
struct BatchProcessor {
    client: Client,
    /// Directory for storing batch files
    batch_dir: String,
    /// Maximum requests per batch (OpenAI limit: 50,000)
    max_batch_size: usize,
    /// Polling interval for batch status checks
    poll_interval: Duration,
    /// Maximum time to wait for batch completion
    max_wait_time: Duration,
}

impl BatchProcessor {
    /// Create a new batch processor with default settings
    fn new(client: Client) -> Self {
        Self {
            client,
            batch_dir: "./batch_files".to_string(),
            max_batch_size: 50_000,
            poll_interval: Duration::from_secs(30),
            max_wait_time: Duration::from_hours(25), // Slightly more than 24h limit
        }
    }

    /// Configure batch processing parameters
    fn with_config(
        mut self,
        batch_dir: &str,
        max_batch_size: usize,
        poll_interval: Duration,
        max_wait_time: Duration,
    ) -> Self {
        self.batch_dir = batch_dir.to_string();
        self.max_batch_size = max_batch_size;
        self.poll_interval = poll_interval;
        self.max_wait_time = max_wait_time;
        self
    }

    /// Process a large collection of requests using the batch API
    async fn process_batch_requests(
        &self,
        requests: Vec<BatchRequest>,
        batch_name: &str,
    ) -> Result<Vec<BatchProcessingResult>> {
        info!("Starting batch processing for {} requests", requests.len());

        // Ensure batch directory exists
        fs::create_dir_all(&self.batch_dir)
            .map_err(|e| Error::InvalidRequest(format!("Failed to create batch directory: {}", e)))?;

        // Split requests into batches if needed
        let batches = self.split_into_batches(requests);
        let mut all_results = Vec::new();

        for (batch_idx, batch_requests) in batches.into_iter().enumerate() {
            let batch_id = format!("{}_batch_{}", batch_name, batch_idx);
            info!(
                "Processing batch {}/{}: {} requests",
                batch_idx + 1,
                batch_requests.len(),
                batch_requests.len()
            );

            let results = self.process_single_batch(batch_requests, &batch_id).await?;
            all_results.extend(results);
        }

        info!(
            "Completed batch processing with {} total results",
            all_results.len()
        );
        Ok(all_results)
    }

    /// Process a single batch (up to max_batch_size requests)
    async fn process_single_batch(
        &self,
        requests: Vec<BatchRequest>,
        batch_id: &str,
    ) -> Result<Vec<BatchProcessingResult>> {
        // Step 1: Create JSONL file with batch requests
        let input_file_path = format!("{}/{}_input.jsonl", self.batch_dir, batch_id);
        self.create_batch_file(&requests, &input_file_path)?;

        // Step 2: Upload the file to OpenAI
        let file_upload_result = self.upload_batch_file(&input_file_path).await?;
        info!("Uploaded batch file with ID: {}", file_upload_result.id);

        // Step 3: Create the batch job
        let batch_job = self
            .create_batch_job(&file_upload_result.id, batch_id)
            .await?;
        info!("Created batch job with ID: {}", batch_job.id);

        // Step 4: Monitor batch progress
        let completed_batch = self.monitor_batch_progress(batch_job).await?;

        // Step 5: Download and process results
        let results = self.download_batch_results(&completed_batch).await?;

        info!(
            "Successfully processed batch with {} results",
            results.len()
        );
        Ok(results)
    }

    /// Split requests into batches respecting the max batch size limit
    fn split_into_batches(&self, requests: Vec<BatchRequest>) -> Vec<Vec<BatchRequest>> {
        requests
            .chunks(self.max_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Create a JSONL file containing batch requests
    fn create_batch_file(&self, requests: &[BatchRequest], file_path: &str) -> Result<()> {
        let mut content = String::new();
        for request in requests {
            let json_line = serde_json::to_string(request)
                .map_err(|e| Error::InvalidRequest(format!("Failed to serialize request: {}", e)))?;
            content.push_str(&json_line);
            content.push('\n');
        }

        fs::write(file_path, content)
            .map_err(|e| Error::InvalidRequest(format!("Failed to write batch file: {}", e)))?;

        debug!(
            "Created batch file: {} ({} requests)",
            file_path,
            requests.len()
        );
        Ok(())
    }

    /// Upload a batch file to OpenAI and return the file ID
    async fn upload_batch_file(&self, file_path: &str) -> Result<FileUploadResult> {
        info!("Uploading batch file: {}", file_path);

        // Note: This is a placeholder implementation
        // In a real implementation, you would use the files API:
        // let response = self.client.files().upload(file_path, "batch").await?;

        // For demonstration, we'll simulate a successful upload
        let file_id = format!("file-{}", uuid::Uuid::new_v4());

        Ok(FileUploadResult {
            id: file_id,
            bytes: fs::metadata(file_path)
                .map_err(|e| Error::InvalidRequest(format!("Failed to get file size: {}", e)))?
                .len(),
            filename: Path::new(file_path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        })
    }

    /// Create a batch job using the uploaded file
    async fn create_batch_job(&self, input_file_id: &str, batch_name: &str) -> Result<BatchJob> {
        info!("Creating batch job for file: {}", input_file_id);

        // Note: This is a placeholder implementation
        // In a real implementation, you would use the batch API:
        // let response = self.client.batches().create(input_file_id, endpoint, completion_window).await?;

        // For demonstration, we'll simulate a successful batch creation
        let batch_id = format!("batch_{}", uuid::Uuid::new_v4());

        Ok(BatchJob {
            id: batch_id,
            status: "validating".to_string(),
            input_file_id: input_file_id.to_string(),
            output_file_id: None,
            error_file_id: None,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            request_counts: BatchRequestCounts {
                total: 0,
                completed: 0,
                failed: 0,
            },
        })
    }

    /// Monitor batch progress until completion or timeout
    async fn monitor_batch_progress(&self, mut batch_job: BatchJob) -> Result<BatchJob> {
        let start_time = Instant::now();

        loop {
            // Check timeout
            if start_time.elapsed() > self.max_wait_time {
                return Err(Error::InvalidRequest(format!(
                    "Batch processing timed out after {:?}",
                    self.max_wait_time
                )));
            }

            // Check batch status
            batch_job = self.get_batch_status(&batch_job.id).await?;

            match batch_job.status.as_str() {
                "completed" => {
                    info!("Batch {} completed successfully", batch_job.id);
                    return Ok(batch_job);
                }
                "failed" | "expired" | "cancelled" => {
                    return Err(Error::InvalidRequest(format!(
                        "Batch {} failed with status: {}",
                        batch_job.id, batch_job.status
                    )));
                }
                "validating" | "in_progress" | "finalizing" => {
                    info!(
                        "Batch {} status: {} ({}s elapsed)",
                        batch_job.id,
                        batch_job.status,
                        start_time.elapsed().as_secs()
                    );
                }
                _ => {
                    warn!("Unknown batch status: {}", batch_job.status);
                }
            }

            // Wait before next poll
            sleep(self.poll_interval).await;
        }
    }

    /// Get current status of a batch job
    async fn get_batch_status(&self, batch_id: &str) -> Result<BatchJob> {
        debug!("Checking status for batch: {}", batch_id);

        // Note: This is a placeholder implementation
        // In a real implementation, you would use the batch API:
        // let response = self.client.batches().retrieve(batch_id).await?;

        // For demonstration, we'll simulate batch progression
        // In a real scenario, this would make an actual API call
        let current_time = chrono::Utc::now().timestamp();

        Ok(BatchJob {
            id: batch_id.to_string(),
            status: "completed".to_string(), // Simulate completion
            input_file_id: format!("file-input-{}", batch_id),
            output_file_id: Some(format!("file-output-{}", batch_id)),
            error_file_id: None,
            created_at: current_time - 3600, // 1 hour ago
            completed_at: Some(current_time),
            request_counts: BatchRequestCounts {
                total: 100,
                completed: 98,
                failed: 2,
            },
        })
    }

    /// Download and parse batch results
    async fn download_batch_results(
        &self,
        batch_job: &BatchJob,
    ) -> Result<Vec<BatchProcessingResult>> {
        let output_file_id = batch_job
            .output_file_id
            .as_ref()
            .ok_or_else(|| Error::InvalidRequest("No output file available".to_string()))?;

        info!("Downloading results from file: {}", output_file_id);

        // Note: This is a placeholder implementation
        // In a real implementation, you would download the file:
        // let content = self.client.files().download(output_file_id).await?;

        // For demonstration, we'll create sample results
        let mut results = Vec::new();
        for i in 0..batch_job.request_counts.completed {
            results.push(BatchProcessingResult {
                custom_id: format!("request_{}", i),
                status: "completed".to_string(),
                response: Some(BatchResponseData {
                    id: format!("chatcmpl_{}", uuid::Uuid::new_v4()),
                    object: "chat.completion".to_string(),
                    model: "gpt-3.5-turbo".to_string(),
                    choices: vec![BatchChoiceData {
                        index: 0,
                        message: BatchMessageData {
                            role: "assistant".to_string(),
                            content: format!("This is a sample response for request {}", i),
                        },
                        finish_reason: "stop".to_string(),
                    }],
                    usage: BatchUsageData {
                        prompt_tokens: 20,
                        completion_tokens: 15,
                        total_tokens: 35,
                    },
                }),
                error: None,
            });
        }

        // Add some failed requests for demonstration
        for i in 0..batch_job.request_counts.failed {
            results.push(BatchProcessingResult {
                custom_id: format!("failed_request_{}", i),
                status: "failed".to_string(),
                response: None,
                error: Some(BatchErrorData {
                    code: "rate_limit_exceeded".to_string(),
                    message: "Rate limit exceeded, please try again later".to_string(),
                }),
            });
        }

        info!("Downloaded {} batch results", results.len());
        Ok(results)
    }

    /// Calculate cost savings compared to synchronous API calls
    fn calculate_cost_savings(&self, results: &[BatchProcessingResult]) -> CostAnalysis {
        let successful_requests = results.iter().filter(|r| r.response.is_some()).count();
        let total_tokens: i32 = results
            .iter()
            .filter_map(|r| r.response.as_ref())
            .map(|resp| resp.usage.total_tokens)
            .sum();

        // Batch API provides 50% cost reduction
        let synchronous_cost = total_tokens as f64 * 0.002; // Example: $0.002 per 1K tokens
        let batch_cost = synchronous_cost * 0.5; // 50% discount
        let savings = synchronous_cost - batch_cost;

        CostAnalysis {
            successful_requests,
            total_tokens,
            synchronous_cost,
            batch_cost,
            savings,
            savings_percentage: (savings / synchronous_cost) * 100.0,
        }
    }
}

/// Result of a file upload operation
#[derive(Debug)]
struct FileUploadResult {
    id: String,
    bytes: u64,
    filename: String,
}

/// Result of a single batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchProcessingResult {
    custom_id: String,
    status: String,
    response: Option<BatchResponseData>,
    error: Option<BatchErrorData>,
}

/// Response data from a successful batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchResponseData {
    id: String,
    object: String,
    model: String,
    choices: Vec<BatchChoiceData>,
    usage: BatchUsageData,
}

/// Choice data from batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchChoiceData {
    index: i32,
    message: BatchMessageData,
    finish_reason: String,
}

/// Message data from batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchMessageData {
    role: String,
    content: String,
}

/// Usage statistics from batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchUsageData {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// Error data from failed batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchErrorData {
    code: String,
    message: String,
}

/// Cost analysis for batch processing
#[derive(Debug)]
struct CostAnalysis {
    successful_requests: usize,
    total_tokens: i32,
    synchronous_cost: f64,
    batch_cost: f64,
    savings: f64,
    savings_percentage: f64,
}

/// Utility for generating batch requests from data
struct BatchRequestGenerator;

impl BatchRequestGenerator {
    /// Generate batch requests for content summarization
    fn generate_summarization_requests(contents: Vec<String>) -> Vec<BatchRequest> {
        contents
            .into_iter()
            .enumerate()
            .map(|(idx, content)| BatchRequest {
                custom_id: format!("summarize_{}", idx),
                method: "POST".to_string(),
                url: "/v1/chat/completions".to_string(),
                body: BatchRequestBody {
                    model: "gpt-3.5-turbo".to_string(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: "You are a helpful assistant that creates concise summaries."
                                .to_string(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Please summarize the following text in 2-3 sentences:\n\n{}",
                                content
                            ),
                        },
                    ],
                    max_tokens: Some(150),
                    temperature: Some(0.3),
                },
            })
            .collect()
    }

    /// Generate batch requests for sentiment analysis
    fn generate_sentiment_requests(texts: Vec<String>) -> Vec<BatchRequest> {
        texts
            .into_iter()
            .enumerate()
            .map(|(idx, text)| BatchRequest {
                custom_id: format!("sentiment_{}", idx),
                method: "POST".to_string(),
                url: "/v1/chat/completions".to_string(),
                body: BatchRequestBody {
                    model: "gpt-3.5-turbo".to_string(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: "Analyze the sentiment of the given text. Respond with only: POSITIVE, NEGATIVE, or NEUTRAL.".to_string(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: text,
                        },
                    ],
                    max_tokens: Some(10),
                    temperature: Some(0.0),
                },
            })
            .collect()
    }

    /// Generate batch requests for translation
    fn generate_translation_requests(
        texts: Vec<String>,
        target_language: &str,
    ) -> Vec<BatchRequest> {
        texts
            .into_iter()
            .enumerate()
            .map(|(idx, text)| BatchRequest {
                custom_id: format!("translate_{}_{}", target_language, idx),
                method: "POST".to_string(),
                url: "/v1/chat/completions".to_string(),
                body: BatchRequestBody {
                    model: "gpt-3.5-turbo".to_string(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: format!(
                                "Translate the following text to {}. Provide only the translation.",
                                target_language
                            ),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: text,
                        },
                    ],
                    max_tokens: Some(200),
                    temperature: Some(0.3),
                },
            })
            .collect()
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

    info!("Starting batch processing example");

    // Create client from environment
    let client = Client::from_env()?;

    // Initialize batch processor with custom configuration
    let batch_processor = BatchProcessor::new(client).with_config(
        "./batch_results",          // Custom batch directory
        1000,                       // Smaller batch size for demo
        Duration::from_secs(10),    // More frequent polling for demo
        Duration::from_secs(30 * 60), // Shorter timeout for demo
    );

    // Example 1: Content summarization batch
    info!("=== Example 1: Content Summarization Batch ===");

    let content_samples = vec![
        "Artificial intelligence (AI) is intelligence demonstrated by machines, in contrast to the natural intelligence displayed by humans and animals. Leading AI textbooks define the field as the study of \"intelligent agents\": any device that perceives its environment and takes actions that maximize its chance of successfully achieving its goals.".to_string(),
        "Machine learning is a method of data analysis that automates analytical model building. It is a branch of artificial intelligence based on the idea that systems can learn from data, identify patterns and make decisions with minimal human intervention.".to_string(),
        "Deep learning is part of a broader family of machine learning methods based on artificial neural networks with representation learning. Learning can be supervised, semi-supervised or unsupervised.".to_string(),
    ];

    let summarization_requests =
        BatchRequestGenerator::generate_summarization_requests(content_samples);

    match batch_processor
        .process_batch_requests(summarization_requests, "content_summarization")
        .await
    {
        Ok(results) => {
            info!(
                "Summarization batch completed with {} results",
                results.len()
            );

            // Process results
            for result in &results {
                match &result.response {
                    Some(response) => {
                        if let Some(choice) = response.choices.first() {
                            info!(
                                "Summary for {}: {}",
                                result.custom_id, choice.message.content
                            );
                        }
                    }
                    None => {
                        if let Some(error) = &result.error {
                            error!(
                                "Failed {}: {} - {}",
                                result.custom_id, error.code, error.message
                            );
                        }
                    }
                }
            }

            // Calculate cost savings
            let cost_analysis = batch_processor.calculate_cost_savings(&results);
            info!("Cost Analysis:");
            info!(
                "  Successful requests: {}",
                cost_analysis.successful_requests
            );
            info!("  Total tokens: {}", cost_analysis.total_tokens);
            info!("  Synchronous cost: ${:.4}", cost_analysis.synchronous_cost);
            info!("  Batch cost: ${:.4}", cost_analysis.batch_cost);
            info!(
                "  Savings: ${:.4} ({:.1}%)",
                cost_analysis.savings, cost_analysis.savings_percentage
            );
        }
        Err(e) => {
            error!("Summarization batch failed: {}", e);
        }
    }

    // Example 2: Sentiment analysis batch
    info!("\n=== Example 2: Sentiment Analysis Batch ===");

    let sentiment_texts = vec![
        "I absolutely love this product! It exceeded all my expectations.".to_string(),
        "The service was terrible and I'm very disappointed.".to_string(),
        "It's an okay product, nothing special but gets the job done.".to_string(),
        "Outstanding quality and amazing customer support!".to_string(),
        "Not worth the money, poor build quality.".to_string(),
    ];

    let sentiment_requests = BatchRequestGenerator::generate_sentiment_requests(sentiment_texts);

    match batch_processor
        .process_batch_requests(sentiment_requests, "sentiment_analysis")
        .await
    {
        Ok(results) => {
            info!(
                "Sentiment analysis batch completed with {} results",
                results.len()
            );

            let mut sentiment_counts = HashMap::new();
            for result in &results {
                if let Some(response) = &result.response {
                    if let Some(choice) = response.choices.first() {
                        let sentiment = choice.message.content.trim();
                        *sentiment_counts.entry(sentiment.to_string()).or_insert(0) += 1;
                        info!("Sentiment for {}: {}", result.custom_id, sentiment);
                    }
                }
            }

            info!("Sentiment Distribution:");
            for (sentiment, count) in sentiment_counts {
                info!("  {}: {} occurrences", sentiment, count);
            }
        }
        Err(e) => {
            error!("Sentiment analysis batch failed: {}", e);
        }
    }

    // Example 3: Translation batch
    info!("\n=== Example 3: Translation Batch ===");

    let english_texts = vec![
        "Hello, how are you today?".to_string(),
        "Thank you for your help.".to_string(),
        "The weather is beautiful today.".to_string(),
    ];

    let translation_requests =
        BatchRequestGenerator::generate_translation_requests(english_texts, "Spanish");

    match batch_processor
        .process_batch_requests(translation_requests, "translation")
        .await
    {
        Ok(results) => {
            info!("Translation batch completed with {} results", results.len());

            for result in &results {
                if let Some(response) = &result.response {
                    if let Some(choice) = response.choices.first() {
                        info!(
                            "Translation for {}: {}",
                            result.custom_id, choice.message.content
                        );
                    }
                }
            }
        }
        Err(e) => {
            error!("Translation batch failed: {}", e);
        }
    }

    // Example 4: Monitoring multiple concurrent batches
    info!("\n=== Example 4: Concurrent Batch Processing ===");

    let small_batch_1 = BatchRequestGenerator::generate_sentiment_requests(vec![
        "Great product!".to_string(),
        "Could be better.".to_string(),
    ]);

    let small_batch_2 = BatchRequestGenerator::generate_summarization_requests(vec![
        "Short text to summarize.".to_string(),
    ]);

    // Process batches concurrently
    let batch_1_future =
        batch_processor.process_batch_requests(small_batch_1, "concurrent_batch_1");
    let batch_2_future =
        batch_processor.process_batch_requests(small_batch_2, "concurrent_batch_2");

    let (result_1, result_2) = tokio::try_join!(batch_1_future, batch_2_future)?;

    info!(
        "Concurrent batch 1 completed with {} results",
        result_1.len()
    );
    info!(
        "Concurrent batch 2 completed with {} results",
        result_2.len()
    );

    info!("Batch processing example completed successfully!");
    Ok(())
}

/// Additional utility module for batch processing optimization
mod uuid {
    /// Simple UUID v4 implementation for demonstration
    pub struct Uuid;

    impl Uuid {
        pub fn new_v4() -> String {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("uuid-{:x}", timestamp)
        }
    }
}

/// Chrono replacement for timestamp handling
mod chrono {
    pub struct Utc;

    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }

    pub struct DateTime;

    impl DateTime {
        pub fn timestamp(&self) -> i64 {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        }
    }
}
