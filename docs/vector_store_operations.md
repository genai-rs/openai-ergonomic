# Vector stores

Vector stores hold uploaded files, chunked and embedded, for semantic search and for the
Responses API's `file_search` tool. `client.vector_stores()` creates, lists, gets, updates and
deletes stores, adds and removes their files, and searches them.

```rust,no_run
use openai_ergonomic::builders::vector_stores::{VectorStoreBuilder, VectorStoreSearchBuilder};
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let stores = client.vector_stores();

    // A store with files already uploaded with the `assistants` purpose
    let store = stores
        .create(
            VectorStoreBuilder::new()
                .name("Engineering knowledge base")
                .add_files(["file-release-notes", "file-architecture-doc"])
                .metadata("owner", "platform-team")
                .expires_after_days(30),
        )
        .await?;

    // Another file later
    stores.add_file(&store.id, "file-customer-faq").await?;
    let files = stores.list_files(&store.id, Some(20), None, None, None, None).await?;
    println!("{} files", files.data.len());

    // Semantic search
    let results = stores
        .search(VectorStoreSearchBuilder::new(&store.id, "how do we roll back a release?").limit(5))
        .await?;
    for result in &results.data {
        println!("{} ({:.2})", result.filename, result.score);
    }

    stores.delete(&store.id).await?;
    Ok(())
}
```

`search` sends the query, `limit` (as `max_num_results`) and the attribute filters of
`VectorStoreSearchBuilder::filter(key, value)`: each is an `eq` comparison, several are combined
with `and`. For other comparisons (`gt`, `in`, ...), `or`, ranking options or query rewriting,
send a `VectorStoreSearchRequest` with the generated `openai-client-base` operation (see
[the generated operations](chat-and-responses.md#the-generated-operations)).

## With the Responses API

Give a Responses request a `file_search` tool with the store's ID:

```rust,no_run
use openai_ergonomic::responses_api::ResponsesTool;
use openai_ergonomic::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?.build();
    let file_search: ResponsesTool = serde_json::from_value(serde_json::json!({
        "type": "file_search",
        "vector_store_ids": ["vs_abc123"]
    }))?;
    let response = client
        .send_responses(client.responses().tool(file_search).user("How do we roll back a release?"))
        .await?;
    println!("{}", response.output_text());
    Ok(())
}
```

Example: [`vector_stores.rs`](../examples/vector_stores.rs). Uploading the files
themselves: [files and media](files-and-media.md).
