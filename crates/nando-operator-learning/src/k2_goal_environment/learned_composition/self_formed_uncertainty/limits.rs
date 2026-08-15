use serde::{Serialize, de::DeserializeOwned};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1,
};

pub const K2_UNCERTAINTY_ACTIONS_V1: usize = 7;
pub const K2_UNCERTAINTY_PATHS_V1: usize = 4;
pub const K2_UNCERTAINTY_CONTENTS_V1: usize = 3;
pub const K2_UNCERTAINTY_EFFECTS_PER_ACTION_V1: usize = 16;
pub const K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1: usize = 3;
pub const K2_UNCERTAINTY_SUPPORT_ROWS_V1: usize = 21;
pub const K2_UNCERTAINTY_RAW_MODEL_COUNT_V1: u64 = 268_435_456;
pub const K2_UNCERTAINTY_CONFIRM_MODELS_V1: usize = 4;
pub const K2_UNCERTAINTY_STATE_COUNT_V1: usize = 256;
pub const K2_UNCERTAINTY_RAW_PROBES_V1: usize = 1_792;
pub const K2_UNCERTAINTY_RAW_PREDICTIONS_V1: usize = 7_168;
pub const K2_UNCERTAINTY_CONSISTENCY_DISPOSITIONS_V1: usize = 336;
pub const K2_UNCERTAINTY_CONFIRM_CASES_V1: usize = 16;
pub const K2_UNCERTAINTY_TOPOLOGY_FAMILIES_V1: usize = 4;
pub const K2_UNCERTAINTY_MATCHED_PAIRS_V1: usize = 8;
pub const K2_UNCERTAINTY_MIN_REPRESENTATIVES_V1: usize = 8;
pub const K2_UNCERTAINTY_MAX_REPRESENTATIVES_V1: usize = K2_UNCERTAINTY_RAW_PROBES_V1;
pub const K2_UNCERTAINTY_SELECTOR_PROBES_V1: usize = 8;
pub const K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1: usize = 256;
pub const K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1: usize = 4_096;
pub const K2_UNCERTAINTY_MAX_MANIFEST_ENTRIES_V1: usize = 4;
pub const K2_UNCERTAINTY_MAX_MANIFEST_BYTES_V1: u64 = 16_384;
pub const K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1: usize = 1_048_576;
pub const K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1: u64 = 512 * 1024 * 1024;
pub const K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1: u64 = 60_000;
pub const K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1: u64 = 20 * 60 * 1_000;
pub const K2_UNCERTAINTY_MAX_RISK_UNITS_V1: u64 = 10;
pub const K2_UNCERTAINTY_MAX_COST_UNITS_V1: u64 = 10;
pub const K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1: usize = 32;
pub const K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS_V1: &str =
    "K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS";

pub const K2_UNCERTAINTY_VOCABULARY_SCHEMA_V1: &str = "nando.k2-self-formed-domain-vocabulary.v1";
pub const K2_UNCERTAINTY_PATH_ATOM_SCHEMA_V1: &str = "nando.k2-self-formed-path-atom.v1";
pub const K2_UNCERTAINTY_CONTENT_ATOM_SCHEMA_V1: &str = "nando.k2-self-formed-content-atom.v1";
pub const K2_UNCERTAINTY_BUDGET_SCHEMA_V1: &str = "nando.k2-self-formed-budget.v1";
pub const K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-self-formed-support-outcome.v1";
pub const K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1: &str =
    "nando.k2-self-formed-support-observation.v1";
pub const K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1: &str = "nando.k2-self-formed-support-set.v1";
pub const K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-learner-request.v1";
pub const K2_UNCERTAINTY_CONSISTENCY_SCHEMA_V1: &str =
    "nando.k2-self-formed-consistency-disposition.v1";
pub const K2_UNCERTAINTY_ACTION_SURVIVORS_SCHEMA_V1: &str =
    "nando.k2-self-formed-action-survivors.v1";
pub const K2_UNCERTAINTY_SEMANTIC_SIGNATURE_SCHEMA_V1: &str =
    "nando.k2-self-formed-semantic-signature.v1";
pub const K2_UNCERTAINTY_SEMANTIC_CLASS_SCHEMA_V1: &str = "nando.k2-self-formed-semantic-class.v1";
pub const K2_UNCERTAINTY_MODEL_SET_SCHEMA_V1: &str = "nando.k2-self-formed-model-set.v1";
pub const K2_UNCERTAINTY_RISK_COST_SCHEMA_V1: &str = "nando.k2-self-formed-risk-cost.v1";
pub const K2_UNCERTAINTY_PREDICTION_WITNESS_SCHEMA_V1: &str =
    "nando.k2-self-formed-prediction-witness.v1";
pub const K2_UNCERTAINTY_RAW_PROBE_SCHEMA_V1: &str =
    "nando.k2-self-formed-raw-probe-disposition.v1";
pub const K2_UNCERTAINTY_FRONTIER_PAGE_SCHEMA_V1: &str = "nando.k2-self-formed-frontier-page.v1";
pub const K2_UNCERTAINTY_PROBE_CLASS_SCHEMA_V1: &str = "nando.k2-self-formed-probe-class.v1";
pub const K2_UNCERTAINTY_FRONTIER_SCHEMA_V1: &str = "nando.k2-self-formed-frontier.v1";
pub const K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1: &str =
    "nando.k2-self-formed-tournament-step.v1";
pub const K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1: &str = "nando.k2-self-formed-tournament.v1";
pub const K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1: &str = "nando.k2-self-formed-direct-winner.v1";
pub const K2_UNCERTAINTY_RESOURCE_TERMINAL_SCHEMA_V1: &str =
    "nando.k2-self-formed-resource-terminal.v1";
pub const K2_UNCERTAINTY_GENERATOR_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-generator-request.v1";
pub const K2_UNCERTAINTY_PUBLIC_CASE_SCHEMA_V1: &str = "nando.k2-self-formed-public-case.v1";
pub const K2_UNCERTAINTY_PRIVATE_CASE_SCHEMA_V1: &str = "nando.k2-self-formed-private-case.v1";
pub const K2_UNCERTAINTY_PUBLIC_BATCH_SCHEMA_V1: &str = "nando.k2-self-formed-public-batch.v1";
pub const K2_UNCERTAINTY_PRIVATE_BATCH_SCHEMA_V1: &str = "nando.k2-self-formed-private-batch.v1";
pub const K2_UNCERTAINTY_GENERATOR_RESPONSE_SCHEMA_V1: &str =
    "nando.k2-self-formed-generator-response.v1";
pub const K2_UNCERTAINTY_LEARNER_RESPONSE_SCHEMA_V1: &str =
    "nando.k2-self-formed-learner-response.v1";
pub const K2_UNCERTAINTY_PROBE_REQUEST_SCHEMA_V1: &str = "nando.k2-self-formed-probe-request.v1";
pub const K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1: &str = "nando.k2-self-formed-probe-output.v1";
pub const K2_UNCERTAINTY_EFFECT_ACCOUNTING_SCHEMA_V1: &str =
    "nando.k2-self-formed-effect-accounting.v1";
pub const K2_UNCERTAINTY_ROBUST_ACCOUNTING_SCHEMA_V1: &str =
    "nando.k2-self-formed-robust-accounting.v1";
pub const K2_UNCERTAINTY_SAFETY_REQUEST_SCHEMA_V1: &str = "nando.k2-self-formed-safety-request.v1";
pub const K2_UNCERTAINTY_SAFETY_RECEIPT_SCHEMA_V1: &str = "nando.k2-self-formed-safety-receipt.v1";

pub fn uncertainty_root_v1<T: Serialize>(value: &T) -> K2CompositionResultV1<String> {
    composition_root_v1(value)
}

pub fn uncertainty_bytes_v1<T: Serialize>(value: &T) -> K2CompositionResultV1<Vec<u8>> {
    let bytes = composition_bytes_v1(value)?;
    if bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_protocol_bytes_exhausted",
        ));
    }
    Ok(bytes)
}

pub fn uncertainty_decode_v1<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> K2CompositionResultV1<T> {
    if bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_protocol_bytes_exhausted",
        ));
    }
    composition_decode_v1(bytes)
}

pub(crate) fn require_denied_authority_v1(
    authority: &K2CompositionAuthorityBoundaryV1,
) -> K2CompositionResultV1<()> {
    authority.validate()
}

pub(crate) fn require_exact_len_v1(
    actual: usize,
    expected: usize,
    reason: &'static str,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid(reason))
    }
}

pub(crate) fn require_sorted_unique_v1<T: Ord>(
    values: &[T],
    reason: &'static str,
) -> K2CompositionResultV1<()> {
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid(reason))
    }
}

pub(crate) fn denied_authority_v1() -> K2CompositionAuthorityBoundaryV1 {
    K2CompositionAuthorityBoundaryV1::denied()
}
