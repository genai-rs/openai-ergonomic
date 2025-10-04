//! Usage API builders.
//!
//! Provides high-level builders for querying usage and cost data from the `OpenAI` API.
//! Supports filtering by date range, aggregation buckets, projects, users, API keys, and models.

/// Bucket width for aggregating usage data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketWidth {
    /// Aggregate data by day.
    Day,
    /// Aggregate data by hour.
    Hour,
}

impl BucketWidth {
    /// Convert to API string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "1d",
            Self::Hour => "1h",
        }
    }
}

impl std::fmt::Display for BucketWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Group by field for usage aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    /// Group by project ID.
    ProjectId,
    /// Group by user ID.
    UserId,
    /// Group by API key ID.
    ApiKeyId,
    /// Group by model.
    Model,
}

impl GroupBy {
    /// Convert to API string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectId => "project_id",
            Self::UserId => "user_id",
            Self::ApiKeyId => "api_key_id",
            Self::Model => "model",
        }
    }
}

impl std::fmt::Display for GroupBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Builder for querying usage data from the `OpenAI` API.
///
/// # Examples
///
/// ```rust
/// use openai_ergonomic::builders::usage::{UsageBuilder, BucketWidth};
///
/// let builder = UsageBuilder::new(1704067200, None) // Start time (Unix timestamp)
///     .bucket_width(BucketWidth::Day)
///     .limit(100);
/// ```
#[derive(Debug, Clone)]
pub struct UsageBuilder {
    start_time: i32,
    end_time: Option<i32>,
    bucket_width: Option<BucketWidth>,
    project_ids: Vec<String>,
    user_ids: Vec<String>,
    api_key_ids: Vec<String>,
    models: Vec<String>,
    group_by: Vec<GroupBy>,
    limit: Option<i32>,
    page: Option<String>,
}

impl UsageBuilder {
    /// Create a new usage builder with the specified start time.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Unix timestamp (in seconds) for the start of the query range
    /// * `end_time` - Optional Unix timestamp (in seconds) for the end of the query range
    #[must_use]
    pub fn new(start_time: i32, end_time: Option<i32>) -> Self {
        Self {
            start_time,
            end_time,
            bucket_width: None,
            project_ids: Vec::new(),
            user_ids: Vec::new(),
            api_key_ids: Vec::new(),
            models: Vec::new(),
            group_by: Vec::new(),
            limit: None,
            page: None,
        }
    }

    /// Set the bucket width for aggregation.
    #[must_use]
    pub fn bucket_width(mut self, width: BucketWidth) -> Self {
        self.bucket_width = Some(width);
        self
    }

    /// Filter by a single project ID.
    #[must_use]
    pub fn project_id(mut self, id: impl Into<String>) -> Self {
        self.project_ids.push(id.into());
        self
    }

    /// Filter by multiple project IDs.
    #[must_use]
    pub fn project_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.project_ids.extend(ids.into_iter().map(Into::into));
        self
    }

    /// Filter by a single user ID.
    #[must_use]
    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_ids.push(id.into());
        self
    }

    /// Filter by multiple user IDs.
    #[must_use]
    pub fn user_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.user_ids.extend(ids.into_iter().map(Into::into));
        self
    }

    /// Filter by a single API key ID.
    #[must_use]
    pub fn api_key_id(mut self, id: impl Into<String>) -> Self {
        self.api_key_ids.push(id.into());
        self
    }

    /// Filter by multiple API key IDs.
    #[must_use]
    pub fn api_key_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.api_key_ids.extend(ids.into_iter().map(Into::into));
        self
    }

    /// Filter by a single model.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.models.push(model.into());
        self
    }

    /// Filter by multiple models.
    #[must_use]
    pub fn models<I, S>(mut self, models: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.models.extend(models.into_iter().map(Into::into));
        self
    }

    /// Add a group by field.
    #[must_use]
    pub fn group_by(mut self, field: GroupBy) -> Self {
        self.group_by.push(field);
        self
    }

    /// Add multiple group by fields.
    #[must_use]
    pub fn group_by_fields<I>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = GroupBy>,
    {
        self.group_by.extend(fields);
        self
    }

    /// Set the maximum number of results to return.
    #[must_use]
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the pagination cursor.
    #[must_use]
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Get the start time.
    #[must_use]
    pub fn start_time(&self) -> i32 {
        self.start_time
    }

    /// Get the end time.
    #[must_use]
    pub fn end_time(&self) -> Option<i32> {
        self.end_time
    }

    /// Get the bucket width.
    #[must_use]
    pub fn bucket_width_ref(&self) -> Option<BucketWidth> {
        self.bucket_width
    }

    /// Get the project IDs.
    #[must_use]
    pub fn project_ids_ref(&self) -> &[String] {
        &self.project_ids
    }

    /// Get the user IDs.
    #[must_use]
    pub fn user_ids_ref(&self) -> &[String] {
        &self.user_ids
    }

    /// Get the API key IDs.
    #[must_use]
    pub fn api_key_ids_ref(&self) -> &[String] {
        &self.api_key_ids
    }

    /// Get the models.
    #[must_use]
    pub fn models_ref(&self) -> &[String] {
        &self.models
    }

    /// Get the group by fields.
    #[must_use]
    pub fn group_by_ref(&self) -> &[GroupBy] {
        &self.group_by
    }

    /// Get the limit.
    #[must_use]
    pub fn limit_ref(&self) -> Option<i32> {
        self.limit
    }

    /// Get the page cursor.
    #[must_use]
    pub fn page_ref(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// Convert project IDs to Option<Vec<String>>.
    #[must_use]
    pub fn project_ids_option(&self) -> Option<Vec<String>> {
        if self.project_ids.is_empty() {
            None
        } else {
            Some(self.project_ids.clone())
        }
    }

    /// Convert user IDs to Option<Vec<String>>.
    #[must_use]
    pub fn user_ids_option(&self) -> Option<Vec<String>> {
        if self.user_ids.is_empty() {
            None
        } else {
            Some(self.user_ids.clone())
        }
    }

    /// Convert API key IDs to Option<Vec<String>>.
    #[must_use]
    pub fn api_key_ids_option(&self) -> Option<Vec<String>> {
        if self.api_key_ids.is_empty() {
            None
        } else {
            Some(self.api_key_ids.clone())
        }
    }

    /// Convert models to Option<Vec<String>>.
    #[must_use]
    pub fn models_option(&self) -> Option<Vec<String>> {
        if self.models.is_empty() {
            None
        } else {
            Some(self.models.clone())
        }
    }

    /// Convert group by fields to Option<Vec<String>>.
    #[must_use]
    pub fn group_by_option(&self) -> Option<Vec<String>> {
        if self.group_by.is_empty() {
            None
        } else {
            Some(self.group_by.iter().map(ToString::to_string).collect())
        }
    }

    /// Get bucket width as Option<&str>.
    #[must_use]
    pub fn bucket_width_str(&self) -> Option<&str> {
        self.bucket_width.as_ref().map(BucketWidth::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_builder_basic() {
        let builder = UsageBuilder::new(1_704_067_200, None);
        assert_eq!(builder.start_time(), 1_704_067_200);
        assert_eq!(builder.end_time(), None);
    }

    #[test]
    fn test_usage_builder_with_end_time() {
        let builder = UsageBuilder::new(1_704_067_200, Some(1_704_153_600));
        assert_eq!(builder.start_time(), 1_704_067_200);
        assert_eq!(builder.end_time(), Some(1_704_153_600));
    }

    #[test]
    fn test_usage_builder_with_bucket_width() {
        let builder = UsageBuilder::new(1_704_067_200, None).bucket_width(BucketWidth::Day);
        assert_eq!(builder.bucket_width_ref(), Some(BucketWidth::Day));
        assert_eq!(builder.bucket_width_str(), Some("1d"));
    }

    #[test]
    fn test_usage_builder_with_filters() {
        let builder = UsageBuilder::new(1_704_067_200, None)
            .project_id("proj_123")
            .user_id("user_456")
            .model("gpt-4");

        assert_eq!(builder.project_ids_ref(), &["proj_123"]);
        assert_eq!(builder.user_ids_ref(), &["user_456"]);
        assert_eq!(builder.models_ref(), &["gpt-4"]);
    }

    #[test]
    fn test_usage_builder_with_multiple_filters() {
        let builder = UsageBuilder::new(1_704_067_200, None)
            .project_ids(vec!["proj_1", "proj_2"])
            .user_ids(vec!["user_1", "user_2"])
            .models(vec!["gpt-4", "gpt-3.5-turbo"]);

        assert_eq!(builder.project_ids_ref().len(), 2);
        assert_eq!(builder.user_ids_ref().len(), 2);
        assert_eq!(builder.models_ref().len(), 2);
    }

    #[test]
    fn test_usage_builder_with_group_by() {
        let builder = UsageBuilder::new(1_704_067_200, None)
            .group_by(GroupBy::ProjectId)
            .group_by(GroupBy::Model);

        assert_eq!(builder.group_by_ref().len(), 2);
        let group_by_strings = builder.group_by_option().unwrap();
        assert_eq!(group_by_strings, vec!["project_id", "model"]);
    }

    #[test]
    fn test_usage_builder_with_pagination() {
        let builder = UsageBuilder::new(1_704_067_200, None)
            .limit(50)
            .page("next_page_token");

        assert_eq!(builder.limit_ref(), Some(50));
        assert_eq!(builder.page_ref(), Some("next_page_token"));
    }

    #[test]
    fn test_bucket_width_display() {
        assert_eq!(BucketWidth::Day.to_string(), "1d");
        assert_eq!(BucketWidth::Hour.to_string(), "1h");
    }

    #[test]
    fn test_group_by_display() {
        assert_eq!(GroupBy::ProjectId.to_string(), "project_id");
        assert_eq!(GroupBy::UserId.to_string(), "user_id");
        assert_eq!(GroupBy::ApiKeyId.to_string(), "api_key_id");
        assert_eq!(GroupBy::Model.to_string(), "model");
    }

    #[test]
    fn test_empty_vectors_to_none() {
        let builder = UsageBuilder::new(1_704_067_200, None);
        assert!(builder.project_ids_option().is_none());
        assert!(builder.user_ids_option().is_none());
        assert!(builder.api_key_ids_option().is_none());
        assert!(builder.models_option().is_none());
        assert!(builder.group_by_option().is_none());
    }
}
