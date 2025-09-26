# Vector Store Playbook for RAG

Retrieval-augmented generation (RAG) relies on consistently curating documents, synchronising uploads, and issuing semantic searches. `openai-ergonomic` ships helper builders that capture the moving pieces—vector store metadata, file membership, and query parameters—so you can model the workflow without hand-crafting request payloads.

> **Status:** the ergonomic client currently keeps builder state and helper functions ready, while send helpers are still pending. Use the patterns below to translate builder state into `openai-client-base` requests until the direct integrations land.

## Modelling Vector Stores

[`VectorStoreBuilder`](../../src/builders/vector_stores.rs) stores the attributes you need for `CreateVectorStoreRequest`: a name, file IDs, optional expiration, and metadata.

```rust,no_run
use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;

let knowledge_base = VectorStoreBuilder::new()
    .name("Support KB")
    .add_files([
        "file-product-spec".to_string(),
        "file-release-notes".to_string(),
    ])
    .metadata("region", "eu")
    .expires_after_days(30);

assert!(knowledge_base.has_files());
```

Convert the builder into the generated client model before sending:

```rust,no_run
use openai_client_base::models::{CreateVectorStoreRequest, VectorStoreExpirationAfter};
use openai_ergonomic::builders::vector_stores::VectorStoreBuilder;

fn into_request(builder: &VectorStoreBuilder) -> CreateVectorStoreRequest {
    let mut request = CreateVectorStoreRequest::new();
    request.name = builder.name_ref().map(str::to_owned);
    request.file_ids = Some(builder.file_ids_ref().to_vec());
    if let Some(exp) = builder.expires_after_ref() {
        request.expires_after = Some(VectorStoreExpirationAfter { days: exp.days });
    }
    request.metadata = Some(builder.metadata_ref().clone());
    request
}
```

Now call the generated API:

```rust,no_run
use openai_client_base::apis::{configuration::Configuration, vector_stores_api};
use openai_ergonomic::{Client, Config, Error};

async fn create_store(builder: &VectorStoreBuilder) -> openai_ergonomic::Result<()> {
    let client = Client::new(Config::from_env()?)?;

    let mut configuration = Configuration::new();
    configuration.bearer_access_token = Some(client.config().api_key().to_string());
    if let Some(base) = client.config().base_url() {
        configuration.base_path = base.to_string();
    }

    let request = into_request(builder);
    vector_stores_api::create_vector_store()
        .configuration(&configuration)
        .create_vector_store_request(request)
        .call()
        .await
        .map_err(Error::from)?;

    Ok(())
}
```

The helper keeps your business logic (file IDs, metadata policy) in one place while the translation shim bridges to the REST model.

## Managing File Membership

To add or batch files post-creation, use [`VectorStoreFileBuilder`](../../src/builders/vector_stores.rs) or the convenience wrappers:

```rust,no_run
use openai_client_base::apis::vector_stores_api;
use openai_client_base::models::{CreateVectorStoreFileBatchRequest, CreateVectorStoreFileRequest};
use openai_ergonomic::builders::vector_stores::{
    add_file_to_vector_store,
    VectorStoreBuilder,
};

let builder = add_file_to_vector_store("vs_support", "file-new-faq");
let request = CreateVectorStoreFileRequest::new(builder.file_id().to_owned());

vector_stores_api::create_vector_store_file()
    .configuration(&configuration)
    .vector_store_id(builder.vector_store_id())
    .create_vector_store_file_request(request)
    .call()
    .await?;

let batch_ids = knowledge_base.file_ids_ref().to_vec();
let batch_request = CreateVectorStoreFileBatchRequest::new(batch_ids);

vector_stores_api::create_vector_store_file_batch()
    .configuration(&configuration)
    .vector_store_id("vs_support")
    .create_vector_store_file_batch_request(batch_request)
    .call()
    .await?;
```

## Searching the Store

[`VectorStoreSearchBuilder`](../../src/builders/vector_stores.rs) captures query, limit, and filter metadata. Translate it into the generated search request until the ergonomic client exposes direct helpers.

```rust,no_run
use openai_client_base::apis::vector_stores_api;
use openai_ergonomic::builders::vector_stores::search_vector_store_with_limit;

let search = search_vector_store_with_limit("vs_support", "deployment issue", 8)
    .filter("product", "widget-plus");

let results = vector_stores_api::search_vector_store()
    .configuration(&configuration)
    .vector_store_id(search.vector_store_id())
    .query(search.query())
    .maybe_limit(search.limit_ref())
    .maybe_filter(Some(search.filter_ref().clone()))
    .call()
    .await?;
```

## Putting It Together

1. Capture store state with `VectorStoreBuilder` (name, files, metadata, expiration).
2. Translate builder state to `CreateVectorStoreRequest` before invoking `vector_stores_api::create_vector_store`.
3. Use `VectorStoreFileBuilder` helpers to mutate membership incrementally.
4. Issue searches with `VectorStoreSearchBuilder`, passing the stored filter map to the generated API.
5. Feed retrieved chunks into your RAG pipeline (for example, summarise matches via the Responses API).

These patterns keep your RAG configuration declarative today and will drop directly into the upcoming client helpers once they ship.
