//! Fine-tuning API builders.
//!
//! This module provides ergonomic builders for `OpenAI` Fine-tuning API operations,
//! including creating fine-tuning jobs, monitoring progress, and managing models.
//!
//! Fine-tuning allows you to customize models on your specific training data
//! to improve performance for your particular use case.

use std::collections::HashMap;

/// Builder for creating fine-tuning jobs.
///
/// Fine-tuning jobs train models on your specific data to improve performance
/// for your particular use case.
#[derive(Debug, Clone)]
pub struct FineTuningJobBuilder {
    model: String,
    training_file: String,
    validation_file: Option<String>,
    hyperparameters: FineTuningHyperparameters,
    suffix: Option<String>,
    integrations: Vec<FineTuningIntegration>,
}

/// Hyperparameters for fine-tuning jobs.
#[derive(Debug, Clone, Default)]
pub struct FineTuningHyperparameters {
    /// Number of epochs to train for
    pub n_epochs: Option<i32>,
    /// Batch size for training
    pub batch_size: Option<i32>,
    /// Learning rate multiplier
    pub learning_rate_multiplier: Option<f64>,
}

/// Integration for fine-tuning jobs (e.g., Weights & Biases).
#[derive(Debug, Clone)]
pub struct FineTuningIntegration {
    /// Type of integration
    pub integration_type: String,
    /// Integration settings
    pub settings: HashMap<String, String>,
}

impl FineTuningJobBuilder {
    /// Create a new fine-tuning job builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openai_ergonomic::builders::fine_tuning::FineTuningJobBuilder;
    ///
    /// let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training-data");
    /// ```
    #[must_use]
    pub fn new(model: impl Into<String>, training_file: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            training_file: training_file.into(),
            validation_file: None,
            hyperparameters: FineTuningHyperparameters::default(),
            suffix: None,
            integrations: Vec::new(),
        }
    }

    /// Set the validation file for the fine-tuning job.
    #[must_use]
    pub fn validation_file(mut self, file_id: impl Into<String>) -> Self {
        self.validation_file = Some(file_id.into());
        self
    }

    /// Set the number of epochs to train for.
    #[must_use]
    pub fn epochs(mut self, epochs: i32) -> Self {
        self.hyperparameters.n_epochs = Some(epochs);
        self
    }

    /// Set the batch size for training.
    #[must_use]
    pub fn batch_size(mut self, batch_size: i32) -> Self {
        self.hyperparameters.batch_size = Some(batch_size);
        self
    }

    /// Set the learning rate multiplier.
    #[must_use]
    pub fn learning_rate_multiplier(mut self, multiplier: f64) -> Self {
        self.hyperparameters.learning_rate_multiplier = Some(multiplier);
        self
    }

    /// Set a suffix for the fine-tuned model name.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Add a Weights & Biases integration.
    #[must_use]
    pub fn with_wandb(mut self, project: impl Into<String>) -> Self {
        let mut settings = HashMap::new();
        settings.insert("project".to_string(), project.into());

        self.integrations.push(FineTuningIntegration {
            integration_type: "wandb".to_string(),
            settings,
        });
        self
    }

    /// Get the base model for this fine-tuning job.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the training file ID.
    #[must_use]
    pub fn training_file(&self) -> &str {
        &self.training_file
    }

    /// Get the validation file ID.
    #[must_use]
    pub fn validation_file_ref(&self) -> Option<&str> {
        self.validation_file.as_deref()
    }

    /// Get the hyperparameters.
    #[must_use]
    pub fn hyperparameters(&self) -> &FineTuningHyperparameters {
        &self.hyperparameters
    }

    /// Get the model suffix.
    #[must_use]
    pub fn suffix_ref(&self) -> Option<&str> {
        self.suffix.as_deref()
    }

    /// Get the integrations.
    #[must_use]
    pub fn integrations(&self) -> &[FineTuningIntegration] {
        &self.integrations
    }
}

/// Builder for listing fine-tuning jobs.
#[derive(Debug, Clone, Default)]
pub struct FineTuningJobListBuilder {
    after: Option<String>,
    limit: Option<i32>,
}

impl FineTuningJobListBuilder {
    /// Create a new fine-tuning job list builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cursor for pagination.
    #[must_use]
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    /// Set the maximum number of jobs to return.
    #[must_use]
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Get the pagination cursor.
    #[must_use]
    pub fn after_ref(&self) -> Option<&str> {
        self.after.as_deref()
    }

    /// Get the limit.
    #[must_use]
    pub fn limit_ref(&self) -> Option<i32> {
        self.limit
    }
}

/// Builder for retrieving fine-tuning job details.
#[derive(Debug, Clone)]
pub struct FineTuningJobRetrievalBuilder {
    job_id: String,
}

impl FineTuningJobRetrievalBuilder {
    /// Create a new fine-tuning job retrieval builder.
    #[must_use]
    pub fn new(job_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
        }
    }

    /// Get the job ID.
    #[must_use]
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
}

/// Builder for cancelling fine-tuning jobs.
#[derive(Debug, Clone)]
pub struct FineTuningJobCancelBuilder {
    job_id: String,
}

impl FineTuningJobCancelBuilder {
    /// Create a new fine-tuning job cancel builder.
    #[must_use]
    pub fn new(job_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
        }
    }

    /// Get the job ID.
    #[must_use]
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
}

/// Helper function to create a basic fine-tuning job.
#[must_use]
pub fn fine_tune_model(
    base_model: impl Into<String>,
    training_file: impl Into<String>,
) -> FineTuningJobBuilder {
    FineTuningJobBuilder::new(base_model, training_file)
}

/// Helper function to fine-tune with validation data.
#[must_use]
pub fn fine_tune_with_validation(
    base_model: impl Into<String>,
    training_file: impl Into<String>,
    validation_file: impl Into<String>,
) -> FineTuningJobBuilder {
    FineTuningJobBuilder::new(base_model, training_file).validation_file(validation_file)
}

/// Helper function to create a fine-tuning job with custom hyperparameters.
#[must_use]
pub fn fine_tune_with_params(
    base_model: impl Into<String>,
    training_file: impl Into<String>,
    epochs: i32,
    learning_rate: f64,
) -> FineTuningJobBuilder {
    FineTuningJobBuilder::new(base_model, training_file)
        .epochs(epochs)
        .learning_rate_multiplier(learning_rate)
}

/// Helper function to list fine-tuning jobs.
#[must_use]
pub fn list_fine_tuning_jobs() -> FineTuningJobListBuilder {
    FineTuningJobListBuilder::new()
}

/// Helper function to retrieve a specific fine-tuning job.
#[must_use]
pub fn get_fine_tuning_job(job_id: impl Into<String>) -> FineTuningJobRetrievalBuilder {
    FineTuningJobRetrievalBuilder::new(job_id)
}

/// Helper function to cancel a fine-tuning job.
#[must_use]
pub fn cancel_fine_tuning_job(job_id: impl Into<String>) -> FineTuningJobCancelBuilder {
    FineTuningJobCancelBuilder::new(job_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fine_tuning_job_builder_new() {
        let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training");

        assert_eq!(builder.model(), "gpt-3.5-turbo");
        assert_eq!(builder.training_file(), "file-training");
        assert!(builder.validation_file_ref().is_none());
        assert!(builder.suffix_ref().is_none());
        assert!(builder.integrations().is_empty());
    }

    #[test]
    fn test_fine_tuning_job_builder_with_validation() {
        let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
            .validation_file("file-validation");

        assert_eq!(builder.validation_file_ref(), Some("file-validation"));
    }

    #[test]
    fn test_fine_tuning_job_builder_with_hyperparameters() {
        let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
            .epochs(3)
            .batch_size(16)
            .learning_rate_multiplier(0.1);

        assert_eq!(builder.hyperparameters().n_epochs, Some(3));
        assert_eq!(builder.hyperparameters().batch_size, Some(16));
        assert_eq!(
            builder.hyperparameters().learning_rate_multiplier,
            Some(0.1)
        );
    }

    #[test]
    fn test_fine_tuning_job_builder_with_suffix() {
        let builder =
            FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training").suffix("my-model-v1");

        assert_eq!(builder.suffix_ref(), Some("my-model-v1"));
    }

    #[test]
    fn test_fine_tuning_job_builder_with_wandb() {
        let builder =
            FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training").with_wandb("my-project");

        assert_eq!(builder.integrations().len(), 1);
        assert_eq!(builder.integrations()[0].integration_type, "wandb");
        assert_eq!(
            builder.integrations()[0].settings.get("project"),
            Some(&"my-project".to_string())
        );
    }

    #[test]
    fn test_fine_tuning_job_list_builder() {
        let builder = FineTuningJobListBuilder::new().after("job-123").limit(10);

        assert_eq!(builder.after_ref(), Some("job-123"));
        assert_eq!(builder.limit_ref(), Some(10));
    }

    #[test]
    fn test_fine_tuning_job_retrieval_builder() {
        let builder = FineTuningJobRetrievalBuilder::new("job-456");
        assert_eq!(builder.job_id(), "job-456");
    }

    #[test]
    fn test_fine_tuning_job_cancel_builder() {
        let builder = FineTuningJobCancelBuilder::new("job-789");
        assert_eq!(builder.job_id(), "job-789");
    }

    #[test]
    fn test_fine_tune_model_helper() {
        let builder = fine_tune_model("gpt-3.5-turbo", "file-training");
        assert_eq!(builder.model(), "gpt-3.5-turbo");
        assert_eq!(builder.training_file(), "file-training");
    }

    #[test]
    fn test_fine_tune_with_validation_helper() {
        let builder =
            fine_tune_with_validation("gpt-3.5-turbo", "file-training", "file-validation");
        assert_eq!(builder.validation_file_ref(), Some("file-validation"));
    }

    #[test]
    fn test_fine_tune_with_params_helper() {
        let builder = fine_tune_with_params("gpt-3.5-turbo", "file-training", 5, 0.2);
        assert_eq!(builder.hyperparameters().n_epochs, Some(5));
        assert_eq!(
            builder.hyperparameters().learning_rate_multiplier,
            Some(0.2)
        );
    }

    #[test]
    fn test_list_fine_tuning_jobs_helper() {
        let builder = list_fine_tuning_jobs();
        assert!(builder.after_ref().is_none());
        assert!(builder.limit_ref().is_none());
    }

    #[test]
    fn test_get_fine_tuning_job_helper() {
        let builder = get_fine_tuning_job("job-123");
        assert_eq!(builder.job_id(), "job-123");
    }

    #[test]
    fn test_cancel_fine_tuning_job_helper() {
        let builder = cancel_fine_tuning_job("job-456");
        assert_eq!(builder.job_id(), "job-456");
    }

    #[test]
    fn test_fine_tuning_hyperparameters_default() {
        let params = FineTuningHyperparameters::default();
        assert!(params.n_epochs.is_none());
        assert!(params.batch_size.is_none());
        assert!(params.learning_rate_multiplier.is_none());
    }
}
