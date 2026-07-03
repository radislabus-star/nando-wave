use nando_core::{
    SURFACE_WAVE_DIM, SurfaceWave4096, WavePredictorActiveCenter, WavePredictorCenterId,
    WavePredictorHebbianConfig, WavePredictorHebbianField, WavePredictorMarginSchedule,
    WavePredictorStateDeltaTarget, WavePredictorStateDeltaTrainTask, WavePredictorStateImpulse,
    WavePredictorTrainer, WavePredictorTrainerConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const RULE_LOGIC_CORPUS: &str = "../../data/rule_logic_corpus_v2/accepted_rule_tasks_v2.jsonl";
const FEATURE_CENTER_BASE: WavePredictorCenterId = 256;
const FEATURE_CENTER_COUNT: WavePredictorCenterId = SURFACE_WAVE_DIM as WavePredictorCenterId;
const TARGET_CENTER_COUNT: usize = 256;
const TOP_ACTIVE_L1_LANES: usize = 24;
const STATE_DELTA_LANES_PER_SIDE: usize = 12;
const TRAIN_TASK_LIMIT: usize = 4_800;
const HELDOUT_TASK_LIMIT: usize = 1_200;

#[derive(Clone, Debug)]
struct RuleRow {
    task_id: String,
    source_group: String,
    surface_family: String,
    proof_rule_id: String,
    answer_status: String,
    input: String,
    target: String,
    near_negative: String,
}

#[derive(Clone, Debug)]
struct PreparedRuleTask {
    train_task: WavePredictorStateDeltaTrainTask,
}

#[derive(Clone, Debug)]
struct RuleSplit {
    train: Vec<RuleRow>,
    heldout: Vec<RuleRow>,
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
    tasks: usize,
    correct: usize,
    accuracy_milli: usize,
}

#[test]
#[ignore = "Rule Logic v2 L3 debt gate; run explicitly when checking the hard corpus"]
fn rule_logic_v2_exposes_current_l3_state_delta_debt_without_rule_id_authority() {
    let rows = load_rule_rows();
    assert_eq!(rows.len(), 12_000);

    let split = split_stratified_by_task_id(&rows);
    let (train_rows, heldout_rows) = limit_balanced_by_rule(&split);
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
            weight_limit: 768,
        },
    );
    let config = WavePredictorTrainerConfig {
        epochs: 5,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 120,
            warmup_epochs: 1,
            ramp_epochs: 4,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };

    let train_report = WavePredictorTrainer::train_state_delta(&mut field, &train_tasks, config);
    assert!(train_report.state_delta_training_used);
    assert!(!train_report.target_center_id_training_used);
    assert!(!train_report.axis_target_id_training_used);
    assert!(!train_report.semantic_grokking_claim_allowed);
    assert!(!train_report.base_mass_drift_detected);

    let train_eval = eval_state_delta(&field, &train);
    let heldout_eval = eval_state_delta(&field, &heldout);
    let source_group_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.source_group.clone());
    let rule_id_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.proof_rule_id.clone());
    let surface_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.surface_family.clone());
    let status_baseline =
        eval_group_prototype_baseline(&train_rows, &heldout_rows, |row| row.answer_status.clone());
    let neighbor_baseline = eval_l1_neighbor_baseline(&train_rows, &heldout_rows);

    println!("rule_logic_v2_wavepredictor_l3_trainer_gate:");
    println!("  train_rows: {}", train_rows.len());
    println!("  heldout_rows: {}", heldout_rows.len());
    println!(
        "  train_state_delta_accuracy_milli: {}",
        train_eval.accuracy_milli
    );
    println!(
        "  heldout_state_delta_accuracy_milli: {}",
        heldout_eval.accuracy_milli
    );
    println!("  heldout_state_delta_tasks: {}", heldout_eval.tasks);
    println!(
        "  heldout_state_delta_median_gap: {}",
        heldout_eval.median_gap
    );
    println!("  heldout_state_delta_p10_gap: {}", heldout_eval.p10_gap);
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
    println!("  state_delta_edges: {}", field.state_delta_edge_count());
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");

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
    println!("  best_shortcut_accuracy_milli: {best_shortcut}");
    println!("  verdict: RULE_LOGIC_V2_HARD_CORPUS_CURRENT_L3_NOT_ABOVE_SHORTCUT");

    assert!(
        best_shortcut <= 550,
        "Rule Logic v2 corpus is still too shortcut-friendly: best_shortcut={best_shortcut}"
    );
    assert!(
        heldout_eval.accuracy_milli <= best_shortcut + 50,
        "Current L3 unexpectedly beat the hard-corpus shortcut band; inspect before claiming grokking: heldout={heldout_eval:#?} best_shortcut={best_shortcut}"
    );
}

fn load_rule_rows() -> Vec<RuleRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RULE_LOGIC_CORPUS);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read Rule Logic v1 corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> RuleRow {
    RuleRow {
        task_id: json_string(line, "task_id"),
        source_group: json_string(line, "source_group"),
        surface_family: json_string(line, "surface_family"),
        proof_rule_id: json_string(line, "proof_rule_id"),
        answer_status: json_string(line, "answer_status"),
        input: json_string(line, "input"),
        target: json_string(line, "target"),
        near_negative: json_string(line, "near_negative"),
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

fn split_stratified_by_task_id(rows: &[RuleRow]) -> RuleSplit {
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for row in rows {
        if task_number(&row.task_id).is_multiple_of(5) {
            heldout.push(row.clone());
        } else {
            train.push(row.clone());
        }
    }
    RuleSplit { train, heldout }
}

fn limit_balanced_by_rule(split: &RuleSplit) -> (Vec<RuleRow>, Vec<RuleRow>) {
    (
        limit_balanced(&split.train, TRAIN_TASK_LIMIT),
        limit_balanced(&split.heldout, HELDOUT_TASK_LIMIT),
    )
}

fn limit_balanced(rows: &[RuleRow], limit: usize) -> Vec<RuleRow> {
    if rows.len() <= limit {
        return rows.to_vec();
    }

    let mut by_rule: BTreeMap<String, Vec<RuleRow>> = BTreeMap::new();
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

fn prepare_rows(rows: &[RuleRow]) -> Vec<PreparedRuleTask> {
    rows.iter().map(PreparedRuleTask::new).collect()
}

impl PreparedRuleTask {
    fn new(row: &RuleRow) -> Self {
        let input_wave = SurfaceWave4096::compile(&row.input);
        let target_wave = SurfaceWave4096::compile(&row.target);
        let negative_wave = SurfaceWave4096::compile(&row.near_negative);
        let positive = discriminative_delta_impulses(
            input_wave.lanes(),
            target_wave.lanes(),
            negative_wave.lanes(),
            STATE_DELTA_LANES_PER_SIDE,
        );
        let negative = discriminative_delta_impulses(
            input_wave.lanes(),
            negative_wave.lanes(),
            target_wave.lanes(),
            STATE_DELTA_LANES_PER_SIDE,
        );
        let target_delta = WavePredictorStateDeltaTarget::from_impulses(&positive, &negative)
            .expect("accepted rule tasks must yield compact state deltas");

        Self {
            train_task: WavePredictorStateDeltaTrainTask {
                active_fringe: active_l1_fringe(&row.input),
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

fn eval_state_delta(field: &WavePredictorHebbianField, tasks: &[PreparedRuleTask]) -> EvalReport {
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
    train_rows: &[RuleRow],
    heldout_rows: &[RuleRow],
    key_fn: F,
) -> BaselineReport
where
    F: Fn(&RuleRow) -> String,
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
        let key = key_fn(row);
        let Some(prototype) = prototypes.get(&key) else {
            continue;
        };
        if prototype_gap(prototype, &task.train_task.target_delta) > 0 {
            correct += 1;
        }
    }

    baseline_report(correct, heldout_rows.len())
}

fn eval_l1_neighbor_baseline(train_rows: &[RuleRow], heldout_rows: &[RuleRow]) -> BaselineReport {
    let train = prepare_rows(train_rows);
    let heldout = prepare_rows(heldout_rows);
    let train_features: Vec<_> = train_rows
        .iter()
        .map(|row| l1_feature_set(&row.input))
        .collect();

    let mut correct = 0usize;
    for (row, task) in heldout_rows.iter().zip(heldout.iter()) {
        let features = l1_feature_set(&row.input);
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

    baseline_report(correct, heldout_rows.len())
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

fn baseline_report(correct: usize, tasks: usize) -> BaselineReport {
    BaselineReport {
        tasks,
        correct,
        accuracy_milli: milli_ratio(correct, tasks),
    }
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
