//! Demonstrates using strongly typed tools with the framework.
#![allow(missing_docs, unused_doc_comments)]

use openai_ergonomic::{
    strongly_typed_tool,
    tool_framework::{StronglyTypedToolAdapter, ToolRegistry},
    tool_schema,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CurrencyParams {
    amount_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct CurrencyResult {
    amount_eur: f64,
}

/// Tool that converts USD to EUR.
strongly_typed_tool!(
    ConvertCurrency,
    "convert_currency",
    "Convert USD to EUR",
    CurrencyParams,
    CurrencyResult,
    tool_schema!(
        amount_usd: "number", "Amount in USD", required: true,
    ),
    |params: CurrencyParams| async move {
        // Pretend exchange rate.
        Ok(CurrencyResult {
            amount_eur: params.amount_usd * 0.92,
        })
    }
);

#[tokio::main]
async fn main() -> openai_ergonomic::Result<()> {
    let registry = ToolRegistry::new().register(StronglyTypedToolAdapter::new(ConvertCurrency));

    let result = registry
        .execute("convert_currency", r#"{\"amount_usd\": 10.0}"#)
        .await?;

    println!("Result: {result}");
    Ok(())
}
