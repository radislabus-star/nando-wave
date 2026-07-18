//! Flat CPU runtime for phase-center operator scoring.
//!
//! This module intentionally contains no corpus loading, no lookup table of
//! answers, and no training loop. It scores a candidate transition against
//! precompiled positive/negative phase centers.

pub const PHASE_CENTER_RUNTIME_PACKAGE_MAGIC: [u8; 8] = *b"NWPCF001";
pub const PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL: [u8; 8] = *b"nwpcpkg1";
pub const PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES: usize = 16;
pub const PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC: [u8; 8] = *b"NWPCH001";
pub const PHASE_CENTER_HOT_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL: [u8; 8] = *b"nwpchot1";
pub const PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES: usize = 56;
const PHASE_CENTER_ONLINE_CHECKPOINT_MAGIC: [u8; 8] = *b"NWPCO001";
pub const PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO: i64 = 300_000;
const PHASE_CENTER_DEFAULT_HOT_ATOM_ROW_CACHE: usize = 64;
const PHASE_CENTER_FIXED_CELL_SCALE: i64 = 16_384;
const PHASE_CENTER_ONLINE_DECAY_INTERVAL: usize = 256;
const PHASE_CENTER_ONLINE_DECAY_FACTOR: f64 = 0.875;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhaseCenterCell {
    pub re: f64,
    pub im: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterFlatRecord {
    pub positive_center: Box<[PhaseCenterCell]>,
    pub negative_center: Box<[PhaseCenterCell]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterFlatRuntime {
    cells: usize,
    records: Box<[PhaseCenterFlatRecord]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOffloadRuntime {
    runtime: PhaseCenterFlatRuntime,
    policy: PhaseCenterOffloadPolicy,
    package_info: PhaseCenterRuntimePackageInfo,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotRuntime {
    cells: usize,
    profiles: Box<[PhaseCenterHotProfile]>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotProfile {
    pub profile_id: u32,
    pub threshold_micro: i64,
    pub center_delta: Box<[PhaseCenterCell]>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotDecision {
    pub profile_id: u32,
    pub margin_micro: i64,
    pub local_operator: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotCandidateDecision {
    pub profile_id: u32,
    pub margin_micro: i64,
    pub score_candidate: bool,
    pub verifier_required: bool,
    pub local_accept: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotRequest<'a> {
    pub route_index: usize,
    pub atom_ids: &'a [u64],
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseCenterPreparedHotRequest<'a> {
    pub route_index: usize,
    pub phase_vector: &'a [PhaseCenterCell],
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotEvidenceRequest<'a> {
    pub request: PhaseCenterHotRequest<'a>,
    pub evidence: PhaseCenterHotRequestEvidence,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseCenterPreparedHotEvidenceRequest<'a> {
    pub request: PhaseCenterPreparedHotRequest<'a>,
    pub evidence: PhaseCenterHotRequestEvidence,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterPreparedHotEvidenceRow {
    pub route_index: usize,
    pub atom_ids: Vec<u64>,
    pub phase_vector: Vec<PhaseCenterCell>,
    pub verified_safe_accept: bool,
    pub exact_cache_hit: bool,
    pub tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterHotRequestEvidence {
    pub verified_safe_accept: bool,
    pub exact_cache_hit: bool,
    pub tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterHotShadowEval {
    pub score_events: usize,
    pub score_candidate_events: usize,
    pub verifier_required_events: usize,
    pub local_accept_events: usize,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub false_accepts: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterPreparedHotDenominator {
    pub total_tokens: u64,
    pub total_cost_microusd: u64,
    pub exact_cache_hits: usize,
    pub exact_cache_tokens: u64,
    pub exact_cache_cost_microusd: u64,
    pub non_exact_rows: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotScratch {
    encoder: PhaseCenterAtomEncoder,
    candidates: Vec<PhaseCenterHotCandidateDecision>,
    scores: Vec<f64>,
    atom_rows: Vec<PhaseCenterHotAtomRow>,
    atom_row_indexes: Vec<usize>,
    max_cached_atom_rows: usize,
    next_atom_row_evict: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotWorker {
    runtime: PhaseCenterHotRuntime,
    routes: PhaseCenterHotRouteTable,
    scratch: PhaseCenterHotScratch,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotRowPreparer {
    encoder: PhaseCenterAtomEncoder,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
struct PhaseCenterHotAtomRow {
    atom_id: u64,
    cells: Box<[PhaseCenterCell]>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterLocalAcceptEvidence {
    pub candidate: PhaseCenterHotCandidateDecision,
    pub verifier_passed: bool,
    pub promotion: PhaseCenterPromotionEvidence,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterLocalAcceptDecision {
    pub local_accept: bool,
    pub blocker: Option<PhaseCenterLocalAcceptBlocker>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterLocalAcceptBlocker {
    CandidateAlreadyClaimsLocalAccept,
    ScoreNotCandidate,
    VerifierRequired,
    PromotionBlocked(PhaseCenterPromotionBlocker),
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotRoutePlan {
    route_id: u32,
    profile_indexes: Box<[usize]>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotRouteTable {
    plans: Box<[PhaseCenterHotRoutePlan]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterAtomEncoder {
    cells: usize,
    scratch: Vec<PhaseCenterCell>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOnlineMinerConfig {
    pub cells: usize,
    pub min_bucket_events: usize,
    pub threshold_floor_micro: i64,
    pub calibration_events: usize,
    pub max_buckets: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOnlineMiner {
    config: PhaseCenterOnlineMinerConfig,
    buckets: Vec<PhaseCenterOnlineBucket>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterLiveOperatorStoreConfig {
    pub miner: PhaseCenterOnlineMinerConfig,
    pub memory: PhaseCenterOperatorMemoryConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterLiveOperatorStore {
    miner: PhaseCenterOnlineMiner,
    memory_config: PhaseCenterOperatorMemoryConfig,
    routes: Vec<PhaseCenterLiveRouteStats>,
    route_buckets: Vec<PhaseCenterLiveRouteBucket>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterLiveOperatorAtomEvent<'a> {
    pub route_id: u32,
    pub bucket_id: u32,
    pub atom_ids: &'a [u64],
    pub verified_safe_accept: bool,
    pub exact_cache_hit: bool,
    pub tokens: u64,
    pub cost_microusd: u64,
}

impl<'a> PhaseCenterLiveOperatorAtomEvent<'a> {
    #[must_use]
    pub const fn new(
        route_id: u32,
        bucket_id: u32,
        atom_ids: &'a [u64],
        evidence: PhaseCenterHotRequestEvidence,
    ) -> Self {
        Self {
            route_id,
            bucket_id,
            atom_ids,
            verified_safe_accept: evidence.verified_safe_accept,
            exact_cache_hit: evidence.exact_cache_hit,
            tokens: evidence.tokens,
            cost_microusd: evidence.cost_microusd,
        }
    }

    #[must_use]
    pub const fn evidence(self) -> PhaseCenterHotRequestEvidence {
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: self.verified_safe_accept,
            exact_cache_hit: self.exact_cache_hit,
            tokens: self.tokens,
            cost_microusd: self.cost_microusd,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterLiveRouteStats {
    pub route_id: u32,
    pub route_bucket_count: usize,
    pub events_seen: usize,
    pub scored_events: usize,
    pub local_operator_shadow_decisions: usize,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub false_accepts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhaseCenterLiveRouteBucket {
    route_id: u32,
    bucket_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseCenterOnlineEvent<'a> {
    pub bucket_id: u32,
    pub vector: &'a [PhaseCenterCell],
    pub verified_safe_accept: bool,
    pub exact_cache_hit: bool,
    pub tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOnlineBucket {
    pub bucket_id: u32,
    positive_sum: Vec<PhaseCenterCell>,
    negative_sum: Vec<PhaseCenterCell>,
    pub positive_events: usize,
    pub negative_events: usize,
    pub events_seen: usize,
    pub scored_events: usize,
    pub calibration_events_seen: usize,
    pub learned_threshold_micro: i64,
    pub max_calibration_false_margin_micro: Option<i64>,
    pub local_operator_shadow_decisions: usize,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub false_accepts: usize,
    pub rejected: bool,
    pub trust_quality_micro: i64,
    pub trust_false_risk_micro: i64,
    pub trust_drift_micro: i64,
    pub trust_token_value_micro: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterOnlineDecision {
    pub bucket_id: u32,
    pub active_before_update: bool,
    pub calibration_event: bool,
    pub margin_micro: i64,
    pub threshold_micro: i64,
    pub raw_local_operator: bool,
    pub local_operator_shadow_decision: bool,
    pub false_accept: bool,
    pub unique_cpu_accept_over_exact_cache: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterOnlineSummary {
    pub bucket_count: usize,
    pub active_bucket_count: usize,
    pub shadow_ready_bucket_count: usize,
    pub candidate_bucket_count: usize,
    pub rejected_bucket_count: usize,
    pub scored_events: usize,
    pub local_operator_shadow_decisions: usize,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub false_accepts: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOnlineCandidatePackage {
    pub bucket_id: u32,
    pub threshold_micro: i64,
    pub verifier_binding: PhaseCenterVerifierBinding,
    pub package_info: PhaseCenterRuntimePackageInfo,
    pub package_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterVerifierBinding {
    pub verifier_id: u32,
    pub verifier_version: u32,
    pub verifier_input_kind_id: u32,
    pub verifier_evidence_source_id: u32,
    pub false_accept_threshold: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterThresholdPolicyEvidence {
    pub candidate_bucket_count: usize,
    pub auto_calibrated_bucket_count: usize,
    pub calibration_window_before_shadow: bool,
    pub shadow_window_after_calibration: bool,
    pub per_bucket_thresholds_reported: bool,
    pub fixed_policy_shadow_replay: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterPromotionEvidence {
    pub future_shadow_events: usize,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub false_accepts: usize,
    pub runtime_margin_parity_mismatches: usize,
    pub verifier_binding: PhaseCenterVerifierBinding,
    pub threshold_policy: PhaseCenterThresholdPolicyEvidence,
    pub exact_cache_overlap_excluded: bool,
    pub token_cost_denominator_present: bool,
    pub local_accept_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterPromotionDecision {
    pub eligible: bool,
    pub blocker: Option<PhaseCenterPromotionBlocker>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterPromotionBlocker {
    NoFutureShadowEvents,
    NoUniqueAcceptsOverExactCache,
    MissingTokenSavings,
    MissingCostSavings,
    FalseAccepts,
    RuntimeParityMismatch,
    MissingVerifierBinding,
    MissingAutomaticThresholdCalibration,
    MissingCalibrationWindowBeforeShadow,
    MissingShadowWindowAfterCalibration,
    MissingPerBucketThresholdReport,
    MissingFixedThresholdPolicy,
    ExactCacheOverlapNotExcluded,
    MissingTokenCostDenominator,
    LocalAcceptAlreadyEnabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOperatorMemoryConfig {
    pub max_hot_profiles_per_worker: usize,
    pub max_hot_bytes_per_worker: usize,
    pub max_warm_profiles_per_process: usize,
    pub max_profiles_per_route: usize,
    pub max_route_top_k: usize,
    pub min_tokens_saved: u64,
    pub min_accept_rate_milli: u16,
    pub false_accepts_must_be_zero: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOperatorMemory {
    config: PhaseCenterOperatorMemoryConfig,
    routes: Vec<PhaseCenterOperatorRoute>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterRuntimeBudgetSnapshot {
    pub max_hot_profiles_per_worker: usize,
    pub max_hot_bytes_per_worker: usize,
    pub max_warm_profiles_per_process: usize,
    pub max_profiles_per_route: usize,
    pub max_route_top_k: usize,
    pub warm_route_count: usize,
    pub warm_profile_count: usize,
    pub warm_metadata_bytes_estimate: usize,
    pub warm_runtime_bytes_estimate: usize,
    pub warm_bytes_estimate: usize,
    pub hot_route_count: usize,
    pub hot_profile_count: usize,
    pub hot_route_profile_edges: usize,
    pub hot_runtime_bytes_estimate: usize,
    pub hot_route_table_bytes_estimate: usize,
    pub hot_bytes_estimate: usize,
    pub warm_profile_budget_passed: bool,
    pub hot_profile_budget_passed: bool,
    pub hot_byte_budget_passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOperatorAdmission {
    pub route_id: u32,
    pub profile_id: u32,
    pub evidence: PhaseCenterPromotionEvidence,
    pub runtime_bytes_estimate: usize,
    pub last_seen_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOperatorAdmissionDecision {
    pub admitted: bool,
    pub blocker: Option<PhaseCenterOperatorAdmissionBlocker>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterOperatorAdmissionBlocker {
    InvalidBudget,
    PromotionBlocked(PhaseCenterPromotionBlocker),
    BelowMinTokensSaved,
    BelowMinAcceptRate,
    FalseAccepts,
    EvictedByWarmBudget,
    EvictedByRouteBudget,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterSavingsDenominator {
    pub total_calls: usize,
    pub total_tokens: u64,
    pub total_cost_microusd: u64,
    pub exact_cache_hits: usize,
    pub exact_cache_tokens_saved: u64,
    pub exact_cache_cost_saved_microusd: u64,
    pub synthetic_trace_used: bool,
    pub provider_billing_evidence_present: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterSavingsEvidence {
    pub denominator: PhaseCenterSavingsDenominator,
    pub nando_unique_accepts_over_exact_cache: usize,
    pub nando_tokens_saved: u64,
    pub nando_cost_saved_microusd: u64,
    pub false_accepts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterSavingsReport {
    pub market_money_claim_allowed: bool,
    pub blocker: Option<PhaseCenterSavingsBlocker>,
    pub exact_cache_calls_saved_milli: usize,
    pub nando_calls_saved_milli: usize,
    pub combined_calls_saved_milli: usize,
    pub exact_cache_tokens_saved_milli: usize,
    pub nando_tokens_saved_milli: usize,
    pub combined_tokens_saved_milli: usize,
    pub exact_cache_cost_saved_milli: usize,
    pub nando_cost_saved_milli: usize,
    pub combined_cost_saved_milli: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterSavingsBlocker {
    EmptyCallDenominator,
    MissingTokenDenominator,
    MissingCostDenominator,
    SyntheticTrace,
    MissingProviderBillingEvidence,
    FalseAccepts,
    NoUniqueAcceptsOverExactCache,
    MissingNandoTokenSavings,
    MissingNandoCostSavings,
    ExactCacheHitsExceedTotalCalls,
    CombinedCallsExceedTotalCalls,
    CombinedTokensExceedTotalTokens,
    CombinedCostExceedTotalCost,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterOperatorRoute {
    pub route_id: u32,
    profiles: Vec<PhaseCenterOperatorProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOperatorProfile {
    pub profile_id: u32,
    pub value_score: u64,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub tokens_saved: u64,
    pub cost_saved_microusd: u64,
    pub runtime_bytes_estimate: usize,
    pub last_seen_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterRuntimePackageInfo {
    pub magic: [u8; PHASE_CENTER_RUNTIME_PACKAGE_MAGIC.len()],
    pub cells: usize,
    pub record_count: usize,
    pub serialized_len: usize,
    pub payload_bytes: usize,
    pub fingerprint64: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotPackagePolicyDefaults {
    pub local_accept_enabled: bool,
    pub require_verifier: bool,
    pub require_false_accepts_zero: bool,
    pub shadow_only: bool,
    pub min_margin_threshold_micro: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterHotRuntimePackageInfo {
    pub magic: [u8; PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC.len()],
    pub cells: usize,
    pub profile_count: usize,
    pub route_count: usize,
    pub route_profile_edges: usize,
    pub serialized_len: usize,
    pub payload_bytes: usize,
    pub fingerprint64: u64,
    pub verifier_binding: PhaseCenterVerifierBinding,
    pub policy_defaults: PhaseCenterHotPackagePolicyDefaults,
    pub hot_runtime_bytes_estimate: usize,
    pub hot_route_table_bytes_estimate: usize,
    pub hot_scratch_bytes_estimate: usize,
    pub hot_bytes_estimate: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterHotRuntimePackage {
    pub info: PhaseCenterHotRuntimePackageInfo,
    pub hot_runtime: PhaseCenterHotRuntime,
    pub route_table: PhaseCenterHotRouteTable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterCompiler {
    cells: usize,
    positive_sums: Vec<Vec<PhaseCenterCell>>,
    negative_sums: Vec<Vec<PhaseCenterCell>>,
    positive_counts: Vec<usize>,
    negative_counts: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCenterEvalTask {
    pub center_index: usize,
    pub correct_vec: Box<[PhaseCenterCell]>,
    pub wrong_vec: Box<[PhaseCenterCell]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterOffloadAction {
    LocalOperator,
    FallbackToLlm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOffloadPolicy {
    pub margin_threshold_micro: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseCenterOffloadDecision {
    pub action: PhaseCenterOffloadAction,
    pub margin_micro: i64,
    pub margin_threshold_micro: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseCenterOffloadSummary {
    pub calls: usize,
    pub local_operator_calls: usize,
    pub fallback_to_llm_calls: usize,
    pub offload_rate_milli: usize,
    pub local_accuracy_milli: usize,
    pub false_local_accepts: usize,
    pub median_margin_micro: i64,
    pub p10_margin_micro: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCenterRuntimeError {
    EmptyRuntime,
    RecordWidthMismatch,
    CenterIndexOutOfBounds,
    ProgramIndexOutOfBounds,
    IncompleteProgram,
    VectorWidthMismatch,
    RuntimePackageTooLarge,
    InvalidRuntimePackage,
    InvalidOffloadThreshold,
    InvalidMargin,
}

impl PhaseCenterOffloadPolicy {
    pub fn new(margin_threshold_micro: i64) -> Result<Self, PhaseCenterRuntimeError> {
        if margin_threshold_micro <= 0 {
            return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
        }
        Ok(Self {
            margin_threshold_micro,
        })
    }

    pub fn default_conservative() -> Self {
        Self {
            margin_threshold_micro: PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO,
        }
    }

    pub fn decide_margin(
        self,
        margin: f64,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin_micro = phase_margin_to_micro(margin)?;
        let action = if margin_micro >= self.margin_threshold_micro {
            PhaseCenterOffloadAction::LocalOperator
        } else {
            PhaseCenterOffloadAction::FallbackToLlm
        };
        Ok(PhaseCenterOffloadDecision {
            action,
            margin_micro,
            margin_threshold_micro: self.margin_threshold_micro,
        })
    }
}

impl Default for PhaseCenterOffloadPolicy {
    fn default() -> Self {
        Self::default_conservative()
    }
}

impl PhaseCenterOffloadRuntime {
    pub fn inspect_package_bytes(
        bytes: &[u8],
    ) -> Result<PhaseCenterRuntimePackageInfo, PhaseCenterRuntimeError> {
        PhaseCenterFlatRuntime::inspect_bytes(bytes)
    }

    pub fn from_package_bytes(
        bytes: &[u8],
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        let package_info = Self::inspect_package_bytes(bytes)?;
        let runtime = PhaseCenterFlatRuntime::from_bytes(bytes)?;
        Ok(Self {
            runtime,
            policy,
            package_info,
        })
    }

    #[must_use]
    pub const fn policy(&self) -> PhaseCenterOffloadPolicy {
        self.policy
    }

    #[must_use]
    pub const fn package_info(&self) -> PhaseCenterRuntimePackageInfo {
        self.package_info
    }

    #[must_use]
    pub const fn runtime(&self) -> &PhaseCenterFlatRuntime {
        &self.runtime
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.runtime.cells()
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.runtime.record_count()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.runtime.bytes_estimate()
    }

    pub fn offload_decision(
        &self,
        task: &PhaseCenterEvalTask,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        self.runtime.offload_decision(task, self.policy)
    }

    pub fn offload_decisions_into<'a, I>(
        &self,
        tasks: I,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.runtime.offload_decisions_into(tasks, self.policy, out)
    }

    pub fn offload_summary_into<'a, I>(
        &self,
        tasks: I,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.runtime
            .offload_summary_into(tasks, self.policy, decision_scratch, margin_scratch)
    }

    pub fn offload_summary_for_into<'a, I>(
        &self,
        tasks: I,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        self.runtime
            .offload_summary_for_into(tasks, self.policy, decision_scratch, margin_scratch)
    }
}

impl Default for PhaseCenterHotPackagePolicyDefaults {
    fn default() -> Self {
        Self {
            local_accept_enabled: false,
            require_verifier: true,
            require_false_accepts_zero: true,
            shadow_only: true,
            min_margin_threshold_micro: PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO,
        }
    }
}

impl PhaseCenterHotPackagePolicyDefaults {
    const LOCAL_ACCEPT_ENABLED: u32 = 1 << 0;
    const REQUIRE_VERIFIER: u32 = 1 << 1;
    const REQUIRE_FALSE_ACCEPTS_ZERO: u32 = 1 << 2;
    const SHADOW_ONLY: u32 = 1 << 3;

    #[must_use]
    const fn to_flags(self) -> u32 {
        (if self.local_accept_enabled {
            Self::LOCAL_ACCEPT_ENABLED
        } else {
            0
        }) | (if self.require_verifier {
            Self::REQUIRE_VERIFIER
        } else {
            0
        }) | (if self.require_false_accepts_zero {
            Self::REQUIRE_FALSE_ACCEPTS_ZERO
        } else {
            0
        }) | (if self.shadow_only {
            Self::SHADOW_ONLY
        } else {
            0
        })
    }

    #[must_use]
    const fn from_flags(flags: u32, min_margin_threshold_micro: i64) -> Self {
        Self {
            local_accept_enabled: flags & Self::LOCAL_ACCEPT_ENABLED != 0,
            require_verifier: flags & Self::REQUIRE_VERIFIER != 0,
            require_false_accepts_zero: flags & Self::REQUIRE_FALSE_ACCEPTS_ZERO != 0,
            shadow_only: flags & Self::SHADOW_ONLY != 0,
            min_margin_threshold_micro,
        }
    }
}

impl PhaseCenterHotRuntimePackage {
    pub fn from_runtime(
        hot_runtime: PhaseCenterHotRuntime,
        route_table: PhaseCenterHotRouteTable,
        verifier_binding: PhaseCenterVerifierBinding,
        policy_defaults: PhaseCenterHotPackagePolicyDefaults,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        let info = hot_runtime_package_info_for_runtime(
            &hot_runtime,
            &route_table,
            verifier_binding,
            policy_defaults,
            0,
            0,
        )?;
        Ok(Self {
            info,
            hot_runtime,
            route_table,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, PhaseCenterRuntimeError> {
        let cells = u32::try_from(self.hot_runtime.cells())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let profile_count = u32::try_from(self.hot_runtime.profile_count())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let route_count = u32::try_from(self.route_table.route_count())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let route_profile_edges = u32::try_from(self.route_table.profile_edge_count())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let false_accept_threshold =
            u32::try_from(self.info.verifier_binding.false_accept_threshold)
                .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let serialized_len = hot_runtime_package_len(
            self.hot_runtime.cells(),
            self.hot_runtime.profile_count(),
            self.route_table.route_count(),
            self.route_table.profile_edge_count(),
        )
        .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;

        let mut bytes = Vec::with_capacity(serialized_len);
        bytes.extend_from_slice(&PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC);
        bytes.extend_from_slice(&cells.to_le_bytes());
        bytes.extend_from_slice(&profile_count.to_le_bytes());
        bytes.extend_from_slice(&route_count.to_le_bytes());
        bytes.extend_from_slice(&route_profile_edges.to_le_bytes());
        bytes.extend_from_slice(&self.info.verifier_binding.verifier_id.to_le_bytes());
        bytes.extend_from_slice(&self.info.verifier_binding.verifier_version.to_le_bytes());
        bytes.extend_from_slice(
            &self
                .info
                .verifier_binding
                .verifier_input_kind_id
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &self
                .info
                .verifier_binding
                .verifier_evidence_source_id
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&false_accept_threshold.to_le_bytes());
        bytes.extend_from_slice(&self.info.policy_defaults.to_flags().to_le_bytes());
        bytes.extend_from_slice(
            &self
                .info
                .policy_defaults
                .min_margin_threshold_micro
                .to_le_bytes(),
        );
        for profile in self.hot_runtime.profiles.iter() {
            bytes.extend_from_slice(&profile.profile_id.to_le_bytes());
            bytes.extend_from_slice(&profile.threshold_micro.to_le_bytes());
            write_phase_center_cells(&mut bytes, &profile.center_delta);
        }
        for plan in self.route_table.plans.iter() {
            let edge_count = u32::try_from(plan.profile_count())
                .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
            bytes.extend_from_slice(&plan.route_id().to_le_bytes());
            bytes.extend_from_slice(&edge_count.to_le_bytes());
            for &profile_index in plan.profile_indexes() {
                let profile_index_u32 = u32::try_from(profile_index)
                    .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
                bytes.extend_from_slice(&profile_index_u32.to_le_bytes());
            }
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PhaseCenterRuntimeError> {
        let info = Self::inspect_bytes(bytes)?;
        let mut offset = PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES;
        let mut profiles = Vec::with_capacity(info.profile_count);
        for _ in 0..info.profile_count {
            let profile_id = read_u32_le_at(bytes, &mut offset)?;
            let threshold_micro = read_i64_le_at(bytes, &mut offset)?;
            if threshold_micro <= 0 {
                return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
            }
            let center_delta = read_phase_center_cells(bytes, &mut offset, info.cells)?;
            profiles.push(PhaseCenterHotProfile {
                profile_id,
                threshold_micro,
                center_delta,
            });
        }
        let hot_runtime = PhaseCenterHotRuntime {
            cells: info.cells,
            profiles: profiles.into_boxed_slice(),
        };

        let mut plans = Vec::with_capacity(info.route_count);
        let mut observed_edges = 0usize;
        for _ in 0..info.route_count {
            let route_id = read_u32_le_at(bytes, &mut offset)?;
            let edge_count = read_u32_le_at(bytes, &mut offset)? as usize;
            observed_edges = observed_edges.saturating_add(edge_count);
            let mut profile_indexes = Vec::with_capacity(edge_count);
            for _ in 0..edge_count {
                let profile_index = read_u32_le_at(bytes, &mut offset)? as usize;
                if profile_index >= hot_runtime.profile_count() {
                    return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
                }
                profile_indexes.push(profile_index);
            }
            if let Some(plan) = PhaseCenterHotRoutePlan::new(route_id, profile_indexes)? {
                plans.push(plan);
            }
        }
        if observed_edges != info.route_profile_edges || offset != bytes.len() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let route_table = PhaseCenterHotRouteTable::from_plans(plans)?;
        if route_table.route_count() != info.route_count {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        Ok(Self {
            info,
            hot_runtime,
            route_table,
        })
    }

    pub fn inspect_bytes(
        bytes: &[u8],
    ) -> Result<PhaseCenterHotRuntimePackageInfo, PhaseCenterRuntimeError> {
        if bytes.len() < PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        if bytes[..PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC.len()]
            != PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC
        {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let cells = read_u32_le(bytes, 8)? as usize;
        let profile_count = read_u32_le(bytes, 12)? as usize;
        let route_count = read_u32_le(bytes, 16)? as usize;
        let route_profile_edges = read_u32_le(bytes, 20)? as usize;
        if cells == 0 || profile_count == 0 || route_count == 0 || route_profile_edges == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        let verifier_binding = PhaseCenterVerifierBinding {
            verifier_id: read_u32_le(bytes, 24)?,
            verifier_version: read_u32_le(bytes, 28)?,
            verifier_input_kind_id: read_u32_le(bytes, 32)?,
            verifier_evidence_source_id: read_u32_le(bytes, 36)?,
            false_accept_threshold: read_u32_le(bytes, 40)? as usize,
        };
        let policy_flags = read_u32_le(bytes, 44)?;
        let min_margin_threshold_micro = read_i64_le(bytes, 48)?;
        if min_margin_threshold_micro <= 0 {
            return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
        }
        let serialized_len =
            hot_runtime_package_len(cells, profile_count, route_count, route_profile_edges)
                .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        if bytes.len() != serialized_len {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        hot_runtime_package_info(
            cells,
            profile_count,
            route_count,
            route_profile_edges,
            verifier_binding,
            PhaseCenterHotPackagePolicyDefaults::from_flags(
                policy_flags,
                min_margin_threshold_micro,
            ),
            serialized_len,
            hot_runtime_package_fingerprint64(bytes),
        )
    }

    pub fn into_worker(self) -> Result<PhaseCenterHotWorker, PhaseCenterRuntimeError> {
        PhaseCenterHotWorker::new(self.hot_runtime, self.route_table)
    }
}

impl PhaseCenterHotRuntimePackageInfo {
    #[must_use]
    pub fn server_policy_allows_local_accept(self) -> bool {
        self.policy_defaults.local_accept_enabled
            && self.policy_defaults.require_verifier
            && self.policy_defaults.require_false_accepts_zero
            && self.verifier_binding.is_bound()
            && self.verifier_binding.false_accept_threshold == 0
    }
}

impl PhaseCenterOffloadDecision {
    #[must_use]
    pub const fn is_local_operator(self) -> bool {
        matches!(self.action, PhaseCenterOffloadAction::LocalOperator)
    }

    #[must_use]
    pub const fn is_fallback_to_llm(self) -> bool {
        matches!(self.action, PhaseCenterOffloadAction::FallbackToLlm)
    }

    #[must_use]
    pub const fn is_false_local_accept(self) -> bool {
        self.is_local_operator() && self.margin_micro <= 0
    }
}

impl PhaseCenterHotDecision {
    #[must_use]
    pub const fn to_candidate_decision(self) -> PhaseCenterHotCandidateDecision {
        PhaseCenterHotCandidateDecision {
            profile_id: self.profile_id,
            margin_micro: self.margin_micro,
            score_candidate: self.local_operator,
            verifier_required: self.local_operator,
            local_accept: false,
        }
    }
}

impl<'a> PhaseCenterHotRequest<'a> {
    #[must_use]
    pub const fn new(route_index: usize, atom_ids: &'a [u64]) -> Self {
        Self {
            route_index,
            atom_ids,
        }
    }
}

impl<'a> PhaseCenterPreparedHotRequest<'a> {
    #[must_use]
    pub const fn new(route_index: usize, phase_vector: &'a [PhaseCenterCell]) -> Self {
        Self {
            route_index,
            phase_vector,
        }
    }
}

impl PhaseCenterHotScratch {
    pub fn new(cells: usize, candidate_capacity: usize) -> Result<Self, PhaseCenterRuntimeError> {
        Self::with_atom_cache_capacity(
            cells,
            candidate_capacity,
            PHASE_CENTER_DEFAULT_HOT_ATOM_ROW_CACHE,
        )
    }

    pub fn with_atom_cache_capacity(
        cells: usize,
        candidate_capacity: usize,
        atom_cache_capacity: usize,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        if atom_cache_capacity == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        Ok(Self {
            encoder: PhaseCenterAtomEncoder::new(cells)?,
            candidates: Vec::with_capacity(candidate_capacity),
            scores: Vec::with_capacity(candidate_capacity),
            atom_rows: Vec::with_capacity(atom_cache_capacity),
            atom_row_indexes: Vec::with_capacity(atom_cache_capacity),
            max_cached_atom_rows: atom_cache_capacity,
            next_atom_row_evict: 0,
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.encoder.cells()
    }

    #[must_use]
    pub fn encoder_scratch_capacity(&self) -> usize {
        self.encoder.scratch_capacity()
    }

    #[must_use]
    pub fn candidate_capacity(&self) -> usize {
        self.candidates.capacity()
    }

    #[must_use]
    pub fn score_capacity(&self) -> usize {
        self.scores.capacity()
    }

    #[must_use]
    pub const fn atom_cache_capacity(&self) -> usize {
        self.max_cached_atom_rows
    }

    #[must_use]
    pub fn cached_atom_rows(&self) -> usize {
        self.atom_rows.len()
    }

    #[must_use]
    pub fn candidates(&self) -> &[PhaseCenterHotCandidateDecision] {
        &self.candidates
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.encoder.scratch_capacity() * std::mem::size_of::<PhaseCenterCell>()
            + self.candidates.capacity() * std::mem::size_of::<PhaseCenterHotCandidateDecision>()
            + self.scores.capacity() * std::mem::size_of::<f64>()
            + self.atom_row_indexes.capacity() * std::mem::size_of::<usize>()
            + self.atom_rows.capacity()
                * (std::mem::size_of::<PhaseCenterHotAtomRow>()
                    + self.cells() * std::mem::size_of::<PhaseCenterCell>())
    }

    #[must_use]
    pub const fn bytes_estimate_for(
        cells: usize,
        candidate_capacity: usize,
        atom_cache_capacity: usize,
    ) -> usize {
        std::mem::size_of::<Self>()
            + cells * std::mem::size_of::<PhaseCenterCell>()
            + candidate_capacity * std::mem::size_of::<PhaseCenterHotCandidateDecision>()
            + candidate_capacity * std::mem::size_of::<f64>()
            + atom_cache_capacity * std::mem::size_of::<usize>()
            + atom_cache_capacity
                * (std::mem::size_of::<PhaseCenterHotAtomRow>()
                    + cells * std::mem::size_of::<PhaseCenterCell>())
    }

    fn ensure_atom_row(
        &mut self,
        atom_id: u64,
        cells: usize,
    ) -> Result<usize, PhaseCenterRuntimeError> {
        if let Some(index) = self.atom_rows.iter().position(|row| row.atom_id == atom_id) {
            return Ok(index);
        }
        let row = PhaseCenterHotAtomRow {
            atom_id,
            cells: (0..cells)
                .map(|cell| stable_phase_atom_id_cell(atom_id, cell))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };
        if self.atom_rows.len() < self.max_cached_atom_rows {
            self.atom_rows.push(row);
            return Ok(self.atom_rows.len() - 1);
        }
        let index = self.next_atom_row_evict;
        self.atom_rows[index] = row;
        self.next_atom_row_evict = (self.next_atom_row_evict + 1) % self.max_cached_atom_rows;
        Ok(index)
    }
}

impl PhaseCenterLocalAcceptEvidence {
    #[must_use]
    pub const fn evaluate(self) -> PhaseCenterLocalAcceptDecision {
        if self.candidate.local_accept {
            return PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::CandidateAlreadyClaimsLocalAccept,
            );
        }
        if !self.candidate.score_candidate {
            return PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::ScoreNotCandidate,
            );
        }
        if self.candidate.verifier_required && !self.verifier_passed {
            return PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::VerifierRequired,
            );
        }
        let promotion = self.promotion.evaluate();
        if let Some(blocker) = promotion.blocker {
            return PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::PromotionBlocked(blocker),
            );
        }
        PhaseCenterLocalAcceptDecision {
            local_accept: true,
            blocker: None,
        }
    }
}

impl PhaseCenterLocalAcceptDecision {
    #[must_use]
    pub const fn blocked(blocker: PhaseCenterLocalAcceptBlocker) -> Self {
        Self {
            local_accept: false,
            blocker: Some(blocker),
        }
    }
}

impl PhaseCenterHotRowPreparer {
    pub fn new(cells: usize) -> Result<Self, PhaseCenterRuntimeError> {
        Ok(Self {
            encoder: PhaseCenterAtomEncoder::new(cells)?,
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.encoder.cells()
    }

    #[must_use]
    pub fn scratch_capacity(&self) -> usize {
        self.encoder.scratch_capacity()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.encoder.scratch_capacity() * std::mem::size_of::<PhaseCenterCell>()
    }

    pub fn prepare_atom_ids<I>(
        &mut self,
        route_index: usize,
        atom_ids: I,
        evidence: PhaseCenterHotRequestEvidence,
    ) -> Result<PhaseCenterPreparedHotEvidenceRow, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        let atom_ids = atom_ids.into_iter().collect::<Vec<_>>();
        let phase_vector = self
            .encoder
            .encode_atom_ids(atom_ids.iter().copied())?
            .to_vec();
        Ok(PhaseCenterPreparedHotEvidenceRow::new(
            route_index,
            atom_ids,
            phase_vector,
            evidence,
        ))
    }

    pub fn prepare_live_atom_event(
        &mut self,
        routes: &PhaseCenterHotRouteTable,
        event: PhaseCenterLiveOperatorAtomEvent<'_>,
    ) -> Result<Option<PhaseCenterPreparedHotEvidenceRow>, PhaseCenterRuntimeError> {
        let Some(route_index) = routes.resolve_route_index(event.route_id) else {
            return Ok(None);
        };
        self.prepare_atom_ids(
            route_index,
            event.atom_ids.iter().copied(),
            event.evidence(),
        )
        .map(Some)
    }
}

impl PhaseCenterHotShadowEval {
    pub fn observe_candidate_decisions(
        &mut self,
        evidence: PhaseCenterHotRequestEvidence,
        decisions: &[PhaseCenterHotCandidateDecision],
    ) {
        self.score_events += 1;
        for decision in decisions {
            if !decision.score_candidate {
                continue;
            }
            self.score_candidate_events += 1;
            self.verifier_required_events += usize::from(decision.verifier_required);
            self.local_accept_events += usize::from(decision.local_accept);
            if evidence.verified_safe_accept {
                if !evidence.exact_cache_hit {
                    self.unique_cpu_accepts_over_exact_cache += 1;
                    self.tokens_saved = self.tokens_saved.saturating_add(evidence.tokens);
                    self.cost_saved_microusd = self
                        .cost_saved_microusd
                        .saturating_add(evidence.cost_microusd);
                }
            } else {
                self.false_accepts += 1;
            }
        }
    }
}

impl<'a> PhaseCenterHotEvidenceRequest<'a> {
    #[must_use]
    pub const fn new(
        route_index: usize,
        atom_ids: &'a [u64],
        evidence: PhaseCenterHotRequestEvidence,
    ) -> Self {
        Self {
            request: PhaseCenterHotRequest::new(route_index, atom_ids),
            evidence,
        }
    }
}

impl<'a> PhaseCenterPreparedHotEvidenceRequest<'a> {
    #[must_use]
    pub const fn new(
        route_index: usize,
        phase_vector: &'a [PhaseCenterCell],
        evidence: PhaseCenterHotRequestEvidence,
    ) -> Self {
        Self {
            request: PhaseCenterPreparedHotRequest::new(route_index, phase_vector),
            evidence,
        }
    }
}

impl PhaseCenterPreparedHotEvidenceRow {
    #[must_use]
    pub fn new(
        route_index: usize,
        atom_ids: Vec<u64>,
        phase_vector: Vec<PhaseCenterCell>,
        evidence: PhaseCenterHotRequestEvidence,
    ) -> Self {
        Self {
            route_index,
            atom_ids,
            phase_vector,
            verified_safe_accept: evidence.verified_safe_accept,
            exact_cache_hit: evidence.exact_cache_hit,
            tokens: evidence.tokens,
            cost_microusd: evidence.cost_microusd,
        }
    }

    #[must_use]
    pub const fn evidence(&self) -> PhaseCenterHotRequestEvidence {
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: self.verified_safe_accept,
            exact_cache_hit: self.exact_cache_hit,
            tokens: self.tokens,
            cost_microusd: self.cost_microusd,
        }
    }

    #[must_use]
    pub fn hot_evidence_request(&self) -> PhaseCenterHotEvidenceRequest<'_> {
        PhaseCenterHotEvidenceRequest::new(self.route_index, &self.atom_ids, self.evidence())
    }

    #[must_use]
    pub fn prepared_evidence_request(&self) -> PhaseCenterPreparedHotEvidenceRequest<'_> {
        PhaseCenterPreparedHotEvidenceRequest::new(
            self.route_index,
            &self.phase_vector,
            self.evidence(),
        )
    }
}

impl PhaseCenterPreparedHotDenominator {
    pub fn observe_evidence(&mut self, evidence: PhaseCenterHotRequestEvidence) {
        self.total_tokens = self.total_tokens.saturating_add(evidence.tokens);
        self.total_cost_microusd = self
            .total_cost_microusd
            .saturating_add(evidence.cost_microusd);
        if evidence.exact_cache_hit {
            self.exact_cache_hits += 1;
            self.exact_cache_tokens = self.exact_cache_tokens.saturating_add(evidence.tokens);
            self.exact_cache_cost_microusd = self
                .exact_cache_cost_microusd
                .saturating_add(evidence.cost_microusd);
        } else {
            self.non_exact_rows += 1;
        }
    }
}

impl PhaseCenterAtomEncoder {
    pub fn new(cells: usize) -> Result<Self, PhaseCenterRuntimeError> {
        if cells == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        Ok(Self {
            cells,
            scratch: vec![PhaseCenterCell::default(); cells],
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn scratch_capacity(&self) -> usize {
        self.scratch.capacity()
    }

    pub fn encode_atoms<'a, I>(
        &mut self,
        atoms: I,
    ) -> Result<&[PhaseCenterCell], PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        fill_phase_vector_from_atoms_into(atoms, self.cells, &mut self.scratch);
        Ok(&self.scratch)
    }

    pub fn encode_atom_ids<I>(
        &mut self,
        atom_ids: I,
    ) -> Result<&[PhaseCenterCell], PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        fill_phase_vector_from_atom_ids_into(atom_ids, self.cells, &mut self.scratch);
        Ok(&self.scratch)
    }
}

impl PhaseCenterOnlineMinerConfig {
    pub fn validate(self) -> Result<Self, PhaseCenterRuntimeError> {
        if self.cells == 0 || self.max_buckets == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        if self.min_bucket_events == 0 || self.calibration_events == 0 {
            return Err(PhaseCenterRuntimeError::IncompleteProgram);
        }
        if self.threshold_floor_micro <= 0 {
            return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
        }
        Ok(self)
    }
}

impl PhaseCenterLiveOperatorStoreConfig {
    pub fn validate(self) -> Result<Self, PhaseCenterRuntimeError> {
        Ok(Self {
            miner: self.miner.validate()?,
            memory: self.memory.validate()?,
        })
    }
}

impl PhaseCenterThresholdPolicyEvidence {
    #[must_use]
    pub const fn automatic_calibration_passed(self) -> bool {
        self.candidate_bucket_count > 0
            && self.auto_calibrated_bucket_count == self.candidate_bucket_count
    }

    #[must_use]
    pub const fn promotion_policy_passed(self) -> bool {
        self.automatic_calibration_passed()
            && self.calibration_window_before_shadow
            && self.shadow_window_after_calibration
            && self.per_bucket_thresholds_reported
            && self.fixed_policy_shadow_replay
    }
}

impl PhaseCenterVerifierBinding {
    #[must_use]
    pub const fn is_bound(self) -> bool {
        self.verifier_id != 0
            && self.verifier_version != 0
            && self.verifier_input_kind_id != 0
            && self.verifier_evidence_source_id != 0
            && self.false_accept_threshold == 0
    }
}

impl PhaseCenterPromotionEvidence {
    #[must_use]
    pub const fn from_online_summary(
        summary: PhaseCenterOnlineSummary,
        future_shadow_events: usize,
        runtime_margin_parity_mismatches: usize,
        exact_cache_overlap_excluded: bool,
        token_cost_denominator_present: bool,
        local_accept_enabled: bool,
    ) -> Self {
        Self {
            future_shadow_events,
            unique_cpu_accepts_over_exact_cache: summary.unique_cpu_accepts_over_exact_cache,
            tokens_saved: summary.tokens_saved,
            cost_saved_microusd: summary.cost_saved_microusd,
            false_accepts: summary.false_accepts,
            runtime_margin_parity_mismatches,
            verifier_binding: PhaseCenterVerifierBinding {
                verifier_id: 0,
                verifier_version: 0,
                verifier_input_kind_id: 0,
                verifier_evidence_source_id: 0,
                false_accept_threshold: 0,
            },
            threshold_policy: PhaseCenterThresholdPolicyEvidence {
                candidate_bucket_count: 0,
                auto_calibrated_bucket_count: 0,
                calibration_window_before_shadow: false,
                shadow_window_after_calibration: false,
                per_bucket_thresholds_reported: false,
                fixed_policy_shadow_replay: false,
            },
            exact_cache_overlap_excluded,
            token_cost_denominator_present,
            local_accept_enabled,
        }
    }

    #[must_use]
    pub const fn with_threshold_policy(
        mut self,
        threshold_policy: PhaseCenterThresholdPolicyEvidence,
    ) -> Self {
        self.threshold_policy = threshold_policy;
        self
    }

    #[must_use]
    pub const fn with_verifier_binding(
        mut self,
        verifier_binding: PhaseCenterVerifierBinding,
    ) -> Self {
        self.verifier_binding = verifier_binding;
        self
    }

    #[must_use]
    pub const fn evaluate(self) -> PhaseCenterPromotionDecision {
        if self.local_accept_enabled {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::LocalAcceptAlreadyEnabled,
            );
        }
        if self.false_accepts > 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::FalseAccepts,
            );
        }
        if self.runtime_margin_parity_mismatches > 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::RuntimeParityMismatch,
            );
        }
        if !self.verifier_binding.is_bound() {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingVerifierBinding,
            );
        }
        if !self.exact_cache_overlap_excluded {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::ExactCacheOverlapNotExcluded,
            );
        }
        if !self.token_cost_denominator_present {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingTokenCostDenominator,
            );
        }
        if !self.threshold_policy.automatic_calibration_passed() {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration,
            );
        }
        if !self.threshold_policy.calibration_window_before_shadow {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingCalibrationWindowBeforeShadow,
            );
        }
        if !self.threshold_policy.shadow_window_after_calibration {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingShadowWindowAfterCalibration,
            );
        }
        if !self.threshold_policy.per_bucket_thresholds_reported {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingPerBucketThresholdReport,
            );
        }
        if !self.threshold_policy.fixed_policy_shadow_replay {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingFixedThresholdPolicy,
            );
        }
        if self.future_shadow_events == 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::NoFutureShadowEvents,
            );
        }
        if self.unique_cpu_accepts_over_exact_cache == 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::NoUniqueAcceptsOverExactCache,
            );
        }
        if self.tokens_saved == 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingTokenSavings,
            );
        }
        if self.cost_saved_microusd == 0 {
            return PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingCostSavings,
            );
        }
        PhaseCenterPromotionDecision {
            eligible: true,
            blocker: None,
        }
    }
}

impl PhaseCenterPromotionDecision {
    #[must_use]
    pub const fn blocked(blocker: PhaseCenterPromotionBlocker) -> Self {
        Self {
            eligible: false,
            blocker: Some(blocker),
        }
    }
}

impl PhaseCenterOperatorMemoryConfig {
    pub fn validate(self) -> Result<Self, PhaseCenterRuntimeError> {
        if self.max_hot_profiles_per_worker == 0
            || self.max_hot_bytes_per_worker == 0
            || self.max_warm_profiles_per_process == 0
            || self.max_profiles_per_route == 0
            || self.max_route_top_k == 0
        {
            return Err(PhaseCenterRuntimeError::IncompleteProgram);
        }
        if self.max_route_top_k > self.max_profiles_per_route
            || self.max_profiles_per_route > self.max_warm_profiles_per_process
        {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        }
        if self.min_accept_rate_milli > 1000 {
            return Err(PhaseCenterRuntimeError::InvalidMargin);
        }
        Ok(self)
    }
}

impl PhaseCenterOperatorMemory {
    pub fn new(config: PhaseCenterOperatorMemoryConfig) -> Result<Self, PhaseCenterRuntimeError> {
        Ok(Self {
            config: config.validate()?,
            routes: Vec::new(),
        })
    }

    #[must_use]
    pub const fn config(&self) -> PhaseCenterOperatorMemoryConfig {
        self.config
    }

    #[must_use]
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    #[must_use]
    pub fn warm_profile_count(&self) -> usize {
        self.routes
            .iter()
            .map(|route| route.profiles.len())
            .sum::<usize>()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.routes.len() * std::mem::size_of::<PhaseCenterOperatorRoute>()
            + self.warm_profile_count() * std::mem::size_of::<PhaseCenterOperatorProfile>()
    }

    #[must_use]
    pub fn warm_runtime_bytes_estimate(&self) -> usize {
        self.routes
            .iter()
            .flat_map(|route| route.profiles.iter())
            .map(|profile| profile.runtime_bytes_estimate)
            .sum()
    }

    #[must_use]
    pub fn runtime_budget_snapshot(
        &self,
        hot_runtime: &PhaseCenterHotRuntime,
        hot_routes: &PhaseCenterHotRouteTable,
    ) -> PhaseCenterRuntimeBudgetSnapshot {
        let warm_profile_count = self.warm_profile_count();
        let warm_metadata_bytes_estimate = self.bytes_estimate();
        let warm_runtime_bytes_estimate = self.warm_runtime_bytes_estimate();
        let hot_runtime_bytes_estimate = hot_runtime.bytes_estimate();
        let hot_route_table_bytes_estimate = hot_routes.bytes_estimate();
        let hot_bytes_estimate =
            hot_runtime_bytes_estimate.saturating_add(hot_route_table_bytes_estimate);
        PhaseCenterRuntimeBudgetSnapshot {
            max_hot_profiles_per_worker: self.config.max_hot_profiles_per_worker,
            max_hot_bytes_per_worker: self.config.max_hot_bytes_per_worker,
            max_warm_profiles_per_process: self.config.max_warm_profiles_per_process,
            max_profiles_per_route: self.config.max_profiles_per_route,
            max_route_top_k: self.config.max_route_top_k,
            warm_route_count: self.route_count(),
            warm_profile_count,
            warm_metadata_bytes_estimate,
            warm_runtime_bytes_estimate,
            warm_bytes_estimate: warm_metadata_bytes_estimate
                .saturating_add(warm_runtime_bytes_estimate),
            hot_route_count: hot_routes.route_count(),
            hot_profile_count: hot_runtime.profile_count(),
            hot_route_profile_edges: hot_routes.profile_edge_count(),
            hot_runtime_bytes_estimate,
            hot_route_table_bytes_estimate,
            hot_bytes_estimate,
            warm_profile_budget_passed: warm_profile_count
                <= self.config.max_warm_profiles_per_process,
            hot_profile_budget_passed: hot_runtime.profile_count()
                <= self.config.max_hot_profiles_per_worker,
            hot_byte_budget_passed: hot_bytes_estimate <= self.config.max_hot_bytes_per_worker,
        }
    }

    pub fn admit(
        &mut self,
        admission: PhaseCenterOperatorAdmission,
    ) -> PhaseCenterOperatorAdmissionDecision {
        if self.config.false_accepts_must_be_zero && admission.evidence.false_accepts > 0 {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::FalseAccepts,
            );
        }
        let promotion = admission.evidence.evaluate();
        if let Some(blocker) = promotion.blocker {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::PromotionBlocked(blocker),
            );
        }
        if admission.evidence.tokens_saved < self.config.min_tokens_saved {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::BelowMinTokensSaved,
            );
        }
        if accept_rate_milli(admission.evidence) < usize::from(self.config.min_accept_rate_milli) {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::BelowMinAcceptRate,
            );
        }

        let profile = PhaseCenterOperatorProfile {
            profile_id: admission.profile_id,
            value_score: operator_value_score(admission.evidence),
            unique_cpu_accepts_over_exact_cache: admission
                .evidence
                .unique_cpu_accepts_over_exact_cache,
            tokens_saved: admission.evidence.tokens_saved,
            cost_saved_microusd: admission.evidence.cost_saved_microusd,
            runtime_bytes_estimate: admission.runtime_bytes_estimate,
            last_seen_tick: admission.last_seen_tick,
        };
        let route_index = self.route_index_or_insert(admission.route_id);
        upsert_profile(
            &mut self.routes[route_index].profiles,
            profile,
            self.config.max_profiles_per_route,
        );
        if self.routes[route_index]
            .profile(admission.profile_id)
            .is_none()
        {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::EvictedByRouteBudget,
            );
        }
        self.evict_warm_over_budget();
        if self
            .route(admission.route_id)
            .and_then(|route| route.profile(admission.profile_id))
            .is_none()
        {
            return PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::EvictedByWarmBudget,
            );
        }
        PhaseCenterOperatorAdmissionDecision {
            admitted: true,
            blocker: None,
        }
    }

    #[must_use]
    pub fn route(&self, route_id: u32) -> Option<&PhaseCenterOperatorRoute> {
        self.routes
            .binary_search_by_key(&route_id, |route| route.route_id)
            .ok()
            .map(|index| &self.routes[index])
    }

    pub fn route_top_k_into(&self, route_id: u32, out: &mut Vec<PhaseCenterOperatorProfile>) {
        out.clear();
        let Some(route) = self.route(route_id) else {
            return;
        };
        out.extend(
            route
                .profiles
                .iter()
                .take(self.config.max_route_top_k)
                .copied(),
        );
    }

    pub fn hot_route_plan(
        &self,
        hot_runtime: &PhaseCenterHotRuntime,
        route_id: u32,
    ) -> Result<Option<PhaseCenterHotRoutePlan>, PhaseCenterRuntimeError> {
        let Some(route) = self.route(route_id) else {
            return Ok(None);
        };
        hot_runtime.route_plan_from_profile_ids(
            route_id,
            route
                .profiles
                .iter()
                .take(self.config.max_route_top_k)
                .map(|profile| profile.profile_id),
        )
    }

    pub fn hot_route_table(
        &self,
        hot_runtime: &PhaseCenterHotRuntime,
    ) -> Result<PhaseCenterHotRouteTable, PhaseCenterRuntimeError> {
        let mut plans = Vec::new();
        for route in &self.routes {
            if let Some(plan) = hot_runtime.route_plan_from_profile_ids(
                route.route_id,
                route
                    .profiles
                    .iter()
                    .take(self.config.max_route_top_k)
                    .map(|profile| profile.profile_id),
            )? {
                plans.push(plan);
            }
        }
        PhaseCenterHotRouteTable::from_plans(plans)
    }

    fn route_index_or_insert(&mut self, route_id: u32) -> usize {
        match self
            .routes
            .binary_search_by_key(&route_id, |route| route.route_id)
        {
            Ok(index) => index,
            Err(index) => {
                self.routes.insert(
                    index,
                    PhaseCenterOperatorRoute {
                        route_id,
                        profiles: Vec::new(),
                    },
                );
                index
            }
        }
    }

    fn evict_warm_over_budget(&mut self) {
        while self.warm_profile_count() > self.config.max_warm_profiles_per_process {
            let mut evict_route_index = None;
            let mut evict_profile_index = None;
            let mut evict_key = None;
            for (route_index, route) in self.routes.iter().enumerate() {
                for (profile_index, profile) in route.profiles.iter().enumerate() {
                    let key = (profile.value_score, profile.last_seen_tick);
                    if evict_key.is_none_or(|current| key < current) {
                        evict_key = Some(key);
                        evict_route_index = Some(route_index);
                        evict_profile_index = Some(profile_index);
                    }
                }
            }
            let (Some(route_index), Some(profile_index)) = (evict_route_index, evict_profile_index)
            else {
                break;
            };
            self.routes[route_index].profiles.remove(profile_index);
            if self.routes[route_index].profiles.is_empty() {
                self.routes.remove(route_index);
            }
        }
    }
}

impl PhaseCenterOperatorAdmissionDecision {
    #[must_use]
    pub const fn blocked(blocker: PhaseCenterOperatorAdmissionBlocker) -> Self {
        Self {
            admitted: false,
            blocker: Some(blocker),
        }
    }
}

impl PhaseCenterRuntimeBudgetSnapshot {
    #[must_use]
    pub const fn hot_budget_passed(self) -> bool {
        self.hot_profile_budget_passed && self.hot_byte_budget_passed
    }

    #[must_use]
    pub const fn warm_budget_passed(self) -> bool {
        self.warm_profile_budget_passed
    }

    #[must_use]
    pub const fn product_runtime_budget_passed(self) -> bool {
        self.hot_budget_passed() && self.warm_budget_passed()
    }
}

impl PhaseCenterSavingsEvidence {
    #[must_use]
    pub fn report(self) -> PhaseCenterSavingsReport {
        let denominator = self.denominator;
        let combined_calls = denominator
            .exact_cache_hits
            .saturating_add(self.nando_unique_accepts_over_exact_cache);
        let combined_tokens = denominator
            .exact_cache_tokens_saved
            .saturating_add(self.nando_tokens_saved);
        let combined_cost = denominator
            .exact_cache_cost_saved_microusd
            .saturating_add(self.nando_cost_saved_microusd);

        let blocker = if denominator.total_calls == 0 {
            Some(PhaseCenterSavingsBlocker::EmptyCallDenominator)
        } else if denominator.total_tokens == 0 {
            Some(PhaseCenterSavingsBlocker::MissingTokenDenominator)
        } else if denominator.total_cost_microusd == 0 {
            Some(PhaseCenterSavingsBlocker::MissingCostDenominator)
        } else if denominator.synthetic_trace_used {
            Some(PhaseCenterSavingsBlocker::SyntheticTrace)
        } else if !denominator.provider_billing_evidence_present {
            Some(PhaseCenterSavingsBlocker::MissingProviderBillingEvidence)
        } else if self.false_accepts > 0 {
            Some(PhaseCenterSavingsBlocker::FalseAccepts)
        } else if self.nando_unique_accepts_over_exact_cache == 0 {
            Some(PhaseCenterSavingsBlocker::NoUniqueAcceptsOverExactCache)
        } else if self.nando_tokens_saved == 0 {
            Some(PhaseCenterSavingsBlocker::MissingNandoTokenSavings)
        } else if self.nando_cost_saved_microusd == 0 {
            Some(PhaseCenterSavingsBlocker::MissingNandoCostSavings)
        } else if denominator.exact_cache_hits > denominator.total_calls {
            Some(PhaseCenterSavingsBlocker::ExactCacheHitsExceedTotalCalls)
        } else if combined_calls > denominator.total_calls {
            Some(PhaseCenterSavingsBlocker::CombinedCallsExceedTotalCalls)
        } else if combined_tokens > denominator.total_tokens {
            Some(PhaseCenterSavingsBlocker::CombinedTokensExceedTotalTokens)
        } else if combined_cost > denominator.total_cost_microusd {
            Some(PhaseCenterSavingsBlocker::CombinedCostExceedTotalCost)
        } else {
            None
        };

        PhaseCenterSavingsReport {
            market_money_claim_allowed: blocker.is_none(),
            blocker,
            exact_cache_calls_saved_milli: milli_ratio_usize(
                denominator.exact_cache_hits,
                denominator.total_calls,
            ),
            nando_calls_saved_milli: milli_ratio_usize(
                self.nando_unique_accepts_over_exact_cache,
                denominator.total_calls,
            ),
            combined_calls_saved_milli: milli_ratio_usize(combined_calls, denominator.total_calls),
            exact_cache_tokens_saved_milli: milli_ratio_u64(
                denominator.exact_cache_tokens_saved,
                denominator.total_tokens,
            ),
            nando_tokens_saved_milli: milli_ratio_u64(
                self.nando_tokens_saved,
                denominator.total_tokens,
            ),
            combined_tokens_saved_milli: milli_ratio_u64(combined_tokens, denominator.total_tokens),
            exact_cache_cost_saved_milli: milli_ratio_u64(
                denominator.exact_cache_cost_saved_microusd,
                denominator.total_cost_microusd,
            ),
            nando_cost_saved_milli: milli_ratio_u64(
                self.nando_cost_saved_microusd,
                denominator.total_cost_microusd,
            ),
            combined_cost_saved_milli: milli_ratio_u64(
                combined_cost,
                denominator.total_cost_microusd,
            ),
        }
    }
}

impl PhaseCenterOperatorRoute {
    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    #[must_use]
    pub fn profile(&self, profile_id: u32) -> Option<&PhaseCenterOperatorProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
    }
}

fn accept_rate_milli(evidence: PhaseCenterPromotionEvidence) -> usize {
    if evidence.future_shadow_events == 0 {
        return 0;
    }
    evidence
        .unique_cpu_accepts_over_exact_cache
        .saturating_mul(1000)
        / evidence.future_shadow_events
}

fn operator_value_score(evidence: PhaseCenterPromotionEvidence) -> u64 {
    evidence
        .tokens_saved
        .saturating_add(evidence.cost_saved_microusd)
        .saturating_add(evidence.unique_cpu_accepts_over_exact_cache as u64)
}

fn milli_ratio_usize(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn milli_ratio_u64(numerator: u64, denominator: u64) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000).saturating_div(denominator) as usize
}

fn ewma_i64(current: i64, observed: i64, numerator: i64, denominator: i64) -> i64 {
    if denominator <= 0 || numerator <= 0 {
        return current;
    }
    let keep = denominator.saturating_sub(numerator);
    current
        .saturating_mul(keep)
        .saturating_add(observed.saturating_mul(numerator))
        / denominator
}

fn upsert_profile(
    profiles: &mut Vec<PhaseCenterOperatorProfile>,
    profile: PhaseCenterOperatorProfile,
    max_profiles_per_route: usize,
) {
    if let Some(existing) = profiles
        .iter_mut()
        .find(|existing| existing.profile_id == profile.profile_id)
    {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
    profiles.sort_by(|left, right| {
        right
            .value_score
            .cmp(&left.value_score)
            .then_with(|| right.last_seen_tick.cmp(&left.last_seen_tick))
            .then_with(|| left.profile_id.cmp(&right.profile_id))
    });
    profiles.truncate(max_profiles_per_route);
}

impl PhaseCenterOnlineMiner {
    pub fn new(config: PhaseCenterOnlineMinerConfig) -> Result<Self, PhaseCenterRuntimeError> {
        Ok(Self {
            config: config.validate()?,
            buckets: Vec::new(),
        })
    }

    /// Serializes the mutable online centers for restart recovery. This is not
    /// an executable package and never grants local execution authority.
    pub fn to_checkpoint_bytes(&self) -> Result<Vec<u8>, PhaseCenterRuntimeError> {
        let cells = u32::try_from(self.config.cells)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let min_bucket_events = u64::try_from(self.config.min_bucket_events)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let calibration_events = u64::try_from(self.config.calibration_events)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let max_buckets = u64::try_from(self.config.max_buckets)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let bucket_count = u32::try_from(self.buckets.len())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let mut bytes = Vec::with_capacity(self.bytes_estimate());
        bytes.extend_from_slice(&PHASE_CENTER_ONLINE_CHECKPOINT_MAGIC);
        bytes.extend_from_slice(&cells.to_le_bytes());
        bytes.extend_from_slice(&min_bucket_events.to_le_bytes());
        bytes.extend_from_slice(&self.config.threshold_floor_micro.to_le_bytes());
        bytes.extend_from_slice(&calibration_events.to_le_bytes());
        bytes.extend_from_slice(&max_buckets.to_le_bytes());
        bytes.extend_from_slice(&bucket_count.to_le_bytes());
        for bucket in &self.buckets {
            bytes.extend_from_slice(&bucket.bucket_id.to_le_bytes());
            write_phase_center_cells(&mut bytes, &bucket.positive_sum);
            write_phase_center_cells(&mut bytes, &bucket.negative_sum);
            for value in [
                bucket.positive_events,
                bucket.negative_events,
                bucket.events_seen,
                bucket.scored_events,
                bucket.calibration_events_seen,
                bucket.local_operator_shadow_decisions,
                bucket.unique_cpu_accepts_over_exact_cache,
                bucket.false_accepts,
            ] {
                bytes.extend_from_slice(
                    &u64::try_from(value)
                        .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?
                        .to_le_bytes(),
                );
            }
            bytes.extend_from_slice(&bucket.learned_threshold_micro.to_le_bytes());
            bytes.extend_from_slice(
                &bucket
                    .max_calibration_false_margin_micro
                    .unwrap_or(i64::MIN)
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(&bucket.tokens_saved.to_le_bytes());
            bytes.extend_from_slice(&bucket.cost_saved_microusd.to_le_bytes());
            bytes.push(u8::from(bucket.rejected));
            for value in [
                bucket.trust_quality_micro,
                bucket.trust_false_risk_micro,
                bucket.trust_drift_micro,
                bucket.trust_token_value_micro,
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        Ok(bytes)
    }

    pub fn from_checkpoint_bytes(bytes: &[u8]) -> Result<Self, PhaseCenterRuntimeError> {
        if bytes.len() < 48 || bytes[..8] != PHASE_CENTER_ONLINE_CHECKPOINT_MAGIC {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let mut offset = 8;
        let config = PhaseCenterOnlineMinerConfig {
            cells: read_u32_le_at(bytes, &mut offset)? as usize,
            min_bucket_events: read_u64_le_at(bytes, &mut offset)? as usize,
            threshold_floor_micro: read_i64_le_at(bytes, &mut offset)?,
            calibration_events: read_u64_le_at(bytes, &mut offset)? as usize,
            max_buckets: read_u64_le_at(bytes, &mut offset)? as usize,
        }
        .validate()?;
        let bucket_count = read_u32_le_at(bytes, &mut offset)? as usize;
        if bucket_count > config.max_buckets {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let mut buckets = Vec::with_capacity(bucket_count);
        for _ in 0..bucket_count {
            let bucket_id = read_u32_le_at(bytes, &mut offset)?;
            let positive_sum =
                read_phase_center_cells(bytes, &mut offset, config.cells)?.into_vec();
            let negative_sum =
                read_phase_center_cells(bytes, &mut offset, config.cells)?.into_vec();
            let positive_events = read_usize_u64_at(bytes, &mut offset)?;
            let negative_events = read_usize_u64_at(bytes, &mut offset)?;
            let events_seen = read_usize_u64_at(bytes, &mut offset)?;
            let scored_events = read_usize_u64_at(bytes, &mut offset)?;
            let calibration_events_seen = read_usize_u64_at(bytes, &mut offset)?;
            let local_operator_shadow_decisions = read_usize_u64_at(bytes, &mut offset)?;
            let unique_cpu_accepts_over_exact_cache = read_usize_u64_at(bytes, &mut offset)?;
            let false_accepts = read_usize_u64_at(bytes, &mut offset)?;
            let learned_threshold_micro = read_i64_le_at(bytes, &mut offset)?;
            let false_margin = read_i64_le_at(bytes, &mut offset)?;
            let tokens_saved = read_u64_le_at(bytes, &mut offset)?;
            let cost_saved_microusd = read_u64_le_at(bytes, &mut offset)?;
            let rejected = *bytes
                .get(offset)
                .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?
                != 0;
            offset += 1;
            buckets.push(PhaseCenterOnlineBucket {
                bucket_id,
                positive_sum,
                negative_sum,
                positive_events,
                negative_events,
                events_seen,
                scored_events,
                calibration_events_seen,
                learned_threshold_micro,
                max_calibration_false_margin_micro: (false_margin != i64::MIN)
                    .then_some(false_margin),
                local_operator_shadow_decisions,
                unique_cpu_accepts_over_exact_cache,
                tokens_saved,
                cost_saved_microusd,
                false_accepts,
                rejected,
                trust_quality_micro: read_i64_le_at(bytes, &mut offset)?,
                trust_false_risk_micro: read_i64_le_at(bytes, &mut offset)?,
                trust_drift_micro: read_i64_le_at(bytes, &mut offset)?,
                trust_token_value_micro: read_i64_le_at(bytes, &mut offset)?,
            });
        }
        if offset != bytes.len() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        buckets.sort_by_key(|bucket| bucket.bucket_id);
        if buckets
            .windows(2)
            .any(|pair| pair[0].bucket_id == pair[1].bucket_id)
        {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        Ok(Self { config, buckets })
    }

    #[must_use]
    pub const fn config(&self) -> PhaseCenterOnlineMinerConfig {
        self.config
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .buckets
                .iter()
                .map(PhaseCenterOnlineBucket::bytes_estimate)
                .sum::<usize>()
    }

    #[must_use]
    pub fn runtime_budget_snapshot(
        &self,
        memory_config: PhaseCenterOperatorMemoryConfig,
    ) -> PhaseCenterRuntimeBudgetSnapshot {
        let summary = self.summary();
        let hot_runtime_bytes_estimate = PhaseCenterHotRuntime::bytes_estimate_for(
            summary.candidate_bucket_count,
            self.config.cells,
        );
        let hot_route_table_bytes_estimate = PhaseCenterHotRouteTable::bytes_estimate_for(
            summary.candidate_bucket_count,
            summary.candidate_bucket_count,
        );
        let hot_bytes_estimate =
            hot_runtime_bytes_estimate.saturating_add(hot_route_table_bytes_estimate);
        let warm_metadata_bytes_estimate = self.bytes_estimate();
        PhaseCenterRuntimeBudgetSnapshot {
            max_hot_profiles_per_worker: memory_config.max_hot_profiles_per_worker,
            max_hot_bytes_per_worker: memory_config.max_hot_bytes_per_worker,
            max_warm_profiles_per_process: memory_config.max_warm_profiles_per_process,
            max_profiles_per_route: memory_config.max_profiles_per_route,
            max_route_top_k: memory_config.max_route_top_k,
            warm_route_count: summary.bucket_count,
            warm_profile_count: summary.bucket_count,
            warm_metadata_bytes_estimate,
            warm_runtime_bytes_estimate: 0,
            warm_bytes_estimate: warm_metadata_bytes_estimate,
            hot_route_count: summary.candidate_bucket_count,
            hot_profile_count: summary.candidate_bucket_count,
            hot_route_profile_edges: summary.candidate_bucket_count,
            hot_runtime_bytes_estimate,
            hot_route_table_bytes_estimate,
            hot_bytes_estimate,
            warm_profile_budget_passed: summary.bucket_count
                <= memory_config.max_warm_profiles_per_process,
            hot_profile_budget_passed: summary.candidate_bucket_count
                <= memory_config.max_hot_profiles_per_worker,
            hot_byte_budget_passed: hot_bytes_estimate <= memory_config.max_hot_bytes_per_worker,
        }
    }

    pub fn observe_event(
        &mut self,
        event: PhaseCenterOnlineEvent<'_>,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError> {
        self.observe(
            event.bucket_id,
            event.vector,
            event.verified_safe_accept,
            event.exact_cache_hit,
            event.tokens,
            event.cost_microusd,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe_atoms<'a, I>(
        &mut self,
        encoder: &mut PhaseCenterAtomEncoder,
        bucket_id: u32,
        atoms: I,
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        tokens: u64,
        cost_microusd: u64,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let vector = encoder.encode_atoms(atoms)?;
        self.observe(
            bucket_id,
            vector,
            verified_safe_accept,
            exact_cache_hit,
            tokens,
            cost_microusd,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe_atom_ids<I>(
        &mut self,
        encoder: &mut PhaseCenterAtomEncoder,
        bucket_id: u32,
        atom_ids: I,
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        tokens: u64,
        cost_microusd: u64,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        let vector = encoder.encode_atom_ids(atom_ids)?;
        self.observe(
            bucket_id,
            vector,
            verified_safe_accept,
            exact_cache_hit,
            tokens,
            cost_microusd,
        )
    }

    /// Bootstrap a center from completed historical evidence without treating
    /// those support rows as frozen-future shadow decisions.
    pub fn train_atom_ids<I>(
        &mut self,
        encoder: &mut PhaseCenterAtomEncoder,
        bucket_id: u32,
        atom_ids: I,
        verified_safe_accept: bool,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        let vector = encoder.encode_atom_ids(atom_ids)?;
        self.train(bucket_id, vector, verified_safe_accept)
    }

    pub fn train(
        &mut self,
        bucket_id: u32,
        vector: &[PhaseCenterCell],
        verified_safe_accept: bool,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if vector.len() != self.config.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let bucket_index = self.bucket_index_or_insert(bucket_id)?;
        self.buckets[bucket_index].add(vector, verified_safe_accept);
        Ok(())
    }

    pub fn observe_events_into<'a, I>(
        &mut self,
        events: I,
        out: &mut Vec<PhaseCenterOnlineDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = PhaseCenterOnlineEvent<'a>>,
    {
        out.clear();
        for event in events {
            out.push(self.observe_event(event)?);
        }
        Ok(())
    }

    pub fn observe(
        &mut self,
        bucket_id: u32,
        vector: &[PhaseCenterCell],
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        tokens: u64,
        cost_microusd: u64,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError> {
        if vector.len() != self.config.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let bucket_index = self.bucket_index_or_insert(bucket_id)?;
        let bucket = &mut self.buckets[bucket_index];
        let active_before_update = bucket.is_active(self.config.min_bucket_events);
        let mut decision = PhaseCenterOnlineDecision {
            bucket_id,
            active_before_update,
            threshold_micro: bucket.learned_threshold_micro,
            ..PhaseCenterOnlineDecision::default()
        };
        if active_before_update {
            let margin_micro = online_bucket_margin_micro(bucket, vector)?;
            decision.margin_micro = margin_micro;
            decision.calibration_event = bucket.calibration_events_seen
                < self.config.calibration_events
                || bucket.max_calibration_false_margin_micro.is_none();
            if decision.calibration_event {
                bucket.calibration_events_seen += 1;
                if !verified_safe_accept {
                    bucket.max_calibration_false_margin_micro = Some(
                        bucket
                            .max_calibration_false_margin_micro
                            .map_or(margin_micro, |current| current.max(margin_micro)),
                    );
                    bucket.learned_threshold_micro = bucket
                        .learned_threshold_micro
                        .max(margin_micro.saturating_add(1));
                    decision.threshold_micro = bucket.learned_threshold_micro;
                }
            } else {
                decision.raw_local_operator =
                    !bucket.rejected && margin_micro >= bucket.learned_threshold_micro;
                if decision.raw_local_operator && !verified_safe_accept {
                    bucket.false_accepts += 1;
                    bucket.rejected = true;
                    bucket.max_calibration_false_margin_micro = Some(
                        bucket
                            .max_calibration_false_margin_micro
                            .map_or(margin_micro, |current| current.max(margin_micro)),
                    );
                    bucket.learned_threshold_micro = bucket
                        .learned_threshold_micro
                        .max(margin_micro.saturating_add(1));
                    decision.threshold_micro = bucket.learned_threshold_micro;
                    decision.false_accept = true;
                }
                decision.local_operator_shadow_decision =
                    decision.raw_local_operator && verified_safe_accept && !bucket.rejected;
                decision.unique_cpu_accept_over_exact_cache =
                    decision.local_operator_shadow_decision && !exact_cache_hit;
                if decision.local_operator_shadow_decision {
                    bucket.local_operator_shadow_decisions += 1;
                    if !exact_cache_hit {
                        bucket.unique_cpu_accepts_over_exact_cache += 1;
                        bucket.tokens_saved = bucket.tokens_saved.saturating_add(tokens);
                        bucket.cost_saved_microusd =
                            bucket.cost_saved_microusd.saturating_add(cost_microusd);
                    }
                }
            }
            bucket.scored_events += 1;
            bucket.observe_trust(
                margin_micro,
                verified_safe_accept,
                decision.raw_local_operator,
                decision.false_accept,
                tokens,
            );
        }
        bucket.add(vector, verified_safe_accept);
        Ok(decision)
    }

    #[must_use]
    pub fn summary(&self) -> PhaseCenterOnlineSummary {
        let mut summary = PhaseCenterOnlineSummary {
            bucket_count: self.buckets.len(),
            active_bucket_count: self
                .buckets
                .iter()
                .filter(|bucket| bucket.is_active(self.config.min_bucket_events))
                .count(),
            shadow_ready_bucket_count: self
                .buckets
                .iter()
                .filter(|bucket| {
                    !bucket.rejected
                        && bucket.is_shadow_ready(
                            self.config.min_bucket_events,
                            self.config.calibration_events,
                        )
                })
                .count(),
            ..PhaseCenterOnlineSummary::default()
        };
        for bucket in &self.buckets {
            summary.scored_events += bucket.scored_events;
            summary.local_operator_shadow_decisions += bucket.local_operator_shadow_decisions;
            summary.unique_cpu_accepts_over_exact_cache +=
                bucket.unique_cpu_accepts_over_exact_cache;
            summary.tokens_saved = summary.tokens_saved.saturating_add(bucket.tokens_saved);
            summary.cost_saved_microusd = summary
                .cost_saved_microusd
                .saturating_add(bucket.cost_saved_microusd);
            summary.false_accepts += bucket.false_accepts;
            if bucket.rejected {
                summary.rejected_bucket_count += 1;
            } else if bucket.is_candidate() {
                summary.candidate_bucket_count += 1;
            }
        }
        summary
    }

    #[must_use]
    pub fn threshold_policy_evidence(&self) -> PhaseCenterThresholdPolicyEvidence {
        let mut evidence = PhaseCenterThresholdPolicyEvidence::default();
        let mut completed_calibration_buckets = 0usize;
        let mut shadow_after_calibration_buckets = 0usize;
        for bucket in &self.buckets {
            if bucket.rejected
                || bucket.false_accepts > 0
                || bucket.unique_cpu_accepts_over_exact_cache == 0
            {
                continue;
            }
            evidence.candidate_bucket_count += 1;
            if bucket.max_calibration_false_margin_micro.is_some() {
                evidence.auto_calibrated_bucket_count += 1;
            }
            if bucket.calibration_events_seen >= self.config.calibration_events {
                completed_calibration_buckets += 1;
            }
            if bucket.scored_events > bucket.calibration_events_seen {
                shadow_after_calibration_buckets += 1;
            }
        }
        evidence.calibration_window_before_shadow = evidence.candidate_bucket_count > 0
            && completed_calibration_buckets == evidence.candidate_bucket_count;
        evidence.shadow_window_after_calibration = evidence.candidate_bucket_count > 0
            && shadow_after_calibration_buckets == evidence.candidate_bucket_count;
        evidence.per_bucket_thresholds_reported = evidence.candidate_bucket_count > 0;
        evidence.fixed_policy_shadow_replay = evidence.shadow_window_after_calibration;
        evidence
    }

    #[must_use]
    pub fn bucket(&self, bucket_id: u32) -> Option<&PhaseCenterOnlineBucket> {
        self.buckets
            .iter()
            .find(|bucket| bucket.bucket_id == bucket_id)
    }

    pub fn candidate_runtime(
        &self,
        bucket_id: u32,
    ) -> Result<Option<PhaseCenterFlatRuntime>, PhaseCenterRuntimeError> {
        let Some(bucket) = self.bucket(bucket_id) else {
            return Ok(None);
        };
        if !bucket.is_candidate() {
            return Ok(None);
        }
        PhaseCenterFlatRuntime::new(
            self.config.cells,
            vec![PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&bucket.positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&bucket.negative_sum).into_boxed_slice(),
            }],
        )
        .map(Some)
    }

    pub fn candidate_hot_runtime(
        &self,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.candidate_hot_runtime_limited(usize::MAX)
    }

    pub fn candidate_hot_runtime_limited(
        &self,
        limit: usize,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.hot_runtime_from_buckets(self.candidate_buckets_ranked().into_iter().take(limit))
    }

    pub fn candidate_hot_runtime_limited_excluding(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.hot_runtime_from_buckets(
            self.candidate_buckets_ranked_excluding(excluded_bucket_ids)
                .into_iter()
                .take(limit),
        )
    }

    pub fn candidate_hot_runtime_limited_excluding_prioritized(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
        priority_bucket_ids: &[u32],
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.hot_runtime_from_buckets(self.candidate_buckets_ranked_excluding_prioritized(
            limit,
            excluded_bucket_ids,
            priority_bucket_ids,
        ))
    }

    pub fn shadow_ready_hot_runtime_limited(
        &self,
        limit: usize,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.hot_runtime_from_buckets(self.shadow_ready_buckets_ranked().into_iter().take(limit))
    }

    fn hot_runtime_from_buckets<'a, I>(
        &self,
        buckets: I,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterOnlineBucket>,
    {
        let mut records = Vec::new();
        let mut profile_ids = Vec::new();
        let mut thresholds_micro = Vec::new();
        for bucket in buckets {
            records.push(PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&bucket.positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&bucket.negative_sum).into_boxed_slice(),
            });
            profile_ids.push(bucket.bucket_id);
            thresholds_micro.push(bucket.learned_threshold_micro);
        }
        if records.is_empty() {
            return Ok(None);
        }
        let flat = PhaseCenterFlatRuntime::new(self.config.cells, records)?;
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &profile_ids, &thresholds_micro).map(Some)
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited(&self, limit: usize) -> Vec<u32> {
        self.candidate_buckets_ranked()
            .into_iter()
            .take(limit)
            .map(|bucket| bucket.bucket_id)
            .collect()
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited_excluding(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
    ) -> Vec<u32> {
        self.candidate_buckets_ranked_excluding(excluded_bucket_ids)
            .into_iter()
            .take(limit)
            .map(|bucket| bucket.bucket_id)
            .collect()
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited_excluding_prioritized(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
        priority_bucket_ids: &[u32],
    ) -> Vec<u32> {
        self.candidate_buckets_ranked_excluding_prioritized(
            limit,
            excluded_bucket_ids,
            priority_bucket_ids,
        )
        .into_iter()
        .map(|bucket| bucket.bucket_id)
        .collect()
    }

    pub fn candidate_packages_into(
        &self,
        out: &mut Vec<PhaseCenterOnlineCandidatePackage>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        out.clear();
        for bucket in &self.buckets {
            if let Some(package) = self.candidate_package_bytes(bucket.bucket_id)? {
                out.push(package);
            }
        }
        Ok(())
    }

    pub fn candidate_packages_into_with_verifier(
        &self,
        verifier_binding: PhaseCenterVerifierBinding,
        out: &mut Vec<PhaseCenterOnlineCandidatePackage>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if !verifier_binding.is_bound() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        out.clear();
        for bucket in &self.buckets {
            if let Some(package) =
                self.candidate_package_bytes_with_verifier(bucket.bucket_id, verifier_binding)?
            {
                out.push(package);
            }
        }
        Ok(())
    }

    pub fn candidate_packages_into_with_verifier_limited(
        &self,
        verifier_binding: PhaseCenterVerifierBinding,
        limit: usize,
        out: &mut Vec<PhaseCenterOnlineCandidatePackage>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if !verifier_binding.is_bound() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        out.clear();
        for bucket in self.candidate_buckets_ranked().into_iter().take(limit) {
            if let Some(package) =
                self.candidate_package_bytes_with_verifier(bucket.bucket_id, verifier_binding)?
            {
                out.push(package);
            }
        }
        Ok(())
    }

    pub fn candidate_packages_into_with_verifier_limited_excluding(
        &self,
        verifier_binding: PhaseCenterVerifierBinding,
        limit: usize,
        excluded_bucket_ids: &[u32],
        out: &mut Vec<PhaseCenterOnlineCandidatePackage>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if !verifier_binding.is_bound() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        out.clear();
        for bucket in self
            .candidate_buckets_ranked_excluding(excluded_bucket_ids)
            .into_iter()
            .take(limit)
        {
            if let Some(package) =
                self.candidate_package_bytes_with_verifier(bucket.bucket_id, verifier_binding)?
            {
                out.push(package);
            }
        }
        Ok(())
    }

    pub fn candidate_package_bytes(
        &self,
        bucket_id: u32,
    ) -> Result<Option<PhaseCenterOnlineCandidatePackage>, PhaseCenterRuntimeError> {
        self.candidate_package_bytes_impl(bucket_id, PhaseCenterVerifierBinding::default())
    }

    /// Exports an active center as a non-authoritative QUARANTINE candidate.
    /// Unlike a shadow-ready package, this only proves that the old online
    /// miner core has accumulated both positive and negative evidence. It must
    /// never be granted execution authority without the external future,
    /// verifier, causal and runtime-parity admission checks.
    pub fn provisional_package_bytes(
        &self,
        bucket_id: u32,
    ) -> Result<Option<PhaseCenterOnlineCandidatePackage>, PhaseCenterRuntimeError> {
        let Some(bucket) = self.bucket(bucket_id) else {
            return Ok(None);
        };
        if bucket.rejected
            || bucket.false_accepts != 0
            || !bucket.is_active(self.config.min_bucket_events)
        {
            return Ok(None);
        }
        let runtime = PhaseCenterFlatRuntime::new(
            self.config.cells,
            vec![PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&bucket.positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&bucket.negative_sum).into_boxed_slice(),
            }],
        )?;
        let package_bytes = runtime.to_bytes()?;
        let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)?;
        Ok(Some(PhaseCenterOnlineCandidatePackage {
            bucket_id,
            threshold_micro: bucket.learned_threshold_micro,
            verifier_binding: PhaseCenterVerifierBinding::default(),
            package_info,
            package_bytes,
        }))
    }

    /// Exports a calibrated shadow package for an independent admission
    /// controller. This package has no execution authority: the caller must
    /// still prove future coverage, causal separation, and runtime parity.
    pub fn shadow_ready_package_bytes(
        &self,
        bucket_id: u32,
    ) -> Result<Option<PhaseCenterOnlineCandidatePackage>, PhaseCenterRuntimeError> {
        let Some(bucket) = self.bucket(bucket_id) else {
            return Ok(None);
        };
        if bucket.rejected
            || bucket.false_accepts != 0
            || !bucket.is_shadow_ready(
                self.config.min_bucket_events,
                self.config.calibration_events,
            )
        {
            return Ok(None);
        }
        let runtime = PhaseCenterFlatRuntime::new(
            self.config.cells,
            vec![PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&bucket.positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&bucket.negative_sum).into_boxed_slice(),
            }],
        )?;
        let package_bytes = runtime.to_bytes()?;
        let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)?;
        Ok(Some(PhaseCenterOnlineCandidatePackage {
            bucket_id,
            threshold_micro: bucket.learned_threshold_micro,
            verifier_binding: PhaseCenterVerifierBinding::default(),
            package_info,
            package_bytes,
        }))
    }

    pub fn candidate_package_bytes_with_verifier(
        &self,
        bucket_id: u32,
        verifier_binding: PhaseCenterVerifierBinding,
    ) -> Result<Option<PhaseCenterOnlineCandidatePackage>, PhaseCenterRuntimeError> {
        if !verifier_binding.is_bound() {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        self.candidate_package_bytes_impl(bucket_id, verifier_binding)
    }

    fn candidate_package_bytes_impl(
        &self,
        bucket_id: u32,
        verifier_binding: PhaseCenterVerifierBinding,
    ) -> Result<Option<PhaseCenterOnlineCandidatePackage>, PhaseCenterRuntimeError> {
        let Some(bucket) = self.bucket(bucket_id) else {
            return Ok(None);
        };
        let Some(runtime) = self.candidate_runtime(bucket_id)? else {
            return Ok(None);
        };
        let package_bytes = runtime.to_bytes()?;
        let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)?;
        Ok(Some(PhaseCenterOnlineCandidatePackage {
            bucket_id,
            threshold_micro: bucket.learned_threshold_micro,
            verifier_binding,
            package_info,
            package_bytes,
        }))
    }

    fn bucket_index_or_insert(&mut self, bucket_id: u32) -> Result<usize, PhaseCenterRuntimeError> {
        if let Some(index) = self
            .buckets
            .iter()
            .position(|bucket| bucket.bucket_id == bucket_id)
        {
            return Ok(index);
        }
        if self.buckets.len() >= self.config.max_buckets {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        }
        self.buckets
            .push(PhaseCenterOnlineBucket::new(bucket_id, self.config));
        Ok(self.buckets.len() - 1)
    }

    fn candidate_buckets_ranked(&self) -> Vec<&PhaseCenterOnlineBucket> {
        let mut buckets = self
            .buckets
            .iter()
            .filter(|bucket| bucket.is_candidate())
            .collect::<Vec<_>>();
        buckets.sort_by(|left, right| {
            right
                .tokens_saved
                .cmp(&left.tokens_saved)
                .then_with(|| {
                    right
                        .unique_cpu_accepts_over_exact_cache
                        .cmp(&left.unique_cpu_accepts_over_exact_cache)
                })
                .then_with(|| right.cost_saved_microusd.cmp(&left.cost_saved_microusd))
                .then_with(|| left.bucket_id.cmp(&right.bucket_id))
        });
        buckets
    }

    fn candidate_buckets_ranked_excluding(
        &self,
        excluded_bucket_ids: &[u32],
    ) -> Vec<&PhaseCenterOnlineBucket> {
        let mut buckets = self.candidate_buckets_ranked();
        buckets.retain(|bucket| !excluded_bucket_ids.contains(&bucket.bucket_id));
        buckets
    }

    fn candidate_buckets_ranked_excluding_prioritized(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
        priority_bucket_ids: &[u32],
    ) -> Vec<&PhaseCenterOnlineBucket> {
        let mut buckets = Vec::new();
        for bucket_id in priority_bucket_ids {
            if buckets.len() >= limit {
                return buckets;
            }
            if excluded_bucket_ids.contains(bucket_id)
                || buckets.iter().any(|bucket| bucket.bucket_id == *bucket_id)
            {
                continue;
            }
            if let Some(bucket) = self
                .buckets
                .iter()
                .find(|bucket| bucket.bucket_id == *bucket_id)
                .filter(|bucket| bucket.is_candidate())
            {
                buckets.push(bucket);
            }
        }
        for bucket in self.candidate_buckets_ranked_excluding(excluded_bucket_ids) {
            if buckets.len() >= limit {
                break;
            }
            if buckets
                .iter()
                .any(|selected| selected.bucket_id == bucket.bucket_id)
            {
                continue;
            }
            buckets.push(bucket);
        }
        buckets
    }

    fn shadow_ready_buckets_ranked(&self) -> Vec<&PhaseCenterOnlineBucket> {
        let mut buckets = self
            .buckets
            .iter()
            .filter(|bucket| {
                !bucket.rejected
                    && bucket.is_shadow_ready(
                        self.config.min_bucket_events,
                        self.config.calibration_events,
                    )
            })
            .collect::<Vec<_>>();
        buckets.sort_by(|left, right| {
            right
                .tokens_saved
                .cmp(&left.tokens_saved)
                .then_with(|| {
                    right
                        .unique_cpu_accepts_over_exact_cache
                        .cmp(&left.unique_cpu_accepts_over_exact_cache)
                })
                .then_with(|| right.events_seen.cmp(&left.events_seen))
                .then_with(|| left.bucket_id.cmp(&right.bucket_id))
        });
        buckets
    }
}

impl PhaseCenterLiveOperatorStore {
    pub fn new(
        config: PhaseCenterLiveOperatorStoreConfig,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        let config = config.validate()?;
        Ok(Self {
            miner: PhaseCenterOnlineMiner::new(config.miner)?,
            memory_config: config.memory,
            routes: Vec::new(),
            route_buckets: Vec::new(),
        })
    }

    #[must_use]
    pub const fn memory_config(&self) -> PhaseCenterOperatorMemoryConfig {
        self.memory_config
    }

    #[must_use]
    pub const fn miner(&self) -> &PhaseCenterOnlineMiner {
        &self.miner
    }

    #[must_use]
    pub fn summary(&self) -> PhaseCenterOnlineSummary {
        self.miner.summary()
    }

    #[must_use]
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    #[must_use]
    pub fn route_bucket_count(&self) -> usize {
        self.route_buckets.len()
    }

    #[must_use]
    pub fn route_stats(&self, route_id: u32) -> Option<&PhaseCenterLiveRouteStats> {
        self.routes.iter().find(|route| route.route_id == route_id)
    }

    #[must_use]
    pub fn route_id_for_bucket(&self, bucket_id: u32) -> Option<u32> {
        self.route_buckets
            .iter()
            .find(|entry| entry.bucket_id == bucket_id)
            .map(|entry| entry.route_id)
    }

    #[must_use]
    pub fn runtime_budget_snapshot(&self) -> PhaseCenterRuntimeBudgetSnapshot {
        let mut snapshot = self.miner.runtime_budget_snapshot(self.memory_config);
        let selected_bucket_ids = self
            .miner
            .candidate_bucket_ids_limited(self.memory_config.max_hot_profiles_per_worker);
        let selected_profile_count = selected_bucket_ids.len();
        let selected_route_profile_edges = self
            .route_buckets
            .iter()
            .filter(|entry| selected_bucket_ids.contains(&entry.bucket_id))
            .count();
        let selected_route_count = self
            .routes
            .iter()
            .filter(|route| {
                self.route_buckets.iter().any(|entry| {
                    entry.route_id == route.route_id
                        && selected_bucket_ids.contains(&entry.bucket_id)
                })
            })
            .count();
        snapshot.warm_route_count = self.routes.len();
        snapshot.hot_profile_count = selected_profile_count;
        snapshot.hot_route_count = selected_route_count;
        snapshot.hot_route_profile_edges = selected_route_profile_edges;
        snapshot.hot_runtime_bytes_estimate = PhaseCenterHotRuntime::bytes_estimate_for(
            selected_profile_count,
            self.miner.config.cells,
        );
        snapshot.hot_route_table_bytes_estimate = PhaseCenterHotRouteTable::bytes_estimate_for(
            snapshot.hot_route_count,
            snapshot.hot_route_profile_edges,
        );
        snapshot.hot_bytes_estimate = snapshot
            .hot_runtime_bytes_estimate
            .saturating_add(snapshot.hot_route_table_bytes_estimate);
        snapshot.hot_profile_budget_passed =
            selected_profile_count <= self.memory_config.max_hot_profiles_per_worker;
        snapshot.hot_byte_budget_passed =
            snapshot.hot_bytes_estimate <= self.memory_config.max_hot_bytes_per_worker;
        snapshot
    }

    #[must_use]
    pub fn threshold_policy_evidence(&self) -> PhaseCenterThresholdPolicyEvidence {
        self.miner.threshold_policy_evidence()
    }

    pub fn observe_event(
        &mut self,
        event: PhaseCenterOnlineEvent<'_>,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError> {
        self.miner.observe_event(event)
    }

    pub fn observe_atom_event(
        &mut self,
        encoder: &mut PhaseCenterAtomEncoder,
        event: PhaseCenterLiveOperatorAtomEvent<'_>,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError> {
        self.ensure_route_bucket(event.route_id, event.bucket_id);
        let decision = self.miner.observe_atom_ids(
            encoder,
            event.bucket_id,
            event.atom_ids.iter().copied(),
            event.verified_safe_accept,
            event.exact_cache_hit,
            event.tokens,
            event.cost_microusd,
        )?;
        self.update_route_stats(
            event.route_id,
            decision,
            event.verified_safe_accept,
            event.tokens,
            event.cost_microusd,
        );
        Ok(decision)
    }

    pub fn observe(
        &mut self,
        bucket_id: u32,
        vector: &[PhaseCenterCell],
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        tokens: u64,
        cost_microusd: u64,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError> {
        self.miner.observe(
            bucket_id,
            vector,
            verified_safe_accept,
            exact_cache_hit,
            tokens,
            cost_microusd,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe_atom_ids<I>(
        &mut self,
        encoder: &mut PhaseCenterAtomEncoder,
        bucket_id: u32,
        atom_ids: I,
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        tokens: u64,
        cost_microusd: u64,
    ) -> Result<PhaseCenterOnlineDecision, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        self.miner.observe_atom_ids(
            encoder,
            bucket_id,
            atom_ids,
            verified_safe_accept,
            exact_cache_hit,
            tokens,
            cost_microusd,
        )
    }

    pub fn candidate_packages_into_with_verifier(
        &self,
        verifier_binding: PhaseCenterVerifierBinding,
        out: &mut Vec<PhaseCenterOnlineCandidatePackage>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        self.miner.candidate_packages_into_with_verifier_limited(
            verifier_binding,
            self.memory_config.max_hot_profiles_per_worker,
            out,
        )
    }

    pub fn candidate_hot_runtime(
        &self,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.miner
            .candidate_hot_runtime_limited(self.memory_config.max_hot_profiles_per_worker)
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited(&self, limit: usize) -> Vec<u32> {
        self.miner.candidate_bucket_ids_limited(limit)
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited_excluding(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
    ) -> Vec<u32> {
        self.miner
            .candidate_bucket_ids_limited_excluding(limit, excluded_bucket_ids)
    }

    #[must_use]
    pub fn candidate_bucket_ids_limited_excluding_prioritized(
        &self,
        limit: usize,
        excluded_bucket_ids: &[u32],
        priority_bucket_ids: &[u32],
    ) -> Vec<u32> {
        self.miner
            .candidate_bucket_ids_limited_excluding_prioritized(
                limit,
                excluded_bucket_ids,
                priority_bucket_ids,
            )
    }

    pub fn shadow_ready_hot_runtime(
        &self,
    ) -> Result<Option<PhaseCenterHotRuntime>, PhaseCenterRuntimeError> {
        self.miner
            .shadow_ready_hot_runtime_limited(self.memory_config.max_hot_profiles_per_worker)
    }

    pub fn candidate_hot_route_table(
        &self,
        hot_runtime: &PhaseCenterHotRuntime,
    ) -> Result<PhaseCenterHotRouteTable, PhaseCenterRuntimeError> {
        let mut plans = Vec::new();
        for route in &self.routes {
            let candidate_bucket_ids = self
                .route_buckets
                .iter()
                .filter(|entry| entry.route_id == route.route_id)
                .map(|entry| entry.bucket_id)
                .filter(|&bucket_id| hot_runtime.resolve_profile_index(bucket_id).is_some());
            if let Some(plan) =
                hot_runtime.route_plan_from_profile_ids(route.route_id, candidate_bucket_ids)?
            {
                plans.push(plan);
            }
        }
        PhaseCenterHotRouteTable::from_plans(plans)
    }

    pub fn candidate_hot_runtime_and_route_table(
        &self,
    ) -> Result<Option<(PhaseCenterHotRuntime, PhaseCenterHotRouteTable)>, PhaseCenterRuntimeError>
    {
        let Some(hot_runtime) = self.candidate_hot_runtime()? else {
            return Ok(None);
        };
        let route_table = self.candidate_hot_route_table(&hot_runtime)?;
        Ok(Some((hot_runtime, route_table)))
    }

    pub fn candidate_hot_runtime_and_route_table_excluding(
        &self,
        excluded_bucket_ids: &[u32],
    ) -> Result<Option<(PhaseCenterHotRuntime, PhaseCenterHotRouteTable)>, PhaseCenterRuntimeError>
    {
        let Some(hot_runtime) = self.miner.candidate_hot_runtime_limited_excluding(
            self.memory_config.max_hot_profiles_per_worker,
            excluded_bucket_ids,
        )?
        else {
            return Ok(None);
        };
        let route_table = self.candidate_hot_route_table(&hot_runtime)?;
        Ok(Some((hot_runtime, route_table)))
    }

    pub fn candidate_hot_runtime_and_route_table_excluding_prioritized(
        &self,
        excluded_bucket_ids: &[u32],
        priority_bucket_ids: &[u32],
    ) -> Result<Option<(PhaseCenterHotRuntime, PhaseCenterHotRouteTable)>, PhaseCenterRuntimeError>
    {
        let Some(hot_runtime) = self
            .miner
            .candidate_hot_runtime_limited_excluding_prioritized(
                self.memory_config.max_hot_profiles_per_worker,
                excluded_bucket_ids,
                priority_bucket_ids,
            )?
        else {
            return Ok(None);
        };
        let route_table = self.candidate_hot_route_table(&hot_runtime)?;
        Ok(Some((hot_runtime, route_table)))
    }

    pub fn shadow_ready_hot_runtime_and_route_table(
        &self,
    ) -> Result<Option<(PhaseCenterHotRuntime, PhaseCenterHotRouteTable)>, PhaseCenterRuntimeError>
    {
        let Some(hot_runtime) = self.shadow_ready_hot_runtime()? else {
            return Ok(None);
        };
        let route_table = self.candidate_hot_route_table(&hot_runtime)?;
        Ok(Some((hot_runtime, route_table)))
    }

    fn ensure_route_bucket(&mut self, route_id: u32, bucket_id: u32) {
        let route_index = self.route_index_or_insert(route_id);
        if self
            .route_buckets
            .iter()
            .any(|entry| entry.route_id == route_id && entry.bucket_id == bucket_id)
        {
            return;
        }
        self.route_buckets.push(PhaseCenterLiveRouteBucket {
            route_id,
            bucket_id,
        });
        self.routes[route_index].route_bucket_count += 1;
    }

    fn route_index_or_insert(&mut self, route_id: u32) -> usize {
        if let Some(index) = self
            .routes
            .iter()
            .position(|route| route.route_id == route_id)
        {
            return index;
        }
        self.routes.push(PhaseCenterLiveRouteStats {
            route_id,
            ..PhaseCenterLiveRouteStats::default()
        });
        self.routes.len() - 1
    }

    fn update_route_stats(
        &mut self,
        route_id: u32,
        decision: PhaseCenterOnlineDecision,
        verified_safe_accept: bool,
        tokens: u64,
        cost_microusd: u64,
    ) {
        let route_index = self.route_index_or_insert(route_id);
        let route = &mut self.routes[route_index];
        route.events_seen += 1;
        if decision.active_before_update {
            route.scored_events += 1;
        }
        if decision.local_operator_shadow_decision {
            route.local_operator_shadow_decisions += 1;
        }
        if decision.unique_cpu_accept_over_exact_cache {
            route.unique_cpu_accepts_over_exact_cache += 1;
            route.tokens_saved = route.tokens_saved.saturating_add(tokens);
            route.cost_saved_microusd = route.cost_saved_microusd.saturating_add(cost_microusd);
        }
        if decision.false_accept || (decision.raw_local_operator && !verified_safe_accept) {
            route.false_accepts += 1;
        }
    }
}

impl PhaseCenterOnlineBucket {
    fn new(bucket_id: u32, config: PhaseCenterOnlineMinerConfig) -> Self {
        Self {
            bucket_id,
            positive_sum: vec![PhaseCenterCell::default(); config.cells],
            negative_sum: vec![PhaseCenterCell::default(); config.cells],
            positive_events: 0,
            negative_events: 0,
            events_seen: 0,
            scored_events: 0,
            calibration_events_seen: 0,
            learned_threshold_micro: config.threshold_floor_micro,
            max_calibration_false_margin_micro: None,
            local_operator_shadow_decisions: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            tokens_saved: 0,
            cost_saved_microusd: 0,
            false_accepts: 0,
            rejected: false,
            trust_quality_micro: 0,
            trust_false_risk_micro: 0,
            trust_drift_micro: 0,
            trust_token_value_micro: 0,
        }
    }

    #[must_use]
    pub fn is_active(&self, min_bucket_events: usize) -> bool {
        self.events_seen >= min_bucket_events
            && self.positive_events > 0
            && self.negative_events > 0
    }

    #[must_use]
    pub const fn is_candidate(&self) -> bool {
        !self.rejected
            && self.false_accepts == 0
            && self.positive_events > 0
            && self.negative_events > 0
            && self.unique_cpu_accepts_over_exact_cache > 0
    }

    #[must_use]
    pub fn is_shadow_ready(&self, min_bucket_events: usize, calibration_events: usize) -> bool {
        self.is_active(min_bucket_events)
            && self.calibration_events_seen >= calibration_events
            && self.max_calibration_false_margin_micro.is_some()
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.positive_sum.len() * std::mem::size_of::<PhaseCenterCell>()
            + self.negative_sum.len() * std::mem::size_of::<PhaseCenterCell>()
    }

    fn observe_trust(
        &mut self,
        margin_micro: i64,
        verified_safe_accept: bool,
        raw_local_operator: bool,
        false_accept: bool,
        tokens: u64,
    ) {
        const ALPHA_NUMERATOR: i64 = 1;
        const ALPHA_DENOMINATOR: i64 = 8;
        let quality_observation = if verified_safe_accept {
            margin_micro.max(0)
        } else {
            margin_micro.saturating_neg().max(0)
        };
        self.trust_quality_micro = ewma_i64(
            self.trust_quality_micro,
            quality_observation,
            ALPHA_NUMERATOR,
            ALPHA_DENOMINATOR,
        );
        let risk_observation = if false_accept {
            1_000_000
        } else if raw_local_operator && !verified_safe_accept {
            750_000
        } else if !verified_safe_accept && margin_micro >= self.learned_threshold_micro {
            500_000
        } else {
            0
        };
        self.trust_false_risk_micro = ewma_i64(
            self.trust_false_risk_micro,
            risk_observation,
            ALPHA_NUMERATOR,
            ALPHA_DENOMINATOR,
        );
        let drift_observation = margin_micro
            .saturating_sub(self.learned_threshold_micro)
            .saturating_abs();
        self.trust_drift_micro = ewma_i64(
            self.trust_drift_micro,
            drift_observation,
            ALPHA_NUMERATOR,
            ALPHA_DENOMINATOR,
        );
        let token_observation = if raw_local_operator && verified_safe_accept {
            i64::try_from(tokens.min(i64::MAX as u64)).unwrap_or(i64::MAX)
        } else {
            0
        };
        self.trust_token_value_micro = ewma_i64(
            self.trust_token_value_micro,
            token_observation,
            ALPHA_NUMERATOR,
            ALPHA_DENOMINATOR,
        );
    }

    fn add(&mut self, vector: &[PhaseCenterCell], verified_safe_accept: bool) {
        if self.events_seen > 0
            && self
                .events_seen
                .is_multiple_of(PHASE_CENTER_ONLINE_DECAY_INTERVAL)
        {
            for cell in self
                .positive_sum
                .iter_mut()
                .chain(self.negative_sum.iter_mut())
            {
                cell.re *= PHASE_CENTER_ONLINE_DECAY_FACTOR;
                cell.im *= PHASE_CENTER_ONLINE_DECAY_FACTOR;
            }
        }
        let target = if verified_safe_accept {
            self.positive_events += 1;
            &mut self.positive_sum
        } else {
            self.negative_events += 1;
            &mut self.negative_sum
        };
        for (sum, cell) in target.iter_mut().zip(vector.iter()) {
            sum.re += cell.re;
            sum.im += cell.im;
        }
        self.events_seen += 1;
    }
}

#[allow(dead_code)]
impl PhaseCenterHotRoutePlan {
    fn new(
        route_id: u32,
        profile_indexes: Vec<usize>,
    ) -> Result<Option<Self>, PhaseCenterRuntimeError> {
        if profile_indexes.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            route_id,
            profile_indexes: profile_indexes.into_boxed_slice(),
        }))
    }

    #[must_use]
    pub const fn route_id(&self) -> u32 {
        self.route_id
    }

    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.profile_indexes.len()
    }

    #[must_use]
    pub fn profile_indexes(&self) -> &[usize] {
        &self.profile_indexes
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>() + self.profile_indexes.len() * std::mem::size_of::<usize>()
    }
}

#[allow(dead_code)]
impl PhaseCenterHotRouteTable {
    pub fn from_plans<I>(plans: I) -> Result<Self, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = PhaseCenterHotRoutePlan>,
    {
        let mut plans = plans.into_iter().collect::<Vec<_>>();
        plans.sort_by_key(PhaseCenterHotRoutePlan::route_id);
        if plans
            .windows(2)
            .any(|window| window[0].route_id() == window[1].route_id())
        {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        Ok(Self {
            plans: plans.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn route_count(&self) -> usize {
        self.plans.len()
    }

    #[must_use]
    pub fn route_id_at(&self, route_index: usize) -> Option<u32> {
        self.plans
            .get(route_index)
            .map(PhaseCenterHotRoutePlan::route_id)
    }

    #[must_use]
    pub fn resolve_route_index(&self, route_id: u32) -> Option<usize> {
        self.plans
            .binary_search_by_key(&route_id, PhaseCenterHotRoutePlan::route_id)
            .ok()
    }

    pub fn route_plan_at(
        &self,
        route_index: usize,
    ) -> Result<&PhaseCenterHotRoutePlan, PhaseCenterRuntimeError> {
        self.plans
            .get(route_index)
            .ok_or(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .plans
                .iter()
                .map(PhaseCenterHotRoutePlan::bytes_estimate)
                .sum::<usize>()
    }

    #[must_use]
    pub const fn bytes_estimate_for(route_count: usize, route_profile_edges: usize) -> usize {
        if route_count == 0 && route_profile_edges == 0 {
            0
        } else {
            std::mem::size_of::<Self>()
                + route_count * std::mem::size_of::<PhaseCenterHotRoutePlan>()
                + route_profile_edges * std::mem::size_of::<usize>()
        }
    }

    #[must_use]
    pub fn profile_edge_count(&self) -> usize {
        self.plans
            .iter()
            .map(PhaseCenterHotRoutePlan::profile_count)
            .sum()
    }
}

#[allow(dead_code)]
impl PhaseCenterHotWorker {
    pub fn new(
        runtime: PhaseCenterHotRuntime,
        routes: PhaseCenterHotRouteTable,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        let candidate_capacity = routes.profile_edge_count().max(1);
        for plan in &routes.plans {
            for &profile_index in plan.profile_indexes() {
                if profile_index >= runtime.profile_count() {
                    return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
                }
            }
        }
        let scratch = PhaseCenterHotScratch::new(runtime.cells(), candidate_capacity)?;
        Ok(Self {
            runtime,
            routes,
            scratch,
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.runtime.cells()
    }

    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.runtime.profile_count()
    }

    #[must_use]
    pub fn route_count(&self) -> usize {
        self.routes.route_count()
    }

    #[must_use]
    pub fn route_profile_edge_count(&self) -> usize {
        self.routes.profile_edge_count()
    }

    #[must_use]
    pub fn resolve_route_index(&self, route_id: u32) -> Option<usize> {
        self.routes.resolve_route_index(route_id)
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.runtime
            .bytes_estimate()
            .saturating_add(self.routes.bytes_estimate())
            .saturating_add(self.scratch.bytes_estimate())
    }

    pub fn score_prepared<'a>(
        &'a mut self,
        request: PhaseCenterPreparedHotRequest<'_>,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        self.runtime
            .score_prepared_hot_request_candidates(&self.routes, request, &mut self.scratch)
    }

    pub fn score_prepared_with_evidence<'a>(
        &'a mut self,
        request: PhaseCenterPreparedHotEvidenceRequest<'_>,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        let decisions = self.score_prepared(request.request)?;
        eval.observe_candidate_decisions(request.evidence, decisions);
        Ok(decisions)
    }

    pub fn score_prepared_row_with_evidence<'a>(
        &'a mut self,
        row: &PhaseCenterPreparedHotEvidenceRow,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        self.score_prepared_with_evidence(row.prepared_evidence_request(), eval)
    }

    pub fn score_prepared_rows_with_evidence(
        &mut self,
        rows: &[PhaseCenterPreparedHotEvidenceRow],
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<(), PhaseCenterRuntimeError> {
        for row in rows {
            let _ = self.score_prepared_row_with_evidence(row, eval)?;
        }
        Ok(())
    }

    pub fn score_atom_ids<'a>(
        &'a mut self,
        request: PhaseCenterHotRequest<'_>,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        self.runtime
            .score_hot_request_candidates(&self.routes, request, &mut self.scratch)
    }

    pub fn score_atom_ids_with_evidence<'a>(
        &'a mut self,
        request: PhaseCenterHotEvidenceRequest<'_>,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        let decisions = self.score_atom_ids(request.request)?;
        eval.observe_candidate_decisions(request.evidence, decisions);
        Ok(decisions)
    }

    pub fn score_live_atom_event_with_evidence<'a>(
        &'a mut self,
        event: PhaseCenterLiveOperatorAtomEvent<'_>,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<Option<&'a [PhaseCenterHotCandidateDecision]>, PhaseCenterRuntimeError> {
        let Some(route_index) = self.resolve_route_index(event.route_id) else {
            return Ok(None);
        };
        self.score_atom_ids_with_evidence(
            PhaseCenterHotEvidenceRequest::new(route_index, event.atom_ids, event.evidence()),
            eval,
        )
        .map(Some)
    }
}

fn online_bucket_margin_micro(
    bucket: &PhaseCenterOnlineBucket,
    vector: &[PhaseCenterCell],
) -> Result<i64, PhaseCenterRuntimeError> {
    let positive_center = phase_center_from_sum(&bucket.positive_sum);
    let negative_center = phase_center_from_sum(&bucket.negative_sum);
    let mut score = 0.0f64;
    for ((cell, positive), negative) in vector
        .iter()
        .zip(positive_center.iter())
        .zip(negative_center.iter())
    {
        let center_delta_re = positive.re - negative.re;
        let center_delta_im = positive.im - negative.im;
        score += cell.re * center_delta_re + cell.im * center_delta_im;
    }
    phase_margin_to_micro(score / vector.len() as f64)
}

#[allow(dead_code)]
impl PhaseCenterHotRuntime {
    pub fn from_flat_runtime(
        runtime: &PhaseCenterFlatRuntime,
        profile_ids: &[u32],
        thresholds_micro: &[i64],
    ) -> Result<Self, PhaseCenterRuntimeError> {
        if profile_ids.len() != runtime.record_count()
            || thresholds_micro.len() != runtime.record_count()
        {
            return Err(PhaseCenterRuntimeError::IncompleteProgram);
        }
        let mut profiles = Vec::with_capacity(runtime.record_count());
        for (index, record) in runtime.records.iter().enumerate() {
            if thresholds_micro[index] <= 0 {
                return Err(PhaseCenterRuntimeError::InvalidOffloadThreshold);
            }
            let center_delta = record
                .positive_center
                .iter()
                .zip(record.negative_center.iter())
                .map(|(positive, negative)| PhaseCenterCell {
                    re: positive.re - negative.re,
                    im: positive.im - negative.im,
                })
                .collect::<Vec<_>>();
            profiles.push(PhaseCenterHotProfile {
                profile_id: profile_ids[index],
                threshold_micro: thresholds_micro[index],
                center_delta: center_delta.into_boxed_slice(),
            });
        }
        Ok(Self {
            cells: runtime.cells(),
            profiles: profiles.into_boxed_slice(),
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    #[must_use]
    pub fn profile_id_at(&self, profile_index: usize) -> Option<u32> {
        self.profiles
            .get(profile_index)
            .map(|profile| profile.profile_id)
    }

    #[must_use]
    pub fn resolve_profile_index(&self, profile_id: u32) -> Option<usize> {
        self.profiles
            .iter()
            .position(|profile| profile.profile_id == profile_id)
    }

    pub fn route_plan_from_profile_ids<I>(
        &self,
        route_id: u32,
        profile_ids: I,
    ) -> Result<Option<PhaseCenterHotRoutePlan>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut profile_indexes = Vec::new();
        for profile_id in profile_ids {
            let Some(profile_index) = self.resolve_profile_index(profile_id) else {
                return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
            };
            profile_indexes.push(profile_index);
        }
        PhaseCenterHotRoutePlan::new(route_id, profile_indexes)
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        Self::bytes_estimate_for(self.profiles.len(), self.cells)
    }

    #[must_use]
    pub const fn bytes_estimate_for(profile_count: usize, cells: usize) -> usize {
        profile_count * std::mem::size_of::<PhaseCenterHotProfile>()
            + profile_count * cells * std::mem::size_of::<PhaseCenterCell>()
    }

    pub fn score_profile(
        &self,
        profile_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<PhaseCenterHotDecision, PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        self.score_profile_unchecked(profile_index, vector)
    }

    pub fn score_profile_fixed_micro(
        &self,
        profile_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<i64, PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(profile) = self.profiles.get(profile_index) else {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        };
        let mut score = 0i128;
        for (cell, delta) in vector.iter().zip(profile.center_delta.iter()) {
            score += i128::from(phase_component_to_fixed(cell.re))
                * i128::from(phase_component_to_fixed(delta.re));
            score += i128::from(phase_component_to_fixed(cell.im))
                * i128::from(phase_component_to_fixed(delta.im));
        }
        fixed_phase_score_to_micro(score, self.cells)
    }

    pub fn score_profile_fixed(
        &self,
        profile_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<PhaseCenterHotDecision, PhaseCenterRuntimeError> {
        let margin_micro = self.score_profile_fixed_micro(profile_index, vector)?;
        let Some(profile) = self.profiles.get(profile_index) else {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        };
        Ok(PhaseCenterHotDecision {
            profile_id: profile.profile_id,
            margin_micro,
            local_operator: margin_micro >= profile.threshold_micro,
        })
    }

    pub fn score_route_plan_into(
        &self,
        plan: &PhaseCenterHotRoutePlan,
        vector: &[PhaseCenterCell],
        out: &mut Vec<PhaseCenterHotDecision>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        out.clear();
        for &profile_index in plan.profile_indexes() {
            out.push(self.score_profile_unchecked(profile_index, vector)?);
        }
        Ok(())
    }

    pub fn score_route_index_into(
        &self,
        routes: &PhaseCenterHotRouteTable,
        route_index: usize,
        vector: &[PhaseCenterCell],
        out: &mut Vec<PhaseCenterHotDecision>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        let plan = routes.route_plan_at(route_index)?;
        self.score_route_plan_into(plan, vector, out)
    }

    pub fn score_route_atom_ids_into<I>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        route_index: usize,
        encoder: &mut PhaseCenterAtomEncoder,
        atom_ids: I,
        out: &mut Vec<PhaseCenterHotDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        let vector = encoder.encode_atom_ids(atom_ids)?;
        self.score_route_index_into(routes, route_index, vector, out)
    }

    pub fn score_route_candidates_into(
        &self,
        routes: &PhaseCenterHotRouteTable,
        route_index: usize,
        vector: &[PhaseCenterCell],
        out: &mut Vec<PhaseCenterHotCandidateDecision>,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let plan = routes.route_plan_at(route_index)?;
        out.clear();
        for &profile_index in plan.profile_indexes() {
            out.push(
                self.score_profile_unchecked(profile_index, vector)?
                    .to_candidate_decision(),
            );
        }
        Ok(())
    }

    pub fn score_route_atom_id_candidates_into<I>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        route_index: usize,
        encoder: &mut PhaseCenterAtomEncoder,
        atom_ids: I,
        out: &mut Vec<PhaseCenterHotCandidateDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = u64>,
    {
        let vector = encoder.encode_atom_ids(atom_ids)?;
        self.score_route_candidates_into(routes, route_index, vector, out)
    }

    pub fn score_hot_request_candidates<'a>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        request: PhaseCenterHotRequest<'_>,
        scratch: &'a mut PhaseCenterHotScratch,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        if scratch.cells() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        if request.atom_ids.len() > scratch.atom_cache_capacity() {
            return Err(PhaseCenterRuntimeError::RuntimePackageTooLarge);
        }
        let plan = routes.route_plan_at(request.route_index)?;
        scratch.atom_row_indexes.clear();
        for &atom_id in request.atom_ids {
            let row_index = scratch.ensure_atom_row(atom_id, self.cells)?;
            scratch.atom_row_indexes.push(row_index);
        }
        scratch.candidates.clear();
        scratch.scores.clear();
        scratch.scores.resize(plan.profile_count(), 0.0);

        for cell in 0..self.cells {
            let mut sum = PhaseCenterCell::default();
            for &row_index in &scratch.atom_row_indexes {
                let phase = scratch.atom_rows[row_index].cells[cell];
                sum.re += phase.re;
                sum.im += phase.im;
            }
            let vector_cell = phase_circular_unit(sum);
            for (score, &profile_index) in scratch.scores.iter_mut().zip(plan.profile_indexes()) {
                let Some(profile) = self.profiles.get(profile_index) else {
                    return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
                };
                let delta = profile.center_delta[cell];
                *score += vector_cell.re * delta.re + vector_cell.im * delta.im;
            }
        }

        for (&score, &profile_index) in scratch.scores.iter().zip(plan.profile_indexes()) {
            let Some(profile) = self.profiles.get(profile_index) else {
                return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
            };
            let margin_micro = phase_margin_to_micro(score / self.cells as f64)?;
            scratch.candidates.push(
                PhaseCenterHotDecision {
                    profile_id: profile.profile_id,
                    margin_micro,
                    local_operator: margin_micro >= profile.threshold_micro,
                }
                .to_candidate_decision(),
            );
        }
        Ok(&scratch.candidates)
    }

    pub fn score_hot_evidence_request_candidates<'a>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        request: PhaseCenterHotEvidenceRequest<'_>,
        scratch: &'a mut PhaseCenterHotScratch,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        let decisions = self.score_hot_request_candidates(routes, request.request, scratch)?;
        eval.observe_candidate_decisions(request.evidence, decisions);
        Ok(decisions)
    }

    pub fn score_prepared_hot_request_candidates<'a>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        request: PhaseCenterPreparedHotRequest<'_>,
        scratch: &'a mut PhaseCenterHotScratch,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        if scratch.cells() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        self.score_route_candidates_into(
            routes,
            request.route_index,
            request.phase_vector,
            &mut scratch.candidates,
        )?;
        Ok(&scratch.candidates)
    }

    pub fn score_prepared_hot_evidence_request_candidates<'a>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        request: PhaseCenterPreparedHotEvidenceRequest<'_>,
        scratch: &'a mut PhaseCenterHotScratch,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        let decisions =
            self.score_prepared_hot_request_candidates(routes, request.request, scratch)?;
        eval.observe_candidate_decisions(request.evidence, decisions);
        Ok(decisions)
    }

    pub fn score_prepared_hot_row_candidates<'a>(
        &self,
        routes: &PhaseCenterHotRouteTable,
        row: &PhaseCenterPreparedHotEvidenceRow,
        scratch: &'a mut PhaseCenterHotScratch,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<&'a [PhaseCenterHotCandidateDecision], PhaseCenterRuntimeError> {
        self.score_prepared_hot_evidence_request_candidates(
            routes,
            row.prepared_evidence_request(),
            scratch,
            eval,
        )
    }

    pub fn score_prepared_hot_rows_into(
        &self,
        routes: &PhaseCenterHotRouteTable,
        rows: &[PhaseCenterPreparedHotEvidenceRow],
        scratch: &mut PhaseCenterHotScratch,
        eval: &mut PhaseCenterHotShadowEval,
    ) -> Result<(), PhaseCenterRuntimeError> {
        for row in rows {
            let _ = self.score_prepared_hot_row_candidates(routes, row, scratch, eval)?;
        }
        Ok(())
    }

    fn score_profile_unchecked(
        &self,
        profile_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<PhaseCenterHotDecision, PhaseCenterRuntimeError> {
        let Some(profile) = self.profiles.get(profile_index) else {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        };
        let mut score = 0.0f64;
        for (cell, delta) in vector.iter().zip(profile.center_delta.iter()) {
            score += cell.re * delta.re + cell.im * delta.im;
        }
        let margin_micro = phase_margin_to_micro(score / self.cells as f64)?;
        Ok(PhaseCenterHotDecision {
            profile_id: profile.profile_id,
            margin_micro,
            local_operator: margin_micro >= profile.threshold_micro,
        })
    }
}

impl PhaseCenterOffloadSummary {
    #[must_use]
    pub fn from_decisions<I>(decisions: I) -> Self
    where
        I: IntoIterator<Item = PhaseCenterOffloadDecision>,
    {
        let decisions = decisions.into_iter().collect::<Vec<_>>();
        Self::from_decision_slice(&decisions)
    }

    #[must_use]
    pub fn from_repeated_decisions<I>(decisions: I, calls: usize) -> Self
    where
        I: IntoIterator<Item = PhaseCenterOffloadDecision>,
    {
        let decisions = decisions.into_iter().collect::<Vec<_>>();
        Self::from_repeated_decision_slice(&decisions, calls)
    }

    #[must_use]
    pub fn from_decision_slice(decisions: &[PhaseCenterOffloadDecision]) -> Self {
        let mut margin_scratch = Vec::new();
        Self::from_decision_slice_into(decisions, &mut margin_scratch)
    }

    #[must_use]
    pub fn from_decision_slice_into(
        decisions: &[PhaseCenterOffloadDecision],
        margin_scratch: &mut Vec<i64>,
    ) -> Self {
        Self::from_repeated_decision_slice_into(decisions, decisions.len(), margin_scratch)
    }

    #[must_use]
    pub fn from_repeated_decision_slice(
        decisions: &[PhaseCenterOffloadDecision],
        calls: usize,
    ) -> Self {
        let mut margin_scratch = Vec::new();
        Self::from_repeated_decision_slice_into(decisions, calls, &mut margin_scratch)
    }

    #[must_use]
    pub fn from_repeated_decision_slice_into(
        decisions: &[PhaseCenterOffloadDecision],
        calls: usize,
        margin_scratch: &mut Vec<i64>,
    ) -> Self {
        Self::from_repeated_decision_fn_into(
            decisions.len(),
            calls,
            |index| decisions[index],
            margin_scratch,
        )
    }

    #[must_use]
    pub fn from_repeated_decision_fn_into<F>(
        decision_count: usize,
        calls: usize,
        decision_at: F,
        margin_scratch: &mut Vec<i64>,
    ) -> Self
    where
        F: Fn(usize) -> PhaseCenterOffloadDecision,
    {
        summarize_repeated_offload_decisions_into(
            decision_count,
            calls,
            decision_at,
            margin_scratch,
        )
    }
}

impl PhaseCenterCompiler {
    pub fn new(cells: usize, program_count: usize) -> Result<Self, PhaseCenterRuntimeError> {
        if cells == 0 || program_count == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        Ok(Self {
            cells,
            positive_sums: vec![vec![PhaseCenterCell::default(); cells]; program_count],
            negative_sums: vec![vec![PhaseCenterCell::default(); cells]; program_count],
            positive_counts: vec![0; program_count],
            negative_counts: vec![0; program_count],
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn program_count(&self) -> usize {
        self.positive_sums.len()
    }

    pub fn add_positive_atoms<'a, I>(
        &mut self,
        program_index: usize,
        atoms: I,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let vector = phase_vector_from_atoms(atoms, self.cells);
        self.add_positive_vector(program_index, &vector)
    }

    pub fn add_negative_atoms<'a, I>(
        &mut self,
        program_index: usize,
        atoms: I,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let vector = phase_vector_from_atoms(atoms, self.cells);
        self.add_negative_vector(program_index, &vector)
    }

    pub fn add_positive_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<(), PhaseCenterRuntimeError> {
        self.add_vector(program_index, vector, true)
    }

    pub fn add_negative_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<(), PhaseCenterRuntimeError> {
        self.add_vector(program_index, vector, false)
    }

    fn add_vector(
        &mut self,
        program_index: usize,
        vector: &[PhaseCenterCell],
        is_positive: bool,
    ) -> Result<(), PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(sum) = (if is_positive {
            self.positive_sums.get_mut(program_index)
        } else {
            self.negative_sums.get_mut(program_index)
        }) else {
            return Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds);
        };
        add_phase_vector(sum, vector, 1.0);
        if is_positive {
            self.positive_counts[program_index] += 1;
        } else {
            self.negative_counts[program_index] += 1;
        }
        Ok(())
    }

    pub fn compile(self) -> Result<PhaseCenterFlatRuntime, PhaseCenterRuntimeError> {
        if self
            .positive_counts
            .iter()
            .zip(self.negative_counts.iter())
            .any(|(positive, negative)| *positive == 0 || *negative == 0)
        {
            return Err(PhaseCenterRuntimeError::IncompleteProgram);
        }

        let records = self
            .positive_sums
            .into_iter()
            .zip(self.negative_sums)
            .map(|(positive_sum, negative_sum)| PhaseCenterFlatRecord {
                positive_center: phase_center_from_sum(&positive_sum).into_boxed_slice(),
                negative_center: phase_center_from_sum(&negative_sum).into_boxed_slice(),
            })
            .collect::<Vec<_>>();
        PhaseCenterFlatRuntime::new(self.cells, records)
    }
}

impl PhaseCenterFlatRuntime {
    pub fn new(
        cells: usize,
        records: Vec<PhaseCenterFlatRecord>,
    ) -> Result<Self, PhaseCenterRuntimeError> {
        if cells == 0 || records.is_empty() {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        if records.iter().any(|record| {
            record.positive_center.len() != cells || record.negative_center.len() != cells
        }) {
            return Err(PhaseCenterRuntimeError::RecordWidthMismatch);
        }
        Ok(Self {
            cells,
            records: records.into_boxed_slice(),
        })
    }

    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn record(
        &self,
        center_index: usize,
    ) -> Result<&PhaseCenterFlatRecord, PhaseCenterRuntimeError> {
        self.records
            .get(center_index)
            .ok_or(PhaseCenterRuntimeError::CenterIndexOutOfBounds)
    }

    #[must_use]
    pub fn bytes_estimate(&self) -> usize {
        self.records.len() * 2 * self.cells * std::mem::size_of::<PhaseCenterCell>()
            + self.records.len() * std::mem::size_of::<PhaseCenterFlatRecord>()
    }

    #[must_use]
    pub fn serialized_len(&self) -> usize {
        runtime_package_len(self.cells, self.records.len()).unwrap_or(usize::MAX)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, PhaseCenterRuntimeError> {
        let cells = u32::try_from(self.cells)
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let records = u32::try_from(self.records.len())
            .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let serialized_len = runtime_package_len(self.cells, self.records.len())
            .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        let mut bytes = Vec::with_capacity(serialized_len);
        bytes.extend_from_slice(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC);
        bytes.extend_from_slice(&cells.to_le_bytes());
        bytes.extend_from_slice(&records.to_le_bytes());
        for record in self.records.iter() {
            write_phase_center_cells(&mut bytes, &record.positive_center);
            write_phase_center_cells(&mut bytes, &record.negative_center);
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PhaseCenterRuntimeError> {
        let info = Self::inspect_bytes(bytes)?;

        let mut offset = PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES;
        let mut records = Vec::with_capacity(info.record_count);
        for _ in 0..info.record_count {
            let positive_center = read_phase_center_cells(bytes, &mut offset, info.cells)?;
            let negative_center = read_phase_center_cells(bytes, &mut offset, info.cells)?;
            records.push(PhaseCenterFlatRecord {
                positive_center,
                negative_center,
            });
        }
        PhaseCenterFlatRuntime::new(info.cells, records)
    }

    pub fn inspect_bytes(
        bytes: &[u8],
    ) -> Result<PhaseCenterRuntimePackageInfo, PhaseCenterRuntimeError> {
        if bytes.len() < PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        if bytes[..PHASE_CENTER_RUNTIME_PACKAGE_MAGIC.len()] != PHASE_CENTER_RUNTIME_PACKAGE_MAGIC {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        let cells = read_u32_le(bytes, 8)? as usize;
        let record_count = read_u32_le(bytes, 12)? as usize;
        if cells == 0 || record_count == 0 {
            return Err(PhaseCenterRuntimeError::EmptyRuntime);
        }
        let serialized_len = runtime_package_len(cells, record_count)
            .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
        if bytes.len() != serialized_len {
            return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
        }
        Ok(PhaseCenterRuntimePackageInfo {
            magic: PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
            cells,
            record_count,
            serialized_len,
            payload_bytes: serialized_len - PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
            fingerprint64: runtime_package_fingerprint64(bytes),
        })
    }

    pub fn margin(&self, task: &PhaseCenterEvalTask) -> Result<f64, PhaseCenterRuntimeError> {
        self.margin_for(task.center_index, &task.correct_vec, &task.wrong_vec)
    }

    pub fn offload_decision(
        &self,
        task: &PhaseCenterEvalTask,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin = self.margin(task)?;
        policy.decide_margin(margin)
    }

    pub fn offload_decisions<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Vec<PhaseCenterOffloadDecision>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        let mut out = Vec::new();
        self.offload_decisions_into(tasks, policy, &mut out)?;
        Ok(out)
    }

    pub fn offload_decisions_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        out.clear();
        for task in tasks {
            out.push(self.offload_decision(task, policy)?);
        }
        Ok(())
    }

    pub fn offload_summary_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = &'a PhaseCenterEvalTask>,
    {
        self.offload_decisions_into(tasks, policy, decision_scratch)?;
        Ok(PhaseCenterOffloadSummary::from_decision_slice_into(
            decision_scratch,
            margin_scratch,
        ))
    }

    pub fn margin_for(
        &self,
        center_index: usize,
        correct_vec: &[PhaseCenterCell],
        wrong_vec: &[PhaseCenterCell],
    ) -> Result<f64, PhaseCenterRuntimeError> {
        if correct_vec.len() != self.cells || wrong_vec.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(record) = self.records.get(center_index) else {
            return Err(PhaseCenterRuntimeError::CenterIndexOutOfBounds);
        };
        Ok(phase_margin_from_centers(
            correct_vec,
            wrong_vec,
            &record.positive_center,
            &record.negative_center,
        ))
    }

    pub fn score_vector_margin_micro(
        &self,
        center_index: usize,
        vector: &[PhaseCenterCell],
    ) -> Result<i64, PhaseCenterRuntimeError> {
        if vector.len() != self.cells {
            return Err(PhaseCenterRuntimeError::VectorWidthMismatch);
        }
        let Some(record) = self.records.get(center_index) else {
            return Err(PhaseCenterRuntimeError::CenterIndexOutOfBounds);
        };
        let mut score = 0.0f64;
        for ((cell, positive), negative) in vector
            .iter()
            .zip(record.positive_center.iter())
            .zip(record.negative_center.iter())
        {
            score += cell.re * (positive.re - negative.re) + cell.im * (positive.im - negative.im);
        }
        phase_margin_to_micro(score / self.cells as f64)
    }

    pub fn offload_decision_for(
        &self,
        center_index: usize,
        correct_vec: &[PhaseCenterCell],
        wrong_vec: &[PhaseCenterCell],
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<PhaseCenterOffloadDecision, PhaseCenterRuntimeError> {
        let margin = self.margin_for(center_index, correct_vec, wrong_vec)?;
        policy.decide_margin(margin)
    }

    pub fn offload_decisions_for<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
    ) -> Result<Vec<PhaseCenterOffloadDecision>, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        let mut out = Vec::new();
        self.offload_decisions_for_into(tasks, policy, &mut out)?;
        Ok(out)
    }

    pub fn offload_decisions_for_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        out: &mut Vec<PhaseCenterOffloadDecision>,
    ) -> Result<(), PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        out.clear();
        for (center_index, correct_vec, wrong_vec) in tasks {
            out.push(self.offload_decision_for(center_index, correct_vec, wrong_vec, policy)?);
        }
        Ok(())
    }

    pub fn offload_summary_for_into<'a, I>(
        &self,
        tasks: I,
        policy: PhaseCenterOffloadPolicy,
        decision_scratch: &mut Vec<PhaseCenterOffloadDecision>,
        margin_scratch: &mut Vec<i64>,
    ) -> Result<PhaseCenterOffloadSummary, PhaseCenterRuntimeError>
    where
        I: IntoIterator<Item = (usize, &'a [PhaseCenterCell], &'a [PhaseCenterCell])>,
    {
        self.offload_decisions_for_into(tasks, policy, decision_scratch)?;
        Ok(PhaseCenterOffloadSummary::from_decision_slice_into(
            decision_scratch,
            margin_scratch,
        ))
    }
}

#[must_use]
pub fn phase_margin_from_centers(
    correct_vec: &[PhaseCenterCell],
    wrong_vec: &[PhaseCenterCell],
    positive_center: &[PhaseCenterCell],
    negative_center: &[PhaseCenterCell],
) -> f64 {
    if correct_vec.len() == wrong_vec.len()
        && correct_vec.len() == positive_center.len()
        && correct_vec.len() == negative_center.len()
    {
        if correct_vec.is_empty() {
            return 0.0;
        }
        let mut score = 0.0f64;
        for (((correct, wrong), positive), negative) in correct_vec
            .iter()
            .zip(wrong_vec.iter())
            .zip(positive_center.iter())
            .zip(negative_center.iter())
        {
            let vector_delta_re = correct.re - wrong.re;
            let vector_delta_im = correct.im - wrong.im;
            let center_delta_re = positive.re - negative.re;
            let center_delta_im = positive.im - negative.im;
            score += vector_delta_re * center_delta_re + vector_delta_im * center_delta_im;
        }
        return score / correct_vec.len() as f64;
    }

    let correct_pos = phase_coherence(correct_vec, positive_center);
    let wrong_pos = phase_coherence(wrong_vec, positive_center);
    let correct_neg = phase_coherence(correct_vec, negative_center);
    let wrong_neg = phase_coherence(wrong_vec, negative_center);
    (correct_pos - correct_neg) - (wrong_pos - wrong_neg)
}

pub fn phase_margin_to_micro(margin: f64) -> Result<i64, PhaseCenterRuntimeError> {
    if !margin.is_finite() {
        return Err(PhaseCenterRuntimeError::InvalidMargin);
    }
    let scaled = (margin * 1_000_000.0).round();
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(PhaseCenterRuntimeError::InvalidMargin);
    }
    Ok(scaled as i64)
}

fn phase_component_to_fixed(value: f64) -> i32 {
    let clamped = value.clamp(-2.0, 2.0);
    (clamped * PHASE_CENTER_FIXED_CELL_SCALE as f64).round() as i32
}

fn fixed_phase_score_to_micro(score: i128, cells: usize) -> Result<i64, PhaseCenterRuntimeError> {
    if cells == 0 {
        return Err(PhaseCenterRuntimeError::EmptyRuntime);
    }
    let denominator = i128::from(PHASE_CENTER_FIXED_CELL_SCALE)
        * i128::from(PHASE_CENTER_FIXED_CELL_SCALE)
        * cells as i128;
    let numerator = score
        .checked_mul(1_000_000)
        .ok_or(PhaseCenterRuntimeError::InvalidMargin)?;
    let rounded = if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    };
    i64::try_from(rounded).map_err(|_| PhaseCenterRuntimeError::InvalidMargin)
}

#[must_use]
fn summarize_repeated_offload_decisions_into<F>(
    decision_count: usize,
    calls: usize,
    decision_at: F,
    margin_scratch: &mut Vec<i64>,
) -> PhaseCenterOffloadSummary
where
    F: Fn(usize) -> PhaseCenterOffloadDecision,
{
    margin_scratch.clear();
    if decision_count == 0 || calls == 0 {
        return PhaseCenterOffloadSummary::default();
    }

    if margin_scratch.capacity() < calls {
        margin_scratch.reserve(calls);
    }
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    for call_index in 0..calls {
        let decision = decision_at(call_index % decision_count);
        margin_scratch.push(decision.margin_micro);
        if decision.is_local_operator() {
            local_operator_calls += 1;
            if decision.is_false_local_accept() {
                false_local_accepts += 1;
            }
        } else {
            fallback_to_llm_calls += 1;
        }
    }
    margin_scratch.sort_unstable();
    let local_correct = local_operator_calls.saturating_sub(false_local_accepts);
    PhaseCenterOffloadSummary {
        calls,
        local_operator_calls,
        fallback_to_llm_calls,
        offload_rate_milli: phase_center_milli_ratio(local_operator_calls, calls),
        local_accuracy_milli: phase_center_milli_ratio(local_correct, local_operator_calls),
        false_local_accepts,
        median_margin_micro: phase_center_percentile_i64(margin_scratch, 50),
        p10_margin_micro: phase_center_percentile_i64(margin_scratch, 10),
    }
}

#[must_use]
fn phase_center_milli_ratio(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (numerator * 1000 + denominator / 2) / denominator
}

#[must_use]
fn phase_center_percentile_i64(sorted: &[i64], percentile: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() - 1) * percentile / 100;
    sorted[index]
}

#[must_use]
pub fn phase_coherence(vector: &[PhaseCenterCell], center: &[PhaseCenterCell]) -> f64 {
    if vector.is_empty() || center.is_empty() {
        return 0.0;
    }
    let mut active = 0usize;
    let mut score = 0.0f64;
    for (value, center) in vector.iter().zip(center.iter()) {
        active += 1;
        score += value.re * center.re + value.im * center.im;
    }
    if active == 0 {
        0.0
    } else {
        score / active as f64
    }
}

#[must_use]
pub fn phase_center_from_sum(values: &[PhaseCenterCell]) -> Vec<PhaseCenterCell> {
    values
        .iter()
        .map(|value| phase_circular_unit(*value))
        .collect()
}

pub fn add_phase_vector(target: &mut [PhaseCenterCell], source: &[PhaseCenterCell], sign: f64) {
    for (target_cell, source_cell) in target.iter_mut().zip(source.iter()) {
        target_cell.re += sign * source_cell.re;
        target_cell.im += sign * source_cell.im;
    }
}

#[must_use]
pub fn phase_vector_from_atoms<'a, I>(atoms: I, cells: usize) -> Vec<PhaseCenterCell>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut sums = Vec::new();
    fill_phase_vector_from_atoms_into(atoms, cells, &mut sums);
    sums
}

fn fill_phase_vector_from_atoms_into<'a, I>(atoms: I, cells: usize, out: &mut Vec<PhaseCenterCell>)
where
    I: IntoIterator<Item = &'a str>,
{
    out.clear();
    out.resize(cells, PhaseCenterCell::default());
    for atom in atoms {
        for (cell, sum) in out.iter_mut().enumerate() {
            let phase = stable_phase_cell(atom, cell);
            sum.re += phase.re;
            sum.im += phase.im;
        }
    }
    for cell in out {
        *cell = phase_circular_unit(*cell);
    }
}

#[must_use]
pub fn phase_vector_from_atom_ids<I>(atom_ids: I, cells: usize) -> Vec<PhaseCenterCell>
where
    I: IntoIterator<Item = u64>,
{
    let mut sums = Vec::new();
    fill_phase_vector_from_atom_ids_into(atom_ids, cells, &mut sums);
    sums
}

fn fill_phase_vector_from_atom_ids_into<I>(
    atom_ids: I,
    cells: usize,
    out: &mut Vec<PhaseCenterCell>,
) where
    I: IntoIterator<Item = u64>,
{
    out.clear();
    out.resize(cells, PhaseCenterCell::default());
    for atom_id in atom_ids {
        for (cell, sum) in out.iter_mut().enumerate() {
            let phase = stable_phase_atom_id_cell(atom_id, cell);
            sum.re += phase.re;
            sum.im += phase.im;
        }
    }
    for cell in out {
        *cell = phase_circular_unit(*cell);
    }
}

#[must_use]
pub fn stable_phase_cell(atom: &str, cell: usize) -> PhaseCenterCell {
    let input = format!("{cell}\0{atom}");
    let hash = blake2b8_personalized(input.as_bytes(), b"nwphase");
    let angle = (hash as f64 / (u64::MAX as f64 + 1.0)) * std::f64::consts::TAU;
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

#[must_use]
pub fn stable_phase_atom_id_cell(atom_id: u64, cell: usize) -> PhaseCenterCell {
    let hash = mix_phase_atom_id(atom_id, cell as u64);
    let angle = (hash as f64 / (u64::MAX as f64 + 1.0)) * std::f64::consts::TAU;
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn mix_phase_atom_id(atom_id: u64, cell: u64) -> u64 {
    let mut value =
        atom_id ^ cell.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17) ^ 0x6e77_7770_6361_746f;
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[must_use]
pub fn phase_circular_unit(value: PhaseCenterCell) -> PhaseCenterCell {
    let magnitude = (value.re * value.re + value.im * value.im).sqrt();
    if magnitude == 0.0 {
        PhaseCenterCell::default()
    } else {
        PhaseCenterCell {
            re: value.re / magnitude,
            im: value.im / magnitude,
        }
    }
}

fn blake2b8_personalized(input: &[u8], personal: &[u8]) -> u64 {
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    let mut h = IV;
    h[0] ^= 0x01010008;
    h[6] ^= le_u64_padded(personal, 0);
    h[7] ^= le_u64_padded(personal, 8);

    let mut offset = 0usize;
    while offset < input.len() || (input.is_empty() && offset == 0) {
        let remaining = input.len().saturating_sub(offset);
        let block_len = remaining.min(128);
        let mut block = [0u8; 128];
        if block_len > 0 {
            block[..block_len].copy_from_slice(&input[offset..offset + block_len]);
        }
        offset += block_len;
        let is_last = offset >= input.len();
        blake2b_compress(&mut h, &block, offset as u128, is_last);
        if is_last {
            break;
        }
    }
    h[0]
}

fn le_u64_padded(bytes: &[u8], start: usize) -> u64 {
    let mut out = [0u8; 8];
    for (dst, src) in out.iter_mut().zip(bytes.iter().skip(start).take(8)) {
        *dst = *src;
    }
    u64::from_le_bytes(out)
}

fn blake2b_compress(h: &mut [u64; 8], block: &[u8; 128], counter: u128, is_last: bool) {
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    const SIGMA: [[usize; 16]; 12] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    ];

    let mut m = [0u64; 16];
    for (index, chunk) in block.chunks_exact(8).enumerate() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(chunk);
        m[index] = u64::from_le_bytes(bytes);
    }

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV);
    v[12] ^= counter as u64;
    v[13] ^= (counter >> 64) as u64;
    if is_last {
        v[14] = !v[14];
    }

    for schedule in SIGMA {
        blake2b_g(&mut v, 0, 4, 8, 12, m[schedule[0]], m[schedule[1]]);
        blake2b_g(&mut v, 1, 5, 9, 13, m[schedule[2]], m[schedule[3]]);
        blake2b_g(&mut v, 2, 6, 10, 14, m[schedule[4]], m[schedule[5]]);
        blake2b_g(&mut v, 3, 7, 11, 15, m[schedule[6]], m[schedule[7]]);
        blake2b_g(&mut v, 0, 5, 10, 15, m[schedule[8]], m[schedule[9]]);
        blake2b_g(&mut v, 1, 6, 11, 12, m[schedule[10]], m[schedule[11]]);
        blake2b_g(&mut v, 2, 7, 8, 13, m[schedule[12]], m[schedule[13]]);
        blake2b_g(&mut v, 3, 4, 9, 14, m[schedule[14]], m[schedule[15]]);
    }

    for index in 0..8 {
        h[index] ^= v[index] ^ v[index + 8];
    }
}

fn blake2b_g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

fn runtime_package_len(cells: usize, records: usize) -> Option<usize> {
    records
        .checked_mul(2)?
        .checked_mul(cells)?
        .checked_mul(2)?
        .checked_mul(std::mem::size_of::<f64>())?
        .checked_add(PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES)
}

fn hot_runtime_package_len(
    cells: usize,
    profile_count: usize,
    route_count: usize,
    route_profile_edges: usize,
) -> Option<usize> {
    let profile_bytes = profile_count
        .checked_mul(std::mem::size_of::<u32>() + std::mem::size_of::<i64>())?
        .checked_add(
            profile_count
                .checked_mul(cells)?
                .checked_mul(2)?
                .checked_mul(std::mem::size_of::<f64>())?,
        )?;
    let route_bytes = route_count
        .checked_mul(2)?
        .checked_mul(std::mem::size_of::<u32>())?
        .checked_add(route_profile_edges.checked_mul(std::mem::size_of::<u32>())?)?;
    PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES
        .checked_add(profile_bytes)?
        .checked_add(route_bytes)
}

fn hot_runtime_package_info_for_runtime(
    hot_runtime: &PhaseCenterHotRuntime,
    route_table: &PhaseCenterHotRouteTable,
    verifier_binding: PhaseCenterVerifierBinding,
    policy_defaults: PhaseCenterHotPackagePolicyDefaults,
    serialized_len: usize,
    fingerprint64: u64,
) -> Result<PhaseCenterHotRuntimePackageInfo, PhaseCenterRuntimeError> {
    let serialized_len = if serialized_len == 0 {
        hot_runtime_package_len(
            hot_runtime.cells(),
            hot_runtime.profile_count(),
            route_table.route_count(),
            route_table.profile_edge_count(),
        )
        .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?
    } else {
        serialized_len
    };
    hot_runtime_package_info(
        hot_runtime.cells(),
        hot_runtime.profile_count(),
        route_table.route_count(),
        route_table.profile_edge_count(),
        verifier_binding,
        policy_defaults,
        serialized_len,
        fingerprint64,
    )
}

#[allow(clippy::too_many_arguments)]
fn hot_runtime_package_info(
    cells: usize,
    profile_count: usize,
    route_count: usize,
    route_profile_edges: usize,
    verifier_binding: PhaseCenterVerifierBinding,
    policy_defaults: PhaseCenterHotPackagePolicyDefaults,
    serialized_len: usize,
    fingerprint64: u64,
) -> Result<PhaseCenterHotRuntimePackageInfo, PhaseCenterRuntimeError> {
    let expected_len =
        hot_runtime_package_len(cells, profile_count, route_count, route_profile_edges)
            .ok_or(PhaseCenterRuntimeError::RuntimePackageTooLarge)?;
    if serialized_len != expected_len {
        return Err(PhaseCenterRuntimeError::InvalidRuntimePackage);
    }
    let hot_runtime_bytes_estimate =
        PhaseCenterHotRuntime::bytes_estimate_for(profile_count, cells);
    let hot_route_table_bytes_estimate =
        PhaseCenterHotRouteTable::bytes_estimate_for(route_count, route_profile_edges);
    let hot_scratch_bytes_estimate = PhaseCenterHotScratch::bytes_estimate_for(
        cells,
        route_profile_edges.max(1),
        PHASE_CENTER_DEFAULT_HOT_ATOM_ROW_CACHE,
    );
    Ok(PhaseCenterHotRuntimePackageInfo {
        magic: PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC,
        cells,
        profile_count,
        route_count,
        route_profile_edges,
        serialized_len,
        payload_bytes: serialized_len - PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES,
        fingerprint64,
        verifier_binding,
        policy_defaults,
        hot_runtime_bytes_estimate,
        hot_route_table_bytes_estimate,
        hot_scratch_bytes_estimate,
        hot_bytes_estimate: hot_runtime_bytes_estimate
            .saturating_add(hot_route_table_bytes_estimate)
            .saturating_add(hot_scratch_bytes_estimate),
    })
}

fn runtime_package_fingerprint64(bytes: &[u8]) -> u64 {
    blake2b8_personalized(bytes, &PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL)
}

fn hot_runtime_package_fingerprint64(bytes: &[u8]) -> u64 {
    blake2b8_personalized(
        bytes,
        &PHASE_CENTER_HOT_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL,
    )
}

fn write_phase_center_cells(bytes: &mut Vec<u8>, cells: &[PhaseCenterCell]) {
    for cell in cells {
        bytes.extend_from_slice(&cell.re.to_le_bytes());
        bytes.extend_from_slice(&cell.im.to_le_bytes());
    }
}

fn read_phase_center_cells(
    bytes: &[u8],
    offset: &mut usize,
    cells: usize,
) -> Result<Box<[PhaseCenterCell]>, PhaseCenterRuntimeError> {
    let mut out = Vec::with_capacity(cells);
    for _ in 0..cells {
        let re = read_f64_le(bytes, *offset)?;
        *offset += std::mem::size_of::<f64>();
        let im = read_f64_le(bytes, *offset)?;
        *offset += std::mem::size_of::<f64>();
        out.push(PhaseCenterCell { re, im });
    }
    Ok(out.into_boxed_slice())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<u32>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 4];
    out.copy_from_slice(chunk);
    Ok(u32::from_le_bytes(out))
}

fn read_u32_le_at(bytes: &[u8], offset: &mut usize) -> Result<u32, PhaseCenterRuntimeError> {
    let value = read_u32_le(bytes, *offset)?;
    *offset += std::mem::size_of::<u32>();
    Ok(value)
}

fn read_u64_le_at(bytes: &[u8], offset: &mut usize) -> Result<u64, PhaseCenterRuntimeError> {
    let end = offset
        .checked_add(8)
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let raw: [u8; 8] = bytes
        .get(*offset..end)
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?
        .try_into()
        .map_err(|_| PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    *offset = end;
    Ok(u64::from_le_bytes(raw))
}

fn read_usize_u64_at(bytes: &[u8], offset: &mut usize) -> Result<usize, PhaseCenterRuntimeError> {
    usize::try_from(read_u64_le_at(bytes, offset)?)
        .map_err(|_| PhaseCenterRuntimeError::RuntimePackageTooLarge)
}

fn read_i64_le(bytes: &[u8], offset: usize) -> Result<i64, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<i64>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(chunk);
    Ok(i64::from_le_bytes(out))
}

fn read_i64_le_at(bytes: &[u8], offset: &mut usize) -> Result<i64, PhaseCenterRuntimeError> {
    let value = read_i64_le(bytes, *offset)?;
    *offset += std::mem::size_of::<i64>();
    Ok(value)
}

fn read_f64_le(bytes: &[u8], offset: usize) -> Result<f64, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<f64>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(chunk);
    Ok(f64::from_le_bytes(out))
}

#[cfg(test)]
mod tests;
