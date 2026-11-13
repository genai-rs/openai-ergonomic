//! Verify `OpenAI` webhook deliveries and decode their events (feature `webhook`).
//!
//! `OpenAI` signs webhooks the [Standard Webhooks](https://www.standardwebhooks.com/) way. Each
//! delivery carries three headers:
//!
//! - `webhook-id`: the ID of the delivery;
//! - `webhook-timestamp`: when it was sent, in seconds since the Unix epoch;
//! - `webhook-signature`: one or more space-separated `v1,<base64 HMAC-SHA256>` signatures of
//!   `{webhook-id}.{webhook-timestamp}.{body}`, keyed with your endpoint's secret (`whsec_...`,
//!   whose part after `whsec_` is base64).
//!
//! [`WebhookVerifier`] checks the signature and the timestamp (within 5 minutes of now by
//! default, against replays) and decodes the body into a [`WebhookEvent`], whose variants hold
//! the `openai-client-base` models:
//!
//! ```rust
//! use openai_ergonomic::webhooks::{WebhookEvent, WebhookVerifier};
//!
//! fn handle(headers: &http::HeaderMap, body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
//!     let verifier = WebhookVerifier::new("whsec_c2VjcmV0LWtleS1mb3ItdGVzdHM=")?;
//!     match verifier.unwrap(headers, body)? {
//!         WebhookEvent::ResponseCompleted(event) => println!("response {} done", event.data.id),
//!         WebhookEvent::BatchFailed(event) => println!("batch {} failed", event.data.id),
//!         other => println!("{} event", other.event_type()),
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Pass the body exactly as received: the signature covers its bytes, so parse it only after
//! verifying (which [`WebhookVerifier::unwrap`] does).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use http::HeaderMap;
use openai_client_base::models;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::Sha256;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// The header with the ID of a delivery.
pub const WEBHOOK_ID: &str = "webhook-id";
/// The header with the time a delivery was sent, in seconds since the Unix epoch.
pub const WEBHOOK_TIMESTAMP: &str = "webhook-timestamp";
/// The header with the signatures of a delivery.
pub const WEBHOOK_SIGNATURE: &str = "webhook-signature";
/// How far a delivery's timestamp may be from now, by default: 5 minutes.
pub const DEFAULT_TOLERANCE: Duration = Duration::from_secs(300);

/// Why a webhook delivery was rejected.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WebhookError {
    /// The secret is not base64 (after an optional `whsec_` prefix), or is empty.
    #[error("invalid webhook secret: {0}")]
    InvalidSecret(String),
    /// A required header is missing, or is not valid text.
    #[error("missing or invalid `{0}` header")]
    MissingHeader(&'static str),
    /// The timestamp is not a number of seconds.
    #[error("invalid webhook timestamp `{0}`")]
    InvalidTimestamp(String),
    /// The timestamp is further from now than the tolerance: a replay, or a clock out of sync.
    #[error("webhook timestamp {timestamp} is more than {tolerance:?} away from now ({now})")]
    TimestampOutOfTolerance {
        /// The delivery's timestamp, in seconds since the Unix epoch.
        timestamp: i64,
        /// Now, in seconds since the Unix epoch.
        now: i64,
        /// The tolerance.
        tolerance: Duration,
    },
    /// No `v1` signature matches the body.
    #[error("no webhook signature matches")]
    InvalidSignature,
    /// The body is not a webhook event: the error and the body.
    #[error("failed to decode the webhook event: {0}")]
    Decode(#[source] serde_json::Error, String),
}

/// Verifies the signatures of webhook deliveries with an endpoint's secret.
#[derive(Clone)]
pub struct WebhookVerifier {
    key: Vec<u8>,
    tolerance: Duration,
}

impl fmt::Debug for WebhookVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookVerifier")
            .field("key", &"***")
            .field("tolerance", &self.tolerance)
            .finish()
    }
}

impl WebhookVerifier {
    /// A verifier for the endpoint secret `secret` (`whsec_<base64>`, or the base64 alone).
    pub fn new(secret: &str) -> Result<Self, WebhookError> {
        let encoded = secret.trim();
        let encoded = encoded.strip_prefix("whsec_").unwrap_or(encoded);
        let key = BASE64
            .decode(encoded)
            .map_err(|e| WebhookError::InvalidSecret(format!("not base64: {e}")))?;
        if key.is_empty() {
            return Err(WebhookError::InvalidSecret("empty".to_string()));
        }
        Ok(Self {
            key,
            tolerance: DEFAULT_TOLERANCE,
        })
    }

    /// How far a delivery's timestamp may be from now, in either direction (default
    /// [`DEFAULT_TOLERANCE`], 5 minutes).
    #[must_use]
    pub fn tolerance(mut self, tolerance: Duration) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Verifies a delivery from its headers (`webhook-id`, `webhook-timestamp`,
    /// `webhook-signature`) and its body, as received.
    pub fn verify(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), WebhookError> {
        self.verify_at(headers, body, SystemTime::now())
    }

    /// [`WebhookVerifier::verify`], with `now` as the current time.
    pub fn verify_at(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: SystemTime,
    ) -> Result<(), WebhookError> {
        let header = |name: &'static str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .ok_or(WebhookError::MissingHeader(name))
        };
        self.verify_parts_at(
            header(WEBHOOK_ID)?,
            header(WEBHOOK_TIMESTAMP)?,
            header(WEBHOOK_SIGNATURE)?,
            body,
            now,
        )
    }

    /// Verifies a delivery from the values of its three headers and its body.
    pub fn verify_parts(
        &self,
        webhook_id: &str,
        timestamp: &str,
        signatures: &str,
        body: &[u8],
    ) -> Result<(), WebhookError> {
        self.verify_parts_at(webhook_id, timestamp, signatures, body, SystemTime::now())
    }

    /// [`WebhookVerifier::verify_parts`], with `now` as the current time.
    pub fn verify_parts_at(
        &self,
        webhook_id: &str,
        timestamp: &str,
        signatures: &str,
        body: &[u8],
        now: SystemTime,
    ) -> Result<(), WebhookError> {
        let seconds: i64 = timestamp
            .trim()
            .parse()
            .map_err(|_| WebhookError::InvalidTimestamp(timestamp.to_string()))?;
        let now_seconds = unix_seconds(now);
        let tolerance = i64::try_from(self.tolerance.as_secs()).unwrap_or(i64::MAX);
        if now_seconds.abs_diff(seconds) > tolerance.unsigned_abs() {
            return Err(WebhookError::TimestampOutOfTolerance {
                timestamp: seconds,
                now: now_seconds,
                tolerance: self.tolerance,
            });
        }
        let mac = self.mac(webhook_id, timestamp.trim(), body);
        let matches = signatures
            .split_whitespace()
            .filter_map(|signature| signature.strip_prefix("v1,"))
            .filter_map(|signature| BASE64.decode(signature).ok())
            .any(|signature| mac.clone().verify_slice(&signature).is_ok());
        if matches {
            Ok(())
        } else {
            Err(WebhookError::InvalidSignature)
        }
    }

    fn mac(&self, webhook_id: &str, timestamp: &str, body: &[u8]) -> HmacSha256 {
        // HMAC takes keys of any length.
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .unwrap_or_else(|_| unreachable!("HMAC accepts keys of any length"));
        mac.update(webhook_id.as_bytes());
        mac.update(b".");
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(body);
        mac
    }

    /// The `webhook-signature` value (`v1,<base64>`) of a delivery: to test a webhook handler,
    /// or to sign deliveries of your own.
    pub fn sign(&self, webhook_id: &str, timestamp: i64, body: &[u8]) -> String {
        let mac = self.mac(webhook_id, &timestamp.to_string(), body);
        format!("v1,{}", BASE64.encode(mac.finalize().into_bytes()))
    }

    /// Verifies a delivery and decodes its event.
    pub fn unwrap(&self, headers: &HeaderMap, body: &[u8]) -> Result<WebhookEvent, WebhookError> {
        self.verify(headers, body)?;
        WebhookEvent::from_slice(body)
    }

    /// Verifies a delivery and decodes its body into a type of your own.
    pub fn unwrap_as<T: DeserializeOwned>(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<T, WebhookError> {
        self.verify(headers, body)?;
        serde_json::from_slice(body).map_err(|e| decode_error(e, body))
    }
}

fn unix_seconds(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => -i64::try_from(before.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

fn decode_error(error: serde_json::Error, body: &[u8]) -> WebhookError {
    WebhookError::Decode(error, String::from_utf8_lossy(body).into_owned())
}

macro_rules! webhook_events {
    ($($(#[$doc:meta])* $variant:ident($model:ident) = $type:literal,)*) => {
        /// A webhook event, decoded by its `type` into the `openai-client-base` model.
        ///
        /// An event of a type this version does not know is kept as JSON in
        /// [`WebhookEvent::Unknown`].
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub enum WebhookEvent {
            $($(#[$doc])* #[doc = concat!("`", $type, "`.")] $variant(Box<models::$model>),)*
            /// An event of another type, as JSON.
            Unknown(serde_json::Value),
        }

        /// The event types [`WebhookEvent`] decodes.
        pub const WEBHOOK_EVENT_TYPES: &[&str] = &[$($type),*];

        impl WebhookEvent {
            /// Decodes an event from its JSON value.
            pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
                let event_type = value.get("type").and_then(serde_json::Value::as_str);
                Ok(match event_type {
                    $(Some($type) => Self::$variant(Box::new(serde_json::from_value(value)?)),)*
                    _ => Self::Unknown(value),
                })
            }

            /// The event's `type`, such as `response.completed`.
            pub fn event_type(&self) -> &str {
                match self {
                    $(Self::$variant(_) => $type,)*
                    Self::Unknown(value) => value
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default(),
                }
            }

            /// The event as JSON.
            pub fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
                match self {
                    $(Self::$variant(event) => serde_json::to_value(event),)*
                    Self::Unknown(value) => Ok(value.clone()),
                }
            }
        }
    };
}

webhook_events! {
    AgentEnvironmentExpired(WebhookAgentEnvironmentExpired) = "agent.environment.expired",
    AgentEnvironmentFailed(WebhookAgentEnvironmentFailed) = "agent.environment.failed",
    AgentEnvironmentReady(WebhookAgentEnvironmentReady) = "agent.environment.ready",
    AgentEnvironmentSuspended(WebhookAgentEnvironmentSuspended) = "agent.environment.suspended",
    AgentSessionActionRequired(WebhookAgentSessionActionRequired) = "agent.session.action_required",
    AgentSessionCreated(WebhookAgentSessionCreated) = "agent.session.created",
    AgentSessionFailed(WebhookAgentSessionFailed) = "agent.session.failed",
    AgentSessionIdle(WebhookAgentSessionIdle) = "agent.session.idle",
    AgentSessionInProgress(WebhookAgentSessionInProgress) = "agent.session.in_progress",
    BatchCancelled(WebhookBatchCancelled) = "batch.cancelled",
    BatchCompleted(WebhookBatchCompleted) = "batch.completed",
    BatchExpired(WebhookBatchExpired) = "batch.expired",
    BatchFailed(WebhookBatchFailed) = "batch.failed",
    EvalRunCanceled(WebhookEvalRunCanceled) = "eval.run.canceled",
    EvalRunFailed(WebhookEvalRunFailed) = "eval.run.failed",
    EvalRunSucceeded(WebhookEvalRunSucceeded) = "eval.run.succeeded",
    FineTuningJobCancelled(WebhookFineTuningJobCancelled) = "fine_tuning.job.cancelled",
    FineTuningJobFailed(WebhookFineTuningJobFailed) = "fine_tuning.job.failed",
    FineTuningJobSucceeded(WebhookFineTuningJobSucceeded) = "fine_tuning.job.succeeded",
    LiveCallIncoming(WebhookLiveCallIncoming) = "live.call.incoming",
    LiveTransportConnected(WebhookLiveTransportConnected) = "live.transport.connected",
    LiveTransportIncoming(WebhookLiveTransportIncoming) = "live.transport.incoming",
    RealtimeCallIncoming(WebhookRealtimeCallIncoming) = "realtime.call.incoming",
    ResponseCancelled(WebhookResponseCancelled) = "response.cancelled",
    ResponseCompleted(WebhookResponseCompleted) = "response.completed",
    ResponseFailed(WebhookResponseFailed) = "response.failed",
    ResponseIncomplete(WebhookResponseIncomplete) = "response.incomplete",
    SafetyAlertCreated(WebhookSafetyAlertCreated) = "safety.alert.created",
    SafetyDeactivationIssued(WebhookSafetyDeactivationIssued) = "safety.deactivation_issued",
    SafetyOrgAlertCreated(WebhookSafetyOrgAlertCreated) = "safety.org_alert.created",
    SafetyWarningIssued(WebhookSafetyWarningIssued) = "safety.warning_issued",
}

impl WebhookEvent {
    /// Decodes an event from its JSON body.
    pub fn from_slice(body: &[u8]) -> Result<Self, WebhookError> {
        let value: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| decode_error(e, body))?;
        Self::from_value(value).map_err(|e| decode_error(e, body))
    }

    /// The event's `id` (`evt_...`).
    pub fn id(&self) -> Option<String> {
        self.to_value()
            .ok()?
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    }
}

impl Serialize for WebhookEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for WebhookEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_value(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    // base64("secret-key-for-tests")
    const SECRET: &str = "whsec_c2VjcmV0LWtleS1mb3ItdGVzdHM=";
    const ID: &str = "wh_123";

    fn body() -> Vec<u8> {
        json!({
            "id": "evt_1",
            "object": "event",
            "created_at": 1_750_000_000,
            "type": "response.completed",
            "data": {"id": "resp_1"}
        })
        .to_string()
        .into_bytes()
    }

    fn now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_750_000_000)
    }

    fn delivery(id: &str, timestamp: i64, signature: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(WEBHOOK_ID, id.parse().unwrap());
        headers.insert(WEBHOOK_TIMESTAMP, timestamp.to_string().parse().unwrap());
        headers.insert(WEBHOOK_SIGNATURE, signature.parse().unwrap());
        headers
    }

    /// The signature computed independently: HMAC-SHA256 over `id.timestamp.body`.
    fn expected_signature(timestamp: i64, body: &[u8]) -> String {
        let key = BASE64
            .decode(SECRET.strip_prefix("whsec_").unwrap())
            .unwrap();
        assert_eq!(key, b"secret-key-for-tests");
        let mut payload = format!("{ID}.{timestamp}.").into_bytes();
        payload.extend_from_slice(body);
        let mut mac = HmacSha256::new_from_slice(&key).unwrap();
        mac.update(&payload);
        format!("v1,{}", BASE64.encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn a_known_vector_verifies() {
        // From the Standard Webhooks specification's reference implementations.
        let verifier = WebhookVerifier::new("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw").unwrap();
        let body = br#"{"test": 2432232314}"#;
        let signature = "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=";
        assert_eq!(
            verifier.sign("msg_p5jXN8AQM9LWM0D4loKWxJek", 1_614_265_330, body),
            signature
        );
        verifier
            .verify_parts_at(
                "msg_p5jXN8AQM9LWM0D4loKWxJek",
                "1614265330",
                signature,
                body,
                UNIX_EPOCH + Duration::from_secs(1_614_265_330),
            )
            .unwrap();
    }

    #[test]
    fn verifies_and_decodes() {
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let body = body();
        let signature = expected_signature(1_750_000_000, &body);
        assert_eq!(verifier.sign(ID, 1_750_000_000, &body), signature);
        let headers = delivery(ID, 1_750_000_000, &signature);
        verifier.verify_at(&headers, &body, now()).unwrap();

        let event = WebhookEvent::from_slice(&body).unwrap();
        assert_eq!(event.event_type(), "response.completed");
        assert_eq!(event.id().as_deref(), Some("evt_1"));
        match event {
            WebhookEvent::ResponseCompleted(event) => assert_eq!(event.data.id, "resp_1"),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn any_of_several_signatures_may_match() {
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let body = body();
        let good = expected_signature(1_750_000_000, &body);
        let signatures = format!("v1,bm90LXRoaXMtb25l v2,whatever {good}");
        verifier
            .verify_parts_at(ID, "1750000000", &signatures, &body, now())
            .unwrap();
    }

    #[test]
    fn rejects_bad_signatures() {
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let body = body();
        let good = expected_signature(1_750_000_000, &body);
        for (id, signatures, body) in [
            // Another body.
            (ID, good.clone(), b"{}".to_vec()),
            // Another ID.
            ("wh_other", good.clone(), body.clone()),
            // The signature without its version, or with another version.
            (ID, good.trim_start_matches("v1,").to_string(), body.clone()),
            (ID, good.replacen("v1,", "v2,", 1), body.clone()),
            // Not base64.
            (ID, "v1,%%%".to_string(), body.clone()),
        ] {
            assert!(matches!(
                verifier.verify_parts_at(id, "1750000000", &signatures, &body, now()),
                Err(WebhookError::InvalidSignature)
            ));
        }
        // Another secret.
        let other = WebhookVerifier::new("whsec_b3RoZXI=").unwrap();
        assert!(matches!(
            other.verify_parts_at(ID, "1750000000", &good, &body, now()),
            Err(WebhookError::InvalidSignature)
        ));
    }

    #[test]
    fn enforces_the_tolerance() {
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let body = body();
        for offset in [-300, 0, 300] {
            let timestamp = 1_750_000_000 + offset;
            let headers = delivery(ID, timestamp, &verifier.sign(ID, timestamp, &body));
            verifier.verify_at(&headers, &body, now()).unwrap();
        }
        for offset in [-301, 301] {
            let timestamp = 1_750_000_000 + offset;
            let headers = delivery(ID, timestamp, &verifier.sign(ID, timestamp, &body));
            assert!(matches!(
                verifier.verify_at(&headers, &body, now()),
                Err(WebhookError::TimestampOutOfTolerance { .. })
            ));
        }
        let lenient = verifier.tolerance(Duration::from_secs(3600));
        let headers = delivery(ID, 1_750_003_000, &lenient.sign(ID, 1_750_003_000, &body));
        lenient.verify_at(&headers, &body, now()).unwrap();
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(matches!(
            WebhookVerifier::new("whsec_not base64!"),
            Err(WebhookError::InvalidSecret(_))
        ));
        assert!(matches!(
            WebhookVerifier::new("whsec_"),
            Err(WebhookError::InvalidSecret(_))
        ));
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let mut headers = delivery(ID, 1_750_000_000, "v1,x");
        headers.remove(WEBHOOK_SIGNATURE);
        assert!(matches!(
            verifier.verify_at(&headers, b"{}", now()),
            Err(WebhookError::MissingHeader(WEBHOOK_SIGNATURE))
        ));
        assert!(matches!(
            verifier.verify_parts_at(ID, "soon", "v1,x", b"{}", now()),
            Err(WebhookError::InvalidTimestamp(_))
        ));
    }

    #[test]
    fn unwrap_verifies_with_the_current_time() {
        let verifier = WebhookVerifier::new(SECRET).unwrap();
        let body = body();
        let timestamp = unix_seconds(SystemTime::now());
        let headers = delivery(ID, timestamp, &verifier.sign(ID, timestamp, &body));
        let event = verifier.unwrap(&headers, &body).unwrap();
        assert!(matches!(event, WebhookEvent::ResponseCompleted(_)));
        let value: serde_json::Value = verifier.unwrap_as(&headers, &body).unwrap();
        assert_eq!(value["type"], "response.completed");
        // An old delivery is rejected, even with a valid signature.
        let old = delivery(ID, 1_750_000_000, &verifier.sign(ID, 1_750_000_000, &body));
        assert!(verifier.unwrap(&old, &body).is_err());
    }

    #[test]
    fn decodes_every_known_type_and_keeps_unknown_ones() {
        let event = WebhookEvent::from_slice(
            json!({
                "id": "evt_2",
                "object": "event",
                "created_at": 1,
                "type": "batch.failed",
                "data": {"id": "batch_1"}
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        assert!(matches!(event, WebhookEvent::BatchFailed(ref e) if e.data.id == "batch_1"));
        let round_trip: WebhookEvent =
            serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
        assert_eq!(round_trip, event);

        let unknown = json!({"id": "evt_3", "type": "something.new", "data": {}});
        let event = WebhookEvent::from_slice(unknown.to_string().as_bytes()).unwrap();
        assert_eq!(event.event_type(), "something.new");
        assert_eq!(event, WebhookEvent::Unknown(unknown));

        // A known type with a body that does not match its model.
        let bad = json!({"type": "batch.failed", "data": 1});
        assert!(matches!(
            WebhookEvent::from_slice(bad.to_string().as_bytes()),
            Err(WebhookError::Decode(..))
        ));
        assert_eq!(WEBHOOK_EVENT_TYPES.len(), 31);
    }
}
