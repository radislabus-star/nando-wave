use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CegisReport, CegisWinner, FamilyDiscoveryReport, FrozenGeneration, OpportunityBoardReport,
    RuntimeParityCase, SemanticAliasEdge, TeacherPoolSnapshot,
};

pub const SELF_TRAINING_STATE_SCHEMA_V2: &str = "nando.self-training-stream-state.v2";
pub const SELF_TRAINING_STATE_SCHEMA_V3: &str = "nando.self-training-stream-state.v3";
pub const SELF_TRAINING_STATE_SCHEMA_V4: &str = "nando.self-training-stream-state.v4";
pub const SELF_TRAINING_STATE_SCHEMA_V5: &str = "nando.self-training-stream-state.v5";
pub const SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1: &str = "nando.semantic-evidence-receipt.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEvidenceOutcome {
    VerifiedEquivalent,
    ApplicabilityNegative,
    HardContradiction,
    CensoredUnknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticEvidenceReceipt {
    pub schema: String,
    pub generation_id_sha256: String,
    pub cohort_id_sha256: String,
    pub winner_program_sha256: String,
    pub frame_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub outcome: SemanticEvidenceOutcome,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingAdmissionCohort {
    pub winner: CegisWinner,
    pub physical_members: Vec<CegisWinner>,
    pub generation: FrozenGeneration,
    pub pool: TeacherPoolSnapshot,
    pub semantic_evidence_receipts: Vec<SemanticEvidenceReceipt>,
    pub runtime_parity_cases: Vec<RuntimeParityCase>,
    pub semantic_alias_edges: Vec<SemanticAliasEdge>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingGenerationReport {
    pub partition_version: u32,
    #[serde(default)]
    pub generation_id_sha256: String,
    pub cohort_id_sha256: String,
    pub teacher_signature_sha256: String,
    #[serde(default)]
    pub physical_adapter_count: usize,
    #[serde(default)]
    pub physical_adapter_signatures: Vec<String>,
    pub generation: u64,
    #[serde(default)]
    pub support_watermark_unix_nanos: u64,
    pub support_rows: usize,
    #[serde(default)]
    pub support_sessions: usize,
    pub support_tokens: u64,
    pub future_rows: usize,
    pub future_tokens: u64,
    pub future_sessions: usize,
    pub surfaces: usize,
    pub wrong_future_rows: usize,
    #[serde(default)]
    pub support_runtime_parity_rows: usize,
    #[serde(default)]
    pub support_runtime_parity_tokens: u64,
    #[serde(default)]
    pub matching_runtime_parity_rows: usize,
    #[serde(default)]
    pub matching_runtime_parity_sessions: usize,
    #[serde(default)]
    pub post_repair_runtime_parity_rows: usize,
    #[serde(default)]
    pub post_repair_runtime_parity_sessions: usize,
    #[serde(default)]
    pub live_runtime_parity_rows: usize,
    #[serde(default)]
    pub after_future_watermark_rows: usize,
    #[serde(default)]
    pub support_frame_rejects: usize,
    #[serde(default)]
    pub support_session_rejects: usize,
    #[serde(default)]
    pub support_intent_rejects: usize,
    #[serde(default)]
    pub support_event_rejects: usize,
    #[serde(default)]
    pub independent_future_rows: usize,
    #[serde(default)]
    pub program_mismatch_rejects: usize,
    #[serde(default)]
    pub program_consistent_future_rows: usize,
    #[serde(default)]
    pub route_mismatch_rejects: usize,
    #[serde(default)]
    pub routed_future_rows: usize,
    pub runtime_parity_rows: usize,
    pub runtime_parity_tokens: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinerSignalStageReport {
    pub stage: String,
    pub verdict: String,
    pub score_out_of_10: u8,
    pub rows: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinerSignalTreeReport {
    pub overall_score_out_of_10: u8,
    pub stages: Vec<MinerSignalStageReport>,
    pub top_blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingStateReport {
    pub schema: String,
    pub transitions_seen: u64,
    pub work_slices_completed: u64,
    pub exact_checks_completed: u64,
    #[serde(default)]
    pub runtime_parity_cases_total: usize,
    #[serde(default)]
    pub runtime_parity_frames_total: usize,
    #[serde(default)]
    pub replay_support_parity_cases_total: usize,
    #[serde(default)]
    pub replay_support_parity_frames_total: usize,
    #[serde(default)]
    pub parity_discovery_key_overlap: usize,
    #[serde(default)]
    pub parity_accepted_frame_rows: usize,
    #[serde(default)]
    pub parity_signature_match_rows: usize,
    #[serde(default)]
    pub parity_rows_by_teacher_signature: BTreeMap<String, usize>,
    #[serde(default)]
    pub semantic_law_cohorts: usize,
    #[serde(default)]
    pub semantic_law_physical_adapters: usize,
    #[serde(default)]
    pub semantic_law_blockers: BTreeMap<String, usize>,
    pub discovery: FamilyDiscoveryReport,
    pub cegis: CegisReport,
    pub opportunity: OpportunityBoardReport,
    pub generations: Vec<SelfTrainingGenerationReport>,
    pub admission_ready_cohorts: usize,
    pub signal_tree: MinerSignalTreeReport,
}
