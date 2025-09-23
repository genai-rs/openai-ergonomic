//! Test data fixtures for consistent testing across the codebase.
//!
//! This module provides predefined test data including requests, responses,
//! tools, schemas, and other common test scenarios.

use serde_json::{json, Value};
use std::collections::HashMap;

/// Chat completion request fixtures.
pub mod chat_requests {
    use super::*;

    /// Basic chat completion request.
    pub fn basic() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        })
    }

    /// Chat request with system message.
    pub fn with_system() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is the capital of France?"}
            ],
            "temperature": 0.7,
            "max_tokens": 150
        })
    }

    /// Chat request with conversation history.
    pub fn with_history() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"},
                {"role": "assistant", "content": "2+2 equals 4."},
                {"role": "user", "content": "What about 3+3?"}
            ],
            "temperature": 0.7
        })
    }

    /// Chat request with function calling.
    pub fn with_functions() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "What's the weather in Paris?"}
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get current weather information",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "location": {
                                    "type": "string",
                                    "description": "The location to get weather for"
                                },
                                "units": {
                                    "type": "string",
                                    "enum": ["celsius", "fahrenheit"],
                                    "description": "Temperature units"
                                }
                            },
                            "required": ["location"]
                        }
                    }
                }
            ]
        })
    }

    /// Chat request with vision (multimodal).
    pub fn with_vision() -> Value {
        json!({
            "model": "gpt-4-vision-preview",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "What do you see in this image?"
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": "https://example.com/image.jpg",
                                "detail": "high"
                            }
                        }
                    ]
                }
            ],
            "max_tokens": 300
        })
    }

    /// Chat request with JSON mode.
    pub fn with_json_mode() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Generate a JSON object representing a person"}
            ],
            "response_format": {
                "type": "json_object"
            }
        })
    }

    /// Chat request with JSON schema.
    pub fn with_json_schema() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Generate a person object"}
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "person",
                    "strict": true,
                    "schema": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "age": {"type": "number"},
                            "email": {"type": "string", "format": "email"}
                        },
                        "required": ["name", "age"],
                        "additionalProperties": false
                    }
                }
            }
        })
    }

    /// Chat request with streaming enabled.
    pub fn with_streaming() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Tell me a short story"}
            ],
            "stream": true,
            "max_tokens": 200
        })
    }

    /// Chat request with o3 reasoning effort.
    pub fn with_reasoning_effort() -> Value {
        json!({
            "model": "o3-mini",
            "messages": [
                {"role": "user", "content": "Solve this complex math problem step by step"}
            ],
            "reasoning_effort": "high"
        })
    }

    /// Chat request with all parameters.
    pub fn comprehensive() -> Value {
        json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Help me with my task"}
            ],
            "temperature": 0.7,
            "max_tokens": 500,
            "max_completion_tokens": 400,
            "top_p": 0.9,
            "frequency_penalty": 0.1,
            "presence_penalty": 0.1,
            "n": 1,
            "stop": ["STOP", "END"],
            "seed": 12345,
            "user": "test-user"
        })
    }
}

/// Chat completion response fixtures.
pub mod chat_responses {
    use super::*;

    /// Basic successful response.
    pub fn basic_success() -> Value {
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        })
    }

    /// Response with function call.
    pub fn with_function_call() -> Value {
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Paris\", \"units\": \"celsius\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 10,
                "total_tokens": 25
            }
        })
    }

    /// Streaming response chunks.
    pub fn streaming_chunks() -> Vec<Value> {
        vec![
            json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion.chunk",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "delta": {"role": "assistant", "content": ""},
                    "finish_reason": null
                }]
            }),
            json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion.chunk",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "delta": {"content": "Hello"},
                    "finish_reason": null
                }]
            }),
            json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion.chunk",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "delta": {"content": " there!"},
                    "finish_reason": null
                }]
            }),
            json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion.chunk",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            })
        ]
    }

    /// JSON mode response.
    pub fn json_mode() -> Value {
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "{\"name\": \"John Doe\", \"age\": 30, \"city\": \"New York\"}"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 18,
                "total_tokens": 30
            }
        })
    }
}

/// Error response fixtures.
pub mod error_responses {
    use super::*;

    /// Rate limit error.
    pub fn rate_limit() -> Value {
        json!({
            "error": {
                "type": "rate_limit_exceeded",
                "message": "Rate limit exceeded. Please try again later.",
                "code": "rate_limit_exceeded"
            }
        })
    }

    /// Authentication error.
    pub fn authentication() -> Value {
        json!({
            "error": {
                "type": "invalid_api_key",
                "message": "Invalid API key provided",
                "code": "invalid_api_key"
            }
        })
    }

    /// Invalid request error.
    pub fn invalid_request() -> Value {
        json!({
            "error": {
                "type": "invalid_request_error",
                "message": "Missing required parameter: 'messages'",
                "code": "missing_required_parameter",
                "param": "messages"
            }
        })
    }

    /// Server error.
    pub fn server_error() -> Value {
        json!({
            "error": {
                "type": "server_error",
                "message": "The server encountered an error while processing your request",
                "code": "internal_server_error"
            }
        })
    }

    /// Context length exceeded.
    pub fn context_length_exceeded() -> Value {
        json!({
            "error": {
                "type": "invalid_request_error",
                "message": "This model's maximum context length is 8192 tokens",
                "code": "context_length_exceeded"
            }
        })
    }

    /// Model not found.
    pub fn model_not_found() -> Value {
        json!({
            "error": {
                "type": "invalid_request_error",
                "message": "The model 'invalid-model' does not exist",
                "code": "model_not_found",
                "param": "model"
            }
        })
    }
}

/// Tool and function fixtures.
pub mod tools {
    use super::*;

    /// Simple function definition.
    pub fn simple_function() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather information",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        }
                    },
                    "required": ["location"]
                }
            }
        })
    }

    /// Complex function with multiple parameters.
    pub fn complex_function() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_database",
                "description": "Search through a database with multiple filters",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "filters": {
                            "type": "object",
                            "properties": {
                                "category": {
                                    "type": "string",
                                    "enum": ["books", "movies", "music", "games"]
                                },
                                "min_rating": {
                                    "type": "number",
                                    "minimum": 0,
                                    "maximum": 10
                                },
                                "max_price": {
                                    "type": "number",
                                    "minimum": 0
                                }
                            }
                        },
                        "sort_by": {
                            "type": "string",
                            "enum": ["relevance", "price", "rating", "date"],
                            "default": "relevance"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }
            }
        })
    }

    /// Function that doesn't require parameters.
    pub fn no_params_function() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_current_time",
                "description": "Get the current time",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        })
    }

    /// Mathematical calculation function.
    pub fn math_function() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "calculate",
                "description": "Perform mathematical calculations",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Mathematical expression to evaluate",
                            "pattern": "^[0-9+\\-*/().\\s]+$"
                        },
                        "precision": {
                            "type": "integer",
                            "description": "Number of decimal places",
                            "minimum": 0,
                            "maximum": 10,
                            "default": 2
                        }
                    },
                    "required": ["expression"]
                }
            }
        })
    }
}

/// JSON schema fixtures.
pub mod schemas {
    use super::*;

    /// Person schema.
    pub fn person() -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Full name of the person"
                },
                "age": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 150,
                    "description": "Age in years"
                },
                "email": {
                    "type": "string",
                    "format": "email",
                    "description": "Email address"
                },
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"},
                        "country": {"type": "string"},
                        "postal_code": {"type": "string"}
                    },
                    "required": ["city", "country"]
                },
                "active": {
                    "type": "boolean",
                    "description": "Whether the person is active",
                    "default": true
                }
            },
            "required": ["name", "email"],
            "additionalProperties": false
        })
    }

    /// Product schema.
    pub fn product() -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "pattern": "^[A-Z0-9]{8}$",
                    "description": "Product ID"
                },
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 100,
                    "description": "Product name"
                },
                "price": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Price in USD"
                },
                "category": {
                    "type": "string",
                    "enum": ["electronics", "clothing", "books", "home", "sports"],
                    "description": "Product category"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "uniqueItems": true,
                    "description": "Product tags"
                },
                "in_stock": {
                    "type": "boolean",
                    "description": "Whether the product is in stock"
                }
            },
            "required": ["id", "name", "price", "category"],
            "additionalProperties": false
        })
    }

    /// Event schema.
    pub fn event() -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Event title"
                },
                "description": {
                    "type": "string",
                    "description": "Event description"
                },
                "start_time": {
                    "type": "string",
                    "format": "date-time",
                    "description": "Event start time"
                },
                "end_time": {
                    "type": "string",
                    "format": "date-time",
                    "description": "Event end time"
                },
                "location": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "address": {"type": "string"},
                        "coordinates": {
                            "type": "object",
                            "properties": {
                                "latitude": {"type": "number", "minimum": -90, "maximum": 90},
                                "longitude": {"type": "number", "minimum": -180, "maximum": 180}
                            },
                            "required": ["latitude", "longitude"]
                        }
                    },
                    "required": ["name"]
                },
                "attendees": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "email": {"type": "string", "format": "email"}
                        },
                        "required": ["name", "email"]
                    }
                }
            },
            "required": ["title", "start_time"],
            "additionalProperties": false
        })
    }
}

/// Other endpoint fixtures.
pub mod other_endpoints {
    use super::*;

    /// Embeddings request.
    pub fn embeddings_request() -> Value {
        json!({
            "model": "text-embedding-ada-002",
            "input": "Hello, world!",
            "encoding_format": "float"
        })
    }

    /// Embeddings response.
    pub fn embeddings_response() -> Value {
        json!({
            "object": "list",
            "data": [{
                "object": "embedding",
                "index": 0,
                "embedding": [0.1, 0.2, 0.3, -0.1, -0.2]
            }],
            "model": "text-embedding-ada-002",
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        })
    }

    /// Models list response.
    pub fn models_list() -> Value {
        json!({
            "object": "list",
            "data": [
                {
                    "id": "gpt-4",
                    "object": "model",
                    "created": 1677610602,
                    "owned_by": "openai"
                },
                {
                    "id": "gpt-3.5-turbo",
                    "object": "model",
                    "created": 1677610602,
                    "owned_by": "openai"
                }
            ]
        })
    }

    /// Model details response.
    pub fn model_details() -> Value {
        json!({
            "id": "gpt-4",
            "object": "model",
            "created": 1677610602,
            "owned_by": "openai"
        })
    }
}

/// Test scenario fixtures combining multiple elements.
pub mod scenarios {
    use super::*;

    /// Complete conversation scenario.
    pub fn complete_conversation() -> HashMap<String, Value> {
        let mut scenario = HashMap::new();

        scenario.insert("initial_request".to_string(), json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful travel assistant."},
                {"role": "user", "content": "I want to plan a trip to Paris. Can you help?"}
            ]
        }));

        scenario.insert("assistant_response".to_string(), json!({
            "role": "assistant",
            "content": "I'd be happy to help you plan your trip to Paris! To give you the best recommendations, could you tell me what time of year you're planning to visit and what you're most interested in seeing?"
        }));

        scenario.insert("follow_up_request".to_string(), json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful travel assistant."},
                {"role": "user", "content": "I want to plan a trip to Paris. Can you help?"},
                {"role": "assistant", "content": "I'd be happy to help you plan your trip to Paris! To give you the best recommendations, could you tell me what time of year you're planning to visit and what you're most interested in seeing?"},
                {"role": "user", "content": "I'm going in spring and love art museums"}
            ]
        }));

        scenario
    }

    /// Function calling scenario.
    pub fn function_calling_flow() -> HashMap<String, Value> {
        let mut scenario = HashMap::new();

        scenario.insert("request_with_tools".to_string(), json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "What's the weather like in Paris today?"}
            ],
            "tools": [tools::simple_function()]
        }));

        scenario.insert("function_call_response".to_string(), chat_responses::with_function_call());

        scenario.insert("function_result_request".to_string(), json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "What's the weather like in Paris today?"},
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Paris\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_test123",
                    "content": "{\"temperature\": \"22°C\", \"condition\": \"sunny\", \"humidity\": \"45%\"}"
                }
            ],
            "tools": [tools::simple_function()]
        }));

        scenario
    }

    /// Error handling scenario.
    pub fn error_scenarios() -> HashMap<String, Value> {
        let mut scenarios = HashMap::new();

        scenarios.insert("rate_limit".to_string(), error_responses::rate_limit());
        scenarios.insert("auth_error".to_string(), error_responses::authentication());
        scenarios.insert("invalid_request".to_string(), error_responses::invalid_request());
        scenarios.insert("server_error".to_string(), error_responses::server_error());

        scenarios
    }
}

/// Test data variations for edge cases.
pub mod edge_cases {
    use super::*;

    /// Empty and minimal content.
    pub fn minimal_data() -> HashMap<String, Value> {
        let mut data = HashMap::new();

        data.insert("empty_string".to_string(), json!(""));
        data.insert("single_char".to_string(), json!("a"));
        data.insert("whitespace_only".to_string(), json!("   "));
        data.insert("newlines_only".to_string(), json!("\n\n\n"));

        data
    }

    /// Large content for boundary testing.
    pub fn large_content() -> HashMap<String, Value> {
        let mut data = HashMap::new();

        let large_text = "Lorem ipsum ".repeat(1000);
        data.insert("large_text".to_string(), json!(large_text));

        let large_array: Vec<i32> = (0..1000).collect();
        data.insert("large_array".to_string(), json!(large_array));

        data
    }

    /// Special characters and unicode.
    pub fn special_content() -> HashMap<String, Value> {
        let mut data = HashMap::new();

        data.insert("unicode_emoji".to_string(), json!("Hello 🌍🚀✨"));
        data.insert("unicode_text".to_string(), json!("Здравствуй мир! 你好世界! مرحبا بالعالم!"));
        data.insert("special_chars".to_string(), json!("!@#$%^&*()_+-=[]{}|;':\",./<>?"));
        data.insert("escaped_json".to_string(), json!("{\"key\": \"value with \\\"quotes\\\" and \\n newlines\"}"));

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_fixtures() {
        let basic = chat_requests::basic();
        assert!(basic.get("model").is_some());
        assert!(basic.get("messages").is_some());

        let with_system = chat_requests::with_system();
        assert_eq!(with_system["messages"].as_array().unwrap().len(), 2);

        let comprehensive = chat_requests::comprehensive();
        assert!(comprehensive.get("temperature").is_some());
        assert!(comprehensive.get("max_tokens").is_some());
    }

    #[test]
    fn test_response_fixtures() {
        let success = chat_responses::basic_success();
        assert!(success.get("id").is_some());
        assert!(success.get("choices").is_some());

        let chunks = chat_responses::streaming_chunks();
        assert!(!chunks.is_empty());
        assert!(chunks.len() >= 4);
    }

    #[test]
    fn test_error_fixtures() {
        let rate_limit = error_responses::rate_limit();
        assert!(rate_limit.get("error").is_some());

        let auth_error = error_responses::authentication();
        let error = auth_error.get("error").unwrap();
        assert!(error.get("type").is_some());
        assert!(error.get("message").is_some());
    }

    #[test]
    fn test_tool_fixtures() {
        let simple = tools::simple_function();
        assert_eq!(simple["type"], "function");
        assert!(simple.get("function").is_some());

        let complex = tools::complex_function();
        let function = complex.get("function").unwrap();
        assert!(function.get("parameters").is_some());
    }

    #[test]
    fn test_schema_fixtures() {
        let person = schemas::person();
        assert_eq!(person["type"], "object");
        assert!(person.get("properties").is_some());
        assert!(person.get("required").is_some());

        let product = schemas::product();
        assert!(product["properties"].get("id").is_some());
        assert!(product["properties"].get("price").is_some());
    }

    #[test]
    fn test_scenario_fixtures() {
        let conversation = scenarios::complete_conversation();
        assert!(conversation.contains_key("initial_request"));
        assert!(conversation.contains_key("assistant_response"));

        let function_flow = scenarios::function_calling_flow();
        assert!(function_flow.contains_key("request_with_tools"));
        assert!(function_flow.contains_key("function_call_response"));
    }

    #[test]
    fn test_edge_case_fixtures() {
        let minimal = edge_cases::minimal_data();
        assert!(minimal.contains_key("empty_string"));
        assert!(minimal.contains_key("single_char"));

        let special = edge_cases::special_content();
        assert!(special.contains_key("unicode_emoji"));
        assert!(special.contains_key("special_chars"));
    }
}