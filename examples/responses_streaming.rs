#![allow(clippy::uninlined_format_args)]
//! Streaming responses example for the openai-ergonomic crate.
//!
//! This example demonstrates streaming patterns for the Responses API, including:
//! - SSE (Server-Sent Events) streaming responses
//! - Chunk processing patterns
//! - Error handling for streaming
//! - Buffer management best practices
//! - Real-time response processing

use futures::StreamExt;
use openai_ergonomic::{Client, Error, Result};
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;

/// Represents a chunk of streaming data from `OpenAI`
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// The raw event data
    pub data: String,
    /// Parsed content delta, if available
    pub content_delta: Option<String>,
    /// Whether this is the final chunk
    pub is_done: bool,
    /// Any tool call data in this chunk
    pub tool_call_delta: Option<Value>,
}

impl StreamChunk {
    /// Parse a raw SSE data line into a `StreamChunk`
    pub fn parse(line: &str) -> Result<Option<Self>> {
        // Handle SSE format: "data: {json}"
        if !line.starts_with("data: ") {
            return Ok(None);
        }

        let data = line.strip_prefix("data: ").unwrap_or("");

        // Check for [DONE] marker
        if data.trim() == "[DONE]" {
            return Ok(Some(Self {
                data: data.to_string(),
                content_delta: None,
                is_done: true,
                tool_call_delta: None,
            }));
        }

        // Parse JSON data
        let json: Value = serde_json::from_str(data).map_err(|e| Error::StreamParsing {
            message: format!("Failed to parse chunk JSON: {e}"),
            chunk: data.to_string(),
        })?;

        // Extract content delta
        let content_delta = json["choices"][0]["delta"]["content"]
            .as_str()
            .map(ToString::to_string);

        // Extract tool call delta
        let tool_call_delta = json["choices"][0]["delta"]["tool_calls"]
            .as_array()
            .and_then(|arr| arr.first())
            .cloned();

        Ok(Some(Self {
            data: data.to_string(),
            content_delta,
            is_done: false,
            tool_call_delta,
        }))
    }

    /// Get the content from this chunk, if any
    pub fn content(&self) -> Option<&str> {
        self.content_delta.as_deref()
    }

    /// Check if this chunk has tool call data
    pub const fn has_tool_call(&self) -> bool {
        self.tool_call_delta.is_some()
    }
}

/// Stream manager for handling SSE responses
pub struct ResponseStream {
    lines_stream: LinesStream<BufReader<Box<dyn tokio::io::AsyncRead + Send + Unpin>>>,
    finished: bool,
}

impl ResponseStream {
    /// Create a new response stream from a reader
    pub fn new(reader: Box<dyn tokio::io::AsyncRead + Send + Unpin>) -> Self {
        let buf_reader = BufReader::new(reader);
        let lines_stream = LinesStream::new(buf_reader.lines());

        Self {
            lines_stream,
            finished: false,
        }
    }

    /// Get the next chunk from the stream
    pub async fn next_chunk(&mut self) -> Result<Option<StreamChunk>> {
        if self.finished {
            return Ok(None);
        }

        while let Some(line_result) = self.lines_stream.next().await {
            let line = line_result.map_err(|e| Error::StreamConnection {
                message: format!("Stream read error: {e}"),
            })?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the chunk
            if let Some(chunk) = StreamChunk::parse(&line)? {
                if chunk.is_done {
                    self.finished = true;
                }
                return Ok(Some(chunk));
            }
        }

        // Stream ended without [DONE] marker
        self.finished = true;
        Ok(None)
    }

    /// Collect all content from the remaining stream
    pub async fn collect_remaining(&mut self) -> Result<String> {
        let mut content = String::new();

        while let Some(chunk) = self.next_chunk().await? {
            if let Some(delta) = chunk.content() {
                content.push_str(delta);
            }
        }

        Ok(content)
    }

    /// Check if the stream has finished
    pub const fn is_finished(&self) -> bool {
        self.finished
    }
}

/// Buffer manager for efficient streaming content handling
pub struct StreamBuffer {
    content: String,
    capacity: usize,
    high_water_mark: usize,
}

impl StreamBuffer {
    /// Create a new buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            content: String::with_capacity(capacity),
            capacity,
            high_water_mark: capacity * 3 / 4, // 75% of capacity
        }
    }

    /// Add content to the buffer
    pub fn append(&mut self, content: &str) -> Result<()> {
        // Check if adding this content would exceed capacity
        if self.content.len() + content.len() > self.capacity {
            return Err(Error::StreamBuffer {
                message: format!(
                    "Buffer capacity exceeded: {} + {} > {}",
                    self.content.len(),
                    content.len(),
                    self.capacity
                ),
            });
        }

        self.content.push_str(content);
        Ok(())
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.content.clear();
    }

    /// Check if buffer is near capacity
    pub fn is_high_water(&self) -> bool {
        self.content.len() >= self.high_water_mark
    }

    /// Get buffer utilization as a percentage
    pub fn utilization(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        {
            (self.content.len() as f64 / self.capacity as f64) * 100.0
        }
    }

    /// Compact the buffer by removing processed content
    pub fn compact(&mut self, keep_last_chars: usize) {
        if self.content.len() > keep_last_chars {
            let start_pos = self.content.len() - keep_last_chars;
            self.content = self.content[start_pos..].to_string();
        }
    }
}

/// Demonstrates basic streaming response handling
async fn example_basic_streaming() -> Result<()> {
    println!("=== Basic Streaming Example ===");

    // Note: This is a conceptual example since actual streaming
    // requires integration with openai-client-base streaming API
    println!("Creating client and streaming request...");

    let client = Client::from_env()?.build();

    // Build a streaming request
    let _streaming_request = client
        .responses()
        .user("Tell me a short story about a robot learning to paint")
        .stream(true)
        .temperature(0.7)
        .max_completion_tokens(500);

    println!("Streaming request configured:");
    println!("- Model: Default (gpt-4)");
    println!("- Stream: true");
    println!("- Temperature: 0.7");
    println!("- Max tokens: 500");

    // Simulate streaming chunks for demonstration
    let sample_chunks = vec![
        "Once", " upon", " a", " time,", " there", " was", " a", " little", " robot", " named",
        " Pixel", "...",
    ];

    println!("\nSimulated streaming output:");
    print!("> ");
    for chunk in sample_chunks {
        print!("{chunk}");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("\n");

    Ok(())
}

/// Demonstrates advanced streaming with buffer management
async fn example_buffered_streaming() -> Result<()> {
    println!("=== Buffered Streaming Example ===");

    let mut buffer = StreamBuffer::new(1024); // 1KB buffer

    // Simulate incoming chunks
    let chunks = [
        "The robot's optical sensors",
        " detected the vibrant colors",
        " of the sunset painting",
        " hanging in the gallery.",
        " For the first time,",
        " Pixel felt something",
        " that could only be",
        " described as wonder.",
    ];

    println!("Processing chunks with buffer management:");

    for (i, chunk) in chunks.iter().enumerate() {
        // Add chunk to buffer
        buffer.append(chunk)?;

        println!(
            "Chunk {}: '{}' (Buffer: {:.1}% full)",
            i + 1,
            chunk,
            buffer.utilization()
        );

        // Check if buffer is getting full
        if buffer.is_high_water() {
            println!("    Buffer high water mark reached, consider processing");

            // In a real application, you might:
            // 1. Process the current content
            // 2. Send to downstream consumers
            // 3. Compact the buffer
            buffer.compact(100); // Keep last 100 chars for context
            println!("   Buffer compacted to {:.1}%", buffer.utilization());
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!(
        "\nFinal content length: {} characters",
        buffer.content().len()
    );
    println!(
        "Final content: \"{}...\"",
        &buffer.content()[..buffer.content().len().min(50)]
    );

    Ok(())
}

/// Demonstrates error handling patterns for streaming
fn example_streaming_error_handling() {
    println!("=== Streaming Error Handling Example ===");

    // Simulate various error conditions that can occur during streaming
    println!("Demonstrating common streaming error scenarios:");

    // 1. Connection errors
    println!("\n1. Connection Error Simulation:");
    let connection_result: Result<()> = Err(Error::StreamConnection {
        message: "Connection lost to streaming endpoint".to_string(),
    });

    match connection_result {
        Err(Error::StreamConnection { message }) => {
            println!("    Connection error handled: {message}");
            println!("    Would implement retry logic here");
        }
        _ => unreachable!(),
    }

    // 2. Parsing errors
    println!("\n2. Parse Error Simulation:");
    let malformed_chunk = "data: {invalid json}";
    match StreamChunk::parse(malformed_chunk) {
        Err(Error::StreamParsing { message, chunk }) => {
            println!("    Parse error handled: {message}");
            println!("    Problematic chunk: {chunk}");
            println!("    Would skip chunk and continue");
        }
        _ => println!("    Chunk parsed successfully"),
    }

    // 3. Buffer overflow
    println!("\n3. Buffer Overflow Simulation:");
    let mut small_buffer = StreamBuffer::new(10); // Very small buffer
    let large_chunk = "This chunk is definitely too large for our tiny buffer";

    match small_buffer.append(large_chunk) {
        Err(Error::StreamBuffer { message }) => {
            println!("    Buffer error handled: {message}");
            println!("    Would implement buffer resizing or chunking");
        }
        Ok(()) => println!("    Content added to buffer"),
        Err(e) => println!("    Unexpected error: {e}"),
    }

    // 4. Timeout handling
    println!("\n4. Timeout Handling:");
    println!("   ⏱  Would implement timeout for stream chunks");
    println!("    Would retry or fail gracefully on timeout");
}

/// Demonstrates tool calling in streaming responses
async fn example_streaming_tool_calls() -> Result<()> {
    println!("=== Streaming Tool Calls Example ===");

    let client = Client::from_env()?.build();

    // Create a tool for getting weather information
    let weather_tool = openai_ergonomic::responses::tool_function(
        "get_weather",
        "Get current weather for a location",
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    );

    // Build streaming request with tools
    let _tool_request = client
        .responses()
        .user("What's the weather like in San Francisco?")
        .tool(weather_tool)
        .stream(true);

    println!("Streaming tool call request configured:");
    println!("- Tool: get_weather function");
    println!("- Streaming: enabled");

    // Simulate streaming tool call chunks
    println!("\nSimulated streaming tool call:");

    let tool_chunks = [
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","type":"function","function":{"name":"get_weather"}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{"}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"location\""}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":":"}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"San Francisco\""}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]}}]}"#,
    ];

    let mut tool_call_buffer = String::new();

    for (i, chunk_data) in tool_chunks.iter().enumerate() {
        let chunk_line = format!("data: {chunk_data}");

        if let Some(chunk) = StreamChunk::parse(&chunk_line)? {
            if chunk.has_tool_call() {
                println!("Chunk {}: Tool call data received", i + 1);

                // In a real implementation, you'd accumulate tool call arguments
                if let Some(tool_data) = &chunk.tool_call_delta {
                    if let Some(args) = tool_data["function"]["arguments"].as_str() {
                        tool_call_buffer.push_str(args);
                        println!("  Arguments so far: {tool_call_buffer}");
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("\n Complete tool call arguments: {tool_call_buffer}");
    println!(" Would now execute get_weather(location='San Francisco')");

    Ok(())
}

/// Demonstrates chunk processing patterns and metrics
#[allow(clippy::cast_precision_loss)]
async fn example_chunk_processing_patterns() -> Result<()> {
    println!("=== Chunk Processing Patterns ===");

    #[allow(clippy::items_after_statements)]
    #[derive(Debug, Default)]
    struct StreamMetrics {
        total_chunks: usize,
        content_chunks: usize,
        tool_chunks: usize,
        total_bytes: usize,
        processing_time: Duration,
    }

    let mut metrics = StreamMetrics::default();
    let start_time = std::time::Instant::now();

    // Simulate various types of chunks
    let sample_chunks = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
        "data: {\"choices\":[{\"delta\":{\"content\":\" world!\"}}]}",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"function\":{\"name\":\"test\"}}]}}]}",
        "data: {\"choices\":[{\"delta\":{\"content\":\" How are you?\"}}]}",
        "data: [DONE]",
    ];

    println!("Processing {} chunks with metrics:", sample_chunks.len());

    for (i, chunk_line) in sample_chunks.iter().enumerate() {
        if let Some(chunk) = StreamChunk::parse(chunk_line)? {
            metrics.total_chunks += 1;
            metrics.total_bytes += chunk.data.len();

            if chunk.content().is_some() {
                metrics.content_chunks += 1;
                println!(
                    "Chunk {}: Content chunk - '{}'",
                    i + 1,
                    chunk.content().unwrap_or("")
                );
            } else if chunk.has_tool_call() {
                metrics.tool_chunks += 1;
                println!("Chunk {}: Tool call chunk", i + 1);
            } else if chunk.is_done {
                println!("Chunk {}: Stream completion marker", i + 1);
            }

            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    metrics.processing_time = start_time.elapsed();

    println!("\n Stream Processing Metrics:");
    println!("   Total chunks: {}", metrics.total_chunks);
    println!("   Content chunks: {}", metrics.content_chunks);
    println!("   Tool call chunks: {}", metrics.tool_chunks);
    println!("   Total bytes: {}", metrics.total_bytes);
    println!("   Processing time: {:?}", metrics.processing_time);
    println!(
        "   Avg bytes/chunk: {:.1}",
        metrics.total_bytes as f64 / metrics.total_chunks as f64
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for better debugging
    tracing_subscriber::fmt::init();

    println!(" OpenAI Ergonomic - Streaming Responses Examples");
    println!("================================================\n");

    // Note: These examples demonstrate streaming patterns and error handling
    // The actual streaming implementation will be completed when the
    // openai-client-base streaming API is fully integrated

    // Run all examples
    if let Err(e) = example_basic_streaming().await {
        eprintln!("Basic streaming example failed: {e}");
    }

    println!();

    if let Err(e) = example_buffered_streaming().await {
        eprintln!("Buffered streaming example failed: {e}");
    }

    println!();

    example_streaming_error_handling();

    println!();

    if let Err(e) = example_streaming_tool_calls().await {
        eprintln!("Tool calls example failed: {e}");
    }

    println!();

    if let Err(e) = example_chunk_processing_patterns().await {
        eprintln!("Chunk processing example failed: {e}");
    }

    println!("\n All streaming examples completed!");
    println!("\n Key Takeaways:");
    println!("   • SSE streaming requires careful chunk parsing");
    println!("   • Buffer management prevents memory issues");
    println!("   • Error handling is crucial for robust streaming");
    println!("   • Tool calls can be streamed incrementally");
    println!("   • Metrics help optimize streaming performance");

    println!("\n Next Steps:");
    println!("   • Integrate with openai-client-base streaming API");
    println!("   • Add real streaming request execution");
    println!("   • Implement automatic retry logic");
    println!("   • Add streaming response caching");

    Ok(())
}
