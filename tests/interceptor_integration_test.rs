//! Integration tests for the interceptor system.

#[cfg(test)]
mod tests {
    use openai_ergonomic::{
        AfterResponseContext, BeforeRequestContext, Client, Config, ErrorContext, Interceptor,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Test interceptor that counts invocations.
    #[allow(clippy::struct_field_names)]
    struct CountingInterceptor {
        before_count: Arc<AtomicUsize>,
        after_count: Arc<AtomicUsize>,
        error_count: Arc<AtomicUsize>,
    }

    impl CountingInterceptor {
        fn new() -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            let before = Arc::new(AtomicUsize::new(0));
            let after = Arc::new(AtomicUsize::new(0));
            let error = Arc::new(AtomicUsize::new(0));

            let interceptor = Self {
                before_count: before.clone(),
                after_count: after.clone(),
                error_count: error.clone(),
            };

            (interceptor, before, after, error)
        }
    }

    #[async_trait::async_trait]
    impl Interceptor for CountingInterceptor {
        async fn before_request(
            &self,
            _ctx: &mut BeforeRequestContext<'_>,
        ) -> openai_ergonomic::Result<()> {
            self.before_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_response(
            &self,
            _ctx: &AfterResponseContext<'_>,
        ) -> openai_ergonomic::Result<()> {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn on_error(&self, _ctx: &ErrorContext<'_>) {
            self.error_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_interceptor_called_on_chat_request() {
        // This test requires a valid API key to run
        let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
            eprintln!("Skipping test: OPENAI_API_KEY not set");
            return;
        };

        let config = Config::builder().api_key(api_key).build();
        let (interceptor, before_count, after_count, _error_count) = CountingInterceptor::new();

        let client = Client::builder(config)
            .unwrap()
            .with_interceptor(Box::new(interceptor))
            .build();

        // Make a simple request
        let result = client
            .send_chat(client.chat_simple("Say 'test' and nothing else"))
            .await;

        // Even if the request fails (e.g., rate limit), before_request should be called
        assert_eq!(
            before_count.load(Ordering::SeqCst),
            1,
            "before_request should be called once"
        );

        // after_response should be called only on success
        if result.is_ok() {
            assert_eq!(
                after_count.load(Ordering::SeqCst),
                1,
                "after_response should be called once on success"
            );
        }
    }

    #[tokio::test]
    async fn test_interceptor_error_handling() {
        // Use an invalid API key to trigger an error
        let config = Config::builder().api_key("invalid-key").build();

        let (interceptor, before_count, _after_count, error_count) = CountingInterceptor::new();

        let client = Client::builder(config)
            .unwrap()
            .with_interceptor(Box::new(interceptor))
            .build();

        // This should fail with an authentication error
        let result = client.send_chat(client.chat_simple("test")).await;

        assert!(result.is_err(), "Request should fail with invalid API key");
        assert_eq!(
            before_count.load(Ordering::SeqCst),
            1,
            "before_request should be called"
        );
        assert_eq!(
            error_count.load(Ordering::SeqCst),
            1,
            "on_error should be called"
        );
    }

    #[tokio::test]
    async fn test_multiple_interceptors() {
        let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
            eprintln!("Skipping test: OPENAI_API_KEY not set");
            return;
        };

        let config = Config::builder().api_key(api_key).build();

        let (interceptor1, before1, _, _) = CountingInterceptor::new();
        let (interceptor2, before2, _, _) = CountingInterceptor::new();

        let client = Client::builder(config)
            .unwrap()
            .add_interceptor(Box::new(interceptor1))
            .add_interceptor(Box::new(interceptor2))
            .build();

        // Make a request
        let _ = client.send_chat(client.chat_simple("Say 'test'")).await;

        // Both interceptors should be called
        assert_eq!(
            before1.load(Ordering::SeqCst),
            1,
            "First interceptor should be called"
        );
        assert_eq!(
            before2.load(Ordering::SeqCst),
            1,
            "Second interceptor should be called"
        );
    }
}
