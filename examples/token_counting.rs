//! Token counting, estimation, and budget management for `OpenAI` API.
#![allow(dead_code)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unused_self)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::eq_op)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::use_self)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::option_if_let_else)]
//!
//! This example demonstrates comprehensive token management including:
//! - Accurate token counting for different models and encodings
//! - Token estimation before API calls to predict costs
//! - Budget management and cost control mechanisms
//! - Token optimization strategies for efficient API usage
//! - Real-time monitoring and alerting for token consumption
//! - Historical analysis and trend tracking
//! - Token-aware request batching and optimization
//! - Cost forecasting and budget planning
//!
//! Token management is crucial for:
//! - Controlling costs in production AI applications
//! - Preventing unexpected billing spikes
//! - Optimizing model selection based on cost/performance
//! - Planning capacity for high-volume applications
//! - Monitoring usage patterns and trends
//!
//! Run with: `cargo run --example token_counting`

use openai_ergonomic::{Client, Config, Error, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Token counting and estimation utilities
#[derive(Debug, Clone)]
struct TokenCounter {
    /// Character-to-token ratios for different languages/content types
    encoding_ratios: HashMap<String, f64>,
    /// Model-specific token limits
    model_limits: HashMap<String, TokenLimits>,
    /// Model pricing information
    model_pricing: HashMap<String, ModelPricing>,
}

/// Token limits for different models
#[derive(Debug, Clone)]
struct TokenLimits {
    /// Maximum context length (input + output)
    max_context_length: i32,
    /// Maximum output tokens
    max_output_tokens: i32,
    /// Recommended safe input limit (leaving room for output)
    safe_input_limit: i32,
}

/// Pricing information for models
#[derive(Debug, Clone)]
struct ModelPricing {
    /// Cost per 1K input tokens
    input_cost_per_1k: f64,
    /// Cost per 1K output tokens
    output_cost_per_1k: f64,
    /// Base cost per request (if any)
    base_cost: f64,
}

impl TokenCounter {
    /// Create a new token counter with default configurations
    fn new() -> Self {
        let mut encoding_ratios = HashMap::new();
        encoding_ratios.insert("english".to_string(), 0.25); // ~4 chars per token
        encoding_ratios.insert("code".to_string(), 0.33); // ~3 chars per token
        encoding_ratios.insert("multilingual".to_string(), 0.2); // ~5 chars per token
        encoding_ratios.insert("json".to_string(), 0.5); // ~2 chars per token

        let mut model_limits = HashMap::new();
        model_limits.insert(
            "gpt-4".to_string(),
            TokenLimits {
                max_context_length: 8192,
                max_output_tokens: 4096,
                safe_input_limit: 6000,
            },
        );
        model_limits.insert(
            "gpt-4-32k".to_string(),
            TokenLimits {
                max_context_length: 32768,
                max_output_tokens: 4096,
                safe_input_limit: 28000,
            },
        );
        model_limits.insert(
            "gpt-3.5-turbo".to_string(),
            TokenLimits {
                max_context_length: 4096,
                max_output_tokens: 4096,
                safe_input_limit: 3000,
            },
        );
        model_limits.insert(
            "gpt-3.5-turbo-16k".to_string(),
            TokenLimits {
                max_context_length: 16384,
                max_output_tokens: 4096,
                safe_input_limit: 12000,
            },
        );

        let mut model_pricing = HashMap::new();
        model_pricing.insert(
            "gpt-4".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.03,
                output_cost_per_1k: 0.06,
                base_cost: 0.0,
            },
        );
        model_pricing.insert(
            "gpt-4-32k".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.06,
                output_cost_per_1k: 0.12,
                base_cost: 0.0,
            },
        );
        model_pricing.insert(
            "gpt-3.5-turbo".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.0015,
                output_cost_per_1k: 0.002,
                base_cost: 0.0,
            },
        );
        model_pricing.insert(
            "gpt-3.5-turbo-16k".to_string(),
            ModelPricing {
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.004,
                base_cost: 0.0,
            },
        );

        Self {
            encoding_ratios,
            model_limits,
            model_pricing,
        }
    }

    /// Estimate tokens for text content
    fn estimate_tokens(&self, text: &str, content_type: &str) -> i32 {
        let ratio = self.encoding_ratios.get(content_type).unwrap_or(&0.25);
        (text.len() as f64 * ratio).ceil() as i32
    }

    /// Estimate tokens for a chat completion request
    fn estimate_chat_tokens(&self, messages: &[ChatMessage], model: &str) -> TokenEstimate {
        let mut total_tokens = 0;

        // Add tokens for each message
        for message in messages {
            // Message overhead (role, formatting, etc.)
            total_tokens += 4;

            // Content tokens
            let content_type = if message.role == "system" {
                "english"
            } else {
                "english"
            };
            total_tokens += self.estimate_tokens(&message.content, content_type);
        }

        // Add overhead for the completion request
        total_tokens += 2;

        // Get model limits for validation
        let limits = self
            .model_limits
            .get(model)
            .cloned()
            .unwrap_or(TokenLimits {
                max_context_length: 4096,
                max_output_tokens: 1000,
                safe_input_limit: 3000,
            });

        TokenEstimate {
            estimated_input_tokens: total_tokens,
            max_output_tokens: limits.max_output_tokens,
            total_estimated_tokens: total_tokens + limits.max_output_tokens,
            exceeds_context_limit: total_tokens > limits.max_context_length,
            exceeds_safe_limit: total_tokens > limits.safe_input_limit,
            model_limits: limits,
        }
    }

    /// Calculate cost estimate for a request
    fn estimate_cost(&self, estimate: &TokenEstimate, model: &str) -> CostEstimate {
        let pricing = self
            .model_pricing
            .get(model)
            .cloned()
            .unwrap_or(ModelPricing {
                input_cost_per_1k: 0.002,
                output_cost_per_1k: 0.002,
                base_cost: 0.0,
            });

        let input_cost =
            (estimate.estimated_input_tokens as f64 / 1000.0) * pricing.input_cost_per_1k;
        let max_output_cost =
            (estimate.max_output_tokens as f64 / 1000.0) * pricing.output_cost_per_1k;

        CostEstimate {
            estimated_input_cost: input_cost,
            max_output_cost,
            total_max_cost: input_cost + max_output_cost + pricing.base_cost,
            pricing_info: pricing,
        }
    }

    /// Optimize messages to fit within token limits
    fn optimize_messages(
        &self,
        messages: &[ChatMessage],
        model: &str,
        target_tokens: i32,
    ) -> Vec<ChatMessage> {
        let mut optimized = messages.to_vec();
        let mut current_estimate = self.estimate_chat_tokens(&optimized, model);

        // If we're already under the limit, return as-is
        if current_estimate.estimated_input_tokens <= target_tokens {
            return optimized;
        }

        info!(
            "Optimizing messages: current {} tokens, target {} tokens",
            current_estimate.estimated_input_tokens, target_tokens
        );

        // Strategy 1: Truncate user messages from the beginning (keep recent context)
        while current_estimate.estimated_input_tokens > target_tokens && optimized.len() > 1 {
            // Find the oldest user message to remove
            if let Some(pos) = optimized.iter().position(|msg| msg.role == "user") {
                if pos > 0 {
                    // Don't remove system messages
                    optimized.remove(pos);
                    current_estimate = self.estimate_chat_tokens(&optimized, model);
                    debug!(
                        "Removed message, now {} tokens",
                        current_estimate.estimated_input_tokens
                    );
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Strategy 2: Truncate long messages
        if current_estimate.estimated_input_tokens > target_tokens {
            for message in &mut optimized {
                if message.role != "system" && message.content.len() > 500 {
                    let max_chars = (target_tokens as f64 * 4.0) as usize; // Rough conversion
                    if message.content.len() > max_chars {
                        message.content =
                            format!("{}...", &message.content[..max_chars.saturating_sub(3)]);
                        debug!(
                            "Truncated long message to {} characters",
                            message.content.len()
                        );
                    }
                }
            }
            current_estimate = self.estimate_chat_tokens(&optimized, model);
        }

        info!(
            "Optimization complete: {} tokens (saved {})",
            current_estimate.estimated_input_tokens,
            current_estimate.estimated_input_tokens - current_estimate.estimated_input_tokens
        );

        optimized
    }

    /// Get the most cost-effective model for a given request
    fn recommend_model(
        &self,
        messages: &[ChatMessage],
        quality_tier: QualityTier,
    ) -> ModelRecommendation {
        let candidates = match quality_tier {
            QualityTier::Budget => vec!["gpt-3.5-turbo", "gpt-3.5-turbo-16k"],
            QualityTier::Balanced => vec!["gpt-3.5-turbo", "gpt-3.5-turbo-16k", "gpt-4"],
            QualityTier::Premium => vec!["gpt-4", "gpt-4-32k"],
        };

        let mut best_option = None;
        let mut best_cost = f64::INFINITY;

        for model in candidates {
            let estimate = self.estimate_chat_tokens(messages, model);
            if !estimate.exceeds_context_limit {
                let cost_estimate = self.estimate_cost(&estimate, model);
                if cost_estimate.total_max_cost < best_cost {
                    best_cost = cost_estimate.total_max_cost;
                    best_option = Some(ModelRecommendation {
                        model: model.to_string(),
                        estimated_cost: cost_estimate.total_max_cost,
                        token_estimate: estimate,
                        cost_details: cost_estimate,
                        reason: format!("Most cost-effective for {} tier", quality_tier.as_str()),
                    });
                }
            }
        }

        best_option.unwrap_or_else(|| {
            // Fallback to the largest model that can handle the request
            let fallback_model = "gpt-4-32k";
            let estimate = self.estimate_chat_tokens(messages, fallback_model);
            let cost_estimate = self.estimate_cost(&estimate, fallback_model);

            ModelRecommendation {
                model: fallback_model.to_string(),
                estimated_cost: cost_estimate.total_max_cost,
                token_estimate: estimate,
                cost_details: cost_estimate,
                reason: "Fallback - requires large context window".to_string(),
            }
        })
    }
}

/// Token usage estimate for a request
#[derive(Debug, Clone)]
struct TokenEstimate {
    estimated_input_tokens: i32,
    max_output_tokens: i32,
    total_estimated_tokens: i32,
    exceeds_context_limit: bool,
    exceeds_safe_limit: bool,
    model_limits: TokenLimits,
}

/// Cost estimate for a request
#[derive(Debug, Clone)]
struct CostEstimate {
    estimated_input_cost: f64,
    max_output_cost: f64,
    total_max_cost: f64,
    pricing_info: ModelPricing,
}

/// Quality tier for model selection
#[derive(Debug, Clone)]
enum QualityTier {
    Budget,
    Balanced,
    Premium,
}

impl QualityTier {
    fn as_str(&self) -> &str {
        match self {
            QualityTier::Budget => "budget",
            QualityTier::Balanced => "balanced",
            QualityTier::Premium => "premium",
        }
    }
}

/// Model recommendation with cost analysis
#[derive(Debug, Clone)]
struct ModelRecommendation {
    model: String,
    estimated_cost: f64,
    token_estimate: TokenEstimate,
    cost_details: CostEstimate,
    reason: String,
}

/// Budget manager for tracking and controlling costs
#[derive(Debug)]
struct BudgetManager {
    /// Daily budget limit in USD
    daily_budget: f64,
    /// Monthly budget limit in USD
    monthly_budget: f64,
    /// Current day spending
    daily_spending: Arc<Mutex<f64>>,
    /// Current month spending
    monthly_spending: Arc<Mutex<f64>>,
    /// Spending history
    spending_history: Arc<Mutex<Vec<SpendingRecord>>>,
    /// Alert thresholds
    alert_thresholds: AlertThresholds,
}

/// Alert thresholds for budget monitoring
#[derive(Debug, Clone)]
struct AlertThresholds {
    /// Percentage of daily budget for warning
    daily_warning_percent: f64,
    /// Percentage of daily budget for critical alert
    daily_critical_percent: f64,
    /// Percentage of monthly budget for warning
    monthly_warning_percent: f64,
    /// Percentage of monthly budget for critical alert
    monthly_critical_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            daily_warning_percent: 80.0,
            daily_critical_percent: 95.0,
            monthly_warning_percent: 80.0,
            monthly_critical_percent: 95.0,
        }
    }
}

/// Record of spending for analytics
#[derive(Debug, Clone)]
struct SpendingRecord {
    timestamp: u64,
    model: String,
    input_tokens: i32,
    output_tokens: i32,
    cost: f64,
    request_type: String,
    user_id: Option<String>,
}

impl BudgetManager {
    /// Create a new budget manager
    fn new(daily_budget: f64, monthly_budget: f64) -> Self {
        Self {
            daily_budget,
            monthly_budget,
            daily_spending: Arc::new(Mutex::new(0.0)),
            monthly_spending: Arc::new(Mutex::new(0.0)),
            spending_history: Arc::new(Mutex::new(Vec::new())),
            alert_thresholds: AlertThresholds::default(),
        }
    }

    /// Check if a request is within budget
    fn check_budget(&self, estimated_cost: f64) -> BudgetCheckResult {
        let daily_spent = *self.daily_spending.lock().unwrap();
        let monthly_spent = *self.monthly_spending.lock().unwrap();

        let daily_after = daily_spent + estimated_cost;
        let monthly_after = monthly_spent + estimated_cost;

        let daily_percent = (daily_after / self.daily_budget) * 100.0;
        let monthly_percent = (monthly_after / self.monthly_budget) * 100.0;

        // Check for budget violations
        if daily_after > self.daily_budget {
            return BudgetCheckResult {
                approved: false,
                reason: format!(
                    "Would exceed daily budget: ${:.4} > ${:.2}",
                    daily_after, self.daily_budget
                ),
                current_daily_usage: daily_percent,
                current_monthly_usage: monthly_percent,
                alerts: vec![BudgetAlert::DailyExceeded],
            };
        }

        if monthly_after > self.monthly_budget {
            return BudgetCheckResult {
                approved: false,
                reason: format!(
                    "Would exceed monthly budget: ${:.4} > ${:.2}",
                    monthly_after, self.monthly_budget
                ),
                current_daily_usage: daily_percent,
                current_monthly_usage: monthly_percent,
                alerts: vec![BudgetAlert::MonthlyExceeded],
            };
        }

        // Check for alerts
        let mut alerts = Vec::new();

        if daily_percent >= self.alert_thresholds.daily_critical_percent {
            alerts.push(BudgetAlert::DailyCritical);
        } else if daily_percent >= self.alert_thresholds.daily_warning_percent {
            alerts.push(BudgetAlert::DailyWarning);
        }

        if monthly_percent >= self.alert_thresholds.monthly_critical_percent {
            alerts.push(BudgetAlert::MonthlyCritical);
        } else if monthly_percent >= self.alert_thresholds.monthly_warning_percent {
            alerts.push(BudgetAlert::MonthlyWarning);
        }

        BudgetCheckResult {
            approved: true,
            reason: "Within budget limits".to_string(),
            current_daily_usage: daily_percent,
            current_monthly_usage: monthly_percent,
            alerts,
        }
    }

    /// Record actual spending
    fn record_spending(&self, record: SpendingRecord) {
        let mut daily_spending = self.daily_spending.lock().unwrap();
        let mut monthly_spending = self.monthly_spending.lock().unwrap();
        let mut history = self.spending_history.lock().unwrap();

        *daily_spending += record.cost;
        *monthly_spending += record.cost;
        history.push(record);

        // Keep only recent history (last 1000 records)
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    /// Get budget status summary
    fn get_budget_status(&self) -> BudgetStatus {
        let daily_spent = *self.daily_spending.lock().unwrap();
        let monthly_spent = *self.monthly_spending.lock().unwrap();
        let history = self.spending_history.lock().unwrap();

        let total_requests = history.len() as u64;
        let total_tokens: i32 = history
            .iter()
            .map(|r| r.input_tokens + r.output_tokens)
            .sum();

        let avg_cost_per_request = if total_requests > 0 {
            monthly_spent / total_requests as f64
        } else {
            0.0
        };

        BudgetStatus {
            daily_budget: self.daily_budget,
            monthly_budget: self.monthly_budget,
            daily_spent,
            monthly_spent,
            daily_remaining: self.daily_budget - daily_spent,
            monthly_remaining: self.monthly_budget - monthly_spent,
            daily_usage_percent: (daily_spent / self.daily_budget) * 100.0,
            monthly_usage_percent: (monthly_spent / self.monthly_budget) * 100.0,
            total_requests,
            total_tokens,
            average_cost_per_request: avg_cost_per_request,
        }
    }

    /// Reset daily spending (should be called daily)
    fn reset_daily_spending(&self) {
        *self.daily_spending.lock().unwrap() = 0.0;
        info!("Daily spending reset");
    }

    /// Reset monthly spending (should be called monthly)
    fn reset_monthly_spending(&self) {
        *self.monthly_spending.lock().unwrap() = 0.0;
        info!("Monthly spending reset");
    }
}

/// Result of budget check
#[derive(Debug, Clone)]
struct BudgetCheckResult {
    approved: bool,
    reason: String,
    current_daily_usage: f64,
    current_monthly_usage: f64,
    alerts: Vec<BudgetAlert>,
}

/// Budget alert types
#[derive(Debug, Clone)]
enum BudgetAlert {
    DailyWarning,
    DailyCritical,
    DailyExceeded,
    MonthlyWarning,
    MonthlyCritical,
    MonthlyExceeded,
}

impl BudgetAlert {
    fn message(&self) -> &str {
        match self {
            BudgetAlert::DailyWarning => "Daily budget usage approaching limit",
            BudgetAlert::DailyCritical => "Daily budget usage critical",
            BudgetAlert::DailyExceeded => "Daily budget exceeded",
            BudgetAlert::MonthlyWarning => "Monthly budget usage approaching limit",
            BudgetAlert::MonthlyCritical => "Monthly budget usage critical",
            BudgetAlert::MonthlyExceeded => "Monthly budget exceeded",
        }
    }
}

/// Budget status summary
#[derive(Debug, Clone)]
struct BudgetStatus {
    daily_budget: f64,
    monthly_budget: f64,
    daily_spent: f64,
    monthly_spent: f64,
    daily_remaining: f64,
    monthly_remaining: f64,
    daily_usage_percent: f64,
    monthly_usage_percent: f64,
    total_requests: u64,
    total_tokens: i32,
    average_cost_per_request: f64,
}

impl BudgetStatus {
    fn print_status(&self) {
        info!("=== Budget Status ===");
        info!(
            "Daily: ${:.4} / ${:.2} ({:.1}% used, ${:.4} remaining)",
            self.daily_spent, self.daily_budget, self.daily_usage_percent, self.daily_remaining
        );
        info!(
            "Monthly: ${:.4} / ${:.2} ({:.1}% used, ${:.4} remaining)",
            self.monthly_spent,
            self.monthly_budget,
            self.monthly_usage_percent,
            self.monthly_remaining
        );
        info!("Total requests: {}", self.total_requests);
        info!("Total tokens: {}", self.total_tokens);
        info!(
            "Average cost per request: ${:.6}",
            self.average_cost_per_request
        );
    }
}

/// Chat message structure
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

/// Token-aware client that integrates counting and budgeting
#[derive(Debug)]
struct TokenAwareClient {
    client: Client,
    token_counter: TokenCounter,
    budget_manager: Arc<BudgetManager>,
}

impl TokenAwareClient {
    /// Create a new token-aware client
    fn new(client: Client, daily_budget: f64, monthly_budget: f64) -> Self {
        Self {
            client,
            token_counter: TokenCounter::new(),
            budget_manager: Arc::new(BudgetManager::new(daily_budget, monthly_budget)),
        }
    }

    /// Send a chat completion with token and budget checking
    async fn chat_completion_with_budget(
        &self,
        messages: &[ChatMessage],
        model: &str,
        max_tokens: Option<i32>,
        user_id: Option<String>,
    ) -> Result<String> {
        // Estimate tokens and cost
        let token_estimate = self.token_counter.estimate_chat_tokens(messages, model);
        let cost_estimate = self.token_counter.estimate_cost(&token_estimate, model);

        info!(
            "Token estimate: {} input, {} max output, ${:.4} max cost",
            token_estimate.estimated_input_tokens,
            token_estimate.max_output_tokens,
            cost_estimate.total_max_cost
        );

        // Check token limits
        if token_estimate.exceeds_context_limit {
            return Err(Error::InvalidRequest(format!(
                "Request exceeds context limit: {} > {}",
                token_estimate.estimated_input_tokens,
                token_estimate.model_limits.max_context_length
            )));
        }

        if token_estimate.exceeds_safe_limit {
            warn!(
                "Request exceeds safe input limit: {} > {}",
                token_estimate.estimated_input_tokens, token_estimate.model_limits.safe_input_limit
            );
        }

        // Check budget
        let budget_check = self
            .budget_manager
            .check_budget(cost_estimate.total_max_cost);

        if !budget_check.approved {
            return Err(Error::InvalidRequest(format!(
                "Budget check failed: {}",
                budget_check.reason
            )));
        }

        // Handle alerts
        for alert in &budget_check.alerts {
            match alert {
                BudgetAlert::DailyWarning | BudgetAlert::MonthlyWarning => {
                    warn!("{}", alert.message());
                }
                BudgetAlert::DailyCritical | BudgetAlert::MonthlyCritical => {
                    error!("{}", alert.message());
                }
                _ => {}
            }
        }

        // Simulate API call
        let response = self.simulate_api_call(messages, model, max_tokens).await?;

        // Calculate actual usage (simplified)
        let actual_output_tokens = self.token_counter.estimate_tokens(&response, "english");
        let actual_cost = (token_estimate.estimated_input_tokens as f64 / 1000.0)
            * cost_estimate.pricing_info.input_cost_per_1k
            + (actual_output_tokens as f64 / 1000.0)
                * cost_estimate.pricing_info.output_cost_per_1k;

        // Record spending
        let spending_record = SpendingRecord {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: model.to_string(),
            input_tokens: token_estimate.estimated_input_tokens,
            output_tokens: actual_output_tokens,
            cost: actual_cost,
            request_type: "chat_completion".to_string(),
            user_id,
        };

        self.budget_manager.record_spending(spending_record);

        info!(
            "Request completed: {} tokens used, ${:.6} actual cost",
            token_estimate.estimated_input_tokens + actual_output_tokens,
            actual_cost
        );

        Ok(response)
    }

    /// Get model recommendation for messages
    fn recommend_model(
        &self,
        messages: &[ChatMessage],
        quality_tier: QualityTier,
    ) -> ModelRecommendation {
        self.token_counter.recommend_model(messages, quality_tier)
    }

    /// Optimize messages for token efficiency
    fn optimize_for_budget(
        &self,
        messages: &[ChatMessage],
        model: &str,
        target_cost: f64,
    ) -> Vec<ChatMessage> {
        // Calculate target tokens based on cost
        let pricing = self.token_counter.model_pricing.get(model).unwrap();
        let target_tokens = ((target_cost / pricing.input_cost_per_1k) * 1000.0) as i32;

        self.token_counter
            .optimize_messages(messages, model, target_tokens)
    }

    /// Get budget status
    fn get_budget_status(&self) -> BudgetStatus {
        self.budget_manager.get_budget_status()
    }

    /// Simulate API call for demonstration
    async fn simulate_api_call(
        &self,
        messages: &[ChatMessage],
        model: &str,
        _max_tokens: Option<i32>,
    ) -> Result<String> {
        // Simulate processing time based on model
        let delay = match model {
            "gpt-4" | "gpt-4-32k" => Duration::from_millis(800),
            _ => Duration::from_millis(400),
        };

        tokio::time::sleep(delay).await;

        // Generate a response based on the last user message
        if let Some(last_message) = messages.iter().rev().find(|m| m.role == "user") {
            Ok(format!(
                "Simulated {} response to: {}",
                model,
                last_message.content.chars().take(50).collect::<String>()
            ))
        } else {
            Ok("Simulated response with no user input".to_string())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting token counting and budget management example");

    // Create client
    let config = Config::builder().api_key("test-api-key").build();
    let client = Client::new(config)?;

    // Example 1: Basic token counting and estimation
    info!("=== Example 1: Token Counting and Estimation ===");

    let token_counter = TokenCounter::new();

    let test_messages = vec![
        ChatMessage::system("You are a helpful assistant that provides detailed explanations."),
        ChatMessage::user("Explain the concept of machine learning in simple terms."),
        ChatMessage::assistant("Machine learning is a way for computers to learn patterns from data without being explicitly programmed for every scenario."),
        ChatMessage::user("Can you give me a practical example?"),
    ];

    for model in ["gpt-3.5-turbo", "gpt-4", "gpt-4-32k"] {
        let estimate = token_counter.estimate_chat_tokens(&test_messages, model);
        let cost_estimate = token_counter.estimate_cost(&estimate, model);

        info!("Model: {}", model);
        info!(
            "  Estimated input tokens: {}",
            estimate.estimated_input_tokens
        );
        info!("  Max output tokens: {}", estimate.max_output_tokens);
        info!(
            "  Total estimated tokens: {}",
            estimate.total_estimated_tokens
        );
        info!(
            "  Exceeds context limit: {}",
            estimate.exceeds_context_limit
        );
        info!("  Exceeds safe limit: {}", estimate.exceeds_safe_limit);
        info!("  Estimated cost: ${:.6}", cost_estimate.total_max_cost);
        info!("");
    }

    // Example 2: Model recommendations based on cost and quality
    info!("=== Example 2: Model Recommendations ===");

    for quality_tier in [
        QualityTier::Budget,
        QualityTier::Balanced,
        QualityTier::Premium,
    ] {
        let recommendation = token_counter.recommend_model(&test_messages, quality_tier.clone());
        info!("Quality tier: {}", quality_tier.as_str());
        info!("  Recommended model: {}", recommendation.model);
        info!("  Estimated cost: ${:.6}", recommendation.estimated_cost);
        info!("  Reason: {}", recommendation.reason);
        info!("");
    }

    // Example 3: Message optimization for token limits
    info!("=== Example 3: Message Optimization ===");

    let long_messages = vec![
        ChatMessage::system("You are an expert assistant with deep knowledge across many domains."),
        ChatMessage::user("Tell me everything you know about artificial intelligence, machine learning, deep learning, neural networks, natural language processing, computer vision, and how they all relate to each other. I want a comprehensive overview."),
        ChatMessage::assistant("Artificial intelligence is a broad field..."),
        ChatMessage::user("Now explain quantum computing and how it might affect AI in the future."),
        ChatMessage::user("What about the ethical implications of AI?"),
        ChatMessage::user("How do transformers work in detail?"),
    ];

    let original_estimate = token_counter.estimate_chat_tokens(&long_messages, "gpt-3.5-turbo");
    info!(
        "Original message tokens: {}",
        original_estimate.estimated_input_tokens
    );

    let optimized_messages = token_counter.optimize_messages(&long_messages, "gpt-3.5-turbo", 2000);
    let optimized_estimate =
        token_counter.estimate_chat_tokens(&optimized_messages, "gpt-3.5-turbo");
    info!(
        "Optimized message tokens: {}",
        optimized_estimate.estimated_input_tokens
    );
    info!(
        "Optimization saved: {} tokens",
        original_estimate.estimated_input_tokens - optimized_estimate.estimated_input_tokens
    );

    // Example 4: Budget management
    info!("\n=== Example 4: Budget Management ===");

    let token_aware_client = TokenAwareClient::new(client, 10.0, 100.0); // $10 daily, $100 monthly

    // Test budget status
    let initial_status = token_aware_client.get_budget_status();
    initial_status.print_status();

    // Make several requests to test budget tracking
    let test_requests = vec![
        ("What is the weather like?", "gpt-3.5-turbo"),
        ("Explain quantum physics", "gpt-4"),
        ("Write a short story", "gpt-3.5-turbo"),
        ("Solve this math problem: 2x + 5 = 15", "gpt-3.5-turbo"),
    ];

    for (prompt, model) in test_requests {
        let messages = vec![ChatMessage::user(prompt)];

        match token_aware_client
            .chat_completion_with_budget(&messages, model, Some(150), Some("test_user".to_string()))
            .await
        {
            Ok(response) => {
                info!("Request successful: {}", response);
            }
            Err(e) => {
                error!("Request failed: {}", e);
            }
        }

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Check final budget status
    let final_status = token_aware_client.get_budget_status();
    info!("\nFinal budget status:");
    final_status.print_status();

    // Example 5: Cost optimization strategies
    info!("\n=== Example 5: Cost Optimization ===");

    let expensive_prompt = vec![
        ChatMessage::system("You are a comprehensive research assistant."),
        ChatMessage::user("I need a detailed analysis of the global economic impact of artificial intelligence across all major industries, including specific case studies, statistical data, future projections, and policy recommendations. Please provide a thorough report with citations and references."),
    ];

    // Get model recommendation for this expensive request
    let budget_recommendation =
        token_aware_client.recommend_model(&expensive_prompt, QualityTier::Budget);
    let balanced_recommendation =
        token_aware_client.recommend_model(&expensive_prompt, QualityTier::Balanced);

    info!("Expensive request analysis:");
    info!(
        "  Budget option: {} (${:.6})",
        budget_recommendation.model, budget_recommendation.estimated_cost
    );
    info!(
        "  Balanced option: {} (${:.6})",
        balanced_recommendation.model, balanced_recommendation.estimated_cost
    );

    // Optimize for a specific budget
    let optimized_for_budget =
        token_aware_client.optimize_for_budget(&expensive_prompt, "gpt-3.5-turbo", 0.05);
    let optimized_estimate =
        token_counter.estimate_chat_tokens(&optimized_for_budget, "gpt-3.5-turbo");
    let optimized_cost = token_counter.estimate_cost(&optimized_estimate, "gpt-3.5-turbo");

    info!("Optimized for $0.05 budget:");
    info!("  Tokens: {}", optimized_estimate.estimated_input_tokens);
    info!("  Estimated cost: ${:.6}", optimized_cost.total_max_cost);

    // Example 6: Real-time monitoring and alerts
    info!("\n=== Example 6: Budget Monitoring ===");

    // Simulate approaching budget limits
    let high_usage_client = TokenAwareClient::new(
        Client::new(Config::builder().api_key("test-api-key").build())?,
        1.0, // Low daily budget for demonstration
        10.0,
    );

    // Make expensive requests to trigger alerts
    let expensive_messages = vec![ChatMessage::user(
        "Generate a very long detailed response about the history of computing.",
    )];

    for i in 1..=5 {
        info!("Making expensive request {}/5", i);

        match high_usage_client
            .chat_completion_with_budget(
                &expensive_messages,
                "gpt-4", // Expensive model
                Some(500),
                Some(format!("user_{}", i)),
            )
            .await
        {
            Ok(response) => {
                info!(
                    "Request {} completed: {}",
                    i,
                    response.chars().take(100).collect::<String>()
                );
            }
            Err(e) => {
                warn!("Request {} blocked: {}", i, e);
                break;
            }
        }

        // Show budget status after each request
        let status = high_usage_client.get_budget_status();
        info!(
            "Budget after request {}: {:.1}% daily, {:.1}% monthly",
            i, status.daily_usage_percent, status.monthly_usage_percent
        );
    }

    // Example 7: Analytics and reporting
    info!("\n=== Example 7: Usage Analytics ===");

    let final_analytics = high_usage_client.get_budget_status();
    info!("=== Usage Analytics Summary ===");
    info!(
        "Total API requests made: {}",
        final_analytics.total_requests
    );
    info!("Total tokens processed: {}", final_analytics.total_tokens);
    info!(
        "Average tokens per request: {:.1}",
        final_analytics.total_tokens as f64 / final_analytics.total_requests.max(1) as f64
    );
    info!(
        "Average cost per request: ${:.6}",
        final_analytics.average_cost_per_request
    );
    info!(
        "Total spending: ${:.4}",
        final_analytics.daily_spent + final_analytics.monthly_spent
    );

    // Calculate efficiency metrics
    let tokens_per_dollar =
        final_analytics.total_tokens as f64 / (final_analytics.daily_spent + 0.001);
    info!("Tokens per dollar: {:.0}", tokens_per_dollar);

    info!("Token counting and budget management example completed successfully!");
    Ok(())
}
