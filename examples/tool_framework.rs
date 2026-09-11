//! Define a typed tool and execute it locally. No API key required.

use async_trait::async_trait;
use openai_ergonomic::{Error, FunctionTool, Result, ToolRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchParams {
    query: String,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SearchResults {
    matches: Vec<String>,
}

struct Search;

#[async_trait]
impl FunctionTool for Search {
    type Input = SearchParams;
    type Output = SearchResults;

    fn name(&self) -> &'static str {
        "search"
    }
    fn description(&self) -> &'static str {
        "Search demo document titles containing the given text."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Case-sensitive title fragment"},
                "limit": {"type": ["integer", "null"], "minimum": 1, "maximum": 10,
                    "description": "Maximum matches; defaults to 3 when omitted or null"}
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }
    async fn execute(&self, input: SearchParams) -> Result<SearchResults> {
        let limit = input.limit.unwrap_or(3);
        if !(1..=10).contains(&limit) {
            return Err(Error::InvalidRequest(
                "limit must be between 1 and 10".into(),
            ));
        }
        Ok(SearchResults {
            matches: ["Rust tools", "Rust async", "JSON schemas"]
                .into_iter()
                .filter(|title| title.contains(&input.query))
                .take(limit)
                .map(str::to_owned)
                .collect(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut tools = ToolRegistry::new();
    tools.register(Search)?;
    let result = tools
        .execute("search", r#"{"query":"Rust","limit":2}"#)
        .await?;
    assert_eq!(result["matches"], json!(["Rust tools", "Rust async"]));
    println!("{result}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn offline_search() {
        super::main().unwrap();
    }
}
