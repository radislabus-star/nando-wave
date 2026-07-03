use nando_core::{
    SURFACE_WAVE_DIM, SurfaceWave4096, WavePredictorActiveCenter, WavePredictorAxisTarget,
    WavePredictorCenterId, WavePredictorCompositionalTrainTask, WavePredictorHebbianConfig,
    WavePredictorHebbianField, WavePredictorMarginSchedule, WavePredictorStateDeltaTarget,
    WavePredictorStateDeltaTrainTask, WavePredictorStateImpulse, WavePredictorTrainTask,
    WavePredictorTrainer, WavePredictorTrainerConfig,
};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const ACCEPTED_10K: &str =
    "../../data/task_candidates/linux_networking_vpn_accepted_10k_v1/accepted_wave_task_v2.jsonl";
const FEATURE_CENTER_BASE: WavePredictorCenterId = 256;
const FEATURE_CENTER_COUNT: WavePredictorCenterId = 4096;
const TARGET_CENTER_COUNT: usize = 256;
const TOP_FEATURES: usize = 16;
const STATE_DELTA_LANES_PER_SIDE: usize = 8;
const STATE_DELTA_TRAIN_TASK_LIMIT: usize = 2_000;
const STATE_DELTA_HELDOUT_TASK_LIMIT: usize = 500;

#[derive(Clone, Debug)]
struct CorpusRow {
    task_id: String,
    source_group: String,
    operator_family: String,
    input: String,
    target: String,
    near_negative: String,
    why_negative_is_wrong: String,
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
struct CompositionalEvalReport {
    tasks: usize,
    exact_correct: usize,
    exact_accuracy_milli: usize,
    axis_total: usize,
    axis_correct: usize,
    axis_accuracy_milli: usize,
    median_gap: i32,
    p10_gap: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct StateDeltaEvalReport {
    tasks: usize,
    correct: usize,
    accuracy_milli: usize,
    median_gap: i32,
    p10_gap: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CombinatorialDeltaSplitAudit {
    train_tasks: usize,
    heldout_tasks: usize,
    heldout_combos: usize,
    leaked_exact_combos: usize,
    missing_seen_parts: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BaselineReport {
    tasks: usize,
    correct: usize,
    accuracy_milli: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ShortcutAuditReport {
    source_group_only: BaselineReport,
    scope_only: BaselineReport,
    window_only: BaselineReport,
    token_bigram_neighbor: BaselineReport,
    bayesian_pairwise: BaselineReport,
    markov_bigram: BaselineReport,
}

#[derive(Clone, Debug)]
struct PreparedTask {
    train_task: WavePredictorTrainTask,
}

#[derive(Clone, Debug)]
struct PreparedCompositionalTask {
    train_task: WavePredictorCompositionalTrainTask,
}

#[derive(Clone, Debug)]
struct PreparedStateDeltaTask {
    train_task: WavePredictorStateDeltaTrainTask,
}

#[test]
#[ignore = "10k corpus trainer gate; run explicitly when checking Step 11"]
fn accepted_10k_trainer_gate_exposes_flat_operator_holdout_debt_and_compositional_path() {
    let rows = load_accepted_rows();
    assert_eq!(rows.len(), 10_000);

    let context = CorpusContext::new(&rows);
    let source_group_split = split_by_source_group(&rows);
    let unseen_heldout_operators =
        unseen_heldout_operator_count(&source_group_split.train, &source_group_split.heldout);
    assert_eq!(unseen_heldout_operators, 5);

    let source_group_report = train_and_eval(&context, &source_group_split);
    assert_eq!(
        source_group_report.accuracy_milli, 0,
        "source_group_report={source_group_report:#?}"
    );

    let unseen_heldout_axis_values =
        unseen_heldout_axis_value_count(&source_group_split.train, &source_group_split.heldout);
    assert_eq!(unseen_heldout_axis_values, 0);

    let compositional_report = train_and_eval_compositional(&context, &source_group_split);
    assert!(
        compositional_report.axis_accuracy_milli > source_group_report.accuracy_milli,
        "compositional_report={compositional_report:#?}"
    );
    assert!(
        compositional_report.exact_accuracy_milli > source_group_report.accuracy_milli,
        "compositional_report={compositional_report:#?}"
    );

    let state_delta_report = train_and_eval_state_delta(&source_group_split);
    assert!(
        state_delta_report.accuracy_milli <= 20,
        "state_delta should not pretend source-group transfer when target wave deltas are unseen: {state_delta_report:#?}"
    );

    let combinatorial_delta_split = split_combinatorial_delta(&rows);
    let combinatorial_delta_audit = audit_combinatorial_delta_split(&combinatorial_delta_split);
    assert_eq!(
        combinatorial_delta_audit.leaked_exact_combos, 0,
        "combinatorial_delta_audit={combinatorial_delta_audit:#?}"
    );
    assert_eq!(
        combinatorial_delta_audit.missing_seen_parts, 0,
        "combinatorial_delta_audit={combinatorial_delta_audit:#?}"
    );
    let combinatorial_delta_report = train_and_eval_state_delta(&combinatorial_delta_split);
    assert!(
        combinatorial_delta_report.accuracy_milli >= 900,
        "combinatorial_delta_report={combinatorial_delta_report:#?} state_delta_report={state_delta_report:#?}"
    );
    let shortcut_audit = run_combinatorial_shortcut_audit(&combinatorial_delta_split);
    assert!(
        shortcut_audit.source_group_only.accuracy_milli >= 900
            || shortcut_audit.token_bigram_neighbor.accuracy_milli >= 900
            || shortcut_audit.bayesian_pairwise.accuracy_milli >= 900,
        "shortcut audit should expose the current combinatorial delta leakage: {shortcut_audit:#?}"
    );

    let stratified_split = split_stratified_by_task_id(&rows);
    let stratified_unseen =
        unseen_heldout_operator_count(&stratified_split.train, &stratified_split.heldout);
    assert_eq!(stratified_unseen, 0);

    let stratified_report = train_and_eval(&context, &stratified_split);
    assert!(
        stratified_report.accuracy_milli >= 990,
        "stratified_report={stratified_report:#?}"
    );

    println!("accepted_10k_wavepredictor_trainer_gate:");
    println!(
        "  source_group_holdout_train_tasks: {}",
        source_group_split.train.len()
    );
    println!(
        "  source_group_holdout_tasks: {}",
        source_group_split.heldout.len()
    );
    println!("  unseen_heldout_target_operators: {unseen_heldout_operators}");
    println!(
        "  source_group_flat_operator_accuracy_milli: {}",
        source_group_report.accuracy_milli
    );
    println!(
        "  source_group_median_gap: {}",
        source_group_report.median_gap
    );
    println!("  source_group_p10_gap: {}", source_group_report.p10_gap);
    println!("  heldout_axis_values_unseen_in_train: {unseen_heldout_axis_values}");
    println!(
        "  source_group_compositional_exact_accuracy_milli: {}",
        compositional_report.exact_accuracy_milli
    );
    println!(
        "  source_group_compositional_axis_accuracy_milli: {}",
        compositional_report.axis_accuracy_milli
    );
    println!(
        "  source_group_compositional_median_gap: {}",
        compositional_report.median_gap
    );
    println!(
        "  source_group_compositional_p10_gap: {}",
        compositional_report.p10_gap
    );
    println!(
        "  source_group_state_delta_accuracy_milli: {}",
        state_delta_report.accuracy_milli
    );
    println!(
        "  source_group_state_delta_tasks: {}",
        state_delta_report.tasks
    );
    println!(
        "  source_group_state_delta_median_gap: {}",
        state_delta_report.median_gap
    );
    println!(
        "  source_group_state_delta_p10_gap: {}",
        state_delta_report.p10_gap
    );
    println!(
        "  combinatorial_delta_train_tasks: {}",
        combinatorial_delta_audit.train_tasks
    );
    println!(
        "  combinatorial_delta_heldout_tasks: {}",
        combinatorial_delta_audit.heldout_tasks
    );
    println!(
        "  combinatorial_delta_heldout_combos: {}",
        combinatorial_delta_audit.heldout_combos
    );
    println!(
        "  combinatorial_delta_leaked_exact_combos: {}",
        combinatorial_delta_audit.leaked_exact_combos
    );
    println!(
        "  combinatorial_delta_missing_seen_parts: {}",
        combinatorial_delta_audit.missing_seen_parts
    );
    println!(
        "  combinatorial_delta_state_delta_accuracy_milli: {}",
        combinatorial_delta_report.accuracy_milli
    );
    println!(
        "  combinatorial_delta_state_delta_tasks: {}",
        combinatorial_delta_report.tasks
    );
    println!(
        "  combinatorial_delta_state_delta_median_gap: {}",
        combinatorial_delta_report.median_gap
    );
    println!(
        "  combinatorial_delta_state_delta_p10_gap: {}",
        combinatorial_delta_report.p10_gap
    );
    println!(
        "  shortcut_source_group_only_accuracy_milli: {}",
        shortcut_audit.source_group_only.accuracy_milli
    );
    println!(
        "  shortcut_scope_only_accuracy_milli: {}",
        shortcut_audit.scope_only.accuracy_milli
    );
    println!(
        "  shortcut_window_only_accuracy_milli: {}",
        shortcut_audit.window_only.accuracy_milli
    );
    println!(
        "  shortcut_token_bigram_neighbor_accuracy_milli: {}",
        shortcut_audit.token_bigram_neighbor.accuracy_milli
    );
    println!(
        "  shortcut_bayesian_pairwise_accuracy_milli: {}",
        shortcut_audit.bayesian_pairwise.accuracy_milli
    );
    println!(
        "  shortcut_markov_bigram_accuracy_milli: {}",
        shortcut_audit.markov_bigram.accuracy_milli
    );
    println!("  stratified_train_tasks: {}", stratified_split.train.len());
    println!(
        "  stratified_heldout_tasks: {}",
        stratified_split.heldout.len()
    );
    println!(
        "  stratified_flat_operator_accuracy_milli: {}",
        stratified_report.accuracy_milli
    );
    println!("  stratified_median_gap: {}", stratified_report.median_gap);
    println!("  stratified_p10_gap: {}", stratified_report.p10_gap);
    println!("  verdict: COMBINATORIAL_DELTA_SHORTCUT_LEAK_DETECTED");
}

fn train_and_eval(context: &CorpusContext, split: &CorpusSplit) -> EvalReport {
    let mut field = WavePredictorHebbianField::new(
        TARGET_CENTER_COUNT + FEATURE_CENTER_COUNT as usize,
        WavePredictorHebbianConfig::default(),
    );
    let train = context.prepare_rows(&split.train);
    let heldout = context.prepare_rows(&split.heldout);
    let train_tasks: Vec<_> = train.iter().map(|task| task.train_task.clone()).collect();
    let config = WavePredictorTrainerConfig {
        epochs: 10,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 120,
            warmup_epochs: 2,
            ramp_epochs: 8,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let train_report = WavePredictorTrainer::train(&mut field, &train_tasks, config);

    assert!(!train_report.base_mass_drift_detected);
    assert!(train_report.dynamic_margin_used);
    assert!(!train_report.eta_ratio_scheduler_used);
    assert!(!train_report.l4_opened);
    assert!(train_report.target_center_id_training_used);
    assert!(!train_report.semantic_grokking_claim_allowed);

    eval(&field, &heldout)
}

fn train_and_eval_compositional(
    context: &CorpusContext,
    split: &CorpusSplit,
) -> CompositionalEvalReport {
    let mut field = WavePredictorHebbianField::new(
        TARGET_CENTER_COUNT + FEATURE_CENTER_COUNT as usize,
        WavePredictorHebbianConfig::default(),
    );
    let train = context.prepare_compositional_rows(&split.train);
    let heldout = context.prepare_compositional_rows(&split.heldout);
    let train_tasks: Vec<_> = train.iter().map(|task| task.train_task.clone()).collect();
    let config = WavePredictorTrainerConfig {
        epochs: 10,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 120,
            warmup_epochs: 2,
            ramp_epochs: 8,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let train_report = WavePredictorTrainer::train_compositional(&mut field, &train_tasks, config);

    assert!(!train_report.base_mass_drift_detected);
    assert!(train_report.dynamic_margin_used);
    assert!(!train_report.eta_ratio_scheduler_used);
    assert!(!train_report.l4_opened);
    assert!(train_report.axis_target_id_training_used);
    assert!(!train_report.semantic_grokking_claim_allowed);

    eval_compositional(&field, &heldout)
}

fn train_and_eval_state_delta(split: &CorpusSplit) -> StateDeltaEvalReport {
    let mut field = WavePredictorHebbianField::new(
        TARGET_CENTER_COUNT + FEATURE_CENTER_COUNT as usize,
        WavePredictorHebbianConfig::default(),
    );
    let (train_rows, heldout_rows) = limited_state_delta_rows(split);
    let train = prepare_state_delta_rows(&train_rows);
    let heldout = prepare_state_delta_rows(&heldout_rows);
    let train_tasks: Vec<_> = train.iter().map(|task| task.train_task.clone()).collect();
    let config = WavePredictorTrainerConfig {
        epochs: 4,
        margin_schedule: WavePredictorMarginSchedule {
            start_margin: 24,
            target_margin: 96,
            warmup_epochs: 1,
            ramp_epochs: 3,
        },
        anti_wave_trap_updates_per_epoch_cap: None,
    };
    let train_report = WavePredictorTrainer::train_state_delta(&mut field, &train_tasks, config);

    assert!(!train_report.base_mass_drift_detected);
    assert!(train_report.dynamic_margin_used);
    assert!(!train_report.eta_ratio_scheduler_used);
    assert!(!train_report.l4_opened);
    assert!(!train_report.target_center_id_training_used);
    assert!(!train_report.axis_target_id_training_used);
    assert!(train_report.state_delta_training_used);
    assert!(!train_report.semantic_grokking_claim_allowed);

    eval_state_delta(&field, &heldout)
}

fn limited_state_delta_rows(split: &CorpusSplit) -> (Vec<CorpusRow>, Vec<CorpusRow>) {
    (
        limit_balanced_by_source_group(&split.train, STATE_DELTA_TRAIN_TASK_LIMIT),
        limit_balanced_by_source_group(&split.heldout, STATE_DELTA_HELDOUT_TASK_LIMIT),
    )
}

fn limit_balanced_by_source_group(rows: &[CorpusRow], limit: usize) -> Vec<CorpusRow> {
    if rows.len() <= limit {
        return rows.to_vec();
    }

    let mut by_group: BTreeMap<String, Vec<CorpusRow>> = BTreeMap::new();
    for row in rows {
        by_group
            .entry(row.source_group.clone())
            .or_default()
            .push(row.clone());
    }

    let groups: Vec<_> = by_group.into_values().collect();
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

fn eval(field: &WavePredictorHebbianField, heldout: &[PreparedTask]) -> EvalReport {
    let mut gaps = Vec::with_capacity(heldout.len());
    let mut correct = 0usize;
    for task in heldout {
        let error = task.train_task.as_error_for_eval(0);
        let gap = field.target_gap(&error);
        gaps.push(gap);
        if gap > 0 {
            correct += 1;
        }
    }
    gaps.sort_unstable();
    let tasks = heldout.len();
    EvalReport {
        tasks,
        correct,
        accuracy_milli: milli_ratio(correct, tasks),
        median_gap: gaps[tasks / 2],
        p10_gap: gaps[tasks / 10],
    }
}

fn eval_compositional(
    field: &WavePredictorHebbianField,
    heldout: &[PreparedCompositionalTask],
) -> CompositionalEvalReport {
    let mut gaps = Vec::new();
    let mut exact_correct = 0usize;
    let mut axis_correct = 0usize;
    let mut axis_total = 0usize;

    for task in heldout {
        let mut all_axes_correct = true;
        for axis in task.train_task.active_axis_targets() {
            let error = nando_core::WavePredictorConvergenceError {
                active_fringe: task.train_task.active_fringe.clone(),
                target_center: axis.target_center,
                nearest_wrong_center: axis.nearest_wrong_center,
                target_gap: 0,
                margin_required: 0,
                trap_accepted: false,
            };
            let gap = field.target_gap(&error);
            gaps.push(gap);
            axis_total += 1;
            if gap > 0 {
                axis_correct += 1;
            } else {
                all_axes_correct = false;
            }
        }
        if all_axes_correct {
            exact_correct += 1;
        }
    }

    gaps.sort_unstable();
    CompositionalEvalReport {
        tasks: heldout.len(),
        exact_correct,
        exact_accuracy_milli: milli_ratio(exact_correct, heldout.len()),
        axis_total,
        axis_correct,
        axis_accuracy_milli: milli_ratio(axis_correct, axis_total),
        median_gap: gaps[gaps.len() / 2],
        p10_gap: gaps[gaps.len() / 10],
    }
}

fn eval_state_delta(
    field: &WavePredictorHebbianField,
    heldout: &[PreparedStateDeltaTask],
) -> StateDeltaEvalReport {
    let mut gaps = Vec::with_capacity(heldout.len());
    let mut correct = 0usize;

    for task in heldout {
        let gap = state_delta_sum_gap(field, &task.train_task);
        gaps.push(gap);
        if gap > 0 {
            correct += 1;
        }
    }

    gaps.sort_unstable();
    StateDeltaEvalReport {
        tasks: heldout.len(),
        correct,
        accuracy_milli: milli_ratio(correct, heldout.len()),
        median_gap: gaps[gaps.len() / 2],
        p10_gap: gaps[gaps.len() / 10],
    }
}

fn run_combinatorial_shortcut_audit(split: &CorpusSplit) -> ShortcutAuditReport {
    let (train_rows, heldout_rows) = limited_state_delta_rows(split);

    ShortcutAuditReport {
        source_group_only: eval_part_only_baseline(&train_rows, &heldout_rows, |row| {
            row.source_group.clone()
        }),
        scope_only: eval_part_only_baseline(&train_rows, &heldout_rows, |row| {
            input_field(&row.input, "Область проверки: ")
        }),
        window_only: eval_part_only_baseline(&train_rows, &heldout_rows, |row| {
            input_field(&row.input, "Evidence window: ")
        }),
        token_bigram_neighbor: eval_token_bigram_neighbor_baseline(&train_rows, &heldout_rows),
        bayesian_pairwise: eval_bayesian_pairwise_baseline(&train_rows, &heldout_rows),
        markov_bigram: eval_markov_bigram_baseline(&train_rows, &heldout_rows),
    }
}

fn eval_part_only_baseline<F>(
    train_rows: &[CorpusRow],
    heldout_rows: &[CorpusRow],
    key_fn: F,
) -> BaselineReport
where
    F: Fn(&CorpusRow) -> String,
{
    let train = prepare_state_delta_rows(train_rows);
    let heldout = prepare_state_delta_rows(heldout_rows);
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

fn eval_token_bigram_neighbor_baseline(
    train_rows: &[CorpusRow],
    heldout_rows: &[CorpusRow],
) -> BaselineReport {
    let train = prepare_state_delta_rows(train_rows);
    let heldout = prepare_state_delta_rows(heldout_rows);
    let train_features: Vec<_> = train_rows
        .iter()
        .map(|row| surface_feature_set(&row.input))
        .collect();

    let mut correct = 0usize;
    for (row, task) in heldout_rows.iter().zip(heldout.iter()) {
        let features = surface_feature_set(&row.input);
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

fn eval_bayesian_pairwise_baseline(
    train_rows: &[CorpusRow],
    heldout_rows: &[CorpusRow],
) -> BaselineReport {
    let mut pair_counts: BTreeMap<(String, String), i32> = BTreeMap::new();
    let mut target_token_counts: BTreeMap<String, i32> = BTreeMap::new();

    for row in train_rows {
        let input_features = surface_feature_set(&row.input);
        let target_tokens = tokens(&row.target);
        for token in &target_tokens {
            *target_token_counts.entry(token.clone()).or_default() += 1;
        }
        for feature in &input_features {
            for token in &target_tokens {
                *pair_counts
                    .entry((feature.clone(), token.clone()))
                    .or_default() += 1;
            }
        }
    }

    let mut correct = 0usize;
    for row in heldout_rows {
        let input_features = surface_feature_set(&row.input);
        let target_score = bayesian_text_score(&input_features, &row.target, &pair_counts);
        let negative_score = bayesian_text_score(&input_features, &row.near_negative, &pair_counts);
        if target_score > negative_score {
            correct += 1;
        }
    }

    baseline_report(correct, heldout_rows.len())
}

fn eval_markov_bigram_baseline(
    train_rows: &[CorpusRow],
    heldout_rows: &[CorpusRow],
) -> BaselineReport {
    let mut unigram_counts: BTreeMap<String, i32> = BTreeMap::new();
    let mut bigram_counts: BTreeMap<(String, String), i32> = BTreeMap::new();

    for row in train_rows {
        let target_tokens = tokens(&row.target);
        for token in &target_tokens {
            *unigram_counts.entry(token.clone()).or_default() += 1;
        }
        for pair in target_tokens.windows(2) {
            *bigram_counts
                .entry((pair[0].clone(), pair[1].clone()))
                .or_default() += 1;
        }
    }

    let mut correct = 0usize;
    for row in heldout_rows {
        let target_score = markov_text_score(&row.target, &unigram_counts, &bigram_counts);
        let negative_score = markov_text_score(&row.near_negative, &unigram_counts, &bigram_counts);
        if target_score > negative_score {
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

fn surface_feature_set(text: &str) -> BTreeSet<String> {
    let tokens = tokens(text);
    let mut features: BTreeSet<String> =
        tokens.iter().map(|token| format!("tok:{token}")).collect();
    for pair in tokens.windows(2) {
        features.insert(format!("bi:{} {}", pair[0], pair[1]));
    }
    features
}

fn feature_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> usize {
    left.intersection(right).count()
}

fn bayesian_text_score(
    input_features: &BTreeSet<String>,
    text: &str,
    pair_counts: &BTreeMap<(String, String), i32>,
) -> i64 {
    let mut score = 0i64;
    for token in tokens(text) {
        for feature in input_features {
            score += i64::from(
                pair_counts
                    .get(&(feature.clone(), token.clone()))
                    .copied()
                    .unwrap_or(0),
            );
        }
    }
    score
}

fn markov_text_score(
    text: &str,
    unigram_counts: &BTreeMap<String, i32>,
    bigram_counts: &BTreeMap<(String, String), i32>,
) -> i64 {
    let tokens = tokens(text);
    let unigram_score: i64 = tokens
        .iter()
        .map(|token| i64::from(unigram_counts.get(token).copied().unwrap_or(0)))
        .sum();
    let bigram_score: i64 = tokens
        .windows(2)
        .map(|pair| {
            i64::from(
                bigram_counts
                    .get(&(pair[0].clone(), pair[1].clone()))
                    .copied()
                    .unwrap_or(0),
            )
        })
        .sum();
    unigram_score + 4 * bigram_score
}

fn baseline_report(correct: usize, tasks: usize) -> BaselineReport {
    BaselineReport {
        tasks,
        correct,
        accuracy_milli: milli_ratio(correct, tasks),
    }
}

impl PreparedTask {
    fn new(row: CorpusRow, context: &CorpusContext) -> Self {
        let target_center = context.operator_center(&row.operator_family);
        let nearest_wrong_center = context
            .wrong_operator_center(&row)
            .unwrap_or((target_center + 1) % context.operator_count() as WavePredictorCenterId);
        let active_fringe = active_fringe(&row.input);
        Self {
            train_task: WavePredictorTrainTask {
                active_fringe,
                target_center,
                nearest_wrong_center,
                trap_accepted: true,
            },
        }
    }
}

impl PreparedCompositionalTask {
    fn new(row: CorpusRow, context: &CorpusContext) -> Self {
        let wrong_group = parenthesized_vpn_group(&row.why_negative_is_wrong);
        let axis_targets = context.axis_targets(&row.source_group, wrong_group.as_deref());
        let active_fringe = active_fringe(&row.input);
        Self {
            train_task: WavePredictorCompositionalTrainTask::from_axis_targets(
                active_fringe,
                &axis_targets,
                true,
            )
            .expect("5-axis VPN target should fit the compositional target cap"),
        }
    }
}

impl PreparedStateDeltaTask {
    fn new(row: CorpusRow) -> Self {
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
            .expect("accepted rows must yield a compact state delta");

        Self {
            train_task: WavePredictorStateDeltaTrainTask {
                active_fringe: active_fringe(&row.input),
                target_delta,
                binding_output_slot: None,
            },
        }
    }
}

trait EvalTaskExt {
    fn as_error_for_eval(&self, target_gap: i32) -> nando_core::WavePredictorConvergenceError;
}

impl EvalTaskExt for WavePredictorTrainTask {
    fn as_error_for_eval(&self, target_gap: i32) -> nando_core::WavePredictorConvergenceError {
        nando_core::WavePredictorConvergenceError {
            active_fringe: self.active_fringe.clone(),
            target_center: self.target_center,
            nearest_wrong_center: self.nearest_wrong_center,
            target_gap,
            margin_required: 0,
            trap_accepted: false,
        }
    }
}

#[derive(Clone, Debug)]
struct CorpusContext {
    operator_to_center: BTreeMap<String, WavePredictorCenterId>,
    source_group_to_operator: BTreeMap<String, String>,
    axis_value_to_center: BTreeMap<(String, String), WavePredictorCenterId>,
    axis_values: BTreeMap<String, Vec<String>>,
}

impl CorpusContext {
    fn new(rows: &[CorpusRow]) -> Self {
        let operators: BTreeSet<_> = rows.iter().map(|row| row.operator_family.clone()).collect();
        let operator_to_center = operators
            .into_iter()
            .enumerate()
            .map(|(index, operator)| (operator, index as WavePredictorCenterId))
            .collect();
        let source_group_to_operator = rows
            .iter()
            .map(|row| (row.source_group.clone(), row.operator_family.clone()))
            .collect();
        let mut axis_value_to_center = BTreeMap::new();
        let mut axis_values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut next_axis_center = 0 as WavePredictorCenterId;
        for row in rows {
            for (axis, value) in source_group_axes(&row.source_group) {
                let key = (axis.to_string(), value.to_string());
                if let Entry::Vacant(entry) = axis_value_to_center.entry(key) {
                    entry.insert(next_axis_center);
                    next_axis_center += 1;
                }
                axis_values
                    .entry(axis.to_string())
                    .or_default()
                    .insert(value.to_string());
            }
        }
        let axis_values = axis_values
            .into_iter()
            .map(|(axis, values)| (axis, values.into_iter().collect()))
            .collect();
        Self {
            operator_to_center,
            source_group_to_operator,
            axis_value_to_center,
            axis_values,
        }
    }

    fn prepare_rows(&self, rows: &[CorpusRow]) -> Vec<PreparedTask> {
        rows.iter()
            .cloned()
            .map(|row| PreparedTask::new(row, self))
            .collect()
    }

    fn prepare_compositional_rows(&self, rows: &[CorpusRow]) -> Vec<PreparedCompositionalTask> {
        rows.iter()
            .cloned()
            .map(|row| PreparedCompositionalTask::new(row, self))
            .collect()
    }

    fn operator_center(&self, operator_family: &str) -> WavePredictorCenterId {
        *self
            .operator_to_center
            .get(operator_family)
            .expect("operator family must be in corpus context")
    }

    fn operator_count(&self) -> usize {
        self.operator_to_center.len()
    }

    fn wrong_operator_center(&self, row: &CorpusRow) -> Option<WavePredictorCenterId> {
        let wrong_group = parenthesized_vpn_group(&row.why_negative_is_wrong)?;
        let wrong_operator = self.source_group_to_operator.get(&wrong_group)?;
        Some(self.operator_center(wrong_operator))
    }

    fn axis_targets(
        &self,
        source_group: &str,
        wrong_group: Option<&str>,
    ) -> Vec<WavePredictorAxisTarget> {
        let target_axes = source_group_axes(source_group);
        let wrong_axes = wrong_group.map(source_group_axes);
        target_axes
            .iter()
            .enumerate()
            .map(|(axis_index, (axis, target_value))| {
                let wrong_value = wrong_axes
                    .as_ref()
                    .map(|axes| axes[axis_index].1)
                    .filter(|wrong_value| wrong_value != target_value)
                    .unwrap_or_else(|| self.next_axis_value(axis, target_value));
                WavePredictorAxisTarget {
                    axis_id: axis_id(axis),
                    target_center: self.axis_center(axis, target_value),
                    nearest_wrong_center: self.axis_center(axis, wrong_value),
                }
            })
            .collect()
    }

    fn axis_center(&self, axis: &str, value: &str) -> WavePredictorCenterId {
        *self
            .axis_value_to_center
            .get(&(axis.to_string(), value.to_string()))
            .expect("axis value must be in corpus context")
    }

    fn next_axis_value(&self, axis: &str, current: &str) -> &str {
        let values = self
            .axis_values
            .get(axis)
            .expect("axis must be in corpus context");
        let index = values
            .iter()
            .position(|value| value == current)
            .expect("axis value must be listed");
        values[(index + 1) % values.len()].as_str()
    }
}

fn prepare_state_delta_rows(rows: &[CorpusRow]) -> Vec<PreparedStateDeltaTask> {
    rows.iter()
        .cloned()
        .map(PreparedStateDeltaTask::new)
        .collect()
}

#[derive(Clone, Debug)]
struct CorpusSplit {
    train: Vec<CorpusRow>,
    heldout: Vec<CorpusRow>,
}

fn split_by_source_group(rows: &[CorpusRow]) -> CorpusSplit {
    let heldout_groups = BTreeSet::from([
        "vpn_app_auth",
        "vpn_dns_suffix",
        "vpn_key_binding",
        "vpn_policy_routing",
        "vpn_safe_change",
    ]);
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for row in rows {
        if heldout_groups.contains(row.source_group.as_str()) {
            heldout.push(row.clone());
        } else {
            train.push(row.clone());
        }
    }
    CorpusSplit { train, heldout }
}

fn split_stratified_by_task_id(rows: &[CorpusRow]) -> CorpusSplit {
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for row in rows {
        if task_number(&row.task_id).is_multiple_of(5) {
            heldout.push(row.clone());
        } else {
            train.push(row.clone());
        }
    }
    CorpusSplit { train, heldout }
}

fn split_combinatorial_delta(rows: &[CorpusRow]) -> CorpusSplit {
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for row in rows {
        let combo = delta_combo_key(row);
        if stable_hash(&combo).is_multiple_of(5) {
            heldout.push(row.clone());
        } else {
            train.push(row.clone());
        }
    }
    CorpusSplit { train, heldout }
}

fn audit_combinatorial_delta_split(split: &CorpusSplit) -> CombinatorialDeltaSplitAudit {
    let train_combos: BTreeSet<_> = split.train.iter().map(delta_combo_key).collect();
    let heldout_combos: BTreeSet<_> = split.heldout.iter().map(delta_combo_key).collect();
    let train_parts: BTreeSet<_> = split.train.iter().flat_map(delta_part_keys).collect();

    let leaked_exact_combos = heldout_combos
        .iter()
        .filter(|combo| train_combos.contains(*combo))
        .count();
    let missing_seen_parts = split
        .heldout
        .iter()
        .flat_map(delta_part_keys)
        .filter(|part| !train_parts.contains(part))
        .count();

    CombinatorialDeltaSplitAudit {
        train_tasks: split.train.len(),
        heldout_tasks: split.heldout.len(),
        heldout_combos: heldout_combos.len(),
        leaked_exact_combos,
        missing_seen_parts,
    }
}

fn delta_combo_key(row: &CorpusRow) -> String {
    format!(
        "{}|{}|{}",
        row.source_group,
        input_field(&row.input, "Область проверки: "),
        input_field(&row.input, "Evidence window: ")
    )
}

fn delta_part_keys(row: &CorpusRow) -> Vec<String> {
    vec![
        format!("source_group={}", row.source_group),
        format!("scope={}", input_field(&row.input, "Область проверки: ")),
        format!("window={}", input_field(&row.input, "Evidence window: ")),
    ]
}

fn input_field(input: &str, prefix: &str) -> String {
    let start = input
        .find(prefix)
        .unwrap_or_else(|| panic!("input field prefix must exist: {prefix}"))
        + prefix.len();
    let rest = &input[start..];
    let end = rest.find('.').unwrap_or(rest.len());
    rest[..end].trim().to_string()
}

fn unseen_heldout_operator_count(train: &[CorpusRow], heldout: &[CorpusRow]) -> usize {
    let train_operators: BTreeSet<_> = train
        .iter()
        .map(|row| row.operator_family.as_str())
        .collect();
    heldout
        .iter()
        .map(|row| row.operator_family.as_str())
        .filter(|operator| !train_operators.contains(operator))
        .collect::<BTreeSet<_>>()
        .len()
}

fn unseen_heldout_axis_value_count(train: &[CorpusRow], heldout: &[CorpusRow]) -> usize {
    let train_axis_values: BTreeSet<_> = train
        .iter()
        .flat_map(|row| source_group_axes(&row.source_group).into_iter())
        .collect();
    heldout
        .iter()
        .flat_map(|row| source_group_axes(&row.source_group).into_iter())
        .filter(|axis_value| !train_axis_values.contains(axis_value))
        .collect::<BTreeSet<_>>()
        .len()
}

fn load_accepted_rows() -> Vec<CorpusRow> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ACCEPTED_10K);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read accepted 10k corpus at {}: {error}",
            path.display()
        )
    });
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> CorpusRow {
    CorpusRow {
        task_id: json_string(line, "task_id"),
        source_group: json_string(line, "source_group"),
        operator_family: json_string(line, "operator_family"),
        input: json_string(line, "input"),
        target: json_string(line, "target"),
        near_negative: json_string(line, "near_negative"),
        why_negative_is_wrong: json_string(line, "why_negative_is_wrong"),
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

fn active_fringe(input: &str) -> Vec<WavePredictorActiveCenter> {
    let tokens = tokens(input);
    let mut features: BTreeSet<String> = BTreeSet::new();
    for token in &tokens {
        features.insert(format!("tok:{token}"));
    }
    for pair in tokens.windows(2) {
        features.insert(format!("bi:{} {}", pair[0], pair[1]));
    }

    let mut features: Vec<_> = features.into_iter().collect();
    features.sort_by_key(|feature| stable_hash(feature));
    features
        .into_iter()
        .take(TOP_FEATURES)
        .map(|feature| WavePredictorActiveCenter {
            center_id: FEATURE_CENTER_BASE
                + (stable_hash(&feature) % u64::from(FEATURE_CENTER_COUNT))
                    as WavePredictorCenterId,
            strength: 4,
        })
        .collect()
}

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '/' | ':' | '-') {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            push_token(&mut out, &mut current);
        }
    }
    if !current.is_empty() {
        push_token(&mut out, &mut current);
    }
    out
}

fn push_token(out: &mut Vec<String>, current: &mut String) {
    if current.len() > 2 && !is_stop_token(current) {
        out.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn is_stop_token(token: &str) -> bool {
    matches!(
        token,
        "это"
            | "как"
            | "что"
            | "для"
            | "или"
            | "есть"
            | "без"
            | "при"
            | "над"
            | "под"
            | "слой"
            | "ход"
            | "контекст"
            | "область"
            | "следующий"
    )
}

fn parenthesized_vpn_group(text: &str) -> Option<String> {
    let start = text.find("(vpn_")? + 1;
    let rest = &text[start..];
    let end = rest.find(')')?;
    Some(rest[..end].to_string())
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

fn source_group_axes(source_group: &str) -> [(&'static str, &'static str); 5] {
    let layer = match source_group {
        "vpn_route_scope" | "vpn_policy_routing" | "vpn_rp_filter" | "vpn_nat_return" => "routing",
        "vpn_dns_layer" | "vpn_dns_suffix" | "vpn_split_dns" => "dns",
        "vpn_app_auth" | "vpn_auth_radius" | "vpn_key_binding" => "auth",
        "vpn_firewall_chain" | "vpn_firewall_port" => "firewall",
        "vpn_interface_layer"
        | "vpn_daemon_layer"
        | "vpn_endpoint_roaming"
        | "vpn_liveness_timeout"
        | "vpn_mtu_mss" => "link",
        "vpn_safe_change"
        | "vpn_minimal_change"
        | "vpn_rollback_smoke"
        | "vpn_rollout_gate"
        | "vpn_underfilled_refusal" => "safety",
        _ => "exposure",
    };
    let action = match source_group {
        "vpn_app_auth"
        | "vpn_auth_radius"
        | "vpn_key_binding"
        | "vpn_firewall_chain"
        | "vpn_capture_boundary"
        | "vpn_daemon_layer"
        | "vpn_interface_layer"
        | "vpn_socket_exposure" => "inspect_boundary",
        "vpn_safe_change"
        | "vpn_minimal_change"
        | "vpn_rollback_smoke"
        | "vpn_rollout_gate"
        | "vpn_underfilled_refusal" => "guard_change",
        "vpn_liveness_timeout" => "tune_parameter",
        _ => "repair_scope",
    };
    let evidence = match source_group {
        "vpn_auth_radius" | "vpn_liveness_timeout" | "vpn_daemon_layer" | "vpn_key_binding" => {
            "log_error"
        }
        "vpn_safe_change"
        | "vpn_minimal_change"
        | "vpn_rollback_smoke"
        | "vpn_rollout_gate"
        | "vpn_underfilled_refusal" => "change_state",
        "vpn_dns_layer" | "vpn_dns_suffix" | "vpn_split_dns" => "name_resolution",
        _ => "reachability_split",
    };
    let safety = match source_group {
        "vpn_underfilled_refusal" => "refusal_required",
        "vpn_safe_change" | "vpn_minimal_change" | "vpn_rollback_smoke" | "vpn_rollout_gate" => {
            "bounded_change"
        }
        _ => "minimal_evidence",
    };
    let scope = match layer {
        "routing" | "firewall" => "path_scope",
        "dns" => "resolver_scope",
        "auth" => "identity_scope",
        "safety" => "change_scope",
        _ => "link_scope",
    };
    [
        ("action", action),
        ("layer", layer),
        ("evidence", evidence),
        ("safety", safety),
        ("scope", scope),
    ]
}

fn axis_id(axis: &str) -> u16 {
    match axis {
        "action" => 0,
        "layer" => 1,
        "evidence" => 2,
        "safety" => 3,
        "scope" => 4,
        _ => 15,
    }
}

fn task_number(task_id: &str) -> usize {
    task_id
        .rsplit_once('_')
        .and_then(|(_, number)| number.parse::<usize>().ok())
        .expect("task id must end with numeric suffix")
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn milli_ratio(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator * 1000 / denominator
}
