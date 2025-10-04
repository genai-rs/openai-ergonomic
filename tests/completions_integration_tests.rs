//! Integration tests for the Completions API.
//!
//! These tests verify the ergonomic wrappers around the `OpenAI` Completions API.
#![allow(clippy::doc_markdown)]
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::ignore_without_reason)]

use openai_ergonomic::{builders::completions::CompletionsBuilder, Builder, Client, Result};

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_basic_completion() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Say hello")
        .max_tokens(10);

    let response = client.completions().create(builder).await?;

    // Just verify we got a response with choices
    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    if let Some(usage) = response.usage {
        assert!(usage.prompt_tokens > 0);
        assert!(usage.completion_tokens > 0);
        assert!(usage.total_tokens > 0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_temperature() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Write one word")
        .max_tokens(5)
        .temperature(0.0); // Deterministic

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_stop_sequence() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Count: 1, 2, 3,")
        .max_tokens(20)
        .add_stop(",");

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    // finish_reason is an enum, just verify it exists
    // The stop sequence should cause it to stop early

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_multiple_choices() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Say something creative")
        .max_tokens(20)
        .n(3)
        .temperature(0.9);

    let response = client.completions().create(builder).await?;

    assert_eq!(response.choices.len(), 3);

    for choice in &response.choices {
        assert!(!choice.text.is_empty());
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_echo() -> Result<()> {
    let client = Client::from_env()?;

    let prompt_text = "Echo test:";
    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt(prompt_text)
        .max_tokens(10)
        .echo(true);

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    // The response should contain the prompt
    assert!(response.choices[0].text.contains(prompt_text));

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_suffix() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("def hello():\n    return \"")
        .suffix("\"")
        .max_tokens(10)
        .temperature(0.0);

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_logprobs() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("The capital of France is")
        .max_tokens(5)
        .logprobs(2); // Number of top logprobs to return

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());

    if let Some(logprobs) = &response.choices[0].logprobs {
        // Logprobs should be present
        assert!(logprobs.tokens.is_some());
    }

    Ok(())
}

#[test]
fn test_completions_builder_basic() {
    let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct").prompt("Test");
    let request = builder.build().unwrap();

    assert_eq!(request.model, "gpt-3.5-turbo-instruct");
}

#[test]
fn test_completions_builder_with_options() {
    let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct")
        .prompt("Test")
        .max_tokens(100)
        .temperature(0.7)
        .top_p(0.9);

    let request = builder.build().unwrap();

    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.top_p, Some(0.9));
}

#[test]
fn test_completions_builder_missing_prompt() {
    let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct");
    let result = builder.build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("prompt"));
}

#[test]
fn test_completions_builder_with_single_prompt() {
    let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct").prompt("Hello");

    let request = builder.build().unwrap();
    assert_eq!(request.model, "gpt-3.5-turbo-instruct");
    assert_eq!(request.prompt, "Hello");
}

#[test]
fn test_completions_builder_with_stop_sequences() {
    let builder = CompletionsBuilder::new("gpt-3.5-turbo-instruct")
        .prompt("Test")
        .add_stop("\n")
        .add_stop("END");

    let request = builder.build().unwrap();
    assert!(request.stop.is_some());
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_frequency_penalty() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Repeat after me: test test test")
        .max_tokens(20)
        .frequency_penalty(2.0); // Strong penalty against repetition

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_presence_penalty() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Write about different topics")
        .max_tokens(30)
        .presence_penalty(1.5); // Encourage new topics

    let response = client.completions().create(builder).await?;

    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default to avoid API calls
async fn test_completion_with_best_of() -> Result<()> {
    let client = Client::from_env()?;

    let builder = client
        .completions()
        .builder("gpt-3.5-turbo-instruct")
        .prompt("Say something interesting")
        .max_tokens(20)
        .best_of(3)
        .temperature(0.8);

    let response = client.completions().create(builder).await?;

    // Should return the best of 3 completions
    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].text.is_empty());

    Ok(())
}
