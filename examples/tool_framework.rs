//! Demonstrates the unified tool framework with typed inputs returning JSON.

use openai_ergonomic::{tool, tool_framework::ToolRegistry, tool_schema, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
/// Parameters for the demo search tool.
pub struct SearchParams {
    query: String,
    #[serde(default)]
    limit: Option<u32>,
}

tool! {
        /// Illustrative search tool used in the example.
    pub struct SearchTool;

    name: "search";
    description: "Perform a mock search over indexed documents";
    input_type: SearchParams;
    schema: tool_schema!(
        query: "string", "Search query to run", required: true,
        limit: "integer", "Maximum results to return", required: false,
    );

    async fn handle(params: SearchParams) -> Result<Value> {
        let limit = params.limit.unwrap_or(3);
        let results: Vec<_> = (1..=limit)
            .map(|idx| format!("Result {idx} for '{}'", params.query))
            .collect();

        Ok(serde_json::json!({
            "query": params.query,
            "results": results,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let registry = ToolRegistry::new().register(SearchTool);

    let payload = r#"{"query":"rust crates","limit":2}"#;
    let json = registry.execute("search", payload).await?;

    println!("Tool response: {json}");
    Ok(())
}
