//! Integration tests for the deprecated interceptor system (stub).
//!
//! These tests verify that the backward compatibility stubs work correctly.
//! For new code, use the middleware system instead.

#[cfg(test)]
mod tests {
    #[allow(deprecated)]
    use openai_ergonomic::interceptor::{
        AfterResponseContext, BeforeRequestContext, ErrorContext, Interceptor,
    };
    use openai_ergonomic::{Client, Config};
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
    #[allow(deprecated)]
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
    #[allow(deprecated)]
    async fn test_interceptor_stubs_work() {
        // Test that the backward compatibility stubs don't break existing code
        let config = Config::builder()
            .api_key("sk-test123")
            .build();

        let (interceptor, _before, _after, _error) = CountingInterceptor::new();

        let _client = Client::new(config)
            .unwrap()
            .with_interceptor(Box::new(interceptor));

        // Note: Interceptors are now no-ops, so counts will remain 0
        // This test just verifies the API still works for backward compatibility
    }
}
