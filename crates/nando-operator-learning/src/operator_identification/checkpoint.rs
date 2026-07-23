use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    OperatorGenerationManifestV3, ProgramSemanticClassDescriptorV1, canonical_json_bytes,
    canonical_json_sha256,
};
use serde::{Deserialize, Serialize};

use crate::{GenerationEvidenceLedgerV3, VersionSpaceArena};

use super::{
    CandidateFreezeReceiptV1, OperatorIdentificationErrorV1, OperatorIdentificationMachineV1,
    OperatorObservationV1,
};

pub const OPERATOR_IDENTIFICATION_CHECKPOINT_SCHEMA_V1: &str =
    "nando.operator-identification-checkpoint.v1";
pub const OPERATOR_IDENTIFICATION_CHECKPOINT_MAX_BYTES_V1: usize = 8 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckpointPayloadV1 {
    manifest_bytes: Vec<u8>,
    arena: VersionSpaceArena,
    descriptors: BTreeMap<String, ProgramSemanticClassDescriptorV1>,
    support: Vec<OperatorObservationV1>,
    last_capture_sequence: u64,
    hard_contradiction: bool,
    zero_gain_observations: usize,
    total_information_gain: usize,
    freeze: Option<CandidateFreezeReceiptV1>,
    evidence_ledger_bytes: Option<Vec<u8>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckpointWireV1 {
    schema: String,
    payload: CheckpointPayloadV1,
    checkpoint_root_sha256: String,
    execution_authority: bool,
}

impl OperatorIdentificationMachineV1 {
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>, OperatorIdentificationErrorV1> {
        let payload = CheckpointPayloadV1 {
            manifest_bytes: self
                .manifest
                .canonical_bytes()
                .map_err(|_| OperatorIdentificationErrorV1::Serialization)?,
            arena: self.arena.clone(),
            descriptors: self.descriptors.clone(),
            support: self.support.clone(),
            last_capture_sequence: self.last_capture_sequence,
            hard_contradiction: self.hard_contradiction,
            zero_gain_observations: self.zero_gain_observations,
            total_information_gain: self.total_information_gain,
            freeze: self.freeze.clone(),
            evidence_ledger_bytes: self
                .evidence_ledger
                .as_ref()
                .map(GenerationEvidenceLedgerV3::canonical_bytes)
                .transpose()
                .map_err(|_| OperatorIdentificationErrorV1::Serialization)?,
        };
        let checkpoint_root_sha256 = canonical_json_sha256(&(
            OPERATOR_IDENTIFICATION_CHECKPOINT_SCHEMA_V1,
            &payload,
            false,
        ))
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        let bytes = canonical_json_bytes(&CheckpointWireV1 {
            schema: OPERATOR_IDENTIFICATION_CHECKPOINT_SCHEMA_V1.to_owned(),
            payload,
            checkpoint_root_sha256,
            execution_authority: false,
        })
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        if bytes.len() > OPERATOR_IDENTIFICATION_CHECKPOINT_MAX_BYTES_V1 {
            return Err(OperatorIdentificationErrorV1::SupportBudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_checkpoint_bytes(bytes: &[u8]) -> Result<Self, OperatorIdentificationErrorV1> {
        if bytes.len() > OPERATOR_IDENTIFICATION_CHECKPOINT_MAX_BYTES_V1 {
            return Err(OperatorIdentificationErrorV1::SupportBudgetExhausted);
        }
        let wire: CheckpointWireV1 = serde_json::from_slice(bytes)
            .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        if wire.schema != OPERATOR_IDENTIFICATION_CHECKPOINT_SCHEMA_V1 || wire.execution_authority {
            return Err(OperatorIdentificationErrorV1::Serialization);
        }
        let expected_root = canonical_json_sha256(&(
            OPERATOR_IDENTIFICATION_CHECKPOINT_SCHEMA_V1,
            &wire.payload,
            false,
        ))
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        if expected_root != wire.checkpoint_root_sha256 {
            return Err(OperatorIdentificationErrorV1::Serialization);
        }
        let manifest =
            OperatorGenerationManifestV3::from_canonical_bytes(&wire.payload.manifest_bytes)
                .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        let evidence_ledger = wire
            .payload
            .evidence_ledger_bytes
            .as_deref()
            .map(|ledger| GenerationEvidenceLedgerV3::from_canonical_bytes(ledger, &manifest))
            .transpose()
            .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        if wire.payload.freeze.is_some() != evidence_ledger.is_some() {
            return Err(OperatorIdentificationErrorV1::Freeze);
        }
        if let (Some(freeze), Some(ledger)) = (&wire.payload.freeze, &evidence_ledger) {
            freeze
                .validate()
                .map_err(|_| OperatorIdentificationErrorV1::Freeze)?;
            if freeze.generation_id_sha256() != manifest.generation_id_sha256()
                || freeze.support_evidence_root_sha256()
                    != ledger
                        .evidence_root_sha256()
                        .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?
            {
                return Err(OperatorIdentificationErrorV1::Freeze);
            }
        }

        let mut observation_roots = BTreeSet::new();
        let mut event_roots = BTreeSet::new();
        let mut request_roots = BTreeSet::new();
        let mut receipt_roots = BTreeSet::new();
        let mut previous_sequence = 0;
        for observation in &wire.payload.support {
            observation
                .validate()
                .map_err(|_| OperatorIdentificationErrorV1::InvalidObservation)?;
            if observation.capture_sequence() <= previous_sequence
                || !observation_roots.insert(observation.observation_id_sha256().to_owned())
                || !event_roots.insert(observation.event_root_sha256().to_owned())
                || !request_roots.insert(observation.request_root_sha256().to_owned())
                || !receipt_roots.insert(observation.verifier_receipt_root_sha256().to_owned())
            {
                return Err(OperatorIdentificationErrorV1::DuplicateObservation);
            }
            previous_sequence = observation.capture_sequence();
        }
        if previous_sequence != wire.payload.last_capture_sequence {
            return Err(OperatorIdentificationErrorV1::InvalidSequence);
        }
        for descriptor in wire.payload.descriptors.values() {
            descriptor
                .validate()
                .map_err(|_| OperatorIdentificationErrorV1::ConflictingSemanticClass)?;
        }
        let machine = Self {
            manifest,
            arena: wire.payload.arena,
            descriptors: wire.payload.descriptors,
            support: wire.payload.support,
            observation_roots,
            event_roots,
            request_roots,
            receipt_roots,
            last_capture_sequence: wire.payload.last_capture_sequence,
            hard_contradiction: wire.payload.hard_contradiction,
            zero_gain_observations: wire.payload.zero_gain_observations,
            total_information_gain: wire.payload.total_information_gain,
            freeze: wire.payload.freeze,
            evidence_ledger,
        };
        if machine.checkpoint_bytes()? != bytes {
            return Err(OperatorIdentificationErrorV1::Serialization);
        }
        Ok(machine)
    }
}
