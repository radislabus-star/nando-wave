use nando_operator_kernel::{BoundProtocolActionV3, RuntimeProjectionV3, valid_nonzero_sha256};

use super::IndependentVerifierArtifactSetV3;

pub const F6_MAX_RAW_REQUEST_BYTES_V3: usize = 256 * 1024;
pub const F6_MAX_REQUEST_TEXT_BYTES_V3: usize = 16 * 1024;
pub const F6_MAX_JSON_NODES_V3: usize = 4_096;
pub const F6_MAX_ROLE_CANDIDATES_V3: usize = 64;
pub const F6_MAX_RELATIONS_V3: usize = 256;
pub const F6_MAX_CAPABILITIES_V3: usize = 64;
pub const F6_MAX_MODES_V3: usize = 32;
pub const F6_MAX_CANDIDATE_PATHS_V3: usize = 2_048;
pub const F6_MAX_ACTOR_OUTPUT_BYTES_V3: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndependentVerifierBudgetV3 {
    pub max_raw_request_bytes: usize,
    pub max_request_text_bytes: usize,
    pub max_json_nodes: usize,
    pub max_role_candidates: usize,
    pub max_relations: usize,
    pub max_capabilities: usize,
    pub max_modes: usize,
    pub max_candidate_paths: usize,
    pub max_actor_output_bytes: usize,
}

impl Default for IndependentVerifierBudgetV3 {
    fn default() -> Self {
        Self {
            max_raw_request_bytes: F6_MAX_RAW_REQUEST_BYTES_V3,
            max_request_text_bytes: F6_MAX_REQUEST_TEXT_BYTES_V3,
            max_json_nodes: F6_MAX_JSON_NODES_V3,
            max_role_candidates: F6_MAX_ROLE_CANDIDATES_V3,
            max_relations: F6_MAX_RELATIONS_V3,
            max_capabilities: F6_MAX_CAPABILITIES_V3,
            max_modes: F6_MAX_MODES_V3,
            max_candidate_paths: F6_MAX_CANDIDATE_PATHS_V3,
            max_actor_output_bytes: F6_MAX_ACTOR_OUTPUT_BYTES_V3,
        }
    }
}

impl IndependentVerifierBudgetV3 {
    pub(super) const fn valid(self) -> bool {
        self.max_raw_request_bytes > 0
            && self.max_raw_request_bytes <= F6_MAX_RAW_REQUEST_BYTES_V3
            && self.max_request_text_bytes > 0
            && self.max_request_text_bytes <= F6_MAX_REQUEST_TEXT_BYTES_V3
            && self.max_json_nodes > 0
            && self.max_json_nodes <= F6_MAX_JSON_NODES_V3
            && self.max_role_candidates > 0
            && self.max_role_candidates <= F6_MAX_ROLE_CANDIDATES_V3
            && self.max_relations > 0
            && self.max_relations <= F6_MAX_RELATIONS_V3
            && self.max_capabilities > 0
            && self.max_capabilities <= F6_MAX_CAPABILITIES_V3
            && self.max_modes > 0
            && self.max_modes <= F6_MAX_MODES_V3
            && self.max_candidate_paths > 0
            && self.max_candidate_paths <= F6_MAX_CANDIDATE_PATHS_V3
            && self.max_actor_output_bytes > 0
            && self.max_actor_output_bytes <= F6_MAX_ACTOR_OUTPUT_BYTES_V3
    }
}

pub struct IndependentVerifierInputV3<'a> {
    request_sha256: &'a str,
    projection: RuntimeProjectionV3,
    provider_payload_bytes: &'a [u8],
    artifact_set: &'a IndependentVerifierArtifactSetV3,
    actor_action: &'a BoundProtocolActionV3,
    actor_output: &'a str,
}

impl<'a> IndependentVerifierInputV3<'a> {
    pub fn new(
        request_sha256: &'a str,
        projection: RuntimeProjectionV3,
        provider_payload_bytes: &'a [u8],
        artifact_set: &'a IndependentVerifierArtifactSetV3,
        actor_action: &'a BoundProtocolActionV3,
        actor_output: &'a str,
    ) -> Result<Self, IndependentVerifierInputErrorV3> {
        if !valid_nonzero_sha256(request_sha256) {
            return Err(IndependentVerifierInputErrorV3::InvalidInput);
        }
        Ok(Self {
            request_sha256,
            projection,
            provider_payload_bytes,
            artifact_set,
            actor_action,
            actor_output,
        })
    }

    pub(super) const fn request_sha256(&self) -> &str {
        self.request_sha256
    }

    pub(super) const fn projection(&self) -> RuntimeProjectionV3 {
        self.projection
    }

    pub(super) const fn provider_payload_bytes(&self) -> &[u8] {
        self.provider_payload_bytes
    }

    pub(super) const fn artifact_set(&self) -> &IndependentVerifierArtifactSetV3 {
        self.artifact_set
    }

    pub(super) const fn actor_action(&self) -> &BoundProtocolActionV3 {
        self.actor_action
    }

    pub(super) const fn actor_output(&self) -> &str {
        self.actor_output
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndependentVerifierInputErrorV3 {
    InvalidInput,
}
