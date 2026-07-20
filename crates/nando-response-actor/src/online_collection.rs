use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nando_core::wave::{
    PhaseCenterCell, phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids,
};

use crate::collection_synthesis::{
    canonical_direct_response_program, enumerate_source_neutral_response_programs_with_coverage,
    enumerate_source_neutral_structural_response_programs, response_law_key,
};
use crate::{
    AstProgramKind, CollectionSynthesisExample, DurableRuntimeParityReceipt,
    ResponseAdapterWaveConsensus, ResponseAdapterWaveRoute, ResponseAdapterWaveSubcenter,
    ResponseConsensusVariant, ResponseExecutionStatus, ResponsePackage, ResponsePackageOrigin,
    ResponsePackageProof, ResponsePackageState, ResponseProgram,
    build_durable_runtime_parity_receipt, canonical_json_sha256,
    diagnose_response_dynamic_coverage, enumerate_source_neutral_response_programs,
    execute_response, is_learned_bounded_response_program, is_privacy_safe_online_response_program,
    is_source_neutral_response_program,
    package::{
        request_phase_atom_ids, response_pre_action_context_atom_ids,
        response_program_external_verifier_schema,
    },
    response_program_authority_matches_example, response_program_exactly_matches_example,
    response_program_kind, response_program_required_routing_atom_ids, sha256_bytes,
    source_neutral_verifier_for_program, verify_response_independently,
};

const ONLINE_COLLECTION_SCHEMA_V1: &str = "nando.online-collection-version-space.v1";
const ONLINE_COLLECTION_SCHEMA_V2: &str = "nando.online-collection-program-pools.v2";
const ONLINE_COLLECTION_SCHEMA_V3: &str = "nando.online-outcome-version-space.v3";
const ONLINE_COLLECTION_POOLING_STRATEGY_V3: u32 = 3;
const ONLINE_COLLECTION_POOLING_STRATEGY_V4: u32 = 4;
const ONLINE_COLLECTION_POOLING_STRATEGY_V5: u32 = 5;
const ONLINE_COLLECTION_POOLING_STRATEGY_V6: u32 = 6;
const ONLINE_COLLECTION_POOLING_STRATEGY_V7: u32 = 7;
const ONLINE_COLLECTION_POOLING_STRATEGY_V8: u32 = 8;
const ONLINE_COLLECTION_POOLING_STRATEGY_V9: u32 = 9;
const ONLINE_COLLECTION_POOLING_STRATEGY_V10: u32 = 10;
const ONLINE_COLLECTION_POOLING_STRATEGY_V12: u32 = 12;
const ONLINE_COLLECTION_POOLING_STRATEGY_V13: u32 = 13;
const ONLINE_COLLECTION_POOLING_STRATEGY_V14: u32 = 14;
const ONLINE_COLLECTION_POOLING_STRATEGY_V15: u32 = 15;
const ONLINE_COLLECTION_POOLING_STRATEGY_V16: u32 = 16;
const ONLINE_COLLECTION_POOLING_STRATEGY_V17: u32 = 17;
const ONLINE_COLLECTION_POOLING_STRATEGY_V18: u32 = 18;
const ONLINE_COLLECTION_POOLING_STRATEGY_V19: u32 = 19;
const ONLINE_COLLECTION_POOLING_STRATEGY_V20: u32 = 20;
const ONLINE_COLLECTION_POOLING_STRATEGY_V21: u32 = 21;
const ONLINE_COLLECTION_POOLING_STRATEGY_V22: u32 = 22;
const ONLINE_COLLECTION_POOLING_STRATEGY_V23: u32 = 23;
const ONLINE_COLLECTION_POOLING_STRATEGY_V24: u32 = 24;
const ONLINE_COLLECTION_POOLING_STRATEGY_V25: u32 = 25;
const ONLINE_COLLECTION_POOLING_STRATEGY_V26: u32 = 26;
const ONLINE_COLLECTION_POOLING_STRATEGY_V27: u32 = 27;
const ONLINE_COLLECTION_POOLING_STRATEGY_V28: u32 = 28;
const ONLINE_COLLECTION_POOLING_STRATEGY_V29: u32 = 29;
const ONLINE_COLLECTION_POOLING_STRATEGY_V31: u32 = 31;
const ONLINE_COLLECTION_POOLING_STRATEGY_V32: u32 = 32;
const ONLINE_COLLECTION_POOLING_STRATEGY_V33: u32 = 33;
const ONLINE_COLLECTION_POOLING_STRATEGY_V34: u32 = 34;
const ONLINE_COLLECTION_POOLING_STRATEGY_V35: u32 = 35;
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2: &[u8; 4] = b"NCO2";
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3: &[u8; 4] = b"NCO3";
const MAX_PERSISTED_PARITY_BYTES_PER_BUCKET: usize = 2 * 1024 * 1024;
const MAX_NEW_ADAPTERS_PER_OBSERVATION: usize = 8;
const MAX_UNFROZEN_ROUTE_BUCKETS: usize = 8;
const MAX_UNFROZEN_ROUTE_PROGRAMS: usize = 8;
const MAX_TARGETED_REHYDRATION_HINTS: usize = 128;
const MAX_DURABLE_ADAPTER_PHASE_ATOMS: usize = 64;
const MAX_ACTIVE_WITNESS_ROUNDS: u8 = 4;
const MAX_EXACT_RECEIPT_MIGRATION_SEEDS_PER_BUCKET: usize = 8;
const MAX_STRUCTURAL_RESYNTHESIS_SEEDS_PER_BUCKET: usize = 2;
const MIN_APPLICABILITY_NEGATIVE_SESSIONS: usize = 3;
const MAX_APPLICABILITY_NEGATIVE_ATOMS_PER_BUCKET: usize = 64;
type ArchetypeProgramPool = (String, BTreeMap<String, ResponseProgram>);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineCollectionConfig {
    pub support_rows: usize,
    pub future_rows: usize,
    pub max_buckets: usize,
    pub max_receipts_per_bucket: usize,
}

impl Default for OnlineCollectionConfig {
    fn default() -> Self {
        Self {
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
pub struct OnlineCollectionAdmissionCandidate {
    pub package: ResponsePackage,
    pub bucket_id: String,
    pub program_sha256: String,
    pub support_watermark_event_time_unix_nanos: u64,
    pub support_manifest_sha256: String,
    pub future_manifest_sha256: String,
    pub causal_report: OnlineCollectionWaveCausalReport,
    pub support_receipts: Vec<OnlineCollectionReceipt>,
    pub future_receipts: Vec<OnlineCollectionReceipt>,
    #[serde(default)]
    pub runtime_parity_cases: Vec<crate::RuntimeParityCase>,
    #[serde(default)]
    pub durable_runtime_parity_receipts: Vec<DurableRuntimeParityReceipt>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OnlineCollectionBucket {
    bucket_id: String,
    #[serde(default)]
    archetype_id: String,
    programs: BTreeMap<String, ResponseProgram>,
    #[serde(default)]
    common_request_atom_ids: BTreeSet<u64>,
    support: Vec<OnlineCollectionReceipt>,
    future: Vec<OnlineCollectionReceipt>,
    #[serde(default)]
    runtime_examples: BTreeMap<String, CollectionSynthesisExample>,
    #[serde(default)]
    durable_adapter_phase_atoms: BTreeMap<String, BTreeMap<String, Vec<u64>>>,
    #[serde(default)]
    durable_runtime_parity_receipts: BTreeMap<String, DurableRuntimeParityReceipt>,
    frozen_program_sha256: Option<String>,
    #[serde(default)]
    support_watermark_event_time_unix_nanos: Option<u64>,
    #[serde(default)]
    support_manifest_sha256: Option<String>,
    #[serde(default)]
    rejected_program_sha256: BTreeSet<String>,
    #[serde(default)]
    learned_anti_atom_ids: BTreeSet<u64>,
    wrong_accepts: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OnlineCollectionCheckpoint {
    schema: String,
    #[serde(default)]
    pooling_strategy_version: u32,
    #[serde(default)]
    structural_resynthesis_pending_bucket_ids: BTreeSet<String>,
    #[serde(default)]
    structural_resynthesis_completed_buckets_total: u64,
    #[serde(default)]
    structural_resynthesis_failed_buckets_total: u64,
    config: OnlineCollectionConfig,
    observations_total: u64,
    #[serde(default)]
    duplicate_observations_total: u64,
    #[serde(default)]
    observed_evidence_graph_sha256: BTreeSet<String>,
    unsupported_total: u64,
    #[serde(default)]
    synthesis_error_total: u64,
    #[serde(default)]
    privacy_rejected_observations_total: u64,
    #[serde(default)]
    unsupported_dynamic_zero_total: u64,
    #[serde(default)]
    unsupported_dynamic_partial_total: u64,
    #[serde(default)]
    unsupported_dynamic_full_total: u64,
    #[serde(default)]
    unsupported_partial_with_request_source_total: u64,
    #[serde(default)]
    unsupported_partial_with_tool_source_total: u64,
    ambiguous_assignment_total: u64,
    exact_checks_total: u64,
    candidates_enumerated_total: u64,
    #[serde(default)]
    full_enumerations_total: u64,
    #[serde(default)]
    version_space_intersection_checks_total: u64,
    #[serde(default)]
    guard_scheduled_buckets_total: u64,
    #[serde(default)]
    guard_pruned_buckets_total: u64,
    #[serde(default)]
    unsupported_expected_in_latest_output: u64,
    #[serde(default)]
    unsupported_expected_in_any_output: u64,
    #[serde(default)]
    unsupported_without_exact_source_span: u64,
    #[serde(default)]
    unsupported_with_scalar_overlap: u64,
    #[serde(default)]
    policy_rejected_exact_matches: u64,
    #[serde(default)]
    policy_rejection_reasons: BTreeMap<String, u64>,
    #[serde(default)]
    counterexamples_total: u64,
    #[serde(default)]
    cegis_subcenters_total: u64,
    #[serde(default)]
    revoked_candidates_total: u64,
    #[serde(default)]
    late_after_freeze_total: u64,
    #[serde(default)]
    future_intent_rejected_total: u64,
    #[serde(default)]
    frozen_route_candidates_considered_total: u64,
    #[serde(default)]
    frozen_route_anti_rejected_total: u64,
    #[serde(default)]
    frozen_route_phase_rejected_total: u64,
    #[serde(default)]
    frozen_route_verifier_rejected_total: u64,
    #[serde(default)]
    frozen_route_rejection_reasons: BTreeMap<String, u64>,
    #[serde(default)]
    frozen_route_witness_pending_total: u64,
    #[serde(default)]
    frozen_route_witness_resolved_total: u64,
    #[serde(default)]
    frozen_route_irreducible_total: u64,
    #[serde(default)]
    frozen_route_applicability_abstain_total: u64,
    #[serde(default)]
    frozen_future_accepted_total: u64,
    #[serde(default)]
    exact_executable_observations_total: u64,
    #[serde(default)]
    semantic_executable_observations_total: u64,
    #[serde(default)]
    teacher_only_observations_total: u64,
    #[serde(default)]
    program_pool_reuse_total: u64,
    #[serde(default)]
    program_pool_receipts_total: u64,
    #[serde(default)]
    renderer_consensus_migrated_examples_total: u64,
    #[serde(default)]
    legacy_partial_observations_discarded_total: u64,
    #[serde(default)]
    legacy_partial_buckets_discarded_total: u64,
    #[serde(default)]
    legacy_partial_receipts_discarded_total: u64,
    #[serde(default)]
    unreplayable_support_discarded_total: u64,
    #[serde(default)]
    applicability_negative_sessions: BTreeMap<String, BTreeMap<u64, BTreeSet<String>>>,
    buckets: Vec<OnlineCollectionBucket>,
}

#[derive(Serialize)]
struct CollectionSupportManifestMaterial<'a> {
    schema: &'static str,
    bucket_id: &'a str,
    program_sha256: &'a str,
    watermark_event_time_unix_nanos: u64,
    receipts: &'a [OnlineCollectionReceipt],
}

#[derive(Serialize)]
struct CollectionFutureManifestMaterial<'a> {
    schema: &'static str,
    support_manifest_sha256: &'a str,
    receipts: &'a [OnlineCollectionReceipt],
}

pub struct OnlineCollectionMiner {
    path: PathBuf,
    checkpoint: OnlineCollectionCheckpoint,
}

impl OnlineCollectionMiner {
    pub fn open(path: impl Into<PathBuf>, config: OnlineCollectionConfig) -> Result<Self, String> {
        validate_config(config)?;
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "online_collection_checkpoint_dir:{}:{error}",
                    parent.display()
                )
            })?;
        }
        let mut checkpoint = if path.exists() {
            decode_collection_checkpoint(&fs::read(&path).map_err(|error| {
                format!(
                    "online_collection_checkpoint_read:{}:{error}",
                    path.display()
                )
            })?)?
        } else {
            OnlineCollectionCheckpoint {
                schema: ONLINE_COLLECTION_SCHEMA_V3.to_owned(),
                pooling_strategy_version: ONLINE_COLLECTION_POOLING_STRATEGY_V35,
                structural_resynthesis_pending_bucket_ids: BTreeSet::new(),
                structural_resynthesis_completed_buckets_total: 0,
                structural_resynthesis_failed_buckets_total: 0,
                config,
                observations_total: 0,
                duplicate_observations_total: 0,
                observed_evidence_graph_sha256: BTreeSet::new(),
                unsupported_total: 0,
                synthesis_error_total: 0,
                privacy_rejected_observations_total: 0,
                unsupported_dynamic_zero_total: 0,
                unsupported_dynamic_partial_total: 0,
                unsupported_dynamic_full_total: 0,
                unsupported_partial_with_request_source_total: 0,
                unsupported_partial_with_tool_source_total: 0,
                ambiguous_assignment_total: 0,
                exact_checks_total: 0,
                candidates_enumerated_total: 0,
                full_enumerations_total: 0,
                version_space_intersection_checks_total: 0,
                guard_scheduled_buckets_total: 0,
                guard_pruned_buckets_total: 0,
                unsupported_expected_in_latest_output: 0,
                unsupported_expected_in_any_output: 0,
                unsupported_without_exact_source_span: 0,
                unsupported_with_scalar_overlap: 0,
                policy_rejected_exact_matches: 0,
                policy_rejection_reasons: BTreeMap::new(),
                counterexamples_total: 0,
                cegis_subcenters_total: 0,
                revoked_candidates_total: 0,
                late_after_freeze_total: 0,
                future_intent_rejected_total: 0,
                frozen_route_candidates_considered_total: 0,
                frozen_route_anti_rejected_total: 0,
                frozen_route_phase_rejected_total: 0,
                frozen_route_verifier_rejected_total: 0,
                frozen_route_rejection_reasons: BTreeMap::new(),
                frozen_route_witness_pending_total: 0,
                frozen_route_witness_resolved_total: 0,
                frozen_route_irreducible_total: 0,
                frozen_route_applicability_abstain_total: 0,
                frozen_future_accepted_total: 0,
                exact_executable_observations_total: 0,
                semantic_executable_observations_total: 0,
                teacher_only_observations_total: 0,
                program_pool_reuse_total: 0,
                program_pool_receipts_total: 0,
                renderer_consensus_migrated_examples_total: 0,
                legacy_partial_observations_discarded_total: 0,
                legacy_partial_buckets_discarded_total: 0,
                legacy_partial_receipts_discarded_total: 0,
                unreplayable_support_discarded_total: 0,
                applicability_negative_sessions: BTreeMap::new(),
                buckets: Vec::new(),
            }
        };
        let legacy_migrated = checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
            || checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V3;
        if legacy_migrated {
            migrate_collection_program_pools(&mut checkpoint)?;
        }
        let archetype_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V4;
        if archetype_migrated {
            migrate_collection_archetype_pools(&mut checkpoint)?;
        }
        let exact_authority_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V5;
        if exact_authority_migrated {
            migrate_collection_exact_authority_pools(&mut checkpoint)?;
        }
        let renderer_consensus_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V6;
        if renderer_consensus_migrated {
            migrate_collection_renderer_consensus_pools(&mut checkpoint)?;
        }
        let invariant_wave_center_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V7;
        if invariant_wave_center_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V7;
        }
        let active_witness_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V8;
        if active_witness_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V8;
            migrate_collection_active_witness_pools(&mut checkpoint)?;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
        }
        let exact_teacher_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V9;
        if exact_teacher_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V9;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
            for bucket in &mut checkpoint.buckets {
                bucket.future.clear();
                bucket.durable_runtime_parity_receipts.clear();
            }
        }
        let typed_negative_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V10;
        if typed_negative_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V10;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_route_applicability_abstain_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
            for bucket in &mut checkpoint.buckets {
                bucket.learned_anti_atom_ids.clear();
            }
        }
        let exact_receipt_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V12;
        if exact_receipt_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V12;
            migrate_collection_exact_receipts(&mut checkpoint)?;
            checkpoint.frozen_route_candidates_considered_total = 0;
            checkpoint.frozen_route_anti_rejected_total = 0;
            checkpoint.frozen_route_phase_rejected_total = 0;
            checkpoint.frozen_route_verifier_rejected_total = 0;
            checkpoint.frozen_route_rejection_reasons.clear();
            checkpoint.frozen_route_witness_pending_total = 0;
            checkpoint.frozen_route_witness_resolved_total = 0;
            checkpoint.frozen_route_irreducible_total = 0;
            checkpoint.frozen_route_applicability_abstain_total = 0;
            checkpoint.frozen_future_accepted_total = 0;
            checkpoint.late_after_freeze_total = 0;
            checkpoint.future_intent_rejected_total = 0;
        }
        let law_quotient_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V13;
        if law_quotient_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V13;
        }
        let keyed_layout_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V14;
        if keyed_layout_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V14;
            migrate_collection_keyed_layouts(&mut checkpoint)?;
        }
        let adapter_intersection_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V15;
        if adapter_intersection_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V15;
        }
        let phase_adapter_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V16;
        if phase_adapter_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V16;
        }
        let decidable_recovery_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V17;
        if decidable_recovery_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V17;
        }
        let relational_role_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V18;
        if relational_role_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V18;
            migrate_collection_relational_role_programs(&mut checkpoint)?;
        }
        let replayable_support_revalidated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V19;
        if replayable_support_revalidated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V19;
        }
        let consensus_policy_reconsidered =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V20;
        if consensus_policy_reconsidered {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V20;
        }
        let structural_resynthesis_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V21;
        if structural_resynthesis_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V21;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let selector_law_quotient_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V22;
        if selector_law_quotient_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V22;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let semantic_adapter_wave_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V23;
        if semantic_adapter_wave_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V23;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let expanded_adapter_library_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V24;
        if expanded_adapter_library_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V24;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let relational_adapter_path_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V25;
        if relational_adapter_path_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V25;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let lexical_adapter_wave_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V26;
        if lexical_adapter_wave_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V26;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let adapter_wave_proof_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V27;
        if adapter_wave_proof_refresh_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V27;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let turn_output_adapter_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V28;
        if turn_output_adapter_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V28;
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
        }
        let concrete_adapter_law_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V29;
        if concrete_adapter_law_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V29;
        }
        let canonical_alignment_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V31;
        if canonical_alignment_refresh_migrated {
            // New canonical alignment is applied only to newly observed or
            // explicitly replayed support. Restart must not duplicate buckets
            // or silently reclassify retained evidence.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V31;
        }
        let durable_phase_adapter_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V32;
        if durable_phase_adapter_refresh_migrated {
            // V32 can reconsider retained support without raw provider data:
            // routing atoms are recovered from durable pre-action receipts.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V32;
        }
        let durable_law_subcenter_refresh_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V33;
        if durable_law_subcenter_refresh_migrated {
            // Matched program digests are exact teacher proofs, so V33 can
            // recover law subcenters without retaining provider payloads.
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V33;
        }
        let exact_subcenter_dedup_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V34;
        if exact_subcenter_dedup_migrated {
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V34;
        }
        let durable_adapter_phase_evidence_migrated =
            checkpoint.pooling_strategy_version < ONLINE_COLLECTION_POOLING_STRATEGY_V35;
        if durable_adapter_phase_evidence_migrated {
            // Hash-only V34 checkpoints cannot manufacture actor phase atoms.
            // Queue retained support for bounded replay; only matching real
            // evidence may populate the compact V35 proof field.
            checkpoint.structural_resynthesis_pending_bucket_ids.extend(
                checkpoint.buckets.iter().filter_map(|bucket| {
                    (bucket.frozen_program_sha256.is_none()
                        && bucket.support.len() >= checkpoint.config.support_rows)
                        .then_some(bucket.bucket_id.clone())
                }),
            );
            checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V35;
        }
        let accounting_repaired = repair_collection_checkpoint_accounting(&mut checkpoint);
        validate_checkpoint(&checkpoint, config)?;
        let mut miner = Self { path, checkpoint };
        if replayable_support_revalidated {
            miner.revalidate_replayable_support_buffered()?;
        }
        let pre_v17_migrated = legacy_migrated
            || archetype_migrated
            || exact_authority_migrated
            || renderer_consensus_migrated
            || invariant_wave_center_migrated
            || active_witness_migrated
            || exact_teacher_migrated
            || typed_negative_migrated
            || exact_receipt_migrated
            || law_quotient_migrated
            || keyed_layout_migrated
            || adapter_intersection_migrated
            || phase_adapter_migrated
            || accounting_repaired;
        let checkpoint_migrated = pre_v17_migrated
            || decidable_recovery_migrated
            || relational_role_migrated
            || replayable_support_revalidated
            || consensus_policy_reconsidered
            || structural_resynthesis_migrated
            || selector_law_quotient_migrated
            || turn_output_adapter_migrated
            || concrete_adapter_law_migrated
            || canonical_alignment_refresh_migrated
            || durable_phase_adapter_refresh_migrated
            || durable_law_subcenter_refresh_migrated
            || exact_subcenter_dedup_migrated;
        if checkpoint_migrated {
            if exact_subcenter_dedup_migrated {
                miner.deduplicate_exact_unfrozen_buckets()?;
            }
            if pre_v17_migrated {
                miner.merge_converged_unfrozen_buckets()?;
            }
            let migration_indices = if pre_v17_migrated {
                (0..miner.checkpoint.buckets.len()).collect::<Vec<_>>()
            } else if durable_phase_adapter_refresh_migrated
                || durable_law_subcenter_refresh_migrated
                || exact_subcenter_dedup_migrated
            {
                miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter(|(_, bucket)| {
                        bucket.frozen_program_sha256.is_none()
                            && bucket.support.len() >= miner.checkpoint.config.support_rows
                            && bucket.programs.len() > 1
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            } else if consensus_policy_reconsidered
                || structural_resynthesis_migrated
                || selector_law_quotient_migrated
                || canonical_alignment_refresh_migrated
            {
                Vec::new()
            } else {
                miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter(|(_, bucket)| {
                        bucket.frozen_program_sha256.is_none()
                            && bucket.support.len() >= miner.checkpoint.config.support_rows
                            && bucket.programs.len() > 1
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            };
            for index in migration_indices {
                miner.normalize_bucket_receipts(index);
                miner.freeze_or_split(index)?;
            }
            miner.persist()?;
        }
        Ok(miner)
    }

    pub fn observe(&mut self, observation: OnlineCollectionObservation) -> Result<(), String> {
        self.observe_with_persistence(observation, true, false)
    }

    pub fn observe_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.observe_with_persistence(observation, false, false)
    }

    pub fn observe_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        self.observe_with_persistence(observation, false, true)
    }

    pub fn rehydrate_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
    ) -> Result<(), String> {
        validate_observation(&observation)?;
        if !self
            .checkpoint
            .observed_evidence_graph_sha256
            .contains(&observation.evidence_graph_sha256)
        {
            return Ok(());
        }
        let evidence_id = observation.evidence_graph_sha256.as_str();
        let indices = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| {
                bucket.frozen_program_sha256.is_none()
                    && !bucket.runtime_examples.contains_key(evidence_id)
                    && bucket
                        .support
                        .iter()
                        .any(|receipt| receipt.evidence_graph_sha256 == evidence_id)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for index in indices {
            let verified = self.checkpoint.buckets[index]
                .support
                .iter()
                .find(|receipt| receipt.evidence_graph_sha256 == evidence_id)
                .is_some_and(|receipt| {
                    receipt.matched_program_sha256.iter().any(|digest| {
                        self.checkpoint.buckets[index]
                            .programs
                            .get(digest)
                            .is_some_and(|program| {
                                independently_verified_teacher_match(program, &observation.example)
                            })
                    })
                });
            if !verified {
                continue;
            }
            let layout_sha256 = structural_layout_sha256(&observation.example.provider_payload)?;
            for receipt in self.checkpoint.buckets[index]
                .support
                .iter_mut()
                .filter(|receipt| receipt.evidence_graph_sha256 == evidence_id)
            {
                receipt.layout_sha256.clone_from(&layout_sha256);
            }
            insert_runtime_example(
                &mut self.checkpoint.buckets[index],
                &observation,
                self.checkpoint.config.max_receipts_per_bucket,
            );
            refresh_durable_adapter_phase_atoms(&mut self.checkpoint.buckets[index]);
            self.freeze_or_split(index)?;
        }
        Ok(())
    }

    pub fn rehydrate_legacy_replay_training_buffered(
        &mut self,
        observation: OnlineCollectionObservation,
        source_session_identities: &BTreeSet<String>,
    ) -> Result<LegacyReplayRehydrationStats, String> {
        validate_observation(&observation)?;
        let layout_sha256 = structural_layout_sha256(&observation.example.provider_payload)?;
        let mut stats = LegacyReplayRehydrationStats::default();
        let indices = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                if bucket.frozen_program_sha256.is_some() {
                    return None;
                }
                let mut matches = Vec::new();
                for receipt in &bucket.support {
                    if !source_session_identities.contains(&receipt.session_id_sha256)
                        || bucket
                            .runtime_examples
                            .contains_key(&receipt.evidence_graph_sha256)
                    {
                        continue;
                    }
                    stats.session_receipts = stats.session_receipts.saturating_add(1);
                    let event_matches = match (
                        receipt.event_time_unix_nanos,
                        observation.event_time_unix_nanos,
                    ) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => true,
                        _ => false,
                    };
                    if !event_matches {
                        continue;
                    }
                    stats.event_time_matches = stats.event_time_matches.saturating_add(1);
                    if receipt.estimated_input_tokens != observation.estimated_input_tokens {
                        continue;
                    }
                    stats.token_matches = stats.token_matches.saturating_add(1);
                    if !receipt.matched_program_sha256.iter().any(|digest| {
                        bucket.programs.get(digest).is_some_and(|program| {
                            independently_verified_teacher_match(program, &observation.example)
                        })
                    }) {
                        continue;
                    }
                    stats.verifier_matches = stats.verifier_matches.saturating_add(1);
                    let layout_matches = receipt.layout_sha256 == layout_sha256;
                    if layout_matches {
                        stats.layout_matches = stats.layout_matches.saturating_add(1);
                    }
                    matches.push((receipt.evidence_graph_sha256.clone(), layout_matches));
                }
                let layout_matches = matches
                    .iter()
                    .filter(|(_, layout_matches)| *layout_matches)
                    .map(|(evidence_id, _)| evidence_id.clone())
                    .collect::<Vec<_>>();
                let selected = if layout_matches.len() == 1 {
                    Some(layout_matches[0].clone())
                } else if layout_matches.is_empty() && matches.len() == 1 {
                    Some(matches[0].0.clone())
                } else {
                    None
                };
                if selected.is_none() && !matches.is_empty() {
                    stats.ambiguous_matches = stats.ambiguous_matches.saturating_add(1);
                }
                selected.map(|evidence_id| (index, evidence_id))
            })
            .collect::<Vec<_>>();
        for (index, evidence_id) in indices {
            insert_runtime_example_for_evidence(
                &mut self.checkpoint.buckets[index],
                &evidence_id,
                &observation,
                self.checkpoint.config.max_receipts_per_bucket,
            );
            self.freeze_or_split(index)?;
            stats.attached_receipts = stats.attached_receipts.saturating_add(1);
        }
        Ok(stats)
    }

    pub fn revalidate_replayable_support_buffered(&mut self) -> Result<u64, String> {
        self.revalidate_support_buffered(false, true)
    }

    fn revalidate_support_buffered(
        &mut self,
        structural_only: bool,
        refresh_proof: bool,
    ) -> Result<u64, String> {
        let initial_bucket_count = self.checkpoint.buckets.len();
        let mut links_added = 0_u64;
        for index in 0..initial_bucket_count {
            links_added = links_added.saturating_add(self.revalidate_bucket_support(
                index,
                structural_only,
                refresh_proof,
            )?);
        }
        Ok(links_added)
    }

    fn revalidate_bucket_support(
        &mut self,
        index: usize,
        structural_only: bool,
        refresh_proof: bool,
    ) -> Result<u64, String> {
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return Ok(0);
        };
        if bucket.frozen_program_sha256.is_some()
            || (structural_only
                && !bucket
                    .programs
                    .values()
                    .any(|program| canonical_dynamic_role_count(program) >= 2))
        {
            return Ok(0);
        }
        let links = {
            let mut links = Vec::new();
            for (receipt_index, receipt) in bucket.support.iter().enumerate() {
                let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256)
                else {
                    continue;
                };
                let has_retained_match = receipt
                    .matched_program_sha256
                    .iter()
                    .any(|digest| bucket.programs.contains_key(digest));
                for (digest, program) in &bucket.programs {
                    if !receipt.matched_program_sha256.contains(digest)
                        && (!structural_only
                            || !has_retained_match
                            || canonical_dynamic_role_count(program) >= 2)
                        && independently_verified_teacher_match(program, example)
                    {
                        links.push((receipt_index, digest.clone()));
                    }
                }
            }
            links
        };
        let links_added = u64::try_from(links.len()).unwrap_or(u64::MAX);
        if !links.is_empty() {
            let bucket = &mut self.checkpoint.buckets[index];
            for (receipt_index, digest) in links {
                bucket.support[receipt_index]
                    .matched_program_sha256
                    .push(digest);
            }
            for receipt in &mut bucket.support {
                receipt.matched_program_sha256.sort();
                receipt.matched_program_sha256.dedup();
            }
        }
        self.normalize_bucket_receipts(index);
        if refresh_proof {
            self.freeze_or_split(index)?;
        }
        Ok(links_added)
    }

    #[must_use]
    pub fn has_structural_resynthesis_work(&self) -> bool {
        !self
            .checkpoint
            .structural_resynthesis_pending_bucket_ids
            .is_empty()
    }

    pub fn run_structural_resynthesis_work_slice(&mut self) -> Result<u64, String> {
        let Some(bucket_id) = self
            .checkpoint
            .structural_resynthesis_pending_bucket_ids
            .pop_first()
        else {
            return Ok(0);
        };
        let Some(index) = self
            .checkpoint
            .buckets
            .iter()
            .position(|bucket| bucket.bucket_id == bucket_id)
        else {
            self.checkpoint
                .structural_resynthesis_completed_buckets_total = self
                .checkpoint
                .structural_resynthesis_completed_buckets_total
                .saturating_add(1);
            return Ok(0);
        };
        let result = self.resynthesize_bucket_structural_programs(index);
        match result {
            Ok(programs_added) => {
                self.freeze_or_split(index)?;
                self.checkpoint
                    .structural_resynthesis_completed_buckets_total = self
                    .checkpoint
                    .structural_resynthesis_completed_buckets_total
                    .saturating_add(1);
                Ok(programs_added)
            }
            Err(error) => {
                self.checkpoint.structural_resynthesis_failed_buckets_total = self
                    .checkpoint
                    .structural_resynthesis_failed_buckets_total
                    .saturating_add(1);
                Err(error)
            }
        }
    }

    fn resynthesize_bucket_structural_programs(&mut self, index: usize) -> Result<u64, String> {
        let bucket = self
            .checkpoint
            .buckets
            .get(index)
            .ok_or_else(|| "online_collection_resynthesis_bucket_missing".to_owned())?;
        if bucket.frozen_program_sha256.is_some()
            || bucket.support.len() < self.checkpoint.config.support_rows
        {
            return Ok(0);
        }
        let archetype_id = bucket.archetype_id.clone();
        let mut seeds = bucket
            .support
            .iter()
            .filter_map(|receipt| {
                let example = bucket
                    .runtime_examples
                    .get(&receipt.evidence_graph_sha256)?;
                let coverage = diagnose_response_dynamic_coverage(example);
                (coverage.matching_selectors >= 2).then_some((
                    coverage.matching_selectors,
                    coverage.tool_dynamic_bytes,
                    coverage.dynamic_bytes,
                    example.expected_response.len(),
                    receipt.evidence_graph_sha256.clone(),
                    receipt.clone(),
                    example.clone(),
                ))
            })
            .collect::<Vec<_>>();
        seeds.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| left.4.cmp(&right.4))
        });
        seeds.truncate(MAX_STRUCTURAL_RESYNTHESIS_SEEDS_PER_BUCKET);

        let mut programs = BTreeMap::new();
        for (_, _, _, _, _, receipt, example) in seeds {
            let observation = OnlineCollectionObservation {
                evidence_graph_sha256: receipt.evidence_graph_sha256,
                client_intent_id_sha256: receipt.client_intent_id_sha256,
                session_id_sha256: receipt.session_id_sha256,
                event_time_unix_nanos: receipt.event_time_unix_nanos,
                estimated_input_tokens: receipt.estimated_input_tokens,
                example,
            };
            for (digest, program) in structural_programs_for_observation(&observation)? {
                if response_program_archetype_id(&program)? == archetype_id {
                    programs.insert(digest, program);
                }
            }
        }
        let programs_added = {
            let bucket = &mut self.checkpoint.buckets[index];
            let added = programs
                .keys()
                .filter(|digest| !bucket.programs.contains_key(*digest))
                .count();
            bucket.programs.extend(programs);
            bucket.programs = bounded_program_map(
                std::mem::take(&mut bucket.programs),
                crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
            );
            bucket.bucket_id =
                collection_archetype_bucket_id(&bucket.archetype_id, bucket.programs.keys())?;
            u64::try_from(added).unwrap_or(u64::MAX)
        };
        self.revalidate_bucket_support(index, true, true)?;
        Ok(programs_added)
    }

    pub fn flush(&self) -> Result<(), String> {
        self.persist()
    }

    fn observe_with_persistence(
        &mut self,
        observation: OnlineCollectionObservation,
        durable: bool,
        support_only: bool,
    ) -> Result<(), String> {
        validate_observation(&observation)?;
        let already_observed = self
            .checkpoint
            .observed_evidence_graph_sha256
            .contains(&observation.evidence_graph_sha256);
        if already_observed && !support_only {
            self.checkpoint.duplicate_observations_total = self
                .checkpoint
                .duplicate_observations_total
                .saturating_add(1);
            return self.persist_if(durable);
        }
        let count_observation = !already_observed;
        let evidence_graph_sha256 = observation.evidence_graph_sha256.clone();
        if count_observation {
            self.checkpoint.observations_total =
                self.checkpoint.observations_total.saturating_add(1);
        }
        let frozen_match = !support_only && self.evaluate_frozen_candidates(&observation)?;
        if frozen_match {
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        let matching_existing = self.matching_unfrozen_buckets(&observation)?;
        match matching_existing.as_slice() {
            [(index, matching_programs)] => {
                let matching_programs = self.checkpoint.buckets[*index]
                    .programs
                    .iter()
                    .filter(|(digest, _)| matching_programs.contains(*digest))
                    .map(|(digest, program)| (digest.clone(), program.clone()))
                    .collect::<BTreeMap<_, _>>();
                let exact_match = matching_programs.values().any(|program| {
                    response_program_exactly_matches_example(program, &observation.example)
                });
                self.record_executable_observation(exact_match, count_observation);
                self.update_bucket(*index, &matching_programs, &observation, true, true)?;
                let structural_programs = structural_programs_for_observation(&observation)?;
                if !structural_programs.is_empty() {
                    self.assign_archetype_programs(structural_programs, &observation, true, false)?;
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, true);
            }
            [_, _, ..] => {
                let exact_match = matching_existing.iter().any(|(index, matching_programs)| {
                    self.checkpoint.buckets[*index]
                        .programs
                        .iter()
                        .filter(|(digest, _)| matching_programs.contains(*digest))
                        .any(|(_, program)| {
                            response_program_exactly_matches_example(program, &observation.example)
                        })
                });
                self.record_executable_observation(exact_match, count_observation);
                if count_observation {
                    self.checkpoint.ambiguous_assignment_total =
                        self.checkpoint.ambiguous_assignment_total.saturating_add(1);
                }
                for (index, matching_programs) in matching_existing.iter().cloned() {
                    let matching_programs = self.checkpoint.buckets[index]
                        .programs
                        .iter()
                        .filter(|(digest, _)| matching_programs.contains(*digest))
                        .map(|(digest, program)| (digest.clone(), program.clone()))
                        .collect::<BTreeMap<_, _>>();
                    self.update_bucket(index, &matching_programs, &observation, true, true)?;
                }
                let structural_programs = structural_programs_for_observation(&observation)?;
                if !structural_programs.is_empty() {
                    self.assign_archetype_programs(structural_programs, &observation, true, false)?;
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, true);
            }
            [] => {}
        }
        self.checkpoint.full_enumerations_total =
            self.checkpoint.full_enumerations_total.saturating_add(1);
        let synthesis_example = compact_active_turn_synthesis_example(&observation.example)
            .unwrap_or_else(|| observation.example.clone());
        let coverage = diagnose_response_dynamic_coverage(&synthesis_example);
        let source_span = unsupported_source_span(&synthesis_example);
        let scalar_overlap = has_scalar_overlap(&synthesis_example);
        let version_space = match enumerate_source_neutral_response_programs_with_coverage(
            &synthesis_example,
            Some(coverage),
        ) {
            Ok(version_space) => version_space,
            Err(_) => {
                if count_observation {
                    self.checkpoint.synthesis_error_total =
                        self.checkpoint.synthesis_error_total.saturating_add(1);
                    self.checkpoint.unsupported_total =
                        self.checkpoint.unsupported_total.saturating_add(1);
                }
                return self.persist_new_observation(evidence_graph_sha256, durable, false);
            }
        };
        self.checkpoint.exact_checks_total = self
            .checkpoint
            .exact_checks_total
            .saturating_add(version_space.exact_checks as u64);
        self.checkpoint.candidates_enumerated_total = self
            .checkpoint
            .candidates_enumerated_total
            .saturating_add(version_space.candidates_enumerated as u64);
        if count_observation {
            self.checkpoint.policy_rejected_exact_matches = self
                .checkpoint
                .policy_rejected_exact_matches
                .saturating_add(version_space.policy_rejected_exact_matches as u64);
            for (reason, count) in &version_space.policy_rejection_reasons {
                let total = self
                    .checkpoint
                    .policy_rejection_reasons
                    .entry(reason.clone())
                    .or_default();
                *total = total.saturating_add(*count as u64);
            }
        }
        let exact_programs = version_space
            .programs
            .iter()
            .filter(|program| {
                crate::response_program_exactly_matches_example(program, &observation.example)
            })
            .cloned()
            .collect::<Vec<_>>();
        let exact_program_count = exact_programs.len();
        let teacher_programs = version_space
            .programs
            .into_iter()
            .filter(|program| {
                response_program_authority_matches_example(program, &observation.example)
            })
            .collect::<Vec<_>>();
        let exact_programs = exact_programs
            .into_iter()
            .filter(is_privacy_safe_online_response_program)
            .filter(|program| {
                independently_verified_authority_response(program, &observation.example).is_some()
            })
            .map(|program| {
                canonical_json_sha256(&program)
                    .map(|digest| (digest, program))
                    .map_err(str::to_owned)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let teacher_programs = teacher_programs
            .into_iter()
            .filter(is_privacy_safe_online_response_program)
            .filter(|program| {
                independently_verified_authority_response(program, &observation.example).is_some()
            })
            .map(|program| {
                canonical_json_sha256(&program)
                    .map(|digest| (digest, program))
                    .map_err(str::to_owned)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let exact_verified = !exact_programs.is_empty();
        // Keep canonical semantic operators alongside surface-specific exact
        // renderers so repeated behavior can converge across response styles.
        let programs = teacher_programs;
        if programs.is_empty() {
            if count_observation {
                self.checkpoint.unsupported_total =
                    self.checkpoint.unsupported_total.saturating_add(1);
                if exact_program_count > 0 {
                    self.checkpoint.privacy_rejected_observations_total = self
                        .checkpoint
                        .privacy_rejected_observations_total
                        .saturating_add(1);
                }
                if coverage.dynamic_bytes == 0 {
                    self.checkpoint.unsupported_dynamic_zero_total = self
                        .checkpoint
                        .unsupported_dynamic_zero_total
                        .saturating_add(1);
                } else if coverage.dynamic_bytes < coverage.response_bytes {
                    self.checkpoint.unsupported_dynamic_partial_total = self
                        .checkpoint
                        .unsupported_dynamic_partial_total
                        .saturating_add(1);
                    if coverage.request_dynamic_bytes > 0 {
                        self.checkpoint
                            .unsupported_partial_with_request_source_total = self
                            .checkpoint
                            .unsupported_partial_with_request_source_total
                            .saturating_add(1);
                    }
                    if coverage.tool_dynamic_bytes > 0 {
                        self.checkpoint.unsupported_partial_with_tool_source_total = self
                            .checkpoint
                            .unsupported_partial_with_tool_source_total
                            .saturating_add(1);
                    }
                } else {
                    self.checkpoint.unsupported_dynamic_full_total = self
                        .checkpoint
                        .unsupported_dynamic_full_total
                        .saturating_add(1);
                }
                match source_span {
                    UnsupportedSourceSpan::Latest => {
                        self.checkpoint.unsupported_expected_in_latest_output = self
                            .checkpoint
                            .unsupported_expected_in_latest_output
                            .saturating_add(1);
                    }
                    UnsupportedSourceSpan::Earlier => {
                        self.checkpoint.unsupported_expected_in_any_output = self
                            .checkpoint
                            .unsupported_expected_in_any_output
                            .saturating_add(1);
                    }
                    UnsupportedSourceSpan::Missing => {
                        self.checkpoint.unsupported_without_exact_source_span = self
                            .checkpoint
                            .unsupported_without_exact_source_span
                            .saturating_add(1);
                    }
                }
                if scalar_overlap {
                    self.checkpoint.unsupported_with_scalar_overlap = self
                        .checkpoint
                        .unsupported_with_scalar_overlap
                        .saturating_add(1);
                }
            }
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        if count_observation {
            if exact_verified {
                self.checkpoint.exact_executable_observations_total = self
                    .checkpoint
                    .exact_executable_observations_total
                    .saturating_add(1);
            } else {
                self.checkpoint.semantic_executable_observations_total = self
                    .checkpoint
                    .semantic_executable_observations_total
                    .saturating_add(1);
            }
        }
        if support_only
            && self.checkpoint.buckets.iter().any(|bucket| {
                bucket.frozen_program_sha256.is_some()
                    && bucket.programs.keys().any(|key| programs.contains_key(key))
            })
        {
            return self.persist_new_observation(evidence_graph_sha256, durable, false);
        }
        self.assign_archetype_programs(programs, &observation, true, count_observation)?;
        self.persist_new_observation(evidence_graph_sha256, durable, true)
    }

    fn record_executable_observation(
        &mut self,
        exact_teacher_match: bool,
        count_observation: bool,
    ) {
        if !count_observation {
            return;
        }
        if exact_teacher_match {
            self.checkpoint.exact_executable_observations_total = self
                .checkpoint
                .exact_executable_observations_total
                .saturating_add(1);
        } else {
            self.checkpoint.semantic_executable_observations_total = self
                .checkpoint
                .semantic_executable_observations_total
                .saturating_add(1);
        }
    }

    #[must_use]
    pub fn consensus_diagnostics(&self) -> Vec<OnlineCollectionConsensusDiagnostic> {
        self.checkpoint
            .buckets
            .iter()
            .filter(|bucket| {
                bucket.frozen_program_sha256.is_none()
                    && bucket.support.len() >= self.checkpoint.config.support_rows
            })
            .map(|bucket| {
                consensus_diagnostic(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
            .collect()
    }

    #[must_use]
    pub fn consensus_diagnostic_for_bucket(
        &self,
        bucket_id: &str,
    ) -> Option<OnlineCollectionConsensusDiagnostic> {
        self.checkpoint
            .buckets
            .iter()
            .find(|bucket| bucket.bucket_id == bucket_id)
            .map(|bucket| {
                consensus_diagnostic(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
    }

    pub fn status(&self) -> OnlineCollectionStatus {
        let mut support_receipts = BTreeMap::new();
        let mut future_receipts = BTreeMap::new();
        let mut runtime_parity_receipts = BTreeSet::new();
        let mut durable_adapter_phase_evidence = BTreeSet::new();
        let mut durable_adapter_phase_pairs = 0_usize;
        for bucket in &self.checkpoint.buckets {
            for (evidence_id, atoms_by_program) in &bucket.durable_adapter_phase_atoms {
                durable_adapter_phase_evidence.insert(evidence_id.clone());
                durable_adapter_phase_pairs =
                    durable_adapter_phase_pairs.saturating_add(atoms_by_program.len());
            }
            for receipt in &bucket.support {
                support_receipts
                    .entry(receipt.evidence_graph_sha256.clone())
                    .or_insert(receipt.estimated_input_tokens);
            }
            for receipt in &bucket.future {
                future_receipts
                    .entry(receipt.evidence_graph_sha256.clone())
                    .or_insert(receipt.estimated_input_tokens);
                if bucket
                    .runtime_examples
                    .contains_key(&receipt.evidence_graph_sha256)
                    || bucket
                        .durable_runtime_parity_receipts
                        .contains_key(&receipt.evidence_graph_sha256)
                {
                    runtime_parity_receipts.insert(receipt.evidence_graph_sha256.clone());
                }
            }
        }
        let mut buckets = self
            .checkpoint
            .buckets
            .iter()
            .map(|bucket| bucket_status(bucket, self.checkpoint.config.support_rows))
            .collect::<Vec<_>>();
        buckets.sort_by(|left, right| left.bucket_id.cmp(&right.bucket_id));
        let frozen_buckets_total = buckets.iter().filter(|bucket| bucket.frozen).count();
        let pre_admission_ready_buckets_total = buckets
            .iter()
            .filter(|bucket| bucket.admission_blocker.is_none())
            .count();
        let wrong_accepts_total = buckets.iter().map(|bucket| bucket.wrong_accepts).sum();
        let mut frozen_program_kinds = BTreeMap::new();
        for kind in buckets
            .iter()
            .filter_map(|bucket| bucket.candidate_program_kind.as_ref())
        {
            *frozen_program_kinds.entry(kind.clone()).or_insert(0) += 1;
        }
        let accounted_ambiguous_total = self.checkpoint.ambiguous_assignment_total;
        let accounted_executable_total = self
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(self.checkpoint.semantic_executable_observations_total)
            .saturating_sub(accounted_ambiguous_total);
        let classified = self
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(self.checkpoint.semantic_executable_observations_total)
            .saturating_add(self.checkpoint.unsupported_total);
        let legacy_unclassified_observations_total = self
            .checkpoint
            .observations_total
            .saturating_sub(classified);
        let accounted_irreducible_total = self
            .checkpoint
            .unsupported_total
            .saturating_add(legacy_unclassified_observations_total);
        OnlineCollectionStatus {
            pooling_strategy_version: self.checkpoint.pooling_strategy_version,
            durable_adapter_phase_evidence_rows: durable_adapter_phase_evidence.len(),
            durable_adapter_phase_pairs,
            structural_resynthesis_pending_buckets: self
                .checkpoint
                .structural_resynthesis_pending_bucket_ids
                .len(),
            structural_resynthesis_completed_buckets_total: self
                .checkpoint
                .structural_resynthesis_completed_buckets_total,
            structural_resynthesis_failed_buckets_total: self
                .checkpoint
                .structural_resynthesis_failed_buckets_total,
            observations_total: self.checkpoint.observations_total,
            duplicate_observations_total: self.checkpoint.duplicate_observations_total,
            unsupported_total: self.checkpoint.unsupported_total,
            synthesis_error_total: self.checkpoint.synthesis_error_total,
            privacy_rejected_observations_total: self
                .checkpoint
                .privacy_rejected_observations_total,
            unsupported_dynamic_zero_total: self.checkpoint.unsupported_dynamic_zero_total,
            unsupported_dynamic_partial_total: self.checkpoint.unsupported_dynamic_partial_total,
            unsupported_dynamic_full_total: self.checkpoint.unsupported_dynamic_full_total,
            unsupported_partial_with_request_source_total: self
                .checkpoint
                .unsupported_partial_with_request_source_total,
            unsupported_partial_with_tool_source_total: self
                .checkpoint
                .unsupported_partial_with_tool_source_total,
            ambiguous_assignment_total: self.checkpoint.ambiguous_assignment_total,
            exact_checks_total: self.checkpoint.exact_checks_total,
            candidates_enumerated_total: self.checkpoint.candidates_enumerated_total,
            full_enumerations_total: self.checkpoint.full_enumerations_total,
            version_space_intersection_checks_total: self
                .checkpoint
                .version_space_intersection_checks_total,
            guard_scheduled_buckets_total: self.checkpoint.guard_scheduled_buckets_total,
            guard_pruned_buckets_total: self.checkpoint.guard_pruned_buckets_total,
            unsupported_expected_in_latest_output: self
                .checkpoint
                .unsupported_expected_in_latest_output,
            unsupported_expected_in_any_output: self.checkpoint.unsupported_expected_in_any_output,
            unsupported_without_exact_source_span: self
                .checkpoint
                .unsupported_without_exact_source_span,
            unsupported_with_scalar_overlap: self.checkpoint.unsupported_with_scalar_overlap,
            policy_rejected_exact_matches: self.checkpoint.policy_rejected_exact_matches,
            policy_rejection_reasons: self.checkpoint.policy_rejection_reasons.clone(),
            counterexamples_total: self.checkpoint.counterexamples_total,
            cegis_subcenters_total: self.checkpoint.cegis_subcenters_total,
            revoked_candidates_total: self.checkpoint.revoked_candidates_total,
            late_after_freeze_total: self.checkpoint.late_after_freeze_total,
            future_intent_rejected_total: self.checkpoint.future_intent_rejected_total,
            frozen_route_candidates_considered_total: self
                .checkpoint
                .frozen_route_candidates_considered_total,
            frozen_route_anti_rejected_total: self.checkpoint.frozen_route_anti_rejected_total,
            frozen_route_phase_rejected_total: self.checkpoint.frozen_route_phase_rejected_total,
            frozen_route_verifier_rejected_total: self
                .checkpoint
                .frozen_route_verifier_rejected_total,
            frozen_route_rejection_reasons: self.checkpoint.frozen_route_rejection_reasons.clone(),
            frozen_route_rejection_accounting_complete: self
                .checkpoint
                .frozen_route_rejection_reasons
                .values()
                .copied()
                .sum::<u64>()
                == self.checkpoint.frozen_route_verifier_rejected_total,
            frozen_route_witness_pending_total: self.checkpoint.frozen_route_witness_pending_total,
            frozen_route_witness_resolved_total: self
                .checkpoint
                .frozen_route_witness_resolved_total,
            frozen_route_irreducible_total: self.checkpoint.frozen_route_irreducible_total,
            frozen_route_applicability_abstain_total: self
                .checkpoint
                .frozen_route_applicability_abstain_total,
            frozen_route_verifier_accounting_complete: self
                .checkpoint
                .frozen_route_verifier_rejected_total
                == self
                    .checkpoint
                    .frozen_route_witness_pending_total
                    .saturating_add(self.checkpoint.frozen_route_witness_resolved_total)
                    .saturating_add(self.checkpoint.frozen_route_irreducible_total)
                    .saturating_add(self.checkpoint.frozen_route_applicability_abstain_total),
            frozen_future_accepted_total: self.checkpoint.frozen_future_accepted_total,
            frozen_route_accounting_complete: self
                .checkpoint
                .frozen_route_candidates_considered_total
                == self
                    .checkpoint
                    .frozen_route_anti_rejected_total
                    .saturating_add(self.checkpoint.frozen_route_phase_rejected_total)
                    .saturating_add(self.checkpoint.frozen_route_verifier_rejected_total)
                    .saturating_add(self.checkpoint.frozen_future_accepted_total)
                    .saturating_add(self.checkpoint.late_after_freeze_total)
                    .saturating_add(self.checkpoint.future_intent_rejected_total),
            exact_executable_observations_total: self
                .checkpoint
                .exact_executable_observations_total,
            semantic_executable_observations_total: self
                .checkpoint
                .semantic_executable_observations_total,
            teacher_only_observations_total: self.checkpoint.teacher_only_observations_total,
            accounted_executable_total,
            accounted_ambiguous_total,
            accounted_irreducible_total,
            legacy_unclassified_observations_total,
            observation_accounting_complete: self.checkpoint.observations_total
                == accounted_executable_total
                    .saturating_add(accounted_ambiguous_total)
                    .saturating_add(accounted_irreducible_total),
            program_pool_reuse_total: self.checkpoint.program_pool_reuse_total,
            program_pool_receipts_total: self.checkpoint.program_pool_receipts_total,
            renderer_consensus_migrated_examples_total: self
                .checkpoint
                .renderer_consensus_migrated_examples_total,
            legacy_partial_observations_discarded_total: self
                .checkpoint
                .legacy_partial_observations_discarded_total,
            legacy_partial_buckets_discarded_total: self
                .checkpoint
                .legacy_partial_buckets_discarded_total,
            legacy_partial_receipts_discarded_total: self
                .checkpoint
                .legacy_partial_receipts_discarded_total,
            unreplayable_support_discarded_total: self
                .checkpoint
                .unreplayable_support_discarded_total,
            buckets_total: buckets.len(),
            frozen_buckets_total,
            pre_admission_ready_buckets_total,
            support_receipts_unique_total: support_receipts.len(),
            future_receipts_unique_total: future_receipts.len(),
            support_tokens_unique_total: support_receipts.values().copied().sum(),
            future_tokens_unique_total: future_receipts.values().copied().sum(),
            wrong_accepts_total,
            runtime_parity_cases_total: runtime_parity_receipts.len(),
            frozen_program_kinds,
            buckets,
        }
    }

    fn persist_new_observation(
        &mut self,
        evidence_graph_sha256: String,
        durable: bool,
        merge_buckets: bool,
    ) -> Result<(), String> {
        if merge_buckets {
            self.merge_converged_unfrozen_buckets()?;
        }
        self.checkpoint
            .observed_evidence_graph_sha256
            .insert(evidence_graph_sha256.clone());
        if let Err(error) = self.persist_if(durable) {
            self.checkpoint
                .observed_evidence_graph_sha256
                .remove(&evidence_graph_sha256);
            return Err(error);
        }
        Ok(())
    }

    fn merge_converged_unfrozen_buckets(&mut self) -> Result<(), String> {
        let max_receipts = self.checkpoint.config.max_receipts_per_bucket;
        let mut index = 0_usize;
        while index < self.checkpoint.buckets.len() {
            if self.checkpoint.buckets[index]
                .frozen_program_sha256
                .is_some()
                || self.checkpoint.buckets[index].programs.is_empty()
            {
                index = index.saturating_add(1);
                continue;
            }
            loop {
                let merge = ((index + 1)..self.checkpoint.buckets.len()).find_map(|candidate| {
                    let bucket = &self.checkpoint.buckets[candidate];
                    let compatible = bucket.frozen_program_sha256.is_none()
                        && bucket.archetype_id == self.checkpoint.buckets[index].archetype_id
                        && buckets_share_execution_law(&self.checkpoint.buckets[index], bucket);
                    if !compatible {
                        return None;
                    }
                    let programs = self.checkpoint.buckets[index]
                        .programs
                        .iter()
                        .chain(&bucket.programs)
                        .map(|(digest, program)| (digest.clone(), program.clone()))
                        .collect::<BTreeMap<_, _>>();
                    let receipts = self.checkpoint.buckets[index]
                        .support
                        .iter()
                        .chain(&bucket.support)
                        .cloned()
                        .collect::<Vec<_>>();
                    select_program_receipt_cover(
                        &programs,
                        &receipts,
                        crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
                    )
                    .map(|selected| (candidate, selected))
                });
                let Some((duplicate, selected_programs)) = merge else {
                    break;
                };
                let other = self.checkpoint.buckets.remove(duplicate);
                let bucket = &mut self.checkpoint.buckets[index];
                bucket.programs.extend(other.programs);
                bucket
                    .programs
                    .retain(|digest, _| selected_programs.contains(digest));
                bucket
                    .common_request_atom_ids
                    .retain(|atom| other.common_request_atom_ids.contains(atom));
                merge_receipts(&mut bucket.support, other.support, max_receipts);
                merge_receipts(&mut bucket.future, other.future, max_receipts);
                for receipt in bucket.support.iter_mut().chain(&mut bucket.future) {
                    receipt
                        .matched_program_sha256
                        .retain(|digest| selected_programs.contains(digest));
                }
                for (digest, example) in other.runtime_examples {
                    bucket.runtime_examples.entry(digest).or_insert(example);
                }
                for (digest, receipt) in other.durable_runtime_parity_receipts {
                    bucket
                        .durable_runtime_parity_receipts
                        .entry(digest)
                        .or_insert(receipt);
                }
                trim_runtime_examples(&mut bucket.runtime_examples, max_receipts);
                let future_refs = bucket
                    .future
                    .iter()
                    .map(|receipt| receipt.evidence_graph_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                bucket
                    .durable_runtime_parity_receipts
                    .retain(|evidence_ref, _| future_refs.contains(evidence_ref.as_str()));
                bucket
                    .rejected_program_sha256
                    .extend(other.rejected_program_sha256);
                bucket
                    .learned_anti_atom_ids
                    .extend(other.learned_anti_atom_ids);
                bucket.wrong_accepts = bucket.wrong_accepts.saturating_add(other.wrong_accepts);
            }
            self.checkpoint.buckets[index].bucket_id = collection_archetype_bucket_id(
                &self.checkpoint.buckets[index].archetype_id,
                self.checkpoint.buckets[index].programs.keys(),
            )?;
            self.normalize_bucket_receipts(index);
            self.freeze_or_split(index)?;
            index = index.saturating_add(1);
        }
        Ok(())
    }

    fn deduplicate_exact_unfrozen_buckets(&mut self) -> Result<(), String> {
        let mut keepers = BTreeMap::<String, (usize, String)>::new();
        let mut remove = BTreeSet::<usize>::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            if bucket.frozen_program_sha256.is_some() {
                continue;
            }
            let fingerprint = canonical_json_sha256(&(
                "nando.collection-unfrozen-proof-state.v1",
                &bucket.programs,
                &bucket.common_request_atom_ids,
                &bucket.support,
                &bucket.future,
                &bucket.runtime_examples,
                &bucket.durable_runtime_parity_receipts,
                &bucket.rejected_program_sha256,
                &bucket.learned_anti_atom_ids,
                bucket.wrong_accepts,
            ))
            .map_err(str::to_owned)?;
            match keepers.get(&fingerprint) {
                Some((keeper_index, keeper_id)) if bucket.bucket_id < *keeper_id => {
                    remove.insert(*keeper_index);
                    keepers.insert(fingerprint, (index, bucket.bucket_id.clone()));
                }
                Some(_) => {
                    remove.insert(index);
                }
                None => {
                    keepers.insert(fingerprint, (index, bucket.bucket_id.clone()));
                }
            }
        }
        for index in remove.into_iter().rev() {
            self.checkpoint.buckets.remove(index);
        }
        Ok(())
    }

    fn persist_if(&self, durable: bool) -> Result<(), String> {
        if durable { self.persist() } else { Ok(()) }
    }

    pub fn quarantine_packages(&self) -> Result<Vec<ResponsePackage>, String> {
        let mut packages = Vec::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            if let Some(package) = self.package_for_bucket(index, bucket, false)? {
                packages.push(package);
            }
        }
        packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        Ok(packages)
    }

    pub fn admission_candidates(&self) -> Result<Vec<OnlineCollectionAdmissionCandidate>, String> {
        let mut candidates = Vec::new();
        for (index, bucket) in self.checkpoint.buckets.iter().enumerate() {
            let Some(mut package) = self.package_for_bucket(index, bucket, false)? else {
                continue;
            };
            let causal_report = self.collection_causal_report(bucket, &package)?;
            if causal_report.verdict != "PASS" {
                continue;
            }
            package.state = ResponsePackageState::Active;
            package.proof.wave_causal_pass = true;
            package.wave_margin_micro = causal_report.wave_margin_micro;
            if !package.eligible_for_admission_candidate() {
                continue;
            }
            let runtime_parity_cases = bucket
                .future
                .iter()
                .filter_map(|receipt| {
                    let example = bucket
                        .runtime_examples
                        .get(&receipt.evidence_graph_sha256)?;
                    let canonical_response =
                        independently_verified_authority_response(&package.program, example)?;
                    Some(crate::RuntimeParityCase {
                        evidence_ref_sha256: receipt.evidence_graph_sha256.clone(),
                        capture_receipt: None,
                        request_text: String::new(),
                        provider_payload: example.provider_payload.clone(),
                        expected_response: canonical_response,
                    })
                })
                .collect();
            let durable_runtime_parity_receipts = bucket
                .future
                .iter()
                .filter_map(|receipt| {
                    bucket
                        .durable_runtime_parity_receipts
                        .get(&receipt.evidence_graph_sha256)
                        .cloned()
                })
                .collect();
            candidates.push(OnlineCollectionAdmissionCandidate {
                package,
                bucket_id: bucket.bucket_id.clone(),
                program_sha256: bucket
                    .frozen_program_sha256
                    .clone()
                    .ok_or_else(|| "online_collection_frozen_program_missing".to_owned())?,
                support_watermark_event_time_unix_nanos: bucket
                    .support_watermark_event_time_unix_nanos
                    .ok_or_else(|| "online_collection_support_watermark_missing".to_owned())?,
                support_manifest_sha256: bucket
                    .support_manifest_sha256
                    .clone()
                    .ok_or_else(|| "online_collection_support_manifest_missing".to_owned())?,
                future_manifest_sha256: collection_future_manifest_digest(bucket)?,
                causal_report,
                support_receipts: bucket.support.clone(),
                future_receipts: bucket.future.clone(),
                runtime_parity_cases,
                durable_runtime_parity_receipts,
            });
        }
        candidates.sort_by(|left, right| left.package.package_id.cmp(&right.package.package_id));
        Ok(candidates)
    }

    fn package_for_bucket(
        &self,
        index: usize,
        bucket: &OnlineCollectionBucket,
        wave_causal_pass: bool,
    ) -> Result<Option<ResponsePackage>, String> {
        let Some(program_sha256) = &bucket.frozen_program_sha256 else {
            return Ok(None);
        };
        let Some(support_manifest_sha256) = &bucket.support_manifest_sha256 else {
            return Ok(None);
        };
        if bucket.wrong_accepts > 0 {
            return Ok(None);
        }
        let future_manifest_sha256 = collection_future_manifest_digest(bucket)?;
        let program = bucket
            .programs
            .get(program_sha256)
            .ok_or_else(|| "online_collection_frozen_program_missing".to_owned())?
            .clone();
        let verifier = source_neutral_verifier_for_program(&program).map_err(str::to_owned)?;
        let verifier_schema = response_program_external_verifier_schema(&program)
            .ok_or_else(|| "online_collection_external_verifier_schema_missing".to_owned())?;
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        let phase_centers = bucket_phase_center_atom_ids(bucket);
        let anti_centers = self.anti_center_atom_ids(index);
        let route_sha256 =
            canonical_json_sha256(&(&required_routing_atom_ids, &phase_centers, &anti_centers))
                .map_err(str::to_owned)?;
        let wave_margin_micro = learned_wave_margin_micro(bucket, &phase_centers, &anti_centers);
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: format!(
                "evidence-collection-{}-{}-{}-{}",
                &program_sha256[..8],
                &support_manifest_sha256[..8],
                &future_manifest_sha256[..8],
                &route_sha256[..8]
            ),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program,
            verifier: Some(verifier),
            routing_predicates: Vec::new(),
            required_routing_atom_ids,
            phase_centers,
            anti_centers,
            wave_margin_micro,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: bucket.support.len(),
                future_rows: bucket.future.len(),
                distinct_sessions: distinct_receipt_sessions(&bucket.future),
                distinct_surfaces: distinct_receipt_layouts(&bucket.future),
                wrong_accepts: bucket.wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: verifier_schema.to_owned(),
            },
        };
        package.validate().map_err(str::to_owned)?;
        Ok(Some(package))
    }

    fn anti_center_atom_ids(&self, index: usize) -> Vec<u64> {
        self.checkpoint.buckets[index]
            .learned_anti_atom_ids
            .iter()
            .copied()
            .take(32)
            .collect()
    }

    fn learn_applicability_anti_atoms(&mut self, index: usize, negative: &OnlineCollectionReceipt) {
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return;
        };
        let support_atoms = bucket
            .support
            .iter()
            .flat_map(|receipt| receipt.request_atom_ids.iter().copied())
            .collect::<BTreeSet<_>>();
        let candidates = negative
            .request_atom_ids
            .iter()
            .copied()
            .filter(|atom| !support_atoms.contains(atom))
            .collect::<BTreeSet<_>>();
        if candidates.is_empty() {
            return;
        }
        let bucket_id = bucket.bucket_id.clone();
        let evidence = self
            .checkpoint
            .applicability_negative_sessions
            .entry(bucket_id)
            .or_default();
        let learned = update_applicability_negative_sessions(
            evidence,
            candidates,
            &negative.session_id_sha256,
        );
        if let Some(bucket) = self.checkpoint.buckets.get_mut(index) {
            bucket.learned_anti_atom_ids.extend(learned);
        }
    }

    fn collection_causal_report(
        &self,
        bucket: &OnlineCollectionBucket,
        package: &ResponsePackage,
    ) -> Result<OnlineCollectionWaveCausalReport, String> {
        let threshold = package.wave_margin_micro;
        let full = phase_vector_from_atom_ids(package.phase_centers.iter().copied(), 16);
        let anti = phase_vector_from_atom_ids(package.anti_centers.iter().copied(), 16);
        let shuffled = phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.rotate_left(17) ^ 0x9e37_79b9_7f4a_7c15),
            16,
        );
        let random = phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.wrapping_mul(0xd6e8_feb8_6659_fd93) ^ 0xa5a5_a5a5_a5a5_a5a5),
            16,
        );
        let no_anti = phase_vector_from_atom_ids(std::iter::empty::<u64>(), 16);
        let routes = |receipt: &OnlineCollectionReceipt,
                      center: &[nando_core::wave::PhaseCenterCell],
                      anti_center: &[nando_core::wave::PhaseCenterCell],
                      hard_anti_atoms: &[u64]| {
            if !package
                .required_routing_atom_ids
                .iter()
                .all(|atom| receipt.request_atom_ids.binary_search(atom).is_ok())
            {
                return false;
            }
            if hard_anti_atoms
                .iter()
                .any(|atom| receipt.request_atom_ids.binary_search(atom).is_ok())
            {
                return false;
            }
            let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
            phase_margin_to_micro(
                phase_coherence(&query, center) - phase_coherence(&query, anti_center),
            )
            .is_ok_and(|margin| margin >= threshold)
        };
        let full_phase_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &full, &anti, &package.anti_centers))
            .count();
        let shuffled_phase_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &shuffled, &anti, &package.anti_centers))
            .count();
        let random_center_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &random, &anti, &package.anti_centers))
            .count();
        let no_anti_center_correct = bucket
            .future
            .iter()
            .filter(|receipt| routes(receipt, &full, &no_anti, &[]))
            .count();
        let no_phase_candidates = self
            .checkpoint
            .buckets
            .iter()
            .map(|candidate| candidate.programs.len().max(1))
            .sum::<usize>()
            .max(1);
        let full_phase_exact_checks = full_phase_correct;
        let no_phase_exact_checks = bucket.future.len().saturating_mul(no_phase_candidates);
        let pass = bucket.support.len() >= 32
            && bucket.future.len() >= 32
            && distinct_receipt_sessions(&bucket.future) >= 3
            && distinct_receipt_layouts(&bucket.future) >= 2
            && bucket.wrong_accepts == 0
            && full_phase_correct == bucket.future.len()
            && full_phase_exact_checks < no_phase_exact_checks
            && full_phase_correct > shuffled_phase_correct
            && full_phase_correct > random_center_correct;
        Ok(OnlineCollectionWaveCausalReport {
            schema: "nando.online-collection-wave-causal-report.v1".to_owned(),
            package_id: package.package_id.clone(),
            verdict: if pass { "PASS" } else { "WATCH" }.to_owned(),
            support_rows: bucket.support.len(),
            future_rows: bucket.future.len(),
            full_phase_correct,
            no_phase_correct: bucket.future.len(),
            shuffled_phase_correct,
            random_center_correct,
            no_anti_center_correct,
            full_phase_exact_checks,
            no_phase_exact_checks,
            shuffled_phase_exact_checks: shuffled_phase_correct,
            random_center_exact_checks: random_center_correct,
            no_anti_center_exact_checks: no_anti_center_correct,
            wrong_accepts: bucket.wrong_accepts,
            wave_margin_micro: threshold,
        })
    }

    fn create_bucket(
        &mut self,
        programs: BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
    ) -> Result<(), String> {
        if programs.is_empty() {
            return Err("online_collection_empty_program_pool".to_owned());
        }
        if self.checkpoint.buckets.len() >= self.checkpoint.config.max_buckets {
            self.checkpoint.unsupported_total = self.checkpoint.unsupported_total.saturating_add(1);
            return Ok(());
        }
        let archetypes = programs
            .values()
            .map(response_program_archetype_id)
            .collect::<Result<BTreeSet<_>, _>>()?;
        if archetypes.len() != 1 {
            return Err("online_collection_mixed_archetype_bucket".to_owned());
        }
        let archetype_id = archetypes
            .into_iter()
            .next()
            .ok_or_else(|| "online_collection_archetype_missing".to_owned())?;
        let request_atoms = observation_request_atom_ids(observation);
        let program_digests = programs.keys().cloned().collect::<Vec<_>>();
        let bucket_id = collection_archetype_bucket_id(&archetype_id, &program_digests)?;
        if self
            .checkpoint
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == bucket_id)
        {
            return Ok(());
        }
        let support = vec![receipt_with_program_atoms(
            observation,
            verifier_pass,
            &programs,
        )?];
        self.checkpoint.program_pool_receipts_total = self
            .checkpoint
            .program_pool_receipts_total
            .saturating_add(1);
        self.checkpoint.buckets.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
            programs,
            common_request_atom_ids: request_atoms,
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::from([(
                observation.evidence_graph_sha256.clone(),
                observation.example.clone(),
            )]),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
        if let Some(bucket) = self.checkpoint.buckets.last_mut() {
            refresh_durable_adapter_phase_atoms(bucket);
        }
        Ok(())
    }

    fn assign_archetype_programs(
        &mut self,
        programs: BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
        count_observation: bool,
    ) -> Result<(), String> {
        let groups = group_programs_by_archetype(programs)?;
        if count_observation && groups.len() > 1 {
            self.checkpoint.ambiguous_assignment_total =
                self.checkpoint.ambiguous_assignment_total.saturating_add(1);
        }
        let mut proof_refresh_used = false;
        for (archetype_id, programs) in groups {
            let target = self
                .checkpoint
                .buckets
                .iter()
                .enumerate()
                .filter(|(_, bucket)| bucket.frozen_program_sha256.is_none())
                .filter(|(_, bucket)| bucket.archetype_id == archetype_id)
                .filter(|(_, bucket)| {
                    let additional = programs
                        .keys()
                        .filter(|digest| !bucket.programs.contains_key(*digest))
                        .count();
                    bucket.programs.len().saturating_add(additional)
                        <= crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS
                })
                .max_by_key(|(_, bucket)| bucket.support.len())
                .map(|(index, _)| index);
            if let Some(index) = target {
                self.update_bucket(
                    index,
                    &programs,
                    observation,
                    verifier_pass,
                    !proof_refresh_used,
                )?;
                proof_refresh_used = true;
            } else {
                self.create_bucket(programs, observation, verifier_pass)?;
            }
        }
        Ok(())
    }

    fn update_bucket(
        &mut self,
        index: usize,
        matching_programs: &BTreeMap<String, ResponseProgram>,
        observation: &OnlineCollectionObservation,
        verifier_pass: bool,
        refresh_proof: bool,
    ) -> Result<(), String> {
        if index >= self.checkpoint.buckets.len() {
            return Err("online_collection_bucket_missing".to_owned());
        }
        self.checkpoint.program_pool_reuse_total =
            self.checkpoint.program_pool_reuse_total.saturating_add(1);
        self.checkpoint.program_pool_receipts_total = self
            .checkpoint
            .program_pool_receipts_total
            .saturating_add(1);
        let bucket = self
            .checkpoint
            .buckets
            .get_mut(index)
            .expect("index checked above");
        for (digest, program) in matching_programs {
            bucket
                .programs
                .entry(digest.clone())
                .or_insert_with(|| program.clone());
        }
        let request_atoms = observation_request_atom_ids(observation);
        bucket
            .common_request_atom_ids
            .retain(|atom| request_atoms.contains(atom));
        if bucket.programs.is_empty() {
            return Err("online_collection_version_space_became_empty".to_owned());
        }
        merge_receipts(
            &mut bucket.support,
            vec![receipt_with_program_atoms(
                observation,
                verifier_pass,
                matching_programs,
            )?],
            self.checkpoint.config.max_receipts_per_bucket,
        );
        insert_runtime_example(
            bucket,
            observation,
            self.checkpoint.config.max_receipts_per_bucket,
        );
        refresh_durable_adapter_phase_atoms(bucket);
        self.normalize_bucket_receipts(index);
        let support_rows = self.checkpoint.buckets[index].support.len();
        if refresh_proof && support_rows >= self.checkpoint.config.support_rows {
            self.freeze_or_split(index)?;
        }
        Ok(())
    }

    fn matching_unfrozen_buckets(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<Vec<(usize, BTreeSet<String>)>, String> {
        let request_atoms = observation_request_atom_ids(observation);
        let query = phase_vector_from_atom_ids(request_atoms.iter().copied(), 16);
        let total_unfrozen = self
            .checkpoint
            .buckets
            .iter()
            .filter(|bucket| bucket.frozen_program_sha256.is_none())
            .count();
        let mut ranked_buckets = self
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.frozen_program_sha256.is_none())
            .filter(|(_, bucket)| {
                !bucket
                    .learned_anti_atom_ids
                    .iter()
                    .any(|atom| request_atoms.contains(atom))
            })
            .map(|(index, bucket)| {
                let phase_centers = bucket_phase_center_atom_ids(bucket);
                let anti_centers = bucket
                    .learned_anti_atom_ids
                    .iter()
                    .copied()
                    .take(32)
                    .collect::<Vec<_>>();
                let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
                let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
                let margin = phase_margin_to_micro(
                    phase_coherence(&query, &positive) - phase_coherence(&query, &negative),
                )
                .unwrap_or(i64::MIN);
                let threshold = learned_wave_margin_micro(
                    bucket,
                    phase_centers.as_slice(),
                    anti_centers.as_slice(),
                );
                let common_match = bucket
                    .common_request_atom_ids
                    .iter()
                    .all(|atom| request_atoms.contains(atom));
                let overlap = phase_centers
                    .iter()
                    .filter(|atom| request_atoms.contains(atom))
                    .count();
                (
                    index,
                    margin >= threshold,
                    common_match,
                    margin,
                    overlap,
                    bucket.support.len(),
                    bucket.bucket_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        ranked_buckets.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| right.4.cmp(&left.4))
                .then_with(|| right.5.cmp(&left.5))
                .then_with(|| left.6.cmp(&right.6))
        });
        ranked_buckets.truncate(MAX_UNFROZEN_ROUTE_BUCKETS);

        let mut checks = 0_u64;
        let scheduled = ranked_buckets.len() as u64;
        let mut matching = Vec::new();
        for (index, ..) in ranked_buckets {
            let bucket = &self.checkpoint.buckets[index];
            let mut ranked_programs = bucket
                .programs
                .iter()
                .map(|(digest, program)| {
                    let support_matches = bucket
                        .support
                        .iter()
                        .filter(|receipt| receipt.matched_program_sha256.contains(digest))
                        .count();
                    let routing_overlap = response_program_required_routing_atom_ids(program)
                        .iter()
                        .filter(|atom| request_atoms.contains(atom))
                        .count();
                    (digest, program, support_matches, routing_overlap)
                })
                .collect::<Vec<_>>();
            ranked_programs.sort_by(|left, right| {
                right
                    .2
                    .cmp(&left.2)
                    .then_with(|| right.3.cmp(&left.3))
                    .then_with(|| left.0.cmp(right.0))
            });
            let mut matching_programs = BTreeSet::new();
            for (digest, program, _, _) in ranked_programs
                .into_iter()
                .take(MAX_UNFROZEN_ROUTE_PROGRAMS)
            {
                checks = checks.saturating_add(1);
                if independently_verified_authority_response(program, &observation.example)
                    .is_some()
                {
                    matching_programs.insert(digest.clone());
                }
            }
            if !matching_programs.is_empty() {
                matching.push((index, matching_programs));
            }
        }
        self.checkpoint.version_space_intersection_checks_total = self
            .checkpoint
            .version_space_intersection_checks_total
            .saturating_add(checks);
        self.checkpoint.guard_scheduled_buckets_total = self
            .checkpoint
            .guard_scheduled_buckets_total
            .saturating_add(scheduled);
        self.checkpoint.guard_pruned_buckets_total = self
            .checkpoint
            .guard_pruned_buckets_total
            .saturating_add(total_unfrozen.saturating_sub(scheduled as usize) as u64);
        self.checkpoint.exact_checks_total =
            self.checkpoint.exact_checks_total.saturating_add(checks);
        Ok(matching)
    }

    fn evaluate_frozen_candidates(
        &mut self,
        observation: &OnlineCollectionObservation,
    ) -> Result<bool, String> {
        let mut verified_match = false;
        let mut verified_exact_teacher_match = false;
        let mut late_after_freeze = 0_u64;
        let mut future_intent_rejected = 0_u64;
        let mut route_candidates_considered = 0_u64;
        let mut route_anti_rejected = 0_u64;
        let mut route_phase_rejected = 0_u64;
        let mut route_verifier_rejected = 0_u64;
        let mut route_rejection_reasons = BTreeMap::<String, u64>::new();
        let mut route_witness_pending = 0_u64;
        let mut route_witness_resolved = 0_u64;
        let mut route_irreducible = 0_u64;
        let mut route_applicability_abstain = 0_u64;
        let mut future_accepted = 0_u64;
        let mut pending_subcenters = Vec::new();
        let mut pending_witness_successors = Vec::new();
        let mut witness_consumed = false;
        for index in 0..self.checkpoint.buckets.len() {
            let Some(program_sha256) = self.checkpoint.buckets[index].frozen_program_sha256.clone()
            else {
                continue;
            };
            route_candidates_considered = route_candidates_considered.saturating_add(1);
            let phase_centers = bucket_phase_center_atom_ids(&self.checkpoint.buckets[index]);
            let anti_centers = self.anti_center_atom_ids(index);
            let threshold = learned_wave_margin_micro(
                &self.checkpoint.buckets[index],
                &phase_centers,
                &anti_centers,
            );
            let routed_receipt = receipt_with_program_atoms(
                observation,
                true,
                &self.checkpoint.buckets[index].programs,
            )?;
            if routed_receipt.request_atom_ids.iter().any(|atom| {
                self.checkpoint.buckets[index]
                    .learned_anti_atom_ids
                    .contains(atom)
            }) {
                route_anti_rejected = route_anti_rejected.saturating_add(1);
                continue;
            }
            if !receipt_routes_phase(&routed_receipt, &phase_centers, &anti_centers, threshold) {
                route_phase_rejected = route_phase_rejected.saturating_add(1);
                continue;
            }
            let authority_result = {
                let bucket = &self.checkpoint.buckets[index];
                let Some(program) = bucket.programs.get(&program_sha256) else {
                    return Err("online_collection_frozen_program_missing".to_owned());
                };
                independently_verified_authority_response_result(program, &observation.example)
                    .and_then(|response| {
                        // Actor/verifier agreement is necessary but not enough:
                        // frozen future must reproduce the completed teacher.
                        (response == observation.example.expected_response)
                            .then_some(response)
                            .ok_or("teacher_response_mismatch")
                    })
            };
            let rejection_reason = authority_rejection_reason(&authority_result);
            let verifier_pass = rejection_reason.is_none();
            if !verifier_pass {
                route_verifier_rejected = route_verifier_rejected.saturating_add(1);
                let reason = rejection_reason.unwrap_or("unknown_verifier_rejection");
                *route_rejection_reasons
                    .entry(reason.to_owned())
                    .or_default() += 1;
                let witness_decision = active_witness_decision(
                    &self.checkpoint.buckets[index],
                    &program_sha256,
                    observation,
                    self.checkpoint.config.max_receipts_per_bucket,
                )?;
                match witness_decision {
                    ActiveWitnessDecision::Successor {
                        bucket: successor,
                        resolved,
                    } => {
                        if resolved {
                            route_witness_resolved = route_witness_resolved.saturating_add(1);
                        } else {
                            route_witness_pending = route_witness_pending.saturating_add(1);
                        }
                        pending_witness_successors.push(successor);
                        witness_consumed = true;
                        self.checkpoint.counterexamples_total =
                            self.checkpoint.counterexamples_total.saturating_add(1);
                        revoke_frozen_bucket(&mut self.checkpoint.buckets[index], &program_sha256);
                        self.checkpoint.revoked_candidates_total =
                            self.checkpoint.revoked_candidates_total.saturating_add(1);
                        continue;
                    }
                    ActiveWitnessDecision::Pending => {
                        route_witness_pending = route_witness_pending.saturating_add(1);
                        continue;
                    }
                    ActiveWitnessDecision::Irreducible => {
                        if !is_hard_teacher_counterexample(reason) {
                            self.learn_applicability_anti_atoms(index, &routed_receipt);
                            route_applicability_abstain =
                                route_applicability_abstain.saturating_add(1);
                            continue;
                        }
                    }
                }
                route_irreducible = route_irreducible.saturating_add(1);
                let bucket = &mut self.checkpoint.buckets[index];
                pending_subcenters.extend(counterexample_subcenters(
                    bucket,
                    &program_sha256,
                    &routed_receipt,
                )?);
                bucket.wrong_accepts = bucket.wrong_accepts.saturating_add(1);
                bucket.frozen_program_sha256 = None;
                bucket.support_watermark_event_time_unix_nanos = None;
                bucket.support_manifest_sha256 = None;
                revoke_frozen_bucket(bucket, &program_sha256);
                self.checkpoint.counterexamples_total =
                    self.checkpoint.counterexamples_total.saturating_add(1);
                self.checkpoint.revoked_candidates_total =
                    self.checkpoint.revoked_candidates_total.saturating_add(1);
            } else {
                let authority_response = authority_result.ok();
                let bucket = &mut self.checkpoint.buckets[index];
                let Some(program) = bucket.programs.get(&program_sha256) else {
                    return Err("online_collection_frozen_program_missing".to_owned());
                };
                verified_exact_teacher_match |= authority_response.as_deref()
                    == Some(observation.example.expected_response.as_str());
                let support_intents = bucket
                    .support
                    .iter()
                    .map(|receipt| receipt.client_intent_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let after_watermark = observation.event_time_unix_nanos.is_some_and(|event_time| {
                    bucket
                        .support_watermark_event_time_unix_nanos
                        .is_some_and(|watermark| event_time > watermark)
                });
                let intent_is_new =
                    !support_intents.contains(observation.client_intent_id_sha256.as_str());
                if after_watermark && intent_is_new {
                    let durable_parity = build_durable_runtime_parity_receipt(
                        program,
                        &observation.evidence_graph_sha256,
                        &observation.example,
                    )
                    .map_err(str::to_owned)?;
                    push_bounded(
                        &mut bucket.future,
                        routed_receipt,
                        self.checkpoint.config.max_receipts_per_bucket,
                    );
                    bucket
                        .durable_runtime_parity_receipts
                        .insert(observation.evidence_graph_sha256.clone(), durable_parity);
                    future_accepted = future_accepted.saturating_add(1);
                    let future_refs = bucket
                        .future
                        .iter()
                        .map(|receipt| receipt.evidence_graph_sha256.as_str())
                        .collect::<BTreeSet<_>>();
                    bucket
                        .durable_runtime_parity_receipts
                        .retain(|evidence_ref, _| future_refs.contains(evidence_ref.as_str()));
                } else if !after_watermark {
                    late_after_freeze = late_after_freeze.saturating_add(1);
                } else {
                    future_intent_rejected = future_intent_rejected.saturating_add(1);
                }
                verified_match = true;
            }
        }
        let available = self
            .checkpoint
            .config
            .max_buckets
            .saturating_sub(self.checkpoint.buckets.len());
        pending_witness_successors.append(&mut pending_subcenters);
        pending_witness_successors.truncate(available);
        for subcenter in pending_witness_successors {
            if self
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.bucket_id == subcenter.bucket_id)
            {
                continue;
            }
            self.checkpoint.buckets.push(subcenter);
            self.checkpoint.cegis_subcenters_total =
                self.checkpoint.cegis_subcenters_total.saturating_add(1);
        }
        self.checkpoint.late_after_freeze_total = self
            .checkpoint
            .late_after_freeze_total
            .saturating_add(late_after_freeze);
        self.checkpoint.future_intent_rejected_total = self
            .checkpoint
            .future_intent_rejected_total
            .saturating_add(future_intent_rejected);
        self.checkpoint.frozen_route_candidates_considered_total = self
            .checkpoint
            .frozen_route_candidates_considered_total
            .saturating_add(route_candidates_considered);
        self.checkpoint.frozen_route_anti_rejected_total = self
            .checkpoint
            .frozen_route_anti_rejected_total
            .saturating_add(route_anti_rejected);
        self.checkpoint.frozen_route_phase_rejected_total = self
            .checkpoint
            .frozen_route_phase_rejected_total
            .saturating_add(route_phase_rejected);
        self.checkpoint.frozen_route_verifier_rejected_total = self
            .checkpoint
            .frozen_route_verifier_rejected_total
            .saturating_add(route_verifier_rejected);
        for (reason, count) in route_rejection_reasons {
            let total = self
                .checkpoint
                .frozen_route_rejection_reasons
                .entry(reason)
                .or_default();
            *total = total.saturating_add(count);
        }
        self.checkpoint.frozen_route_witness_pending_total = self
            .checkpoint
            .frozen_route_witness_pending_total
            .saturating_add(route_witness_pending);
        self.checkpoint.frozen_route_witness_resolved_total = self
            .checkpoint
            .frozen_route_witness_resolved_total
            .saturating_add(route_witness_resolved);
        self.checkpoint.frozen_route_irreducible_total = self
            .checkpoint
            .frozen_route_irreducible_total
            .saturating_add(route_irreducible);
        self.checkpoint.frozen_route_applicability_abstain_total = self
            .checkpoint
            .frozen_route_applicability_abstain_total
            .saturating_add(route_applicability_abstain);
        self.checkpoint.frozen_future_accepted_total = self
            .checkpoint
            .frozen_future_accepted_total
            .saturating_add(future_accepted);
        if verified_match {
            self.record_executable_observation(verified_exact_teacher_match, true);
        }
        Ok(verified_match || witness_consumed)
    }

    fn maybe_freeze(&mut self, index: usize) -> Result<(), String> {
        let Some(bucket) = self.checkpoint.buckets.get_mut(index) else {
            return Ok(());
        };
        refresh_durable_adapter_phase_atoms(bucket);
        if bucket.support.len() >= self.checkpoint.config.support_rows
            && bucket.frozen_program_sha256.is_none()
            && bucket.support.iter().all(|receipt| receipt.verifier_pass)
        {
            if let SupportConsensusCandidate::Ready(candidate) =
                support_consensus_candidate(bucket)?
            {
                let digest = canonical_json_sha256(&candidate).map_err(str::to_owned)?;
                if !candidate_authority_verified_on_support(bucket, &candidate) {
                    return Err("online_collection_consensus_support_authority_unproven".to_owned());
                }
                for receipt in &mut bucket.support {
                    receipt.matched_program_sha256 = vec![digest.clone()];
                }
                bucket.programs = BTreeMap::from([(digest, candidate)]);
            }
        }
        if bucket.support.len() >= self.checkpoint.config.support_rows
            && bucket.programs.len() == 1
            && bucket.support.iter().all(|receipt| receipt.verifier_pass)
            // A singleton version space is still only a hypothesis until its
            // complete teacher response is proven on every support receipt.
            && bucket
                .programs
                .values()
                .next()
                .is_some_and(|program| candidate_authority_verified_on_support(bucket, program))
            && !bucket_program_atom_ids(bucket).is_empty()
            && bucket
                .support
                .iter()
                .all(|receipt| receipt.event_time_unix_nanos.is_some())
        {
            bucket.frozen_program_sha256 = bucket.programs.keys().next().cloned();
            bucket.support_watermark_event_time_unix_nanos = bucket
                .support
                .iter()
                .filter_map(|receipt| receipt.event_time_unix_nanos)
                .max();
            bucket.support_manifest_sha256 = Some(collection_support_manifest_digest(bucket)?);
            bucket.runtime_examples.clear();
            bucket.durable_adapter_phase_atoms.clear();
        }
        Ok(())
    }

    fn freeze_or_split(&mut self, index: usize) -> Result<(), String> {
        let law_subcenters = self
            .checkpoint
            .buckets
            .get(index)
            .filter(|bucket| {
                bucket.frozen_program_sha256.is_none()
                    && bucket.support.len() >= self.checkpoint.config.support_rows
            })
            .map(|bucket| {
                support_law_subcenters(
                    bucket,
                    self.checkpoint.config.support_rows,
                    self.checkpoint.config.max_receipts_per_bucket,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let preferred = law_subcenters.iter().cloned().find(|subcenter| {
            matches!(
                support_consensus_candidate(subcenter),
                Ok(SupportConsensusCandidate::Ready(_))
            )
        });
        if let Some(subcenter) = preferred {
            let subcenter_index = if let Some(existing) = self
                .checkpoint
                .buckets
                .iter()
                .position(|candidate| candidate.bucket_id == subcenter.bucket_id)
            {
                if self.checkpoint.buckets[existing]
                    .frozen_program_sha256
                    .is_none()
                {
                    self.checkpoint.buckets[existing] = subcenter;
                }
                existing
            } else {
                if self.checkpoint.buckets.len() >= self.checkpoint.config.max_buckets {
                    return Ok(());
                }
                self.checkpoint.buckets.push(subcenter);
                self.checkpoint.cegis_subcenters_total =
                    self.checkpoint.cegis_subcenters_total.saturating_add(1);
                self.checkpoint.buckets.len().saturating_sub(1)
            };
            self.normalize_bucket_receipts(subcenter_index);
            self.maybe_freeze(subcenter_index)?;
            return Ok(());
        }
        self.maybe_freeze(index)?;
        let Some(bucket) = self.checkpoint.buckets.get(index) else {
            return Ok(());
        };
        let blocker = support_freeze_blocker(bucket, self.checkpoint.config.support_rows);
        if !support_blocker_requires_subcenter_split(blocker.as_deref()) {
            return Ok(());
        }
        let law_subcenters = support_law_subcenters(
            bucket,
            self.checkpoint.config.support_rows,
            self.checkpoint.config.max_receipts_per_bucket,
        )?;
        let mut subcenters = Vec::new();
        for law_subcenter in law_subcenters {
            if let Some(decidable) = maximal_decidable_support_subcenter(
                &law_subcenter,
                self.checkpoint.config.support_rows,
                self.checkpoint.config.max_receipts_per_bucket,
            )? {
                subcenters.push(decidable);
            }
            if let Some(decidable) = clean_pre_action_program_subcenter(
                &law_subcenter,
                self.checkpoint.config.support_rows,
                self.checkpoint.config.max_receipts_per_bucket,
            )? {
                subcenters.push(decidable);
            }
            subcenters.push(law_subcenter);
        }
        subcenters.extend(support_program_subcenters(
            bucket,
            self.checkpoint.config.support_rows,
            self.checkpoint.config.max_receipts_per_bucket,
        )?);
        let mut seen = BTreeSet::new();
        subcenters.retain(|subcenter| seen.insert(subcenter.bucket_id.clone()));
        let available = self
            .checkpoint
            .config
            .max_buckets
            .saturating_sub(self.checkpoint.buckets.len());
        subcenters.truncate(available.min(4));
        for subcenter in subcenters {
            if let Some(existing) = self
                .checkpoint
                .buckets
                .iter()
                .position(|candidate| candidate.bucket_id == subcenter.bucket_id)
            {
                if self.checkpoint.buckets[existing]
                    .frozen_program_sha256
                    .is_none()
                {
                    self.checkpoint.buckets[existing] = subcenter;
                    self.normalize_bucket_receipts(existing);
                    self.maybe_freeze(existing)?;
                }
                continue;
            }
            self.checkpoint.buckets.push(subcenter);
            let subcenter_index = self.checkpoint.buckets.len().saturating_sub(1);
            self.normalize_bucket_receipts(subcenter_index);
            self.maybe_freeze(subcenter_index)?;
            self.checkpoint.cegis_subcenters_total =
                self.checkpoint.cegis_subcenters_total.saturating_add(1);
        }
        Ok(())
    }

    fn normalize_bucket_receipts(&mut self, index: usize) {
        let Some(bucket) = self.checkpoint.buckets.get_mut(index) else {
            return;
        };
        let atoms = bucket_program_atom_ids(bucket);
        for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
            receipt.request_atom_ids.extend(atoms.iter().copied());
            receipt.request_atom_ids.sort_unstable();
            receipt.request_atom_ids.dedup();
        }
    }

    fn persist(&self) -> Result<(), String> {
        // Raw provider payloads are bounded working memory, never durable
        // evidence. Receipts and the intersected program pool are sufficient
        // to resume; explicit replay can rehydrate examples when required.
        let mut durable_checkpoint = self.checkpoint.clone();
        for bucket in &mut durable_checkpoint.buckets {
            bucket.runtime_examples.clear();
            if bucket.frozen_program_sha256.is_some() {
                bucket.durable_adapter_phase_atoms.clear();
            } else {
                let support_refs = bucket
                    .support
                    .iter()
                    .map(|receipt| receipt.evidence_graph_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                bucket
                    .durable_adapter_phase_atoms
                    .retain(|evidence, _| support_refs.contains(evidence.as_str()));
            }
        }
        let payload = serde_cbor::to_vec(&durable_checkpoint)
            .map_err(|error| format!("online_collection_checkpoint_encode:{error}"))?;
        let mut bytes = Vec::with_capacity(
            ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3
                .len()
                .saturating_add(payload.len()),
        );
        bytes.extend_from_slice(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3);
        bytes.extend_from_slice(&payload);
        let temporary = self.path.with_extension("tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "online_collection_checkpoint_create:{}:{error}",
                    temporary.display()
                )
            })?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_collection_checkpoint_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_collection_checkpoint_sync:{error}"))?;
        fs::rename(&temporary, &self.path)
            .map_err(|error| format!("online_collection_checkpoint_rename:{error}"))?;
        sync_parent(&self.path)
    }
}

fn support_blocker_requires_subcenter_split(blocker: Option<&str>) -> bool {
    matches!(
        blocker,
        Some(
            "support_program_cover_empty"
                | "support_program_cover_incomplete"
                | "support_layout_adapter_unproven"
                | "support_phase_adapter_unproven"
                | "support_consensus_variant_budget_exceeded"
                | "support_consensus_authority_unproven"
        )
    )
}

fn migrate_collection_keyed_layouts(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    for bucket in &mut checkpoint.buckets {
        let layouts = bucket
            .runtime_examples
            .iter()
            .map(|(evidence_id, example)| {
                structural_layout_sha256(&example.provider_payload)
                    .map(|layout| (evidence_id.clone(), layout))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
            if let Some(layout) = layouts.get(&receipt.evidence_graph_sha256) {
                receipt.layout_sha256.clone_from(layout);
            }
        }
    }
    Ok(())
}

fn response_program_surface_priority(program: &ResponseProgram) -> u8 {
    let renderer = match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue { renderer, .. }
        | crate::ResponseOperation::ProjectStatus { renderer, .. }
        | crate::ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return 2,
    };
    u8::from(renderer.is_direct())
}

fn decode_collection_checkpoint(bytes: &[u8]) -> Result<OnlineCollectionCheckpoint, String> {
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    if let Some(payload) = bytes.strip_prefix(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2) {
        return serde_cbor::from_slice(payload)
            .map_err(|error| format!("online_collection_checkpoint_decode_cbor:{error}"));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| format!("online_collection_checkpoint_decode_legacy_json:{error}"))
}

fn migrate_collection_program_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V1
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V2
        && checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
    {
        return Err("online_collection_checkpoint_schema_unknown".to_owned());
    }
    let legacy_observations = checkpoint.observations_total;
    let legacy_buckets = checkpoint.buckets.len() as u64;
    let legacy_receipts = checkpoint
        .buckets
        .iter()
        .map(|bucket| bucket.support.len().saturating_add(bucket.future.len()) as u64)
        .sum::<u64>();

    // V1 accepted component matches. Those receipts cannot prove an exact CPU
    // response after the raw example has been intentionally discarded.
    checkpoint.schema = ONLINE_COLLECTION_SCHEMA_V3.to_owned();
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V3;
    checkpoint.observations_total = 0;
    checkpoint.duplicate_observations_total = 0;
    checkpoint.observed_evidence_graph_sha256.clear();
    checkpoint.unsupported_total = 0;
    checkpoint.ambiguous_assignment_total = 0;
    checkpoint.exact_checks_total = 0;
    checkpoint.candidates_enumerated_total = 0;
    checkpoint.full_enumerations_total = 0;
    checkpoint.version_space_intersection_checks_total = 0;
    checkpoint.guard_scheduled_buckets_total = 0;
    checkpoint.guard_pruned_buckets_total = 0;
    checkpoint.unsupported_expected_in_latest_output = 0;
    checkpoint.unsupported_expected_in_any_output = 0;
    checkpoint.unsupported_without_exact_source_span = 0;
    checkpoint.unsupported_with_scalar_overlap = 0;
    checkpoint.policy_rejected_exact_matches = 0;
    checkpoint.counterexamples_total = 0;
    checkpoint.cegis_subcenters_total = 0;
    checkpoint.revoked_candidates_total = 0;
    checkpoint.late_after_freeze_total = 0;
    checkpoint.future_intent_rejected_total = 0;
    checkpoint.frozen_route_candidates_considered_total = 0;
    checkpoint.frozen_route_anti_rejected_total = 0;
    checkpoint.frozen_route_phase_rejected_total = 0;
    checkpoint.frozen_route_verifier_rejected_total = 0;
    checkpoint.frozen_future_accepted_total = 0;
    checkpoint.exact_executable_observations_total = 0;
    checkpoint.teacher_only_observations_total = 0;
    checkpoint.program_pool_reuse_total = 0;
    checkpoint.program_pool_receipts_total = 0;
    checkpoint.legacy_partial_observations_discarded_total = checkpoint
        .legacy_partial_observations_discarded_total
        .saturating_add(legacy_observations);
    checkpoint.legacy_partial_buckets_discarded_total = checkpoint
        .legacy_partial_buckets_discarded_total
        .saturating_add(legacy_buckets);
    checkpoint.legacy_partial_receipts_discarded_total = checkpoint
        .legacy_partial_receipts_discarded_total
        .saturating_add(legacy_receipts);
    checkpoint.unreplayable_support_discarded_total = 0;
    checkpoint.buckets.clear();
    Ok(())
}

fn migrate_collection_archetype_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut rebuilt = Vec::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        if bucket.programs.is_empty() {
            continue;
        }
        if bucket.frozen_program_sha256.is_some() {
            let mut bucket = bucket;
            bucket.archetype_id = bucket
                .programs
                .values()
                .next()
                .map(response_program_archetype_id)
                .transpose()?
                .unwrap_or_else(|| format!("legacy-frozen:{}", bucket.bucket_id));
            let program_digests = bucket.programs.keys().cloned().collect::<Vec<_>>();
            for receipt in bucket.support.iter_mut().chain(bucket.future.iter_mut()) {
                if receipt.matched_program_sha256.is_empty() {
                    receipt.matched_program_sha256 = program_digests.clone();
                }
            }
            rebuilt.push(bucket);
            continue;
        }

        let mut groups = BTreeMap::<String, BTreeMap<String, ResponseProgram>>::new();
        for (digest, program) in &bucket.programs {
            groups
                .entry(response_program_archetype_id(program)?)
                .or_default()
                .insert(digest.clone(), program.clone());
        }
        for (archetype_id, programs) in groups {
            let mut migrated = bucket.clone();
            migrated.archetype_id = archetype_id.clone();
            migrated.programs =
                bounded_program_map(programs, crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS);
            let program_digests = migrated.programs.keys().cloned().collect::<Vec<_>>();
            for receipt in &mut migrated.support {
                receipt.matched_program_sha256 = program_digests.clone();
            }
            migrated.future.clear();
            migrated.runtime_examples.clear();
            migrated.durable_runtime_parity_receipts.clear();
            migrated.frozen_program_sha256 = None;
            migrated.support_watermark_event_time_unix_nanos = None;
            migrated.support_manifest_sha256 = None;
            migrated.bucket_id =
                collection_archetype_bucket_id(&archetype_id, migrated.programs.keys())?;
            rebuilt.push(migrated);
        }
    }
    checkpoint.buckets = rebuilt;
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V4;
    Ok(())
}

fn migrate_collection_exact_authority_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut rebuilt = Vec::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        let mut groups = BTreeMap::<String, BTreeMap<String, ResponseProgram>>::new();
        for (digest, program) in &bucket.programs {
            groups
                .entry(response_program_archetype_id(program)?)
                .or_default()
                .insert(digest.clone(), program.clone());
        }
        for (archetype_id, programs) in groups {
            let mut migrated = bucket.clone();
            migrated.archetype_id = archetype_id.clone();
            migrated.programs =
                bounded_program_map(programs, crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS);
            checkpoint.unreplayable_support_discarded_total = checkpoint
                .unreplayable_support_discarded_total
                .saturating_add(migrated.support.len() as u64);
            migrated.support.clear();
            migrated.future.clear();
            migrated.runtime_examples.clear();
            migrated.durable_runtime_parity_receipts.clear();
            migrated.frozen_program_sha256 = None;
            migrated.support_watermark_event_time_unix_nanos = None;
            migrated.support_manifest_sha256 = None;
            migrated.rejected_program_sha256.clear();
            migrated.learned_anti_atom_ids.clear();
            migrated.wrong_accepts = 0;
            migrated.bucket_id =
                collection_archetype_bucket_id(&archetype_id, migrated.programs.keys())?;
            rebuilt.push(migrated);
        }
    }
    checkpoint.buckets = rebuilt;
    checkpoint.exact_executable_observations_total = 0;
    checkpoint.teacher_only_observations_total = checkpoint.observations_total;
    checkpoint.unsupported_total = checkpoint.observations_total;
    checkpoint.ambiguous_assignment_total = 0;
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V5;
    Ok(())
}

fn migrate_collection_renderer_consensus_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    type RankedProgram = (ResponseProgram, usize);
    type RendererPool = (
        BTreeMap<String, RankedProgram>,
        BTreeMap<String, CollectionSynthesisExample>,
    );

    let mut preserved = Vec::new();
    let mut pools = BTreeMap::<String, RendererPool>::new();
    let mut migrated_examples = BTreeSet::new();
    for bucket in std::mem::take(&mut checkpoint.buckets) {
        if bucket.frozen_program_sha256.is_some()
            || !bucket.support.is_empty()
            || !bucket.future.is_empty()
            || bucket.runtime_examples.is_empty()
        {
            preserved.push(bucket);
            continue;
        }

        let mut bucket_produced_exact = false;
        for (evidence_id, example) in &bucket.runtime_examples {
            let Ok(space) = enumerate_source_neutral_response_programs(example) else {
                continue;
            };
            for program in space.programs.into_iter().filter(|program| {
                crate::response_program_exactly_matches_example(program, example)
                    && is_privacy_safe_online_response_program(program)
            }) {
                let archetype_id = response_program_archetype_id(&program)?;
                let digest = canonical_json_sha256(&program).map_err(str::to_owned)?;
                let (programs, examples) = pools.entry(archetype_id).or_default();
                let ranked = programs.entry(digest).or_insert((program, 0));
                ranked.1 = ranked.1.saturating_add(1);
                examples
                    .entry(evidence_id.clone())
                    .or_insert_with(|| example.clone());
                migrated_examples.insert(evidence_id.clone());
                bucket_produced_exact = true;
            }
        }
        if !bucket_produced_exact {
            preserved.push(bucket);
        }
    }

    for (archetype_id, (programs, examples)) in pools {
        let mut ranked = programs.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .1
                .cmp(&left.1.1)
                .then_with(|| {
                    response_program_surface_priority(&right.1.0)
                        .cmp(&response_program_surface_priority(&left.1.0))
                })
                .then_with(|| left.0.cmp(&right.0))
        });
        let programs = ranked
            .into_iter()
            .take(crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS)
            .map(|(digest, (program, _))| (digest, program))
            .collect::<BTreeMap<_, _>>();
        if programs.is_empty() {
            continue;
        }
        let bucket_id = collection_archetype_bucket_id(&archetype_id, programs.keys())?;
        let mut runtime_examples = examples;
        trim_runtime_examples(
            &mut runtime_examples,
            checkpoint.config.max_receipts_per_bucket,
        );
        preserved.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support: Vec::new(),
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
    }
    checkpoint.buckets = preserved;
    checkpoint.renderer_consensus_migrated_examples_total = checkpoint
        .renderer_consensus_migrated_examples_total
        .saturating_add(migrated_examples.len() as u64);
    checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V6;
    Ok(())
}

fn repair_collection_checkpoint_accounting(checkpoint: &mut OnlineCollectionCheckpoint) -> bool {
    let mut repaired = false;
    if checkpoint.pooling_strategy_version == ONLINE_COLLECTION_POOLING_STRATEGY_V5
        && checkpoint.teacher_only_observations_total == checkpoint.observations_total
        && checkpoint.exact_executable_observations_total == 0
        && checkpoint.unsupported_total == checkpoint.observations_total
        && checkpoint.ambiguous_assignment_total > 0
    {
        checkpoint.ambiguous_assignment_total = 0;
        repaired = true;
    }
    for bucket in &mut checkpoint.buckets {
        if bucket.frozen_program_sha256.is_none() && !bucket.support.is_empty() {
            let before = bucket.support.len();
            bucket.support.retain(|receipt| {
                receipt.verifier_pass && !receipt.matched_program_sha256.is_empty()
            });
            let discarded = before.saturating_sub(bucket.support.len());
            if discarded > 0 {
                checkpoint.unreplayable_support_discarded_total = checkpoint
                    .unreplayable_support_discarded_total
                    .saturating_add(discarded as u64);
                repaired = true;
            }
        }
    }
    repaired
}

fn response_program_archetype_id(program: &ResponseProgram) -> Result<String, String> {
    let material = match &program.operation {
        crate::ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut archetypes = variants
                .iter()
                .map(|variant| response_program_archetype_id(&variant.program))
                .collect::<Result<BTreeSet<_>, _>>()?;
            if archetypes.len() != 1 {
                return Err("online_collection_consensus_archetype_mismatch".to_owned());
            }
            archetypes
                .pop_first()
                .ok_or_else(|| "online_collection_consensus_archetype_empty".to_owned())?
        }
        crate::ResponseOperation::ProjectSelectedValue { .. } => "project".to_owned(),
        crate::ResponseOperation::ProjectStatus { .. } => "status".to_owned(),
        crate::ResponseOperation::ComposeCollection { steps, .. } => {
            let has_filter = steps.iter().any(|step| {
                matches!(
                    step,
                    crate::CollectionProgramStep::FilterUniqueFieldEquals { .. }
                        | crate::CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        | crate::CollectionProgramStep::FilterFieldEquals { .. }
                )
            });
            let has_count = steps
                .iter()
                .any(|step| matches!(step, crate::CollectionProgramStep::Count));
            let has_aggregate = steps.iter().any(|step| {
                matches!(
                    step,
                    crate::CollectionProgramStep::AggregateUniqueIntegerField { .. }
                )
            });
            match (has_filter, has_count, has_aggregate) {
                (true, true, _) => "collection:compose_filter_count".to_owned(),
                (false, true, _) => "collection:count".to_owned(),
                (true, false, _) => "collection:filter".to_owned(),
                (false, false, true) => "collection:aggregate".to_owned(),
                (false, false, false) => "collection:compose".to_owned(),
            }
        }
        _ => return Err("online_collection_program_archetype_unsupported".to_owned()),
    };
    canonical_json_sha256(&("nando.collection-archetype.v1", material)).map_err(str::to_owned)
}

fn group_programs_by_archetype(
    programs: BTreeMap<String, ResponseProgram>,
) -> Result<Vec<ArchetypeProgramPool>, String> {
    let mut groups = BTreeMap::<String, Vec<(String, ResponseProgram)>>::new();
    for (digest, program) in programs {
        groups
            .entry(response_program_archetype_id(&program)?)
            .or_default()
            .push((digest, program));
    }
    groups
        .into_iter()
        .map(|(archetype, variants)| {
            Ok((
                archetype,
                bounded_program_map(
                    variants.into_iter().collect(),
                    MAX_NEW_ADAPTERS_PER_OBSERVATION,
                ),
            ))
        })
        .collect()
}

fn structural_programs_for_observation(
    observation: &OnlineCollectionObservation,
) -> Result<BTreeMap<String, ResponseProgram>, String> {
    let synthesis_example = compact_active_turn_synthesis_example(&observation.example)
        .unwrap_or_else(|| observation.example.clone());
    enumerate_source_neutral_structural_response_programs(&synthesis_example)
        .map_err(str::to_owned)?
        .into_iter()
        .filter(is_privacy_safe_online_response_program)
        .filter(|program| {
            independently_verified_authority_response(program, &observation.example).is_some()
        })
        .map(|program| {
            canonical_json_sha256(&program)
                .map(|digest| (digest, program))
                .map_err(str::to_owned)
        })
        .collect()
}

fn compact_active_turn_synthesis_example(
    example: &CollectionSynthesisExample,
) -> Option<CollectionSynthesisExample> {
    let input = example.provider_payload.get("input")?.as_array()?;
    let last_user = input
        .iter()
        .rposition(|item| item.get("role").and_then(Value::as_str) == Some("user"))?;
    if last_user == 0 {
        return None;
    }
    let mut provider_payload = example.provider_payload.clone();
    provider_payload["input"] = Value::Array(input[last_user..].to_vec());
    Some(CollectionSynthesisExample {
        provider_payload,
        expected_response: example.expected_response.clone(),
    })
}

fn bounded_program_map(
    programs: BTreeMap<String, ResponseProgram>,
    limit: usize,
) -> BTreeMap<String, ResponseProgram> {
    let mut variants = programs.into_iter().collect::<Vec<_>>();
    let preferred_dynamic = variants
        .iter()
        .filter(|(_, program)| canonical_dynamic_role_count(program) >= 2)
        .max_by(|(left_digest, left), (right_digest, right)| {
            canonical_dynamic_role_count(left)
                .cmp(&canonical_dynamic_role_count(right))
                .then_with(|| {
                    serde_json::to_vec(right)
                        .unwrap_or_default()
                        .len()
                        .cmp(&serde_json::to_vec(left).unwrap_or_default().len())
                })
                .then_with(|| right_digest.cmp(left_digest))
        })
        .cloned();
    variants.sort_by(|(left_digest, left), (right_digest, right)| {
        response_program_surface_priority(left)
            .cmp(&response_program_surface_priority(right))
            .then_with(|| {
                serde_json::to_vec(left)
                    .unwrap_or_default()
                    .len()
                    .cmp(&serde_json::to_vec(right).unwrap_or_default().len())
            })
            .then_with(|| left_digest.cmp(right_digest))
    });
    if let Some((preferred_digest, _)) = &preferred_dynamic {
        variants.retain(|(digest, _)| digest != preferred_digest);
        variants.truncate(limit.saturating_sub(1));
        if limit > 0 {
            variants.push(preferred_dynamic.expect("preferred dynamic program"));
        }
    } else {
        variants.truncate(limit);
    }
    variants.into_iter().collect()
}

fn buckets_share_execution_law(
    left: &OnlineCollectionBucket,
    right: &OnlineCollectionBucket,
) -> bool {
    let left_laws = left
        .programs
        .values()
        .filter_map(|program| response_law_key(program).ok())
        .collect::<BTreeSet<_>>();
    right.programs.values().any(|program| {
        response_law_key(program)
            .ok()
            .is_some_and(|law| left_laws.contains(&law))
    })
}

fn select_program_receipt_cover(
    programs: &BTreeMap<String, ResponseProgram>,
    receipts: &[OnlineCollectionReceipt],
    budget: usize,
) -> Option<BTreeSet<String>> {
    if programs.len() <= budget {
        return Some(programs.keys().cloned().collect());
    }
    let mut coverage = BTreeMap::<String, BTreeSet<usize>>::new();
    for (index, receipt) in receipts.iter().enumerate() {
        for digest in &receipt.matched_program_sha256 {
            if programs.contains_key(digest) {
                coverage.entry(digest.clone()).or_default().insert(index);
            }
        }
    }
    let mut uncovered = (0..receipts.len()).collect::<BTreeSet<_>>();
    let mut selected = BTreeSet::<String>::new();
    while !uncovered.is_empty() && selected.len() < budget {
        let next = coverage
            .iter()
            .filter(|(digest, _)| !selected.contains(*digest))
            .map(|(digest, covered)| {
                let gain = covered.intersection(&uncovered).count();
                let program = &programs[digest];
                let bytes = serde_json::to_vec(program).map_or(usize::MAX, |value| value.len());
                (
                    gain,
                    canonical_direct_response_program(program)
                        .is_ok_and(|canonical| is_source_neutral_response_program(&canonical)),
                    canonical_dynamic_role_count(program),
                    bytes,
                    digest,
                    covered,
                )
            })
            .filter(|(gain, _, _, _, _, _)| *gain > 0)
            .max_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| right.3.cmp(&left.3))
                    .then_with(|| right.4.cmp(left.4))
            })?;
        selected.insert(next.4.clone());
        for index in next.5 {
            uncovered.remove(index);
        }
    }
    if !uncovered.is_empty() {
        return None;
    }
    let mut remainder = programs
        .iter()
        .filter(|(digest, _)| !selected.contains(*digest))
        .map(|(digest, program)| {
            (
                coverage.get(digest).map_or(0, BTreeSet::len),
                canonical_direct_response_program(program)
                    .is_ok_and(|canonical| is_source_neutral_response_program(&canonical)),
                canonical_dynamic_role_count(program),
                serde_json::to_vec(program).map_or(usize::MAX, |value| value.len()),
                digest.clone(),
            )
        })
        .collect::<Vec<_>>();
    remainder.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
    });
    for (_, _, _, _, digest) in remainder {
        if selected.len() >= budget {
            break;
        }
        selected.insert(digest);
    }
    Some(selected)
}

fn canonical_dynamic_role_count(program: &ResponseProgram) -> usize {
    let Ok(canonical) = canonical_direct_response_program(program) else {
        return 0;
    };
    let renderer = match canonical.operation {
        crate::ResponseOperation::ProjectSelectedValue { renderer, .. }
        | crate::ResponseOperation::ProjectStatus { renderer, .. }
        | crate::ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return 0,
    };
    match renderer {
        crate::CollectionOutputRenderer::RenderSequence { segments } => segments
            .iter()
            .filter(|segment| !matches!(segment, crate::ResponseRenderSegment::Static { .. }))
            .count(),
        crate::CollectionOutputRenderer::Direct => 1,
        _ => 0,
    }
}

fn collection_archetype_bucket_id<'a>(
    archetype_id: &str,
    program_digests: impl IntoIterator<Item = &'a String>,
) -> Result<String, String> {
    let digests = program_digests.into_iter().cloned().collect::<Vec<_>>();
    canonical_json_sha256(&("nando.collection-archetype-pool.v1", archetype_id, digests))
        .map_err(str::to_owned)
}

fn insert_runtime_example(
    bucket: &mut OnlineCollectionBucket,
    observation: &OnlineCollectionObservation,
    limit: usize,
) {
    insert_runtime_example_for_evidence(
        bucket,
        &observation.evidence_graph_sha256,
        observation,
        limit,
    );
}

fn insert_runtime_example_for_evidence(
    bucket: &mut OnlineCollectionBucket,
    evidence_id: &str,
    observation: &OnlineCollectionObservation,
    limit: usize,
) {
    let stored_example =
        compact_runtime_example(bucket, observation).unwrap_or_else(|| observation.example.clone());
    bucket
        .runtime_examples
        .insert(evidence_id.to_owned(), stored_example);
    trim_bucket_runtime_examples(bucket, limit);
}

fn trim_bucket_runtime_examples(bucket: &mut OnlineCollectionBucket, limit: usize) {
    let best_law_key = best_bucket_law_key(bucket);
    while bucket.runtime_examples.len() > limit
        || persisted_runtime_example_bytes(&bucket.runtime_examples)
            > MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    {
        let candidate = bucket
            .runtime_examples
            .iter()
            .max_by(|(left_id, left), (right_id, right)| {
                let left_outside_best = best_law_key
                    .as_ref()
                    .is_some_and(|law_key| !receipt_supports_law(bucket, left_id, law_key));
                let right_outside_best = best_law_key
                    .as_ref()
                    .is_some_and(|law_key| !receipt_supports_law(bucket, right_id, law_key));
                left_outside_best
                    .cmp(&right_outside_best)
                    .then_with(|| {
                        persisted_runtime_example_size(left_id, left)
                            .cmp(&persisted_runtime_example_size(right_id, right))
                    })
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(evidence_id, _)| evidence_id.clone());
        let Some(candidate) = candidate else {
            break;
        };
        bucket.runtime_examples.remove(&candidate);
    }
}

fn refresh_durable_adapter_phase_atoms(bucket: &mut OnlineCollectionBucket) {
    let support_refs = bucket
        .support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>();
    bucket
        .durable_adapter_phase_atoms
        .retain(|evidence, _| support_refs.contains(evidence.as_str()));
    for atoms_by_program in bucket.durable_adapter_phase_atoms.values_mut() {
        atoms_by_program.retain(|program_sha256, _| bucket.programs.contains_key(program_sha256));
    }
    for (evidence, example) in &bucket.runtime_examples {
        if !support_refs.contains(evidence.as_str()) {
            continue;
        }
        let atoms_by_program = bucket
            .durable_adapter_phase_atoms
            .entry(evidence.clone())
            .or_default();
        for (program_sha256, program) in &bucket.programs {
            let mut atoms =
                crate::runtime::actor_adapter_phase_atom_ids(program, &example.provider_payload);
            atoms.sort_unstable();
            atoms.dedup();
            if atoms.is_empty() || atoms.len() > MAX_DURABLE_ADAPTER_PHASE_ATOMS {
                atoms_by_program.remove(program_sha256);
            } else {
                atoms_by_program.insert(program_sha256.clone(), atoms);
            }
        }
    }
    bucket
        .durable_adapter_phase_atoms
        .retain(|_, atoms_by_program| !atoms_by_program.is_empty());
}

fn durable_adapter_phase_subset(
    bucket: &OnlineCollectionBucket,
    evidence_ids: &BTreeSet<String>,
    program_ids: &BTreeSet<String>,
) -> BTreeMap<String, BTreeMap<String, Vec<u64>>> {
    bucket
        .durable_adapter_phase_atoms
        .iter()
        .filter(|(evidence_id, _)| evidence_ids.contains(*evidence_id))
        .filter_map(|(evidence_id, atoms_by_program)| {
            let retained = atoms_by_program
                .iter()
                .filter(|(program_sha256, _)| program_ids.contains(*program_sha256))
                .map(|(program_sha256, atoms)| (program_sha256.clone(), atoms.clone()))
                .collect::<BTreeMap<_, _>>();
            (!retained.is_empty()).then(|| (evidence_id.clone(), retained))
        })
        .collect()
}

fn best_bucket_law_key(bucket: &OnlineCollectionBucket) -> Option<Vec<u8>> {
    let digest_law_keys = bucket
        .programs
        .iter()
        .filter_map(|(digest, program)| {
            response_law_key(program)
                .ok()
                .map(|law_key| (digest.as_str(), law_key))
        })
        .collect::<BTreeMap<_, _>>();
    let mut support = BTreeMap::<Vec<u8>, usize>::new();
    for receipt in &bucket.support {
        let receipt_laws = receipt
            .matched_program_sha256
            .iter()
            .filter_map(|digest| digest_law_keys.get(digest.as_str()).cloned())
            .collect::<BTreeSet<_>>();
        for law_key in receipt_laws {
            *support.entry(law_key).or_default() += 1;
        }
    }
    support
        .into_iter()
        .max_by(|(left_key, left_rows), (right_key, right_rows)| {
            left_rows
                .cmp(right_rows)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(law_key, _)| law_key)
}

fn receipt_supports_law(
    bucket: &OnlineCollectionBucket,
    evidence_id: &str,
    law_key: &[u8],
) -> bool {
    bucket
        .support
        .iter()
        .find(|receipt| receipt.evidence_graph_sha256 == evidence_id)
        .is_some_and(|receipt| {
            receipt.matched_program_sha256.iter().any(|digest| {
                bucket
                    .programs
                    .get(digest)
                    .and_then(|program| response_law_key(program).ok())
                    .is_some_and(|candidate| candidate == law_key)
            })
        })
}

fn compact_runtime_example(
    bucket: &OnlineCollectionBucket,
    observation: &OnlineCollectionObservation,
) -> Option<CollectionSynthesisExample> {
    let input = observation
        .example
        .provider_payload
        .get("input")?
        .as_array()?;
    let last_user = input
        .iter()
        .rposition(|item| item.get("role").and_then(Value::as_str) == Some("user"));
    let compact_input = input
        .iter()
        .enumerate()
        .filter(|(index, item)| {
            Some(*index) == last_user
                || matches!(
                    item.get("type").and_then(Value::as_str),
                    Some("function_call_output" | "custom_tool_call_output")
                )
        })
        .map(|(_, item)| item.clone())
        .collect::<Vec<_>>();
    if compact_input.is_empty() || compact_input.len() == input.len() {
        return None;
    }
    let compact = CollectionSynthesisExample {
        provider_payload: serde_json::json!({"input": compact_input}),
        expected_response: observation.example.expected_response.clone(),
    };
    let matched_digests = bucket
        .support
        .iter()
        .find(|receipt| receipt.evidence_graph_sha256 == observation.evidence_graph_sha256)
        .map(|receipt| &receipt.matched_program_sha256)?;
    if matched_digests.is_empty() {
        return None;
    }
    for digest in matched_digests {
        let program = bucket.programs.get(digest)?;
        let full_response =
            independently_verified_authority_response(program, &observation.example)?;
        let compact_response = independently_verified_authority_response(program, &compact)?;
        if compact_response != full_response {
            return None;
        }
    }
    Some(compact)
}

fn trim_runtime_examples(
    examples: &mut BTreeMap<String, CollectionSynthesisExample>,
    limit: usize,
) {
    while examples.len() > limit
        || persisted_runtime_example_bytes(examples) > MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    {
        let Some(oldest) = examples.keys().next().cloned() else {
            break;
        };
        examples.remove(&oldest);
    }
}

fn persisted_runtime_example_bytes(
    examples: &BTreeMap<String, CollectionSynthesisExample>,
) -> usize {
    examples
        .iter()
        .map(|(digest, example)| persisted_runtime_example_size(digest, example))
        .fold(0_usize, usize::saturating_add)
}

fn persisted_runtime_example_size(digest: &str, example: &CollectionSynthesisExample) -> usize {
    digest
        .len()
        .saturating_add(serde_cbor::to_vec(example).map_or(0, |bytes| bytes.len()))
}

enum UnsupportedSourceSpan {
    Latest,
    Earlier,
    Missing,
}

fn unsupported_source_span(example: &CollectionSynthesisExample) -> UnsupportedSourceSpan {
    let outputs = example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if outputs
        .last()
        .is_some_and(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Latest
    } else if outputs
        .iter()
        .any(|output| output.contains(&example.expected_response))
    {
        UnsupportedSourceSpan::Earlier
    } else {
        UnsupportedSourceSpan::Missing
    }
}

fn has_scalar_overlap(example: &CollectionSynthesisExample) -> bool {
    example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .any(|output| {
            let mut scalars = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(output) {
                collect_scalar_strings(&value, &mut scalars);
            }
            scalars.extend(
                output
                    .split(|character: char| {
                        character.is_whitespace()
                            || matches!(character, ':' | '=' | ',' | ';' | '[' | ']' | '{' | '}')
                    })
                    .filter(|value| value.len() >= 2 && value.len() <= 128)
                    .map(str::to_owned),
            );
            scalars
                .iter()
                .any(|scalar| example.expected_response.contains(scalar))
        })
}

fn collect_scalar_strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Null => {}
        Value::Bool(value) => output.push(value.to_string()),
        Value::Number(value) => output.push(value.to_string()),
        Value::String(value) if value.len() >= 2 && value.len() <= 128 => {
            output.push(value.clone());
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_strings(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_scalar_strings(value, output);
            }
        }
        Value::String(_) => {}
    }
}

fn validate_config(config: OnlineCollectionConfig) -> Result<(), String> {
    if config.support_rows == 0
        || config.future_rows == 0
        || config.max_buckets == 0
        || config.max_receipts_per_bucket < config.support_rows.max(config.future_rows)
    {
        return Err("online_collection_invalid_config".to_owned());
    }
    Ok(())
}

fn support_program_subcenters(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let mut ranked = Vec::new();
    for (program_sha256, program) in &bucket.programs {
        if !is_source_neutral_response_program(program) {
            continue;
        }
        let mut support = bucket
            .support
            .iter()
            .filter(|receipt| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == program_sha256)
            })
            .cloned()
            .collect::<Vec<_>>();
        if support.len() < required_support_rows {
            continue;
        }
        support.truncate(max_receipts_per_bucket);
        for receipt in &mut support {
            receipt.matched_program_sha256 = vec![program_sha256.clone()];
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-support-program-subcenter.v1",
            &bucket.archetype_id,
            program_sha256,
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-support-program-subcenter-archetype.v1",
            &bucket.archetype_id,
            program_sha256,
        ))
        .map_err(str::to_owned)?;
        let program_bytes = serde_json::to_vec(program).map_or(usize::MAX, |value| value.len());
        ranked.push((
            support.len(),
            program_bytes,
            program_sha256.clone(),
            OnlineCollectionBucket {
                bucket_id,
                archetype_id,
                programs: BTreeMap::from([(program_sha256.clone(), program.clone())]),
                common_request_atom_ids,
                support,
                future: Vec::new(),
                runtime_examples: bucket
                    .runtime_examples
                    .iter()
                    .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
                    .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
                    .collect(),
                durable_adapter_phase_atoms: durable_adapter_phase_subset(
                    bucket,
                    &support_ids,
                    &BTreeSet::from([program_sha256.clone()]),
                ),
                durable_runtime_parity_receipts: BTreeMap::new(),
                frozen_program_sha256: None,
                support_watermark_event_time_unix_nanos: None,
                support_manifest_sha256: None,
                rejected_program_sha256: BTreeSet::new(),
                learned_anti_atom_ids: BTreeSet::new(),
                wrong_accepts: 0,
            },
        ));
    }
    ranked.sort_by(
        |(left_rows, left_bytes, left_digest, _), (right_rows, right_bytes, right_digest, _)| {
            right_rows
                .cmp(left_rows)
                .then_with(|| left_bytes.cmp(right_bytes))
                .then_with(|| left_digest.cmp(right_digest))
        },
    );
    Ok(ranked
        .into_iter()
        .map(|(_, _, _, subcenter)| subcenter)
        .collect())
}

fn support_law_subcenters(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let mut law_groups = BTreeMap::<Vec<u8>, BTreeMap<String, ResponseProgram>>::new();
    for (program_sha256, program) in &bucket.programs {
        if !is_privacy_safe_online_response_program(program)
            || !is_learned_bounded_response_program(program)
        {
            continue;
        }
        let law_key = response_law_key(program).map_err(str::to_owned)?;
        law_groups
            .entry(law_key)
            .or_default()
            .entry(program_sha256.clone())
            .or_insert_with(|| program.clone());
    }

    let mut ranked = Vec::new();
    for (law_key, adapters) in law_groups {
        let mut support = Vec::new();
        for receipt in &bucket.support {
            if !receipt.verifier_pass {
                continue;
            }
            // matched_program_sha256 is written only after the program has
            // reproduced the complete teacher response. Requiring the raw
            // example again made a valid proof disappear after restart.
            let matched_program_sha256 = adapters
                .iter()
                .filter(|(program_sha256, _)| {
                    receipt
                        .matched_program_sha256
                        .iter()
                        .any(|digest| digest == *program_sha256)
                })
                .map(|(program_sha256, _)| program_sha256.clone())
                .collect::<Vec<_>>();
            if matched_program_sha256.is_empty() {
                continue;
            }
            let mut canonical_receipt = receipt.clone();
            canonical_receipt.matched_program_sha256 = matched_program_sha256;
            support.push(canonical_receipt);
        }
        if support.len() < required_support_rows {
            continue;
        }
        support.truncate(max_receipts_per_bucket);
        let selected_adapter_digests = support
            .iter()
            .flat_map(|receipt| receipt.matched_program_sha256.iter().cloned())
            .collect::<BTreeSet<_>>();
        let programs = adapters
            .into_iter()
            .filter(|(digest, _)| selected_adapter_digests.contains(digest))
            .collect::<BTreeMap<_, _>>();
        if programs.is_empty() {
            continue;
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let parent_support_ids = bucket
            .support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        let selected_program_ids = programs.keys().cloned().collect::<BTreeSet<_>>();
        let parent_program_ids = bucket.programs.keys().cloned().collect::<BTreeSet<_>>();
        if support_ids == parent_support_ids && selected_program_ids == parent_program_ids {
            continue;
        }
        let law_commitment_sha256 = sha256_bytes(&law_key);
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-support-law-subcenter.v1",
            &bucket.archetype_id,
            &law_commitment_sha256,
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-support-law-subcenter-archetype.v1",
            &bucket.archetype_id,
            &law_commitment_sha256,
        ))
        .map_err(str::to_owned)?;
        let program_bytes = programs
            .values()
            .map(|program| serde_json::to_vec(program).map_or(usize::MAX, |value| value.len()))
            .sum::<usize>();
        let rank_digest = programs.keys().next().cloned().unwrap_or_default();
        ranked.push((
            support.len(),
            program_bytes,
            rank_digest,
            OnlineCollectionBucket {
                bucket_id,
                archetype_id,
                programs,
                common_request_atom_ids,
                support,
                future: Vec::new(),
                runtime_examples: bucket
                    .runtime_examples
                    .iter()
                    .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
                    .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
                    .collect(),
                durable_adapter_phase_atoms: durable_adapter_phase_subset(
                    bucket,
                    &support_ids,
                    &selected_program_ids,
                ),
                durable_runtime_parity_receipts: BTreeMap::new(),
                frozen_program_sha256: None,
                support_watermark_event_time_unix_nanos: None,
                support_manifest_sha256: None,
                rejected_program_sha256: BTreeSet::new(),
                learned_anti_atom_ids: BTreeSet::new(),
                wrong_accepts: 0,
            },
        ));
    }
    ranked.sort_by(
        |(left_rows, left_bytes, left_digest, _), (right_rows, right_bytes, right_digest, _)| {
            right_rows
                .cmp(left_rows)
                .then_with(|| left_bytes.cmp(right_bytes))
                .then_with(|| left_digest.cmp(right_digest))
        },
    );
    Ok(ranked
        .into_iter()
        .map(|(_, _, _, subcenter)| subcenter)
        .collect())
}

fn maximal_decidable_support_subcenter(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Option<OnlineCollectionBucket>, String> {
    if bucket.support.len() < required_support_rows || bucket.wrong_accepts > 0 {
        return Ok(None);
    }
    match support_consensus_candidate(bucket)? {
        SupportConsensusCandidate::Ready(_) => return Ok(None),
        SupportConsensusCandidate::Blocked(
            "support_phase_adapter_unproven"
            | "support_layout_adapter_unproven"
            | "support_consensus_variant_budget_exceeded",
        ) => {}
        SupportConsensusCandidate::Blocked(_) => return Ok(None),
    }

    let mut by_layout = BTreeMap::<String, Vec<OnlineCollectionReceipt>>::new();
    for receipt in &bucket.support {
        by_layout
            .entry(receipt.layout_sha256.clone())
            .or_default()
            .push(receipt.clone());
    }
    let mut layout_groups = by_layout.into_iter().collect::<Vec<_>>();
    layout_groups.sort_by(|(left_layout, left), (right_layout, right)| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left_layout.cmp(right_layout))
    });

    let mut selected_support = Vec::new();
    for (_, layout_support) in layout_groups {
        let common_adapter_exists = bucket.programs.keys().any(|digest| {
            layout_support.iter().all(|receipt| {
                receipt.verifier_pass && receipt.matched_program_sha256.contains(digest)
            })
        });
        if common_adapter_exists {
            selected_support.extend(layout_support);
        }
    }
    selected_support.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    if selected_support.len() > max_receipts_per_bucket {
        selected_support.drain(
            ..selected_support
                .len()
                .saturating_sub(max_receipts_per_bucket),
        );
    }
    if selected_support.len() < required_support_rows {
        return Ok(None);
    }

    let selected_layouts = selected_support
        .iter()
        .map(|receipt| receipt.layout_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut child = support_subset_bucket(bucket, selected_support);
    child.bucket_id = canonical_json_sha256(&(
        "nando.collection-maximal-decidable-subcenter.v1",
        &bucket.archetype_id,
        &selected_layouts,
    ))
    .map_err(str::to_owned)?;
    child.archetype_id = canonical_json_sha256(&(
        "nando.collection-maximal-decidable-subcenter-archetype.v1",
        &bucket.archetype_id,
        &selected_layouts,
    ))
    .map_err(str::to_owned)?;
    if !matches!(
        support_consensus_candidate(&child)?,
        SupportConsensusCandidate::Ready(_)
    ) {
        return Ok(None);
    }
    Ok(Some(child))
}

fn support_subset_bucket(
    bucket: &OnlineCollectionBucket,
    support: Vec<OnlineCollectionReceipt>,
) -> OnlineCollectionBucket {
    let support_ids = support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
        receipt.request_atom_ids.iter().copied().collect()
    });
    for receipt in support.iter().skip(1) {
        common_request_atom_ids
            .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
    }
    OnlineCollectionBucket {
        bucket_id: bucket.bucket_id.clone(),
        archetype_id: bucket.archetype_id.clone(),
        programs: bucket.programs.clone(),
        common_request_atom_ids,
        support,
        future: Vec::new(),
        runtime_examples: bucket
            .runtime_examples
            .iter()
            .filter(|(evidence_id, _)| support_ids.contains(*evidence_id))
            .map(|(evidence_id, example)| (evidence_id.clone(), example.clone()))
            .collect(),
        durable_adapter_phase_atoms: durable_adapter_phase_subset(
            bucket,
            &support_ids,
            &bucket.programs.keys().cloned().collect(),
        ),
        durable_runtime_parity_receipts: BTreeMap::new(),
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    }
}

fn clean_pre_action_program_subcenter(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> Result<Option<OnlineCollectionBucket>, String> {
    let rows = bucket
        .support
        .iter()
        .filter_map(|receipt| {
            bucket
                .runtime_examples
                .get(&receipt.evidence_graph_sha256)
                .map(|example| {
                    let mut atoms = request_atoms_for_example(example).unwrap_or_default();
                    atoms.extend(response_pre_action_context_atom_ids(
                        &example.provider_payload,
                    ));
                    (receipt, atoms)
                })
        })
        .collect::<Vec<_>>();
    if rows.len() < required_support_rows {
        return Ok(None);
    }

    let mut best = None::<(u64, usize, Vec<u64>, String, Vec<usize>)>;
    for digest in bucket.programs.keys() {
        let positive_indices = rows
            .iter()
            .enumerate()
            .filter(|(_, (receipt, _))| {
                receipt.verifier_pass && receipt.matched_program_sha256.contains(digest)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if positive_indices.len() < required_support_rows {
            continue;
        }
        let mut frequencies = BTreeMap::<u64, usize>::new();
        for index in &positive_indices {
            for atom in &rows[*index].1 {
                *frequencies.entry(*atom).or_default() += 1;
            }
        }
        let mut atoms = frequencies
            .into_iter()
            .filter(|(_, count)| *count >= required_support_rows)
            .collect::<Vec<_>>();
        atoms.sort_by(|(left_atom, left), (right_atom, right)| {
            right.cmp(left).then_with(|| left_atom.cmp(right_atom))
        });
        atoms.truncate(32);
        let atoms = atoms.into_iter().map(|(atom, _)| atom).collect::<Vec<_>>();

        let mut evaluate = |required_atoms: &[u64]| {
            let selected = rows
                .iter()
                .enumerate()
                .filter(|(_, (_, row_atoms))| {
                    required_atoms.iter().all(|atom| row_atoms.contains(atom))
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if selected.len() < required_support_rows
                || selected.iter().any(|index| {
                    let receipt = rows[*index].0;
                    !receipt.verifier_pass || !receipt.matched_program_sha256.contains(digest)
                })
            {
                return;
            }
            let tokens = selected.iter().fold(0_u64, |total, index| {
                total.saturating_add(rows[*index].0.estimated_input_tokens)
            });
            let candidate = (
                tokens,
                selected.len(),
                required_atoms.to_vec(),
                digest.clone(),
                selected,
            );
            let replace = best.as_ref().is_none_or(|current| {
                candidate.0 > current.0
                    || (candidate.0 == current.0 && candidate.1 > current.1)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2.len() < current.2.len())
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2.len() == current.2.len()
                        && (&candidate.2, &candidate.3) < (&current.2, &current.3))
            });
            if replace {
                best = Some(candidate);
            }
        };
        for left in 0..atoms.len() {
            evaluate(&atoms[left..=left]);
            for right in left + 1..atoms.len() {
                evaluate(&[atoms[left], atoms[right]]);
                for third in right + 1..atoms.len() {
                    evaluate(&[atoms[left], atoms[right], atoms[third]]);
                }
            }
        }
    }

    let Some((_, _, required_atoms, digest, mut selected)) = best else {
        return Ok(None);
    };
    selected.sort_by_key(|index| {
        (
            rows[*index].0.event_time_unix_nanos,
            rows[*index].0.evidence_graph_sha256.as_str(),
        )
    });
    if selected.len() > max_receipts_per_bucket {
        selected.drain(..selected.len().saturating_sub(max_receipts_per_bucket));
    }
    let mut support = selected
        .into_iter()
        .map(|index| {
            let mut receipt = rows[index].0.clone();
            receipt.matched_program_sha256 = vec![digest.clone()];
            receipt
        })
        .collect::<Vec<_>>();
    support.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    let mut child = support_subset_bucket(bucket, support);
    child.programs.retain(|candidate, _| candidate == &digest);
    child.common_request_atom_ids = required_atoms.iter().copied().collect();
    child.bucket_id = canonical_json_sha256(&(
        "nando.collection-clean-pre-action-subcenter.v1",
        &bucket.archetype_id,
        &digest,
        &required_atoms,
    ))
    .map_err(str::to_owned)?;
    child.archetype_id = canonical_json_sha256(&(
        "nando.collection-clean-pre-action-subcenter-archetype.v1",
        &bucket.archetype_id,
        &digest,
        &required_atoms,
    ))
    .map_err(str::to_owned)?;
    if !matches!(
        support_consensus_candidate(&child)?,
        SupportConsensusCandidate::Ready(_)
    ) {
        return Err("online_collection_pre_action_subcenter_not_ready".to_owned());
    }
    Ok(Some(child))
}

enum ActiveWitnessDecision {
    Successor {
        bucket: OnlineCollectionBucket,
        resolved: bool,
    },
    Pending,
    Irreducible,
}

fn active_witness_decision(
    bucket: &OnlineCollectionBucket,
    program_sha256: &str,
    observation: &OnlineCollectionObservation,
    max_receipts: usize,
) -> Result<ActiveWitnessDecision, String> {
    let Some(program) = bucket.programs.get(program_sha256) else {
        return Err("online_collection_witness_program_missing".to_owned());
    };
    let crate::ResponseOperation::UniqueConsensus { variants, .. } = &program.operation else {
        return Ok(ActiveWitnessDecision::Irreducible);
    };
    if variants.len() < 2 {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    let next_round = bucket
        .support
        .iter()
        .filter_map(|receipt| receipt.witness_round)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    if next_round > MAX_ACTIVE_WITNESS_ROUNDS {
        return Ok(ActiveWitnessDecision::Irreducible);
    }

    let mut candidates = BTreeMap::new();
    for variant in variants {
        let digest = canonical_json_sha256(&variant.program).map_err(str::to_owned)?;
        candidates
            .entry(digest)
            .or_insert_with(|| variant.program.clone());
    }
    if candidates.len() < 2 {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    let survivors = candidates
        .iter()
        .filter(|(_, candidate)| {
            independently_verified_authority_response(candidate, &observation.example)
                .is_some_and(|response| response == observation.example.expected_response)
        })
        .map(|(digest, candidate)| (digest.clone(), candidate.clone()))
        .collect::<BTreeMap<_, _>>();
    if survivors.is_empty() {
        return Ok(ActiveWitnessDecision::Irreducible);
    }
    if survivors.len() == candidates.len() {
        return Ok(ActiveWitnessDecision::Pending);
    }

    let candidate_digests = candidates.keys().cloned().collect::<Vec<_>>();
    let survivor_digests = survivors.keys().cloned().collect::<BTreeSet<_>>();
    let class_commitment_sha256 = canonical_json_sha256(&(
        "nando.collection-active-witness-class.v1",
        &bucket.bucket_id,
        program_sha256,
        &bucket.support_manifest_sha256,
        &candidate_digests,
    ))
    .map_err(str::to_owned)?;
    let mut support = bucket
        .support
        .iter()
        .filter_map(|receipt| {
            let mut receipt = receipt.clone();
            receipt
                .matched_program_sha256
                .retain(|digest| survivor_digests.contains(digest));
            (!receipt.matched_program_sha256.is_empty()).then_some(receipt)
        })
        .collect::<Vec<_>>();
    let mut witness = receipt_with_program_atoms(observation, true, &survivors)?;
    witness.witness_class_commitment_sha256 = Some(class_commitment_sha256.clone());
    witness.witness_round = Some(next_round);
    witness.witness_candidates_before = Some(candidates.len());
    witness.witness_candidates_after = Some(survivors.len());
    push_bounded(&mut support, witness, max_receipts);

    let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
        receipt.request_atom_ids.iter().copied().collect()
    });
    for receipt in support.iter().skip(1) {
        common_request_atom_ids.retain(|atom| receipt.request_atom_ids.binary_search(atom).is_ok());
    }
    let successor_id = canonical_json_sha256(&(
        "nando.collection-active-witness-successor.v1",
        &bucket.bucket_id,
        &class_commitment_sha256,
        &observation.evidence_graph_sha256,
        survivors.keys().collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let successor_archetype_id = canonical_json_sha256(&(
        "nando.collection-active-witness-successor-archetype.v1",
        &bucket.archetype_id,
        &class_commitment_sha256,
    ))
    .map_err(str::to_owned)?;
    let support_ids = support
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.clone())
        .collect::<BTreeSet<_>>();
    let survivor_ids = survivors.keys().cloned().collect::<BTreeSet<_>>();
    let mut successor = OnlineCollectionBucket {
        bucket_id: successor_id,
        archetype_id: successor_archetype_id,
        programs: survivors,
        common_request_atom_ids,
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::from([(
            observation.evidence_graph_sha256.clone(),
            observation.example.clone(),
        )]),
        durable_adapter_phase_atoms: durable_adapter_phase_subset(
            bucket,
            &support_ids,
            &survivor_ids,
        ),
        durable_runtime_parity_receipts: BTreeMap::new(),
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    refresh_durable_adapter_phase_atoms(&mut successor);
    Ok(ActiveWitnessDecision::Successor {
        resolved: successor.programs.len() == 1,
        bucket: successor,
    })
}

fn revoke_frozen_bucket(bucket: &mut OnlineCollectionBucket, program_sha256: &str) {
    let rejected = bucket_adapter_digests(bucket);
    bucket.frozen_program_sha256 = None;
    bucket.support_watermark_event_time_unix_nanos = None;
    bucket.support_manifest_sha256 = None;
    bucket.programs.clear();
    bucket.rejected_program_sha256.extend(rejected);
    bucket
        .rejected_program_sha256
        .insert(program_sha256.to_owned());
}

fn counterexample_subcenters(
    bucket: &OnlineCollectionBucket,
    program_sha256: &str,
    negative: &OnlineCollectionReceipt,
) -> Result<Vec<OnlineCollectionBucket>, String> {
    let Some(program) = bucket.programs.get(program_sha256) else {
        return Ok(Vec::new());
    };
    let negative_atoms = negative
        .request_atom_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut frequencies = BTreeMap::<u64, usize>::new();
    for receipt in &bucket.support {
        for atom in &receipt.request_atom_ids {
            if !negative_atoms.contains(atom) {
                *frequencies.entry(*atom).or_default() += 1;
            }
        }
    }
    let mut atoms = frequencies.into_iter().collect::<Vec<_>>();
    atoms.sort_by(|(left_atom, left_rows), (right_atom, right_rows)| {
        right_rows
            .cmp(left_rows)
            .then_with(|| left_atom.cmp(right_atom))
    });
    let mut seen_partitions = BTreeSet::new();
    let mut output = Vec::new();
    for (atom, rows) in atoms {
        if rows < 8 || output.len() >= 4 {
            continue;
        }
        let support = bucket
            .support
            .iter()
            .filter(|receipt| receipt.request_atom_ids.binary_search(&atom).is_ok())
            .cloned()
            .collect::<Vec<_>>();
        let partition = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.as_str())
            .collect::<Vec<_>>();
        let partition_sha256 = canonical_json_sha256(&partition).map_err(str::to_owned)?;
        if !seen_partitions.insert(partition_sha256.clone()) {
            continue;
        }
        let mut common_request_atom_ids = support.first().map_or_else(BTreeSet::new, |receipt| {
            receipt.request_atom_ids.iter().copied().collect()
        });
        for receipt in support.iter().skip(1) {
            common_request_atom_ids
                .retain(|candidate| receipt.request_atom_ids.binary_search(candidate).is_ok());
        }
        if !common_request_atom_ids.contains(&atom) {
            continue;
        }
        let bucket_id = canonical_json_sha256(&(
            "nando.collection-cegis-subcenter.v1",
            program_sha256,
            atom,
            partition_sha256.clone(),
        ))
        .map_err(str::to_owned)?;
        let archetype_id = canonical_json_sha256(&(
            "nando.collection-cegis-subcenter-archetype.v1",
            &bucket.archetype_id,
            program_sha256,
            atom,
            &partition_sha256,
        ))
        .map_err(str::to_owned)?;
        let support_ids = support
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.clone())
            .collect::<BTreeSet<_>>();
        output.push(OnlineCollectionBucket {
            bucket_id,
            archetype_id,
            programs: BTreeMap::from([(program_sha256.to_owned(), program.clone())]),
            common_request_atom_ids,
            support,
            future: Vec::new(),
            runtime_examples: bucket
                .runtime_examples
                .iter()
                .filter(|(id, _)| support_ids.contains(*id))
                .map(|(id, example)| (id.clone(), example.clone()))
                .collect(),
            durable_adapter_phase_atoms: durable_adapter_phase_subset(
                bucket,
                &support_ids,
                &BTreeSet::from([program_sha256.to_owned()]),
            ),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
    }
    Ok(output)
}

fn validate_checkpoint(
    checkpoint: &OnlineCollectionCheckpoint,
    config: OnlineCollectionConfig,
) -> Result<(), String> {
    if checkpoint.schema != ONLINE_COLLECTION_SCHEMA_V3
        || checkpoint.pooling_strategy_version != ONLINE_COLLECTION_POOLING_STRATEGY_V35
        || checkpoint.config != config
    {
        return Err("online_collection_checkpoint_contract_mismatch".to_owned());
    }
    if checkpoint
        .observed_evidence_graph_sha256
        .iter()
        .any(|digest| !is_sha256(digest))
        || checkpoint.observed_evidence_graph_sha256.len()
            > usize::try_from(checkpoint.observations_total).unwrap_or(usize::MAX)
    {
        return Err("online_collection_checkpoint_observation_index_invalid".to_owned());
    }
    for bucket in &checkpoint.buckets {
        if let Some(reason) = invalid_collection_bucket_reason(bucket) {
            return Err(format!(
                "online_collection_checkpoint_program_invalid:{}:{reason}",
                bucket.bucket_id
            ));
        }
    }
    Ok(())
}

fn migrate_collection_active_witness_pools(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut discarded_support = 0_u64;
    for bucket in &mut checkpoint.buckets {
        let known_digests = bucket_adapter_digests(bucket);
        for digest in bucket
            .support
            .iter()
            .chain(bucket.future.iter())
            .flat_map(|receipt| receipt.matched_program_sha256.iter())
            .filter(|digest| !known_digests.contains(*digest))
        {
            bucket.rejected_program_sha256.insert(digest.clone());
        }

        let invalid_programs = bucket
            .programs
            .iter()
            .filter_map(|(digest, program)| {
                let valid = canonical_json_sha256(program).ok().as_ref() == Some(digest)
                    && program.validate().is_ok()
                    && is_privacy_safe_online_response_program(program);
                (!valid).then_some(digest.clone())
            })
            .collect::<Vec<_>>();
        for digest in invalid_programs {
            bucket.programs.remove(&digest);
            bucket.rejected_program_sha256.insert(digest);
        }

        let support_before = bucket.support.len();
        bucket.support.retain(valid_witness_receipt_metadata);
        discarded_support = discarded_support
            .saturating_add(support_before.saturating_sub(bucket.support.len()) as u64);
        bucket.future.retain(valid_witness_receipt_metadata);

        if bucket.archetype_id.is_empty() {
            bucket.archetype_id = bucket
                .programs
                .values()
                .next()
                .map(response_program_archetype_id)
                .transpose()?
                .unwrap_or_else(|| format!("rejected: {}", bucket.bucket_id));
        }

        let frozen_valid = bucket.frozen_program_sha256.as_ref().is_some_and(|digest| {
            bucket.programs.contains_key(digest)
                && bucket.support_watermark_event_time_unix_nanos.is_some()
                && bucket.support.iter().all(|receipt| {
                    receipt.event_time_unix_nanos.is_some_and(|event_time| {
                        bucket
                            .support_watermark_event_time_unix_nanos
                            .is_some_and(|watermark| event_time <= watermark)
                    })
                })
                && collection_support_manifest_digest(bucket).ok().as_ref()
                    == bucket.support_manifest_sha256.as_ref()
        });
        if bucket.frozen_program_sha256.is_some() && !frozen_valid {
            bucket.future.clear();
            bucket.durable_runtime_parity_receipts.clear();
            bucket.frozen_program_sha256 = None;
            bucket.support_watermark_event_time_unix_nanos = None;
            bucket.support_manifest_sha256 = None;
        } else if bucket.frozen_program_sha256.is_none() {
            bucket.support_manifest_sha256 = None;
        }
    }
    checkpoint.unreplayable_support_discarded_total = checkpoint
        .unreplayable_support_discarded_total
        .saturating_add(discarded_support);
    Ok(())
}

fn migrate_collection_exact_receipts(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let mut discarded_support = 0_u64;
    for bucket in &mut checkpoint.buckets {
        let previous_programs = std::mem::take(&mut bucket.programs);
        let mut exact_programs = BTreeMap::new();
        for example in bucket
            .runtime_examples
            .values()
            .take(MAX_EXACT_RECEIPT_MIGRATION_SEEDS_PER_BUCKET)
        {
            let Ok(space) = enumerate_source_neutral_response_programs(example) else {
                continue;
            };
            for program in space.programs {
                if independently_verified_authority_response(&program, example).as_deref()
                    != Some(example.expected_response.as_str())
                    || !is_privacy_safe_online_response_program(&program)
                    || response_program_archetype_id(&program)? != bucket.archetype_id
                {
                    continue;
                }
                let digest = canonical_json_sha256(&program).map_err(str::to_owned)?;
                exact_programs.entry(digest).or_insert(program);
            }
        }
        if exact_programs.is_empty() {
            bucket.programs = previous_programs;
        } else {
            bucket.rejected_program_sha256.extend(
                previous_programs
                    .keys()
                    .filter(|digest| !exact_programs.contains_key(*digest))
                    .cloned(),
            );
            bucket.programs = bounded_program_map(
                exact_programs,
                crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
            );
        }

        let previous_support = std::mem::take(&mut bucket.support);
        for old_receipt in previous_support {
            let Some(example) = bucket
                .runtime_examples
                .get(&old_receipt.evidence_graph_sha256)
            else {
                discarded_support = discarded_support.saturating_add(1);
                continue;
            };
            let observation = OnlineCollectionObservation {
                evidence_graph_sha256: old_receipt.evidence_graph_sha256.clone(),
                client_intent_id_sha256: old_receipt.client_intent_id_sha256.clone(),
                session_id_sha256: old_receipt.session_id_sha256.clone(),
                event_time_unix_nanos: old_receipt.event_time_unix_nanos,
                estimated_input_tokens: old_receipt.estimated_input_tokens,
                example: example.clone(),
            };
            let mut rebuilt = receipt_with_program_atoms(&observation, true, &bucket.programs)?;
            if rebuilt.matched_program_sha256.is_empty() {
                discarded_support = discarded_support.saturating_add(1);
                continue;
            }
            rebuilt.witness_class_commitment_sha256 = old_receipt.witness_class_commitment_sha256;
            rebuilt.witness_round = old_receipt.witness_round;
            rebuilt.witness_candidates_before = old_receipt.witness_candidates_before;
            rebuilt.witness_candidates_after = old_receipt.witness_candidates_after;
            bucket.support.push(rebuilt);
        }
        bucket.common_request_atom_ids = bucket
            .support
            .first()
            .map_or_else(BTreeSet::new, |receipt| {
                receipt.request_atom_ids.iter().copied().collect()
            });
        for receipt in bucket.support.iter().skip(1) {
            bucket
                .common_request_atom_ids
                .retain(|atom| receipt.request_atom_ids.binary_search(atom).is_ok());
        }
        bucket.future.clear();
        bucket.durable_runtime_parity_receipts.clear();
        bucket.frozen_program_sha256 = None;
        bucket.support_watermark_event_time_unix_nanos = None;
        bucket.support_manifest_sha256 = None;
        bucket.learned_anti_atom_ids.clear();
        bucket.wrong_accepts = 0;
    }
    checkpoint.unreplayable_support_discarded_total = checkpoint
        .unreplayable_support_discarded_total
        .saturating_add(discarded_support);
    Ok(())
}

fn migrate_collection_relational_role_programs(
    checkpoint: &mut OnlineCollectionCheckpoint,
) -> Result<(), String> {
    let required_support_rows = checkpoint.config.support_rows;
    let mut authority_cache = BTreeMap::<(String, String), bool>::new();
    for bucket in &mut checkpoint.buckets {
        if bucket.frozen_program_sha256.is_some() || bucket.runtime_examples.is_empty() {
            continue;
        }
        let law_keys = bucket
            .programs
            .values()
            .filter_map(|program| response_law_key(program).ok())
            .collect::<BTreeSet<_>>();
        let mut relational = BTreeMap::<String, ResponseProgram>::new();
        let mut candidates = Vec::new();
        for program in bucket.programs.values() {
            collect_relational_role_programs(program, &mut candidates);
        }
        for canonical in candidates {
            if !is_privacy_safe_online_response_program(&canonical)
                || response_law_key(&canonical)
                    .ok()
                    .is_none_or(|law| !law_keys.contains(&law))
            {
                continue;
            }
            let digest = canonical_json_sha256(&canonical).map_err(str::to_owned)?;
            let support = bucket
                .support
                .iter()
                .filter(|receipt| {
                    let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256)
                    else {
                        return false;
                    };
                    *authority_cache
                        .entry((receipt.evidence_graph_sha256.clone(), digest.clone()))
                        .or_insert_with(|| {
                            response_program_authority_matches_example(&canonical, example)
                        })
                })
                .count();
            if support >= required_support_rows {
                relational.entry(digest).or_insert(canonical);
            }
        }
        if relational.is_empty() {
            continue;
        }
        for (digest, program) in &relational {
            bucket.programs.insert(digest.clone(), program.clone());
        }
        for receipt in &mut bucket.support {
            let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256) else {
                continue;
            };
            for (digest, program) in &relational {
                if *authority_cache
                    .entry((receipt.evidence_graph_sha256.clone(), digest.clone()))
                    .or_insert_with(|| response_program_authority_matches_example(program, example))
                {
                    receipt.matched_program_sha256.push(digest.clone());
                }
            }
            receipt.matched_program_sha256.sort();
            receipt.matched_program_sha256.dedup();
        }
        while bucket.programs.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
            let relational_digests = relational.keys().collect::<BTreeSet<_>>();
            let Some(evicted) = bucket
                .programs
                .keys()
                .filter(|digest| !relational_digests.contains(digest))
                .map(|digest| {
                    let support = bucket
                        .support
                        .iter()
                        .filter(|receipt| receipt.matched_program_sha256.contains(digest))
                        .count();
                    (support, digest.clone())
                })
                .min()
                .map(|(_, digest)| digest)
            else {
                break;
            };
            bucket.programs.remove(&evicted);
            bucket.rejected_program_sha256.insert(evicted);
        }
    }
    Ok(())
}

fn collect_relational_role_programs(program: &ResponseProgram, output: &mut Vec<ResponseProgram>) {
    match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            completion_state,
            ..
        } => {
            let value_type = online_selector_value_type(selector);
            output.push(ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::RequestReferencedJsonField { value_type },
                *format,
                completion_state.clone(),
            ));
            for reverse_ordinal in 0..4 {
                output.push(ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                        reverse_ordinal,
                        value_type,
                    },
                    *format,
                    completion_state.clone(),
                ));
            }
        }
        crate::ResponseOperation::ProjectStatus {
            mapping,
            completion_state,
            ..
        } => {
            output.push(ResponseProgram::project_status(
                crate::ResponseValueSelector::RequestReferencedJsonField {
                    value_type: crate::AtomValueType::Integer,
                },
                *mapping,
                completion_state.clone(),
            ));
            for reverse_ordinal in 0..4 {
                output.push(ResponseProgram::project_status(
                    crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                        reverse_ordinal,
                        value_type: crate::AtomValueType::Integer,
                    },
                    *mapping,
                    completion_state.clone(),
                ));
            }
        }
        crate::ResponseOperation::UniqueConsensus { variants, .. } => {
            for variant in variants {
                collect_relational_role_programs(&variant.program, output);
            }
        }
        _ => {}
    }
}

const fn online_selector_value_type(
    selector: &crate::ResponseValueSelector,
) -> crate::AtomValueType {
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { value_type }
        | crate::ResponseValueSelector::UniqueScalar { value_type }
        | crate::ResponseValueSelector::UniqueTurnScalar { value_type }
        | crate::ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | crate::ResponseValueSelector::JsonField { value_type, .. }
        | crate::ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::RequestReferencedJsonField { value_type }
        | crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::CommandOutputBody
        | crate::ResponseValueSelector::RequestLastToken
        | crate::ResponseValueSelector::RequestUniqueLiteral => crate::AtomValueType::String,
    }
}

fn invalid_collection_bucket_reason(bucket: &OnlineCollectionBucket) -> Option<String> {
    if bucket.programs.is_empty()
        && (bucket.rejected_program_sha256.is_empty() || bucket.frozen_program_sha256.is_some())
    {
        return Some("empty_program_pool_without_rejected_history".to_owned());
    }
    if bucket.archetype_id.is_empty() {
        return Some("empty_archetype_id".to_owned());
    }
    if let Some(frozen_digest) = &bucket.frozen_program_sha256 {
        if !bucket.programs.contains_key(frozen_digest) {
            return Some(format!("frozen_program_missing:{frozen_digest}"));
        }
        let Some(watermark) = bucket.support_watermark_event_time_unix_nanos else {
            return Some("frozen_support_watermark_missing".to_owned());
        };
        if bucket.support.iter().any(|receipt| {
            receipt
                .event_time_unix_nanos
                .is_none_or(|event_time| event_time > watermark)
        }) {
            return Some("frozen_support_after_watermark".to_owned());
        }
        if collection_support_manifest_digest(bucket).ok().as_ref()
            != bucket.support_manifest_sha256.as_ref()
        {
            return Some("frozen_support_manifest_mismatch".to_owned());
        }
    } else if bucket.support_manifest_sha256.is_some() {
        return Some("unfrozen_bucket_has_support_manifest".to_owned());
    }
    for (digest, program) in &bucket.programs {
        if canonical_json_sha256(program).ok().as_ref() != Some(digest) {
            return Some(format!("program_digest_mismatch:{digest}"));
        }
        if let Err(reason) = program.validate() {
            return Some(format!("program_contract_invalid:{digest}:{reason}"));
        }
        if !is_privacy_safe_online_response_program(program) {
            return Some(format!("program_privacy_invalid:{digest}"));
        }
    }
    let adapter_digests = bucket_adapter_digests(bucket);
    for (kind, receipts) in [("support", &bucket.support), ("future", &bucket.future)] {
        for receipt in receipts {
            if receipt.matched_program_sha256.is_empty() {
                return Some(format!("{kind}_receipt_programs_empty"));
            }
            if !valid_witness_receipt_metadata(receipt) {
                return Some(format!("{kind}_receipt_witness_metadata_invalid"));
            }
            if let Some(digest) = receipt
                .matched_program_sha256
                .iter()
                .find(|digest| !adapter_digests.contains(*digest))
            {
                return Some(format!("{kind}_receipt_program_unknown:{digest}"));
            }
        }
    }
    None
}

fn bucket_adapter_digests(bucket: &OnlineCollectionBucket) -> BTreeSet<String> {
    let mut digests = bucket.programs.keys().cloned().collect::<BTreeSet<_>>();
    digests.extend(bucket.rejected_program_sha256.iter().cloned());
    for program in bucket.programs.values() {
        if let crate::ResponseOperation::UniqueConsensus { variants, .. } = &program.operation {
            for variant in variants {
                if let Ok(digest) = canonical_json_sha256(&variant.program) {
                    digests.insert(digest);
                }
            }
        }
    }
    digests
}

fn validate_observation(observation: &OnlineCollectionObservation) -> Result<(), String> {
    if !is_sha256(&observation.evidence_graph_sha256)
        || !is_sha256(&observation.client_intent_id_sha256)
        || !is_sha256(&observation.session_id_sha256)
    {
        return Err("online_collection_observation_identity_invalid".to_owned());
    }
    Ok(())
}

fn receipt(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
) -> Result<OnlineCollectionReceipt, String> {
    Ok(OnlineCollectionReceipt {
        evidence_graph_sha256: observation.evidence_graph_sha256.clone(),
        client_intent_id_sha256: observation.client_intent_id_sha256.clone(),
        session_id_sha256: observation.session_id_sha256.clone(),
        event_time_unix_nanos: observation.event_time_unix_nanos,
        layout_sha256: structural_layout_sha256(&observation.example.provider_payload)?,
        estimated_input_tokens: observation.estimated_input_tokens,
        verifier_pass,
        request_atom_ids: observation_request_atom_ids(observation)
            .into_iter()
            .collect(),
        matched_program_sha256: Vec::new(),
        witness_class_commitment_sha256: None,
        witness_round: None,
        witness_candidates_before: None,
        witness_candidates_after: None,
    })
}

fn receipt_with_program_atoms(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<OnlineCollectionReceipt, String> {
    let mut value = receipt(observation, verifier_pass)?;
    value
        .request_atom_ids
        .extend(common_program_atom_ids(programs));
    value.request_atom_ids.sort_unstable();
    value.request_atom_ids.dedup();
    value.matched_program_sha256 = programs
        .iter()
        .filter(|(_, program)| independently_verified_teacher_match(program, &observation.example))
        .map(|(digest, _)| digest.clone())
        .collect();
    Ok(value)
}

fn common_program_atom_ids(programs: &BTreeMap<String, ResponseProgram>) -> BTreeSet<u64> {
    let mut programs = programs.values();
    let Some(first) = programs.next() else {
        return BTreeSet::new();
    };
    let mut common = response_program_required_routing_atom_ids(first)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for program in programs {
        let atoms = response_program_required_routing_atom_ids(program)
            .into_iter()
            .collect::<BTreeSet<_>>();
        common.retain(|atom| atoms.contains(atom));
    }
    common
}

fn bucket_program_atom_ids(bucket: &OnlineCollectionBucket) -> BTreeSet<u64> {
    common_program_atom_ids(&bucket.programs)
}

fn durable_pre_action_atom_ids(
    bucket: &OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
) -> BTreeSet<u64> {
    // Receipts are durable but may also contain routing atoms contributed by
    // the support-side program pool. Remove the union of every known program
    // atom so a teacher-derived program cannot become runtime evidence. A hash
    // collision can only remove a real pre-action atom and reduce recall.
    let program_atoms = bucket
        .programs
        .values()
        .flat_map(response_program_required_routing_atom_ids)
        .collect::<BTreeSet<_>>();
    receipt
        .request_atom_ids
        .iter()
        .copied()
        .filter(|atom| !program_atoms.contains(atom))
        .collect()
}

fn bucket_phase_center_atom_ids(bucket: &OnlineCollectionBucket) -> Vec<u64> {
    let program_atoms = bucket_program_atom_ids(bucket);
    let mut atoms = program_atoms.into_iter().collect::<Vec<_>>();
    atoms.extend(bucket.common_request_atom_ids.iter().copied());
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn distinct_receipt_sessions(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.session_id_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

fn distinct_receipt_layouts(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.layout_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

fn learned_wave_margin_micro(
    bucket: &OnlineCollectionBucket,
    phase_centers: &[u64],
    anti_centers: &[u64],
) -> i64 {
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    bucket
        .support
        .iter()
        .filter_map(|receipt| {
            let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
            phase_margin_to_micro(
                phase_coherence(&query, &positive) - phase_coherence(&query, &negative),
            )
            .ok()
        })
        .min()
        .map(|minimum| minimum.saturating_mul(9).saturating_div(10).max(1))
        .unwrap_or(1)
}

fn receipt_routes_phase(
    receipt: &OnlineCollectionReceipt,
    phase_centers: &[u64],
    anti_centers: &[u64],
    threshold: i64,
) -> bool {
    let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    phase_margin_to_micro(phase_coherence(&query, &positive) - phase_coherence(&query, &negative))
        .is_ok_and(|margin| margin >= threshold)
}

fn update_applicability_negative_sessions(
    evidence: &mut BTreeMap<u64, BTreeSet<String>>,
    candidates: BTreeSet<u64>,
    session_id_sha256: &str,
) -> BTreeSet<u64> {
    for atom in candidates
        .into_iter()
        .take(MAX_APPLICABILITY_NEGATIVE_ATOMS_PER_BUCKET)
    {
        evidence
            .entry(atom)
            .or_default()
            .insert(session_id_sha256.to_owned());
    }
    while evidence.len() > MAX_APPLICABILITY_NEGATIVE_ATOMS_PER_BUCKET {
        let Some(atom) = evidence.keys().next_back().copied() else {
            break;
        };
        evidence.remove(&atom);
    }
    evidence
        .iter()
        .filter_map(|(atom, sessions)| {
            (sessions.len() >= MIN_APPLICABILITY_NEGATIVE_SESSIONS).then_some(*atom)
        })
        .collect()
}

fn structural_layout_sha256(value: &Value) -> Result<String, String> {
    canonical_json_sha256(&structural_layout(value)).map_err(str::to_owned)
}

fn independently_verified_authority_response(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Option<String> {
    independently_verified_authority_response_result(program, example).ok()
}

fn independently_verified_teacher_match(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    // Discovery may retain structurally aligned laws, but a durable proof link
    // is exact only when the independently verified output equals the teacher.
    independently_verified_authority_response(program, example).as_deref()
        == Some(example.expected_response.as_str())
}

fn independently_verified_authority_response_result(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Result<String, &'static str> {
    if !response_program_authority_matches_example(program, example) {
        return Err("authority_mismatch");
    }
    let execution = execute_response(program, "", &example.provider_payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return Err("actor_abstain");
    }
    let response = execution.response.ok_or("actor_response_missing")?;
    let verifier =
        source_neutral_verifier_for_program(program).map_err(|_| "verifier_build_failed")?;
    verify_response_independently(&verifier, &example.provider_payload, &response)
        .map_err(|_| "verifier_rejected")?;
    Ok(response)
}

fn authority_rejection_reason(result: &Result<String, &'static str>) -> Option<&'static str> {
    match result {
        Err(reason) => Some(*reason),
        Ok(_) => None,
    }
}

fn is_hard_teacher_counterexample(reason: &str) -> bool {
    matches!(
        reason,
        "verifier_build_failed"
            | "verifier_rejected"
            | "actor_response_missing"
            | "teacher_response_mismatch"
    )
}

enum SupportConsensusCandidate {
    Ready(ResponseProgram),
    Blocked(&'static str),
}

fn best_adapter<'a>(
    adapters: impl Iterator<Item = (&'a String, &'a ResponseProgram)>,
) -> Option<(&'a String, &'a ResponseProgram)> {
    adapters.min_by(|(left_digest, left), (right_digest, right)| {
        u8::from(!is_source_neutral_response_program(left))
            .cmp(&u8::from(!is_source_neutral_response_program(right)))
            .then_with(|| {
                serde_json::to_vec(left)
                    .map_or(usize::MAX, |bytes| bytes.len())
                    .cmp(&serde_json::to_vec(right).map_or(usize::MAX, |bytes| bytes.len()))
            })
            .then_with(|| left_digest.cmp(right_digest))
    })
}

fn request_atoms_for_example(example: &CollectionSynthesisExample) -> Option<BTreeSet<u64>> {
    let text = example
        .provider_payload
        .get("input")?
        .as_array()?
        .iter()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content"))
        .filter_map(request_content_text)
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then(|| request_phase_atom_ids(&text).into_iter().collect())
}

fn phase_ranked_semantic_adapters(bucket: &OnlineCollectionBucket) -> Option<ResponseProgram> {
    const CELLS: usize = 16;
    const MAX_VARIANTS: usize = crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS;
    let rows = bucket.support.iter().collect::<Vec<_>>();
    if rows.len() < 4 {
        return None;
    }
    let programs = concrete_adapter_program_classes(bucket);
    if !(2..=MAX_VARIANTS).contains(&programs.len()) {
        return None;
    }
    let globally_proven = programs.iter().filter(|(_, (_, source_digests))| {
        rows.iter().all(|receipt| {
            receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
        })
    });
    if let Some((_, program)) =
        best_adapter(globally_proven.map(|(digest, (program, _))| (digest, program)))
    {
        return Some(program.clone());
    }
    let mut variants = Vec::new();
    let mut routes = Vec::new();
    for (_, (program, source_digests)) in programs {
        let row_atoms = rows
            .iter()
            .map(|receipt| durable_adapter_atoms(bucket, receipt, &source_digests))
            .collect::<Option<Vec<_>>>();
        let Some(row_atoms) = row_atoms else {
            continue;
        };
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        for (receipt, atoms) in rows.iter().zip(row_atoms) {
            if receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
            {
                positives.push(atoms.to_vec());
            } else {
                negatives.push(atoms.to_vec());
            }
        }
        if positives.is_empty() || negatives.is_empty() {
            continue;
        }
        let Some(route) = fit_adapter_wave_route(&positives, &negatives, CELLS) else {
            continue;
        };
        variants.push(ResponseConsensusVariant {
            program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        });
        routes.push(route);
    }
    if variants.len() < 2 {
        return None;
    }
    let candidate = ResponseProgram::unique_consensus(variants).with_adapter_wave(
        ResponseAdapterWaveConsensus {
            exact_budget: u16::try_from(routes.len().min(16)).ok()?,
            routes,
        },
    );
    candidate.validate().ok()?;
    candidate_authority_verified_on_support(bucket, &candidate).then_some(candidate)
}

fn concrete_adapter_program_classes(
    bucket: &OnlineCollectionBucket,
) -> BTreeMap<String, (ResponseProgram, BTreeSet<String>)> {
    let mut classes = BTreeMap::<String, (ResponseProgram, BTreeSet<String>)>::new();
    for (source_digest, program) in &bucket.programs {
        let Ok(class_digest) = canonical_json_sha256(program) else {
            continue;
        };
        classes
            .entry(class_digest)
            .or_insert_with(|| (program.clone(), BTreeSet::new()))
            .1
            .insert(source_digest.clone());
    }
    classes
}

fn durable_adapter_atoms<'a>(
    bucket: &'a OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
    source_digests: &BTreeSet<String>,
) -> Option<&'a [u64]> {
    let atoms_by_program = bucket
        .durable_adapter_phase_atoms
        .get(&receipt.evidence_graph_sha256)?;
    source_digests
        .iter()
        .find_map(|digest| atoms_by_program.get(digest))
        .map(Vec::as_slice)
}

pub(crate) fn fit_adapter_wave_route(
    positive_atoms: &[Vec<u64>],
    negative_atoms: &[Vec<u64>],
    cells: usize,
) -> Option<ResponseAdapterWaveRoute> {
    if cells == 0 || positive_atoms.is_empty() || negative_atoms.is_empty() {
        return None;
    }
    let anchor_atom_ids = fit_adapter_wave_anchor_atoms(positive_atoms, negative_atoms);
    let positive_fingerprint_ids = positive_atoms
        .iter()
        .map(|atoms| adapter_wave_atom_fingerprint(atoms))
        .collect::<BTreeSet<_>>();
    let negative_fingerprint_ids = negative_atoms
        .iter()
        .map(|atoms| adapter_wave_atom_fingerprint(atoms))
        .collect::<BTreeSet<_>>();
    if positive_fingerprint_ids
        .iter()
        .any(|fingerprint| negative_fingerprint_ids.contains(fingerprint))
        || positive_fingerprint_ids.len() > crate::program::MAX_ADAPTER_WAVE_FINGERPRINTS
    {
        return None;
    }
    let positives = positive_atoms
        .iter()
        .map(|atoms| phase_vector_from_atom_ids(atoms.iter().copied(), cells))
        .collect::<Vec<_>>();
    let negatives = negative_atoms
        .iter()
        .map(|atoms| phase_vector_from_atom_ids(atoms.iter().copied(), cells))
        .collect::<Vec<_>>();
    let mut negative_center = vec![PhaseCenterCell::default(); cells];
    for vector in &negatives {
        for (center, cell) in negative_center.iter_mut().zip(vector) {
            center.re += cell.re / negatives.len() as f64;
            center.im += cell.im / negatives.len() as f64;
        }
    }
    let score = |vector: &[PhaseCenterCell], center: &[i32]| {
        phase_margin_to_micro(
            vector
                .iter()
                .zip(center.chunks_exact(2))
                .map(|(query, center)| {
                    query.re * f64::from(center[0]) / 1_000_000.0
                        + query.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };
    let mut candidates = Vec::<(BTreeSet<usize>, Vec<i32>, i64, i64)>::new();
    for representative in &positives {
        let center = representative
            .iter()
            .zip(&negative_center)
            .flat_map(|(positive, negative)| {
                [
                    ((positive.re - negative.re) * 1_000_000.0).round() as i32,
                    ((positive.im - negative.im) * 1_000_000.0).round() as i32,
                ]
            })
            .collect::<Vec<_>>();
        let maximum_negative = negatives
            .iter()
            .map(|vector| score(vector, &center))
            .max()?;
        let threshold = maximum_negative.checked_add(1)?.max(1);
        let coverage = positives
            .iter()
            .enumerate()
            .filter_map(|(index, vector)| (score(vector, &center) >= threshold).then_some(index))
            .collect::<BTreeSet<_>>();
        if coverage.is_empty() {
            continue;
        }
        let gap = coverage
            .iter()
            .map(|index| score(&positives[*index], &center))
            .min()
            .unwrap_or(i64::MIN)
            .saturating_sub(maximum_negative);
        candidates.push((coverage, center, threshold, gap));
    }
    let mut uncovered = (0..positives.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<usize>::new();
    while !uncovered.is_empty() && selected.len() < crate::program::MAX_ADAPTER_WAVE_SUBCENTERS {
        let next = candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .map(|(index, candidate)| {
                (
                    candidate.0.intersection(&uncovered).count(),
                    candidate.3,
                    index,
                )
            })
            .filter(|(gain, _, _)| *gain > 0)
            .max_by(|left, right| left.cmp(right))?;
        selected.push(next.2);
        for covered in &candidates[next.2].0 {
            uncovered.remove(covered);
        }
    }
    if !uncovered.is_empty() {
        return None;
    }
    let primary = candidates.get(*selected.first()?)?;
    Some(ResponseAdapterWaveRoute {
        cells: u16::try_from(cells).ok()?,
        center_delta_micro: primary.1.clone(),
        threshold_micro: primary.2,
        anchor_atom_ids,
        positive_fingerprint_ids: positive_fingerprint_ids.into_iter().collect(),
        subcenters: selected
            .iter()
            .skip(1)
            .filter_map(|index| candidates.get(*index))
            .map(|candidate| ResponseAdapterWaveSubcenter {
                center_delta_micro: candidate.1.clone(),
                threshold_micro: candidate.2,
            })
            .collect(),
    })
}

fn fit_adapter_wave_anchor_atoms(positives: &[Vec<u64>], negatives: &[Vec<u64>]) -> Vec<u64> {
    let negative_atoms = negatives
        .iter()
        .flat_map(|atoms| atoms.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut coverage = BTreeMap::<u64, BTreeSet<usize>>::new();
    for (index, atoms) in positives.iter().enumerate() {
        for atom in atoms {
            if !negative_atoms.contains(atom) {
                coverage.entry(*atom).or_default().insert(index);
            }
        }
    }
    let mut uncovered = (0..positives.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<u64>::new();
    while !uncovered.is_empty() && selected.len() < crate::program::MAX_ADAPTER_WAVE_ANCHOR_ATOMS {
        let Some((atom, covered)) = coverage
            .iter()
            .filter(|(atom, _)| !selected.contains(*atom))
            .map(|(atom, indices)| {
                (
                    *atom,
                    indices
                        .intersection(&uncovered)
                        .copied()
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, covered)| !covered.is_empty())
            .max_by(|left, right| {
                left.1
                    .len()
                    .cmp(&right.1.len())
                    .then_with(|| right.0.cmp(&left.0))
            })
        else {
            break;
        };
        selected.push(atom);
        for index in covered {
            uncovered.remove(&index);
        }
    }
    selected.sort_unstable();
    selected
}

pub(crate) fn adapter_wave_atom_fingerprint(atoms: &[u64]) -> u64 {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
    for byte in atoms.iter().flat_map(|atom| atom.to_le_bytes()) {
        fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    fingerprint
}

fn phase_guarded_layout_adapters(
    bucket: &OnlineCollectionBucket,
    rows: &[&OnlineCollectionReceipt],
) -> Option<Vec<(String, ResponseProgram, Vec<u64>)>> {
    type GuardedAdapter = (String, ResponseProgram, Vec<u64>, BTreeSet<usize>);
    let row_atoms = rows
        .iter()
        .map(|receipt| durable_pre_action_atom_ids(bucket, receipt))
        .collect::<Vec<_>>();
    if row_atoms.iter().any(BTreeSet::is_empty) {
        return None;
    }
    let mut safe = Vec::<GuardedAdapter>::new();
    for (digest, program) in &bucket.programs {
        let positives = rows
            .iter()
            .enumerate()
            .filter(|(_, receipt)| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == digest)
            })
            .map(|(index, _)| index)
            .collect::<BTreeSet<_>>();
        if positives.is_empty() {
            continue;
        }
        let mut common = positives
            .iter()
            .next()
            .map(|index| row_atoms[*index].clone())?;
        for index in positives.iter().skip(1) {
            common.retain(|atom| row_atoms[*index].contains(atom));
        }
        let mut remaining_negatives = (0..rows.len())
            .filter(|index| !positives.contains(index))
            .collect::<BTreeSet<_>>();
        let mut guard = Vec::<u64>::new();
        while !remaining_negatives.is_empty() && guard.len() < 8 {
            let next = common
                .iter()
                .filter(|atom| !guard.contains(atom))
                .map(|atom| {
                    let excluded = remaining_negatives
                        .iter()
                        .filter(|index| !row_atoms[**index].contains(atom))
                        .count();
                    (*atom, excluded)
                })
                .max_by(|(left_atom, left), (right_atom, right)| {
                    left.cmp(right).then_with(|| right_atom.cmp(left_atom))
                });
            let Some((atom, excluded)) = next else {
                break;
            };
            if excluded == 0 {
                break;
            }
            guard.push(atom);
            remaining_negatives.retain(|index| row_atoms[*index].contains(&atom));
        }
        if remaining_negatives.is_empty() {
            guard.sort_unstable();
            safe.push((digest.clone(), program.clone(), guard, positives));
        }
    }
    let mut uncovered = (0..rows.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    while !uncovered.is_empty() {
        let candidate = safe
            .iter()
            .filter(|(digest, _, guard, _)| {
                !selected.iter().any(|(selected_digest, _, selected_guard)| {
                    selected_digest == digest && selected_guard == guard
                })
            })
            .map(|(digest, program, guard, covered)| {
                let gain = covered.intersection(&uncovered).count();
                (gain, digest, program, guard, covered)
            })
            .filter(|(gain, _, _, _, _)| *gain > 0)
            .max_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| right.3.len().cmp(&left.3.len()))
                    .then_with(|| {
                        is_source_neutral_response_program(left.2)
                            .cmp(&is_source_neutral_response_program(right.2))
                    })
                    .then_with(|| right.1.cmp(left.1))
            });
        let Some((_, digest, program, guard, covered)) = candidate else {
            return None;
        };
        selected.push((digest.clone(), program.clone(), guard.clone()));
        for index in covered {
            uncovered.remove(index);
        }
    }
    Some(selected)
}

fn response_selector_family(program: &ResponseProgram) -> &'static str {
    let selector = match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue { selector, .. }
        | crate::ResponseOperation::ProjectStatus { selector, .. } => selector,
        crate::ResponseOperation::ComposeCollection { .. } => return "collection",
        _ => return "other",
    };
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { .. } => "continuation_handle",
        crate::ResponseValueSelector::UniqueScalar { .. } => "unique_scalar",
        crate::ResponseValueSelector::UniqueTurnScalar { .. } => "unique_turn_scalar",
        crate::ResponseValueSelector::ContentLinePrefix { .. } => "content_line_prefix",
        crate::ResponseValueSelector::JsonField { .. } => "json_field",
        crate::ResponseValueSelector::JsonScalarOrdinal { .. } => "json_scalar_ordinal",
        crate::ResponseValueSelector::UniqueTurnJsonField { .. } => "unique_turn_json_field",
        crate::ResponseValueSelector::UniqueActiveTurnJsonField { .. } => {
            "unique_active_turn_json_field"
        }
        crate::ResponseValueSelector::RequestReferencedJsonField { .. } => {
            "request_referenced_json_field"
        }
        crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. } => {
            "request_referenced_json_field_ordinal"
        }
        crate::ResponseValueSelector::TurnOutputLine { .. } => "turn_output_line",
        crate::ResponseValueSelector::TurnOutputScalarOrdinal { .. } => {
            "turn_output_scalar_ordinal"
        }
        crate::ResponseValueSelector::LatestTurnOutputLine { .. } => "latest_turn_output_line",
        crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. } => {
            "latest_turn_output_scalar_ordinal"
        }
        crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. } => {
            "latest_turn_output_scalar_from_end"
        }
        crate::ResponseValueSelector::CommandOutputBody => "command_output_body",
        crate::ResponseValueSelector::RequestLastToken => "request_last_token",
        crate::ResponseValueSelector::RequestUniqueLiteral => "request_unique_literal",
    }
}

#[derive(Default)]
struct AdapterWaveDiagnostic {
    programs_considered: usize,
    programs_with_positive_and_negative: usize,
    routes_fitted: usize,
    candidate_valid: bool,
    authority_pass: bool,
    authority_rejection_counts: BTreeMap<String, usize>,
    first_rejected_evidence_sha256: String,
    blocker: String,
}

fn adapter_wave_diagnostic(bucket: &OnlineCollectionBucket) -> AdapterWaveDiagnostic {
    const CELLS: usize = 16;
    let mut diagnostic = AdapterWaveDiagnostic::default();
    let rows = bucket.support.iter().collect::<Vec<_>>();
    let programs = concrete_adapter_program_classes(bucket);
    diagnostic.programs_considered = programs.len();
    if !(2..=crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&programs.len()) {
        diagnostic.blocker = "adapter_wave_variant_count".to_owned();
        return diagnostic;
    }
    let mut variants = Vec::new();
    let mut routes = Vec::new();
    for (_, (program, source_digests)) in programs {
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        for receipt in &rows {
            let Some(atoms) = durable_adapter_atoms(bucket, receipt, &source_digests) else {
                diagnostic.blocker = "adapter_wave_missing_durable_phase_atoms".to_owned();
                return diagnostic;
            };
            if receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
            {
                positives.push(atoms.to_vec());
            } else {
                negatives.push(atoms.to_vec());
            }
        }
        if positives.is_empty() || negatives.is_empty() {
            continue;
        }
        diagnostic.programs_with_positive_and_negative = diagnostic
            .programs_with_positive_and_negative
            .saturating_add(1);
        let Some(route) = fit_adapter_wave_route(&positives, &negatives, CELLS) else {
            continue;
        };
        diagnostic.routes_fitted = diagnostic.routes_fitted.saturating_add(1);
        variants.push(ResponseConsensusVariant {
            program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        });
        routes.push(route);
    }
    if variants.len() < 2 {
        diagnostic.blocker = "adapter_wave_routes_below_two".to_owned();
        return diagnostic;
    }
    let candidate = ResponseProgram::unique_consensus(variants).with_adapter_wave(
        ResponseAdapterWaveConsensus {
            exact_budget: u16::try_from(routes.len().min(16)).unwrap_or(16),
            routes,
        },
    );
    diagnostic.candidate_valid = candidate.validate().is_ok();
    if !diagnostic.candidate_valid {
        diagnostic.blocker = "adapter_wave_candidate_invalid".to_owned();
        return diagnostic;
    }
    for receipt in &bucket.support {
        if durable_adapter_wave_proves_candidate(bucket, receipt, &candidate) {
            continue;
        }
        let reason = "adapter_wave_durable_authority_unproven".to_owned();
        if diagnostic.first_rejected_evidence_sha256.is_empty() {
            diagnostic.first_rejected_evidence_sha256 = receipt.evidence_graph_sha256.clone();
        }
        *diagnostic
            .authority_rejection_counts
            .entry(reason)
            .or_default() += 1;
    }
    diagnostic.authority_pass = diagnostic.authority_rejection_counts.is_empty();
    diagnostic.blocker = if diagnostic.authority_pass {
        String::new()
    } else {
        "adapter_wave_support_authority_failed".to_owned()
    };
    diagnostic
}

fn consensus_diagnostic(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> OnlineCollectionConsensusDiagnostic {
    let adapter_wave = adapter_wave_diagnostic(bucket);
    let law_subcenters =
        support_law_subcenters(bucket, required_support_rows, max_receipts_per_bucket)
            .unwrap_or_default();
    let best_law_subcenter = law_subcenters.first();
    let best_law_subcenter_consensus =
        best_law_subcenter.map_or_else(String::new, |subcenter| match support_consensus_candidate(
            subcenter,
        ) {
            Ok(SupportConsensusCandidate::Ready(_)) => "READY".to_owned(),
            Ok(SupportConsensusCandidate::Blocked(reason)) => reason.to_owned(),
            Err(error) => format!("ERROR:{error}"),
        });
    let best_law_subcenter_freeze_blocker = best_law_subcenter
        .and_then(|subcenter| support_freeze_blocker(subcenter, required_support_rows))
        .unwrap_or_default();
    let mut canonical = BTreeMap::<String, ResponseProgram>::new();
    for program in bucket.programs.values() {
        let Ok(direct) = canonical_direct_response_program(program) else {
            continue;
        };
        if !is_source_neutral_response_program(&direct) {
            continue;
        }
        if let Ok(digest) = canonical_json_sha256(&direct) {
            canonical.entry(digest).or_insert(direct);
        }
    }
    let mut selector_families = BTreeMap::<String, usize>::new();
    for program in canonical.values() {
        *selector_families
            .entry(response_selector_family(program).to_owned())
            .or_default() += 1;
    }
    let rows = bucket
        .support
        .iter()
        .filter_map(|receipt| bucket.runtime_examples.get(&receipt.evidence_graph_sha256))
        .collect::<Vec<_>>();
    let targets = rows
        .iter()
        .map(|example| {
            canonical
                .values()
                .filter_map(|program| independently_verified_authority_response(program, example))
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let unique_target_rows = targets.iter().filter(|values| values.len() == 1).count();
    let missing_target_rows = targets.iter().filter(|values| values.is_empty()).count();
    let ambiguous_target_rows = targets.iter().filter(|values| values.len() > 1).count();
    let mut safe_programs = 0_usize;
    let mut unsafe_disagreement_programs = 0_usize;
    let mut safely_covered = BTreeSet::<usize>::new();
    let mut max_safe_program_coverage = 0_usize;
    for program in canonical.values() {
        let mut coverage = BTreeSet::new();
        let mut disagrees = false;
        for (index, (example, targets)) in rows.iter().zip(&targets).enumerate() {
            let Some(target) = (targets.len() == 1)
                .then(|| targets.iter().next())
                .flatten()
            else {
                continue;
            };
            let execution = execute_response(program, "", &example.provider_payload);
            if execution.status != ResponseExecutionStatus::Executed {
                continue;
            }
            if execution.response.as_deref() != Some(target.as_str()) {
                disagrees = true;
                break;
            }
            if independently_verified_authority_response(program, example).as_deref()
                == Some(target.as_str())
            {
                coverage.insert(index);
            }
        }
        if disagrees {
            unsafe_disagreement_programs = unsafe_disagreement_programs.saturating_add(1);
        } else if !coverage.is_empty() {
            safe_programs = safe_programs.saturating_add(1);
            max_safe_program_coverage = max_safe_program_coverage.max(coverage.len());
            safely_covered.extend(coverage);
        }
    }
    OnlineCollectionConsensusDiagnostic {
        bucket_id: bucket.bucket_id.clone(),
        support_rows: bucket.support.len(),
        replayable_rows: rows.len(),
        canonical_programs: canonical.len(),
        unique_target_rows,
        missing_target_rows,
        ambiguous_target_rows,
        safe_programs,
        unsafe_disagreement_programs,
        safely_coverable_rows: safely_covered.len(),
        max_safe_program_coverage,
        selector_families,
        candidate_present: unguarded_unique_consensus_candidate(bucket).is_some(),
        adapter_wave_programs_considered: adapter_wave.programs_considered,
        adapter_wave_programs_with_positive_and_negative: adapter_wave
            .programs_with_positive_and_negative,
        adapter_wave_routes_fitted: adapter_wave.routes_fitted,
        adapter_wave_candidate_valid: adapter_wave.candidate_valid,
        adapter_wave_authority_pass: adapter_wave.authority_pass,
        adapter_wave_authority_rejection_counts: adapter_wave.authority_rejection_counts,
        adapter_wave_first_rejected_evidence_sha256: adapter_wave.first_rejected_evidence_sha256,
        adapter_wave_blocker: adapter_wave.blocker,
        law_subcenters_total: law_subcenters.len(),
        best_law_subcenter_support_rows: best_law_subcenter.map_or(0, |value| value.support.len()),
        best_law_subcenter_programs: best_law_subcenter.map_or(0, |value| value.programs.len()),
        best_law_subcenter_consensus,
        best_law_subcenter_freeze_blocker,
    }
}

fn unguarded_unique_consensus_candidate(
    bucket: &OnlineCollectionBucket,
) -> Option<ResponseProgram> {
    let mut canonical = BTreeMap::<String, ResponseProgram>::new();
    for program in bucket.programs.values() {
        let Ok(direct) = canonical_direct_response_program(program) else {
            continue;
        };
        if !is_source_neutral_response_program(&direct) {
            continue;
        }
        let digest = canonical_json_sha256(&direct).ok()?;
        canonical.entry(digest).or_insert(direct);
    }
    if !(2..=crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&canonical.len()) {
        return None;
    }
    let rows = bucket
        .support
        .iter()
        .map(|receipt| bucket.runtime_examples.get(&receipt.evidence_graph_sha256))
        .collect::<Option<Vec<_>>>()?;
    let targets = rows
        .iter()
        .map(|example| {
            let responses = canonical
                .values()
                .filter_map(|program| independently_verified_authority_response(program, example))
                .collect::<BTreeSet<_>>();
            (responses.len() == 1).then(|| responses.into_iter().next())?
        })
        .collect::<Option<Vec<_>>>()?;
    let mut safe = Vec::<(String, ResponseProgram, BTreeSet<usize>)>::new();
    for (digest, program) in canonical {
        let mut covered = BTreeSet::new();
        let mut disagrees = false;
        for (index, (example, target)) in rows.iter().zip(&targets).enumerate() {
            let execution = execute_response(&program, "", &example.provider_payload);
            if execution.status != ResponseExecutionStatus::Executed {
                continue;
            }
            if execution.response.as_deref() != Some(target.as_str()) {
                disagrees = true;
                break;
            }
            if independently_verified_authority_response(&program, example).as_deref()
                == Some(target.as_str())
            {
                covered.insert(index);
            }
        }
        if !disagrees && !covered.is_empty() {
            safe.push((digest, program, covered));
        }
    }
    let mut uncovered = (0..rows.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<ResponseProgram>::new();
    while !uncovered.is_empty() {
        let candidate = safe
            .iter()
            .filter(|(_, program, _)| !selected.contains(program))
            .map(|(digest, program, covered)| {
                (
                    covered.intersection(&uncovered).count(),
                    digest,
                    program,
                    covered,
                )
            })
            .filter(|(gain, _, _, _)| *gain > 0)
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(left.1)))?;
        selected.push(candidate.2.clone());
        for index in candidate.3 {
            uncovered.remove(index);
        }
    }
    let candidate = if selected.len() == 1 {
        selected.pop()?
    } else {
        ResponseProgram::unique_consensus(
            selected
                .into_iter()
                .map(|program| ResponseConsensusVariant {
                    program,
                    allowed_layout_sha256: Vec::new(),
                    required_request_atom_ids: Vec::new(),
                })
                .collect(),
        )
    };
    candidate.validate().ok()?;
    candidate_authority_verified_on_support(bucket, &candidate).then_some(candidate)
}

fn support_consensus_candidate(
    bucket: &OnlineCollectionBucket,
) -> Result<SupportConsensusCandidate, String> {
    let globally_proven = bucket
        .programs
        .iter()
        .filter(|(digest, _)| {
            bucket.support.iter().all(|receipt| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == *digest)
            })
        })
        .map(|(digest, program)| (digest.clone(), program.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut variants = if let Some((_, program)) = best_adapter(globally_proven.iter()) {
        vec![ResponseConsensusVariant {
            program: program.clone(),
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        }]
    } else if let Some(candidate) = phase_ranked_semantic_adapters(bucket) {
        return Ok(SupportConsensusCandidate::Ready(candidate));
    } else if let Some(candidate) = unguarded_unique_consensus_candidate(bucket) {
        return Ok(SupportConsensusCandidate::Ready(candidate));
    } else {
        let mut by_adapter = BTreeMap::<(String, Vec<u64>), (ResponseProgram, Vec<String>)>::new();
        let layouts = bucket
            .support
            .iter()
            .map(|receipt| receipt.layout_sha256.clone())
            .collect::<BTreeSet<_>>();
        for layout in layouts {
            let rows = bucket
                .support
                .iter()
                .filter(|receipt| receipt.layout_sha256 == layout)
                .collect::<Vec<_>>();
            let common = bucket.programs.iter().filter(|(digest, _)| {
                rows.iter().all(|receipt| {
                    receipt.verifier_pass
                        && receipt
                            .matched_program_sha256
                            .iter()
                            .any(|matched| matched == *digest)
                })
            });
            if let Some((digest, program)) = best_adapter(common) {
                by_adapter
                    .entry((digest.clone(), Vec::new()))
                    .or_insert_with(|| (program.clone(), Vec::new()))
                    .1
                    .push(layout);
            } else {
                let Some(adapters) = phase_guarded_layout_adapters(bucket, &rows) else {
                    return Ok(SupportConsensusCandidate::Blocked(
                        "support_phase_adapter_unproven",
                    ));
                };
                for (digest, program, guard) in adapters {
                    by_adapter
                        .entry((digest, guard))
                        .or_insert_with(|| (program, Vec::new()))
                        .1
                        .push(layout.clone());
                }
            }
        }
        by_adapter
            .into_iter()
            .flat_map(|((_, required_request_atom_ids), (program, layouts))| {
                layouts
                    .chunks(16)
                    .map(|layout_chunk| ResponseConsensusVariant {
                        program: program.clone(),
                        allowed_layout_sha256: layout_chunk.to_vec(),
                        required_request_atom_ids: required_request_atom_ids.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    variants.sort_by(|left, right| {
        canonical_json_sha256(&left.program)
            .unwrap_or_default()
            .cmp(&canonical_json_sha256(&right.program).unwrap_or_default())
            .then_with(|| left.allowed_layout_sha256.cmp(&right.allowed_layout_sha256))
    });
    if variants.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
        return Ok(SupportConsensusCandidate::Blocked(
            "support_consensus_variant_budget_exceeded",
        ));
    }
    let candidate = if variants.len() == 1 && variants[0].allowed_layout_sha256.is_empty() {
        variants.into_iter().next().expect("one variant").program
    } else {
        ResponseProgram::unique_consensus(variants)
    };
    candidate.validate().map_err(str::to_owned)?;
    if !candidate_authority_verified_on_support(bucket, &candidate) {
        return Ok(SupportConsensusCandidate::Blocked(
            "support_consensus_authority_unproven",
        ));
    }
    Ok(SupportConsensusCandidate::Ready(candidate))
}

fn candidate_authority_verified_on_support(
    bucket: &OnlineCollectionBucket,
    candidate: &ResponseProgram,
) -> bool {
    bucket.support.iter().all(|receipt| {
        if let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256) {
            // Structural teacher alignment may train a canonical law, but a
            // frozen CPU package must reproduce the complete teacher response.
            return independently_verified_teacher_match(candidate, example);
        }
        durable_adapter_wave_proves_candidate(bucket, receipt, candidate)
            || receipt_proves_candidate_authority(receipt, candidate)
    })
}

fn durable_adapter_wave_proves_candidate(
    bucket: &OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
    candidate: &ResponseProgram,
) -> bool {
    if !receipt.verifier_pass {
        return false;
    }
    let crate::ResponseOperation::UniqueConsensus {
        variants,
        adapter_wave: Some(wave),
    } = &candidate.operation
    else {
        return false;
    };
    if variants.len() != wave.routes.len() || variants.is_empty() {
        return false;
    }

    // The compact checkpoint can prove a unique phase winner without retaining
    // raw payload. Equal-margin routes remain unknown because output parity
    // cannot be reconstructed from phase atoms alone.
    let classes = concrete_adapter_program_classes(bucket);
    let mut ranked = Vec::with_capacity(variants.len());
    for (index, (variant, route)) in variants.iter().zip(&wave.routes).enumerate() {
        let Ok(class_digest) = canonical_json_sha256(&variant.program) else {
            return false;
        };
        let Some((_, source_digests)) = classes.get(&class_digest) else {
            return false;
        };
        let Some(atoms) = durable_adapter_atoms(bucket, receipt, source_digests) else {
            return false;
        };
        if let Some(margin) = crate::runtime::adapter_wave_margin_from_atoms(atoms, route) {
            ranked.push((margin, index, source_digests));
        }
    }
    ranked.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let Some((best_margin, _, winner_digests)) = ranked.first() else {
        return false;
    };
    if ranked
        .iter()
        .filter(|entry| entry.0 == *best_margin)
        .count()
        != 1
    {
        return false;
    }
    receipt
        .matched_program_sha256
        .iter()
        .any(|matched| winner_digests.contains(matched))
}

fn receipt_proves_candidate_authority(
    receipt: &OnlineCollectionReceipt,
    candidate: &ResponseProgram,
) -> bool {
    if !receipt.verifier_pass {
        return false;
    }
    if canonical_json_sha256(candidate).is_ok_and(|digest| {
        receipt
            .matched_program_sha256
            .iter()
            .any(|matched| matched == &digest)
    }) {
        return true;
    }
    let crate::ResponseOperation::UniqueConsensus { variants, .. } = &candidate.operation else {
        return false;
    };
    let mut applicable = false;
    for variant in variants {
        if !variant.allowed_layout_sha256.is_empty()
            && !variant
                .allowed_layout_sha256
                .iter()
                .any(|layout| layout == &receipt.layout_sha256)
        {
            continue;
        }
        if variant
            .required_request_atom_ids
            .iter()
            .any(|atom| receipt.request_atom_ids.binary_search(atom).is_err())
        {
            continue;
        }
        applicable = true;
        let Ok(digest) = canonical_json_sha256(&variant.program) else {
            return false;
        };
        if !receipt
            .matched_program_sha256
            .iter()
            .any(|matched| matched == &digest)
        {
            return false;
        }
    }
    applicable
}

fn collection_support_manifest_digest(bucket: &OnlineCollectionBucket) -> Result<String, String> {
    let program_sha256 = bucket
        .frozen_program_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_program_missing".to_owned())?;
    let watermark_event_time_unix_nanos = bucket
        .support_watermark_event_time_unix_nanos
        .ok_or_else(|| "online_collection_support_watermark_missing".to_owned())?;
    canonical_json_sha256(&CollectionSupportManifestMaterial {
        schema: "nando.collection-support-manifest.v1",
        bucket_id: &bucket.bucket_id,
        program_sha256,
        watermark_event_time_unix_nanos,
        receipts: &bucket.support,
    })
    .map_err(str::to_owned)
}

fn collection_future_manifest_digest(bucket: &OnlineCollectionBucket) -> Result<String, String> {
    let support_manifest_sha256 = bucket
        .support_manifest_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_manifest_missing".to_owned())?;
    canonical_json_sha256(&CollectionFutureManifestMaterial {
        schema: "nando.collection-future-manifest.v1",
        support_manifest_sha256,
        receipts: &bucket.future,
    })
    .map_err(str::to_owned)
}

pub fn online_collection_support_manifest_digest(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, String> {
    canonical_json_sha256(&CollectionSupportManifestMaterial {
        schema: "nando.collection-support-manifest.v1",
        bucket_id: &candidate.bucket_id,
        program_sha256: &candidate.program_sha256,
        watermark_event_time_unix_nanos: candidate.support_watermark_event_time_unix_nanos,
        receipts: &candidate.support_receipts,
    })
    .map_err(str::to_owned)
}

pub fn online_collection_future_manifest_digest(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, String> {
    canonical_json_sha256(&CollectionFutureManifestMaterial {
        schema: "nando.collection-future-manifest.v1",
        support_manifest_sha256: &candidate.support_manifest_sha256,
        receipts: &candidate.future_receipts,
    })
    .map_err(str::to_owned)
}

fn observation_request_atom_ids(observation: &OnlineCollectionObservation) -> BTreeSet<u64> {
    let mut atoms: BTreeSet<u64> = observation
        .example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .and_then(|item| item.get("content"))
        .and_then(request_content_text)
        .map(|text| request_phase_atom_ids(&text).into_iter().collect())
        .unwrap_or_default();
    atoms.extend(response_pre_action_context_atom_ids(
        &observation.example.provider_payload,
    ));
    atoms
}

fn request_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.is_empty()).then(|| text.to_owned());
    }
    let text = content
        .as_array()?
        .iter()
        .filter(|part| {
            matches!(
                part.get("type").and_then(Value::as_str),
                Some("text" | "input_text" | "output_text")
            )
        })
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

fn structural_layout(value: &Value) -> Value {
    match value {
        Value::Null => Value::String("null".to_owned()),
        Value::Bool(_) => Value::String("bool".to_owned()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Value::String("integer".to_owned())
        }
        Value::Number(_) => Value::String("number".to_owned()),
        Value::String(value) => serde_json::from_str::<Value>(value)
            .ok()
            .filter(|parsed| !matches!(parsed, Value::String(_)))
            .map_or_else(
                || Value::String("string".to_owned()),
                |parsed| structural_layout(&parsed),
            ),
        Value::Array(values) => Value::Array(values.iter().map(structural_layout).collect()),
        Value::Object(values) => {
            let mut shapes = values
                .iter()
                .map(|(key, value)| {
                    Value::Array(vec![
                        Value::String(sha256_bytes(key.as_bytes())),
                        structural_layout(value),
                    ])
                })
                .collect::<Vec<_>>();
            shapes.sort_by_cached_key(|shape| serde_json::to_vec(shape).unwrap_or_default());
            Value::Array(shapes)
        }
    }
}

fn support_freeze_blocker(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
) -> Option<String> {
    if bucket.frozen_program_sha256.is_some() {
        return None;
    }
    if bucket.support.len() < required_support_rows {
        return Some(format!("support_rows_below_{required_support_rows}"));
    }
    if bucket.support.iter().any(|receipt| !receipt.verifier_pass) {
        return Some("support_verifier_incomplete".to_owned());
    }
    match support_consensus_candidate(bucket) {
        Ok(SupportConsensusCandidate::Blocked(reason)) => return Some(reason.to_owned()),
        Err(_) => return Some("support_consensus_invalid".to_owned()),
        Ok(SupportConsensusCandidate::Ready(_)) => {}
    }
    if bucket_program_atom_ids(bucket).is_empty() {
        return Some("support_program_atoms_empty".to_owned());
    }
    if bucket
        .support
        .iter()
        .any(|receipt| receipt.event_time_unix_nanos.is_none())
    {
        return Some("support_event_time_missing".to_owned());
    }
    Some("support_freeze_ready_not_applied".to_owned())
}

fn bucket_status(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
) -> OnlineCollectionBucketStatus {
    let retained_runtime_examples = bucket.runtime_examples.len();
    let support_rows_with_runtime_examples = bucket
        .support
        .iter()
        .filter(|receipt| {
            bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    let digest_law_keys = bucket
        .programs
        .iter()
        .filter_map(|(digest, program)| {
            response_law_key(program)
                .ok()
                .map(|law_key| (digest.as_str(), law_key))
        })
        .collect::<BTreeMap<_, _>>();
    let mut abstract_law_support = BTreeMap::<Vec<u8>, usize>::new();
    let mut abstract_law_replayable_support = BTreeMap::<Vec<u8>, usize>::new();
    let mut abstract_law_sessions = BTreeMap::<Vec<u8>, BTreeSet<String>>::new();
    for receipt in &bucket.support {
        let receipt_laws = receipt
            .matched_program_sha256
            .iter()
            .filter_map(|digest| digest_law_keys.get(digest.as_str()).cloned())
            .collect::<BTreeSet<_>>();
        for law_key in receipt_laws {
            *abstract_law_support.entry(law_key.clone()).or_default() += 1;
            abstract_law_sessions
                .entry(law_key.clone())
                .or_default()
                .insert(receipt.session_id_sha256.clone());
            if bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
            {
                *abstract_law_replayable_support.entry(law_key).or_default() += 1;
            }
        }
    }
    let abstract_law_groups = abstract_law_support.len();
    let best_law_key = abstract_law_support
        .iter()
        .max_by(|(left_key, left_rows), (right_key, right_rows)| {
            left_rows
                .cmp(right_rows)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(law_key, _)| law_key.clone());
    let best_abstract_law_support_rows =
        abstract_law_support.into_values().max().unwrap_or_default();
    let best_abstract_law_replayable_support_rows = abstract_law_replayable_support
        .into_values()
        .max()
        .unwrap_or_default();
    let best_abstract_law_missing_replay_hints = best_law_key
        .as_ref()
        .map(|law_key| {
            bucket
                .support
                .iter()
                .filter(|receipt| {
                    !bucket
                        .runtime_examples
                        .contains_key(&receipt.evidence_graph_sha256)
                        && receipt.matched_program_sha256.iter().any(|digest| {
                            digest_law_keys
                                .get(digest.as_str())
                                .is_some_and(|candidate| candidate == law_key)
                        })
                })
                .take(MAX_TARGETED_REHYDRATION_HINTS)
                .map(|receipt| OnlineCollectionRehydrationHint {
                    evidence_graph_sha256: receipt.evidence_graph_sha256.clone(),
                    session_id_sha256: receipt.session_id_sha256.clone(),
                    event_time_unix_nanos: receipt.event_time_unix_nanos,
                    estimated_input_tokens: receipt.estimated_input_tokens,
                })
                .collect()
        })
        .unwrap_or_default();
    let best_abstract_law_session_ids_sha256 = best_law_key
        .clone()
        .and_then(|law_key| abstract_law_sessions.remove(&law_key))
        .unwrap_or_default()
        .into_iter()
        .take(MAX_TARGETED_REHYDRATION_HINTS)
        .collect();
    // Matched digests are durable exact teacher proofs. Runtime examples are
    // tracked separately because they are optional synthesis working memory.
    let best_verified_law_support_rows = best_abstract_law_support_rows;
    let future_sessions = distinct_receipt_sessions(&bucket.future);
    let future_layouts = distinct_receipt_layouts(&bucket.future);
    let runtime_parity_cases = bucket
        .future
        .iter()
        .filter(|receipt| {
            bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
                || bucket
                    .durable_runtime_parity_receipts
                    .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    let admission_blocker = if bucket.frozen_program_sha256.is_none() {
        status_support_freeze_blocker(
            bucket,
            required_support_rows,
            best_verified_law_support_rows,
        )
    } else if bucket.future.len() < 32 {
        Some("future_rows_below_32".to_owned())
    } else if future_sessions < 3 {
        Some("future_sessions_below_3".to_owned())
    } else if future_layouts < 2 {
        Some("future_layouts_below_2".to_owned())
    } else if bucket.wrong_accepts > 0 {
        Some("wrong_accepts_nonzero".to_owned())
    } else if runtime_parity_cases < 32 {
        Some("runtime_parity_cases_below_32".to_owned())
    } else {
        None
    };
    OnlineCollectionBucketStatus {
        bucket_id: bucket.bucket_id.clone(),
        version_space_size: bucket.programs.len(),
        support_rows: bucket.support.len(),
        retained_runtime_examples,
        support_rows_with_runtime_examples,
        abstract_law_groups,
        best_abstract_law_support_rows,
        best_abstract_law_replayable_support_rows,
        best_abstract_law_session_ids_sha256,
        best_abstract_law_missing_replay_hints,
        best_verified_law_support_rows,
        future_rows: bucket.future.len(),
        future_sessions,
        future_layouts,
        wrong_accepts: bucket.wrong_accepts,
        frozen: bucket.frozen_program_sha256.is_some(),
        candidate_program_sha256: bucket.frozen_program_sha256.clone(),
        candidate_program_kind: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|digest| bucket.programs.get(digest))
            .map(response_program_kind_code)
            .map(str::to_owned),
        program_kinds: bucket
            .programs
            .values()
            .map(response_program_kind_code)
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        rejected_programs: bucket.rejected_program_sha256.len(),
        learned_anti_atoms: bucket.learned_anti_atom_ids.len(),
        common_request_atoms: bucket.common_request_atom_ids.len(),
        support_tokens: bucket
            .support
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        future_tokens: bucket
            .future
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        support_watermark_event_time_unix_nanos: bucket.support_watermark_event_time_unix_nanos,
        support_manifest_sha256: bucket.support_manifest_sha256.clone(),
        future_manifest_sha256: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|_| collection_future_manifest_digest(bucket).ok()),
        runtime_parity_cases,
        admission_blocker,
    }
}

fn status_support_freeze_blocker(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    verified_law_support_rows: usize,
) -> Option<String> {
    if bucket.wrong_accepts > 0 {
        return Some("support_wrong_accepts_nonzero".to_owned());
    }
    if bucket.support.len() < required_support_rows {
        // The blocker is part of the operator-facing accounting contract, so
        // it must report the threshold used by this bucket rather than the
        // production default. Tests and migrations intentionally use smaller
        // thresholds while preserving the same admission logic.
        return Some(format!("support_rows_below_{required_support_rows}"));
    }
    if bucket
        .support
        .iter()
        .any(|receipt| !receipt.verifier_pass || receipt.matched_program_sha256.is_empty())
    {
        return Some("support_verifier_incomplete".to_owned());
    }
    if verified_law_support_rows < required_support_rows {
        return Some("support_consensus_authority_unproven".to_owned());
    }
    if bucket_program_atom_ids(bucket).is_empty() {
        return Some("support_program_atoms_empty".to_owned());
    }
    if bucket
        .support
        .iter()
        .any(|receipt| receipt.event_time_unix_nanos.is_none())
    {
        return Some("support_event_time_missing".to_owned());
    }
    Some("support_phase_adapter_unproven".to_owned())
}

fn response_program_kind_code(program: &ResponseProgram) -> &'static str {
    match response_program_kind(program) {
        AstProgramKind::PlanAdvance => "plan_advance",
        AstProgramKind::FunctionCall => "function_call",
        AstProgramKind::CustomToolCall => "custom_tool_call",
        AstProgramKind::Project => "project",
        AstProgramKind::Status => "status",
        AstProgramKind::Collection => "collection",
        AstProgramKind::Legacy => "legacy",
    }
}

fn merge_receipts(
    target: &mut Vec<OnlineCollectionReceipt>,
    source: Vec<OnlineCollectionReceipt>,
    max: usize,
) {
    let mut by_evidence = BTreeMap::<String, OnlineCollectionReceipt>::new();
    for mut receipt in target.drain(..).chain(source) {
        let evidence = receipt.evidence_graph_sha256.clone();
        if let Some(existing) = by_evidence.get_mut(&evidence) {
            existing
                .request_atom_ids
                .append(&mut receipt.request_atom_ids);
            existing.request_atom_ids.sort_unstable();
            existing.request_atom_ids.dedup();
            existing
                .matched_program_sha256
                .append(&mut receipt.matched_program_sha256);
            existing.matched_program_sha256.sort();
            existing.matched_program_sha256.dedup();
            if existing.witness_class_commitment_sha256.is_none() {
                existing.witness_class_commitment_sha256 = receipt.witness_class_commitment_sha256;
                existing.witness_round = receipt.witness_round;
                existing.witness_candidates_before = receipt.witness_candidates_before;
                existing.witness_candidates_after = receipt.witness_candidates_after;
            }
        } else {
            receipt.matched_program_sha256.sort();
            receipt.matched_program_sha256.dedup();
            by_evidence.insert(evidence, receipt);
        }
    }
    let mut receipts = by_evidence.into_values().collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    if receipts.len() > max {
        receipts.drain(..receipts.len().saturating_sub(max));
    }
    *target = receipts;
}

fn push_bounded<T>(values: &mut Vec<T>, value: T, max: usize) {
    if values.len() == max {
        values.remove(0);
    }
    values.push(value);
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_witness_receipt_metadata(receipt: &OnlineCollectionReceipt) -> bool {
    match (
        receipt.witness_class_commitment_sha256.as_deref(),
        receipt.witness_round,
        receipt.witness_candidates_before,
        receipt.witness_candidates_after,
    ) {
        (None, None, None, None) => true,
        (Some(commitment), Some(round), Some(before), Some(after)) => {
            is_sha256(commitment)
                && (1..=MAX_ACTIVE_WITNESS_ROUNDS).contains(&round)
                && after > 0
                && after < before
        }
        _ => false,
    }
}

fn sync_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("online_collection_checkpoint_parent_sync:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mixed_support_blockers_trigger_program_subcenter_split() {
        for blocker in [
            "support_program_cover_empty",
            "support_program_cover_incomplete",
            "support_consensus_authority_unproven",
        ] {
            assert!(support_blocker_requires_subcenter_split(Some(blocker)));
        }
        assert!(!support_blocker_requires_subcenter_split(Some(
            "support_rows_below_32"
        )));
        assert!(!support_blocker_requires_subcenter_split(None));
    }

    #[test]
    fn teacher_mismatch_never_attaches_empty_program_receipt() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-teacher-mismatch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        miner
            .observe_replay_training_buffered(observation(1, "3"))
            .expect("initial support");

        let mismatch = observation(2, "4");
        assert!(
            miner
                .matching_unfrozen_buckets(&mismatch)
                .expect("matching buckets")
                .is_empty()
        );
        miner
            .observe_replay_training_buffered(mismatch)
            .expect("mismatched teacher observation");
        assert!(miner.checkpoint.buckets.iter().all(|bucket| {
            bucket
                .support
                .iter()
                .all(|receipt| !receipt.matched_program_sha256.is_empty())
        }));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn unfrozen_matching_is_bounded_by_wave_route_budget() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-bounded-route-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 32,
            future_rows: 32,
            max_buckets: 32,
            max_receipts_per_bucket: 64,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        let matching_observation = observation(1, "3");
        miner
            .observe_replay_training_buffered(matching_observation.clone())
            .expect("initial support");
        let template = miner.checkpoint.buckets[0].clone();
        let program = template.programs.values().next().expect("program").clone();
        miner.checkpoint.buckets.clear();
        for bucket_index in 0..20 {
            let mut bucket = template.clone();
            bucket.bucket_id = format!("{bucket_index:064x}");
            bucket.programs = (0..16)
                .map(|program_index| {
                    (
                        format!("{bucket_index:032x}{program_index:032x}"),
                        program.clone(),
                    )
                })
                .collect();
            miner.checkpoint.buckets.push(bucket);
        }

        let checks_before = miner.checkpoint.version_space_intersection_checks_total;
        let scheduled_before = miner.checkpoint.guard_scheduled_buckets_total;
        let pruned_before = miner.checkpoint.guard_pruned_buckets_total;
        let matching = miner
            .matching_unfrozen_buckets(&matching_observation)
            .expect("matching buckets");

        assert_eq!(matching.len(), MAX_UNFROZEN_ROUTE_BUCKETS);
        assert_eq!(
            miner.checkpoint.guard_scheduled_buckets_total - scheduled_before,
            MAX_UNFROZEN_ROUTE_BUCKETS as u64
        );
        assert_eq!(
            miner.checkpoint.guard_pruned_buckets_total - pruned_before,
            12
        );
        assert_eq!(
            miner.checkpoint.version_space_intersection_checks_total - checks_before,
            (MAX_UNFROZEN_ROUTE_BUCKETS * MAX_UNFROZEN_ROUTE_PROGRAMS) as u64
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn support_program_subcenter_survives_restart_without_parent_remerge() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-subcenter-restart-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let examples = (1..=4)
            .map(|index| observation(index, "3"))
            .collect::<Vec<_>>();
        let program = enumerate_source_neutral_response_programs(&examples[0].example)
            .expect("version space")
            .programs
            .into_iter()
            .find(|program| {
                is_source_neutral_response_program(program)
                    && examples.iter().all(|example| {
                        independently_verified_authority_response(program, &example.example)
                            .as_deref()
                            == Some("3")
                    })
            })
            .expect("portable count program");
        let program_sha256 = canonical_json_sha256(&program).expect("program digest");
        let programs = BTreeMap::from([(program_sha256.clone(), program)]);
        let support = examples
            .iter()
            .map(|example| {
                receipt_with_program_atoms(example, true, &programs).expect("support receipt")
            })
            .collect::<Vec<_>>();
        let parent = OnlineCollectionBucket {
            bucket_id: "a".repeat(64),
            archetype_id: "b".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        let subcenter = support_program_subcenters(&parent, 4, 16)
            .expect("subcenters")
            .into_iter()
            .next()
            .expect("program subcenter");
        assert_ne!(parent.archetype_id, subcenter.archetype_id);
        let subcenter_archetype_id = subcenter.archetype_id.clone();

        let path = root.join("checkpoint.cbor");
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        miner.checkpoint.buckets = vec![parent, subcenter];
        miner.persist().expect("persist parent and subcenter");
        let checkpoint_before = fs::read(&path).expect("checkpoint before reopen");
        drop(miner);

        let reopened = OnlineCollectionMiner::open(&path, config).expect("reopen");
        let checkpoint_after = fs::read(&path).expect("checkpoint after reopen");
        assert_eq!(checkpoint_before, checkpoint_after);
        assert_eq!(reopened.checkpoint.buckets.len(), 2);
        assert!(
            reopened
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.archetype_id == subcenter_archetype_id)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn renderer_variants_form_one_law_subcenter_and_survive_restart() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-law-subcenter-restart-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let examples = (1_usize..=4)
            .map(|index| {
                let rendered = !index.is_multiple_of(2);
                let mut observation = observation(index, if rendered { "Total: 3." } else { "3" });
                observation.example.provider_payload["input"][0]["content"][0]["text"] =
                    Value::String(if rendered {
                        "Return exactly \"Total: {count}.\"".to_owned()
                    } else {
                        "Return direct count".to_owned()
                    });
                observation
            })
            .collect::<Vec<_>>();
        let direct = ResponseProgram::compose_collection(
            vec![
                crate::CollectionProgramStep::SelectOnlyArrayField,
                crate::CollectionProgramStep::Count,
            ],
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let rendered = direct.clone().with_collection_renderer(
            crate::CollectionOutputRenderer::RequestTemplate {
                marker: crate::RequestTemplateMarker::BracedCount,
            },
        );
        assert_eq!(
            response_law_key(&direct).expect("direct law"),
            response_law_key(&rendered).expect("rendered law")
        );
        let direct_sha256 = canonical_json_sha256(&direct).expect("direct digest");
        let rendered_sha256 = canonical_json_sha256(&rendered).expect("rendered digest");
        for (index, example) in examples.iter().enumerate() {
            let program = if (index + 1).is_multiple_of(2) {
                &direct
            } else {
                &rendered
            };
            assert_eq!(
                independently_verified_authority_response_result(program, &example.example),
                Ok(example.example.expected_response.clone()),
                "concrete adapter {}",
                index + 1
            );
        }
        let programs = BTreeMap::from([
            (direct_sha256.clone(), direct),
            (rendered_sha256.clone(), rendered),
        ]);
        let runtime_examples = examples
            .iter()
            .map(|example| {
                (
                    example.evidence_graph_sha256.clone(),
                    example.example.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let support = examples
            .iter()
            .enumerate()
            .map(|(index, example)| {
                let mut receipt =
                    receipt_with_program_atoms(example, true, &programs).expect("support receipt");
                receipt.matched_program_sha256 = vec![if (index + 1).is_multiple_of(2) {
                    direct_sha256.clone()
                } else {
                    rendered_sha256.clone()
                }];
                receipt
            })
            .collect::<Vec<_>>();
        let parent = OnlineCollectionBucket {
            bucket_id: "c".repeat(64),
            archetype_id: "d".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        let law_child = support_law_subcenters(&parent, 4, 16)
            .expect("law subcenters")
            .into_iter()
            .next()
            .expect("law subcenter");
        assert_eq!(law_child.support.len(), 4);
        assert_eq!(law_child.programs.len(), 2);
        assert!(law_child.programs.values().any(|program| matches!(
            &program.operation,
            crate::ResponseOperation::ComposeCollection {
                renderer: crate::CollectionOutputRenderer::Direct,
                ..
            }
        )));
        assert!(law_child.programs.values().any(|program| matches!(
            &program.operation,
            crate::ResponseOperation::ComposeCollection {
                renderer: crate::CollectionOutputRenderer::RequestTemplate { .. },
                ..
            }
        )));
        let law_archetype_id = law_child.archetype_id.clone();

        let path = root.join("checkpoint.cbor");
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        miner.checkpoint.buckets = vec![parent, law_child];
        miner.maybe_freeze(1).expect("freeze law child");
        assert!(miner.checkpoint.buckets[0].frozen_program_sha256.is_none());
        assert!(miner.checkpoint.buckets[1].frozen_program_sha256.is_some());
        miner
            .persist()
            .expect("persist parent and frozen law child");
        drop(miner);

        let reopened = OnlineCollectionMiner::open(&path, config).expect("reopen");
        assert_eq!(reopened.checkpoint.buckets.len(), 2);
        assert!(reopened.checkpoint.buckets.iter().any(|bucket| {
            bucket.archetype_id == law_archetype_id && bucket.frozen_program_sha256.is_some()
        }));
        assert!(
            reopened
                .checkpoint
                .buckets
                .iter()
                .any(|bucket| bucket.bucket_id == "c".repeat(64))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn maximal_decidable_subcenter_keeps_32_clean_rows_and_excludes_ambiguous_layout() {
        let alpha = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "alpha".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let beta = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "beta".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        assert_eq!(
            response_law_key(&alpha).expect("alpha law"),
            response_law_key(&beta).expect("beta law")
        );
        let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
        let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
        let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
        let clean_layouts = ["a".repeat(64), "b".repeat(64)];
        let ambiguous_layout = "c".repeat(64);
        let support = (0..36)
            .map(|index| OnlineCollectionReceipt {
                evidence_graph_sha256: format!("{:064x}", index + 1),
                client_intent_id_sha256: format!("{:064x}", index + 1_000),
                session_id_sha256: format!("{:064x}", index % 8 + 2_000),
                event_time_unix_nanos: Some(index as u64 + 1),
                layout_sha256: if index < 32 {
                    clean_layouts[index / 16].clone()
                } else {
                    ambiguous_layout.clone()
                },
                estimated_input_tokens: 100,
                verifier_pass: true,
                request_atom_ids: vec![7, 11],
                matched_program_sha256: vec![if index < 32 || index.is_multiple_of(2) {
                    alpha_digest.clone()
                } else {
                    beta_digest.clone()
                }],
                witness_class_commitment_sha256: None,
                witness_round: None,
                witness_candidates_before: None,
                witness_candidates_after: None,
            })
            .collect::<Vec<_>>();
        let parent = OnlineCollectionBucket {
            bucket_id: "d".repeat(64),
            archetype_id: "e".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::from([7, 11]),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        assert!(matches!(
            support_consensus_candidate(&parent).expect("parent consensus"),
            SupportConsensusCandidate::Blocked("support_phase_adapter_unproven")
        ));

        let child = maximal_decidable_support_subcenter(&parent, 32, 128)
            .expect("decidable split")
            .expect("clean child");
        assert_eq!(child.support.len(), 32);
        assert!(
            child
                .support
                .iter()
                .all(|receipt| receipt.layout_sha256 != ambiguous_layout)
        );
        assert_eq!(
            child
                .support
                .iter()
                .map(|receipt| receipt.layout_sha256.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(clean_layouts)
        );
        assert!(matches!(
            support_consensus_candidate(&child).expect("child consensus"),
            SupportConsensusCandidate::Ready(_)
        ));
        assert_eq!(child.wrong_accepts, 0);
    }

    #[test]
    fn durable_pre_action_atoms_restore_phase_adapter_without_raw_examples() {
        let alpha = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "alpha".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let beta = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "beta".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
        let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
        let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
        let alpha_atom = crate::stable_atom_id("request:select-alpha");
        let beta_atom = crate::stable_atom_id("request:select-beta");
        let program_atoms = programs
            .values()
            .flat_map(response_program_required_routing_atom_ids)
            .collect::<BTreeSet<_>>();
        let support = (0..32)
            .map(|index| {
                let mut request_atom_ids = program_atoms.iter().copied().collect::<Vec<_>>();
                request_atom_ids.push(if index < 16 { alpha_atom } else { beta_atom });
                request_atom_ids.sort_unstable();
                request_atom_ids.dedup();
                OnlineCollectionReceipt {
                    evidence_graph_sha256: format!("{:064x}", index + 40_000),
                    client_intent_id_sha256: format!("{:064x}", index + 50_000),
                    session_id_sha256: format!("{:064x}", index % 8 + 60_000),
                    event_time_unix_nanos: Some(index as u64 + 1),
                    layout_sha256: "a".repeat(64),
                    estimated_input_tokens: 100,
                    verifier_pass: true,
                    request_atom_ids,
                    matched_program_sha256: vec![if index < 16 {
                        alpha_digest.clone()
                    } else {
                        beta_digest.clone()
                    }],
                    witness_class_commitment_sha256: None,
                    witness_round: None,
                    witness_candidates_before: None,
                    witness_candidates_after: None,
                }
            })
            .collect();
        let bucket = OnlineCollectionBucket {
            bucket_id: "d".repeat(64),
            archetype_id: "e".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };

        let candidate = match support_consensus_candidate(&bucket).expect("consensus") {
            SupportConsensusCandidate::Ready(candidate) => candidate,
            SupportConsensusCandidate::Blocked(reason) => panic!("blocked: {reason}"),
        };
        let crate::ResponseOperation::UniqueConsensus { variants, .. } = candidate.operation else {
            panic!("expected guarded consensus");
        };
        assert_eq!(variants.len(), 2);
        assert!(variants.iter().all(|variant| {
            variant.required_request_atom_ids == vec![alpha_atom]
                || variant.required_request_atom_ids == vec![beta_atom]
        }));

        let root = std::env::temp_dir().join(format!(
            "nando-durable-phase-migration-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("collection.checkpoint");
        let config = OnlineCollectionConfig::default();
        let mut legacy = OnlineCollectionMiner::open(&path, config).expect("legacy shell");
        legacy.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V31;
        legacy.checkpoint.buckets = vec![bucket];
        legacy.persist().expect("persist v31");
        drop(legacy);

        let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v32");
        assert_eq!(
            migrated.checkpoint.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(migrated.status().frozen_buckets_total, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn durable_law_subcenter_restores_verified_rows_without_raw_examples() {
        let plain = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "value".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let json = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "value".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        let plain_digest = canonical_json_sha256(&plain).expect("plain digest");
        let json_digest = canonical_json_sha256(&json).expect("json digest");
        let support = (0..60)
            .map(|index| OnlineCollectionReceipt {
                evidence_graph_sha256: format!("{:064x}", index + 70_000),
                client_intent_id_sha256: format!("{:064x}", index + 80_000),
                session_id_sha256: format!("{:064x}", index % 8 + 90_000),
                event_time_unix_nanos: Some(index as u64 + 1),
                layout_sha256: "a".repeat(64),
                estimated_input_tokens: 100,
                verifier_pass: true,
                request_atom_ids: vec![crate::stable_atom_id("request:project")],
                matched_program_sha256: vec![if index < 40 {
                    plain_digest.clone()
                } else {
                    json_digest.clone()
                }],
                witness_class_commitment_sha256: None,
                witness_round: None,
                witness_candidates_before: None,
                witness_candidates_after: None,
            })
            .collect();
        let bucket = OnlineCollectionBucket {
            bucket_id: "1".repeat(64),
            archetype_id: "2".repeat(64),
            programs: BTreeMap::from([(plain_digest.clone(), plain), (json_digest, json)]),
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };

        let subcenters = support_law_subcenters(&bucket, 32, 128).expect("law subcenters");
        assert_eq!(subcenters.len(), 1);
        assert_eq!(subcenters[0].support.len(), 40);
        assert_eq!(subcenters[0].programs.len(), 1);
        assert!(subcenters[0].programs.contains_key(&plain_digest));
        assert!(
            support_law_subcenters(&subcenters[0], 32, 128)
                .expect("no recursive law subcenter")
                .is_empty()
        );

        let root = std::env::temp_dir().join(format!(
            "nando-exact-subcenter-dedup-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("collection.checkpoint");
        let config = OnlineCollectionConfig::default();
        let mut left = subcenters[0].clone();
        left.bucket_id = "3".repeat(64);
        left.archetype_id = "4".repeat(64);
        let mut right = left.clone();
        right.bucket_id = "5".repeat(64);
        right.archetype_id = "6".repeat(64);
        let mut legacy = OnlineCollectionMiner::open(&path, config).expect("v33 shell");
        legacy.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V33;
        legacy.checkpoint.buckets = vec![right, left];
        legacy.persist().expect("persist duplicate v33 children");
        drop(legacy);

        let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v34");
        assert_eq!(
            migrated.checkpoint.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(migrated.checkpoint.buckets.len(), 1);
        assert_eq!(migrated.checkpoint.buckets[0].bucket_id, "3".repeat(64));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn clean_pre_action_atoms_recover_32_rows_from_one_ambiguous_layout() {
        let alpha = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "alpha".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let beta = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonField {
                field: "beta".to_owned(),
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
        let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
        let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
        let observations = (0..42)
            .map(|index| OnlineCollectionObservation {
                evidence_graph_sha256: format!("{:064x}", index + 10_000),
                client_intent_id_sha256: format!("{:064x}", index + 20_000),
                session_id_sha256: format!("{:064x}", index % 8 + 30_000),
                event_time_unix_nanos: Some(index as u64 + 1),
                estimated_input_tokens: 100,
                example: CollectionSynthesisExample {
                    provider_payload: json!({
                        "input": [
                            {"type":"message", "role":"user", "content":[{
                                "type":"input_text",
                                "text": if index < 32 { "select alpha" } else { "select value" }
                            }]},
                            {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
                        ]
                    }),
                    expected_response: if index < 36 { "7" } else { "8" }.to_owned(),
                },
            })
            .collect::<Vec<_>>();
        let layout = structural_layout_sha256(&observations[0].example.provider_payload)
            .expect("shared layout");
        assert!(observations.iter().all(|observation| {
            structural_layout_sha256(&observation.example.provider_payload).as_deref()
                == Ok(layout.as_str())
        }));
        let support = observations
            .iter()
            .enumerate()
            .map(|(index, observation)| {
                let mut value = receipt(observation, true).expect("receipt");
                value.matched_program_sha256 = vec![if index < 36 {
                    alpha_digest.clone()
                } else {
                    beta_digest.clone()
                }];
                value
            })
            .collect::<Vec<_>>();
        let runtime_examples = observations
            .iter()
            .map(|observation| {
                (
                    observation.evidence_graph_sha256.clone(),
                    observation.example.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let parent = OnlineCollectionBucket {
            bucket_id: "8".repeat(64),
            archetype_id: "9".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        assert!(matches!(
            support_consensus_candidate(&parent).expect("parent consensus"),
            SupportConsensusCandidate::Blocked("support_phase_adapter_unproven")
        ));

        let child = clean_pre_action_program_subcenter(&parent, 32, 128)
            .expect("pre-action split")
            .expect("clean child");
        assert_eq!(child.support.len(), 32);
        assert_eq!(child.programs.len(), 1);
        assert!(child.programs.contains_key(&alpha_digest));
        assert!(!child.common_request_atom_ids.is_empty());
        assert!(matches!(
            support_consensus_candidate(&child).expect("child consensus"),
            SupportConsensusCandidate::Ready(_)
        ));
        assert_eq!(child.wrong_accepts, 0);
    }

    #[test]
    fn runtime_example_compaction_preserves_independent_authority() {
        let mut example = observation(1, "3");
        let program = enumerate_source_neutral_response_programs(&example.example)
            .expect("version space")
            .programs
            .into_iter()
            .find(|program| {
                is_source_neutral_response_program(program)
                    && program_any(program, is_count_operation)
                    && independently_verified_authority_response(program, &example.example)
                        .as_deref()
                        == Some("3")
            })
            .expect("count program");
        let input = example.example.provider_payload["input"]
            .as_array_mut()
            .expect("input");
        input.insert(
            0,
            serde_json::json!({
                "type": "message",
                "role": "system",
                "content": [{"type":"input_text", "text":"x".repeat(4_096)}]
            }),
        );
        input.insert(
            2,
            serde_json::json!({
                "type": "function_call",
                "name": "query",
                "arguments": "{}"
            }),
        );
        let program_sha256 = canonical_json_sha256(&program).expect("program digest");
        let programs = BTreeMap::from([(program_sha256, program.clone())]);
        let support =
            vec![receipt_with_program_atoms(&example, true, &programs).expect("support receipt")];
        let mut bucket = OnlineCollectionBucket {
            bucket_id: "e".repeat(64),
            archetype_id: "f".repeat(64),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        let full_bytes = serde_cbor::to_vec(&example.example)
            .expect("full example")
            .len();
        insert_runtime_example(&mut bucket, &example, 128);
        let compact = bucket
            .runtime_examples
            .get(&example.evidence_graph_sha256)
            .expect("compact example");
        let compact_bytes = serde_cbor::to_vec(compact).expect("compact example").len();
        assert!(compact_bytes < full_bytes / 2);
        assert_eq!(
            independently_verified_authority_response(&program, compact),
            independently_verified_authority_response(&program, &example.example)
        );
        assert_eq!(
            compact.provider_payload["input"]
                .as_array()
                .expect("compact input")
                .len(),
            2
        );
    }

    #[test]
    fn runtime_reservoir_preserves_top_law_under_byte_pressure() {
        let seed = observation(1, "3");
        let top_program = enumerate_source_neutral_response_programs(&seed.example)
            .expect("version space")
            .programs
            .into_iter()
            .find(|program| {
                is_source_neutral_response_program(program)
                    && program_any(program, is_count_operation)
            })
            .expect("count program");
        let mut secondary_program = top_program.clone();
        let crate::ResponseOperation::ComposeCollection { max_items, .. } =
            &mut secondary_program.operation
        else {
            panic!("count program");
        };
        *max_items = max_items.saturating_sub(1).max(1);
        let top_digest = canonical_json_sha256(&top_program).expect("top digest");
        let secondary_digest = canonical_json_sha256(&secondary_program).expect("secondary digest");
        assert_ne!(
            response_law_key(&top_program).expect("top law"),
            response_law_key(&secondary_program).expect("secondary law")
        );
        let mut support = Vec::new();
        let mut runtime_examples = BTreeMap::new();
        for index in 1..=60 {
            let mut example = observation(index, "3");
            example.example.provider_payload["retained_noise"] = Value::String("x".repeat(50_000));
            let mut receipt = receipt(&example, true).expect("receipt");
            receipt.matched_program_sha256 = vec![if index <= 40 {
                top_digest.clone()
            } else {
                secondary_digest.clone()
            }];
            support.push(receipt);
            runtime_examples.insert(
                example.evidence_graph_sha256.clone(),
                example.example.clone(),
            );
        }
        let mut bucket = OnlineCollectionBucket {
            bucket_id: "1".repeat(64),
            archetype_id: "2".repeat(64),
            programs: BTreeMap::from([
                (top_digest.clone(), top_program),
                (secondary_digest, secondary_program),
            ]),
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        trim_bucket_runtime_examples(&mut bucket, 128);
        let retained_top_law = bucket
            .support
            .iter()
            .filter(|receipt| {
                receipt.matched_program_sha256 == [top_digest.clone()]
                    && bucket
                        .runtime_examples
                        .contains_key(&receipt.evidence_graph_sha256)
            })
            .count();
        assert!(retained_top_law >= 32, "retained {retained_top_law}");
        assert!(
            persisted_runtime_example_bytes(&bucket.runtime_examples)
                <= MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
        );
    }

    #[test]
    fn unguarded_consensus_unifies_equivalent_selectors_and_abstains_on_disagreement() {
        let make_observation = |index: usize, left: Value, right: Value| {
            OnlineCollectionObservation {
                evidence_graph_sha256: format!("{:064x}", index + 70_000),
                client_intent_id_sha256: format!("{:064x}", index + 80_000),
                session_id_sha256: format!("{:064x}", index % 4 + 90_000),
                event_time_unix_nanos: Some(index as u64),
                estimated_input_tokens: 100,
                example: CollectionSynthesisExample {
                    provider_payload: json!({
                        "input": [
                            {"type":"message","role":"user","content":[{
                                "type":"input_text","text":"Return the selected value"
                            }]},
                            {"type":"function_call_output","output":json!({"left":left}).to_string()},
                            {"type":"function_call_output","output":json!({"right":right}).to_string()}
                        ]
                    }),
                    expected_response: "3".to_owned(),
                },
            }
        };
        let left = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::TurnOutputScalarOrdinal {
                output_ordinal: 1,
                scalar_ordinal: 0,
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let right = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::TurnOutputScalarOrdinal {
                output_ordinal: 2,
                scalar_ordinal: 0,
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let left_digest = canonical_json_sha256(&left).expect("left digest");
        let right_digest = canonical_json_sha256(&right).expect("right digest");
        let programs = BTreeMap::from([(left_digest.clone(), left), (right_digest.clone(), right)]);
        let observations = (1_usize..=4)
            .map(|index| {
                if index.is_multiple_of(2) {
                    make_observation(index, json!(3), json!("n/a"))
                } else {
                    make_observation(index, json!("n/a"), json!(3))
                }
            })
            .collect::<Vec<_>>();
        let support = observations
            .iter()
            .enumerate()
            .map(|(index, observation)| {
                let mut value = receipt_with_program_atoms(observation, true, &programs)
                    .expect("support receipt");
                value.matched_program_sha256 = vec![if index.is_multiple_of(2) {
                    left_digest.clone()
                } else {
                    right_digest.clone()
                }];
                value
            })
            .collect::<Vec<_>>();
        let runtime_examples = observations
            .iter()
            .map(|observation| {
                (
                    observation.evidence_graph_sha256.clone(),
                    observation.example.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for observation in &observations {
            let responses = programs
                .values()
                .filter_map(|program| {
                    independently_verified_authority_response(program, &observation.example)
                })
                .collect::<BTreeSet<_>>();
            assert_eq!(responses, BTreeSet::from(["3".to_owned()]));
        }
        let manual_consensus = ResponseProgram::unique_consensus(
            programs
                .values()
                .cloned()
                .map(|program| ResponseConsensusVariant {
                    program,
                    allowed_layout_sha256: Vec::new(),
                    required_request_atom_ids: Vec::new(),
                })
                .collect(),
        );
        for observation in &observations {
            assert_eq!(
                independently_verified_authority_response_result(
                    &manual_consensus,
                    &observation.example
                ),
                Ok("3".to_owned())
            );
        }
        let bucket = OnlineCollectionBucket {
            bucket_id: "a".repeat(64),
            archetype_id: "project".to_owned(),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        let candidate = match support_consensus_candidate(&bucket).expect("candidate") {
            SupportConsensusCandidate::Ready(candidate) => candidate,
            SupportConsensusCandidate::Blocked(reason) => panic!("blocked: {reason}"),
        };
        assert!(matches!(
            &candidate.operation,
            crate::ResponseOperation::UniqueConsensus { .. }
        ));
        for observation in &observations {
            assert_eq!(
                independently_verified_authority_response(&candidate, &observation.example)
                    .as_deref(),
                Some("3")
            );
        }

        let conflict = make_observation(1, json!(3), json!(4));
        let execution = execute_response(&candidate, "", &conflict.example.provider_payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        let mut conflicting_bucket = bucket;
        conflicting_bucket.runtime_examples.insert(
            conflict.evidence_graph_sha256.clone(),
            conflict.example.clone(),
        );
        assert!(!matches!(
            support_consensus_candidate(&conflicting_bucket).expect("conflicting candidate"),
            SupportConsensusCandidate::Ready(_)
        ));
    }

    #[test]
    fn keyed_surface_layout_routes_actor_and_independent_verifier() {
        let alpha_payload = serde_json::json!({
            "input": [{"type":"function_call_output", "output":"{\"alpha\":7}"}]
        });
        let beta_payload = serde_json::json!({
            "input": [{"type":"function_call_output", "output":"{\"beta\":8}"}]
        });
        let alpha_layout = structural_layout_sha256(&alpha_payload).expect("alpha layout");
        let beta_layout = structural_layout_sha256(&beta_payload).expect("beta layout");
        assert_ne!(alpha_layout, beta_layout);
        let consensus = ResponseProgram::unique_consensus(vec![
            ResponseConsensusVariant {
                program: ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::JsonField {
                        field: "alpha".to_owned(),
                        value_type: crate::AtomValueType::Integer,
                    },
                    crate::ValueProjectionFormat::PlainText,
                    "completed",
                ),
                allowed_layout_sha256: vec![alpha_layout],
                required_request_atom_ids: Vec::new(),
            },
            ResponseConsensusVariant {
                program: ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::JsonField {
                        field: "beta".to_owned(),
                        value_type: crate::AtomValueType::Integer,
                    },
                    crate::ValueProjectionFormat::PlainText,
                    "completed",
                ),
                allowed_layout_sha256: vec![beta_layout],
                required_request_atom_ids: Vec::new(),
            },
        ]);
        consensus.validate().expect("consensus");
        for (provider_payload, expected_response) in [(alpha_payload, "7"), (beta_payload, "8")] {
            let example = CollectionSynthesisExample {
                provider_payload,
                expected_response: expected_response.to_owned(),
            };
            assert_eq!(
                independently_verified_authority_response(&consensus, &example).as_deref(),
                Some(expected_response)
            );
        }
    }

    #[test]
    fn request_phase_guard_routes_same_layout_adapters() {
        let payload = |request: &str| {
            serde_json::json!({
                "input": [
                    {"type":"message", "role":"user", "content":[{"type":"input_text", "text":request}]},
                    {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
                ]
            })
        };
        let alpha_payload = payload("select alpha");
        let beta_payload = payload("select beta");
        let layout = structural_layout_sha256(&alpha_payload).expect("layout");
        assert_eq!(
            layout,
            structural_layout_sha256(&beta_payload).expect("same layout")
        );
        let alpha_atoms = request_phase_atom_ids("select alpha");
        let beta_atoms = request_phase_atom_ids("select beta");
        let alpha_guard = alpha_atoms
            .iter()
            .copied()
            .find(|atom| beta_atoms.binary_search(atom).is_err())
            .expect("alpha discriminator");
        let beta_guard = beta_atoms
            .iter()
            .copied()
            .find(|atom| alpha_atoms.binary_search(atom).is_err())
            .expect("beta discriminator");
        let consensus = ResponseProgram::unique_consensus(vec![
            ResponseConsensusVariant {
                program: ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::JsonField {
                        field: "alpha".to_owned(),
                        value_type: crate::AtomValueType::Integer,
                    },
                    crate::ValueProjectionFormat::PlainText,
                    "completed",
                ),
                allowed_layout_sha256: vec![layout.clone()],
                required_request_atom_ids: vec![alpha_guard],
            },
            ResponseConsensusVariant {
                program: ResponseProgram::project_selected_value(
                    crate::ResponseValueSelector::JsonField {
                        field: "beta".to_owned(),
                        value_type: crate::AtomValueType::Integer,
                    },
                    crate::ValueProjectionFormat::PlainText,
                    "completed",
                ),
                allowed_layout_sha256: vec![layout],
                required_request_atom_ids: vec![beta_guard],
            },
        ]);
        consensus.validate().expect("consensus");
        for (provider_payload, expected_response) in [(alpha_payload, "7"), (beta_payload, "8")] {
            let example = CollectionSynthesisExample {
                provider_payload,
                expected_response: expected_response.to_owned(),
            };
            assert_eq!(
                independently_verified_authority_response(&consensus, &example).as_deref(),
                Some(expected_response)
            );
        }
    }

    #[test]
    fn request_referenced_json_field_has_actor_verifier_parity() {
        let payload = |request: &str| {
            json!({
                "input": [
                    {"type":"message", "role":"user", "content":[{
                        "type":"input_text", "text":request
                    }]},
                    {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
                ]
            })
        };
        let program = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::RequestReferencedJsonField {
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        program.validate().expect("program");
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        let selected = payload("Return alpha");
        let execution = execute_response(&program, "", &selected);
        assert_eq!(execution.response.as_deref(), Some("7"));
        assert!(verify_response_independently(&verifier, &selected, "7").is_ok());

        let ambiguous = payload("Return alpha and beta");
        assert_eq!(
            execute_response(&program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());

        let from_end = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                reverse_ordinal: 0,
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let from_end_verifier =
            source_neutral_verifier_for_program(&from_end).expect("from-end verifier");
        let from_end_payload = json!({
            "input": [
                {"type":"message", "role":"user", "content":"Return the result"},
                {"type":"function_call_output", "output":"{\"values\":[1,2,7]}"}
            ]
        });
        assert_eq!(
            execute_response(&from_end, "", &from_end_payload)
                .response
                .as_deref(),
            Some("7")
        );
        assert!(verify_response_independently(&from_end_verifier, &from_end_payload, "7").is_ok());
        assert!(verify_response_independently(&from_end_verifier, &from_end_payload, "2").is_err());
    }

    #[test]
    fn phase_ranked_adapter_selects_unique_physical_role_and_abstains_on_tie() {
        let program = |ordinal: u16| {
            ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::JsonScalarOrdinal {
                    ordinal,
                    value_type: crate::AtomValueType::Integer,
                },
                crate::ValueProjectionFormat::PlainText,
                "completed",
            )
        };
        let alpha = program(0);
        let beta = program(1);
        let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
        let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
        let programs = BTreeMap::from([(alpha_digest, alpha), (beta_digest, beta)]);
        let observation = |index: usize, requested: &str, alpha: i64, beta: i64| {
            let expected = if requested == "alpha" { alpha } else { beta };
            OnlineCollectionObservation {
                evidence_graph_sha256: format!("{index:064x}"),
                client_intent_id_sha256: format!("{:064x}", index + 100),
                session_id_sha256: format!("{:064x}", index % 2 + 200),
                event_time_unix_nanos: Some(index as u64),
                estimated_input_tokens: 100,
                example: CollectionSynthesisExample {
                    provider_payload: serde_json::json!({
                        "input": [
                            {"type":"message", "role":"user", "content":[{
                                "type":"input_text",
                                "text":format!("Return {requested}")
                            }]},
                            {"type":"function_call_output", "output":serde_json::json!({
                                "alpha":alpha,
                                "beta":beta
                            }).to_string()}
                        ]
                    }),
                    expected_response: expected.to_string(),
                },
            }
        };
        let support_observations = [
            observation(1, "alpha", 11, 21),
            observation(2, "beta", 12, 22),
            observation(3, "alpha", 13, 23),
            observation(4, "beta", 14, 24),
        ];
        let support = support_observations
            .iter()
            .map(|observation| {
                receipt_with_program_atoms(observation, true, &programs).expect("receipt")
            })
            .collect::<Vec<_>>();
        let runtime_examples = support_observations
            .iter()
            .map(|observation| {
                (
                    observation.evidence_graph_sha256.clone(),
                    observation.example.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut bucket = OnlineCollectionBucket {
            bucket_id: "a".repeat(64),
            archetype_id: "project".to_owned(),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples,
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        refresh_durable_adapter_phase_atoms(&mut bucket);
        assert_eq!(bucket.durable_adapter_phase_atoms.len(), 4);
        bucket.runtime_examples.clear();
        let candidate = phase_ranked_semantic_adapters(&bucket).expect("wave candidate");
        for future in [
            observation(5, "alpha", 105, 205),
            observation(6, "beta", 106, 206),
        ] {
            assert_eq!(
                independently_verified_authority_response(&candidate, &future.example),
                Some(future.example.expected_response)
            );
        }
        let mut prose = observation(8, "alpha", 108, 208);
        prose.example.expected_response = "Selected: 108.".to_owned();
        assert!(response_program_authority_matches_example(
            &candidate,
            &prose.example
        ));
        assert_eq!(
            independently_verified_authority_response(&candidate, &prose.example).as_deref(),
            Some("108")
        );
        let ambiguous = observation(7, "result", 107, 207);
        let execution = execute_response(&candidate, "", &ambiguous.example.provider_payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        assert!(
            independently_verified_authority_response(&candidate, &ambiguous.example).is_none()
        );

        let root = std::env::temp_dir().join(format!(
            "nando-adapter-wave-maintenance-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        let bucket_id = bucket.bucket_id.clone();
        miner.checkpoint.buckets = vec![bucket];
        miner
            .checkpoint
            .structural_resynthesis_pending_bucket_ids
            .insert(bucket_id);
        miner
            .run_structural_resynthesis_work_slice()
            .expect("proof refresh");
        let frozen = miner
            .checkpoint
            .buckets
            .iter()
            .find(|bucket| bucket.frozen_program_sha256.is_some())
            .expect("frozen law subcenter");
        assert_eq!(frozen.future.len(), 0);
        assert_eq!(frozen.wrong_accepts, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn program_any(
        program: &ResponseProgram,
        predicate: fn(&crate::ResponseOperation) -> bool,
    ) -> bool {
        predicate(&program.operation)
            || matches!(
                &program.operation,
                crate::ResponseOperation::UniqueConsensus { variants, .. }
                    if variants.iter().any(|variant| program_any(&variant.program, predicate))
            )
    }

    fn is_count_operation(operation: &crate::ResponseOperation) -> bool {
        matches!(
            operation,
            crate::ResponseOperation::ComposeCollection { steps, .. }
                if steps.iter().any(|step| matches!(step, crate::CollectionProgramStep::Count))
        )
    }

    fn is_multi_output_project(operation: &crate::ResponseOperation) -> bool {
        matches!(
            operation,
            crate::ResponseOperation::ProjectSelectedValue {
                selector: crate::ResponseValueSelector::UniqueTurnScalar { .. },
                renderer: crate::CollectionOutputRenderer::RenderSequence { .. },
                ..
            }
        )
    }

    fn observation(index: usize, expected: &str) -> OnlineCollectionObservation {
        let field = format!("surface_{index}");
        let base = i64::try_from(index).unwrap_or_default().saturating_mul(10);
        OnlineCollectionObservation {
            evidence_graph_sha256: format!("{index:064x}"),
            client_intent_id_sha256: format!("{:064x}", index + 10_000),
            session_id_sha256: format!("{:064x}", index % 4 + 20_000),
            event_time_unix_nanos: Some(index as u64),
            estimated_input_tokens: 100,
            example: CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[
                        {"type":"message","role":"user","content":[{
                            "type":"input_text",
                            "text":format!("Count records for batch {index}")
                        }]},
                        {"type":"function_call_output","output":json!({
                            field: [
                                {"value":base + 1},
                                {"value":base + 2},
                                {"value":base + 3}
                            ]
                        }).to_string()}
                    ]
                }),
                expected_response: expected.to_owned(),
            },
        }
    }

    fn ordinal_count_observation(
        index: usize,
        expected: &str,
        output_ordinal: u16,
    ) -> OnlineCollectionObservation {
        let mut observation = observation(index, expected);
        let input = observation.example.provider_payload["input"]
            .as_array_mut()
            .expect("input array");
        input[0]["content"][0]["text"] = Value::String("Count the records".to_owned());
        let noise = json!({
            "type":"function_call_output",
            "output":"command completed"
        });
        match output_ordinal {
            1 => input.push(noise),
            2 => input.insert(1, noise),
            _ => panic!("unsupported test ordinal"),
        }
        observation
    }

    fn routed_observation(
        index: usize,
        expected: &str,
        prompt: &str,
        alternate_layout: bool,
    ) -> OnlineCollectionObservation {
        let mut observation = observation(index, expected);
        observation.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String(prompt.to_owned());
        if alternate_layout {
            let output = observation.example.provider_payload["input"][1]["output"]
                .as_str()
                .expect("tool output");
            let mut parsed = serde_json::from_str::<Value>(output).expect("tool json");
            parsed["layout_marker"] = Value::Bool(true);
            observation.example.provider_payload["input"][1]["output"] =
                Value::String(parsed.to_string());
        }
        observation
    }

    fn multi_output_observation(index: usize) -> OnlineCollectionObservation {
        let total_field = format!("total_surface_{index}");
        let status_field = format!("status_surface_{index}");
        let total = index.saturating_add(40);
        let status = format!("ready-{index}");
        let total_output = if index.is_multiple_of(2) {
            json!({"wrapper": {(total_field.clone()): total}})
        } else {
            json!({(total_field): total})
        };
        let status_output = if index.is_multiple_of(2) {
            json!({"result": {(status_field.clone()): status}})
        } else {
            json!({(status_field): status})
        };
        OnlineCollectionObservation {
            evidence_graph_sha256: format!("{:064x}", index + 30_000),
            client_intent_id_sha256: format!("{:064x}", index + 40_000),
            session_id_sha256: format!("{:064x}", index % 4 + 50_000),
            event_time_unix_nanos: Some(index as u64),
            estimated_input_tokens: 1_000,
            example: CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[
                        {"type":"message","role":"user","content":[{
                            "type":"input_text","text":"Summarize the verified result"
                        }]},
                        {"type":"function_call_output","call_id":format!("a-{index}"),"output":total_output.to_string()},
                        {"type":"function_call_output","call_id":format!("b-{index}"),"output":status_output.to_string()}
                    ]
                }),
                expected_response: format!("Total: {total}; status: ready-{index}."),
            },
        }
    }

    #[test]
    fn teacher_prose_trains_canonical_count_without_storing_static_text() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-canonical-count-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let mut miner = OnlineCollectionMiner::open(
            root.join("checkpoint.cbor"),
            OnlineCollectionConfig {
                support_rows: 4,
                future_rows: 4,
                max_buckets: 8,
                max_receipts_per_bucket: 16,
            },
        )
        .expect("miner");
        miner
            .observe_replay_training_buffered(observation(
                1,
                "The verified batch contains 3 usable objects.",
            ))
            .expect("support");

        assert_eq!(miner.checkpoint.unsupported_total, 0);
        assert_eq!(
            miner
                .checkpoint
                .exact_executable_observations_total
                .saturating_add(miner.checkpoint.semantic_executable_observations_total),
            1
        );
        assert_eq!(miner.checkpoint.buckets.len(), 1);
        assert!(
            miner.checkpoint.buckets[0]
                .support
                .iter()
                .all(|receipt| receipt.verifier_pass)
        );
        assert!(miner.checkpoint.buckets[0].frozen_program_sha256.is_none());
        assert!(
            miner.checkpoint.buckets[0]
                .programs
                .values()
                .any(|program| {
                    matches!(
                        &program.operation,
                        crate::ResponseOperation::ComposeCollection {
                            steps,
                            renderer: crate::CollectionOutputRenderer::Direct,
                            ..
                        } if matches!(steps.last(), Some(crate::CollectionProgramStep::Count))
                    )
                })
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn different_teacher_surfaces_converge_to_one_canonical_program() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-surface-convergence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let mut miner = OnlineCollectionMiner::open(
            root.join("checkpoint.cbor"),
            OnlineCollectionConfig {
                support_rows: 4,
                future_rows: 4,
                max_buckets: 8,
                max_receipts_per_bucket: 16,
            },
        )
        .expect("miner");
        for (index, surface) in [
            "Total records: 3",
            "Count: 3",
            "Found 3 records",
            "Matching rows: 3",
            "There are 3 rows",
            "Verified count is 3",
            "The batch contains 3 records",
            "Result: 3 items",
        ]
        .into_iter()
        .enumerate()
        {
            let observation = observation(index + 1, surface);
            if index < 4 {
                miner
                    .observe_replay_training_buffered(observation)
                    .expect("support");
            } else {
                miner.observe(observation).expect("future");
            }
        }

        let status = miner.status();
        assert_eq!(
            status
                .exact_executable_observations_total
                .saturating_add(status.semantic_executable_observations_total),
            8
        );
        assert_eq!(status.unsupported_total, 0);
        assert_eq!(status.frozen_buckets_total, 1);
        assert_eq!(status.future_receipts_unique_total, 4);
        assert_eq!(status.wrong_accepts_total, 0);
        assert!(miner.checkpoint.buckets.len() <= 2);
        assert!(
            miner
                .checkpoint
                .buckets
                .iter()
                .flat_map(|bucket| bucket.programs.values())
                .any(|program| program_any(program, is_count_operation))
        );
        assert_eq!(miner.quarantine_packages().expect("packages").len(), 1);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn output_ordinal_surfaces_share_one_count_law_and_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-output-ordinal-law-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let mut miner = OnlineCollectionMiner::open(
            root.join("checkpoint.cbor"),
            OnlineCollectionConfig {
                support_rows: 4,
                future_rows: 4,
                max_buckets: 8,
                max_receipts_per_bucket: 16,
            },
        )
        .expect("miner");
        for index in 1..=4 {
            let observation =
                ordinal_count_observation(index, "3", if index.is_multiple_of(2) { 2 } else { 1 });
            miner
                .observe_replay_training_buffered(observation)
                .expect("support");
        }
        for index in 5..=8 {
            let observation =
                ordinal_count_observation(index, "3", if index.is_multiple_of(2) { 2 } else { 1 });
            miner.observe(observation).expect("future");
        }

        let status = miner.status();
        assert_eq!(status.frozen_buckets_total, 1);
        assert_eq!(status.future_receipts_unique_total, 4);
        assert_eq!(status.runtime_parity_cases_total, 4);
        assert_eq!(status.wrong_accepts_total, 0);
        assert_eq!(miner.quarantine_packages().expect("packages").len(), 1);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn converged_unfrozen_program_pools_merge_without_touching_frozen_pool() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-merge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 32,
            future_rows: 32,
            max_buckets: 64,
            max_receipts_per_bucket: 64,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        miner
            .observe_replay_training_buffered(observation(1, "3"))
            .expect("support");
        let original = miner.checkpoint.buckets[0].clone();
        let program_digests = original.programs.keys().cloned().collect::<Vec<_>>();
        let mut duplicate = original.clone();
        duplicate.bucket_id = "d".repeat(64);
        miner.checkpoint.buckets.push(duplicate);
        miner
            .merge_converged_unfrozen_buckets()
            .expect("merge converged");
        assert_eq!(
            miner
                .checkpoint
                .buckets
                .iter()
                .filter(|bucket| bucket.programs.keys().eq(program_digests.iter()))
                .count(),
            1
        );
        assert_eq!(miner.checkpoint.buckets[0].support.len(), 1);

        let mut frozen = miner.checkpoint.buckets[0].clone();
        frozen.bucket_id = "f".repeat(64);
        frozen.frozen_program_sha256 = frozen.programs.keys().next().cloned();
        miner.checkpoint.buckets.push(frozen);
        miner
            .merge_converged_unfrozen_buckets()
            .expect("keep frozen");
        assert_eq!(
            miner
                .checkpoint
                .buckets
                .iter()
                .filter(|bucket| bucket.programs.keys().eq(program_digests.iter()))
                .count(),
            2
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn overlapping_unfrozen_version_spaces_merge_to_proven_intersection() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-overlap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 32,
            future_rows: 32,
            max_buckets: 64,
            max_receipts_per_bucket: 64,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        miner
            .observe_replay_training_buffered(observation(1, "3"))
            .expect("support");
        let original = miner.checkpoint.buckets[0].clone();
        let shared = original.programs.keys().next().cloned().expect("program");
        let shared_program = original.programs.get(&shared).cloned().expect("program");
        let mut left = original.clone();
        left.programs.retain(|digest, _| digest == &shared);
        let extra_left = canonical_json_sha256(&"left").expect("digest");
        left.programs.insert(extra_left, shared_program.clone());
        let mut right = original;
        right.bucket_id = "d".repeat(64);
        right.programs.retain(|digest, _| digest == &shared);
        let extra_right = canonical_json_sha256(&"right").expect("digest");
        right.programs.insert(extra_right, shared_program);
        miner.checkpoint.buckets = vec![left, right];
        miner
            .merge_converged_unfrozen_buckets()
            .expect("merge overlap");
        assert_eq!(miner.checkpoint.buckets.len(), 1);
        assert_eq!(miner.checkpoint.buckets[0].programs.len(), 3);
        assert!(miner.checkpoint.buckets[0].programs.contains_key(&shared));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn version_space_restart_preserves_privacy_safe_runtime_parity_receipts() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.json");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=4 {
            miner.observe(observation(index, "3")).expect("observe");
        }
        let support_only_package_id = miner
            .quarantine_packages()
            .expect("support package")
            .into_iter()
            .next()
            .expect("frozen package")
            .package_id;
        let mut late = observation(9, "3");
        late.event_time_unix_nanos = Some(3);
        miner.observe(late).expect("late after freeze");
        let mut leaked_intent = observation(10, "3");
        leaked_intent.client_intent_id_sha256 = observation(1, "3").client_intent_id_sha256;
        miner
            .observe(leaked_intent)
            .expect("support intent after freeze");
        for index in 5..=8 {
            miner.observe(observation(index, "3")).expect("future");
        }
        miner
            .observe(observation(8, "3"))
            .expect("duplicate observation");
        let status = miner.status();
        assert_eq!(status.observations_total, 10);
        assert_eq!(status.duplicate_observations_total, 1);
        assert_eq!(status.late_after_freeze_total, 1);
        assert_eq!(status.future_intent_rejected_total, 1);
        assert_eq!(status.full_enumerations_total, 1);
        assert_eq!(status.version_space_intersection_checks_total, 3);
        assert_eq!(status.guard_scheduled_buckets_total, 3);
        assert_eq!(status.guard_pruned_buckets_total, 0);
        assert_eq!(status.buckets.len(), 1);
        assert!(status.buckets[0].frozen);
        assert_eq!(status.buckets[0].support_rows, 4);
        assert_eq!(status.buckets[0].future_rows, 4);
        assert_eq!(status.buckets[0].wrong_accepts, 0);
        assert!(status.buckets[0].support_manifest_sha256.is_some());
        assert!(status.buckets[0].future_manifest_sha256.is_some());
        let packages = miner.quarantine_packages().expect("packages");
        assert_eq!(packages.len(), 1);
        assert_ne!(packages[0].package_id, support_only_package_id);
        assert_eq!(packages[0].state, ResponsePackageState::Quarantine);
        assert!(!packages[0].eligible_for_admission_candidate());
        drop(miner);
        let durable = fs::read(&path).expect("checkpoint");
        assert!(durable.starts_with(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3));
        assert!(
            !durable
                .windows(b"surface_".len())
                .any(|row| row == b"surface_")
        );
        assert!(
            !durable
                .windows(b"provider_payload".len())
                .any(|row| row == b"provider_payload")
        );
        let restored = OnlineCollectionMiner::open(&path, config).expect("restart");
        assert_eq!(restored.status(), status);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn prefreeze_support_receipts_survive_restart_without_raw_examples() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-prefreeze-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=2 {
            miner
                .observe_replay_training_buffered(observation(index, "3"))
                .expect("support before restart");
        }
        miner.flush().expect("flush support");
        let before = miner.status();
        assert_eq!(before.support_receipts_unique_total, 2);
        assert_eq!(before.future_receipts_unique_total, 0);
        assert_eq!(before.frozen_buckets_total, 0);
        drop(miner);

        let durable = fs::read(&path).expect("checkpoint");
        for private in [b"provider_payload".as_slice(), b"surface_".as_slice()] {
            assert!(
                !durable
                    .windows(private.len())
                    .any(|window| window == private),
                "checkpoint leaked replay input"
            );
        }

        let mut restored = OnlineCollectionMiner::open(&path, config).expect("restart");
        let after = restored.status();
        assert_eq!(after.support_receipts_unique_total, 2);
        assert_eq!(after.future_receipts_unique_total, 0);
        assert_eq!(after.unreplayable_support_discarded_total, 0);
        for index in 3..=4 {
            restored
                .observe_replay_training_buffered(observation(index, "3"))
                .expect("support after restart");
        }
        let frozen = restored.status();
        assert_eq!(frozen.support_receipts_unique_total, 4);
        assert_eq!(frozen.future_receipts_unique_total, 0);
        assert_eq!(frozen.frozen_buckets_total, 1);
        assert_eq!(frozen.wrong_accepts_total, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v4_checkpoint_restarts_as_non_authoritative_teacher_history() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-v5-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=8 {
            miner.observe(observation(index, "3")).expect("evidence");
        }
        assert_eq!(miner.status().future_receipts_unique_total, 4);
        miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V4;
        miner.persist().expect("persist v4 marker");
        drop(miner);

        let migrated = OnlineCollectionMiner::open(&path, config).expect("migrated miner");
        let status = migrated.status();
        assert_eq!(
            status.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(status.frozen_buckets_total, 0);
        assert_eq!(status.future_receipts_unique_total, 0);
        assert!(status.observation_accounting_complete);
        assert_eq!(
            status.teacher_only_observations_total,
            status.observations_total
        );
        assert!(migrated.checkpoint.buckets.iter().all(|bucket| {
            bucket.frozen_program_sha256.is_none()
                && bucket.future.is_empty()
                && bucket.support.is_empty()
        }));
        assert!(status.unreplayable_support_discarded_total >= 4);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v6_rebuilds_exact_renderer_candidates_without_claiming_evidence() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-v6-renderer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig::default();
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        let evidence_id = "f".repeat(64);
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"ok\":3}"}]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        let teacher = ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::UniqueTurnScalar {
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        );
        let teacher_digest = canonical_json_sha256(&teacher).expect("teacher digest");
        let archetype_id = response_program_archetype_id(&teacher).expect("archetype");
        miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V5;
        miner.checkpoint.observations_total = 1;
        miner.checkpoint.unsupported_total = 1;
        miner.checkpoint.teacher_only_observations_total = 1;
        miner
            .checkpoint
            .observed_evidence_graph_sha256
            .insert(evidence_id.clone());
        miner.checkpoint.buckets.push(OnlineCollectionBucket {
            bucket_id: "legacy-teacher".to_owned(),
            archetype_id,
            programs: BTreeMap::from([(teacher_digest, teacher)]),
            common_request_atom_ids: BTreeSet::new(),
            support: Vec::new(),
            future: Vec::new(),
            runtime_examples: BTreeMap::from([(evidence_id, example.clone())]),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: None,
            support_watermark_event_time_unix_nanos: None,
            support_manifest_sha256: None,
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        });
        miner.persist().expect("persist v5 checkpoint");
        drop(miner);

        let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v9");
        let status = migrated.status();
        assert_eq!(
            status.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(status.renderer_consensus_migrated_examples_total, 1);
        assert_eq!(status.support_receipts_unique_total, 0);
        assert_eq!(status.future_receipts_unique_total, 0);
        assert!(migrated.checkpoint.buckets.iter().any(|bucket| {
            bucket.programs.values().any(|program| {
                crate::response_program_exactly_matches_example(program, &example)
                    && !is_source_neutral_response_program(program)
            })
        }));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v7_repairs_historical_variant_digest_and_revokes_invalid_freeze() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-v7-witness-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig::default();
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        let observation = observation(1, "3");
        let program = enumerate_source_neutral_response_programs(&observation.example)
            .expect("programs")
            .programs
            .into_iter()
            .find(|program| {
                crate::response_program_exactly_matches_example(program, &observation.example)
                    && is_privacy_safe_online_response_program(program)
            })
            .expect("exact program");
        let program_digest = canonical_json_sha256(&program).expect("program digest");
        let programs = BTreeMap::from([(program_digest.clone(), program.clone())]);
        let mut receipt =
            receipt_with_program_atoms(&observation, true, &programs).expect("receipt");
        let historical_digest = "c".repeat(64);
        receipt
            .matched_program_sha256
            .push(historical_digest.clone());
        miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V7;
        miner.checkpoint.buckets = vec![OnlineCollectionBucket {
            bucket_id: "legacy-v7-witness".to_owned(),
            archetype_id: response_program_archetype_id(&program).expect("archetype"),
            programs,
            common_request_atom_ids: BTreeSet::new(),
            support: vec![receipt.clone()],
            future: vec![receipt],
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: Some(program_digest),
            support_watermark_event_time_unix_nanos: Some(1),
            support_manifest_sha256: Some("d".repeat(64)),
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::from([42]),
            wrong_accepts: 0,
        }];
        miner.persist().expect("persist v7 checkpoint");
        drop(miner);

        let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v9");
        let bucket = migrated.checkpoint.buckets.first().expect("bucket");
        assert_eq!(
            migrated.status().pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert!(bucket.rejected_program_sha256.contains(&historical_digest));
        assert!(bucket.learned_anti_atom_ids.is_empty());
        assert!(bucket.support.is_empty());
        assert!(bucket.future.is_empty());
        assert!(bucket.frozen_program_sha256.is_none());
        assert!(bucket.support_manifest_sha256.is_none());
        assert!(migrated.status().unreplayable_support_discarded_total >= 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_program_pool_reuses_across_rephrased_requests() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-rephrased-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        miner
            .observe(routed_observation(
                1,
                "Total records: 3",
                "Count records for this batch",
                false,
            ))
            .expect("first surface");
        miner
            .observe(routed_observation(
                2,
                "Total records: 3",
                "How many entries are present?",
                true,
            ))
            .expect("rephrased surface");
        let status = miner.status();
        assert_eq!(status.buckets_total, 1);
        assert_eq!(status.program_pool_reuse_total, 1);
        assert_eq!(status.buckets[0].support_rows, 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn latest_output_program_pool_reuses_across_different_output_ordinals() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-latest-output-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        for (index, preceding_outputs) in [(1_usize, 1_usize), (2, 3)] {
            let mut input = vec![json!({
                "type":"message",
                "role":"user",
                "content":format!("Select the completed value for layout {index}")
            })];
            input.extend((0..preceding_outputs).map(|ordinal| {
                json!({
                    "type":"function_call_output",
                    "output":format!("unrelated-{index}-{ordinal}")
                })
            }));
            input.push(json!({
                "type":"function_call_output",
                "output":"header\nselected-result\nfooter"
            }));
            miner
                .observe(OnlineCollectionObservation {
                    evidence_graph_sha256: format!("{:064x}", index + 30_000),
                    client_intent_id_sha256: format!("{:064x}", index + 40_000),
                    session_id_sha256: format!("{:064x}", index + 50_000),
                    event_time_unix_nanos: Some(index as u64),
                    estimated_input_tokens: 100,
                    example: CollectionSynthesisExample {
                        provider_payload: json!({"input":input}),
                        expected_response: "selected-result".to_owned(),
                    },
                })
                .expect("observation");
        }
        assert_eq!(miner.checkpoint.buckets.len(), 1);
        let bucket = &miner.checkpoint.buckets[0];
        assert_eq!(bucket.support.len(), 2);
        assert!(bucket.programs.values().any(|program| {
            matches!(
                &program.operation,
                crate::ResponseOperation::ProjectSelectedValue {
                    selector: crate::ResponseValueSelector::LatestTurnOutputLine {
                        line_index: 1,
                        ..
                    },
                    ..
                }
            )
        }));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn replay_training_freezes_support_but_never_claims_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-replay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        for index in 1..=8 {
            miner
                .observe_replay_training_buffered(observation(index, "3"))
                .expect("replay support");
        }
        miner.flush().expect("flush replay support");
        let replay_status = miner.status();
        assert_eq!(replay_status.buckets.len(), 1);
        assert!(replay_status.buckets[0].frozen);
        assert_eq!(replay_status.buckets[0].support_rows, 4);
        assert_eq!(replay_status.buckets[0].future_rows, 0);

        for index in 9..=12 {
            let mut live = observation(index, "3");
            live.session_id_sha256 = format!("{:064x}", index + 60_000);
            miner.observe(live).expect("live future");
        }
        let live_status = miner.status();
        assert_eq!(live_status.buckets[0].future_rows, 4);
        assert_eq!(live_status.buckets[0].wrong_accepts, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn replay_training_rehydrates_discarded_support_without_double_accounting() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-rehydrate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        let evidence = observation(1, "3");
        miner
            .observe(evidence.clone())
            .expect("initial observation");
        assert_eq!(miner.status().observations_total, 1);
        assert_eq!(miner.status().support_receipts_unique_total, 1);

        for bucket in &mut miner.checkpoint.buckets {
            bucket.support.clear();
            bucket.runtime_examples.clear();
        }
        assert_eq!(miner.status().support_receipts_unique_total, 0);

        miner
            .observe_replay_training_buffered(evidence.clone())
            .expect("rehydrate discarded support");
        let repaired = miner.status();
        assert_eq!(repaired.observations_total, 1);
        assert_eq!(repaired.duplicate_observations_total, 0);
        assert_eq!(repaired.support_receipts_unique_total, 1);
        assert!(repaired.observation_accounting_complete);

        let stale_digest = "f".repeat(64);
        miner.checkpoint.buckets[0].support[0].matched_program_sha256 = vec![stale_digest];
        miner
            .observe_replay_training_buffered(evidence)
            .expect("refresh retained stale receipt");
        let refreshed = miner.status();
        assert_eq!(refreshed.observations_total, 1);
        assert_eq!(refreshed.duplicate_observations_total, 0);
        assert_eq!(refreshed.support_receipts_unique_total, 1);
        assert!(
            miner.checkpoint.buckets[0].support[0]
                .matched_program_sha256
                .iter()
                .any(|digest| miner.checkpoint.buckets[0].programs.contains_key(digest))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v20_restart_preserves_support_without_bulk_revalidation() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-v20-revalidate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        miner
            .observe(observation(1, "3"))
            .expect("initial observation");
        let observations = miner.status().observations_total;
        let support = miner.status().support_receipts_unique_total;
        let original_digest = miner.checkpoint.buckets[0].support[0]
            .matched_program_sha256
            .first()
            .cloned()
            .expect("at least one proven program link");
        let mut equivalent = miner.checkpoint.buckets[0].programs[&original_digest].clone();
        equivalent.max_output_bytes = equivalent.max_output_bytes.saturating_add(1);
        let equivalent_digest = canonical_json_sha256(&equivalent).expect("equivalent digest");
        assert_ne!(equivalent_digest, original_digest);
        miner.checkpoint.buckets[0]
            .programs
            .insert(equivalent_digest.clone(), equivalent);
        assert!(
            !miner.checkpoint.buckets[0].support[0]
                .matched_program_sha256
                .contains(&equivalent_digest)
        );
        miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V19;
        miner.persist().expect("persist v19 checkpoint");
        drop(miner);

        let restored = OnlineCollectionMiner::open(&path, config).expect("migrate to v20");
        let status = restored.status();
        assert_eq!(
            status.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(status.observations_total, observations);
        assert_eq!(status.support_receipts_unique_total, support);
        assert_eq!(status.future_receipts_unique_total, 0);
        assert_eq!(status.wrong_accepts_total, 0);
        assert!(
            !restored.checkpoint.buckets[0].support[0]
                .matched_program_sha256
                .contains(&equivalent_digest)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v21_restart_resynthesizes_retained_support_without_creating_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-v21-resynthesis-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 32,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=4 {
            miner
                .observe(multi_output_observation(index))
                .expect("support");
        }
        let bucket_index = miner
            .checkpoint
            .buckets
            .iter()
            .position(|bucket| {
                bucket.support.len() >= config.support_rows
                    && bucket
                        .programs
                        .values()
                        .any(|program| canonical_dynamic_role_count(program) >= 2)
            })
            .expect("multi-scalar support bucket");
        let removed = miner.checkpoint.buckets[bucket_index]
            .programs
            .iter()
            .filter_map(|(digest, program)| {
                let example = miner.checkpoint.buckets[bucket_index]
                    .runtime_examples
                    .values()
                    .next()?;
                let response = independently_verified_authority_response(program, example)?;
                (canonical_dynamic_role_count(program) >= 2
                    && response != example.expected_response)
                    .then_some(digest.clone())
            })
            .collect::<BTreeSet<_>>();
        assert!(
            !removed.is_empty(),
            "canonical law must exist before downgrade"
        );
        let bucket = &mut miner.checkpoint.buckets[bucket_index];
        bucket
            .programs
            .retain(|digest, _| !removed.contains(digest));
        assert!(
            !bucket.programs.is_empty(),
            "surface programs remain in V20"
        );
        for receipt in &mut bucket.support {
            receipt
                .matched_program_sha256
                .retain(|digest| !removed.contains(digest));
        }
        bucket.frozen_program_sha256 = None;
        bucket.support_watermark_event_time_unix_nanos = None;
        bucket.support_manifest_sha256 = None;
        bucket.future.clear();
        bucket.durable_runtime_parity_receipts.clear();
        miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V20;
        miner.persist().expect("persist V20 checkpoint");
        drop(miner);

        let mut restored = OnlineCollectionMiner::open(&path, config).expect("migrate to V21");
        assert!(restored.has_structural_resynthesis_work());
        while restored.has_structural_resynthesis_work() {
            restored
                .run_structural_resynthesis_work_slice()
                .expect("bounded structural resynthesis");
        }
        let status = restored.status();
        assert_eq!(
            status.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert_eq!(status.future_receipts_unique_total, 0);
        assert_eq!(status.wrong_accepts_total, 0);
        assert_eq!(status.structural_resynthesis_pending_buckets, 0);
        assert!(status.structural_resynthesis_completed_buckets_total >= 1);
        assert_eq!(status.structural_resynthesis_failed_buckets_total, 0);
        assert!(restored.checkpoint.buckets.iter().any(|bucket| {
            bucket.programs.values().any(|program| {
                canonical_dynamic_role_count(program) >= 2
                    && bucket.runtime_examples.values().any(|example| {
                        independently_verified_authority_response(program, example)
                            .is_some_and(|response| response != example.expected_response)
                    })
            })
        }));
        assert!(status.support_receipts_unique_total >= config.support_rows);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn legacy_rehydration_joins_unique_verified_receipt_across_identity_and_layout_versions() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-legacy-rehydrate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        let original = observation(1, "3");
        miner
            .observe_replay_training_buffered(original.clone())
            .expect("legacy support");

        let legacy_evidence = "e".repeat(64);
        let legacy_session = "d".repeat(64);
        let bucket = &mut miner.checkpoint.buckets[0];
        bucket.runtime_examples.clear();
        bucket.support[0].evidence_graph_sha256 = legacy_evidence.clone();
        bucket.support[0].session_id_sha256 = legacy_session.clone();

        let mut reconstructed = original;
        reconstructed.evidence_graph_sha256 = "c".repeat(64);
        reconstructed.session_id_sha256 = "b".repeat(64);
        reconstructed.example.provider_payload["migration_layout_marker"] = json!(true);
        let stats = miner
            .rehydrate_legacy_replay_training_buffered(
                reconstructed,
                &BTreeSet::from([legacy_session]),
            )
            .expect("verified legacy join");
        assert_eq!(stats.session_receipts, 1);
        assert_eq!(stats.event_time_matches, 1);
        assert_eq!(stats.token_matches, 1);
        assert_eq!(stats.verifier_matches, 1);
        assert_eq!(stats.layout_matches, 0);
        assert_eq!(stats.ambiguous_matches, 0);
        assert_eq!(stats.attached_receipts, 1);
        assert!(
            miner.checkpoint.buckets[0]
                .runtime_examples
                .contains_key(&legacy_evidence)
        );
        assert_eq!(miner.status().future_receipts_unique_total, 0);
        assert_eq!(miner.status().wrong_accepts_total, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn semantic_program_pool_survives_field_renames_and_collects_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-outcome-multi-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 32,
            max_receipts_per_bucket: 16,
        };
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        for index in 1..=4 {
            miner
                .observe(multi_output_observation(index))
                .expect("support");
        }
        for index in 5..=8 {
            miner
                .observe(multi_output_observation(index))
                .expect("future");
        }
        let package = miner
            .quarantine_packages()
            .expect("packages")
            .into_iter()
            .find(|package| program_any(&package.program, is_multi_output_project))
            .expect("portable multi-output package");
        assert_eq!(package.proof.support_rows, 4);
        assert_eq!(package.proof.future_rows, 4);
        assert_eq!(package.proof.wrong_accepts, 0);
        assert!(package.proof.distinct_surfaces >= 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn frozen_wave_center_uses_support_invariants_across_new_request_wording() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-invariant-wave-center-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig::default();
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
        let support_wordings = [
            "Count records for this batch",
            "How many entries are present?",
            "Return the collection cardinality",
            "Determine the row total",
            "Summarize the number of items",
            "Report how large this list is",
            "Give the amount of matching objects",
            "Calculate the payload size in rows",
        ];
        for index in 1..=32 {
            miner
                .observe_replay_training_buffered(routed_observation(
                    index,
                    "Total records: 3",
                    support_wordings[(index - 1) % support_wordings.len()],
                    index.is_multiple_of(2),
                ))
                .expect("support");
        }
        miner.flush().expect("flush support");
        let bucket = miner
            .checkpoint
            .buckets
            .iter()
            .find(|bucket| bucket.frozen_program_sha256.is_some())
            .expect("frozen bucket");
        let mut expected_center = bucket_program_atom_ids(bucket);
        expected_center.extend(bucket.common_request_atom_ids.iter().copied());
        assert_eq!(
            bucket_phase_center_atom_ids(bucket)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected_center
        );

        let future = routed_observation(
            10_000,
            "Total records: 3",
            "Provide the cardinality using this unseen wording",
            true,
        );
        let frozen_program = bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|digest| bucket.programs.get(digest))
            .expect("frozen program");
        assert_eq!(
            independently_verified_authority_response(frozen_program, &future.example).as_deref(),
            Some(future.example.expected_response.as_str()),
            "frozen program did not transfer: {frozen_program:#?}"
        );
        miner.observe(future).expect("future");
        let status = miner.status();
        assert_eq!(status.future_receipts_unique_total, 1, "{status:#?}");
        assert_eq!(status.frozen_future_accepted_total, 1);
        assert!(status.frozen_route_accounting_complete);
        assert_eq!(status.wrong_accepts_total, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn frozen_consensus_counterexample_creates_witness_successor() {
        let example = observation(1, "3");
        let witness_example = observation(10, "3");
        let version_space =
            enumerate_source_neutral_response_programs(&example.example).expect("version space");
        let correct = version_space
            .programs
            .into_iter()
            .find(|program| {
                program_any(program, is_count_operation)
                    && independently_verified_authority_response(program, &example.example)
                        .as_deref()
                        == Some("3")
                    && independently_verified_authority_response(program, &witness_example.example)
                        .as_deref()
                        == Some("3")
            })
            .expect("count program");
        let mut competing = correct.clone();
        let crate::ResponseOperation::ComposeCollection { renderer, .. } = &mut competing.operation
        else {
            panic!("count operation");
        };
        *renderer = crate::CollectionOutputRenderer::RenderTemplate {
            prefix: "(".to_owned(),
            suffix: ")".to_owned(),
        };
        let correct_digest = canonical_json_sha256(&correct).expect("correct digest");
        let competing_digest = canonical_json_sha256(&competing).expect("competing digest");
        let consensus = ResponseProgram::unique_consensus(vec![
            ResponseConsensusVariant {
                program: correct,
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            },
            ResponseConsensusVariant {
                program: competing,
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            },
        ]);
        let consensus_digest = canonical_json_sha256(&consensus).expect("consensus digest");
        assert!(independently_verified_authority_response(&consensus, &example.example).is_none());
        let support_programs = BTreeMap::from([(
            correct_digest.clone(),
            match &consensus.operation {
                crate::ResponseOperation::UniqueConsensus { variants, .. } => {
                    variants[0].program.clone()
                }
                _ => unreachable!(),
            },
        )]);
        let support = (1..=4)
            .map(|index| {
                receipt_with_program_atoms(&observation(index, "3"), true, &support_programs)
                    .expect("support receipt")
            })
            .collect::<Vec<_>>();
        let bucket = OnlineCollectionBucket {
            bucket_id: "a".repeat(64),
            archetype_id: "count".to_owned(),
            programs: BTreeMap::from([(consensus_digest.clone(), consensus)]),
            common_request_atom_ids: BTreeSet::new(),
            support,
            future: Vec::new(),
            runtime_examples: BTreeMap::new(),
            durable_adapter_phase_atoms: BTreeMap::new(),
            durable_runtime_parity_receipts: BTreeMap::new(),
            frozen_program_sha256: Some(consensus_digest.clone()),
            support_watermark_event_time_unix_nanos: Some(4),
            support_manifest_sha256: Some("b".repeat(64)),
            rejected_program_sha256: BTreeSet::new(),
            learned_anti_atom_ids: BTreeSet::new(),
            wrong_accepts: 0,
        };
        let decision = active_witness_decision(&bucket, &consensus_digest, &witness_example, 16)
            .expect("witness decision");
        let ActiveWitnessDecision::Successor {
            bucket: successor,
            resolved,
        } = decision
        else {
            panic!("witness successor");
        };
        assert!(resolved);
        assert_eq!(successor.programs.len(), 1);
        assert!(successor.programs.contains_key(&correct_digest));
        assert!(!successor.programs.contains_key(&competing_digest));
        let witness = successor.support.last().expect("witness receipt");
        assert_eq!(witness.witness_round, Some(1));
        assert_eq!(witness.witness_candidates_before, Some(2));
        assert_eq!(witness.witness_candidates_after, Some(1));
        assert!(valid_witness_receipt_metadata(witness));
    }

    #[test]
    fn frozen_future_accepts_independently_verified_canonical_response() {
        assert_eq!(authority_rejection_reason(&Ok("3".to_owned())), None);
        assert_eq!(
            authority_rejection_reason(&Err("actor_abstain")),
            Some("actor_abstain")
        );
        assert!(!is_hard_teacher_counterexample("authority_mismatch"));
        assert!(!is_hard_teacher_counterexample("actor_abstain"));
        assert!(is_hard_teacher_counterexample("verifier_rejected"));
    }

    #[test]
    fn semantic_count_inside_teacher_prose_reaches_external_admission() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-semantic-count-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let checkpoint_path = root.join("checkpoint.cbor");
        let config = OnlineCollectionConfig::default();
        let mut miner = OnlineCollectionMiner::open(&checkpoint_path, config).expect("miner");
        for index in 1..=32 {
            miner
                .observe(routed_observation(
                    index,
                    "Total records: 3",
                    "Count records for this batch",
                    index.is_multiple_of(2),
                ))
                .expect("semantic count observation");
        }
        miner
            .observe(routed_observation(
                10_000,
                "Rows: 3",
                "Summarize the selected payload",
                false,
            ))
            .expect("competing deterministic family");
        for index in 33..=64 {
            miner
                .observe(routed_observation(
                    index,
                    "Total records: 3",
                    "Count records for this batch",
                    index.is_multiple_of(2),
                ))
                .expect("semantic count future");
        }
        let package = miner
            .quarantine_packages()
            .expect("packages")
            .into_iter()
            .find(|package| program_any(&package.program, is_count_operation))
            .expect("semantic count package");
        assert_eq!(package.proof.support_rows, 32);
        assert_eq!(package.proof.future_rows, 32);
        assert_eq!(package.proof.wrong_accepts, 0);
        let heldout = routed_observation(
            100,
            "Total records: 3",
            "Count records for this batch",
            true,
        );
        let execution = execute_response(&package.program, "", &heldout.example.provider_payload);
        assert_eq!(execution.response.as_deref(), Some("3"));
        assert!(
            verify_response_independently(
                package.verifier.as_ref().expect("verifier"),
                &heldout.example.provider_payload,
                "3"
            )
            .is_ok()
        );
        drop(miner);
        let miner = OnlineCollectionMiner::open(&checkpoint_path, config).expect("restart miner");
        let candidates = miner.admission_candidates().expect("candidates");
        let candidate = candidates
            .into_iter()
            .find(|candidate| program_any(&candidate.package.program, is_count_operation))
            .unwrap_or_else(|| {
                let diagnostics = miner
                    .checkpoint
                    .buckets
                    .iter()
                    .enumerate()
                    .filter_map(|(index, bucket)| {
                        let mut package = miner.package_for_bucket(index, bucket, false).ok()??;
                        let causal = miner.collection_causal_report(bucket, &package).ok()?;
                        package.state = ResponsePackageState::Active;
                        package.proof.wave_causal_pass = causal.verdict == "PASS";
                        package.wave_margin_micro = causal.wave_margin_micro;
                        Some((causal, package.admission_candidate_blocker()))
                    })
                    .collect::<Vec<_>>();
                panic!(
                    "semantic count admission candidate: {:#?}\ndiagnostics={diagnostics:#?}",
                    miner.status()
                )
            });
        assert_eq!(candidate.future_receipts.len(), 32);
        assert_eq!(candidate.runtime_parity_cases.len(), 0);
        assert_eq!(candidate.durable_runtime_parity_receipts.len(), 32);
        let mut tampered = candidate.clone();
        tampered.durable_runtime_parity_receipts[0].input_sha256 = "f".repeat(64);
        assert!(
            crate::build_online_collection_admission_snapshot(
                &[tampered],
                "project",
                1,
                100,
                60,
                &"a".repeat(64),
                &"b".repeat(64),
            )
            .expect("tampered admission")
            .is_none()
        );
        let snapshot = crate::build_online_collection_admission_snapshot(
            &[candidate],
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission")
        .expect("authorized snapshot");
        assert_eq!(snapshot.registry.packages.len(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn multi_output_semantic_program_reaches_external_admission() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-outcome-admission-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let mut miner = OnlineCollectionMiner::open(
            root.join("checkpoint.cbor"),
            OnlineCollectionConfig::default(),
        )
        .expect("miner");
        for index in 1..=32 {
            miner
                .observe(multi_output_observation(index))
                .expect("support");
        }
        let mut competing = multi_output_observation(10_000);
        competing.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String("Emit an alternate verified template".to_owned());
        competing.example.expected_response = "Result: 10040; status: ready-10000.".to_owned();
        miner.observe(competing).expect("competing family");
        for index in 33..=64 {
            miner
                .observe(multi_output_observation(index))
                .expect("future");
        }
        let status = miner.status();
        assert_eq!(
            status.pooling_strategy_version,
            ONLINE_COLLECTION_POOLING_STRATEGY_V35
        );
        assert!(
            status
                .exact_executable_observations_total
                .saturating_add(status.semantic_executable_observations_total)
                >= 32,
            "{status:#?}"
        );
        assert!(status.program_pool_receipts_total >= 32);
        assert!(status.frozen_buckets_total >= 1, "{status:#?}");
        assert!(status.pre_admission_ready_buckets_total >= 1, "{status:#?}");
        assert!(status.support_receipts_unique_total >= 32);
        assert!(status.future_receipts_unique_total >= 32);
        assert_eq!(status.wrong_accepts_total, 0);
        assert!(status.runtime_parity_cases_total >= 32);
        assert!(
            status
                .frozen_program_kinds
                .get("project")
                .copied()
                .unwrap_or(0)
                >= 1
        );
        let candidates = miner.admission_candidates().expect("candidates");
        let causal_reports = miner
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                miner
                    .package_for_bucket(index, bucket, false)
                    .ok()
                    .flatten()
                    .and_then(|package| miner.collection_causal_report(bucket, &package).ok())
            })
            .collect::<Vec<_>>();
        let candidate_blockers = miner
            .checkpoint
            .buckets
            .iter()
            .enumerate()
            .filter_map(|(index, bucket)| {
                let mut package = miner.package_for_bucket(index, bucket, false).ok()??;
                let causal = miner.collection_causal_report(bucket, &package).ok()?;
                package.state = ResponsePackageState::Active;
                package.proof.wave_causal_pass = causal.verdict == "PASS";
                package.wave_margin_micro = causal.wave_margin_micro;
                Some(package.admission_candidate_blocker())
            })
            .collect::<Vec<_>>();
        let candidate = candidates
            .iter()
            .find(|candidate| program_any(&candidate.package.program, is_multi_output_project))
            .unwrap_or_else(|| {
                panic!(
                    "admission-ready multi-output candidate: {:#?}\ncausal={causal_reports:#?}\nblockers={candidate_blockers:#?}",
                    miner.status()
                )
            });
        assert_eq!(candidate.causal_report.verdict, "PASS");
        assert_eq!(candidate.future_receipts.len(), 32);
        assert_eq!(candidate.runtime_parity_cases.len(), 0);
        assert_eq!(candidate.durable_runtime_parity_receipts.len(), 32);
        let snapshot = crate::build_online_collection_admission_snapshot(
            std::slice::from_ref(candidate),
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission")
        .expect("authorized snapshot");
        assert_eq!(snapshot.registry.packages.len(), 1);
        assert!(snapshot.admission.eligible_for_local_accept);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn counterexample_learns_anti_center_then_revokes_only_when_unseparable() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-counterexample-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let path = root.join("checkpoint.json");
        let config = OnlineCollectionConfig {
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        };
        let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
        for index in 1..=4 {
            miner.observe(observation(index, "3")).expect("support");
        }
        miner
            .observe(observation(5, "not-three"))
            .expect("counterexample");
        let status = miner.status();
        assert_eq!(status.counterexamples_total, 1);
        assert_eq!(status.revoked_candidates_total, 0);
        assert_eq!(status.buckets.len(), 1);
        assert!(status.buckets[0].frozen);
        assert_eq!(status.buckets[0].wrong_accepts, 0);
        assert!(status.buckets[0].learned_anti_atoms > 0);

        let mut unseparable = observation(6, "not-three");
        unseparable.example.provider_payload["input"][0]["content"][0]["text"] =
            Value::String("Count records for batch 1".to_owned());
        miner.observe(unseparable).expect("unseparable");
        let status = miner.status();
        assert_eq!(status.revoked_candidates_total, 1);
        assert!(!status.buckets[0].frozen);
        assert_eq!(status.buckets[0].wrong_accepts, 1);
        assert_eq!(status.buckets[0].rejected_programs, 1);
        assert!(miner.quarantine_packages().expect("packages").is_empty());
        drop(miner);
        OnlineCollectionMiner::open(&path, config).expect("restart");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn collection_wave_ablation_builds_external_admission_after_frozen_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-online-collection-admission-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let config = OnlineCollectionConfig::default();
        let mut miner =
            OnlineCollectionMiner::open(root.join("checkpoint.json"), config).expect("miner");
        for index in 1..=32 {
            miner
                .observe(routed_observation(
                    index,
                    "3",
                    "Count records in the verified collection",
                    false,
                ))
                .expect("support");
        }
        miner
            .observe(routed_observation(
                10_000,
                "Rows: 3",
                "Summarize the selected payload",
                false,
            ))
            .expect("competing family");
        for index in 33..=64 {
            miner
                .observe(routed_observation(
                    index,
                    "3",
                    "Count records in the verified collection",
                    index % 2 == 0,
                ))
                .expect("future");
        }
        let candidates = miner.admission_candidates().expect("candidates");
        assert_eq!(candidates.len(), 1, "{:#?}", miner.status());
        let candidate = &candidates[0];
        assert_eq!(candidate.causal_report.verdict, "PASS");
        assert_eq!(candidate.causal_report.full_phase_correct, 32);
        assert!(
            candidate.causal_report.full_phase_exact_checks
                < candidate.causal_report.no_phase_exact_checks
        );
        assert_eq!(candidate.package.state, ResponsePackageState::Active);
        let snapshot = crate::build_online_collection_admission_snapshot(
            &candidates,
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission")
        .expect("authorized snapshot");
        assert_eq!(snapshot.registry.packages.len(), 1);
        assert!(snapshot.admission.eligible_for_local_accept);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn applicability_anti_atom_requires_three_distinct_sessions() {
        let mut evidence = BTreeMap::new();
        let candidates = BTreeSet::from([7, 9]);

        assert!(
            update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-a")
                .is_empty()
        );
        assert!(
            update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-a")
                .is_empty()
        );
        assert!(
            update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-b")
                .is_empty()
        );
        assert_eq!(
            update_applicability_negative_sessions(&mut evidence, candidates, "session-c"),
            BTreeSet::from([7, 9])
        );
    }
}
