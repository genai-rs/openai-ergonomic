#![allow(missing_docs)]

use mockito::{self, Matcher};
use openai_ergonomic::{
    builders::images::{ImageEditBuilder, ImageGenerationBuilder, ImageVariationBuilder},
    Client, Config,
};
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn images_client_create_hits_generation_endpoint() {
    let mut server = mockito::Server::new_async().await;

    let expected_body = json!({
        "prompt": "A cat on a surfboard",
        "model": "gpt-image-1"
    });

    let mock = server
        .mock("POST", "/images/generations")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(expected_body))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "created": 12,
                "data": [
                    { "url": "https://example.test/image.png" }
                ]
            }"#,
        )
        .create();

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-image-1")
        .build();
    let client = Client::builder(config).expect("client builds").build();

    let builder = ImageGenerationBuilder::new("A cat on a surfboard").model("gpt-image-1");
    let response = client
        .images()
        .create(builder)
        .await
        .expect("request succeeds");

    assert_eq!(response.created, 12);
    let urls: Vec<_> = response
        .data
        .unwrap_or_default()
        .into_iter()
        .filter_map(|image| image.url)
        .collect();
    assert_eq!(urls, vec!["https://example.test/image.png".to_string()]);

    mock.assert();
    drop(server);
}

#[tokio::test]
async fn images_client_create_edit_posts_multipart() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("POST", "/images/edits")
        .match_header("authorization", "Bearer test-key")
        .match_header(
            "content-type",
            Matcher::Regex("^multipart/form-data; boundary=.*$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "created": 21,
                "data": [
                    { "b64_json": "ZGF0YQ==" }
                ]
            }"#,
        )
        .create();

    let mut image = NamedTempFile::new().expect("create temp image");
    writeln!(image, "fake image data").expect("write image");

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-image-1")
        .build();
    let client = Client::builder(config).expect("client builds").build();

    let builder = ImageEditBuilder::new(image.path(), "Remove background").model("gpt-image-1");
    let response = client
        .images()
        .create_edit(builder)
        .await
        .expect("request succeeds");

    assert_eq!(response.created, 21);
    mock.assert();
    drop(server);
}

#[tokio::test]
async fn images_client_create_variation_posts_multipart() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("POST", "/images/variations")
        .match_header("authorization", "Bearer test-key")
        .match_header(
            "content-type",
            Matcher::Regex("^multipart/form-data; boundary=.*$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "created": 33,
                "data": [
                    { "url": "https://example.test/variation.png" }
                ]
            }"#,
        )
        .create();

    let mut image = NamedTempFile::new().expect("create temp image");
    writeln!(image, "fake image data").expect("write image");

    let config = Config::builder()
        .api_key("test-key")
        .api_base(server.url())
        .default_model("gpt-image-1")
        .build();
    let client = Client::builder(config).expect("client builds").build();

    let builder = ImageVariationBuilder::new(image.path()).model("dall-e-2");
    let response = client
        .images()
        .create_variation(builder)
        .await
        .expect("request succeeds");

    assert_eq!(response.created, 33);
    mock.assert();
    drop(server);
}
