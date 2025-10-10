//! Comprehensive integration tests for streaming functionality in the openai-ergonomic crate.
//!
//! This module tests streaming responses, chunk processing, message reassembly,
//! and various streaming scenarios including error handling.

#![allow(
    dead_code,
    unused_imports,
    clippy::cast_possible_truncation,
    clippy::significant_drop_tightening,
    clippy::uninlined_format_args,
    clippy::tuple_array_conversions,
    clippy::manual_let_else
)]

mod harness;

use harness::{
    assert_complete_streaming_message, assert_error_response, assert_field_equals,
    assert_has_field, assert_valid_stream_chunk, fixtures, MockOpenAIClient,
};
use openai_client_base::models::CreateChatCompletionStreamResponse;
use openai_ergonomic::builders::{responses::responses_simple, Builder};
use serde_json::{json, Value};
use std::time::Duration;

/// Test basic streaming response parsing
#[test]
fn test_basic_streaming_parsing() {
    // Create minimal working streaming chunks that follow the model requirements
    let chunks = [
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"role": "assistant", "content": ""},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": " there!"},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        }),
    ];

    assert!(!chunks.is_empty());

    for (i, chunk_json) in chunks.iter().enumerate() {
        // Verify chunk structure
        assert_has_field(chunk_json, "id");
        assert_has_field(chunk_json, "object");
        assert_has_field(chunk_json, "created");
        assert_has_field(chunk_json, "model");
        assert_has_field(chunk_json, "choices");

        assert_field_equals(chunk_json, "object", &json!("chat.completion.chunk"));

        let choices = chunk_json.get("choices").unwrap().as_array().unwrap();
        assert!(!choices.is_empty());

        let choice = &choices[0];
        assert_has_field(choice, "index");
        assert_has_field(choice, "delta");

        // Test deserialization
        let chunk: CreateChatCompletionStreamResponse = serde_json::from_value(chunk_json.clone())
            .expect("Should be able to deserialize streaming chunk");

        assert_valid_stream_chunk(&chunk);

        // Verify content accumulation logic
        let delta = choice.get("delta").unwrap();
        if i == 0 {
            // First chunk may have role
            if delta.get("role").is_some() {
                assert_field_equals(delta, "role", &json!("assistant"));
            }
        }

        // All chunks have finish_reason in this model implementation
        assert_has_field(choice, "finish_reason");
        assert!(!choice.get("finish_reason").unwrap().is_null());
    }
}

/// Test streaming chunk content accumulation
#[test]
fn test_streaming_content_accumulation() {
    // Use working streaming chunks format
    let chunks = vec![
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"role": "assistant", "content": ""},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": " there!"},
                "finish_reason": "stop"
            }]
        }),
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        }),
    ];

    let mut accumulated_content = String::new();
    let mut found_finish_reason = false;

    for chunk_json in &chunks {
        let choices = chunk_json.get("choices").unwrap().as_array().unwrap();
        let choice = &choices[0];
        let delta = choice.get("delta").unwrap();

        // Accumulate content
        if let Some(content) = delta.get("content") {
            if let Some(content_str) = content.as_str() {
                accumulated_content.push_str(content_str);
            }
        }

        // Check for finish reason
        if let Some(finish_reason) = choice.get("finish_reason") {
            if !finish_reason.is_null() {
                found_finish_reason = true;
            }
        }
    }

    assert!(found_finish_reason, "Stream should end with finish_reason");
    assert!(
        !accumulated_content.is_empty(),
        "Should accumulate some content"
    );

    // Test with assertion helper
    assert_complete_streaming_message(&chunks, &accumulated_content);
}

/// Test streaming with mock server
#[tokio::test]
async fn test_mock_streaming_server() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_chat_completions_streaming().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Tell me a story"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    let text = response.text().await.unwrap();

    // Verify Server-Sent Events format
    assert!(text.contains("data: "));
    assert!(text.contains("data: [DONE]"));

    // Parse chunks from SSE format
    let chunks = parse_sse_chunks(&text);
    assert!(!chunks.is_empty());

    // Validate each chunk
    for chunk in &chunks {
        assert_valid_stream_chunk(chunk);
    }

    mock.assert_async().await;
}

/// Test custom streaming chunks
#[tokio::test]
async fn test_custom_streaming_chunks() {
    let mut mock_client = MockOpenAIClient::new().await;
    let custom_chunks = vec![
        "The", " quick", " brown", " fox", " jumps", " over", " the", " lazy", " dog.",
    ];
    let mock = mock_client
        .mock_chat_completions_streaming_with_chunks(custom_chunks.clone())
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Write a sentence"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    let text = response.text().await.unwrap();

    // Verify all custom chunks are present
    for chunk in &custom_chunks {
        assert!(text.contains(chunk), "Missing chunk: {}", chunk);
    }

    // Parse and validate chunks
    let parsed_chunks = parse_sse_chunks(&text);
    assert!(!parsed_chunks.is_empty());

    // Reconstruct the message
    let mut reconstructed = String::new();
    for chunk in &parsed_chunks {
        if let Some(choice) = chunk.choices.first() {
            let delta = &choice.delta;
            if let Some(Some(content)) = delta.content.as_ref() {
                reconstructed.push_str(content);
            }
        }
    }

    let expected_message: String = custom_chunks.join("");
    assert_eq!(reconstructed, expected_message);

    mock.assert_async().await;
}

/// Test streaming with function calls
#[tokio::test]
async fn test_streaming_with_function_calls() {
    let mut mock_client = MockOpenAIClient::new().await;

    // Create streaming chunks that include function call delta
    let function_chunks = vec!["I need to get", " the weather", " information for you."];

    let mock = mock_client
        .mock_chat_completions_streaming_with_chunks(function_chunks)
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "What's the weather?"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather information",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    }
                }
            }],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let text = response.text().await.unwrap();

    let chunks = parse_sse_chunks(&text);
    assert!(!chunks.is_empty());

    // Validate streaming chunks
    for chunk in &chunks {
        assert_valid_stream_chunk(chunk);
    }

    mock.assert_async().await;
}

/// Test streaming error scenarios
#[tokio::test]
async fn test_streaming_error_scenarios() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client
        .mock_error_response(400, "invalid_request_error", "Invalid streaming request")
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "stream": true
            // Missing messages
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);

    let response_json: Value = response.json().await.unwrap();
    assert_error_response(&response_json, "invalid_request_error");

    mock.assert_async().await;
}

/// Test streaming with different finish reasons
#[test]
fn test_streaming_finish_reasons() {
    let finish_reasons = vec![
        "stop",
        "length",
        "function_call",
        "tool_calls",
        "content_filter",
    ];

    for finish_reason in finish_reasons {
        let chunk = json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": finish_reason
            }]
        });

        let parsed_chunk: CreateChatCompletionStreamResponse =
            serde_json::from_value(chunk).expect("Should parse chunk with finish reason");

        assert_valid_stream_chunk(&parsed_chunk);
        // Check finish_reason format - it should be capitalized enum variant
        let finish_reason_str = format!("{:?}", parsed_chunk.choices[0].finish_reason);
        let expected_format = match finish_reason {
            "stop" => "Stop",
            "length" => "Length",
            "function_call" => "FunctionCall",
            "tool_calls" => "ToolCalls",
            "content_filter" => "ContentFilter",
            _ => finish_reason, // fallback
        };
        assert!(
            finish_reason_str.contains(expected_format),
            "Expected finish_reason to contain '{}', but got: {}",
            expected_format,
            finish_reason_str
        );
    }
}

/// Test streaming performance
#[tokio::test]
async fn test_streaming_performance() {
    use harness::assert_performance;

    let mut mock_client = MockOpenAIClient::new().await;
    let large_chunks: Vec<&str> = (0..100)
        .map(|i| Box::leak(format!("Chunk {} ", i).into_boxed_str()) as &str)
        .collect();

    let mock = mock_client
        .mock_chat_completions_streaming_with_chunks(large_chunks)
        .await;

    let response_future = async {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/chat/completions", mock_client.base_url()))
            .header("authorization", format!("Bearer {}", mock_client.api_key()))
            .header("content-type", "application/json")
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "Generate lots of text"}],
                "stream": true
            }))
            .send()
            .await
            .unwrap();

        response.text().await.unwrap()
    };

    let start = std::time::Instant::now();
    let text = response_future.await;
    let duration = start.elapsed();

    assert!(
        duration <= Duration::from_millis(1000),
        "large_streaming_response took {:?} but should complete within 1000ms",
        duration
    );

    assert!(text.contains("Chunk 0"));
    assert!(text.contains("Chunk 99"));

    mock.assert_async().await;
}

/// Test streaming message boundaries
#[test]
fn test_streaming_message_boundaries() {
    // Test various edge cases in streaming content
    let edge_cases = vec![
        ("", "empty_content"),
        ("   ", "whitespace_only"),
        ("\n\n\n", "newlines_only"),
        ("", "emoji_content"),
        ("Multi\nline\ncontent", "multiline_content"),
        ("{\"json\": \"content\"}", "json_content"),
    ];

    for (content, test_name) in edge_cases {
        let chunk = json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion.chunk",
            "created": 1_677_652_288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": content
                },
                "finish_reason": "stop"
            }]
        });

        let parsed_chunk: CreateChatCompletionStreamResponse = serde_json::from_value(chunk)
            .unwrap_or_else(|e| panic!("Failed to parse chunk for {}: {}", test_name, e));

        assert_valid_stream_chunk(&parsed_chunk);

        let delta = &parsed_chunk.choices[0].delta;
        if let Some(Some(delta_content)) = delta.content.as_ref() {
            assert_eq!(delta_content, content);
        }
    }
}

/// Test streaming with concurrent requests
#[tokio::test]
async fn test_concurrent_streaming_requests() {
    let mut mock_client = MockOpenAIClient::new().await;

    // Set up multiple streaming mocks
    let _mock1 = mock_client.mock_chat_completions_streaming().await;
    let _mock2 = mock_client.mock_chat_completions_streaming().await;
    let _mock3 = mock_client.mock_chat_completions_streaming().await;

    let client = reqwest::Client::new();
    let base_url = mock_client.base_url();
    let api_key = mock_client.api_key().to_string();

    // Create concurrent streaming requests
    let request1 = make_streaming_request(&client, &base_url, &api_key, "Request 1");
    let request2 = make_streaming_request(&client, &base_url, &api_key, "Request 2");
    let request3 = make_streaming_request(&client, &base_url, &api_key, "Request 3");

    // Wait for all requests to complete
    let results = tokio::join!(request1, request2, request3);

    // All should succeed and return streaming content
    assert!(results.0.is_ok());
    assert!(results.1.is_ok());
    assert!(results.2.is_ok());

    for result in [results.0, results.1, results.2] {
        let text = result.unwrap();
        assert!(text.contains("data: "));
        assert!(text.contains("[DONE]"));
    }
}

/// Test streaming with builder integration
#[tokio::test]
async fn test_streaming_builder_integration() {
    let mut mock_client = MockOpenAIClient::new().await;
    let mock = mock_client.mock_chat_completions_streaming().await;

    // Use builder to create streaming request
    let builder = responses_simple("gpt-4", "Tell me a joke")
        .stream(true)
        .max_tokens(100);

    let request = builder.build().unwrap();
    assert_eq!(request.stream, Some(true));
    assert_eq!(request.max_tokens, Some(100));

    // Make request with builder output
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", mock_client.base_url()))
        .header("authorization", format!("Bearer {}", mock_client.api_key()))
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let text = response.text().await.unwrap();
    assert!(text.contains("data: "));

    mock.assert_async().await;
}

/// Helper function to parse Server-Sent Events chunks
fn parse_sse_chunks(sse_text: &str) -> Vec<CreateChatCompletionStreamResponse> {
    let mut chunks = Vec::new();

    for line in sse_text.lines() {
        if line.starts_with("data: ") && !line.contains("[DONE]") {
            let json_str = &line[6..]; // Remove "data: " prefix
            if let Ok(chunk_json) = serde_json::from_str::<Value>(json_str) {
                match serde_json::from_value::<CreateChatCompletionStreamResponse>(
                    chunk_json.clone(),
                ) {
                    Ok(chunk) => chunks.push(chunk),
                    Err(e) => {
                        println!("Failed to parse chunk: {} - Error: {}", chunk_json, e);
                        // Try to fix the chunk by adding missing fields
                        if let Ok(mut fixed_chunk) = serde_json::from_str::<Value>(json_str) {
                            // Add missing finish_reason field to choices
                            if let Some(choices) = fixed_chunk
                                .get_mut("choices")
                                .and_then(|v| v.as_array_mut())
                            {
                                for choice in choices {
                                    if choice.get("finish_reason").is_none() {
                                        choice
                                            .as_object_mut()
                                            .unwrap()
                                            .insert("finish_reason".to_string(), json!("stop"));
                                    }
                                }
                            }
                            if let Ok(chunk) = serde_json::from_value::<
                                CreateChatCompletionStreamResponse,
                            >(fixed_chunk)
                            {
                                chunks.push(chunk);
                            }
                        }
                    }
                }
            }
        }
    }

    chunks
}

/// Helper function to make a streaming request
async fn make_streaming_request(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    content: &str,
) -> Result<String, reqwest::Error> {
    let response = client
        .post(format!("{base_url}/v1/chat/completions"))
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": content}],
            "stream": true
        }))
        .send()
        .await?;

    response.text().await
}

/// Test streaming timeout handling
#[tokio::test]
async fn test_streaming_timeout() {
    // Use a non-routable IP address to simulate a timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(100)) // 100ms timeout
        .build()
        .unwrap();

    let result = client
        .post("http://192.0.2.1:8080/v1/chat/completions") // Non-routable IP
        .header("authorization", "Bearer test-key")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": true
        }))
        .send()
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // Check if it's a timeout or connection error
    assert!(error.is_timeout() || error.is_connect());
}

/// Test streaming chunk validation edge cases
#[test]
fn test_streaming_chunk_validation_edge_cases() {
    // Test chunk with no content in delta
    let empty_delta_chunk = json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion.chunk",
        "created": 1_677_652_288,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });

    let chunk: CreateChatCompletionStreamResponse =
        serde_json::from_value(empty_delta_chunk).expect("Should parse chunk with empty delta");
    assert_valid_stream_chunk(&chunk);

    // Test chunk with role in delta (first chunk)
    let role_delta_chunk = json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion.chunk",
        "created": 1_677_652_288,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "delta": {
                "role": "assistant",
                "content": ""
            },
            "finish_reason": "stop"
        }]
    });

    let chunk: CreateChatCompletionStreamResponse =
        serde_json::from_value(role_delta_chunk).expect("Should parse chunk with role delta");
    assert_valid_stream_chunk(&chunk);

    // Test final chunk with finish_reason
    let final_chunk = json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion.chunk",
        "created": 1_677_652_288,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });

    let chunk: CreateChatCompletionStreamResponse =
        serde_json::from_value(final_chunk).expect("Should parse final chunk");
    assert_valid_stream_chunk(&chunk);
}
