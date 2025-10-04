//! Integration tests for the Fine-tuning API.

use openai_ergonomic::builders::fine_tuning::{
    fine_tune_model, fine_tune_with_params, fine_tune_with_validation, FineTuningJobBuilder,
    FineTuningJobListBuilder,
};

#[test]
fn test_fine_tuning_job_builder_basic() {
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
fn test_fine_tuning_job_builder_complete() {
    let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
        .validation_file("file-validation")
        .epochs(5)
        .batch_size(32)
        .learning_rate_multiplier(0.05)
        .suffix("custom-model")
        .with_wandb("ml-project");

    assert_eq!(builder.model(), "gpt-3.5-turbo");
    assert_eq!(builder.training_file(), "file-training");
    assert_eq!(builder.validation_file_ref(), Some("file-validation"));
    assert_eq!(builder.suffix_ref(), Some("custom-model"));
    assert_eq!(builder.hyperparameters().n_epochs, Some(5));
    assert_eq!(builder.hyperparameters().batch_size, Some(32));
    assert_eq!(builder.hyperparameters().learning_rate_multiplier, Some(0.05));
    assert_eq!(builder.integrations().len(), 1);
}

#[test]
fn test_fine_tuning_job_list_builder() {
    let builder = FineTuningJobListBuilder::new().after("job-123").limit(10);

    assert_eq!(builder.after_ref(), Some("job-123"));
    assert_eq!(builder.limit_ref(), Some(10));
}

#[test]
fn test_fine_tuning_job_list_builder_default() {
    let builder = FineTuningJobListBuilder::new();

    assert!(builder.after_ref().is_none());
    assert!(builder.limit_ref().is_none());
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
fn test_hyperparameters_defaults() {
    let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training");

    assert!(builder.hyperparameters().n_epochs.is_none());
    assert!(builder.hyperparameters().batch_size.is_none());
    assert!(builder.hyperparameters().learning_rate_multiplier.is_none());
}

#[test]
fn test_fine_tuning_multiple_integrations() {
    // Currently only wandb is supported, but test the structure
    let builder = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
        .with_wandb("project-1");

    assert_eq!(builder.integrations().len(), 1);

    // If we add another integration in the future, we can test:
    // .with_wandb("project-2")
    // assert_eq!(builder.integrations().len(), 2);
}

#[test]
fn test_learning_rate_bounds() {
    let builder1 = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
        .learning_rate_multiplier(0.01);
    assert_eq!(builder1.hyperparameters().learning_rate_multiplier, Some(0.01));

    let builder2 = FineTuningJobBuilder::new("gpt-3.5-turbo", "file-training")
        .learning_rate_multiplier(2.0);
    assert_eq!(builder2.hyperparameters().learning_rate_multiplier, Some(2.0));
}
