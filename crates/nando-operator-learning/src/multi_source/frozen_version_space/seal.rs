use nando_operator_kernel::{
    CANONICAL_OPERATOR_IR_V1_SCHEMA, canonical_json_sha256, sha256_bytes, valid_nonzero_sha256,
};
use nando_operator_proof::independent_verifier_v3::INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3;

use crate::{OperatorIdentificationMachineV1, OperatorIdentificationStateV1};

use super::super::identification::MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2;
use super::types::{
    ContractDigestV1, FrozenVersionSpaceContractV1, FrozenVersionSpaceEnvelopeV1,
    MAX_ENVELOPE_BYTES, MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1,
    MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1, MS3_PRE_FREEZE_BUFFER_EXCLUDED,
    MS3_T1_GRAMMAR_SCHEMA_V1, Ms3FrozenVersionSpaceErrorV1, Ms3FrozenVersionSpaceStateV1,
    Ms3VersionSpaceVersionsV1, PreparedMs3VersionSpaceV1, PreparedStateV1,
};

impl PreparedMs3VersionSpaceV1 {
    pub fn seal(
        mut self,
        contract_watermark: u64,
        versions: Ms3VersionSpaceVersionsV1,
    ) -> Result<FrozenVersionSpaceEnvelopeV1, Ms3FrozenVersionSpaceErrorV1> {
        if contract_watermark < self.support_watermark
            || versions.compiler_version.is_empty()
            || versions.vm_abi.is_empty()
        {
            return Err(Ms3FrozenVersionSpaceErrorV1::InvalidContractWatermark);
        }
        let future_min_sequence = contract_watermark
            .checked_add(1)
            .ok_or(Ms3FrozenVersionSpaceErrorV1::InvalidContractWatermark)?;
        let state = match self.state {
            PreparedStateV1::ZeroClasses { reason, blocker } => {
                Ms3FrozenVersionSpaceStateV1::ZeroClasses { reason, blocker }
            }
            PreparedStateV1::Unique {
                semantic_class_root_sha256,
                canonical_program,
                protocol_mode_root_sha256,
            } => {
                let scope = canonical_json_sha256(&(
                    "nando.ms3-version-space-applicability-scope.v1",
                    protocol_mode_root_sha256.as_str(),
                    semantic_class_root_sha256.as_str(),
                    nando_operator_kernel::response_program_required_routing_atom_ids(
                        &canonical_program,
                    ),
                ))
                .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
                let freeze = self
                    .machine
                    .freeze_candidate(future_min_sequence, scope)
                    .map_err(|error| Ms3FrozenVersionSpaceErrorV1::Freeze(error.to_string()))?;
                Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen {
                    semantic_class_root_sha256,
                    candidate_freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
                }
            }
            PreparedStateV1::Ambiguous { semantic_classes } => {
                Ms3FrozenVersionSpaceStateV1::Ambiguous { semantic_classes }
            }
        };
        let machine_checkpoint = self
            .machine
            .checkpoint_bytes()
            .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
        let machine_checkpoint_sha256 = sha256_bytes(&machine_checkpoint);
        let grammar_root_sha256 = canonical_json_sha256(&(
            MS3_T1_GRAMMAR_SCHEMA_V1,
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            CANONICAL_OPERATOR_IR_V1_SCHEMA,
            4_096_u64,
        ))
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
        let mut contract = FrozenVersionSpaceContractV1 {
            schema: MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            acquisition_report_root_sha256: self.acquisition_report_root_sha256,
            linked_receipt_root_sha256: self.linked_receipt.receipt_root_sha256,
            topology_root_sha256: self.linked_receipt.topology_commitment_root_sha256,
            frame_root_sha256: self.linked_receipt.completed_frame_root_sha256,
            terminal_root_sha256: self.linked_receipt.terminal_receipt_root_sha256,
            transport_binding_root_sha256: self.linked_receipt.transport_binding_root_sha256,
            session_lineage_sha256: self.linked_receipt.session_lineage_sha256,
            session_id_sha256: self.linked_receipt.session_id_sha256,
            turn_intent_id_sha256: self.linked_receipt.turn_intent_id_sha256,
            request_event_id_sha256: self.linked_receipt.request_event_id_sha256,
            action_event_id_sha256: self.linked_receipt.action_event_id_sha256,
            extractor_schema: self.extractor_schema,
            extractor_version: self.extractor_version,
            generator_version: MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2.to_owned(),
            grammar_root_sha256,
            compiler_version: versions.compiler_version,
            vm_abi: versions.vm_abi,
            verifier_schema: INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3.to_owned(),
            support_rows_root_sha256: self.support_rows_root_sha256,
            support_watermark: self.support_watermark,
            contract_watermark,
            future_min_sequence,
            pre_freeze_buffer_sequence_span: contract_watermark
                .saturating_sub(self.support_watermark),
            pre_freeze_buffer_disposition: MS3_PRE_FREEZE_BUFFER_EXCLUDED.to_owned(),
            candidate_program_roots_sha256: self.candidate_program_roots_sha256,
            semantic_class_roots_sha256: self.semantic_class_roots_sha256,
            quotient_root_sha256: self.quotient_root_sha256,
            class_predictions_root_sha256: self.class_predictions_root_sha256,
            machine_checkpoint_sha256,
            machine_checkpoint_bytes: machine_checkpoint.len(),
            passive_probe: self.passive_probe,
            state,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        let mut envelope = FrozenVersionSpaceEnvelopeV1 {
            schema: MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1.to_owned(),
            envelope_root_sha256: String::new(),
            contract,
            machine_checkpoint,
        };
        envelope.envelope_root_sha256 = envelope.expected_root()?;
        envelope.validate()?;
        Ok(envelope)
    }
}

impl FrozenVersionSpaceContractV1 {
    pub fn expected_root(&self) -> Result<String, Ms3FrozenVersionSpaceErrorV1> {
        canonical_json_sha256(&ContractDigestV1 {
            schema: MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1,
            acquisition_report_root_sha256: &self.acquisition_report_root_sha256,
            linked_receipt_root_sha256: &self.linked_receipt_root_sha256,
            topology_root_sha256: &self.topology_root_sha256,
            frame_root_sha256: &self.frame_root_sha256,
            terminal_root_sha256: &self.terminal_root_sha256,
            transport_binding_root_sha256: &self.transport_binding_root_sha256,
            session_lineage_sha256: &self.session_lineage_sha256,
            session_id_sha256: &self.session_id_sha256,
            turn_intent_id_sha256: &self.turn_intent_id_sha256,
            request_event_id_sha256: &self.request_event_id_sha256,
            action_event_id_sha256: &self.action_event_id_sha256,
            extractor_schema: &self.extractor_schema,
            extractor_version: &self.extractor_version,
            generator_version: &self.generator_version,
            grammar_root_sha256: &self.grammar_root_sha256,
            compiler_version: &self.compiler_version,
            vm_abi: &self.vm_abi,
            verifier_schema: &self.verifier_schema,
            support_rows_root_sha256: &self.support_rows_root_sha256,
            support_watermark: self.support_watermark,
            contract_watermark: self.contract_watermark,
            future_min_sequence: self.future_min_sequence,
            pre_freeze_buffer_sequence_span: self.pre_freeze_buffer_sequence_span,
            pre_freeze_buffer_disposition: &self.pre_freeze_buffer_disposition,
            candidate_program_roots_sha256: &self.candidate_program_roots_sha256,
            semantic_class_roots_sha256: &self.semantic_class_roots_sha256,
            quotient_root_sha256: &self.quotient_root_sha256,
            class_predictions_root_sha256: &self.class_predictions_root_sha256,
            machine_checkpoint_sha256: &self.machine_checkpoint_sha256,
            machine_checkpoint_bytes: self.machine_checkpoint_bytes,
            passive_probe: &self.passive_probe,
            state: &self.state,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)
    }

    #[must_use]
    pub fn future_collector_kind(&self) -> Option<&'static str> {
        match self.state {
            Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen { .. } => Some("independent_future"),
            Ms3FrozenVersionSpaceStateV1::Ambiguous { .. } => Some("distinguishing_observation"),
            Ms3FrozenVersionSpaceStateV1::ZeroClasses { .. } => None,
        }
    }

    fn validate(&self) -> Result<(), Ms3FrozenVersionSpaceErrorV1> {
        let roots = [
            self.contract_root_sha256.as_str(),
            self.acquisition_report_root_sha256.as_str(),
            self.linked_receipt_root_sha256.as_str(),
            self.topology_root_sha256.as_str(),
            self.frame_root_sha256.as_str(),
            self.terminal_root_sha256.as_str(),
            self.transport_binding_root_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.session_id_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.request_event_id_sha256.as_str(),
            self.action_event_id_sha256.as_str(),
            self.grammar_root_sha256.as_str(),
            self.support_rows_root_sha256.as_str(),
            self.quotient_root_sha256.as_str(),
            self.class_predictions_root_sha256.as_str(),
            self.machine_checkpoint_sha256.as_str(),
        ];
        let state_valid = match &self.state {
            Ms3FrozenVersionSpaceStateV1::ZeroClasses { blocker, .. } => {
                !blocker.is_empty() && self.semantic_class_roots_sha256.is_empty()
            }
            Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen {
                semantic_class_root_sha256,
                candidate_freeze_root_sha256,
            } => {
                self.semantic_class_roots_sha256.len() == 1
                    && self.semantic_class_roots_sha256[0] == *semantic_class_root_sha256
                    && valid_nonzero_sha256(candidate_freeze_root_sha256)
            }
            Ms3FrozenVersionSpaceStateV1::Ambiguous { semantic_classes } => {
                *semantic_classes > 1
                    && *semantic_classes == self.semantic_class_roots_sha256.len()
                    && self.passive_probe.is_some()
            }
        };
        if self.schema != MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.extractor_schema.is_empty()
            || self.extractor_version.is_empty()
            || self.generator_version != MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2
            || self.compiler_version.is_empty()
            || self.vm_abi.is_empty()
            || self.verifier_schema != INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3
            || self.support_watermark == 0
            || self.contract_watermark < self.support_watermark
            || self.future_min_sequence != self.contract_watermark.saturating_add(1)
            || self.pre_freeze_buffer_sequence_span
                != self
                    .contract_watermark
                    .saturating_sub(self.support_watermark)
            || self.pre_freeze_buffer_disposition != MS3_PRE_FREEZE_BUFFER_EXCLUDED
            || self.machine_checkpoint_bytes == 0
            || !sorted_unique_valid(&self.candidate_program_roots_sha256)
            || !sorted_unique_valid(&self.semantic_class_roots_sha256)
            || !state_valid
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.contract_root_sha256 != self.expected_root()?
        {
            return Err(Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope);
        }
        Ok(())
    }
}

impl FrozenVersionSpaceEnvelopeV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, Ms3FrozenVersionSpaceErrorV1> {
        self.validate()?;
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
        if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(Ms3FrozenVersionSpaceErrorV1::Serialization);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, Ms3FrozenVersionSpaceErrorV1> {
        if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope);
        }
        let envelope: Self = serde_cbor::from_slice(bytes)
            .map_err(|_| Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope)?;
        envelope.validate()?;
        if envelope.canonical_bytes()? != bytes {
            return Err(Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope);
        }
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), Ms3FrozenVersionSpaceErrorV1> {
        self.contract.validate()?;
        let restored =
            OperatorIdentificationMachineV1::from_checkpoint_bytes(&self.machine_checkpoint)
                .map_err(|_| Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope)?;
        let restored_bytes = restored
            .checkpoint_bytes()
            .map_err(|_| Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope)?;
        let machine_state_valid = matches!(
            (&self.contract.state, restored.state()),
            (
                Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen { .. },
                Ok(OperatorIdentificationStateV1::Frozen { .. }),
            ) | (
                Ms3FrozenVersionSpaceStateV1::Ambiguous { .. },
                Ok(OperatorIdentificationStateV1::Ambiguous { .. }),
            ) | (Ms3FrozenVersionSpaceStateV1::ZeroClasses { .. }, _)
        );
        if self.schema != MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1
            || self.machine_checkpoint.len() != self.contract.machine_checkpoint_bytes
            || sha256_bytes(&self.machine_checkpoint) != self.contract.machine_checkpoint_sha256
            || restored_bytes != self.machine_checkpoint
            || !machine_state_valid
            || self.envelope_root_sha256 != self.expected_root()?
        {
            return Err(Ms3FrozenVersionSpaceErrorV1::InvalidEnvelope);
        }
        Ok(())
    }

    #[must_use]
    pub fn machine_checkpoint(&self) -> &[u8] {
        &self.machine_checkpoint
    }

    pub(super) fn expected_root(&self) -> Result<String, Ms3FrozenVersionSpaceErrorV1> {
        canonical_json_sha256(&(
            MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1,
            self.contract.contract_root_sha256.as_str(),
            self.contract.machine_checkpoint_sha256.as_str(),
        ))
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)
    }
}

fn sorted_unique_valid(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}
