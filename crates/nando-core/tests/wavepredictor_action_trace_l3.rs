use nando_core::{
    SURFACE_WAVE_DIM, SurfaceWave4096, WavePredictorActiveCenter, WavePredictorCenterId,
    WavePredictorHebbianConfig, WavePredictorHebbianField, WavePredictorMarginSchedule,
    WavePredictorStateDeltaTarget, WavePredictorStateDeltaTrainTask, WavePredictorStateImpulse,
    WavePredictorTrainer, WavePredictorTrainerConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const ACTION_TRACE_CORPUS: &str =
    "../../data/rule_logic_corpus_v3/accepted_action_trace_tasks_v3.jsonl";
const FEATURE_CENTER_BASE: WavePredictorCenterId = 256;
const FEATURE_CENTER_COUNT: WavePredictorCenterId = SURFACE_WAVE_DIM as WavePredictorCenterId;
const TARGET_CENTER_COUNT: usize = 256;
const TOP_ACTIVE_L1_LANES: usize = 32;
const STATE_DELTA_LANES_PER_SIDE: usize = 16;
const TRAIN_TASK_LIMIT: usize = 6_000;
const HELDOUT_TASK_LIMIT: usize = 1_500;

#[derive(Clone, Debug)]
struct TraceRow {
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
struct PreparedTraceTask {
    train_task: WavePredictorStateDeltaTrainTask,
}

#[derive(Clone, Debug)]
struct TraceSplit {
    train: Vec<TraceRow>,
    heldout: Vec<TraceRow>,
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
#[ignore = "Rule Logic v3 action-trace L3 gate; run explicitly for L3 training"]
fn rule_logic_v3_action_trace_trains_l3_transition_without_rule_id_authority() {
    let rows = load_rows();
    assert_eq!(rows.len(), 12_000);

    let split = split_stratified_by_task_id(&rows);
    let train_rows = limit_balanced(&split.train, TRAIN_TASK_LIMIT);
    let heldout_rows = limit_balanced(&split.heldout, HELDOUT_TASK_LIMIT);
    let train = prepare_rows(&train_rows);
    let heldout = prepare_rows(&heldout_rows);
    let train_tasks: Vec<_> = train.iter().map(|task| task.train_task.clone()).collect();

    let mut field = WavePredictorHebbianField::new(
        TARGET_CENTER_COUNT + FEATURE_CENTER_COUNT as usize,
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
        },
    );
    let config = WavePredictorTrainerConfig {
        epochs: 6,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 144,
            warmup_epochs: 1,
            ramp_epochs: 5,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };

    let report = WavePredictorTrainer::train_state_delta(&mut field, &train_tasks, config);
    assert!(report.state_delta_training_used);
    assert!(!report.target_center_id_training_used);
    assert!(!report.axis_target_id_training_used);
    assert!(!report.semantic_grokking_claim_allowed);
    assert!(!report.base_mass_drift_detected);

    let train_eval = eval_state_delta(&field, &train);
    let heldout_eval = eval_state_delta(&field, &heldout);
    let error_audit = audit_errors(&field, &heldout_rows, &heldout);
    let source_group_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.source_group.clone());
    let rule_id_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.proof_rule_id.clone());
    let surface_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.surface_family.clone());
    let status_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.answer_status.clone());
    let neighbor_baseline = eval_l1_neighbor_baseline(&train_rows, &heldout_rows);
    let best_shortcut = [
        source_group_baseline.accuracy_milli,
        rule_id_baseline.accuracy_milli,
        surface_baseline.accuracy_milli,
        status_baseline.accuracy_milli,
        neighbor_baseline.accuracy_milli,
    ]
    .into_iter()
    .max()
    .expect("shortcut list must not be empty");

    println!("rule_logic_v3_action_trace_l3_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  train_action_trace_accuracy_milli: {}",
        train_eval.accuracy_milli
    );
    println!(
        "  heldout_action_trace_accuracy_milli: {}",
        heldout_eval.accuracy_milli
    );
    println!(
        "  heldout_action_trace_median_gap: {}",
        heldout_eval.median_gap
    );
    println!("  heldout_action_trace_p10_gap: {}", heldout_eval.p10_gap);
    println!(
        "  source_group_prototype_accuracy_milli: {}",
        source_group_baseline.accuracy_milli
    );
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
    println!("  best_shortcut_accuracy_milli: {best_shortcut}");
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    print_error_audit(&error_audit);

    assert!(
        heldout_eval.accuracy_milli >= 700 && heldout_eval.accuracy_milli > best_shortcut,
        "L3 action trace must beat shortcut baselines: heldout={heldout_eval:#?} best_shortcut={best_shortcut}"
    );
}

fn audit_errors(
    field: &WavePredictorHebbianField,
    rows: &[TraceRow],
    tasks: &[PreparedTraceTask],
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

fn load_rows() -> Vec<TraceRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ACTION_TRACE_CORPUS);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read action-trace corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> TraceRow {
    TraceRow {
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

fn split_stratified_by_task_id(rows: &[TraceRow]) -> TraceSplit {
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for row in rows {
        if task_number(&row.task_id).is_multiple_of(5) {
            heldout.push(row.clone());
        } else {
            train.push(row.clone());
        }
    }
    TraceSplit { train, heldout }
}

fn limit_balanced(rows: &[TraceRow], limit: usize) -> Vec<TraceRow> {
    if rows.len() <= limit {
        return rows.to_vec();
    }
    let mut by_rule: BTreeMap<String, Vec<TraceRow>> = BTreeMap::new();
    for row in rows {
        by_rule
            .entry(row.proof_rule_id.clone())
            .or_default()
            .push(row.clone());
    }
    let groups: Vec<_> = by_rule.into_values().collect();
    let mut out = Vec::with_capacity(limit);
    let mut index = 0usize;
    while out.len() < limit {
        let mut progressed = false;
        for group in &groups {
            if let Some(row) = group.get(index) {
                out.push(row.clone());
                progressed = true;
                if out.len() == limit {
                    break;
                }
            }
        }
        if !progressed {
            break;
        }
        index += 1;
    }
    out
}

fn prepare_rows(rows: &[TraceRow]) -> Vec<PreparedTraceTask> {
    rows.iter().map(PreparedTraceTask::new).collect()
}

impl PreparedTraceTask {
    fn new(row: &TraceRow) -> Self {
        let active_context = format!("{} {}", row.state_before, row.rule_action_example);
        let base_wave = SurfaceWave4096::compile(&row.state_before);
        let target_wave = SurfaceWave4096::compile(&row.state_after_correct);
        let wrong_wave = SurfaceWave4096::compile(&row.state_after_wrong);
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
            .expect("accepted action trace tasks must yield compact state deltas");

        Self {
            train_task: WavePredictorStateDeltaTrainTask {
                active_fringe: active_l1_fringe(&active_context),
                target_delta,
                binding_output_slot: None,
            },
        }
    }
}

fn active_l1_fringe(input: &str) -> Vec<WavePredictorActiveCenter> {
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
        .take(TOP_ACTIVE_L1_LANES)
        .map(|(_, lane, value)| WavePredictorActiveCenter {
            center_id: FEATURE_CENTER_BASE + WavePredictorCenterId::from(lane),
            strength: value.abs().clamp(1, 8),
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

fn eval_state_delta(field: &WavePredictorHebbianField, tasks: &[PreparedTraceTask]) -> EvalReport {
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

fn state_delta_sum_gap(
    field: &WavePredictorHebbianField,
    task: &WavePredictorStateDeltaTrainTask,
) -> i32 {
    let target_score: i32 = task
        .target_delta
        .positive_impulses()
        .iter()
        .map(|impulse| state_delta_impulse_alignment(field, &task.active_fringe, *impulse))
        .sum();
    let negative_score: i32 = task
        .target_delta
        .negative_impulses()
        .iter()
        .map(|impulse| state_delta_impulse_alignment(field, &task.active_fringe, *impulse))
        .sum();
    target_score - negative_score
}

fn state_delta_impulse_alignment(
    field: &WavePredictorHebbianField,
    active_fringe: &[WavePredictorActiveCenter],
    impulse: WavePredictorStateImpulse,
) -> i32 {
    let sign = if impulse.signed_strength < 0 { -1 } else { 1 };
    sign * field.score_state_delta_lane(impulse.lane_id, active_fringe)
}

fn eval_group_prototype_baseline<F>(
    train_rows: &[TraceRow],
    heldout_rows: &[TraceRow],
    key_fn: F,
) -> BaselineReport
where
    F: Fn(&TraceRow) -> String,
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

fn eval_l1_neighbor_baseline(train_rows: &[TraceRow], heldout_rows: &[TraceRow]) -> BaselineReport {
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
    active_l1_fringe(text)
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
    (numerator * 1000 + denominator / 2) / denominator
}

fn task_number(task_id: &str) -> usize {
    task_id
        .rsplit('_')
        .next()
        .expect("task id must contain numeric suffix")
        .parse()
        .expect("task id suffix must be numeric")
}
