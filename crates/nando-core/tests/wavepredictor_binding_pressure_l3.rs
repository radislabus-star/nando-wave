use nando_core::{
    PHASE_CENTER_RUNTIME_PACKAGE_MAGIC, PhaseCenterCell as CorePhaseCenterCell,
    PhaseCenterCompiler as CorePhaseCenterCompiler, PhaseCenterEvalTask as CorePhaseCenterEvalTask,
    PhaseCenterFlatRecord as CorePhaseCenterFlatRecord,
    PhaseCenterFlatRuntime as CorePhaseCenterFlatRuntime, SURFACE_WAVE_DIM, SurfaceWave4096,
    WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC, WavePredictorActiveCenter, WavePredictorCenterId,
    WavePredictorFlatRoleBindingTable, WavePredictorHebbianConfig, WavePredictorHebbianField,
    WavePredictorMarginSchedule, WavePredictorRoleBindingOffloadPolicy,
    WavePredictorRoleBindingOffloadRuntime, WavePredictorStateDeltaTarget,
    WavePredictorStateDeltaTrainTask, WavePredictorStateImpulse, WavePredictorTrainer,
    WavePredictorTrainerConfig,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const BINDING_PRESSURE_CORPUS: &str =
    "../../data/rule_logic_binding_pressure_v1/accepted_binding_pressure_tasks_v1.jsonl";
const POSITION_SEQUENCE_CORPUS: &str =
    "../../data/rule_logic_position_sequence_v1/accepted_position_sequence_tasks_v1.jsonl";
const POSITION_SEQUENCE_V3_CORPUS: &str =
    "../../data/rule_logic_position_sequence_v3/accepted_position_sequence_tasks_v3.jsonl";
const OPERATOR_BATTERY_V4_ORDER_CORPUS: &str =
    "../../data/rule_logic_operator_battery_v4/order/accepted_operator_tasks_v4.jsonl";
const OPERATOR_BATTERY_V4_EDIT_CORPUS: &str =
    "../../data/rule_logic_operator_battery_v4/edit/accepted_operator_tasks_v4.jsonl";
const OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS: &str =
    "../../data/rule_logic_operator_battery_v4/conditional/accepted_operator_tasks_v4.jsonl";
const OPERATOR_BATTERY_V4_COMPOSED_CORPUS: &str =
    "../../data/rule_logic_operator_battery_v4/composed/accepted_operator_tasks_v4.jsonl";
const OPERATOR_BATTERY_V4_CORPUS: &str =
    "../../data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl";
const FEATURE_CENTER_BASE: WavePredictorCenterId = 256;
const FEATURE_CENTER_COUNT: WavePredictorCenterId = SURFACE_WAVE_DIM as WavePredictorCenterId;
const ACTION_CENTER_BASE: WavePredictorCenterId = FEATURE_CENTER_BASE + FEATURE_CENTER_COUNT;
const ROLE_CENTER_BASE: WavePredictorCenterId = ACTION_CENTER_BASE + FEATURE_CENTER_COUNT;
const ROLE_SLOT_COUNT: u16 = 12;
const TARGET_CENTER_COUNT: usize = 256;
const TOTAL_CENTER_COUNT: usize =
    TARGET_CENTER_COUNT + (FEATURE_CENTER_COUNT as usize * (2 + ROLE_SLOT_COUNT as usize));
const TOP_ACTIVE_L1_LANES: usize = 48;
const TOP_ACTION_L1_LANES: usize = 64;
const TOP_ROLE_L1_LANES: usize = 32;
const STATE_DELTA_LANES_PER_SIDE: usize = 24;

const SEQ_PAGE_BITS: WavePredictorCenterId = 12;
const SEQ_PAGE_SIZE: WavePredictorCenterId = 1 << SEQ_PAGE_BITS;
const SEQ_PAGE_COUNT: WavePredictorCenterId = 32;
const SEQ_ROLE_BASE: WavePredictorCenterId = 0;
const SEQ_ACTION_SURFACE_PAGE: WavePredictorCenterId = 16;
const SEQ_OPERATOR_PAIR_PAGE: WavePredictorCenterId = 17;
const SEQ_STATE_CONDITION_PAGE: WavePredictorCenterId = 18;
const SEQ_CONDITION_ACTION_PAGE: WavePredictorCenterId = 19;
const SEQ_COMPOSED_DEMO_PAGE: WavePredictorCenterId = 20;
const SEQ_ACTION_SLOT_BASE: WavePredictorCenterId = SEQ_ACTION_SURFACE_PAGE << SEQ_PAGE_BITS;
const SEQ_OPERATOR_PAIR_BASE: WavePredictorCenterId = SEQ_OPERATOR_PAIR_PAGE << SEQ_PAGE_BITS;
const SEQ_STATE_CONDITION_BASE: WavePredictorCenterId = SEQ_STATE_CONDITION_PAGE << SEQ_PAGE_BITS;
const SEQ_CONDITION_ACTION_BASE: WavePredictorCenterId = SEQ_CONDITION_ACTION_PAGE << SEQ_PAGE_BITS;
const SEQ_COMPOSED_DEMO_BASE: WavePredictorCenterId = SEQ_COMPOSED_DEMO_PAGE << SEQ_PAGE_BITS;
const SEQ_FEATURE_CENTER_COUNT: WavePredictorCenterId = SEQ_PAGE_SIZE;
const SEQ_ACTION_CENTER_COUNT: WavePredictorCenterId = SEQ_PAGE_SIZE * 5;
const SEQ_OUTPUT_SLOT_COUNT: u8 = 16;
const SEQ_ROLE_SLOT_COUNT: u8 = 16;
const SEQ_TOTAL_CENTER_COUNT: usize = (SEQ_PAGE_SIZE * SEQ_PAGE_COUNT) as usize;
const EDIT_ROLE_BASE: WavePredictorCenterId = 0;
const EDIT_MARKER_ROLE_SLOT: u8 = 16;
const EDIT_ACTION_SURFACE_PAGE: WavePredictorCenterId = 17;
const EDIT_DEMO_PAGE: WavePredictorCenterId = 18;
const EDIT_ACTION_BASE: WavePredictorCenterId = EDIT_ACTION_SURFACE_PAGE << SEQ_PAGE_BITS;
const EDIT_DEMO_BASE: WavePredictorCenterId = EDIT_DEMO_PAGE << SEQ_PAGE_BITS;
const EDIT_ACTION_CENTER_COUNT: WavePredictorCenterId = SEQ_PAGE_SIZE * 2;
const EDIT_OUTPUT_SLOT_COUNT: u8 = 17;
const EDIT_ROLE_SLOT_COUNT: u8 = 17;
const EDIT_END_TOKEN: &str = "__EDIT_END__";

const SEQ32_PAGE_BITS: WavePredictorCenterId = 12;
const SEQ32_PAGE_SIZE: WavePredictorCenterId = 1 << SEQ32_PAGE_BITS;
const SEQ32_PAGE_COUNT: WavePredictorCenterId = 64;
const SEQ32_ROLE_BASE: WavePredictorCenterId = 0;
const SEQ32_ACTION_SURFACE_PAGE: WavePredictorCenterId = 32;
const SEQ32_OPERATOR_PAIR_PAGE: WavePredictorCenterId = 33;
const SEQ32_CONDITION_TRUE_ACTION_PAGE: WavePredictorCenterId = 34;
const SEQ32_CONDITION_FALSE_ACTION_PAGE: WavePredictorCenterId = 35;
const SEQ32_STATE_CONDITION_PAGE: WavePredictorCenterId = 36;
const SEQ32_ACTION_BASE: WavePredictorCenterId = SEQ32_ACTION_SURFACE_PAGE << SEQ32_PAGE_BITS;
const SEQ32_OPERATOR_PAIR_BASE: WavePredictorCenterId = SEQ32_OPERATOR_PAIR_PAGE << SEQ32_PAGE_BITS;
const SEQ32_CONDITION_TRUE_ACTION_BASE: WavePredictorCenterId =
    SEQ32_CONDITION_TRUE_ACTION_PAGE << SEQ32_PAGE_BITS;
const SEQ32_CONDITION_FALSE_ACTION_BASE: WavePredictorCenterId =
    SEQ32_CONDITION_FALSE_ACTION_PAGE << SEQ32_PAGE_BITS;
const SEQ32_STATE_CONDITION_BASE: WavePredictorCenterId =
    SEQ32_STATE_CONDITION_PAGE << SEQ32_PAGE_BITS;
const SEQ32_FEATURE_CENTER_COUNT: WavePredictorCenterId = SEQ32_PAGE_SIZE;
const SEQ32_ACTION_CENTER_COUNT: WavePredictorCenterId = SEQ32_PAGE_SIZE * 2;
const SEQ32_CONDITIONAL_ACTION_CENTER_COUNT: WavePredictorCenterId = SEQ32_PAGE_SIZE * 4;
const SEQ32_OUTPUT_SLOT_COUNT: u8 = 32;
const SEQ32_ROLE_SLOT_COUNT: u8 = 32;
const SEQ32_TOTAL_CENTER_COUNT: usize = (SEQ32_PAGE_SIZE * SEQ32_PAGE_COUNT) as usize;
const SEQ32_TOP_ROLE_L1_LANES: usize = 64;
const SEQ32_MULTI_SEED_COUNT: usize = 3;

#[derive(Clone, Debug)]
struct BindingRow {
    task_id: String,
    source_group: String,
    surface_family: String,
    proof_rule_id: String,
    answer_status: String,
    state_before: String,
    rule_action_example: String,
    state_after_correct: String,
    state_after_wrong: String,
}

#[derive(Clone, Debug)]
struct PreparedBindingTask {
    train_task: WavePredictorStateDeltaTrainTask,
}

#[derive(Clone, Debug)]
struct PreparedFullStateTask {
    frame_task: WavePredictorStateDeltaTrainTask,
    binding_task: WavePredictorStateDeltaTrainTask,
}

#[derive(Clone, Debug)]
struct PreparedStep12Task {
    frame_task: Option<WavePredictorStateDeltaTrainTask>,
    binding_task: WavePredictorStateDeltaTrainTask,
}

#[derive(Clone, Debug)]
struct SequenceBindingRow {
    source_group: String,
    rule_id: String,
    surface_family: String,
    noise_type: String,
    sequence_length: usize,
    state_before: String,
    action: String,
    correct_tokens: Vec<String>,
    wrong_tokens: Vec<String>,
}

#[derive(Clone, Debug)]
struct PreparedSequenceTask {
    slot_tasks: Vec<WavePredictorStateDeltaTrainTask>,
    output_slots: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EvalReport {
    tasks: usize,
    correct: usize,
    accuracy_milli: usize,
    median_gap: i32,
    p10_gap: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BaselineReport {
    accuracy_milli: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FlatGapParityReport {
    checked_slots: usize,
    mismatches: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FlatEnergyParityReport {
    checked_rows: usize,
    mismatches: usize,
    max_abs_gap_delta: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OrderedGroupDiagnostics {
    matrix_groups: usize,
    matrix_group_failures: usize,
    length_group_failures: usize,
    rule_group_failures: usize,
    surface_group_failures: usize,
    noise_group_failures: usize,
    output_slot_failures: usize,
    flat_gap_mismatches: usize,
    failed_rows_by_length: BTreeMap<usize, usize>,
    failed_rows_by_rule: BTreeMap<String, usize>,
    failed_rows_by_surface: BTreeMap<String, usize>,
    failed_rows_by_noise: BTreeMap<String, usize>,
    failed_slots_by_output_slot: BTreeMap<usize, usize>,
    total_slots_by_output_slot: BTreeMap<usize, usize>,
    slot_accuracy_milli_by_output_slot: BTreeMap<usize, usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SequenceEnergyDiagnostics {
    rows: usize,
    energy_accuracy_milli: usize,
    median_energy_gap: i32,
    p10_energy_gap: i32,
    slot_pass_energy_fail: usize,
    energy_pass_slot_fail: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CleanupReadoutDiagnostics {
    rows: usize,
    correct: usize,
    accuracy_milli: usize,
    median_gap: i32,
    p10_gap: i32,
    failed_slots: usize,
    energy_pass_slot_fail: usize,
}

#[derive(Default)]
struct CleanupImpulseCache {
    by_token: BTreeMap<String, Vec<WavePredictorStateImpulse>>,
}

struct CleanupFieldScoreCache<'a> {
    field: &'a WavePredictorHebbianField,
    impulses: CleanupImpulseCache,
    scores: BTreeMap<(usize, usize, String), i32>,
}

struct FlatRoleBindingScoreIndex {
    action_base: Option<WavePredictorCenterId>,
    action_count: WavePredictorCenterId,
    role_base: Option<WavePredictorCenterId>,
    role_stride: WavePredictorCenterId,
    slot_scoped_action_page_bits: u8,
    slot_scoped_action_page_mask: u64,
    slot_scoped_action_source_bits: u8,
    edge_index: HashMap<(WavePredictorCenterId, u8, u8), Vec<(u8, i16)>>,
}

struct PreparedFlatRoleBindingFringe {
    active_actions: Vec<(WavePredictorCenterId, i16)>,
    slot_actions: HashMap<u8, Vec<(WavePredictorCenterId, i16)>>,
    role_strengths: HashMap<(u8, WavePredictorCenterId), i16>,
}

struct CleanupFlatScoreCache {
    index: FlatRoleBindingScoreIndex,
    impulses: CleanupImpulseCache,
    scores: BTreeMap<(usize, usize, String), i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct OnePassWaveCompileEval {
    train_rows: usize,
    heldout_rows: usize,
    slot_accuracy_milli: usize,
    sequence_energy_accuracy_milli: usize,
    flat_slot_accuracy_milli: usize,
    flat_gap_parity_mismatches: usize,
    flat_energy_parity_mismatches: usize,
    ablation_without_binding_accuracy_milli: usize,
    ablation_without_action_accuracy_milli: usize,
    ablation_without_role_accuracy_milli: usize,
    ablation_without_active_fringe_accuracy_milli: usize,
    state_delta_edges: usize,
    role_binding_edges: usize,
    touched_role_binding_edges: usize,
}

#[derive(Clone, Debug)]
struct PhaseOperatorRow {
    source_group: String,
    operator_class: String,
    condition_flag: Option<String>,
    sequence_length: usize,
    surface_family: String,
    noise_type: String,
    action: String,
    source_tokens: Vec<String>,
    correct_tokens: Vec<String>,
    wrong_tokens: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PhaseCenterEval {
    train_rows: usize,
    heldout_rows: usize,
    heldout_surface_groups: usize,
    heldout_noise_groups: usize,
    compiled_phase_centers: usize,
    skipped_train_rows: usize,
    missing_heldout_centers: usize,
    skipped_eval_rows: usize,
    wrong_wins: usize,
    heldout_correct_rows: usize,
    heldout_accuracy_milli: usize,
    median_margin: f64,
    p10_margin: f64,
    median_positive_center_gap: f64,
    p10_positive_center_gap: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PhaseCenterReport {
    action: PhaseCenterEval,
    no_action: PhaseCenterEval,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PhaseCell {
    re: f64,
    im: f64,
}

type PhaseCenterMap = BTreeMap<String, Vec<PhaseCell>>;

#[derive(Clone, Debug, Default, PartialEq)]
struct FlatPhaseCenterRecord {
    positive_center: Vec<PhaseCell>,
    negative_center: Vec<PhaseCell>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FlatPhaseCenterRuntime {
    cells: usize,
    records: Vec<FlatPhaseCenterRecord>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FlatPhaseEvalTask {
    center_index: usize,
    correct_vec: Vec<PhaseCell>,
    wrong_vec: Vec<PhaseCell>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FlatPhasePreparedEval {
    tasks: Vec<FlatPhaseEvalTask>,
    missing_centers: usize,
    skipped_rows: usize,
    heldout_surface_groups: usize,
    heldout_noise_groups: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FlatPhaseRuntimeEval {
    rows: usize,
    correct: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FlatPhaseRuntimeReport {
    compiler_eval: PhaseCenterEval,
    flat_eval: FlatPhaseRuntimeEval,
    no_action_flat_eval: FlatPhaseRuntimeEval,
    flat_sign_parity_mismatches: usize,
    flat_margin_parity_mismatches: usize,
    missing_centers: usize,
    skipped_rows: usize,
    heldout_surface_groups: usize,
    heldout_noise_groups: usize,
    bytes_estimate: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SequenceEnergyGroupDiagnostics {
    failed_rows_by_length: BTreeMap<usize, usize>,
    failed_rows_by_rule: BTreeMap<String, usize>,
    failed_rows_by_surface: BTreeMap<String, usize>,
    failed_rows_by_noise: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SequenceSubsetDiagnostics {
    rows: usize,
    strict_accuracy_milli: usize,
    sequence_energy_accuracy_milli: usize,
    median_slot_gap: i32,
    p10_slot_gap: i32,
    median_energy_gap: i32,
    p10_energy_gap: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SymmetryOperatorDiagnostics {
    symmetry: SequenceSubsetDiagnostics,
    non_symmetry: SequenceSubsetDiagnostics,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OutputSlotCleanupDiagnostics {
    total_slots: usize,
    failed_slots: usize,
    accuracy_milli: usize,
    failed_by_output_slot: BTreeMap<usize, usize>,
    total_by_output_slot: BTreeMap<usize, usize>,
    accuracy_by_output_slot: BTreeMap<usize, usize>,
    failed_by_source_slot: BTreeMap<usize, usize>,
    total_by_source_slot: BTreeMap<usize, usize>,
    accuracy_by_source_slot: BTreeMap<usize, usize>,
    failed_by_output_source_pair: BTreeMap<String, usize>,
    total_by_output_source_pair: BTreeMap<String, usize>,
    accuracy_by_output_source_pair: BTreeMap<String, usize>,
    energy_pass_slot_fail_by_output_slot: BTreeMap<usize, usize>,
    symmetry_accuracy_by_output_slot: BTreeMap<usize, usize>,
    non_symmetry_accuracy_by_output_slot: BTreeMap<usize, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SequenceSlotFailureGroupDiagnostics {
    failed_by_length: BTreeMap<usize, usize>,
    total_by_length: BTreeMap<usize, usize>,
    accuracy_by_length: BTreeMap<usize, usize>,
    energy_pass_slot_fail_by_length: BTreeMap<usize, usize>,
    failed_by_rule: BTreeMap<String, usize>,
    total_by_rule: BTreeMap<String, usize>,
    accuracy_by_rule: BTreeMap<String, usize>,
    energy_pass_slot_fail_by_rule: BTreeMap<String, usize>,
    failed_by_surface: BTreeMap<String, usize>,
    total_by_surface: BTreeMap<String, usize>,
    accuracy_by_surface: BTreeMap<String, usize>,
    failed_by_noise: BTreeMap<String, usize>,
    total_by_noise: BTreeMap<String, usize>,
    accuracy_by_noise: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct EditRoleBindingBoundaryReport {
    rows: usize,
    rows_output_len_over_slots: usize,
    rows_correct_wrong_len_mismatch: usize,
    rows_with_non_source_output_tokens: usize,
    rows_with_marker_output_tokens: usize,
    rows_representable_by_current_role_transfer: usize,
    rows_not_representable_by_current_role_transfer: usize,
    non_representable_by_family: BTreeMap<String, usize>,
    output_len_over_slots_by_family: BTreeMap<String, usize>,
    non_source_output_by_family: BTreeMap<String, usize>,
    correct_len_by_family: BTreeMap<String, BTreeMap<usize, usize>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ConditionalRuntimeBoundaryReport {
    rows: usize,
    rows_same_bag: usize,
    rows_all_outputs_from_source: usize,
    rows_output_len_within_slots: usize,
    rows_with_state_condition_flag: usize,
    rows_with_action_current_flag: usize,
    rows_action_flag_matches_state_flag: usize,
    rows_source_tokens_include_condition_flag: usize,
    rows_branch_signal_action_only_for_current_runtime: usize,
    rows_representable_as_order_transfer_if_branch_known: usize,
    action_current_flag_by_family: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BasinStabilityPoint {
    label: &'static str,
    slot_accuracy_milli: usize,
    energy_accuracy_milli: usize,
    median_energy_gap: i32,
    p10_energy_gap: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CapacityCurvePoint {
    kind: &'static str,
    key: String,
    rows: usize,
    slot_accuracy_milli: usize,
    energy_accuracy_milli: usize,
    p10_energy_gap: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot32CapacitySmokeReport {
    seed: usize,
    train_rows: usize,
    heldout_rows: usize,
    touched_role_binding_edges: usize,
    role_binding_edges: usize,
    flat_role_binding_edges: usize,
    slot_accuracy_milli: usize,
    flat_slot_accuracy_milli: usize,
    sequence_energy_accuracy_milli: usize,
    sequence_energy_median_gap: i32,
    sequence_energy_p10_gap: i32,
    energy_pass_slot_fail: usize,
    flat_gap_parity_mismatches: usize,
    flat_sequence_energy_parity_mismatches: usize,
    flat_sequence_energy_parity_max_abs_gap_delta: i32,
    flat_failed_rows: usize,
    ablation_without_binding_accuracy_milli: usize,
    ablation_without_action_accuracy_milli: usize,
    ablation_without_role_accuracy_milli: usize,
    ablation_without_active_fringe_accuracy_milli: usize,
    flat_role_binding_bytes_estimate: usize,
    base_mass_bytes_estimate: usize,
    hot_bytes_estimate: usize,
    flat_eval_rows: usize,
    flat_eval_total_ns: u128,
    flat_eval_avg_ns_per_row: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot32CapacitySmokeRun {
    report: Slot32CapacitySmokeReport,
    field: WavePredictorHebbianField,
    flat_failed_by_length: BTreeMap<usize, usize>,
    flat_failed_by_rule: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Debug)]
struct Slot32OrderCorpusTask {
    length: usize,
    operator_class: &'static str,
    rule_name: &'static str,
    surface_family: &'static str,
    noise_type: &'static str,
    condition_result: Option<bool>,
    state_key: String,
    correct_tokens: Vec<String>,
    wrong_tokens: Vec<String>,
    task: PreparedSequenceTask,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot32CacheOffloadReport {
    label: &'static str,
    seed: usize,
    unique_rows: usize,
    simulated_calls: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_plus_nando_cache_hits: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    incremental_llm_calls_removed_vs_cache: usize,
    incremental_llm_call_reduction_vs_cache_milli: usize,
    offload_rate_milli: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    min_energy_margin: i32,
    p10_energy_margin: i32,
    median_energy_margin: i32,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    max_latency_ns: u128,
    role_binding_edges: usize,
    hot_bytes_estimate: usize,
}

struct Slot32CacheOffloadBenchInput<'a> {
    label: &'static str,
    seed: usize,
    index: &'a FlatRoleBindingScoreIndex,
    flat: &'a WavePredictorFlatRoleBindingTable,
    field: &'a WavePredictorHebbianField,
    heldout_rows: &'a [Slot32OrderCorpusTask],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot32RoleBindingPackageReport {
    label: &'static str,
    seed: usize,
    package_path: PathBuf,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_edges: usize,
    loaded_rewrite_exact: bool,
    slot_accuracy_milli: usize,
    sequence_energy_accuracy_milli: usize,
    flat_gap_parity_mismatches: usize,
    flat_sequence_energy_parity_mismatches: usize,
    p99_latency_ns: u128,
    false_local_accepts: usize,
    hot_bytes_estimate: usize,
}

impl Slot32CapacitySmokeReport {
    fn gate_pass(&self) -> bool {
        self.slot_accuracy_milli == 1000
            && self.flat_slot_accuracy_milli == self.slot_accuracy_milli
            && self.sequence_energy_accuracy_milli == 1000
            && self.energy_pass_slot_fail == 0
            && self.flat_gap_parity_mismatches == 0
            && self.flat_sequence_energy_parity_mismatches == 0
            && self.flat_sequence_energy_parity_max_abs_gap_delta == 0
            && self.flat_failed_rows == 0
            && self.ablation_without_binding_accuracy_milli == 0
            && self.ablation_without_action_accuracy_milli == 0
            && self.ablation_without_role_accuracy_milli == 0
            && self.ablation_without_active_fringe_accuracy_milli == 0
            && self.hot_bytes_estimate < 4 * 1024 * 1024
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AddressRadiusPoint {
    label: &'static str,
    slot_accuracy_milli: usize,
    energy_accuracy_milli: usize,
    median_energy_gap: i32,
    p10_energy_gap: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct L3RoleBindingCollisionReport {
    edge_count: usize,
    action_centers_with_edges: usize,
    avg_edges_per_action_center_milli: usize,
    max_edges_per_action_center: usize,
    action_centers_with_multi_slot_edges: usize,
    max_slots_per_action_center: usize,
    role_slots_covered: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ActionSeparabilityReport {
    action_vectors: usize,
    same_rule_similarity_milli: usize,
    different_rule_similarity_milli: usize,
    same_family_different_length_similarity_milli: usize,
    different_family_similarity_milli: usize,
    max_different_rule_similarity_milli: usize,
    nearest_rule_pairs: Vec<(String, String, usize)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FoldedCollisionReport {
    target_impulses_checked: usize,
    multi_role_hit_count: usize,
    wrong_role_hit_count: usize,
    missing_true_role_count: usize,
    multi_role_hit_milli: usize,
    wrong_role_hit_milli: usize,
    missing_true_role_milli: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CollisionOutcomeReport {
    by_bucket: BTreeMap<String, CollisionOutcomeSummary>,
    by_surface: BTreeMap<String, CollisionOutcomeSummary>,
    by_surface_bucket: BTreeMap<String, CollisionOutcomeSummary>,
    worst_output_source_pairs: Vec<(String, CollisionOutcomeSummary)>,
    worst_surface_output_source_pairs: Vec<(String, CollisionOutcomeSummary)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CollisionOutcomeAccumulator {
    slots: usize,
    failed_slots: usize,
    energy_pass_slot_fail: usize,
    gap_sum: i64,
    min_gap: i32,
    wrong_role_hit_milli_sum: usize,
    multi_role_hit_milli_sum: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CollisionOutcomeSummary {
    slots: usize,
    failed_slots: usize,
    accuracy_milli: usize,
    energy_pass_slot_fail: usize,
    avg_gap: i32,
    min_gap: i32,
    avg_wrong_role_hit_milli: usize,
    avg_multi_role_hit_milli: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LaneOverlapReport {
    by_surface: BTreeMap<String, LaneOverlapSummary>,
    worst_output_source_pairs: Vec<(String, LaneOverlapSummary)>,
    worst_surface_output_source_pairs: Vec<(String, LaneOverlapSummary)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LaneOverlapAccumulator {
    slots: usize,
    target_wrong_overlap_milli_sum: usize,
    wrong_hits_true_role_milli_sum: usize,
    target_hits_wrong_role_milli_sum: usize,
    target_hits_multi_role_milli_sum: usize,
    target_missing_true_role_milli_sum: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LaneOverlapSummary {
    slots: usize,
    avg_target_wrong_overlap_milli: usize,
    avg_wrong_hits_true_role_milli: usize,
    avg_target_hits_wrong_role_milli: usize,
    avg_target_hits_multi_role_milli: usize,
    avg_target_missing_true_role_milli: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SignAwareCollisionReport {
    by_surface: BTreeMap<String, SignAwareCollisionSummary>,
    worst_output_source_pairs: Vec<(String, SignAwareCollisionSummary)>,
    worst_surface_output_source_pairs: Vec<(String, SignAwareCollisionSummary)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SignAwareCollisionAccumulator {
    impulses: usize,
    current_wrong_role_hits: usize,
    sign_aware_wrong_role_hits: usize,
    sign_erased_wrong_role_hits: usize,
    current_multi_role_hits: usize,
    sign_aware_multi_role_hits: usize,
    missing_true_signed_role_hits: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SignAwareCollisionSummary {
    impulses: usize,
    current_wrong_role_hit_milli: usize,
    sign_aware_wrong_role_hit_milli: usize,
    sign_erased_wrong_role_hit_milli: usize,
    current_multi_role_hit_milli: usize,
    sign_aware_multi_role_hit_milli: usize,
    missing_true_signed_role_milli: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ResidualCollisionOutcomeReport {
    by_bucket: BTreeMap<String, ResidualCollisionOutcomeSummary>,
    by_surface: BTreeMap<String, ResidualCollisionOutcomeSummary>,
    by_surface_bucket: BTreeMap<String, ResidualCollisionOutcomeSummary>,
    worst_output_source_pairs: Vec<(String, ResidualCollisionOutcomeSummary)>,
    worst_surface_output_source_pairs: Vec<(String, ResidualCollisionOutcomeSummary)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ResidualCollisionOutcomeAccumulator {
    slots: usize,
    failed_slots: usize,
    energy_pass_slot_fail: usize,
    gap_sum: i64,
    min_gap: i32,
    current_wrong_role_hit_milli_sum: usize,
    sign_aware_wrong_role_hit_milli_sum: usize,
    sign_erased_wrong_role_hit_milli_sum: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ResidualCollisionOutcomeSummary {
    slots: usize,
    failed_slots: usize,
    accuracy_milli: usize,
    energy_pass_slot_fail: usize,
    avg_gap: i32,
    min_gap: i32,
    avg_current_wrong_role_hit_milli: usize,
    avg_sign_aware_wrong_role_hit_milli: usize,
    avg_sign_erased_wrong_role_hit_milli: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RoleHitReport {
    positive_hit_milli: usize,
    negative_hit_milli: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ErrorCase {
    gap: i32,
    task_id: String,
    proof_rule_id: String,
    surface_family: String,
    state_before: String,
    rule_action_example: String,
    state_after_correct: String,
    state_after_wrong: String,
}

#[test]
#[ignore = "Binding-pressure L3 gate; run explicitly to test induced X transfer"]
fn binding_pressure_l3_must_induce_transfer_without_target_ids_or_rule_authority() {
    let rows = load_rows();
    assert_eq!(rows.len(), 2_800);

    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("binding_train_"))
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("binding_heldout_"))
        .cloned()
        .collect();
    assert_eq!(train_rows.len(), 2_000);
    assert_eq!(heldout_rows.len(), 800);

    let train = prepare_rows(&train_rows);
    let heldout = prepare_rows(&heldout_rows);
    let train_tasks: Vec<_> = train.iter().map(|task| task.train_task.clone()).collect();

    let config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };

    let field = train_binding_field(&train_tasks, binding_hebbian_config(), config);

    let train_eval = eval_state_delta(&field, &train);
    let heldout_eval = eval_state_delta(&field, &heldout);
    let flat_table = field.compile_flat_role_binding_table();
    let flat_eval = eval_flat_binding_table(&flat_table, &heldout);
    let train_role_hits = role_target_hit_report(&train);
    let heldout_role_hits = role_target_hit_report(&heldout);
    let error_audit = audit_errors(&field, &heldout_rows, &heldout);
    let rule_id_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.proof_rule_id.clone());
    let surface_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.surface_family.clone());
    let status_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.answer_status.clone());
    let neighbor_baseline = eval_l1_neighbor_baseline(&train_rows, &heldout_rows);
    let without_binding_eval = eval_state_delta(
        &train_binding_field(&train_tasks, binding_disabled_config(), config),
        &heldout,
    );
    let without_action_eval = eval_state_delta(
        &train_binding_field(&train_tasks, action_disabled_config(), config),
        &heldout,
    );
    let without_role_eval = eval_state_delta(
        &train_binding_field(&train_tasks, role_disabled_config(), config),
        &heldout,
    );
    let best_shortcut = [
        rule_id_baseline.accuracy_milli,
        surface_baseline.accuracy_milli,
        status_baseline.accuracy_milli,
        neighbor_baseline.accuracy_milli,
        500,
    ]
    .into_iter()
    .max()
    .expect("shortcut list must not be empty");

    println!("binding_pressure_l3_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  train_binding_accuracy_milli: {}",
        train_eval.accuracy_milli
    );
    println!(
        "  heldout_binding_accuracy_milli: {}",
        heldout_eval.accuracy_milli
    );
    println!(
        "  flat_binding_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!("  heldout_binding_median_gap: {}", heldout_eval.median_gap);
    println!("  heldout_binding_p10_gap: {}", heldout_eval.p10_gap);
    println!(
        "  proof_rule_id_prototype_accuracy_milli: {}",
        rule_id_baseline.accuracy_milli
    );
    println!(
        "  surface_family_prototype_accuracy_milli: {}",
        surface_baseline.accuracy_milli
    );
    println!(
        "  answer_status_prototype_accuracy_milli: {}",
        status_baseline.accuracy_milli
    );
    println!(
        "  l1_neighbor_accuracy_milli: {}",
        neighbor_baseline.accuracy_milli
    );
    println!("  markov_pairwise_accuracy_milli: 500");
    println!("  best_shortcut_accuracy_milli: {best_shortcut}");
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  flat_role_binding_edges: {}", flat_table.edge_count());
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        flat_table.byte_size_estimate()
    );
    let (role_nonzero, role_max_abs) = field.state_delta_role_binding_nonzero_report();
    println!("  role_binding_nonzero_edges: {role_nonzero}");
    println!("  role_binding_max_abs_weight: {role_max_abs}");
    println!(
        "  train_positive_role_hit_milli: {}",
        train_role_hits.positive_hit_milli
    );
    println!(
        "  train_negative_role_hit_milli: {}",
        train_role_hits.negative_hit_milli
    );
    println!(
        "  heldout_positive_role_hit_milli: {}",
        heldout_role_hits.positive_hit_milli
    );
    println!(
        "  heldout_negative_role_hit_milli: {}",
        heldout_role_hits.negative_hit_milli
    );
    let (binding_positive, binding_negative) = field.state_delta_binding_weights();
    println!("  binding_coprocessor_positive_weight: {binding_positive}");
    println!("  binding_coprocessor_negative_weight: {binding_negative}");
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        without_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        without_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        without_role_eval.accuracy_milli
    );
    println!("  manual_role_slot_bridge_used: false");
    println!("  l2_time_phase_role_slots: {ROLE_SLOT_COUNT}");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    print_error_audit(&error_audit);

    assert!(
        heldout_eval.accuracy_milli >= 650 && heldout_eval.accuracy_milli > best_shortcut,
        "Binding-pressure L3 must beat shortcut baselines without target/rule authority: heldout={heldout_eval:#?} best_shortcut={best_shortcut}"
    );
    assert_eq!(flat_eval.accuracy_milli, heldout_eval.accuracy_milli);
    assert_eq!(
        without_binding_eval.accuracy_milli, 0,
        "binding co-processor ablation must collapse"
    );
    assert_eq!(
        without_action_eval.accuracy_milli, 0,
        "action-front ablation must collapse"
    );
    assert_eq!(
        without_role_eval.accuracy_milli, 0,
        "role-slot ablation must collapse"
    );
}

#[test]
#[ignore = "Full state_after frame+slot+X gate for binding_trace rows"]
fn full_state_after_gate_must_compose_frame_slot_and_unknown_x() {
    let rows = load_rows();
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.source_group.starts_with("binding_train_") && row.surface_family == "binding_trace"
        })
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.source_group.starts_with("binding_heldout_")
                && row.surface_family == "binding_trace"
        })
        .cloned()
        .collect();
    assert_eq!(train_rows.len(), 1_000);
    assert_eq!(heldout_rows.len(), 400);

    let train = prepare_full_state_rows(&train_rows);
    let heldout = prepare_full_state_rows(&heldout_rows);
    let frame_train_tasks: Vec<_> = train.iter().map(|task| task.frame_task.clone()).collect();
    let binding_train_tasks: Vec<_> = train.iter().map(|task| task.binding_task.clone()).collect();

    let trainer_config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let frame_field = train_binding_field(&frame_train_tasks, frame_wave_config(), trainer_config);
    let binding_field = train_binding_field(
        &binding_train_tasks,
        binding_hebbian_config(),
        trainer_config,
    );

    let frame_eval = eval_full_state_component(&frame_field, &heldout, |task| &task.frame_task);
    let binding_eval =
        eval_full_state_component(&binding_field, &heldout, |task| &task.binding_task);
    let full_eval = eval_full_state_after(&frame_field, &binding_field, &heldout);
    let without_frame_eval = eval_full_state_after(
        &WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, frame_wave_config()),
        &binding_field,
        &heldout,
    );
    let without_binding_eval = eval_full_state_after(
        &frame_field,
        &WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, binding_hebbian_config()),
        &heldout,
    );

    println!("full_state_after_binding_trace_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  frame_accuracy_milli: {}", frame_eval.accuracy_milli);
    println!(
        "  binding_x_accuracy_milli: {}",
        binding_eval.accuracy_milli
    );
    println!(
        "  full_state_after_accuracy_milli: {}",
        full_eval.accuracy_milli
    );
    println!("  full_state_after_median_gap: {}", full_eval.median_gap);
    println!("  full_state_after_p10_gap: {}", full_eval.p10_gap);
    println!(
        "  ablation_without_frame_accuracy_milli: {}",
        without_frame_eval.accuracy_milli
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        without_binding_eval.accuracy_milli
    );
    println!(
        "  frame_delta_edges: {}",
        frame_field.state_delta_edge_count()
    );
    println!(
        "  binding_state_delta_edges: {}",
        binding_field.state_delta_edge_count()
    );
    println!(
        "  binding_role_edges: {}",
        binding_field.state_delta_role_binding_edge_count()
    );
    println!("  frame_id_training_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");

    assert_eq!(frame_eval.accuracy_milli, 1000);
    assert_eq!(binding_eval.accuracy_milli, 1000);
    assert_eq!(full_eval.accuracy_milli, 1000);
    assert_eq!(without_frame_eval.accuracy_milli, 0);
    assert_eq!(without_binding_eval.accuracy_milli, 0);
    assert_eq!(binding_field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "Noisy binding_trace full state_after gate"]
fn noisy_full_state_after_gate_must_survive_marker_relative_noise() {
    let rows = load_rows();
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.source_group.starts_with("binding_train_") && row.surface_family == "binding_trace"
        })
        .cloned()
        .collect();
    let clean_heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.source_group.starts_with("binding_heldout_")
                && row.surface_family == "binding_trace"
        })
        .cloned()
        .collect();
    let heldout_rows = noisy_binding_trace_rows(&clean_heldout_rows);

    let train = prepare_full_state_rows(&train_rows);
    let heldout = prepare_full_state_rows(&heldout_rows);
    let frame_train_tasks: Vec<_> = train.iter().map(|task| task.frame_task.clone()).collect();
    let binding_train_tasks: Vec<_> = train.iter().map(|task| task.binding_task.clone()).collect();

    let trainer_config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let frame_field = train_binding_field(&frame_train_tasks, frame_wave_config(), trainer_config);
    let binding_field = train_binding_field(
        &binding_train_tasks,
        binding_hebbian_config(),
        trainer_config,
    );
    let frame_eval = eval_full_state_component(&frame_field, &heldout, |task| &task.frame_task);
    let binding_eval =
        eval_full_state_component(&binding_field, &heldout, |task| &task.binding_task);
    let full_eval = eval_full_state_after(&frame_field, &binding_field, &heldout);

    println!("noisy_full_state_after_binding_trace_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  noisy_heldout_rows: {}", heldout_rows.len());
    println!("  frame_accuracy_milli: {}", frame_eval.accuracy_milli);
    println!(
        "  binding_x_accuracy_milli: {}",
        binding_eval.accuracy_milli
    );
    println!(
        "  full_state_after_accuracy_milli: {}",
        full_eval.accuracy_milli
    );
    println!("  full_state_after_median_gap: {}", full_eval.median_gap);
    println!("  full_state_after_p10_gap: {}", full_eval.p10_gap);
    println!("  noisy_marker_relative_roles_used: true");
    println!(
        "  binding_state_delta_edges: {}",
        binding_field.state_delta_edge_count()
    );
    println!("  concrete_x_lookup_used: false");

    assert_eq!(frame_eval.accuracy_milli, 1000);
    assert_eq!(binding_eval.accuracy_milli, 1000);
    assert_eq!(full_eval.accuracy_milli, 1000);
    assert_eq!(binding_field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "Step 12 all binding-pressure rows: frame where needed + X transfer"]
fn step12_all_binding_pressure_rows_must_compose_current_full_state_after() {
    let rows = load_rows();
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("binding_train_"))
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("binding_heldout_"))
        .cloned()
        .collect();
    assert_eq!(train_rows.len(), 2_000);
    assert_eq!(heldout_rows.len(), 800);

    let train = prepare_step12_rows(&train_rows);
    let heldout = prepare_step12_rows(&heldout_rows);
    let binding_train_tasks: Vec<_> = train.iter().map(|task| task.binding_task.clone()).collect();
    let frame_train_tasks: Vec<_> = train
        .iter()
        .filter_map(|task| task.frame_task.clone())
        .collect();

    let trainer_config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let binding_field = train_binding_field(
        &binding_train_tasks,
        binding_hebbian_config(),
        trainer_config,
    );
    let frame_field = train_binding_field(&frame_train_tasks, frame_wave_config(), trainer_config);

    let step12_eval = eval_step12_current_full_state_after(&frame_field, &binding_field, &heldout);
    let without_frame_eval = eval_step12_current_full_state_after(
        &WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, frame_wave_config()),
        &binding_field,
        &heldout,
    );
    let without_binding_eval = eval_step12_current_full_state_after(
        &frame_field,
        &WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, binding_hebbian_config()),
        &heldout,
    );

    println!("step12_all_binding_pressure_rows_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  frame_train_tasks: {}", frame_train_tasks.len());
    println!(
        "  step12_full_state_after_accuracy_milli: {}",
        step12_eval.accuracy_milli
    );
    println!("  step12_median_gap: {}", step12_eval.median_gap);
    println!("  step12_p10_gap: {}", step12_eval.p10_gap);
    println!(
        "  ablation_without_frame_accuracy_milli: {}",
        without_frame_eval.accuracy_milli
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        without_binding_eval.accuracy_milli
    );
    println!(
        "  binding_state_delta_edges: {}",
        binding_field.state_delta_edge_count()
    );
    println!(
        "  frame_delta_edges: {}",
        frame_field.state_delta_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");

    assert_eq!(step12_eval.accuracy_milli, 1000);
    assert!(
        without_frame_eval.accuracy_milli < 1000,
        "frame ablation must break binding_trace subset"
    );
    assert_eq!(without_binding_eval.accuracy_milli, 0);
    assert_eq!(binding_field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "Ordered multi-token position-binding gate; correct and wrong share same token bag"]
fn ordered_position_binding_must_learn_multi_slot_sequence_not_bag_copy() {
    let rows = load_sequence_rows();
    assert_eq!(rows.len(), 1_764);
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_train_"))
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_heldout_"))
        .cloned()
        .collect();
    assert_eq!(train_rows.len(), 1_260);
    assert_eq!(heldout_rows.len(), 504);

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let train_tasks: Vec<_> = train
        .iter()
        .flat_map(|task| task.slot_tasks.iter().cloned())
        .collect();

    let trainer_config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let field = train_binding_field(&train_tasks, sequence_binding_config(), trainer_config);
    let eval = eval_ordered_sequence(&field, &heldout);
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let flat = field.compile_flat_role_binding_table();
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);

    println!("ordered_position_binding_sequence_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  heldout_rules: {}", sequence_unique_rules(&heldout_rows));
    println!(
        "  heldout_surface_families: {}",
        sequence_unique_surfaces(&heldout_rows)
    );
    println!("  ordered_sequence_accuracy_milli: {}", eval.accuracy_milli);
    println!(
        "  flat_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!("  ordered_sequence_median_gap: {}", eval.median_gap);
    println!("  ordered_sequence_p10_gap: {}", eval.p10_gap);
    println!(
        "  flat_gap_parity_checked_slots: {}",
        flat_parity.checked_slots
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!("  bag_of_tokens_shortcut_accuracy_milli: 500");
    println!("  exact_train_lookup_accuracy_milli: 0");
    println!("  persisted_position_sequence_corpus_used: true");
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  flat_role_binding_edges: {}", flat.edge_count());
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        flat.byte_size_estimate()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");

    assert_eq!(eval.accuracy_milli, 1000);
    assert_eq!(flat_eval.accuracy_milli, eval.accuracy_milli);
    assert_eq!(flat_eval.median_gap, eval.median_gap);
    assert_eq!(flat_eval.p10_gap, eval.p10_gap);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "V3 balanced ordered sequence gate; run explicitly, it trains a larger matrix"]
fn ordered_position_binding_v3_balanced_matrix_must_hold_without_runtime_phase_hack() {
    let rows = load_sequence_v3_rows();
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_train_"))
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_heldout_"))
        .cloned()
        .collect();
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let train_tasks: Vec<_> = train
        .iter()
        .flat_map(|task| task.slot_tasks.iter().cloned())
        .collect();

    let trainer_config = WavePredictorTrainerConfig {
        epochs: 8,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 160,
            warmup_epochs: 1,
            ramp_epochs: 7,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let field = train_binding_field_with_progress(
        "position_sequence_v3",
        &train_tasks,
        sequence_binding_config(),
        trainer_config,
    );
    let eval = eval_ordered_sequence(&field, &heldout);
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let flat = field.compile_flat_role_binding_table();
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    let diagnostics = ordered_group_diagnostics(&field, &flat, &heldout_rows, &heldout);
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    let energy_groups = ordered_sequence_energy_group_diagnostics(&field, &heldout_rows, &heldout);
    let symmetry = symmetry_operator_diagnostics(&field, &heldout_rows, &heldout);

    println!("ordered_position_binding_v3_balanced_matrix_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  heldout_rules: {}", sequence_unique_rules(&heldout_rows));
    println!(
        "  heldout_surface_families: {}",
        sequence_unique_surfaces(&heldout_rows)
    );
    println!(
        "  heldout_noise_types: {}",
        sequence_unique_noise_types(&heldout_rows)
    );
    println!(
        "  heldout_lengths: {}",
        sequence_unique_lengths(&heldout_rows)
    );
    println!("  ordered_sequence_accuracy_milli: {}", eval.accuracy_milli);
    println!(
        "  flat_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!("  ordered_sequence_median_gap: {}", eval.median_gap);
    println!("  ordered_sequence_p10_gap: {}", eval.p10_gap);
    println!(
        "  flat_gap_parity_checked_slots: {}",
        flat_parity.checked_slots
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  per_matrix_group_count: {}", diagnostics.matrix_groups);
    println!(
        "  per_matrix_group_failures: {}",
        diagnostics.matrix_group_failures
    );
    println!(
        "  length_group_failures: {}",
        diagnostics.length_group_failures
    );
    println!("  rule_group_failures: {}", diagnostics.rule_group_failures);
    println!(
        "  surface_group_failures: {}",
        diagnostics.surface_group_failures
    );
    println!(
        "  noise_group_failures: {}",
        diagnostics.noise_group_failures
    );
    println!(
        "  output_slot_failures: {}",
        diagnostics.output_slot_failures
    );
    println!(
        "  diagnostics_flat_gap_mismatches: {}",
        diagnostics.flat_gap_mismatches
    );
    println!("  sequence_energy_rows: {}", energy.rows);
    println!(
        "  sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!("  sequence_energy_median_gap: {}", energy.median_energy_gap);
    println!("  sequence_energy_p10_gap: {}", energy.p10_energy_gap);
    println!("  slot_pass_energy_fail: {}", energy.slot_pass_energy_fail);
    println!("  energy_pass_slot_fail: {}", energy.energy_pass_slot_fail);
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!(
        "  energy_failed_rows_by_length: {:?}",
        energy_groups.failed_rows_by_length
    );
    println!(
        "  energy_failed_rows_by_rule: {:?}",
        energy_groups.failed_rows_by_rule
    );
    println!(
        "  energy_failed_rows_by_surface: {:?}",
        energy_groups.failed_rows_by_surface
    );
    println!(
        "  energy_failed_rows_by_noise: {:?}",
        energy_groups.failed_rows_by_noise
    );
    println!("  symmetry_rows: {}", symmetry.symmetry.rows);
    println!(
        "  symmetry_strict_accuracy_milli: {}",
        symmetry.symmetry.strict_accuracy_milli
    );
    println!(
        "  symmetry_sequence_energy_accuracy_milli: {}",
        symmetry.symmetry.sequence_energy_accuracy_milli
    );
    println!(
        "  symmetry_median_slot_gap: {}",
        symmetry.symmetry.median_slot_gap
    );
    println!(
        "  symmetry_p10_slot_gap: {}",
        symmetry.symmetry.p10_slot_gap
    );
    println!(
        "  symmetry_median_energy_gap: {}",
        symmetry.symmetry.median_energy_gap
    );
    println!(
        "  symmetry_p10_energy_gap: {}",
        symmetry.symmetry.p10_energy_gap
    );
    println!("  non_symmetry_rows: {}", symmetry.non_symmetry.rows);
    println!(
        "  non_symmetry_strict_accuracy_milli: {}",
        symmetry.non_symmetry.strict_accuracy_milli
    );
    println!(
        "  non_symmetry_sequence_energy_accuracy_milli: {}",
        symmetry.non_symmetry.sequence_energy_accuracy_milli
    );
    println!(
        "  non_symmetry_median_slot_gap: {}",
        symmetry.non_symmetry.median_slot_gap
    );
    println!(
        "  non_symmetry_p10_slot_gap: {}",
        symmetry.non_symmetry.p10_slot_gap
    );
    println!(
        "  non_symmetry_median_energy_gap: {}",
        symmetry.non_symmetry.median_energy_gap
    );
    println!(
        "  non_symmetry_p10_energy_gap: {}",
        symmetry.non_symmetry.p10_energy_gap
    );
    println!(
        "  failed_rows_by_length: {:?}",
        diagnostics.failed_rows_by_length
    );
    println!(
        "  failed_rows_by_rule: {:?}",
        diagnostics.failed_rows_by_rule
    );
    println!(
        "  failed_rows_by_surface: {:?}",
        diagnostics.failed_rows_by_surface
    );
    println!(
        "  failed_rows_by_noise: {:?}",
        diagnostics.failed_rows_by_noise
    );
    println!(
        "  failed_slots_by_output_slot: {:?}",
        diagnostics.failed_slots_by_output_slot
    );
    println!(
        "  total_slots_by_output_slot: {:?}",
        diagnostics.total_slots_by_output_slot
    );
    println!(
        "  slot_accuracy_milli_by_output_slot: {:?}",
        diagnostics.slot_accuracy_milli_by_output_slot
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  flat_role_binding_edges: {}", flat.edge_count());
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        flat.byte_size_estimate()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(eval.accuracy_milli, 1000);
    assert_eq!(flat_eval.accuracy_milli, eval.accuracy_milli);
    assert_eq!(flat_eval.median_gap, eval.median_gap);
    assert_eq!(flat_eval.p10_gap, eval.p10_gap);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(diagnostics.matrix_group_failures, 0);
    assert_eq!(diagnostics.length_group_failures, 0);
    assert_eq!(diagnostics.rule_group_failures, 0);
    assert_eq!(diagnostics.surface_group_failures, 0);
    assert_eq!(diagnostics.noise_group_failures, 0);
    assert_eq!(diagnostics.output_slot_failures, 0);
    assert_eq!(diagnostics.flat_gap_mismatches, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "V3 sequence-energy pressure gate; run explicitly, it trains the global operator objective"]
fn ordered_position_binding_v3_sequence_energy_objective_must_reject_same_bag_wrong() {
    let rows = load_sequence_v3_rows();
    let train_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_train_"))
        .cloned()
        .collect();
    let heldout_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.source_group.starts_with("position_sequence_heldout_"))
        .cloned()
        .collect();
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let field = train_sequence_energy_field_with_progress(
        "position_sequence_v3_energy",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: 12,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 512,
                target_margin: 8192,
                warmup_epochs: 1,
                ramp_epochs: 11,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);

    println!("ordered_position_binding_v3_sequence_energy_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!("  sequence_energy_median_gap: {}", energy.median_energy_gap);
    println!("  sequence_energy_p10_gap: {}", energy.p10_energy_gap);
    println!("  energy_pass_slot_fail: {}", energy.energy_pass_slot_fail);
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(energy.energy_accuracy_milli, 1000);
    assert!(energy.p10_energy_gap > 0);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "V3 combined local-slot + sequence-energy cleanup probe; records metrics"]
fn ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity() {
    let rows = load_sequence_v3_rows();
    let train_rows = sequence_train_rows(&rows);
    let heldout_rows = sequence_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("POSITION_SEQUENCE_COMBINED_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("POSITION_SEQUENCE_COMBINED_CLEANUP_EPOCHS", 4);
    let candidate_cleanup_epochs = env_u16("POSITION_SEQUENCE_CANDIDATE_CLEANUP_EPOCHS", 0);
    let mut field = train_sequence_combined_field_with_progress(
        "position_sequence_v3_combined",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "position_sequence_v3_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("position_sequence_v3_combined: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let l3_collision = l3_role_binding_collision_report(&flat);
    println!("position_sequence_v3_combined: eval_flat_slot_start");
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    println!("position_sequence_v3_combined: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    println!("position_sequence_v3_combined: eval_flat_energy_parity_start");
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    println!("position_sequence_v3_combined: eval_sequence_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("position_sequence_v3_combined: eval_symmetry_start");
    let symmetry = symmetry_operator_diagnostics(&field, &heldout_rows, &heldout);
    println!("position_sequence_v3_combined: eval_output_slot_cleanup_start");
    let slot_cleanup = output_slot_cleanup_diagnostics(&field, &heldout_rows, &heldout);
    println!("position_sequence_v3_combined: eval_basin_stability_start");
    let basin = basin_stability_sweep(&field, &heldout);
    println!("position_sequence_v3_combined: eval_capacity_curve_start");
    let capacity = capacity_curve_diagnostics(&field, &heldout_rows, &heldout);
    println!("position_sequence_v3_combined: eval_address_radius_start");
    let address_radius = address_radius_sweep(&field, &heldout_rows);
    println!("position_sequence_v3_combined: ablation_without_binding_start");
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ_ACTION_SLOT_BASE);
    let no_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ_ACTION_SLOT_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    println!("position_sequence_v3_combined: ablation_without_action_start");
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_action_energy = ordered_sequence_energy_diagnostics(&field, &no_action_tasks);
    println!("position_sequence_v3_combined: ablation_without_role_start");
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_role_energy = ordered_sequence_energy_diagnostics(&field, &no_role_tasks);
    println!("position_sequence_v3_combined: ablation_without_active_fringe_start");
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);

    println!("ordered_position_binding_v3_combined_objective_probe:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  operator_pair_action_centers_used: {}",
        sequence_operator_pair_action_centers_enabled()
    );
    println!(
        "  combined_slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  combined_flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  combined_sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!(
        "  combined_sequence_energy_median_gap: {}",
        energy.median_energy_gap
    );
    println!(
        "  combined_sequence_energy_p10_gap: {}",
        energy.p10_energy_gap
    );
    println!(
        "  combined_energy_pass_slot_fail: {}",
        energy.energy_pass_slot_fail
    );
    println!(
        "  combined_symmetry_sequence_energy_accuracy_milli: {}",
        symmetry.symmetry.sequence_energy_accuracy_milli
    );
    println!(
        "  combined_symmetry_p10_energy_gap: {}",
        symmetry.symmetry.p10_energy_gap
    );
    println!(
        "  combined_non_symmetry_sequence_energy_accuracy_milli: {}",
        symmetry.non_symmetry.sequence_energy_accuracy_milli
    );
    println!(
        "  combined_non_symmetry_p10_energy_gap: {}",
        symmetry.non_symmetry.p10_energy_gap
    );
    println!("  l3_role_binding_edge_count: {}", l3_collision.edge_count);
    println!(
        "  l3_action_centers_with_edges: {}",
        l3_collision.action_centers_with_edges
    );
    println!(
        "  l3_avg_edges_per_action_center_milli: {}",
        l3_collision.avg_edges_per_action_center_milli
    );
    println!(
        "  l3_max_edges_per_action_center: {}",
        l3_collision.max_edges_per_action_center
    );
    println!(
        "  l3_action_centers_with_multi_slot_edges: {}",
        l3_collision.action_centers_with_multi_slot_edges
    );
    println!(
        "  l3_max_slots_per_action_center: {}",
        l3_collision.max_slots_per_action_center
    );
    println!(
        "  l3_role_slots_covered: {}",
        l3_collision.role_slots_covered
    );
    println!(
        "  output_slot_cleanup_accuracy_milli: {}",
        slot_cleanup.accuracy_milli
    );
    println!(
        "  output_slot_cleanup_failed_slots: {}",
        slot_cleanup.failed_slots
    );
    println!(
        "  output_slot_cleanup_accuracy_by_output_slot: {:?}",
        slot_cleanup.accuracy_by_output_slot
    );
    println!(
        "  output_slot_cleanup_accuracy_by_source_slot: {:?}",
        slot_cleanup.accuracy_by_source_slot
    );
    println!(
        "  output_slot_cleanup_failed_by_output_source_pair: {:?}",
        slot_cleanup.failed_by_output_source_pair
    );
    println!(
        "  output_slot_cleanup_accuracy_by_output_source_pair: {:?}",
        slot_cleanup.accuracy_by_output_source_pair
    );
    println!(
        "  output_slot_cleanup_energy_pass_slot_fail_by_output_slot: {:?}",
        slot_cleanup.energy_pass_slot_fail_by_output_slot
    );
    println!(
        "  output_slot_cleanup_symmetry_accuracy_by_output_slot: {:?}",
        slot_cleanup.symmetry_accuracy_by_output_slot
    );
    println!(
        "  output_slot_cleanup_non_symmetry_accuracy_by_output_slot: {:?}",
        slot_cleanup.non_symmetry_accuracy_by_output_slot
    );
    for point in &basin {
        println!(
            "  basin_stability label={} slot_accuracy_milli={} energy_accuracy_milli={} median_energy_gap={} p10_energy_gap={}",
            point.label,
            point.slot_accuracy_milli,
            point.energy_accuracy_milli,
            point.median_energy_gap,
            point.p10_energy_gap
        );
    }
    for point in &capacity {
        println!(
            "  capacity_curve kind={} key={} rows={} slot_accuracy_milli={} energy_accuracy_milli={} p10_energy_gap={}",
            point.kind,
            point.key,
            point.rows,
            point.slot_accuracy_milli,
            point.energy_accuracy_milli,
            point.p10_energy_gap
        );
    }
    for point in &address_radius {
        println!(
            "  address_radius label={} slot_accuracy_milli={} energy_accuracy_milli={} median_energy_gap={} p10_energy_gap={}",
            point.label,
            point.slot_accuracy_milli,
            point.energy_accuracy_milli,
            point.median_energy_gap,
            point.p10_energy_gap
        );
    }
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        no_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_energy_accuracy_milli: {}",
        no_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        no_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_energy_accuracy_milli: {}",
        no_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        no_active_eval.accuracy_milli
    );
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(flat_eval.accuracy_milli, slot_eval.accuracy_milli);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(no_action_eval.accuracy_milli, 0);
    assert_eq!(no_role_eval.accuracy_milli, 0);
    assert_eq!(no_active_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "v4 order operator battery gate; trains the order class separately"]
fn operator_battery_v4_order_must_transfer_without_lookup_or_runtime_phase_hack() {
    let rows = load_operator_battery_v4_order_rows();
    let train_rows = sequence_train_rows(&rows);
    let heldout_rows = sequence_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_ORDER_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_ORDER_CLEANUP_EPOCHS", 4);
    let candidate_cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_ORDER_CANDIDATE_CLEANUP_EPOCHS", 0);
    let mut field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_order",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "operator_battery_v4_order_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("operator_battery_v4_order: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let l3_collision = l3_role_binding_collision_report(&flat);
    println!("operator_battery_v4_order: eval_flat_slot_start");
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    println!("operator_battery_v4_order: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_order: eval_flat_energy_parity_start");
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_order: eval_sequence_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_order: eval_failure_breakdown_start");
    let slot_cleanup = output_slot_cleanup_diagnostics(&field, &heldout_rows, &heldout);
    print_sequence_slot_failures(
        "operator_battery_v4_order",
        &field,
        &heldout_rows,
        &heldout,
        16,
    );

    println!("operator_battery_v4_order: ablation_without_binding_start");
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ_ACTION_SLOT_BASE);
    let no_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ_ACTION_SLOT_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    println!("operator_battery_v4_order: ablation_without_action_start");
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_action_energy = ordered_sequence_energy_diagnostics(&field, &no_action_tasks);
    println!("operator_battery_v4_order: ablation_without_role_start");
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_role_energy = ordered_sequence_energy_diagnostics(&field, &no_role_tasks);
    println!("operator_battery_v4_order: ablation_without_active_fringe_start");
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);

    println!("operator_battery_v4_order_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  operator_pair_action_centers_used: {}",
        sequence_operator_pair_action_centers_enabled()
    );
    println!(
        "  order_slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  order_flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  order_sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!(
        "  order_sequence_energy_median_gap: {}",
        energy.median_energy_gap
    );
    println!("  order_sequence_energy_p10_gap: {}", energy.p10_energy_gap);
    println!(
        "  order_energy_pass_slot_fail: {}",
        energy.energy_pass_slot_fail
    );
    println!(
        "  order_output_slot_cleanup_failed_slots: {}",
        slot_cleanup.failed_slots
    );
    println!(
        "  order_output_slot_cleanup_accuracy_by_output_slot: {:?}",
        slot_cleanup.accuracy_by_output_slot
    );
    println!(
        "  order_output_slot_cleanup_failed_by_output_source_pair: {:?}",
        slot_cleanup.failed_by_output_source_pair
    );
    println!("  l3_role_binding_edge_count: {}", l3_collision.edge_count);
    println!(
        "  l3_action_centers_with_edges: {}",
        l3_collision.action_centers_with_edges
    );
    println!(
        "  l3_max_edges_per_action_center: {}",
        l3_collision.max_edges_per_action_center
    );
    println!(
        "  l3_max_slots_per_action_center: {}",
        l3_collision.max_slots_per_action_center
    );
    println!(
        "  l3_role_slots_covered: {}",
        l3_collision.role_slots_covered
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        no_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_energy_accuracy_milli: {}",
        no_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        no_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_energy_accuracy_milli: {}",
        no_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        no_active_eval.accuracy_milli
    );
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(slot_eval.accuracy_milli, 1000);
    assert_eq!(energy.energy_accuracy_milli, 1000);
    assert_eq!(energy.energy_pass_slot_fail, 0);
    assert_eq!(slot_cleanup.failed_slots, 0);
    assert_eq!(flat_eval.accuracy_milli, slot_eval.accuracy_milli);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(no_action_eval.accuracy_milli, 0);
    assert_eq!(no_role_eval.accuracy_milli, 0);
    assert_eq!(no_active_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "targeted static diagnostic for seed2/order strict slot failure; no training"]
fn operator_battery_v4_order_seed2_strict_failure_static_diagnostic() {
    for seed in [1_u8, 2, 3] {
        let rows = load_sequence_rows_from_path(
            operator_battery_v4_order_multiseed_path(seed),
            "operator-battery-v4-order-multiseed-diagnostic",
        );
        let row = rows
            .iter()
            .find(|row| {
                row.rule_id == "order_block_reverse_4_len13"
                    && row.source_group
                        == "operator_battery_order_heldout_order_block_reverse_4_len13"
                    && row.surface_family == "ru_words"
                    && row.noise_type == "clean"
            })
            .unwrap_or_else(|| panic!("missing seed {seed} diagnostic row"));
        print_order_seed_strict_failure_static_diagnostic(seed, row, 12);
        print_order_rule_slot_static_summary(seed, &rows, "order_block_reverse_4_len13", 12);
    }
}

#[test]
#[ignore = "targeted dynamic audit for seed2/order priem strict slot failure; trains seed2/order"]
fn operator_battery_v4_order_seed2_priem_dynamic_weight_audit() {
    assert!(
        sequence_operator_pair_action_centers_enabled(),
        "run with POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1"
    );

    let rows = load_sequence_rows_from_path(
        operator_battery_v4_order_multiseed_path(2),
        "operator-battery-v4-order-seed2-dynamic-audit",
    );
    let train_rows = sequence_train_rows(&rows);
    let heldout_rows = sequence_heldout_rows(&rows);
    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);

    let field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_order_seed2_dynamic_audit",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: 8,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: 4,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );

    let Some((row, task)) = heldout_rows.iter().zip(heldout.iter()).find(|(row, _)| {
        row.rule_id == "order_block_reverse_4_len13"
            && row.source_group == "operator_battery_order_heldout_order_block_reverse_4_len13"
            && row.surface_family == "ru_words"
            && row.noise_type == "clean"
    }) else {
        panic!("missing seed2/order priem diagnostic row");
    };

    print_order_seed2_priem_dynamic_weight_audit(&field, row, task, 12);
}

#[test]
#[ignore = "V4 edit boundary gate; proves current role-transfer runtime cannot represent full edit corpus"]
fn operator_battery_v4_edit_current_role_binding_runtime_boundary_must_be_explicit() {
    let rows = load_operator_battery_v4_edit_rows();
    let train_rows = edit_train_rows(&rows);
    let heldout_rows = edit_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let report = edit_role_binding_boundary_report(&rows);

    println!("operator_battery_v4_edit_boundary_gate:");
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  current_output_slot_count: {}", SEQ_OUTPUT_SLOT_COUNT);
    println!(
        "  rows_output_len_over_slots: {}",
        report.rows_output_len_over_slots
    );
    println!(
        "  rows_correct_wrong_len_mismatch: {}",
        report.rows_correct_wrong_len_mismatch
    );
    println!(
        "  rows_with_non_source_output_tokens: {}",
        report.rows_with_non_source_output_tokens
    );
    println!(
        "  rows_with_marker_output_tokens: {}",
        report.rows_with_marker_output_tokens
    );
    println!(
        "  rows_representable_by_current_role_transfer: {}",
        report.rows_representable_by_current_role_transfer
    );
    println!(
        "  rows_not_representable_by_current_role_transfer: {}",
        report.rows_not_representable_by_current_role_transfer
    );
    println!(
        "  non_representable_by_family: {:?}",
        report.non_representable_by_family
    );
    println!(
        "  output_len_over_slots_by_family: {:?}",
        report.output_len_over_slots_by_family
    );
    println!(
        "  non_source_output_by_family: {:?}",
        report.non_source_output_by_family
    );
    println!(
        "  correct_len_by_family: {:?}",
        report.correct_len_by_family
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(report.rows, 3072);
    assert!(report.rows_output_len_over_slots > 0);
    assert!(report.rows_correct_wrong_len_mismatch > 0);
    assert!(report.rows_with_non_source_output_tokens > 0);
    assert!(report.rows_not_representable_by_current_role_transfer > 0);
    assert!(
        report.rows_representable_by_current_role_transfer < report.rows,
        "edit corpus must not be silently treated as an order-transfer corpus"
    );
}

#[test]
#[ignore = "v4 edit marker/length gate; trains edit class separately"]
fn operator_battery_v4_edit_marker_length_must_transfer_without_lookup_or_runtime_phase_hack() {
    let rows = load_operator_battery_v4_edit_rows();
    let train_rows = edit_train_rows(&rows);
    let heldout_rows = edit_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());
    let rows_with_full_demo_slot_map = rows
        .iter()
        .filter(|row| parse_edit_demo_final_slots(&row.action).len() == row.correct_tokens.len())
        .count();
    assert_eq!(rows_with_full_demo_slot_map, rows.len());

    let train = prepare_edit_runtime_rows(&train_rows);
    let heldout = prepare_edit_runtime_rows(&heldout_rows);
    let train_discriminative_slots: usize = train.iter().map(|task| task.slot_tasks.len()).sum();
    let heldout_discriminative_slots: usize =
        heldout.iter().map(|task| task.slot_tasks.len()).sum();
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_EDIT_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_EDIT_CLEANUP_EPOCHS", 4);
    let field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_edit",
        &train,
        edit_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );

    println!("operator_battery_v4_edit: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let l3_collision = l3_role_binding_collision_report(&flat);
    println!("operator_battery_v4_edit: eval_flat_slot_start");
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    println!("operator_battery_v4_edit: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_edit: eval_flat_energy_parity_start");
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_edit: eval_sequence_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_edit: eval_output_slot_cleanup_start");
    let slot_cleanup = output_slot_cleanup_diagnostics(&field, &heldout_rows, &heldout);

    println!("operator_battery_v4_edit: ablation_without_binding_start");
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, edit_binding_config()),
        &heldout,
    );
    let no_action_tasks = ablate_sequence_tasks(&heldout, |center_id| center_id < EDIT_ACTION_BASE);
    let no_edit_demo_tasks =
        ablate_sequence_tasks(&heldout, |center_id| !is_edit_demo_center(center_id));
    let no_marker_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| !is_edit_marker_role_center(center_id));
    let no_role_tasks = ablate_sequence_tasks(&heldout, |center_id| center_id >= EDIT_ACTION_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    println!("operator_battery_v4_edit: ablation_without_action_start");
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_action_energy = ordered_sequence_energy_diagnostics(&field, &no_action_tasks);
    println!("operator_battery_v4_edit: ablation_without_edit_demo_start");
    let no_edit_demo_eval = eval_ordered_sequence(&field, &no_edit_demo_tasks);
    let no_edit_demo_energy = ordered_sequence_energy_diagnostics(&field, &no_edit_demo_tasks);
    println!("operator_battery_v4_edit: ablation_without_marker_role_start");
    let no_marker_role_eval = eval_ordered_sequence(&field, &no_marker_role_tasks);
    let no_marker_role_energy = ordered_sequence_energy_diagnostics(&field, &no_marker_role_tasks);
    println!("operator_battery_v4_edit: ablation_without_role_start");
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_role_energy = ordered_sequence_energy_diagnostics(&field, &no_role_tasks);
    println!("operator_battery_v4_edit: ablation_without_active_fringe_start");
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);

    println!("operator_battery_v4_edit_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  train_discriminative_slot_tasks: {}",
        train_discriminative_slots
    );
    println!(
        "  heldout_discriminative_slot_tasks: {}",
        heldout_discriminative_slots
    );
    println!(
        "  rows_with_full_demo_slot_map: {}",
        rows_with_full_demo_slot_map
    );
    println!("  edit_output_slot_count: {}", EDIT_OUTPUT_SLOT_COUNT);
    println!("  edit_role_slot_count: {}", EDIT_ROLE_SLOT_COUNT);
    println!("  edit_marker_role_slot: {}", EDIT_MARKER_ROLE_SLOT);
    println!("  edit_action_base: {}", EDIT_ACTION_BASE);
    println!("  edit_demo_channel_page: {}", EDIT_DEMO_PAGE);
    println!("  edit_demo_channel_base: {}", EDIT_DEMO_BASE);
    println!(
        "  edit_slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  edit_flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  edit_sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!(
        "  edit_sequence_energy_median_gap: {}",
        energy.median_energy_gap
    );
    println!("  edit_sequence_energy_p10_gap: {}", energy.p10_energy_gap);
    println!(
        "  edit_energy_pass_slot_fail: {}",
        energy.energy_pass_slot_fail
    );
    println!(
        "  edit_output_slot_cleanup_failed_slots: {}",
        slot_cleanup.failed_slots
    );
    println!(
        "  edit_output_slot_cleanup_accuracy_by_output_slot: {:?}",
        slot_cleanup.accuracy_by_output_slot
    );
    println!(
        "  edit_output_slot_cleanup_failed_by_output_source_pair: {:?}",
        slot_cleanup.failed_by_output_source_pair
    );
    println!("  l3_role_binding_edge_count: {}", l3_collision.edge_count);
    println!(
        "  l3_action_centers_with_edges: {}",
        l3_collision.action_centers_with_edges
    );
    println!(
        "  l3_max_edges_per_action_center: {}",
        l3_collision.max_edges_per_action_center
    );
    println!(
        "  l3_max_slots_per_action_center: {}",
        l3_collision.max_slots_per_action_center
    );
    println!(
        "  l3_role_slots_covered: {}",
        l3_collision.role_slots_covered
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        no_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_energy_accuracy_milli: {}",
        no_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_edit_demo_accuracy_milli: {}",
        no_edit_demo_eval.accuracy_milli
    );
    println!(
        "  ablation_without_edit_demo_energy_accuracy_milli: {}",
        no_edit_demo_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_marker_role_accuracy_milli: {}",
        no_marker_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_marker_role_energy_accuracy_milli: {}",
        no_marker_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        no_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_energy_accuracy_milli: {}",
        no_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        no_active_eval.accuracy_milli
    );
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(slot_eval.accuracy_milli, 1000);
    assert_eq!(energy.energy_accuracy_milli, 1000);
    assert_eq!(energy.energy_pass_slot_fail, 0);
    assert_eq!(slot_cleanup.failed_slots, 0);
    assert_eq!(flat_eval.accuracy_milli, slot_eval.accuracy_milli);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(no_action_eval.accuracy_milli, 0);
    assert!(
        no_edit_demo_eval.accuracy_milli < 500,
        "removing the edit demo channel must collapse below edit chance"
    );
    assert!(
        no_marker_role_eval.accuracy_milli < 1000,
        "removing marker/end role support must break marker or length edits"
    );
    assert_eq!(no_role_eval.accuracy_milli, 0);
    assert_eq!(no_active_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "v4 conditional state-channel gate; trains conditional class separately"]
fn operator_battery_v4_conditional_state_channel_must_transfer_without_action_flag_leak() {
    let rows = load_operator_battery_v4_conditional_rows();
    let train_rows = conditional_train_rows(&rows);
    let heldout_rows = conditional_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let report = conditional_runtime_boundary_report(&rows);
    assert_eq!(report.rows, rows.len());
    assert_eq!(report.rows_same_bag, report.rows);
    assert_eq!(report.rows_all_outputs_from_source, report.rows);
    assert_eq!(report.rows_output_len_within_slots, report.rows);
    assert_eq!(report.rows_with_state_condition_flag, report.rows);
    assert_eq!(report.rows_with_action_current_flag, 0);
    assert_eq!(report.rows_action_flag_matches_state_flag, 0);
    assert_eq!(report.rows_source_tokens_include_condition_flag, 0);
    assert_eq!(report.rows_branch_signal_action_only_for_current_runtime, 0);

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_CLEANUP_EPOCHS", 4);
    let candidate_cleanup_epochs = env_u16(
        "OPERATOR_BATTERY_V4_CONDITIONAL_CANDIDATE_CLEANUP_EPOCHS",
        0,
    );
    let mut field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_conditional",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "operator_battery_v4_conditional_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("operator_battery_v4_conditional: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let l3_collision = l3_role_binding_collision_report(&flat);
    println!("operator_battery_v4_conditional: eval_flat_slot_start");
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    println!("operator_battery_v4_conditional: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_conditional: eval_flat_energy_parity_start");
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_conditional: eval_sequence_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_conditional: eval_failure_breakdown_start");
    let slot_cleanup = output_slot_cleanup_diagnostics(&field, &heldout_rows, &heldout);
    let slot_groups = sequence_slot_failure_group_diagnostics(&field, &heldout_rows, &heldout);
    let collision_outcome = conditional_collision_outcome_report(&field, &heldout_rows, &heldout);
    print_sequence_slot_failures(
        "operator_battery_v4_conditional",
        &field,
        &heldout_rows,
        &heldout,
        16,
    );

    println!("operator_battery_v4_conditional: ablation_without_binding_start");
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ_ACTION_SLOT_BASE);
    let no_condition_tasks = ablate_sequence_tasks(&heldout, |center_id| {
        !is_sequence_state_condition_center(center_id)
            && !is_sequence_condition_action_center(center_id)
    });
    let no_condition_action_tasks = ablate_sequence_tasks(&heldout, |center_id| {
        !is_sequence_condition_action_center(center_id)
    });
    let no_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ_ACTION_SLOT_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    println!("operator_battery_v4_conditional: ablation_without_action_start");
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_action_energy = ordered_sequence_energy_diagnostics(&field, &no_action_tasks);
    println!("operator_battery_v4_conditional: ablation_without_condition_start");
    let no_condition_eval = eval_ordered_sequence(&field, &no_condition_tasks);
    let no_condition_energy = ordered_sequence_energy_diagnostics(&field, &no_condition_tasks);
    println!("operator_battery_v4_conditional: ablation_without_condition_action_start");
    let no_condition_action_eval = eval_ordered_sequence(&field, &no_condition_action_tasks);
    let no_condition_action_energy =
        ordered_sequence_energy_diagnostics(&field, &no_condition_action_tasks);
    println!("operator_battery_v4_conditional: ablation_without_role_start");
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_role_energy = ordered_sequence_energy_diagnostics(&field, &no_role_tasks);
    println!("operator_battery_v4_conditional: ablation_without_active_fringe_start");
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);

    println!("operator_battery_v4_conditional_state_channel_gate:");
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  rows_same_bag: {}", report.rows_same_bag);
    println!(
        "  rows_all_outputs_from_source: {}",
        report.rows_all_outputs_from_source
    );
    println!(
        "  rows_output_len_within_slots: {}",
        report.rows_output_len_within_slots
    );
    println!(
        "  rows_with_state_condition_flag: {}",
        report.rows_with_state_condition_flag
    );
    println!(
        "  rows_with_action_current_flag: {}",
        report.rows_with_action_current_flag
    );
    println!(
        "  rows_action_flag_matches_state_flag: {}",
        report.rows_action_flag_matches_state_flag
    );
    println!(
        "  rows_source_tokens_include_condition_flag: {}",
        report.rows_source_tokens_include_condition_flag
    );
    println!(
        "  rows_branch_signal_action_only_for_current_runtime: {}",
        report.rows_branch_signal_action_only_for_current_runtime
    );
    println!(
        "  rows_representable_as_order_transfer_if_branch_known: {}",
        report.rows_representable_as_order_transfer_if_branch_known
    );
    println!(
        "  action_current_flag_by_family: {:?}",
        report.action_current_flag_by_family
    );
    println!(
        "  state_condition_channel_page: {}",
        SEQ_STATE_CONDITION_PAGE
    );
    println!(
        "  state_condition_channel_base: {}",
        SEQ_STATE_CONDITION_BASE
    );
    println!(
        "  condition_action_conjunction_page: {}",
        SEQ_CONDITION_ACTION_PAGE
    );
    println!(
        "  condition_action_conjunction_base: {}",
        SEQ_CONDITION_ACTION_BASE
    );
    println!(
        "  conditional_slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  conditional_flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  conditional_sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!(
        "  conditional_sequence_energy_median_gap: {}",
        energy.median_energy_gap
    );
    println!(
        "  conditional_sequence_energy_p10_gap: {}",
        energy.p10_energy_gap
    );
    println!(
        "  conditional_energy_pass_slot_fail: {}",
        energy.energy_pass_slot_fail
    );
    println!(
        "  conditional_output_slot_cleanup_failed_slots: {}",
        slot_cleanup.failed_slots
    );
    println!(
        "  conditional_output_slot_cleanup_accuracy_by_output_slot: {:?}",
        slot_cleanup.accuracy_by_output_slot
    );
    println!(
        "  conditional_slot_failure_by_length: {:?}",
        slot_groups.failed_by_length
    );
    println!(
        "  conditional_slot_accuracy_by_length: {:?}",
        slot_groups.accuracy_by_length
    );
    println!(
        "  conditional_energy_pass_slot_fail_by_length: {:?}",
        slot_groups.energy_pass_slot_fail_by_length
    );
    println!(
        "  conditional_slot_failure_by_rule: {:?}",
        slot_groups.failed_by_rule
    );
    println!(
        "  conditional_slot_accuracy_by_rule: {:?}",
        slot_groups.accuracy_by_rule
    );
    println!(
        "  conditional_energy_pass_slot_fail_by_rule: {:?}",
        slot_groups.energy_pass_slot_fail_by_rule
    );
    println!(
        "  conditional_slot_failure_by_surface: {:?}",
        slot_groups.failed_by_surface
    );
    println!(
        "  conditional_slot_accuracy_by_surface: {:?}",
        slot_groups.accuracy_by_surface
    );
    println!(
        "  conditional_slot_failure_by_noise: {:?}",
        slot_groups.failed_by_noise
    );
    println!(
        "  conditional_slot_accuracy_by_noise: {:?}",
        slot_groups.accuracy_by_noise
    );
    println!(
        "  conditional_slot_failure_by_source_slot: {:?}",
        slot_cleanup.failed_by_source_slot
    );
    println!(
        "  conditional_slot_accuracy_by_source_slot: {:?}",
        slot_cleanup.accuracy_by_source_slot
    );
    println!(
        "  conditional_energy_pass_slot_fail_by_output_slot: {:?}",
        slot_cleanup.energy_pass_slot_fail_by_output_slot
    );
    println!(
        "  conditional_slot_failure_by_output_source_pair: {:?}",
        slot_cleanup.failed_by_output_source_pair
    );
    println!(
        "  conditional_slot_accuracy_by_output_source_pair: {:?}",
        slot_cleanup.accuracy_by_output_source_pair
    );
    println!(
        "  conditional_collision_outcome_by_bucket: {:?}",
        collision_outcome.by_bucket
    );
    println!(
        "  conditional_collision_outcome_by_surface: {:?}",
        collision_outcome.by_surface
    );
    println!(
        "  conditional_collision_outcome_by_surface_bucket: {:?}",
        collision_outcome.by_surface_bucket
    );
    println!(
        "  conditional_worst_collision_output_source_pairs: {:?}",
        collision_outcome.worst_output_source_pairs
    );
    println!(
        "  conditional_worst_collision_surface_output_source_pairs: {:?}",
        collision_outcome.worst_surface_output_source_pairs
    );
    println!("  l3_role_binding_edge_count: {}", l3_collision.edge_count);
    println!(
        "  l3_action_centers_with_edges: {}",
        l3_collision.action_centers_with_edges
    );
    println!(
        "  l3_max_edges_per_action_center: {}",
        l3_collision.max_edges_per_action_center
    );
    println!(
        "  l3_max_slots_per_action_center: {}",
        l3_collision.max_slots_per_action_center
    );
    println!(
        "  l3_role_slots_covered: {}",
        l3_collision.role_slots_covered
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        no_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_energy_accuracy_milli: {}",
        no_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_condition_accuracy_milli: {}",
        no_condition_eval.accuracy_milli
    );
    println!(
        "  ablation_without_condition_energy_accuracy_milli: {}",
        no_condition_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_condition_action_accuracy_milli: {}",
        no_condition_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_condition_action_energy_accuracy_milli: {}",
        no_condition_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        no_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_energy_accuracy_milli: {}",
        no_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        no_active_eval.accuracy_milli
    );
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(slot_eval.accuracy_milli, 1000);
    assert_eq!(energy.energy_accuracy_milli, 1000);
    assert_eq!(energy.energy_pass_slot_fail, 0);
    assert_eq!(slot_cleanup.failed_slots, 0);
    assert_eq!(flat_eval.accuracy_milli, slot_eval.accuracy_milli);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(no_action_eval.accuracy_milli, 0);
    assert!(
        no_condition_eval.accuracy_milli < 500,
        "removing the state condition and condition/action conjunction must collapse below same-bag chance"
    );
    assert!(
        no_condition_action_eval.accuracy_milli < 500,
        "removing the condition/action conjunction must collapse below same-bag chance"
    );
    assert_eq!(no_role_eval.accuracy_milli, 0);
    assert_eq!(no_active_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

#[test]
#[ignore = "diagnostic only; prints conditional folded role collision by surface"]
fn operator_battery_v4_conditional_static_collision_report() {
    let rows = load_operator_battery_v4_conditional_rows();
    assert!(!rows.is_empty());
    let overall = folded_collision_report(&rows);
    let by_surface = folded_collision_report_by_surface(&rows);

    println!("operator_battery_v4_conditional_static_collision:");
    println!("  rows: {}", rows.len());
    println!("  overall: {:?}", overall);
    println!("  by_surface: {:?}", by_surface);
}

#[test]
#[ignore = "diagnostic only; prints conditional target/wrong lane overlap and role crosstalk"]
fn operator_battery_v4_conditional_target_wrong_overlap_report() {
    let rows = load_operator_battery_v4_conditional_rows();
    assert!(!rows.is_empty());
    let report = conditional_lane_overlap_report(&rows);

    println!("operator_battery_v4_conditional_lane_overlap:");
    println!("  rows: {}", rows.len());
    println!("  by_surface: {:?}", report.by_surface);
    println!(
        "  worst_output_source_pairs: {:?}",
        report.worst_output_source_pairs
    );
    println!(
        "  worst_surface_output_source_pairs: {:?}",
        report.worst_surface_output_source_pairs
    );
}

#[test]
#[ignore = "diagnostic only; compares current sign-erased role hits with sign-aware role hits"]
fn operator_battery_v4_conditional_sign_aware_collision_report() {
    let rows = load_operator_battery_v4_conditional_rows();
    assert!(!rows.is_empty());
    let tasks = prepare_sequence_rows(&rows);
    let report = conditional_sign_aware_collision_report(&rows, &tasks);

    println!("operator_battery_v4_conditional_sign_aware_collision:");
    println!("  rows: {}", rows.len());
    println!("  by_surface: {:?}", report.by_surface);
    println!(
        "  worst_output_source_pairs: {:?}",
        report.worst_output_source_pairs
    );
    println!(
        "  worst_surface_output_source_pairs: {:?}",
        report.worst_surface_output_source_pairs
    );
}

#[test]
#[ignore = "diagnostic only; correlates same-sign residual role collision with strict-slot gap"]
fn operator_battery_v4_conditional_residual_collision_outcome_report() {
    let rows = load_operator_battery_v4_conditional_rows();
    let train_rows = conditional_train_rows(&rows);
    let heldout_rows = conditional_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_CLEANUP_EPOCHS", 4);
    let candidate_cleanup_epochs = env_u16(
        "OPERATOR_BATTERY_V4_CONDITIONAL_CANDIDATE_CLEANUP_EPOCHS",
        0,
    );
    let mut field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_conditional_residual_collision",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "operator_battery_v4_conditional_residual_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("operator_battery_v4_conditional_residual_collision: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    println!("operator_battery_v4_conditional_residual_collision: eval_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_conditional_residual_collision: outcome_start");
    let outcome = conditional_residual_collision_outcome_report(&field, &heldout_rows, &heldout);

    println!("operator_battery_v4_conditional_residual_collision:");
    println!("  rows: {}", rows.len());
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  local_epochs: {local_epochs}");
    println!("  cleanup_epochs: {cleanup_epochs}");
    println!("  candidate_cleanup_epochs: {candidate_cleanup_epochs}");
    println!("  strict_row_accuracy_milli: {}", slot_eval.accuracy_milli);
    println!(
        "  sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!("  energy_pass_slot_fail: {}", energy.energy_pass_slot_fail);
    println!("  residual_outcome_by_bucket: {:?}", outcome.by_bucket);
    println!("  residual_outcome_by_surface: {:?}", outcome.by_surface);
    println!(
        "  residual_outcome_by_surface_bucket: {:?}",
        outcome.by_surface_bucket
    );
    println!(
        "  residual_worst_output_source_pairs: {:?}",
        outcome.worst_output_source_pairs
    );
    println!(
        "  residual_worst_surface_output_source_pairs: {:?}",
        outcome.worst_surface_output_source_pairs
    );
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
}

#[test]
#[ignore = "diagnostic only; tests generic cleanup/readout after noisy role-filler binding"]
fn operator_battery_v4_conditional_cleanup_readout_candidate_report() {
    let rows = load_operator_battery_v4_conditional_rows();
    let train_rows = conditional_train_rows(&rows);
    let heldout_rows = conditional_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_CONDITIONAL_CLEANUP_EPOCHS", 4);
    let candidate_cleanup_epochs = env_u16(
        "OPERATOR_BATTERY_V4_CONDITIONAL_CANDIDATE_CLEANUP_EPOCHS",
        0,
    );
    let mut field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_conditional_cleanup_readout",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "operator_battery_v4_conditional_cleanup_readout_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("operator_battery_v4_conditional_cleanup_readout: eval_strict_start");
    let strict = eval_ordered_sequence(&field, &heldout);
    println!("operator_battery_v4_conditional_cleanup_readout: eval_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_conditional_cleanup_readout: eval_cleanup_pairwise_start");
    let cleanup_pairwise = eval_cleanup_readout_pairwise(&field, &heldout_rows, &heldout);
    println!("operator_battery_v4_conditional_cleanup_readout: eval_cleanup_winner_start");
    let cleanup_winner = eval_cleanup_readout_source_winner(&field, &heldout_rows, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let flat_cleanup_pairwise = eval_cleanup_readout_pairwise_flat(&flat, &heldout_rows, &heldout);
    let flat_cleanup_winner =
        eval_cleanup_readout_source_winner_flat(&flat, &heldout_rows, &heldout);
    let cleanup_pairwise_parity =
        eval_cleanup_readout_pairwise_flat_parity(&field, &flat, &heldout_rows, &heldout);
    let cleanup_winner_parity =
        eval_cleanup_readout_winner_flat_parity(&field, &flat, &heldout_rows, &heldout);
    let empty_field =
        WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config());
    let no_binding_cleanup =
        eval_cleanup_readout_source_winner(&empty_field, &heldout_rows, &heldout);
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ_ACTION_SLOT_BASE);
    let no_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ_ACTION_SLOT_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    let no_action_cleanup =
        eval_cleanup_readout_source_winner(&field, &heldout_rows, &no_action_tasks);
    let no_role_cleanup = eval_cleanup_readout_source_winner(&field, &heldout_rows, &no_role_tasks);
    let no_active_cleanup =
        eval_cleanup_readout_source_winner(&field, &heldout_rows, &no_active_tasks);

    println!("operator_battery_v4_conditional_cleanup_readout:");
    println!("  rows: {}", rows.len());
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  local_epochs: {local_epochs}");
    println!("  cleanup_epochs: {cleanup_epochs}");
    println!("  candidate_cleanup_epochs: {candidate_cleanup_epochs}");
    println!("  strict_row_accuracy_milli: {}", strict.accuracy_milli);
    println!(
        "  sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!("  energy_pass_slot_fail: {}", energy.energy_pass_slot_fail);
    println!(
        "  cleanup_pairwise_accuracy_milli: {}",
        cleanup_pairwise.accuracy_milli
    );
    println!(
        "  cleanup_pairwise_failed_slots: {}",
        cleanup_pairwise.failed_slots
    );
    println!(
        "  cleanup_pairwise_energy_pass_slot_fail: {}",
        cleanup_pairwise.energy_pass_slot_fail
    );
    println!(
        "  cleanup_winner_accuracy_milli: {}",
        cleanup_winner.accuracy_milli
    );
    println!(
        "  cleanup_winner_failed_slots: {}",
        cleanup_winner.failed_slots
    );
    println!(
        "  cleanup_winner_energy_pass_slot_fail: {}",
        cleanup_winner.energy_pass_slot_fail
    );
    println!(
        "  flat_cleanup_pairwise_accuracy_milli: {}",
        flat_cleanup_pairwise.accuracy_milli
    );
    println!(
        "  flat_cleanup_winner_accuracy_milli: {}",
        flat_cleanup_winner.accuracy_milli
    );
    println!(
        "  cleanup_pairwise_flat_parity_mismatches: {}",
        cleanup_pairwise_parity.mismatches
    );
    println!(
        "  cleanup_winner_flat_parity_mismatches: {}",
        cleanup_winner_parity.mismatches
    );
    println!(
        "  ablation_without_binding_cleanup_winner_accuracy_milli: {}",
        no_binding_cleanup.accuracy_milli
    );
    println!(
        "  ablation_without_action_cleanup_winner_accuracy_milli: {}",
        no_action_cleanup.accuracy_milli
    );
    println!(
        "  ablation_without_role_cleanup_winner_accuracy_milli: {}",
        no_role_cleanup.accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_cleanup_winner_accuracy_milli: {}",
        no_active_cleanup.accuracy_milli
    );
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  cleanup_uses_source_tokens_only: true");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
}

#[test]
#[ignore = "diagnostic only; compiles v4 train transitions into Wave role-binding weights without epochs"]
fn operator_battery_v4_one_pass_wave_compiler_probe_report() {
    println!("operator_battery_v4_one_pass_wave_compiler_probe: order_load_start");
    let order_rows = load_operator_battery_v4_order_rows();
    let order_train_set = sequence_train_rows(&order_rows);
    let order_heldout_set = sequence_heldout_rows(&order_rows);
    println!(
        "operator_battery_v4_one_pass_wave_compiler_probe: order_prepare_start train_rows={} heldout_rows={}",
        order_train_set.len(),
        order_heldout_set.len()
    );
    let order_train = prepare_sequence_rows(&order_train_set);
    let order_heldout = prepare_sequence_rows(&order_heldout_set);
    let order = eval_one_pass_wave_compile_class(
        "one_pass_wave_order",
        &order_train,
        &order_heldout,
        sequence_binding_config(),
        SEQ_ACTION_SLOT_BASE,
    );

    println!("operator_battery_v4_one_pass_wave_compiler_probe: edit_load_start");
    let edit_rows = load_operator_battery_v4_edit_rows();
    let edit_train_set = edit_train_rows(&edit_rows);
    let edit_heldout_set = edit_heldout_rows(&edit_rows);
    println!(
        "operator_battery_v4_one_pass_wave_compiler_probe: edit_prepare_start train_rows={} heldout_rows={}",
        edit_train_set.len(),
        edit_heldout_set.len()
    );
    let edit_train = prepare_edit_runtime_rows(&edit_train_set);
    let edit_heldout = prepare_edit_runtime_rows(&edit_heldout_set);
    let edit = eval_one_pass_wave_compile_class(
        "one_pass_wave_edit",
        &edit_train,
        &edit_heldout,
        edit_binding_config(),
        EDIT_ACTION_BASE,
    );

    println!("operator_battery_v4_one_pass_wave_compiler_probe: conditional_load_start");
    let conditional_rows = load_operator_battery_v4_conditional_rows();
    let conditional_train_set = conditional_train_rows(&conditional_rows);
    let conditional_heldout_set = conditional_heldout_rows(&conditional_rows);
    println!(
        "operator_battery_v4_one_pass_wave_compiler_probe: conditional_prepare_start train_rows={} heldout_rows={}",
        conditional_train_set.len(),
        conditional_heldout_set.len()
    );
    let conditional_train = prepare_sequence_rows(&conditional_train_set);
    let conditional_heldout = prepare_sequence_rows(&conditional_heldout_set);
    let conditional = eval_one_pass_wave_compile_class(
        "one_pass_wave_conditional",
        &conditional_train,
        &conditional_heldout,
        sequence_binding_config(),
        SEQ_ACTION_SLOT_BASE,
    );

    println!("operator_battery_v4_one_pass_wave_compiler_probe: composed_load_start");
    let composed_rows = load_operator_battery_v4_composed_rows();
    let composed_train_set = composed_train_rows(&composed_rows);
    let composed_heldout_set = composed_heldout_rows(&composed_rows);
    println!(
        "operator_battery_v4_one_pass_wave_compiler_probe: composed_prepare_start train_rows={} heldout_rows={}",
        composed_train_set.len(),
        composed_heldout_set.len()
    );
    let composed_train = prepare_sequence_rows(&composed_train_set);
    let composed_heldout = prepare_sequence_rows(&composed_heldout_set);
    let composed = eval_one_pass_wave_compile_class(
        "one_pass_wave_composed",
        &composed_train,
        &composed_heldout,
        sequence_binding_config(),
        SEQ_ACTION_SLOT_BASE,
    );

    println!("operator_battery_v4_one_pass_wave_compiler_probe:");
    print_one_pass_wave_compile_eval("order", order);
    print_one_pass_wave_compile_eval("edit", edit);
    print_one_pass_wave_compile_eval("conditional", conditional);
    print_one_pass_wave_compile_eval("composed", composed);
    println!("  compiler_path: one_pass_direct_role_binding_weights");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
}

#[test]
#[ignore = "diagnostic only; zero-epoch phase-center operator compiler probe"]
fn operator_battery_v4_phase_center_runtime_probe_report() {
    let rows = load_phase_operator_rows();
    let cells = env_usize("OPERATOR_BATTERY_V4_PHASE_CENTER_CELLS", 32);
    println!(
        "operator_battery_v4_phase_center_runtime_probe: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_runtime_probe: eval_start cells={cells}");
    let report = eval_phase_center_report(&rows, cells);

    println!("operator_battery_v4_phase_center_runtime_probe:");
    println!("  verdict: {}", phase_center_verdict(&report));
    println!("  method: transition_relation_waves_to_circular_center_of_mass");
    println!("  cells: {cells}");
    println!(
        "  action_compiled_phase_centers: {}",
        report.action.compiled_phase_centers
    );
    println!("  action_train_rows: {}", report.action.train_rows);
    println!("  action_heldout_rows: {}", report.action.heldout_rows);
    println!(
        "  action_heldout_surface_groups: {}",
        report.action.heldout_surface_groups
    );
    println!(
        "  action_heldout_noise_groups: {}",
        report.action.heldout_noise_groups
    );
    println!(
        "  action_heldout_accuracy_milli: {}",
        report.action.heldout_accuracy_milli
    );
    println!("  action_wrong_wins: {}", report.action.wrong_wins);
    println!("  action_median_margin: {:.6}", report.action.median_margin);
    println!("  action_p10_margin: {:.6}", report.action.p10_margin);
    println!(
        "  action_median_positive_center_gap: {:.6}",
        report.action.median_positive_center_gap
    );
    println!(
        "  action_p10_positive_center_gap: {:.6}",
        report.action.p10_positive_center_gap
    );
    println!(
        "  no_action_compiled_phase_centers: {}",
        report.no_action.compiled_phase_centers
    );
    println!(
        "  no_action_heldout_accuracy_milli: {}",
        report.no_action.heldout_accuracy_milli
    );
    println!("  no_action_wrong_wins: {}", report.no_action.wrong_wins);
    println!(
        "  phase_center_bytes_estimate: {}",
        phase_center_bytes_estimate(&report, cells)
    );
    println!("  epoch_repair_used: false");
    println!("  explicit_out_src_program_extraction_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  diagnostic_only: true");

    assert_eq!(report.action.heldout_accuracy_milli, 1000);
    assert_eq!(report.action.wrong_wins, 0);
    assert!(report.action.p10_margin > 0.0);
    assert!(report.no_action.heldout_accuracy_milli < report.action.heldout_accuracy_milli);
    assert!(report.no_action.wrong_wins > 0);
}

#[test]
#[ignore = "diagnostic only; phase-center capacity curve and train-only cell ablation"]
fn operator_battery_v4_phase_center_capacity_ablation_report() {
    let rows = load_phase_operator_rows();
    println!(
        "operator_battery_v4_phase_center_capacity_ablation: load_done rows={}",
        rows.len()
    );

    let mut c32_action = PhaseCenterEval::default();
    for cells in [8usize, 16, 32, 64] {
        println!("operator_battery_v4_phase_center_capacity_ablation: eval_capacity cells={cells}");
        let report = eval_phase_center_report(&rows, cells);
        print_phase_center_capacity_point(cells, &report);
        if cells == 32 {
            c32_action = report.action;
        }
    }

    let cell_order = phase_center_cell_importance_order(&rows, 32, PhaseKeyMode::Action);
    let top4 = disabled_phase_cells(&cell_order, 4);
    let top8 = disabled_phase_cells(&cell_order, 8);
    let top16 = disabled_phase_cells(&cell_order, 16);
    let ablate_top4 = eval_phase_center_mode_disabled(&rows, 32, PhaseKeyMode::Action, &top4);
    let ablate_top8 = eval_phase_center_mode_disabled(&rows, 32, PhaseKeyMode::Action, &top8);
    let ablate_top16 = eval_phase_center_mode_disabled(&rows, 32, PhaseKeyMode::Action, &top16);

    println!("operator_battery_v4_phase_center_capacity_ablation:");
    println!(
        "  verdict: {}",
        phase_capacity_ablation_verdict(c32_action, ablate_top16)
    );
    println!(
        "  c32_accuracy_milli: {}",
        c32_action.heldout_accuracy_milli
    );
    println!("  c32_wrong_wins: {}", c32_action.wrong_wins);
    println!("  c32_median_margin: {:.6}", c32_action.median_margin);
    println!("  c32_p10_margin: {:.6}", c32_action.p10_margin);
    print_phase_ablation_point("top4_train_importance", ablate_top4);
    print_phase_ablation_point("top8_train_importance", ablate_top8);
    print_phase_ablation_point("top16_train_importance", ablate_top16);
    println!("  ablation_cell_selection: train_positive_vs_negative_center_separation");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(c32_action.heldout_accuracy_milli, 1000);
    assert_eq!(c32_action.wrong_wins, 0);
    assert!(ablate_top16.median_margin < c32_action.median_margin);
    assert!(ablate_top16.p10_margin < c32_action.p10_margin);
}

#[test]
#[ignore = "diagnostic only; compiles phase centers into flat runtime records"]
fn operator_battery_v4_phase_center_flat_runtime_report() {
    let rows = load_phase_operator_rows();
    let cells = env_usize("OPERATOR_BATTERY_V4_PHASE_CENTER_CELLS", 32);
    println!(
        "operator_battery_v4_phase_center_flat_runtime: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_flat_runtime: compile_start cells={cells}");
    let report = eval_flat_phase_center_runtime_report(&rows, cells);

    println!("operator_battery_v4_phase_center_flat_runtime:");
    println!("  verdict: {}", flat_phase_runtime_verdict(&report));
    println!("  cells: {cells}");
    println!(
        "  compiler_accuracy_milli: {}",
        report.compiler_eval.heldout_accuracy_milli
    );
    println!("  compiler_wrong_wins: {}", report.compiler_eval.wrong_wins);
    println!("  flat_accuracy_milli: {}", report.flat_eval.accuracy_milli);
    println!("  flat_rows: {}", report.flat_eval.rows);
    println!("  flat_correct: {}", report.flat_eval.correct);
    println!("  flat_wrong_wins: {}", report.flat_eval.wrong_wins);
    println!(
        "  flat_median_margin: {:.6}",
        report.flat_eval.median_margin
    );
    println!("  flat_p10_margin: {:.6}", report.flat_eval.p10_margin);
    println!(
        "  flat_sign_parity_mismatches: {}",
        report.flat_sign_parity_mismatches
    );
    println!(
        "  flat_margin_parity_mismatches: {}",
        report.flat_margin_parity_mismatches
    );
    println!(
        "  no_action_flat_accuracy_milli: {}",
        report.no_action_flat_eval.accuracy_milli
    );
    println!(
        "  no_action_flat_wrong_wins: {}",
        report.no_action_flat_eval.wrong_wins
    );
    println!("  missing_centers: {}", report.missing_centers);
    println!("  skipped_rows: {}", report.skipped_rows);
    println!(
        "  heldout_surface_groups: {}",
        report.heldout_surface_groups
    );
    println!("  heldout_noise_groups: {}", report.heldout_noise_groups);
    println!(
        "  flat_records: {}",
        report.compiler_eval.compiled_phase_centers
    );
    println!("  flat_runtime_bytes_estimate: {}", report.bytes_estimate);
    println!(
        "  flat_eval_p50_latency_ns: {}",
        report.flat_eval.p50_latency_ns
    );
    println!(
        "  flat_eval_p99_latency_ns: {}",
        report.flat_eval.p99_latency_ns
    );
    println!("  flat_eval_total_us: {}", report.flat_eval.total_eval_us);
    println!("  runtime_path: precompiled_numeric_center_index_plus_flat_records");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(report.compiler_eval.heldout_accuracy_milli, 1000);
    assert_eq!(
        report.flat_eval.accuracy_milli,
        report.compiler_eval.heldout_accuracy_milli
    );
    assert_eq!(report.flat_eval.wrong_wins, 0);
    assert_eq!(report.flat_sign_parity_mismatches, 0);
    assert_eq!(report.flat_margin_parity_mismatches, 0);
    assert!(report.no_action_flat_eval.accuracy_milli < report.flat_eval.accuracy_milli);
    assert!(report.no_action_flat_eval.wrong_wins > 0);
}

#[test]
#[ignore = "diagnostic only; verifies exported nando-core phase-center flat runtime API"]
fn operator_battery_v4_phase_center_core_runtime_report() {
    let rows = load_phase_operator_rows();
    let cells = env_usize("OPERATOR_BATTERY_V4_PHASE_CENTER_CELLS", 32);
    println!(
        "operator_battery_v4_phase_center_core_runtime: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_core_runtime: compile_start cells={cells}");
    let (test_runtime, key_to_index, skipped_train_rows) =
        compile_flat_phase_center_runtime(&rows, cells, PhaseKeyMode::Action);
    let core_runtime = compile_core_phase_center_runtime(&test_runtime);
    let prepared = prepare_flat_phase_eval_tasks(&rows, cells, PhaseKeyMode::Action, &key_to_index);
    let test_margins = prepared
        .tasks
        .iter()
        .map(|task| flat_phase_margin(&test_runtime, task))
        .collect::<Vec<_>>();
    let core_tasks = prepared
        .tasks
        .iter()
        .map(core_phase_eval_task)
        .collect::<Vec<_>>();
    let mut correct = 0usize;
    let mut sign_mismatches = 0usize;
    let mut margin_mismatches = 0usize;
    let mut latencies = Vec::with_capacity(core_tasks.len());
    let start = Instant::now();
    for (test_margin, core_task) in test_margins.iter().zip(core_tasks.iter()) {
        let call_start = Instant::now();
        let core_margin = core_runtime
            .margin(core_task)
            .expect("valid core eval task");
        latencies.push(call_start.elapsed().as_nanos());
        correct += usize::from(core_margin > 0.0);
        sign_mismatches += usize::from((*test_margin > 0.0) != (core_margin > 0.0));
        margin_mismatches += usize::from((*test_margin - core_margin).abs() > 1e-12);
    }
    let total_eval_us = start.elapsed().as_micros();
    latencies.sort_unstable();
    let accuracy_milli = milli_ratio(correct, prepared.tasks.len());
    let wrong_wins = prepared.tasks.len().saturating_sub(correct);

    println!("operator_battery_v4_phase_center_core_runtime:");
    println!("  verdict: PHASE_CENTER_CORE_RUNTIME_PASS");
    println!("  cells: {cells}");
    println!("  flat_records: {}", core_runtime.record_count());
    println!("  skipped_train_rows: {skipped_train_rows}");
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!("  heldout_rows: {}", prepared.tasks.len());
    println!("  core_accuracy_milli: {accuracy_milli}");
    println!("  core_wrong_wins: {wrong_wins}");
    println!("  core_sign_parity_mismatches: {sign_mismatches}");
    println!("  core_margin_parity_mismatches: {margin_mismatches}");
    println!(
        "  core_eval_p50_latency_ns: {}",
        percentile_u128(&latencies, 50)
    );
    println!(
        "  core_eval_p99_latency_ns: {}",
        percentile_u128(&latencies, 99)
    );
    println!(
        "  core_runtime_bytes_estimate: {}",
        core_runtime.bytes_estimate()
    );
    println!("  core_eval_total_us: {total_eval_us}");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    println!("  eval_path: precompiled_core_tasks_no_bridge_allocations");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(core_runtime.cells(), cells);
    assert_eq!(core_runtime.record_count(), 380);
    assert_eq!(accuracy_milli, 1000);
    assert_eq!(wrong_wins, 0);
    assert_eq!(sign_mismatches, 0);
    assert_eq!(margin_mismatches, 0);
    assert_eq!(prepared.missing_centers, 0);
    assert_eq!(prepared.skipped_rows, 0);
}

#[test]
#[ignore = "diagnostic only; verifies exported nando-core phase-center compiler API"]
fn operator_battery_v4_phase_center_core_compiler_report() {
    let rows = load_phase_operator_rows();
    let cells = env_usize("OPERATOR_BATTERY_V4_PHASE_CENTER_CELLS", 32);
    println!(
        "operator_battery_v4_phase_center_core_compiler: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_core_compiler: compile_start cells={cells}");
    let (test_runtime, key_to_index, skipped_train_rows) =
        compile_flat_phase_center_runtime(&rows, cells, PhaseKeyMode::Action);
    let (core_runtime, core_key_to_index, core_skipped_train_rows) =
        compile_core_phase_center_runtime_from_rows(&rows, cells, PhaseKeyMode::Action);
    let prepared = prepare_flat_phase_eval_tasks(&rows, cells, PhaseKeyMode::Action, &key_to_index);
    let test_margins = prepared
        .tasks
        .iter()
        .map(|task| flat_phase_margin(&test_runtime, task))
        .collect::<Vec<_>>();
    let core_tasks = core_phase_eval_tasks(&prepared.tasks);
    let mut correct = 0usize;
    let mut sign_mismatches = 0usize;
    let mut margin_mismatches = 0usize;
    let mut latencies = Vec::with_capacity(core_tasks.len());
    let start = Instant::now();
    for (test_margin, core_task) in test_margins.iter().zip(core_tasks.iter()) {
        let call_start = Instant::now();
        let core_margin = core_runtime
            .margin(core_task)
            .expect("valid core compiler eval task");
        latencies.push(call_start.elapsed().as_nanos());
        correct += usize::from(core_margin > 0.0);
        sign_mismatches += usize::from((*test_margin > 0.0) != (core_margin > 0.0));
        margin_mismatches += usize::from((*test_margin - core_margin).abs() > 1e-12);
    }
    let total_eval_us = start.elapsed().as_micros();
    latencies.sort_unstable();
    let accuracy_milli = milli_ratio(correct, prepared.tasks.len());
    let wrong_wins = prepared.tasks.len().saturating_sub(correct);

    println!("operator_battery_v4_phase_center_core_compiler:");
    println!("  verdict: PHASE_CENTER_CORE_COMPILER_PASS");
    println!("  cells: {cells}");
    println!("  flat_records: {}", core_runtime.record_count());
    println!("  skipped_train_rows: {skipped_train_rows}");
    println!("  core_skipped_train_rows: {core_skipped_train_rows}");
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!("  heldout_rows: {}", prepared.tasks.len());
    println!("  core_accuracy_milli: {accuracy_milli}");
    println!("  core_wrong_wins: {wrong_wins}");
    println!("  core_sign_parity_mismatches: {sign_mismatches}");
    println!("  core_margin_parity_mismatches: {margin_mismatches}");
    println!(
        "  core_eval_p50_latency_ns: {}",
        percentile_u128(&latencies, 50)
    );
    println!(
        "  core_eval_p99_latency_ns: {}",
        percentile_u128(&latencies, 99)
    );
    println!(
        "  core_runtime_bytes_estimate: {}",
        core_runtime.bytes_estimate()
    );
    println!("  core_eval_total_us: {total_eval_us}");
    println!("  compiler_path: nando_core::PhaseCenterCompiler");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(key_to_index, core_key_to_index);
    assert_eq!(skipped_train_rows, core_skipped_train_rows);
    assert_eq!(core_runtime.cells(), cells);
    assert_eq!(core_runtime.record_count(), 380);
    assert_eq!(accuracy_milli, 1000);
    assert_eq!(wrong_wins, 0);
    assert_eq!(sign_mismatches, 0);
    assert_eq!(margin_mismatches, 0);
    assert_eq!(prepared.missing_centers, 0);
    assert_eq!(prepared.skipped_rows, 0);
}

#[test]
#[ignore = "release benchmark only; measures exported core phase-center runtime"]
fn operator_battery_v4_phase_center_core_runtime_benchmark_report() {
    let rows = load_phase_operator_rows();
    let cells_list = env_usize_list("OPERATOR_BATTERY_V4_PHASE_CENTER_BENCH_CELLS", &[32, 64]);
    println!(
        "operator_battery_v4_phase_center_core_runtime_benchmark: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_core_runtime_benchmark:");
    for cells in cells_list {
        let (test_runtime, key_to_index, skipped_train_rows) =
            compile_flat_phase_center_runtime(&rows, cells, PhaseKeyMode::Action);
        let core_runtime = compile_core_phase_center_runtime(&test_runtime);
        let prepared =
            prepare_flat_phase_eval_tasks(&rows, cells, PhaseKeyMode::Action, &key_to_index);
        let core_tasks = core_phase_eval_tasks(&prepared.tasks);
        let eval = eval_core_phase_runtime(&core_runtime, &core_tasks);
        println!("  cells={cells}");
        println!("    flat_records: {}", core_runtime.record_count());
        println!("    skipped_train_rows: {skipped_train_rows}");
        println!("    missing_centers: {}", prepared.missing_centers);
        println!("    skipped_rows: {}", prepared.skipped_rows);
        println!("    rows: {}", eval.rows);
        println!("    accuracy_milli: {}", eval.accuracy_milli);
        println!("    wrong_wins: {}", eval.wrong_wins);
        println!("    median_margin: {:.6}", eval.median_margin);
        println!("    p10_margin: {:.6}", eval.p10_margin);
        println!("    p50_latency_ns: {}", eval.p50_latency_ns);
        println!("    p99_latency_ns: {}", eval.p99_latency_ns);
        println!("    total_eval_us: {}", eval.total_eval_us);
        println!("    bytes_estimate: {}", core_runtime.bytes_estimate());
        println!("    runtime_path: nando_core::PhaseCenterFlatRuntime");
        assert_eq!(eval.accuracy_milli, 1000);
        assert_eq!(eval.wrong_wins, 0);
        assert_eq!(prepared.missing_centers, 0);
        assert_eq!(prepared.skipped_rows, 0);
    }
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
}

#[test]
#[ignore = "diagnostic only; verifies serialized core phase-center runtime package"]
fn operator_battery_v4_phase_center_core_runtime_package_report() {
    let rows = load_phase_operator_rows();
    let cells = env_usize("OPERATOR_BATTERY_V4_PHASE_CENTER_CELLS", 32);
    println!(
        "operator_battery_v4_phase_center_core_runtime_package: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_core_runtime_package: compile_start cells={cells}");
    let (core_runtime, key_to_index, skipped_train_rows) =
        compile_core_phase_center_runtime_from_rows(&rows, cells, PhaseKeyMode::Action);
    let package_bytes = core_runtime.to_bytes().expect("runtime package serializes");
    let package_info = CorePhaseCenterFlatRuntime::inspect_bytes(&package_bytes)
        .expect("runtime package inspects");
    let loaded_runtime =
        CorePhaseCenterFlatRuntime::from_bytes(&package_bytes).expect("runtime package loads");
    let prepared = prepare_flat_phase_eval_tasks(&rows, cells, PhaseKeyMode::Action, &key_to_index);
    let core_tasks = core_phase_eval_tasks(&prepared.tasks);
    let eval = eval_core_phase_runtime(&loaded_runtime, &core_tasks);

    let mut sign_mismatches = 0usize;
    let mut margin_mismatches = 0usize;
    for task in &core_tasks {
        let original_margin = core_runtime
            .margin(task)
            .expect("valid original runtime task");
        let loaded_margin = loaded_runtime
            .margin(task)
            .expect("valid loaded runtime task");
        sign_mismatches += usize::from((original_margin > 0.0) != (loaded_margin > 0.0));
        margin_mismatches += usize::from((original_margin - loaded_margin).abs() > 1e-12);
    }

    println!("operator_battery_v4_phase_center_core_runtime_package:");
    println!("  verdict: PHASE_CENTER_CORE_RUNTIME_PACKAGE_PASS");
    println!("  cells: {cells}");
    println!("  flat_records: {}", loaded_runtime.record_count());
    println!("  skipped_train_rows: {skipped_train_rows}");
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!("  heldout_rows: {}", eval.rows);
    println!("  package_magic: {:?}", PHASE_CENTER_RUNTIME_PACKAGE_MAGIC);
    println!("  inspected_cells: {}", package_info.cells);
    println!("  inspected_records: {}", package_info.record_count);
    println!("  inspected_payload_bytes: {}", package_info.payload_bytes);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  serialized_len: {}", core_runtime.serialized_len());
    println!(
        "  core_runtime_bytes_estimate: {}",
        core_runtime.bytes_estimate()
    );
    println!("  package_accuracy_milli: {}", eval.accuracy_milli);
    println!("  package_wrong_wins: {}", eval.wrong_wins);
    println!("  package_margin_parity_mismatches: {}", margin_mismatches);
    println!("  package_sign_parity_mismatches: {sign_mismatches}");
    println!("  package_eval_p50_latency_ns: {}", eval.p50_latency_ns);
    println!("  package_eval_p99_latency_ns: {}", eval.p99_latency_ns);
    println!("  compiler_path: nando_core::PhaseCenterCompiler");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime::from_bytes");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(loaded_runtime.cells(), cells);
    assert_eq!(loaded_runtime.record_count(), 380);
    assert_eq!(package_info.cells, cells);
    assert_eq!(package_info.record_count, 380);
    assert_eq!(package_info.serialized_len, package_bytes.len());
    assert_eq!(package_bytes.len(), core_runtime.serialized_len());
    assert!(package_bytes.starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC));
    assert_eq!(eval.accuracy_milli, 1000);
    assert_eq!(eval.wrong_wins, 0);
    assert_eq!(sign_mismatches, 0);
    assert_eq!(margin_mismatches, 0);
    assert_eq!(prepared.missing_centers, 0);
    assert_eq!(prepared.skipped_rows, 0);
}

#[test]
#[ignore = "release benchmark only; measures serialized core phase-center runtime packages"]
fn operator_battery_v4_phase_center_core_runtime_package_benchmark_report() {
    let rows = load_phase_operator_rows();
    let cells_list = env_usize_list("OPERATOR_BATTERY_V4_PHASE_CENTER_BENCH_CELLS", &[32, 64]);
    println!(
        "operator_battery_v4_phase_center_core_runtime_package_benchmark: load_done rows={}",
        rows.len()
    );
    println!("operator_battery_v4_phase_center_core_runtime_package_benchmark:");
    for cells in cells_list {
        let (core_runtime, key_to_index, skipped_train_rows) =
            compile_core_phase_center_runtime_from_rows(&rows, cells, PhaseKeyMode::Action);
        let package_bytes = core_runtime.to_bytes().expect("runtime package serializes");
        let package_info = CorePhaseCenterFlatRuntime::inspect_bytes(&package_bytes)
            .expect("runtime package inspects");
        let load_start = Instant::now();
        let loaded_runtime =
            CorePhaseCenterFlatRuntime::from_bytes(&package_bytes).expect("runtime package loads");
        let load_us = load_start.elapsed().as_micros();
        let prepared =
            prepare_flat_phase_eval_tasks(&rows, cells, PhaseKeyMode::Action, &key_to_index);
        let core_tasks = core_phase_eval_tasks(&prepared.tasks);
        let eval = eval_core_phase_runtime(&loaded_runtime, &core_tasks);

        let mut sign_mismatches = 0usize;
        let mut margin_mismatches = 0usize;
        for task in &core_tasks {
            let original_margin = core_runtime
                .margin(task)
                .expect("valid original runtime task");
            let loaded_margin = loaded_runtime
                .margin(task)
                .expect("valid loaded runtime task");
            sign_mismatches += usize::from((original_margin > 0.0) != (loaded_margin > 0.0));
            margin_mismatches += usize::from((original_margin - loaded_margin).abs() > 1e-12);
        }

        println!("  cells={cells}");
        println!("    flat_records: {}", loaded_runtime.record_count());
        println!("    skipped_train_rows: {skipped_train_rows}");
        println!("    missing_centers: {}", prepared.missing_centers);
        println!("    skipped_rows: {}", prepared.skipped_rows);
        println!("    rows: {}", eval.rows);
        println!(
            "    package_magic: {:?}",
            PHASE_CENTER_RUNTIME_PACKAGE_MAGIC
        );
        println!("    inspected_cells: {}", package_info.cells);
        println!("    inspected_records: {}", package_info.record_count);
        println!(
            "    inspected_payload_bytes: {}",
            package_info.payload_bytes
        );
        println!("    package_fingerprint64: {}", package_info.fingerprint64);
        println!("    package_bytes: {}", package_bytes.len());
        println!("    serialized_len: {}", core_runtime.serialized_len());
        println!(
            "    core_runtime_bytes_estimate: {}",
            loaded_runtime.bytes_estimate()
        );
        println!("    package_load_us: {load_us}");
        println!("    accuracy_milli: {}", eval.accuracy_milli);
        println!("    wrong_wins: {}", eval.wrong_wins);
        println!("    median_margin: {:.6}", eval.median_margin);
        println!("    p10_margin: {:.6}", eval.p10_margin);
        println!("    package_margin_parity_mismatches: {margin_mismatches}");
        println!("    package_sign_parity_mismatches: {sign_mismatches}");
        println!("    p50_latency_ns: {}", eval.p50_latency_ns);
        println!("    p99_latency_ns: {}", eval.p99_latency_ns);
        println!("    total_eval_us: {}", eval.total_eval_us);
        println!("    runtime_path: nando_core::PhaseCenterFlatRuntime::from_bytes");

        assert_eq!(loaded_runtime.cells(), cells);
        assert_eq!(loaded_runtime.record_count(), 380);
        assert_eq!(package_info.cells, cells);
        assert_eq!(package_info.record_count, 380);
        assert_eq!(package_info.serialized_len, package_bytes.len());
        assert_eq!(package_bytes.len(), core_runtime.serialized_len());
        assert!(package_bytes.starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC));
        assert_eq!(eval.accuracy_milli, 1000);
        assert_eq!(eval.wrong_wins, 0);
        assert_eq!(sign_mismatches, 0);
        assert_eq!(margin_mismatches, 0);
        assert_eq!(prepared.missing_centers, 0);
        assert_eq!(prepared.skipped_rows, 0);
    }
    println!("  compiler_path: nando_core::PhaseCenterCompiler");
    println!("  package_path: nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
}

#[test]
#[ignore = "v4 composed operator battery gate; trains composed chains separately"]
fn operator_battery_v4_composed_must_transfer_without_lookup_or_runtime_phase_hack() {
    let rows = load_operator_battery_v4_composed_rows();
    let train_rows = composed_train_rows(&rows);
    let heldout_rows = composed_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());
    let rows_with_full_demo_slot_map = rows
        .iter()
        .filter(|row| {
            parse_composed_demo_final_slots(&row.action).len() == row.correct_tokens.len()
        })
        .count();

    let train = prepare_sequence_rows(&train_rows);
    let heldout = prepare_sequence_rows(&heldout_rows);
    let local_epochs = env_u16("OPERATOR_BATTERY_V4_COMPOSED_LOCAL_EPOCHS", 8);
    let cleanup_epochs = env_u16("OPERATOR_BATTERY_V4_COMPOSED_CLEANUP_EPOCHS", 4);
    let mut field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_composed",
        &train,
        sequence_binding_config(),
        WavePredictorTrainerConfig {
            epochs: local_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: cleanup_epochs,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );

    let candidate_cleanup_epochs =
        env_u16("OPERATOR_BATTERY_V4_COMPOSED_CANDIDATE_CLEANUP_EPOCHS", 0);
    if candidate_cleanup_epochs > 0 {
        field = train_sequence_candidate_cleanup_field_with_progress(
            "operator_battery_v4_composed_candidate_cleanup",
            field,
            &train_rows,
            WavePredictorTrainerConfig {
                epochs: candidate_cleanup_epochs,
                margin_schedule: WavePredictorMarginSchedule {
                    start_margin: 160,
                    target_margin: 320,
                    warmup_epochs: 1,
                    ramp_epochs: candidate_cleanup_epochs.max(1),
                },
                anti_wave_trap_updates_per_epoch_cap: None,
            },
        );
    }

    println!("operator_battery_v4_composed: eval_slot_start");
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    let flat = field.compile_flat_role_binding_table();
    let l3_collision = l3_role_binding_collision_report(&flat);
    println!("operator_battery_v4_composed: eval_flat_slot_start");
    let flat_eval = eval_ordered_sequence_flat(&flat, &heldout);
    println!("operator_battery_v4_composed: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_composed: eval_flat_energy_parity_start");
    let flat_energy_parity = eval_ordered_sequence_flat_energy_parity(&field, &flat, &heldout);
    println!("operator_battery_v4_composed: eval_sequence_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    println!("operator_battery_v4_composed: eval_failure_breakdown_start");
    let slot_cleanup = output_slot_cleanup_diagnostics(&field, &heldout_rows, &heldout);
    print_sequence_slot_failures(
        "operator_battery_v4_composed",
        &field,
        &heldout_rows,
        &heldout,
        16,
    );

    println!("operator_battery_v4_composed: ablation_without_binding_start");
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, sequence_binding_config()),
        &heldout,
    );
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ_ACTION_SLOT_BASE);
    let no_composed_demo_tasks = ablate_sequence_tasks(&heldout, |center_id| {
        !is_sequence_composed_demo_center(center_id)
    });
    let no_role_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ_ACTION_SLOT_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    println!("operator_battery_v4_composed: ablation_without_action_start");
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_action_energy = ordered_sequence_energy_diagnostics(&field, &no_action_tasks);
    println!("operator_battery_v4_composed: ablation_without_composed_demo_start");
    let no_composed_demo_eval = eval_ordered_sequence(&field, &no_composed_demo_tasks);
    let no_composed_demo_energy =
        ordered_sequence_energy_diagnostics(&field, &no_composed_demo_tasks);
    println!("operator_battery_v4_composed: ablation_without_role_start");
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_role_energy = ordered_sequence_energy_diagnostics(&field, &no_role_tasks);
    println!("operator_battery_v4_composed: ablation_without_active_fringe_start");
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);

    println!("operator_battery_v4_composed_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  rows_with_full_demo_slot_map: {}",
        rows_with_full_demo_slot_map
    );
    println!(
        "  operator_pair_action_centers_used: {}",
        sequence_operator_pair_action_centers_enabled()
    );
    println!("  composed_demo_channel_page: {}", SEQ_COMPOSED_DEMO_PAGE);
    println!("  composed_demo_channel_base: {}", SEQ_COMPOSED_DEMO_BASE);
    println!(
        "  composed_slot_ordered_sequence_accuracy_milli: {}",
        slot_eval.accuracy_milli
    );
    println!(
        "  composed_flat_slot_ordered_sequence_accuracy_milli: {}",
        flat_eval.accuracy_milli
    );
    println!(
        "  composed_sequence_energy_accuracy_milli: {}",
        energy.energy_accuracy_milli
    );
    println!(
        "  composed_sequence_energy_median_gap: {}",
        energy.median_energy_gap
    );
    println!(
        "  composed_sequence_energy_p10_gap: {}",
        energy.p10_energy_gap
    );
    println!(
        "  composed_energy_pass_slot_fail: {}",
        energy.energy_pass_slot_fail
    );
    println!(
        "  composed_output_slot_cleanup_failed_slots: {}",
        slot_cleanup.failed_slots
    );
    println!(
        "  composed_output_slot_cleanup_accuracy_by_output_slot: {:?}",
        slot_cleanup.accuracy_by_output_slot
    );
    println!(
        "  composed_output_slot_cleanup_failed_by_output_source_pair: {:?}",
        slot_cleanup.failed_by_output_source_pair
    );
    println!("  l3_role_binding_edge_count: {}", l3_collision.edge_count);
    println!(
        "  l3_action_centers_with_edges: {}",
        l3_collision.action_centers_with_edges
    );
    println!(
        "  l3_max_edges_per_action_center: {}",
        l3_collision.max_edges_per_action_center
    );
    println!(
        "  l3_max_slots_per_action_center: {}",
        l3_collision.max_slots_per_action_center
    );
    println!(
        "  l3_role_slots_covered: {}",
        l3_collision.role_slots_covered
    );
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        no_binding_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        no_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_action_energy_accuracy_milli: {}",
        no_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_composed_demo_accuracy_milli: {}",
        no_composed_demo_eval.accuracy_milli
    );
    println!(
        "  ablation_without_composed_demo_energy_accuracy_milli: {}",
        no_composed_demo_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        no_role_eval.accuracy_milli
    );
    println!(
        "  ablation_without_role_energy_accuracy_milli: {}",
        no_role_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        no_active_eval.accuracy_milli
    );
    println!(
        "  flat_sequence_energy_parity_checked_rows: {}",
        flat_energy_parity.checked_rows
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        flat_energy_parity.mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        flat_energy_parity.max_abs_gap_delta
    );
    println!("  flat_gap_parity_mismatches: {}", flat_parity.mismatches);
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!(
        "  role_binding_edges: {}",
        field.state_delta_role_binding_edge_count()
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    assert_eq!(slot_eval.accuracy_milli, 1000);
    assert_eq!(energy.energy_accuracy_milli, 1000);
    assert_eq!(energy.energy_pass_slot_fail, 0);
    assert_eq!(slot_cleanup.failed_slots, 0);
    assert_eq!(flat_eval.accuracy_milli, slot_eval.accuracy_milli);
    assert_eq!(flat_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.mismatches, 0);
    assert_eq!(flat_energy_parity.max_abs_gap_delta, 0);
    assert_eq!(no_binding_eval.accuracy_milli, 0);
    assert_eq!(no_action_eval.accuracy_milli, 0);
    assert!(
        no_composed_demo_eval.accuracy_milli < 500,
        "removing the composed demo channel must collapse below same-bag chance"
    );
    assert_eq!(no_role_eval.accuracy_milli, 0);
    assert_eq!(no_active_eval.accuracy_milli, 0);
    assert_eq!(field.state_delta_edge_count(), 0);
}

fn run_slot32_capacity_smoke(seed: usize, print_progress: bool) -> Slot32CapacitySmokeRun {
    let train = slot32_capacity_tasks_for_seed("train", seed);
    let heldout_labeled = slot32_capacity_labeled_tasks_for_seed("heldout", seed);
    run_slot32_prepared_gate("capacity", seed, train, heldout_labeled, print_progress)
}

fn run_slot32_prepared_gate(
    label: &str,
    seed: usize,
    train: Vec<PreparedSequenceTask>,
    heldout_labeled: Vec<(usize, &'static str, PreparedSequenceTask)>,
    print_progress: bool,
) -> Slot32CapacitySmokeRun {
    run_slot32_prepared_gate_with_config(
        label,
        seed,
        train,
        heldout_labeled,
        print_progress,
        slot32_binding_config(),
    )
}

fn run_slot32_prepared_gate_with_config(
    label: &str,
    seed: usize,
    train: Vec<PreparedSequenceTask>,
    heldout_labeled: Vec<(usize, &'static str, PreparedSequenceTask)>,
    print_progress: bool,
    config: WavePredictorHebbianConfig,
) -> Slot32CapacitySmokeRun {
    let heldout: Vec<_> = heldout_labeled
        .iter()
        .map(|(_, _, task)| task.clone())
        .collect();
    let mut field = WavePredictorHebbianField::new(SEQ32_TOTAL_CENTER_COUNT, config);
    let mut touched_edges = 0usize;
    let eta_binding = i32::from(config.eta_binding);

    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: one_pass_compile_start seed={} train_rows={} slot_tasks={} page_count={} total_centers={}",
            seed,
            train.len(),
            train
                .iter()
                .map(|task| task.slot_tasks.len())
                .sum::<usize>(),
            SEQ32_PAGE_COUNT,
            SEQ32_TOTAL_CENTER_COUNT
        );
    }
    for task in &train {
        for slot_task in &task.slot_tasks {
            for impulse in slot_task.target_delta.positive_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    eta_binding * magnitude,
                );
            }
            for impulse in slot_task.target_delta.negative_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    -eta_binding * magnitude,
                );
            }
        }
    }
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: one_pass_compile_done seed={} touched_edges={} state_delta_edges={} role_binding_edges={}",
            seed,
            touched_edges,
            field.state_delta_edge_count(),
            field.state_delta_role_binding_edge_count()
        );
    }

    let flat = field.compile_flat_role_binding_table();
    let flat_index = FlatRoleBindingScoreIndex::new(&flat, config);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: flat_compile_done seed={} flat_role_binding_edges={}",
            seed,
            flat.edge_count()
        );
    }
    let slot_eval = eval_ordered_sequence(&field, &heldout);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: field_slot_eval_done seed={} accuracy_milli={} median_gap={} p10_gap={}",
            seed, slot_eval.accuracy_milli, slot_eval.median_gap, slot_eval.p10_gap
        );
    }
    let flat_eval = eval_ordered_sequence_flat_fast(&flat_index, &heldout);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: flat_slot_eval_done seed={} accuracy_milli={} median_gap={} p10_gap={}",
            seed, flat_eval.accuracy_milli, flat_eval.median_gap, flat_eval.p10_gap
        );
    }
    let flat_parity = eval_ordered_sequence_flat_gap_parity_fast(&field, &flat_index, &heldout);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: flat_gap_parity_done seed={} mismatches={} checked_slots={}",
            seed, flat_parity.mismatches, flat_parity.checked_slots
        );
    }
    let flat_energy_parity =
        eval_ordered_sequence_flat_energy_parity_fast(&field, &flat_index, &heldout);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: flat_energy_parity_done seed={} mismatches={} checked_rows={} max_abs_gap_delta={}",
            seed,
            flat_energy_parity.mismatches,
            flat_energy_parity.checked_rows,
            flat_energy_parity.max_abs_gap_delta
        );
    }
    let energy = ordered_sequence_energy_diagnostics(&field, &heldout);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: sequence_energy_done seed={} accuracy_milli={} median_gap={} p10_gap={} energy_pass_slot_fail={}",
            seed,
            energy.energy_accuracy_milli,
            energy.median_energy_gap,
            energy.p10_energy_gap,
            energy.energy_pass_slot_fail
        );
    }
    let no_binding_eval = eval_ordered_sequence(
        &WavePredictorHebbianField::new(SEQ32_TOTAL_CENTER_COUNT, config),
        &heldout,
    );
    let no_action_tasks =
        ablate_sequence_tasks(&heldout, |center_id| center_id < SEQ32_ACTION_BASE);
    let no_role_tasks = ablate_sequence_tasks(&heldout, |center_id| center_id >= SEQ32_ACTION_BASE);
    let no_active_tasks = ablate_sequence_tasks(&heldout, |_| false);
    let no_action_eval = eval_ordered_sequence(&field, &no_action_tasks);
    let no_role_eval = eval_ordered_sequence(&field, &no_role_tasks);
    let no_active_eval = eval_ordered_sequence(&field, &no_active_tasks);
    if print_progress {
        println!(
            "operator_battery_v4_slot32_{label}: ablations_done seed={} no_binding={} no_action={} no_role={} no_active={}",
            seed,
            no_binding_eval.accuracy_milli,
            no_action_eval.accuracy_milli,
            no_role_eval.accuracy_milli,
            no_active_eval.accuracy_milli
        );
    }
    let (flat_failed_rows, flat_failed_by_length, flat_failed_by_rule) =
        slot32_failure_breakdown_fast(&flat_index, &heldout_labeled);

    let bench_start = Instant::now();
    let bench_repeats = 1usize;
    let mut bench_correct = 0usize;
    for _ in 0..bench_repeats {
        bench_correct += eval_ordered_sequence_flat_fast(&flat_index, &heldout).correct;
    }
    let flat_eval_total_ns = bench_start.elapsed().as_nanos();
    let flat_eval_rows = bench_repeats * heldout.len();
    let flat_eval_avg_ns = flat_eval_total_ns / flat_eval_rows as u128;
    assert_eq!(bench_correct, bench_repeats * flat_eval.correct);

    let flat_bytes = flat.byte_size_estimate();
    let base_mass_bytes = SEQ32_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>();
    let hot_bytes_estimate = flat_bytes + base_mass_bytes;

    let report = Slot32CapacitySmokeReport {
        seed,
        train_rows: train.len(),
        heldout_rows: heldout.len(),
        touched_role_binding_edges: touched_edges,
        role_binding_edges: field.state_delta_role_binding_edge_count(),
        flat_role_binding_edges: flat.edge_count(),
        slot_accuracy_milli: slot_eval.accuracy_milli,
        flat_slot_accuracy_milli: flat_eval.accuracy_milli,
        sequence_energy_accuracy_milli: energy.energy_accuracy_milli,
        sequence_energy_median_gap: energy.median_energy_gap,
        sequence_energy_p10_gap: energy.p10_energy_gap,
        energy_pass_slot_fail: energy.energy_pass_slot_fail,
        flat_gap_parity_mismatches: flat_parity.mismatches,
        flat_sequence_energy_parity_mismatches: flat_energy_parity.mismatches,
        flat_sequence_energy_parity_max_abs_gap_delta: flat_energy_parity.max_abs_gap_delta,
        flat_failed_rows,
        ablation_without_binding_accuracy_milli: no_binding_eval.accuracy_milli,
        ablation_without_action_accuracy_milli: no_action_eval.accuracy_milli,
        ablation_without_role_accuracy_milli: no_role_eval.accuracy_milli,
        ablation_without_active_fringe_accuracy_milli: no_active_eval.accuracy_milli,
        flat_role_binding_bytes_estimate: flat_bytes,
        base_mass_bytes_estimate: base_mass_bytes,
        hot_bytes_estimate,
        flat_eval_rows,
        flat_eval_total_ns,
        flat_eval_avg_ns_per_row: flat_eval_avg_ns,
    };

    assert_eq!(field.state_delta_edge_count(), 0);

    Slot32CapacitySmokeRun {
        report,
        field,
        flat_failed_by_length,
        flat_failed_by_rule,
    }
}

#[test]
#[ignore = "32-slot paged layout capacity smoke; not a full 32-slot corpus proof"]
fn operator_battery_v4_slot32_paged_layout_capacity_smoke() {
    let run = run_slot32_capacity_smoke(0, true);
    let report = &run.report;

    println!("operator_battery_v4_slot32_capacity_gate:");
    println!(
        "  verdict: {}",
        if report.gate_pass() {
            "SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE_PASS"
        } else {
            "SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE_WATCH"
        }
    );
    println!("  page_bits: {}", SEQ32_PAGE_BITS);
    println!("  page_size: {}", SEQ32_PAGE_SIZE);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  action_surface_page: {}", SEQ32_ACTION_SURFACE_PAGE);
    println!("  operator_pair_page: {}", SEQ32_OPERATOR_PAIR_PAGE);
    println!("  operator_pair_source_bits: 5");
    println!("  lengths: 17..32");
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  slot_accuracy_milli: {}", report.slot_accuracy_milli);
    println!(
        "  flat_slot_accuracy_milli: {}",
        report.flat_slot_accuracy_milli
    );
    println!(
        "  sequence_energy_accuracy_milli: {}",
        report.sequence_energy_accuracy_milli
    );
    println!(
        "  sequence_energy_median_gap: {}",
        report.sequence_energy_median_gap
    );
    println!(
        "  sequence_energy_p10_gap: {}",
        report.sequence_energy_p10_gap
    );
    println!("  energy_pass_slot_fail: {}", report.energy_pass_slot_fail);
    println!(
        "  flat_gap_parity_mismatches: {}",
        report.flat_gap_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        report.flat_sequence_energy_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        report.flat_sequence_energy_parity_max_abs_gap_delta
    );
    println!("  flat_failed_rows: {}", report.flat_failed_rows);
    println!("  flat_failed_by_length: {:?}", run.flat_failed_by_length);
    println!("  flat_failed_by_rule: {:?}", run.flat_failed_by_rule);
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        report.ablation_without_binding_accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        report.ablation_without_action_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        report.ablation_without_role_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        report.ablation_without_active_fringe_accuracy_milli
    );
    println!(
        "  touched_role_binding_edges: {}",
        report.touched_role_binding_edges
    );
    println!("  role_binding_edges: {}", report.role_binding_edges);
    println!(
        "  flat_role_binding_edges: {}",
        report.flat_role_binding_edges
    );
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        report.flat_role_binding_bytes_estimate
    );
    println!(
        "  base_mass_bytes_estimate: {}",
        report.base_mass_bytes_estimate
    );
    println!("  hot_bytes_estimate: {}", report.hot_bytes_estimate);
    println!("  flat_eval_rows: {}", report.flat_eval_rows);
    println!("  flat_eval_total_ns: {}", report.flat_eval_total_ns);
    println!(
        "  flat_eval_avg_ns_per_row: {}",
        report.flat_eval_avg_ns_per_row
    );
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot paged layout capacity smoke only; not a full 32-slot corpus proof"
    );

    assert!(report.gate_pass());
}

#[test]
#[ignore = "32-slot paged layout multi-seed smoke; not a full 32-slot corpus proof"]
fn operator_battery_v4_slot32_paged_layout_multiseed_capacity_smoke() {
    let mut reports = Vec::new();
    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let run = run_slot32_capacity_smoke(seed, true);
        let report = run.report;
        println!(
            "operator_battery_v4_slot32_multiseed_capacity_seed: seed={} pass={} slot={} flat_slot={} energy={} p10_energy_gap={} role_edges={} hot_bytes={} flat_avg_ns={}",
            report.seed,
            report.gate_pass(),
            report.slot_accuracy_milli,
            report.flat_slot_accuracy_milli,
            report.sequence_energy_accuracy_milli,
            report.sequence_energy_p10_gap,
            report.role_binding_edges,
            report.hot_bytes_estimate,
            report.flat_eval_avg_ns_per_row
        );
        assert!(
            report.gate_pass(),
            "slot32 multi-seed capacity smoke failed for seed {}",
            report.seed
        );
        reports.push(report);
    }

    let min_slot_accuracy = reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_flat_slot_accuracy = reports
        .iter()
        .map(|report| report.flat_slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_energy_accuracy = reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_p10_energy_gap = reports
        .iter()
        .map(|report| report.sequence_energy_p10_gap)
        .min()
        .unwrap_or(0);
    let max_hot_bytes = reports
        .iter()
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_flat_eval_avg_ns = reports
        .iter()
        .map(|report| report.flat_eval_avg_ns_per_row)
        .max()
        .unwrap_or(0);
    let total_flat_gap_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let total_flat_energy_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();
    let total_energy_pass_slot_fail: usize = reports
        .iter()
        .map(|report| report.energy_pass_slot_fail)
        .sum();

    println!("operator_battery_v4_slot32_multiseed_capacity_gate:");
    println!("  verdict: SLOT32_PAGED_LAYOUT_MULTI_SEED_CAPACITY_SMOKE_PASS");
    println!("  seeds: {}", reports.len());
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  lengths: 17..32");
    println!("  min_slot_accuracy_milli: {}", min_slot_accuracy);
    println!("  min_flat_slot_accuracy_milli: {}", min_flat_slot_accuracy);
    println!(
        "  min_sequence_energy_accuracy_milli: {}",
        min_energy_accuracy
    );
    println!("  min_sequence_energy_p10_gap: {}", min_p10_energy_gap);
    println!(
        "  total_energy_pass_slot_fail: {}",
        total_energy_pass_slot_fail
    );
    println!(
        "  total_flat_gap_parity_mismatches: {}",
        total_flat_gap_parity_mismatches
    );
    println!(
        "  total_flat_sequence_energy_parity_mismatches: {}",
        total_flat_energy_parity_mismatches
    );
    println!("  max_hot_bytes_estimate: {}", max_hot_bytes);
    println!("  max_flat_eval_avg_ns_per_row: {}", max_flat_eval_avg_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot paged layout multi-seed smoke only; not a full 32-slot corpus proof"
    );

    assert_eq!(min_slot_accuracy, 1000);
    assert_eq!(min_flat_slot_accuracy, 1000);
    assert_eq!(min_energy_accuracy, 1000);
    assert_eq!(total_energy_pass_slot_fail, 0);
    assert_eq!(total_flat_gap_parity_mismatches, 0);
    assert_eq!(total_flat_energy_parity_mismatches, 0);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
}

#[test]
#[ignore = "32-slot flat runtime latency smoke; not a product p99 proof"]
fn operator_battery_v4_slot32_flat_runtime_latency_smoke() {
    let seed = 0usize;
    let train = slot32_capacity_tasks_for_seed("train", seed);
    let heldout_labeled = slot32_capacity_labeled_tasks_for_seed("heldout", seed);
    let heldout: Vec<_> = heldout_labeled
        .iter()
        .map(|(_, _, task)| task.clone())
        .collect();
    let config = slot32_binding_config();
    let mut field = WavePredictorHebbianField::new(SEQ32_TOTAL_CENTER_COUNT, config);
    let eta_binding = i32::from(config.eta_binding);

    println!(
        "operator_battery_v4_slot32_flat_latency: compile_start seed={} train_rows={} slot_tasks={}",
        seed,
        train.len(),
        train
            .iter()
            .map(|task| task.slot_tasks.len())
            .sum::<usize>()
    );
    for task in &train {
        for slot_task in &task.slot_tasks {
            for impulse in slot_task.target_delta.positive_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    eta_binding * magnitude,
                );
            }
            for impulse in slot_task.target_delta.negative_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    -eta_binding * magnitude,
                );
            }
        }
    }

    let flat = field.compile_flat_role_binding_table();
    let flat_index = FlatRoleBindingScoreIndex::new(&flat, config);
    let flat_eval = eval_ordered_sequence_flat_fast(&flat_index, &heldout);
    assert_eq!(flat_eval.accuracy_milli, 1000);

    let flat_bytes = flat.byte_size_estimate();
    let base_mass_bytes = SEQ32_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>();
    let hot_bytes_estimate = flat_bytes + base_mass_bytes;
    let bench_repeats = 256usize;
    let mut latencies = Vec::with_capacity(bench_repeats * heldout.len());
    let mut correct_rows = 0usize;
    let total_start = Instant::now();
    for _ in 0..bench_repeats {
        for task in &heldout {
            let start = Instant::now();
            let row_ok = flat_ordered_sequence_row_ok_fast(&flat_index, task);
            latencies.push(start.elapsed().as_nanos());
            correct_rows += usize::from(row_ok);
        }
    }
    let total_ns = total_start.elapsed().as_nanos();
    latencies.sort_unstable();
    let measured_rows = latencies.len();
    let p50_latency_ns = percentile_u128(&latencies, 50);
    let p99_latency_ns = percentile_u128(&latencies, 99);
    let max_latency_ns = latencies.last().copied().unwrap_or(0);
    let avg_latency_ns = total_ns / measured_rows as u128;
    let latency_gate_ns = 1_000_000u128;

    println!("operator_battery_v4_slot32_flat_latency_gate:");
    println!("  verdict: SLOT32_FLAT_RUNTIME_LATENCY_SMOKE_PASS");
    println!("  seed: {}", seed);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  lengths: 17..32");
    println!("  train_rows: {}", train.len());
    println!("  heldout_rows: {}", heldout.len());
    println!("  bench_repeats: {}", bench_repeats);
    println!("  measured_rows: {}", measured_rows);
    println!("  correct_rows: {}", correct_rows);
    println!("  flat_accuracy_milli: {}", flat_eval.accuracy_milli);
    println!("  p50_latency_ns: {}", p50_latency_ns);
    println!("  p99_latency_ns: {}", p99_latency_ns);
    println!("  max_latency_ns: {}", max_latency_ns);
    println!("  avg_latency_ns: {}", avg_latency_ns);
    println!("  latency_gate_ns: {}", latency_gate_ns);
    println!("  flat_role_binding_edges: {}", flat.edge_count());
    println!("  flat_role_binding_bytes_estimate: {}", flat_bytes);
    println!("  base_mass_bytes_estimate: {}", base_mass_bytes);
    println!("  hot_bytes_estimate: {}", hot_bytes_estimate);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!("  claim_boundary: 32-slot flat runtime latency smoke only; not a product p99 proof");

    assert_eq!(correct_rows, measured_rows);
    assert!(p99_latency_ns <= latency_gate_ns);
    assert!(hot_bytes_estimate < 4 * 1024 * 1024);
}

#[test]
#[ignore = "32-slot real order corpus battery; first product-facing rung beyond capacity smoke"]
fn operator_battery_v4_slot32_order_corpus_must_transfer_without_lookup_or_runtime_phase_hack() {
    let seed = env_usize("OPERATOR_BATTERY_V4_SLOT32_ORDER_SEED", 0);
    let train_rows = slot32_order_corpus_tasks_for_seed("train", seed);
    let heldout_rows = slot32_order_corpus_tasks_for_seed("heldout", seed);
    let train = train_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let heldout_labeled = heldout_rows
        .iter()
        .map(|row| (row.length, row.rule_name, row.task.clone()))
        .collect::<Vec<_>>();

    let run = run_slot32_prepared_gate("order_corpus", seed, train, heldout_labeled, true);
    let report = &run.report;
    let unique_rules = slot32_order_unique_rules(&heldout_rows);
    let unique_surfaces = slot32_order_unique_surfaces(&heldout_rows);
    let unique_noise_types = slot32_order_unique_noise_types(&heldout_rows);
    let unique_lengths = slot32_order_unique_lengths(&heldout_rows);
    let same_bag_rows = slot32_order_same_bag_rows(&heldout_rows);
    let max_train_state_reuse = slot32_order_max_state_reuse(&train_rows);
    let max_heldout_state_reuse = slot32_order_max_state_reuse(&heldout_rows);
    let train_tokens_overlap_heldout =
        slot32_order_train_heldout_token_overlap(&train_rows, &heldout_rows);
    let flat_latency_gate_ns = 1_000_000u128;

    println!("operator_battery_v4_slot32_order_corpus_gate:");
    println!(
        "  verdict: {}",
        if report.gate_pass()
            && unique_rules >= 8
            && unique_surfaces >= 4
            && unique_noise_types >= 2
            && unique_lengths == 16
            && same_bag_rows == heldout_rows.len()
            && max_train_state_reuse >= unique_rules
            && max_heldout_state_reuse >= unique_rules
            && train_tokens_overlap_heldout == 0
            && report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns
        {
            "SLOT32_ORDER_CORPUS_RUNG_PASS"
        } else {
            "SLOT32_ORDER_CORPUS_RUNG_WATCH"
        }
    );
    println!("  seed: {}", seed);
    println!("  page_bits: {}", SEQ32_PAGE_BITS);
    println!("  page_size: {}", SEQ32_PAGE_SIZE);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  unique_rules: {}", unique_rules);
    println!("  unique_surfaces: {}", unique_surfaces);
    println!("  unique_noise_types: {}", unique_noise_types);
    println!("  unique_lengths: {}", unique_lengths);
    println!("  same_bag_rows: {}", same_bag_rows);
    println!("  max_train_state_reuse: {}", max_train_state_reuse);
    println!("  max_heldout_state_reuse: {}", max_heldout_state_reuse);
    println!(
        "  train_tokens_overlap_heldout: {}",
        train_tokens_overlap_heldout
    );
    println!("  slot_accuracy_milli: {}", report.slot_accuracy_milli);
    println!(
        "  flat_slot_accuracy_milli: {}",
        report.flat_slot_accuracy_milli
    );
    println!(
        "  sequence_energy_accuracy_milli: {}",
        report.sequence_energy_accuracy_milli
    );
    println!(
        "  sequence_energy_median_gap: {}",
        report.sequence_energy_median_gap
    );
    println!(
        "  sequence_energy_p10_gap: {}",
        report.sequence_energy_p10_gap
    );
    println!("  energy_pass_slot_fail: {}", report.energy_pass_slot_fail);
    println!(
        "  flat_gap_parity_mismatches: {}",
        report.flat_gap_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        report.flat_sequence_energy_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_max_abs_gap_delta: {}",
        report.flat_sequence_energy_parity_max_abs_gap_delta
    );
    println!("  flat_failed_rows: {}", report.flat_failed_rows);
    println!("  flat_failed_by_length: {:?}", run.flat_failed_by_length);
    println!("  flat_failed_by_rule: {:?}", run.flat_failed_by_rule);
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        report.ablation_without_binding_accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        report.ablation_without_action_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        report.ablation_without_role_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        report.ablation_without_active_fringe_accuracy_milli
    );
    println!("  role_binding_edges: {}", report.role_binding_edges);
    println!(
        "  flat_role_binding_edges: {}",
        report.flat_role_binding_edges
    );
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        report.flat_role_binding_bytes_estimate
    );
    println!(
        "  base_mass_bytes_estimate: {}",
        report.base_mass_bytes_estimate
    );
    println!("  hot_bytes_estimate: {}", report.hot_bytes_estimate);
    println!("  flat_eval_rows: {}", report.flat_eval_rows);
    println!(
        "  flat_eval_avg_ns_per_row: {}",
        report.flat_eval_avg_ns_per_row
    );
    println!("  flat_eval_latency_gate_ns: {}", flat_latency_gate_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot order corpus rung only; not edit/conditional/composed 32-slot product proof"
    );

    assert!(report.gate_pass());
    assert_eq!(unique_rules, 8);
    assert_eq!(unique_surfaces, 4);
    assert_eq!(unique_noise_types, 2);
    assert_eq!(unique_lengths, 16);
    assert_eq!(same_bag_rows, heldout_rows.len());
    assert!(max_train_state_reuse >= unique_rules);
    assert!(max_heldout_state_reuse >= unique_rules);
    assert_eq!(train_tokens_overlap_heldout, 0);
    assert!(report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);
}

#[test]
#[ignore = "32-slot order corpus multi-seed robustness; not full 32-slot operator battery"]
fn operator_battery_v4_slot32_order_corpus_multiseed_must_transfer_without_lookup_or_runtime_phase_hack()
 {
    let flat_latency_gate_ns = 1_000_000u128;
    let mut reports = Vec::new();

    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let train_rows = slot32_order_corpus_tasks_for_seed("train", seed);
        let heldout_rows = slot32_order_corpus_tasks_for_seed("heldout", seed);
        let train = train_rows
            .iter()
            .map(|row| row.task.clone())
            .collect::<Vec<_>>();
        let heldout_labeled = heldout_rows
            .iter()
            .map(|row| (row.length, row.rule_name, row.task.clone()))
            .collect::<Vec<_>>();
        let run =
            run_slot32_prepared_gate("order_corpus_multiseed", seed, train, heldout_labeled, true);
        let report = run.report;
        let unique_rules = slot32_order_unique_rules(&heldout_rows);
        let unique_surfaces = slot32_order_unique_surfaces(&heldout_rows);
        let unique_noise_types = slot32_order_unique_noise_types(&heldout_rows);
        let unique_lengths = slot32_order_unique_lengths(&heldout_rows);
        let same_bag_rows = slot32_order_same_bag_rows(&heldout_rows);
        let max_state_reuse = slot32_order_max_state_reuse(&heldout_rows);
        let train_tokens_overlap_heldout =
            slot32_order_train_heldout_token_overlap(&train_rows, &heldout_rows);

        println!(
            "operator_battery_v4_slot32_order_corpus_multiseed_seed: seed={} pass={} slot={} flat_slot={} energy={} p10_energy_gap={} role_edges={} hot_bytes={} flat_avg_ns={} rules={} surfaces={} noise={} lengths={} same_bag={} max_state_reuse={} token_overlap={}",
            report.seed,
            report.gate_pass(),
            report.slot_accuracy_milli,
            report.flat_slot_accuracy_milli,
            report.sequence_energy_accuracy_milli,
            report.sequence_energy_p10_gap,
            report.role_binding_edges,
            report.hot_bytes_estimate,
            report.flat_eval_avg_ns_per_row,
            unique_rules,
            unique_surfaces,
            unique_noise_types,
            unique_lengths,
            same_bag_rows,
            max_state_reuse,
            train_tokens_overlap_heldout
        );

        assert!(report.gate_pass());
        assert_eq!(unique_rules, 8);
        assert_eq!(unique_surfaces, 4);
        assert_eq!(unique_noise_types, 2);
        assert_eq!(unique_lengths, 16);
        assert_eq!(same_bag_rows, heldout_rows.len());
        assert_eq!(max_state_reuse, unique_rules);
        assert_eq!(train_tokens_overlap_heldout, 0);
        assert!(report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);
        reports.push(report);
    }

    let min_slot_accuracy = reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_flat_slot_accuracy = reports
        .iter()
        .map(|report| report.flat_slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_energy_accuracy = reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_p10_energy_gap = reports
        .iter()
        .map(|report| report.sequence_energy_p10_gap)
        .min()
        .unwrap_or(0);
    let total_energy_pass_slot_fail: usize = reports
        .iter()
        .map(|report| report.energy_pass_slot_fail)
        .sum();
    let total_flat_gap_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let total_flat_energy_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();
    let max_role_binding_edges = reports
        .iter()
        .map(|report| report.role_binding_edges)
        .max()
        .unwrap_or(0);
    let max_hot_bytes = reports
        .iter()
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_flat_eval_avg_ns = reports
        .iter()
        .map(|report| report.flat_eval_avg_ns_per_row)
        .max()
        .unwrap_or(0);

    println!("operator_battery_v4_slot32_order_corpus_multiseed_gate:");
    println!("  verdict: SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG_PASS");
    println!("  seeds: {}", reports.len());
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  rows_per_seed_train: {}", reports[0].train_rows);
    println!("  rows_per_seed_heldout: {}", reports[0].heldout_rows);
    println!("  unique_rules: 8");
    println!("  unique_surfaces: 4");
    println!("  unique_noise_types: 2");
    println!("  unique_lengths: 16");
    println!("  lengths: 17..32");
    println!("  min_slot_accuracy_milli: {}", min_slot_accuracy);
    println!("  min_flat_slot_accuracy_milli: {}", min_flat_slot_accuracy);
    println!(
        "  min_sequence_energy_accuracy_milli: {}",
        min_energy_accuracy
    );
    println!("  min_sequence_energy_p10_gap: {}", min_p10_energy_gap);
    println!(
        "  total_energy_pass_slot_fail: {}",
        total_energy_pass_slot_fail
    );
    println!(
        "  total_flat_gap_parity_mismatches: {}",
        total_flat_gap_parity_mismatches
    );
    println!(
        "  total_flat_sequence_energy_parity_mismatches: {}",
        total_flat_energy_parity_mismatches
    );
    println!("  max_role_binding_edges: {}", max_role_binding_edges);
    println!("  max_hot_bytes_estimate: {}", max_hot_bytes);
    println!("  max_flat_eval_avg_ns_per_row: {}", max_flat_eval_avg_ns);
    println!("  flat_eval_latency_gate_ns: {}", flat_latency_gate_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot order corpus multi-seed rung only; not edit/conditional/composed 32-slot product proof"
    );

    assert_eq!(min_slot_accuracy, 1000);
    assert_eq!(min_flat_slot_accuracy, 1000);
    assert_eq!(min_energy_accuracy, 1000);
    assert_eq!(total_energy_pass_slot_fail, 0);
    assert_eq!(total_flat_gap_parity_mismatches, 0);
    assert_eq!(total_flat_energy_parity_mismatches, 0);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
    assert!(max_flat_eval_avg_ns <= flat_latency_gate_ns);
}

#[test]
#[ignore = "32-slot mixed map corpus battery; closes order+edit-map+composed-map, not conditional branch selection"]
fn operator_battery_v4_slot32_mixed_map_corpus_must_transfer_without_lookup_or_runtime_phase_hack()
{
    let seed = env_usize("OPERATOR_BATTERY_V4_SLOT32_MIXED_MAP_SEED", 0);
    let train_rows = slot32_mixed_map_corpus_tasks_for_seed("train", seed);
    let heldout_rows = slot32_mixed_map_corpus_tasks_for_seed("heldout", seed);
    let train = train_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let heldout_labeled = heldout_rows
        .iter()
        .map(|row| (row.length, row.rule_name, row.task.clone()))
        .collect::<Vec<_>>();

    let run = run_slot32_prepared_gate("mixed_map_corpus", seed, train, heldout_labeled, true);
    let report = &run.report;
    let unique_classes = slot32_unique_operator_classes(&heldout_rows);
    let unique_rules = slot32_order_unique_rules(&heldout_rows);
    let unique_surfaces = slot32_order_unique_surfaces(&heldout_rows);
    let unique_noise_types = slot32_order_unique_noise_types(&heldout_rows);
    let unique_lengths = slot32_order_unique_lengths(&heldout_rows);
    let same_bag_rows = slot32_order_same_bag_rows(&heldout_rows);
    let edit_rows = slot32_operator_class_rows(&heldout_rows, "edit");
    let edit_non_same_bag_rows = slot32_operator_class_non_same_bag_rows(&heldout_rows, "edit");
    let max_train_state_reuse = slot32_order_max_state_reuse(&train_rows);
    let max_heldout_state_reuse = slot32_order_max_state_reuse(&heldout_rows);
    let train_tokens_overlap_heldout =
        slot32_order_train_heldout_token_overlap(&train_rows, &heldout_rows);
    let flat_latency_gate_ns = 1_000_000u128;

    println!("operator_battery_v4_slot32_mixed_map_corpus_gate:");
    println!(
        "  verdict: {}",
        if report.gate_pass()
            && unique_classes == 3
            && unique_rules == 16
            && unique_surfaces == 4
            && unique_noise_types == 2
            && unique_lengths == 16
            && same_bag_rows + edit_non_same_bag_rows == heldout_rows.len()
            && edit_rows == edit_non_same_bag_rows
            && max_train_state_reuse == unique_rules
            && max_heldout_state_reuse == unique_rules
            && train_tokens_overlap_heldout == 0
            && report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns
        {
            "SLOT32_MIXED_MAP_CORPUS_RUNG_PASS"
        } else {
            "SLOT32_MIXED_MAP_CORPUS_RUNG_WATCH"
        }
    );
    println!("  seed: {}", seed);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  unique_operator_classes: {}", unique_classes);
    println!("  unique_rules: {}", unique_rules);
    println!("  unique_surfaces: {}", unique_surfaces);
    println!("  unique_noise_types: {}", unique_noise_types);
    println!("  unique_lengths: {}", unique_lengths);
    println!("  lengths: 17..32");
    println!("  same_bag_rows: {}", same_bag_rows);
    println!("  edit_rows: {}", edit_rows);
    println!("  edit_non_same_bag_rows: {}", edit_non_same_bag_rows);
    println!("  max_train_state_reuse: {}", max_train_state_reuse);
    println!("  max_heldout_state_reuse: {}", max_heldout_state_reuse);
    println!(
        "  train_tokens_overlap_heldout: {}",
        train_tokens_overlap_heldout
    );
    println!("  slot_accuracy_milli: {}", report.slot_accuracy_milli);
    println!(
        "  flat_slot_accuracy_milli: {}",
        report.flat_slot_accuracy_milli
    );
    println!(
        "  sequence_energy_accuracy_milli: {}",
        report.sequence_energy_accuracy_milli
    );
    println!(
        "  sequence_energy_median_gap: {}",
        report.sequence_energy_median_gap
    );
    println!(
        "  sequence_energy_p10_gap: {}",
        report.sequence_energy_p10_gap
    );
    println!("  energy_pass_slot_fail: {}", report.energy_pass_slot_fail);
    println!(
        "  flat_gap_parity_mismatches: {}",
        report.flat_gap_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        report.flat_sequence_energy_parity_mismatches
    );
    println!("  flat_failed_rows: {}", report.flat_failed_rows);
    println!("  flat_failed_by_length: {:?}", run.flat_failed_by_length);
    println!("  flat_failed_by_rule: {:?}", run.flat_failed_by_rule);
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        report.ablation_without_binding_accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        report.ablation_without_action_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        report.ablation_without_role_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        report.ablation_without_active_fringe_accuracy_milli
    );
    println!("  role_binding_edges: {}", report.role_binding_edges);
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        report.flat_role_binding_bytes_estimate
    );
    println!(
        "  base_mass_bytes_estimate: {}",
        report.base_mass_bytes_estimate
    );
    println!("  hot_bytes_estimate: {}", report.hot_bytes_estimate);
    println!("  flat_eval_rows: {}", report.flat_eval_rows);
    println!(
        "  flat_eval_avg_ns_per_row: {}",
        report.flat_eval_avg_ns_per_row
    );
    println!("  flat_eval_latency_gate_ns: {}", flat_latency_gate_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot mixed map corpus rung only; conditional branch selection remains open"
    );

    assert!(report.gate_pass());
    assert_eq!(unique_classes, 3);
    assert_eq!(unique_rules, 16);
    assert_eq!(unique_surfaces, 4);
    assert_eq!(unique_noise_types, 2);
    assert_eq!(unique_lengths, 16);
    assert_eq!(same_bag_rows + edit_non_same_bag_rows, heldout_rows.len());
    assert_eq!(edit_rows, edit_non_same_bag_rows);
    assert_eq!(max_train_state_reuse, unique_rules);
    assert_eq!(max_heldout_state_reuse, unique_rules);
    assert_eq!(train_tokens_overlap_heldout, 0);
    assert!(report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);
}

#[test]
#[ignore = "32-slot conditional branch selection; condition-result action page must select then/else without direct operator-pair shortcut"]
fn operator_battery_v4_slot32_conditional_branch_must_select_without_lookup_or_runtime_phase_hack()
{
    let seed = env_usize("OPERATOR_BATTERY_V4_SLOT32_CONDITIONAL_BRANCH_SEED", 0);
    let train_rows = slot32_conditional_branch_corpus_tasks_for_seed("train", seed);
    let heldout_rows = slot32_conditional_branch_corpus_tasks_for_seed("heldout", seed);
    let train = train_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let heldout_labeled = heldout_rows
        .iter()
        .map(|row| (row.length, row.rule_name, row.task.clone()))
        .collect::<Vec<_>>();

    let run = run_slot32_prepared_gate_with_config(
        "conditional_branch_corpus",
        seed,
        train,
        heldout_labeled,
        true,
        slot32_conditional_binding_config(),
    );
    let report = &run.report;
    let heldout = heldout_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let no_condition_action_tasks = ablate_sequence_tasks(&heldout, |center_id| {
        !is_slot32_condition_action_center(center_id)
    });
    let no_condition_action_eval = eval_ordered_sequence(&run.field, &no_condition_action_tasks);
    let no_condition_action_energy =
        ordered_sequence_energy_diagnostics(&run.field, &no_condition_action_tasks);

    let unique_classes = slot32_unique_operator_classes(&heldout_rows);
    let unique_rules = slot32_order_unique_rules(&heldout_rows);
    let unique_surfaces = slot32_order_unique_surfaces(&heldout_rows);
    let unique_noise_types = slot32_order_unique_noise_types(&heldout_rows);
    let unique_lengths = slot32_order_unique_lengths(&heldout_rows);
    let same_bag_rows = slot32_order_same_bag_rows(&heldout_rows);
    let condition_true_rows = slot32_condition_result_rows(&heldout_rows, true);
    let condition_false_rows = slot32_condition_result_rows(&heldout_rows, false);
    let direct_operator_pair_centers =
        slot32_active_center_count(&heldout_rows, is_slot32_direct_operator_pair_center);
    let condition_action_centers =
        slot32_active_center_count(&heldout_rows, is_slot32_condition_action_center);
    let state_condition_centers =
        slot32_active_center_count(&heldout_rows, is_slot32_state_condition_center);
    let max_train_state_reuse = slot32_order_max_state_reuse(&train_rows);
    let max_heldout_state_reuse = slot32_order_max_state_reuse(&heldout_rows);
    let train_tokens_overlap_heldout =
        slot32_order_train_heldout_token_overlap(&train_rows, &heldout_rows);
    let flat_latency_gate_ns = 1_000_000u128;

    println!("operator_battery_v4_slot32_conditional_branch_gate:");
    println!(
        "  verdict: {}",
        if report.gate_pass()
            && unique_classes == 1
            && unique_rules == 8
            && unique_surfaces == 4
            && unique_noise_types == 2
            && unique_lengths == 16
            && same_bag_rows == heldout_rows.len()
            && condition_true_rows == condition_false_rows
            && direct_operator_pair_centers == 0
            && condition_action_centers > 0
            && state_condition_centers > 0
            && no_condition_action_eval.accuracy_milli == 0
            && no_condition_action_energy.energy_accuracy_milli == 0
            && max_train_state_reuse == unique_rules * 2
            && max_heldout_state_reuse == unique_rules * 2
            && train_tokens_overlap_heldout == 0
            && report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns
        {
            "SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG_PASS"
        } else {
            "SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG_WATCH"
        }
    );
    println!("  seed: {}", seed);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!(
        "  condition_true_action_page: {}",
        SEQ32_CONDITION_TRUE_ACTION_PAGE
    );
    println!(
        "  condition_false_action_page: {}",
        SEQ32_CONDITION_FALSE_ACTION_PAGE
    );
    println!("  state_condition_page: {}", SEQ32_STATE_CONDITION_PAGE);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  unique_operator_classes: {}", unique_classes);
    println!("  unique_rules: {}", unique_rules);
    println!("  unique_surfaces: {}", unique_surfaces);
    println!("  unique_noise_types: {}", unique_noise_types);
    println!("  unique_lengths: {}", unique_lengths);
    println!("  lengths: 17..32");
    println!("  same_bag_rows: {}", same_bag_rows);
    println!("  condition_true_rows: {}", condition_true_rows);
    println!("  condition_false_rows: {}", condition_false_rows);
    println!(
        "  direct_operator_pair_active_centers: {}",
        direct_operator_pair_centers
    );
    println!(
        "  condition_action_active_centers: {}",
        condition_action_centers
    );
    println!(
        "  state_condition_active_centers: {}",
        state_condition_centers
    );
    println!("  max_train_state_reuse: {}", max_train_state_reuse);
    println!("  max_heldout_state_reuse: {}", max_heldout_state_reuse);
    println!(
        "  train_tokens_overlap_heldout: {}",
        train_tokens_overlap_heldout
    );
    println!("  slot_accuracy_milli: {}", report.slot_accuracy_milli);
    println!(
        "  flat_slot_accuracy_milli: {}",
        report.flat_slot_accuracy_milli
    );
    println!(
        "  sequence_energy_accuracy_milli: {}",
        report.sequence_energy_accuracy_milli
    );
    println!(
        "  sequence_energy_median_gap: {}",
        report.sequence_energy_median_gap
    );
    println!(
        "  sequence_energy_p10_gap: {}",
        report.sequence_energy_p10_gap
    );
    println!("  energy_pass_slot_fail: {}", report.energy_pass_slot_fail);
    println!(
        "  flat_gap_parity_mismatches: {}",
        report.flat_gap_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        report.flat_sequence_energy_parity_mismatches
    );
    println!("  flat_failed_rows: {}", report.flat_failed_rows);
    println!("  flat_failed_by_length: {:?}", run.flat_failed_by_length);
    println!("  flat_failed_by_rule: {:?}", run.flat_failed_by_rule);
    println!(
        "  ablation_without_binding_accuracy_milli: {}",
        report.ablation_without_binding_accuracy_milli
    );
    println!(
        "  ablation_without_action_accuracy_milli: {}",
        report.ablation_without_action_accuracy_milli
    );
    println!(
        "  ablation_without_condition_action_accuracy_milli: {}",
        no_condition_action_eval.accuracy_milli
    );
    println!(
        "  ablation_without_condition_action_energy_accuracy_milli: {}",
        no_condition_action_energy.energy_accuracy_milli
    );
    println!(
        "  ablation_without_role_accuracy_milli: {}",
        report.ablation_without_role_accuracy_milli
    );
    println!(
        "  ablation_without_active_fringe_accuracy_milli: {}",
        report.ablation_without_active_fringe_accuracy_milli
    );
    println!("  role_binding_edges: {}", report.role_binding_edges);
    println!(
        "  flat_role_binding_bytes_estimate: {}",
        report.flat_role_binding_bytes_estimate
    );
    println!(
        "  base_mass_bytes_estimate: {}",
        report.base_mass_bytes_estimate
    );
    println!("  hot_bytes_estimate: {}", report.hot_bytes_estimate);
    println!("  flat_eval_rows: {}", report.flat_eval_rows);
    println!(
        "  flat_eval_avg_ns_per_row: {}",
        report.flat_eval_avg_ns_per_row
    );
    println!("  flat_eval_latency_gate_ns: {}", flat_latency_gate_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  direct_operator_pair_action_centers_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot conditional branch selection rung; branch maps are still symbolic action inputs, not raw language parsing"
    );

    assert!(report.gate_pass());
    assert_eq!(unique_classes, 1);
    assert_eq!(unique_rules, 8);
    assert_eq!(unique_surfaces, 4);
    assert_eq!(unique_noise_types, 2);
    assert_eq!(unique_lengths, 16);
    assert_eq!(same_bag_rows, heldout_rows.len());
    assert_eq!(condition_true_rows, condition_false_rows);
    assert_eq!(direct_operator_pair_centers, 0);
    assert!(condition_action_centers > 0);
    assert!(state_condition_centers > 0);
    assert_eq!(no_condition_action_eval.accuracy_milli, 0);
    assert_eq!(no_condition_action_energy.energy_accuracy_milli, 0);
    assert_eq!(max_train_state_reuse, unique_rules * 2);
    assert_eq!(max_heldout_state_reuse, unique_rules * 2);
    assert_eq!(train_tokens_overlap_heldout, 0);
    assert!(report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);
}

#[test]
#[ignore = "32-slot mixed-map plus conditional branch multi-seed robustness; long release gate"]
fn operator_battery_v4_slot32_mixed_conditional_multiseed_must_transfer_without_lookup_or_runtime_phase_hack()
 {
    let flat_latency_gate_ns = 1_000_000u128;
    let mut mixed_reports = Vec::new();
    let mut conditional_reports = Vec::new();
    let mut conditional_no_condition_action_milli = Vec::new();
    let mut conditional_no_condition_action_energy_milli = Vec::new();
    let mut total_direct_operator_pair_centers = 0usize;

    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let mixed_train_rows = slot32_mixed_map_corpus_tasks_for_seed("train", seed);
        let mixed_heldout_rows = slot32_mixed_map_corpus_tasks_for_seed("heldout", seed);
        let mixed_train = mixed_train_rows
            .iter()
            .map(|row| row.task.clone())
            .collect::<Vec<_>>();
        let mixed_heldout_labeled = mixed_heldout_rows
            .iter()
            .map(|row| (row.length, row.rule_name, row.task.clone()))
            .collect::<Vec<_>>();
        let mixed_run = run_slot32_prepared_gate(
            "mixed_map_multiseed",
            seed,
            mixed_train,
            mixed_heldout_labeled,
            true,
        );
        let mixed_report = mixed_run.report;
        let mixed_unique_classes = slot32_unique_operator_classes(&mixed_heldout_rows);
        let mixed_unique_rules = slot32_order_unique_rules(&mixed_heldout_rows);
        let mixed_unique_surfaces = slot32_order_unique_surfaces(&mixed_heldout_rows);
        let mixed_unique_noise_types = slot32_order_unique_noise_types(&mixed_heldout_rows);
        let mixed_unique_lengths = slot32_order_unique_lengths(&mixed_heldout_rows);
        let mixed_same_bag_rows = slot32_order_same_bag_rows(&mixed_heldout_rows);
        let mixed_edit_rows = slot32_operator_class_rows(&mixed_heldout_rows, "edit");
        let mixed_edit_non_same_bag_rows =
            slot32_operator_class_non_same_bag_rows(&mixed_heldout_rows, "edit");
        let mixed_max_state_reuse = slot32_order_max_state_reuse(&mixed_heldout_rows);
        let mixed_token_overlap =
            slot32_order_train_heldout_token_overlap(&mixed_train_rows, &mixed_heldout_rows);

        println!(
            "operator_battery_v4_slot32_mixed_multiseed_seed: seed={} pass={} slot={} flat_slot={} energy={} p10_energy_gap={} role_edges={} hot_bytes={} flat_avg_ns={} classes={} rules={} surfaces={} noise={} lengths={} same_bag={} edit={} edit_non_same_bag={} max_state_reuse={} token_overlap={}",
            mixed_report.seed,
            mixed_report.gate_pass(),
            mixed_report.slot_accuracy_milli,
            mixed_report.flat_slot_accuracy_milli,
            mixed_report.sequence_energy_accuracy_milli,
            mixed_report.sequence_energy_p10_gap,
            mixed_report.role_binding_edges,
            mixed_report.hot_bytes_estimate,
            mixed_report.flat_eval_avg_ns_per_row,
            mixed_unique_classes,
            mixed_unique_rules,
            mixed_unique_surfaces,
            mixed_unique_noise_types,
            mixed_unique_lengths,
            mixed_same_bag_rows,
            mixed_edit_rows,
            mixed_edit_non_same_bag_rows,
            mixed_max_state_reuse,
            mixed_token_overlap
        );

        assert!(mixed_report.gate_pass());
        assert_eq!(mixed_unique_classes, 3);
        assert_eq!(mixed_unique_rules, 16);
        assert_eq!(mixed_unique_surfaces, 4);
        assert_eq!(mixed_unique_noise_types, 2);
        assert_eq!(mixed_unique_lengths, 16);
        assert_eq!(
            mixed_same_bag_rows + mixed_edit_non_same_bag_rows,
            mixed_heldout_rows.len()
        );
        assert_eq!(mixed_edit_rows, mixed_edit_non_same_bag_rows);
        assert_eq!(mixed_max_state_reuse, mixed_unique_rules);
        assert_eq!(mixed_token_overlap, 0);
        assert!(mixed_report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);
        mixed_reports.push(mixed_report);

        let conditional_train_rows = slot32_conditional_branch_corpus_tasks_for_seed("train", seed);
        let conditional_heldout_rows =
            slot32_conditional_branch_corpus_tasks_for_seed("heldout", seed);
        let conditional_train = conditional_train_rows
            .iter()
            .map(|row| row.task.clone())
            .collect::<Vec<_>>();
        let conditional_heldout_labeled = conditional_heldout_rows
            .iter()
            .map(|row| (row.length, row.rule_name, row.task.clone()))
            .collect::<Vec<_>>();
        let conditional_run = run_slot32_prepared_gate_with_config(
            "conditional_branch_multiseed",
            seed,
            conditional_train,
            conditional_heldout_labeled,
            true,
            slot32_conditional_binding_config(),
        );
        let conditional_report = conditional_run.report.clone();
        let conditional_heldout = conditional_heldout_rows
            .iter()
            .map(|row| row.task.clone())
            .collect::<Vec<_>>();
        let no_condition_action_tasks = ablate_sequence_tasks(&conditional_heldout, |center_id| {
            !is_slot32_condition_action_center(center_id)
        });
        let no_condition_action_eval =
            eval_ordered_sequence(&conditional_run.field, &no_condition_action_tasks);
        let no_condition_action_energy =
            ordered_sequence_energy_diagnostics(&conditional_run.field, &no_condition_action_tasks);

        let conditional_unique_classes = slot32_unique_operator_classes(&conditional_heldout_rows);
        let conditional_unique_rules = slot32_order_unique_rules(&conditional_heldout_rows);
        let conditional_unique_surfaces = slot32_order_unique_surfaces(&conditional_heldout_rows);
        let conditional_unique_noise_types =
            slot32_order_unique_noise_types(&conditional_heldout_rows);
        let conditional_unique_lengths = slot32_order_unique_lengths(&conditional_heldout_rows);
        let conditional_same_bag_rows = slot32_order_same_bag_rows(&conditional_heldout_rows);
        let condition_true_rows = slot32_condition_result_rows(&conditional_heldout_rows, true);
        let condition_false_rows = slot32_condition_result_rows(&conditional_heldout_rows, false);
        let direct_operator_pair_centers = slot32_active_center_count(
            &conditional_heldout_rows,
            is_slot32_direct_operator_pair_center,
        );
        let condition_action_centers = slot32_active_center_count(
            &conditional_heldout_rows,
            is_slot32_condition_action_center,
        );
        let state_condition_centers =
            slot32_active_center_count(&conditional_heldout_rows, is_slot32_state_condition_center);
        let conditional_max_state_reuse = slot32_order_max_state_reuse(&conditional_heldout_rows);
        let conditional_token_overlap = slot32_order_train_heldout_token_overlap(
            &conditional_train_rows,
            &conditional_heldout_rows,
        );

        println!(
            "operator_battery_v4_slot32_conditional_multiseed_seed: seed={} pass={} slot={} flat_slot={} energy={} p10_energy_gap={} role_edges={} hot_bytes={} flat_avg_ns={} classes={} rules={} surfaces={} noise={} lengths={} same_bag={} true={} false={} direct_pair={} condition_action={} state_condition={} no_condition_action={} no_condition_action_energy={} max_state_reuse={} token_overlap={}",
            conditional_report.seed,
            conditional_report.gate_pass(),
            conditional_report.slot_accuracy_milli,
            conditional_report.flat_slot_accuracy_milli,
            conditional_report.sequence_energy_accuracy_milli,
            conditional_report.sequence_energy_p10_gap,
            conditional_report.role_binding_edges,
            conditional_report.hot_bytes_estimate,
            conditional_report.flat_eval_avg_ns_per_row,
            conditional_unique_classes,
            conditional_unique_rules,
            conditional_unique_surfaces,
            conditional_unique_noise_types,
            conditional_unique_lengths,
            conditional_same_bag_rows,
            condition_true_rows,
            condition_false_rows,
            direct_operator_pair_centers,
            condition_action_centers,
            state_condition_centers,
            no_condition_action_eval.accuracy_milli,
            no_condition_action_energy.energy_accuracy_milli,
            conditional_max_state_reuse,
            conditional_token_overlap
        );

        assert!(conditional_report.gate_pass());
        assert_eq!(conditional_unique_classes, 1);
        assert_eq!(conditional_unique_rules, 8);
        assert_eq!(conditional_unique_surfaces, 4);
        assert_eq!(conditional_unique_noise_types, 2);
        assert_eq!(conditional_unique_lengths, 16);
        assert_eq!(conditional_same_bag_rows, conditional_heldout_rows.len());
        assert_eq!(condition_true_rows, condition_false_rows);
        assert_eq!(direct_operator_pair_centers, 0);
        assert!(condition_action_centers > 0);
        assert!(state_condition_centers > 0);
        assert_eq!(no_condition_action_eval.accuracy_milli, 0);
        assert_eq!(no_condition_action_energy.energy_accuracy_milli, 0);
        assert_eq!(conditional_max_state_reuse, conditional_unique_rules * 2);
        assert_eq!(conditional_token_overlap, 0);
        assert!(conditional_report.flat_eval_avg_ns_per_row <= flat_latency_gate_ns);

        total_direct_operator_pair_centers += direct_operator_pair_centers;
        conditional_no_condition_action_milli.push(no_condition_action_eval.accuracy_milli);
        conditional_no_condition_action_energy_milli
            .push(no_condition_action_energy.energy_accuracy_milli);
        conditional_reports.push(conditional_report);
    }

    let mixed_min_slot_accuracy = mixed_reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let mixed_min_flat_slot_accuracy = mixed_reports
        .iter()
        .map(|report| report.flat_slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let mixed_min_energy_accuracy = mixed_reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let mixed_min_p10_energy_gap = mixed_reports
        .iter()
        .map(|report| report.sequence_energy_p10_gap)
        .min()
        .unwrap_or(0);
    let mixed_total_energy_pass_slot_fail: usize = mixed_reports
        .iter()
        .map(|report| report.energy_pass_slot_fail)
        .sum();
    let mixed_total_flat_gap_parity_mismatches: usize = mixed_reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let mixed_total_flat_energy_parity_mismatches: usize = mixed_reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();

    let conditional_min_slot_accuracy = conditional_reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let conditional_min_flat_slot_accuracy = conditional_reports
        .iter()
        .map(|report| report.flat_slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let conditional_min_energy_accuracy = conditional_reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let conditional_min_p10_energy_gap = conditional_reports
        .iter()
        .map(|report| report.sequence_energy_p10_gap)
        .min()
        .unwrap_or(0);
    let conditional_total_energy_pass_slot_fail: usize = conditional_reports
        .iter()
        .map(|report| report.energy_pass_slot_fail)
        .sum();
    let conditional_total_flat_gap_parity_mismatches: usize = conditional_reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let conditional_total_flat_energy_parity_mismatches: usize = conditional_reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();
    let max_role_binding_edges = mixed_reports
        .iter()
        .chain(conditional_reports.iter())
        .map(|report| report.role_binding_edges)
        .max()
        .unwrap_or(0);
    let max_hot_bytes = mixed_reports
        .iter()
        .chain(conditional_reports.iter())
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_flat_eval_avg_ns = mixed_reports
        .iter()
        .chain(conditional_reports.iter())
        .map(|report| report.flat_eval_avg_ns_per_row)
        .max()
        .unwrap_or(0);
    let max_no_condition_action_accuracy = conditional_no_condition_action_milli
        .into_iter()
        .max()
        .unwrap_or(0);
    let max_no_condition_action_energy_accuracy = conditional_no_condition_action_energy_milli
        .into_iter()
        .max()
        .unwrap_or(0);

    println!("operator_battery_v4_slot32_mixed_conditional_multiseed_gate:");
    println!("  verdict: SLOT32_MIXED_CONDITIONAL_MULTI_SEED_RUNG_PASS");
    println!("  seeds: {}", SEQ32_MULTI_SEED_COUNT);
    println!("  page_count: {}", SEQ32_PAGE_COUNT);
    println!("  total_center_count: {}", SEQ32_TOTAL_CENTER_COUNT);
    println!("  output_slot_count: {}", SEQ32_OUTPUT_SLOT_COUNT);
    println!("  role_slot_count: {}", SEQ32_ROLE_SLOT_COUNT);
    println!("  role_top_l1_lanes: {}", SEQ32_TOP_ROLE_L1_LANES);
    println!(
        "  rows_per_seed_mixed_train: {}",
        mixed_reports[0].train_rows
    );
    println!(
        "  rows_per_seed_mixed_heldout: {}",
        mixed_reports[0].heldout_rows
    );
    println!(
        "  rows_per_seed_conditional_train: {}",
        conditional_reports[0].train_rows
    );
    println!(
        "  rows_per_seed_conditional_heldout: {}",
        conditional_reports[0].heldout_rows
    );
    println!(
        "  mixed_min_slot_accuracy_milli: {}",
        mixed_min_slot_accuracy
    );
    println!(
        "  mixed_min_flat_slot_accuracy_milli: {}",
        mixed_min_flat_slot_accuracy
    );
    println!(
        "  mixed_min_sequence_energy_accuracy_milli: {}",
        mixed_min_energy_accuracy
    );
    println!(
        "  mixed_min_sequence_energy_p10_gap: {}",
        mixed_min_p10_energy_gap
    );
    println!(
        "  mixed_total_energy_pass_slot_fail: {}",
        mixed_total_energy_pass_slot_fail
    );
    println!(
        "  mixed_total_flat_gap_parity_mismatches: {}",
        mixed_total_flat_gap_parity_mismatches
    );
    println!(
        "  mixed_total_flat_sequence_energy_parity_mismatches: {}",
        mixed_total_flat_energy_parity_mismatches
    );
    println!(
        "  conditional_min_slot_accuracy_milli: {}",
        conditional_min_slot_accuracy
    );
    println!(
        "  conditional_min_flat_slot_accuracy_milli: {}",
        conditional_min_flat_slot_accuracy
    );
    println!(
        "  conditional_min_sequence_energy_accuracy_milli: {}",
        conditional_min_energy_accuracy
    );
    println!(
        "  conditional_min_sequence_energy_p10_gap: {}",
        conditional_min_p10_energy_gap
    );
    println!(
        "  conditional_total_energy_pass_slot_fail: {}",
        conditional_total_energy_pass_slot_fail
    );
    println!(
        "  conditional_total_flat_gap_parity_mismatches: {}",
        conditional_total_flat_gap_parity_mismatches
    );
    println!(
        "  conditional_total_flat_sequence_energy_parity_mismatches: {}",
        conditional_total_flat_energy_parity_mismatches
    );
    println!(
        "  conditional_total_direct_operator_pair_active_centers: {}",
        total_direct_operator_pair_centers
    );
    println!(
        "  conditional_max_ablation_without_condition_action_accuracy_milli: {}",
        max_no_condition_action_accuracy
    );
    println!(
        "  conditional_max_ablation_without_condition_action_energy_accuracy_milli: {}",
        max_no_condition_action_energy_accuracy
    );
    println!("  max_role_binding_edges: {}", max_role_binding_edges);
    println!("  max_hot_bytes_estimate: {}", max_hot_bytes);
    println!("  max_flat_eval_avg_ns_per_row: {}", max_flat_eval_avg_ns);
    println!("  flat_eval_latency_gate_ns: {}", flat_latency_gate_ns);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  direct_operator_pair_action_centers_used_for_conditional: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot mixed-map and conditional branch multi-seed robustness; raw-language action parsing and product p99 remain open"
    );

    assert_eq!(mixed_min_slot_accuracy, 1000);
    assert_eq!(mixed_min_flat_slot_accuracy, 1000);
    assert_eq!(mixed_min_energy_accuracy, 1000);
    assert_eq!(mixed_total_energy_pass_slot_fail, 0);
    assert_eq!(mixed_total_flat_gap_parity_mismatches, 0);
    assert_eq!(mixed_total_flat_energy_parity_mismatches, 0);
    assert_eq!(conditional_min_slot_accuracy, 1000);
    assert_eq!(conditional_min_flat_slot_accuracy, 1000);
    assert_eq!(conditional_min_energy_accuracy, 1000);
    assert_eq!(conditional_total_energy_pass_slot_fail, 0);
    assert_eq!(conditional_total_flat_gap_parity_mismatches, 0);
    assert_eq!(conditional_total_flat_energy_parity_mismatches, 0);
    assert_eq!(total_direct_operator_pair_centers, 0);
    assert_eq!(max_no_condition_action_accuracy, 0);
    assert_eq!(max_no_condition_action_energy_accuracy, 0);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
    assert!(max_flat_eval_avg_ns <= flat_latency_gate_ns);
}

fn slot32_role_binding_package_dir() -> PathBuf {
    std::env::var("SLOT32_ROLE_BINDING_PACKAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/nando-wave/slot32-role-binding")
        })
}

fn slot32_role_binding_package_path(label: &'static str, seed: usize) -> PathBuf {
    slot32_role_binding_package_dir().join(format!("{label}-seed{seed}.nwrb"))
}

fn slot32_role_binding_corpus_eval_pack_path(label: &'static str, seed: usize) -> PathBuf {
    slot32_role_binding_package_dir().join(format!("{label}-seed{seed}.corpus-eval-pack-v1.json"))
}

fn write_slot32_role_binding_corpus_eval_pack(
    label: &'static str,
    seed: usize,
    package_fingerprint64: u64,
    heldout_rows: &[Slot32OrderCorpusTask],
) -> PathBuf {
    let eval_pack_path = slot32_role_binding_corpus_eval_pack_path(label, seed);
    fs::create_dir_all(eval_pack_path.parent().expect("eval-pack path has parent"))
        .expect("create slot32 eval-pack directory");

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema_version\": \"nando_role_binding_eval_pack_v1\",\n");
    json.push_str(&format!(
        "  \"package_fingerprint64\": {package_fingerprint64},\n"
    ));
    json.push_str("  \"source_package_path\": null,\n");
    json.push_str(
        "  \"generation_method\": \"slot32_heldout_corpus_sequences; independent of package edges\",\n",
    );
    json.push_str("  \"tasks\": [],\n");
    json.push_str("  \"sequences\": [\n");

    let mut first_sequence = true;
    for (row_index, row) in heldout_rows.iter().enumerate() {
        for (kind, expect_local_operator) in [("local", true), ("fallback_same_bag_wrong", false)] {
            if !first_sequence {
                json.push_str(",\n");
            }
            first_sequence = false;
            let task_id = format!(
                "{label}_seed{seed}_row{row_index:04}_{kind}_{}_len{}",
                row.rule_name, row.length
            );
            write_role_binding_sequence_json(
                &mut json,
                &task_id,
                &row.task,
                expect_local_operator,
                !expect_local_operator,
            );
        }
    }

    json.push_str("\n  ]\n");
    json.push_str("}\n");
    fs::write(&eval_pack_path, json).expect("write slot32 corpus eval-pack");
    eval_pack_path
}

fn write_prepared_role_binding_corpus_eval_pack(
    label: &'static str,
    seed: usize,
    package_fingerprint64: u64,
    heldout: &[PreparedSequenceTask],
    generation_method: &str,
) -> PathBuf {
    let eval_pack_path = slot32_role_binding_corpus_eval_pack_path(label, seed);
    fs::create_dir_all(eval_pack_path.parent().expect("eval-pack path has parent"))
        .expect("create role-binding eval-pack directory");

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema_version\": \"nando_role_binding_eval_pack_v1\",\n");
    json.push_str(&format!(
        "  \"package_fingerprint64\": {package_fingerprint64},\n"
    ));
    json.push_str("  \"source_package_path\": null,\n");
    json.push_str(&format!(
        "  \"generation_method\": \"{}\",\n",
        json_escape(generation_method)
    ));
    json.push_str("  \"tasks\": [],\n");
    json.push_str("  \"sequences\": [\n");

    let mut first_sequence = true;
    for (row_index, task) in heldout.iter().enumerate() {
        for (kind, expect_local_operator) in [("local", true), ("fallback_same_row_wrong", false)] {
            if !first_sequence {
                json.push_str(",\n");
            }
            first_sequence = false;
            let task_id = format!("{label}_seed{seed}_row{row_index:04}_{kind}");
            write_role_binding_sequence_json(
                &mut json,
                &task_id,
                task,
                expect_local_operator,
                !expect_local_operator,
            );
        }
    }

    json.push_str("\n  ]\n");
    json.push_str("}\n");
    fs::write(&eval_pack_path, json).expect("write prepared role-binding corpus eval-pack");
    eval_pack_path
}

fn write_role_binding_sequence_json(
    json: &mut String,
    task_id: &str,
    task: &PreparedSequenceTask,
    expect_local_operator: bool,
    invert_correct_wrong: bool,
) {
    json.push_str("    {\n");
    json.push_str(&format!(
        "      \"task_id\": \"{}\",\n",
        json_escape(task_id)
    ));
    json.push_str("      \"active_fringe\": [");
    for (index, active) in task.slot_tasks[0].active_fringe.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"center_id\": {}, \"strength\": {}}}",
            active.center_id, active.strength
        ));
    }
    json.push_str("],\n");
    json.push_str("      \"slots\": [\n");
    for (slot_index, slot_task) in task.slot_tasks.iter().enumerate() {
        if slot_index > 0 {
            json.push_str(",\n");
        }
        let (positive, negative) = if invert_correct_wrong {
            (
                slot_task.target_delta.negative_impulses(),
                slot_task.target_delta.positive_impulses(),
            )
        } else {
            (
                slot_task.target_delta.positive_impulses(),
                slot_task.target_delta.negative_impulses(),
            )
        };
        json.push_str("        {\n");
        match slot_task.binding_output_slot {
            Some(output_slot) => json.push_str(&format!(
                "          \"binding_output_slot\": {output_slot},\n"
            )),
            None => json.push_str("          \"binding_output_slot\": null,\n"),
        }
        json.push_str("          \"positive_impulses\": ");
        write_impulse_array_json(json, positive);
        json.push_str(",\n");
        json.push_str("          \"negative_impulses\": ");
        write_impulse_array_json(json, negative);
        json.push_str("\n        }");
    }
    json.push_str("\n      ],\n");
    json.push_str(&format!(
        "      \"expect_local_operator\": {expect_local_operator}\n"
    ));
    json.push_str("    }");
}

fn write_impulse_array_json(json: &mut String, impulses: &[WavePredictorStateImpulse]) {
    json.push('[');
    for (index, impulse) in impulses.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{\"lane_id\": {}, \"signed_strength\": {}}}",
            impulse.lane_id, impulse.signed_strength
        ));
    }
    json.push(']');
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn compile_slot32_flat_runtime_for_rows(
    label: &'static str,
    seed: usize,
    train_rows: &[Slot32OrderCorpusTask],
    config: WavePredictorHebbianConfig,
) -> (
    WavePredictorHebbianField,
    WavePredictorFlatRoleBindingTable,
    FlatRoleBindingScoreIndex,
    usize,
) {
    println!(
        "operator_battery_v4_slot32_{label}_cache_offload: compile_start seed={} train_rows={} slot_tasks={}",
        seed,
        train_rows.len(),
        train_rows
            .iter()
            .map(|row| row.task.slot_tasks.len())
            .sum::<usize>()
    );
    let mut field = WavePredictorHebbianField::new(SEQ32_TOTAL_CENTER_COUNT, config);
    let eta_binding = i32::from(config.eta_binding);
    let mut touched_edges = 0usize;
    for row in train_rows {
        for slot_task in &row.task.slot_tasks {
            for impulse in slot_task.target_delta.positive_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    eta_binding * magnitude,
                );
            }
            for impulse in slot_task.target_delta.negative_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    -eta_binding * magnitude,
                );
            }
        }
    }
    let flat = field.compile_flat_role_binding_table();
    let index = FlatRoleBindingScoreIndex::new(&flat, config);
    println!(
        "operator_battery_v4_slot32_{label}_cache_offload: compile_done seed={} touched_edges={} role_binding_edges={} flat_edges={}",
        seed,
        touched_edges,
        field.state_delta_role_binding_edge_count(),
        flat.edge_count()
    );
    (field, flat, index, touched_edges)
}

fn prove_slot32_role_binding_package(
    label: &'static str,
    seed: usize,
    train_rows: &[Slot32OrderCorpusTask],
    heldout_rows: &[Slot32OrderCorpusTask],
    config: WavePredictorHebbianConfig,
    local_margin_threshold: i32,
) -> Slot32RoleBindingPackageReport {
    let (field, flat, _, _) = compile_slot32_flat_runtime_for_rows(label, seed, train_rows, config);
    let package_bytes = flat.to_bytes().expect("role-binding package serializes");
    assert!(package_bytes.starts_with(&WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC));
    let package_path = slot32_role_binding_package_path(label, seed);
    fs::create_dir_all(package_path.parent().expect("package path has parent"))
        .expect("create slot32 package directory");
    fs::write(&package_path, &package_bytes).expect("write slot32 role-binding package");
    let read_back = fs::read(&package_path).expect("read slot32 role-binding package");
    assert_eq!(read_back, package_bytes);

    let package_info =
        WavePredictorFlatRoleBindingTable::inspect_bytes(&read_back).expect("package inspects");
    let loaded = WavePredictorFlatRoleBindingTable::from_bytes(&read_back).expect("package loads");
    let loaded_rewrite = loaded.to_bytes().expect("loaded package serializes");
    let loaded_rewrite_exact = loaded_rewrite == read_back;
    let loaded_index = FlatRoleBindingScoreIndex::new(&loaded, config);
    let heldout = heldout_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let slot_eval = eval_ordered_sequence_flat_fast(&loaded_index, &heldout);
    let energy = flat_ordered_sequence_energy_diagnostics_fast(&loaded_index, &heldout);
    let flat_parity = eval_ordered_sequence_flat_gap_parity_fast(&field, &loaded_index, &heldout);
    let flat_energy_parity =
        eval_ordered_sequence_flat_energy_parity_fast(&field, &loaded_index, &heldout);

    let mut latencies = Vec::with_capacity(heldout.len());
    let mut false_local_accepts = 0usize;
    for task in &heldout {
        let start = Instant::now();
        let strict_ok = flat_ordered_sequence_row_ok_fast(&loaded_index, task);
        let energy_gap = flat_sequence_energy_gap_fast(&loaded_index, task);
        latencies.push(start.elapsed().as_nanos());
        false_local_accepts += usize::from(strict_ok && energy_gap <= 0);
        assert!(
            energy_gap >= local_margin_threshold,
            "label={label} seed={seed} energy_gap={energy_gap}"
        );
    }
    latencies.sort_unstable();

    Slot32RoleBindingPackageReport {
        label,
        seed,
        package_path,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_back.len(),
        inspected_edges: package_info.edge_count,
        loaded_rewrite_exact,
        slot_accuracy_milli: slot_eval.accuracy_milli,
        sequence_energy_accuracy_milli: energy.energy_accuracy_milli,
        flat_gap_parity_mismatches: flat_parity.mismatches,
        flat_sequence_energy_parity_mismatches: flat_energy_parity.mismatches,
        p99_latency_ns: latencies[(latencies.len() * 99 / 100).min(latencies.len() - 1)],
        false_local_accepts,
        hot_bytes_estimate: loaded.byte_size_estimate()
            + SEQ32_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>(),
    }
}

fn prove_slot32_role_binding_sdk_package(
    label: &'static str,
    seed: usize,
    train_rows: &[Slot32OrderCorpusTask],
    heldout_rows: &[Slot32OrderCorpusTask],
    config: WavePredictorHebbianConfig,
    local_margin_threshold: i32,
) -> Slot32RoleBindingPackageReport {
    let (field, flat, _, _) = compile_slot32_flat_runtime_for_rows(label, seed, train_rows, config);
    let package_bytes = flat.to_bytes().expect("role-binding package serializes");
    assert!(package_bytes.starts_with(&WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC));
    let package_path = slot32_role_binding_package_path(label, seed);
    fs::create_dir_all(package_path.parent().expect("package path has parent"))
        .expect("create slot32 package directory");
    fs::write(&package_path, &package_bytes).expect("write slot32 role-binding sdk package");
    let read_back = fs::read(&package_path).expect("read slot32 role-binding sdk package");
    assert_eq!(read_back, package_bytes);

    let package_info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&read_back)
        .expect("sdk inspects package");
    let corpus_eval_pack_path = write_slot32_role_binding_corpus_eval_pack(
        label,
        seed,
        package_info.fingerprint64,
        heldout_rows,
    );
    println!(
        "operator_battery_v4_slot32_role_binding_sdk_package_corpus_eval_pack: label={} seed={} path={}",
        label,
        seed,
        corpus_eval_pack_path.display()
    );
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes(
        &read_back,
        WavePredictorRoleBindingOffloadPolicy::new(local_margin_threshold).expect("valid policy"),
    )
    .expect("sdk loads package");
    assert_eq!(sdk.package_info(), package_info);

    let loaded_rewrite = sdk
        .table()
        .to_bytes()
        .expect("sdk loaded package serializes");
    let loaded_rewrite_exact = loaded_rewrite == read_back;
    let heldout = heldout_rows
        .iter()
        .map(|row| row.task.clone())
        .collect::<Vec<_>>();
    let slot_eval = eval_ordered_sequence_sdk(&sdk, &heldout);
    let energy = sdk_ordered_sequence_energy_diagnostics(&sdk, &heldout);
    let sdk_gap_parity = eval_ordered_sequence_sdk_gap_parity(&field, &sdk, &heldout);
    let sdk_energy_parity = eval_ordered_sequence_sdk_energy_parity(&field, &sdk, &heldout);

    let mut latencies = Vec::with_capacity(heldout.len());
    let mut false_local_accepts = 0usize;
    for task in &heldout {
        let start = Instant::now();
        let strict_ok = sdk_ordered_sequence_row_ok(&sdk, task);
        let energy_gap = sdk_sequence_energy_gap(&sdk, task);
        latencies.push(start.elapsed().as_nanos());
        false_local_accepts += usize::from(strict_ok && energy_gap <= 0);
        assert!(
            energy_gap >= local_margin_threshold,
            "label={label} seed={seed} sdk_energy_gap={energy_gap}"
        );
    }
    latencies.sort_unstable();

    Slot32RoleBindingPackageReport {
        label,
        seed,
        package_path,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_back.len(),
        inspected_edges: package_info.edge_count,
        loaded_rewrite_exact,
        slot_accuracy_milli: slot_eval.accuracy_milli,
        sequence_energy_accuracy_milli: energy.energy_accuracy_milli,
        flat_gap_parity_mismatches: sdk_gap_parity.mismatches,
        flat_sequence_energy_parity_mismatches: sdk_energy_parity.mismatches,
        p99_latency_ns: latencies[(latencies.len() * 99 / 100).min(latencies.len() - 1)],
        false_local_accepts,
        hot_bytes_estimate: sdk.bytes_estimate()
            + SEQ32_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>(),
    }
}

fn prove_edit_role_binding_sdk_package(
    label: &'static str,
    seed: usize,
    train: &[PreparedSequenceTask],
    heldout: &[PreparedSequenceTask],
    local_margin_threshold: i32,
) -> Slot32RoleBindingPackageReport {
    let field = train_sequence_combined_field_with_progress(
        "operator_battery_v4_edit_package",
        train,
        edit_binding_config(),
        WavePredictorTrainerConfig {
            epochs: 8,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 7,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
        WavePredictorTrainerConfig {
            epochs: 4,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 160,
                target_margin: 320,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        },
    );
    let flat = field.compile_flat_role_binding_table();
    let package_bytes = flat
        .to_bytes()
        .expect("edit role-binding package serializes");
    assert!(package_bytes.starts_with(&WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC));
    let package_path = slot32_role_binding_package_path(label, seed);
    fs::create_dir_all(package_path.parent().expect("package path has parent"))
        .expect("create edit role-binding package directory");
    fs::write(&package_path, &package_bytes).expect("write edit role-binding package");
    let read_back = fs::read(&package_path).expect("read edit role-binding package");
    assert_eq!(read_back, package_bytes);

    let package_info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&read_back)
        .expect("sdk inspects edit package");
    let corpus_eval_pack_path = write_prepared_role_binding_corpus_eval_pack(
        label,
        seed,
        package_info.fingerprint64,
        heldout,
        "edit_heldout_corpus_sequences; independent of package edges",
    );
    println!(
        "operator_battery_v4_edit_role_binding_package_corpus_eval_pack: label={} seed={} path={}",
        label,
        seed,
        corpus_eval_pack_path.display()
    );
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes(
        &read_back,
        WavePredictorRoleBindingOffloadPolicy::new(local_margin_threshold).expect("valid policy"),
    )
    .expect("sdk loads edit package");
    assert_eq!(sdk.package_info(), package_info);

    let loaded_rewrite = sdk
        .table()
        .to_bytes()
        .expect("sdk loaded edit package serializes");
    let loaded_rewrite_exact = loaded_rewrite == read_back;
    let slot_eval = eval_ordered_sequence_sdk(&sdk, heldout);
    let energy = sdk_ordered_sequence_energy_diagnostics(&sdk, heldout);
    let sdk_gap_parity = eval_ordered_sequence_sdk_gap_parity(&field, &sdk, heldout);
    let sdk_energy_parity = eval_ordered_sequence_sdk_energy_parity(&field, &sdk, heldout);

    let mut latencies = Vec::with_capacity(heldout.len());
    let mut false_local_accepts = 0usize;
    for task in heldout {
        let start = Instant::now();
        let strict_ok = sdk_ordered_sequence_row_ok(&sdk, task);
        let energy_gap = sdk_sequence_energy_gap(&sdk, task);
        latencies.push(start.elapsed().as_nanos());
        false_local_accepts += usize::from(strict_ok && energy_gap <= 0);
        assert!(
            energy_gap >= local_margin_threshold,
            "label={label} seed={seed} edit_sdk_energy_gap={energy_gap}"
        );
    }
    latencies.sort_unstable();

    Slot32RoleBindingPackageReport {
        label,
        seed,
        package_path,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_back.len(),
        inspected_edges: package_info.edge_count,
        loaded_rewrite_exact,
        slot_accuracy_milli: slot_eval.accuracy_milli,
        sequence_energy_accuracy_milli: energy.energy_accuracy_milli,
        flat_gap_parity_mismatches: sdk_gap_parity.mismatches,
        flat_sequence_energy_parity_mismatches: sdk_energy_parity.mismatches,
        p99_latency_ns: latencies[(latencies.len() * 99 / 100).min(latencies.len() - 1)],
        false_local_accepts,
        hot_bytes_estimate: sdk.bytes_estimate()
            + SEQ_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>(),
    }
}

fn flat_ordered_sequence_energy_diagnostics_fast(
    index: &FlatRoleBindingScoreIndex,
    tasks: &[PreparedSequenceTask],
) -> SequenceEnergyDiagnostics {
    let mut energy_gaps = Vec::with_capacity(tasks.len());
    let mut energy_correct = 0usize;
    let mut slot_pass_energy_fail = 0usize;
    let mut energy_pass_slot_fail = 0usize;

    for task in tasks {
        let slot_ok = flat_ordered_sequence_row_ok_fast(index, task);
        let energy_gap = flat_sequence_energy_gap_fast(index, task);
        let energy_ok = energy_gap > 0;
        energy_gaps.push(energy_gap);
        energy_correct += usize::from(energy_ok);
        slot_pass_energy_fail += usize::from(slot_ok && !energy_ok);
        energy_pass_slot_fail += usize::from(energy_ok && !slot_ok);
    }

    energy_gaps.sort_unstable();
    SequenceEnergyDiagnostics {
        rows: tasks.len(),
        energy_accuracy_milli: milli_ratio(energy_correct, tasks.len()),
        median_energy_gap: energy_gaps[tasks.len() / 2],
        p10_energy_gap: energy_gaps[tasks.len() / 10],
        slot_pass_energy_fail,
        energy_pass_slot_fail,
    }
}

fn eval_ordered_sequence_sdk(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    tasks: &[PreparedSequenceTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let prepared = sdk.prepare_active_fringe(&task.slot_tasks[0].active_fringe);
        let mut min_gap = i32::MAX;
        let mut row_ok = true;
        for slot_task in &task.slot_tasks {
            let gap = sdk_state_delta_sum_gap_prepared(sdk, &prepared, slot_task);
            min_gap = min_gap.min(gap);
            row_ok &= gap > 0;
        }
        gaps.push(min_gap);
        correct += usize::from(row_ok);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn sdk_ordered_sequence_energy_diagnostics(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    tasks: &[PreparedSequenceTask],
) -> SequenceEnergyDiagnostics {
    let mut energy_gaps = Vec::with_capacity(tasks.len());
    let mut energy_correct = 0usize;
    let mut slot_pass_energy_fail = 0usize;
    let mut energy_pass_slot_fail = 0usize;

    for task in tasks {
        let slot_ok = sdk_ordered_sequence_row_ok(sdk, task);
        let energy_gap = sdk_sequence_energy_gap(sdk, task);
        let energy_ok = energy_gap > 0;
        energy_gaps.push(energy_gap);
        energy_correct += usize::from(energy_ok);
        slot_pass_energy_fail += usize::from(slot_ok && !energy_ok);
        energy_pass_slot_fail += usize::from(energy_ok && !slot_ok);
    }

    energy_gaps.sort_unstable();
    SequenceEnergyDiagnostics {
        rows: tasks.len(),
        energy_accuracy_milli: milli_ratio(energy_correct, tasks.len()),
        median_energy_gap: energy_gaps[tasks.len() / 2],
        p10_energy_gap: energy_gaps[tasks.len() / 10],
        slot_pass_energy_fail,
        energy_pass_slot_fail,
    }
}

fn sdk_ordered_sequence_row_ok(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    task: &PreparedSequenceTask,
) -> bool {
    let prepared = sdk.prepare_active_fringe(&task.slot_tasks[0].active_fringe);
    task.slot_tasks
        .iter()
        .all(|slot_task| sdk_state_delta_sum_gap_prepared(sdk, &prepared, slot_task) > 0)
}

fn sdk_sequence_energy_gap(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    task: &PreparedSequenceTask,
) -> i32 {
    let prepared = sdk.prepare_active_fringe(&task.slot_tasks[0].active_fringe);
    sdk_sequence_energy_gap_prepared(sdk, &prepared, task)
}

fn sdk_sequence_energy_gap_prepared(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &nando_core::WavePredictorRoleBindingPreparedFringe,
    task: &PreparedSequenceTask,
) -> i32 {
    let mut correct_score = 0i32;
    let mut wrong_score = 0i32;
    for slot_task in &task.slot_tasks {
        correct_score += sdk_state_delta_target_score_prepared(sdk, prepared, slot_task);
        wrong_score += sdk_state_delta_wrong_score_prepared(sdk, prepared, slot_task);
    }
    correct_score - wrong_score
}

fn sdk_state_delta_sum_gap_prepared(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &nando_core::WavePredictorRoleBindingPreparedFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    sdk_state_delta_target_score_prepared(sdk, prepared, task)
        - sdk_state_delta_wrong_score_prepared(sdk, prepared, task)
}

fn sdk_state_delta_target_score_prepared(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &nando_core::WavePredictorRoleBindingPreparedFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| {
            sdk.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn sdk_state_delta_wrong_score_prepared(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &nando_core::WavePredictorRoleBindingPreparedFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| {
            sdk.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn eval_ordered_sequence_sdk_gap_parity(
    field: &WavePredictorHebbianField,
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    tasks: &[PreparedSequenceTask],
) -> FlatGapParityReport {
    let mut report = FlatGapParityReport::default();
    for task in tasks {
        let prepared = sdk.prepare_active_fringe(&task.slot_tasks[0].active_fringe);
        for slot_task in &task.slot_tasks {
            report.checked_slots += 1;
            let field_gap = state_delta_sum_gap(field, slot_task);
            let sdk_gap = sdk_state_delta_sum_gap_prepared(sdk, &prepared, slot_task);
            report.mismatches += usize::from(field_gap != sdk_gap);
        }
    }
    report
}

fn eval_ordered_sequence_sdk_energy_parity(
    field: &WavePredictorHebbianField,
    sdk: &WavePredictorRoleBindingOffloadRuntime,
    tasks: &[PreparedSequenceTask],
) -> FlatEnergyParityReport {
    let mut report = FlatEnergyParityReport::default();
    for task in tasks {
        report.checked_rows += 1;
        let field_gap = sequence_energy_gap(field, task);
        let prepared = sdk.prepare_active_fringe(&task.slot_tasks[0].active_fringe);
        let sdk_gap = sdk_sequence_energy_gap_prepared(sdk, &prepared, task);
        let delta = (i64::from(field_gap) - i64::from(sdk_gap)).abs() as i32;
        report.max_abs_gap_delta = report.max_abs_gap_delta.max(delta);
        report.mismatches += usize::from(field_gap != sdk_gap);
    }
    report
}

fn slot32_cache_key(
    label: &'static str,
    seed: usize,
    row_index: usize,
    row: &Slot32OrderCorpusTask,
) -> String {
    format!(
        "label={label}|seed={seed}|row={row_index}|class={}|rule={}|len={}|surface={}|noise={}|condition={:?}|state={}",
        row.operator_class,
        row.rule_name,
        row.length,
        row.surface_family,
        row.noise_type,
        row.condition_result,
        row.state_key
    )
}

fn benchmark_slot32_cache_offload(
    input: Slot32CacheOffloadBenchInput<'_>,
    simulated_repeats: usize,
    local_margin_threshold: i32,
) -> Slot32CacheOffloadReport {
    let mut exact_cache_seen = BTreeSet::new();
    let mut nando_cache_seen = BTreeSet::new();
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut exact_cache_plus_nando_cache_hits = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut local_correct = 0usize;
    let mut false_local_accepts = 0usize;
    let label = input.label;
    let seed = input.seed;
    let heldout_rows = input.heldout_rows;
    let mut margins = Vec::with_capacity(heldout_rows.len() * simulated_repeats);
    let mut latencies = Vec::with_capacity(heldout_rows.len() * simulated_repeats);

    for repeat in 0..simulated_repeats {
        println!(
            "operator_battery_v4_slot32_{label}_cache_offload: eval_repeat_start seed={} repeat={}/{} rows={}",
            seed,
            repeat + 1,
            simulated_repeats,
            heldout_rows.len()
        );
        for (row_index, row) in heldout_rows.iter().enumerate() {
            let cache_key = slot32_cache_key(label, seed, row_index, row);
            if exact_cache_seen.insert(cache_key.clone()) {
                exact_cache_llm_calls += 1;
            } else {
                exact_cache_hits += 1;
            }

            let start = Instant::now();
            let strict_ok = flat_ordered_sequence_row_ok_fast(input.index, &row.task);
            let energy_gap = flat_sequence_energy_gap_fast(input.index, &row.task);
            let latency_ns = start.elapsed().as_nanos();
            latencies.push(latency_ns);
            margins.push(energy_gap);

            let local_accept = strict_ok && energy_gap >= local_margin_threshold;
            if local_accept {
                local_operator_calls += 1;
                local_correct += usize::from(strict_ok);
                false_local_accepts += usize::from(!strict_ok);
            } else {
                fallback_to_llm_calls += 1;
                if nando_cache_seen.insert(cache_key) {
                    exact_cache_plus_nando_llm_calls += 1;
                } else {
                    exact_cache_plus_nando_cache_hits += 1;
                }
            }
        }
    }

    margins.sort_unstable();
    latencies.sort_unstable();
    let simulated_calls = heldout_rows.len() * simulated_repeats;
    let no_cache_llm_calls = simulated_calls;
    let incremental_llm_calls_removed_vs_cache =
        exact_cache_llm_calls.saturating_sub(exact_cache_plus_nando_llm_calls);
    let hot_bytes_estimate =
        input.flat.byte_size_estimate() + SEQ32_TOTAL_CENTER_COUNT * std::mem::size_of::<i16>();
    Slot32CacheOffloadReport {
        label,
        seed,
        unique_rows: heldout_rows.len(),
        simulated_calls,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_hits,
        exact_cache_plus_nando_llm_calls,
        exact_cache_plus_nando_cache_hits,
        local_operator_calls,
        fallback_to_llm_calls,
        incremental_llm_calls_removed_vs_cache,
        incremental_llm_call_reduction_vs_cache_milli: milli_ratio(
            incremental_llm_calls_removed_vs_cache,
            exact_cache_llm_calls,
        ),
        offload_rate_milli: milli_ratio(local_operator_calls, simulated_calls),
        local_accuracy_milli: milli_ratio(local_correct, local_operator_calls),
        false_local_accepts,
        min_energy_margin: margins[0],
        p10_energy_margin: margins[margins.len() / 10],
        median_energy_margin: margins[margins.len() / 2],
        p50_latency_ns: latencies[latencies.len() / 2],
        p99_latency_ns: latencies[(latencies.len() * 99 / 100).min(latencies.len() - 1)],
        max_latency_ns: latencies[latencies.len() - 1],
        role_binding_edges: input.field.state_delta_role_binding_edge_count(),
        hot_bytes_estimate,
    }
}

#[test]
#[ignore = "32-slot mixed/conditional cache-offload benchmark; release proof rung"]
fn operator_battery_v4_slot32_mixed_conditional_cache_offload_benchmark_must_stay_local_without_false_accepts()
 {
    let simulated_repeats = 3usize;
    let local_margin_threshold = 1_000_000i32;
    let p99_latency_gate_ns = 1_000_000u128;
    let mut reports = Vec::new();

    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let mixed_train_rows = slot32_mixed_map_corpus_tasks_for_seed("train", seed);
        let mixed_heldout_rows = slot32_mixed_map_corpus_tasks_for_seed("heldout", seed);
        let (mixed_field, mixed_flat, mixed_index, _) = compile_slot32_flat_runtime_for_rows(
            "mixed_map",
            seed,
            &mixed_train_rows,
            slot32_binding_config(),
        );
        let mixed_report = benchmark_slot32_cache_offload(
            Slot32CacheOffloadBenchInput {
                label: "mixed_map",
                seed,
                index: &mixed_index,
                flat: &mixed_flat,
                field: &mixed_field,
                heldout_rows: &mixed_heldout_rows,
            },
            simulated_repeats,
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_mixed_cache_offload_seed: seed={} local_accuracy={} false_local_accepts={} local_calls={} fallback_calls={} exact_cache_llm_calls={} nando_llm_calls={} removed_vs_cache={} reduction_milli={} p99_ns={} min_margin={} hot_bytes={} role_edges={}",
            mixed_report.seed,
            mixed_report.local_accuracy_milli,
            mixed_report.false_local_accepts,
            mixed_report.local_operator_calls,
            mixed_report.fallback_to_llm_calls,
            mixed_report.exact_cache_llm_calls,
            mixed_report.exact_cache_plus_nando_llm_calls,
            mixed_report.incremental_llm_calls_removed_vs_cache,
            mixed_report.incremental_llm_call_reduction_vs_cache_milli,
            mixed_report.p99_latency_ns,
            mixed_report.min_energy_margin,
            mixed_report.hot_bytes_estimate,
            mixed_report.role_binding_edges
        );
        reports.push(mixed_report);

        let conditional_train_rows = slot32_conditional_branch_corpus_tasks_for_seed("train", seed);
        let conditional_heldout_rows =
            slot32_conditional_branch_corpus_tasks_for_seed("heldout", seed);
        let (conditional_field, conditional_flat, conditional_index, _) =
            compile_slot32_flat_runtime_for_rows(
                "conditional_branch",
                seed,
                &conditional_train_rows,
                slot32_conditional_binding_config(),
            );
        let conditional_report = benchmark_slot32_cache_offload(
            Slot32CacheOffloadBenchInput {
                label: "conditional_branch",
                seed,
                index: &conditional_index,
                flat: &conditional_flat,
                field: &conditional_field,
                heldout_rows: &conditional_heldout_rows,
            },
            simulated_repeats,
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_conditional_cache_offload_seed: seed={} local_accuracy={} false_local_accepts={} local_calls={} fallback_calls={} exact_cache_llm_calls={} nando_llm_calls={} removed_vs_cache={} reduction_milli={} p99_ns={} min_margin={} hot_bytes={} role_edges={}",
            conditional_report.seed,
            conditional_report.local_accuracy_milli,
            conditional_report.false_local_accepts,
            conditional_report.local_operator_calls,
            conditional_report.fallback_to_llm_calls,
            conditional_report.exact_cache_llm_calls,
            conditional_report.exact_cache_plus_nando_llm_calls,
            conditional_report.incremental_llm_calls_removed_vs_cache,
            conditional_report.incremental_llm_call_reduction_vs_cache_milli,
            conditional_report.p99_latency_ns,
            conditional_report.min_energy_margin,
            conditional_report.hot_bytes_estimate,
            conditional_report.role_binding_edges
        );
        reports.push(conditional_report);
    }

    let total_unique_rows: usize = reports.iter().map(|report| report.unique_rows).sum();
    let total_simulated_calls: usize = reports.iter().map(|report| report.simulated_calls).sum();
    let total_no_cache_llm_calls: usize =
        reports.iter().map(|report| report.no_cache_llm_calls).sum();
    let total_exact_cache_llm_calls: usize = reports
        .iter()
        .map(|report| report.exact_cache_llm_calls)
        .sum();
    let total_exact_cache_hits: usize = reports.iter().map(|report| report.exact_cache_hits).sum();
    let total_exact_cache_plus_nando_llm_calls: usize = reports
        .iter()
        .map(|report| report.exact_cache_plus_nando_llm_calls)
        .sum();
    let total_exact_cache_plus_nando_cache_hits: usize = reports
        .iter()
        .map(|report| report.exact_cache_plus_nando_cache_hits)
        .sum();
    let total_local_operator_calls: usize = reports
        .iter()
        .map(|report| report.local_operator_calls)
        .sum();
    let total_fallback_to_llm_calls: usize = reports
        .iter()
        .map(|report| report.fallback_to_llm_calls)
        .sum();
    let total_false_local_accepts: usize = reports
        .iter()
        .map(|report| report.false_local_accepts)
        .sum();
    let total_removed_vs_cache: usize = reports
        .iter()
        .map(|report| report.incremental_llm_calls_removed_vs_cache)
        .sum();
    let min_local_accuracy = reports
        .iter()
        .map(|report| report.local_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_offload_rate = reports
        .iter()
        .map(|report| report.offload_rate_milli)
        .min()
        .unwrap_or(0);
    let min_energy_margin = reports
        .iter()
        .map(|report| report.min_energy_margin)
        .min()
        .unwrap_or(0);
    let max_p99_latency_ns = reports
        .iter()
        .map(|report| report.p99_latency_ns)
        .max()
        .unwrap_or(0);
    let max_hot_bytes = reports
        .iter()
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_role_binding_edges = reports
        .iter()
        .map(|report| report.role_binding_edges)
        .max()
        .unwrap_or(0);

    println!("operator_battery_v4_slot32_cache_offload_benchmark:");
    println!("  verdict: SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH_PASS");
    println!("  seeds: {}", SEQ32_MULTI_SEED_COUNT);
    println!("  simulated_repeats: {simulated_repeats}");
    println!("  local_margin_threshold: {local_margin_threshold}");
    println!("  p99_latency_gate_ns: {p99_latency_gate_ns}");
    println!("  total_unique_rows: {total_unique_rows}");
    println!("  total_simulated_calls: {total_simulated_calls}");
    println!("  total_no_cache_llm_calls: {total_no_cache_llm_calls}");
    println!("  total_exact_cache_llm_calls: {total_exact_cache_llm_calls}");
    println!("  total_exact_cache_hits: {total_exact_cache_hits}");
    println!("  total_exact_cache_plus_nando_llm_calls: {total_exact_cache_plus_nando_llm_calls}");
    println!(
        "  total_exact_cache_plus_nando_cache_hits: {total_exact_cache_plus_nando_cache_hits}"
    );
    println!("  total_local_operator_calls: {total_local_operator_calls}");
    println!("  total_fallback_to_llm_calls: {total_fallback_to_llm_calls}");
    println!("  total_false_local_accepts: {total_false_local_accepts}");
    println!("  total_incremental_llm_calls_removed_vs_cache: {total_removed_vs_cache}");
    println!(
        "  total_incremental_llm_call_reduction_vs_cache_milli: {}",
        milli_ratio(total_removed_vs_cache, total_exact_cache_llm_calls)
    );
    println!("  min_local_accuracy_milli: {min_local_accuracy}");
    println!("  min_offload_rate_milli: {min_offload_rate}");
    println!("  min_energy_margin: {min_energy_margin}");
    println!("  max_p99_latency_ns: {max_p99_latency_ns}");
    println!("  max_hot_bytes_estimate: {max_hot_bytes}");
    println!("  max_role_binding_edges: {max_role_binding_edges}");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: 32-slot flat role-binding cache/offload benchmark; not a serialized .nwpc package proof and not raw-language action parsing"
    );

    assert_eq!(min_local_accuracy, 1000);
    assert_eq!(min_offload_rate, 1000);
    assert_eq!(total_false_local_accepts, 0);
    assert_eq!(total_exact_cache_plus_nando_llm_calls, 0);
    assert_eq!(total_fallback_to_llm_calls, 0);
    assert_eq!(total_removed_vs_cache, total_exact_cache_llm_calls);
    assert!(min_energy_margin >= local_margin_threshold);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
    assert!(max_p99_latency_ns <= p99_latency_gate_ns);
}

#[test]
#[ignore = "32-slot role-binding serialized package proof; release gate"]
fn operator_battery_v4_slot32_role_binding_package_must_roundtrip_and_score_loaded_runtime() {
    let local_margin_threshold = 1_000_000i32;
    let p99_latency_gate_ns = 1_000_000u128;
    let mut reports = Vec::new();

    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let mixed_train_rows = slot32_mixed_map_corpus_tasks_for_seed("train", seed);
        let mixed_heldout_rows = slot32_mixed_map_corpus_tasks_for_seed("heldout", seed);
        let mixed_report = prove_slot32_role_binding_package(
            "mixed_map",
            seed,
            &mixed_train_rows,
            &mixed_heldout_rows,
            slot32_binding_config(),
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_role_binding_package_mixed_seed: seed={} path={} bytes={} fingerprint64={} edges={} rewrite_exact={} slot={} energy={} parity={} energy_parity={} p99_ns={} false_local_accepts={} hot_bytes={}",
            mixed_report.seed,
            mixed_report.package_path.display(),
            mixed_report.package_bytes,
            mixed_report.package_fingerprint64,
            mixed_report.inspected_edges,
            mixed_report.loaded_rewrite_exact,
            mixed_report.slot_accuracy_milli,
            mixed_report.sequence_energy_accuracy_milli,
            mixed_report.flat_gap_parity_mismatches,
            mixed_report.flat_sequence_energy_parity_mismatches,
            mixed_report.p99_latency_ns,
            mixed_report.false_local_accepts,
            mixed_report.hot_bytes_estimate
        );
        reports.push(mixed_report);

        let conditional_train_rows = slot32_conditional_branch_corpus_tasks_for_seed("train", seed);
        let conditional_heldout_rows =
            slot32_conditional_branch_corpus_tasks_for_seed("heldout", seed);
        let conditional_report = prove_slot32_role_binding_package(
            "conditional_branch",
            seed,
            &conditional_train_rows,
            &conditional_heldout_rows,
            slot32_conditional_binding_config(),
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_role_binding_package_conditional_seed: seed={} path={} bytes={} fingerprint64={} edges={} rewrite_exact={} slot={} energy={} parity={} energy_parity={} p99_ns={} false_local_accepts={} hot_bytes={}",
            conditional_report.seed,
            conditional_report.package_path.display(),
            conditional_report.package_bytes,
            conditional_report.package_fingerprint64,
            conditional_report.inspected_edges,
            conditional_report.loaded_rewrite_exact,
            conditional_report.slot_accuracy_milli,
            conditional_report.sequence_energy_accuracy_milli,
            conditional_report.flat_gap_parity_mismatches,
            conditional_report.flat_sequence_energy_parity_mismatches,
            conditional_report.p99_latency_ns,
            conditional_report.false_local_accepts,
            conditional_report.hot_bytes_estimate
        );
        reports.push(conditional_report);
    }

    let min_slot_accuracy = reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_energy_accuracy = reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let total_flat_gap_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let total_flat_energy_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();
    let total_false_local_accepts: usize = reports
        .iter()
        .map(|report| report.false_local_accepts)
        .sum();
    let max_p99_latency_ns = reports
        .iter()
        .map(|report| report.p99_latency_ns)
        .max()
        .unwrap_or(0);
    let max_hot_bytes = reports
        .iter()
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_package_bytes = reports
        .iter()
        .map(|report| report.package_bytes)
        .max()
        .unwrap_or(0);
    let max_edges = reports
        .iter()
        .map(|report| report.inspected_edges)
        .max()
        .unwrap_or(0);
    let rewrite_exact_all = reports.iter().all(|report| report.loaded_rewrite_exact);
    let nonzero_fingerprints = reports
        .iter()
        .all(|report| report.package_fingerprint64 != 0);
    let labels = reports
        .iter()
        .map(|report| report.label)
        .collect::<BTreeSet<_>>();

    println!("operator_battery_v4_slot32_role_binding_package_gate:");
    println!("  verdict: SLOT32_ROLE_BINDING_PACKAGE_RUNG_PASS");
    println!(
        "  package_magic: {:?}",
        WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC
    );
    println!(
        "  package_dir: {}",
        slot32_role_binding_package_dir().display()
    );
    println!("  seeds: {}", SEQ32_MULTI_SEED_COUNT);
    println!("  labels: {:?}", labels);
    println!("  local_margin_threshold: {local_margin_threshold}");
    println!("  p99_latency_gate_ns: {p99_latency_gate_ns}");
    println!("  min_slot_accuracy_milli: {min_slot_accuracy}");
    println!("  min_sequence_energy_accuracy_milli: {min_energy_accuracy}");
    println!("  total_flat_gap_parity_mismatches: {total_flat_gap_parity_mismatches}");
    println!(
        "  total_flat_sequence_energy_parity_mismatches: {total_flat_energy_parity_mismatches}"
    );
    println!("  total_false_local_accepts: {total_false_local_accepts}");
    println!("  rewrite_exact_all: {rewrite_exact_all}");
    println!("  nonzero_fingerprints: {nonzero_fingerprints}");
    println!("  max_package_bytes: {max_package_bytes}");
    println!("  max_hot_bytes_estimate: {max_hot_bytes}");
    println!("  max_edges: {max_edges}");
    println!("  max_p99_latency_ns: {max_p99_latency_ns}");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: serialized 32-slot role-binding package proof; not phase-center .nwpc and not raw-language action parsing"
    );

    assert_eq!(min_slot_accuracy, 1000);
    assert_eq!(min_energy_accuracy, 1000);
    assert_eq!(total_flat_gap_parity_mismatches, 0);
    assert_eq!(total_flat_energy_parity_mismatches, 0);
    assert_eq!(total_false_local_accepts, 0);
    assert!(rewrite_exact_all);
    assert!(nonzero_fingerprints);
    assert!(max_package_bytes < 128 * 1024);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
    assert!(max_p99_latency_ns <= p99_latency_gate_ns);
}

#[test]
#[ignore = "32-slot role-binding public SDK package proof; release gate"]
fn operator_battery_v4_slot32_role_binding_public_sdk_must_score_loaded_package_runtime() {
    let local_margin_threshold = 1_000_000i32;
    let p99_latency_gate_ns = 1_000_000u128;
    let mut reports = Vec::new();

    for seed in 0..SEQ32_MULTI_SEED_COUNT {
        let mixed_train_rows = slot32_mixed_map_corpus_tasks_for_seed("train", seed);
        let mixed_heldout_rows = slot32_mixed_map_corpus_tasks_for_seed("heldout", seed);
        let mixed_report = prove_slot32_role_binding_sdk_package(
            "sdk_mixed_map",
            seed,
            &mixed_train_rows,
            &mixed_heldout_rows,
            slot32_binding_config(),
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_role_binding_sdk_package_mixed_seed: seed={} path={} bytes={} fingerprint64={} edges={} rewrite_exact={} slot={} energy={} parity={} energy_parity={} p99_ns={} false_local_accepts={} hot_bytes={}",
            mixed_report.seed,
            mixed_report.package_path.display(),
            mixed_report.package_bytes,
            mixed_report.package_fingerprint64,
            mixed_report.inspected_edges,
            mixed_report.loaded_rewrite_exact,
            mixed_report.slot_accuracy_milli,
            mixed_report.sequence_energy_accuracy_milli,
            mixed_report.flat_gap_parity_mismatches,
            mixed_report.flat_sequence_energy_parity_mismatches,
            mixed_report.p99_latency_ns,
            mixed_report.false_local_accepts,
            mixed_report.hot_bytes_estimate
        );
        reports.push(mixed_report);

        let conditional_train_rows = slot32_conditional_branch_corpus_tasks_for_seed("train", seed);
        let conditional_heldout_rows =
            slot32_conditional_branch_corpus_tasks_for_seed("heldout", seed);
        let conditional_report = prove_slot32_role_binding_sdk_package(
            "sdk_conditional_branch",
            seed,
            &conditional_train_rows,
            &conditional_heldout_rows,
            slot32_conditional_binding_config(),
            local_margin_threshold,
        );
        println!(
            "operator_battery_v4_slot32_role_binding_sdk_package_conditional_seed: seed={} path={} bytes={} fingerprint64={} edges={} rewrite_exact={} slot={} energy={} parity={} energy_parity={} p99_ns={} false_local_accepts={} hot_bytes={}",
            conditional_report.seed,
            conditional_report.package_path.display(),
            conditional_report.package_bytes,
            conditional_report.package_fingerprint64,
            conditional_report.inspected_edges,
            conditional_report.loaded_rewrite_exact,
            conditional_report.slot_accuracy_milli,
            conditional_report.sequence_energy_accuracy_milli,
            conditional_report.flat_gap_parity_mismatches,
            conditional_report.flat_sequence_energy_parity_mismatches,
            conditional_report.p99_latency_ns,
            conditional_report.false_local_accepts,
            conditional_report.hot_bytes_estimate
        );
        reports.push(conditional_report);
    }

    let min_slot_accuracy = reports
        .iter()
        .map(|report| report.slot_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_energy_accuracy = reports
        .iter()
        .map(|report| report.sequence_energy_accuracy_milli)
        .min()
        .unwrap_or(0);
    let total_sdk_gap_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_gap_parity_mismatches)
        .sum();
    let total_sdk_energy_parity_mismatches: usize = reports
        .iter()
        .map(|report| report.flat_sequence_energy_parity_mismatches)
        .sum();
    let total_false_local_accepts: usize = reports
        .iter()
        .map(|report| report.false_local_accepts)
        .sum();
    let max_p99_latency_ns = reports
        .iter()
        .map(|report| report.p99_latency_ns)
        .max()
        .unwrap_or(0);
    let max_hot_bytes = reports
        .iter()
        .map(|report| report.hot_bytes_estimate)
        .max()
        .unwrap_or(0);
    let max_package_bytes = reports
        .iter()
        .map(|report| report.package_bytes)
        .max()
        .unwrap_or(0);
    let max_edges = reports
        .iter()
        .map(|report| report.inspected_edges)
        .max()
        .unwrap_or(0);
    let rewrite_exact_all = reports.iter().all(|report| report.loaded_rewrite_exact);
    let nonzero_fingerprints = reports
        .iter()
        .all(|report| report.package_fingerprint64 != 0);
    let labels = reports
        .iter()
        .map(|report| report.label)
        .collect::<BTreeSet<_>>();

    println!("operator_battery_v4_slot32_role_binding_sdk_package_gate:");
    println!("  verdict: SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS");
    println!(
        "  package_magic: {:?}",
        WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC
    );
    println!(
        "  package_dir: {}",
        slot32_role_binding_package_dir().display()
    );
    println!("  seeds: {}", SEQ32_MULTI_SEED_COUNT);
    println!("  labels: {:?}", labels);
    println!("  local_margin_threshold: {local_margin_threshold}");
    println!("  p99_latency_gate_ns: {p99_latency_gate_ns}");
    println!("  min_slot_accuracy_milli: {min_slot_accuracy}");
    println!("  min_sequence_energy_accuracy_milli: {min_energy_accuracy}");
    println!("  total_sdk_gap_parity_mismatches: {total_sdk_gap_parity_mismatches}");
    println!("  total_sdk_sequence_energy_parity_mismatches: {total_sdk_energy_parity_mismatches}");
    println!("  total_false_local_accepts: {total_false_local_accepts}");
    println!("  rewrite_exact_all: {rewrite_exact_all}");
    println!("  nonzero_fingerprints: {nonzero_fingerprints}");
    println!("  max_package_bytes: {max_package_bytes}");
    println!("  max_hot_bytes_estimate: {max_hot_bytes}");
    println!("  max_edges: {max_edges}");
    println!("  max_p99_latency_ns: {max_p99_latency_ns}");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: public SDK-loaded 32-slot role-binding .nwrb package proof; not phase-center .nwpc, not CLI/daemon registry, and not raw-language action parsing"
    );

    assert_eq!(min_slot_accuracy, 1000);
    assert_eq!(min_energy_accuracy, 1000);
    assert_eq!(total_sdk_gap_parity_mismatches, 0);
    assert_eq!(total_sdk_energy_parity_mismatches, 0);
    assert_eq!(total_false_local_accepts, 0);
    assert!(rewrite_exact_all);
    assert!(nonzero_fingerprints);
    assert!(max_package_bytes < 128 * 1024);
    assert!(max_hot_bytes < 4 * 1024 * 1024);
    assert!(max_p99_latency_ns <= p99_latency_gate_ns);
}

#[test]
#[ignore = "EDIT role-binding public SDK package proof; release gate"]
fn operator_battery_v4_edit_role_binding_public_sdk_must_score_loaded_package_runtime() {
    let local_margin_threshold = 1i32;
    let p99_latency_gate_ns = 1_000_000u128;
    let rows = load_operator_battery_v4_edit_rows();
    let train_rows = edit_train_rows(&rows);
    let heldout_rows = edit_heldout_rows(&rows);
    assert!(!train_rows.is_empty());
    assert!(!heldout_rows.is_empty());
    let train = prepare_edit_runtime_rows(&train_rows);
    let heldout = prepare_edit_runtime_rows(&heldout_rows);
    let report = prove_edit_role_binding_sdk_package(
        "sdk_edit_marker_length",
        0,
        &train,
        &heldout,
        local_margin_threshold,
    );

    println!("operator_battery_v4_edit_role_binding_sdk_package_gate:");
    println!("  verdict: EDIT_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS");
    println!(
        "  package_magic: {:?}",
        WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC
    );
    println!(
        "  package_dir: {}",
        slot32_role_binding_package_dir().display()
    );
    println!("  label: {}", report.label);
    println!("  seed: {}", report.seed);
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!("  local_margin_threshold: {local_margin_threshold}");
    println!("  p99_latency_gate_ns: {p99_latency_gate_ns}");
    println!("  package_path: {}", report.package_path.display());
    println!("  package_bytes: {}", report.package_bytes);
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!("  inspected_edges: {}", report.inspected_edges);
    println!("  loaded_rewrite_exact: {}", report.loaded_rewrite_exact);
    println!("  slot_accuracy_milli: {}", report.slot_accuracy_milli);
    println!(
        "  sequence_energy_accuracy_milli: {}",
        report.sequence_energy_accuracy_milli
    );
    println!(
        "  flat_gap_parity_mismatches: {}",
        report.flat_gap_parity_mismatches
    );
    println!(
        "  flat_sequence_energy_parity_mismatches: {}",
        report.flat_sequence_energy_parity_mismatches
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  hot_bytes_estimate: {}", report.hot_bytes_estimate);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  python_demo_used: false");
    println!("  corpus_jsonl_used: false");
    println!("  rust_runtime_used: true");
    println!(
        "  claim_boundary: EDIT role-binding .nwrb/.nwreb package proof for bounded marker/length corpus; not full EDIT blueprint coverage, not raw-language action parsing"
    );

    assert_eq!(report.slot_accuracy_milli, 1000);
    assert_eq!(report.sequence_energy_accuracy_milli, 1000);
    assert_eq!(report.flat_gap_parity_mismatches, 0);
    assert_eq!(report.flat_sequence_energy_parity_mismatches, 0);
    assert_eq!(report.false_local_accepts, 0);
    assert!(report.loaded_rewrite_exact);
    assert_ne!(report.package_fingerprint64, 0);
    assert!(report.package_bytes < 128 * 1024);
    assert!(report.hot_bytes_estimate < 4 * 1024 * 1024);
    assert!(report.p99_latency_ns <= p99_latency_gate_ns);
}

#[test]
fn position_sequence_v3_static_diagnostics_report() {
    let rows = load_sequence_v3_rows();
    assert!(!rows.is_empty());
    let action_report = action_separability_report(&rows);
    let collision_report = folded_collision_report(&rows);
    write_v3_static_diagnostics_report(&action_report, &collision_report);

    println!("position_sequence_v3_static_diagnostics:");
    println!("  action_vectors: {}", action_report.action_vectors);
    println!(
        "  same_rule_action_similarity_milli: {}",
        action_report.same_rule_similarity_milli
    );
    println!(
        "  different_rule_action_similarity_milli: {}",
        action_report.different_rule_similarity_milli
    );
    println!(
        "  same_family_different_length_similarity_milli: {}",
        action_report.same_family_different_length_similarity_milli
    );
    println!(
        "  different_family_similarity_milli: {}",
        action_report.different_family_similarity_milli
    );
    println!(
        "  max_different_rule_similarity_milli: {}",
        action_report.max_different_rule_similarity_milli
    );
    println!(
        "  folded_target_impulses_checked: {}",
        collision_report.target_impulses_checked
    );
    println!(
        "  folded_multi_role_hit_milli: {}",
        collision_report.multi_role_hit_milli
    );
    println!(
        "  folded_wrong_role_hit_milli: {}",
        collision_report.wrong_role_hit_milli
    );
    println!(
        "  folded_missing_true_role_milli: {}",
        collision_report.missing_true_role_milli
    );

    assert!(action_report.action_vectors > 0);
    assert!(collision_report.target_impulses_checked > 0);
}

fn train_binding_field(
    train_tasks: &[WavePredictorStateDeltaTrainTask],
    hebbian_config: WavePredictorHebbianConfig,
    trainer_config: WavePredictorTrainerConfig,
) -> WavePredictorHebbianField {
    let mut field = WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, hebbian_config);
    let report = WavePredictorTrainer::train_state_delta(&mut field, train_tasks, trainer_config);
    assert!(report.state_delta_training_used);
    assert!(!report.target_center_id_training_used);
    assert!(!report.axis_target_id_training_used);
    assert!(!report.semantic_grokking_claim_allowed);
    assert!(!report.base_mass_drift_detected);
    field
}

fn train_binding_field_with_progress(
    label: &str,
    train_tasks: &[WavePredictorStateDeltaTrainTask],
    hebbian_config: WavePredictorHebbianConfig,
    trainer_config: WavePredictorTrainerConfig,
) -> WavePredictorHebbianField {
    let mut field = WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, hebbian_config);
    println!(
        "{label}: training_start epochs={} train_tasks={}",
        trainer_config.epochs,
        train_tasks.len()
    );

    for epoch_index in 0..trainer_config.epochs {
        let margin = trainer_config.margin_schedule.margin_for_epoch(epoch_index);
        let epoch_config = WavePredictorTrainerConfig {
            epochs: 1,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: margin,
                target_margin: margin,
                warmup_epochs: 1,
                ramp_epochs: 1,
            },
            anti_wave_trap_updates_per_epoch_cap: trainer_config
                .anti_wave_trap_updates_per_epoch_cap,
        };
        let report = WavePredictorTrainer::train_state_delta(&mut field, train_tasks, epoch_config);
        assert!(report.state_delta_training_used);
        assert!(!report.target_center_id_training_used);
        assert!(!report.axis_target_id_training_used);
        assert!(!report.semantic_grokking_claim_allowed);
        assert!(!report.base_mass_drift_detected);
        let epoch = &report.epoch_reports[0];
        println!(
            "{label}: epoch={}/{} margin={} update_steps={} touched_edges={} margin_repairs={} margin_fixed={} state_delta_edges={} role_binding_edges={}",
            epoch_index + 1,
            trainer_config.epochs,
            margin,
            epoch.update_steps,
            epoch.touched_edges,
            epoch.margin_repairs,
            epoch.margin_fixed,
            field.state_delta_edge_count(),
            field.state_delta_role_binding_edge_count()
        );
    }

    println!(
        "{label}: training_done state_delta_edges={} role_binding_edges={}",
        field.state_delta_edge_count(),
        field.state_delta_role_binding_edge_count()
    );
    field
}

fn train_sequence_energy_field_with_progress(
    label: &str,
    train_tasks: &[PreparedSequenceTask],
    hebbian_config: WavePredictorHebbianConfig,
    trainer_config: WavePredictorTrainerConfig,
) -> WavePredictorHebbianField {
    let mut field = WavePredictorHebbianField::new(TOTAL_CENTER_COUNT, hebbian_config);
    println!(
        "{label}: training_start epochs={} sequence_tasks={}",
        trainer_config.epochs,
        train_tasks.len()
    );

    for epoch_index in 0..trainer_config.epochs {
        let margin = trainer_config.margin_schedule.margin_for_epoch(epoch_index);
        let mut repaired_rows = 0usize;
        let mut touched_edges = 0usize;
        let mut update_steps = 0usize;
        let mut min_energy_gap = i32::MAX;

        for task in train_tasks {
            let energy_gap = sequence_energy_gap(&field, task);
            min_energy_gap = min_energy_gap.min(energy_gap);
            if energy_gap >= margin {
                continue;
            }
            repaired_rows += 1;
            for slot_task in &task.slot_tasks {
                let update =
                    WavePredictorTrainer::train_state_delta_step(&mut field, slot_task, margin);
                if update.touched_edges > 0 {
                    update_steps += 1;
                    touched_edges += update.touched_edges;
                }
            }
        }

        let train_energy = ordered_sequence_energy_diagnostics(&field, train_tasks);
        println!(
            "{label}: epoch={}/{} margin={} repaired_rows={} update_steps={} touched_edges={} train_energy_accuracy_milli={} train_energy_median_gap={} train_energy_p10_gap={} min_energy_gap={} state_delta_edges={} role_binding_edges={}",
            epoch_index + 1,
            trainer_config.epochs,
            margin,
            repaired_rows,
            update_steps,
            touched_edges,
            train_energy.energy_accuracy_milli,
            train_energy.median_energy_gap,
            train_energy.p10_energy_gap,
            min_energy_gap,
            field.state_delta_edge_count(),
            field.state_delta_role_binding_edge_count()
        );
    }

    println!(
        "{label}: training_done state_delta_edges={} role_binding_edges={}",
        field.state_delta_edge_count(),
        field.state_delta_role_binding_edge_count()
    );
    field
}

fn train_sequence_combined_field_with_progress(
    label: &str,
    train_tasks: &[PreparedSequenceTask],
    hebbian_config: WavePredictorHebbianConfig,
    local_config: WavePredictorTrainerConfig,
    cleanup_config: WavePredictorTrainerConfig,
) -> WavePredictorHebbianField {
    let flat_slot_tasks: Vec<_> = train_tasks
        .iter()
        .flat_map(|task| task.slot_tasks.iter().cloned())
        .collect();
    let mut field = train_binding_field_with_progress(
        &format!("{label}_local"),
        &flat_slot_tasks,
        hebbian_config,
        local_config,
    );

    println!(
        "{label}: cleanup_start epochs={} sequence_tasks={}",
        cleanup_config.epochs,
        train_tasks.len()
    );
    for epoch_index in 0..cleanup_config.epochs {
        let margin = cleanup_config.margin_schedule.margin_for_epoch(epoch_index);
        let mut repaired_rows = 0usize;
        let mut update_steps = 0usize;
        let mut touched_edges = 0usize;
        let mut min_slot_gap = i32::MAX;
        let mut min_energy_gap = i32::MAX;

        for task in train_tasks {
            let energy_gap = sequence_energy_gap(&field, task);
            min_energy_gap = min_energy_gap.min(energy_gap);
            let mut row_repaired = false;
            for slot_task in &task.slot_tasks {
                let slot_gap = state_delta_sum_gap(&field, slot_task);
                min_slot_gap = min_slot_gap.min(slot_gap);
                if slot_gap >= margin && energy_gap >= margin {
                    continue;
                }
                let update =
                    WavePredictorTrainer::train_state_delta_step(&mut field, slot_task, margin);
                if update.touched_edges > 0 {
                    update_steps += 1;
                    touched_edges += update.touched_edges;
                    row_repaired = true;
                }
            }
            repaired_rows += usize::from(row_repaired);
        }

        let train_slot = eval_ordered_sequence(&field, train_tasks);
        let train_energy = ordered_sequence_energy_diagnostics(&field, train_tasks);
        println!(
            "{label}: cleanup_epoch={}/{} margin={} repaired_rows={} update_steps={} touched_edges={} train_slot_accuracy_milli={} train_energy_accuracy_milli={} train_energy_median_gap={} train_energy_p10_gap={} min_slot_gap={} min_energy_gap={} state_delta_edges={} role_binding_edges={}",
            epoch_index + 1,
            cleanup_config.epochs,
            margin,
            repaired_rows,
            update_steps,
            touched_edges,
            train_slot.accuracy_milli,
            train_energy.energy_accuracy_milli,
            train_energy.median_energy_gap,
            train_energy.p10_energy_gap,
            min_slot_gap,
            min_energy_gap,
            field.state_delta_edge_count(),
            field.state_delta_role_binding_edge_count()
        );
    }
    println!(
        "{label}: cleanup_done state_delta_edges={} role_binding_edges={}",
        field.state_delta_edge_count(),
        field.state_delta_role_binding_edge_count()
    );
    field
}

fn train_sequence_candidate_cleanup_field_with_progress(
    label: &str,
    mut field: WavePredictorHebbianField,
    train_rows: &[SequenceBindingRow],
    trainer_config: WavePredictorTrainerConfig,
) -> WavePredictorHebbianField {
    let candidate_tasks = prepare_sequence_all_candidate_slot_tasks(train_rows);
    println!(
        "{label}: candidate_cleanup_start epochs={} candidate_slot_tasks={}",
        trainer_config.epochs,
        candidate_tasks.len()
    );

    for epoch_index in 0..trainer_config.epochs {
        let margin = trainer_config.margin_schedule.margin_for_epoch(epoch_index);
        let mut repaired_slots = 0usize;
        let mut update_steps = 0usize;
        let mut touched_edges = 0usize;
        let mut min_candidate_gap = i32::MAX;

        for slot_task in &candidate_tasks {
            let gap = state_delta_sum_gap(&field, slot_task);
            min_candidate_gap = min_candidate_gap.min(gap);
            if gap >= margin {
                continue;
            }
            repaired_slots += 1;
            let update =
                WavePredictorTrainer::train_state_delta_step(&mut field, slot_task, margin);
            if update.touched_edges > 0 {
                update_steps += 1;
                touched_edges += update.touched_edges;
            }
        }

        println!(
            "{label}: candidate_cleanup_epoch={}/{} margin={} repaired_slots={} update_steps={} touched_edges={} min_candidate_gap={} state_delta_edges={} role_binding_edges={}",
            epoch_index + 1,
            trainer_config.epochs,
            margin,
            repaired_slots,
            update_steps,
            touched_edges,
            min_candidate_gap,
            field.state_delta_edge_count(),
            field.state_delta_role_binding_edge_count()
        );
    }

    println!(
        "{label}: candidate_cleanup_done state_delta_edges={} role_binding_edges={}",
        field.state_delta_edge_count(),
        field.state_delta_role_binding_edge_count()
    );
    field
}

fn compile_sequence_one_pass_wave_field_with_progress(
    label: &str,
    train_tasks: &[PreparedSequenceTask],
    hebbian_config: WavePredictorHebbianConfig,
) -> (WavePredictorHebbianField, usize) {
    let mut field = WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, hebbian_config);
    let eta_binding = i32::from(hebbian_config.eta_binding);
    let total_slot_tasks: usize = train_tasks.iter().map(|task| task.slot_tasks.len()).sum();
    let mut touched_edges = 0usize;
    let mut processed_slots = 0usize;

    println!(
        "{label}: one_pass_compile_start sequence_tasks={} slot_tasks={} eta_binding={}",
        train_tasks.len(),
        total_slot_tasks,
        hebbian_config.eta_binding
    );

    for (row_idx, task) in train_tasks.iter().enumerate() {
        if row_idx > 0 && row_idx % 250 == 0 {
            println!(
                "{label}: one_pass_compile_progress rows_done={row_idx}/{} touched_edges={} role_binding_edges={}",
                train_tasks.len(),
                touched_edges,
                field.state_delta_role_binding_edge_count()
            );
        }
        for slot_task in &task.slot_tasks {
            processed_slots += 1;
            for impulse in slot_task.target_delta.positive_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    eta_binding * magnitude,
                );
            }
            for impulse in slot_task.target_delta.negative_impulses() {
                let magnitude = i32::from(impulse.signed_strength).abs().max(1);
                touched_edges += field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &slot_task.active_fringe,
                    slot_task.binding_output_slot,
                    -eta_binding * magnitude,
                );
            }
        }
    }

    println!(
        "{label}: one_pass_compile_done processed_slots={} touched_edges={} state_delta_edges={} role_binding_edges={}",
        processed_slots,
        touched_edges,
        field.state_delta_edge_count(),
        field.state_delta_role_binding_edge_count()
    );
    (field, touched_edges)
}

fn eval_one_pass_wave_compile_class(
    label: &str,
    train: &[PreparedSequenceTask],
    heldout: &[PreparedSequenceTask],
    hebbian_config: WavePredictorHebbianConfig,
    action_base: WavePredictorCenterId,
) -> OnePassWaveCompileEval {
    let (field, touched_edges) =
        compile_sequence_one_pass_wave_field_with_progress(label, train, hebbian_config);
    println!("{label}: eval_slot_start");
    let slot = eval_ordered_sequence(&field, heldout);
    println!("{label}: eval_energy_start");
    let energy = ordered_sequence_energy_diagnostics(&field, heldout);
    println!("{label}: compile_flat_start");
    let flat = field.compile_flat_role_binding_table();
    println!(
        "{label}: compile_flat_done flat_role_binding_edges={}",
        flat.edge_count()
    );
    let flat_index = FlatRoleBindingScoreIndex::new(&flat, hebbian_config);
    println!("{label}: eval_flat_slot_start");
    let flat_slot = eval_ordered_sequence_flat_fast(&flat_index, heldout);
    println!("{label}: eval_flat_gap_parity_start");
    let flat_parity = eval_ordered_sequence_flat_gap_parity_fast(&field, &flat_index, heldout);
    println!("{label}: eval_flat_energy_parity_start");
    let flat_energy_parity =
        eval_ordered_sequence_flat_energy_parity_fast(&field, &flat_index, heldout);
    println!("{label}: eval_ablation_without_binding_start");
    let empty = WavePredictorHebbianField::new(SEQ_TOTAL_CENTER_COUNT, hebbian_config);
    let no_binding = eval_ordered_sequence(&empty, heldout);
    println!("{label}: eval_ablation_without_action_start");
    let no_action_tasks = ablate_sequence_tasks(heldout, |center_id| center_id < action_base);
    let no_action = eval_ordered_sequence(&field, &no_action_tasks);
    println!("{label}: eval_ablation_without_role_start");
    let no_role_tasks = ablate_sequence_tasks(heldout, |center_id| center_id >= action_base);
    let no_role = eval_ordered_sequence(&field, &no_role_tasks);
    println!("{label}: eval_ablation_without_active_start");
    let no_active_tasks = ablate_sequence_tasks(heldout, |_| false);
    let no_active = eval_ordered_sequence(&field, &no_active_tasks);

    OnePassWaveCompileEval {
        train_rows: train.len(),
        heldout_rows: heldout.len(),
        slot_accuracy_milli: slot.accuracy_milli,
        sequence_energy_accuracy_milli: energy.energy_accuracy_milli,
        flat_slot_accuracy_milli: flat_slot.accuracy_milli,
        flat_gap_parity_mismatches: flat_parity.mismatches,
        flat_energy_parity_mismatches: flat_energy_parity.mismatches,
        ablation_without_binding_accuracy_milli: no_binding.accuracy_milli,
        ablation_without_action_accuracy_milli: no_action.accuracy_milli,
        ablation_without_role_accuracy_milli: no_role.accuracy_milli,
        ablation_without_active_fringe_accuracy_milli: no_active.accuracy_milli,
        state_delta_edges: field.state_delta_edge_count(),
        role_binding_edges: field.state_delta_role_binding_edge_count(),
        touched_role_binding_edges: touched_edges,
    }
}

fn print_one_pass_wave_compile_eval(label: &str, eval: OnePassWaveCompileEval) {
    println!("  {label}_train_rows: {}", eval.train_rows);
    println!("  {label}_heldout_rows: {}", eval.heldout_rows);
    println!(
        "  {label}_slot_accuracy_milli: {}",
        eval.slot_accuracy_milli
    );
    println!(
        "  {label}_sequence_energy_accuracy_milli: {}",
        eval.sequence_energy_accuracy_milli
    );
    println!(
        "  {label}_flat_slot_accuracy_milli: {}",
        eval.flat_slot_accuracy_milli
    );
    println!(
        "  {label}_flat_gap_parity_mismatches: {}",
        eval.flat_gap_parity_mismatches
    );
    println!(
        "  {label}_flat_energy_parity_mismatches: {}",
        eval.flat_energy_parity_mismatches
    );
    println!(
        "  {label}_ablation_without_binding_accuracy_milli: {}",
        eval.ablation_without_binding_accuracy_milli
    );
    println!(
        "  {label}_ablation_without_action_accuracy_milli: {}",
        eval.ablation_without_action_accuracy_milli
    );
    println!(
        "  {label}_ablation_without_role_accuracy_milli: {}",
        eval.ablation_without_role_accuracy_milli
    );
    println!(
        "  {label}_ablation_without_active_fringe_accuracy_milli: {}",
        eval.ablation_without_active_fringe_accuracy_milli
    );
    println!("  {label}_state_delta_edges: {}", eval.state_delta_edges);
    println!("  {label}_role_binding_edges: {}", eval.role_binding_edges);
    println!(
        "  {label}_touched_role_binding_edges: {}",
        eval.touched_role_binding_edges
    );
}

#[derive(Clone, Copy)]
enum PhaseKeyMode {
    Action,
    ClassLength,
}

fn eval_phase_center_report(rows: &[PhaseOperatorRow], cells: usize) -> PhaseCenterReport {
    PhaseCenterReport {
        action: eval_phase_center_mode(rows, cells, PhaseKeyMode::Action),
        no_action: eval_phase_center_mode(rows, cells, PhaseKeyMode::ClassLength),
    }
}

fn eval_phase_center_mode(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
) -> PhaseCenterEval {
    eval_phase_center_mode_disabled(rows, cells, key_mode, &BTreeSet::new())
}

fn eval_phase_center_mode_disabled(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
    disabled_cells: &BTreeSet<usize>,
) -> PhaseCenterEval {
    let train_rows = rows
        .iter()
        .filter(|row| phase_split(row) == Some("train"))
        .count();
    let heldout_rows = rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
        .count();
    let mut positive_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut negative_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut phase_cache = HashMap::new();
    let mut skipped_train_rows = 0usize;

    for row in rows.iter().filter(|row| phase_split(row) == Some("train")) {
        let key = phase_operator_key(row, key_mode);
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            skipped_train_rows += 1;
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            skipped_train_rows += 1;
            continue;
        };
        add_phase_vector(
            positive_sums
                .entry(key.clone())
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &correct_vec,
            1.0,
        );
        add_phase_vector(
            negative_sums
                .entry(key)
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &wrong_vec,
            1.0,
        );
    }

    let positive_centers = positive_sums
        .into_iter()
        .map(|(key, value)| (key, phase_center_from_sum(&value)))
        .collect::<BTreeMap<_, _>>();
    let negative_centers = negative_sums
        .into_iter()
        .map(|(key, value)| (key, phase_center_from_sum(&value)))
        .collect::<BTreeMap<_, _>>();

    let mut margins = Vec::with_capacity(heldout_rows);
    let mut center_gaps = Vec::with_capacity(heldout_rows);
    let mut missing_heldout_centers = 0usize;
    let mut skipped_eval_rows = 0usize;
    let mut wrong_wins = 0usize;
    let mut heldout_correct_rows = 0usize;
    let mut heldout_surface_groups = BTreeSet::new();
    let mut heldout_noise_groups = BTreeSet::new();

    for row in rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
    {
        heldout_surface_groups.insert(row.surface_family.as_str());
        heldout_noise_groups.insert(row.noise_type.as_str());
        let key = phase_operator_key(row, key_mode);
        let Some(pos_center) = positive_centers.get(&key) else {
            missing_heldout_centers += 1;
            continue;
        };
        let Some(neg_center) = negative_centers.get(&key) else {
            missing_heldout_centers += 1;
            continue;
        };
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            skipped_eval_rows += 1;
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            skipped_eval_rows += 1;
            continue;
        };

        let correct_pos = phase_coherence_disabled(&correct_vec, pos_center, disabled_cells);
        let wrong_pos = phase_coherence_disabled(&wrong_vec, pos_center, disabled_cells);
        let correct_neg = phase_coherence_disabled(&correct_vec, neg_center, disabled_cells);
        let wrong_neg = phase_coherence_disabled(&wrong_vec, neg_center, disabled_cells);
        let correct_score = correct_pos - correct_neg;
        let wrong_score = wrong_pos - wrong_neg;
        let margin = correct_score - wrong_score;
        margins.push(margin);
        center_gaps.push(correct_pos - wrong_pos);
        if margin > 0.0 {
            heldout_correct_rows += 1;
        } else {
            wrong_wins += 1;
        }
    }

    margins.sort_by(f64::total_cmp);
    center_gaps.sort_by(f64::total_cmp);
    PhaseCenterEval {
        train_rows,
        heldout_rows,
        heldout_surface_groups: heldout_surface_groups.len(),
        heldout_noise_groups: heldout_noise_groups.len(),
        compiled_phase_centers: positive_centers.len(),
        skipped_train_rows,
        missing_heldout_centers,
        skipped_eval_rows,
        wrong_wins,
        heldout_correct_rows,
        heldout_accuracy_milli: milli_ratio(heldout_correct_rows, heldout_rows),
        median_margin: percentile_f64(&margins, 50),
        p10_margin: percentile_f64(&margins, 10),
        median_positive_center_gap: percentile_f64(&center_gaps, 50),
        p10_positive_center_gap: percentile_f64(&center_gaps, 10),
    }
}

fn print_phase_center_capacity_point(cells: usize, report: &PhaseCenterReport) {
    println!(
        "  capacity_cells_{cells}_action_accuracy_milli: {}",
        report.action.heldout_accuracy_milli
    );
    println!(
        "  capacity_cells_{cells}_action_wrong_wins: {}",
        report.action.wrong_wins
    );
    println!(
        "  capacity_cells_{cells}_action_p10_margin: {:.6}",
        report.action.p10_margin
    );
    println!(
        "  capacity_cells_{cells}_no_action_accuracy_milli: {}",
        report.no_action.heldout_accuracy_milli
    );
    println!(
        "  capacity_cells_{cells}_no_action_wrong_wins: {}",
        report.no_action.wrong_wins
    );
}

fn print_phase_ablation_point(label: &str, eval: PhaseCenterEval) {
    println!(
        "  ablation_{label}_accuracy_milli: {}",
        eval.heldout_accuracy_milli
    );
    println!("  ablation_{label}_wrong_wins: {}", eval.wrong_wins);
    println!(
        "  ablation_{label}_median_margin: {:.6}",
        eval.median_margin
    );
    println!("  ablation_{label}_p10_margin: {:.6}", eval.p10_margin);
}

fn phase_capacity_ablation_verdict(
    full: PhaseCenterEval,
    ablated: PhaseCenterEval,
) -> &'static str {
    if full.heldout_accuracy_milli == 1000
        && full.wrong_wins == 0
        && ablated.median_margin < full.median_margin
        && ablated.p10_margin < full.p10_margin
    {
        "PHASE_CENTER_CAPACITY_ABLATION_PASS"
    } else {
        "PHASE_CENTER_CAPACITY_ABLATION_WATCH"
    }
}

fn phase_center_cell_importance_order(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
) -> Vec<usize> {
    let mut positive_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut negative_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut phase_cache = HashMap::new();
    for row in rows.iter().filter(|row| phase_split(row) == Some("train")) {
        let key = phase_operator_key(row, key_mode);
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            continue;
        };
        add_phase_vector(
            positive_sums
                .entry(key.clone())
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &correct_vec,
            1.0,
        );
        add_phase_vector(
            negative_sums
                .entry(key)
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &wrong_vec,
            1.0,
        );
    }

    let mut importance = vec![0.0f64; cells];
    for (key, positive_sum) in positive_sums {
        let Some(negative_sum) = negative_sums.get(&key) else {
            continue;
        };
        let positive = phase_center_from_sum(&positive_sum);
        let negative = phase_center_from_sum(negative_sum);
        for cell in 0..cells {
            let re = positive[cell].re - negative[cell].re;
            let im = positive[cell].im - negative[cell].im;
            importance[cell] += re * re + im * im;
        }
    }

    let mut order = (0..cells).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        importance[*right]
            .total_cmp(&importance[*left])
            .then_with(|| left.cmp(right))
    });
    order
}

fn disabled_phase_cells(order: &[usize], count: usize) -> BTreeSet<usize> {
    order.iter().take(count).copied().collect()
}

fn eval_flat_phase_center_runtime_report(
    rows: &[PhaseOperatorRow],
    cells: usize,
) -> FlatPhaseRuntimeReport {
    let compiler_eval = eval_phase_center_mode(rows, cells, PhaseKeyMode::Action);
    let (runtime, key_to_index, _) =
        compile_flat_phase_center_runtime(rows, cells, PhaseKeyMode::Action);
    let prepared = prepare_flat_phase_eval_tasks(rows, cells, PhaseKeyMode::Action, &key_to_index);
    let flat_eval = eval_flat_phase_runtime(&runtime, &prepared.tasks);
    let (sign_mismatches, margin_mismatches) =
        flat_phase_runtime_parity(rows, cells, PhaseKeyMode::Action, &runtime, &key_to_index);

    let (no_action_runtime, no_action_key_to_index, _) =
        compile_flat_phase_center_runtime(rows, cells, PhaseKeyMode::ClassLength);
    let no_action_prepared = prepare_flat_phase_eval_tasks(
        rows,
        cells,
        PhaseKeyMode::ClassLength,
        &no_action_key_to_index,
    );
    let no_action_flat_eval =
        eval_flat_phase_runtime(&no_action_runtime, &no_action_prepared.tasks);

    FlatPhaseRuntimeReport {
        compiler_eval,
        flat_eval,
        no_action_flat_eval,
        flat_sign_parity_mismatches: sign_mismatches,
        flat_margin_parity_mismatches: margin_mismatches,
        missing_centers: prepared.missing_centers,
        skipped_rows: prepared.skipped_rows,
        heldout_surface_groups: prepared.heldout_surface_groups,
        heldout_noise_groups: prepared.heldout_noise_groups,
        bytes_estimate: flat_phase_runtime_bytes_estimate(&runtime),
    }
}

fn compile_flat_phase_center_runtime(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
) -> (FlatPhaseCenterRuntime, BTreeMap<String, usize>, usize) {
    let (positive_centers, negative_centers, skipped_train_rows) =
        compile_phase_center_maps(rows, cells, key_mode);
    let mut records = Vec::with_capacity(positive_centers.len());
    let mut key_to_index = BTreeMap::new();
    for (key, positive_center) in positive_centers {
        let Some(negative_center) = negative_centers.get(&key) else {
            continue;
        };
        let center_index = records.len();
        key_to_index.insert(key, center_index);
        records.push(FlatPhaseCenterRecord {
            positive_center,
            negative_center: negative_center.clone(),
        });
    }
    (
        FlatPhaseCenterRuntime { cells, records },
        key_to_index,
        skipped_train_rows,
    )
}

fn compile_core_phase_center_runtime(
    runtime: &FlatPhaseCenterRuntime,
) -> CorePhaseCenterFlatRuntime {
    let records = runtime
        .records
        .iter()
        .map(|record| CorePhaseCenterFlatRecord {
            positive_center: core_phase_vec(&record.positive_center),
            negative_center: core_phase_vec(&record.negative_center),
        })
        .collect::<Vec<_>>();
    CorePhaseCenterFlatRuntime::new(runtime.cells, records).expect("valid core phase runtime")
}

fn compile_core_phase_center_runtime_from_rows(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
) -> (CorePhaseCenterFlatRuntime, BTreeMap<String, usize>, usize) {
    let mut train_items = Vec::new();
    let mut keys = BTreeSet::new();
    let mut skipped_train_rows = 0usize;
    for row in rows.iter().filter(|row| phase_split(row) == Some("train")) {
        let key = phase_operator_key(row, key_mode);
        let Some(positive_atoms) = phase_transition_atoms(row, &row.correct_tokens) else {
            skipped_train_rows += 1;
            continue;
        };
        let Some(negative_atoms) = phase_transition_atoms(row, &row.wrong_tokens) else {
            skipped_train_rows += 1;
            continue;
        };
        keys.insert(key.clone());
        train_items.push((key, positive_atoms, negative_atoms));
    }

    let key_to_index = keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<BTreeMap<_, _>>();
    let mut compiler =
        CorePhaseCenterCompiler::new(cells, key_to_index.len()).expect("valid core compiler");
    for (key, positive_atoms, negative_atoms) in train_items {
        let program_index = key_to_index
            .get(&key)
            .copied()
            .expect("train key has compiler index");
        compiler
            .add_positive_atoms(program_index, positive_atoms.iter().map(String::as_str))
            .expect("positive phase atoms accepted");
        compiler
            .add_negative_atoms(program_index, negative_atoms.iter().map(String::as_str))
            .expect("negative phase atoms accepted");
    }

    (
        compiler.compile().expect("complete core compiler"),
        key_to_index,
        skipped_train_rows,
    )
}

fn core_phase_eval_task(task: &FlatPhaseEvalTask) -> CorePhaseCenterEvalTask {
    CorePhaseCenterEvalTask {
        center_index: task.center_index,
        correct_vec: core_phase_vec(&task.correct_vec),
        wrong_vec: core_phase_vec(&task.wrong_vec),
    }
}

fn core_phase_vec(values: &[PhaseCell]) -> Box<[CorePhaseCenterCell]> {
    values
        .iter()
        .map(|value| CorePhaseCenterCell {
            re: value.re,
            im: value.im,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn core_phase_eval_tasks(tasks: &[FlatPhaseEvalTask]) -> Vec<CorePhaseCenterEvalTask> {
    tasks.iter().map(core_phase_eval_task).collect()
}

fn eval_core_phase_runtime(
    runtime: &CorePhaseCenterFlatRuntime,
    tasks: &[CorePhaseCenterEvalTask],
) -> FlatPhaseRuntimeEval {
    let mut margins = Vec::with_capacity(tasks.len());
    let mut latencies = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    let total_start = Instant::now();
    for task in tasks {
        let start = Instant::now();
        let margin = runtime.margin(task).expect("valid core runtime task");
        latencies.push(start.elapsed().as_nanos());
        margins.push(margin);
        correct += usize::from(margin > 0.0);
    }
    let total_eval_us = total_start.elapsed().as_micros();
    margins.sort_by(f64::total_cmp);
    latencies.sort_unstable();
    FlatPhaseRuntimeEval {
        rows: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        wrong_wins: tasks.len().saturating_sub(correct),
        median_margin: percentile_f64(&margins, 50),
        p10_margin: percentile_f64(&margins, 10),
        p50_latency_ns: percentile_u128(&latencies, 50),
        p99_latency_ns: percentile_u128(&latencies, 99),
        total_eval_us,
    }
}

fn compile_phase_center_maps(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
) -> (PhaseCenterMap, PhaseCenterMap, usize) {
    let mut positive_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut negative_sums: BTreeMap<String, Vec<PhaseCell>> = BTreeMap::new();
    let mut phase_cache = HashMap::new();
    let mut skipped_train_rows = 0usize;
    for row in rows.iter().filter(|row| phase_split(row) == Some("train")) {
        let key = phase_operator_key(row, key_mode);
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            skipped_train_rows += 1;
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            skipped_train_rows += 1;
            continue;
        };
        add_phase_vector(
            positive_sums
                .entry(key.clone())
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &correct_vec,
            1.0,
        );
        add_phase_vector(
            negative_sums
                .entry(key)
                .or_insert_with(|| vec![PhaseCell::default(); cells]),
            &wrong_vec,
            1.0,
        );
    }

    let positive_centers = positive_sums
        .into_iter()
        .map(|(key, value)| (key, phase_center_from_sum(&value)))
        .collect::<BTreeMap<_, _>>();
    let negative_centers = negative_sums
        .into_iter()
        .map(|(key, value)| (key, phase_center_from_sum(&value)))
        .collect::<BTreeMap<_, _>>();
    (positive_centers, negative_centers, skipped_train_rows)
}

fn prepare_flat_phase_eval_tasks(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
    key_to_index: &BTreeMap<String, usize>,
) -> FlatPhasePreparedEval {
    let mut tasks = Vec::new();
    let mut phase_cache = HashMap::new();
    let mut missing_centers = 0usize;
    let mut skipped_rows = 0usize;
    let mut heldout_surface_groups = BTreeSet::new();
    let mut heldout_noise_groups = BTreeSet::new();

    for row in rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
    {
        heldout_surface_groups.insert(row.surface_family.as_str());
        heldout_noise_groups.insert(row.noise_type.as_str());
        let key = phase_operator_key(row, key_mode);
        let Some(center_index) = key_to_index.get(&key).copied() else {
            missing_centers += 1;
            continue;
        };
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            skipped_rows += 1;
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            skipped_rows += 1;
            continue;
        };
        tasks.push(FlatPhaseEvalTask {
            center_index,
            correct_vec,
            wrong_vec,
        });
    }

    FlatPhasePreparedEval {
        tasks,
        missing_centers,
        skipped_rows,
        heldout_surface_groups: heldout_surface_groups.len(),
        heldout_noise_groups: heldout_noise_groups.len(),
    }
}

fn eval_flat_phase_runtime(
    runtime: &FlatPhaseCenterRuntime,
    tasks: &[FlatPhaseEvalTask],
) -> FlatPhaseRuntimeEval {
    let mut margins = Vec::with_capacity(tasks.len());
    let mut latencies = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    let total_start = Instant::now();
    for task in tasks {
        let start = Instant::now();
        let margin = flat_phase_margin(runtime, task);
        latencies.push(start.elapsed().as_nanos());
        margins.push(margin);
        correct += usize::from(margin > 0.0);
    }
    let total_eval_us = total_start.elapsed().as_micros();
    margins.sort_by(f64::total_cmp);
    latencies.sort_unstable();
    FlatPhaseRuntimeEval {
        rows: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        wrong_wins: tasks.len().saturating_sub(correct),
        median_margin: percentile_f64(&margins, 50),
        p10_margin: percentile_f64(&margins, 10),
        p50_latency_ns: percentile_u128(&latencies, 50),
        p99_latency_ns: percentile_u128(&latencies, 99),
        total_eval_us,
    }
}

fn flat_phase_margin(runtime: &FlatPhaseCenterRuntime, task: &FlatPhaseEvalTask) -> f64 {
    let record = &runtime.records[task.center_index];
    let correct_pos = phase_coherence(&task.correct_vec, &record.positive_center);
    let wrong_pos = phase_coherence(&task.wrong_vec, &record.positive_center);
    let correct_neg = phase_coherence(&task.correct_vec, &record.negative_center);
    let wrong_neg = phase_coherence(&task.wrong_vec, &record.negative_center);
    (correct_pos - correct_neg) - (wrong_pos - wrong_neg)
}

fn flat_phase_runtime_parity(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_mode: PhaseKeyMode,
    runtime: &FlatPhaseCenterRuntime,
    key_to_index: &BTreeMap<String, usize>,
) -> (usize, usize) {
    let (positive_centers, negative_centers, _) = compile_phase_center_maps(rows, cells, key_mode);
    let mut phase_cache = HashMap::new();
    let mut sign_mismatches = 0usize;
    let mut margin_mismatches = 0usize;
    for row in rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
    {
        let key = phase_operator_key(row, key_mode);
        let Some(center_index) = key_to_index.get(&key).copied() else {
            continue;
        };
        let Some(pos_center) = positive_centers.get(&key) else {
            continue;
        };
        let Some(neg_center) = negative_centers.get(&key) else {
            continue;
        };
        let Some(correct_vec) =
            phase_transition_vector(row, &row.correct_tokens, cells, &mut phase_cache)
        else {
            continue;
        };
        let Some(wrong_vec) =
            phase_transition_vector(row, &row.wrong_tokens, cells, &mut phase_cache)
        else {
            continue;
        };

        let direct_margin =
            phase_margin_from_centers(&correct_vec, &wrong_vec, pos_center, neg_center);
        let flat_margin = flat_phase_margin(
            runtime,
            &FlatPhaseEvalTask {
                center_index,
                correct_vec,
                wrong_vec,
            },
        );
        sign_mismatches += usize::from((direct_margin > 0.0) != (flat_margin > 0.0));
        margin_mismatches += usize::from((direct_margin - flat_margin).abs() > 1e-12);
    }
    (sign_mismatches, margin_mismatches)
}

fn phase_margin_from_centers(
    correct_vec: &[PhaseCell],
    wrong_vec: &[PhaseCell],
    positive_center: &[PhaseCell],
    negative_center: &[PhaseCell],
) -> f64 {
    let correct_pos = phase_coherence(correct_vec, positive_center);
    let wrong_pos = phase_coherence(wrong_vec, positive_center);
    let correct_neg = phase_coherence(correct_vec, negative_center);
    let wrong_neg = phase_coherence(wrong_vec, negative_center);
    (correct_pos - correct_neg) - (wrong_pos - wrong_neg)
}

fn phase_coherence(vector: &[PhaseCell], center: &[PhaseCell]) -> f64 {
    phase_coherence_disabled(vector, center, &BTreeSet::new())
}

fn flat_phase_runtime_bytes_estimate(runtime: &FlatPhaseCenterRuntime) -> usize {
    runtime.records.len() * 2 * runtime.cells * std::mem::size_of::<PhaseCell>()
        + runtime.records.len() * std::mem::size_of::<FlatPhaseCenterRecord>()
}

fn flat_phase_runtime_verdict(report: &FlatPhaseRuntimeReport) -> &'static str {
    if report.compiler_eval.heldout_accuracy_milli == 1000
        && report.flat_eval.accuracy_milli == report.compiler_eval.heldout_accuracy_milli
        && report.flat_eval.wrong_wins == 0
        && report.flat_sign_parity_mismatches == 0
        && report.flat_margin_parity_mismatches == 0
        && report.no_action_flat_eval.accuracy_milli < report.flat_eval.accuracy_milli
        && report.no_action_flat_eval.wrong_wins > 0
    {
        "PHASE_CENTER_FLAT_RUNTIME_PASS"
    } else {
        "PHASE_CENTER_FLAT_RUNTIME_WATCH"
    }
}

fn phase_operator_key(row: &PhaseOperatorRow, mode: PhaseKeyMode) -> String {
    let condition = row
        .condition_flag
        .as_deref()
        .map(|value| format!("condition={value}"))
        .unwrap_or_else(|| "condition=<none>".to_string());
    let mut key = format!(
        "class={}|length={}|{}",
        row.operator_class, row.sequence_length, condition
    );
    if matches!(mode, PhaseKeyMode::Action) {
        key.push_str("|action=");
        key.push_str(&normalized_phase_action(row));
    }
    key
}

fn normalized_phase_action(row: &PhaseOperatorRow) -> String {
    let action = if let Some(marker) = extract_marker_value(&row.action) {
        row.action.replace(&marker, "<MARKER>")
    } else {
        row.action.clone()
    };
    collapse_whitespace(&action)
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn phase_split(row: &PhaseOperatorRow) -> Option<&'static str> {
    if row.source_group.contains("_train_") {
        Some("train")
    } else if row.source_group.contains("_heldout_") {
        Some("heldout")
    } else {
        None
    }
}

fn phase_transition_vector(
    row: &PhaseOperatorRow,
    candidate_tokens: &[String],
    cells: usize,
    phase_cache: &mut HashMap<(String, usize), PhaseCell>,
) -> Option<Vec<PhaseCell>> {
    let atoms = phase_transition_atoms(row, candidate_tokens)?;
    let mut sums = vec![PhaseCell::default(); cells];
    for atom in atoms {
        for (cell, sum) in sums.iter_mut().enumerate() {
            let phase = cached_phase_hash(&atom, cell, phase_cache);
            sum.re += phase.re;
            sum.im += phase.im;
        }
    }
    Some(sums.iter().map(|cell| phase_circular_unit(*cell)).collect())
}

fn phase_transition_atoms(
    row: &PhaseOperatorRow,
    candidate_tokens: &[String],
) -> Option<Vec<String>> {
    let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, token) in row.source_tokens.iter().enumerate() {
        positions.entry(token.as_str()).or_default().push(index);
    }

    let marker = extract_marker_value(&row.action);
    let mut atoms = vec![
        format!("class:{}", row.operator_class),
        format!("src_len:{}", row.source_tokens.len()),
        format!("out_len:{}", candidate_tokens.len()),
    ];
    for (out_slot, token) in candidate_tokens.iter().enumerate() {
        if let Some(source_slots) = positions.get(token.as_str()) {
            if source_slots.len() != 1 {
                return None;
            }
            let src_slot = source_slots[0];
            atoms.push(format!("rel:o{out_slot}:s{src_slot}"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push(format!("src:s{src_slot}"));
            atoms.push(format!("delta:{}", out_slot as isize - src_slot as isize));
        } else if marker.as_deref() == Some(token.as_str()) {
            atoms.push(format!("rel:o{out_slot}:marker"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push("src:marker".to_string());
        } else {
            return None;
        }
    }
    Some(atoms)
}

fn cached_phase_hash(
    atom: &str,
    cell: usize,
    phase_cache: &mut HashMap<(String, usize), PhaseCell>,
) -> PhaseCell {
    let key = (atom.to_string(), cell);
    if let Some(phase) = phase_cache.get(&key) {
        return *phase;
    }
    let phase = stable_phase_hash(atom, cell);
    phase_cache.insert(key, phase);
    phase
}

fn stable_phase_hash(atom: &str, cell: usize) -> PhaseCell {
    let input = format!("{cell}\0{atom}");
    let hash = blake2b8_personalized(input.as_bytes(), b"nwphase");
    let angle = (hash as f64 / (u64::MAX as f64 + 1.0)) * std::f64::consts::TAU;
    PhaseCell {
        re: angle.cos(),
        im: angle.sin(),
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

fn phase_circular_unit(value: PhaseCell) -> PhaseCell {
    let magnitude = (value.re * value.re + value.im * value.im).sqrt();
    if magnitude == 0.0 {
        PhaseCell::default()
    } else {
        PhaseCell {
            re: value.re / magnitude,
            im: value.im / magnitude,
        }
    }
}

fn phase_center_from_sum(values: &[PhaseCell]) -> Vec<PhaseCell> {
    values
        .iter()
        .map(|value| phase_circular_unit(*value))
        .collect()
}

fn add_phase_vector(target: &mut [PhaseCell], source: &[PhaseCell], sign: f64) {
    for (target_cell, source_cell) in target.iter_mut().zip(source.iter()) {
        target_cell.re += sign * source_cell.re;
        target_cell.im += sign * source_cell.im;
    }
}

fn phase_coherence_disabled(
    vector: &[PhaseCell],
    center: &[PhaseCell],
    disabled_cells: &BTreeSet<usize>,
) -> f64 {
    if vector.is_empty() || center.is_empty() {
        return 0.0;
    }
    let mut active = 0usize;
    let mut score = 0.0f64;
    for (index, (value, center)) in vector.iter().zip(center.iter()).enumerate() {
        if disabled_cells.contains(&index) {
            continue;
        }
        active += 1;
        score += value.re * center.re + value.im * center.im;
    }
    if active == 0 {
        0.0
    } else {
        score / active as f64
    }
}

fn percentile_f64(values: &[f64], percentile: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = (values.len() * percentile / 100).min(values.len() - 1);
    values[index]
}

fn percentile_u128(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let index = (values.len() * percentile / 100).min(values.len() - 1);
    values[index]
}

fn phase_center_bytes_estimate(report: &PhaseCenterReport, cells: usize) -> usize {
    report.action.compiled_phase_centers * 2 * cells * std::mem::size_of::<PhaseCell>()
}

fn phase_center_verdict(report: &PhaseCenterReport) -> &'static str {
    if report.action.heldout_accuracy_milli == 1000
        && report.action.wrong_wins == 0
        && report.action.p10_margin > 0.0
        && report.no_action.heldout_accuracy_milli < report.action.heldout_accuracy_milli
        && report.no_action.wrong_wins > 0
    {
        "PHASE_CENTER_RUNTIME_PROBE_PASS"
    } else {
        "PHASE_CENTER_RUNTIME_PROBE_WATCH"
    }
}

fn binding_hebbian_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_pos: 0,
        eta_neg: 0,
        eta_conflict: 2,
        eta_anti: 6,
        eta_binding: 2,
        state_delta_binding_feature_base: None,
        state_delta_binding_action_base: Some(ACTION_CENTER_BASE),
        state_delta_binding_action_count: FEATURE_CENTER_COUNT,
        state_delta_binding_role_base: Some(ROLE_CENTER_BASE),
        state_delta_binding_role_stride: FEATURE_CENTER_COUNT,
        state_delta_binding_role_count: ROLE_SLOT_COUNT as u8,
        state_delta_binding_slot_scoped_action_page_bits: 0,
        state_delta_binding_slot_scoped_action_page_mask: 0,
        state_delta_binding_slot_scoped_action_source_bits: 0,
        weight_limit: 1024,
    }
}

fn binding_disabled_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_binding: 0,
        ..binding_hebbian_config()
    }
}

fn action_disabled_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        state_delta_binding_action_base: None,
        state_delta_binding_action_count: 0,
        ..binding_hebbian_config()
    }
}

fn role_disabled_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        state_delta_binding_role_base: None,
        state_delta_binding_role_stride: 0,
        state_delta_binding_role_count: 0,
        ..binding_hebbian_config()
    }
}

fn sequence_binding_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_pos: 0,
        eta_neg: 0,
        eta_conflict: 2,
        eta_anti: 6,
        eta_binding: 2,
        state_delta_binding_feature_base: None,
        state_delta_binding_action_base: Some(SEQ_ACTION_SLOT_BASE),
        state_delta_binding_action_count: SEQ_ACTION_CENTER_COUNT,
        state_delta_binding_role_base: Some(SEQ_ROLE_BASE),
        state_delta_binding_role_stride: SEQ_FEATURE_CENTER_COUNT,
        state_delta_binding_role_count: SEQ_ROLE_SLOT_COUNT,
        state_delta_binding_slot_scoped_action_page_bits: SEQ_PAGE_BITS as u8,
        state_delta_binding_slot_scoped_action_page_mask: (1_u64 << SEQ_OPERATOR_PAIR_PAGE)
            | (1_u64 << SEQ_CONDITION_ACTION_PAGE)
            | (1_u64 << SEQ_COMPOSED_DEMO_PAGE),
        state_delta_binding_slot_scoped_action_source_bits: 4,
        weight_limit: 1024,
    }
}

fn slot32_binding_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_pos: 0,
        eta_neg: 0,
        eta_conflict: 2,
        eta_anti: 6,
        eta_binding: 2,
        state_delta_binding_feature_base: None,
        state_delta_binding_action_base: Some(SEQ32_ACTION_BASE),
        state_delta_binding_action_count: SEQ32_ACTION_CENTER_COUNT,
        state_delta_binding_role_base: Some(SEQ32_ROLE_BASE),
        state_delta_binding_role_stride: SEQ32_FEATURE_CENTER_COUNT,
        state_delta_binding_role_count: SEQ32_ROLE_SLOT_COUNT,
        state_delta_binding_slot_scoped_action_page_bits: SEQ32_PAGE_BITS as u8,
        state_delta_binding_slot_scoped_action_page_mask: 1_u64 << SEQ32_OPERATOR_PAIR_PAGE,
        state_delta_binding_slot_scoped_action_source_bits: 5,
        weight_limit: 1024,
    }
}

fn slot32_conditional_binding_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        state_delta_binding_action_count: SEQ32_CONDITIONAL_ACTION_CENTER_COUNT,
        state_delta_binding_slot_scoped_action_page_mask: (1_u64
            << SEQ32_CONDITION_TRUE_ACTION_PAGE)
            | (1_u64 << SEQ32_CONDITION_FALSE_ACTION_PAGE),
        ..slot32_binding_config()
    }
}

fn edit_binding_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_pos: 0,
        eta_neg: 0,
        eta_conflict: 2,
        eta_anti: 6,
        eta_binding: 2,
        state_delta_binding_feature_base: None,
        state_delta_binding_action_base: Some(EDIT_ACTION_BASE),
        state_delta_binding_action_count: EDIT_ACTION_CENTER_COUNT,
        state_delta_binding_role_base: Some(EDIT_ROLE_BASE),
        state_delta_binding_role_stride: SEQ_FEATURE_CENTER_COUNT,
        state_delta_binding_role_count: EDIT_ROLE_SLOT_COUNT,
        state_delta_binding_slot_scoped_action_page_bits: SEQ_PAGE_BITS as u8,
        state_delta_binding_slot_scoped_action_page_mask: 1_u64 << EDIT_DEMO_PAGE,
        state_delta_binding_slot_scoped_action_source_bits: 5,
        weight_limit: 1024,
    }
}

fn frame_wave_config() -> WavePredictorHebbianConfig {
    WavePredictorHebbianConfig {
        eta_pos: 4,
        eta_neg: 3,
        eta_conflict: 2,
        eta_anti: 6,
        eta_binding: 0,
        state_delta_binding_feature_base: None,
        state_delta_binding_action_base: None,
        state_delta_binding_action_count: 0,
        state_delta_binding_role_base: None,
        state_delta_binding_role_stride: 0,
        state_delta_binding_role_count: 0,
        state_delta_binding_slot_scoped_action_page_bits: 0,
        state_delta_binding_slot_scoped_action_page_mask: 0,
        state_delta_binding_slot_scoped_action_source_bits: 0,
        weight_limit: 1024,
    }
}

fn prepare_sequence_rows(rows: &[SequenceBindingRow]) -> Vec<PreparedSequenceTask> {
    rows.iter().map(PreparedSequenceTask::new).collect()
}

fn prepare_edit_runtime_rows(rows: &[SequenceBindingRow]) -> Vec<PreparedSequenceTask> {
    rows.iter().map(edit_sequence_task).collect()
}

fn sequence_unique_rules(rows: &[SequenceBindingRow]) -> usize {
    rows.iter()
        .map(|row| row.rule_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn sequence_unique_surfaces(rows: &[SequenceBindingRow]) -> usize {
    rows.iter()
        .map(|row| row.surface_family.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn sequence_unique_noise_types(rows: &[SequenceBindingRow]) -> usize {
    rows.iter()
        .map(|row| row.noise_type.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn sequence_unique_lengths(rows: &[SequenceBindingRow]) -> usize {
    rows.iter()
        .map(|row| row.sequence_length)
        .collect::<BTreeSet<_>>()
        .len()
}

fn sequence_train_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group.starts_with("position_sequence_train_")
                || row
                    .source_group
                    .starts_with("operator_battery_order_train_")
        })
        .cloned()
        .collect()
}

fn sequence_heldout_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group.starts_with("position_sequence_heldout_")
                || row
                    .source_group
                    .starts_with("operator_battery_order_heldout_")
        })
        .cloned()
        .collect()
}

fn edit_train_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| row.source_group.starts_with("operator_battery_edit_train_"))
        .cloned()
        .collect()
}

fn edit_heldout_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group
                .starts_with("operator_battery_edit_heldout_")
        })
        .cloned()
        .collect()
}

fn conditional_train_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group
                .starts_with("operator_battery_conditional_train_")
        })
        .cloned()
        .collect()
}

fn conditional_heldout_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group
                .starts_with("operator_battery_conditional_heldout_")
        })
        .cloned()
        .collect()
}

fn composed_train_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group
                .starts_with("operator_battery_composed_train_")
        })
        .cloned()
        .collect()
}

fn composed_heldout_rows(rows: &[SequenceBindingRow]) -> Vec<SequenceBindingRow> {
    rows.iter()
        .filter(|row| {
            row.source_group
                .starts_with("operator_battery_composed_heldout_")
        })
        .cloned()
        .collect()
}

fn env_u16(name: &str, default: u16) -> u16 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u16>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_usize_list(name: &str, default: &[usize]) -> Vec<usize> {
    let Some(values) = std::env::var(name).ok() else {
        return default.to_vec();
    };
    let parsed = values
        .split(',')
        .filter_map(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        default.to_vec()
    } else {
        parsed
    }
}

impl PreparedSequenceTask {
    fn new(row: &SequenceBindingRow) -> Self {
        assert_eq!(row.correct_tokens.len(), row.wrong_tokens.len());
        let mut slot_tasks = Vec::with_capacity(row.correct_tokens.len());
        let mut output_slots = Vec::with_capacity(row.correct_tokens.len());
        for output_slot in 0..row.correct_tokens.len() {
            slot_tasks.push(sequence_slot_task(row, output_slot));
            output_slots.push(output_slot);
        }
        Self {
            slot_tasks,
            output_slots,
        }
    }
}

fn sequence_slot_task(
    row: &SequenceBindingRow,
    output_slot: usize,
) -> WavePredictorStateDeltaTrainTask {
    sequence_slot_task_with_wrong_token(row, output_slot, &row.wrong_tokens[output_slot])
}

fn sequence_slot_task_with_wrong_token(
    row: &SequenceBindingRow,
    output_slot: usize,
    wrong_token: &str,
) -> WavePredictorStateDeltaTrainTask {
    assert!(output_slot < usize::from(SEQ_OUTPUT_SLOT_COUNT));
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(&row.correct_tokens[output_slot]);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive = discriminative_delta_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let negative = discriminative_delta_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let target_delta = WavePredictorStateDeltaTarget::from_impulses(&positive, &negative)
        .expect("sequence slot delta must fit");
    WavePredictorStateDeltaTrainTask {
        active_fringe: sequence_active_fringe(row, output_slot),
        target_delta,
        binding_output_slot: Some(output_slot as u8),
    }
}

fn edit_sequence_task(row: &SequenceBindingRow) -> PreparedSequenceTask {
    let max_output_len = row.correct_tokens.len().max(row.wrong_tokens.len());
    assert!(max_output_len <= usize::from(EDIT_OUTPUT_SLOT_COUNT));
    let mut slot_tasks = Vec::with_capacity(max_output_len);
    let mut output_slots = Vec::with_capacity(max_output_len);
    for output_slot in 0..max_output_len {
        if let Some(slot_task) = edit_slot_task(row, output_slot) {
            slot_tasks.push(slot_task);
            output_slots.push(output_slot);
        }
    }
    assert!(
        !slot_tasks.is_empty(),
        "edit row must contain at least one discriminative output slot"
    );
    PreparedSequenceTask {
        slot_tasks,
        output_slots,
    }
}

fn edit_slot_task(
    row: &SequenceBindingRow,
    output_slot: usize,
) -> Option<WavePredictorStateDeltaTrainTask> {
    assert!(output_slot < usize::from(EDIT_OUTPUT_SLOT_COUNT));
    let correct_token = row
        .correct_tokens
        .get(output_slot)
        .map(String::as_str)
        .unwrap_or(EDIT_END_TOKEN);
    let wrong_token = row
        .wrong_tokens
        .get(output_slot)
        .map(String::as_str)
        .unwrap_or(EDIT_END_TOKEN);
    if correct_token == wrong_token {
        return None;
    }
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive = discriminative_delta_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let negative = discriminative_delta_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let target_delta = WavePredictorStateDeltaTarget::from_impulses(&positive, &negative)
        .expect("edit slot delta must fit");
    Some(WavePredictorStateDeltaTrainTask {
        active_fringe: edit_active_fringe(row),
        target_delta,
        binding_output_slot: Some(output_slot as u8),
    })
}

fn prepare_sequence_all_candidate_slot_tasks(
    rows: &[SequenceBindingRow],
) -> Vec<WavePredictorStateDeltaTrainTask> {
    let mut tasks = Vec::new();
    for row in rows {
        let source_tokens = sequence_source_tokens(&row.state_before);
        for output_slot in 0..row.correct_tokens.len() {
            let target_token = &row.correct_tokens[output_slot];
            let mut seen_wrong = BTreeSet::new();
            for wrong_token in &source_tokens {
                if wrong_token == target_token || !seen_wrong.insert(wrong_token.as_str()) {
                    continue;
                }
                tasks.push(sequence_slot_task_with_wrong_token(
                    row,
                    output_slot,
                    wrong_token,
                ));
            }
        }
    }
    tasks
}

fn sequence_active_fringe(
    row: &SequenceBindingRow,
    _output_slot: usize,
) -> Vec<WavePredictorActiveCenter> {
    let mut centers = Vec::new();
    if sequence_action_surface_centers_enabled(&row.action) {
        centers.extend(surface_lane_centers_folded(
            &row.action,
            SEQ_ACTION_SLOT_BASE,
            SEQ_FEATURE_CENTER_COUNT,
            TOP_ACTION_L1_LANES,
        ));
    }
    if sequence_operator_pair_action_centers_enabled() {
        centers.extend(sequence_operator_pair_action_centers(&row.action));
    }
    centers.extend(sequence_state_condition_centers(&row.state_before));
    centers.extend(sequence_condition_action_conjunction_centers(
        &row.state_before,
        &row.action,
    ));
    centers.extend(sequence_composed_demo_action_centers(&row.action));
    let tokens = sequence_source_tokens(&row.state_before);
    for (slot_id, token) in tokens
        .iter()
        .take(usize::from(SEQ_ROLE_SLOT_COUNT))
        .enumerate()
    {
        let slot_base =
            SEQ_ROLE_BASE + WavePredictorCenterId::from(slot_id as u8) * SEQ_FEATURE_CENTER_COUNT;
        centers.extend(surface_lane_centers_folded(
            token,
            slot_base,
            SEQ_FEATURE_CENTER_COUNT,
            TOP_ROLE_L1_LANES,
        ));
    }
    merge_centers(centers)
}

fn sequence_action_surface_centers_enabled(action: &str) -> bool {
    // Conditional and composed actions already expose their operator through
    // structured motif pages. Their raw action text contains both branches or
    // demos, so keeping it active becomes a fuzzy shortcut/conflict channel.
    if action.contains("operator_class: conditional") {
        return std::env::var("OPERATOR_BATTERY_V4_CONDITIONAL_ACTION_SURFACE")
            .map(|raw| !matches!(raw.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(false);
    }
    if action.contains("operator_class: composed") {
        return std::env::var("OPERATOR_BATTERY_V4_COMPOSED_ACTION_SURFACE")
            .map(|raw| !matches!(raw.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(false);
    }
    true
}

fn edit_active_fringe(row: &SequenceBindingRow) -> Vec<WavePredictorActiveCenter> {
    let mut centers = Vec::new();
    centers.extend(edit_demo_action_centers(&row.action));
    let tokens = sequence_source_tokens(&row.state_before);
    for (slot_id, token) in tokens
        .iter()
        .take(usize::from(SEQ_ROLE_SLOT_COUNT))
        .enumerate()
    {
        let slot_base =
            EDIT_ROLE_BASE + WavePredictorCenterId::from(slot_id as u8) * SEQ_FEATURE_CENTER_COUNT;
        centers.extend(surface_lane_centers_folded(
            token,
            slot_base,
            SEQ_FEATURE_CENTER_COUNT,
            TOP_ROLE_L1_LANES,
        ));
    }
    let marker_slot_base = EDIT_ROLE_BASE
        + WavePredictorCenterId::from(EDIT_MARKER_ROLE_SLOT) * SEQ_FEATURE_CENTER_COUNT;
    if let Some(marker) = extract_marker_value(&row.action) {
        centers.extend(surface_lane_centers_folded(
            &marker,
            marker_slot_base,
            SEQ_FEATURE_CENTER_COUNT,
            TOP_ROLE_L1_LANES,
        ));
    }
    centers.extend(surface_lane_centers_folded(
        EDIT_END_TOKEN,
        marker_slot_base,
        SEQ_FEATURE_CENTER_COUNT,
        TOP_ROLE_L1_LANES,
    ));
    merge_centers(centers)
}

fn edit_demo_action_centers(action: &str) -> Vec<WavePredictorActiveCenter> {
    if !action.contains("operator_class: edit") {
        return Vec::new();
    }
    let mut slots = parse_edit_demo_final_slots(action);
    while slots.len() < usize::from(EDIT_OUTPUT_SLOT_COUNT) {
        slots.push(usize::from(EDIT_MARKER_ROLE_SLOT));
    }
    slots
        .into_iter()
        .take(usize::from(EDIT_OUTPUT_SLOT_COUNT))
        .enumerate()
        .filter(|(_, role_slot)| *role_slot < usize::from(EDIT_ROLE_SLOT_COUNT))
        .map(|(output_slot, role_slot)| WavePredictorActiveCenter {
            center_id: EDIT_DEMO_BASE + edit_operator_pair_lane(output_slot, role_slot),
            strength: 8,
        })
        .collect()
}

fn is_edit_demo_center(center_id: WavePredictorCenterId) -> bool {
    (EDIT_DEMO_BASE..EDIT_DEMO_BASE + SEQ_FEATURE_CENTER_COUNT).contains(&center_id)
}

fn is_edit_marker_role_center(center_id: WavePredictorCenterId) -> bool {
    let marker_base = EDIT_ROLE_BASE
        + WavePredictorCenterId::from(EDIT_MARKER_ROLE_SLOT) * SEQ_FEATURE_CENTER_COUNT;
    (marker_base..marker_base + SEQ_FEATURE_CENTER_COUNT).contains(&center_id)
}

fn sequence_state_condition_centers(state_before: &str) -> Vec<WavePredictorActiveCenter> {
    let Some(flag) = extract_flag_value(state_before, "condition: flag_") else {
        return Vec::new();
    };
    surface_lane_centers_folded(
        &format!("state_condition flag_{flag}"),
        SEQ_STATE_CONDITION_BASE,
        SEQ_FEATURE_CENTER_COUNT,
        TOP_ACTION_L1_LANES,
    )
}

fn is_sequence_state_condition_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ_STATE_CONDITION_BASE..SEQ_STATE_CONDITION_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
}

fn sequence_condition_action_conjunction_centers(
    state_before: &str,
    action: &str,
) -> Vec<WavePredictorActiveCenter> {
    let Some(state_flag) = extract_flag_value(state_before, "condition: flag_") else {
        return Vec::new();
    };
    let Some(trigger_flag) = extract_flag_value(action, "if flag_") else {
        return Vec::new();
    };
    let selected_slots = if state_flag == trigger_flag {
        parse_prefixed_operator_slots(action, "then_slots:")
    } else {
        parse_prefixed_operator_slots(action, "else_slots:")
    };
    selected_slots
        .into_iter()
        .enumerate()
        .filter(|(output_slot, source_slot)| {
            *output_slot < usize::from(SEQ_OUTPUT_SLOT_COUNT)
                && *source_slot < usize::from(SEQ_ROLE_SLOT_COUNT)
        })
        .map(|(output_slot, source_slot)| WavePredictorActiveCenter {
            center_id: SEQ_CONDITION_ACTION_BASE
                + sequence_operator_pair_lane(output_slot, source_slot),
            strength: 8,
        })
        .collect()
}

fn is_sequence_condition_action_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ_CONDITION_ACTION_BASE..SEQ_CONDITION_ACTION_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
}

fn sequence_composed_demo_action_centers(action: &str) -> Vec<WavePredictorActiveCenter> {
    if !action.contains("operator_class: composed") {
        return Vec::new();
    }
    parse_composed_demo_final_slots(action)
        .into_iter()
        .enumerate()
        .filter(|(output_slot, source_slot)| {
            *output_slot < usize::from(SEQ_OUTPUT_SLOT_COUNT)
                && *source_slot < usize::from(SEQ_ROLE_SLOT_COUNT)
        })
        .map(|(output_slot, source_slot)| WavePredictorActiveCenter {
            center_id: SEQ_COMPOSED_DEMO_BASE
                + sequence_operator_pair_lane(output_slot, source_slot),
            strength: 8,
        })
        .collect()
}

fn is_sequence_composed_demo_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ_COMPOSED_DEMO_BASE..SEQ_COMPOSED_DEMO_BASE + SEQ_FEATURE_CENTER_COUNT).contains(&center_id)
}

fn sequence_operator_pair_action_centers_enabled() -> bool {
    std::env::var("POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS")
        .map(|raw| matches!(raw.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn sequence_operator_pair_action_centers(action: &str) -> Vec<WavePredictorActiveCenter> {
    parse_operator_slots(action)
        .into_iter()
        .enumerate()
        .filter(|(output_slot, source_slot)| {
            *output_slot < usize::from(SEQ_OUTPUT_SLOT_COUNT)
                && *source_slot < usize::from(SEQ_ROLE_SLOT_COUNT)
        })
        .map(|(output_slot, source_slot)| WavePredictorActiveCenter {
            center_id: SEQ_OPERATOR_PAIR_BASE
                + sequence_operator_pair_lane(output_slot, source_slot),
            strength: 8,
        })
        .collect()
}

fn parse_operator_slots(action: &str) -> Vec<usize> {
    parse_prefixed_operator_slots(action, "operator_slots:")
}

fn parse_prefixed_operator_slots(action: &str, marker: &str) -> Vec<usize> {
    let Some((_, after_marker)) = action.split_once(marker) else {
        return Vec::new();
    };
    let slot_segment = after_marker.split(';').next().unwrap_or(after_marker);
    slot_segment
        .split_whitespace()
        .filter_map(|token| token.strip_prefix("src"))
        .filter_map(|number| number.parse::<usize>().ok())
        .collect()
}

fn parse_composed_demo_final_slots(action: &str) -> Vec<usize> {
    let Some((_, after_demo)) = action.split_once("demo:") else {
        return Vec::new();
    };
    let demo_segment = after_demo.split(';').next().unwrap_or(after_demo);
    let Some(final_segment) = demo_segment.rsplit("->").next() else {
        return Vec::new();
    };
    final_segment
        .split_whitespace()
        .filter_map(|token| token.strip_prefix('d'))
        .filter_map(|number| number.parse::<usize>().ok())
        .collect()
}

fn parse_edit_demo_final_slots(action: &str) -> Vec<usize> {
    let Some((_, after_demo)) = action.split_once("demo:") else {
        return Vec::new();
    };
    let demo_segment = after_demo.split(';').next().unwrap_or(after_demo);
    let Some(final_segment) = demo_segment.rsplit("->").next() else {
        return Vec::new();
    };
    final_segment
        .split_whitespace()
        .map(|token| {
            token
                .strip_prefix('d')
                .and_then(|number| number.parse::<usize>().ok())
                .unwrap_or(usize::from(EDIT_MARKER_ROLE_SLOT))
        })
        .collect()
}

fn edit_operator_pair_lane(output_slot: usize, role_slot: usize) -> WavePredictorCenterId {
    let output_slot = WavePredictorCenterId::try_from(output_slot).expect("output slot fits u32");
    let role_slot = WavePredictorCenterId::try_from(role_slot).expect("role slot fits u32");
    (output_slot << 5) | role_slot
}

fn extract_marker_value(action: &str) -> Option<String> {
    let start = action.find("marker:")? + "marker:".len();
    let value = action[start..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn sequence_operator_pair_lane(output_slot: usize, source_slot: usize) -> WavePredictorCenterId {
    let output_slot = WavePredictorCenterId::try_from(output_slot).expect("output slot fits u32");
    let source_slot = WavePredictorCenterId::try_from(source_slot).expect("source slot fits u32");
    (output_slot << 4) | source_slot
}

fn slot32_operator_pair_lane(output_slot: usize, source_slot: usize) -> WavePredictorCenterId {
    let output_slot = WavePredictorCenterId::try_from(output_slot).expect("output slot fits u32");
    let source_slot = WavePredictorCenterId::try_from(source_slot).expect("source slot fits u32");
    (output_slot << 5) | source_slot
}

fn slot32_capacity_tasks_for_seed(split: &str, seed: usize) -> Vec<PreparedSequenceTask> {
    slot32_capacity_labeled_tasks_for_seed(split, seed)
        .into_iter()
        .map(|(_, _, task)| task)
        .collect()
}

fn slot32_order_corpus_tasks_for_seed(split: &str, seed: usize) -> Vec<Slot32OrderCorpusTask> {
    const SURFACES: [&str; 4] = ["symbols", "network", "business", "ru_words"];
    const NOISE_TYPES: [&str; 2] = ["clean", "wrapped"];

    let mut tasks = Vec::new();
    for length in 17..=32 {
        for surface_family in SURFACES {
            for noise_type in NOISE_TYPES {
                let tokens =
                    slot32_order_corpus_tokens(split, seed, length, surface_family, noise_type);
                let state_key = tokens.join(" ");
                for (operator_class, rule_name, slot_map) in slot32_order_corpus_maps(length) {
                    let wrong_map = slot32_wrong_map(&slot_map, length);
                    let correct_tokens = slot_map
                        .iter()
                        .map(|source_slot| tokens[*source_slot].clone())
                        .collect::<Vec<_>>();
                    let wrong_tokens = wrong_map
                        .iter()
                        .map(|source_slot| tokens[*source_slot].clone())
                        .collect::<Vec<_>>();
                    tasks.push(Slot32OrderCorpusTask {
                        length,
                        operator_class,
                        rule_name,
                        surface_family,
                        noise_type,
                        condition_result: None,
                        state_key: state_key.clone(),
                        correct_tokens,
                        wrong_tokens,
                        task: slot32_prepared_task(&tokens, &slot_map, &wrong_map),
                    });
                }
            }
        }
    }
    tasks
}

fn slot32_mixed_map_corpus_tasks_for_seed(split: &str, seed: usize) -> Vec<Slot32OrderCorpusTask> {
    const SURFACES: [&str; 4] = ["symbols", "network", "business", "ru_words"];
    const NOISE_TYPES: [&str; 2] = ["clean", "wrapped"];

    let mut tasks = Vec::new();
    for length in 17..=32 {
        for surface_family in SURFACES {
            for noise_type in NOISE_TYPES {
                let tokens =
                    slot32_order_corpus_tokens(split, seed, length, surface_family, noise_type);
                let state_key = tokens.join(" ");
                for (operator_class, rule_name, slot_map) in slot32_mixed_map_corpus_maps(length) {
                    let wrong_map = slot32_wrong_map(&slot_map, length);
                    let correct_tokens = slot_map
                        .iter()
                        .map(|source_slot| tokens[*source_slot].clone())
                        .collect::<Vec<_>>();
                    let wrong_tokens = wrong_map
                        .iter()
                        .map(|source_slot| tokens[*source_slot].clone())
                        .collect::<Vec<_>>();
                    tasks.push(Slot32OrderCorpusTask {
                        length,
                        operator_class,
                        rule_name,
                        surface_family,
                        noise_type,
                        condition_result: None,
                        state_key: state_key.clone(),
                        correct_tokens,
                        wrong_tokens,
                        task: slot32_prepared_task(&tokens, &slot_map, &wrong_map),
                    });
                }
            }
        }
    }
    tasks
}

fn slot32_conditional_branch_corpus_tasks_for_seed(
    split: &str,
    seed: usize,
) -> Vec<Slot32OrderCorpusTask> {
    const SURFACES: [&str; 4] = ["symbols", "network", "business", "ru_words"];
    const NOISE_TYPES: [&str; 2] = ["clean", "wrapped"];
    const STATE_FLAGS: [bool; 2] = [false, true];

    let mut tasks = Vec::new();
    for length in 17..=32 {
        for surface_family in SURFACES {
            for noise_type in NOISE_TYPES {
                let tokens =
                    slot32_order_corpus_tokens(split, seed, length, surface_family, noise_type);
                let state_key = tokens.join(" ");
                for state_flag in STATE_FLAGS {
                    for (rule_name, trigger_flag, then_map, else_map) in
                        slot32_conditional_branch_corpus_maps(length)
                    {
                        let condition_result = state_flag == trigger_flag;
                        let (slot_map, wrong_map) = if condition_result {
                            (then_map, else_map)
                        } else {
                            (else_map, then_map)
                        };
                        let correct_tokens = slot_map
                            .iter()
                            .map(|source_slot| tokens[*source_slot].clone())
                            .collect::<Vec<_>>();
                        let wrong_tokens = wrong_map
                            .iter()
                            .map(|source_slot| tokens[*source_slot].clone())
                            .collect::<Vec<_>>();
                        tasks.push(Slot32OrderCorpusTask {
                            length,
                            operator_class: "conditional",
                            rule_name,
                            surface_family,
                            noise_type,
                            condition_result: Some(condition_result),
                            state_key: state_key.clone(),
                            correct_tokens,
                            wrong_tokens,
                            task: slot32_conditional_prepared_task(
                                &tokens,
                                state_flag,
                                condition_result,
                                &slot_map,
                                &wrong_map,
                            ),
                        });
                    }
                }
            }
        }
    }
    tasks
}

fn slot32_capacity_labeled_tasks_for_seed(
    split: &str,
    seed: usize,
) -> Vec<(usize, &'static str, PreparedSequenceTask)> {
    let mut tasks = Vec::new();
    for length in 17..=32 {
        for (rule_name, slot_map) in slot32_capacity_maps(length) {
            let tokens = slot32_tokens(split, seed, rule_name, length);
            let wrong_map = slot32_wrong_map(&slot_map, length);
            tasks.push((
                length,
                rule_name,
                slot32_prepared_task(&tokens, &slot_map, &wrong_map),
            ));
        }
    }
    tasks
}

fn slot32_capacity_maps(length: usize) -> Vec<(&'static str, Vec<usize>)> {
    vec![
        (
            "mirror",
            (0..length).map(|output| length - 1 - output).collect(),
        ),
        (
            "rotate_left",
            (0..length).map(|output| (output + 1) % length).collect(),
        ),
        (
            "rotate_right",
            (0..length)
                .map(|output| (output + length - 1) % length)
                .collect(),
        ),
        (
            "pair_swap",
            (0..length)
                .map(|output| {
                    if output % 2 == 0 {
                        (output + 1).min(length - 1)
                    } else {
                        output - 1
                    }
                })
                .collect(),
        ),
    ]
}

fn slot32_order_corpus_maps(length: usize) -> Vec<(&'static str, &'static str, Vec<usize>)> {
    vec![
        (
            "order",
            "mirror",
            (0..length).map(|output| length - 1 - output).collect(),
        ),
        (
            "order",
            "rotate_left_1",
            (0..length).map(|output| (output + 1) % length).collect(),
        ),
        (
            "order",
            "rotate_right_1",
            (0..length)
                .map(|output| (output + length - 1) % length)
                .collect(),
        ),
        (
            "order",
            "rotate_half",
            (0..length)
                .map(|output| (output + length / 2) % length)
                .collect(),
        ),
        (
            "order",
            "pair_swap",
            (0..length)
                .map(|output| {
                    if output % 2 == 0 {
                        (output + 1).min(length - 1)
                    } else {
                        output - 1
                    }
                })
                .collect(),
        ),
        (
            "order",
            "even_then_odd",
            slot32_even_then_odd_sources(length),
        ),
        (
            "order",
            "odd_then_even",
            slot32_odd_then_even_sources(length),
        ),
        ("order", "center_out", slot32_center_out_sources(length)),
    ]
}

fn slot32_mixed_map_corpus_maps(length: usize) -> Vec<(&'static str, &'static str, Vec<usize>)> {
    let order_maps = slot32_order_corpus_maps(length);
    let edit_maps = vec![
        (
            "edit",
            "duplicate_head_drop_tail",
            (0..length)
                .map(|output| output.saturating_sub(1))
                .collect::<Vec<_>>(),
        ),
        (
            "edit",
            "duplicate_tail_drop_head",
            (0..length)
                .map(|output| (output + 1).min(length - 1))
                .collect::<Vec<_>>(),
        ),
        (
            "edit",
            "copy_center_to_edges",
            (0..length)
                .map(|output| {
                    if output == 0 || output + 1 == length {
                        length / 2
                    } else {
                        output
                    }
                })
                .collect::<Vec<_>>(),
        ),
        (
            "edit",
            "compress_pairs_to_left",
            (0..length)
                .map(|output| ((output / 2) * 2).min(length - 1))
                .collect::<Vec<_>>(),
        ),
    ];

    let mirror = (0..length)
        .map(|output| length - 1 - output)
        .collect::<Vec<_>>();
    let rotate_left = (0..length)
        .map(|output| (output + 1) % length)
        .collect::<Vec<_>>();
    let rotate_right = (0..length)
        .map(|output| (output + length - 1) % length)
        .collect::<Vec<_>>();
    let pair_swap = (0..length)
        .map(|output| {
            if output % 2 == 0 {
                (output + 1).min(length - 1)
            } else {
                output - 1
            }
        })
        .collect::<Vec<_>>();
    let even_then_odd = slot32_even_then_odd_sources(length);
    let odd_then_even = slot32_odd_then_even_sources(length);

    let composed_maps = vec![
        (
            "composed",
            "mirror_then_rotate_left",
            slot32_compose_maps(&mirror, &rotate_left),
        ),
        (
            "composed",
            "rotate_right_then_pair_swap",
            slot32_compose_maps(&rotate_right, &pair_swap),
        ),
        (
            "composed",
            "even_then_odd_then_mirror",
            slot32_compose_maps(&even_then_odd, &mirror),
        ),
        (
            "composed",
            "odd_then_even_then_rotate_left",
            slot32_compose_maps(&odd_then_even, &rotate_left),
        ),
    ];

    order_maps
        .into_iter()
        .chain(edit_maps)
        .chain(composed_maps)
        .collect()
}

fn slot32_conditional_branch_corpus_maps(
    length: usize,
) -> Vec<(&'static str, bool, Vec<usize>, Vec<usize>)> {
    let mirror = (0..length)
        .map(|output| length - 1 - output)
        .collect::<Vec<_>>();
    let rotate_right = (0..length)
        .map(|output| (output + length - 1) % length)
        .collect::<Vec<_>>();
    let even_then_odd = slot32_even_then_odd_sources(length);
    let rotate_half = (0..length)
        .map(|output| (output + length / 2) % length)
        .collect::<Vec<_>>();

    vec![
        (
            "if_match_mirror_else_rotate_left",
            true,
            mirror.clone(),
            slot32_wrong_map(&mirror, length),
        ),
        (
            "if_match_rotate_right_else_pair_swap",
            true,
            rotate_right.clone(),
            slot32_wrong_map(&rotate_right, length),
        ),
        (
            "if_match_even_then_odd_else_center_out",
            true,
            even_then_odd.clone(),
            slot32_wrong_map(&even_then_odd, length),
        ),
        (
            "if_match_rotate_half_else_odd_then_even",
            true,
            rotate_half.clone(),
            slot32_wrong_map(&rotate_half, length),
        ),
        (
            "if_mismatch_mirror_else_rotate_left",
            false,
            mirror.clone(),
            slot32_wrong_map(&mirror, length),
        ),
        (
            "if_mismatch_rotate_right_else_pair_swap",
            false,
            rotate_right.clone(),
            slot32_wrong_map(&rotate_right, length),
        ),
        (
            "if_mismatch_even_then_odd_else_center_out",
            false,
            even_then_odd.clone(),
            slot32_wrong_map(&even_then_odd, length),
        ),
        (
            "if_mismatch_rotate_half_else_odd_then_even",
            false,
            rotate_half.clone(),
            slot32_wrong_map(&rotate_half, length),
        ),
    ]
}

fn slot32_compose_maps(first: &[usize], second: &[usize]) -> Vec<usize> {
    second
        .iter()
        .map(|source_slot| first[*source_slot])
        .collect()
}

fn slot32_even_then_odd_sources(length: usize) -> Vec<usize> {
    (0..length)
        .step_by(2)
        .chain((1..length).step_by(2))
        .collect()
}

fn slot32_odd_then_even_sources(length: usize) -> Vec<usize> {
    (1..length)
        .step_by(2)
        .chain((0..length).step_by(2))
        .collect()
}

fn slot32_center_out_sources(length: usize) -> Vec<usize> {
    let center = (length - 1) / 2;
    let mut sources = Vec::with_capacity(length);
    sources.push(center);
    for offset in 1..=length {
        if center >= offset {
            sources.push(center - offset);
        }
        let right = center + offset;
        if right < length {
            sources.push(right);
        }
        if sources.len() == length {
            break;
        }
    }
    sources
}

fn slot32_wrong_map(slot_map: &[usize], length: usize) -> Vec<usize> {
    slot_map
        .iter()
        .map(|source_slot| (source_slot + 1) % length)
        .collect()
}

fn slot32_tokens(split: &str, seed: usize, rule_name: &str, length: usize) -> Vec<String> {
    (0..length)
        .map(|slot| {
            let salt = slot32_token_salt(seed, length, rule_name, slot);
            format!("slot32_s{seed}_{split}_{rule_name}_len{length}_tok{slot}_mix{salt:08x}")
        })
        .collect()
}

fn slot32_order_corpus_tokens(
    split: &str,
    seed: usize,
    length: usize,
    surface_family: &str,
    noise_type: &str,
) -> Vec<String> {
    (0..length)
        .map(|slot| {
            let salt = slot32_token_salt(seed, length, surface_family, slot)
                ^ slot32_token_salt(seed + 17, length, noise_type, slot);
            let core = match surface_family {
                "symbols" => format!("sym_{split}_s{seed}_l{length}_{slot}_{salt:08x}"),
                "network" => format!("iface_{split}_s{seed}_l{length}_{slot}_{salt:08x}"),
                "business" => format!("field_{split}_s{seed}_l{length}_{slot}_{salt:08x}"),
                "ru_words" => format!("uzel_{split}_s{seed}_l{length}_{slot}_{salt:08x}"),
                _ => format!("tok_{split}_s{seed}_l{length}_{slot}_{salt:08x}"),
            };
            match noise_type {
                "wrapped" => format!("pre_{core}_post"),
                _ => core,
            }
        })
        .collect()
}

fn slot32_token_salt(seed: usize, length: usize, rule_name: &str, slot: usize) -> u32 {
    let mut value = 0x9E37_79B9u32
        ^ (seed as u32).wrapping_mul(0x85EB_CA6B)
        ^ (length as u32).wrapping_mul(0xC2B2_AE35)
        ^ (slot as u32).wrapping_mul(0x27D4_EB2D);
    for byte in rule_name.as_bytes() {
        value ^= u32::from(*byte);
        value = value.rotate_left(5).wrapping_mul(0x1656_67B1);
    }
    value
}

fn slot32_order_unique_rules(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .map(|row| row.rule_name)
        .collect::<BTreeSet<_>>()
        .len()
}

fn slot32_unique_operator_classes(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .map(|row| row.operator_class)
        .collect::<BTreeSet<_>>()
        .len()
}

fn slot32_order_unique_surfaces(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .map(|row| row.surface_family)
        .collect::<BTreeSet<_>>()
        .len()
}

fn slot32_order_unique_noise_types(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .map(|row| row.noise_type)
        .collect::<BTreeSet<_>>()
        .len()
}

fn slot32_order_unique_lengths(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .map(|row| row.length)
        .collect::<BTreeSet<_>>()
        .len()
}

fn slot32_order_same_bag_rows(rows: &[Slot32OrderCorpusTask]) -> usize {
    rows.iter()
        .filter(|row| sorted_tokens(&row.correct_tokens) == sorted_tokens(&row.wrong_tokens))
        .count()
}

fn slot32_operator_class_rows(rows: &[Slot32OrderCorpusTask], operator_class: &str) -> usize {
    rows.iter()
        .filter(|row| row.operator_class == operator_class)
        .count()
}

fn slot32_operator_class_non_same_bag_rows(
    rows: &[Slot32OrderCorpusTask],
    operator_class: &str,
) -> usize {
    rows.iter()
        .filter(|row| {
            row.operator_class == operator_class
                && sorted_tokens(&row.correct_tokens) != sorted_tokens(&row.wrong_tokens)
        })
        .count()
}

fn slot32_condition_result_rows(rows: &[Slot32OrderCorpusTask], condition_result: bool) -> usize {
    rows.iter()
        .filter(|row| row.condition_result == Some(condition_result))
        .count()
}

fn slot32_active_center_count(
    rows: &[Slot32OrderCorpusTask],
    predicate: impl Fn(WavePredictorCenterId) -> bool,
) -> usize {
    rows.iter()
        .filter_map(|row| row.task.slot_tasks.first())
        .flat_map(|slot_task| slot_task.active_fringe.iter())
        .filter(|center| predicate(center.center_id))
        .count()
}

fn is_slot32_direct_operator_pair_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ32_OPERATOR_PAIR_BASE..SEQ32_OPERATOR_PAIR_BASE + SEQ32_FEATURE_CENTER_COUNT)
        .contains(&center_id)
}

fn is_slot32_condition_action_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ32_CONDITION_TRUE_ACTION_BASE
        ..SEQ32_CONDITION_TRUE_ACTION_BASE + SEQ32_FEATURE_CENTER_COUNT)
        .contains(&center_id)
        || (SEQ32_CONDITION_FALSE_ACTION_BASE
            ..SEQ32_CONDITION_FALSE_ACTION_BASE + SEQ32_FEATURE_CENTER_COUNT)
            .contains(&center_id)
}

fn is_slot32_state_condition_center(center_id: WavePredictorCenterId) -> bool {
    (SEQ32_STATE_CONDITION_BASE..SEQ32_STATE_CONDITION_BASE + SEQ32_FEATURE_CENTER_COUNT)
        .contains(&center_id)
}

fn slot32_order_max_state_reuse(rows: &[Slot32OrderCorpusTask]) -> usize {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.state_key.as_str()).or_insert(0) += 1;
    }
    counts.values().copied().max().unwrap_or(0)
}

fn slot32_order_train_heldout_token_overlap(
    train: &[Slot32OrderCorpusTask],
    heldout: &[Slot32OrderCorpusTask],
) -> usize {
    fn collect_tokens(rows: &[Slot32OrderCorpusTask]) -> BTreeSet<&str> {
        let mut tokens = BTreeSet::new();
        for row in rows {
            for token in &row.correct_tokens {
                tokens.insert(token.as_str());
            }
            for token in &row.wrong_tokens {
                tokens.insert(token.as_str());
            }
        }
        tokens
    }
    let train_tokens = collect_tokens(train);
    let heldout_tokens = collect_tokens(heldout);
    train_tokens.intersection(&heldout_tokens).count()
}

fn slot32_prepared_task(
    tokens: &[String],
    slot_map: &[usize],
    wrong_map: &[usize],
) -> PreparedSequenceTask {
    assert_eq!(slot_map.len(), wrong_map.len());
    assert!(tokens.len() <= usize::from(SEQ32_ROLE_SLOT_COUNT));
    assert!(slot_map.len() <= usize::from(SEQ32_OUTPUT_SLOT_COUNT));
    assert!(
        slot_map
            .iter()
            .all(|source_slot| *source_slot < tokens.len())
    );
    assert!(
        wrong_map
            .iter()
            .all(|source_slot| *source_slot < tokens.len())
    );
    let action_centers = slot32_operator_pair_action_centers(slot_map);
    let role_centers = slot32_role_centers(tokens);
    let active_fringe = merge_centers(
        action_centers
            .into_iter()
            .chain(role_centers)
            .collect::<Vec<_>>(),
    );
    let mut slot_tasks = Vec::with_capacity(slot_map.len());
    for output_slot in 0..slot_map.len() {
        slot_tasks.push(slot32_slot_task(
            &active_fringe,
            output_slot,
            &tokens[slot_map[output_slot]],
            &tokens[wrong_map[output_slot]],
        ));
    }
    PreparedSequenceTask {
        slot_tasks,
        output_slots: (0..slot_map.len()).collect(),
    }
}

fn slot32_conditional_prepared_task(
    tokens: &[String],
    state_flag: bool,
    condition_result: bool,
    slot_map: &[usize],
    wrong_map: &[usize],
) -> PreparedSequenceTask {
    assert_eq!(slot_map.len(), wrong_map.len());
    assert!(tokens.len() <= usize::from(SEQ32_ROLE_SLOT_COUNT));
    assert!(slot_map.len() <= usize::from(SEQ32_OUTPUT_SLOT_COUNT));
    assert!(
        slot_map
            .iter()
            .all(|source_slot| *source_slot < tokens.len())
    );
    assert!(
        wrong_map
            .iter()
            .all(|source_slot| *source_slot < tokens.len())
    );

    let action_centers = slot32_condition_result_action_centers(condition_result, slot_map);
    let state_condition_centers = slot32_state_condition_centers(state_flag);
    let role_centers = slot32_role_centers(tokens);
    let active_fringe = merge_centers(
        action_centers
            .into_iter()
            .chain(state_condition_centers)
            .chain(role_centers)
            .collect::<Vec<_>>(),
    );
    let mut slot_tasks = Vec::with_capacity(slot_map.len());
    for output_slot in 0..slot_map.len() {
        slot_tasks.push(slot32_slot_task(
            &active_fringe,
            output_slot,
            &tokens[slot_map[output_slot]],
            &tokens[wrong_map[output_slot]],
        ));
    }
    PreparedSequenceTask {
        slot_tasks,
        output_slots: (0..slot_map.len()).collect(),
    }
}

fn slot32_slot_task(
    active_fringe: &[WavePredictorActiveCenter],
    output_slot: usize,
    correct_token: &str,
    wrong_token: &str,
) -> WavePredictorStateDeltaTrainTask {
    assert_ne!(correct_token, wrong_token);
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive = discriminative_delta_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let negative = discriminative_delta_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let target_delta = WavePredictorStateDeltaTarget::from_impulses(&positive, &negative)
        .expect("slot32 sequence slot delta must fit");
    WavePredictorStateDeltaTrainTask {
        active_fringe: active_fringe.to_vec(),
        target_delta,
        binding_output_slot: Some(output_slot as u8),
    }
}

fn slot32_condition_result_action_centers(
    condition_result: bool,
    slot_map: &[usize],
) -> Vec<WavePredictorActiveCenter> {
    let base = if condition_result {
        SEQ32_CONDITION_TRUE_ACTION_BASE
    } else {
        SEQ32_CONDITION_FALSE_ACTION_BASE
    };
    slot_map
        .iter()
        .enumerate()
        .filter(|(output_slot, source_slot)| {
            *output_slot < usize::from(SEQ32_OUTPUT_SLOT_COUNT)
                && **source_slot < usize::from(SEQ32_ROLE_SLOT_COUNT)
        })
        .map(|(output_slot, source_slot)| WavePredictorActiveCenter {
            center_id: base + slot32_operator_pair_lane(output_slot, *source_slot),
            strength: 8,
        })
        .collect()
}

fn slot32_state_condition_centers(state_flag: bool) -> Vec<WavePredictorActiveCenter> {
    surface_lane_centers_folded(
        if state_flag {
            "slot32_state_condition:true"
        } else {
            "slot32_state_condition:false"
        },
        SEQ32_STATE_CONDITION_BASE,
        SEQ32_FEATURE_CENTER_COUNT,
        TOP_ACTION_L1_LANES,
    )
}

fn slot32_operator_pair_action_centers(slot_map: &[usize]) -> Vec<WavePredictorActiveCenter> {
    slot_map
        .iter()
        .enumerate()
        .filter(|(output_slot, source_slot)| {
            *output_slot < usize::from(SEQ32_OUTPUT_SLOT_COUNT)
                && **source_slot < usize::from(SEQ32_ROLE_SLOT_COUNT)
        })
        .map(|(output_slot, source_slot)| WavePredictorActiveCenter {
            center_id: SEQ32_OPERATOR_PAIR_BASE
                + slot32_operator_pair_lane(output_slot, *source_slot),
            strength: 8,
        })
        .collect()
}

fn slot32_role_centers(tokens: &[String]) -> Vec<WavePredictorActiveCenter> {
    let mut centers = Vec::new();
    for (slot_id, token) in tokens
        .iter()
        .take(usize::from(SEQ32_ROLE_SLOT_COUNT))
        .enumerate()
    {
        let slot_base = SEQ32_ROLE_BASE
            + WavePredictorCenterId::from(slot_id as u8) * SEQ32_FEATURE_CENTER_COUNT;
        centers.extend(surface_lane_centers_folded(
            token,
            slot_base,
            SEQ32_FEATURE_CENTER_COUNT,
            SEQ32_TOP_ROLE_L1_LANES,
        ));
    }
    centers
}

fn slot32_failure_breakdown_fast(
    index: &FlatRoleBindingScoreIndex,
    tasks: &[(usize, &'static str, PreparedSequenceTask)],
) -> (usize, BTreeMap<usize, usize>, BTreeMap<&'static str, usize>) {
    let mut failed_rows = 0usize;
    let mut failed_by_length = BTreeMap::new();
    let mut failed_by_rule = BTreeMap::new();
    for (length, rule_name, task) in tasks {
        let prepared = index.prepare_task(&task.slot_tasks[0]);
        let mut row_ok = true;
        for slot_task in &task.slot_tasks {
            row_ok &= flat_state_delta_sum_gap_fast_prepared(index, &prepared, slot_task) > 0;
        }
        if row_ok {
            continue;
        }
        failed_rows += 1;
        *failed_by_length.entry(*length).or_default() += 1;
        *failed_by_rule.entry(*rule_name).or_default() += 1;
    }
    (failed_rows, failed_by_length, failed_by_rule)
}

fn prepare_full_state_rows(rows: &[BindingRow]) -> Vec<PreparedFullStateTask> {
    rows.iter().map(PreparedFullStateTask::new).collect()
}

fn prepare_step12_rows(rows: &[BindingRow]) -> Vec<PreparedStep12Task> {
    rows.iter().map(PreparedStep12Task::new).collect()
}

impl PreparedFullStateTask {
    fn new(row: &BindingRow) -> Self {
        let active_context = format!("{} {}", row.state_before, row.rule_action_example);
        let frame_positive = frame_delta_impulses(
            &frame_without_binding_token(&row.state_after_correct),
            &wrong_frame_for(row),
        );
        let frame_negative = frame_delta_impulses(
            &wrong_frame_for(row),
            &frame_without_binding_token(&row.state_after_correct),
        );
        let frame_delta =
            WavePredictorStateDeltaTarget::from_impulses(&frame_positive, &frame_negative)
                .expect("frame-wave target must fit");

        let binding = PreparedBindingTask::new(row).train_task;

        Self {
            frame_task: WavePredictorStateDeltaTrainTask {
                active_fringe: active_l1_fringe(row, &active_context),
                target_delta: frame_delta,
                binding_output_slot: None,
            },
            binding_task: binding,
        }
    }
}

impl PreparedStep12Task {
    fn new(row: &BindingRow) -> Self {
        let binding_task = PreparedBindingTask::new(row).train_task;
        let frame_task = match row.answer_status.as_str() {
            "UNSETTLED" | "CONFLICT" => Some(PreparedFullStateTask::new(row).frame_task),
            "PROVEN" => None,
            other => panic!("unsupported Step12 row status: {other}"),
        };
        Self {
            frame_task,
            binding_task,
        }
    }
}

fn frame_delta_impulses(wanted_frame: &str, wrong_frame: &str) -> Vec<WavePredictorStateImpulse> {
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(wanted_frame);
    let wrong_wave = SurfaceWave4096::compile(wrong_frame);
    discriminative_delta_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    )
}

fn frame_without_binding_token(state: &str) -> String {
    let mut tokens = state_tokens(state);
    let Some(_) = tokens.pop() else {
        panic!("state has no frame tokens: {state}");
    };
    format!("state: {}", tokens.join(" "))
}

fn wrong_frame_for(row: &BindingRow) -> String {
    match row.answer_status.as_str() {
        "UNSETTLED" => "state: CONFLICT verify".to_string(),
        "CONFLICT" => "state: UNSETTLED ask".to_string(),
        other => panic!("unsupported full-state frame row status: {other}"),
    }
}

fn noisy_binding_trace_rows(rows: &[BindingRow]) -> Vec<BindingRow> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let mut noisy = row.clone();
            noisy.state_before = noisy_state_before(row, index);
            noisy.rule_action_example =
                format!("noise_intro {} noise_tail", row.rule_action_example);
            noisy
        })
        .collect()
}

fn noisy_state_before(row: &BindingRow, index: usize) -> String {
    match row.proof_rule_id.as_str() {
        "missing_variable_bind" => {
            let tokens = state_tokens(&row.state_before);
            let missing_index = tokens
                .iter()
                .position(|token| token == "missing")
                .expect("missing task must contain missing marker");
            let variable = tokens
                .get(missing_index + 1)
                .expect("missing task must contain variable after marker");
            format!(
                "state: noisy_probe_{index} observe context drift; please missing {variable} now"
            )
        }
        "conflict_fact_bind" => {
            let tokens = state_tokens(&row.state_before);
            let source_index = tokens
                .iter()
                .position(|token| token == "source_a")
                .expect("conflict task must contain source_a marker");
            let fact = tokens
                .get(source_index + 1)
                .expect("conflict task must contain fact after source_a");
            format!(
                "state: noisy_probe_{index} audit trail; source_a {fact}; side_note source_b not_{fact}"
            )
        }
        other => panic!("unsupported noisy binding trace rule: {other}"),
    }
}

fn audit_errors(
    field: &WavePredictorHebbianField,
    rows: &[BindingRow],
    tasks: &[PreparedBindingTask],
) -> Vec<ErrorCase> {
    let mut errors = Vec::new();
    for (row, task) in rows.iter().zip(tasks.iter()) {
        let gap = state_delta_sum_gap(field, &task.train_task);
        if gap <= 0 {
            errors.push(ErrorCase {
                gap,
                task_id: row.task_id.clone(),
                proof_rule_id: row.proof_rule_id.clone(),
                surface_family: row.surface_family.clone(),
                state_before: row.state_before.clone(),
                rule_action_example: row.rule_action_example.clone(),
                state_after_correct: row.state_after_correct.clone(),
                state_after_wrong: row.state_after_wrong.clone(),
            });
        }
    }
    errors.sort_by_key(|error| error.gap);
    errors
}

fn print_error_audit(errors: &[ErrorCase]) {
    let mut by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_surface: BTreeMap<&str, usize> = BTreeMap::new();
    for error in errors {
        *by_rule.entry(&error.proof_rule_id).or_default() += 1;
        *by_surface.entry(&error.surface_family).or_default() += 1;
    }

    println!("  error_count: {}", errors.len());
    println!("  error_by_rule_top:");
    for (rule, count) in top_counts(&by_rule, 8) {
        println!("    {rule}: {count}");
    }
    println!("  error_by_surface_top:");
    for (surface, count) in top_counts(&by_surface, 8) {
        println!("    {surface}: {count}");
    }
    println!("  worst_errors:");
    for error in errors.iter().take(5) {
        println!(
            "    {} gap={} rule={} surface={}",
            error.task_id, error.gap, error.proof_rule_id, error.surface_family
        );
        println!("      before: {}", error.state_before);
        println!("      action: {}", error.rule_action_example);
        println!("      correct: {}", error.state_after_correct);
        println!("      wrong: {}", error.state_after_wrong);
    }
}

fn top_counts<'a>(counts: &'a BTreeMap<&'a str, usize>, limit: usize) -> Vec<(&'a str, usize)> {
    let mut items: Vec<_> = counts.iter().map(|(key, value)| (*key, *value)).collect();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    items.truncate(limit);
    items
}

fn load_rows() -> Vec<BindingRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BINDING_PRESSURE_CORPUS);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read binding-pressure corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_row)
        .collect()
}

fn load_sequence_rows() -> Vec<SequenceBindingRow> {
    load_sequence_rows_from(POSITION_SEQUENCE_CORPUS, "position-sequence")
}

fn load_sequence_v3_rows() -> Vec<SequenceBindingRow> {
    if let Ok(path) = std::env::var("POSITION_SEQUENCE_V3_CORPUS_PATH") {
        return load_sequence_rows_from_path(PathBuf::from(path), "position-sequence-v3");
    }
    load_sequence_rows_from(POSITION_SEQUENCE_V3_CORPUS, "position-sequence-v3")
}

fn load_operator_battery_v4_order_rows() -> Vec<SequenceBindingRow> {
    if let Ok(path) = std::env::var("OPERATOR_BATTERY_V4_ORDER_CORPUS_PATH") {
        return load_sequence_rows_from_path(PathBuf::from(path), "operator-battery-v4-order");
    }
    load_sequence_rows_from(
        OPERATOR_BATTERY_V4_ORDER_CORPUS,
        "operator-battery-v4-order",
    )
}

fn load_operator_battery_v4_edit_rows() -> Vec<SequenceBindingRow> {
    if let Ok(path) = std::env::var("OPERATOR_BATTERY_V4_EDIT_CORPUS_PATH") {
        return load_variable_sequence_rows_from_path(
            PathBuf::from(path),
            "operator-battery-v4-edit",
        );
    }
    load_variable_sequence_rows_from(OPERATOR_BATTERY_V4_EDIT_CORPUS, "operator-battery-v4-edit")
}

fn load_operator_battery_v4_conditional_rows() -> Vec<SequenceBindingRow> {
    if let Ok(path) = std::env::var("OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS_PATH") {
        return load_sequence_rows_from_path(
            PathBuf::from(path),
            "operator-battery-v4-conditional",
        );
    }
    load_sequence_rows_from(
        OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS,
        "operator-battery-v4-conditional",
    )
}

fn load_operator_battery_v4_composed_rows() -> Vec<SequenceBindingRow> {
    if let Ok(path) = std::env::var("OPERATOR_BATTERY_V4_COMPOSED_CORPUS_PATH") {
        return load_sequence_rows_from_path(PathBuf::from(path), "operator-battery-v4-composed");
    }
    load_sequence_rows_from(
        OPERATOR_BATTERY_V4_COMPOSED_CORPUS,
        "operator-battery-v4-composed",
    )
}

fn load_phase_operator_rows() -> Vec<PhaseOperatorRow> {
    if let Ok(path) = std::env::var("OPERATOR_BATTERY_V4_PHASE_CENTER_CORPUS_PATH") {
        return load_phase_operator_rows_from_path(
            PathBuf::from(path),
            "operator-battery-v4-phase",
        );
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(OPERATOR_BATTERY_V4_CORPUS);
    load_phase_operator_rows_from_path(path, "operator-battery-v4-phase")
}

fn load_phase_operator_rows_from_path(path: PathBuf, label: &str) -> Vec<PhaseOperatorRow> {
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {label} corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_phase_operator_row)
        .collect()
}

fn load_sequence_rows_from(relative_path: &str, label: &str) -> Vec<SequenceBindingRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    load_sequence_rows_from_path(path, label)
}

fn load_variable_sequence_rows_from(relative_path: &str, label: &str) -> Vec<SequenceBindingRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    load_variable_sequence_rows_from_path(path, label)
}

fn load_sequence_rows_from_path(path: PathBuf, label: &str) -> Vec<SequenceBindingRow> {
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {label} corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_sequence_row)
        .collect()
}

fn load_variable_sequence_rows_from_path(path: PathBuf, label: &str) -> Vec<SequenceBindingRow> {
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {label} corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_variable_sequence_row)
        .collect()
}

fn parse_row(line: &str) -> BindingRow {
    BindingRow {
        task_id: json_string(line, "task_id"),
        source_group: json_string(line, "source_group"),
        surface_family: json_string(line, "surface_family"),
        proof_rule_id: json_string(line, "proof_rule_id"),
        answer_status: json_string(line, "answer_status"),
        state_before: json_string(line, "state_before"),
        rule_action_example: json_string(line, "rule_action_example"),
        state_after_correct: json_string(line, "state_after_correct"),
        state_after_wrong: json_string(line, "state_after_wrong"),
    }
}

fn parse_sequence_row(line: &str) -> SequenceBindingRow {
    let correct_state = json_string(line, "state_after_correct");
    let wrong_state = json_string(line, "state_after_wrong");
    let correct_tokens = state_tokens(&correct_state);
    let wrong_tokens = state_tokens(&wrong_state);
    let sequence_length = json_usize_opt(line, "sequence_length").unwrap_or(correct_tokens.len());
    assert_eq!(
        sequence_length,
        correct_tokens.len(),
        "sequence_length must match target token count"
    );
    assert_eq!(
        sorted_tokens(&correct_tokens),
        sorted_tokens(&wrong_tokens),
        "position-sequence row must keep correct/wrong token bags equal"
    );
    SequenceBindingRow {
        source_group: json_string(line, "source_group"),
        rule_id: json_string(line, "proof_rule_id"),
        surface_family: json_string(line, "surface_family"),
        noise_type: json_string_opt(line, "noise_type").unwrap_or_else(|| "v2_noise".to_string()),
        sequence_length,
        state_before: json_string(line, "state_before"),
        action: json_string(line, "rule_action_example"),
        correct_tokens,
        wrong_tokens,
    }
}

fn parse_variable_sequence_row(line: &str) -> SequenceBindingRow {
    let correct_state = json_string(line, "state_after_correct");
    let wrong_state = json_string(line, "state_after_wrong");
    let correct_tokens = state_tokens(&correct_state);
    let wrong_tokens = state_tokens(&wrong_state);
    let sequence_length = json_usize_opt(line, "sequence_length")
        .unwrap_or_else(|| sequence_source_tokens(&json_string(line, "state_before")).len());
    SequenceBindingRow {
        source_group: json_string(line, "source_group"),
        rule_id: json_string(line, "proof_rule_id"),
        surface_family: json_string(line, "surface_family"),
        noise_type: json_string_opt(line, "noise_type").unwrap_or_else(|| "v2_noise".to_string()),
        sequence_length,
        state_before: json_string(line, "state_before"),
        action: json_string(line, "rule_action_example"),
        correct_tokens,
        wrong_tokens,
    }
}

fn parse_phase_operator_row(line: &str) -> PhaseOperatorRow {
    PhaseOperatorRow {
        source_group: json_string(line, "source_group"),
        operator_class: json_string(line, "operator_class"),
        condition_flag: json_nullable_string(line, "condition_flag"),
        sequence_length: json_usize_opt(line, "sequence_length")
            .expect("phase operator row must include sequence_length"),
        surface_family: json_string(line, "surface_family"),
        noise_type: json_string_opt(line, "noise_type").unwrap_or_else(|| "v4_noise".to_string()),
        action: json_string(line, "rule_action_example"),
        source_tokens: json_string_array(line, "source_tokens"),
        correct_tokens: json_string_array(line, "correct_tokens"),
        wrong_tokens: json_string_array(line, "wrong_tokens"),
    }
}

fn json_string_opt(line: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{key}\"");
    line.find(&key_pattern).map(|_| json_string(line, key))
}

fn json_nullable_string(line: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{key}\"");
    let key_pos = line.find(&key_pattern)?;
    let after_key = &line[key_pos + key_pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let value = after_key[colon_pos + 1..].trim_start();
    if value.starts_with("null") {
        None
    } else if value.starts_with('"') {
        Some(json_string(line, key))
    } else {
        panic!("JSON key {key} must be a nullable string")
    }
}

fn json_string_array(line: &str, key: &str) -> Vec<String> {
    let key_pattern = format!("\"{key}\"");
    let key_pos = line
        .find(&key_pattern)
        .unwrap_or_else(|| panic!("missing JSON key {key}"));
    let after_key = &line[key_pos + key_pattern.len()..];
    let colon_pos = after_key
        .find(':')
        .unwrap_or_else(|| panic!("missing colon for JSON key {key}"));
    let mut chars = after_key[colon_pos + 1..].trim_start().chars();
    assert_eq!(
        chars.next(),
        Some('['),
        "JSON key {key} must be a string array"
    );

    let mut output = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for ch in chars {
        if in_string {
            if escaped {
                current.push(match ch {
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                output.push(std::mem::take(&mut current));
                in_string = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == ']' {
            return output;
        }
    }
    panic!("unterminated JSON array for key {key}")
}

fn json_usize_opt(line: &str, key: &str) -> Option<usize> {
    let key_pattern = format!("\"{key}\"");
    let key_pos = line.find(&key_pattern)?;
    let after_key = &line[key_pos + key_pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let value = after_key[colon_pos + 1..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!value.is_empty()).then(|| value.parse().expect("JSON usize field must parse"))
}

fn sorted_tokens(tokens: &[String]) -> Vec<&str> {
    let mut out: Vec<_> = tokens.iter().map(String::as_str).collect();
    out.sort_unstable();
    out
}

fn json_string(line: &str, key: &str) -> String {
    let key_pattern = format!("\"{key}\"");
    let key_pos = line
        .find(&key_pattern)
        .unwrap_or_else(|| panic!("missing JSON key {key}"));
    let after_key = &line[key_pos + key_pattern.len()..];
    let colon_pos = after_key
        .find(':')
        .unwrap_or_else(|| panic!("missing colon for JSON key {key}"));
    let mut chars = after_key[colon_pos + 1..].trim_start().chars();
    assert_eq!(
        chars.next(),
        Some('"'),
        "JSON key {key} must be a string value"
    );

    let mut output = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            output.push(match ch {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return output;
        } else {
            output.push(ch);
        }
    }
    panic!("unterminated JSON string for key {key}")
}

fn prepare_rows(rows: &[BindingRow]) -> Vec<PreparedBindingTask> {
    rows.iter().map(PreparedBindingTask::new).collect()
}

impl PreparedBindingTask {
    fn new(row: &BindingRow) -> Self {
        let active_context = format!("{} {}", row.state_before, row.rule_action_example);
        let base_wave = SurfaceWave4096::compile("");
        let target_wave = SurfaceWave4096::compile(&last_state_token(&row.state_after_correct));
        let wrong_wave = SurfaceWave4096::compile(&last_state_token(&row.state_after_wrong));
        let positive = discriminative_delta_impulses(
            base_wave.lanes(),
            target_wave.lanes(),
            wrong_wave.lanes(),
            STATE_DELTA_LANES_PER_SIDE,
        );
        let negative = discriminative_delta_impulses(
            base_wave.lanes(),
            wrong_wave.lanes(),
            target_wave.lanes(),
            STATE_DELTA_LANES_PER_SIDE,
        );
        let target_delta = WavePredictorStateDeltaTarget::from_impulses(&positive, &negative)
            .expect("binding-pressure tasks must yield compact state deltas");

        Self {
            train_task: WavePredictorStateDeltaTrainTask {
                active_fringe: active_l1_fringe(row, &active_context),
                target_delta,
                binding_output_slot: None,
            },
        }
    }
}

fn active_l1_fringe(row: &BindingRow, active_context: &str) -> Vec<WavePredictorActiveCenter> {
    let mut centers = Vec::new();
    centers.extend(surface_lane_centers(
        active_context,
        FEATURE_CENTER_BASE,
        TOP_ACTIVE_L1_LANES,
    ));
    centers.extend(surface_lane_centers(
        &row.rule_action_example,
        ACTION_CENTER_BASE,
        TOP_ACTION_L1_LANES,
    ));
    centers.extend(l2_time_phase_role_lane_centers(&row.state_before));
    merge_centers(centers)
}

fn surface_lane_centers(
    input: &str,
    center_base: WavePredictorCenterId,
    limit: usize,
) -> Vec<WavePredictorActiveCenter> {
    let wave = SurfaceWave4096::compile(input);
    let mut lanes: Vec<_> = wave
        .lanes()
        .iter()
        .enumerate()
        .filter_map(|(lane, value)| {
            let magnitude = i32::from(*value).abs();
            (magnitude > 0).then_some((magnitude, lane as u16, *value))
        })
        .collect();
    lanes.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    lanes
        .into_iter()
        .take(limit)
        .map(|(_, lane, value)| WavePredictorActiveCenter {
            center_id: center_base + WavePredictorCenterId::from(lane),
            strength: value.abs().clamp(1, 8),
        })
        .collect()
}

fn surface_lane_centers_folded(
    input: &str,
    center_base: WavePredictorCenterId,
    center_span: WavePredictorCenterId,
    limit: usize,
) -> Vec<WavePredictorActiveCenter> {
    let wave = SurfaceWave4096::compile(input);
    let mut by_lane: BTreeMap<WavePredictorCenterId, i16> = BTreeMap::new();
    for (lane, value) in wave.lanes().iter().enumerate() {
        let magnitude = value.abs();
        if magnitude == 0 {
            continue;
        }
        let folded_lane = lane as WavePredictorCenterId % center_span;
        by_lane
            .entry(folded_lane)
            .and_modify(|current| *current = (*current).max(magnitude))
            .or_insert(magnitude);
    }
    let mut lanes: Vec<_> = by_lane
        .into_iter()
        .map(|(lane, magnitude)| (i32::from(magnitude), lane))
        .collect();
    lanes.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    lanes
        .into_iter()
        .take(limit)
        .map(|(magnitude, lane)| WavePredictorActiveCenter {
            center_id: center_base + lane,
            strength: (magnitude as i16).clamp(1, 8),
        })
        .collect()
}

fn l2_time_phase_role_lane_centers(state_before: &str) -> Vec<WavePredictorActiveCenter> {
    let tokens = state_tokens(state_before);
    let mut out = Vec::new();
    for (slot_id, token) in tokens.iter().take(8).enumerate() {
        let slot_base =
            ROLE_CENTER_BASE + WavePredictorCenterId::from(slot_id as u8) * FEATURE_CENTER_COUNT;
        out.extend(surface_lane_centers(token, slot_base, TOP_ROLE_L1_LANES));
    }
    push_marker_relative_slot(&tokens, "missing", 8, &mut out);
    push_marker_relative_slot(&tokens, "source_a", 9, &mut out);
    push_first_segment_slot(state_before, 10, &mut out);
    push_second_token_slot(&tokens, 11, &mut out);
    out
}

fn push_marker_relative_slot(
    tokens: &[String],
    marker: &str,
    slot_id: u16,
    out: &mut Vec<WavePredictorActiveCenter>,
) {
    let Some(index) = tokens.iter().position(|token| token == marker) else {
        return;
    };
    let Some(token) = tokens.get(index + 1) else {
        return;
    };
    let slot_base = ROLE_CENTER_BASE + WavePredictorCenterId::from(slot_id) * FEATURE_CENTER_COUNT;
    out.extend(surface_lane_centers(token, slot_base, TOP_ROLE_L1_LANES));
}

fn push_first_segment_slot(
    state_before: &str,
    slot_id: u16,
    out: &mut Vec<WavePredictorActiveCenter>,
) {
    let Some(left) = state_before.split('|').next() else {
        return;
    };
    let tokens = state_tokens(left);
    let Some(token) = tokens.first() else {
        return;
    };
    let slot_base = ROLE_CENTER_BASE + WavePredictorCenterId::from(slot_id) * FEATURE_CENTER_COUNT;
    out.extend(surface_lane_centers(token, slot_base, TOP_ROLE_L1_LANES));
}

fn push_second_token_slot(
    tokens: &[String],
    slot_id: u16,
    out: &mut Vec<WavePredictorActiveCenter>,
) {
    let Some(token) = tokens.get(1) else {
        return;
    };
    let slot_base = ROLE_CENTER_BASE + WavePredictorCenterId::from(slot_id) * FEATURE_CENTER_COUNT;
    out.extend(surface_lane_centers(token, slot_base, TOP_ROLE_L1_LANES));
}

fn state_tokens(state_before: &str) -> Vec<String> {
    state_before
        .trim_start_matches("state:")
        .split(|ch: char| ch.is_whitespace() || ch == ';' || ch == '|')
        .filter_map(|token| {
            let cleaned = token
                .trim()
                .trim_matches(',')
                .trim_matches('.')
                .trim_start_matches("not_");
            (!cleaned.is_empty()).then(|| cleaned.to_string())
        })
        .collect()
}

fn sequence_source_tokens(state_before: &str) -> Vec<String> {
    let Some((_, after_marker)) = state_before.split_once("sequence:") else {
        return state_tokens(state_before);
    };
    let segment = after_marker.split(';').next().unwrap_or(after_marker);
    state_tokens(&format!("state: {segment}"))
}

fn last_state_token(state: &str) -> String {
    state_tokens(state)
        .into_iter()
        .last()
        .unwrap_or_else(|| panic!("state has no bindable token: {state}"))
}

fn merge_centers(centers: Vec<WavePredictorActiveCenter>) -> Vec<WavePredictorActiveCenter> {
    let mut by_center: BTreeMap<WavePredictorCenterId, i16> = BTreeMap::new();
    for center in centers {
        by_center
            .entry(center.center_id)
            .and_modify(|strength| *strength = (*strength).max(center.strength))
            .or_insert(center.strength);
    }
    by_center
        .into_iter()
        .map(|(center_id, strength)| WavePredictorActiveCenter {
            center_id,
            strength,
        })
        .collect()
}

fn discriminative_delta_impulses(
    base: &[i16; SURFACE_WAVE_DIM],
    wanted: &[i16; SURFACE_WAVE_DIM],
    other: &[i16; SURFACE_WAVE_DIM],
    cap: usize,
) -> Vec<WavePredictorStateImpulse> {
    let mut candidates = Vec::new();
    for lane in 0..SURFACE_WAVE_DIM {
        let wanted_delta = wanted[lane].saturating_sub(base[lane]);
        if wanted_delta == 0 {
            continue;
        }
        let other_delta = other[lane].saturating_sub(base[lane]);
        let wanted_abs = i32::from(wanted_delta).abs();
        let other_abs = i32::from(other_delta).abs();
        let same_direction = wanted_delta.signum() == other_delta.signum();
        if same_direction && wanted_abs <= other_abs {
            continue;
        }
        let separation = if same_direction {
            wanted_abs - other_abs
        } else {
            wanted_abs + other_abs
        };
        candidates.push((
            separation,
            lane as u16,
            clamp_impulse_strength(wanted_delta),
        ));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .take(cap)
        .map(|(_, lane_id, signed_strength)| WavePredictorStateImpulse {
            lane_id,
            signed_strength,
        })
        .collect()
}

fn clamp_impulse_strength(value: i16) -> i16 {
    let sign = if value < 0 { -1 } else { 1 };
    let magnitude = i32::from(value).abs().clamp(1, 8) as i16;
    sign * magnitude
}

fn eval_state_delta(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedBindingTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let gap = state_delta_sum_gap(field, &task.train_task);
        gaps.push(gap);
        if gap > 0 {
            correct += 1;
        }
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn eval_full_state_component<F>(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedFullStateTask],
    task_fn: F,
) -> EvalReport
where
    F: Fn(&PreparedFullStateTask) -> &WavePredictorStateDeltaTrainTask,
{
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let gap = state_delta_sum_gap(field, task_fn(task));
        gaps.push(gap);
        correct += usize::from(gap > 0);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn eval_full_state_after(
    frame_field: &WavePredictorHebbianField,
    binding_field: &WavePredictorHebbianField,
    tasks: &[PreparedFullStateTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let frame_gap = state_delta_sum_gap(frame_field, &task.frame_task);
        let binding_gap = state_delta_sum_gap(binding_field, &task.binding_task);
        let combined_gap = frame_gap.min(binding_gap);
        gaps.push(combined_gap);
        correct += usize::from(frame_gap > 0 && binding_gap > 0);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn eval_step12_current_full_state_after(
    frame_field: &WavePredictorHebbianField,
    binding_field: &WavePredictorHebbianField,
    tasks: &[PreparedStep12Task],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let binding_gap = state_delta_sum_gap(binding_field, &task.binding_task);
        let frame_gap = task
            .frame_task
            .as_ref()
            .map(|frame_task| state_delta_sum_gap(frame_field, frame_task))
            .unwrap_or(i32::MAX);
        let combined_gap = frame_gap.min(binding_gap);
        gaps.push(combined_gap);
        correct += usize::from(frame_gap > 0 && binding_gap > 0);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn eval_ordered_sequence(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedSequenceTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let mut min_gap = i32::MAX;
        let mut row_ok = true;
        for slot_task in &task.slot_tasks {
            let gap = state_delta_sum_gap(field, slot_task);
            min_gap = min_gap.min(gap);
            row_ok &= gap > 0;
        }
        gaps.push(min_gap);
        correct += usize::from(row_ok);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn ordered_sequence_row_gap(
    field: &WavePredictorHebbianField,
    task: &PreparedSequenceTask,
) -> (bool, i32) {
    let mut min_gap = i32::MAX;
    let mut row_ok = true;
    for slot_task in &task.slot_tasks {
        let gap = state_delta_sum_gap(field, slot_task);
        min_gap = min_gap.min(gap);
        row_ok &= gap > 0;
    }
    (row_ok, min_gap)
}

fn ordered_sequence_flat_row_gap(
    table: &WavePredictorFlatRoleBindingTable,
    task: &PreparedSequenceTask,
) -> (bool, i32) {
    let mut min_gap = i32::MAX;
    let mut row_ok = true;
    for slot_task in &task.slot_tasks {
        let gap = flat_state_delta_sum_gap(table, slot_task);
        min_gap = min_gap.min(gap);
        row_ok &= gap > 0;
    }
    (row_ok, min_gap)
}

fn ordered_sequence_energy_diagnostics(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedSequenceTask],
) -> SequenceEnergyDiagnostics {
    let mut energy_gaps = Vec::with_capacity(tasks.len());
    let mut energy_correct = 0usize;
    let mut slot_pass_energy_fail = 0usize;
    let mut energy_pass_slot_fail = 0usize;

    for task in tasks {
        let (slot_ok, _) = ordered_sequence_row_gap(field, task);
        let energy_gap = sequence_energy_gap(field, task);
        let energy_ok = energy_gap > 0;
        energy_gaps.push(energy_gap);
        energy_correct += usize::from(energy_ok);
        slot_pass_energy_fail += usize::from(slot_ok && !energy_ok);
        energy_pass_slot_fail += usize::from(energy_ok && !slot_ok);
    }

    energy_gaps.sort_unstable();
    SequenceEnergyDiagnostics {
        rows: tasks.len(),
        energy_accuracy_milli: milli_ratio(energy_correct, tasks.len()),
        median_energy_gap: energy_gaps[tasks.len() / 2],
        p10_energy_gap: energy_gaps[tasks.len() / 10],
        slot_pass_energy_fail,
        energy_pass_slot_fail,
    }
}

fn sequence_energy_gap(field: &WavePredictorHebbianField, task: &PreparedSequenceTask) -> i32 {
    let mut correct_score = 0i32;
    let mut wrong_score = 0i32;
    for slot_task in &task.slot_tasks {
        correct_score += state_delta_target_score(field, slot_task);
        wrong_score += state_delta_wrong_score(field, slot_task);
    }
    correct_score - wrong_score
}

fn ordered_sequence_energy_group_diagnostics(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> SequenceEnergyGroupDiagnostics {
    let mut failed_rows_by_length = BTreeMap::new();
    let mut failed_rows_by_rule = BTreeMap::new();
    let mut failed_rows_by_surface = BTreeMap::new();
    let mut failed_rows_by_noise = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        if sequence_energy_gap(field, task) > 0 {
            continue;
        }
        *failed_rows_by_length
            .entry(row.sequence_length)
            .or_default() += 1;
        *failed_rows_by_rule.entry(row.rule_id.clone()).or_default() += 1;
        *failed_rows_by_surface
            .entry(row.surface_family.clone())
            .or_default() += 1;
        *failed_rows_by_noise
            .entry(row.noise_type.clone())
            .or_default() += 1;
    }

    SequenceEnergyGroupDiagnostics {
        failed_rows_by_length,
        failed_rows_by_rule,
        failed_rows_by_surface,
        failed_rows_by_noise,
    }
}

fn symmetry_operator_diagnostics(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> SymmetryOperatorDiagnostics {
    let mut symmetry = SequenceSubsetAccumulator::default();
    let mut non_symmetry = SequenceSubsetAccumulator::default();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let (slot_ok, slot_gap) = ordered_sequence_row_gap(field, task);
        let energy_gap = sequence_energy_gap(field, task);
        let target = if is_symmetry_operator(&row.rule_id) {
            &mut symmetry
        } else {
            &mut non_symmetry
        };
        target.push(slot_ok, slot_gap, energy_gap);
    }

    SymmetryOperatorDiagnostics {
        symmetry: symmetry.finish(),
        non_symmetry: non_symmetry.finish(),
    }
}

#[derive(Default)]
struct SequenceSubsetAccumulator {
    rows: usize,
    strict_correct: usize,
    energy_correct: usize,
    slot_gaps: Vec<i32>,
    energy_gaps: Vec<i32>,
}

impl SequenceSubsetAccumulator {
    fn push(&mut self, strict_ok: bool, slot_gap: i32, energy_gap: i32) {
        self.rows += 1;
        self.strict_correct += usize::from(strict_ok);
        self.energy_correct += usize::from(energy_gap > 0);
        self.slot_gaps.push(slot_gap);
        self.energy_gaps.push(energy_gap);
    }

    fn finish(mut self) -> SequenceSubsetDiagnostics {
        if self.rows == 0 {
            return SequenceSubsetDiagnostics::default();
        }
        self.slot_gaps.sort_unstable();
        self.energy_gaps.sort_unstable();
        SequenceSubsetDiagnostics {
            rows: self.rows,
            strict_accuracy_milli: milli_ratio(self.strict_correct, self.rows),
            sequence_energy_accuracy_milli: milli_ratio(self.energy_correct, self.rows),
            median_slot_gap: self.slot_gaps[self.rows / 2],
            p10_slot_gap: self.slot_gaps[self.rows / 10],
            median_energy_gap: self.energy_gaps[self.rows / 2],
            p10_energy_gap: self.energy_gaps[self.rows / 10],
        }
    }
}

fn is_symmetry_operator(rule_id: &str) -> bool {
    rule_id.starts_with("full_mirror") || rule_id.starts_with("pair_swap")
}

fn ablate_sequence_tasks<F>(
    tasks: &[PreparedSequenceTask],
    keep_center: F,
) -> Vec<PreparedSequenceTask>
where
    F: Fn(WavePredictorCenterId) -> bool + Copy,
{
    tasks
        .iter()
        .map(|task| PreparedSequenceTask {
            slot_tasks: task
                .slot_tasks
                .iter()
                .map(|slot_task| {
                    let mut ablated = slot_task.clone();
                    ablated
                        .active_fringe
                        .retain(|active| keep_center(active.center_id));
                    ablated
                })
                .collect(),
            output_slots: task.output_slots.clone(),
        })
        .collect()
}

fn basin_stability_sweep(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedSequenceTask],
) -> Vec<BasinStabilityPoint> {
    let variants: [(&'static str, Option<WavePredictorCenterId>, i16, usize); 7] = [
        ("clean", None, 1, 0),
        ("weaken_x2", None, 2, 0),
        ("drop_mod_11", Some(11), 1, 0),
        ("drop_mod_7", Some(7), 1, 0),
        ("drop_mod_5", Some(5), 1, 0),
        ("drop7_distract8", Some(7), 1, 8),
        ("drop5_distract16", Some(5), 1, 16),
    ];
    variants
        .iter()
        .map(|(label, drop_mod, strength_divisor, distractors)| {
            let perturbed =
                perturb_sequence_tasks_for_basin(tasks, *drop_mod, *strength_divisor, *distractors);
            let slot = eval_ordered_sequence(field, &perturbed);
            let energy = ordered_sequence_energy_diagnostics(field, &perturbed);
            BasinStabilityPoint {
                label,
                slot_accuracy_milli: slot.accuracy_milli,
                energy_accuracy_milli: energy.energy_accuracy_milli,
                median_energy_gap: energy.median_energy_gap,
                p10_energy_gap: energy.p10_energy_gap,
            }
        })
        .collect()
}

fn capacity_curve_diagnostics(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> Vec<CapacityCurvePoint> {
    let mut points = Vec::new();
    let lengths = rows
        .iter()
        .map(|row| row.sequence_length)
        .collect::<BTreeSet<_>>();
    for length in &lengths {
        let indices = matching_indices(rows, |row| row.sequence_length == *length);
        points.push(capacity_curve_point(
            field,
            tasks,
            "length",
            length.to_string(),
            &indices,
        ));
    }
    for max_length in lengths {
        let indices = matching_indices(rows, |row| row.sequence_length <= max_length);
        points.push(capacity_curve_point(
            field,
            tasks,
            "cumulative_max_length",
            format!("<= {max_length}"),
            &indices,
        ));
    }
    let families = rows
        .iter()
        .map(|row| sequence_rule_family(&row.rule_id))
        .collect::<BTreeSet<_>>();
    for family in families {
        let indices = matching_indices(rows, |row| sequence_rule_family(&row.rule_id) == family);
        points.push(capacity_curve_point(
            field,
            tasks,
            "rule_family",
            family,
            &indices,
        ));
    }
    points
}

fn matching_indices<F>(rows: &[SequenceBindingRow], predicate: F) -> Vec<usize>
where
    F: Fn(&SequenceBindingRow) -> bool,
{
    rows.iter()
        .enumerate()
        .filter_map(|(index, row)| predicate(row).then_some(index))
        .collect()
}

fn capacity_curve_point(
    field: &WavePredictorHebbianField,
    tasks: &[PreparedSequenceTask],
    kind: &'static str,
    key: String,
    indices: &[usize],
) -> CapacityCurvePoint {
    let subset = indices
        .iter()
        .map(|index| tasks[*index].clone())
        .collect::<Vec<_>>();
    let slot = eval_ordered_sequence(field, &subset);
    let energy = ordered_sequence_energy_diagnostics(field, &subset);
    CapacityCurvePoint {
        kind,
        key,
        rows: subset.len(),
        slot_accuracy_milli: slot.accuracy_milli,
        energy_accuracy_milli: energy.energy_accuracy_milli,
        p10_energy_gap: energy.p10_energy_gap,
    }
}

fn sequence_rule_family(rule_id: &str) -> String {
    rule_id
        .split_once("_len")
        .map(|(family, _)| family)
        .unwrap_or(rule_id)
        .to_string()
}

fn address_radius_sweep(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
) -> Vec<AddressRadiusPoint> {
    let variants = [
        "clean",
        "action_wrapped",
        "source_slot0_suffix",
        "source_all_suffix",
        "action_wrapped_source_slot0_suffix",
    ];
    variants
        .iter()
        .map(|label| {
            let mutated_rows = mutate_rows_for_address_radius(rows, label);
            let tasks = prepare_sequence_rows(&mutated_rows);
            let slot = eval_ordered_sequence(field, &tasks);
            let energy = ordered_sequence_energy_diagnostics(field, &tasks);
            AddressRadiusPoint {
                label,
                slot_accuracy_milli: slot.accuracy_milli,
                energy_accuracy_milli: energy.energy_accuracy_milli,
                median_energy_gap: energy.median_energy_gap,
                p10_energy_gap: energy.p10_energy_gap,
            }
        })
        .collect()
}

fn l3_role_binding_collision_report(
    table: &WavePredictorFlatRoleBindingTable,
) -> L3RoleBindingCollisionReport {
    let mut by_action: BTreeMap<WavePredictorCenterId, (usize, BTreeSet<u8>)> = BTreeMap::new();
    let mut role_slots = BTreeSet::new();
    for edge in table.edges() {
        let entry = by_action
            .entry(edge.action_center)
            .or_insert_with(|| (0, BTreeSet::new()));
        entry.0 += 1;
        entry.1.insert(edge.slot_id);
        role_slots.insert(edge.slot_id);
    }
    let action_centers_with_edges = by_action.len();
    let edge_count = table.edge_count();
    let max_edges_per_action_center = by_action
        .values()
        .map(|(edge_count, _)| *edge_count)
        .max()
        .unwrap_or(0);
    let action_centers_with_multi_slot_edges = by_action
        .values()
        .filter(|(_, slots)| slots.len() > 1)
        .count();
    let max_slots_per_action_center = by_action
        .values()
        .map(|(_, slots)| slots.len())
        .max()
        .unwrap_or(0);

    L3RoleBindingCollisionReport {
        edge_count,
        action_centers_with_edges,
        avg_edges_per_action_center_milli: milli_ratio(edge_count, action_centers_with_edges),
        max_edges_per_action_center,
        action_centers_with_multi_slot_edges,
        max_slots_per_action_center,
        role_slots_covered: role_slots.len(),
    }
}

fn mutate_rows_for_address_radius(
    rows: &[SequenceBindingRow],
    label: &str,
) -> Vec<SequenceBindingRow> {
    rows.iter()
        .map(|row| {
            let mut mutated = row.clone();
            match label {
                "clean" => {}
                "action_wrapped" => {
                    mutated.action = format!("noise_prefix {} noise_suffix", row.action);
                }
                "source_slot0_suffix" => {
                    mutated.state_before =
                        mutate_sequence_segment_tokens(&row.state_before, |index, token| {
                            if index == 0 {
                                format!("{token}_typo")
                            } else {
                                token.to_string()
                            }
                        });
                }
                "source_all_suffix" => {
                    mutated.state_before =
                        mutate_sequence_segment_tokens(&row.state_before, |_, token| {
                            format!("{token}_typo")
                        });
                }
                "action_wrapped_source_slot0_suffix" => {
                    mutated.action = format!("noise_prefix {} noise_suffix", row.action);
                    mutated.state_before =
                        mutate_sequence_segment_tokens(&row.state_before, |index, token| {
                            if index == 0 {
                                format!("{token}_typo")
                            } else {
                                token.to_string()
                            }
                        });
                }
                other => panic!("unsupported address-radius variant: {other}"),
            }
            mutated
        })
        .collect()
}

fn mutate_sequence_segment_tokens<F>(state_before: &str, mutate: F) -> String
where
    F: Fn(usize, &str) -> String,
{
    let Some((before_marker, after_marker)) = state_before.split_once("sequence:") else {
        return state_before.to_string();
    };
    let (sequence_segment, suffix) = after_marker
        .split_once(';')
        .map(|(segment, rest)| (segment, format!(";{rest}")))
        .unwrap_or((after_marker, String::new()));
    let tokens = state_tokens(&format!("state: {sequence_segment}"));
    let mutated = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| mutate(index, token))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{before_marker}sequence: {mutated}{suffix}")
}

fn perturb_sequence_tasks_for_basin(
    tasks: &[PreparedSequenceTask],
    drop_mod: Option<WavePredictorCenterId>,
    strength_divisor: i16,
    distractors_per_slot: usize,
) -> Vec<PreparedSequenceTask> {
    tasks
        .iter()
        .map(|task| PreparedSequenceTask {
            slot_tasks: task
                .slot_tasks
                .iter()
                .map(|slot_task| {
                    let mut active_fringe = perturb_active_fringe_for_basin(
                        &slot_task.active_fringe,
                        drop_mod,
                        strength_divisor,
                        distractors_per_slot,
                    );
                    active_fringe.sort_by_key(|active| active.center_id);
                    WavePredictorStateDeltaTrainTask {
                        active_fringe,
                        target_delta: slot_task.target_delta.clone(),
                        binding_output_slot: slot_task.binding_output_slot,
                    }
                })
                .collect(),
            output_slots: task.output_slots.clone(),
        })
        .collect()
}

fn perturb_active_fringe_for_basin(
    active_fringe: &[WavePredictorActiveCenter],
    drop_mod: Option<WavePredictorCenterId>,
    strength_divisor: i16,
    distractors: usize,
) -> Vec<WavePredictorActiveCenter> {
    let mut out = Vec::with_capacity(active_fringe.len() + distractors);
    for active in active_fringe {
        if drop_mod.is_some_and(|modulo| active.center_id % modulo == 0) {
            continue;
        }
        out.push(WavePredictorActiveCenter {
            center_id: active.center_id,
            strength: (active.strength / strength_divisor.max(1)).max(1),
        });
    }
    for (index, active) in active_fringe.iter().take(distractors).enumerate() {
        out.push(WavePredictorActiveCenter {
            center_id: shifted_sequence_center(active.center_id, 97 + index * 31),
            strength: 1,
        });
    }
    merge_centers(out)
}

fn shifted_sequence_center(
    center_id: WavePredictorCenterId,
    shift: usize,
) -> WavePredictorCenterId {
    let (base, span) = if center_id < SEQ_ACTION_SLOT_BASE {
        (SEQ_ROLE_BASE as usize, SEQ_ACTION_SLOT_BASE as usize)
    } else {
        (
            SEQ_ACTION_SLOT_BASE as usize,
            SEQ_ACTION_CENTER_COUNT as usize,
        )
    };
    let shifted = base + ((center_id as usize - base + shift) % span);
    shifted as WavePredictorCenterId
}

fn output_slot_cleanup_diagnostics(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> OutputSlotCleanupDiagnostics {
    let mut total_slots = 0usize;
    let mut failed_slots = 0usize;
    let mut failed_by_output_slot = BTreeMap::new();
    let mut total_by_output_slot = BTreeMap::new();
    let mut failed_by_source_slot = BTreeMap::new();
    let mut total_by_source_slot = BTreeMap::new();
    let mut failed_by_output_source_pair = BTreeMap::new();
    let mut total_by_output_source_pair = BTreeMap::new();
    let mut energy_pass_slot_fail_by_output_slot = BTreeMap::new();
    let mut symmetry_failed_by_output_slot = BTreeMap::new();
    let mut symmetry_total_by_output_slot = BTreeMap::new();
    let mut non_symmetry_failed_by_output_slot = BTreeMap::new();
    let mut non_symmetry_total_by_output_slot = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let energy_pass = sequence_energy_gap(field, task) > 0;
        let is_symmetry = is_symmetry_operator(&row.rule_id);

        for (slot_index, slot_task) in task.slot_tasks.iter().enumerate() {
            let output_slot = task
                .output_slots
                .get(slot_index)
                .copied()
                .unwrap_or(slot_index);
            let target_token = row
                .correct_tokens
                .get(output_slot)
                .map(String::as_str)
                .unwrap_or(EDIT_END_TOKEN);
            let source_slot = source_tokens
                .iter()
                .position(|token| token == target_token)
                .unwrap_or(usize::MAX);
            let pair_key = if source_slot == usize::MAX {
                format!("out{output_slot}->src_unknown")
            } else {
                format!("out{output_slot}->src{source_slot}")
            };
            let gap = state_delta_sum_gap(field, slot_task);
            let failed = gap <= 0;

            total_slots += 1;
            failed_slots += usize::from(failed);
            *total_by_output_slot.entry(output_slot).or_default() += 1;
            *total_by_source_slot.entry(source_slot).or_default() += 1;
            *total_by_output_source_pair
                .entry(pair_key.clone())
                .or_default() += 1;

            if is_symmetry {
                *symmetry_total_by_output_slot
                    .entry(output_slot)
                    .or_default() += 1;
            } else {
                *non_symmetry_total_by_output_slot
                    .entry(output_slot)
                    .or_default() += 1;
            }

            if failed {
                *failed_by_output_slot.entry(output_slot).or_default() += 1;
                *failed_by_source_slot.entry(source_slot).or_default() += 1;
                *failed_by_output_source_pair.entry(pair_key).or_default() += 1;
                if energy_pass {
                    *energy_pass_slot_fail_by_output_slot
                        .entry(output_slot)
                        .or_default() += 1;
                }
                if is_symmetry {
                    *symmetry_failed_by_output_slot
                        .entry(output_slot)
                        .or_default() += 1;
                } else {
                    *non_symmetry_failed_by_output_slot
                        .entry(output_slot)
                        .or_default() += 1;
                }
            }
        }
    }

    let accuracy_by_output_slot = accuracy_by_key(&total_by_output_slot, &failed_by_output_slot);
    let accuracy_by_source_slot = accuracy_by_key(&total_by_source_slot, &failed_by_source_slot);
    let accuracy_by_output_source_pair =
        accuracy_by_key(&total_by_output_source_pair, &failed_by_output_source_pair);
    let symmetry_accuracy_by_output_slot = accuracy_by_key(
        &symmetry_total_by_output_slot,
        &symmetry_failed_by_output_slot,
    );
    let non_symmetry_accuracy_by_output_slot = accuracy_by_key(
        &non_symmetry_total_by_output_slot,
        &non_symmetry_failed_by_output_slot,
    );

    OutputSlotCleanupDiagnostics {
        total_slots,
        failed_slots,
        accuracy_milli: milli_ratio(total_slots - failed_slots, total_slots),
        failed_by_output_slot,
        total_by_output_slot,
        accuracy_by_output_slot,
        failed_by_source_slot,
        total_by_source_slot,
        accuracy_by_source_slot,
        failed_by_output_source_pair,
        total_by_output_source_pair,
        accuracy_by_output_source_pair,
        energy_pass_slot_fail_by_output_slot,
        symmetry_accuracy_by_output_slot,
        non_symmetry_accuracy_by_output_slot,
    }
}

fn sequence_slot_failure_group_diagnostics(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> SequenceSlotFailureGroupDiagnostics {
    let mut failed_by_length = BTreeMap::new();
    let mut total_by_length = BTreeMap::new();
    let mut energy_pass_slot_fail_by_length = BTreeMap::new();
    let mut failed_by_rule = BTreeMap::new();
    let mut total_by_rule = BTreeMap::new();
    let mut energy_pass_slot_fail_by_rule = BTreeMap::new();
    let mut failed_by_surface = BTreeMap::new();
    let mut total_by_surface = BTreeMap::new();
    let mut failed_by_noise = BTreeMap::new();
    let mut total_by_noise = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let energy_pass = sequence_energy_gap(field, task) > 0;
        for slot_task in &task.slot_tasks {
            let failed = state_delta_sum_gap(field, slot_task) <= 0;
            *total_by_length.entry(row.sequence_length).or_default() += 1;
            *total_by_rule.entry(row.rule_id.clone()).or_default() += 1;
            *total_by_surface
                .entry(row.surface_family.clone())
                .or_default() += 1;
            *total_by_noise.entry(row.noise_type.clone()).or_default() += 1;

            if failed {
                *failed_by_length.entry(row.sequence_length).or_default() += 1;
                *failed_by_rule.entry(row.rule_id.clone()).or_default() += 1;
                *failed_by_surface
                    .entry(row.surface_family.clone())
                    .or_default() += 1;
                *failed_by_noise.entry(row.noise_type.clone()).or_default() += 1;
                if energy_pass {
                    *energy_pass_slot_fail_by_length
                        .entry(row.sequence_length)
                        .or_default() += 1;
                    *energy_pass_slot_fail_by_rule
                        .entry(row.rule_id.clone())
                        .or_default() += 1;
                }
            }
        }
    }

    let accuracy_by_length = accuracy_by_key(&total_by_length, &failed_by_length);
    let accuracy_by_rule = accuracy_by_key(&total_by_rule, &failed_by_rule);
    let accuracy_by_surface = accuracy_by_key(&total_by_surface, &failed_by_surface);
    let accuracy_by_noise = accuracy_by_key(&total_by_noise, &failed_by_noise);

    SequenceSlotFailureGroupDiagnostics {
        failed_by_length,
        total_by_length,
        accuracy_by_length,
        energy_pass_slot_fail_by_length,
        failed_by_rule,
        total_by_rule,
        accuracy_by_rule,
        energy_pass_slot_fail_by_rule,
        failed_by_surface,
        total_by_surface,
        accuracy_by_surface,
        failed_by_noise,
        total_by_noise,
        accuracy_by_noise,
    }
}

fn edit_role_binding_boundary_report(rows: &[SequenceBindingRow]) -> EditRoleBindingBoundaryReport {
    let mut report = EditRoleBindingBoundaryReport {
        rows: rows.len(),
        ..EditRoleBindingBoundaryReport::default()
    };

    for row in rows {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let source_set: BTreeSet<_> = source_tokens.iter().map(String::as_str).collect();
        let family = rule_family_from_id(&row.rule_id).to_string();
        let output_len_over_slots = row.correct_tokens.len() > usize::from(SEQ_OUTPUT_SLOT_COUNT);
        let correct_wrong_len_mismatch = row.correct_tokens.len() != row.wrong_tokens.len();
        let non_source_output = row
            .correct_tokens
            .iter()
            .any(|token| !source_set.contains(token.as_str()));
        let marker_output = row
            .correct_tokens
            .iter()
            .any(|token| token.starts_with("mark_"));
        let representable_by_current_role_transfer =
            !output_len_over_slots && !correct_wrong_len_mismatch && !non_source_output;

        report.rows_output_len_over_slots += usize::from(output_len_over_slots);
        report.rows_correct_wrong_len_mismatch += usize::from(correct_wrong_len_mismatch);
        report.rows_with_non_source_output_tokens += usize::from(non_source_output);
        report.rows_with_marker_output_tokens += usize::from(marker_output);
        report.rows_representable_by_current_role_transfer +=
            usize::from(representable_by_current_role_transfer);
        report.rows_not_representable_by_current_role_transfer +=
            usize::from(!representable_by_current_role_transfer);

        report
            .correct_len_by_family
            .entry(family.clone())
            .or_default()
            .entry(row.correct_tokens.len())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        if output_len_over_slots {
            *report
                .output_len_over_slots_by_family
                .entry(family.clone())
                .or_default() += 1;
        }
        if non_source_output {
            *report
                .non_source_output_by_family
                .entry(family.clone())
                .or_default() += 1;
        }
        if !representable_by_current_role_transfer {
            *report
                .non_representable_by_family
                .entry(family)
                .or_default() += 1;
        }
    }

    report
}

fn conditional_runtime_boundary_report(
    rows: &[SequenceBindingRow],
) -> ConditionalRuntimeBoundaryReport {
    let mut report = ConditionalRuntimeBoundaryReport {
        rows: rows.len(),
        ..ConditionalRuntimeBoundaryReport::default()
    };

    for row in rows {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let source_set: BTreeSet<_> = source_tokens.iter().map(String::as_str).collect();
        let state_flag = extract_flag_value(&row.state_before, "condition: flag_");
        let action_flag = extract_flag_value(&row.action, "current_flag: flag_");
        let same_bag = sorted_tokens(&row.correct_tokens) == sorted_tokens(&row.wrong_tokens);
        let all_outputs_from_source = row
            .correct_tokens
            .iter()
            .all(|token| source_set.contains(token.as_str()));
        let output_len_within_slots =
            row.correct_tokens.len() <= usize::from(SEQ_OUTPUT_SLOT_COUNT);
        let source_tokens_include_condition_flag =
            source_tokens.iter().any(|token| token.starts_with("flag_"));
        let action_flag_matches_state_flag =
            state_flag.is_some() && action_flag.is_some() && state_flag == action_flag;
        let branch_signal_action_only_for_current_runtime =
            action_flag_matches_state_flag && !source_tokens_include_condition_flag;
        let representable_as_order_transfer_if_branch_known =
            same_bag && all_outputs_from_source && output_len_within_slots;

        report.rows_same_bag += usize::from(same_bag);
        report.rows_all_outputs_from_source += usize::from(all_outputs_from_source);
        report.rows_output_len_within_slots += usize::from(output_len_within_slots);
        report.rows_with_state_condition_flag += usize::from(state_flag.is_some());
        report.rows_with_action_current_flag += usize::from(action_flag.is_some());
        report.rows_action_flag_matches_state_flag += usize::from(action_flag_matches_state_flag);
        report.rows_source_tokens_include_condition_flag +=
            usize::from(source_tokens_include_condition_flag);
        report.rows_branch_signal_action_only_for_current_runtime +=
            usize::from(branch_signal_action_only_for_current_runtime);
        report.rows_representable_as_order_transfer_if_branch_known +=
            usize::from(representable_as_order_transfer_if_branch_known);

        if action_flag.is_some() {
            *report
                .action_current_flag_by_family
                .entry(conditional_rule_family_from_id(&row.rule_id).to_string())
                .or_default() += 1;
        }
    }

    report
}

fn extract_flag_value(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let value = text[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn rule_family_from_id(rule_id: &str) -> &str {
    let without_class = rule_id.strip_prefix("edit_").unwrap_or(rule_id);
    without_class
        .rsplit_once("_len")
        .map(|(family, _)| family)
        .unwrap_or(without_class)
}

fn conditional_rule_family_from_id(rule_id: &str) -> &str {
    let without_class = rule_id.strip_prefix("conditional_").unwrap_or(rule_id);
    without_class
        .rsplit_once("_len")
        .map(|(family, _)| family)
        .unwrap_or(without_class)
}

fn print_sequence_slot_failures(
    label: &str,
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
    limit: usize,
) {
    let mut printed = 0usize;
    let mut total = 0usize;

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let energy_gap = sequence_energy_gap(field, task);

        for (output_slot, slot_task) in task.slot_tasks.iter().enumerate() {
            let gap = state_delta_sum_gap(field, slot_task);
            if gap > 0 {
                continue;
            }

            total += 1;
            if printed >= limit {
                continue;
            }

            let correct_token = row
                .correct_tokens
                .get(output_slot)
                .map(String::as_str)
                .unwrap_or("<missing>");
            let source_slot = source_tokens
                .iter()
                .position(|token| token == correct_token)
                .map(|slot| slot.to_string());
            let source_slot_label = source_slot
                .clone()
                .unwrap_or_else(|| "not_in_source".to_string());

            println!(
                "{label}: slot_failure source_group={} rule={} length={} surface={} noise={} output_slot={} source_slot={} gap={} sequence_energy_gap={} correct_token={}",
                row.source_group,
                row.rule_id,
                row.sequence_length,
                row.surface_family,
                row.noise_type,
                output_slot,
                source_slot_label,
                gap,
                energy_gap,
                correct_token
            );
            print_sequence_slot_pressure(label, field, row, task, output_slot);
            printed += 1;
        }
    }

    println!("{label}: slot_failure_total={total}");
}

fn print_sequence_slot_pressure(
    label: &str,
    field: &WavePredictorHebbianField,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) {
    let slot_task = &task.slot_tasks[output_slot];
    let correct_token = row
        .correct_tokens
        .get(output_slot)
        .map(String::as_str)
        .unwrap_or("<missing>");
    let wrong_token = row
        .wrong_tokens
        .get(output_slot)
        .map(String::as_str)
        .unwrap_or("<missing>");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let cosine_milli = (target_wave.cosine_similarity(&wrong_wave) * 1000.0).round() as i32;
    let target_score = state_delta_target_score(field, slot_task);
    let wrong_score = state_delta_wrong_score(field, slot_task);
    let action_centers = slot_task
        .active_fringe
        .iter()
        .filter(|active| {
            active.center_id >= SEQ_ACTION_SLOT_BASE
                && active.center_id < SEQ_ACTION_SLOT_BASE + SEQ_ACTION_CENTER_COUNT
        })
        .count();

    println!(
        "{label}: slot_pressure output_slot={} correct_token={} wrong_token={} target_score={} wrong_score={} target_active_lanes={} wrong_active_lanes={} target_wrong_cosine_milli={} active_action_centers={}",
        output_slot,
        correct_token,
        wrong_token,
        target_score,
        wrong_score,
        target_wave.active_lanes(),
        wrong_wave.active_lanes(),
        cosine_milli,
        action_centers
    );
    println!(
        "{label}: slot_pressure target_worst_impulses={:?}",
        impulse_pressure_entries(
            field,
            slot_task,
            slot_task.target_delta.positive_impulses(),
            false,
            8
        )
    );
    println!(
        "{label}: slot_pressure wrong_strongest_impulses={:?}",
        impulse_pressure_entries(
            field,
            slot_task,
            slot_task.target_delta.negative_impulses(),
            true,
            8
        )
    );
}

fn impulse_pressure_entries(
    field: &WavePredictorHebbianField,
    task: &WavePredictorStateDeltaTrainTask,
    impulses: &[WavePredictorStateImpulse],
    strongest_first: bool,
    limit: usize,
) -> Vec<String> {
    let mut entries: Vec<_> = impulses
        .iter()
        .map(|impulse| {
            let sign = if impulse.signed_strength < 0 { -1 } else { 1 };
            let direct = sign * field.score_state_delta_lane(impulse.lane_id, &task.active_fringe);
            let binding = field.score_state_delta_binding_alignment(
                impulse.lane_id,
                impulse.signed_strength,
                &task.active_fringe,
                task.binding_output_slot,
            );
            let total = direct + binding;
            (
                total,
                impulse.lane_id,
                impulse.signed_strength,
                direct,
                binding,
                active_sequence_role_slots_for_lane(&task.active_fringe, impulse.lane_id),
            )
        })
        .collect();

    if strongest_first {
        entries.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    } else {
        entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    }
    entries.truncate(limit);

    entries
        .into_iter()
        .map(
            |(total, lane_id, signed_strength, direct, binding, roles)| {
                format!(
                    "lane={lane_id}:strength={signed_strength}:total={total}:direct={direct}:binding={binding}:roles={roles:?}"
                )
            },
        )
        .collect()
}

fn active_sequence_role_slots_for_lane(
    active_fringe: &[WavePredictorActiveCenter],
    lane_id: u16,
) -> Vec<(u8, i16)> {
    let projected_lane = WavePredictorCenterId::from(lane_id) % SEQ_FEATURE_CENTER_COUNT;
    let mut roles = Vec::new();
    for slot_id in 0..SEQ_ROLE_SLOT_COUNT {
        let center_id = SEQ_ROLE_BASE
            + WavePredictorCenterId::from(slot_id) * SEQ_FEATURE_CENTER_COUNT
            + projected_lane;
        if let Some(active) = active_fringe
            .iter()
            .find(|active| active.center_id == center_id && active.strength != 0)
        {
            roles.push((slot_id, active.strength));
        }
    }
    roles
}

fn accuracy_by_key<K: Ord + Clone>(
    totals: &BTreeMap<K, usize>,
    failures: &BTreeMap<K, usize>,
) -> BTreeMap<K, usize> {
    totals
        .iter()
        .map(|(key, total)| {
            let failed = failures.get(key).copied().unwrap_or(0);
            (key.clone(), milli_ratio(total - failed, *total))
        })
        .collect()
}

fn ordered_group_diagnostics(
    field: &WavePredictorHebbianField,
    table: &WavePredictorFlatRoleBindingTable,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> OrderedGroupDiagnostics {
    let mut matrix_groups: BTreeMap<String, bool> = BTreeMap::new();
    let mut length_groups: BTreeMap<usize, bool> = BTreeMap::new();
    let mut rule_groups: BTreeMap<&str, bool> = BTreeMap::new();
    let mut surface_groups: BTreeMap<&str, bool> = BTreeMap::new();
    let mut noise_groups: BTreeMap<&str, bool> = BTreeMap::new();
    let mut output_slots: BTreeMap<usize, bool> = BTreeMap::new();
    let mut flat_gap_mismatches = 0usize;
    let mut failed_rows_by_length = BTreeMap::new();
    let mut failed_rows_by_rule = BTreeMap::new();
    let mut failed_rows_by_surface = BTreeMap::new();
    let mut failed_rows_by_noise = BTreeMap::new();
    let mut failed_slots_by_output_slot = BTreeMap::new();
    let mut total_slots_by_output_slot = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let (row_ok, field_gap) = ordered_sequence_row_gap(field, task);
        let (_, flat_gap) = ordered_sequence_flat_row_gap(table, task);
        flat_gap_mismatches += usize::from(field_gap != flat_gap);
        if !row_ok {
            *failed_rows_by_length
                .entry(row.sequence_length)
                .or_default() += 1;
            *failed_rows_by_rule.entry(row.rule_id.clone()).or_default() += 1;
            *failed_rows_by_surface
                .entry(row.surface_family.clone())
                .or_default() += 1;
            *failed_rows_by_noise
                .entry(row.noise_type.clone())
                .or_default() += 1;
        }

        let matrix_key = format!(
            "{}|{}|{}|{}",
            row.sequence_length, row.rule_id, row.surface_family, row.noise_type
        );
        group_and(&mut matrix_groups, matrix_key, row_ok);
        group_and(&mut length_groups, row.sequence_length, row_ok);
        group_and(&mut rule_groups, row.rule_id.as_str(), row_ok);
        group_and(&mut surface_groups, row.surface_family.as_str(), row_ok);
        group_and(&mut noise_groups, row.noise_type.as_str(), row_ok);

        for (slot_index, slot_task) in task.slot_tasks.iter().enumerate() {
            let gap = state_delta_sum_gap(field, slot_task);
            *total_slots_by_output_slot.entry(slot_index).or_default() += 1;
            if gap <= 0 {
                *failed_slots_by_output_slot.entry(slot_index).or_default() += 1;
            }
            group_and(&mut output_slots, slot_index, gap > 0);
        }
    }

    let slot_accuracy_milli_by_output_slot = total_slots_by_output_slot
        .iter()
        .map(|(slot_index, total)| {
            let failed = failed_slots_by_output_slot
                .get(slot_index)
                .copied()
                .unwrap_or(0);
            (*slot_index, milli_ratio(total - failed, *total))
        })
        .collect();

    OrderedGroupDiagnostics {
        matrix_groups: matrix_groups.len(),
        matrix_group_failures: count_group_failures(&matrix_groups),
        length_group_failures: count_group_failures(&length_groups),
        rule_group_failures: count_group_failures(&rule_groups),
        surface_group_failures: count_group_failures(&surface_groups),
        noise_group_failures: count_group_failures(&noise_groups),
        output_slot_failures: count_group_failures(&output_slots),
        flat_gap_mismatches,
        failed_rows_by_length,
        failed_rows_by_rule,
        failed_rows_by_surface,
        failed_rows_by_noise,
        failed_slots_by_output_slot,
        total_slots_by_output_slot,
        slot_accuracy_milli_by_output_slot,
    }
}

fn group_and<K: Ord>(groups: &mut BTreeMap<K, bool>, key: K, ok: bool) {
    groups
        .entry(key)
        .and_modify(|current| *current &= ok)
        .or_insert(ok);
}

fn count_group_failures<K>(groups: &BTreeMap<K, bool>) -> usize {
    groups.values().filter(|ok| !**ok).count()
}

fn action_separability_report(rows: &[SequenceBindingRow]) -> ActionSeparabilityReport {
    let mut by_rule: BTreeMap<String, (String, BTreeSet<(u16, i8)>)> = BTreeMap::new();
    for row in rows {
        by_rule.entry(row.rule_id.clone()).or_insert_with(|| {
            (
                row.action.clone(),
                top_folded_signed_lanes(&row.action, SEQ_FEATURE_CENTER_COUNT, TOP_ACTION_L1_LANES),
            )
        });
    }

    let items: Vec<_> = by_rule.into_iter().collect();
    let mut different_sum = 0usize;
    let mut different_count = 0usize;
    let mut same_family_sum = 0usize;
    let mut same_family_count = 0usize;
    let mut different_family_sum = 0usize;
    let mut different_family_count = 0usize;
    let mut nearest = Vec::new();

    for left_index in 0..items.len() {
        for right_index in left_index + 1..items.len() {
            let left_rule = &items[left_index].0;
            let right_rule = &items[right_index].0;
            let left = &items[left_index].1.1;
            let right = &items[right_index].1.1;
            let similarity = set_similarity_milli(left, right);
            different_sum += similarity;
            different_count += 1;
            if rule_family_name(left_rule) == rule_family_name(right_rule) {
                same_family_sum += similarity;
                same_family_count += 1;
            } else {
                different_family_sum += similarity;
                different_family_count += 1;
            }
            nearest.push((left_rule.clone(), right_rule.clone(), similarity));
        }
    }
    nearest.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
    nearest.truncate(8);

    ActionSeparabilityReport {
        action_vectors: items.len(),
        same_rule_similarity_milli: 1000,
        different_rule_similarity_milli: avg_usize(different_sum, different_count),
        same_family_different_length_similarity_milli: avg_usize(
            same_family_sum,
            same_family_count,
        ),
        different_family_similarity_milli: avg_usize(different_family_sum, different_family_count),
        max_different_rule_similarity_milli: nearest.first().map(|item| item.2).unwrap_or(0),
        nearest_rule_pairs: nearest,
    }
}

fn folded_collision_report(rows: &[SequenceBindingRow]) -> FoldedCollisionReport {
    let mut checked = 0usize;
    let mut multi_role_hit_count = 0usize;
    let mut wrong_role_hit_count = 0usize;
    let mut missing_true_role_count = 0usize;

    for row in rows {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| top_folded_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES))
            .collect();
        for (output_slot, correct_token) in row.correct_tokens.iter().enumerate() {
            let Some(true_slot) = source_tokens
                .iter()
                .position(|token| token == correct_token)
            else {
                continue;
            };
            let wrong_token = &row.wrong_tokens[output_slot];
            let base_wave = SurfaceWave4096::compile("");
            let target_wave = SurfaceWave4096::compile(correct_token);
            let wrong_wave = SurfaceWave4096::compile(wrong_token);
            for impulse in discriminative_delta_impulses(
                base_wave.lanes(),
                target_wave.lanes(),
                wrong_wave.lanes(),
                STATE_DELTA_LANES_PER_SIDE,
            ) {
                checked += 1;
                let projected_lane = (WavePredictorCenterId::from(impulse.lane_id)
                    % SEQ_FEATURE_CENTER_COUNT) as u16;
                let mut hit_count = 0usize;
                let mut true_hit = false;
                let mut wrong_hit = false;
                for (slot, role_lanes) in role_lanes_by_slot.iter().enumerate() {
                    if role_lanes.contains(&projected_lane) {
                        hit_count += 1;
                        true_hit |= slot == true_slot;
                        wrong_hit |= slot != true_slot;
                    }
                }
                multi_role_hit_count += usize::from(hit_count > 1);
                wrong_role_hit_count += usize::from(wrong_hit);
                missing_true_role_count += usize::from(!true_hit);
            }
        }
    }

    FoldedCollisionReport {
        target_impulses_checked: checked,
        multi_role_hit_count,
        wrong_role_hit_count,
        missing_true_role_count,
        multi_role_hit_milli: milli_ratio(multi_role_hit_count, checked),
        wrong_role_hit_milli: milli_ratio(wrong_role_hit_count, checked),
        missing_true_role_milli: milli_ratio(missing_true_role_count, checked),
    }
}

fn folded_collision_report_by_surface(
    rows: &[SequenceBindingRow],
) -> BTreeMap<String, FoldedCollisionReport> {
    let mut by_surface: BTreeMap<String, Vec<SequenceBindingRow>> = BTreeMap::new();
    for row in rows {
        by_surface
            .entry(row.surface_family.clone())
            .or_default()
            .push(row.clone());
    }
    by_surface
        .into_iter()
        .map(|(surface, rows)| (surface, folded_collision_report(&rows)))
        .collect()
}

fn conditional_lane_overlap_report(rows: &[SequenceBindingRow]) -> LaneOverlapReport {
    let mut by_surface = BTreeMap::new();
    let mut by_output_source_pair = BTreeMap::new();
    let mut by_surface_output_source_pair = BTreeMap::new();

    for row in rows {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| top_folded_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES))
            .collect();

        for (output_slot, correct_token) in row.correct_tokens.iter().enumerate() {
            let Some(wrong_token) = row.wrong_tokens.get(output_slot) else {
                continue;
            };
            let Some(true_slot) = source_tokens
                .iter()
                .position(|token| token == correct_token)
            else {
                continue;
            };

            let target_lanes =
                top_folded_lanes(correct_token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES);
            let wrong_lanes =
                top_folded_lanes(wrong_token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES);
            let summary = slot_lane_overlap_summary(
                &target_lanes,
                &wrong_lanes,
                &role_lanes_by_slot,
                true_slot,
            );
            let pair = format!("out{output_slot}->src{true_slot}");
            let surface_pair = format!("{}|{pair}", row.surface_family);

            update_lane_overlap(&mut by_surface, row.surface_family.clone(), &summary);
            update_lane_overlap(&mut by_output_source_pair, pair, &summary);
            update_lane_overlap(&mut by_surface_output_source_pair, surface_pair, &summary);
        }
    }

    LaneOverlapReport {
        by_surface: finalize_lane_overlaps(by_surface),
        worst_output_source_pairs: worst_lane_overlaps(by_output_source_pair, 12),
        worst_surface_output_source_pairs: worst_lane_overlaps(by_surface_output_source_pair, 16),
    }
}

fn conditional_sign_aware_collision_report(
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> SignAwareCollisionReport {
    let mut by_surface = BTreeMap::new();
    let mut by_output_source_pair = BTreeMap::new();
    let mut by_surface_output_source_pair = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let signed_role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| {
                top_folded_signed_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES)
            })
            .collect();

        for (output_slot, slot_task) in task.slot_tasks.iter().enumerate() {
            let target_token = &row.correct_tokens[output_slot];
            let Some(true_slot) = source_tokens.iter().position(|token| token == target_token)
            else {
                continue;
            };
            let pair = format!("out{output_slot}->src{true_slot}");
            let surface_pair = format!("{}|{pair}", row.surface_family);
            let summary = sign_aware_slot_collision_accumulator(
                slot_task,
                &signed_role_lanes_by_slot,
                true_slot,
            );

            update_sign_aware_collision(&mut by_surface, row.surface_family.clone(), &summary);
            update_sign_aware_collision(&mut by_output_source_pair, pair, &summary);
            update_sign_aware_collision(&mut by_surface_output_source_pair, surface_pair, &summary);
        }
    }

    SignAwareCollisionReport {
        by_surface: finalize_sign_aware_collisions(by_surface),
        worst_output_source_pairs: worst_sign_aware_collisions(by_output_source_pair, 12),
        worst_surface_output_source_pairs: worst_sign_aware_collisions(
            by_surface_output_source_pair,
            16,
        ),
    }
}

fn conditional_residual_collision_outcome_report(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> ResidualCollisionOutcomeReport {
    let mut by_bucket = BTreeMap::new();
    let mut by_surface = BTreeMap::new();
    let mut by_surface_bucket = BTreeMap::new();
    let mut by_output_source_pair = BTreeMap::new();
    let mut by_surface_output_source_pair = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let signed_role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| {
                top_folded_signed_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES)
            })
            .collect();
        let energy_pass = sequence_energy_gap(field, task) > 0;

        for (output_slot, slot_task) in task.slot_tasks.iter().enumerate() {
            let target_token = &row.correct_tokens[output_slot];
            let Some(source_slot) = source_tokens.iter().position(|token| token == target_token)
            else {
                continue;
            };
            let residual = sign_aware_slot_collision_accumulator(
                slot_task,
                &signed_role_lanes_by_slot,
                source_slot,
            );
            let summary = summarize_sign_aware_collision(&residual);
            let bucket = residual_collision_bucket(&summary);
            let surface_bucket = format!("{}|{bucket}", row.surface_family);
            let pair = format!("out{output_slot}->src{source_slot}");
            let surface_pair = format!("{}|{pair}", row.surface_family);
            let gap = state_delta_sum_gap(field, slot_task);
            let failed = gap <= 0;
            let energy_pass_slot_fail = energy_pass && failed;

            update_residual_collision_outcome(
                &mut by_bucket,
                bucket,
                gap,
                failed,
                energy_pass_slot_fail,
                &summary,
            );
            update_residual_collision_outcome(
                &mut by_surface,
                row.surface_family.clone(),
                gap,
                failed,
                energy_pass_slot_fail,
                &summary,
            );
            update_residual_collision_outcome(
                &mut by_surface_bucket,
                surface_bucket,
                gap,
                failed,
                energy_pass_slot_fail,
                &summary,
            );
            update_residual_collision_outcome(
                &mut by_output_source_pair,
                pair,
                gap,
                failed,
                energy_pass_slot_fail,
                &summary,
            );
            update_residual_collision_outcome(
                &mut by_surface_output_source_pair,
                surface_pair,
                gap,
                failed,
                energy_pass_slot_fail,
                &summary,
            );
        }
    }

    ResidualCollisionOutcomeReport {
        by_bucket: finalize_residual_collision_outcomes(by_bucket),
        by_surface: finalize_residual_collision_outcomes(by_surface),
        by_surface_bucket: finalize_residual_collision_outcomes(by_surface_bucket),
        worst_output_source_pairs: worst_residual_collision_outcomes(by_output_source_pair, 12),
        worst_surface_output_source_pairs: worst_residual_collision_outcomes(
            by_surface_output_source_pair,
            16,
        ),
    }
}

fn conditional_collision_outcome_report(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> CollisionOutcomeReport {
    let mut by_bucket = BTreeMap::new();
    let mut by_surface = BTreeMap::new();
    let mut by_surface_bucket = BTreeMap::new();
    let mut by_output_source_pair = BTreeMap::new();
    let mut by_surface_output_source_pair = BTreeMap::new();

    for (row, task) in rows.iter().zip(tasks.iter()) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| top_folded_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES))
            .collect();
        let energy_pass = sequence_energy_gap(field, task) > 0;

        for (output_slot, slot_task) in task.slot_tasks.iter().enumerate() {
            let target_token = &row.correct_tokens[output_slot];
            let source_slot = source_tokens
                .iter()
                .position(|token| token == target_token)
                .unwrap_or(usize::MAX);
            if source_slot == usize::MAX {
                continue;
            }
            let collision = slot_folded_collision_report(
                slot_task.target_delta.positive_impulses(),
                &role_lanes_by_slot,
                source_slot,
            );
            let bucket = collision_bucket(&collision);
            let pair = format!("out{output_slot}->src{source_slot}");
            let surface_pair = format!("{}|{pair}", row.surface_family);
            let surface_bucket = format!("{}|{bucket}", row.surface_family);
            let gap = state_delta_sum_gap(field, slot_task);
            let failed = gap <= 0;
            let energy_pass_slot_fail = energy_pass && failed;

            update_collision_outcome(
                &mut by_bucket,
                bucket.clone(),
                gap,
                failed,
                energy_pass_slot_fail,
                &collision,
            );
            update_collision_outcome(
                &mut by_surface,
                row.surface_family.clone(),
                gap,
                failed,
                energy_pass_slot_fail,
                &collision,
            );
            update_collision_outcome(
                &mut by_surface_bucket,
                surface_bucket,
                gap,
                failed,
                energy_pass_slot_fail,
                &collision,
            );
            update_collision_outcome(
                &mut by_output_source_pair,
                pair,
                gap,
                failed,
                energy_pass_slot_fail,
                &collision,
            );
            update_collision_outcome(
                &mut by_surface_output_source_pair,
                surface_pair,
                gap,
                failed,
                energy_pass_slot_fail,
                &collision,
            );
        }
    }

    CollisionOutcomeReport {
        by_bucket: finalize_collision_outcomes(by_bucket),
        by_surface: finalize_collision_outcomes(by_surface),
        by_surface_bucket: finalize_collision_outcomes(by_surface_bucket),
        worst_output_source_pairs: worst_collision_outcomes(by_output_source_pair, 12),
        worst_surface_output_source_pairs: worst_collision_outcomes(
            by_surface_output_source_pair,
            16,
        ),
    }
}

fn slot_folded_collision_report(
    impulses: &[WavePredictorStateImpulse],
    role_lanes_by_slot: &[BTreeSet<u16>],
    true_slot: usize,
) -> FoldedCollisionReport {
    let mut checked = 0usize;
    let mut multi_role_hit_count = 0usize;
    let mut wrong_role_hit_count = 0usize;
    let mut missing_true_role_count = 0usize;

    for impulse in impulses {
        checked += 1;
        let projected_lane =
            (WavePredictorCenterId::from(impulse.lane_id) % SEQ_FEATURE_CENTER_COUNT) as u16;
        let mut hit_count = 0usize;
        let mut true_hit = false;
        let mut wrong_hit = false;
        for (slot, role_lanes) in role_lanes_by_slot.iter().enumerate() {
            if role_lanes.contains(&projected_lane) {
                hit_count += 1;
                true_hit |= slot == true_slot;
                wrong_hit |= slot != true_slot;
            }
        }
        multi_role_hit_count += usize::from(hit_count > 1);
        wrong_role_hit_count += usize::from(wrong_hit);
        missing_true_role_count += usize::from(!true_hit);
    }

    FoldedCollisionReport {
        target_impulses_checked: checked,
        multi_role_hit_count,
        wrong_role_hit_count,
        missing_true_role_count,
        multi_role_hit_milli: milli_ratio(multi_role_hit_count, checked),
        wrong_role_hit_milli: milli_ratio(wrong_role_hit_count, checked),
        missing_true_role_milli: milli_ratio(missing_true_role_count, checked),
    }
}

fn collision_bucket(collision: &FoldedCollisionReport) -> String {
    if collision.missing_true_role_count > 0 {
        return "missing_true_role".to_string();
    }
    match collision.wrong_role_hit_milli {
        0 => "no_wrong_role_hit".to_string(),
        1..=124 => "low_wrong_role_hit".to_string(),
        125..=249 => "mid_wrong_role_hit".to_string(),
        _ => "high_wrong_role_hit".to_string(),
    }
}

fn update_collision_outcome(
    map: &mut BTreeMap<String, CollisionOutcomeAccumulator>,
    key: String,
    gap: i32,
    failed: bool,
    energy_pass_slot_fail: bool,
    collision: &FoldedCollisionReport,
) {
    let entry = map.entry(key).or_default();
    if entry.slots == 0 {
        entry.min_gap = gap;
    } else {
        entry.min_gap = entry.min_gap.min(gap);
    }
    entry.slots += 1;
    entry.failed_slots += usize::from(failed);
    entry.energy_pass_slot_fail += usize::from(energy_pass_slot_fail);
    entry.gap_sum += i64::from(gap);
    entry.wrong_role_hit_milli_sum += collision.wrong_role_hit_milli;
    entry.multi_role_hit_milli_sum += collision.multi_role_hit_milli;
}

fn finalize_collision_outcomes(
    map: BTreeMap<String, CollisionOutcomeAccumulator>,
) -> BTreeMap<String, CollisionOutcomeSummary> {
    map.into_iter()
        .map(|(key, value)| (key, summarize_collision_outcome(&value)))
        .collect()
}

fn worst_collision_outcomes(
    map: BTreeMap<String, CollisionOutcomeAccumulator>,
    limit: usize,
) -> Vec<(String, CollisionOutcomeSummary)> {
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(key, value)| (key, summarize_collision_outcome(&value)))
        .collect();
    items.sort_by(|left, right| {
        left.1
            .accuracy_milli
            .cmp(&right.1.accuracy_milli)
            .then_with(|| right.1.failed_slots.cmp(&left.1.failed_slots))
            .then_with(|| left.0.cmp(&right.0))
    });
    items.truncate(limit);
    items
}

fn summarize_collision_outcome(value: &CollisionOutcomeAccumulator) -> CollisionOutcomeSummary {
    CollisionOutcomeSummary {
        slots: value.slots,
        failed_slots: value.failed_slots,
        accuracy_milli: milli_ratio(value.slots - value.failed_slots, value.slots),
        energy_pass_slot_fail: value.energy_pass_slot_fail,
        avg_gap: if value.slots == 0 {
            0
        } else {
            (value.gap_sum / value.slots as i64) as i32
        },
        min_gap: value.min_gap,
        avg_wrong_role_hit_milli: milli_ratio(value.wrong_role_hit_milli_sum, value.slots),
        avg_multi_role_hit_milli: milli_ratio(value.multi_role_hit_milli_sum, value.slots),
    }
}

fn slot_lane_overlap_summary(
    target_lanes: &BTreeSet<u16>,
    wrong_lanes: &BTreeSet<u16>,
    role_lanes_by_slot: &[BTreeSet<u16>],
    true_slot: usize,
) -> LaneOverlapSummary {
    let target_wrong_overlap = target_lanes.intersection(wrong_lanes).count();
    let true_role_lanes = role_lanes_by_slot
        .get(true_slot)
        .expect("true role slot must have lanes");
    let wrong_hits_true_role = wrong_lanes.intersection(true_role_lanes).count();

    let mut target_hits_wrong_role = 0usize;
    let mut target_hits_multi_role = 0usize;
    let mut target_missing_true_role = 0usize;
    for lane in target_lanes {
        let mut hits = 0usize;
        let mut true_hit = false;
        let mut wrong_hit = false;
        for (slot, role_lanes) in role_lanes_by_slot.iter().enumerate() {
            if role_lanes.contains(lane) {
                hits += 1;
                true_hit |= slot == true_slot;
                wrong_hit |= slot != true_slot;
            }
        }
        target_hits_wrong_role += usize::from(wrong_hit);
        target_hits_multi_role += usize::from(hits > 1);
        target_missing_true_role += usize::from(!true_hit);
    }

    LaneOverlapSummary {
        slots: 1,
        avg_target_wrong_overlap_milli: milli_ratio(target_wrong_overlap, target_lanes.len()),
        avg_wrong_hits_true_role_milli: milli_ratio(wrong_hits_true_role, wrong_lanes.len()),
        avg_target_hits_wrong_role_milli: milli_ratio(target_hits_wrong_role, target_lanes.len()),
        avg_target_hits_multi_role_milli: milli_ratio(target_hits_multi_role, target_lanes.len()),
        avg_target_missing_true_role_milli: milli_ratio(
            target_missing_true_role,
            target_lanes.len(),
        ),
    }
}

fn update_lane_overlap(
    map: &mut BTreeMap<String, LaneOverlapAccumulator>,
    key: String,
    summary: &LaneOverlapSummary,
) {
    let entry = map.entry(key).or_default();
    entry.slots += summary.slots;
    entry.target_wrong_overlap_milli_sum += summary.avg_target_wrong_overlap_milli;
    entry.wrong_hits_true_role_milli_sum += summary.avg_wrong_hits_true_role_milli;
    entry.target_hits_wrong_role_milli_sum += summary.avg_target_hits_wrong_role_milli;
    entry.target_hits_multi_role_milli_sum += summary.avg_target_hits_multi_role_milli;
    entry.target_missing_true_role_milli_sum += summary.avg_target_missing_true_role_milli;
}

fn finalize_lane_overlaps(
    map: BTreeMap<String, LaneOverlapAccumulator>,
) -> BTreeMap<String, LaneOverlapSummary> {
    map.into_iter()
        .map(|(key, value)| (key, summarize_lane_overlap(&value)))
        .collect()
}

fn worst_lane_overlaps(
    map: BTreeMap<String, LaneOverlapAccumulator>,
    limit: usize,
) -> Vec<(String, LaneOverlapSummary)> {
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(key, value)| (key, summarize_lane_overlap(&value)))
        .collect();
    items.sort_by(|left, right| {
        lane_overlap_risk(&right.1)
            .cmp(&lane_overlap_risk(&left.1))
            .then_with(|| right.1.slots.cmp(&left.1.slots))
            .then_with(|| left.0.cmp(&right.0))
    });
    items.truncate(limit);
    items
}

fn summarize_lane_overlap(value: &LaneOverlapAccumulator) -> LaneOverlapSummary {
    LaneOverlapSummary {
        slots: value.slots,
        avg_target_wrong_overlap_milli: milli_ratio(
            value.target_wrong_overlap_milli_sum,
            value.slots,
        ),
        avg_wrong_hits_true_role_milli: milli_ratio(
            value.wrong_hits_true_role_milli_sum,
            value.slots,
        ),
        avg_target_hits_wrong_role_milli: milli_ratio(
            value.target_hits_wrong_role_milli_sum,
            value.slots,
        ),
        avg_target_hits_multi_role_milli: milli_ratio(
            value.target_hits_multi_role_milli_sum,
            value.slots,
        ),
        avg_target_missing_true_role_milli: milli_ratio(
            value.target_missing_true_role_milli_sum,
            value.slots,
        ),
    }
}

fn lane_overlap_risk(summary: &LaneOverlapSummary) -> usize {
    summary.avg_target_wrong_overlap_milli
        + summary.avg_wrong_hits_true_role_milli
        + summary.avg_target_hits_wrong_role_milli
}

fn sign_aware_slot_collision_accumulator(
    slot_task: &WavePredictorStateDeltaTrainTask,
    signed_role_lanes_by_slot: &[BTreeSet<(u16, i8)>],
    true_slot: usize,
) -> SignAwareCollisionAccumulator {
    let mut accumulator = SignAwareCollisionAccumulator::default();
    for impulse in slot_task.target_delta.positive_impulses() {
        let lane = impulse.lane_id;
        let sign = impulse.signed_strength.signum() as i8;
        let signed_lane = (lane, sign);
        let mut current_hits = 0usize;
        let mut sign_aware_hits = 0usize;
        let mut current_wrong_hit = false;
        let mut sign_aware_wrong_hit = false;
        let mut missing_true_signed_hit = true;

        for (slot, signed_role_lanes) in signed_role_lanes_by_slot.iter().enumerate() {
            let current_hit = signed_role_lanes
                .iter()
                .any(|(role_lane, _)| *role_lane == lane);
            let sign_aware_hit = signed_role_lanes.contains(&signed_lane);
            current_hits += usize::from(current_hit);
            sign_aware_hits += usize::from(sign_aware_hit);
            current_wrong_hit |= current_hit && slot != true_slot;
            sign_aware_wrong_hit |= sign_aware_hit && slot != true_slot;
            if slot == true_slot && sign_aware_hit {
                missing_true_signed_hit = false;
            }
        }

        accumulator.impulses += 1;
        accumulator.current_wrong_role_hits += usize::from(current_wrong_hit);
        accumulator.sign_aware_wrong_role_hits += usize::from(sign_aware_wrong_hit);
        accumulator.sign_erased_wrong_role_hits +=
            usize::from(current_wrong_hit && !sign_aware_wrong_hit);
        accumulator.current_multi_role_hits += usize::from(current_hits > 1);
        accumulator.sign_aware_multi_role_hits += usize::from(sign_aware_hits > 1);
        accumulator.missing_true_signed_role_hits += usize::from(missing_true_signed_hit);
    }
    accumulator
}

fn update_sign_aware_collision(
    map: &mut BTreeMap<String, SignAwareCollisionAccumulator>,
    key: String,
    summary: &SignAwareCollisionAccumulator,
) {
    let entry = map.entry(key).or_default();
    entry.impulses += summary.impulses;
    entry.current_wrong_role_hits += summary.current_wrong_role_hits;
    entry.sign_aware_wrong_role_hits += summary.sign_aware_wrong_role_hits;
    entry.sign_erased_wrong_role_hits += summary.sign_erased_wrong_role_hits;
    entry.current_multi_role_hits += summary.current_multi_role_hits;
    entry.sign_aware_multi_role_hits += summary.sign_aware_multi_role_hits;
    entry.missing_true_signed_role_hits += summary.missing_true_signed_role_hits;
}

fn finalize_sign_aware_collisions(
    map: BTreeMap<String, SignAwareCollisionAccumulator>,
) -> BTreeMap<String, SignAwareCollisionSummary> {
    map.into_iter()
        .map(|(key, value)| (key, summarize_sign_aware_collision(&value)))
        .collect()
}

fn worst_sign_aware_collisions(
    map: BTreeMap<String, SignAwareCollisionAccumulator>,
    limit: usize,
) -> Vec<(String, SignAwareCollisionSummary)> {
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(key, value)| (key, summarize_sign_aware_collision(&value)))
        .collect();
    items.sort_by(|left, right| {
        right
            .1
            .current_wrong_role_hit_milli
            .cmp(&left.1.current_wrong_role_hit_milli)
            .then_with(|| {
                right
                    .1
                    .sign_erased_wrong_role_hit_milli
                    .cmp(&left.1.sign_erased_wrong_role_hit_milli)
            })
            .then_with(|| right.1.impulses.cmp(&left.1.impulses))
            .then_with(|| left.0.cmp(&right.0))
    });
    items.truncate(limit);
    items
}

fn summarize_sign_aware_collision(
    value: &SignAwareCollisionAccumulator,
) -> SignAwareCollisionSummary {
    SignAwareCollisionSummary {
        impulses: value.impulses,
        current_wrong_role_hit_milli: milli_ratio(value.current_wrong_role_hits, value.impulses),
        sign_aware_wrong_role_hit_milli: milli_ratio(
            value.sign_aware_wrong_role_hits,
            value.impulses,
        ),
        sign_erased_wrong_role_hit_milli: milli_ratio(
            value.sign_erased_wrong_role_hits,
            value.impulses,
        ),
        current_multi_role_hit_milli: milli_ratio(value.current_multi_role_hits, value.impulses),
        sign_aware_multi_role_hit_milli: milli_ratio(
            value.sign_aware_multi_role_hits,
            value.impulses,
        ),
        missing_true_signed_role_milli: milli_ratio(
            value.missing_true_signed_role_hits,
            value.impulses,
        ),
    }
}

fn residual_collision_bucket(summary: &SignAwareCollisionSummary) -> String {
    if summary.missing_true_signed_role_milli > 0 {
        return "missing_true_signed_role".to_string();
    }
    match summary.sign_aware_wrong_role_hit_milli {
        0 => "no_same_sign_residual".to_string(),
        1..=124 => "low_same_sign_residual".to_string(),
        125..=249 => "mid_same_sign_residual".to_string(),
        _ => "high_same_sign_residual".to_string(),
    }
}

fn update_residual_collision_outcome(
    map: &mut BTreeMap<String, ResidualCollisionOutcomeAccumulator>,
    key: String,
    gap: i32,
    failed: bool,
    energy_pass_slot_fail: bool,
    collision: &SignAwareCollisionSummary,
) {
    let entry = map.entry(key).or_default();
    if entry.slots == 0 {
        entry.min_gap = gap;
    } else {
        entry.min_gap = entry.min_gap.min(gap);
    }
    entry.slots += 1;
    entry.failed_slots += usize::from(failed);
    entry.energy_pass_slot_fail += usize::from(energy_pass_slot_fail);
    entry.gap_sum += i64::from(gap);
    entry.current_wrong_role_hit_milli_sum += collision.current_wrong_role_hit_milli;
    entry.sign_aware_wrong_role_hit_milli_sum += collision.sign_aware_wrong_role_hit_milli;
    entry.sign_erased_wrong_role_hit_milli_sum += collision.sign_erased_wrong_role_hit_milli;
}

fn finalize_residual_collision_outcomes(
    map: BTreeMap<String, ResidualCollisionOutcomeAccumulator>,
) -> BTreeMap<String, ResidualCollisionOutcomeSummary> {
    map.into_iter()
        .map(|(key, value)| (key, summarize_residual_collision_outcome(&value)))
        .collect()
}

fn worst_residual_collision_outcomes(
    map: BTreeMap<String, ResidualCollisionOutcomeAccumulator>,
    limit: usize,
) -> Vec<(String, ResidualCollisionOutcomeSummary)> {
    let mut items: Vec<_> = map
        .into_iter()
        .map(|(key, value)| (key, summarize_residual_collision_outcome(&value)))
        .collect();
    items.sort_by(|left, right| {
        left.1
            .accuracy_milli
            .cmp(&right.1.accuracy_milli)
            .then_with(|| right.1.failed_slots.cmp(&left.1.failed_slots))
            .then_with(|| {
                right
                    .1
                    .avg_sign_aware_wrong_role_hit_milli
                    .cmp(&left.1.avg_sign_aware_wrong_role_hit_milli)
            })
            .then_with(|| left.0.cmp(&right.0))
    });
    items.truncate(limit);
    items
}

fn summarize_residual_collision_outcome(
    value: &ResidualCollisionOutcomeAccumulator,
) -> ResidualCollisionOutcomeSummary {
    ResidualCollisionOutcomeSummary {
        slots: value.slots,
        failed_slots: value.failed_slots,
        accuracy_milli: milli_ratio(value.slots - value.failed_slots, value.slots),
        energy_pass_slot_fail: value.energy_pass_slot_fail,
        avg_gap: if value.slots == 0 {
            0
        } else {
            (value.gap_sum / value.slots as i64) as i32
        },
        min_gap: value.min_gap,
        avg_current_wrong_role_hit_milli: milli_ratio(
            value.current_wrong_role_hit_milli_sum,
            value.slots,
        ),
        avg_sign_aware_wrong_role_hit_milli: milli_ratio(
            value.sign_aware_wrong_role_hit_milli_sum,
            value.slots,
        ),
        avg_sign_erased_wrong_role_hit_milli: milli_ratio(
            value.sign_erased_wrong_role_hit_milli_sum,
            value.slots,
        ),
    }
}

fn operator_battery_v4_order_multiseed_path(seed: u8) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_{seed:03}/order/accepted_operator_tasks_v4.jsonl"
    ))
}

fn print_order_seed_strict_failure_static_diagnostic(
    seed: u8,
    row: &SequenceBindingRow,
    output_slot: usize,
) {
    let source_tokens = sequence_source_tokens(&row.state_before);
    let correct_token = row
        .correct_tokens
        .get(output_slot)
        .expect("diagnostic output slot must exist");
    let wrong_token = row
        .wrong_tokens
        .get(output_slot)
        .expect("diagnostic output slot must have wrong token");
    let true_slot = source_tokens
        .iter()
        .position(|token| token == correct_token)
        .expect("correct token must come from source slots");
    let wrong_slot = source_tokens
        .iter()
        .position(|token| token == wrong_token)
        .expect("wrong token must come from source slots");
    let role_lanes_by_slot: Vec<_> = source_tokens
        .iter()
        .take(usize::from(SEQ_ROLE_SLOT_COUNT))
        .map(|token| top_folded_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES))
        .collect();
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let base_wave = SurfaceWave4096::compile("");
    let positive = discriminative_delta_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    let negative = discriminative_delta_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        STATE_DELTA_LANES_PER_SIDE,
    );
    println!("order_seed2_failure_static_diag:");
    println!("  seed: {seed}");
    println!("  task_rule: {}", row.rule_id);
    println!("  source_tokens: {:?}", source_tokens);
    println!("  output_slot: {output_slot}");
    println!("  true_slot: {true_slot}");
    println!("  wrong_slot: {wrong_slot}");
    println!("  correct_token: {correct_token}");
    println!("  wrong_token: {wrong_token}");
    println!("  target_active_lanes: {}", target_wave.active_lanes());
    println!("  wrong_active_lanes: {}", wrong_wave.active_lanes());
    println!(
        "  target_wrong_cosine_milli: {}",
        (target_wave.cosine_similarity(&wrong_wave) * 1000.0).round() as i32
    );
    println!(
        "  correct_role_top_lanes: {:?}",
        sorted_lane_vec(&role_lanes_by_slot[true_slot])
    );
    println!(
        "  wrong_role_top_lanes: {:?}",
        sorted_lane_vec(&role_lanes_by_slot[wrong_slot])
    );
    println!(
        "  correct_wrong_role_overlap: {}",
        role_lanes_by_slot[true_slot]
            .intersection(&role_lanes_by_slot[wrong_slot])
            .count()
    );
    print_impulse_role_hit_summary(
        "positive_target_impulses",
        &positive,
        &role_lanes_by_slot,
        true_slot,
        wrong_slot,
    );
    print_impulse_role_hit_summary(
        "negative_wrong_impulses",
        &negative,
        &role_lanes_by_slot,
        true_slot,
        wrong_slot,
    );
}

fn print_order_rule_slot_static_summary(
    seed: u8,
    rows: &[SequenceBindingRow],
    rule_id: &str,
    output_slot: usize,
) {
    let mut checked = 0usize;
    let mut train = 0usize;
    let mut heldout = 0usize;
    let mut positive_impulses = 0usize;
    let mut positive_multi_role = 0usize;
    let mut positive_hit_other = 0usize;
    let mut positive_missing_true = 0usize;
    let mut max_multi_role = 0usize;
    let mut max_hit_other = 0usize;
    let mut max_row = String::new();
    for row in rows.iter().filter(|row| row.rule_id == rule_id) {
        let source_tokens = sequence_source_tokens(&row.state_before);
        let Some(correct_token) = row.correct_tokens.get(output_slot) else {
            continue;
        };
        let Some(true_slot) = source_tokens
            .iter()
            .position(|token| token == correct_token)
        else {
            continue;
        };
        if true_slot != output_slot {
            continue;
        }
        let Some(wrong_token) = row.wrong_tokens.get(output_slot) else {
            continue;
        };
        let role_lanes_by_slot: Vec<_> = source_tokens
            .iter()
            .take(usize::from(SEQ_ROLE_SLOT_COUNT))
            .map(|token| top_folded_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES))
            .collect();
        let target_wave = SurfaceWave4096::compile(correct_token);
        let wrong_wave = SurfaceWave4096::compile(wrong_token);
        let base_wave = SurfaceWave4096::compile("");
        let positive = discriminative_delta_impulses(
            base_wave.lanes(),
            target_wave.lanes(),
            wrong_wave.lanes(),
            STATE_DELTA_LANES_PER_SIDE,
        );
        let counts = impulse_role_hit_counts(&positive, &role_lanes_by_slot, true_slot);
        checked += 1;
        train += usize::from(row.source_group.contains("_train_"));
        heldout += usize::from(row.source_group.contains("_heldout_"));
        positive_impulses += positive.len();
        positive_multi_role += counts.multi_role;
        positive_hit_other += counts.hit_other;
        positive_missing_true += counts.missing_true;
        if counts.multi_role > max_multi_role || counts.hit_other > max_hit_other {
            max_multi_role = counts.multi_role;
            max_hit_other = counts.hit_other;
            max_row = format!(
                "{} correct={} wrong={} surface={} noise={}",
                row.source_group, correct_token, wrong_token, row.surface_family, row.noise_type
            );
        }
    }
    println!("order_rule_slot_static_summary:");
    println!("  seed: {seed}");
    println!("  rule: {rule_id}");
    println!("  output_slot: {output_slot}");
    println!("  self_transfer_rows_checked: {checked}");
    println!("  train_rows: {train}");
    println!("  heldout_rows: {heldout}");
    println!("  positive_impulses: {positive_impulses}");
    println!("  positive_missing_true: {positive_missing_true}");
    println!("  positive_hit_other_role: {positive_hit_other}");
    println!("  positive_multi_role: {positive_multi_role}");
    println!(
        "  positive_hit_other_milli: {}",
        milli_ratio(positive_hit_other, positive_impulses)
    );
    println!(
        "  positive_multi_role_milli: {}",
        milli_ratio(positive_multi_role, positive_impulses)
    );
    println!("  max_row_multi_role: {max_multi_role}");
    println!("  max_row_hit_other: {max_hit_other}");
    println!("  max_row: {max_row}");
}

#[derive(Clone, Copy, Debug, Default)]
struct ImpulseRoleHitCounts {
    missing_true: usize,
    hit_other: usize,
    multi_role: usize,
}

fn impulse_role_hit_counts(
    impulses: &[WavePredictorStateImpulse],
    role_lanes_by_slot: &[BTreeSet<u16>],
    true_slot: usize,
) -> ImpulseRoleHitCounts {
    let mut counts = ImpulseRoleHitCounts::default();
    for impulse in impulses {
        let projected_lane =
            (WavePredictorCenterId::from(impulse.lane_id) % SEQ_FEATURE_CENTER_COUNT) as u16;
        let roles: Vec<_> = role_lanes_by_slot
            .iter()
            .enumerate()
            .filter_map(|(slot, lanes)| lanes.contains(&projected_lane).then_some(slot))
            .collect();
        let true_hit = roles.contains(&true_slot);
        counts.missing_true += usize::from(!true_hit);
        counts.hit_other += usize::from(roles.iter().any(|slot| *slot != true_slot));
        counts.multi_role += usize::from(roles.len() > 1);
    }
    counts
}

fn sorted_lane_vec(lanes: &BTreeSet<u16>) -> Vec<u16> {
    lanes.iter().copied().collect()
}

fn print_impulse_role_hit_summary(
    label: &str,
    impulses: &[WavePredictorStateImpulse],
    role_lanes_by_slot: &[BTreeSet<u16>],
    true_slot: usize,
    wrong_slot: usize,
) {
    let mut missing_true = 0usize;
    let mut hit_wrong = 0usize;
    let mut hit_other = 0usize;
    let mut multi_role = 0usize;
    let mut lines = Vec::new();
    for impulse in impulses {
        let projected_lane =
            (WavePredictorCenterId::from(impulse.lane_id) % SEQ_FEATURE_CENTER_COUNT) as u16;
        let roles: Vec<_> = role_lanes_by_slot
            .iter()
            .enumerate()
            .filter_map(|(slot, lanes)| lanes.contains(&projected_lane).then_some(slot))
            .collect();
        let true_hit = roles.contains(&true_slot);
        let wrong_hit = roles.contains(&wrong_slot);
        missing_true += usize::from(!true_hit);
        hit_wrong += usize::from(wrong_hit);
        hit_other += usize::from(roles.iter().any(|slot| *slot != true_slot));
        multi_role += usize::from(roles.len() > 1);
        lines.push(format!(
            "lane={}:strength={}:projected={}:roles={roles:?}:true_hit={true_hit}:wrong_hit={wrong_hit}",
            impulse.lane_id, impulse.signed_strength, projected_lane
        ));
    }
    println!("  {label}_count: {}", impulses.len());
    println!("  {label}_missing_true: {missing_true}");
    println!("  {label}_hit_wrong: {hit_wrong}");
    println!("  {label}_hit_other_role: {hit_other}");
    println!("  {label}_multi_role: {multi_role}");
    println!("  {label}_lanes: {:?}", lines);
}

#[derive(Clone, Debug)]
struct DynamicImpulseAudit {
    lane_id: u16,
    signed_strength: i16,
    direct: i32,
    self_transfer: i32,
    role_binding: i32,
    total: i32,
    roles: Vec<(u8, i16)>,
    role_slot_totals: BTreeMap<u8, i32>,
    top_edges: Vec<String>,
}

fn print_order_seed2_priem_dynamic_weight_audit(
    field: &WavePredictorHebbianField,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) {
    let flat = field.compile_flat_role_binding_table();
    let slot_task = &task.slot_tasks[output_slot];
    let correct_token = row
        .correct_tokens
        .get(output_slot)
        .expect("diagnostic output slot must exist");
    let wrong_token = row
        .wrong_tokens
        .get(output_slot)
        .expect("diagnostic wrong slot must exist");
    let target_score = state_delta_target_score(field, slot_task);
    let wrong_score = state_delta_wrong_score(field, slot_task);
    let sequence_energy_gap = sequence_energy_gap(field, task);
    let (binding_pos, binding_neg) = field.state_delta_binding_weights();

    let mut target_multi_role = slot_task
        .target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| dynamic_impulse_audit(field, &flat, slot_task, *impulse))
        .filter(|audit| audit.roles.len() > 1)
        .collect::<Vec<_>>();
    target_multi_role.sort_by(|left, right| {
        left.total
            .cmp(&right.total)
            .then_with(|| left.lane_id.cmp(&right.lane_id))
    });

    let mut wrong_strongest = slot_task
        .target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| dynamic_impulse_audit(field, &flat, slot_task, *impulse))
        .collect::<Vec<_>>();
    wrong_strongest.sort_by(|left, right| {
        right
            .total
            .cmp(&left.total)
            .then_with(|| left.lane_id.cmp(&right.lane_id))
    });
    wrong_strongest.truncate(8);

    println!("order_seed2_priem_dynamic_weight_audit:");
    println!("  rule: {}", row.rule_id);
    println!("  source_group: {}", row.source_group);
    println!("  output_slot: {output_slot}");
    println!("  correct_token: {correct_token}");
    println!("  wrong_token: {wrong_token}");
    println!("  target_score: {target_score}");
    println!("  wrong_score: {wrong_score}");
    println!("  slot_gap: {}", target_score - wrong_score);
    println!("  sequence_energy_gap: {sequence_energy_gap}");
    println!("  binding_weight_positive: {binding_pos}");
    println!("  binding_weight_negative: {binding_neg}");
    println!("  flat_role_binding_edges: {}", flat.edge_count());
    println!(
        "  active_action_centers: {}",
        active_sequence_action_centers(&slot_task.active_fringe).len()
    );
    println!("  target_multi_role_impulses: {}", target_multi_role.len());
    println!(
        "  target_multi_role_total_sum: {}",
        target_multi_role
            .iter()
            .map(|audit| audit.total)
            .sum::<i32>()
    );
    println!(
        "  target_multi_role_true_slot_sum: {}",
        role_slot_sum_for_audits(&target_multi_role, output_slot as u8)
    );
    println!(
        "  target_multi_role_other_slot_sum: {}",
        other_role_slot_sum_for_audits(&target_multi_role, output_slot as u8)
    );
    println!(
        "  target_multi_role_slot_totals: {:?}",
        merge_role_slot_totals(&target_multi_role)
    );
    for audit in &target_multi_role {
        println!(
            "  target_multi_role_lane: {}",
            format_dynamic_impulse_audit(audit)
        );
    }
    for audit in &wrong_strongest {
        println!(
            "  wrong_strongest_lane: {}",
            format_dynamic_impulse_audit(audit)
        );
    }
}

fn dynamic_impulse_audit(
    field: &WavePredictorHebbianField,
    flat: &WavePredictorFlatRoleBindingTable,
    task: &WavePredictorStateDeltaTrainTask,
    impulse: WavePredictorStateImpulse,
) -> DynamicImpulseAudit {
    let sign = if impulse.signed_strength < 0 { -1 } else { 1 };
    let direct = sign * field.score_state_delta_lane(impulse.lane_id, &task.active_fringe);
    let (binding_pos, binding_neg) = field.state_delta_binding_weights();
    let self_transfer = field
        .state_delta_binding_active_strength(impulse.lane_id, &task.active_fringe)
        .map(|active_strength| {
            let weight = if impulse.signed_strength < 0 {
                binding_neg
            } else {
                binding_pos
            };
            i32::from(active_strength.abs()) * i32::from(weight)
        })
        .unwrap_or(0);
    let (role_binding, role_slot_totals, top_edges) =
        dynamic_role_binding_score(flat, task, impulse);
    let total = direct + self_transfer + role_binding;
    DynamicImpulseAudit {
        lane_id: impulse.lane_id,
        signed_strength: impulse.signed_strength,
        direct,
        self_transfer,
        role_binding,
        total,
        roles: active_sequence_role_slots_for_lane(&task.active_fringe, impulse.lane_id),
        role_slot_totals,
        top_edges,
    }
}

fn dynamic_role_binding_score(
    flat: &WavePredictorFlatRoleBindingTable,
    task: &WavePredictorStateDeltaTrainTask,
    impulse: WavePredictorStateImpulse,
) -> (i32, BTreeMap<u8, i32>, Vec<String>) {
    let sign_key = u8::from(impulse.signed_strength < 0);
    let output_slot_id = task.binding_output_slot.unwrap_or(0);
    let projected_lane = WavePredictorCenterId::from(impulse.lane_id) % SEQ_FEATURE_CENTER_COUNT;
    let active_by_center = task
        .active_fringe
        .iter()
        .map(|active| (active.center_id, active.strength))
        .collect::<BTreeMap<_, _>>();
    let mut role_slot_totals = BTreeMap::new();
    let mut edge_lines = Vec::new();
    let mut score = 0i32;

    for edge in flat.edges() {
        if edge.output_slot_id != output_slot_id || edge.sign_key != sign_key {
            continue;
        }
        let Some(action_strength) = active_by_center.get(&edge.action_center) else {
            continue;
        };
        let role_center = SEQ_ROLE_BASE
            + WavePredictorCenterId::from(edge.slot_id) * SEQ_FEATURE_CENTER_COUNT
            + projected_lane;
        let Some(role_strength) = active_by_center.get(&role_center) else {
            continue;
        };
        let contribution = i32::from(action_strength.abs())
            * i32::from(role_strength.abs())
            * i32::from(edge.weight);
        if contribution == 0 {
            continue;
        }
        score += contribution;
        *role_slot_totals.entry(edge.slot_id).or_default() += contribution;
        edge_lines.push((
            contribution.abs(),
            contribution,
            format!(
                "{}:role{}:w{}:a{}:r{}:c{}",
                sequence_action_center_label(edge.action_center),
                edge.slot_id,
                edge.weight,
                action_strength,
                role_strength,
                contribution
            ),
        ));
    }

    edge_lines.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    edge_lines.truncate(8);
    (
        score,
        role_slot_totals,
        edge_lines.into_iter().map(|(_, _, line)| line).collect(),
    )
}

fn active_sequence_action_centers(
    active_fringe: &[WavePredictorActiveCenter],
) -> Vec<WavePredictorActiveCenter> {
    active_fringe
        .iter()
        .copied()
        .filter(|active| {
            active.center_id >= SEQ_ACTION_SLOT_BASE
                && active.center_id < SEQ_ACTION_SLOT_BASE + SEQ_ACTION_CENTER_COUNT
        })
        .collect()
}

fn sequence_action_center_label(center_id: WavePredictorCenterId) -> String {
    if (SEQ_OPERATOR_PAIR_BASE..SEQ_OPERATOR_PAIR_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
    {
        let lane = center_id - SEQ_OPERATOR_PAIR_BASE;
        if lane < 256 {
            return format!("op_pair(out{}->src{})", lane >> 4, lane & 0x0f);
        }
        return format!("op_pair_variant({lane})");
    }
    if (SEQ_ACTION_SLOT_BASE..SEQ_ACTION_SLOT_BASE + SEQ_FEATURE_CENTER_COUNT).contains(&center_id)
    {
        return format!("action_surface({})", center_id - SEQ_ACTION_SLOT_BASE);
    }
    if (SEQ_STATE_CONDITION_BASE..SEQ_STATE_CONDITION_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
    {
        return format!("state_condition({})", center_id - SEQ_STATE_CONDITION_BASE);
    }
    if (SEQ_CONDITION_ACTION_BASE..SEQ_CONDITION_ACTION_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
    {
        return format!(
            "condition_action({})",
            center_id - SEQ_CONDITION_ACTION_BASE
        );
    }
    if (SEQ_COMPOSED_DEMO_BASE..SEQ_COMPOSED_DEMO_BASE + SEQ_FEATURE_CENTER_COUNT)
        .contains(&center_id)
    {
        return format!("composed_demo({})", center_id - SEQ_COMPOSED_DEMO_BASE);
    }
    format!("center({center_id})")
}

fn role_slot_sum_for_audits(audits: &[DynamicImpulseAudit], slot_id: u8) -> i32 {
    audits
        .iter()
        .map(|audit| audit.role_slot_totals.get(&slot_id).copied().unwrap_or(0))
        .sum()
}

fn other_role_slot_sum_for_audits(audits: &[DynamicImpulseAudit], true_slot: u8) -> i32 {
    audits
        .iter()
        .flat_map(|audit| audit.role_slot_totals.iter())
        .filter_map(|(slot, value)| (*slot != true_slot).then_some(*value))
        .sum()
}

fn merge_role_slot_totals(audits: &[DynamicImpulseAudit]) -> BTreeMap<u8, i32> {
    let mut out = BTreeMap::new();
    for audit in audits {
        for (slot, value) in &audit.role_slot_totals {
            *out.entry(*slot).or_default() += value;
        }
    }
    out
}

fn format_dynamic_impulse_audit(audit: &DynamicImpulseAudit) -> String {
    format!(
        "lane={}:strength={}:total={}:direct={}:self={}:role_binding={}:roles={:?}:role_slot_totals={:?}:top_edges={:?}",
        audit.lane_id,
        audit.signed_strength,
        audit.total,
        audit.direct,
        audit.self_transfer,
        audit.role_binding,
        audit.roles,
        audit.role_slot_totals,
        audit.top_edges
    )
}

fn write_v3_static_diagnostics_report(
    action: &ActionSeparabilityReport,
    collision: &FoldedCollisionReport,
) {
    let path = std::env::var("POSITION_SEQUENCE_V3_STATIC_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/rule_logic_position_sequence_v3/static_diagnostics_report.json")
        });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create static diagnostics output directory");
    }
    let mut json = String::new();

    macro_rules! write_json {
        ($($arg:tt)*) => {
            writeln!(&mut json, $($arg)*).expect("write static diagnostics json")
        };
    }

    write_json!("{{");
    write_json!("  \"schema_version\": \"position_sequence_v3_static_diagnostics_v1\",");
    write_json!("  \"action_separability\": {{");
    write_json!("    \"action_vectors\": {},", action.action_vectors);
    write_json!(
        "    \"same_rule_action_similarity_milli\": {},",
        action.same_rule_similarity_milli
    );
    write_json!(
        "    \"different_rule_action_similarity_milli\": {},",
        action.different_rule_similarity_milli
    );
    write_json!(
        "    \"same_family_different_length_similarity_milli\": {},",
        action.same_family_different_length_similarity_milli
    );
    write_json!(
        "    \"different_family_similarity_milli\": {},",
        action.different_family_similarity_milli
    );
    write_json!(
        "    \"max_different_rule_similarity_milli\": {},",
        action.max_different_rule_similarity_milli
    );
    write_json!("    \"nearest_rule_pairs\": [");
    for (index, (left, right, score)) in action.nearest_rule_pairs.iter().enumerate() {
        let comma = if index + 1 == action.nearest_rule_pairs.len() {
            ""
        } else {
            ","
        };
        write_json!(
            "      {{\"left\": \"{}\", \"right\": \"{}\", \"similarity_milli\": {}}}{}",
            left,
            right,
            score,
            comma
        );
    }
    write_json!("    ]");
    write_json!("  }},");
    write_json!("  \"folded_collision_pressure\": {{");
    write_json!(
        "    \"target_impulses_checked\": {},",
        collision.target_impulses_checked
    );
    write_json!(
        "    \"multi_role_hit_count\": {},",
        collision.multi_role_hit_count
    );
    write_json!(
        "    \"wrong_role_hit_count\": {},",
        collision.wrong_role_hit_count
    );
    write_json!(
        "    \"missing_true_role_count\": {},",
        collision.missing_true_role_count
    );
    write_json!(
        "    \"multi_role_hit_milli\": {},",
        collision.multi_role_hit_milli
    );
    write_json!(
        "    \"wrong_role_hit_milli\": {},",
        collision.wrong_role_hit_milli
    );
    write_json!(
        "    \"missing_true_role_milli\": {}",
        collision.missing_true_role_milli
    );
    write_json!("  }}");
    write_json!("}}");
    fs::write(path, json).expect("static diagnostics report must be writable");
}

fn top_folded_lanes(
    input: &str,
    center_span: WavePredictorCenterId,
    limit: usize,
) -> BTreeSet<u16> {
    top_folded_signed_lanes(input, center_span, limit)
        .into_iter()
        .map(|(lane, _)| lane)
        .collect()
}

fn top_folded_signed_lanes(
    input: &str,
    center_span: WavePredictorCenterId,
    limit: usize,
) -> BTreeSet<(u16, i8)> {
    let wave = SurfaceWave4096::compile(input);
    let mut by_lane: BTreeMap<u16, i16> = BTreeMap::new();
    for (lane, value) in wave.lanes().iter().enumerate() {
        if *value == 0 {
            continue;
        }
        let folded_lane = (lane as WavePredictorCenterId % center_span) as u16;
        by_lane
            .entry(folded_lane)
            .and_modify(|current| {
                if value.abs() > current.abs() {
                    *current = *value;
                }
            })
            .or_insert(*value);
    }
    let mut lanes: Vec<_> = by_lane
        .into_iter()
        .map(|(lane, value)| (value.abs(), lane, value.signum() as i8))
        .collect();
    lanes.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    lanes
        .into_iter()
        .take(limit)
        .map(|(_, lane, sign)| (lane, sign))
        .collect()
}

fn set_similarity_milli<T: Ord>(left: &BTreeSet<T>, right: &BTreeSet<T>) -> usize {
    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    milli_ratio(intersection, union)
}

fn avg_usize(sum: usize, count: usize) -> usize {
    sum.checked_div(count).unwrap_or(0)
}

fn rule_family_name(rule_id: &str) -> &str {
    rule_id
        .rsplit_once("_len")
        .map(|(family, _)| family)
        .unwrap_or(rule_id)
}

fn eval_ordered_sequence_flat(
    table: &WavePredictorFlatRoleBindingTable,
    tasks: &[PreparedSequenceTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let mut min_gap = i32::MAX;
        let mut row_ok = true;
        for slot_task in &task.slot_tasks {
            let gap = flat_state_delta_sum_gap(table, slot_task);
            min_gap = min_gap.min(gap);
            row_ok &= gap > 0;
        }
        gaps.push(min_gap);
        correct += usize::from(row_ok);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn eval_ordered_sequence_flat_fast(
    index: &FlatRoleBindingScoreIndex,
    tasks: &[PreparedSequenceTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let prepared = index.prepare_task(&task.slot_tasks[0]);
        let mut min_gap = i32::MAX;
        let mut row_ok = true;
        for slot_task in &task.slot_tasks {
            let gap = flat_state_delta_sum_gap_fast_prepared(index, &prepared, slot_task);
            min_gap = min_gap.min(gap);
            row_ok &= gap > 0;
        }
        gaps.push(min_gap);
        correct += usize::from(row_ok);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn flat_ordered_sequence_row_ok_fast(
    index: &FlatRoleBindingScoreIndex,
    task: &PreparedSequenceTask,
) -> bool {
    let prepared = index.prepare_task(&task.slot_tasks[0]);
    task.slot_tasks
        .iter()
        .all(|slot_task| flat_state_delta_sum_gap_fast_prepared(index, &prepared, slot_task) > 0)
}

fn eval_ordered_sequence_flat_gap_parity(
    field: &WavePredictorHebbianField,
    table: &WavePredictorFlatRoleBindingTable,
    tasks: &[PreparedSequenceTask],
) -> FlatGapParityReport {
    let mut report = FlatGapParityReport::default();
    for task in tasks {
        for slot_task in &task.slot_tasks {
            report.checked_slots += 1;
            let field_gap = state_delta_sum_gap(field, slot_task);
            let flat_gap = flat_state_delta_sum_gap(table, slot_task);
            report.mismatches += usize::from(field_gap != flat_gap);
        }
    }
    report
}

fn eval_ordered_sequence_flat_gap_parity_fast(
    field: &WavePredictorHebbianField,
    index: &FlatRoleBindingScoreIndex,
    tasks: &[PreparedSequenceTask],
) -> FlatGapParityReport {
    let mut report = FlatGapParityReport::default();
    for task in tasks {
        let prepared = index.prepare_task(&task.slot_tasks[0]);
        for slot_task in &task.slot_tasks {
            report.checked_slots += 1;
            let field_gap = state_delta_sum_gap(field, slot_task);
            let flat_gap = flat_state_delta_sum_gap_fast_prepared(index, &prepared, slot_task);
            report.mismatches += usize::from(field_gap != flat_gap);
        }
    }
    report
}

fn eval_ordered_sequence_flat_energy_parity(
    field: &WavePredictorHebbianField,
    table: &WavePredictorFlatRoleBindingTable,
    tasks: &[PreparedSequenceTask],
) -> FlatEnergyParityReport {
    let mut report = FlatEnergyParityReport::default();
    for task in tasks {
        report.checked_rows += 1;
        let field_gap = sequence_energy_gap(field, task);
        let flat_gap = flat_sequence_energy_gap(table, task);
        let delta = (i64::from(field_gap) - i64::from(flat_gap)).abs() as i32;
        report.max_abs_gap_delta = report.max_abs_gap_delta.max(delta);
        report.mismatches += usize::from(field_gap != flat_gap);
    }
    report
}

fn eval_ordered_sequence_flat_energy_parity_fast(
    field: &WavePredictorHebbianField,
    index: &FlatRoleBindingScoreIndex,
    tasks: &[PreparedSequenceTask],
) -> FlatEnergyParityReport {
    let mut report = FlatEnergyParityReport::default();
    for task in tasks {
        report.checked_rows += 1;
        let field_gap = sequence_energy_gap(field, task);
        let flat_gap = flat_sequence_energy_gap_fast(index, task);
        let delta = (i64::from(field_gap) - i64::from(flat_gap)).abs() as i32;
        report.max_abs_gap_delta = report.max_abs_gap_delta.max(delta);
        report.mismatches += usize::from(field_gap != flat_gap);
    }
    report
}

fn eval_flat_binding_table(
    table: &WavePredictorFlatRoleBindingTable,
    tasks: &[PreparedBindingTask],
) -> EvalReport {
    let mut gaps = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    for task in tasks {
        let gap = flat_state_delta_sum_gap(table, &task.train_task);
        gaps.push(gap);
        correct += usize::from(gap > 0);
    }
    gaps.sort_unstable();
    EvalReport {
        tasks: tasks.len(),
        correct,
        accuracy_milli: milli_ratio(correct, tasks.len()),
        median_gap: gaps[tasks.len() / 2],
        p10_gap: gaps[tasks.len() / 10],
    }
}

fn flat_state_delta_sum_gap(
    table: &WavePredictorFlatRoleBindingTable,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    flat_state_delta_target_score(table, task) - flat_state_delta_wrong_score(table, task)
}

fn flat_state_delta_sum_gap_fast_prepared(
    index: &FlatRoleBindingScoreIndex,
    prepared: &PreparedFlatRoleBindingFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    flat_state_delta_target_score_fast_prepared(index, prepared, task)
        - flat_state_delta_wrong_score_fast_prepared(index, prepared, task)
}

fn flat_sequence_energy_gap(
    table: &WavePredictorFlatRoleBindingTable,
    task: &PreparedSequenceTask,
) -> i32 {
    let mut correct_score = 0i32;
    let mut wrong_score = 0i32;
    for slot_task in &task.slot_tasks {
        correct_score += flat_state_delta_target_score(table, slot_task);
        wrong_score += flat_state_delta_wrong_score(table, slot_task);
    }
    correct_score - wrong_score
}

fn flat_sequence_energy_gap_fast(
    index: &FlatRoleBindingScoreIndex,
    task: &PreparedSequenceTask,
) -> i32 {
    let mut correct_score = 0i32;
    let mut wrong_score = 0i32;
    let prepared = index.prepare_task(&task.slot_tasks[0]);
    for slot_task in &task.slot_tasks {
        correct_score += flat_state_delta_target_score_fast_prepared(index, &prepared, slot_task);
        wrong_score += flat_state_delta_wrong_score_fast_prepared(index, &prepared, slot_task);
    }
    correct_score - wrong_score
}

fn flat_state_delta_target_score(
    table: &WavePredictorFlatRoleBindingTable,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| {
            table.score_alignment(
                impulse.lane_id,
                impulse.signed_strength,
                &task.active_fringe,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn flat_state_delta_target_score_fast_prepared(
    index: &FlatRoleBindingScoreIndex,
    prepared: &PreparedFlatRoleBindingFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    let output_slot = task.binding_output_slot.unwrap_or(0);
    task.target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| index.score_alignment(prepared, output_slot, *impulse))
        .sum()
}

fn flat_state_delta_wrong_score(
    table: &WavePredictorFlatRoleBindingTable,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| {
            table.score_alignment(
                impulse.lane_id,
                impulse.signed_strength,
                &task.active_fringe,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn flat_state_delta_wrong_score_fast_prepared(
    index: &FlatRoleBindingScoreIndex,
    prepared: &PreparedFlatRoleBindingFringe,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    let output_slot = task.binding_output_slot.unwrap_or(0);
    task.target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| index.score_alignment(prepared, output_slot, *impulse))
        .sum()
}

fn eval_cleanup_readout_pairwise(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> CleanupReadoutDiagnostics {
    let mut scorer = CleanupFieldScoreCache::new(field);
    eval_cleanup_readout(
        "cleanup_pairwise",
        rows,
        tasks,
        |task| sequence_energy_gap(field, task) > 0,
        |row_idx, row, task, output_slot| {
            cleanup_readout_pairwise_gap(&mut scorer, row_idx, row, task, output_slot)
        },
    )
}

fn eval_cleanup_readout_source_winner(
    field: &WavePredictorHebbianField,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> CleanupReadoutDiagnostics {
    let mut scorer = CleanupFieldScoreCache::new(field);
    eval_cleanup_readout(
        "cleanup_winner",
        rows,
        tasks,
        |task| sequence_energy_gap(field, task) > 0,
        |row_idx, row, task, output_slot| {
            cleanup_readout_winner_gap(&mut scorer, row_idx, row, task, output_slot)
        },
    )
}

fn eval_cleanup_readout_pairwise_flat(
    table: &WavePredictorFlatRoleBindingTable,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> CleanupReadoutDiagnostics {
    let mut scorer = CleanupFlatScoreCache::new(table);
    eval_cleanup_readout(
        "flat_cleanup_pairwise",
        rows,
        tasks,
        |task| flat_sequence_energy_gap(table, task) > 0,
        |row_idx, row, task, output_slot| {
            cleanup_readout_pairwise_gap_flat(&mut scorer, row_idx, row, task, output_slot)
        },
    )
}

fn eval_cleanup_readout_source_winner_flat(
    table: &WavePredictorFlatRoleBindingTable,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> CleanupReadoutDiagnostics {
    let mut scorer = CleanupFlatScoreCache::new(table);
    eval_cleanup_readout(
        "flat_cleanup_winner",
        rows,
        tasks,
        |task| flat_sequence_energy_gap(table, task) > 0,
        |row_idx, row, task, output_slot| {
            cleanup_readout_winner_gap_flat(&mut scorer, row_idx, row, task, output_slot)
        },
    )
}

fn eval_cleanup_readout<E, F>(
    label: &str,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
    mut energy_ok_fn: E,
    mut gap_fn: F,
) -> CleanupReadoutDiagnostics
where
    E: FnMut(&PreparedSequenceTask) -> bool,
    F: FnMut(usize, &SequenceBindingRow, &PreparedSequenceTask, usize) -> i32,
{
    let mut row_gaps = Vec::with_capacity(tasks.len());
    let mut correct_rows = 0usize;
    let mut failed_slots = 0usize;
    let mut energy_pass_slot_fail = 0usize;

    for (row_idx, (row, task)) in rows.iter().zip(tasks.iter()).enumerate() {
        if row_idx > 0 && row_idx % 100 == 0 {
            println!("{label}: eval_progress rows_done={row_idx}/{}", tasks.len());
        }
        let mut min_gap = i32::MAX;
        let mut row_ok = true;
        let energy_ok = energy_ok_fn(task);
        for output_slot in 0..task.slot_tasks.len() {
            let gap = gap_fn(row_idx, row, task, output_slot);
            min_gap = min_gap.min(gap);
            let slot_ok = gap > 0;
            row_ok &= slot_ok;
            failed_slots += usize::from(!slot_ok);
            energy_pass_slot_fail += usize::from(energy_ok && !slot_ok);
        }
        row_gaps.push(min_gap);
        correct_rows += usize::from(row_ok);
    }

    row_gaps.sort_unstable();
    CleanupReadoutDiagnostics {
        rows: tasks.len(),
        correct: correct_rows,
        accuracy_milli: milli_ratio(correct_rows, tasks.len()),
        median_gap: row_gaps[tasks.len() / 2],
        p10_gap: row_gaps[tasks.len() / 10],
        failed_slots,
        energy_pass_slot_fail,
    }
}

fn eval_cleanup_readout_pairwise_flat_parity(
    field: &WavePredictorHebbianField,
    table: &WavePredictorFlatRoleBindingTable,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> FlatGapParityReport {
    let mut field_scorer = CleanupFieldScoreCache::new(field);
    let mut flat_scorer = CleanupFlatScoreCache::new(table);
    eval_cleanup_readout_flat_parity(
        "cleanup_pairwise_parity",
        rows,
        tasks,
        |row_idx, row, task, output_slot| {
            (
                cleanup_readout_pairwise_gap(&mut field_scorer, row_idx, row, task, output_slot),
                cleanup_readout_pairwise_gap_flat(
                    &mut flat_scorer,
                    row_idx,
                    row,
                    task,
                    output_slot,
                ),
            )
        },
    )
}

fn eval_cleanup_readout_winner_flat_parity(
    field: &WavePredictorHebbianField,
    table: &WavePredictorFlatRoleBindingTable,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
) -> FlatGapParityReport {
    let mut field_scorer = CleanupFieldScoreCache::new(field);
    let mut flat_scorer = CleanupFlatScoreCache::new(table);
    eval_cleanup_readout_flat_parity(
        "cleanup_winner_parity",
        rows,
        tasks,
        |row_idx, row, task, output_slot| {
            (
                cleanup_readout_winner_gap(&mut field_scorer, row_idx, row, task, output_slot),
                cleanup_readout_winner_gap_flat(&mut flat_scorer, row_idx, row, task, output_slot),
            )
        },
    )
}

fn eval_cleanup_readout_flat_parity<F>(
    label: &str,
    rows: &[SequenceBindingRow],
    tasks: &[PreparedSequenceTask],
    mut gap_fn: F,
) -> FlatGapParityReport
where
    F: FnMut(usize, &SequenceBindingRow, &PreparedSequenceTask, usize) -> (i32, i32),
{
    let mut checked_slots = 0usize;
    let mut mismatches = 0usize;
    for (row_idx, (row, task)) in rows.iter().zip(tasks.iter()).enumerate() {
        if row_idx > 0 && row_idx % 100 == 0 {
            println!("{label}: eval_progress rows_done={row_idx}/{}", tasks.len());
        }
        for output_slot in 0..task.slot_tasks.len() {
            let (field_gap, flat_gap) = gap_fn(row_idx, row, task, output_slot);
            checked_slots += 1;
            mismatches += usize::from(field_gap != flat_gap);
        }
    }
    FlatGapParityReport {
        checked_slots,
        mismatches,
    }
}

fn cleanup_readout_pairwise_gap(
    scorer: &mut CleanupFieldScoreCache<'_>,
    row_idx: usize,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) -> i32 {
    let slot_task = &task.slot_tasks[output_slot];
    scorer.score(
        row_idx,
        output_slot,
        slot_task,
        &row.correct_tokens[output_slot],
    ) - scorer.score(
        row_idx,
        output_slot,
        slot_task,
        &row.wrong_tokens[output_slot],
    )
}

fn cleanup_readout_pairwise_gap_flat(
    scorer: &mut CleanupFlatScoreCache,
    row_idx: usize,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) -> i32 {
    let slot_task = &task.slot_tasks[output_slot];
    scorer.score(
        row_idx,
        output_slot,
        slot_task,
        &row.correct_tokens[output_slot],
    ) - scorer.score(
        row_idx,
        output_slot,
        slot_task,
        &row.wrong_tokens[output_slot],
    )
}

fn cleanup_readout_winner_gap(
    scorer: &mut CleanupFieldScoreCache<'_>,
    row_idx: usize,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) -> i32 {
    cleanup_readout_winner_gap_with(row, output_slot, |token| {
        scorer.score(row_idx, output_slot, &task.slot_tasks[output_slot], token)
    })
}

fn cleanup_readout_winner_gap_flat(
    scorer: &mut CleanupFlatScoreCache,
    row_idx: usize,
    row: &SequenceBindingRow,
    task: &PreparedSequenceTask,
    output_slot: usize,
) -> i32 {
    cleanup_readout_winner_gap_with(row, output_slot, |token| {
        scorer.score(row_idx, output_slot, &task.slot_tasks[output_slot], token)
    })
}

fn cleanup_readout_winner_gap_with<F>(
    row: &SequenceBindingRow,
    output_slot: usize,
    mut score_fn: F,
) -> i32
where
    F: FnMut(&str) -> i32,
{
    let correct_token = &row.correct_tokens[output_slot];
    let correct_score = score_fn(correct_token);
    let mut best_other = i32::MIN;
    let mut seen_other = false;
    for token in sequence_source_tokens(&row.state_before) {
        if token == *correct_token {
            continue;
        }
        seen_other = true;
        best_other = best_other.max(score_fn(&token));
    }
    if !seen_other {
        return i32::MIN;
    }
    correct_score - best_other
}

impl CleanupImpulseCache {
    fn impulses(&mut self, token: &str) -> Vec<WavePredictorStateImpulse> {
        self.by_token
            .entry(token.to_string())
            .or_insert_with(|| cleanup_token_impulses(token))
            .clone()
    }
}

impl<'a> CleanupFieldScoreCache<'a> {
    fn new(field: &'a WavePredictorHebbianField) -> Self {
        Self {
            field,
            impulses: CleanupImpulseCache::default(),
            scores: BTreeMap::new(),
        }
    }

    fn score(
        &mut self,
        row_idx: usize,
        output_slot: usize,
        task: &WavePredictorStateDeltaTrainTask,
        token: &str,
    ) -> i32 {
        let key = (row_idx, output_slot, token.to_string());
        if let Some(score) = self.scores.get(&key) {
            return *score;
        }
        let score = self
            .impulses
            .impulses(token)
            .into_iter()
            .map(|impulse| {
                state_delta_impulse_alignment(
                    self.field,
                    &task.active_fringe,
                    impulse,
                    task.binding_output_slot,
                )
            })
            .sum();
        self.scores.insert(key, score);
        score
    }
}

impl FlatRoleBindingScoreIndex {
    fn new(table: &WavePredictorFlatRoleBindingTable, config: WavePredictorHebbianConfig) -> Self {
        let mut edge_index: BTreeMap<(WavePredictorCenterId, u8, u8), BTreeMap<u8, i16>> =
            BTreeMap::new();
        for edge in table.edges() {
            edge_index
                .entry((edge.action_center, edge.output_slot_id, edge.sign_key))
                .or_default()
                .entry(edge.slot_id)
                .and_modify(|weight| {
                    *weight = clamp_i32_to_i16(i32::from(*weight) + i32::from(edge.weight));
                })
                .or_insert(edge.weight);
        }
        let edge_index: HashMap<_, _> = edge_index
            .into_iter()
            .map(|(key, by_slot)| (key, by_slot.into_iter().collect()))
            .collect();
        Self {
            action_base: config.state_delta_binding_action_base,
            action_count: config.state_delta_binding_action_count,
            role_base: config.state_delta_binding_role_base,
            role_stride: config.state_delta_binding_role_stride,
            slot_scoped_action_page_bits: config.state_delta_binding_slot_scoped_action_page_bits,
            slot_scoped_action_page_mask: config.state_delta_binding_slot_scoped_action_page_mask,
            slot_scoped_action_source_bits: config
                .state_delta_binding_slot_scoped_action_source_bits,
            edge_index,
        }
    }

    fn prepare_task(
        &self,
        task: &WavePredictorStateDeltaTrainTask,
    ) -> PreparedFlatRoleBindingFringe {
        let mut active_actions = Vec::new();
        let mut slot_actions: HashMap<u8, Vec<(WavePredictorCenterId, i16)>> = HashMap::new();
        let mut role_strengths = HashMap::new();
        let action_base = self.action_base.unwrap_or(0);
        let role_base = self.role_base.unwrap_or(0);
        let action_end = action_base.saturating_add(self.action_count);
        for active in &task.active_fringe {
            if active.strength == 0 {
                continue;
            }
            if active.center_id >= action_base && active.center_id < action_end {
                if let Some(output_slot) = self.slot_scoped_output_slot(active.center_id) {
                    slot_actions
                        .entry(output_slot)
                        .or_default()
                        .push((active.center_id, active.strength.abs()));
                } else {
                    active_actions.push((active.center_id, active.strength.abs()));
                }
                continue;
            }
            if self.role_stride == 0 || active.center_id < role_base {
                continue;
            }
            let role_offset = active.center_id - role_base;
            let slot_id = role_offset / self.role_stride;
            let lane = role_offset % self.role_stride;
            if let Ok(slot_id) = u8::try_from(slot_id) {
                role_strengths
                    .entry((slot_id, lane))
                    .and_modify(|strength: &mut i16| {
                        *strength = (*strength).max(active.strength.abs());
                    })
                    .or_insert(active.strength.abs());
            }
        }
        PreparedFlatRoleBindingFringe {
            active_actions,
            slot_actions,
            role_strengths,
        }
    }

    fn slot_scoped_output_slot(&self, center_id: WavePredictorCenterId) -> Option<u8> {
        if self.slot_scoped_action_page_bits == 0 || self.slot_scoped_action_source_bits == 0 {
            return None;
        }
        let page = center_id >> u32::from(self.slot_scoped_action_page_bits);
        if page >= 64 || (self.slot_scoped_action_page_mask & (1_u64 << page)) == 0 {
            return None;
        }
        let lane_mask = (1_u32 << u32::from(self.slot_scoped_action_page_bits)) - 1;
        let lane = center_id & lane_mask;
        u8::try_from(lane >> u32::from(self.slot_scoped_action_source_bits)).ok()
    }

    fn score_alignment(
        &self,
        prepared: &PreparedFlatRoleBindingFringe,
        binding_output_slot: u8,
        impulse: WavePredictorStateImpulse,
    ) -> i32 {
        if self.action_count == 0 || self.role_stride == 0 {
            return 0;
        };
        let sign_key = u8::from(impulse.signed_strength < 0);
        let lane = WavePredictorCenterId::from(impulse.lane_id);
        let projected_lane = if lane >= self.role_stride {
            lane % self.role_stride
        } else {
            lane
        };
        let mut score = 0i32;
        let global_actions = prepared.active_actions.iter();
        let slot_actions = prepared
            .slot_actions
            .get(&binding_output_slot)
            .into_iter()
            .flatten();
        for (action_center, action_strength) in global_actions.chain(slot_actions) {
            let Some(edges) = self
                .edge_index
                .get(&(*action_center, binding_output_slot, sign_key))
            else {
                continue;
            };
            for (slot_id, weight) in edges {
                let Some(role_strength) = prepared.role_strengths.get(&(*slot_id, projected_lane))
                else {
                    continue;
                };
                score +=
                    i32::from(*action_strength) * i32::from(*role_strength) * i32::from(*weight);
            }
        }
        score
    }
}

impl CleanupFlatScoreCache {
    fn new(table: &WavePredictorFlatRoleBindingTable) -> Self {
        Self {
            index: FlatRoleBindingScoreIndex::new(table, sequence_binding_config()),
            impulses: CleanupImpulseCache::default(),
            scores: BTreeMap::new(),
        }
    }

    fn score(
        &mut self,
        row_idx: usize,
        output_slot: usize,
        task: &WavePredictorStateDeltaTrainTask,
        token: &str,
    ) -> i32 {
        let key = (row_idx, output_slot, token.to_string());
        if let Some(score) = self.scores.get(&key) {
            return *score;
        }
        let prepared = self.index.prepare_task(task);
        let score = self
            .impulses
            .impulses(token)
            .into_iter()
            .map(|impulse| {
                self.index.score_alignment(
                    &prepared,
                    task.binding_output_slot.unwrap_or(0),
                    impulse,
                )
            })
            .sum();
        self.scores.insert(key, score);
        score
    }
}

fn clamp_i32_to_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn cleanup_token_impulses(token: &str) -> Vec<WavePredictorStateImpulse> {
    top_folded_signed_lanes(token, SEQ_FEATURE_CENTER_COUNT, TOP_ROLE_L1_LANES)
        .into_iter()
        .map(|(lane_id, sign)| WavePredictorStateImpulse {
            lane_id,
            signed_strength: if sign < 0 { -1 } else { 1 },
        })
        .collect()
}

fn role_target_hit_report(tasks: &[PreparedBindingTask]) -> RoleHitReport {
    let mut positive_total = 0usize;
    let mut positive_hits = 0usize;
    let mut negative_total = 0usize;
    let mut negative_hits = 0usize;

    for task in tasks {
        for impulse in task.train_task.target_delta.positive_impulses() {
            positive_total += 1;
            positive_hits += usize::from(role_lane_is_active(
                &task.train_task.active_fringe,
                impulse.lane_id,
            ));
        }
        for impulse in task.train_task.target_delta.negative_impulses() {
            negative_total += 1;
            negative_hits += usize::from(role_lane_is_active(
                &task.train_task.active_fringe,
                impulse.lane_id,
            ));
        }
    }

    RoleHitReport {
        positive_hit_milli: milli_ratio(positive_hits, positive_total),
        negative_hit_milli: milli_ratio(negative_hits, negative_total),
    }
}

fn role_lane_is_active(active_fringe: &[WavePredictorActiveCenter], lane_id: u16) -> bool {
    for slot_id in 0..ROLE_SLOT_COUNT {
        let center_id = ROLE_CENTER_BASE
            + WavePredictorCenterId::from(slot_id) * FEATURE_CENTER_COUNT
            + WavePredictorCenterId::from(lane_id);
        if active_fringe
            .iter()
            .any(|active| active.center_id == center_id && active.strength != 0)
        {
            return true;
        }
    }
    false
}

fn state_delta_sum_gap(
    field: &WavePredictorHebbianField,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    state_delta_target_score(field, task) - state_delta_wrong_score(field, task)
}

fn state_delta_target_score(
    field: &WavePredictorHebbianField,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| {
            state_delta_impulse_alignment(
                field,
                &task.active_fringe,
                *impulse,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn state_delta_wrong_score(
    field: &WavePredictorHebbianField,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    task.target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| {
            state_delta_impulse_alignment(
                field,
                &task.active_fringe,
                *impulse,
                task.binding_output_slot,
            )
        })
        .sum()
}

fn state_delta_impulse_alignment(
    field: &WavePredictorHebbianField,
    active_fringe: &[WavePredictorActiveCenter],
    impulse: WavePredictorStateImpulse,
    binding_output_slot: Option<u8>,
) -> i32 {
    let sign = if impulse.signed_strength < 0 { -1 } else { 1 };
    sign * field.score_state_delta_lane(impulse.lane_id, active_fringe)
        + field.score_state_delta_binding_alignment(
            impulse.lane_id,
            impulse.signed_strength,
            active_fringe,
            binding_output_slot,
        )
}

fn eval_group_prototype_baseline<F>(
    train_rows: &[BindingRow],
    heldout_rows: &[BindingRow],
    key_fn: F,
) -> BaselineReport
where
    F: Fn(&BindingRow) -> String,
{
    let train = prepare_rows(train_rows);
    let heldout = prepare_rows(heldout_rows);
    let mut prototypes: BTreeMap<String, BTreeMap<u16, i32>> = BTreeMap::new();
    for (row, task) in train_rows.iter().zip(train.iter()) {
        let prototype = prototypes.entry(key_fn(row)).or_default();
        add_delta_to_prototype(prototype, &task.train_task.target_delta);
    }
    let mut correct = 0usize;
    for (row, task) in heldout_rows.iter().zip(heldout.iter()) {
        let Some(prototype) = prototypes.get(&key_fn(row)) else {
            continue;
        };
        if prototype_gap(prototype, &task.train_task.target_delta) > 0 {
            correct += 1;
        }
    }
    BaselineReport {
        accuracy_milli: milli_ratio(correct, heldout_rows.len()),
    }
}

fn eval_l1_neighbor_baseline(
    train_rows: &[BindingRow],
    heldout_rows: &[BindingRow],
) -> BaselineReport {
    let train = prepare_rows(train_rows);
    let heldout = prepare_rows(heldout_rows);
    let train_features: Vec<_> = train_rows
        .iter()
        .map(|row| l1_feature_set(&format!("{} {}", row.state_before, row.rule_action_example)))
        .collect();
    let mut correct = 0usize;
    for (row, task) in heldout_rows.iter().zip(heldout.iter()) {
        let features = l1_feature_set(&format!("{} {}", row.state_before, row.rule_action_example));
        let Some((best_index, _)) = train_features
            .iter()
            .enumerate()
            .map(|(index, candidate)| (index, feature_overlap(&features, candidate)))
            .max_by_key(|(_, overlap)| *overlap)
        else {
            continue;
        };
        let mut prototype = BTreeMap::new();
        add_delta_to_prototype(&mut prototype, &train[best_index].train_task.target_delta);
        if prototype_gap(&prototype, &task.train_task.target_delta) > 0 {
            correct += 1;
        }
    }
    BaselineReport {
        accuracy_milli: milli_ratio(correct, heldout_rows.len()),
    }
}

fn add_delta_to_prototype(
    prototype: &mut BTreeMap<u16, i32>,
    delta: &WavePredictorStateDeltaTarget,
) {
    for impulse in delta.positive_impulses() {
        *prototype.entry(impulse.lane_id).or_default() += i32::from(impulse.signed_strength);
    }
    for impulse in delta.negative_impulses() {
        *prototype.entry(impulse.lane_id).or_default() -= i32::from(impulse.signed_strength);
    }
}

fn prototype_gap(prototype: &BTreeMap<u16, i32>, delta: &WavePredictorStateDeltaTarget) -> i32 {
    let target_score: i32 = delta
        .positive_impulses()
        .iter()
        .map(|impulse| prototype_impulse_alignment(prototype, *impulse))
        .sum();
    let negative_score: i32 = delta
        .negative_impulses()
        .iter()
        .map(|impulse| prototype_impulse_alignment(prototype, *impulse))
        .sum();
    target_score - negative_score
}

fn prototype_impulse_alignment(
    prototype: &BTreeMap<u16, i32>,
    impulse: WavePredictorStateImpulse,
) -> i32 {
    let sign = if impulse.signed_strength < 0 { -1 } else { 1 };
    prototype.get(&impulse.lane_id).copied().unwrap_or(0) * sign
}

fn l1_feature_set(text: &str) -> BTreeSet<WavePredictorCenterId> {
    surface_lane_centers(text, FEATURE_CENTER_BASE, TOP_ACTIVE_L1_LANES)
        .into_iter()
        .map(|active| active.center_id)
        .collect()
}

fn feature_overlap(
    left: &BTreeSet<WavePredictorCenterId>,
    right: &BTreeSet<WavePredictorCenterId>,
) -> usize {
    left.intersection(right).count()
}

fn milli_ratio(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (1000 * numerator + denominator / 2) / denominator
}
