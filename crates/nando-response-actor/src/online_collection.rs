use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nando_core::wave::{phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids};
use nando_operator_learning::fit_adapter_wave_route;
pub use nando_operator_learning::{
    LegacyReplayRehydrationStats, OnlineCollectionBucketStatus, OnlineCollectionConfig,
    OnlineCollectionConsensusDiagnostic, OnlineCollectionObservation, OnlineCollectionProofMode,
    OnlineCollectionReceipt, OnlineCollectionRehydrationHint, OnlineCollectionStatus,
    OnlineCollectionWaveCausalReport,
};

use crate::collection_synthesis::{
    canonical_direct_response_program, enumerate_source_neutral_response_programs_with_coverage,
    enumerate_source_neutral_structural_response_programs, response_law_key,
};
use crate::{
    AstProgramKind, CollectionSynthesisExample, DurableRuntimeParityReceipt,
    LEGACY_CONTROL_FUTURE_ROWS, LEGACY_CONTROL_MIN_SESSIONS, LEGACY_CONTROL_MIN_SURFACES,
    LEGACY_CONTROL_SUPPORT_ROWS, ResponseAdapterWaveConsensus, ResponseConsensusVariant,
    ResponseExecutionStatus, ResponsePackage, ResponsePackageOrigin, ResponsePackageProof,
    ResponsePackageState, ResponseProgram, build_durable_runtime_parity_receipt,
    canonical_json_sha256, diagnose_response_dynamic_coverage,
    enumerate_source_neutral_response_programs, execute_response,
    is_learned_bounded_response_program, is_privacy_safe_online_response_program,
    is_source_neutral_response_program, is_transfer_bound_response_program,
    package::{
        request_phase_atom_ids, response_pre_action_context_atom_ids,
        response_program_external_verifier_schema,
    },
    response_program_authority_matches_example, response_program_dynamic_value_root_sha256,
    response_program_exactly_matches_example, response_program_kind,
    response_program_required_routing_atom_ids, response_program_requires_static_frame_transfer,
    sha256_bytes, source_neutral_verifier_for_program, verify_response_independently,
};

mod admission;
mod authority;
mod identification;
mod ingest;
mod migration;
mod natural_artifacts;
mod status;
mod subcenter;

// These bridges are visible only to sibling owner modules; none extends the crate API.
use authority::*;
use identification::*;
use migration::*;
use status::*;
use subcenter::*;

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
const ONLINE_COLLECTION_POOLING_STRATEGY_V36: u32 = 36;
const ONLINE_COLLECTION_POOLING_STRATEGY_V37: u32 = 37;
const ONLINE_COLLECTION_POOLING_STRATEGY_V38: u32 = 38;
const ONLINE_COLLECTION_POOLING_STRATEGY_V39: u32 = 39;
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V2: &[u8; 4] = b"NCO2";
const ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3: &[u8; 4] = b"NCO3";
const MAX_PERSISTED_PARITY_BYTES_PER_BUCKET: usize = 2 * 1024 * 1024;
// The live compact checkpoint is currently about 175 MiB; keep restart bounded
// while leaving enough headroom for one full generation rollover.
const MAX_COLLECTION_CHECKPOINT_BYTES: u64 = 256 * 1024 * 1024;
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
    #[serde(default)]
    pub archetype_id: String,
    #[serde(default)]
    pub identification_programs: Vec<ResponseProgram>,
    #[serde(default)]
    pub candidate_freeze: Option<nando_operator_learning::CandidateFreezeReceiptV1>,
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
    #[serde(default)]
    adaptive_candidate_freeze: Option<nando_operator_learning::CandidateFreezeReceiptV1>,
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
    _owner_lock: Option<File>,
}

pub struct OnlineCollectionReadSnapshot {
    miner: OnlineCollectionMiner,
}

impl OnlineCollectionMiner {
    #[must_use]
    pub fn read_snapshot(&self) -> OnlineCollectionReadSnapshot {
        OnlineCollectionReadSnapshot {
            miner: Self {
                path: PathBuf::new(),
                checkpoint: self.checkpoint.clone(),
                _owner_lock: None,
            },
        }
    }
}

impl OnlineCollectionReadSnapshot {
    #[must_use]
    pub fn status(&self) -> OnlineCollectionStatus {
        self.miner.status()
    }

    pub fn quarantine_packages(&self) -> Result<Vec<ResponsePackage>, String> {
        self.miner.quarantine_packages()
    }

    pub fn admission_candidates(&self) -> Result<Vec<OnlineCollectionAdmissionCandidate>, String> {
        self.miner.admission_candidates()
    }

    pub fn natural_t1_program_artifacts(
        &self,
    ) -> Result<Vec<nando_operator_learning::multi_source::NaturalT1ProgramArtifactV1>, String>
    {
        natural_artifacts::natural_t1_program_artifacts(&self.miner.checkpoint)
    }

    #[must_use]
    pub fn consensus_diagnostics(&self) -> Vec<OnlineCollectionConsensusDiagnostic> {
        self.miner.consensus_diagnostics()
    }

    #[must_use]
    pub fn consensus_diagnostic_for_bucket(
        &self,
        bucket_id: &str,
    ) -> Option<OnlineCollectionConsensusDiagnostic> {
        self.miner.consensus_diagnostic_for_bucket(bucket_id)
    }
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

pub fn online_collection_adaptive_transfer_proof_digest(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, String> {
    adaptive_transfer_proof_root(
        &candidate.future_manifest_sha256,
        &candidate.program_sha256,
        &candidate.package.program,
        &candidate.support_receipts,
        &candidate.future_receipts,
        &candidate.durable_runtime_parity_receipts,
    )
}

pub fn online_collection_candidate_freeze(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<Option<nando_operator_learning::CandidateFreezeReceiptV1>, String> {
    identify_collection_candidate(candidate)
        .map(|identification| identification.map(|identified| identified.freeze))
}

#[cfg(test)]
#[path = "online_collection_tests.rs"]
mod tests;
