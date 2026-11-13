# Webhooks

With the `webhook` feature, `webhooks::WebhookVerifier` verifies `OpenAI` webhook deliveries
([Standard Webhooks](https://www.standardwebhooks.com/) signatures: the `webhook-id`,
`webhook-timestamp` and `webhook-signature` headers, HMAC-SHA256 with your `whsec_...` secret,
any of several signatures, timestamps within 5 minutes by default) and decodes them into
`WebhookEvent`, whose variants hold the `openai-client-base` `Webhook*` models.

```bash
cargo add openai-ergonomic --features webhook
```

Pass the body exactly as received: the signature covers its bytes, so parse it only after
verifying (which `unwrap` does).

```rust
use openai_ergonomic::webhooks::{WebhookEvent, WebhookVerifier};

/// In your HTTP handler, with the request's headers and raw body.
fn handle(
    verifier: &WebhookVerifier,
    headers: &http::HeaderMap,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    match verifier.unwrap(headers, body)? {
        WebhookEvent::ResponseCompleted(event) => println!("response {} completed", event.data.id),
        WebhookEvent::BatchCompleted(event) => println!("batch {} completed", event.data.id),
        other => println!("ignoring {}", other.event_type()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let verifier = WebhookVerifier::new("whsec_c2VjcmV0LWtleS1mb3ItdGVzdHM=")?;

    // `sign` produces a valid `webhook-signature`, to test your handler.
    let body = br#"{"id":"evt_1","object":"event","created_at":1,"type":"batch.completed","data":{"id":"batch_1"}}"#;
    let timestamp: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let mut headers = http::HeaderMap::new();
    headers.insert("webhook-id", "msg_1".parse()?);
    headers.insert("webhook-timestamp", timestamp.to_string().parse()?);
    headers.insert("webhook-signature", verifier.sign("msg_1", timestamp, body).parse()?);
    handle(&verifier, &headers, body)
}
```

The verifier also has `verify` (the signature only), `unwrap_as` (into a type of your own),
`tolerance` (another timestamp window) and `verify_at` (a fixed time, for tests). On
WebAssembly it works as natively; see [WebAssembly](webassembly.md).
