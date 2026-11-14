# Streaming

`send_chat_stream` sends a chat builder with `stream: true` and returns a `BoxedChatStream` of
`ChatCompletionChunk`s, decoded from the server-sent events as they arrive;
`send_responses_stream` does the same for the Responses API and returns a `BoxedResponsesStream`
of `ResponsesEvent`s (see [Responses workflows](responses_workflows.md#streaming)). An error
status before the first event is an `Error::Api` with the `type` and `code` of the `OpenAI`
error body.

A chat stream completes at `data: [DONE]`, or when the body ends after a chunk with a
`finish_reason`; a body that ends before either was cut off, and the stream yields
`Error::StreamConnection` as its last item. A Responses stream completes with its terminal event
(`response.completed`, `response.incomplete`, `response.failed` or `error`); a body that ends
before one yields `Error::StreamConnection` too. With
[`stream_idle_timeout`](configuration.md#retries-timeouts-and-custom-http-clients) set, a stream
that receives no event for that long ends with `Error::StreamConnection` as well.

```rust,no_run
use futures::StreamExt;
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let mut stream = client
        .send_chat_stream(client.chat().user("Tell me a story"))
        .await?;
    while let Some(chunk) = stream.next().await {
        if let Some(content) = chunk?.content() {
            print!("{content}");
        }
    }
    Ok(())
}
```

## Waiting for the interceptors

[Interceptors](interceptors.md) see every chunk and the end of the stream; their callbacks run
in the background (a tokio task natively, `spawn_local` on WebAssembly) so that they do not slow
the stream down. `send_chat_stream_with_completion` and `send_responses_stream_with_completion`
also return a `StreamCompletion`, whose `wait()` resolves once the stream has ended (completed,
failed or dropped) and every callback has returned: wait for it before shutting down an exporter,
so that no span is lost.

```rust,no_run
use futures::StreamExt;
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let (mut stream, completion) = client
        .send_responses_stream_with_completion(client.responses().user("Count to five"))
        .await?;
    while let Some(event) = stream.next().await {
        print!("{}", event?.text_delta().unwrap_or_default());
    }
    // Drop the stream first when stopping early: the outcome is then a cancellation.
    drop(stream);
    let report = completion.wait().await?;
    println!(
        "\n{:?} after {} events; {} callback failures",
        report.outcome, report.total_chunks, report.callback_failure_count
    );
    Ok(())
}
```

The [completion contract](streaming-completion-contract.md) describes the guarantees in detail.
To stream events into types of your own, see `send_json_stream` in
[chat and responses](chat-and-responses.md#bring-your-own-types). Examples:
[`chat_streaming.rs`](../examples/chat_streaming.rs),
[`responses_streaming.rs`](../examples/responses_streaming.rs).
