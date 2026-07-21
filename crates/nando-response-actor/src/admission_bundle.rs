use std::io::Read;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{
    LiveScalarAdmissionCandidate, OnlineCollectionAdmissionCandidate,
    OnlineResponseAdmissionCandidate,
};

pub use nando_operator_admission::{
    DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1, DurableRuntimeParityReceipt,
};

pub const ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1: &str =
    "nando.online-admission-candidate-bundle.v1";
const MAX_RUNTIME_PARITY_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeParityCase {
    pub evidence_ref_sha256: String,
    pub capture_receipt: Option<crate::CaptureEvidenceReceipt>,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
}

impl Serialize for RuntimeParityCase {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let payload =
            serde_json::to_vec(&self.provider_payload).map_err(serde::ser::Error::custom)?;
        if u64::try_from(payload.len()).unwrap_or(u64::MAX) > MAX_RUNTIME_PARITY_PAYLOAD_BYTES {
            return Err(serde::ser::Error::custom(
                "runtime parity payload exceeds compression budget",
            ));
        }
        let compressed =
            zstd::stream::encode_all(payload.as_slice(), 1).map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("RuntimeParityCase", 5)?;
        state.serialize_field("evidence_ref_sha256", &self.evidence_ref_sha256)?;
        state.serialize_field("capture_receipt", &self.capture_receipt)?;
        state.serialize_field("request_text", &self.request_text)?;
        state.serialize_field(
            "provider_payload_zstd",
            serde_bytes::Bytes::new(&compressed),
        )?;
        state.serialize_field("expected_response", &self.expected_response)?;
        state.end()
    }
}

#[derive(Deserialize)]
struct RuntimeParityCaseWire {
    evidence_ref_sha256: String,
    #[serde(default)]
    capture_receipt: Option<crate::CaptureEvidenceReceipt>,
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
            capture_receipt: wire.capture_receipt,
            request_text: wire.request_text,
            provider_payload,
            expected_response: wire.expected_response,
        })
    }
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

    #[test]
    fn runtime_parity_writes_compact_zstd_payload() {
        let parity = RuntimeParityCase {
            evidence_ref_sha256: "c".repeat(64),
            capture_receipt: None,
            request_text: "return value".to_owned(),
            provider_payload: json!({"body": "x".repeat(32 * 1024)}),
            expected_response: "ok".to_owned(),
        };

        let encoded = serde_cbor::to_vec(&parity).expect("compact parity CBOR");
        let restored: RuntimeParityCase =
            serde_cbor::from_slice(&encoded).expect("compact parity roundtrip");

        assert!(encoded.len() < 1_024, "encoded bytes={}", encoded.len());
        assert_eq!(restored, parity);
    }
}
