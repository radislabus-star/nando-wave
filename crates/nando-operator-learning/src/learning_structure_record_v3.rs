use nando_operator_kernel::{
    LearningRequestStructureV2, PreActionTopologyCommitV1, canonical_json_bytes, sha256_bytes,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::{LearningRequestStructureV1, ProviderRequestCaptureReceiptV3};

pub const LEARNING_STRUCTURE_RECORD_SCHEMA_V3: &str = "nando.learning-structure-record.v3";
pub const LEARNING_STRUCTURE_RECORD_MAX_BYTES_V3: usize = 32 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningStructureRecordV3 {
    bridge_epoch_sha256: String,
    bridge_sequence: u64,
    capture_receipt: ProviderRequestCaptureReceiptV3,
    structure_v1: LearningRequestStructureV1,
    structure_v2: LearningRequestStructureV2,
    topology_commit: PreActionTopologyCommitV1,
    record_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningStructureRecordErrorV3 {
    InvalidInput,
    InvalidReceipt,
    InvalidStructure,
    DigestMismatch,
    BudgetExhausted,
    Serialization,
}

#[derive(Deserialize, Serialize)]
struct LearningStructureRecordWireV3 {
    schema: String,
    bridge_epoch_sha256: String,
    bridge_sequence: u64,
    #[serde(with = "serde_bytes")]
    capture_receipt: Vec<u8>,
    #[serde(with = "serde_bytes")]
    structure_v1: Vec<u8>,
    #[serde(with = "serde_bytes")]
    structure_v2: Vec<u8>,
    #[serde(with = "serde_bytes")]
    topology_commit: Vec<u8>,
    record_sha256: String,
}

impl LearningStructureRecordV3 {
    pub fn new(
        bridge_epoch_sha256: String,
        bridge_sequence: u64,
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure_v1: LearningRequestStructureV1,
        structure_v2: LearningRequestStructureV2,
        topology_commit: PreActionTopologyCommitV1,
    ) -> Result<Self, LearningStructureRecordErrorV3> {
        let mut record = Self {
            bridge_epoch_sha256,
            bridge_sequence,
            capture_receipt,
            structure_v1,
            structure_v2,
            topology_commit,
            record_sha256: String::new(),
        };
        record.record_sha256 = record.digest()?;
        record.validate()?;
        Ok(record)
    }

    pub fn canonical_cbor(&self) -> Result<Vec<u8>, LearningStructureRecordErrorV3> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(&self.wire()?)
            .map_err(|_| LearningStructureRecordErrorV3::Serialization)?;
        if bytes.len() > LEARNING_STRUCTURE_RECORD_MAX_BYTES_V3 {
            return Err(LearningStructureRecordErrorV3::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, LearningStructureRecordErrorV3> {
        if bytes.is_empty() || bytes.len() > LEARNING_STRUCTURE_RECORD_MAX_BYTES_V3 {
            return Err(LearningStructureRecordErrorV3::BudgetExhausted);
        }
        let wire: LearningStructureRecordWireV3 = serde_cbor::from_slice(bytes)
            .map_err(|_| LearningStructureRecordErrorV3::Serialization)?;
        if wire.schema != LEARNING_STRUCTURE_RECORD_SCHEMA_V3 {
            return Err(LearningStructureRecordErrorV3::InvalidInput);
        }
        let record = Self {
            bridge_epoch_sha256: wire.bridge_epoch_sha256,
            bridge_sequence: wire.bridge_sequence,
            capture_receipt: ProviderRequestCaptureReceiptV3::from_canonical_bytes(
                &wire.capture_receipt,
            )
            .map_err(|_| LearningStructureRecordErrorV3::InvalidReceipt)?,
            structure_v1: LearningRequestStructureV1::from_canonical_cbor(&wire.structure_v1)
                .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?,
            structure_v2: serde_json::from_slice(&wire.structure_v2)
                .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?,
            topology_commit: serde_json::from_slice(&wire.topology_commit)
                .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?,
            record_sha256: wire.record_sha256,
        };
        record.validate()?;
        if record.canonical_cbor()?.as_slice() != bytes {
            return Err(LearningStructureRecordErrorV3::InvalidInput);
        }
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), LearningStructureRecordErrorV3> {
        if self.bridge_sequence == 0 || !valid_nonzero_sha256(&self.bridge_epoch_sha256) {
            return Err(LearningStructureRecordErrorV3::InvalidInput);
        }
        self.capture_receipt
            .canonical_bytes()
            .map_err(|_| LearningStructureRecordErrorV3::InvalidReceipt)?;
        self.structure_v1
            .canonical_cbor()
            .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?;
        self.structure_v2
            .validate()
            .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?;
        self.topology_commit
            .validate()
            .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?;
        if self.structure_v1.client_intent_id_sha256() != self.structure_v2.turn_intent_id_sha256
            || self.capture_receipt.request_root_sha256().to_hex()
                != self.structure_v2.provider_capture_request_root_sha256
            || self.topology_commit.turn_intent_id_sha256 != self.structure_v2.turn_intent_id_sha256
            || self.topology_commit.provider_capture_request_root_sha256
                != self.structure_v2.provider_capture_request_root_sha256
            || self.topology_commit.topology_root_sha256
                != self
                    .structure_v2
                    .topology_root_sha256()
                    .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?
            || self.record_sha256 != self.digest()?
        {
            return Err(LearningStructureRecordErrorV3::DigestMismatch);
        }
        Ok(())
    }

    #[must_use]
    pub const fn bridge_sequence(&self) -> u64 {
        self.bridge_sequence
    }

    #[must_use]
    pub fn bridge_epoch_sha256(&self) -> &str {
        &self.bridge_epoch_sha256
    }

    #[must_use]
    pub const fn structure_v1(&self) -> &LearningRequestStructureV1 {
        &self.structure_v1
    }

    #[must_use]
    pub const fn structure_v2(&self) -> &LearningRequestStructureV2 {
        &self.structure_v2
    }

    #[must_use]
    pub const fn topology_commit(&self) -> &PreActionTopologyCommitV1 {
        &self.topology_commit
    }

    #[must_use]
    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }

    fn digest(&self) -> Result<String, LearningStructureRecordErrorV3> {
        let wire = self.wire_without_digest()?;
        let bytes =
            serde_cbor::to_vec(&wire).map_err(|_| LearningStructureRecordErrorV3::Serialization)?;
        Ok(sha256_bytes(&bytes))
    }

    fn wire(&self) -> Result<LearningStructureRecordWireV3, LearningStructureRecordErrorV3> {
        let mut wire = self.wire_without_digest()?;
        wire.record_sha256.clone_from(&self.record_sha256);
        Ok(wire)
    }

    fn wire_without_digest(
        &self,
    ) -> Result<LearningStructureRecordWireV3, LearningStructureRecordErrorV3> {
        Ok(LearningStructureRecordWireV3 {
            schema: LEARNING_STRUCTURE_RECORD_SCHEMA_V3.to_owned(),
            bridge_epoch_sha256: self.bridge_epoch_sha256.clone(),
            bridge_sequence: self.bridge_sequence,
            capture_receipt: self
                .capture_receipt
                .canonical_bytes()
                .map_err(|_| LearningStructureRecordErrorV3::InvalidReceipt)?
                .into_vec(),
            structure_v1: self
                .structure_v1
                .canonical_cbor()
                .map_err(|_| LearningStructureRecordErrorV3::InvalidStructure)?,
            structure_v2: canonical_json_bytes(&self.structure_v2)
                .map_err(|_| LearningStructureRecordErrorV3::Serialization)?,
            topology_commit: canonical_json_bytes(&self.topology_commit)
                .map_err(|_| LearningStructureRecordErrorV3::Serialization)?,
            record_sha256: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, MultiSourceEvidenceOriginV1,
        MultiSourceExtractionStatusV1, PreActionMultiSourceTopologyV1, RuntimeProjectionV3,
        Sha256CommitmentV3,
    };

    use crate::{
        LearningRequestStructureInputV1, ProviderRequestCaptureInputV3,
        seal_provider_request_capture_v3,
    };

    use super::*;

    #[test]
    fn v3_record_roundtrip_binds_v1_v2_capture_and_commit() {
        let request_root = Sha256CommitmentV3::digest_bytes(b"request");
        let capture = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: 7,
            capture_epoch_root: Sha256CommitmentV3::digest_bytes(b"capture-epoch"),
            lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"lineage"),
            request_root_sha256: request_root,
            projection: RuntimeProjectionV3::Responses,
            streaming: true,
            observed_at_unix_ms: 1,
        })
        .expect("capture");
        let turn = sha256_bytes(b"turn");
        let session = sha256_bytes(b"session");
        let v1 = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
            client_intent_id_sha256: turn.clone(),
            session_identity_sha256s: vec![session.clone()],
            request_phase_atom_ids: vec![1],
            pre_action_context_atom_ids: vec![2],
            capability_atom_ids: vec![3],
            provider_bound_turn_identity: true,
            estimated_input_tokens: 4,
            provider_payload_bytes: 7,
        })
        .expect("v1");
        let v2 = LearningRequestStructureV2 {
            schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
            turn_intent_id_sha256: turn,
            session_lineage_roots_sha256: vec![session],
            request_phase_atom_ids: vec![1],
            pre_action_context_atom_ids: vec![2],
            capability_atom_ids: vec![3],
            estimated_input_tokens: 4,
            provider_payload_bytes: 7,
            provider_capture_request_root_sha256: request_root.to_hex(),
            decidability_reason_code: "pre_action_pending".to_owned(),
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 0,
                output_part_count: 0,
                roles: Vec::new(),
                relations: Vec::new(),
            },
        };
        let commit = PreActionTopologyCommitV1::seal(
            &v2,
            MultiSourceEvidenceOriginV1::FreshLive,
            sha256_bytes(b"extractor"),
            sha256_bytes(b"config"),
            capture.capture_sequence(),
        )
        .expect("commit");
        let record =
            LearningStructureRecordV3::new(sha256_bytes(b"bridge"), 1, capture, v1, v2, commit)
                .expect("record");
        let bytes = record.canonical_cbor().expect("encode");
        assert_eq!(
            LearningStructureRecordV3::from_canonical_cbor(&bytes).expect("decode"),
            record
        );
    }
}
