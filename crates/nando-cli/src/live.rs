use nando_core::{
    BytePhaseLut, Cell32Learner, Cell32PromotionReport, LinkProfile, LinkTissue, LiveByteLearner,
    LiveBytePrediction, LiveByteTrainReport, OrganState, Stage2Organ, TickTrace,
};

pub(crate) fn print_live_byte_train(seed: u64, text: &str) {
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        println!("Nando Wave live byte train");
        println!("seed: {seed}");
        println!("cases: 0");
        println!("status: needs_at_least_two_bytes");
        return;
    }

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut reward_sum = 0.0f32;
    let mut confidence_sum = 0.0f32;
    let mut last_prediction = 0u8;
    let mut last_target = 0u8;

    for pair in bytes.windows(2) {
        let input = pair[0];
        let target = pair[1];
        let cycle = state.live_cycle(&organ, &lut, input, target);
        if cycle.update.correct {
            correct += 1;
        }
        reward_sum += cycle.update.reward;
        confidence_sum += cycle.prediction.confidence;
        last_prediction = cycle.prediction.predicted_byte;
        last_target = target;
    }

    let cases = bytes.len() - 1;
    println!("Nando Wave live byte train");
    println!("seed: {seed}");
    println!("cases: {cases}");
    println!("correct: {correct}");
    println!("accuracy: {:.4}", correct as f32 / cases as f32);
    println!("mean_reward: {:.4}", reward_sum / cases as f32);
    println!("mean_confidence: {:.4}", confidence_sum / cases as f32);
    println!("ticks: {}", state.tick_index);
    println!("coupling_mean: {:.4}", state.coupling_mean());
    println!("last_prediction_byte: {last_prediction}");
    println!("last_target_byte: {last_target}");
}

pub(crate) fn print_live_byte_learn(seed: u64, text: &str) {
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        println!("Nando Wave live byte learn");
        println!("seed: {seed}");
        println!("cases: 0");
        println!("status: needs_at_least_two_bytes");
        return;
    }

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut state = OrganState::new(seed, bytes[0]);
    let mut learner = LiveByteLearner::default();
    let mut steps = Vec::with_capacity(bytes.len() - 1);
    let mut traces = Vec::with_capacity(bytes.len() - 1);
    let mut primitive_correct = 0usize;
    let mut markov_seen = [false; 256];
    let mut markov_next = [0u8; 256];
    let mut markov_correct = 0usize;

    for pair in bytes.windows(2) {
        let input = pair[0];
        let target = pair[1];
        if markov_seen[input as usize] && markov_next[input as usize] == target {
            markov_correct += 1;
        }
        markov_seen[input as usize] = true;
        markov_next[input as usize] = target;

        let cycle = state.live_cycle(&organ, &lut, input, target);
        if cycle.update.correct {
            primitive_correct += 1;
        }
        steps.push(learner.update(&cycle.tick.trace, target));
        traces.push((cycle.tick.trace, target));
    }

    let report = LiveByteTrainReport::from_steps(&steps, &learner);
    let replay_correct = traces
        .iter()
        .filter(|(trace, target)| learner.predict(trace).predicted_byte == *target)
        .count();
    println!("Nando Wave live byte learn");
    println!("seed: {seed}");
    println!("cases: {}", report.cases);
    println!("primitive_correct: {primitive_correct}");
    println!(
        "primitive_accuracy: {:.4}",
        primitive_correct as f32 / report.cases.max(1) as f32
    );
    println!("last_next_baseline_correct: {markov_correct}");
    println!(
        "last_next_baseline_accuracy: {:.4}",
        markov_correct as f32 / report.cases.max(1) as f32
    );
    println!("learner_correct: {}", report.correct_before_update);
    println!("learner_accuracy: {:.4}", report.accuracy_before_update);
    println!("replay_correct_after_train: {replay_correct}");
    println!(
        "replay_accuracy_after_train: {:.4}",
        replay_correct as f32 / report.cases.max(1) as f32
    );
    println!("mean_confidence: {:.4}", report.mean_confidence);
    println!("mean_margin: {:.4}", report.mean_margin);
    println!("bias_abs_mean: {:.6}", report.bias_abs_mean);
    println!("class_weight_abs_mean: {:.6}", report.class_weight_abs_mean);
    println!("mode_weight_abs_mean: {:.6}", report.mode_weight_abs_mean);
    println!(
        "transition_weight_abs_mean: {:.6}",
        report.transition_weight_abs_mean
    );
    println!("weight_abs_mean: {:.6}", report.weight_abs_mean);
    println!(
        "context_weight_abs_mean: {:.6}",
        report.context_weight_abs_mean
    );
    println!("ticks: {}", state.tick_index);
    println!("coupling_mean: {:.4}", state.coupling_mean());
}

pub(crate) fn print_live_byte_holdout(seed: u64, text: &str) {
    let bytes = text.as_bytes();
    if bytes.len() < 4 {
        println!("Nando Wave live byte holdout");
        println!("seed: {seed}");
        println!("status: needs_at_least_four_bytes");
        return;
    }

    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut learner = LiveByteLearner::default();
    let train_report = train_live_adapter(seed, &organ, &lut, &mut learner, train);
    let holdout_report = eval_live_adapter(seed, &organ, &lut, &learner, holdout);

    println!("Nando Wave live byte holdout");
    println!("seed: {seed}");
    println!("train_bytes: {}", train.len());
    println!("holdout_bytes: {}", holdout.len());
    println!("train_cases: {}", train_report.cases);
    println!(
        "train_learner_accuracy: {:.4}",
        train_report.learner_accuracy
    );
    println!(
        "train_last_next_baseline_accuracy: {:.4}",
        train_report.last_next_baseline_accuracy
    );
    println!("holdout_cases: {}", holdout_report.cases);
    println!(
        "holdout_learner_accuracy: {:.4}",
        holdout_report.learner_accuracy
    );
    println!(
        "holdout_last_next_baseline_accuracy: {:.4}",
        holdout_report.last_next_baseline_accuracy
    );
    println!(
        "holdout_replay_gap: {:.4}",
        holdout_report.learner_accuracy - holdout_report.last_next_baseline_accuracy
    );
    println!(
        "context_weight_abs_mean: {:.6}",
        train_report.context_weight_abs_mean
    );
}

pub(crate) fn print_live_byte_holdout_suite(seed: u64) {
    println!("Nando Wave live byte holdout suite");
    println!("seed: {seed}");
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>9} {:>8} {:>11}",
        "case", "train_n", "hold_n", "train", "holdout", "base", "gap"
    );
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>9} {:>8} {:>11}",
        "--------------",
        "--------",
        "--------",
        "--------",
        "---------",
        "--------",
        "-----------"
    );

    for (name, text) in LIVE_HOLDOUT_CASES {
        let result = live_byte_holdout_result(seed, text.as_bytes());
        println!(
            "{:<14} {:>8} {:>8} {:>8.4} {:>9.4} {:>8.4} {:>+11.4}",
            name,
            result.train_cases,
            result.holdout_cases,
            result.train_learner_accuracy,
            result.holdout_learner_accuracy,
            result.holdout_last_next_baseline_accuracy,
            result.holdout_gap,
        );
    }
}

pub(crate) fn print_live_byte_holdout_seed_sweep() {
    let seeds = [1, 7, 13, 29, 97];

    println!("Nando Wave live byte holdout seed sweep");
    println!("seeds: {:?}", seeds);
    println!(
        "{:<14} {:>5} {:>8} {:>8} {:>9} {:>8} {:>9}",
        "case", "wins", "mean", "base", "mean_gap", "worst", "oos"
    );
    println!(
        "{:<14} {:>5} {:>8} {:>8} {:>9} {:>8} {:>9}",
        "--------------", "-----", "--------", "--------", "---------", "--------", "---------"
    );

    for (name, text) in LIVE_HOLDOUT_CASES {
        let mut wins = 0usize;
        let mut holdout_sum = 0.0f32;
        let mut baseline_sum = 0.0f32;
        let mut gap_sum = 0.0f32;
        let mut oos_sum = 0.0f32;
        let mut worst_gap = f32::INFINITY;

        for seed in seeds {
            let result = live_byte_holdout_result(seed, text.as_bytes());
            if result.holdout_gap > 0.0 {
                wins += 1;
            }
            holdout_sum += result.holdout_learner_accuracy;
            baseline_sum += result.holdout_last_next_baseline_accuracy;
            gap_sum += result.holdout_gap;
            oos_sum += result.holdout_oos_target_rate;
            worst_gap = worst_gap.min(result.holdout_gap);
        }

        let total = seeds.len() as f32;
        println!(
            "{:<14} {:>5} {:>8.4} {:>8.4} {:>+9.4} {:>+8.4} {:>9.4}",
            name,
            wins,
            holdout_sum / total,
            baseline_sum / total,
            gap_sum / total,
            worst_gap,
            oos_sum / total
        );
    }
}

pub(crate) fn print_live_cell_promote(seed: u64, text: &str) {
    let bytes = text.as_bytes();
    if bytes.len() < 4 {
        println!("Nando Wave live cell promote");
        println!("seed: {seed}");
        println!("status: needs_at_least_four_bytes");
        return;
    }

    let result = live_cell_promotion_result(seed, bytes, 6);
    println!("Nando Wave live cell promote");
    println!("seed: {seed}");
    println!("cells_enabled: {}", result.cells_enabled);
    println!("train_cases: {}", result.report.train_cases);
    println!("holdout_cases: {}", result.report.holdout_cases);
    println!("base_accuracy: {:.4}", result.report.base_accuracy);
    println!(
        "candidate_accuracy: {:.4}",
        result.report.candidate_accuracy
    );
    println!("holdout_gap: {:+.4}", result.report.holdout_gap);
    println!("oos_target_rate: {:.4}", result.report.oos_target_rate);
    println!(
        "candidate_state_abs_mean: {:.6}",
        result.candidate_state_abs_mean
    );
    println!("accepted: {}", result.report.accepted);
    println!(
        "mode_status: {}",
        if result.report.accepted {
            "cell32_candidate_promoted"
        } else {
            "not_found_cell32_candidate_rejected"
        }
    );
}

pub(crate) fn print_live_architecture_compare(seed: u64) {
    println!("Nando Wave live architecture compare");
    println!("seed: {seed}");
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9}",
        "case", "cell3", "mono96", "cell6", "pair", "triple", "mono192", "status"
    );
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9}",
        "--------------",
        "--------",
        "--------",
        "--------",
        "--------",
        "--------",
        "--------",
        "---------"
    );

    let mut cell3_wins = 0usize;
    let mut cell6_wins = 0usize;
    let mut pair_wins = 0usize;
    let mut triple_wins = 0usize;

    for (name, text) in LIVE_HOLDOUT_CASES {
        let row = architecture_compare_result(seed, text.as_bytes());
        if row.cell3_accuracy > row.mono96_accuracy {
            cell3_wins += 1;
        }
        if row.cell6_accuracy > row.mono192_accuracy {
            cell6_wins += 1;
        }
        if row.cell6_pair_accuracy > row.cell6_accuracy {
            pair_wins += 1;
        }
        if row.cell6_triple_accuracy > row.cell6_pair_accuracy {
            triple_wins += 1;
        }

        println!(
            "{:<14} {:>8.4} {:>8.4} {:>8.4} {:>8.4} {:>8.4} {:>8.4} {:>9}",
            name,
            row.cell3_accuracy,
            row.mono96_accuracy,
            row.cell6_accuracy,
            row.cell6_pair_accuracy,
            row.cell6_triple_accuracy,
            row.mono192_accuracy,
            row.status
        );
    }

    let mode_status = if triple_wins >= 3 && cell6_wins >= 3 {
        "link_tissue_topology_candidate"
    } else if pair_wins >= 3 {
        "pair_tissue_candidate_needs_triples"
    } else if cell3_wins >= 3 && cell6_wins >= 3 {
        "cellular_topology_candidate"
    } else {
        "not_found_cellular_topology_advantage"
    };
    println!("cell3_wins_over_mono96: {cell3_wins}");
    println!("cell6_wins_over_mono192: {cell6_wins}");
    println!("pair_tissue_wins_over_cell6: {pair_wins}");
    println!("triple_tissue_wins_over_pair: {triple_wins}");
    println!("mode_status: {mode_status}");
}

pub(crate) fn print_live_tissue_diagnose(seed: u64) {
    println!("Nando Wave live tissue diagnose");
    println!("seed: {seed}");
    println!(
        "{:<14} {:>7} {:>7} {:>7} {:>7} {:>9} {:>9} {:>9}",
        "case", "cell6", "pair", "typed2", "typed3", "best_pair", "pair_drop", "triple"
    );
    println!(
        "{:<14} {:>7} {:>7} {:>7} {:>7} {:>9} {:>9} {:>9}",
        "--------------",
        "-------",
        "-------",
        "-------",
        "-------",
        "---------",
        "---------",
        "---------"
    );

    let mut typed_pair_wins = 0usize;
    let mut typed_triple_wins = 0usize;
    let mut ablation_positive = 0usize;

    for (name, text) in LIVE_HOLDOUT_CASES {
        let row = tissue_diagnose_result(seed, text.as_bytes());
        if row.typed_pair_accuracy > row.cell6_accuracy {
            typed_pair_wins += 1;
        }
        if row.typed_triple_accuracy > row.typed_pair_accuracy {
            typed_triple_wins += 1;
        }
        if row.best_pair_drop > 0.0 {
            ablation_positive += 1;
        }

        println!(
            "{:<14} {:>7.4} {:>7.4} {:>7.4} {:>7.4} {:>9} {:>+9.4} {:>9}",
            name,
            row.cell6_accuracy,
            row.pair_accuracy,
            row.typed_pair_accuracy,
            row.typed_triple_accuracy,
            row.best_pair_label,
            row.best_pair_drop,
            row.best_triple_label
        );
    }

    let mode_status = if typed_triple_wins >= 3 && ablation_positive >= 3 {
        "typed_triple_tissue_candidate"
    } else if typed_pair_wins >= 3 && ablation_positive >= 2 {
        "typed_pair_tissue_candidate_needs_triples"
    } else if ablation_positive >= 2 {
        "pair_ablation_signal_needs_typed_gain"
    } else {
        "not_found_tissue_synergy"
    };
    println!("typed_pair_wins_over_cell6: {typed_pair_wins}");
    println!("typed_triple_wins_over_typed_pair: {typed_triple_wins}");
    println!("positive_pair_ablation_cases: {ablation_positive}");
    println!("mode_status: {mode_status}");
}

pub(crate) fn print_live_grok_trace(seed: u64, epochs: usize, interval: usize) {
    print_live_grok_trace_for_rule(seed, epochs, interval, GrokUpdateRule::Decay);
}

fn print_live_grok_trace_for_rule(seed: u64, epochs: usize, interval: usize, rule: GrokUpdateRule) {
    let (case_name, text) = LIVE_HOLDOUT_CASES[1];
    let bytes = text.as_bytes();
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];
    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut cell6 = Cell32Learner::new(6, 0.08);
    let mut tissue = LinkTissue::with_profile(6, true, 0.08, LinkProfile::Typed);
    let mut summary = GrokTraceSummary::default();

    println!("Nando Wave live grok trace");
    println!("seed: {seed}");
    println!("case: {case_name}");
    println!("rule: {}", rule.label());
    println!("epochs: {epochs}");
    println!("interval: {interval}");
    println!(
        "{:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>9} {:>8} {:>8} {:>7}",
        "epoch",
        "cell6",
        "full",
        "fgain",
        "restr",
        "excl",
        "drop",
        "top_pair",
        "p_energy",
        "p_gini",
        "signal"
    );
    println!(
        "{:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>9} {:>8} {:>8} {:>7}",
        "-----",
        "-------",
        "-------",
        "-------",
        "-------",
        "-------",
        "-------",
        "---------",
        "--------",
        "--------",
        "-------"
    );

    for epoch in 0..=epochs {
        if epoch == 0 || epoch == epochs || epoch % interval == 0 {
            let row = grok_trace_row(seed, &organ, &lut, &cell6, &tissue, holdout);
            summary.observe(row);
            println!(
                "{:>5} {:>7.4} {:>7.4} {:>+7.4} {:>7.4} {:>7.4} {:>+7.4} {:>9} {:>8.5} {:>8.4} {:>7}",
                epoch,
                row.cell6_accuracy,
                row.full_accuracy,
                row.full_gain,
                row.restricted_accuracy,
                row.excluded_accuracy,
                row.excluded_drop,
                row.top_pair_label,
                row.pair_energy,
                row.pair_gini,
                row.signal
            );
        }
        if epoch < epochs {
            train_cell32_candidate(seed, &organ, &lut, &mut cell6, train);
            train_link_tissue_with_rule(seed, &organ, &lut, &cell6, &mut tissue, train, rule);
        }
    }

    println!("signal_legend: none < warmup < circuit_seed < grok_candidate");
    println!("circuit_seed_count: {}", summary.circuit_seed_count);
    println!("grok_candidate_count: {}", summary.grok_candidate_count);
    println!("best_same_pair_streak: {}", summary.best_same_pair_streak);
    println!("best_full_gain: {:+.4}", summary.best_full_gain);
    println!("best_restricted_gain: {:+.4}", summary.best_restricted_gain);
    println!("best_excluded_drop: {:+.4}", summary.best_excluded_drop);
    println!("stable_status: {}", summary.stable_status());
}

pub(crate) fn print_live_grok_sweep(epochs: usize, interval: usize) {
    println!("Nando Wave live grok sweep");
    println!("case: code_like");
    println!("epochs: {epochs}");
    println!("interval: {interval}");
    println!(
        "{:>5} {:>10} {:>23} {:>5} {:>5} {:>6} {:>6} {:>6} {:>5}",
        "seed", "rule", "status", "circ", "grok", "streak", "fgain", "drop", "rgain"
    );
    println!(
        "{:>5} {:>10} {:>23} {:>5} {:>5} {:>6} {:>6} {:>6} {:>5}",
        "-----",
        "----------",
        "-----------------------",
        "-----",
        "-----",
        "------",
        "------",
        "------",
        "-----"
    );

    let seeds = [1_u64, 3, 5, 7, 11, 13, 17, 19, 23, 29];
    let rules = [
        GrokUpdateRule::Perceptron,
        GrokUpdateRule::Decay,
        GrokUpdateRule::Margin,
        GrokUpdateRule::RestrictedPairBoost,
    ];
    let mut stable_count = 0usize;
    let mut unstable_count = 0usize;

    for seed in seeds {
        for rule in rules {
            let summary = grok_summary_for_rule(seed, epochs, interval, rule);
            if summary.stable_status() == "stable_grok_candidate" {
                stable_count += 1;
            } else if summary.stable_status() == "unstable_circuit_seed" {
                unstable_count += 1;
            }
            println!(
                "{:>5} {:>10} {:>23} {:>5} {:>5} {:>6} {:>+6.3} {:>+6.3} {:>+5.3}",
                seed,
                rule.label(),
                summary.stable_status(),
                summary.circuit_seed_count,
                summary.grok_candidate_count,
                summary.best_same_pair_streak,
                summary.best_full_gain,
                summary.best_excluded_drop,
                summary.best_restricted_gain
            );
        }
    }

    println!("stable_grok_candidate_count: {stable_count}");
    println!("unstable_circuit_seed_count: {unstable_count}");
}

const LIVE_HOLDOUT_CASES: [(&str, &str); 5] = [
    ("repeat", "abababababababababababababababab"),
    (
        "code_like",
        "let value = value + 1; let value = value + 2; let value = value + 3;",
    ),
    (
        "ru_text",
        "privet mir privet nanda privet volny privet kletki",
    ),
    (
        "mixed_balanced",
        "ghbdtn privet файл file ghbdtn privet файл file ghbdtn privet файл file",
    ),
    (
        "mixed_shift",
        "ghbdtn privet ghbdtn privet файл file файл file",
    ),
];

const PAIR_LABELS: [((usize, usize), &str); 15] = [
    ((0, 1), "0-1"),
    ((0, 2), "0-2"),
    ((0, 3), "0-3"),
    ((0, 4), "0-4"),
    ((0, 5), "0-5"),
    ((1, 2), "1-2"),
    ((1, 3), "1-3"),
    ((1, 4), "1-4"),
    ((1, 5), "1-5"),
    ((2, 3), "2-3"),
    ((2, 4), "2-4"),
    ((2, 5), "2-5"),
    ((3, 4), "3-4"),
    ((3, 5), "3-5"),
    ((4, 5), "4-5"),
];

#[derive(Debug, Clone, Copy)]
struct LiveAdapterPassReport {
    cases: usize,
    learner_accuracy: f32,
    last_next_baseline_accuracy: f32,
    context_weight_abs_mean: f32,
}

#[derive(Debug, Clone, Copy)]
struct LiveHoldoutResult {
    train_cases: usize,
    holdout_cases: usize,
    train_learner_accuracy: f32,
    holdout_learner_accuracy: f32,
    holdout_last_next_baseline_accuracy: f32,
    holdout_gap: f32,
    holdout_oos_target_rate: f32,
}

#[derive(Debug, Clone, Copy)]
struct LiveCellPromotionResult {
    cells_enabled: usize,
    report: Cell32PromotionReport,
    candidate_state_abs_mean: f32,
}

#[derive(Debug, Clone, Copy)]
struct ArchitectureCompareRow {
    cell3_accuracy: f32,
    mono96_accuracy: f32,
    cell6_accuracy: f32,
    cell6_pair_accuracy: f32,
    cell6_triple_accuracy: f32,
    mono192_accuracy: f32,
    status: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct TissueDiagnoseRow {
    cell6_accuracy: f32,
    pair_accuracy: f32,
    typed_pair_accuracy: f32,
    typed_triple_accuracy: f32,
    best_pair_label: &'static str,
    best_pair_drop: f32,
    best_triple_label: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct GrokTraceRow {
    cell6_accuracy: f32,
    full_accuracy: f32,
    full_gain: f32,
    restricted_accuracy: f32,
    restricted_gain: f32,
    excluded_accuracy: f32,
    excluded_drop: f32,
    top_pair_label: &'static str,
    pair_energy: f32,
    pair_gini: f32,
    signal: &'static str,
}

#[derive(Debug, Clone)]
struct GrokTraceSummary {
    previous_pair_label: &'static str,
    same_pair_streak: usize,
    best_same_pair_streak: usize,
    circuit_seed_count: usize,
    grok_candidate_count: usize,
    best_full_gain: f32,
    best_restricted_gain: f32,
    best_excluded_drop: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokUpdateRule {
    Perceptron,
    Decay,
    Margin,
    RestrictedPairBoost,
}

impl GrokUpdateRule {
    fn label(self) -> &'static str {
        match self {
            Self::Perceptron => "perceptron",
            Self::Decay => "decay",
            Self::Margin => "margin",
            Self::RestrictedPairBoost => "boost",
        }
    }
}

impl Default for GrokTraceSummary {
    fn default() -> Self {
        Self {
            previous_pair_label: "",
            same_pair_streak: 0,
            best_same_pair_streak: 0,
            circuit_seed_count: 0,
            grok_candidate_count: 0,
            best_full_gain: f32::NEG_INFINITY,
            best_restricted_gain: f32::NEG_INFINITY,
            best_excluded_drop: f32::NEG_INFINITY,
        }
    }
}

impl GrokTraceSummary {
    fn observe(&mut self, row: GrokTraceRow) {
        if row.top_pair_label == self.previous_pair_label {
            self.same_pair_streak += 1;
        } else {
            self.previous_pair_label = row.top_pair_label;
            self.same_pair_streak = 1;
        }
        self.best_same_pair_streak = self.best_same_pair_streak.max(self.same_pair_streak);

        match row.signal {
            "grok_candidate" => self.grok_candidate_count += 1,
            "circuit_seed" => self.circuit_seed_count += 1,
            _ => {}
        }

        self.best_full_gain = self.best_full_gain.max(row.full_gain);
        self.best_restricted_gain = self.best_restricted_gain.max(row.restricted_gain);
        self.best_excluded_drop = self.best_excluded_drop.max(row.excluded_drop);
    }

    fn stable_status(&self) -> &'static str {
        if self.grok_candidate_count >= 3
            && self.best_same_pair_streak >= 3
            && self.best_full_gain > 0.0
            && self.best_restricted_gain > 0.0
            && self.best_excluded_drop > 0.0
        {
            "stable_grok_candidate"
        } else if self.circuit_seed_count + self.grok_candidate_count >= 3
            && self.best_restricted_gain > 0.0
            && self.best_excluded_drop > 0.0
        {
            "unstable_circuit_seed"
        } else if self.circuit_seed_count + self.grok_candidate_count > 0 {
            "weak_circuit_hint"
        } else {
            "not_found"
        }
    }
}

#[derive(Debug, Clone)]
struct MonoByteLearner {
    rows_enabled: usize,
    learning_rate: f32,
    byte_bias: [f32; 256],
    row_byte_weights: Vec<[f32; 256]>,
    input_row_byte_weights: Vec<[f32; 256]>,
}

fn live_byte_holdout_result(seed: u64, bytes: &[u8]) -> LiveHoldoutResult {
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut learner = LiveByteLearner::default();
    let train_report = train_live_adapter(seed, &organ, &lut, &mut learner, train);
    let holdout_report = eval_live_adapter(seed, &organ, &lut, &learner, holdout);
    let holdout_oos_target_rate = holdout_oos_target_rate(train, holdout);

    LiveHoldoutResult {
        train_cases: train_report.cases,
        holdout_cases: holdout_report.cases,
        train_learner_accuracy: train_report.learner_accuracy,
        holdout_learner_accuracy: holdout_report.learner_accuracy,
        holdout_last_next_baseline_accuracy: holdout_report.last_next_baseline_accuracy,
        holdout_gap: holdout_report.learner_accuracy - holdout_report.last_next_baseline_accuracy,
        holdout_oos_target_rate,
    }
}

fn live_cell_promotion_result(
    seed: u64,
    bytes: &[u8],
    cells_enabled: usize,
) -> LiveCellPromotionResult {
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut candidate = Cell32Learner::new(cells_enabled, 0.08);
    let train_cases = train_cell32_candidate(seed, &organ, &lut, &mut candidate, train);
    let candidate_accuracy = eval_cell32_candidate(seed, &organ, &lut, &candidate, holdout);
    let base_accuracy = eval_last_next_baseline(holdout);
    let oos_target_rate = holdout_oos_target_rate(train, holdout);
    let report = Cell32PromotionReport::new(
        train_cases,
        holdout.len().saturating_sub(1),
        base_accuracy,
        candidate_accuracy,
        oos_target_rate,
    );

    LiveCellPromotionResult {
        cells_enabled,
        report,
        candidate_state_abs_mean: candidate.state_abs_mean(),
    }
}

fn architecture_compare_result(seed: u64, bytes: &[u8]) -> ArchitectureCompareRow {
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut cell3 = Cell32Learner::new(3, 0.08);
    let mut cell6 = Cell32Learner::new(6, 0.08);
    let mut pair_tissue = LinkTissue::new(6, false, 0.08);
    let mut triple_tissue = LinkTissue::new(6, true, 0.08);
    let mut mono96 = MonoByteLearner::new(3, 0.08);
    let mut mono192 = MonoByteLearner::new(6, 0.08);

    train_cell32_candidate(seed, &organ, &lut, &mut cell3, train);
    train_cell32_candidate(seed, &organ, &lut, &mut cell6, train);
    train_link_tissue(seed, &organ, &lut, &cell6, &mut pair_tissue, train);
    train_link_tissue(seed, &organ, &lut, &cell6, &mut triple_tissue, train);
    train_mono_candidate(seed, &organ, &lut, &mut mono96, train);
    train_mono_candidate(seed, &organ, &lut, &mut mono192, train);

    let cell3_accuracy = eval_cell32_candidate(seed, &organ, &lut, &cell3, holdout);
    let cell6_accuracy = eval_cell32_candidate(seed, &organ, &lut, &cell6, holdout);
    let cell6_pair_accuracy =
        eval_cell32_with_tissue(seed, &organ, &lut, &cell6, &pair_tissue, holdout);
    let cell6_triple_accuracy =
        eval_cell32_with_tissue(seed, &organ, &lut, &cell6, &triple_tissue, holdout);
    let mono96_accuracy = eval_mono_candidate(seed, &organ, &lut, &mono96, holdout);
    let mono192_accuracy = eval_mono_candidate(seed, &organ, &lut, &mono192, holdout);
    let status =
        if cell6_triple_accuracy > mono192_accuracy && cell6_triple_accuracy > cell6_accuracy {
            "tissue"
        } else if cell3_accuracy <= mono96_accuracy && cell6_accuracy <= mono192_accuracy {
            "mono"
        } else {
            "mixed"
        };

    ArchitectureCompareRow {
        cell3_accuracy,
        mono96_accuracy,
        cell6_accuracy,
        cell6_pair_accuracy,
        cell6_triple_accuracy,
        mono192_accuracy,
        status,
    }
}

fn tissue_diagnose_result(seed: u64, bytes: &[u8]) -> TissueDiagnoseRow {
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];

    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut cell6 = Cell32Learner::new(6, 0.08);
    let mut pair = LinkTissue::new(6, false, 0.08);
    let mut typed_pair = LinkTissue::with_profile(6, false, 0.08, LinkProfile::Typed);
    let mut typed_triple = LinkTissue::with_profile(6, true, 0.08, LinkProfile::Typed);

    train_cell32_candidate(seed, &organ, &lut, &mut cell6, train);
    train_link_tissue(seed, &organ, &lut, &cell6, &mut pair, train);
    train_link_tissue(seed, &organ, &lut, &cell6, &mut typed_pair, train);
    train_link_tissue(seed, &organ, &lut, &cell6, &mut typed_triple, train);

    let cell6_accuracy = eval_cell32_candidate(seed, &organ, &lut, &cell6, holdout);
    let pair_accuracy = eval_cell32_with_tissue(seed, &organ, &lut, &cell6, &pair, holdout);
    let typed_pair_accuracy =
        eval_cell32_with_tissue(seed, &organ, &lut, &cell6, &typed_pair, holdout);
    let typed_triple_accuracy =
        eval_cell32_with_tissue(seed, &organ, &lut, &cell6, &typed_triple, holdout);
    let (best_pair_label, best_pair_drop) =
        best_pair_ablation(seed, &organ, &lut, &cell6, &pair, holdout, pair_accuracy);
    let best_triple_label = best_triple_energy_label(&typed_triple);

    TissueDiagnoseRow {
        cell6_accuracy,
        pair_accuracy,
        typed_pair_accuracy,
        typed_triple_accuracy,
        best_pair_label,
        best_pair_drop,
        best_triple_label,
    }
}

fn grok_summary_for_rule(
    seed: u64,
    epochs: usize,
    interval: usize,
    rule: GrokUpdateRule,
) -> GrokTraceSummary {
    let (_, text) = LIVE_HOLDOUT_CASES[1];
    let bytes = text.as_bytes();
    let split = bytes.len() / 2;
    let train = &bytes[..split];
    let holdout = &bytes[split.saturating_sub(1)..];
    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut cell6 = Cell32Learner::new(6, 0.08);
    let mut tissue = LinkTissue::with_profile(6, true, 0.08, LinkProfile::Typed);
    let mut summary = GrokTraceSummary::default();

    for epoch in 0..=epochs {
        if epoch == 0 || epoch == epochs || epoch % interval == 0 {
            let row = grok_trace_row(seed, &organ, &lut, &cell6, &tissue, holdout);
            summary.observe(row);
        }
        if epoch < epochs {
            train_cell32_candidate(seed, &organ, &lut, &mut cell6, train);
            train_link_tissue_with_rule(seed, &organ, &lut, &cell6, &mut tissue, train, rule);
        }
    }

    summary
}

fn grok_trace_row(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    holdout: &[u8],
) -> GrokTraceRow {
    let cell6_accuracy = eval_cell32_candidate(seed, organ, lut, cell, holdout);
    let full_accuracy = eval_cell32_with_tissue(seed, organ, lut, cell, tissue, holdout);
    let (top_pair, top_pair_label, pair_energy) = top_pair_energy(tissue);
    let restricted_accuracy =
        eval_cell32_with_tissue_restricted_pair(seed, organ, lut, cell, tissue, top_pair, holdout);
    let excluded_accuracy =
        eval_cell32_with_tissue_without_pair(seed, organ, lut, cell, tissue, top_pair, holdout);
    let excluded_drop = full_accuracy - excluded_accuracy;
    let full_gain = full_accuracy - cell6_accuracy;
    let restricted_gain = restricted_accuracy - cell6_accuracy;
    let pair_gini = pair_energy_gini(tissue);
    let signal = grok_signal(
        full_gain,
        restricted_gain,
        excluded_drop,
        pair_energy,
        pair_gini,
    );

    GrokTraceRow {
        cell6_accuracy,
        full_accuracy,
        full_gain,
        restricted_accuracy,
        restricted_gain,
        excluded_accuracy,
        excluded_drop,
        top_pair_label,
        pair_energy,
        pair_gini,
        signal,
    }
}

fn grok_signal(
    full_gain: f32,
    restricted_gain: f32,
    excluded_drop: f32,
    pair_energy: f32,
    pair_gini: f32,
) -> &'static str {
    if full_gain > 0.0 && restricted_gain > 0.0 && excluded_drop > 0.0 && pair_gini >= 0.55 {
        "grok_candidate"
    } else if restricted_gain > 0.0 || excluded_drop > 0.0 {
        "circuit_seed"
    } else if pair_energy > 0.0 {
        "warmup"
    } else {
        "none"
    }
}

fn holdout_oos_target_rate(train: &[u8], holdout: &[u8]) -> f32 {
    let mut seen = [false; 256];
    for byte in train {
        seen[*byte as usize] = true;
    }

    let mut cases = 0usize;
    let mut oos = 0usize;
    for pair in holdout.windows(2) {
        let target = pair[1];
        if !seen[target as usize] {
            oos += 1;
        }
        cases += 1;
    }

    oos as f32 / cases.max(1) as f32
}

fn eval_last_next_baseline(bytes: &[u8]) -> f32 {
    let mut markov_seen = [false; 256];
    let mut markov_next = [0u8; 256];
    let mut markov_correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let input = pair[0];
        let target = pair[1];
        if markov_seen[input as usize] && markov_next[input as usize] == target {
            markov_correct += 1;
        }
        markov_seen[input as usize] = true;
        markov_next[input as usize] = target;
        cases += 1;
    }

    markov_correct as f32 / cases.max(1) as f32
}

fn train_cell32_candidate(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &mut Cell32Learner,
    bytes: &[u8],
) -> usize {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        learner.update(&cycle.tick.trace, pair[1]);
        cases += 1;
    }

    cases
}

fn eval_cell32_candidate(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &Cell32Learner,
    bytes: &[u8],
) -> f32 {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        if learner.predict(&cycle.tick.trace).predicted_byte == pair[1] {
            correct += 1;
        }
        cases += 1;
    }

    correct as f32 / cases.max(1) as f32
}

fn train_link_tissue(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &mut LinkTissue,
    bytes: &[u8],
) {
    let mut state = OrganState::new(seed, bytes[0]);

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        let prediction = predict_cell32_with_tissue(cell, tissue, &cycle.tick.trace);
        tissue.update_from_prediction(&cycle.tick.trace, pair[1], prediction.predicted_byte);
    }
}

fn train_link_tissue_with_rule(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &mut LinkTissue,
    bytes: &[u8],
    rule: GrokUpdateRule,
) {
    let mut state = OrganState::new(seed, bytes[0]);

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        let trace = &cycle.tick.trace;
        let prediction = predict_cell32_with_tissue(cell, tissue, trace);

        match rule {
            GrokUpdateRule::Perceptron => {
                tissue.update_from_prediction(trace, pair[1], prediction.predicted_byte);
            }
            GrokUpdateRule::Decay => {
                tissue.apply_decay(0.999);
                tissue.update_from_prediction(trace, pair[1], prediction.predicted_byte);
            }
            GrokUpdateRule::Margin => {
                let target_score = cell.score(trace, pair[1]) + tissue.score(trace, pair[1]);
                if target_score <= prediction.score + 0.05 {
                    tissue.update_from_prediction(trace, pair[1], prediction.predicted_byte);
                }
            }
            GrokUpdateRule::RestrictedPairBoost => {
                tissue.apply_decay(0.999);
                tissue.update_from_prediction(trace, pair[1], prediction.predicted_byte);
                let (top_pair, _, _) = top_pair_energy(tissue);
                tissue.update_pair_from_prediction(
                    trace,
                    pair[1],
                    prediction.predicted_byte,
                    top_pair,
                    0.75,
                );
            }
        }
    }
}

fn eval_cell32_with_tissue(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    bytes: &[u8],
) -> f32 {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        if predict_cell32_with_tissue(cell, tissue, &cycle.tick.trace).predicted_byte == pair[1] {
            correct += 1;
        }
        cases += 1;
    }

    correct as f32 / cases.max(1) as f32
}

fn eval_cell32_with_tissue_without_pair(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    disabled_pair: (usize, usize),
    bytes: &[u8],
) -> f32 {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        if predict_cell32_with_tissue_without_pair(cell, tissue, &cycle.tick.trace, disabled_pair)
            .predicted_byte
            == pair[1]
        {
            correct += 1;
        }
        cases += 1;
    }

    correct as f32 / cases.max(1) as f32
}

fn eval_cell32_with_tissue_restricted_pair(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    restricted_pair: (usize, usize),
    bytes: &[u8],
) -> f32 {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        if predict_cell32_with_tissue_restricted_pair(
            cell,
            tissue,
            &cycle.tick.trace,
            restricted_pair,
        )
        .predicted_byte
            == pair[1]
        {
            correct += 1;
        }
        cases += 1;
    }

    correct as f32 / cases.max(1) as f32
}

fn predict_cell32_with_tissue(
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    trace: &TickTrace,
) -> LiveBytePrediction {
    let mut best_byte = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    let mut second_score = f32::NEG_INFINITY;

    for byte in 0..=u8::MAX {
        let score = cell.score(trace, byte) + tissue.score(trace, byte);
        if score > best_score {
            second_score = best_score;
            best_score = score;
            best_byte = byte;
        } else if score > second_score {
            second_score = score;
        }
    }

    let margin = best_score - second_score;
    LiveBytePrediction {
        predicted_byte: best_byte,
        confidence: (1.0 / (1.0 + (-margin).exp())).clamp(0.0, 1.0),
        score: best_score,
    }
}

fn predict_cell32_with_tissue_restricted_pair(
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    trace: &TickTrace,
    restricted_pair: (usize, usize),
) -> LiveBytePrediction {
    let mut best_byte = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    let mut second_score = f32::NEG_INFINITY;

    for byte in 0..=u8::MAX {
        let full_tissue = tissue.score(trace, byte);
        let without_pair = tissue.score_without_pair(trace, byte, restricted_pair);
        let score = cell.score(trace, byte) + full_tissue - without_pair;
        if score > best_score {
            second_score = best_score;
            best_score = score;
            best_byte = byte;
        } else if score > second_score {
            second_score = score;
        }
    }

    let margin = best_score - second_score;
    LiveBytePrediction {
        predicted_byte: best_byte,
        confidence: (1.0 / (1.0 + (-margin).exp())).clamp(0.0, 1.0),
        score: best_score,
    }
}

fn predict_cell32_with_tissue_without_pair(
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    trace: &TickTrace,
    disabled_pair: (usize, usize),
) -> LiveBytePrediction {
    let mut best_byte = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    let mut second_score = f32::NEG_INFINITY;

    for byte in 0..=u8::MAX {
        let score = cell.score(trace, byte) + tissue.score_without_pair(trace, byte, disabled_pair);
        if score > best_score {
            second_score = best_score;
            best_score = score;
            best_byte = byte;
        } else if score > second_score {
            second_score = score;
        }
    }

    let margin = best_score - second_score;
    LiveBytePrediction {
        predicted_byte: best_byte,
        confidence: (1.0 / (1.0 + (-margin).exp())).clamp(0.0, 1.0),
        score: best_score,
    }
}

fn top_pair_energy(tissue: &LinkTissue) -> ((usize, usize), &'static str, f32) {
    let mut best_pair = (0, 1);
    let mut best_label = "0-1";
    let mut best_energy = f32::NEG_INFINITY;

    for (pair, label) in PAIR_LABELS {
        let energy = tissue.pair_state_abs_mean(pair);
        if energy > best_energy {
            best_energy = energy;
            best_pair = pair;
            best_label = label;
        }
    }

    (best_pair, best_label, best_energy.max(0.0))
}

fn pair_energy_gini(tissue: &LinkTissue) -> f32 {
    let mut energies = [0.0f32; 15];
    for (index, (pair, _)) in PAIR_LABELS.iter().copied().enumerate() {
        energies[index] = tissue.pair_state_abs_mean(pair).max(0.0);
    }
    gini(&mut energies)
}

fn gini(values: &mut [f32]) -> f32 {
    values.sort_by(|left, right| left.total_cmp(right));
    let sum: f32 = values.iter().sum();
    if sum <= f32::EPSILON {
        return 0.0;
    }

    let mut weighted_sum = 0.0f32;
    for (index, value) in values.iter().enumerate() {
        weighted_sum += (index + 1) as f32 * *value;
    }

    (2.0 * weighted_sum) / (values.len() as f32 * sum)
        - (values.len() as f32 + 1.0) / values.len() as f32
}

fn best_pair_ablation(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    cell: &Cell32Learner,
    tissue: &LinkTissue,
    bytes: &[u8],
    full_accuracy: f32,
) -> (&'static str, f32) {
    let mut best_label = "none";
    let mut best_drop = f32::NEG_INFINITY;

    for (pair, label) in PAIR_LABELS {
        if tissue.pair_state_abs_mean(pair) == 0.0 {
            continue;
        }
        let ablated =
            eval_cell32_with_tissue_without_pair(seed, organ, lut, cell, tissue, pair, bytes);
        let drop = full_accuracy - ablated;
        if drop > best_drop {
            best_drop = drop;
            best_label = label;
        }
    }

    (best_label, best_drop.max(0.0))
}

fn best_triple_energy_label(tissue: &LinkTissue) -> &'static str {
    let triples = [
        ((0, 1, 2), "0-1-2"),
        ((0, 1, 3), "0-1-3"),
        ((0, 1, 4), "0-1-4"),
        ((0, 1, 5), "0-1-5"),
        ((0, 2, 3), "0-2-3"),
        ((0, 2, 4), "0-2-4"),
        ((0, 2, 5), "0-2-5"),
        ((0, 3, 4), "0-3-4"),
        ((0, 3, 5), "0-3-5"),
        ((0, 4, 5), "0-4-5"),
        ((1, 2, 3), "1-2-3"),
        ((1, 2, 4), "1-2-4"),
        ((1, 2, 5), "1-2-5"),
        ((1, 3, 4), "1-3-4"),
        ((1, 3, 5), "1-3-5"),
        ((1, 4, 5), "1-4-5"),
        ((2, 3, 4), "2-3-4"),
        ((2, 3, 5), "2-3-5"),
        ((2, 4, 5), "2-4-5"),
        ((3, 4, 5), "3-4-5"),
    ];
    let mut best_label = "none";
    let mut best_energy = 0.0f32;

    for (triple, label) in triples {
        let energy = tissue.triple_state_abs_mean(triple);
        if energy > best_energy {
            best_energy = energy;
            best_label = label;
        }
    }

    best_label
}

fn train_live_adapter(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &mut LiveByteLearner,
    bytes: &[u8],
) -> LiveAdapterPassReport {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut steps = Vec::with_capacity(bytes.len().saturating_sub(1));
    let mut markov_seen = [false; 256];
    let mut markov_next = [0u8; 256];
    let mut markov_correct = 0usize;

    for pair in bytes.windows(2) {
        let input = pair[0];
        let target = pair[1];
        if markov_seen[input as usize] && markov_next[input as usize] == target {
            markov_correct += 1;
        }
        markov_seen[input as usize] = true;
        markov_next[input as usize] = target;

        let cycle = state.live_cycle(organ, lut, input, target);
        steps.push(learner.update(&cycle.tick.trace, target));
    }

    let report = LiveByteTrainReport::from_steps(&steps, learner);
    LiveAdapterPassReport {
        cases: report.cases,
        learner_accuracy: report.accuracy_before_update,
        last_next_baseline_accuracy: markov_correct as f32 / report.cases.max(1) as f32,
        context_weight_abs_mean: report.context_weight_abs_mean,
    }
}

impl MonoByteLearner {
    fn new(rows_enabled: usize, learning_rate: f32) -> Self {
        let rows_enabled = rows_enabled.max(1);
        Self {
            rows_enabled,
            learning_rate,
            byte_bias: [0.0; 256],
            row_byte_weights: vec![[0.0; 256]; rows_enabled],
            input_row_byte_weights: vec![[0.0; 256]; rows_enabled * 256],
        }
    }

    fn predict(&self, trace: &TickTrace) -> u8 {
        let mut best_byte = 0u8;
        let mut best_score = f32::NEG_INFINITY;

        for byte in 0..=u8::MAX {
            let score = self.score_byte(trace, byte);
            if score > best_score {
                best_score = score;
                best_byte = byte;
            }
        }

        best_byte
    }

    fn update(&mut self, trace: &TickTrace, target_byte: u8) {
        let predicted = self.predict(trace);
        if predicted == target_byte {
            return;
        }

        self.byte_bias[target_byte as usize] += self.learning_rate * 0.25;
        self.byte_bias[predicted as usize] -= self.learning_rate * 0.25;

        for rank in 0..trace.active_count.min(self.rows_enabled) {
            let gain = self.learning_rate * (self.rows_enabled - rank) as f32
                / self.rows_enabled as f32
                * trace.coherence.max(0.05);
            self.row_byte_weights[rank][target_byte as usize] += gain;
            self.row_byte_weights[rank][predicted as usize] -= gain;
            let context_row = rank * 256 + trace.input_byte as usize;
            self.input_row_byte_weights[context_row][target_byte as usize] += gain * 1.40;
            self.input_row_byte_weights[context_row][predicted as usize] -= gain * 1.40;
        }
    }

    fn score_byte(&self, trace: &TickTrace, byte: u8) -> f32 {
        let byte_index = byte as usize;
        let mut score = self.byte_bias[byte_index];

        for rank in 0..trace.active_count.min(self.rows_enabled) {
            let gain = (self.rows_enabled - rank) as f32 / self.rows_enabled as f32;
            score += self.row_byte_weights[rank][byte_index] * gain;
            score += self.input_row_byte_weights[rank * 256 + trace.input_byte as usize]
                [byte_index]
                * gain;
        }

        score
    }
}

fn train_mono_candidate(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &mut MonoByteLearner,
    bytes: &[u8],
) {
    let mut state = OrganState::new(seed, bytes[0]);
    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        learner.update(&cycle.tick.trace, pair[1]);
    }
}

fn eval_mono_candidate(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &MonoByteLearner,
    bytes: &[u8],
) -> f32 {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let cycle = state.live_cycle(organ, lut, pair[0], pair[1]);
        if learner.predict(&cycle.tick.trace) == pair[1] {
            correct += 1;
        }
        cases += 1;
    }

    correct as f32 / cases.max(1) as f32
}

fn eval_live_adapter(
    seed: u64,
    organ: &Stage2Organ,
    lut: &BytePhaseLut,
    learner: &LiveByteLearner,
    bytes: &[u8],
) -> LiveAdapterPassReport {
    let mut state = OrganState::new(seed, bytes[0]);
    let mut eval_learner = learner.clone();
    let mut learner_correct = 0usize;
    let mut markov_seen = [false; 256];
    let mut markov_next = [0u8; 256];
    let mut markov_correct = 0usize;
    let mut cases = 0usize;

    for pair in bytes.windows(2) {
        let input = pair[0];
        let target = pair[1];
        if markov_seen[input as usize] && markov_next[input as usize] == target {
            markov_correct += 1;
        }
        markov_seen[input as usize] = true;
        markov_next[input as usize] = target;

        let cycle = state.live_cycle(organ, lut, input, target);
        if eval_learner
            .predict_observed(&cycle.tick.trace)
            .predicted_byte
            == target
        {
            learner_correct += 1;
        }
        cases += 1;
    }

    let (_, _, _, _, _, context_weight_abs_mean) = learner.state_energy();
    LiveAdapterPassReport {
        cases,
        learner_accuracy: learner_correct as f32 / cases.max(1) as f32,
        last_next_baseline_accuracy: markov_correct as f32 / cases.max(1) as f32,
        context_weight_abs_mean,
    }
}
