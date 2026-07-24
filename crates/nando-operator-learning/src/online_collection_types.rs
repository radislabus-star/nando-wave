use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::CollectionSynthesisExample;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionConfig {
    #[serde(default)]
    pub proof_mode: OnlineCollectionProofMode,
    pub support_rows: usize,
    pub future_rows: usize,
    pub max_buckets: usize,
    pub max_receipts_per_bucket: usize,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnlineCollectionProofMode {
    #[default]
    AdaptiveVersionSpace,
    LegacyFixedRows,
}

impl Default for OnlineCollectionConfig {
    fn default() -> Self {
        Self {
            proof_mode: OnlineCollectionProofMode::AdaptiveVersionSpace,
            support_rows: 32,
            future_rows: 32,
            max_buckets: 1_024,
            max_receipts_per_bucket: 128,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionObservation {
    pub evidence_graph_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub event_time_unix_nanos: Option<u64>,
    pub estimated_input_tokens: u64,
    pub example: CollectionSynthesisExample,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionReceipt {
    pub evidence_graph_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub event_time_unix_nanos: Option<u64>,
    pub layout_sha256: String,
    pub estimated_input_tokens: u64,
    pub verifier_pass: bool,
    #[serde(default)]
    pub request_atom_ids: Vec<u64>,
    #[serde(default)]
    pub matched_program_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub matched_program_dynamic_value_root_sha256: BTreeMap<String, String>,
    #[serde(default)]
    pub witness_class_commitment_sha256: Option<String>,
    #[serde(default)]
    pub witness_round: Option<u8>,
    #[serde(default)]
    pub witness_candidates_before: Option<usize>,
    #[serde(default)]
    pub witness_candidates_after: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionWaveCausalReport {
    pub schema: String,
    pub package_id: String,
    pub verdict: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub full_phase_correct: usize,
    pub no_phase_correct: usize,
    pub shuffled_phase_correct: usize,
    pub random_center_correct: usize,
    pub no_anti_center_correct: usize,
    pub full_phase_exact_checks: usize,
    pub no_phase_exact_checks: usize,
    pub shuffled_phase_exact_checks: usize,
    pub random_center_exact_checks: usize,
    pub no_anti_center_exact_checks: usize,
    pub wrong_accepts: usize,
    pub wave_margin_micro: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionRehydrationHint {
    pub evidence_graph_sha256: String,
    pub session_id_sha256: String,
    pub event_time_unix_nanos: Option<u64>,
    pub estimated_input_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionConsensusDiagnostic {
    pub bucket_id: String,
    pub support_rows: usize,
    pub replayable_rows: usize,
    pub canonical_programs: usize,
    pub unique_target_rows: usize,
    pub missing_target_rows: usize,
    pub ambiguous_target_rows: usize,
    pub safe_programs: usize,
    pub unsafe_disagreement_programs: usize,
    pub safely_coverable_rows: usize,
    pub max_safe_program_coverage: usize,
    pub selector_families: BTreeMap<String, usize>,
    pub candidate_present: bool,
    pub adapter_wave_programs_considered: usize,
    pub adapter_wave_programs_with_positive_and_negative: usize,
    pub adapter_wave_routes_fitted: usize,
    pub adapter_wave_candidate_valid: bool,
    pub adapter_wave_authority_pass: bool,
    pub adapter_wave_authority_rejection_counts: BTreeMap<String, usize>,
    pub adapter_wave_first_rejected_evidence_sha256: String,
    pub adapter_wave_blocker: String,
    #[serde(default)]
    pub law_subcenters_total: usize,
    #[serde(default)]
    pub best_law_subcenter_support_rows: usize,
    #[serde(default)]
    pub best_law_subcenter_programs: usize,
    #[serde(default)]
    pub best_law_subcenter_consensus: String,
    #[serde(default)]
    pub best_law_subcenter_freeze_blocker: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionBucketStatus {
    pub bucket_id: String,
    pub version_space_size: usize,
    pub support_rows: usize,
    #[serde(default)]
    pub retained_runtime_examples: usize,
    #[serde(default)]
    pub support_rows_with_runtime_examples: usize,
    #[serde(default)]
    pub abstract_law_groups: usize,
    #[serde(default)]
    pub best_abstract_law_support_rows: usize,
    #[serde(default)]
    pub best_abstract_law_replayable_support_rows: usize,
    #[serde(default)]
    pub best_abstract_law_session_ids_sha256: Vec<String>,
    #[serde(default)]
    pub best_abstract_law_missing_replay_hints: Vec<OnlineCollectionRehydrationHint>,
    #[serde(default)]
    pub best_verified_law_support_rows: usize,
    pub future_rows: usize,
    pub future_sessions: usize,
    pub future_layouts: usize,
    pub wrong_accepts: usize,
    pub frozen: bool,
    pub candidate_program_sha256: Option<String>,
    pub candidate_program_kind: Option<String>,
    pub program_kinds: Vec<String>,
    pub rejected_programs: usize,
    pub learned_anti_atoms: usize,
    pub common_request_atoms: usize,
    pub support_tokens: u64,
    pub future_tokens: u64,
    pub support_watermark_event_time_unix_nanos: Option<u64>,
    pub support_manifest_sha256: Option<String>,
    pub future_manifest_sha256: Option<String>,
    pub runtime_parity_cases: usize,
    pub admission_blocker: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionStatus {
    pub pooling_strategy_version: u32,
    pub durable_adapter_phase_evidence_rows: usize,
    pub durable_adapter_phase_pairs: usize,
    pub structural_resynthesis_pending_buckets: usize,
    pub structural_resynthesis_completed_buckets_total: u64,
    pub structural_resynthesis_failed_buckets_total: u64,
    pub observations_total: u64,
    pub duplicate_observations_total: u64,
    pub unsupported_total: u64,
    pub synthesis_error_total: u64,
    pub privacy_rejected_observations_total: u64,
    pub unsupported_dynamic_zero_total: u64,
    pub unsupported_dynamic_partial_total: u64,
    pub unsupported_dynamic_full_total: u64,
    pub unsupported_partial_with_request_source_total: u64,
    pub unsupported_partial_with_tool_source_total: u64,
    pub ambiguous_assignment_total: u64,
    pub exact_checks_total: u64,
    pub candidates_enumerated_total: u64,
    pub full_enumerations_total: u64,
    pub version_space_intersection_checks_total: u64,
    pub guard_scheduled_buckets_total: u64,
    pub guard_pruned_buckets_total: u64,
    pub unsupported_expected_in_latest_output: u64,
    pub unsupported_expected_in_any_output: u64,
    pub unsupported_without_exact_source_span: u64,
    pub unsupported_with_scalar_overlap: u64,
    pub policy_rejected_exact_matches: u64,
    pub policy_rejection_reasons: BTreeMap<String, u64>,
    pub counterexamples_total: u64,
    pub cegis_subcenters_total: u64,
    pub revoked_candidates_total: u64,
    pub late_after_freeze_total: u64,
    pub future_intent_rejected_total: u64,
    pub frozen_route_candidates_considered_total: u64,
    pub frozen_route_anti_rejected_total: u64,
    pub frozen_route_phase_rejected_total: u64,
    pub frozen_route_verifier_rejected_total: u64,
    pub frozen_route_rejection_reasons: BTreeMap<String, u64>,
    pub frozen_route_rejection_accounting_complete: bool,
    pub frozen_route_witness_pending_total: u64,
    pub frozen_route_witness_resolved_total: u64,
    pub frozen_route_irreducible_total: u64,
    pub frozen_route_applicability_abstain_total: u64,
    pub frozen_route_verifier_accounting_complete: bool,
    pub frozen_future_accepted_total: u64,
    pub frozen_route_accounting_complete: bool,
    pub exact_executable_observations_total: u64,
    pub semantic_executable_observations_total: u64,
    pub teacher_only_observations_total: u64,
    pub accounted_executable_total: u64,
    pub accounted_ambiguous_total: u64,
    pub accounted_irreducible_total: u64,
    pub legacy_unclassified_observations_total: u64,
    pub observation_accounting_complete: bool,
    pub program_pool_reuse_total: u64,
    pub program_pool_receipts_total: u64,
    pub renderer_consensus_migrated_examples_total: u64,
    pub legacy_partial_observations_discarded_total: u64,
    pub legacy_partial_buckets_discarded_total: u64,
    pub legacy_partial_receipts_discarded_total: u64,
    pub unreplayable_support_discarded_total: u64,
    pub buckets_total: usize,
    pub frozen_buckets_total: usize,
    pub pre_admission_ready_buckets_total: usize,
    pub support_receipts_unique_total: usize,
    pub future_receipts_unique_total: usize,
    pub support_tokens_unique_total: u64,
    pub future_tokens_unique_total: u64,
    pub wrong_accepts_total: usize,
    pub runtime_parity_cases_total: usize,
    pub frozen_program_kinds: BTreeMap<String, usize>,
    pub buckets: Vec<OnlineCollectionBucketStatus>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LegacyReplayRehydrationStats {
    pub session_receipts: u64,
    pub event_time_matches: u64,
    pub layout_matches: u64,
    pub token_matches: u64,
    pub verifier_matches: u64,
    pub ambiguous_matches: u64,
    pub attached_receipts: u64,
}

impl LegacyReplayRehydrationStats {
    pub fn merge(&mut self, other: Self) {
        self.session_receipts = self.session_receipts.saturating_add(other.session_receipts);
        self.event_time_matches = self
            .event_time_matches
            .saturating_add(other.event_time_matches);
        self.layout_matches = self.layout_matches.saturating_add(other.layout_matches);
        self.token_matches = self.token_matches.saturating_add(other.token_matches);
        self.verifier_matches = self.verifier_matches.saturating_add(other.verifier_matches);
        self.ambiguous_matches = self
            .ambiguous_matches
            .saturating_add(other.ambiguous_matches);
        self.attached_receipts = self
            .attached_receipts
            .saturating_add(other.attached_receipts);
    }
}
