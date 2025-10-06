//! Tests for the interceptor system.

#![allow(clippy::significant_drop_tightening)]

use openai_ergonomic::{
    AfterResponseContext, BeforeRequestContext, Client, ErrorContext, Interceptor,
};
use std::sync::{Arc, Mutex};

/// Test interceptor that tracks calls.
#[allow(clippy::struct_field_names)]
#[derive(Clone)]
struct TestInterceptor {
    before_calls: Arc<Mutex<Vec<String>>>,
    after_calls: Arc<Mutex<Vec<String>>>,
    error_calls: Arc<Mutex<Vec<String>>>,
}

impl TestInterceptor {
    fn new() -> Self {
        Self {
            before_calls: Arc::new(Mutex::new(Vec::new())),
            after_calls: Arc::new(Mutex::new(Vec::new())),
            error_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl Interceptor for TestInterceptor {
    async fn before_request(
        &self,
        ctx: &mut BeforeRequestContext<'_>,
    ) -> openai_ergonomic::Result<()> {
        self.before_calls
            .lock()
            .unwrap()
            .push(ctx.operation.to_string());
        Ok(())
    }

    async fn after_response(&self, ctx: &AfterResponseContext<'_>) -> openai_ergonomic::Result<()> {
        self.after_calls
            .lock()
            .unwrap()
            .push(ctx.operation.to_string());
        Ok(())
    }

    async fn on_error(&self, ctx: &ErrorContext<'_>) {
        self.error_calls
            .lock()
            .unwrap()
            .push(ctx.operation.to_string());
    }
}

#[tokio::test]
async fn test_interceptor_called() {
    // Skip if no API key (this would be a real integration test)
    if std::env::var("OPENAI_API_KEY").is_err() {
        return;
    }

    let interceptor = TestInterceptor::new();
    let interceptor_clone = interceptor.clone();

    let client = Client::from_env()
        .unwrap()
        .with_interceptor(Box::new(interceptor));

    // Make a request
    let result = client.send_chat(client.chat_simple("test")).await;

    // Check interceptors were called
    {
        let before_calls = interceptor_clone.before_calls.lock().unwrap();
        assert!(!before_calls.is_empty(), "before_request should be called");
        assert_eq!(before_calls[0], "chat");
    }

    if result.is_ok() {
        let after_calls = interceptor_clone.after_calls.lock().unwrap();
        assert!(
            !after_calls.is_empty(),
            "after_response should be called on success"
        );
        assert_eq!(after_calls[0], "chat");
    } else {
        let error_calls = interceptor_clone.error_calls.lock().unwrap();
        assert!(
            !error_calls.is_empty(),
            "on_error should be called on failure"
        );
    }
}

#[tokio::test]
async fn test_multiple_interceptors() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        return;
    }

    let interceptor1 = TestInterceptor::new();
    let interceptor2 = TestInterceptor::new();

    let interceptor1_clone = interceptor1.clone();
    let interceptor2_clone = interceptor2.clone();

    let client = Client::from_env()
        .unwrap()
        .with_interceptor(Box::new(interceptor1))
        .with_interceptor(Box::new(interceptor2));

    // Make a request
    let _ = client.send_chat(client.chat_simple("test")).await;

    // Both interceptors should be called
    {
        let before_calls1 = interceptor1_clone.before_calls.lock().unwrap();
        assert!(!before_calls1.is_empty(), "interceptor1 should be called");
    }

    {
        let before_calls2 = interceptor2_clone.before_calls.lock().unwrap();
        assert!(!before_calls2.is_empty(), "interceptor2 should be called");
    }
}

#[test]
fn test_interceptor_contexts() {
    // Test that context types can be constructed
    let metadata = std::collections::HashMap::new();

    let _before_ctx = BeforeRequestContext {
        operation: "test",
        model: "gpt-4",
        request_json: "{}",
        metadata: std::collections::HashMap::new(),
    };

    let _after_ctx = AfterResponseContext {
        operation: "test",
        model: "gpt-4",
        request_json: "{}",
        response_json: "{}",
        duration: std::time::Duration::from_secs(1),
        input_tokens: Some(10),
        output_tokens: Some(20),
        metadata: &metadata,
    };

    let _error_ctx = ErrorContext {
        operation: "test",
        model: Some("gpt-4"),
        request_json: Some("{}"),
        error: "test error",
        duration: std::time::Duration::from_secs(1),
        metadata: Some(&metadata),
    };
}
