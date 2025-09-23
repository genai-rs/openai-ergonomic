//! Moderations API builders.
//!
//! This module provides ergonomic builders for `OpenAI` Moderations API operations,
//! which help detect potentially harmful content across various categories.
//!
//! The Moderations API can identify content that may be:
//! - Hate speech
//! - Harassment
//! - Self-harm related
//! - Sexual content
//! - Violence
//! - And other harmful categories

/// Builder for content moderation requests.
///
/// This builder provides a fluent interface for creating moderation requests
/// to check if content violates `OpenAI`'s usage policies.
#[derive(Debug, Clone)]
pub struct ModerationBuilder {
    input: ModerationInput,
    model: Option<String>,
}

/// Input for moderation requests.
#[derive(Debug, Clone)]
pub enum ModerationInput {
    /// Single text input
    Text(String),
    /// Multiple text inputs
    TextArray(Vec<String>),
}

/// Result of a moderation check.
#[derive(Debug, Clone)]
pub struct ModerationResult {
    /// Whether the content was flagged
    pub flagged: bool,
    /// The flagged categories
    pub categories: ModerationCategories,
    /// The confidence scores for each category
    pub category_scores: ModerationCategoryScores,
}

/// Categories that can be flagged by moderation.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ModerationCategories {
    /// Hate speech
    pub hate: bool,
    /// Threatening hate speech
    pub hate_threatening: bool,
    /// Harassment
    pub harassment: bool,
    /// Threatening harassment
    pub harassment_threatening: bool,
    /// Self-harm content
    pub self_harm: bool,
    /// Intent to self-harm
    pub self_harm_intent: bool,
    /// Instructions for self-harm
    pub self_harm_instructions: bool,
    /// Sexual content
    pub sexual: bool,
    /// Sexual content involving minors
    pub sexual_minors: bool,
    /// Violence
    pub violence: bool,
    /// Graphic violence
    pub violence_graphic: bool,
}

/// Confidence scores for each moderation category.
#[derive(Debug, Clone)]
pub struct ModerationCategoryScores {
    /// Hate speech score
    pub hate: f64,
    /// Threatening hate speech score
    pub hate_threatening: f64,
    /// Harassment score
    pub harassment: f64,
    /// Threatening harassment score
    pub harassment_threatening: f64,
    /// Self-harm content score
    pub self_harm: f64,
    /// Intent to self-harm score
    pub self_harm_intent: f64,
    /// Instructions for self-harm score
    pub self_harm_instructions: f64,
    /// Sexual content score
    pub sexual: f64,
    /// Sexual content involving minors score
    pub sexual_minors: f64,
    /// Violence score
    pub violence: f64,
    /// Graphic violence score
    pub violence_graphic: f64,
}

impl ModerationBuilder {
    /// Create a new moderation builder with text input.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::moderations::ModerationBuilder;
    ///
    /// let builder = ModerationBuilder::new("Check this text for harmful content");
    /// ```
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: ModerationInput::Text(input.into()),
            model: None,
        }
    }

    /// Create a moderation builder with multiple text inputs.
    #[must_use]
    pub fn new_array(inputs: Vec<String>) -> Self {
        Self {
            input: ModerationInput::TextArray(inputs),
            model: None,
        }
    }

    /// Set the moderation model to use.
    ///
    /// Common models include:
    /// - `text-moderation-latest` (default)
    /// - `text-moderation-stable`
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Get the input for this moderation request.
    #[must_use]
    pub fn input(&self) -> &ModerationInput {
        &self.input
    }

    /// Get the model for this moderation request.
    #[must_use]
    pub fn model_ref(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Check if this request has multiple inputs.
    #[must_use]
    pub fn has_multiple_inputs(&self) -> bool {
        matches!(self.input, ModerationInput::TextArray(_))
    }

    /// Get the number of inputs in this request.
    #[must_use]
    pub fn input_count(&self) -> usize {
        match &self.input {
            ModerationInput::Text(_) => 1,
            ModerationInput::TextArray(texts) => texts.len(),
        }
    }

    /// Get the first input text (useful for single input requests).
    #[must_use]
    pub fn first_input(&self) -> Option<&str> {
        match &self.input {
            ModerationInput::Text(text) => Some(text),
            ModerationInput::TextArray(texts) => texts.first().map(std::string::String::as_str),
        }
    }

    /// Get all input texts as a vector.
    #[must_use]
    pub fn all_inputs(&self) -> Vec<&str> {
        match &self.input {
            ModerationInput::Text(text) => vec![text],
            ModerationInput::TextArray(texts) => {
                texts.iter().map(std::string::String::as_str).collect()
            }
        }
    }
}

impl ModerationCategories {
    /// Create a new `ModerationCategories` with all categories set to false.
    #[must_use]
    pub fn new_clean() -> Self {
        Self {
            hate: false,
            hate_threatening: false,
            harassment: false,
            harassment_threatening: false,
            self_harm: false,
            self_harm_intent: false,
            self_harm_instructions: false,
            sexual: false,
            sexual_minors: false,
            violence: false,
            violence_graphic: false,
        }
    }

    /// Check if any category is flagged.
    #[must_use]
    pub fn any_flagged(&self) -> bool {
        self.hate
            || self.hate_threatening
            || self.harassment
            || self.harassment_threatening
            || self.self_harm
            || self.self_harm_intent
            || self.self_harm_instructions
            || self.sexual
            || self.sexual_minors
            || self.violence
            || self.violence_graphic
    }

    /// Get all flagged categories as a vector of strings.
    #[must_use]
    pub fn flagged_categories(&self) -> Vec<&'static str> {
        let mut flagged = Vec::new();
        if self.hate {
            flagged.push("hate");
        }
        if self.hate_threatening {
            flagged.push("hate/threatening");
        }
        if self.harassment {
            flagged.push("harassment");
        }
        if self.harassment_threatening {
            flagged.push("harassment/threatening");
        }
        if self.self_harm {
            flagged.push("self-harm");
        }
        if self.self_harm_intent {
            flagged.push("self-harm/intent");
        }
        if self.self_harm_instructions {
            flagged.push("self-harm/instructions");
        }
        if self.sexual {
            flagged.push("sexual");
        }
        if self.sexual_minors {
            flagged.push("sexual/minors");
        }
        if self.violence {
            flagged.push("violence");
        }
        if self.violence_graphic {
            flagged.push("violence/graphic");
        }
        flagged
    }
}

impl ModerationCategoryScores {
    /// Create a new `ModerationCategoryScores` with all scores set to 0.0.
    #[must_use]
    pub fn new_zero() -> Self {
        Self {
            hate: 0.0,
            hate_threatening: 0.0,
            harassment: 0.0,
            harassment_threatening: 0.0,
            self_harm: 0.0,
            self_harm_intent: 0.0,
            self_harm_instructions: 0.0,
            sexual: 0.0,
            sexual_minors: 0.0,
            violence: 0.0,
            violence_graphic: 0.0,
        }
    }

    /// Get the highest score across all categories.
    #[must_use]
    pub fn max_score(&self) -> f64 {
        [
            self.hate,
            self.hate_threatening,
            self.harassment,
            self.harassment_threatening,
            self.self_harm,
            self.self_harm_intent,
            self.self_harm_instructions,
            self.sexual,
            self.sexual_minors,
            self.violence,
            self.violence_graphic,
        ]
        .iter()
        .fold(0.0, |max, &score| if score > max { score } else { max })
    }

    /// Get scores above a certain threshold.
    #[must_use]
    pub fn scores_above_threshold(&self, threshold: f64) -> Vec<(&'static str, f64)> {
        let mut high_scores = Vec::new();
        if self.hate > threshold {
            high_scores.push(("hate", self.hate));
        }
        if self.hate_threatening > threshold {
            high_scores.push(("hate/threatening", self.hate_threatening));
        }
        if self.harassment > threshold {
            high_scores.push(("harassment", self.harassment));
        }
        if self.harassment_threatening > threshold {
            high_scores.push(("harassment/threatening", self.harassment_threatening));
        }
        if self.self_harm > threshold {
            high_scores.push(("self-harm", self.self_harm));
        }
        if self.self_harm_intent > threshold {
            high_scores.push(("self-harm/intent", self.self_harm_intent));
        }
        if self.self_harm_instructions > threshold {
            high_scores.push(("self-harm/instructions", self.self_harm_instructions));
        }
        if self.sexual > threshold {
            high_scores.push(("sexual", self.sexual));
        }
        if self.sexual_minors > threshold {
            high_scores.push(("sexual/minors", self.sexual_minors));
        }
        if self.violence > threshold {
            high_scores.push(("violence", self.violence));
        }
        if self.violence_graphic > threshold {
            high_scores.push(("violence/graphic", self.violence_graphic));
        }
        high_scores
    }
}

impl ModerationResult {
    /// Create a new clean moderation result (not flagged).
    #[must_use]
    pub fn new_clean() -> Self {
        Self {
            flagged: false,
            categories: ModerationCategories::new_clean(),
            category_scores: ModerationCategoryScores::new_zero(),
        }
    }

    /// Check if the content is safe (not flagged).
    #[must_use]
    pub fn is_safe(&self) -> bool {
        !self.flagged
    }

    /// Get a summary of why content was flagged (if it was).
    #[must_use]
    pub fn flagged_summary(&self) -> Option<Vec<&'static str>> {
        if self.flagged {
            Some(self.categories.flagged_categories())
        } else {
            None
        }
    }
}

/// Helper function to create a simple moderation request.
#[must_use]
pub fn moderate_text(input: impl Into<String>) -> ModerationBuilder {
    ModerationBuilder::new(input)
}

/// Helper function to create a moderation request with multiple inputs.
#[must_use]
pub fn moderate_texts(inputs: Vec<String>) -> ModerationBuilder {
    ModerationBuilder::new_array(inputs)
}

/// Helper function to create a moderation request with a specific model.
#[must_use]
pub fn moderate_text_with_model(
    input: impl Into<String>,
    model: impl Into<String>,
) -> ModerationBuilder {
    ModerationBuilder::new(input).model(model)
}

/// Helper function to moderate a batch of messages.
#[must_use]
pub fn moderate_messages(messages: &[impl AsRef<str>]) -> ModerationBuilder {
    let inputs = messages
        .iter()
        .map(|msg| msg.as_ref().to_string())
        .collect();
    ModerationBuilder::new_array(inputs)
}

/// Check if a given text is likely to be flagged based on simple heuristics.
/// This is not a replacement for the actual API, just a helper for testing.
#[must_use]
pub fn likely_flagged(text: &str) -> bool {
    let lower = text.to_lowercase();
    // This is a very basic heuristic - the real API is much more sophisticated
    lower.contains("hate") || lower.contains("violence") || lower.contains("harmful")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moderation_builder_new() {
        let builder = ModerationBuilder::new("Test content");

        assert_eq!(builder.input_count(), 1);
        assert_eq!(builder.first_input(), Some("Test content"));
        assert!(!builder.has_multiple_inputs());
        assert!(builder.model_ref().is_none());
    }

    #[test]
    fn test_moderation_builder_new_array() {
        let inputs = vec!["First text".to_string(), "Second text".to_string()];
        let builder = ModerationBuilder::new_array(inputs);

        assert_eq!(builder.input_count(), 2);
        assert_eq!(builder.first_input(), Some("First text"));
        assert!(builder.has_multiple_inputs());
        assert_eq!(builder.all_inputs(), vec!["First text", "Second text"]);
    }

    #[test]
    fn test_moderation_builder_with_model() {
        let builder = ModerationBuilder::new("Test").model("text-moderation-stable");

        assert_eq!(builder.model_ref(), Some("text-moderation-stable"));
    }

    #[test]
    fn test_moderation_categories_new_clean() {
        let categories = ModerationCategories::new_clean();
        assert!(!categories.any_flagged());
        assert!(categories.flagged_categories().is_empty());
    }

    #[test]
    fn test_moderation_categories_flagged() {
        let mut categories = ModerationCategories::new_clean();
        categories.hate = true;
        categories.violence = true;

        assert!(categories.any_flagged());
        let flagged = categories.flagged_categories();
        assert_eq!(flagged.len(), 2);
        assert!(flagged.contains(&"hate"));
        assert!(flagged.contains(&"violence"));
    }

    #[test]
    fn test_moderation_category_scores_new_zero() {
        let scores = ModerationCategoryScores::new_zero();
        assert!((scores.max_score() - 0.0).abs() < f64::EPSILON);
        assert!(scores.scores_above_threshold(0.1).is_empty());
    }

    #[test]
    fn test_moderation_category_scores_max_and_threshold() {
        let mut scores = ModerationCategoryScores::new_zero();
        scores.hate = 0.8;
        scores.violence = 0.6;
        scores.sexual = 0.3;

        assert!((scores.max_score() - 0.8).abs() < f64::EPSILON);

        let high_scores = scores.scores_above_threshold(0.5);
        assert_eq!(high_scores.len(), 2);
        assert!(high_scores.contains(&("hate", 0.8)));
        assert!(high_scores.contains(&("violence", 0.6)));
    }

    #[test]
    fn test_moderation_result_new_clean() {
        let result = ModerationResult::new_clean();
        assert!(result.is_safe());
        assert!(result.flagged_summary().is_none());
    }

    #[test]
    fn test_moderation_result_flagged() {
        let mut result = ModerationResult::new_clean();
        result.flagged = true;
        result.categories.hate = true;

        assert!(!result.is_safe());
        let summary = result.flagged_summary().unwrap();
        assert_eq!(summary, vec!["hate"]);
    }

    #[test]
    fn test_moderate_text_helper() {
        let builder = moderate_text("Test content");
        assert_eq!(builder.first_input(), Some("Test content"));
        assert!(!builder.has_multiple_inputs());
    }

    #[test]
    fn test_moderate_texts_helper() {
        let inputs = vec!["Text 1".to_string(), "Text 2".to_string()];
        let builder = moderate_texts(inputs);
        assert_eq!(builder.input_count(), 2);
        assert!(builder.has_multiple_inputs());
    }

    #[test]
    fn test_moderate_text_with_model_helper() {
        let builder = moderate_text_with_model("Test", "text-moderation-latest");
        assert_eq!(builder.first_input(), Some("Test"));
        assert_eq!(builder.model_ref(), Some("text-moderation-latest"));
    }

    #[test]
    fn test_moderate_messages_helper() {
        let messages = ["Hello", "World"];
        let builder = moderate_messages(&messages);
        assert_eq!(builder.input_count(), 2);
        assert_eq!(builder.all_inputs(), vec!["Hello", "World"]);
    }

    #[test]
    fn test_likely_flagged_helper() {
        assert!(likely_flagged("This contains hate speech"));
        assert!(likely_flagged("Violence is not good"));
        assert!(likely_flagged("This is harmful content"));
        assert!(!likely_flagged("This is normal content"));
        assert!(!likely_flagged("Hello, how are you?"));
    }

    #[test]
    fn test_moderation_input_variants() {
        let single = ModerationInput::Text("single".to_string());
        let multiple = ModerationInput::TextArray(vec!["one".to_string(), "two".to_string()]);

        match single {
            ModerationInput::Text(text) => assert_eq!(text, "single"),
            ModerationInput::TextArray(_) => panic!("Expected Text variant"),
        }

        match multiple {
            ModerationInput::TextArray(texts) => assert_eq!(texts.len(), 2),
            ModerationInput::Text(_) => panic!("Expected TextArray variant"),
        }
    }
}
