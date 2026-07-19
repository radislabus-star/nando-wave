use std::io::Read;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    LiveScalarAdmissionCandidate, OnlineCollectionAdmissionCandidate,
    OnlineResponseAdmissionCandidate,
};

pub const ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1: &str =
    "nando.online-admission-candidate-bundle.v1";
pub const DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1: &str =
    "nando.durable-runtime-parity-receipt.v1";
const MAX_RUNTIME_PARITY_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeParityCase {
    pub evidence_ref_sha256: String,
    #[serde(default)]
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
}

#[derive(Deserialize)]
struct RuntimeParityCaseWire {
    evidence_ref_sha256: String,
    #[serde(default)]
    request_text: String,
    #[serde(default)]
    provider_payload: Option<Value>,
    #[serde(default)]
    provider_payload_json: Option<Box<str>>,
    #[serde(default)]
    provider_payload_zstd: Option<serde_bytes::ByteBuf>,
    expected_response: String,
}

impl<'de> Deserialize<'de> for RuntimeParityCase {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeParityCaseWire::deserialize(deserializer)?;
        let provider_payload = if let Some(payload) = wire.provider_payload {
            payload
        } else if let Some(json) = wire.provider_payload_json {
            serde_json::from_str(&json).map_err(serde::de::Error::custom)?
        } else if let Some(compressed) = wire.provider_payload_zstd {
            let decoder = zstd::stream::read::Decoder::new(compressed.as_slice())
                .map_err(serde::de::Error::custom)?;
            let mut json = Vec::new();
            decoder
                .take(MAX_RUNTIME_PARITY_PAYLOAD_BYTES.saturating_add(1))
                .read_to_end(&mut json)
                .map_err(serde::de::Error::custom)?;
            if u64::try_from(json.len()).unwrap_or(u64::MAX) > MAX_RUNTIME_PARITY_PAYLOAD_BYTES {
                return Err(serde::de::Error::custom(
                    "runtime parity payload exceeds decompression budget",
                ));
            }
            serde_json::from_slice(&json).map_err(serde::de::Error::custom)?
        } else {
            return Err(serde::de::Error::missing_field("provider_payload"));
        };
        Ok(Self {
            evidence_ref_sha256: wire.evidence_ref_sha256,
            request_text: wire.request_text,
            provider_payload,
            expected_response: wire.expected_response,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DurableRuntimeParityReceipt {
    pub schema: String,
    pub receipt_sha256: String,
    pub evidence_ref_sha256: String,
    pub program_sha256: String,
    pub verifier_sha256: String,
    pub input_sha256: String,
    pub teacher_response_sha256: String,
    pub actor_response_sha256: String,
    pub actor_executed: bool,
    pub teacher_authority_match: bool,
    pub independent_verifier_pass: bool,
    pub exact_teacher_match: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OnlineAdmissionCandidateBundle {
    pub schema: String,
    pub project_id: String,
    pub revision: u64,
    pub relation_candidates: Vec<OnlineResponseAdmissionCandidate>,
    pub collection_candidates: Vec<OnlineCollectionAdmissionCandidate>,
    #[serde(default)]
    pub crystallized_candidates: Vec<LiveScalarAdmissionCandidate>,
}

impl OnlineAdmissionCandidateBundle {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1 {
            return Err("online_admission_candidate_bundle_schema_invalid");
        }
        if self.project_id.is_empty() || self.project_id.len() > 128 || self.revision == 0 {
            return Err("online_admission_candidate_bundle_identity_invalid");
        }
        if self.relation_candidates.len() > 256
            || self.collection_candidates.len() > 256
            || self.crystallized_candidates.len() > 64
        {
            return Err("online_admission_candidate_bundle_capacity_exceeded");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::RuntimeParityCase;

    #[test]
    fn runtime_parity_reads_compact_checkpoint_payload() {
        let restored: RuntimeParityCase = serde_json::from_value(json!({
            "evidence_ref_sha256": "a".repeat(64),
            "request_text": "continue",
            "provider_payload_json": "{\"input\":[{\"value\":7}]}",
            "expected_response": "7"
        }))
        .expect("compact checkpoint parity");

        assert_eq!(restored.provider_payload, json!({"input": [{"value": 7}]}));
        assert_eq!(restored.expected_response, "7");
    }

    #[test]
    fn runtime_parity_reads_zstd_checkpoint_payload() {
        let payload = json!({"input": [{"value": 9}], "mode": "zstd"});
        let encoded = serde_json::to_vec(&payload).expect("payload JSON");
        let compressed = zstd::stream::encode_all(encoded.as_slice(), 1).expect("payload Zstd");
        let restored: RuntimeParityCase = serde_json::from_value(json!({
            "evidence_ref_sha256": "b".repeat(64),
            "request_text": "continue",
            "provider_payload_zstd": compressed,
            "expected_response": "9"
        }))
        .expect("Zstd checkpoint parity");

        assert_eq!(restored.provider_payload, payload);
        assert_eq!(restored.expected_response, "9");
    }
}
