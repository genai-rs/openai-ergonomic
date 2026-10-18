# Migrating to openai-ergonomic 0.7

Version 0.7 targets `openai-client-base` 0.16.0 and requires Rust 1.88 or newer.
The 0.6 release remains paired with `openai-client-base` 0.15.0.
While 0.16.0 is unpublished, this development line pins the exact source commit
`9ee34f77abd4b7aa0ae2234ef78b78dd2171d838` in `Cargo.toml`.

Most builder method signatures stay the same. The generated request structs
exposed by `Builder::build()` now use `Option<Option<T>>` for optional fields
whose wire format distinguishes an omitted property from an explicit `null`.
This affects changed fields in chat and legacy completion requests, image
generation requests, run and fine-tuning requests, and vector-store updates.

When constructing a generated request directly, wrap a concrete value twice.
Use the outer `None` to omit a field, `Some(None)` to send JSON `null`, and
`Some(Some(value))` to send a value:

```rust
use openai_client_base::models::CreateCompletionRequest;

let mut request = CreateCompletionRequest::new("gpt-4".into(), "Hello".into());
request.stream = Some(Some(false)); // sends `"stream": false`
request.stop = Some(None);          // sends `"stop": null`
request.n = None;                   // omits `n`

let max_tokens: Option<i32> = request.max_tokens.flatten();
let json = serde_json::to_value(request).unwrap();
assert_eq!(json["stream"], false);
assert!(json["stop"].is_null());
assert!(json.get("n").is_none());
```

Ergonomic builders generally omit optional fields the caller leaves unset and
preserve explicitly configured values, including empty strings, `false`, and
zero. For example, `.stream(false)` sends `"stream": false`. Some builders have
intentional defaults: `RunBuilder` sends `"stream": false` unless streaming is
enabled, so that default is not omitted. Streaming completion chunks also retain
the generated distinction between a missing `finish_reason` and a `null`
`finish_reason`; the high-level `finish_reason()` helper returns `None` for both.
This version preparation does not change the separately tracked
`PromptCacheOptions` requiredness blocker.
