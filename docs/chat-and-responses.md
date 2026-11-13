# Chat and responses

`client.chat()` starts a `ChatCompletionBuilder` and `client.responses()` a `ResponsesBuilder`,
both with the configured default model (`Config::builder().default_model(..)`);
`ChatCompletionBuilder::new(model)` and `ResponsesBuilder::new(model)` start one for another model.
`send_chat` and `send_responses` send them and return a `ChatCompletionResponseWrapper`:
`content()`, `tool_calls()`, `finish_reason()`, `refusal()`, and `inner()` for the generated
response. For multi-step Responses workflows (tools, web search, structured output) see
[Responses workflows](responses_workflows.md); for streams, [streaming](streaming.md); for tools,
the [tool framework](tool_framework.md).

## Messages, parameters and images

```rust,no_run
use openai_ergonomic::{ChatCompletionBuilder, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = ChatCompletionBuilder::new("gpt-4o-mini")
        .system("You describe images in one sentence.")
        .user_with_image_url(
            "What is in this picture?",
            "https://upload.wikimedia.org/wikipedia/commons/3/3a/Cat03.jpg",
        )
        .temperature(0.2)
        .max_completion_tokens(200);
    let response = client.send_chat(builder).await?;
    println!("{}", response.content().unwrap_or_default());
    Ok(())
}
```

## Structured output

```rust,no_run
use openai_ergonomic::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .responses()
        .user("Name a city and its country.")
        .json_schema(
            "city",
            json!({
                "type": "object",
                "properties": {"city": {"type": "string"}, "country": {"type": "string"}},
                "required": ["city", "country"],
                "additionalProperties": false
            }),
        );
    let response = client.send_responses(builder).await?;
    let city: serde_json::Value = serde_json::from_str(response.content().unwrap_or("{}"))?;
    println!("{} ({})", city["city"], city["country"]);
    Ok(())
}
```

`json_mode()` asks for any JSON object instead. See
[`examples/structured_outputs.rs`](../examples/structured_outputs.rs).

## Bring your own types

The builders use the `OpenAI` spec's models. To send and decode types of your own (a struct with
only the fields you need, `serde_json::Value`, or an `OpenAI`-compatible provider whose replies
differ from the spec), use `post_json`, `get_json`, `send_json` and `send_json_stream`. They send
to `{api_base}{path}` through the same HTTP client (retries, layers, the provider's layer,
organization and project headers) with the same authentication, and map an error status to
`Error::Api`. Interceptors (such as Langfuse) are not called for these requests.

```rust,no_run
use futures::StreamExt;
use openai_ergonomic::{Client, Config};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Any OpenAI-compatible endpoint
    let config = Config::builder()
        .api_key("provider-key")
        .api_base("https://openai-compatible.example.com/v1")
        .build();
    let client = Client::builder(config)?.build();

    let request = json!({
        "model": "some-model",
        "messages": [{"role": "user", "content": "Hello!"}]
    });
    let completion: Completion = client.post_json("/chat/completions", &request).await?;
    println!("{:?}", completion.choices[0].message.content);

    // Streaming: each server-sent event decoded into a type of your own
    let mut request = request;
    request["stream"] = json!(true);
    let mut events = client
        .send_json_stream::<_, serde_json::Value>("/chat/completions", &request)
        .await?;
    while let Some(event) = events.next().await {
        print!("{}", event?["choices"][0]["delta"]["content"].as_str().unwrap_or_default());
    }
    Ok(())
}
```

## The generated operations

Every operation of the API groups the crate enables is available from
[`openai-client-base`](https://docs.rs/openai-client-base) as a function with a builder. Point its
`Configuration` at the client's HTTP stack to send through the same provider, retries and layers;
[configuration](configuration.md#per-call-options) has an example, with per-call options.
