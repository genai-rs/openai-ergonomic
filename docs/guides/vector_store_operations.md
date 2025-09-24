# Vector Store Operations

Vector stores let assistants perform Retrieval-Augmented Generation (RAG) over large document corpora. The builder module in `openai-ergonomic` wraps the verbose OpenAI payloads with fluent, type-safe helpers.

## Creating a Vector Store

```rust
use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;

let builder = VectorStoreBuilder::new()
    .name("Engineering Knowledge Base")
    .add_files([
        "file-release-notes".to_string(),
        "file-architecture-doc".to_string(),
    ])
    .metadata("owner", "platform-team")
    .expires_after_days(30);

let mut request = openai_client_base::models::CreateVectorStoreRequest::new();
request.name = builder.name_ref().map(|name| name.to_string());
if builder.has_files() {
    request.file_ids = Some(builder.file_ids_ref().to_vec());
}
if let Some(expiry) = builder.expires_after_ref() {
    request.expires_after = Some(Box::new(
        openai_client_base::models::VectorStoreExpirationAfter::new(
            openai_client_base::models::vector_store_expiration_after::Anchor::LastActiveAt,
            expiry.days,
        ),
    ));
}
if !builder.metadata_ref().is_empty() {
    request.metadata = Some(builder.metadata_ref().clone());
}
```

`VectorStoreBuilder` keeps track of file IDs, expiration policies, and metadata. The resulting request can be passed to `openai-client-base` or any HTTP client targeting the `/vector_stores` endpoint.

## Managing Files in a Store

The `VectorStoreFileBuilder` helper targets endpoints that manipulate individual files inside a store.

```rust
use openai_ergonomic::builders::vector_stores::VectorStoreFileBuilder;

let add_file = VectorStoreFileBuilder::new("vs_123", "file_customer_faq");
let request = add_file.build()?; // Creates `CreateVectorStoreFileRequest`
```

You can combine this with batch operations (e.g. `add_files`) to keep stores in sync with your document pipeline.

## Searching a Store

Vector store search requests allow structured filtering in addition to the query text.

```rust
use openai_ergonomic::builders::vector_stores::VectorStoreSearchBuilder;

let search = VectorStoreSearchBuilder::new("vs_123", "rust mutex explanation")
    .limit(5)
    .filter("document_type", "postmortem");

let request = search.build()?; // `QueryVectorStoreRequest`
```

Filters are recorded in a `HashMap<String, String>` and serialised automatically. This makes it easy to build higher-level abstractions (for example, mapping structured UI filters into query metadata).

## Thread Integration

Combine vector stores with the thread builders introduced in `threads.rs` to expose knowledge bases to the file search tool:

```rust
use openai_ergonomic::builders::threads::{
    MessageAttachment, ThreadMessageBuilder, ThreadRequestBuilder,
};

let thread_builder = ThreadRequestBuilder::new()
    .user_message("Summarise the latest release notes")
    .message_builder(
        ThreadMessageBuilder::assistant("I will reference the knowledge base.")
            .attachment(MessageAttachment::for_file_search("file-release-notes")),
    )?;
```

Once the vector store is attached to a thread on the server, assistants can seamlessly ground their answers using the uploaded documents.

## Summary

- Use `VectorStoreBuilder` to construct create/update payloads without manually juggling JSON.
- `VectorStoreFileBuilder` and `VectorStoreSearchBuilder` cover file-level and semantic search operations.
- Thread attachments (`MessageAttachment::for_file_search`) bridge stores into the Assistants surface, enabling consistent RAG experiences.

These builders isolate the low-level wiring so you can focus on the workflow logic—whether you're syncing documents, performing ad-hoc searches, or powering full assistants.
