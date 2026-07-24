use serde::{Deserialize, Serialize};

use crate::{PreActionMultiSourceTopologyV1, canonical_json_sha256, valid_nonzero_sha256};

pub const LEARNING_REQUEST_STRUCTURE_SCHEMA_V2: &str = "nando.learning-request-structure.v2";
pub const PRE_ACTION_TOPOLOGY_COMMIT_SCHEMA_V1: &str = "nando.pre-action-topology-commit.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningRequestStructureV2 {
    pub schema: String,
    pub turn_intent_id_sha256: String,
    #[serde(default)]
    pub request_event_id_sha256: String,
    #[serde(default)]
    pub provider_bound_turn_identity: bool,
    pub session_lineage_roots_sha256: Vec<String>,
    pub request_phase_atom_ids: Vec<u64>,
    pub pre_action_context_atom_ids: Vec<u64>,
    pub capability_atom_ids: Vec<u64>,
    pub estimated_input_tokens: u64,
    pub provider_payload_bytes: u64,
    pub provider_capture_request_root_sha256: String,
    pub decidability_reason_code: String,
    pub topology: PreActionMultiSourceTopologyV1,
}

impl LearningRequestStructureV2 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != LEARNING_REQUEST_STRUCTURE_SCHEMA_V2
            || !valid_nonzero_sha256(&self.turn_intent_id_sha256)
            || !valid_nonzero_sha256(&self.provider_capture_request_root_sha256)
            || (!self.request_event_id_sha256.is_empty()
                && !valid_nonzero_sha256(&self.request_event_id_sha256))
            || self.session_lineage_roots_sha256.len() > 4
            || self.request_phase_atom_ids.len() > 256
            || self.pre_action_context_atom_ids.len() > 256
            || self.capability_atom_ids.len() > 64
            || self.provider_payload_bytes == 0
            || self.decidability_reason_code.is_empty()
            || self
                .session_lineage_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !strictly_ordered(&self.session_lineage_roots_sha256)
            || !strictly_ordered(&self.request_phase_atom_ids)
            || !strictly_ordered(&self.pre_action_context_atom_ids)
            || !strictly_ordered(&self.capability_atom_ids)
        {
            return Err("learning_request_structure_v2_invalid");
        }
        self.topology.validate()
    }

    pub fn topology_root_sha256(&self) -> Result<String, &'static str> {
        self.validate()?;
        canonical_json_sha256(&self.topology)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceEvidenceOriginV1 {
    FreshLive,
    RecoveredArchive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreActionTopologyCommitV1 {
    pub schema: String,
    pub turn_intent_id_sha256: String,
    pub evidence_origin: MultiSourceEvidenceOriginV1,
    pub provider_capture_request_root_sha256: String,
    pub topology_root_sha256: String,
    pub extractor_root_sha256: String,
    pub config_root_sha256: String,
    pub capture_sequence: u64,
    pub commitment_root_sha256: String,
}

impl PreActionTopologyCommitV1 {
    pub fn seal(
        structure: &LearningRequestStructureV2,
        evidence_origin: MultiSourceEvidenceOriginV1,
        extractor_root_sha256: String,
        config_root_sha256: String,
        capture_sequence: u64,
    ) -> Result<Self, &'static str> {
        structure.validate()?;
        if capture_sequence == 0
            || !valid_nonzero_sha256(&extractor_root_sha256)
            || !valid_nonzero_sha256(&config_root_sha256)
        {
            return Err("pre_action_topology_commit_input_invalid");
        }
        let topology_root_sha256 = structure.topology_root_sha256()?;
        let mut commit = Self {
            schema: PRE_ACTION_TOPOLOGY_COMMIT_SCHEMA_V1.to_owned(),
            turn_intent_id_sha256: structure.turn_intent_id_sha256.clone(),
            evidence_origin,
            provider_capture_request_root_sha256: structure
                .provider_capture_request_root_sha256
                .clone(),
            topology_root_sha256,
            extractor_root_sha256,
            config_root_sha256,
            capture_sequence,
            commitment_root_sha256: String::new(),
        };
        commit.commitment_root_sha256 = commit.expected_root()?;
        Ok(commit)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != PRE_ACTION_TOPOLOGY_COMMIT_SCHEMA_V1
            || self.capture_sequence == 0
            || !valid_nonzero_sha256(&self.turn_intent_id_sha256)
            || !valid_nonzero_sha256(&self.provider_capture_request_root_sha256)
            || !valid_nonzero_sha256(&self.topology_root_sha256)
            || !valid_nonzero_sha256(&self.extractor_root_sha256)
            || !valid_nonzero_sha256(&self.config_root_sha256)
            || self.commitment_root_sha256 != self.expected_root()?
        {
            return Err("pre_action_topology_commit_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            PRE_ACTION_TOPOLOGY_COMMIT_SCHEMA_V1,
            &self.turn_intent_id_sha256,
            self.evidence_origin,
            &self.provider_capture_request_root_sha256,
            &self.topology_root_sha256,
            &self.extractor_root_sha256,
            &self.config_root_sha256,
            self.capture_sequence,
        ))
    }
}

fn strictly_ordered<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
