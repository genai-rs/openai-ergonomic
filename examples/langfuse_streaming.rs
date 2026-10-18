#![allow(clippy::uninlined_format_args)]
//! Streaming chat completions with Langfuse observability.
//!
//! This example demonstrates how to use streaming responses with the Langfuse interceptor
//! for real-time observability and tracing.
//!
//! ## Setup
//!
//! Before running this example, set the following environment variables:
//! - `OPENAI_API_KEY`: Your `OpenAI` API key
//! - `LANGFUSE_PUBLIC_KEY`: Your Langfuse public key (starts with "pk-lf-")
//! - `LANGFUSE_SECRET_KEY`: Your Langfuse secret key (starts with "sk-lf-")
//! - `LANGFUSE_HOST` (optional): Langfuse API host (defaults to <https://cloud.langfuse.com>)
//!
//! ## Running the example
//!
//! ```bash
//! cargo run --example langfuse_streaming
//! ```

use futures::StreamExt;
use openai_ergonomic::streaming::BoxedChatStream;
use openai_ergonomic::{
    langfuse_exporter_from_env, ChatCompletionChunk, Client, Error, LangfuseConfig,
    LangfuseInterceptor, LangfuseState, StreamCompletion,
};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider, Span},
};
use std::{error::Error as StdError, fmt, time::Duration};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("openai_ergonomic=debug".parse()?),
        )
        .init();

    println!("🚀 Initializing OpenAI client with Langfuse streaming observability...\n");

    // 1. Build Langfuse exporter from environment variables
    let exporter = langfuse_exporter_from_env()?;

    // 2. Create tracer provider with batch processor
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
        .build();

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    // 3. Get tracer and create interceptor
    let tracer = provider.tracer("openai-ergonomic");
    let langfuse_interceptor = LangfuseInterceptor::new(tracer, LangfuseConfig::new());

    // 4. Create the OpenAI client and add the Langfuse interceptor
    let client = Client::from_env()?
        .with_interceptor(Box::new(langfuse_interceptor))
        .build();

    println!("✅ Client initialized successfully!");
    println!("📊 Streaming traces will be sent to Langfuse for monitoring\n");

    let streaming_result = async {
        println!("=== Example 1: Basic Streaming ===");
        basic_streaming(&client).await?;

        println!("\n=== Example 2: Streaming with Parameters ===");
        streaming_with_parameters(&client).await?;

        println!("\n=== Example 3: Collect Full Content ===");
        collect_content(&client).await?;

        Ok(())
    }
    .await;

    // A stream error is returned only after its callback report is sealed, so its telemetry can
    // still be exported. An incomplete drain deliberately skips shutdown and is never success.
    match shutdown_after_streaming(streaming_result, || provider.shutdown()) {
        Ok(()) => {
            println!("\n✅ Done! Check your Langfuse dashboard to see the streaming traces.");
            println!("   - Look for traces with operation names 'chat' or 'responses'");
            println!(
                "   - Each trace includes before_request, on_stream_chunk, and on_stream_terminal"
            );
            Ok(())
        }
        Err(ExampleError::Stream(error)) => Err(Box::new(error) as Box<dyn StdError>),
        Err(error) => Err(Box::new(error) as Box<dyn StdError>),
    }
}

async fn basic_streaming(client: &Client<LangfuseState<Span>>) -> ExampleResult<()> {
    println!("Question: Tell me a short joke");

    let builder = client.chat().user("Tell me a short joke");

    let (stream, completion) = client.send_chat_stream_with_completion(builder).await?;

    print!("Response: ");
    let mut chunk_count = 0;
    consume_stream(stream, completion, |chunk| {
        if let Some(content) = chunk.content() {
            print!("{}", content);
            chunk_count += 1;
        }
    })
    .await?;
    println!(
        "\n(Received {} chunks, all traced to Langfuse)",
        chunk_count
    );
    Ok(())
}

async fn streaming_with_parameters(client: &Client<LangfuseState<Span>>) -> ExampleResult<()> {
    println!("Question: Write a creative tagline for a bakery");

    let builder = client
        .chat()
        .user("Write a creative tagline for a bakery")
        .temperature(0.9)
        .max_tokens(50);

    let (stream, completion) = client.send_chat_stream_with_completion(builder).await?;

    print!("Response: ");
    let mut chunk_count = 0;
    consume_stream(stream, completion, |chunk| {
        if let Some(content) = chunk.content() {
            print!("{}", content);
            chunk_count += 1;
        }
    })
    .await?;
    println!(
        "\n(Received {} chunks, all traced to Langfuse)",
        chunk_count
    );
    Ok(())
}

async fn collect_content(client: &Client<LangfuseState<Span>>) -> ExampleResult<()> {
    println!("Question: What is the capital of France?");

    let builder = client.chat().user("What is the capital of France?");

    let (stream, completion) = client.send_chat_stream_with_completion(builder).await?;

    // Manually collect content (interceptor hooks are still called for each chunk)
    let mut content = String::new();
    consume_stream(stream, completion, |chunk| {
        if let Some(text) = chunk.content() {
            content.push_str(text);
        }
    })
    .await?;
    println!("Full response: {}", content);
    println!("(All chunks were traced to Langfuse during collection)");
    Ok(())
}

#[derive(Debug)]
enum ExampleError {
    Stream(Error),
    DrainIncomplete(String),
    ExporterShutdown(String),
}

impl fmt::Display for ExampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stream(error) => write!(formatter, "stream failed: {error}"),
            Self::DrainIncomplete(message) => {
                write!(formatter, "callback drain incomplete: {message}")
            }
            Self::ExporterShutdown(message) => {
                write!(formatter, "exporter shutdown failed: {message}")
            }
        }
    }
}

impl StdError for ExampleError {}

impl From<Error> for ExampleError {
    fn from(error: Error) -> Self {
        Self::Stream(error)
    }
}

type ExampleResult<T> = std::result::Result<T, ExampleError>;

fn shutdown_after_streaming<E: fmt::Display>(
    result: ExampleResult<()>,
    shutdown: impl FnOnce() -> std::result::Result<(), E>,
) -> ExampleResult<()> {
    match result {
        Ok(()) => shutdown().map_err(|error| ExampleError::ExporterShutdown(error.to_string())),
        Err(error @ ExampleError::Stream(_)) => {
            if let Err(shutdown_error) = shutdown() {
                eprintln!("Exporter shutdown also failed: {shutdown_error}");
            }
            Err(error)
        }
        Err(error) => Err(error),
    }
}

async fn consume_stream(
    mut stream: BoxedChatStream,
    completion: StreamCompletion,
    mut on_chunk: impl FnMut(ChatCompletionChunk),
) -> ExampleResult<()> {
    let mut stream_error = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => on_chunk(chunk),
            Err(error) => {
                stream_error = Some(error);
                break;
            }
        }
    }
    drop(stream);

    let report = match tokio::time::timeout(Duration::from_secs(10), completion.wait()).await {
        Ok(Ok(report)) => report,
        Ok(Err(error)) => {
            return Err(ExampleError::DrainIncomplete(format!(
                "completion report unavailable: {error}"
            )))
        }
        Err(_) => {
            return Err(ExampleError::DrainIncomplete(
                "timed out; retain the completion handle and retry before exporter shutdown".into(),
            ))
        }
    };

    println!(
        "Stream callbacks drained: {:?}, {} callbacks, {} failures",
        report.outcome, report.callback_attempts, report.callback_failure_count
    );
    for failure in &report.callback_failures {
        eprintln!(
            "Interceptor {:?} callback failed: {}",
            failure.hook, failure.diagnostic
        );
    }
    if report.truncated_failure_details > 0 {
        eprintln!(
            "{} additional callback failure details were omitted",
            report.truncated_failure_details
        );
    }

    stream_error.map_or(Ok(()), |error| Err(ExampleError::Stream(error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_ergonomic::{
        interceptor::{InterceptorChain, StreamTerminalContext},
        ChatCompletionStream, InterceptedStream, Interceptor,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct TerminalCounter(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl Interceptor for TerminalCounter {
        async fn on_stream_terminal(
            &self,
            _ctx: &StreamTerminalContext<'_>,
        ) -> openai_ergonomic::Result<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn stream_error_is_returned_after_drain_and_still_shuts_down_exporter() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/stream")
            .with_header("content-type", "text/event-stream")
            .with_body("data: {not-json}\n\n")
            .create_async()
            .await;
        let response = reqwest::Client::new()
            .get(format!("{}/stream", server.url()))
            .send()
            .await
            .unwrap();
        let bytes = response.bytes().await.unwrap();
        mock.assert_async().await;
        drop(mock);
        drop(server);

        let terminal_calls = Arc::new(AtomicUsize::new(0));
        let mut interceptors = InterceptorChain::new();
        interceptors.add(Box::new(TerminalCounter(Arc::clone(&terminal_calls))));
        let response = reqwest::Response::from(http::Response::new(bytes));
        let (stream, completion) = InterceptedStream::new_with_completion(
            ChatCompletionStream::new(response),
            Arc::new(interceptors),
            "chat".into(),
            "mock-model".into(),
            "{}".into(),
            (),
        )
        .unwrap();

        let stream_result = consume_stream(Box::pin(stream), completion, |_| {}).await;
        let exporter_shutdowns = Arc::new(AtomicUsize::new(0));
        let shutdown_counter = Arc::clone(&exporter_shutdowns);
        let result = shutdown_after_streaming(stream_result, move || {
            shutdown_counter.fetch_add(1, Ordering::SeqCst);
            Ok::<(), std::io::Error>(())
        });

        assert!(matches!(
            result,
            Err(ExampleError::Stream(Error::StreamParsing { .. }))
        ));
        assert_eq!(terminal_calls.load(Ordering::SeqCst), 1);
        assert_eq!(exporter_shutdowns.load(Ordering::SeqCst), 1);
    }
}
