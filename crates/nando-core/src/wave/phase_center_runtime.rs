//! Flat CPU runtime for phase-center operator scoring.
//!
//! This module intentionally contains no corpus loading, no lookup table of
//! answers, and no training loop. It scores a candidate transition against
//! precompiled positive/negative phase centers.

pub const PHASE_CENTER_RUNTIME_PACKAGE_MAGIC: [u8; 8] = *b"NWPCF001";
pub const PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL: [u8; 8] = *b"nwpcpkg1";
pub const PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES: usize = 16;
pub const PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO: i64 = 300_000;
const PHASE_CENTER_DEFAULT_HOT_ATOM_ROW_CACHE: usize = 64;

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
                    if evict_key.map_or(true, |current| key < current) {
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

fn runtime_package_fingerprint64(bytes: &[u8]) -> u64 {
    blake2b8_personalized(bytes, &PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL)
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

fn read_f64_le(bytes: &[u8], offset: usize) -> Result<f64, PhaseCenterRuntimeError> {
    let chunk = bytes
        .get(offset..offset + std::mem::size_of::<f64>())
        .ok_or(PhaseCenterRuntimeError::InvalidRuntimePackage)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(chunk);
    Ok(f64::from_le_bytes(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_threshold_policy() -> PhaseCenterThresholdPolicyEvidence {
        PhaseCenterThresholdPolicyEvidence {
            candidate_bucket_count: 1,
            auto_calibrated_bucket_count: 1,
            calibration_window_before_shadow: true,
            shadow_window_after_calibration: true,
            per_bucket_thresholds_reported: true,
            fixed_policy_shadow_replay: true,
        }
    }

    fn test_verifier_binding() -> PhaseCenterVerifierBinding {
        PhaseCenterVerifierBinding {
            verifier_id: 11,
            verifier_version: 1,
            verifier_input_kind_id: 22,
            verifier_evidence_source_id: 33,
            false_accept_threshold: 0,
        }
    }

    fn promotion_evidence(
        future_shadow_events: usize,
        unique_accepts: usize,
        tokens_saved: u64,
        cost_saved_microusd: u64,
    ) -> PhaseCenterPromotionEvidence {
        PhaseCenterPromotionEvidence {
            future_shadow_events,
            unique_cpu_accepts_over_exact_cache: unique_accepts,
            tokens_saved,
            cost_saved_microusd,
            false_accepts: 0,
            runtime_margin_parity_mismatches: 0,
            verifier_binding: test_verifier_binding(),
            threshold_policy: passing_threshold_policy(),
            exact_cache_overlap_excluded: true,
            token_cost_denominator_present: true,
            local_accept_enabled: false,
        }
    }

    #[test]
    fn phase_hash_is_unit_and_deterministic() {
        let a = stable_phase_cell("rel:o0:s1", 7);
        let b = stable_phase_cell("rel:o0:s1", 7);
        let magnitude = (a.re * a.re + a.im * a.im).sqrt();
        assert_eq!(a, b);
        assert!((magnitude - 1.0).abs() < 1e-12);
    }

    #[test]
    fn runtime_scores_correct_transition_above_wrong() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let task = PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.into_boxed_slice(),
            wrong_vec: negative.into_boxed_slice(),
        };
        assert!(runtime.margin(&task).expect("valid task") > 0.0);
    }

    #[test]
    fn offload_policy_rejects_invalid_threshold() {
        assert_eq!(
            PhaseCenterOffloadPolicy::new(0),
            Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
        );
        assert_eq!(
            PhaseCenterOffloadPolicy::new(-1),
            Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
        );
    }

    #[test]
    fn offload_policy_routes_by_margin_micro_threshold() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid threshold");
        let local = policy.decide_margin(0.3004).expect("finite margin");
        let fallback = policy.decide_margin(0.2994).expect("finite margin");
        assert_eq!(local.margin_micro, 300_400);
        assert_eq!(local.action, PhaseCenterOffloadAction::LocalOperator);
        assert!(local.is_local_operator());
        assert_eq!(fallback.margin_micro, 299_400);
        assert_eq!(fallback.action, PhaseCenterOffloadAction::FallbackToLlm);
        assert!(fallback.is_fallback_to_llm());
    }

    #[test]
    fn offload_policy_rejects_nonfinite_margin() {
        let policy = PhaseCenterOffloadPolicy::default_conservative();
        assert_eq!(
            policy.decide_margin(f64::NAN),
            Err(PhaseCenterRuntimeError::InvalidMargin)
        );
        assert_eq!(
            phase_margin_to_micro(f64::INFINITY),
            Err(PhaseCenterRuntimeError::InvalidMargin)
        );
    }

    #[test]
    fn runtime_offload_decision_uses_packaged_margin() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let task = PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.into_boxed_slice(),
            wrong_vec: negative.into_boxed_slice(),
        };
        let decision = runtime
            .offload_decision(&task, policy)
            .expect("valid offload decision");
        assert!(decision.is_local_operator());
        assert!(decision.margin_micro > 0);
    }

    #[test]
    fn runtime_offload_decisions_batch_matches_per_task_decisions() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let batch = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let per_task = tasks
            .iter()
            .map(|task| {
                runtime
                    .offload_decision(task, policy)
                    .expect("valid per-task decision")
            })
            .collect::<Vec<_>>();
        assert_eq!(batch, per_task);
        assert!(batch[0].is_local_operator());
        assert!(batch[1].is_fallback_to_llm());
    }

    #[test]
    fn runtime_offload_decisions_into_reuses_caller_buffer() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let expected = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let mut out = Vec::with_capacity(8);
        let original_capacity = out.capacity();
        runtime
            .offload_decisions_into(&tasks, policy, &mut out)
            .expect("valid reused-buffer batch decisions");
        assert_eq!(out, expected);
        assert_eq!(out.capacity(), original_capacity);

        runtime
            .offload_decisions_into(tasks.iter().take(1), policy, &mut out)
            .expect("valid shorter reused-buffer batch decisions");
        assert_eq!(out.len(), 1);
        assert_eq!(out.capacity(), original_capacity);
    }

    #[test]
    fn runtime_offload_decisions_for_batch_reports_first_error() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let valid_task = (0, positive.as_slice(), negative.as_slice());
        let invalid_width = (0, positive[..7].as_ref(), negative.as_slice());
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        assert_eq!(
            runtime.offload_decisions_for([valid_task, invalid_width], policy),
            Err(PhaseCenterRuntimeError::VectorWidthMismatch)
        );
    }

    #[test]
    fn runtime_offload_decisions_for_into_reuses_buffer_and_reports_error() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let mut out = Vec::with_capacity(4);
        let original_capacity = out.capacity();
        runtime
            .offload_decisions_for_into(
                [(0, positive.as_slice(), negative.as_slice())],
                policy,
                &mut out,
            )
            .expect("valid raw-slice batch decisions");
        assert_eq!(out.len(), 1);
        assert_eq!(out.capacity(), original_capacity);
        assert!(out[0].is_local_operator());

        assert_eq!(
            runtime.offload_decisions_for_into(
                [(0, positive[..7].as_ref(), negative.as_slice())],
                policy,
                &mut out,
            ),
            Err(PhaseCenterRuntimeError::VectorWidthMismatch)
        );
        assert!(out.is_empty());
        assert_eq!(out.capacity(), original_capacity);
    }

    #[test]
    fn runtime_offload_summary_into_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let expected_decisions = runtime
            .offload_decisions(&tasks, policy)
            .expect("valid batch decisions");
        let expected_summary = PhaseCenterOffloadSummary::from_decision_slice(&expected_decisions);
        let mut decision_scratch = Vec::with_capacity(8);
        let mut margin_scratch = Vec::with_capacity(8);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();

        let summary = runtime
            .offload_summary_into(&tasks, policy, &mut decision_scratch, &mut margin_scratch)
            .expect("valid summary");

        assert_eq!(decision_scratch, expected_decisions);
        assert_eq!(summary, expected_summary);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn runtime_offload_summary_for_into_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let mut decision_scratch = Vec::with_capacity(4);
        let mut margin_scratch = Vec::with_capacity(4);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();
        let summary = runtime
            .offload_summary_for_into(
                [
                    (0, positive.as_slice(), negative.as_slice()),
                    (0, negative.as_slice(), positive.as_slice()),
                ],
                policy,
                &mut decision_scratch,
                &mut margin_scratch,
            )
            .expect("valid raw-slice summary");

        assert_eq!(summary.calls, 2);
        assert_eq!(summary.local_operator_calls, 1);
        assert_eq!(summary.fallback_to_llm_calls, 1);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn offload_runtime_from_package_bytes_reuses_caller_buffers() {
        let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
        let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
        let runtime = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid runtime");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&bytes).expect("sdk inspects");
        assert_eq!(
            package_info,
            PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects")
        );
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let offload_runtime =
            PhaseCenterOffloadRuntime::from_package_bytes(&bytes, policy).expect("sdk loads");
        assert_eq!(offload_runtime.package_info(), package_info);
        assert_eq!(offload_runtime.policy(), policy);
        assert_eq!(offload_runtime.cells(), 8);
        assert_eq!(offload_runtime.record_count(), 1);
        assert_eq!(offload_runtime.bytes_estimate(), runtime.bytes_estimate());
        assert_eq!(
            offload_runtime.runtime().record_count(),
            runtime.record_count()
        );

        let tasks = vec![
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: positive.clone().into_boxed_slice(),
                wrong_vec: negative.clone().into_boxed_slice(),
            },
            PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: negative.clone().into_boxed_slice(),
                wrong_vec: positive.clone().into_boxed_slice(),
            },
        ];
        let expected_summary = runtime
            .offload_summary_into(
                &tasks,
                policy,
                &mut Vec::with_capacity(2),
                &mut Vec::with_capacity(2),
            )
            .expect("runtime summary");
        let mut decision_scratch = Vec::with_capacity(4);
        let mut margin_scratch = Vec::with_capacity(4);
        let decision_capacity = decision_scratch.capacity();
        let margin_capacity = margin_scratch.capacity();
        let summary = offload_runtime
            .offload_summary_into(&tasks, &mut decision_scratch, &mut margin_scratch)
            .expect("sdk summary");
        assert_eq!(summary, expected_summary);
        assert_eq!(decision_scratch.capacity(), decision_capacity);
        assert_eq!(margin_scratch.capacity(), margin_capacity);
    }

    #[test]
    fn offload_runtime_rejects_bad_package_bytes() {
        let policy = PhaseCenterOffloadPolicy::default_conservative();
        assert_eq!(
            PhaseCenterOffloadRuntime::inspect_package_bytes(b"bad"),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
        assert_eq!(
            PhaseCenterOffloadRuntime::from_package_bytes(b"bad", policy),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
    }

    #[test]
    fn offload_summary_counts_unique_decisions_and_false_local_accepts() {
        let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
        let local = policy.decide_margin(0.5).expect("finite margin");
        let fallback = policy.decide_margin(-0.1).expect("finite margin");
        let false_local = PhaseCenterOffloadDecision {
            action: PhaseCenterOffloadAction::LocalOperator,
            margin_micro: 0,
            margin_threshold_micro: 1,
        };
        let summary = PhaseCenterOffloadSummary::from_decisions([local, fallback, false_local]);
        assert_eq!(
            summary,
            PhaseCenterOffloadSummary {
                calls: 3,
                local_operator_calls: 2,
                fallback_to_llm_calls: 1,
                offload_rate_milli: 667,
                local_accuracy_milli: 500,
                false_local_accepts: 1,
                median_margin_micro: 0,
                p10_margin_micro: -100_000,
            }
        );
    }

    #[test]
    fn offload_summary_repeats_decision_ring_for_simulated_calls() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
        let local = policy.decide_margin(0.4).expect("finite margin");
        let fallback = policy.decide_margin(0.2).expect("finite margin");
        let summary = PhaseCenterOffloadSummary::from_repeated_decisions([local, fallback], 5);
        assert_eq!(summary.calls, 5);
        assert_eq!(summary.local_operator_calls, 3);
        assert_eq!(summary.fallback_to_llm_calls, 2);
        assert_eq!(summary.offload_rate_milli, 600);
        assert_eq!(summary.local_accuracy_milli, 1000);
        assert_eq!(summary.false_local_accepts, 0);
        assert_eq!(summary.median_margin_micro, 400_000);
        assert_eq!(summary.p10_margin_micro, 200_000);
    }

    #[test]
    fn offload_summary_into_reuses_caller_margin_scratch() {
        let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
        let local = policy.decide_margin(0.4).expect("finite margin");
        let fallback = policy.decide_margin(0.2).expect("finite margin");
        let decisions = [local, fallback];
        let mut margin_scratch = Vec::with_capacity(8);
        let original_capacity = margin_scratch.capacity();
        let unique =
            PhaseCenterOffloadSummary::from_decision_slice_into(&decisions, &mut margin_scratch);
        assert_eq!(unique.calls, 2);
        assert_eq!(unique.local_operator_calls, 1);
        assert_eq!(unique.fallback_to_llm_calls, 1);
        assert_eq!(margin_scratch, [200_000, 400_000]);
        assert_eq!(margin_scratch.capacity(), original_capacity);

        let repeated = PhaseCenterOffloadSummary::from_repeated_decision_fn_into(
            decisions.len(),
            5,
            |index| decisions[index],
            &mut margin_scratch,
        );
        assert_eq!(repeated.calls, 5);
        assert_eq!(repeated.local_operator_calls, 3);
        assert_eq!(repeated.fallback_to_llm_calls, 2);
        assert_eq!(repeated.median_margin_micro, 400_000);
        assert_eq!(
            margin_scratch,
            [200_000, 200_000, 400_000, 400_000, 400_000]
        );
        assert_eq!(margin_scratch.capacity(), original_capacity);
    }

    #[test]
    fn atom_encoder_matches_allocating_phase_vector_and_reuses_scratch() {
        let expected = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let capacity_before = encoder.scratch_capacity();
        let encoded = encoder
            .encode_atoms(["family:test_output_parse", "state:exit0", "result:pass"])
            .expect("atoms encode")
            .to_vec();
        assert_eq!(encoded, expected);
        assert_eq!(encoder.cells(), 16);
        assert_eq!(encoder.scratch_capacity(), capacity_before);

        let other = encoder
            .encode_atoms(["family:test_output_parse", "state:panic", "result:fail"])
            .expect("second atoms encode")
            .to_vec();
        let other_expected = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );
        assert_eq!(other, other_expected);
        assert_eq!(encoder.scratch_capacity(), capacity_before);
        assert_eq!(
            PhaseCenterAtomEncoder::new(0),
            Err(PhaseCenterRuntimeError::EmptyRuntime)
        );
    }

    #[test]
    fn atom_id_encoder_matches_allocating_phase_vector_and_reuses_scratch() {
        let expected = phase_vector_from_atom_ids([101, 202, 303], 16);
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let capacity_before = encoder.scratch_capacity();
        let encoded = encoder
            .encode_atom_ids([101, 202, 303])
            .expect("atom ids encode")
            .to_vec();
        assert_eq!(encoded, expected);
        assert_eq!(encoder.scratch_capacity(), capacity_before);

        let other = encoder
            .encode_atom_ids([101, 404, 505])
            .expect("second atom ids encode")
            .to_vec();
        assert_ne!(other, expected);
        assert_eq!(other, phase_vector_from_atom_ids([101, 404, 505], 16));
        assert_eq!(encoder.scratch_capacity(), capacity_before);

        let cell = stable_phase_atom_id_cell(101, 7);
        let magnitude = (cell.re * cell.re + cell.im * cell.im).sqrt();
        assert!((magnitude - 1.0).abs() < 0.000_000_001);
        assert_eq!(
            stable_phase_atom_id_cell(101, 7),
            stable_phase_atom_id_cell(101, 7)
        );
        assert_ne!(
            stable_phase_atom_id_cell(101, 7),
            stable_phase_atom_id_cell(101, 8)
        );
    }

    #[test]
    fn online_miner_learns_then_scores_future_events() {
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );

        let first = miner
            .observe(7, &positive, true, false, 100, 300)
            .expect("first event accepted");
        let second = miner
            .observe(7, &negative, false, false, 100, 300)
            .expect("second event accepted");
        assert!(!first.active_before_update);
        assert!(!second.active_before_update);

        let calibration_positive = miner
            .observe(7, &positive, true, false, 100, 300)
            .expect("positive calibration event accepted");
        let calibration_negative = miner
            .observe(7, &negative, false, false, 100, 300)
            .expect("negative calibration event accepted");
        assert!(calibration_positive.calibration_event);
        assert!(calibration_negative.calibration_event);
        assert!(!calibration_positive.local_operator_shadow_decision);
        assert!(!calibration_negative.local_operator_shadow_decision);

        let accepted = miner
            .observe(7, &positive, true, false, 123, 456)
            .expect("future positive event scored");
        let rejected_wrong = miner
            .observe(7, &negative, false, false, 123, 456)
            .expect("future negative event scored");
        assert!(accepted.active_before_update);
        assert!(!accepted.calibration_event);
        assert!(accepted.raw_local_operator);
        assert!(accepted.local_operator_shadow_decision);
        assert!(accepted.unique_cpu_accept_over_exact_cache);
        assert!(!accepted.false_accept);
        assert!(!rejected_wrong.raw_local_operator);
        assert!(!rejected_wrong.false_accept);

        let summary = miner.summary();
        assert_eq!(summary.bucket_count, 1);
        assert_eq!(summary.active_bucket_count, 1);
        assert_eq!(summary.candidate_bucket_count, 1);
        assert_eq!(summary.rejected_bucket_count, 0);
        assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(summary.tokens_saved, 123);
        assert_eq!(summary.cost_saved_microusd, 456);
        assert_eq!(summary.false_accepts, 0);

        let runtime = miner
            .candidate_runtime(7)
            .expect("candidate runtime builds")
            .expect("safe bucket emits candidate runtime");
        assert_eq!(runtime.record_count(), 1);
        assert!(runtime.margin_for(0, &positive, &negative).expect("margin") > 0.0);

        let bucket = miner.bucket(7).expect("bucket exists");
        assert!(bucket.trust_quality_micro > 0);
        assert_eq!(bucket.trust_false_risk_micro, 0);
        assert!(bucket.trust_drift_micro > 0);
        assert!(bucket.trust_token_value_micro > 0);
    }

    #[test]
    fn online_miner_waits_for_false_margin_before_shadow_accept() {
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );

        miner
            .observe(11, &positive, true, false, 0, 0)
            .expect("seed positive");
        miner
            .observe(11, &negative, false, false, 0, 0)
            .expect("seed negative");
        let calibration_positive = miner
            .observe(11, &positive, true, false, 100, 300)
            .expect("positive-only calibration");
        assert!(calibration_positive.active_before_update);
        assert!(calibration_positive.calibration_event);
        assert!(!calibration_positive.raw_local_operator);

        let first_false_margin = miner
            .observe(11, &negative, false, false, 100, 300)
            .expect("first false margin calibrates threshold");
        assert!(first_false_margin.active_before_update);
        assert!(first_false_margin.calibration_event);
        assert!(!first_false_margin.raw_local_operator);
        assert!(!first_false_margin.false_accept);

        let accepted = miner
            .observe(11, &positive, true, false, 123, 456)
            .expect("future positive scored after false-margin calibration");
        assert!(!accepted.calibration_event);
        assert!(accepted.raw_local_operator);
        assert!(accepted.unique_cpu_accept_over_exact_cache);

        let summary = miner.summary();
        assert_eq!(summary.false_accepts, 0);
        assert_eq!(summary.rejected_bucket_count, 0);
        assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
    }

    #[test]
    fn online_miner_quarantines_bucket_after_verified_false_accept() {
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );

        miner
            .observe(9, &positive, true, false, 0, 0)
            .expect("seed positive");
        miner
            .observe(9, &negative, false, false, 0, 0)
            .expect("seed negative");
        miner
            .observe(9, &positive, true, false, 0, 0)
            .expect("calibration positive");
        miner
            .observe(9, &negative, false, false, 0, 0)
            .expect("calibration negative");

        let unsafe_decision = miner
            .observe(9, &positive, false, false, 500, 700)
            .expect("unsafe event scored");
        assert!(unsafe_decision.raw_local_operator);
        assert!(unsafe_decision.false_accept);
        assert!(!unsafe_decision.local_operator_shadow_decision);
        assert!(!unsafe_decision.unique_cpu_accept_over_exact_cache);

        let summary = miner.summary();
        assert_eq!(summary.rejected_bucket_count, 1);
        assert_eq!(summary.candidate_bucket_count, 0);
        assert_eq!(summary.false_accepts, 1);
        let bucket = miner.bucket(9).expect("bucket exists");
        assert!(bucket.rejected);
        assert!(bucket.trust_false_risk_micro > 0);
        assert_eq!(
            bucket.learned_threshold_micro,
            unsafe_decision.margin_micro.saturating_add(1)
        );
        let after_quarantine_positive = miner
            .observe(9, &positive, true, false, 500, 700)
            .expect("quarantined bucket still learns but does not accept");
        assert!(after_quarantine_positive.active_before_update);
        assert!(!after_quarantine_positive.raw_local_operator);
        assert!(!after_quarantine_positive.local_operator_shadow_decision);
        assert!(!after_quarantine_positive.unique_cpu_accept_over_exact_cache);
        assert!(
            miner
                .candidate_runtime(9)
                .expect("candidate check")
                .is_none()
        );
    }

    #[test]
    fn live_operator_store_tracks_mutable_budget_and_verifier_bound_export() {
        let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
            miner: PhaseCenterOnlineMinerConfig {
                cells: 16,
                min_bucket_events: 2,
                threshold_floor_micro: 1,
                calibration_events: 2,
                max_buckets: 4,
            },
            memory: PhaseCenterOperatorMemoryConfig {
                max_hot_profiles_per_worker: 4,
                max_hot_bytes_per_worker: 64 * 1024,
                max_warm_profiles_per_process: 4,
                max_profiles_per_route: 2,
                max_route_top_k: 2,
                min_tokens_saved: 1,
                min_accept_rate_milli: 1,
                false_accepts_must_be_zero: true,
            },
        })
        .expect("valid live operator store");
        let positive = phase_vector_from_atom_ids([101, 201, 301], 16);
        let negative = phase_vector_from_atom_ids([101, 999, 998], 16);

        for (verified_safe_accept, vector) in [
            (true, &positive),
            (false, &negative),
            (true, &positive),
            (false, &negative),
        ] {
            store
                .observe(71, vector, verified_safe_accept, false, 10, 30)
                .expect("live event observed");
        }

        let future = store
            .observe(71, &positive, true, false, 55, 165)
            .expect("future event scored");
        assert!(future.active_before_update);
        assert!(future.local_operator_shadow_decision);
        assert!(future.unique_cpu_accept_over_exact_cache);
        assert!(!future.false_accept);

        let summary = store.summary();
        assert_eq!(summary.bucket_count, 1);
        assert_eq!(summary.candidate_bucket_count, 1);
        assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(summary.tokens_saved, 55);
        assert_eq!(summary.cost_saved_microusd, 165);
        assert_eq!(summary.false_accepts, 0);

        let snapshot = store.runtime_budget_snapshot();
        assert_eq!(snapshot.warm_route_count, 0);
        assert_eq!(snapshot.warm_profile_count, 1);
        assert_eq!(snapshot.hot_route_count, 0);
        assert_eq!(snapshot.hot_profile_count, 1);
        assert_eq!(snapshot.hot_route_profile_edges, 0);
        assert!(snapshot.warm_metadata_bytes_estimate > 0);
        assert!(snapshot.hot_runtime_bytes_estimate > 0);
        assert!(snapshot.hot_budget_passed());
        assert!(snapshot.warm_budget_passed());
        assert!(snapshot.product_runtime_budget_passed());

        let verifier_binding = test_verifier_binding();
        let mut packages = Vec::with_capacity(2);
        let capacity = packages.capacity();
        store
            .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
            .expect("verifier-bound candidates exported");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages.capacity(), capacity);
        assert_eq!(packages[0].bucket_id, 71);
        assert_eq!(packages[0].verifier_binding, verifier_binding);
        assert!(packages[0].verifier_binding.is_bound());
    }

    #[test]
    fn online_miner_ranks_candidate_recovery_by_tokens_before_call_count() {
        let config = PhaseCenterOnlineMinerConfig {
            cells: 4,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 1,
            max_buckets: 4,
        };
        let mut miner = PhaseCenterOnlineMiner::new(config).expect("valid miner");

        let mut call_heavy = PhaseCenterOnlineBucket::new(10, config);
        call_heavy.positive_events = 1;
        call_heavy.negative_events = 1;
        call_heavy.events_seen = 2;
        call_heavy.unique_cpu_accepts_over_exact_cache = 50;
        call_heavy.tokens_saved = 1_000;
        call_heavy.cost_saved_microusd = 1_000;

        let mut token_heavy = PhaseCenterOnlineBucket::new(20, config);
        token_heavy.positive_events = 1;
        token_heavy.negative_events = 1;
        token_heavy.events_seen = 2;
        token_heavy.unique_cpu_accepts_over_exact_cache = 2;
        token_heavy.tokens_saved = 10_000;
        token_heavy.cost_saved_microusd = 10_000;

        let mut token_tie_accept_heavy = PhaseCenterOnlineBucket::new(30, config);
        token_tie_accept_heavy.positive_events = 1;
        token_tie_accept_heavy.negative_events = 1;
        token_tie_accept_heavy.events_seen = 2;
        token_tie_accept_heavy.unique_cpu_accepts_over_exact_cache = 3;
        token_tie_accept_heavy.tokens_saved = 10_000;
        token_tie_accept_heavy.cost_saved_microusd = 9_000;

        miner
            .buckets
            .extend([call_heavy, token_heavy, token_tie_accept_heavy]);

        assert_eq!(miner.candidate_bucket_ids_limited(3), vec![30, 20, 10]);
    }

    #[test]
    fn live_operator_store_observes_numeric_route_atom_events() {
        let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
            miner: PhaseCenterOnlineMinerConfig {
                cells: 16,
                min_bucket_events: 2,
                threshold_floor_micro: 1,
                calibration_events: 2,
                max_buckets: 4,
            },
            memory: PhaseCenterOperatorMemoryConfig {
                max_hot_profiles_per_worker: 4,
                max_hot_bytes_per_worker: 64 * 1024,
                max_warm_profiles_per_process: 4,
                max_profiles_per_route: 2,
                max_route_top_k: 2,
                min_tokens_saved: 1,
                min_accept_rate_milli: 1,
                false_accepts_must_be_zero: true,
            },
        })
        .expect("valid live operator store");
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let positive_atoms = [10_001, 20_001, 30_001];
        let negative_atoms = [10_001, 20_999, 30_999];
        let evidence = PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 10,
            cost_microusd: 30,
        };
        assert_eq!(
            PhaseCenterLiveOperatorAtomEvent::new(700, 701, &positive_atoms, evidence).evidence(),
            evidence
        );

        for (verified_safe_accept, atom_ids, tokens, cost_microusd) in [
            (true, positive_atoms.as_slice(), 10, 30),
            (false, negative_atoms.as_slice(), 10, 30),
            (true, positive_atoms.as_slice(), 10, 30),
            (false, negative_atoms.as_slice(), 10, 30),
            (true, positive_atoms.as_slice(), 55, 165),
        ] {
            store
                .observe_atom_event(
                    &mut encoder,
                    PhaseCenterLiveOperatorAtomEvent::new(
                        700,
                        701,
                        atom_ids,
                        PhaseCenterHotRequestEvidence {
                            verified_safe_accept,
                            exact_cache_hit: false,
                            tokens,
                            cost_microusd,
                        },
                    ),
                )
                .expect("numeric atom event observed");
        }

        let route = store.route_stats(700).expect("route stats exist");
        assert_eq!(store.route_count(), 1);
        assert_eq!(store.route_bucket_count(), 1);
        assert_eq!(route.route_bucket_count, 1);
        assert_eq!(route.events_seen, 5);
        assert_eq!(route.scored_events, 3);
        assert_eq!(route.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(route.tokens_saved, 55);
        assert_eq!(route.cost_saved_microusd, 165);
        assert_eq!(route.false_accepts, 0);

        let snapshot = store.runtime_budget_snapshot();
        assert_eq!(snapshot.warm_route_count, 1);
        assert_eq!(snapshot.warm_profile_count, 1);
        assert_eq!(snapshot.hot_route_count, 1);
        assert_eq!(snapshot.hot_profile_count, 1);
        assert_eq!(snapshot.hot_route_profile_edges, 1);
        assert!(snapshot.product_runtime_budget_passed());
    }

    #[test]
    fn live_operator_store_exports_product_hot_runtime_without_package_roundtrip() {
        let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
            miner: PhaseCenterOnlineMinerConfig {
                cells: 16,
                min_bucket_events: 2,
                threshold_floor_micro: 1,
                calibration_events: 2,
                max_buckets: 4,
            },
            memory: PhaseCenterOperatorMemoryConfig {
                max_hot_profiles_per_worker: 4,
                max_hot_bytes_per_worker: 64 * 1024,
                max_warm_profiles_per_process: 4,
                max_profiles_per_route: 2,
                max_route_top_k: 2,
                min_tokens_saved: 1,
                min_accept_rate_milli: 1,
                false_accepts_must_be_zero: true,
            },
        })
        .expect("valid live operator store");
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let positive_atoms = [80_001, 80_002, 80_003];
        let negative_atoms = [80_001, 80_999, 80_998];

        for (verified_safe_accept, atom_ids, tokens, cost_microusd) in [
            (true, positive_atoms.as_slice(), 10, 30),
            (false, negative_atoms.as_slice(), 10, 30),
            (true, positive_atoms.as_slice(), 10, 30),
            (false, negative_atoms.as_slice(), 10, 30),
            (true, positive_atoms.as_slice(), 55, 165),
        ] {
            store
                .observe_atom_event(
                    &mut encoder,
                    PhaseCenterLiveOperatorAtomEvent::new(
                        8_000,
                        8_001,
                        atom_ids,
                        PhaseCenterHotRequestEvidence {
                            verified_safe_accept,
                            exact_cache_hit: false,
                            tokens,
                            cost_microusd,
                        },
                    ),
                )
                .expect("numeric atom event observed");
        }

        let (hot_runtime, route_table) = store
            .candidate_hot_runtime_and_route_table()
            .expect("direct product hot runtime builds")
            .expect("candidate hot runtime exists");
        assert_eq!(hot_runtime.profile_count(), 1);
        assert_eq!(route_table.route_count(), 1);
        assert_eq!(route_table.profile_edge_count(), 1);
        let route_index = route_table
            .resolve_route_index(8_000)
            .expect("route index exists");
        let mut scratch = PhaseCenterHotScratch::new(16, 1).expect("valid scratch");
        let decisions = hot_runtime
            .score_hot_request_candidates(
                &route_table,
                PhaseCenterHotRequest::new(route_index, &positive_atoms),
                &mut scratch,
            )
            .expect("product hot request scores");
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].profile_id, 8_001);
        assert!(decisions[0].score_candidate);
        assert!(decisions[0].verifier_required);
        assert!(!decisions[0].local_accept);

        let snapshot = store.runtime_budget_snapshot();
        assert_eq!(snapshot.hot_route_count, 1);
        assert_eq!(snapshot.hot_profile_count, 1);
        assert_eq!(snapshot.hot_route_profile_edges, 1);
        assert!(snapshot.product_runtime_budget_passed());
    }

    #[test]
    fn live_operator_store_rejects_false_accept_before_export() {
        let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
            miner: PhaseCenterOnlineMinerConfig {
                cells: 16,
                min_bucket_events: 2,
                threshold_floor_micro: 1,
                calibration_events: 2,
                max_buckets: 4,
            },
            memory: PhaseCenterOperatorMemoryConfig {
                max_hot_profiles_per_worker: 4,
                max_hot_bytes_per_worker: 64 * 1024,
                max_warm_profiles_per_process: 4,
                max_profiles_per_route: 2,
                max_route_top_k: 2,
                min_tokens_saved: 1,
                min_accept_rate_milli: 1,
                false_accepts_must_be_zero: true,
            },
        })
        .expect("valid live operator store");
        let positive = phase_vector_from_atom_ids([401, 501, 601], 16);
        let negative = phase_vector_from_atom_ids([401, 999, 998], 16);

        for (verified_safe_accept, vector) in [
            (true, &positive),
            (false, &negative),
            (true, &positive),
            (false, &negative),
        ] {
            store
                .observe(72, vector, verified_safe_accept, false, 0, 0)
                .expect("live event observed");
        }

        let false_accept = store
            .observe(72, &positive, false, false, 55, 165)
            .expect("unsafe event scored");
        assert!(false_accept.raw_local_operator);
        assert!(false_accept.false_accept);
        assert!(!false_accept.local_operator_shadow_decision);

        let summary = store.summary();
        assert_eq!(summary.rejected_bucket_count, 1);
        assert_eq!(summary.candidate_bucket_count, 0);
        assert_eq!(summary.false_accepts, 1);

        let snapshot = store.runtime_budget_snapshot();
        assert_eq!(snapshot.warm_profile_count, 1);
        assert_eq!(snapshot.hot_profile_count, 0);
        assert_eq!(snapshot.hot_bytes_estimate, 0);

        let mut packages = Vec::new();
        store
            .candidate_packages_into_with_verifier(test_verifier_binding(), &mut packages)
            .expect("export stays safe");
        assert!(packages.is_empty());
    }

    #[test]
    fn online_event_adapter_emits_verifier_bound_nwpc_package() {
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );
        let positive_event = PhaseCenterOnlineEvent {
            bucket_id: 11,
            vector: &positive,
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 100,
            cost_microusd: 300,
        };
        let negative_event = PhaseCenterOnlineEvent {
            bucket_id: 11,
            vector: &negative,
            verified_safe_accept: false,
            exact_cache_hit: false,
            tokens: 100,
            cost_microusd: 300,
        };

        miner
            .observe_event(positive_event)
            .expect("seed positive event");
        miner
            .observe_event(negative_event)
            .expect("seed negative event");
        miner
            .observe_event(positive_event)
            .expect("calibration positive event");
        miner
            .observe_event(negative_event)
            .expect("calibration negative event");
        let decision = miner
            .observe_event(positive_event)
            .expect("future positive event scored");
        assert!(decision.local_operator_shadow_decision);
        assert!(decision.unique_cpu_accept_over_exact_cache);

        let verifier_binding = test_verifier_binding();
        let package = miner
            .candidate_package_bytes_with_verifier(11, verifier_binding)
            .expect("candidate package builds")
            .expect("safe bucket emits package");
        assert_eq!(package.bucket_id, 11);
        assert!(package.threshold_micro > 0);
        assert_eq!(package.verifier_binding, verifier_binding);
        assert!(package.verifier_binding.is_bound());
        assert!(
            package
                .package_bytes
                .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
        );
        assert_eq!(package.package_info.cells, 16);
        assert_eq!(package.package_info.record_count, 1);
        assert_eq!(
            package.package_info.serialized_len,
            package.package_bytes.len()
        );
        let loaded = PhaseCenterFlatRuntime::from_bytes(&package.package_bytes)
            .expect("candidate package loads");
        assert!(loaded.margin_for(0, &positive, &negative).expect("margin") > 0.0);
    }

    #[test]
    fn online_stream_api_reuses_caller_buffers() {
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );
        let positive_event = PhaseCenterOnlineEvent {
            bucket_id: 13,
            vector: &positive,
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 17,
            cost_microusd: 23,
        };
        let negative_event = PhaseCenterOnlineEvent {
            bucket_id: 13,
            vector: &negative,
            verified_safe_accept: false,
            exact_cache_hit: false,
            tokens: 17,
            cost_microusd: 23,
        };
        let mut decisions = Vec::with_capacity(8);
        let decision_capacity = decisions.capacity();
        miner
            .observe_events_into(
                [
                    positive_event,
                    negative_event,
                    positive_event,
                    negative_event,
                    positive_event,
                ],
                &mut decisions,
            )
            .expect("stream events accepted");

        assert_eq!(decisions.len(), 5);
        assert_eq!(decisions.capacity(), decision_capacity);
        assert!(decisions[4].local_operator_shadow_decision);
        assert!(decisions[4].unique_cpu_accept_over_exact_cache);

        let mut packages = Vec::with_capacity(4);
        let package_capacity = packages.capacity();
        miner
            .candidate_packages_into(&mut packages)
            .expect("candidate packages emitted into caller buffer");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages.capacity(), package_capacity);
        assert_eq!(packages[0].bucket_id, 13);
        assert!(!packages[0].verifier_binding.is_bound());
        let verifier_binding = test_verifier_binding();
        miner
            .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
            .expect("verifier-bound candidate packages emitted into caller buffer");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages.capacity(), package_capacity);
        assert_eq!(packages[0].bucket_id, 13);
        assert_eq!(packages[0].verifier_binding, verifier_binding);
        assert!(packages[0].verifier_binding.is_bound());
        assert!(
            packages[0]
                .package_bytes
                .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
        );
    }

    #[test]
    fn online_atom_adapter_learns_then_emits_candidate_package() {
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let capacity_before = encoder.scratch_capacity();

        miner
            .observe_atoms(
                &mut encoder,
                17,
                ["family:test_output_parse", "state:exit0", "result:pass"],
                true,
                false,
                10,
                30,
            )
            .expect("seed positive atoms");
        miner
            .observe_atoms(
                &mut encoder,
                17,
                ["family:test_output_parse", "state:panic", "result:fail"],
                false,
                false,
                10,
                30,
            )
            .expect("seed negative atoms");
        miner
            .observe_atoms(
                &mut encoder,
                17,
                ["family:test_output_parse", "state:exit0", "result:pass"],
                true,
                false,
                10,
                30,
            )
            .expect("calibration positive atoms");
        miner
            .observe_atoms(
                &mut encoder,
                17,
                ["family:test_output_parse", "state:panic", "result:fail"],
                false,
                false,
                10,
                30,
            )
            .expect("calibration negative atoms");
        let decision = miner
            .observe_atoms(
                &mut encoder,
                17,
                ["family:test_output_parse", "state:exit0", "result:pass"],
                true,
                false,
                25,
                75,
            )
            .expect("future positive atoms");

        assert_eq!(encoder.scratch_capacity(), capacity_before);
        assert!(decision.local_operator_shadow_decision);
        assert!(decision.unique_cpu_accept_over_exact_cache);
        assert!(!decision.false_accept);
        let summary = miner.summary();
        assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(summary.tokens_saved, 25);
        assert_eq!(summary.cost_saved_microusd, 75);
        assert_eq!(summary.false_accepts, 0);

        let verifier_binding = test_verifier_binding();
        let package = miner
            .candidate_package_bytes_with_verifier(17, verifier_binding)
            .expect("candidate package builds")
            .expect("safe atom bucket emits package");
        assert_eq!(package.bucket_id, 17);
        assert_eq!(package.verifier_binding, verifier_binding);
        assert_eq!(package.package_info.record_count, 1);
        assert!(
            package
                .package_bytes
                .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
        );
    }

    #[test]
    fn online_atom_id_adapter_learns_then_emits_verifier_bound_candidate_package() {
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");
        let capacity_before = encoder.scratch_capacity();

        for (verified_safe_accept, atom_ids) in [
            (true, [10_001, 20_001, 30_001]),
            (false, [10_001, 20_999, 30_999]),
            (true, [10_001, 20_001, 30_001]),
            (false, [10_001, 20_999, 30_999]),
        ] {
            miner
                .observe_atom_ids(
                    &mut encoder,
                    19,
                    atom_ids,
                    verified_safe_accept,
                    false,
                    10,
                    30,
                )
                .expect("atom ids observed");
        }

        let decision = miner
            .observe_atom_ids(
                &mut encoder,
                19,
                [10_001, 20_001, 30_001],
                true,
                false,
                25,
                75,
            )
            .expect("future positive atom ids");

        assert_eq!(encoder.scratch_capacity(), capacity_before);
        assert!(decision.local_operator_shadow_decision);
        assert!(decision.unique_cpu_accept_over_exact_cache);
        assert!(!decision.false_accept);

        let verifier_binding = test_verifier_binding();
        let package = miner
            .candidate_package_bytes_with_verifier(19, verifier_binding)
            .expect("candidate package builds")
            .expect("safe atom-id bucket emits package");
        assert_eq!(package.bucket_id, 19);
        assert_eq!(package.verifier_binding, verifier_binding);
        assert!(package.verifier_binding.is_bound());
        assert!(
            package
                .package_bytes
                .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
        );
    }

    #[test]
    fn online_miner_exports_only_safe_buckets_to_hot_runtime() {
        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        })
        .expect("valid online miner");

        for (bucket_id, verified_safe_accept, atoms) in [
            (
                21,
                true,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
            (
                21,
                false,
                ["family:test_output_parse", "state:panic", "result:fail"],
            ),
            (
                21,
                true,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
            (
                21,
                false,
                ["family:test_output_parse", "state:panic", "result:fail"],
            ),
            (
                21,
                true,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
            (
                22,
                true,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
            (
                22,
                false,
                ["family:test_output_parse", "state:panic", "result:fail"],
            ),
            (
                22,
                true,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
            (
                22,
                false,
                ["family:test_output_parse", "state:panic", "result:fail"],
            ),
            (
                22,
                false,
                ["family:test_output_parse", "state:exit0", "result:pass"],
            ),
        ] {
            miner
                .observe_atoms(
                    &mut encoder,
                    bucket_id,
                    atoms,
                    verified_safe_accept,
                    false,
                    10,
                    30,
                )
                .expect("atoms observed");
        }

        assert_eq!(miner.summary().candidate_bucket_count, 1);
        assert_eq!(miner.summary().rejected_bucket_count, 1);
        let hot = miner
            .candidate_hot_runtime()
            .expect("hot runtime builds")
            .expect("safe bucket exported");
        assert_eq!(hot.profile_count(), 1);
        assert_eq!(hot.profile_id_at(0), Some(21));
        assert_eq!(hot.profile_id_at(1), None);
        assert_eq!(hot.resolve_profile_index(21), Some(0));
        assert_eq!(hot.resolve_profile_index(22), None);

        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let profile_index = hot.resolve_profile_index(21).expect("resolved profile");
        let decision = hot
            .score_profile(profile_index, &positive)
            .expect("hot score");
        assert_eq!(decision.profile_id, 21);
        assert!(decision.local_operator);
    }

    #[test]
    fn promotion_gate_allows_only_verified_future_shadow_savings() {
        let summary = PhaseCenterOnlineSummary {
            unique_cpu_accepts_over_exact_cache: 3,
            tokens_saved: 1200,
            cost_saved_microusd: 3600,
            false_accepts: 0,
            ..PhaseCenterOnlineSummary::default()
        };
        let evidence =
            PhaseCenterPromotionEvidence::from_online_summary(summary, 10, 0, true, true, false)
                .with_verifier_binding(test_verifier_binding())
                .with_threshold_policy(passing_threshold_policy());
        assert_eq!(
            evidence.evaluate(),
            PhaseCenterPromotionDecision {
                eligible: true,
                blocker: None,
            }
        );
    }

    #[test]
    fn promotion_gate_blocks_unsafe_or_unproven_candidates() {
        let safe = PhaseCenterPromotionEvidence {
            future_shadow_events: 10,
            unique_cpu_accepts_over_exact_cache: 3,
            tokens_saved: 1200,
            cost_saved_microusd: 3600,
            false_accepts: 0,
            runtime_margin_parity_mismatches: 0,
            verifier_binding: test_verifier_binding(),
            threshold_policy: passing_threshold_policy(),
            exact_cache_overlap_excluded: true,
            token_cost_denominator_present: true,
            local_accept_enabled: false,
        };
        assert_eq!(
            PhaseCenterPromotionEvidence {
                verifier_binding: PhaseCenterVerifierBinding::default(),
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingVerifierBinding
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                threshold_policy: PhaseCenterThresholdPolicyEvidence::default(),
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                false_accepts: 1,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(PhaseCenterPromotionBlocker::FalseAccepts)
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                runtime_margin_parity_mismatches: 1,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::RuntimeParityMismatch
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                exact_cache_overlap_excluded: false,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::ExactCacheOverlapNotExcluded
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                token_cost_denominator_present: false,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::MissingTokenCostDenominator
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                unique_cpu_accepts_over_exact_cache: 0,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::NoUniqueAcceptsOverExactCache
            )
        );
        assert_eq!(
            PhaseCenterPromotionEvidence {
                local_accept_enabled: true,
                ..safe
            }
            .evaluate(),
            PhaseCenterPromotionDecision::blocked(
                PhaseCenterPromotionBlocker::LocalAcceptAlreadyEnabled
            )
        );
    }

    #[test]
    fn online_miner_reports_threshold_policy_evidence() {
        let config = PhaseCenterOnlineMinerConfig {
            cells: 4,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        };
        let miner = PhaseCenterOnlineMiner {
            config,
            buckets: vec![
                PhaseCenterOnlineBucket {
                    bucket_id: 10,
                    positive_sum: vec![PhaseCenterCell::default(); config.cells],
                    negative_sum: vec![PhaseCenterCell::default(); config.cells],
                    positive_events: 3,
                    negative_events: 2,
                    events_seen: 6,
                    scored_events: 4,
                    calibration_events_seen: 2,
                    learned_threshold_micro: 77,
                    max_calibration_false_margin_micro: Some(76),
                    local_operator_shadow_decisions: 1,
                    unique_cpu_accepts_over_exact_cache: 1,
                    tokens_saved: 120,
                    cost_saved_microusd: 360,
                    false_accepts: 0,
                    rejected: false,
                    trust_quality_micro: 0,
                    trust_false_risk_micro: 0,
                    trust_drift_micro: 0,
                    trust_token_value_micro: 0,
                },
                PhaseCenterOnlineBucket {
                    bucket_id: 11,
                    positive_sum: vec![PhaseCenterCell::default(); config.cells],
                    negative_sum: vec![PhaseCenterCell::default(); config.cells],
                    positive_events: 3,
                    negative_events: 2,
                    events_seen: 6,
                    scored_events: 4,
                    calibration_events_seen: 2,
                    learned_threshold_micro: 77,
                    max_calibration_false_margin_micro: None,
                    local_operator_shadow_decisions: 1,
                    unique_cpu_accepts_over_exact_cache: 1,
                    tokens_saved: 120,
                    cost_saved_microusd: 360,
                    false_accepts: 0,
                    rejected: false,
                    trust_quality_micro: 0,
                    trust_false_risk_micro: 0,
                    trust_drift_micro: 0,
                    trust_token_value_micro: 0,
                },
            ],
        };

        let evidence = miner.threshold_policy_evidence();
        assert_eq!(evidence.candidate_bucket_count, 2);
        assert_eq!(evidence.auto_calibrated_bucket_count, 1);
        assert!(!evidence.automatic_calibration_passed());
        assert!(!evidence.promotion_policy_passed());

        let miner = PhaseCenterOnlineMiner {
            buckets: vec![PhaseCenterOnlineBucket {
                max_calibration_false_margin_micro: Some(76),
                ..miner.buckets[0].clone()
            }],
            ..miner
        };
        let evidence = miner.threshold_policy_evidence();
        assert_eq!(evidence.candidate_bucket_count, 1);
        assert_eq!(evidence.auto_calibrated_bucket_count, 1);
        assert!(evidence.automatic_calibration_passed());
        assert!(evidence.promotion_policy_passed());
    }

    #[test]
    fn operator_memory_admits_only_promoted_profiles() {
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 1,
            min_tokens_saved: 100,
            min_accept_rate_milli: 100,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");

        let unsafe_decision = memory.admit(PhaseCenterOperatorAdmission {
            route_id: 7,
            profile_id: 1,
            evidence: PhaseCenterPromotionEvidence {
                threshold_policy: PhaseCenterThresholdPolicyEvidence::default(),
                ..promotion_evidence(10, 3, 120, 360)
            },
            runtime_bytes_estimate: 256,
            last_seen_tick: 1,
        });
        assert_eq!(
            unsafe_decision,
            PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::PromotionBlocked(
                    PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration
                )
            )
        );
        assert_eq!(memory.warm_profile_count(), 0);

        let low_value_decision = memory.admit(PhaseCenterOperatorAdmission {
            route_id: 7,
            profile_id: 2,
            evidence: promotion_evidence(10, 3, 99, 360),
            runtime_bytes_estimate: 256,
            last_seen_tick: 2,
        });
        assert_eq!(
            low_value_decision,
            PhaseCenterOperatorAdmissionDecision::blocked(
                PhaseCenterOperatorAdmissionBlocker::BelowMinTokensSaved
            )
        );
        assert_eq!(memory.warm_profile_count(), 0);

        let admitted = memory.admit(PhaseCenterOperatorAdmission {
            route_id: 7,
            profile_id: 3,
            evidence: promotion_evidence(10, 3, 120, 360),
            runtime_bytes_estimate: 256,
            last_seen_tick: 3,
        });
        assert_eq!(
            admitted,
            PhaseCenterOperatorAdmissionDecision {
                admitted: true,
                blocker: None
            }
        );
        assert_eq!(memory.warm_profile_count(), 1);
        assert_eq!(memory.route(7).expect("route exists").profile_count(), 1);
    }

    #[test]
    fn operator_memory_bounds_route_top_k_and_warm_profiles() {
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 2,
            max_profiles_per_route: 2,
            max_route_top_k: 1,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");

        for (profile_id, route_id, tokens_saved, tick) in [
            (1, 10, 100, 1),
            (2, 10, 300, 2),
            (3, 10, 200, 3),
            (4, 20, 400, 4),
        ] {
            memory.admit(PhaseCenterOperatorAdmission {
                route_id,
                profile_id,
                evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                runtime_bytes_estimate: 256,
                last_seen_tick: tick,
            });
        }

        assert_eq!(memory.warm_profile_count(), 2);
        assert_eq!(
            memory.route(10).expect("route 10 exists").profile_count(),
            1
        );
        assert_eq!(
            memory.route(20).expect("route 20 exists").profile_count(),
            1
        );

        let mut top = Vec::new();
        memory.route_top_k_into(10, &mut top);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].profile_id, 2);
        memory.route_top_k_into(20, &mut top);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].profile_id, 4);
        memory.route_top_k_into(30, &mut top);
        assert!(top.is_empty());
    }

    #[test]
    fn hot_route_plan_scores_bounded_top_k_without_registry_scan() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let p3 = phase_vector_from_atom_ids([3, 33], 16);
        let n3 = phase_vector_from_atom_ids([3, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.clone().into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.clone().into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p3.clone().into_boxed_slice(),
                    negative_center: n3.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2, 4], &[1, 1, 1])
            .expect("valid hot runtime");
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 3,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");

        for (profile_id, route_id, tokens_saved, tick) in
            [(1, 10, 100, 1), (2, 10, 300, 2), (4, 20, 400, 3)]
        {
            assert!(
                memory
                    .admit(PhaseCenterOperatorAdmission {
                        route_id,
                        profile_id,
                        evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                        runtime_bytes_estimate: 256,
                        last_seen_tick: tick,
                    })
                    .admitted
            );
        }

        let plan = memory
            .hot_route_plan(&hot, 10)
            .expect("route plan builds")
            .expect("route has profiles");
        assert_eq!(plan.route_id(), 10);
        assert_eq!(plan.profile_count(), 2);
        assert_eq!(plan.profile_indexes(), &[1, 0]);
        assert!(plan.bytes_estimate() >= 2 * std::mem::size_of::<usize>());
        assert!(
            memory
                .hot_route_plan(&hot, 30)
                .expect("missing route")
                .is_none()
        );

        let mut decisions = Vec::with_capacity(8);
        let decision_capacity = decisions.capacity();
        hot.score_route_plan_into(&plan, &p2, &mut decisions)
            .expect("route plan scores");
        assert_eq!(decisions.capacity(), decision_capacity);
        assert_eq!(decisions.len(), 2);
        assert_eq!(
            decisions[0],
            hot.score_profile(1, &p2).expect("manual profile 2 score")
        );
        assert_eq!(
            decisions[1],
            hot.score_profile(0, &p2).expect("manual profile 1 score")
        );
        assert_eq!(decisions[0].profile_id, 2);
        assert!(decisions[0].local_operator);
    }

    #[test]
    fn hot_route_plan_rejects_profile_missing_from_hot_runtime() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: p1.into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot =
            PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1], &[1]).expect("valid hot runtime");
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 2,
            max_profiles_per_route: 2,
            max_route_top_k: 1,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");
        assert!(
            memory
                .admit(PhaseCenterOperatorAdmission {
                    route_id: 10,
                    profile_id: 2,
                    evidence: promotion_evidence(10, 5, 100, 300),
                    runtime_bytes_estimate: 256,
                    last_seen_tick: 1,
                })
                .admitted
        );

        assert_eq!(
            memory.hot_route_plan(&hot, 10),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        );
    }

    #[test]
    fn hot_route_table_scores_route_index_without_warm_memory() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let p4 = phase_vector_from_atom_ids([4, 44], 16);
        let n4 = phase_vector_from_atom_ids([4, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.clone().into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.clone().into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p4.clone().into_boxed_slice(),
                    negative_center: n4.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2, 4], &[1, 1, 1])
            .expect("valid hot runtime");
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 3,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");
        for (profile_id, route_id, tokens_saved, tick) in
            [(4, 20, 400, 1), (1, 10, 100, 2), (2, 10, 300, 3)]
        {
            assert!(
                memory
                    .admit(PhaseCenterOperatorAdmission {
                        route_id,
                        profile_id,
                        evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                        runtime_bytes_estimate: 256,
                        last_seen_tick: tick,
                    })
                    .admitted
            );
        }

        let table = memory.hot_route_table(&hot).expect("route table builds");
        assert_eq!(table.route_count(), 2);
        assert_eq!(table.route_id_at(0), Some(10));
        assert_eq!(table.route_id_at(1), Some(20));
        assert_eq!(table.route_id_at(2), None);
        assert_eq!(table.resolve_route_index(10), Some(0));
        assert_eq!(table.resolve_route_index(20), Some(1));
        assert_eq!(table.resolve_route_index(30), None);
        assert!(table.bytes_estimate() >= table.route_count() * std::mem::size_of::<usize>());

        let route_index = table.resolve_route_index(10).expect("route 10 index");
        let plan = table.route_plan_at(route_index).expect("route 10 plan");
        let mut by_plan = Vec::with_capacity(4);
        let mut by_route_index = Vec::with_capacity(4);
        let route_capacity = by_route_index.capacity();
        hot.score_route_plan_into(plan, &p2, &mut by_plan)
            .expect("plan scores");
        hot.score_route_index_into(&table, route_index, &p2, &mut by_route_index)
            .expect("route index scores");
        assert_eq!(by_route_index.capacity(), route_capacity);
        assert_eq!(by_route_index, by_plan);
        assert_eq!(by_route_index[0].profile_id, 2);
        assert!(by_route_index[0].local_operator);

        assert_eq!(
            hot.score_route_index_into(&table, 99, &p2, &mut by_route_index),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        );
    }

    #[test]
    fn operator_memory_runtime_budget_snapshot_reports_hot_and_warm_bounds() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
            .expect("valid hot runtime");
        let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 2,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid operator memory");

        for (profile_id, route_id, tokens_saved, tick) in [(1, 10, 100, 1), (2, 10, 200, 2)] {
            assert!(
                memory
                    .admit(PhaseCenterOperatorAdmission {
                        route_id,
                        profile_id,
                        evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                        runtime_bytes_estimate: 512,
                        last_seen_tick: tick,
                    })
                    .admitted
            );
        }

        let table = memory.hot_route_table(&hot).expect("route table builds");
        let snapshot = memory.runtime_budget_snapshot(&hot, &table);
        assert_eq!(snapshot.max_hot_profiles_per_worker, 2);
        assert_eq!(snapshot.max_warm_profiles_per_process, 4);
        assert_eq!(snapshot.warm_route_count, 1);
        assert_eq!(snapshot.warm_profile_count, 2);
        assert_eq!(snapshot.warm_runtime_bytes_estimate, 1024);
        assert!(snapshot.warm_bytes_estimate >= snapshot.warm_runtime_bytes_estimate);
        assert_eq!(snapshot.hot_route_count, 1);
        assert_eq!(snapshot.hot_profile_count, 2);
        assert_eq!(snapshot.hot_route_profile_edges, 2);
        assert_eq!(
            snapshot.hot_bytes_estimate,
            snapshot
                .hot_runtime_bytes_estimate
                .saturating_add(snapshot.hot_route_table_bytes_estimate)
        );
        assert!(snapshot.hot_budget_passed());
        assert!(snapshot.warm_budget_passed());
        assert!(snapshot.product_runtime_budget_passed());

        let mut tight_memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 1,
            max_hot_bytes_per_worker: 1,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        })
        .expect("valid tight operator memory");
        for (profile_id, route_id, tokens_saved, tick) in [(1, 10, 100, 1), (2, 10, 200, 2)] {
            assert!(
                tight_memory
                    .admit(PhaseCenterOperatorAdmission {
                        route_id,
                        profile_id,
                        evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                        runtime_bytes_estimate: 512,
                        last_seen_tick: tick,
                    })
                    .admitted
            );
        }
        let tight_table = tight_memory
            .hot_route_table(&hot)
            .expect("tight route table still builds for explicit audit");
        let tight_snapshot = tight_memory.runtime_budget_snapshot(&hot, &tight_table);
        assert!(!tight_snapshot.hot_profile_budget_passed);
        assert!(!tight_snapshot.hot_byte_budget_passed);
        assert!(!tight_snapshot.hot_budget_passed());
        assert!(!tight_snapshot.product_runtime_budget_passed());
    }

    #[test]
    fn hot_route_table_rejects_duplicate_route_plans() {
        let table = PhaseCenterHotRouteTable::from_plans([
            PhaseCenterHotRoutePlan::new(7, vec![0])
                .expect("plan builds")
                .expect("plan exists"),
            PhaseCenterHotRoutePlan::new(7, vec![1])
                .expect("plan builds")
                .expect("plan exists"),
        ]);
        assert_eq!(table, Err(PhaseCenterRuntimeError::InvalidRuntimePackage));
    }

    #[test]
    fn hot_route_table_scores_atom_ids_with_reused_buffers() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.clone().into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(10, [2, 1])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let route_index = table.resolve_route_index(10).expect("route index");

        let mut expected = Vec::with_capacity(4);
        hot.score_route_index_into(&table, route_index, &p2, &mut expected)
            .expect("vector route score");

        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let encoder_capacity = encoder.scratch_capacity();
        let mut decisions = Vec::with_capacity(4);
        let decision_capacity = decisions.capacity();
        hot.score_route_atom_ids_into(&table, route_index, &mut encoder, [2, 22], &mut decisions)
            .expect("atom-id route score");

        assert_eq!(encoder.scratch_capacity(), encoder_capacity);
        assert_eq!(decisions.capacity(), decision_capacity);
        assert_eq!(decisions, expected);
        assert_eq!(decisions[0].profile_id, 2);
        assert!(decisions[0].local_operator);

        assert_eq!(
            hot.score_route_atom_ids_into(&table, 99, &mut encoder, [2, 22], &mut decisions,),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        );
    }

    #[test]
    fn hot_candidate_scoring_requires_verifier_and_never_local_accepts() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(10, [2, 1])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let route_index = table.resolve_route_index(10).expect("route index");

        let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let encoder_capacity = encoder.scratch_capacity();
        let mut candidates = Vec::with_capacity(4);
        let candidate_capacity = candidates.capacity();
        hot.score_route_atom_id_candidates_into(
            &table,
            route_index,
            &mut encoder,
            [2, 22],
            &mut candidates,
        )
        .expect("candidate score");

        assert_eq!(encoder.scratch_capacity(), encoder_capacity);
        assert_eq!(candidates.capacity(), candidate_capacity);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].profile_id, 2);
        assert!(candidates[0].score_candidate);
        assert!(candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);

        hot.score_route_atom_id_candidates_into(
            &table,
            route_index,
            &mut encoder,
            [2, 99],
            &mut candidates,
        )
        .expect("negative candidate score");
        assert_eq!(candidates[0].profile_id, 2);
        assert!(!candidates[0].score_candidate);
        assert!(!candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);

        assert_eq!(
            hot.score_route_atom_id_candidates_into(
                &table,
                99,
                &mut encoder,
                [2, 22],
                &mut candidates,
            ),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        );
    }

    #[test]
    fn hot_request_adapter_scores_candidates_with_reused_scratch() {
        let p1 = phase_vector_from_atom_ids([1, 11], 16);
        let n1 = phase_vector_from_atom_ids([1, 99], 16);
        let p2 = phase_vector_from_atom_ids([2, 22], 16);
        let n2 = phase_vector_from_atom_ids([2, 99], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![
                PhaseCenterFlatRecord {
                    positive_center: p1.into_boxed_slice(),
                    negative_center: n1.into_boxed_slice(),
                },
                PhaseCenterFlatRecord {
                    positive_center: p2.into_boxed_slice(),
                    negative_center: n2.into_boxed_slice(),
                },
            ],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(10, [2, 1])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let route_index = table.resolve_route_index(10).expect("route index");
        let positive_atoms = [2, 22];
        let negative_atoms = [2, 99];

        let mut scratch = PhaseCenterHotScratch::new(16, 4).expect("valid scratch");
        let encoder_capacity = scratch.encoder_scratch_capacity();
        let candidate_capacity = scratch.candidate_capacity();
        let score_capacity = scratch.score_capacity();
        let atom_cache_capacity = scratch.atom_cache_capacity();
        let mut reference_encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
        let mut reference = Vec::with_capacity(4);
        hot.score_route_atom_id_candidates_into(
            &table,
            route_index,
            &mut reference_encoder,
            positive_atoms,
            &mut reference,
        )
        .expect("reference candidate score");

        {
            let candidates = hot
                .score_hot_request_candidates(
                    &table,
                    PhaseCenterHotRequest::new(route_index, &positive_atoms),
                    &mut scratch,
                )
                .expect("hot request scores");
            assert_eq!(candidates.len(), 2);
            assert_eq!(candidates[0].profile_id, 2);
            assert!(candidates[0].score_candidate);
            assert!(candidates[0].verifier_required);
            assert!(!candidates[0].local_accept);
            assert_eq!(candidates, reference);
        }
        assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
        assert_eq!(scratch.candidate_capacity(), candidate_capacity);
        assert_eq!(scratch.score_capacity(), score_capacity);
        assert_eq!(scratch.atom_cache_capacity(), atom_cache_capacity);
        assert_eq!(scratch.cached_atom_rows(), positive_atoms.len());

        let positive_vector = phase_vector_from_atom_ids(positive_atoms, 16);
        {
            let candidates = hot
                .score_prepared_hot_request_candidates(
                    &table,
                    PhaseCenterPreparedHotRequest::new(route_index, &positive_vector),
                    &mut scratch,
                )
                .expect("prepared hot request scores");
            assert_eq!(candidates, reference);
            assert!(candidates[0].score_candidate);
            assert!(!candidates[0].local_accept);
        }
        assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
        assert_eq!(scratch.candidate_capacity(), candidate_capacity);
        assert_eq!(scratch.score_capacity(), score_capacity);

        {
            let candidates = hot
                .score_hot_request_candidates(
                    &table,
                    PhaseCenterHotRequest::new(route_index, &negative_atoms),
                    &mut scratch,
                )
                .expect("negative hot request scores");
            assert_eq!(candidates[0].profile_id, 2);
            assert!(!candidates[0].score_candidate);
            assert!(!candidates[0].verifier_required);
            assert!(!candidates[0].local_accept);
        }
        assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
        assert_eq!(scratch.candidate_capacity(), candidate_capacity);
        assert_eq!(scratch.score_capacity(), score_capacity);
        assert_eq!(scratch.atom_cache_capacity(), atom_cache_capacity);
        assert_eq!(scratch.cached_atom_rows(), 3);

        assert!(matches!(
            hot.score_hot_request_candidates(
                &table,
                PhaseCenterHotRequest::new(99, &positive_atoms),
                &mut scratch,
            ),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        ));

        let mut wrong_width_scratch = PhaseCenterHotScratch::new(8, 4).expect("valid scratch");
        assert!(matches!(
            hot.score_hot_request_candidates(
                &table,
                PhaseCenterHotRequest::new(route_index, &positive_atoms),
                &mut wrong_width_scratch,
            ),
            Err(PhaseCenterRuntimeError::VectorWidthMismatch)
        ));

        let too_many_atoms = [1_u64, 2, 3];
        let mut tiny_cache_scratch =
            PhaseCenterHotScratch::with_atom_cache_capacity(16, 4, 2).expect("valid scratch");
        assert!(matches!(
            hot.score_hot_request_candidates(
                &table,
                PhaseCenterHotRequest::new(route_index, &too_many_atoms),
                &mut tiny_cache_scratch,
            ),
            Err(PhaseCenterRuntimeError::RuntimePackageTooLarge)
        ));
    }

    #[test]
    fn local_accept_gate_requires_candidate_verifier_and_promotion() {
        let candidate = PhaseCenterHotCandidateDecision {
            profile_id: 7,
            margin_micro: 400_000,
            score_candidate: true,
            verifier_required: true,
            local_accept: false,
        };
        let promotion = promotion_evidence(10, 5, 120, 360);

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate,
                verifier_passed: true,
                promotion,
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision {
                local_accept: true,
                blocker: None,
            }
        );

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate,
                verifier_passed: false,
                promotion,
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::VerifierRequired
            )
        );

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate: PhaseCenterHotCandidateDecision {
                    score_candidate: false,
                    verifier_required: false,
                    ..candidate
                },
                verifier_passed: true,
                promotion,
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::ScoreNotCandidate
            )
        );

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate: PhaseCenterHotCandidateDecision {
                    local_accept: true,
                    ..candidate
                },
                verifier_passed: true,
                promotion,
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::CandidateAlreadyClaimsLocalAccept
            )
        );

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate,
                verifier_passed: true,
                promotion: PhaseCenterPromotionEvidence {
                    verifier_binding: PhaseCenterVerifierBinding::default(),
                    ..promotion
                },
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::PromotionBlocked(
                    PhaseCenterPromotionBlocker::MissingVerifierBinding
                )
            )
        );

        assert_eq!(
            PhaseCenterLocalAcceptEvidence {
                candidate,
                verifier_passed: true,
                promotion: PhaseCenterPromotionEvidence {
                    false_accepts: 1,
                    ..promotion
                },
            }
            .evaluate(),
            PhaseCenterLocalAcceptDecision::blocked(
                PhaseCenterLocalAcceptBlocker::PromotionBlocked(
                    PhaseCenterPromotionBlocker::FalseAccepts
                )
            )
        );
    }

    #[test]
    fn savings_report_requires_real_denominator_and_provider_costs() {
        let evidence = PhaseCenterSavingsEvidence {
            denominator: PhaseCenterSavingsDenominator {
                total_calls: 1000,
                total_tokens: 1_000_000,
                total_cost_microusd: 3_000_000,
                exact_cache_hits: 50,
                exact_cache_tokens_saved: 50_000,
                exact_cache_cost_saved_microusd: 150_000,
                synthetic_trace_used: false,
                provider_billing_evidence_present: true,
            },
            nando_unique_accepts_over_exact_cache: 100,
            nando_tokens_saved: 200_000,
            nando_cost_saved_microusd: 600_000,
            false_accepts: 0,
        };
        let report = evidence.report();
        assert!(report.market_money_claim_allowed);
        assert_eq!(report.blocker, None);
        assert_eq!(report.exact_cache_calls_saved_milli, 50);
        assert_eq!(report.nando_calls_saved_milli, 100);
        assert_eq!(report.combined_calls_saved_milli, 150);
        assert_eq!(report.exact_cache_tokens_saved_milli, 50);
        assert_eq!(report.nando_tokens_saved_milli, 200);
        assert_eq!(report.combined_tokens_saved_milli, 250);
        assert_eq!(report.exact_cache_cost_saved_milli, 50);
        assert_eq!(report.nando_cost_saved_milli, 200);
        assert_eq!(report.combined_cost_saved_milli, 250);
    }

    #[test]
    fn savings_report_blocks_synthetic_or_unsafe_claims() {
        let safe = PhaseCenterSavingsEvidence {
            denominator: PhaseCenterSavingsDenominator {
                total_calls: 100,
                total_tokens: 1000,
                total_cost_microusd: 3000,
                exact_cache_hits: 5,
                exact_cache_tokens_saved: 50,
                exact_cache_cost_saved_microusd: 150,
                synthetic_trace_used: false,
                provider_billing_evidence_present: true,
            },
            nando_unique_accepts_over_exact_cache: 10,
            nando_tokens_saved: 100,
            nando_cost_saved_microusd: 300,
            false_accepts: 0,
        };

        assert_eq!(
            PhaseCenterSavingsEvidence {
                denominator: PhaseCenterSavingsDenominator {
                    synthetic_trace_used: true,
                    ..safe.denominator
                },
                ..safe
            }
            .report()
            .blocker,
            Some(PhaseCenterSavingsBlocker::SyntheticTrace)
        );
        assert_eq!(
            PhaseCenterSavingsEvidence {
                denominator: PhaseCenterSavingsDenominator {
                    provider_billing_evidence_present: false,
                    ..safe.denominator
                },
                ..safe
            }
            .report()
            .blocker,
            Some(PhaseCenterSavingsBlocker::MissingProviderBillingEvidence)
        );
        assert_eq!(
            PhaseCenterSavingsEvidence {
                false_accepts: 1,
                ..safe
            }
            .report()
            .blocker,
            Some(PhaseCenterSavingsBlocker::FalseAccepts)
        );
        assert_eq!(
            PhaseCenterSavingsEvidence {
                nando_unique_accepts_over_exact_cache: 0,
                ..safe
            }
            .report()
            .blocker,
            Some(PhaseCenterSavingsBlocker::NoUniqueAcceptsOverExactCache)
        );
        assert_eq!(
            PhaseCenterSavingsEvidence {
                nando_unique_accepts_over_exact_cache: 96,
                ..safe
            }
            .report()
            .blocker,
            Some(PhaseCenterSavingsBlocker::CombinedCallsExceedTotalCalls)
        );
    }

    #[test]
    fn hot_shadow_eval_counts_candidate_savings_and_false_accepts() {
        let decisions = [
            PhaseCenterHotCandidateDecision {
                profile_id: 7,
                margin_micro: 10,
                score_candidate: false,
                verifier_required: true,
                local_accept: false,
            },
            PhaseCenterHotCandidateDecision {
                profile_id: 9,
                margin_micro: 1000,
                score_candidate: true,
                verifier_required: true,
                local_accept: false,
            },
        ];
        let mut eval = PhaseCenterHotShadowEval::default();
        eval.observe_candidate_decisions(
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: true,
                exact_cache_hit: true,
                tokens: 10,
                cost_microusd: 30,
            },
            &decisions,
        );
        eval.observe_candidate_decisions(
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: true,
                exact_cache_hit: false,
                tokens: 20,
                cost_microusd: 60,
            },
            &decisions,
        );
        eval.observe_candidate_decisions(
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: false,
                exact_cache_hit: false,
                tokens: 40,
                cost_microusd: 120,
            },
            &decisions,
        );

        assert_eq!(eval.score_events, 3);
        assert_eq!(eval.score_candidate_events, 3);
        assert_eq!(eval.verifier_required_events, 3);
        assert_eq!(eval.local_accept_events, 0);
        assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(eval.tokens_saved, 20);
        assert_eq!(eval.cost_saved_microusd, 60);
        assert_eq!(eval.false_accepts, 1);
    }

    #[test]
    fn prepared_hot_evidence_row_exposes_source_neutral_requests_and_denominator() {
        let vector = phase_vector_from_atoms(["route:run_check", "result:pass"], 8);
        let row = PhaseCenterPreparedHotEvidenceRow::new(
            3,
            vec![11, 22, 33],
            vector.clone(),
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: true,
                exact_cache_hit: false,
                tokens: 44,
                cost_microusd: 132,
            },
        );

        let atom_request = row.hot_evidence_request();
        assert_eq!(atom_request.request.route_index, 3);
        assert_eq!(atom_request.request.atom_ids, &[11, 22, 33]);
        assert_eq!(atom_request.evidence, row.evidence());

        let prepared_request = row.prepared_evidence_request();
        assert_eq!(prepared_request.request.route_index, 3);
        assert_eq!(prepared_request.request.phase_vector, vector.as_slice());
        assert_eq!(prepared_request.evidence, row.evidence());

        let mut denominator = PhaseCenterPreparedHotDenominator::default();
        denominator.observe_evidence(row.evidence());
        denominator.observe_evidence(PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: true,
            tokens: 10,
            cost_microusd: 30,
        });

        assert_eq!(denominator.total_tokens, 54);
        assert_eq!(denominator.total_cost_microusd, 162);
        assert_eq!(denominator.exact_cache_hits, 1);
        assert_eq!(denominator.exact_cache_tokens, 10);
        assert_eq!(denominator.exact_cache_cost_microusd, 30);
        assert_eq!(denominator.non_exact_rows, 1);
    }

    #[test]
    fn hot_row_preparer_converts_live_atom_event_without_source_strings() {
        let positive = phase_vector_from_atoms(["route:run_check", "result:pass"], 8);
        let negative = phase_vector_from_atoms(["route:run_check", "result:fail"], 8);
        let flat = PhaseCenterFlatRuntime::new(
            8,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[900], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(700, [900])
            .expect("valid route plan")
            .expect("non-empty route plan");
        let routes = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid routes");
        let route_index = routes.resolve_route_index(700).expect("route exists");
        let atoms = [11_u64, 22, 33];
        let evidence = PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 44,
            cost_microusd: 132,
        };
        let mut preparer = PhaseCenterHotRowPreparer::new(8).expect("preparer");

        let row = preparer
            .prepare_live_atom_event(
                &routes,
                PhaseCenterLiveOperatorAtomEvent::new(700, 701, &atoms, evidence),
            )
            .expect("prepare succeeds")
            .expect("route is known");

        assert_eq!(preparer.cells(), 8);
        assert_eq!(row.route_index, route_index);
        assert_eq!(row.atom_ids.as_slice(), atoms.as_slice());
        assert_eq!(row.phase_vector.len(), 8);
        assert_eq!(row.evidence(), evidence);

        let missing = preparer
            .prepare_live_atom_event(
                &routes,
                PhaseCenterLiveOperatorAtomEvent::new(701, 701, &atoms, evidence),
            )
            .expect("missing route is not an error");
        assert!(missing.is_none());
    }

    #[test]
    fn hot_worker_scores_prepared_evidence_request_into_shadow_eval() {
        let positive = phase_vector_from_atoms(["route:tool_status", "result:ok"], 16);
        let negative = phase_vector_from_atoms(["route:tool_status", "result:error"], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("valid route plan")
            .expect("non-empty route plan");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid route table");
        let route_index = table.resolve_route_index(11).expect("route exists");
        let mut worker =
            PhaseCenterHotWorker::new(hot.clone(), table.clone()).expect("valid worker");
        let mut eval = PhaseCenterHotShadowEval::default();

        let positive_row = PhaseCenterPreparedHotEvidenceRow::new(
            route_index,
            vec![100, 200],
            positive.clone(),
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: true,
                exact_cache_hit: false,
                tokens: 12,
                cost_microusd: 36,
            },
        );
        let false_row = PhaseCenterPreparedHotEvidenceRow::new(
            route_index,
            vec![100, 200],
            positive.clone(),
            PhaseCenterHotRequestEvidence {
                verified_safe_accept: false,
                exact_cache_hit: false,
                tokens: 12,
                cost_microusd: 36,
            },
        );

        let decisions = worker
            .score_prepared_row_with_evidence(&positive_row, &mut eval)
            .expect("prepared evidence request scores");
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].score_candidate);

        worker
            .score_prepared_rows_with_evidence(&[false_row.clone()], &mut eval)
            .expect("false evidence row scores");

        assert_eq!(eval.score_events, 2);
        assert_eq!(eval.score_candidate_events, 2);
        assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(eval.tokens_saved, 12);
        assert_eq!(eval.cost_saved_microusd, 36);
        assert_eq!(eval.false_accepts, 1);

        let mut runtime_eval = PhaseCenterHotShadowEval::default();
        let mut scratch = PhaseCenterHotScratch::new(16, 1).expect("scratch");
        hot.score_prepared_hot_rows_into(
            &table,
            &[positive_row, false_row],
            &mut scratch,
            &mut runtime_eval,
        )
        .expect("runtime scores prepared rows");
        assert_eq!(runtime_eval, eval);
    }

    #[test]
    fn hot_worker_scores_live_atom_event_without_prepared_row_or_local_accept() {
        let atoms = [100_u64, 200, 300];
        let positive = phase_vector_from_atom_ids(atoms, 16);
        let negative = phase_vector_from_atoms(["route:tool_status", "result:error"], 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("valid route plan")
            .expect("non-empty route plan");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid route table");
        let mut worker = PhaseCenterHotWorker::new(hot, table).expect("valid worker");
        let mut eval = PhaseCenterHotShadowEval::default();

        let decisions = worker
            .score_live_atom_event_with_evidence(
                PhaseCenterLiveOperatorAtomEvent::new(
                    11,
                    99,
                    &atoms,
                    PhaseCenterHotRequestEvidence {
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        tokens: 15,
                        cost_microusd: 45,
                    },
                ),
                &mut eval,
            )
            .expect("live atom event scores")
            .expect("route exists");

        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].score_candidate);
        assert!(decisions[0].verifier_required);
        assert!(!decisions[0].local_accept);
        assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(eval.tokens_saved, 15);
        assert_eq!(eval.cost_saved_microusd, 45);
        assert_eq!(eval.false_accepts, 0);

        let missing = worker
            .score_live_atom_event_with_evidence(
                PhaseCenterLiveOperatorAtomEvent::new(
                    12,
                    99,
                    &atoms,
                    PhaseCenterHotRequestEvidence {
                        verified_safe_accept: false,
                        exact_cache_hit: false,
                        tokens: 15,
                        cost_microusd: 45,
                    },
                ),
                &mut eval,
            )
            .expect("missing route is not an error");
        assert!(missing.is_none());
        assert_eq!(eval.false_accepts, 0);
    }

    #[test]
    fn hot_runtime_scores_numeric_profile_without_cold_path() {
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");

        let positive_decision = hot.score_profile(0, &positive).expect("positive score");
        let negative_decision = hot.score_profile(0, &negative).expect("negative score");

        assert_eq!(hot.cells(), 16);
        assert_eq!(hot.profile_count(), 1);
        assert_eq!(hot.profile_id_at(0), Some(42));
        assert_eq!(hot.resolve_profile_index(42), Some(0));
        assert_eq!(hot.resolve_profile_index(7), None);
        assert_eq!(
            flat.record(0).expect("record exists").positive_center.len(),
            16
        );
        assert_eq!(
            flat.record(1),
            Err(PhaseCenterRuntimeError::CenterIndexOutOfBounds)
        );
        assert_eq!(positive_decision.profile_id, 42);
        assert_eq!(
            flat.score_vector_margin_micro(0, &positive)
                .expect("flat positive score"),
            positive_decision.margin_micro
        );
        assert_eq!(
            flat.score_vector_margin_micro(0, &negative)
                .expect("flat negative score"),
            negative_decision.margin_micro
        );
        assert!(positive_decision.local_operator);
        assert!(positive_decision.margin_micro > 0);
        assert!(!negative_decision.local_operator);
        assert!(negative_decision.margin_micro < positive_decision.margin_micro);
    }

    #[test]
    #[ignore]
    fn hot_runtime_numeric_score_path_p99_budget() {
        let positive = phase_vector_from_atoms(
            ["family:test_output_parse", "state:exit0", "result:pass"],
            16,
        );
        let negative = phase_vector_from_atoms(
            ["family:test_output_parse", "state:panic", "result:fail"],
            16,
        );
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let mut latencies = Vec::with_capacity(50_000);
        for _ in 0..50_000 {
            let start = std::time::Instant::now();
            let decision = hot.score_profile(0, &positive).expect("hot score");
            latencies.push(start.elapsed().as_nanos());
            assert!(decision.local_operator);
        }
        latencies.sort_unstable();
        let p99 = latencies[latencies.len() * 99 / 100];
        println!("phase_center_hot_runtime_numeric_score_path_p99_ns={p99}");
        assert!(p99 <= 1_000, "hot path p99 budget exceeded: p99_ns={p99}");
    }

    #[test]
    #[ignore]
    fn hot_atom_request_candidate_path_adapter_cost_smoke() {
        let atom_ids = [42_u64, 7, 9];
        let wrong_atom_ids = [42_u64, 7, 99];
        let positive = phase_vector_from_atom_ids(atom_ids, 16);
        let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let route_index = table.resolve_route_index(11).expect("route index");
        let request = PhaseCenterHotRequest::new(route_index, &atom_ids);
        let mut scratch = PhaseCenterHotScratch::new(16, 2).expect("valid scratch");

        let mut latencies = Vec::with_capacity(50_000);
        for _ in 0..50_000 {
            let start = std::time::Instant::now();
            let candidates = hot
                .score_hot_request_candidates(&table, request, &mut scratch)
                .expect("hot request score");
            latencies.push(start.elapsed().as_nanos());
            assert_eq!(candidates.len(), 1);
            assert!(candidates[0].score_candidate);
            assert!(candidates[0].verifier_required);
            assert!(!candidates[0].local_accept);
        }
        latencies.sort_unstable();
        let p99 = latencies[latencies.len() * 99 / 100];
        println!("phase_center_hot_atom_request_candidate_path_p99_ns={p99}");
        assert!(
            p99 <= 20_000,
            "hot atom request adapter path regression: p99_ns={p99}"
        );
    }

    #[test]
    #[ignore]
    fn prepared_hot_request_candidate_path_p99_budget() {
        let atom_ids = [42_u64, 7, 9];
        let wrong_atom_ids = [42_u64, 7, 99];
        let positive = phase_vector_from_atom_ids(atom_ids, 16);
        let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let route_index = table.resolve_route_index(11).expect("route index");
        let request = PhaseCenterPreparedHotRequest::new(route_index, &positive);
        let mut scratch = PhaseCenterHotScratch::new(16, 2).expect("valid scratch");

        let mut latencies = Vec::with_capacity(50_000);
        for _ in 0..50_000 {
            let start = std::time::Instant::now();
            let candidates = hot
                .score_prepared_hot_request_candidates(&table, request, &mut scratch)
                .expect("prepared hot request score");
            latencies.push(start.elapsed().as_nanos());
            assert_eq!(candidates.len(), 1);
            assert!(candidates[0].score_candidate);
            assert!(candidates[0].verifier_required);
            assert!(!candidates[0].local_accept);
        }
        latencies.sort_unstable();
        let p99 = latencies[latencies.len() * 99 / 100];
        println!("phase_center_prepared_hot_request_candidate_path_p99_ns={p99}");
        assert!(
            p99 <= 1_000,
            "prepared hot request candidate path p99 budget exceeded: p99_ns={p99}"
        );
    }

    #[test]
    fn hot_worker_owns_runtime_route_table_and_scratch() {
        let atom_ids = [42_u64, 7, 9];
        let wrong_atom_ids = [42_u64, 7, 99];
        let positive = phase_vector_from_atom_ids(atom_ids, 16);
        let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let mut worker = PhaseCenterHotWorker::new(hot, table).expect("worker builds");
        let route_index = worker.resolve_route_index(11).expect("route index");
        assert_eq!(worker.cells(), 16);
        assert_eq!(worker.profile_count(), 1);
        assert_eq!(worker.route_count(), 1);
        assert_eq!(worker.route_profile_edge_count(), 1);
        assert!(worker.bytes_estimate() > 0);

        let prepared = worker
            .score_prepared(PhaseCenterPreparedHotRequest::new(route_index, &positive))
            .expect("prepared worker score");
        assert_eq!(prepared.len(), 1);
        assert!(prepared[0].score_candidate);
        assert!(prepared[0].verifier_required);
        assert!(!prepared[0].local_accept);
        let prepared_profile_id = prepared[0].profile_id;
        let prepared_score_candidate = prepared[0].score_candidate;
        let prepared_local_accept = prepared[0].local_accept;

        let atom = worker
            .score_atom_ids(PhaseCenterHotRequest::new(route_index, &atom_ids))
            .expect("atom worker score");
        assert_eq!(atom.len(), 1);
        assert_eq!(atom[0].profile_id, prepared_profile_id);
        assert_eq!(atom[0].score_candidate, prepared_score_candidate);
        assert_eq!(atom[0].local_accept, prepared_local_accept);
    }

    #[test]
    #[ignore]
    fn hot_worker_prepared_request_p99_budget() {
        let atom_ids = [42_u64, 7, 9];
        let wrong_atom_ids = [42_u64, 7, 99];
        let positive = phase_vector_from_atom_ids(atom_ids, 16);
        let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
        let flat = PhaseCenterFlatRuntime::new(
            16,
            vec![PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.into_boxed_slice(),
            }],
        )
        .expect("valid flat runtime");
        let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1])
            .expect("valid hot runtime");
        let route_plan = hot
            .route_plan_from_profile_ids(11, [42])
            .expect("route plan builds")
            .expect("route plan exists");
        let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
        let mut worker = PhaseCenterHotWorker::new(hot, table).expect("worker builds");
        let route_index = worker.resolve_route_index(11).expect("route index");

        let mut latencies = Vec::with_capacity(50_000);
        for _ in 0..50_000 {
            let start = std::time::Instant::now();
            let candidates = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(route_index, &positive))
                .expect("worker prepared score");
            latencies.push(start.elapsed().as_nanos());
            assert_eq!(candidates.len(), 1);
            assert!(candidates[0].score_candidate);
            assert!(candidates[0].verifier_required);
            assert!(!candidates[0].local_accept);
        }
        latencies.sort_unstable();
        let p99 = latencies[latencies.len() * 99 / 100];
        println!("phase_center_hot_worker_prepared_request_p99_ns={p99}");
        assert!(
            p99 <= 1_000,
            "hot worker prepared request p99 budget exceeded: p99_ns={p99}"
        );
    }

    #[test]
    fn compiler_builds_runtime_from_relation_atoms() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
        let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
        assert_eq!(runtime.record_count(), 1);
        assert!(
            runtime
                .margin_for(0, &correct, &wrong)
                .expect("valid compiled runtime")
                > 0.0
        );
    }

    #[test]
    fn compiler_rejects_incomplete_programs() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        assert_eq!(
            compiler.compile(),
            Err(PhaseCenterRuntimeError::IncompleteProgram)
        );
    }

    #[test]
    fn runtime_package_roundtrip_preserves_margin() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let loaded = PhaseCenterFlatRuntime::from_bytes(&bytes).expect("runtime loads");
        let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
        let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
        assert_eq!(bytes.len(), runtime.serialized_len());
        assert_eq!(loaded.cells(), runtime.cells());
        assert_eq!(loaded.record_count(), runtime.record_count());
        assert_eq!(
            loaded.margin_for(0, &correct, &wrong),
            runtime.margin_for(0, &correct, &wrong)
        );
    }

    #[test]
    fn runtime_package_inspect_reports_header_without_loading_scores() {
        let mut compiler = PhaseCenterCompiler::new(8, 2).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
            .expect("negative atoms accepted");
        compiler
            .add_positive_atoms(1, ["class:edit", "rel:o1:s2"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(1, ["class:edit", "rel:o1:s1"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let bytes = runtime.to_bytes().expect("runtime serializes");
        let info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
        let repeat_info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
        let mut mutated_bytes = bytes.clone();
        let last = mutated_bytes.last_mut().expect("package has payload");
        *last ^= 0x01;
        let mutated_info =
            PhaseCenterFlatRuntime::inspect_bytes(&mutated_bytes).expect("runtime inspects");
        assert_eq!(
            info,
            PhaseCenterRuntimePackageInfo {
                magic: PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
                cells: 8,
                record_count: 2,
                serialized_len: bytes.len(),
                payload_bytes: bytes.len() - PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
                fingerprint64: runtime_package_fingerprint64(&bytes),
            }
        );
        assert_ne!(info.fingerprint64, 0);
        assert_eq!(info.fingerprint64, repeat_info.fingerprint64);
        assert_ne!(info.fingerprint64, mutated_info.fingerprint64);
    }

    #[test]
    fn runtime_package_rejects_bad_magic() {
        let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
        compiler
            .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
            .expect("positive atoms accepted");
        compiler
            .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
            .expect("negative atoms accepted");
        let runtime = compiler.compile().expect("complete compiler");
        let mut bytes = runtime.to_bytes().expect("runtime serializes");
        bytes[0] = b'X';
        assert_eq!(
            PhaseCenterFlatRuntime::from_bytes(&bytes),
            Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
        );
    }
}
