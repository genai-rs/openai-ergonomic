# Getting Started with openai-ergonomic

This guide helps you configure the crate, create a client, and make your first
request against the OpenAI API using the ergonomic builders provided by
`openai-ergonomic`.

## Installation

Add the crate to your `Cargo.toml` (Tokio is required for async examples):

```toml
[dependencies]
openai-ergonomic = "0.1"
tokio = { version = "1.37", features = ["full"] }
```

## Prerequisites

### 1. Obtain an OpenAI API Key
Create an API key from the [OpenAI dashboard](https://platform.openai.com/api-keys)
and export it locally:

```bash
export OPENAI_API_KEY="sk-your-key"
```

Optionally set `OPENAI_ORG_ID` if you need to scope requests to a specific
organization.

### 2. Minimum Tooling
- Rust 1.82 or newer
- `cargo` and `rustup`
- Network access to the OpenAI REST endpoints

## Creating a Client

The simplest way to construct a client is via environment variables:

```rust
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let client = Client::from_env()?;
    println!("Client ready: default model = {:?}", client.config().default_model());
    Ok(())
}
```

You can also build a configuration explicitly:

```rust
use openai_ergonomic::{Client, Config};

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let config = Config::builder()
        .api_key("sk-your-key")
        .api_base("https://api.openai.com/v1")
        .default_model("gpt-4.1-mini")
        .timeout_seconds(30)
        .build();

    let client = Client::new(config)?;
    // Use the client …
    Ok(())
}
```

## First Request: Responses API

```rust
use openai_ergonomic::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .responses()
        .system("You are a concise assistant.")
        .user("Summarise the benefits of type-safe builders in Rust.")
        .temperature(0.4);

    let response = client.send_responses(builder).await?;
    println!("Answer: {}", response.primary_text());
    Ok(())
}
```

`ChatCompletionResponseWrapper::primary_text()` extracts the first text chunk for
convenience. Inspect `.usage()` or iterate through `.choices()` for more detail.

## Chat Completions

```rust
use openai_ergonomic::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .chat()
        .system("You are an upbeat assistant.")
        .user("Suggest a codename for a Rust project building on OpenAI.");

    let response = client.send_chat(builder).await?;
    println!("Suggestion: {}", response.primary_text());
    Ok(())
}
```

## Text-to-Speech

```rust
use openai_client_base::models::create_speech_request::ResponseFormat as SpeechResponseFormat;
use openai_ergonomic::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    let speech = client
        .audio()
        .speech("gpt-4o-mini-tts", "Narrate this update", "alloy")
        .response_format(SpeechResponseFormat::Mp3)
        .speed(1.1);

    let audio_bytes = client.audio().create_speech(speech).await?;
    std::fs::write("speech.mp3", audio_bytes)?;
    Ok(())
}
```

## Audio Transcription

```rust
use openai_client_base::models::AudioResponseFormat;
use openai_ergonomic::{Client, Result, TimestampGranularity};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    let transcription = client
        .audio()
        .transcription("meeting.wav", "gpt-4o-mini-transcribe")
        .language("en")
        .response_format(AudioResponseFormat::VerboseJson)
        .timestamp_granularities([TimestampGranularity::Segment, TimestampGranularity::Word]);

    let response = client.audio().create_transcription(transcription).await?;
    println!("Transcript: {}", response.text);
    Ok(())
}
```

## Image Generation

```rust
use openai_ergonomic::{Client, Result, Size, Style};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    let request = client
        .images()
        .generate("A watercolor landscape with snow-capped mountains")
        .size(Size::Variant1024x1024)
        .style(Style::Natural);

    let response = client.images().create(request).await?;

    if let Some(first) = response.data.first() {
        if let Some(url) = &first.url {
            println!("Download the generated image at: {url}");
        }
    }

    Ok(())
}
```

## Working with Tools

The assistants builders expose convenience helpers for common tooling. For
example, enabling the code interpreter while configuring metadata:

```rust
use openai_ergonomic::builders::assistants::{
    assistant_with_instructions, tool_code_interpreter,
};

let assistant = assistant_with_instructions(
        "gpt-4.1", "Analytics Buddy", "You analyse CSV datasets and surface insights.")
    .description("Data analysis assistant")
    .add_tool(tool_code_interpreter())
    .add_metadata("team", "data-platform");

println!("Assistant ready with {} tools", assistant.tools_ref().len());
```

Combine these builders with the examples in `examples/assistants_*` to orchestrate
threads, file search, and vector store runs.

## Error Handling

All fallible operations return `openai_ergonomic::Result<T>`. Handle errors with
the re-exported `Error` type:

```rust
use openai_ergonomic::{Client, Error, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

    match client.send_chat(client.chat().user("Hello?")).await {
        Ok(resp) => println!("{}", resp.primary_text()),
        Err(Error::RateLimit { .. }) => eprintln!("Back off and retry later"),
        Err(other) => eprintln!("Request failed: {other}"),
    }

    Ok(())
}
```

## Next Steps

- Browse the curated `examples/` directory for end-to-end workflows
- Review the [architecture overview](architecture.md) and [API surface design](design/api_surface.md)
- Run `cargo doc --open` locally to explore the generated documentation
- Read [CONTRIBUTING.md](../CONTRIBUTING.md) and [docs/workflow.md](workflow.md) before opening PRs

## Getting Help

- Check the docs on [docs.rs](https://docs.rs/openai-ergonomic)
- Search existing issues or open a new one on GitHub
- Reach out on the maintainer channels listed in `AGENTS.md`

Happy coding with `openai-ergonomic`!
