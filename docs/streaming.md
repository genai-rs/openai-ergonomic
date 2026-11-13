# Streaming

`send_chat_stream` and `send_responses_stream` send a builder with `stream: true` and return a
`BoxedChatStream` of `ChatCompletionChunk`s, decoded from the server-sent events as they arrive.
An error status before the first event is an `Error::Api` with the `type` and `code` of the
`OpenAI` error body.

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
    while let Some(chunk) = stream.next().await {
        print!("{}", chunk?.content().unwrap_or_default());
    }
    // Drop the stream first when stopping early: the outcome is then a cancellation.
    drop(stream);
    let report = completion.wait().await?;
    println!(
        "\n{:?} after {} chunks; {} callback failures",
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
