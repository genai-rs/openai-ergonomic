//! Integration tests for the Moderations API.

use openai_ergonomic::builders::moderations::ModerationBuilder;

#[test]
fn test_moderation_builder_basic() {
    let builder = ModerationBuilder::new("This is a test message");
    let request = builder.build().unwrap();

    assert_eq!(request.input, "This is a test message");
    assert!(request.model.is_none());
}

#[test]
fn test_moderation_builder_with_model() {
    let builder = ModerationBuilder::new("Test content").model("text-moderation-latest");
    let request = builder.build().unwrap();

    assert_eq!(request.input, "Test content");
    assert_eq!(request.model, Some("text-moderation-latest".to_string()));
}

#[test]
fn test_moderation_builder_array() {
    let inputs = vec!["Message 1".to_string(), "Message 2".to_string()];
    let builder = ModerationBuilder::new_array(inputs);
    let request = builder.build().unwrap();

    // Array inputs are joined with newlines
    assert_eq!(request.input, "Message 1\nMessage 2");
}

#[test]
fn test_moderation_builder_helpers() {
    use openai_ergonomic::builders::moderations::{
        moderate_text, moderate_text_with_model, moderate_texts,
    };

    // Test moderate_text
    let builder = moderate_text("Test");
    assert_eq!(builder.first_input(), Some("Test"));

    // Test moderate_text_with_model
    let builder = moderate_text_with_model("Test", "text-moderation-stable");
    assert_eq!(builder.first_input(), Some("Test"));
    assert_eq!(builder.model_ref(), Some("text-moderation-stable"));

    // Test moderate_texts
    let inputs = vec!["Text 1".to_string(), "Text 2".to_string()];
    let builder = moderate_texts(inputs);
    assert_eq!(builder.input_count(), 2);
}

#[test]
fn test_moderation_categories() {
    use openai_ergonomic::builders::moderations::ModerationCategories;

    let categories = ModerationCategories::new_clean();
    assert!(!categories.any_flagged());
    assert!(categories.flagged_categories().is_empty());
}

#[test]
fn test_moderation_category_scores() {
    use openai_ergonomic::builders::moderations::ModerationCategoryScores;

    let mut scores = ModerationCategoryScores::new_zero();
    assert!((scores.max_score() - 0.0).abs() < f64::EPSILON);

    scores.hate = 0.9;
    scores.violence = 0.3;

    assert!((scores.max_score() - 0.9).abs() < f64::EPSILON);

    let high_scores = scores.scores_above_threshold(0.5);
    assert_eq!(high_scores.len(), 1);
    assert!(high_scores.contains(&("hate", 0.9)));
}

#[test]
fn test_moderation_result() {
    use openai_ergonomic::builders::moderations::ModerationResult;

    let result = ModerationResult::new_clean();
    assert!(result.is_safe());
    assert!(result.flagged_summary().is_none());
}

// Note: These tests require API credentials and should be run with care
// They are commented out by default to avoid consuming API credits during regular testing

/*
#[tokio::test]
async fn test_moderations_api_integration() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .moderations()
        .check("This is a normal, safe message about weather");

    let response = client.moderations().create(builder).await?;

    assert!(!response.results.is_empty());
    assert_eq!(response.model, "text-moderation-latest");

    Ok(())
}

#[tokio::test]
async fn test_moderations_with_custom_model() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    let builder = client
        .moderations()
        .builder("Test content")
        .model("text-moderation-stable");

    let response = client.moderations().create(builder).await?;

    assert!(!response.results.is_empty());
    // The API may normalize the model name
    assert!(response.model.contains("moderation"));

    Ok(())
}

#[tokio::test]
async fn test_moderations_flagging() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();

    // Test with safe content
    let safe_builder = client.moderations().check("Hello, how are you today?");
    let safe_response = client.moderations().create(safe_builder).await?;

    assert!(!safe_response.results.is_empty());
    let safe_result = &safe_response.results[0];
    assert!(!safe_result.flagged, "Safe content should not be flagged");

    Ok(())
}
*/
