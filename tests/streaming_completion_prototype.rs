//! OE05 baseline characterizations, not assertions of the desired OE06 behavior.
//! Gates delay real interceptor callbacks deterministically; timeouts only bound hangs.

use futures::StreamExt;
use openai_ergonomic::interceptor::{
    ErrorContext, InterceptorChain, StreamChunkContext, StreamEndContext,
};
use openai_ergonomic::{ChatCompletionStream, Error, InterceptedStream, Interceptor, Result};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Semaphore};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Chunk(usize),
    End(usize),
    Error,
}

struct GatedInterceptor {
    gates: Vec<Arc<Semaphore>>,
    events: mpsc::UnboundedSender<Event>,
    fail_chunks: bool,
}

#[async_trait::async_trait]
impl Interceptor for GatedInterceptor {
    async fn on_stream_chunk(&self, ctx: &StreamChunkContext<'_>) -> Result<()> {
        self.gates[ctx.chunk_index]
            .acquire()
            .await
            .unwrap()
            .forget();
        self.events.send(Event::Chunk(ctx.chunk_index)).unwrap();
        if self.fail_chunks {
            Err(Error::Internal("mock callback failure".into()))
        } else {
            Ok(())
        }
    }

    async fn on_stream_end(&self, ctx: &StreamEndContext<'_>) -> Result<()> {
        self.events.send(Event::End(ctx.total_chunks)).unwrap();
        Ok(())
    }

    async fn on_error(&self, _ctx: &ErrorContext<'_>) {
        self.events.send(Event::Error).unwrap();
    }
}

fn chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"mock\",\"object\":\"chat.completion.chunk\",",
        "\"created\":1,\"model\":\"mock-model\",\"choices\":[{\"index\":0,",
        "\"delta\":{\"content\":\"hello\"},\"finish_reason\":null}]}\n\n"
    )
}

async fn stream(
    body: String,
    gates: Vec<Arc<Semaphore>>,
    fail_chunks: bool,
) -> (InterceptedStream, mpsc::UnboundedReceiver<Event>) {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/stream")
        .with_header("content-type", "text/event-stream")
        .with_body(body)
        .create_async()
        .await;
    let response = reqwest::Client::new()
        .get(format!("{}/stream", server.url()))
        .send()
        .await
        .unwrap();
    // Retain the local server through body consumption: buffer the response body
    // into an HTTP response without changing the production SSE parser/wrapper.
    let bytes = response.bytes().await.unwrap();
    mock.assert_async().await;
    drop(mock);
    drop(server);
    let response = reqwest::Response::from(http::Response::new(bytes));
    let (events, receiver) = mpsc::unbounded_channel();
    let mut chain = InterceptorChain::new();
    chain.add(Box::new(GatedInterceptor {
        gates,
        events,
        fail_chunks,
    }));
    (
        InterceptedStream::new(
            ChatCompletionStream::new(response),
            Arc::new(chain),
            "chat".into(),
            "mock-model".into(),
            "{}".into(),
            (),
        ),
        receiver,
    )
}

async fn bounded(future: impl Future<Output = ()>) {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("mock lifecycle must finish without a timing sleep");
}

#[tokio::test]
async fn baseline_eof_can_precede_callbacks_and_chunk_order_can_reverse() {
    bounded(async {
        let gates = vec![Arc::new(Semaphore::new(0)), Arc::new(Semaphore::new(0))];
        let (mut stream, mut events) = stream(chunk().repeat(2), gates.clone(), false).await;
        assert!(stream.next().await.unwrap().is_ok());
        assert!(stream.next().await.unwrap().is_ok());
        assert!(stream.next().await.is_none());
        // Both gates are closed: EOF and end have occurred with zero completed chunks.
        assert_eq!(events.recv().await, Some(Event::End(2)));
        gates[1].add_permits(1);
        assert_eq!(events.recv().await, Some(Event::Chunk(1)));
        gates[0].add_permits(1);
        assert_eq!(events.recv().await, Some(Event::Chunk(0)));
        drop(stream);
        assert_eq!(events.recv().await, None);
    })
    .await;
}

#[tokio::test]
async fn baseline_repeated_eof_invokes_end_twice() {
    bounded(async {
        let (mut stream, mut events) = stream(String::new(), vec![], false).await;
        assert!(stream.next().await.is_none());
        assert_eq!(events.recv().await, Some(Event::End(0)));
        assert!(stream.next().await.is_none());
        assert_eq!(events.recv().await, Some(Event::End(0)));
        drop(stream);
        assert_eq!(events.recv().await, None);
    })
    .await;
}

#[tokio::test]
async fn baseline_drop_leaves_callback_running_without_terminal_notification() {
    bounded(async {
        let gate = Arc::new(Semaphore::new(0));
        let (mut stream, mut events) = stream(chunk().repeat(2), vec![gate.clone()], false).await;
        assert!(stream.next().await.unwrap().is_ok());
        drop(stream); // Second chunk is never polled; no successful EOF was observed.
        gate.add_permits(1);
        assert_eq!(events.recv().await, Some(Event::Chunk(0)));
        // Channel closure proves no terminal callback remains, without a negative timeout.
        assert_eq!(events.recv().await, None);
    })
    .await;
}

#[tokio::test]
async fn baseline_stream_error_is_followed_by_success_end_hook() {
    bounded(async {
        let (mut stream, mut events) = stream("data: not-json\n\n".into(), vec![], false).await;
        assert!(matches!(
            stream.next().await.unwrap(),
            Err(Error::StreamParsing { .. })
        ));
        assert!(stream.next().await.is_none());
        assert_eq!(events.recv().await, Some(Event::End(0)));
        drop(stream);
        assert_eq!(events.recv().await, None); // No on_error notification.
    })
    .await;
}

#[tokio::test]
async fn baseline_callback_failure_is_not_reported_to_stream_consumer() {
    bounded(async {
        let gate = Arc::new(Semaphore::new(0));
        let (mut stream, mut events) = stream(chunk().into(), vec![gate.clone()], true).await;
        assert!(stream.next().await.unwrap().is_ok());
        gate.add_permits(1);
        assert_eq!(events.recv().await, Some(Event::Chunk(0)));
        assert!(stream.next().await.is_none());
        assert_eq!(events.recv().await, Some(Event::End(1)));
        drop(stream);
        assert_eq!(events.recv().await, None);
    })
    .await;
}
