use nando_operator_kernel::{Sha256CommitmentV3, sha256_bytes};
use nando_operator_learning::{evidence_client_intent_id_sha256, evidence_session_id_sha256};
use serde_json::Value;

const MAX_PROVIDER_ID_BYTES: usize = 256;
const SESSION_LINEAGE_DOMAIN_V1: &[u8] = b"nando.provider-session-lineage.v1";
const UNATTRIBUTED_SESSION_V1: &[u8] = b"unattributed-session";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TurnIntentIdentitySourceV1 {
    ProviderMetadata,
    TransportFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderRequestIdentityV1 {
    turn_intent_id: String,
    turn_intent_sha256: String,
    request_event_id: String,
    request_event_sha256: String,
    session_lineage_root: Sha256CommitmentV3,
    session_identity_sha256s: Vec<String>,
    source: TurnIntentIdentitySourceV1,
}

impl ProviderRequestIdentityV1 {
    pub(crate) fn from_payload(payload: &Value, transport_request_id: &str) -> Self {
        let metadata = payload.get("client_metadata");
        let session_id = provider_id(metadata, "session_id");
        let thread_id = provider_id(metadata, "thread_id");
        let turn_id = provider_id(metadata, "turn_id");
        let prompt_cache_key = payload
            .get("prompt_cache_key")
            .and_then(Value::as_str)
            .filter(|value| valid_provider_id(value));

        let (turn_intent_id, source) = match turn_id {
            Some(value) => (
                value.to_owned(),
                TurnIntentIdentitySourceV1::ProviderMetadata,
            ),
            None => (
                transport_request_id.to_owned(),
                TurnIntentIdentitySourceV1::TransportFallback,
            ),
        };
        let session_lineage_root = session_id.or(thread_id).map_or_else(
            || {
                Sha256CommitmentV3::digest_parts(
                    SESSION_LINEAGE_DOMAIN_V1,
                    &[UNATTRIBUTED_SESSION_V1],
                )
            },
            |value| {
                Sha256CommitmentV3::digest_parts(SESSION_LINEAGE_DOMAIN_V1, &[value.as_bytes()])
            },
        );
        let mut session_identity_sha256s = session_id
            .map(evidence_session_id_sha256)
            .into_iter()
            .chain(thread_id.map(|value| sha256_bytes(value.as_bytes())))
            .chain(prompt_cache_key.map(|value| sha256_bytes(value.as_bytes())))
            .collect::<Vec<_>>();
        session_identity_sha256s.sort();
        session_identity_sha256s.dedup();

        Self {
            turn_intent_sha256: evidence_client_intent_id_sha256(&turn_intent_id),
            turn_intent_id,
            request_event_sha256: sha256_bytes(transport_request_id.as_bytes()),
            request_event_id: transport_request_id.to_owned(),
            session_lineage_root,
            session_identity_sha256s,
            source,
        }
    }

    pub(crate) fn turn_intent_id(&self) -> &str {
        &self.turn_intent_id
    }

    pub(crate) fn turn_intent_sha256(&self) -> &str {
        &self.turn_intent_sha256
    }

    pub(crate) fn request_event_id(&self) -> &str {
        &self.request_event_id
    }

    pub(crate) fn request_event_sha256(&self) -> &str {
        &self.request_event_sha256
    }

    pub(crate) const fn session_lineage_root(&self) -> Sha256CommitmentV3 {
        self.session_lineage_root
    }

    pub(crate) fn session_identity_sha256s(&self) -> &[String] {
        &self.session_identity_sha256s
    }

    pub(crate) const fn provider_bound_turn_identity(&self) -> bool {
        matches!(self.source, TurnIntentIdentitySourceV1::ProviderMetadata)
    }
}

fn provider_id<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .filter(|value| valid_provider_id(value))
}

fn valid_provider_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_ID_BYTES
        && value.bytes().all(|byte| !byte.is_ascii_control())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn provider_turn_and_session_identities_are_kept_separate() {
        let identity = ProviderRequestIdentityV1::from_payload(
            &json!({
                "client_metadata": {
                    "session_id": "session-a",
                    "thread_id": "thread-a",
                    "turn_id": "turn-a"
                },
                "prompt_cache_key": "cache-a"
            }),
            "nginx-event-a",
        );

        assert_eq!(identity.turn_intent_id(), "turn-a");
        assert_eq!(
            identity.turn_intent_sha256(),
            evidence_client_intent_id_sha256("turn-a")
        );
        assert_eq!(identity.request_event_id(), "nginx-event-a");
        assert_eq!(
            identity.request_event_sha256(),
            sha256_bytes(b"nginx-event-a")
        );
        assert!(identity.provider_bound_turn_identity());
        assert_eq!(identity.session_identity_sha256s().len(), 3);
        assert!(
            identity
                .session_identity_sha256s()
                .contains(&evidence_session_id_sha256("session-a"))
        );
        assert_ne!(
            identity.session_lineage_root(),
            Sha256CommitmentV3::digest_parts(SESSION_LINEAGE_DOMAIN_V1, &[UNATTRIBUTED_SESSION_V1])
        );
    }

    #[test]
    fn unattributed_requests_share_one_conservative_lineage() {
        let first = ProviderRequestIdentityV1::from_payload(&json!({}), "event-a");
        let second = ProviderRequestIdentityV1::from_payload(&json!({}), "event-b");

        assert_ne!(first.turn_intent_id(), second.turn_intent_id());
        assert_ne!(first.request_event_id(), second.request_event_id());
        assert_eq!(first.session_lineage_root(), second.session_lineage_root());
        assert!(first.session_identity_sha256s().is_empty());
        assert!(!first.provider_bound_turn_identity());
    }

    #[test]
    fn one_turn_can_contain_multiple_independent_provider_requests() {
        let payload = json!({
            "client_metadata": {
                "session_id": "session-a",
                "turn_id": "turn-a"
            }
        });
        let first = ProviderRequestIdentityV1::from_payload(&payload, "nginx-event-a");
        let second = ProviderRequestIdentityV1::from_payload(&payload, "nginx-event-b");

        assert_eq!(first.turn_intent_sha256(), second.turn_intent_sha256());
        assert_ne!(first.request_event_sha256(), second.request_event_sha256());
        assert_eq!(first.session_lineage_root(), second.session_lineage_root());
    }
}
