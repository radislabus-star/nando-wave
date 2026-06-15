use crate::{
    Chat0Result, corrupted_carrier_wave, format_chat0_result,
    math::{circular_delta, prompt_hash, splitmix64},
    no_carrier_wave, wrong_carrier_wave,
};
use nando_core::{OrganState, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, run_stage2_tick};

/// Report for the first multi-tick word-settling probe.
#[derive(Debug, Clone, PartialEq)]
pub struct SettleWordEvalReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub random: Chat0Result,
    pub mono192: Chat0Result,
    pub voting: Chat0Result,
    pub organ_one_tick: Chat0Result,
    pub organ_settle3: Chat0Result,
    pub organ_settle5: Chat0Result,
    pub organ_settle8: Chat0Result,
    pub organ_stable: Chat0Result,
    pub organ_gated: Chat0Result,
    pub no_carrier_settle5: Chat0Result,
    pub wrong_carrier_settle5: Chat0Result,
    pub corrupted_carrier_settle5: Chat0Result,
    pub ablations: [Chat0Result; STAGE2_ORGAN_CELLS],
    pub settle5_mean_coherence_gain: f32,
    pub settle5_mean_entropy_drop: f32,
    pub settle5_mean_phase_velocity: f32,
    pub settle5_over_best_control: f32,
    pub settle5_over_voting: f32,
    pub settle5_ablation_max_drop: f32,
    pub stable_over_best_control: f32,
    pub stable_ablation_max_drop: f32,
    pub gated_mean_selected_ticks: f32,
    pub gated_over_best_control: f32,
    pub gated_ablation_max_drop: f32,
    pub carrier_integrity_gap: f32,
    pub carrier_guard_rejections: usize,
    pub mode_status: &'static str,
}

/// Per-seed-pair row for settle-word gated robustness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettleWordSeedSweepRow {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub gated_accuracy: f32,
    pub mono192_accuracy: f32,
    pub best_bad_carrier_accuracy: f32,
    pub gated_over_best_control: f32,
    pub gated_ablation_max_drop: f32,
    pub carrier_integrity_gap: f32,
    pub passed: bool,
}

/// Seed sweep for the first gated word-settle candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct SettleWordSeedSweepReport {
    pub cases_per_split: usize,
    pub rows: [SettleWordSeedSweepRow; 4],
    pub passed_seed_pairs: usize,
    pub min_gated_over_best_control: f32,
    pub min_gated_ablation_max_drop: f32,
    pub min_carrier_integrity_gap: f32,
    pub mode_status: &'static str,
}

impl SettleWordEvalReport {
    /// Render a stable report for the first word-settling probe.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave settle-word eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format_chat0_result(self.random));
        output.push_str(&format_chat0_result(self.mono192));
        output.push_str(&format_chat0_result(self.voting));
        output.push_str(&format_chat0_result(self.organ_one_tick));
        output.push_str(&format_chat0_result(self.organ_settle3));
        output.push_str(&format_chat0_result(self.organ_settle5));
        output.push_str(&format_chat0_result(self.organ_settle8));
        output.push_str(&format_chat0_result(self.organ_stable));
        output.push_str(&format_chat0_result(self.organ_gated));
        output.push_str(&format_chat0_result(self.no_carrier_settle5));
        output.push_str(&format_chat0_result(self.wrong_carrier_settle5));
        output.push_str(&format_chat0_result(self.corrupted_carrier_settle5));
        for ablation in self.ablations {
            output.push_str(&format_chat0_result(ablation));
        }
        output.push_str(&format!(
            "settle5_mean_coherence_gain: {:.6}\n",
            self.settle5_mean_coherence_gain
        ));
        output.push_str(&format!(
            "settle5_mean_entropy_drop: {:.6}\n",
            self.settle5_mean_entropy_drop
        ));
        output.push_str(&format!(
            "settle5_mean_phase_velocity: {:.6}\n",
            self.settle5_mean_phase_velocity
        ));
        output.push_str(&format!(
            "settle5_over_best_control: {:.6}\n",
            self.settle5_over_best_control
        ));
        output.push_str(&format!(
            "settle5_over_voting: {:.6}\n",
            self.settle5_over_voting
        ));
        output.push_str(&format!(
            "settle5_ablation_max_drop: {:.6}\n",
            self.settle5_ablation_max_drop
        ));
        output.push_str(&format!(
            "stable_over_best_control: {:.6}\n",
            self.stable_over_best_control
        ));
        output.push_str(&format!(
            "stable_ablation_max_drop: {:.6}\n",
            self.stable_ablation_max_drop
        ));
        output.push_str(&format!(
            "gated_mean_selected_ticks: {:.6}\n",
            self.gated_mean_selected_ticks
        ));
        output.push_str(&format!(
            "gated_over_best_control: {:.6}\n",
            self.gated_over_best_control
        ));
        output.push_str(&format!(
            "gated_ablation_max_drop: {:.6}\n",
            self.gated_ablation_max_drop
        ));
        output.push_str(&format!(
            "carrier_integrity_gap: {:.6}\n",
            self.carrier_integrity_gap
        ));
        output.push_str(&format!(
            "carrier_guard_rejections: {}\n",
            self.carrier_guard_rejections
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SettleWordSeedSweepReport {
    /// Render a stable report for the settle-word gated seed sweep.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave settle-word-seed-sweep eval\n");
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        for (index, row) in self.rows.iter().enumerate() {
            output.push_str(&format!(
                concat!(
                    "seed_pair_{}.train_seed: {}\n",
                    "seed_pair_{}.holdout_seed: {}\n",
                    "seed_pair_{}.gated_accuracy: {:.6}\n",
                    "seed_pair_{}.mono192_accuracy: {:.6}\n",
                    "seed_pair_{}.best_bad_carrier_accuracy: {:.6}\n",
                    "seed_pair_{}.gated_over_best_control: {:.6}\n",
                    "seed_pair_{}.gated_ablation_max_drop: {:.6}\n",
                    "seed_pair_{}.carrier_integrity_gap: {:.6}\n",
                    "seed_pair_{}.passed: {}\n"
                ),
                index,
                row.train_seed,
                index,
                row.holdout_seed,
                index,
                row.gated_accuracy,
                index,
                row.mono192_accuracy,
                index,
                row.best_bad_carrier_accuracy,
                index,
                row.gated_over_best_control,
                index,
                row.gated_ablation_max_drop,
                index,
                row.carrier_integrity_gap,
                index,
                row.passed
            ));
        }
        output.push_str(&format!("passed_seed_pairs: {}\n", self.passed_seed_pairs));
        output.push_str(&format!(
            "min_gated_over_best_control: {:.6}\n",
            self.min_gated_over_best_control
        ));
        output.push_str(&format!(
            "min_gated_ablation_max_drop: {:.6}\n",
            self.min_gated_ablation_max_drop
        ));
        output.push_str(&format!(
            "min_carrier_integrity_gap: {:.6}\n",
            self.min_carrier_integrity_gap
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

/// Evaluate whether a short word can be read after a multi-tick settle loop.
///
/// This is the first instrument for "word birth" in the architecture v2 sense.
/// It is intentionally still a probe: the readout is a tiny centroid trained on
/// settle traces, and the report compares it with monolith, voting, one-tick,
/// no-carrier, wrong-carrier, and ablated-cell controls.
#[must_use]
pub fn settle_word_eval(train_seed: u64, holdout_seed: u64, cases: usize) -> SettleWordEvalReport {
    let mut random = Chat0Result::new("settle_word_random", cases);
    let mut mono192 = Chat0Result::new("settle_word_mono192", cases);
    let mut voting = Chat0Result::new("settle_word_voting", cases);
    let mut organ_one_tick = Chat0Result::new("settle_word_organ_one_tick", cases);
    let mut organ_settle3 = Chat0Result::new("settle_word_organ_settle3", cases);
    let mut organ_settle5 = Chat0Result::new("settle_word_organ_settle5", cases);
    let mut organ_settle8 = Chat0Result::new("settle_word_organ_settle8", cases);
    let mut organ_stable = Chat0Result::new("settle_word_organ_stable", cases);
    let mut organ_gated = Chat0Result::new("settle_word_organ_gated", cases);
    let mut no_carrier_settle5 = Chat0Result::new("settle_word_no_carrier_settle5", cases);
    let mut wrong_carrier_settle5 = Chat0Result::new("settle_word_wrong_carrier_settle5", cases);
    let mut corrupted_carrier_settle5 =
        Chat0Result::new("settle_word_corrupted_carrier_settle5", cases);
    let mut ablations: [Chat0Result; STAGE2_ORGAN_CELLS] =
        std::array::from_fn(|cell_id| Chat0Result::new(settle_word_ablation_name(cell_id), cases));

    let mut mono_classifier = SettleWordClassifier::new();
    let mut voting_classifier = SettleWordClassifier::new();
    let mut one_tick_classifier = SettleWordClassifier::new();
    let mut settle3_classifier = SettleWordClassifier::new();
    let mut settle5_classifier = SettleWordClassifier::new();
    let mut settle8_classifier = SettleWordClassifier::new();
    let mut stable_classifier = SettleWordClassifier::new();
    let mut gated_classifiers: [SettleWordClassifier; 8] =
        std::array::from_fn(|_| SettleWordClassifier::new());

    for case_index in 0..cases {
        let sample = settle_word_sample(train_seed, case_index, false);
        mono_classifier.observe(
            sample.label,
            settle_word_mono_features(train_seed, &sample.prompt),
        );
        voting_classifier.observe(
            sample.label,
            settle_word_voting_features(train_seed, &sample.prompt),
        );
        one_tick_classifier.observe(
            sample.label,
            settle_word_trace_features(train_seed, &sample.prompt, 1, CarrierMode::Correct, None)
                .features,
        );
        settle3_classifier.observe(
            sample.label,
            settle_word_trace_features(train_seed, &sample.prompt, 3, CarrierMode::Correct, None)
                .features,
        );
        settle5_classifier.observe(
            sample.label,
            settle_word_trace_features(train_seed, &sample.prompt, 5, CarrierMode::Correct, None)
                .features,
        );
        settle8_classifier.observe(
            sample.label,
            settle_word_trace_features(train_seed, &sample.prompt, 8, CarrierMode::Correct, None)
                .features,
        );
        stable_classifier.observe(
            sample.label,
            settle_word_stable_trace_features(
                train_seed,
                &sample.prompt,
                CarrierMode::Correct,
                None,
            )
            .features,
        );
        for (tick_index, classifier) in gated_classifiers.iter_mut().enumerate() {
            classifier.observe(
                sample.label,
                settle_word_trace_features(
                    train_seed,
                    &sample.prompt,
                    tick_index + 1,
                    CarrierMode::Correct,
                    None,
                )
                .features,
            );
        }
    }

    mono_classifier.finish();
    voting_classifier.finish();
    one_tick_classifier.finish();
    settle3_classifier.finish();
    settle5_classifier.finish();
    settle8_classifier.finish();
    stable_classifier.finish();
    for classifier in &mut gated_classifiers {
        classifier.finish();
    }
    let gated_ticks = settle_word_select_gate_ticks(train_seed, cases, &gated_classifiers);

    let mut settle5_mean_coherence_gain = 0.0;
    let mut settle5_mean_entropy_drop = 0.0;
    let mut settle5_mean_phase_velocity = 0.0;
    let mut gated_mean_selected_ticks = 0.0;
    let mut carrier_guard_rejections = 0usize;

    for case_index in 0..cases {
        let sample = settle_word_sample(holdout_seed, case_index, true);
        let expected = sample.expected;

        let random_label = splitmix64(holdout_seed ^ case_index as u64 ^ 0x005E_771E) as usize
            % SETTLE_WORD_TASKS.len();
        random.score(settle_word_by_label(random_label), expected);

        let mono_label =
            mono_classifier.predict(settle_word_mono_features(holdout_seed, &sample.prompt));
        mono192.score(settle_word_by_label(mono_label), expected);

        let voting_label =
            voting_classifier.predict(settle_word_voting_features(holdout_seed, &sample.prompt));
        voting.score(settle_word_by_label(voting_label), expected);

        let one_tick =
            settle_word_trace_features(holdout_seed, &sample.prompt, 1, CarrierMode::Correct, None);
        let one_tick_label = one_tick_classifier.predict(one_tick.features);
        organ_one_tick.score(settle_word_by_label(one_tick_label), expected);

        let settle3 =
            settle_word_trace_features(holdout_seed, &sample.prompt, 3, CarrierMode::Correct, None);
        let settle3_label = settle3_classifier.predict(settle3.features);
        organ_settle3.score(settle_word_by_label(settle3_label), expected);

        let settle5 =
            settle_word_trace_features(holdout_seed, &sample.prompt, 5, CarrierMode::Correct, None);
        let settle5_label = settle5_classifier.predict(settle5.features);
        organ_settle5.score(settle_word_by_label(settle5_label), expected);
        settle5_mean_coherence_gain += settle5.coherence_gain;
        settle5_mean_entropy_drop += settle5.entropy_drop;
        settle5_mean_phase_velocity += settle5.phase_velocity;

        let settle8 =
            settle_word_trace_features(holdout_seed, &sample.prompt, 8, CarrierMode::Correct, None);
        let settle8_label = settle8_classifier.predict(settle8.features);
        organ_settle8.score(settle_word_by_label(settle8_label), expected);

        let stable = settle_word_stable_trace_features(
            holdout_seed,
            &sample.prompt,
            CarrierMode::Correct,
            None,
        );
        let stable_label = stable_classifier.predict(stable.features);
        organ_stable.score(settle_word_by_label(stable_label), expected);

        let gated = settle_word_trace_features(
            holdout_seed,
            &sample.prompt,
            gated_ticks,
            CarrierMode::Correct,
            None,
        );
        let gated_label = gated_classifiers[gated_ticks - 1].predict(gated.features);
        organ_gated.score(settle_word_by_label(gated_label), expected);
        gated_mean_selected_ticks += gated_ticks as f32;

        let no_carrier =
            settle_word_trace_features(holdout_seed, &sample.prompt, 5, CarrierMode::None, None);
        if settle_word_carrier_guard_allows(no_carrier) {
            let no_carrier_label = settle5_classifier.predict(no_carrier.features);
            no_carrier_settle5.score(settle_word_by_label(no_carrier_label), expected);
        } else {
            carrier_guard_rejections += 1;
            no_carrier_settle5.score("__carrier_blocked__", expected);
        }

        let wrong_carrier =
            settle_word_trace_features(holdout_seed, &sample.prompt, 5, CarrierMode::Wrong, None);
        if settle_word_carrier_guard_allows(wrong_carrier) {
            let wrong_carrier_label = settle5_classifier.predict(wrong_carrier.features);
            wrong_carrier_settle5.score(settle_word_by_label(wrong_carrier_label), expected);
        } else {
            carrier_guard_rejections += 1;
            wrong_carrier_settle5.score("__carrier_blocked__", expected);
        }

        let corrupted_carrier = settle_word_trace_features(
            holdout_seed,
            &sample.prompt,
            5,
            CarrierMode::Corrupted,
            None,
        );
        if settle_word_carrier_guard_allows(corrupted_carrier) {
            let corrupted_carrier_label = settle5_classifier.predict(corrupted_carrier.features);
            corrupted_carrier_settle5
                .score(settle_word_by_label(corrupted_carrier_label), expected);
        } else {
            carrier_guard_rejections += 1;
            corrupted_carrier_settle5.score("__carrier_blocked__", expected);
        }

        for (cell_id, ablation) in ablations.iter_mut().enumerate() {
            let ablated = settle_word_trace_features(
                holdout_seed,
                &sample.prompt,
                5,
                CarrierMode::Correct,
                Some(cell_id as u32),
            );
            let label = settle5_classifier.predict(ablated.features);
            ablation.score(settle_word_by_label(label), expected);
        }
    }

    random.finish();
    mono192.finish();
    voting.finish();
    organ_one_tick.finish();
    organ_settle3.finish();
    organ_settle5.finish();
    organ_settle8.finish();
    organ_stable.finish();
    organ_gated.finish();
    no_carrier_settle5.finish();
    wrong_carrier_settle5.finish();
    corrupted_carrier_settle5.finish();
    for ablation in &mut ablations {
        ablation.finish();
    }

    let cases_f32 = cases as f32;
    if cases_f32 > 0.0 {
        settle5_mean_coherence_gain /= cases_f32;
        settle5_mean_entropy_drop /= cases_f32;
        settle5_mean_phase_velocity /= cases_f32;
        gated_mean_selected_ticks /= cases_f32;
    }

    let best_control_accuracy = [
        random.exact_accuracy,
        mono192.exact_accuracy,
        voting.exact_accuracy,
        organ_one_tick.exact_accuracy,
        no_carrier_settle5.exact_accuracy,
        wrong_carrier_settle5.exact_accuracy,
        corrupted_carrier_settle5.exact_accuracy,
    ]
    .into_iter()
    .fold(f32::NEG_INFINITY, f32::max);
    let settle5_over_best_control = organ_settle5.exact_accuracy - best_control_accuracy;
    let settle5_over_voting = organ_settle5.exact_accuracy - voting.exact_accuracy;
    let settle5_ablation_max_drop = ablations
        .iter()
        .map(|ablation| organ_settle5.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let stable_ablation_max_drop = (0..STAGE2_ORGAN_CELLS)
        .map(|cell_id| {
            let mut ablation = Chat0Result::new("settle_word_stable_ablation_tmp", cases);
            for case_index in 0..cases {
                let sample = settle_word_sample(holdout_seed, case_index, true);
                let ablated = settle_word_stable_trace_features(
                    holdout_seed,
                    &sample.prompt,
                    CarrierMode::Correct,
                    Some(cell_id as u32),
                );
                let label = stable_classifier.predict(ablated.features);
                ablation.score(settle_word_by_label(label), sample.expected);
            }
            ablation.finish();
            organ_stable.exact_accuracy - ablation.exact_accuracy
        })
        .fold(f32::NEG_INFINITY, f32::max);
    let stable_over_best_control = organ_stable.exact_accuracy - best_control_accuracy;
    let gated_ablation_max_drop = (0..STAGE2_ORGAN_CELLS)
        .map(|cell_id| {
            let mut ablation = Chat0Result::new("settle_word_gated_ablation_tmp", cases);
            for case_index in 0..cases {
                let sample = settle_word_sample(holdout_seed, case_index, true);
                let ablated = settle_word_trace_features(
                    holdout_seed,
                    &sample.prompt,
                    gated_ticks,
                    CarrierMode::Correct,
                    Some(cell_id as u32),
                );
                let label = gated_classifiers[gated_ticks - 1].predict(ablated.features);
                ablation.score(settle_word_by_label(label), sample.expected);
            }
            ablation.finish();
            organ_gated.exact_accuracy - ablation.exact_accuracy
        })
        .fold(f32::NEG_INFINITY, f32::max);
    let gated_over_best_control = organ_gated.exact_accuracy - best_control_accuracy;
    let best_correct_settle_accuracy = organ_settle3
        .exact_accuracy
        .max(organ_settle5.exact_accuracy)
        .max(organ_settle8.exact_accuracy)
        .max(organ_stable.exact_accuracy)
        .max(organ_gated.exact_accuracy);
    let carrier_integrity_gap = best_correct_settle_accuracy
        - no_carrier_settle5
            .exact_accuracy
            .max(wrong_carrier_settle5.exact_accuracy)
            .max(corrupted_carrier_settle5.exact_accuracy);

    let mode_status = if gated_over_best_control > 0.0
        && gated_ablation_max_drop > 0.0
        && organ_gated.exact_accuracy >= organ_settle3.exact_accuracy
        && carrier_integrity_gap > 0.0
    {
        "settle_word_gated_candidate_needs_seed_sweep"
    } else if stable_over_best_control > 0.0
        && stable_ablation_max_drop > 0.0
        && organ_stable.exact_accuracy >= organ_settle3.exact_accuracy
        && settle5_mean_entropy_drop >= 0.0
        && carrier_integrity_gap > 0.0
    {
        "settle_word_stable_candidate_needs_seed_sweep"
    } else if settle5_over_best_control > 0.0
        && settle5_over_voting > 0.0
        && settle5_ablation_max_drop > 0.0
        && organ_settle5.exact_accuracy >= organ_settle3.exact_accuracy
        && settle5_mean_entropy_drop >= 0.0
        && carrier_integrity_gap > 0.0
    {
        "settle_word_candidate_needs_seed_sweep"
    } else {
        "not_found_settle_word"
    };

    SettleWordEvalReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        random,
        mono192,
        voting,
        organ_one_tick,
        organ_settle3,
        organ_settle5,
        organ_settle8,
        organ_stable,
        organ_gated,
        no_carrier_settle5,
        wrong_carrier_settle5,
        corrupted_carrier_settle5,
        ablations,
        settle5_mean_coherence_gain,
        settle5_mean_entropy_drop,
        settle5_mean_phase_velocity,
        settle5_over_best_control,
        settle5_over_voting,
        settle5_ablation_max_drop,
        stable_over_best_control,
        stable_ablation_max_drop,
        gated_mean_selected_ticks,
        gated_over_best_control,
        gated_ablation_max_drop,
        carrier_integrity_gap,
        carrier_guard_rejections,
        mode_status,
    }
}

/// Sweep the gated word-settle candidate across fixed seed pairs.
#[must_use]
pub fn settle_word_seed_sweep_eval(cases: usize) -> SettleWordSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = settle_word_eval(train_seed, holdout_seed, cases);
        let best_bad_carrier_accuracy = report
            .no_carrier_settle5
            .exact_accuracy
            .max(report.wrong_carrier_settle5.exact_accuracy)
            .max(report.corrupted_carrier_settle5.exact_accuracy);
        let passed = report.mode_status == "settle_word_gated_candidate_needs_seed_sweep";

        SettleWordSeedSweepRow {
            train_seed,
            holdout_seed,
            gated_accuracy: report.organ_gated.exact_accuracy,
            mono192_accuracy: report.mono192.exact_accuracy,
            best_bad_carrier_accuracy,
            gated_over_best_control: report.gated_over_best_control,
            gated_ablation_max_drop: report.gated_ablation_max_drop,
            carrier_integrity_gap: report.carrier_integrity_gap,
            passed,
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_gated_over_best_control = rows
        .iter()
        .map(|row| row.gated_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_gated_ablation_max_drop = rows
        .iter()
        .map(|row| row.gated_ablation_max_drop)
        .fold(f32::INFINITY, f32::min);
    let min_carrier_integrity_gap = rows
        .iter()
        .map(|row| row.carrier_integrity_gap)
        .fold(f32::INFINITY, f32::min);

    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_gated_over_best_control > 0.0
        && min_gated_ablation_max_drop > 0.0
        && min_carrier_integrity_gap > 0.0
    {
        "settle_word_gated_seed_sweep_passed"
    } else if passed_seed_pairs >= 3
        && min_gated_over_best_control >= 0.0
        && min_gated_ablation_max_drop > 0.0
        && min_carrier_integrity_gap > 0.0
    {
        "settle_word_gated_seed_sweep_partial_3_of_4"
    } else {
        "not_found_settle_word_gated_seed_sweep"
    };

    SettleWordSeedSweepReport {
        cases_per_split: cases,
        rows,
        passed_seed_pairs,
        min_gated_over_best_control,
        min_gated_ablation_max_drop,
        min_carrier_integrity_gap,
        mode_status,
    }
}

const SETTLE_WORD_FEATURES: usize = 32;
const SETTLE_WORD_TASKS: [(&str, &str, &str, &str); 8] = [
    (
        "let",
        "rust immutable binding keyword after equals cue",
        "code wants a fixed local binding keyword",
        "binding stays fixed in rust",
    ),
    (
        "mut",
        "rust changeable binding marker after local cue",
        "code wants a changeable local marker",
        "binding may change in rust",
    ),
    (
        "fn",
        "rust callable definition marker after item cue",
        "code wants a callable item marker",
        "callable item starts in rust",
    ),
    (
        "struct",
        "rust record shape item marker after type cue",
        "code wants a named record shape marker",
        "record shaped type starts in rust",
    ),
    (
        "enum",
        "rust variant set item marker after type cue",
        "code wants a variant choice type marker",
        "variant family type starts in rust",
    ),
    (
        "impl",
        "rust attach methods block marker after type cue",
        "code wants a method attachment block marker",
        "methods attach to type in rust",
    ),
    (
        "use",
        "rust bring path into scope marker after import cue",
        "code wants a scope import marker",
        "path enters scope in rust",
    ),
    (
        "pub",
        "rust visible item marker after exposure cue",
        "code wants an exposed item marker",
        "item becomes visible in rust",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct SettleWordSample {
    label: usize,
    prompt: Vec<u8>,
    expected: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarrierMode {
    Correct,
    None,
    Wrong,
    Corrupted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SettleWordTraceFeatures {
    features: [f32; SETTLE_WORD_FEATURES],
    coherence_gain: f32,
    entropy_drop: f32,
    phase_velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SettleWordClassifier {
    centroids: [[f32; SETTLE_WORD_FEATURES]; 8],
    counts: [usize; 8],
}

impl SettleWordClassifier {
    fn new() -> Self {
        Self {
            centroids: [[0.0; SETTLE_WORD_FEATURES]; 8],
            counts: [0; 8],
        }
    }

    fn observe(&mut self, label: usize, features: [f32; SETTLE_WORD_FEATURES]) {
        self.counts[label] += 1;
        for (slot, feature) in self.centroids[label].iter_mut().zip(features.iter()) {
            *slot += feature;
        }
    }

    fn finish(&mut self) {
        for (centroid, count) in self.centroids.iter_mut().zip(self.counts.iter()) {
            if *count > 0 {
                let scale = *count as f32;
                for slot in centroid {
                    *slot /= scale;
                }
            }
        }
    }

    fn predict(self, features: [f32; SETTLE_WORD_FEATURES]) -> usize {
        let mut best_label = 0;
        let mut best_distance = f32::INFINITY;

        for (label, centroid) in self.centroids.iter().enumerate() {
            if self.counts[label] == 0 {
                continue;
            }
            let distance = centroid
                .iter()
                .zip(features.iter())
                .map(|(left, right)| {
                    let delta = left - right;
                    delta * delta
                })
                .sum::<f32>();
            if distance < best_distance {
                best_label = label;
                best_distance = distance;
            }
        }

        best_label
    }
}

fn settle_word_sample(seed: u64, case_index: usize, holdout: bool) -> SettleWordSample {
    let label = (case_index + seed as usize) % SETTLE_WORD_TASKS.len();
    let (word, train_cue, holdout_cue, shared_cue) = SETTLE_WORD_TASKS[label];
    let cue = if holdout { holdout_cue } else { train_cue };
    let jitter = splitmix64(seed ^ (case_index as u64).rotate_left(17)).to_le_bytes()[0] & 7;
    let prompt = format!("{cue} | {shared_cue} | trace:{jitter} -> ");

    SettleWordSample {
        label,
        prompt: prompt.into_bytes(),
        expected: word,
    }
}

fn settle_word_by_label(label: usize) -> &'static str {
    SETTLE_WORD_TASKS[label % SETTLE_WORD_TASKS.len()].0
}

fn settle_word_ablation_name(cell_id: usize) -> &'static str {
    match cell_id {
        0 => "settle_word_ablate_cell0",
        1 => "settle_word_ablate_cell1",
        2 => "settle_word_ablate_cell2",
        3 => "settle_word_ablate_cell3",
        4 => "settle_word_ablate_cell4",
        5 => "settle_word_ablate_cell5",
        _ => "settle_word_ablate_cell_unknown",
    }
}

fn settle_word_mono_features(seed: u64, prompt: &[u8]) -> [f32; SETTLE_WORD_FEATURES] {
    let hash = prompt_hash(seed, prompt);
    let first = prompt.first().copied().unwrap_or(0);
    let last = prompt.last().copied().unwrap_or(0);
    let mut features = [0.0; SETTLE_WORD_FEATURES];
    features[0] = 1.0;
    features[1] = prompt.len() as f32 / 128.0;
    features[2] = f32::from(first) / 255.0;
    features[3] = f32::from(last) / 255.0;
    features[4] = hash.sin();
    features[5] = hash.cos();
    features[6] = (hash * 2.0).sin();
    features[7] = (hash * 2.0).cos();
    for (index, byte) in prompt.iter().copied().take(8).enumerate() {
        features[8 + index] = f32::from(byte) / 255.0;
    }
    features
}

fn settle_word_voting_features(seed: u64, prompt: &[u8]) -> [f32; SETTLE_WORD_FEATURES] {
    let input = prompt.last().copied().unwrap_or(0);
    let tick = run_stage2_tick(seed, input);
    let mut features = [0.0; SETTLE_WORD_FEATURES];
    features[0] = 1.0;
    for (rank, cell_id) in tick.trace.active_cell_ids.iter().copied().enumerate() {
        let slot = 1 + (cell_id as usize % STAGE2_ORGAN_CELLS);
        features[slot] += (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
    }
    features[7] = tick.trace.coherence;
    features[8] = tick.trace.spectral_entropy;
    features[9] = tick.trace.center_phase.sin();
    features[10] = tick.trace.center_phase.cos();
    features[11] = tick.trace.center_magnitude;
    let hash = prompt_hash(seed ^ 0x901E_501D, prompt);
    features[12] = hash.sin();
    features[13] = hash.cos();
    features[14] = prompt.len() as f32 / 128.0;
    features[15] = f32::from(input) / 255.0;
    features
}

fn settle_word_trace_features(
    seed: u64,
    prompt: &[u8],
    settle_ticks: usize,
    carrier_mode: CarrierMode,
    disabled_cell_id: Option<u32>,
) -> SettleWordTraceFeatures {
    let first = prompt.first().copied().unwrap_or(0);
    let mut organ_state = OrganState::new(seed, first);
    let mut features = [0.0; SETTLE_WORD_FEATURES];
    let mut first_coherence = 0.0;
    let mut final_coherence = 0.0;
    let mut first_entropy = 1.0;
    let mut final_entropy = 1.0;
    let mut previous_phase = None;
    let mut phase_velocity = 0.0;
    let mut final_phase = 0.0;
    let mut final_carrier_phase = organ_state.carrier.phase;
    let mut carrier_phase_error = 0.0;
    let mut carrier_amplitude_error = 0.0;
    let mut carrier_frequency_error = 0.0;
    let mut carrier_boundary_error = 0.0;
    let ticks = settle_ticks.max(1);

    features[0] = 1.0;
    for tick_index in 0..ticks {
        let byte = prompt
            .get((tick_index * 7 + prompt.len() / 3) % prompt.len().max(1))
            .copied()
            .unwrap_or(0);
        let locked_carrier = organ_state.next_locked_carrier(byte);
        let tick_carrier = match carrier_mode {
            CarrierMode::Correct => locked_carrier,
            CarrierMode::None => no_carrier_wave(),
            CarrierMode::Wrong => wrong_carrier_wave(seed + tick_index as u64, byte),
            CarrierMode::Corrupted => corrupted_carrier_wave(locked_carrier),
        };
        carrier_phase_error +=
            circular_delta(locked_carrier.phase, tick_carrier.phase).abs() / std::f32::consts::TAU;
        carrier_amplitude_error += (locked_carrier.amplitude - tick_carrier.amplitude).abs();
        carrier_frequency_error += (locked_carrier.frequency - tick_carrier.frequency).abs();
        carrier_boundary_error += (locked_carrier.boundary - tick_carrier.boundary).abs();
        let tick = organ_state.settle_tick_with_carrier(byte, tick_carrier, disabled_cell_id);

        if tick_index == 0 {
            first_coherence = tick.trace.coherence;
            first_entropy = tick.trace.spectral_entropy;
        }
        final_coherence = tick.trace.coherence;
        final_entropy = tick.trace.spectral_entropy;
        final_phase = tick.trace.center_phase;
        final_carrier_phase = tick_carrier.phase;

        if let Some(previous) = previous_phase {
            phase_velocity += circular_delta(previous, final_phase).abs();
        }
        previous_phase = Some(final_phase);

        for (rank, cell_id) in tick.trace.active_cell_ids.iter().copied().enumerate() {
            let slot = 1 + (cell_id as usize % STAGE2_ORGAN_CELLS);
            features[slot] += (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
        }
    }

    let scale = ticks as f32;
    for slot in &mut features[1..=6] {
        *slot /= scale;
    }
    let velocity_scale = (ticks.saturating_sub(1).max(1)) as f32;
    phase_velocity /= velocity_scale * std::f32::consts::TAU;
    let hash = prompt_hash(seed ^ 0x005E_771E_FEA7, prompt);
    features[7] = final_phase.sin();
    features[8] = final_phase.cos();
    features[9] = final_carrier_phase.sin();
    features[10] = final_carrier_phase.cos();
    features[11] = final_coherence;
    features[12] = final_entropy;
    features[13] = (first_entropy - final_entropy).clamp(-1.0, 1.0);
    features[14] = phase_velocity.clamp(0.0, 1.0);
    features[15] = hash.sin() * 0.5 + hash.cos() * 0.5;
    for (offset, coupling) in organ_state.cell_coupling.iter().enumerate() {
        features[16 + offset] = *coupling;
    }
    features[22] = organ_state.previous_coherence;
    features[23] = organ_state.previous_entropy;
    features[24] = (carrier_phase_error / scale).clamp(0.0, 1.0);
    features[25] = (carrier_amplitude_error / scale).clamp(0.0, 1.0);
    features[26] = (carrier_frequency_error / scale).clamp(0.0, 1.0);
    features[27] = (carrier_boundary_error / scale).clamp(0.0, 1.0);
    features[28] = (1.0 - features[24]).clamp(0.0, 1.0);
    features[29] = (1.0 - features[25]).clamp(0.0, 1.0);
    features[30] = (1.0 - features[26]).clamp(0.0, 1.0);
    features[31] = (1.0 - features[27]).clamp(0.0, 1.0);

    SettleWordTraceFeatures {
        features,
        coherence_gain: final_coherence - first_coherence,
        entropy_drop: first_entropy - final_entropy,
        phase_velocity,
    }
}

fn settle_word_stable_trace_features(
    seed: u64,
    prompt: &[u8],
    carrier_mode: CarrierMode,
    disabled_cell_id: Option<u32>,
) -> SettleWordTraceFeatures {
    let mut best = settle_word_trace_features(seed, prompt, 1, carrier_mode, disabled_cell_id);
    let mut best_score = settle_word_stability_score(best);

    for ticks in 2..=8 {
        let candidate =
            settle_word_trace_features(seed, prompt, ticks, carrier_mode, disabled_cell_id);
        let score = settle_word_stability_score(candidate);
        if score > best_score {
            best = candidate;
            best_score = score;
        }
    }

    best
}

fn settle_word_select_gate_ticks(
    train_seed: u64,
    cases: usize,
    classifiers: &[SettleWordClassifier; 8],
) -> usize {
    let mut validation_correct_by_tick = [0usize; 8];
    let mut validation_seen_by_tick = [0usize; 8];
    let mut stability_by_tick = [0.0f32; 8];

    for (tick_index, classifier) in classifiers.iter().copied().enumerate() {
        let ticks = tick_index + 1;
        let mut correct = 0usize;
        let mut seen = 0usize;
        let mut stability = 0.0;
        for case_index in 0..cases {
            if case_index % 2 == 0 {
                continue;
            }
            let sample = settle_word_sample(train_seed, case_index, false);
            let trace = settle_word_trace_features(
                train_seed,
                &sample.prompt,
                ticks,
                CarrierMode::Correct,
                None,
            );
            let label = classifier.predict(trace.features);
            if label == sample.label {
                correct += 1;
            }
            stability += settle_word_stability_score(trace);
            seen += 1;
        }

        validation_correct_by_tick[tick_index] = correct;
        validation_seen_by_tick[tick_index] = seen;
        stability_by_tick[tick_index] = if seen > 0 {
            stability / seen as f32
        } else {
            f32::NEG_INFINITY
        };
    }

    let mut best_ticks = 1;
    let mut best_score = f32::NEG_INFINITY;
    for tick_index in 0..8 {
        let seen = validation_seen_by_tick[tick_index].max(1) as f32;
        let accuracy = validation_correct_by_tick[tick_index] as f32 / seen;
        let score = accuracy + stability_by_tick[tick_index] * 0.05;
        if score > best_score {
            best_score = score;
            best_ticks = tick_index + 1;
        }
    }

    best_ticks
}

fn settle_word_stability_score(trace: SettleWordTraceFeatures) -> f32 {
    let coherence = trace.features[11];
    let entropy = trace.features[12];
    coherence + trace.entropy_drop * 0.35 - trace.phase_velocity * 0.60 - entropy * 0.05
}

fn settle_word_carrier_guard_allows(trace: SettleWordTraceFeatures) -> bool {
    trace.features[24] <= 0.020
        && trace.features[25] <= 0.050
        && trace.features[26] <= 0.050
        && trace.features[27] <= 0.050
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settle_word_report_has_required_controls() {
        let report = settle_word_eval(13, 97, 16);

        assert_eq!(report.cases_per_split, 16);
        assert_eq!(report.random.cases, 16);
        assert_eq!(report.mono192.cases, 16);
        assert_eq!(report.voting.cases, 16);
        assert_eq!(report.organ_one_tick.cases, 16);
        assert_eq!(report.organ_settle3.cases, 16);
        assert_eq!(report.organ_settle5.cases, 16);
        assert_eq!(report.organ_settle8.cases, 16);
        assert_eq!(report.organ_stable.cases, 16);
        assert_eq!(report.organ_gated.cases, 16);
        assert_eq!(report.no_carrier_settle5.cases, 16);
        assert_eq!(report.wrong_carrier_settle5.cases, 16);
        assert_eq!(report.corrupted_carrier_settle5.cases, 16);
        assert_eq!(report.ablations.len(), STAGE2_ORGAN_CELLS);
        assert!(report.to_text().contains("Nando Wave settle-word eval"));
        assert!(
            report
                .to_text()
                .contains("settle_word_organ_settle5.exact_accuracy")
        );
        assert!(report.to_text().contains("settle5_mean_entropy_drop"));
        assert!(report.to_text().contains("gated_mean_selected_ticks"));
        assert!(report.to_text().contains("carrier_integrity_gap"));
        assert!(
            report.mode_status == "settle_word_candidate_needs_seed_sweep"
                || report.mode_status == "settle_word_stable_candidate_needs_seed_sweep"
                || report.mode_status == "settle_word_gated_candidate_needs_seed_sweep"
                || report.mode_status == "not_found_settle_word"
        );
    }

    #[test]
    fn settle_word_seed_sweep_report_has_required_rows() {
        let report = settle_word_seed_sweep_eval(16);

        assert_eq!(report.cases_per_split, 16);
        assert_eq!(report.rows.len(), 4);
        assert!(report.to_text().contains("settle-word-seed-sweep eval"));
        assert!(report.to_text().contains("seed_pair_0.gated_accuracy"));
        assert!(report.to_text().contains("min_carrier_integrity_gap"));
        assert!(
            report.mode_status == "settle_word_gated_seed_sweep_passed"
                || report.mode_status == "settle_word_gated_seed_sweep_partial_3_of_4"
                || report.mode_status == "not_found_settle_word_gated_seed_sweep"
        );
    }
}
