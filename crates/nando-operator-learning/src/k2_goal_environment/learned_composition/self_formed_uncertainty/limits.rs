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
pub const K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1: &str =
    "733b9b59fdfd7e2b5ed68461da89a27c84f04ade2e4e51ae5243dbb7175ef390";
pub const K2_UNCERTAINTY_BASELINE_SOURCE_SHA256_V1: &str =
    "febf3c09ae22de3bcf0989ce6aeb569925124a2c1b32277b1f5cb3083736974b";
pub const K2_UNCERTAINTY_MAX_CONTENT_BYTES_V1: usize = 4_096;
pub const K2_UNCERTAINTY_MAX_MANIFEST_ENTRIES_V1: usize = 4;
pub const K2_UNCERTAINTY_MAX_MANIFEST_BYTES_V1: u64 = 16_384;
pub const K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1: usize = 1_048_576;
pub const K2_UNCERTAINTY_MAX_RESIDENT_BYTES_V1: u64 = 512 * 1024 * 1024;
pub const K2_UNCERTAINTY_MAX_CASE_WALL_MS_V1: u64 = 60_000;
pub const K2_UNCERTAINTY_MAX_BATCH_WALL_MS_V1: u64 = 20 * 60 * 1_000;
pub const K2_UNCERTAINTY_MAX_RISK_UNITS_V1: u64 = 10;
pub const K2_UNCERTAINTY_MAX_COST_UNITS_V1: u64 = 10;
pub const K2_UNCERTAINTY_MAX_PLAN_PROBES_V1: usize = 2;
pub const K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1: u64 = 20;
pub const K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1: u64 = 20;
pub const K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1: usize = 32;
pub const K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS_V1: &str =
    "K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS";
pub const K2_UNCERTAINTY_ORACLE_MAX_PLANS_PER_CASE_V1: u64 = 3_211_264;
pub const K2_UNCERTAINTY_SUCCESSOR_STATIC_LEGACY_CONTROLS_V1: usize = 32;
pub const K2_UNCERTAINTY_SUCCESSOR_STATIC_V3_CONTROLS_V1: usize = 4;
pub const K2_UNCERTAINTY_SUCCESSOR_STATIC_V4_CONTROLS_V1: usize = 16;
pub const K2_UNCERTAINTY_V5_CONTROLS_V1: usize = 12;

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
pub const K2_UNCERTAINTY_CONFIRM_GENERATOR_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-generator-request.v1";
pub const K2_UNCERTAINTY_CONFIRM_GENERATOR_RESPONSE_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-generator-response.v1";
pub const K2_UNCERTAINTY_R10_AUTHORIZATION_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-r10-authorization-receipt.v1";
pub const K2_UNCERTAINTY_AUTHORIZATION_SLOT_KEY_SCHEMA_V1: &str =
    "nando.k2-self-formed-authorization-slot-key.v1";
pub const K2_UNCERTAINTY_AUTHORIZATION_SLOT_CLAIM_SCHEMA_V1: &str =
    "nando.k2-self-formed-authorization-slot-claim.v1";
pub const K2_UNCERTAINTY_CONFIRM_ATTEMPT_DESCRIPTOR_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-attempt-descriptor.v1";
pub const K2_UNCERTAINTY_CONFIRM_ATTEMPT_EVENT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-attempt-event.v1";
pub const K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-attempt-journal.v1";
pub const K2_UNCERTAINTY_CLASSIFIED_PATH_SCHEMA_V1: &str =
    "nando.k2-self-formed-classified-path.v1";
pub const K2_UNCERTAINTY_CONFIRM_PUBLIC_DENOMINATOR_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-public-denominator.v1";
pub const K2_UNCERTAINTY_CONFIRM_RESOLVER_TABLE_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-resolver-table.v1";
pub const K2_UNCERTAINTY_CONFIRM_FINAL_TRUTH_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-final-truth.v1";
pub const K2_UNCERTAINTY_CONFIRM_STORED_ARTIFACT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-stored-artifact.v1";
pub const K2_UNCERTAINTY_CONFIRM_PRIVATE_SPLIT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-private-split.v1";
pub const K2_UNCERTAINTY_CONFIRM_SPLIT_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-split-receipt.v1";
pub const K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-nonce-receipt.v1";
pub const K2_UNCERTAINTY_CONFIRM_PIPE_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-pipe-receipt.v1";
pub const K2_UNCERTAINTY_CONFIRM_OWNER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-owner-request.v1";
pub const K2_UNCERTAINTY_CONFIRM_OWNER_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-owner-receipt.v1";
pub const K2_UNCERTAINTY_PUBLIC_OWNER_SET_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-owner-set.v1";
pub const K2_UNCERTAINTY_PUBLIC_COORDINATOR_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-coordinator-request.v1";
pub const K2_UNCERTAINTY_PUBLIC_PREPARED_CASE_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-prepared-case.v1";
pub const K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-case-artifact.v1";
pub const K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-component-artifact.v1";
pub const K2_UNCERTAINTY_PUBLIC_PRECOMMIT_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-public-precommit-receipt.v1";
pub const K2_UNCERTAINTY_PRIVATE_RESOLVER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-private-resolver-request.v1";
pub const K2_UNCERTAINTY_PRIVATE_RESOLVER_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-private-resolver-receipt.v1";
pub const K2_UNCERTAINTY_CONFIRM_SAFETY_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-safety-request.v1";
pub const K2_UNCERTAINTY_CONFIRM_SAFETY_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-safety-receipt.v1";
pub const K2_UNCERTAINTY_CONFIRM_FINAL_VERIFIER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-final-verifier-request.v1";
pub const K2_UNCERTAINTY_CONFIRM_FINAL_VERIFIER_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-confirm-final-verifier-receipt.v1";
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
pub const K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1: &str = "nando.k2-self-formed-artifact-entry.v1";
pub const K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1: &str =
    "nando.k2-self-formed-probe-artifacts.v1";
pub const K2_UNCERTAINTY_BATCH_JOURNAL_EVENT_SCHEMA_V1: &str =
    "nando.k2-self-formed-batch-journal-event.v1";
pub const K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1: &str = "nando.k2-self-formed-batch-journal.v1";
pub const K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1: &str =
    "nando.k2-self-formed-baseline-summary.v1";
pub const K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1: &str =
    "nando.k2-self-formed-case-preverification.v1";
pub const K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V1: &str =
    "nando.k2-self-formed-batch-precommit.v1";
pub const K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-dispatch-receipt.v1";
pub const K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-final-verifier-request.v1";
pub const K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-self-formed-case-verification.v1";
pub const K2_UNCERTAINTY_CLOSURE_PLANNER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-closure-planner-request.v1";
pub const K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1: &str =
    "nando.k2-self-formed-completion-candidate.v1";
pub const K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1: &str = "nando.k2-self-formed-closure-census.v1";
pub const K2_UNCERTAINTY_CLOSURE_VERIFICATION_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-closure-verification-request.v1";
pub const K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-closure-verification-receipt.v1";
pub const K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1: &str = "nando.k2-self-formed-closure-plan.v1";
pub const K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V2: &str =
    "nando.k2-self-formed-case-preverification.v2";
pub const K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2: &str =
    "nando.k2-self-formed-case-precommit-entry.v2";
pub const K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2: &str =
    "nando.k2-self-formed-batch-precommit.v2";
pub const K2_UNCERTAINTY_WORKSPACE_IDENTITY_SCHEMA_V2: &str =
    "nando.k2-self-formed-workspace-identity.v2";
pub const K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2: &str =
    "nando.k2-self-formed-probe-dispatch-item.v2";
pub const K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2: &str = "nando.k2-self-formed-plan-dispatch.v2";
pub const K2_UNCERTAINTY_CASE_JOURNAL_EVENT_SCHEMA_V2: &str =
    "nando.k2-self-formed-case-journal-event.v2";
pub const K2_UNCERTAINTY_CASE_JOURNAL_SCHEMA_V2: &str = "nando.k2-self-formed-case-journal.v2";
pub const K2_UNCERTAINTY_PROBE_EXECUTION_EVIDENCE_SCHEMA_V2: &str =
    "nando.k2-self-formed-probe-execution-evidence.v2";
pub const K2_UNCERTAINTY_OBSERVATION_VECTOR_SCHEMA_V2: &str =
    "nando.k2-self-formed-observation-vector.v2";
pub const K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-final-verifier-request.v2";
pub const K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2: &str =
    "nando.k2-self-formed-case-verification.v2";
pub const K2_UNCERTAINTY_FINAL_VERIFIER_ARTIFACT_SCHEMA_V2: &str =
    "nando.k2-self-formed-final-verifier-artifact.v2";
pub const K2_UNCERTAINTY_FINAL_VERIFIER_MATERIAL_SCHEMA_V2: &str =
    "nando.k2-self-formed-final-verifier-material.v2";
pub const K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-descriptor.v1";
pub const K2_UNCERTAINTY_ORACLE_PUBLIC_BINDINGS_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-public-bindings.v1";
pub const K2_UNCERTAINTY_ORACLE_EVIDENCE_ENTRY_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-evidence-entry.v1";
pub const K2_UNCERTAINTY_ORACLE_EVIDENCE_MANIFEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-evidence-manifest.v1";
pub const K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-frontier-receipt.v1";
pub const K2_UNCERTAINTY_ORACLE_PLAN_RESULT_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-plan-result.v1";
pub const K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-enumeration.v1";
pub const K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-baseline-result.v1";
pub const K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-case-receipt.v1";
pub const K2_UNCERTAINTY_ORACLE_BASELINE_AGGREGATE_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-baseline-aggregate.v1";
pub const K2_UNCERTAINTY_ORACLE_BATCH_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-oracle-batch-receipt.v1";
pub const K2_UNCERTAINTY_CONTROL_PROCESS_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-self-formed-control-process-outcome.v1";
pub const K2_UNCERTAINTY_CONTROL_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-control-request.v1";
pub const K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-control-receipt.v1";
pub const K2_UNCERTAINTY_ROUTE_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-evaluation-route-receipt.v1";
pub const K2_UNCERTAINTY_RESOURCE_MEASUREMENTS_SCHEMA_V1: &str =
    "nando.k2-self-formed-resource-measurements.v1";
pub const K2_UNCERTAINTY_DEVELOPMENT_TERMINAL_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-development-terminal-request.v1";
pub const K2_UNCERTAINTY_SEALED_TERMINAL_REQUEST_SCHEMA_V1: &str =
    "nando.k2-self-formed-sealed-terminal-request.v1";
pub const K2_UNCERTAINTY_TERMINAL_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-self-formed-terminal-receipt.v1";
pub const K2_UNCERTAINTY_BATCH_JOURNAL_EVENTS_V1: usize =
    6 + K2_UNCERTAINTY_CONFIRM_CASES_V1 * 3 + 3;

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
