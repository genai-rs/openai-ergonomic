# Vector stores

Vector stores hold uploaded files, chunked and embedded, for semantic search and for the
assistants' `file_search` tool. `client.vector_stores()` creates, lists, gets, updates and
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

`VectorStoreSearchBuilder::filter(key, value)` records attribute filters on the builder, but
`search` sends only the query and `limit` (as `max_num_results`). For attribute filters, ranking
options or query rewriting, send a `VectorStoreSearchRequest` with the generated
`openai-client-base` operation (see [the generated operations](chat-and-responses.md#the-generated-operations)).

## With assistants

Attach a store to an assistant's `file_search` tool, or attach files to a thread message for
file search with `MessageAttachment::for_file_search(file_id)` (see
[tool orchestration](tool_orchestration.md#assistants-and-threads)).

Examples: [`vector_stores.rs`](../examples/vector_stores.rs),
[`assistants_file_search.rs`](../examples/assistants_file_search.rs). Uploading the files
themselves: [files and media](files-and-media.md).
