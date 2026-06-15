use crate::BaselineResult;

use super::BYTE_CONTEXT_TASKS;

mod reports;
pub use reports::{
    ByteContextCellularCarrierAblationReport, ByteContextCentroidAblationReport,
    ByteContextCentroidReport, ByteContextCentroidSeedRow, ByteContextCentroidSeedSweepReport,
    ByteContextPromptCarrierAblationReport, ByteContextReport,
    ByteContextTrainedCarrierAblationReport,
};

use crate::{
    Chat0EvalReport, Chat0FeedbackEntry, Chat0PromoteEvalReport, Chat0PromotedHoldoutEvalReport,
    Chat0PromotedState, Chat0Result, Chat0RouteEvalReport, Chat0Trace, best_baseline,
    best_chat0_control, build_corrupted_carrier_snapshot, build_corrupted_snapshot, byte_to_phase,
    chat0_response_for_target, chat0_target_for_response, chat0_task_for_target, circular_delta,
    random_predict, score_prediction, snapshot_roundtrip, splitmix64, voting_predict,
    wave_bus_predict,
};
use nando_core::{
    CarrierWave, SNAPSHOT_V1_BYTES, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, SpectrumSnapshot,
    run_stage2_tick_with_carrier,
};

/// First byte-level context eval: predict the next answer byte from prompt state.
#[must_use]
pub fn byte_context_eval(train_seed: u64, holdout_seed: u64, cases: usize) -> ByteContextReport {
    const LEARNING_RATE: f32 = 0.18;

    let mut mono_decoder = BytePhaseDecoder::new(LEARNING_RATE);
    let mut no_snapshot_decoder = BytePhaseDecoder::new(LEARNING_RATE);
    let mut snapshot_decoder = BytePhaseDecoder::new(LEARNING_RATE);
    let mut wrong_snapshot_decoder = BytePhaseDecoder::new(LEARNING_RATE);

    for case_index in 0..cases {
        let sample = byte_context_sample(train_seed, case_index, true);
        train_byte_context_sample(
            sample,
            train_seed,
            case_index,
            &mut mono_decoder,
            &mut no_snapshot_decoder,
            &mut snapshot_decoder,
            &mut wrong_snapshot_decoder,
        );
    }

    let mut random = BaselineResult::new("random", cases);
    let mut mono192_prompt_decoder = BaselineResult::new("mono192_prompt_decoder", cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_decoder", cases);
    let mut voting = BaselineResult::new("cell32_voting", cases);
    let mut snapshot = BaselineResult::new("snapshot_decoder", cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_decoder", cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot_decoder", cases);

    for case_index in 0..cases {
        let sample = byte_context_sample(holdout_seed, case_index, false);
        score_byte_context_sample(
            sample,
            holdout_seed,
            case_index,
            &mut mono_decoder,
            &mut no_snapshot_decoder,
            &mut snapshot_decoder,
            &mut wrong_snapshot_decoder,
            &mut random,
            &mut mono192_prompt_decoder,
            &mut no_snapshot,
            &mut voting,
            &mut snapshot,
            &mut wrong_snapshot,
            &mut corrupted_snapshot,
        );
    }

    random.finish();
    mono192_prompt_decoder.finish();
    no_snapshot.finish();
    voting.finish();
    snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let best_control = best_baseline([random, mono192_prompt_decoder, no_snapshot, voting]);
    let snapshot_accuracy_over_best_control = snapshot.accuracy - best_control.accuracy;
    let snapshot_error_gain_over_best_control =
        best_control.mean_circular_error - snapshot.mean_circular_error;
    let snapshot_error_gain_over_wrong_snapshot =
        wrong_snapshot.mean_circular_error - snapshot.mean_circular_error;
    let mode_status = if snapshot_accuracy_over_best_control > 0.02
        && snapshot_error_gain_over_best_control > 0.0
        && snapshot_error_gain_over_wrong_snapshot > 0.0
        && snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "byte_context_candidate_needs_seed_sweep"
    } else {
        "not_found_byte_context"
    };

    ByteContextReport {
        train_seed,
        holdout_seed,
        train_cases: cases,
        holdout_cases: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192_prompt_decoder,
        no_snapshot_decoder: no_snapshot,
        cell32_voting: voting,
        snapshot_decoder: snapshot,
        wrong_snapshot_decoder: wrong_snapshot,
        corrupted_snapshot_decoder: corrupted_snapshot,
        snapshot_accuracy_over_best_control,
        snapshot_error_gain_over_best_control,
        snapshot_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Prototype byte-level context eval: train frozen label centroids, then test holdout.
#[must_use]
pub fn byte_context_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_snapshot_mask(
        train_seed,
        holdout_seed,
        cases,
        [true; 6],
        "snapshot_centroid",
        "wrong_snapshot_centroid",
        "corrupted_snapshot_centroid",
        "byte_context_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_centroid",
    )
}

/// Test the offset-only feature mode found by byte-context centroid ablation.
#[must_use]
pub fn byte_context_offset_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_snapshot_mask(
        train_seed,
        holdout_seed,
        cases,
        [true, true, false, false, false, false],
        "snapshot_offset_centroid",
        "wrong_snapshot_offset_centroid",
        "corrupted_snapshot_offset_centroid",
        "byte_context_offset_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_offset_centroid",
    )
}

/// Test a denoised offset + top-sin mode found by feature ablation.
#[must_use]
pub fn byte_context_denoised_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_snapshot_mask(
        train_seed,
        holdout_seed,
        cases,
        [true, true, false, false, true, false],
        "snapshot_denoised_centroid",
        "wrong_snapshot_denoised_centroid",
        "corrupted_snapshot_denoised_centroid",
        "byte_context_denoised_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_denoised_centroid",
    )
}

/// Test seed-normalized snapshot features: keep phase relations, not raw top phases.
#[must_use]
pub fn byte_context_relative_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_feature_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_snapshot_relative_features,
        "snapshot_relative_centroid",
        "wrong_snapshot_relative_centroid",
        "corrupted_snapshot_relative_centroid",
        "byte_context_relative_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_relative_centroid",
    )
}

/// Test byte-context transfer when CarrierWave is keyed by stable lexical content.
#[must_use]
pub fn byte_context_lexical_carrier_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_context_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_lexical_carrier_features,
        byte_snapshot_carrier_state_features,
        "snapshot_lexical_carrier_centroid",
        "wrong_snapshot_lexical_carrier_centroid",
        "corrupted_snapshot_lexical_carrier_centroid",
        "byte_context_lexical_carrier_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_lexical_carrier_centroid",
    )
}

/// Test byte-context transfer when CarrierWave is locked by task-cell resonance.
#[must_use]
pub fn byte_context_cellular_carrier_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_context_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_cellular_carrier_features,
        byte_snapshot_carrier_state_features,
        "snapshot_cellular_carrier_centroid",
        "wrong_snapshot_cellular_carrier_centroid",
        "corrupted_snapshot_cellular_carrier_centroid",
        "byte_context_cellular_carrier_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_cellular_carrier_centroid",
    )
}

/// Test byte-context transfer when CarrierWave is locked by trained harmonic cells.
#[must_use]
pub fn byte_context_trained_carrier_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    let bank = train_byte_context_carrier_lock_bank(train_seed, cases);
    byte_context_centroid_eval_with_context_fn(
        train_seed,
        holdout_seed,
        cases,
        move |seed, case_index, prompt| {
            byte_context_trained_carrier_features(seed, case_index, prompt, bank, None)
        },
        byte_snapshot_carrier_state_features,
        "snapshot_trained_carrier_centroid",
        "wrong_snapshot_trained_carrier_centroid",
        "corrupted_snapshot_trained_carrier_centroid",
        "byte_context_trained_carrier_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_trained_carrier_centroid",
    )
}

/// Test byte-context transfer when CarrierWave is locked by a full-prompt harmonic bank.
#[must_use]
pub fn byte_context_prompt_carrier_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    let bank = train_byte_context_prompt_carrier_lock_bank(train_seed, cases);
    byte_context_centroid_eval_with_context_fn(
        train_seed,
        holdout_seed,
        cases,
        move |seed, case_index, prompt| {
            byte_context_trained_carrier_features(seed, case_index, prompt, bank, None)
        },
        byte_snapshot_carrier_state_features,
        "snapshot_prompt_carrier_centroid",
        "wrong_snapshot_prompt_carrier_centroid",
        "corrupted_snapshot_prompt_carrier_centroid",
        "byte_context_prompt_carrier_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_prompt_carrier_centroid",
    )
}

/// Test full-prompt harmonic lock transfer across diverse prompt templates.
#[must_use]
pub fn byte_context_prompt_carrier_diverse_centroid_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidReport {
    let bank = train_byte_context_prompt_carrier_lock_bank_with_sample_fn(
        train_seed,
        cases,
        byte_context_diverse_sample,
    );
    byte_context_centroid_eval_with_sample_and_context_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_diverse_sample,
        move |seed, case_index, prompt| {
            byte_context_trained_carrier_features(seed, case_index, prompt, bank, None)
        },
        byte_snapshot_carrier_state_features,
        "snapshot_prompt_carrier_diverse_centroid",
        "wrong_snapshot_prompt_carrier_diverse_centroid",
        "corrupted_snapshot_prompt_carrier_diverse_centroid",
        "byte_context_prompt_carrier_diverse_centroid_candidate_needs_seed_sweep",
        "not_found_byte_context_prompt_carrier_diverse_centroid",
    )
}

#[allow(clippy::too_many_arguments)]
fn byte_context_centroid_eval_with_snapshot_mask(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    snapshot_mask: [bool; 6],
    snapshot_name: &'static str,
    wrong_snapshot_name: &'static str,
    corrupted_snapshot_name: &'static str,
    pass_status: &'static str,
    fail_status: &'static str,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_feature_fn(
        train_seed,
        holdout_seed,
        cases,
        |snapshot| byte_snapshot_features_masked(snapshot, snapshot_mask),
        snapshot_name,
        wrong_snapshot_name,
        corrupted_snapshot_name,
        pass_status,
        fail_status,
    )
}

#[allow(clippy::too_many_arguments)]
fn byte_context_centroid_eval_with_feature_fn(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    snapshot_features_fn: impl Fn(SpectrumSnapshot) -> [f32; 6] + Copy,
    snapshot_name: &'static str,
    wrong_snapshot_name: &'static str,
    corrupted_snapshot_name: &'static str,
    pass_status: &'static str,
    fail_status: &'static str,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_context_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_features,
        snapshot_features_fn,
        snapshot_name,
        wrong_snapshot_name,
        corrupted_snapshot_name,
        pass_status,
        fail_status,
    )
}

#[allow(clippy::too_many_arguments)]
fn byte_context_centroid_eval_with_context_fn(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    context_fn: impl Fn(u64, usize, &[u8]) -> ByteContextFeatures + Copy,
    snapshot_features_fn: impl Fn(SpectrumSnapshot) -> [f32; 6] + Copy,
    snapshot_name: &'static str,
    wrong_snapshot_name: &'static str,
    corrupted_snapshot_name: &'static str,
    pass_status: &'static str,
    fail_status: &'static str,
) -> ByteContextCentroidReport {
    byte_context_centroid_eval_with_sample_and_context_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_sample,
        context_fn,
        snapshot_features_fn,
        snapshot_name,
        wrong_snapshot_name,
        corrupted_snapshot_name,
        pass_status,
        fail_status,
    )
}

#[allow(clippy::too_many_arguments)]
fn byte_context_centroid_eval_with_sample_and_context_fn(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    sample_fn: impl Fn(u64, usize, bool) -> ByteContextSample + Copy,
    context_fn: impl Fn(u64, usize, &[u8]) -> ByteContextFeatures + Copy,
    snapshot_features_fn: impl Fn(SpectrumSnapshot) -> [f32; 6] + Copy,
    snapshot_name: &'static str,
    wrong_snapshot_name: &'static str,
    corrupted_snapshot_name: &'static str,
    pass_status: &'static str,
    fail_status: &'static str,
) -> ByteContextCentroidReport {
    let mut mono_centroid = ByteCentroidClassifier::new();
    let mut no_snapshot_centroid = ByteCentroidClassifier::new();
    let mut snapshot_centroid = ByteCentroidClassifier::new();
    let mut wrong_snapshot_centroid = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = sample_fn(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = context_fn(train_seed, case_index, &sample.prompt);
        mono_centroid.observe(
            label,
            mono_prompt_features(train_seed, case_index, &sample.prompt),
        );
        no_snapshot_centroid.observe(label, no_snapshot_byte_features(context.current_byte));
        snapshot_centroid.observe(label, snapshot_features_fn(context.snapshot));
        wrong_snapshot_centroid.observe(label, snapshot_features_fn(context.wrong_snapshot));
    }
    mono_centroid.finish();
    no_snapshot_centroid.finish();
    snapshot_centroid.finish();
    wrong_snapshot_centroid.finish();

    let mut random = BaselineResult::new("random", cases);
    let mut mono192_prompt_centroid = BaselineResult::new("mono192_prompt_centroid", cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_centroid", cases);
    let mut voting = BaselineResult::new("cell32_voting", cases);
    let mut snapshot = BaselineResult::new(snapshot_name, cases);
    let mut wrong_snapshot = BaselineResult::new(wrong_snapshot_name, cases);
    let mut corrupted_snapshot = BaselineResult::new(corrupted_snapshot_name, cases);

    for case_index in 0..cases {
        let sample = sample_fn(holdout_seed, case_index, false);
        let context = context_fn(holdout_seed, case_index, &sample.prompt);
        let mono_features = mono_prompt_features(holdout_seed, case_index, &sample.prompt);
        let no_features = no_snapshot_byte_features(context.current_byte);
        let snapshot_features = snapshot_features_fn(context.snapshot);
        let wrong_features = snapshot_features_fn(context.wrong_snapshot);
        let corrupted_features = snapshot_features_fn(context.corrupted_snapshot);

        let random_prediction = random_predict(holdout_seed, case_index, context.current_byte);
        score_prediction(&mut random, random_prediction, sample.target, 0.0, 1.0);

        score_prediction(
            &mut mono192_prompt_centroid,
            mono_centroid.predict(mono_features),
            sample.target,
            0.0,
            1.0,
        );
        score_prediction(
            &mut no_snapshot,
            no_snapshot_centroid.predict(no_features),
            sample.target,
            0.0,
            1.0,
        );

        let voting_prediction = voting_predict(context.current_byte, context.active_cell_ids);
        score_prediction(
            &mut voting,
            voting_prediction,
            sample.target,
            context.coherence * 0.75,
            context.spectral_entropy,
        );
        score_prediction(
            &mut snapshot,
            snapshot_centroid.predict(snapshot_features),
            sample.target,
            context.snapshot.coherence,
            context.snapshot.spectral_entropy,
        );
        score_prediction(
            &mut wrong_snapshot,
            wrong_snapshot_centroid.predict(wrong_features),
            sample.target,
            context.wrong_snapshot.coherence,
            context.wrong_snapshot.spectral_entropy,
        );
        score_prediction(
            &mut corrupted_snapshot,
            snapshot_centroid.predict(corrupted_features),
            sample.target,
            context.corrupted_snapshot.coherence,
            context.corrupted_snapshot.spectral_entropy,
        );
    }

    random.finish();
    mono192_prompt_centroid.finish();
    no_snapshot.finish();
    voting.finish();
    snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let best_control = best_baseline([random, mono192_prompt_centroid, no_snapshot, voting]);
    let snapshot_accuracy_over_best_control = snapshot.accuracy - best_control.accuracy;
    let snapshot_error_gain_over_best_control =
        best_control.mean_circular_error - snapshot.mean_circular_error;
    let snapshot_error_gain_over_wrong_snapshot =
        wrong_snapshot.mean_circular_error - snapshot.mean_circular_error;
    let mode_status = if snapshot_accuracy_over_best_control > 0.02
        && snapshot_error_gain_over_best_control > 0.0
        && snapshot_error_gain_over_wrong_snapshot > 0.0
        && snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        pass_status
    } else {
        fail_status
    };

    ByteContextCentroidReport {
        train_seed,
        holdout_seed,
        train_cases: cases,
        holdout_cases: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192_prompt_centroid,
        no_snapshot_centroid: no_snapshot,
        cell32_voting: voting,
        snapshot_centroid: snapshot,
        wrong_snapshot_centroid: wrong_snapshot,
        corrupted_snapshot_centroid: corrupted_snapshot,
        snapshot_accuracy_over_best_control,
        snapshot_error_gain_over_best_control,
        snapshot_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep the byte-context centroid candidate over fixed seed pairs.
#[must_use]
pub fn byte_context_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = byte_context_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status == "byte_context_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep the offset-only byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_offset_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = byte_context_offset_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status == "byte_context_offset_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_offset_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_offset_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep the denoised byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_denoised_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = byte_context_denoised_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_denoised_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_denoised_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_denoised_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep the seed-normalized relative byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_relative_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = byte_context_relative_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_relative_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_relative_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_relative_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep lexical-carrier byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_lexical_carrier_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report =
            byte_context_lexical_carrier_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_lexical_carrier_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_lexical_carrier_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_lexical_carrier_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep cellular CarrierWave byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_cellular_carrier_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report =
            byte_context_cellular_carrier_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_cellular_carrier_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_cellular_carrier_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_cellular_carrier_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep trained CarrierWave byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_trained_carrier_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report =
            byte_context_trained_carrier_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_trained_carrier_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_trained_carrier_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_trained_carrier_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep prompt-cloud CarrierWave byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_prompt_carrier_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report =
            byte_context_prompt_carrier_centroid_eval(train_seed, holdout_seed, cases_per_split);
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_prompt_carrier_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_prompt_carrier_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_prompt_carrier_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep diverse prompt-cloud CarrierWave byte-context centroid over fixed seed pairs.
#[must_use]
pub fn byte_context_prompt_carrier_diverse_centroid_seed_sweep_eval(
    cases_per_split: usize,
) -> ByteContextCentroidSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let report = byte_context_prompt_carrier_diverse_centroid_eval(
            train_seed,
            holdout_seed,
            cases_per_split,
        );
        let best_control = best_baseline([
            report.random,
            report.mono192_prompt_centroid,
            report.no_snapshot_centroid,
            report.cell32_voting,
        ]);
        ByteContextCentroidSeedRow {
            train_seed,
            holdout_seed,
            snapshot_accuracy: report.snapshot_centroid.accuracy,
            best_control_accuracy: best_control.accuracy,
            wrong_snapshot_accuracy: report.wrong_snapshot_centroid.accuracy,
            corrupted_snapshot_accuracy: report.corrupted_snapshot_centroid.accuracy,
            snapshot_accuracy_over_best_control: report.snapshot_accuracy_over_best_control,
            snapshot_error_gain_over_best_control: report.snapshot_error_gain_over_best_control,
            snapshot_error_gain_over_wrong_snapshot: report.snapshot_error_gain_over_wrong_snapshot,
            passed: report.mode_status
                == "byte_context_prompt_carrier_diverse_centroid_candidate_needs_seed_sweep",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_snapshot_accuracy_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_accuracy_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_best_control = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_best_control)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.snapshot_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_snapshot_accuracy_over_best_control > 0.0
        && min_error_gain_over_best_control > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "byte_context_prompt_carrier_diverse_centroid_seed_sweep_passed"
    } else {
        "not_found_byte_context_prompt_carrier_diverse_centroid_seed_sweep"
    };

    ByteContextCentroidSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_snapshot_accuracy_over_best_control,
        min_error_gain_over_best_control,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Ablate each non-bias snapshot feature in the byte-context centroid probe.
#[must_use]
pub fn byte_context_centroid_ablation_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCentroidAblationReport {
    const ABLATIONS: [(&str, [bool; 6]); 5] = [
        (
            "ablate_snapshot_offset",
            [true, false, true, true, true, true],
        ),
        (
            "ablate_snapshot_coherence",
            [true, true, false, true, true, true],
        ),
        (
            "ablate_snapshot_magnitude",
            [true, true, true, false, true, true],
        ),
        (
            "ablate_snapshot_top_sin",
            [true, true, true, true, false, true],
        ),
        (
            "ablate_snapshot_top_cos",
            [true, true, true, true, true, false],
        ),
    ];
    const FULL_MASK: [bool; 6] = [true; 6];

    let full_snapshot = byte_context_centroid_masked_result(
        "snapshot_centroid_full",
        train_seed,
        holdout_seed,
        cases,
        FULL_MASK,
    );
    let ablations = ABLATIONS.map(|(name, mask)| {
        byte_context_centroid_masked_result(name, train_seed, holdout_seed, cases, mask)
    });

    let mut key_feature = ABLATIONS[0].0;
    let mut max_accuracy_drop = full_snapshot.accuracy - ablations[0].accuracy;
    let mut max_error_increase =
        ablations[0].mean_circular_error - full_snapshot.mean_circular_error;

    for ((feature_name, _), ablation) in ABLATIONS.iter().zip(ablations.iter()).skip(1) {
        let accuracy_drop = full_snapshot.accuracy - ablation.accuracy;
        let error_increase = ablation.mean_circular_error - full_snapshot.mean_circular_error;
        if accuracy_drop > max_accuracy_drop
            || (accuracy_drop == max_accuracy_drop && error_increase > max_error_increase)
        {
            key_feature = feature_name;
            max_accuracy_drop = accuracy_drop;
            max_error_increase = error_increase;
        }
    }

    let mode_status = if max_accuracy_drop > 0.0 && max_error_increase > 0.0 {
        "byte_context_centroid_ablation_sensitive"
    } else {
        "not_found_byte_context_centroid_ablation"
    };

    ByteContextCentroidAblationReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        full_snapshot,
        ablations,
        key_feature,
        max_accuracy_drop,
        max_error_increase,
        mode_status,
    }
}

/// Ablate cellular CarrierWave lock cells after training the normal lock bridge.
#[must_use]
pub fn byte_context_cellular_carrier_ablation_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextCellularCarrierAblationReport {
    let mut classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context =
            byte_context_cellular_carrier_features(train_seed, case_index, &sample.prompt);
        classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    classifier.finish();

    let full_snapshot = byte_context_cellular_carrier_ablation_result(
        "snapshot_cellular_carrier_full",
        holdout_seed,
        cases,
        classifier,
        None,
    );
    let ablations = std::array::from_fn(|task_index| {
        let name = BYTE_CONTEXT_TASKS[task_index].0;
        let result_name = match name {
            "ping" => "ablate_lock_ping",
            "name" => "ablate_lock_name",
            "time" => "ablate_lock_time",
            "help" => "ablate_lock_help",
            "echo" => "ablate_lock_echo",
            "save" => "ablate_lock_save",
            "open" => "ablate_lock_open",
            "close" => "ablate_lock_close",
            _ => "ablate_lock_unknown",
        };
        byte_context_cellular_carrier_ablation_result(
            result_name,
            holdout_seed,
            cases,
            classifier,
            Some(task_index),
        )
    });

    let min_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::INFINITY, f32::min);
    let max_error_increase = ablations
        .iter()
        .map(|ablation| ablation.mean_circular_error - full_snapshot.mean_circular_error)
        .fold(f32::NEG_INFINITY, f32::max);
    let mode_status =
        if full_snapshot.accuracy > 0.9 && min_accuracy_drop > 0.05 && max_error_increase > 0.0 {
            "byte_context_cellular_carrier_ablation_sensitive"
        } else {
            "not_found_byte_context_cellular_carrier_ablation"
        };

    ByteContextCellularCarrierAblationReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        full_snapshot,
        ablations,
        min_accuracy_drop,
        max_error_increase,
        mode_status,
    }
}

/// Ablate trained CarrierWave lock cells after supervised harmonic lock training.
#[must_use]
pub fn byte_context_trained_carrier_ablation_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextTrainedCarrierAblationReport {
    let bank = train_byte_context_carrier_lock_bank(train_seed, cases);
    let mut classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    classifier.finish();

    let full_snapshot = byte_context_trained_carrier_ablation_result(
        "snapshot_trained_carrier_full",
        holdout_seed,
        cases,
        classifier,
        bank,
        None,
    );
    let ablations = std::array::from_fn(|task_index| {
        let name = BYTE_CONTEXT_TASKS[task_index].0;
        let result_name = match name {
            "ping" => "ablate_trained_lock_ping",
            "name" => "ablate_trained_lock_name",
            "time" => "ablate_trained_lock_time",
            "help" => "ablate_trained_lock_help",
            "echo" => "ablate_trained_lock_echo",
            "save" => "ablate_trained_lock_save",
            "open" => "ablate_trained_lock_open",
            "close" => "ablate_trained_lock_close",
            _ => "ablate_trained_lock_unknown",
        };
        byte_context_trained_carrier_ablation_result(
            result_name,
            holdout_seed,
            cases,
            classifier,
            bank,
            Some(task_index),
        )
    });

    let min_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::INFINITY, f32::min);
    let max_error_increase = ablations
        .iter()
        .map(|ablation| ablation.mean_circular_error - full_snapshot.mean_circular_error)
        .fold(f32::NEG_INFINITY, f32::max);
    let mode_status =
        if full_snapshot.accuracy > 0.9 && min_accuracy_drop > 0.05 && max_error_increase > 0.0 {
            "byte_context_trained_carrier_ablation_sensitive"
        } else {
            "not_found_byte_context_trained_carrier_ablation"
        };

    ByteContextTrainedCarrierAblationReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        full_snapshot,
        ablations,
        min_accuracy_drop,
        max_error_increase,
        mode_status,
    }
}

/// Ablate prompt-cloud CarrierWave lock cells after full-prompt lock training.
#[must_use]
pub fn byte_context_prompt_carrier_ablation_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextPromptCarrierAblationReport {
    let bank = train_byte_context_prompt_carrier_lock_bank(train_seed, cases);
    let mut classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    classifier.finish();

    let full_snapshot = byte_context_trained_carrier_ablation_result(
        "snapshot_prompt_carrier_full",
        holdout_seed,
        cases,
        classifier,
        bank,
        None,
    );
    let empty_bank = TrainedCarrierLockBank::new(CarrierLockFeatureMode::PromptCloud);
    let all_disabled = byte_context_trained_carrier_ablation_result(
        "ablate_prompt_lock_all",
        holdout_seed,
        cases,
        classifier,
        empty_bank,
        None,
    );
    let ablations = std::array::from_fn(|task_index| {
        let name = BYTE_CONTEXT_TASKS[task_index].0;
        let result_name = match name {
            "ping" => "ablate_prompt_lock_ping",
            "name" => "ablate_prompt_lock_name",
            "time" => "ablate_prompt_lock_time",
            "help" => "ablate_prompt_lock_help",
            "echo" => "ablate_prompt_lock_echo",
            "save" => "ablate_prompt_lock_save",
            "open" => "ablate_prompt_lock_open",
            "close" => "ablate_prompt_lock_close",
            _ => "ablate_prompt_lock_unknown",
        };
        byte_context_trained_carrier_ablation_result(
            result_name,
            holdout_seed,
            cases,
            classifier,
            bank,
            Some(task_index),
        )
    });

    let min_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::INFINITY, f32::min);
    let max_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_error_increase = ablations
        .iter()
        .map(|ablation| ablation.mean_circular_error - full_snapshot.mean_circular_error)
        .fold(f32::NEG_INFINITY, f32::max);
    let accuracy_over_all_disabled = full_snapshot.accuracy - all_disabled.accuracy;
    let error_gain_over_all_disabled =
        all_disabled.mean_circular_error - full_snapshot.mean_circular_error;
    let mode_status = if full_snapshot.accuracy > 0.5
        && accuracy_over_all_disabled > 0.2
        && error_gain_over_all_disabled > 0.0
        && max_accuracy_drop > 0.05
    {
        "byte_context_prompt_carrier_bank_ablation_sensitive"
    } else {
        "not_found_byte_context_prompt_carrier_ablation"
    };

    ByteContextPromptCarrierAblationReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        full_snapshot,
        all_disabled,
        ablations,
        min_accuracy_drop,
        max_accuracy_drop,
        max_error_increase,
        accuracy_over_all_disabled,
        error_gain_over_all_disabled,
        mode_status,
    }
}

/// Ablate prompt-cloud lock cells on diverse prompt templates.
#[must_use]
pub fn byte_context_prompt_carrier_diverse_ablation_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
) -> ByteContextPromptCarrierAblationReport {
    byte_context_prompt_carrier_ablation_eval_with_sample_fn(
        train_seed,
        holdout_seed,
        cases,
        byte_context_diverse_sample,
        "snapshot_prompt_carrier_diverse_full",
        "ablate_prompt_diverse_lock_all",
        "byte_context_prompt_carrier_diverse_bank_ablation_sensitive",
        "not_found_byte_context_prompt_carrier_diverse_ablation",
    )
}

/// Run the first minimal Chat-0 loop over diverse prompt templates.
///
/// This is intentionally small: it generates short fixed responses from the
/// mode label recovered through the prompt-cloud CarrierWave snapshot, then
/// records feedback events for wrong responses. It is a gate for the loop
/// shape, not a claim of open-ended language modeling.
#[must_use]
pub fn chat0_eval(train_seed: u64, holdout_seed: u64, cases: usize) -> Chat0EvalReport {
    let bank = train_byte_context_prompt_carrier_lock_bank_with_sample_fn(
        train_seed,
        cases,
        byte_context_diverse_sample,
    );
    let mut mono_classifier = ByteCentroidClassifier::new();
    let mut no_snapshot_classifier = ByteCentroidClassifier::new();
    let mut snapshot_classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_diverse_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        mono_classifier.observe(
            label,
            mono_prompt_features(train_seed, case_index, &sample.prompt),
        );
        no_snapshot_classifier.observe(label, no_snapshot_byte_features(context.current_byte));
        snapshot_classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    mono_classifier.finish();
    no_snapshot_classifier.finish();
    snapshot_classifier.finish();

    let mut random = Chat0Result::new("random_chat0", cases);
    let mut mono192_prompt = Chat0Result::new("mono192_prompt_chat0", cases);
    let mut no_snapshot = Chat0Result::new("no_snapshot_chat0", cases);
    let mut wrong_snapshot = Chat0Result::new("wrong_snapshot_chat0", cases);
    let mut corrupted_snapshot = Chat0Result::new("corrupted_snapshot_chat0", cases);
    let mut prompt_cloud_snapshot = Chat0Result::new("prompt_cloud_snapshot_chat0", cases);
    let mut feedback_log_entries = 0usize;

    for case_index in 0..cases {
        let sample = byte_context_diverse_sample(holdout_seed, case_index, false);
        let expected = chat0_response_for_target(sample.target);
        let context = byte_context_trained_carrier_features(
            holdout_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );

        let random_target = random_task_target(holdout_seed, case_index);
        random.score(chat0_response_for_target(random_target), expected);

        let mono_target = mono_classifier.predict(mono_prompt_features(
            holdout_seed,
            case_index,
            &sample.prompt,
        ));
        mono192_prompt.score(chat0_response_for_target(mono_target), expected);

        let no_snapshot_target =
            no_snapshot_classifier.predict(no_snapshot_byte_features(context.current_byte));
        no_snapshot.score(chat0_response_for_target(no_snapshot_target), expected);

        let wrong_target = snapshot_classifier
            .predict(byte_snapshot_carrier_state_features(context.wrong_snapshot));
        wrong_snapshot.score(chat0_response_for_target(wrong_target), expected);

        let corrupted_target = snapshot_classifier.predict(byte_snapshot_carrier_state_features(
            context.corrupted_snapshot,
        ));
        corrupted_snapshot.score(chat0_response_for_target(corrupted_target), expected);

        let prompt_cloud_target =
            snapshot_classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
        let prompt_cloud_response = chat0_response_for_target(prompt_cloud_target);
        if prompt_cloud_response != expected {
            feedback_log_entries += 1;
        }
        prompt_cloud_snapshot.score(prompt_cloud_response, expected);
    }

    random.finish();
    mono192_prompt.finish();
    no_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();
    prompt_cloud_snapshot.finish();

    let best_control = best_chat0_control([
        random,
        mono192_prompt,
        no_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
    ]);
    let prompt_cloud_over_best_control =
        prompt_cloud_snapshot.exact_accuracy - best_control.exact_accuracy;
    let prompt_cloud_over_wrong_snapshot =
        prompt_cloud_snapshot.exact_accuracy - wrong_snapshot.exact_accuracy;
    let mode_status = if prompt_cloud_snapshot.exact_accuracy > 0.5
        && prompt_cloud_over_best_control > 0.2
        && prompt_cloud_over_wrong_snapshot > 0.2
        && feedback_log_entries < cases
    {
        "chat0_prompt_cloud_loop_passed"
    } else {
        "not_found_chat0_prompt_cloud_loop"
    };

    Chat0EvalReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192_prompt,
        no_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        prompt_cloud_snapshot,
        feedback_log_entries,
        prompt_cloud_over_best_control,
        prompt_cloud_over_wrong_snapshot,
        mode_status,
    }
}

/// Evaluate the route used by manual Chat-0 prompts.
///
/// This tests whether the one-shot CLI route is useful on prompt shapes that
/// are outside the automatic `eval-chat0` holdout templates.
#[must_use]
pub fn chat0_route_eval(train_seed: u64, holdout_seed: u64, cases: usize) -> Chat0RouteEvalReport {
    let bank = train_chat0_route_lock_bank(train_seed, cases);
    let mut mono_classifier = ByteCentroidClassifier::new();
    let mut snapshot_classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        observe_chat0_route_training_sample(
            train_seed,
            case_index,
            byte_context_diverse_sample(train_seed, case_index, true),
            bank,
            &mut mono_classifier,
            &mut snapshot_classifier,
        );
        observe_chat0_route_training_sample(
            train_seed,
            case_index + cases,
            chat0_free_prompt_sample(train_seed, case_index),
            bank,
            &mut mono_classifier,
            &mut snapshot_classifier,
        );
    }
    mono_classifier.finish();
    snapshot_classifier.finish();

    let mut random = Chat0Result::new("random_chat0_route", cases);
    let mut mono192_prompt = Chat0Result::new("mono192_prompt_chat0_route", cases);
    let mut snapshot = Chat0Result::new("snapshot_classifier_chat0_route", cases);
    let mut lock_bank = Chat0Result::new("prompt_cloud_lock_bank_chat0_route", cases);
    let mut hybrid = Chat0Result::new("hybrid_chat0_route", cases);
    let mut lock_bank_route_count = 0usize;
    let mut feedback_log_entries = 0usize;

    for case_index in 0..cases {
        let sample = chat0_free_prompt_sample(holdout_seed, case_index);
        let expected = chat0_response_for_target(sample.target);
        let context = byte_context_trained_carrier_features(
            holdout_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );

        let random_target = random_task_target(holdout_seed, case_index);
        random.score(chat0_response_for_target(random_target), expected);

        let mono_target = mono_classifier.predict(mono_prompt_features(
            holdout_seed,
            case_index,
            &sample.prompt,
        ));
        mono192_prompt.score(chat0_response_for_target(mono_target), expected);

        let snapshot_target =
            snapshot_classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
        snapshot.score(chat0_response_for_target(snapshot_target), expected);

        let bank_target = bank
            .predict_label(&sample.prompt, None)
            .map(|label| BYTE_CONTEXT_TASKS[label].1);
        let hybrid_target = bank_target.unwrap_or(snapshot_target);
        if bank_target.is_some() {
            lock_bank_route_count += 1;
        }
        lock_bank.score(
            chat0_response_for_target(bank_target.unwrap_or(b'?')),
            expected,
        );
        let hybrid_response = chat0_response_for_target(hybrid_target);
        if hybrid_response != expected {
            feedback_log_entries += 1;
        }
        hybrid.score(hybrid_response, expected);
    }

    random.finish();
    mono192_prompt.finish();
    snapshot.finish();
    lock_bank.finish();
    hybrid.finish();

    let best_control = best_chat0_control([random, mono192_prompt]);
    let lock_bank_over_snapshot = lock_bank.exact_accuracy - snapshot.exact_accuracy;
    let hybrid_over_best_control = hybrid.exact_accuracy - best_control.exact_accuracy;
    let mode_status = if hybrid.exact_accuracy > 0.8
        && hybrid_over_best_control > 0.5
        && lock_bank_over_snapshot >= -0.05
        && lock_bank_route_count == cases
        && feedback_log_entries < cases / 2
    {
        "chat0_route_usable_snapshot_tied_or_better"
    } else {
        "not_found_chat0_route_quality"
    };

    Chat0RouteEvalReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192_prompt,
        snapshot_classifier: snapshot,
        prompt_cloud_lock_bank: lock_bank,
        hybrid_route: hybrid,
        lock_bank_route_count,
        feedback_log_entries,
        lock_bank_over_snapshot,
        hybrid_over_best_control,
        mode_status,
    }
}

/// Evaluate whether logged Chat-0 feedback is safe enough to become a
/// promotion candidate.
///
/// This deliberately does not mutate any runtime state. It only compares the
/// current answer replay with an exact feedback-replay candidate, then keeps the
/// normal route eval as a guard so a local correction cannot masquerade as a
/// new ensemble mode.
#[must_use]
pub fn chat0_promote_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    feedback: &[Chat0FeedbackEntry],
) -> Chat0PromoteEvalReport {
    let mut replay_base = Chat0Result::new("chat0_feedback_replay_base", feedback.len());
    let mut replay_candidate = Chat0Result::new("chat0_feedback_replay_candidate", feedback.len());
    let mut correction_entries = 0usize;

    for entry in feedback {
        let trace = chat0_once(
            train_seed,
            cases,
            entry.prompt.as_bytes(),
            Some(&entry.expected),
        );
        replay_base.score_text(trace.response, &entry.expected);

        let candidate_response =
            if !entry.feedback_correct && chat0_target_for_response(&entry.expected).is_some() {
                correction_entries += 1;
                entry.expected.as_str()
            } else {
                trace.response
            };
        replay_candidate.score_text(candidate_response, &entry.expected);
    }

    replay_base.finish();
    replay_candidate.finish();

    let route_eval = chat0_route_eval(train_seed, holdout_seed, cases);
    let replay_improvement = replay_candidate.exact_accuracy - replay_base.exact_accuracy;
    let mode_status = if feedback.is_empty() {
        "not_found_chat0_promote_no_feedback"
    } else if correction_entries == 0 {
        "not_found_chat0_promote_no_corrections"
    } else if replay_improvement > 0.0
        && route_eval.mode_status == "chat0_route_usable_snapshot_tied_or_better"
        && route_eval.hybrid_route.exact_accuracy >= 0.8
    {
        "chat0_feedback_replay_promote_candidate_passed"
    } else {
        "not_found_chat0_feedback_promote"
    };

    Chat0PromoteEvalReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        feedback_entries: feedback.len(),
        correction_entries,
        replay_base,
        replay_candidate,
        route_eval,
        replay_improvement,
        mode_status,
    }
}

/// Evaluate whether promoted feedback transfers beyond exact prompt replay.
///
/// The `exact_overlay` line uses the persisted `.nwps` semantics. The
/// `harmonic_transfer_overlay` trains a tiny centroid over promoted carrier
/// snapshots and tries to transfer corrections through the wave state rather
/// than exact prompt text. `task_hint_overlay` is an explicit upper-bound probe:
/// it applies a promoted target to other holdout prompts that contain the same
/// fixed Chat-0 task word. Passing the task-hint probe is useful engineering
/// evidence, not a proof of a new ensemble mode.
#[must_use]
pub fn chat0_promoted_holdout_eval(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    feedback: &[Chat0FeedbackEntry],
) -> Chat0PromotedHoldoutEvalReport {
    let state = Chat0PromotedState::from_feedback(train_seed, cases, feedback);
    let bank = train_chat0_route_lock_bank(train_seed, cases);
    let mut snapshot_classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_diverse_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        snapshot_classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    snapshot_classifier.finish();

    let mut base = Chat0Result::new("chat0_promoted_holdout_base", cases);
    let mut exact_overlay = Chat0Result::new("chat0_promoted_holdout_exact_overlay", cases);
    let mut harmonic_transfer_overlay =
        Chat0Result::new("chat0_promoted_holdout_harmonic_transfer_overlay", cases);
    let mut selective_harmonic_transfer_overlay = Chat0Result::new(
        "chat0_promoted_holdout_selective_harmonic_transfer_overlay",
        cases,
    );
    let mut cell_signature_transfer_overlay = Chat0Result::new(
        "chat0_promoted_holdout_cell_signature_transfer_overlay",
        cases,
    );
    let mut trajectory_transfer_overlay =
        Chat0Result::new("chat0_promoted_holdout_trajectory_transfer_overlay", cases);
    let mut task_hint_overlay = Chat0Result::new("chat0_promoted_holdout_task_hint_overlay", cases);
    let mut exact_overlay_applied = 0usize;
    let mut harmonic_transfer_applied = 0usize;
    let mut selective_harmonic_transfer_applied = 0usize;
    let mut cell_signature_transfer_applied = 0usize;
    let mut trajectory_transfer_applied = 0usize;
    let mut task_hint_overlay_applied = 0usize;
    let mut promoted_transfer = ByteCentroidClassifier::new();
    let mut promoted_cell_transfer = CellSignatureClassifier::new();
    let mut promoted_trajectory_transfer = TrajectorySignatureClassifier::new();
    for (entry_index, entry) in state.entries.iter().enumerate() {
        let context = byte_context_trained_carrier_features(
            train_seed,
            entry_index,
            entry.prompt.as_bytes(),
            bank,
            None,
        );
        promoted_transfer.observe(
            byte_context_label_index(entry.target),
            byte_snapshot_carrier_state_features(context.snapshot),
        );
        promoted_cell_transfer.observe(
            byte_context_label_index(entry.target),
            cell_signature_features(&context),
        );
        promoted_trajectory_transfer.observe(
            byte_context_label_index(entry.target),
            trajectory_signature_features(train_seed, entry.prompt.as_bytes(), bank, None),
        );
    }
    promoted_transfer.finish();
    promoted_cell_transfer.finish();
    promoted_trajectory_transfer.finish();
    let mut harmonic_ablation_results = BYTE_CONTEXT_TASKS
        .map(|_| Chat0Result::new("chat0_promoted_holdout_harmonic_transfer_ablation", cases));
    let mut cell_signature_ablation_results = BYTE_CONTEXT_TASKS
        .map(|_| Chat0Result::new("chat0_promoted_holdout_cell_signature_ablation", cases));
    let mut trajectory_ablation_results = BYTE_CONTEXT_TASKS
        .map(|_| Chat0Result::new("chat0_promoted_holdout_trajectory_ablation", cases));
    let mut selective_threshold_results = PROMOTED_HARMONIC_TRANSFER_MARGIN_SWEEP
        .map(|_| Chat0Result::new("chat0_promoted_holdout_selective_harmonic_threshold", cases));

    for case_index in 0..cases {
        let sample = chat0_free_prompt_sample(holdout_seed, case_index);
        let expected = chat0_response_for_target(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        let snapshot_target =
            snapshot_classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
        let base_target = bank
            .predict_label(&sample.prompt, None)
            .map(|label| BYTE_CONTEXT_TASKS[label].1)
            .unwrap_or(snapshot_target);
        let base_response = chat0_response_for_target(base_target);
        base.score_text(base_response, expected);

        let exact_response = match state.target_for_prompt(train_seed, cases, &sample.prompt) {
            Some(target) => {
                exact_overlay_applied += 1;
                chat0_response_for_target(target)
            }
            None => base_response,
        };
        exact_overlay.score_text(exact_response, expected);

        let harmonic_response = if state.entries.is_empty() {
            base_response
        } else {
            harmonic_transfer_applied += 1;
            let target =
                promoted_transfer.predict(byte_snapshot_carrier_state_features(context.snapshot));
            chat0_response_for_target(target)
        };
        harmonic_transfer_overlay.score_text(harmonic_response, expected);

        let selective_prediction = (!state.entries.is_empty()).then(|| {
            promoted_transfer
                .predict_with_margin(byte_snapshot_carrier_state_features(context.snapshot))
        });

        let selective_harmonic_response = if let Some(prediction) = selective_prediction {
            if prediction.margin >= PROMOTED_HARMONIC_TRANSFER_MIN_MARGIN {
                selective_harmonic_transfer_applied += 1;
                chat0_response_for_target(prediction.target)
            } else {
                base_response
            }
        } else {
            base_response
        };
        selective_harmonic_transfer_overlay.score_text(selective_harmonic_response, expected);

        let cell_signature_response = if state.entries.is_empty() {
            base_response
        } else {
            cell_signature_transfer_applied += 1;
            let target = promoted_cell_transfer.predict(cell_signature_features(&context));
            chat0_response_for_target(target)
        };
        cell_signature_transfer_overlay.score_text(cell_signature_response, expected);

        let trajectory_response = if state.entries.is_empty() {
            base_response
        } else {
            trajectory_transfer_applied += 1;
            let target = promoted_trajectory_transfer.predict(trajectory_signature_features(
                train_seed,
                &sample.prompt,
                bank,
                None,
            ));
            chat0_response_for_target(target)
        };
        trajectory_transfer_overlay.score_text(trajectory_response, expected);

        for (threshold, result) in PROMOTED_HARMONIC_TRANSFER_MARGIN_SWEEP
            .iter()
            .zip(selective_threshold_results.iter_mut())
        {
            let response = if let Some(prediction) = selective_prediction {
                if prediction.margin >= *threshold {
                    chat0_response_for_target(prediction.target)
                } else {
                    base_response
                }
            } else {
                base_response
            };
            result.score_text(response, expected);
        }

        if !state.entries.is_empty() {
            for (task_index, ablation) in harmonic_ablation_results.iter_mut().enumerate() {
                let ablated_context = byte_context_trained_carrier_features(
                    train_seed,
                    case_index,
                    &sample.prompt,
                    bank,
                    Some(task_index),
                );
                let target = promoted_transfer.predict(byte_snapshot_carrier_state_features(
                    ablated_context.snapshot,
                ));
                ablation.score_text(chat0_response_for_target(target), expected);
            }

            for (task_index, ablation) in cell_signature_ablation_results.iter_mut().enumerate() {
                let ablated_context = byte_context_trained_carrier_features(
                    train_seed,
                    case_index,
                    &sample.prompt,
                    bank,
                    Some(task_index),
                );
                let target =
                    promoted_cell_transfer.predict(cell_signature_features(&ablated_context));
                ablation.score_text(chat0_response_for_target(target), expected);
            }

            for (task_index, ablation) in trajectory_ablation_results.iter_mut().enumerate() {
                let target = promoted_trajectory_transfer.predict(trajectory_signature_features(
                    train_seed,
                    &sample.prompt,
                    bank,
                    Some(task_index),
                ));
                ablation.score_text(chat0_response_for_target(target), expected);
            }
        }

        let hinted_response = match state.target_for_task_hint(train_seed, cases, &sample.prompt) {
            Some(target) => {
                task_hint_overlay_applied += 1;
                chat0_response_for_target(target)
            }
            None => base_response,
        };
        task_hint_overlay.score_text(hinted_response, expected);
    }

    base.finish();
    exact_overlay.finish();
    harmonic_transfer_overlay.finish();
    selective_harmonic_transfer_overlay.finish();
    cell_signature_transfer_overlay.finish();
    trajectory_transfer_overlay.finish();
    for ablation in &mut harmonic_ablation_results {
        ablation.finish();
    }
    for ablation in &mut cell_signature_ablation_results {
        ablation.finish();
    }
    for ablation in &mut trajectory_ablation_results {
        ablation.finish();
    }
    for threshold_result in &mut selective_threshold_results {
        threshold_result.finish();
    }
    task_hint_overlay.finish();

    let exact_over_base = exact_overlay.exact_accuracy - base.exact_accuracy;
    let harmonic_transfer_over_base =
        harmonic_transfer_overlay.exact_accuracy - base.exact_accuracy;
    let selective_harmonic_transfer_over_base =
        selective_harmonic_transfer_overlay.exact_accuracy - base.exact_accuracy;
    let cell_signature_transfer_over_base =
        cell_signature_transfer_overlay.exact_accuracy - base.exact_accuracy;
    let trajectory_transfer_over_base =
        trajectory_transfer_overlay.exact_accuracy - base.exact_accuracy;
    let harmonic_transfer_ablation_min_accuracy = harmonic_ablation_results
        .iter()
        .map(|ablation| ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let harmonic_transfer_ablation_min_drop = harmonic_ablation_results
        .iter()
        .map(|ablation| harmonic_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let harmonic_transfer_ablation_max_drop = harmonic_ablation_results
        .iter()
        .map(|ablation| harmonic_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let cell_signature_ablation_min_accuracy = cell_signature_ablation_results
        .iter()
        .map(|ablation| ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let cell_signature_ablation_min_drop = cell_signature_ablation_results
        .iter()
        .map(|ablation| cell_signature_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let cell_signature_ablation_max_drop = cell_signature_ablation_results
        .iter()
        .map(|ablation| cell_signature_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let trajectory_ablation_min_accuracy = trajectory_ablation_results
        .iter()
        .map(|ablation| ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let trajectory_ablation_min_drop = trajectory_ablation_results
        .iter()
        .map(|ablation| trajectory_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::INFINITY, f32::min);
    let trajectory_ablation_max_drop = trajectory_ablation_results
        .iter()
        .map(|ablation| trajectory_transfer_overlay.exact_accuracy - ablation.exact_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let (selective_harmonic_best_threshold, selective_harmonic_best_accuracy) =
        PROMOTED_HARMONIC_TRANSFER_MARGIN_SWEEP
            .iter()
            .copied()
            .zip(
                selective_threshold_results
                    .iter()
                    .map(|result| result.exact_accuracy),
            )
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .unwrap_or((PROMOTED_HARMONIC_TRANSFER_MIN_MARGIN, base.exact_accuracy));
    let selective_harmonic_best_over_base = selective_harmonic_best_accuracy - base.exact_accuracy;
    let task_hint_over_base = task_hint_overlay.exact_accuracy - base.exact_accuracy;
    let mode_status = if feedback.is_empty() {
        "not_found_chat0_promoted_holdout_no_feedback"
    } else if state.entries.is_empty() {
        "not_found_chat0_promoted_holdout_no_state"
    } else if exact_over_base > 0.0 {
        "chat0_promoted_exact_overlay_leaked_to_holdout"
    } else if harmonic_transfer_over_base > 0.0
        && harmonic_transfer_applied > exact_overlay_applied
        && harmonic_transfer_overlay.exact_accuracy < task_hint_overlay.exact_accuracy
    {
        "chat0_promoted_harmonic_transfer_candidate_below_task_hint"
    } else if harmonic_transfer_over_base > 0.0 && harmonic_transfer_applied > exact_overlay_applied
    {
        "chat0_promoted_harmonic_transfer_candidate_needs_ablation"
    } else if task_hint_over_base > 0.0 && task_hint_overlay_applied > exact_overlay_applied {
        "chat0_promoted_task_hint_holdout_candidate_not_mode"
    } else {
        "not_found_chat0_promoted_holdout_generalization"
    };

    Chat0PromotedHoldoutEvalReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        feedback_entries: feedback.len(),
        promoted_entries: state.entries.len(),
        base,
        exact_overlay,
        harmonic_transfer_overlay,
        selective_harmonic_transfer_overlay,
        cell_signature_transfer_overlay,
        trajectory_transfer_overlay,
        task_hint_overlay,
        harmonic_transfer_ablation_min_accuracy,
        harmonic_transfer_ablation_min_drop,
        harmonic_transfer_ablation_max_drop,
        cell_signature_ablation_min_accuracy,
        cell_signature_ablation_min_drop,
        cell_signature_ablation_max_drop,
        trajectory_ablation_min_accuracy,
        trajectory_ablation_min_drop,
        trajectory_ablation_max_drop,
        selective_harmonic_best_threshold,
        selective_harmonic_best_accuracy,
        selective_harmonic_best_over_base,
        exact_overlay_applied,
        harmonic_transfer_applied,
        selective_harmonic_transfer_applied,
        cell_signature_transfer_applied,
        trajectory_transfer_applied,
        task_hint_overlay_applied,
        exact_over_base,
        harmonic_transfer_over_base,
        selective_harmonic_transfer_over_base,
        cell_signature_transfer_over_base,
        trajectory_transfer_over_base,
        task_hint_over_base,
        mode_status,
    }
}

/// Run one Chat-0 prompt through the prompt-cloud CarrierWave snapshot loop.
///
/// This is a small interactive contour over the same mechanism as
/// [`chat0_eval`]. It does not mutate weights directly; feedback is exposed as
/// trace data so learning can remain eval-gated.
#[must_use]
pub fn chat0_once(
    train_seed: u64,
    cases: usize,
    prompt: &[u8],
    expected_response: Option<&str>,
) -> Chat0Trace {
    chat0_once_with_state(train_seed, cases, prompt, expected_response, None)
}

/// Run one Chat-0 prompt with an optional eval-promoted feedback overlay.
#[must_use]
pub fn chat0_once_with_promoted_state(
    train_seed: u64,
    cases: usize,
    prompt: &[u8],
    expected_response: Option<&str>,
    promoted_state: &Chat0PromotedState,
) -> Chat0Trace {
    chat0_once_with_state(
        train_seed,
        cases,
        prompt,
        expected_response,
        Some(promoted_state),
    )
}

fn chat0_once_with_state(
    train_seed: u64,
    cases: usize,
    prompt: &[u8],
    expected_response: Option<&str>,
    promoted_state: Option<&Chat0PromotedState>,
) -> Chat0Trace {
    let bank = train_chat0_route_lock_bank(train_seed, cases);
    let mut snapshot_classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_diverse_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        snapshot_classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    snapshot_classifier.finish();

    let context = byte_context_trained_carrier_features(train_seed, 0, prompt, bank, None);
    let snapshot_target =
        snapshot_classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
    let bank_label = bank.predict_label(prompt, None);
    let predicted_target = bank_label
        .map(|label| BYTE_CONTEXT_TASKS[label].1)
        .unwrap_or(snapshot_target);
    let route = if bank_label.is_some() {
        "prompt_cloud_lock_bank"
    } else {
        "snapshot_classifier"
    };
    let promoted_target =
        promoted_state.and_then(|state| state.target_for_prompt(train_seed, cases, prompt));
    let (predicted_target, route, mode_status) = match promoted_target {
        Some(target) => (
            target,
            "promoted_feedback_state",
            "chat0_once_answered_promoted_state",
        ),
        None => (predicted_target, route, "chat0_once_answered_eval_gated"),
    };
    let response = chat0_response_for_target(predicted_target);
    let feedback_correct = expected_response.map(|expected| expected == response);

    Chat0Trace {
        train_seed,
        cases_per_split: cases,
        prompt: String::from_utf8_lossy(prompt).into_owned(),
        route,
        predicted_task: chat0_task_for_target(predicted_target),
        predicted_target,
        response,
        expected_response: expected_response.map(str::to_owned),
        feedback_correct,
        active_cell_ids: context.active_cell_ids,
        coherence: context.snapshot.coherence,
        spectral_entropy: context.snapshot.spectral_entropy,
        center_phase: context.snapshot.center_phase,
        center_magnitude: context.snapshot.center_magnitude,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        mode_status,
    }
}

#[allow(clippy::too_many_arguments)]
fn byte_context_prompt_carrier_ablation_eval_with_sample_fn(
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    sample_fn: impl Fn(u64, usize, bool) -> ByteContextSample + Copy,
    full_name: &'static str,
    all_disabled_name: &'static str,
    pass_status: &'static str,
    fail_status: &'static str,
) -> ByteContextPromptCarrierAblationReport {
    let bank =
        train_byte_context_prompt_carrier_lock_bank_with_sample_fn(train_seed, cases, sample_fn);
    let mut classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = sample_fn(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_trained_carrier_features(
            train_seed,
            case_index,
            &sample.prompt,
            bank,
            None,
        );
        classifier.observe(
            label,
            byte_snapshot_carrier_state_features(context.snapshot),
        );
    }
    classifier.finish();

    let full_snapshot = byte_context_trained_carrier_ablation_result_with_sample_fn(
        full_name,
        holdout_seed,
        cases,
        classifier,
        bank,
        None,
        sample_fn,
    );
    let empty_bank = TrainedCarrierLockBank::new(CarrierLockFeatureMode::PromptCloud);
    let all_disabled = byte_context_trained_carrier_ablation_result_with_sample_fn(
        all_disabled_name,
        holdout_seed,
        cases,
        classifier,
        empty_bank,
        None,
        sample_fn,
    );
    let ablations = std::array::from_fn(|task_index| {
        let name = BYTE_CONTEXT_TASKS[task_index].0;
        let result_name = match name {
            "ping" => "ablate_prompt_lock_ping",
            "name" => "ablate_prompt_lock_name",
            "time" => "ablate_prompt_lock_time",
            "help" => "ablate_prompt_lock_help",
            "echo" => "ablate_prompt_lock_echo",
            "save" => "ablate_prompt_lock_save",
            "open" => "ablate_prompt_lock_open",
            "close" => "ablate_prompt_lock_close",
            _ => "ablate_prompt_lock_unknown",
        };
        byte_context_trained_carrier_ablation_result_with_sample_fn(
            result_name,
            holdout_seed,
            cases,
            classifier,
            bank,
            Some(task_index),
            sample_fn,
        )
    });

    let min_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::INFINITY, f32::min);
    let max_accuracy_drop = ablations
        .iter()
        .map(|ablation| full_snapshot.accuracy - ablation.accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_error_increase = ablations
        .iter()
        .map(|ablation| ablation.mean_circular_error - full_snapshot.mean_circular_error)
        .fold(f32::NEG_INFINITY, f32::max);
    let accuracy_over_all_disabled = full_snapshot.accuracy - all_disabled.accuracy;
    let error_gain_over_all_disabled =
        all_disabled.mean_circular_error - full_snapshot.mean_circular_error;
    let mode_status = if full_snapshot.accuracy > 0.5
        && accuracy_over_all_disabled > 0.2
        && error_gain_over_all_disabled > 0.0
        && max_accuracy_drop > 0.05
    {
        pass_status
    } else {
        fail_status
    };

    ByteContextPromptCarrierAblationReport {
        train_seed,
        holdout_seed,
        cases_per_split: cases,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        full_snapshot,
        all_disabled,
        ablations,
        min_accuracy_drop,
        max_accuracy_drop,
        max_error_increase,
        accuracy_over_all_disabled,
        error_gain_over_all_disabled,
        mode_status,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ByteContextSample {
    prompt: Vec<u8>,
    target: u8,
}

fn random_task_target(seed: u64, case_index: usize) -> u8 {
    let index =
        splitmix64(seed ^ (case_index as u64).rotate_left(29)) as usize % BYTE_CONTEXT_TASKS.len();
    BYTE_CONTEXT_TASKS[index].1
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BytePhaseDecoder {
    weights: [f32; 6],
    learning_rate: f32,
}

impl BytePhaseDecoder {
    fn new(learning_rate: f32) -> Self {
        Self {
            weights: [0.0; 6],
            learning_rate,
        }
    }

    fn predict(self, features: [f32; 6]) -> u8 {
        let phase = self
            .weights
            .iter()
            .zip(features.iter())
            .map(|(weight, feature)| weight * feature)
            .sum::<f32>()
            .rem_euclid(std::f32::consts::TAU);
        wave_bus_predict(phase)
    }

    fn update(&mut self, prediction: u8, target: u8, features: [f32; 6]) {
        let predicted_phase = byte_to_phase(prediction);
        let target_phase = byte_to_phase(target);
        let phase_error = circular_delta(predicted_phase, target_phase);
        for (weight, feature) in self.weights.iter_mut().zip(features.iter()) {
            *weight += self.learning_rate * phase_error * feature;
            *weight = weight.clamp(-std::f32::consts::TAU, std::f32::consts::TAU);
        }
    }
}

const PROMOTED_HARMONIC_TRANSFER_MIN_MARGIN: f32 = 1.15;
const PROMOTED_HARMONIC_TRANSFER_MARGIN_SWEEP: [f32; 8] =
    [1.0, 1.05, 1.15, 1.35, 1.75, 2.50, 4.00, 8.00];
const CELL_SIGNATURE_FEATURES: usize = 12;
const TRAJECTORY_SIGNATURE_FEATURES: usize = 24;
#[derive(Debug, Clone, Copy, PartialEq)]
struct ByteCentroidPrediction {
    target: u8,
    margin: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ByteCentroidClassifier {
    centroids: [[f32; 6]; 8],
    counts: [usize; 8],
}

impl ByteCentroidClassifier {
    fn new() -> Self {
        Self {
            centroids: [[0.0; 6]; 8],
            counts: [0; 8],
        }
    }

    fn observe(&mut self, label: usize, features: [f32; 6]) {
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

    fn predict(self, features: [f32; 6]) -> u8 {
        self.predict_with_margin(features).target
    }

    fn predict_with_margin(self, features: [f32; 6]) -> ByteCentroidPrediction {
        let mut best_label = 0;
        let mut best_distance = f32::INFINITY;
        let mut second_distance = f32::INFINITY;

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
                second_distance = best_distance;
                best_label = label;
                best_distance = distance;
            } else if distance < second_distance {
                second_distance = distance;
            }
        }

        let margin = if best_distance.is_finite() && second_distance.is_finite() {
            second_distance / (best_distance + f32::EPSILON)
        } else {
            0.0
        };

        ByteCentroidPrediction {
            target: BYTE_CONTEXT_TASKS[best_label].1,
            margin,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CellSignatureClassifier {
    centroids: [[f32; CELL_SIGNATURE_FEATURES]; 8],
    counts: [usize; 8],
}

impl CellSignatureClassifier {
    fn new() -> Self {
        Self {
            centroids: [[0.0; CELL_SIGNATURE_FEATURES]; 8],
            counts: [0; 8],
        }
    }

    fn observe(&mut self, label: usize, features: [f32; CELL_SIGNATURE_FEATURES]) {
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

    fn predict(self, features: [f32; CELL_SIGNATURE_FEATURES]) -> u8 {
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

        BYTE_CONTEXT_TASKS[best_label].1
    }
}

fn cell_signature_features(context: &ByteContextFeatures) -> [f32; CELL_SIGNATURE_FEATURES] {
    let mut features = [0.0; CELL_SIGNATURE_FEATURES];
    features[0] = 1.0;
    for (rank, cell_id) in context.active_cell_ids.iter().copied().enumerate() {
        let slot = 1 + (cell_id as usize % STAGE2_ORGAN_CELLS);
        features[slot] += (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
    }
    features[7] = context.coherence.clamp(0.0, 1.0);
    features[8] = context.spectral_entropy.clamp(0.0, 1.0);
    features[9] = context.snapshot.center_phase.sin();
    features[10] = context.snapshot.center_phase.cos();
    features[11] = context.snapshot.center_magnitude.clamp(0.0, 1.0);
    features
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TrajectorySignatureClassifier {
    centroids: [[f32; TRAJECTORY_SIGNATURE_FEATURES]; 8],
    counts: [usize; 8],
}

impl TrajectorySignatureClassifier {
    fn new() -> Self {
        Self {
            centroids: [[0.0; TRAJECTORY_SIGNATURE_FEATURES]; 8],
            counts: [0; 8],
        }
    }

    fn observe(&mut self, label: usize, features: [f32; TRAJECTORY_SIGNATURE_FEATURES]) {
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

    fn predict(self, features: [f32; TRAJECTORY_SIGNATURE_FEATURES]) -> u8 {
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

        BYTE_CONTEXT_TASKS[best_label].1
    }
}

const TRAINED_LOCK_FEATURES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarrierLockFeatureMode {
    MiddleToken,
    PromptCloud,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TrainedCarrierLockBank {
    feature_mode: CarrierLockFeatureMode,
    centroids: [[f32; TRAINED_LOCK_FEATURES]; 8],
    counts: [usize; 8],
}

impl TrainedCarrierLockBank {
    fn new(feature_mode: CarrierLockFeatureMode) -> Self {
        Self {
            feature_mode,
            centroids: [[0.0; TRAINED_LOCK_FEATURES]; 8],
            counts: [0; 8],
        }
    }

    fn observe(&mut self, label: usize, prompt: &[u8]) {
        self.counts[label] += 1;
        let features = prompt_harmonic_lock_features(prompt, self.feature_mode);
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

    fn predict_label(self, prompt: &[u8], disabled_label: Option<usize>) -> Option<usize> {
        let features = prompt_harmonic_lock_features(prompt, self.feature_mode);
        let mut best_label = None;
        let mut best_distance = f32::INFINITY;

        for (label, centroid) in self.centroids.iter().enumerate() {
            if self.counts[label] == 0 || disabled_label == Some(label) {
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
                best_label = Some(label);
                best_distance = distance;
            }
        }

        best_label
    }
}

fn byte_context_sample(seed: u64, case_index: usize, train: bool) -> ByteContextSample {
    let task_index =
        (case_index + splitmix64(seed ^ case_index as u64) as usize) % BYTE_CONTEXT_TASKS.len();
    let (task, target) = BYTE_CONTEXT_TASKS[task_index];
    let variant = (splitmix64(seed ^ (case_index as u64).rotate_left(11)) & 3) as u8;
    let prompt = if train {
        format!("user:{}:{} -> bot: ", task, variant)
    } else {
        format!("cmd {} #{} answer: ", task, variant)
    };

    ByteContextSample {
        prompt: prompt.into_bytes(),
        target,
    }
}

fn byte_context_diverse_sample(seed: u64, case_index: usize, train: bool) -> ByteContextSample {
    const TRAIN_TEMPLATES: [&str; 4] = [
        "user:{task}:{variant} -> bot: ",
        "intent {task} variant {variant} reply: ",
        "please {task} mode {variant}; answer ",
        "request/{variant}/{task}/out ",
    ];
    const HOLDOUT_TEMPLATES: [&str; 4] = [
        "cmd {task} #{variant} answer: ",
        "do:{variant}:{task}:response ",
        "operator asks {task} [{variant}] => ",
        "route {variant} to {task}; say ",
    ];

    let task_index =
        (case_index + splitmix64(seed ^ case_index as u64) as usize) % BYTE_CONTEXT_TASKS.len();
    let (task, target) = BYTE_CONTEXT_TASKS[task_index];
    let variant = (splitmix64(seed ^ (case_index as u64).rotate_left(11)) & 7) as u8;
    let templates = if train {
        TRAIN_TEMPLATES
    } else {
        HOLDOUT_TEMPLATES
    };
    let template_index =
        (splitmix64(seed ^ (case_index as u64).rotate_left(17)) as usize) % templates.len();
    let prompt = templates[template_index]
        .replace("{task}", task)
        .replace("{variant}", &variant.to_string());

    ByteContextSample {
        prompt: prompt.into_bytes(),
        target,
    }
}

fn chat0_free_prompt_sample(seed: u64, case_index: usize) -> ByteContextSample {
    const FREE_TEMPLATES: [&str; 8] = [
        "cmd {task} #{variant} answer: ",
        "nando please {task} now / {variant} ",
        "manual:{variant}: {task}? ",
        "keyboard event says {task} then answer ",
        "short {task} request {variant} -> ",
        "operator wants {task}; reply {variant}: ",
        "local chat0 {task} sample {variant} ",
        "route free prompt to {task} [{variant}] ",
    ];

    let task_index =
        (case_index + splitmix64(seed ^ case_index as u64) as usize) % BYTE_CONTEXT_TASKS.len();
    let (task, target) = BYTE_CONTEXT_TASKS[task_index];
    let variant = (splitmix64(seed ^ (case_index as u64).rotate_left(23)) & 15) as u8;
    let template_index =
        (splitmix64(seed ^ (case_index as u64).rotate_left(31)) as usize) % FREE_TEMPLATES.len();
    let prompt = FREE_TEMPLATES[template_index]
        .replace("{task}", task)
        .replace("{variant}", &variant.to_string());

    ByteContextSample {
        prompt: prompt.into_bytes(),
        target,
    }
}

fn train_chat0_route_lock_bank(seed: u64, cases: usize) -> TrainedCarrierLockBank {
    let mut bank = TrainedCarrierLockBank::new(CarrierLockFeatureMode::PromptCloud);
    for case_index in 0..cases {
        let diverse = byte_context_diverse_sample(seed, case_index, true);
        bank.observe(byte_context_label_index(diverse.target), &diverse.prompt);
        let free = chat0_free_prompt_sample(seed, case_index);
        bank.observe(byte_context_label_index(free.target), &free.prompt);
    }
    bank.finish();
    bank
}

fn observe_chat0_route_training_sample(
    seed: u64,
    case_index: usize,
    sample: ByteContextSample,
    bank: TrainedCarrierLockBank,
    mono_classifier: &mut ByteCentroidClassifier,
    snapshot_classifier: &mut ByteCentroidClassifier,
) {
    let label = byte_context_label_index(sample.target);
    let context =
        byte_context_trained_carrier_features(seed, case_index, &sample.prompt, bank, None);
    mono_classifier.observe(
        label,
        mono_prompt_features(seed, case_index, &sample.prompt),
    );
    snapshot_classifier.observe(
        label,
        byte_snapshot_carrier_state_features(context.snapshot),
    );
}

fn byte_context_label_index(target: u8) -> usize {
    BYTE_CONTEXT_TASKS
        .iter()
        .position(|(_, label)| *label == target)
        .expect("byte-context target must be one of the fixed labels")
}

fn train_byte_context_carrier_lock_bank(seed: u64, cases: usize) -> TrainedCarrierLockBank {
    train_byte_context_carrier_lock_bank_with_mode(seed, cases, CarrierLockFeatureMode::MiddleToken)
}

fn train_byte_context_prompt_carrier_lock_bank(seed: u64, cases: usize) -> TrainedCarrierLockBank {
    train_byte_context_prompt_carrier_lock_bank_with_sample_fn(seed, cases, byte_context_sample)
}

fn train_byte_context_prompt_carrier_lock_bank_with_sample_fn(
    seed: u64,
    cases: usize,
    sample_fn: impl Fn(u64, usize, bool) -> ByteContextSample,
) -> TrainedCarrierLockBank {
    train_byte_context_carrier_lock_bank_with_mode_and_sample_fn(
        seed,
        cases,
        CarrierLockFeatureMode::PromptCloud,
        sample_fn,
    )
}

fn train_byte_context_carrier_lock_bank_with_mode(
    seed: u64,
    cases: usize,
    feature_mode: CarrierLockFeatureMode,
) -> TrainedCarrierLockBank {
    train_byte_context_carrier_lock_bank_with_mode_and_sample_fn(
        seed,
        cases,
        feature_mode,
        byte_context_sample,
    )
}

fn train_byte_context_carrier_lock_bank_with_mode_and_sample_fn(
    seed: u64,
    cases: usize,
    feature_mode: CarrierLockFeatureMode,
    sample_fn: impl Fn(u64, usize, bool) -> ByteContextSample,
) -> TrainedCarrierLockBank {
    let mut bank = TrainedCarrierLockBank::new(feature_mode);

    for case_index in 0..cases {
        let sample = sample_fn(seed, case_index, true);
        bank.observe(byte_context_label_index(sample.target), &sample.prompt);
    }
    bank.finish();
    bank
}

fn prompt_harmonic_lock_features(
    prompt: &[u8],
    mode: CarrierLockFeatureMode,
) -> [f32; TRAINED_LOCK_FEATURES] {
    match mode {
        CarrierLockFeatureMode::MiddleToken => prompt_middle_token_harmonic_features(prompt),
        CarrierLockFeatureMode::PromptCloud => prompt_cloud_harmonic_features(prompt),
    }
}

fn prompt_middle_token_harmonic_features(prompt: &[u8]) -> [f32; TRAINED_LOCK_FEATURES] {
    let mut tokens: Vec<&[u8]> = Vec::new();
    let mut token_start = None;

    for (index, byte) in prompt.iter().copied().enumerate() {
        if byte.is_ascii_alphabetic() {
            token_start.get_or_insert(index);
        } else if let Some(start) = token_start.take() {
            tokens.push(&prompt[start..index]);
        }
    }
    if let Some(start) = token_start {
        tokens.push(&prompt[start..]);
    }

    let token = tokens.get(tokens.len() / 2).copied().unwrap_or(prompt);
    let mut features = [0.0; TRAINED_LOCK_FEATURES];
    let mut letter_count = 0usize;

    for (index, byte) in token.iter().copied().enumerate() {
        if !byte.is_ascii_alphabetic() {
            continue;
        }
        let lower = byte.to_ascii_lowercase();
        let phase = byte_to_phase(lower);
        let slot = (index % 4) * 2;
        features[slot] += phase.sin();
        features[slot + 1] += phase.cos();
        let bucket = usize::from(lower.wrapping_sub(b'a')) % 8;
        features[8 + bucket] += 1.0;
        letter_count += 1;
    }

    if letter_count > 0 {
        let scale = letter_count as f32;
        for feature in &mut features {
            *feature /= scale;
        }
    }

    features
}

fn prompt_cloud_harmonic_features(prompt: &[u8]) -> [f32; TRAINED_LOCK_FEATURES] {
    let mut features = [0.0; TRAINED_LOCK_FEATURES];
    let mut letter_count = 0usize;
    let mut token_count = 0usize;
    let prompt_len = prompt.len().max(1) as f32;
    let mut token_hash = 0xcbf2_9ce4_8422_2325u64;
    let mut token_phase_sum = 0.0f32;
    let mut token_start = None;

    for (index, byte) in prompt.iter().copied().enumerate() {
        if byte.is_ascii_alphabetic() {
            token_start.get_or_insert(index);
        } else if let Some(start) = token_start.take() {
            let token_position_phase = ((start as f32 + 0.5) / prompt_len * std::f32::consts::TAU)
                .rem_euclid(std::f32::consts::TAU);
            prompt_cloud_observe_token(
                &mut features,
                token_hash,
                token_phase_sum,
                token_position_phase,
            );
            token_hash = 0xcbf2_9ce4_8422_2325u64;
            token_phase_sum = 0.0;
            token_count += 1;
            continue;
        } else {
            continue;
        }

        let lower = byte.to_ascii_lowercase();
        let byte_phase = byte_to_phase(lower);
        let position_phase = ((index as f32 + 0.5) / prompt_len * std::f32::consts::TAU)
            .rem_euclid(std::f32::consts::TAU);
        let bucket = usize::from(lower.wrapping_sub(b'a')) % 4;
        let harmonic_slot = bucket * 2;
        features[harmonic_slot] += byte_phase.sin();
        features[harmonic_slot + 1] += byte_phase.cos();
        features[8 + bucket * 2] += (byte_phase + position_phase).sin();
        features[9 + bucket * 2] += (byte_phase - position_phase).cos();
        token_hash ^= u64::from(lower);
        token_hash = token_hash.wrapping_mul(0x100_0000_01b3);
        token_phase_sum += byte_phase;
        letter_count += 1;
    }
    if let Some(start) = token_start {
        let token_position_phase = ((start as f32 + 0.5) / prompt_len * std::f32::consts::TAU)
            .rem_euclid(std::f32::consts::TAU);
        prompt_cloud_observe_token(
            &mut features,
            token_hash,
            token_phase_sum,
            token_position_phase,
        );
        token_count += 1;
    }

    if letter_count > 0 {
        let scale = letter_count as f32;
        for feature in &mut features[..16] {
            *feature /= scale;
        }
    }
    if token_count > 0 {
        let scale = token_count as f32;
        for feature in &mut features[16..] {
            *feature = (*feature / scale) * 0.5;
        }
    }

    features
}

fn prompt_cloud_observe_token(
    features: &mut [f32; TRAINED_LOCK_FEATURES],
    token_hash: u64,
    token_phase_sum: f32,
    token_position_phase: f32,
) {
    let mixed = splitmix64(token_hash);
    let token_phase = ((mixed as f32 / u64::MAX as f32) * std::f32::consts::TAU + token_phase_sum)
        .rem_euclid(std::f32::consts::TAU);
    let bucket = mixed as usize % 4;
    let harmonic_slot = 16 + bucket * 2;
    features[harmonic_slot] += token_phase.sin();
    features[harmonic_slot + 1] += token_phase.cos();
    features[24 + bucket * 2] += (token_phase + token_position_phase).sin();
    features[25 + bucket * 2] += (token_phase - token_position_phase).cos();
}

fn byte_context_centroid_masked_result(
    name: &'static str,
    train_seed: u64,
    holdout_seed: u64,
    cases: usize,
    mask: [bool; 6],
) -> BaselineResult {
    let mut classifier = ByteCentroidClassifier::new();

    for case_index in 0..cases {
        let sample = byte_context_sample(train_seed, case_index, true);
        let label = byte_context_label_index(sample.target);
        let context = byte_context_features(train_seed, case_index, &sample.prompt);
        classifier.observe(label, byte_snapshot_features_masked(context.snapshot, mask));
    }
    classifier.finish();

    let mut result = BaselineResult::new(name, cases);
    for case_index in 0..cases {
        let sample = byte_context_sample(holdout_seed, case_index, false);
        let context = byte_context_features(holdout_seed, case_index, &sample.prompt);
        let prediction = classifier.predict(byte_snapshot_features_masked(context.snapshot, mask));
        score_prediction(
            &mut result,
            prediction,
            sample.target,
            context.snapshot.coherence,
            context.snapshot.spectral_entropy,
        );
    }
    result.finish();
    result
}

fn byte_context_cellular_carrier_ablation_result(
    name: &'static str,
    holdout_seed: u64,
    cases: usize,
    classifier: ByteCentroidClassifier,
    disabled_task_index: Option<usize>,
) -> BaselineResult {
    let mut result = BaselineResult::new(name, cases);
    for case_index in 0..cases {
        let sample = byte_context_sample(holdout_seed, case_index, false);
        let context = byte_context_cellular_carrier_features_with_disabled(
            holdout_seed,
            case_index,
            &sample.prompt,
            disabled_task_index,
        );
        let prediction = classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
        score_prediction(
            &mut result,
            prediction,
            sample.target,
            context.snapshot.coherence,
            context.snapshot.spectral_entropy,
        );
    }
    result.finish();
    result
}

fn byte_context_trained_carrier_ablation_result(
    name: &'static str,
    holdout_seed: u64,
    cases: usize,
    classifier: ByteCentroidClassifier,
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
) -> BaselineResult {
    byte_context_trained_carrier_ablation_result_with_sample_fn(
        name,
        holdout_seed,
        cases,
        classifier,
        bank,
        disabled_task_index,
        byte_context_sample,
    )
}

fn byte_context_trained_carrier_ablation_result_with_sample_fn(
    name: &'static str,
    holdout_seed: u64,
    cases: usize,
    classifier: ByteCentroidClassifier,
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
    sample_fn: impl Fn(u64, usize, bool) -> ByteContextSample,
) -> BaselineResult {
    let mut result = BaselineResult::new(name, cases);
    for case_index in 0..cases {
        let sample = sample_fn(holdout_seed, case_index, false);
        let context = byte_context_trained_carrier_features(
            holdout_seed,
            case_index,
            &sample.prompt,
            bank,
            disabled_task_index,
        );
        let prediction = classifier.predict(byte_snapshot_carrier_state_features(context.snapshot));
        score_prediction(
            &mut result,
            prediction,
            sample.target,
            context.snapshot.coherence,
            context.snapshot.spectral_entropy,
        );
    }
    result.finish();
    result
}

fn train_byte_context_sample(
    sample: ByteContextSample,
    seed: u64,
    case_index: usize,
    mono_decoder: &mut BytePhaseDecoder,
    no_snapshot_decoder: &mut BytePhaseDecoder,
    snapshot_decoder: &mut BytePhaseDecoder,
    wrong_snapshot_decoder: &mut BytePhaseDecoder,
) {
    let context = byte_context_features(seed, case_index, &sample.prompt);
    let mono_features = mono_prompt_features(seed, case_index, &sample.prompt);
    let no_features = no_snapshot_byte_features(context.current_byte);
    let snapshot_features = byte_snapshot_features(context.snapshot);
    let wrong_features = byte_snapshot_features(context.wrong_snapshot);

    let mono_prediction = mono_decoder.predict(mono_features);
    let no_prediction = no_snapshot_decoder.predict(no_features);
    let snapshot_prediction = snapshot_decoder.predict(snapshot_features);
    let wrong_prediction = wrong_snapshot_decoder.predict(wrong_features);

    mono_decoder.update(mono_prediction, sample.target, mono_features);
    no_snapshot_decoder.update(no_prediction, sample.target, no_features);
    snapshot_decoder.update(snapshot_prediction, sample.target, snapshot_features);
    wrong_snapshot_decoder.update(wrong_prediction, sample.target, wrong_features);
}

#[allow(clippy::too_many_arguments)]
fn score_byte_context_sample(
    sample: ByteContextSample,
    seed: u64,
    case_index: usize,
    mono_decoder: &mut BytePhaseDecoder,
    no_snapshot_decoder: &mut BytePhaseDecoder,
    snapshot_decoder: &mut BytePhaseDecoder,
    wrong_snapshot_decoder: &mut BytePhaseDecoder,
    random: &mut BaselineResult,
    mono_result: &mut BaselineResult,
    no_snapshot_result: &mut BaselineResult,
    voting_result: &mut BaselineResult,
    snapshot_result: &mut BaselineResult,
    wrong_snapshot_result: &mut BaselineResult,
    corrupted_snapshot_result: &mut BaselineResult,
) {
    let context = byte_context_features(seed, case_index, &sample.prompt);
    let mono_features = mono_prompt_features(seed, case_index, &sample.prompt);
    let no_features = no_snapshot_byte_features(context.current_byte);
    let snapshot_features = byte_snapshot_features(context.snapshot);
    let wrong_features = byte_snapshot_features(context.wrong_snapshot);
    let corrupted_features = byte_snapshot_features(context.corrupted_snapshot);

    let random_prediction = random_predict(seed, case_index, context.current_byte);
    score_prediction(random, random_prediction, sample.target, 0.0, 1.0);

    let mono_prediction = mono_decoder.predict(mono_features);
    score_prediction(mono_result, mono_prediction, sample.target, 0.0, 1.0);

    let no_prediction = no_snapshot_decoder.predict(no_features);
    score_prediction(no_snapshot_result, no_prediction, sample.target, 0.0, 1.0);

    let voting_prediction = voting_predict(context.current_byte, context.active_cell_ids);
    score_prediction(
        voting_result,
        voting_prediction,
        sample.target,
        context.coherence * 0.75,
        context.spectral_entropy,
    );

    let snapshot_prediction = snapshot_decoder.predict(snapshot_features);
    score_prediction(
        snapshot_result,
        snapshot_prediction,
        sample.target,
        context.snapshot.coherence,
        context.snapshot.spectral_entropy,
    );

    let wrong_prediction = wrong_snapshot_decoder.predict(wrong_features);
    score_prediction(
        wrong_snapshot_result,
        wrong_prediction,
        sample.target,
        context.wrong_snapshot.coherence,
        context.wrong_snapshot.spectral_entropy,
    );

    let corrupted_prediction = snapshot_decoder.predict(corrupted_features);
    score_prediction(
        corrupted_snapshot_result,
        corrupted_prediction,
        sample.target,
        context.corrupted_snapshot.coherence,
        context.corrupted_snapshot.spectral_entropy,
    );

    mono_decoder.update(mono_prediction, sample.target, mono_features);
    no_snapshot_decoder.update(no_prediction, sample.target, no_features);
    snapshot_decoder.update(snapshot_prediction, sample.target, snapshot_features);
    wrong_snapshot_decoder.update(wrong_prediction, sample.target, wrong_features);
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ByteContextFeatures {
    current_byte: u8,
    active_cell_ids: [u32; STAGE2_TOP_K],
    coherence: f32,
    spectral_entropy: f32,
    snapshot: SpectrumSnapshot,
    wrong_snapshot: SpectrumSnapshot,
    corrupted_snapshot: SpectrumSnapshot,
}

fn byte_context_features(seed: u64, case_index: usize, prompt: &[u8]) -> ByteContextFeatures {
    let current_byte = *prompt.last().unwrap_or(&0);
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick = run_stage2_tick_with_carrier(seed, current_byte, carrier, None);

    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }

    let snapshot = snapshot_roundtrip(tick.snapshot);
    let wrong_prompt = byte_context_sample(seed ^ 0xB17E_C0DE_5EED, case_index + 5, false).prompt;
    let wrong = prompt_snapshot(seed ^ 0xB17E_C0DE_5EED, &wrong_prompt);
    let corrupted = build_corrupted_snapshot(snapshot);

    ByteContextFeatures {
        current_byte,
        active_cell_ids: tick.trace.active_cell_ids,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
        snapshot,
        wrong_snapshot: wrong,
        corrupted_snapshot: corrupted,
    }
}

fn byte_context_lexical_carrier_features(
    seed: u64,
    case_index: usize,
    prompt: &[u8],
) -> ByteContextFeatures {
    let current_byte = *prompt.last().unwrap_or(&0);
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick = run_stage2_tick_with_carrier(seed, current_byte, carrier, None);

    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = lexical_carrier_lock(carrier, prompt);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }

    let snapshot = snapshot_roundtrip(tick.snapshot);
    let wrong_prompt = byte_context_sample(seed ^ 0xB17E_C0DE_5EED, case_index + 5, false).prompt;
    let wrong = prompt_lexical_carrier_snapshot(seed ^ 0xB17E_C0DE_5EED, &wrong_prompt);
    let corrupted = build_corrupted_carrier_snapshot(snapshot);

    ByteContextFeatures {
        current_byte,
        active_cell_ids: tick.trace.active_cell_ids,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
        snapshot,
        wrong_snapshot: wrong,
        corrupted_snapshot: corrupted,
    }
}

fn byte_context_cellular_carrier_features(
    seed: u64,
    case_index: usize,
    prompt: &[u8],
) -> ByteContextFeatures {
    byte_context_cellular_carrier_features_with_disabled(seed, case_index, prompt, None)
}

fn byte_context_cellular_carrier_features_with_disabled(
    seed: u64,
    case_index: usize,
    prompt: &[u8],
    disabled_task_index: Option<usize>,
) -> ByteContextFeatures {
    let current_byte = *prompt.last().unwrap_or(&0);
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick = run_stage2_tick_with_carrier(seed, current_byte, carrier, None);

    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = cellular_carrier_lock(carrier, prompt, disabled_task_index);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }

    let snapshot = snapshot_roundtrip(tick.snapshot);
    let wrong_prompt = byte_context_sample(seed ^ 0xB17E_C0DE_5EED, case_index + 5, false).prompt;
    let wrong = prompt_cellular_carrier_snapshot(
        seed ^ 0xB17E_C0DE_5EED,
        &wrong_prompt,
        disabled_task_index,
    );
    let corrupted = build_corrupted_carrier_snapshot(snapshot);

    ByteContextFeatures {
        current_byte,
        active_cell_ids: tick.trace.active_cell_ids,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
        snapshot,
        wrong_snapshot: wrong,
        corrupted_snapshot: corrupted,
    }
}

fn byte_context_trained_carrier_features(
    seed: u64,
    case_index: usize,
    prompt: &[u8],
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
) -> ByteContextFeatures {
    let current_byte = *prompt.last().unwrap_or(&0);
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick = run_stage2_tick_with_carrier(seed, current_byte, carrier, None);

    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = trained_carrier_lock(carrier, prompt, bank, disabled_task_index);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }

    let snapshot = snapshot_roundtrip(tick.snapshot);
    let wrong_prompt = byte_context_sample(seed ^ 0xB17E_C0DE_5EED, case_index + 5, false).prompt;
    let wrong = prompt_trained_carrier_snapshot(
        seed ^ 0xB17E_C0DE_5EED,
        &wrong_prompt,
        bank,
        disabled_task_index,
    );
    let corrupted = build_corrupted_carrier_snapshot(snapshot);

    ByteContextFeatures {
        current_byte,
        active_cell_ids: tick.trace.active_cell_ids,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
        snapshot,
        wrong_snapshot: wrong,
        corrupted_snapshot: corrupted,
    }
}

fn trajectory_signature_features(
    seed: u64,
    prompt: &[u8],
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
) -> [f32; TRAJECTORY_SIGNATURE_FEATURES] {
    let mut features = [0.0; TRAJECTORY_SIGNATURE_FEATURES];
    features[0] = 1.0;

    let first_byte = prompt.first().copied().unwrap_or(0);
    let mut carrier = CarrierWave::from_seed(seed, first_byte);
    let mut previous_phase = None;
    let mut count = 0.0f32;
    let mut final_phase = 0.0f32;
    let mut final_delta = 0.0f32;
    let mut final_carrier_phase = carrier.phase;

    if prompt.is_empty() {
        let tick = run_stage2_tick_with_carrier(seed, 0, carrier, None);
        features[7] += tick.snapshot.center_phase.sin();
        features[8] += tick.snapshot.center_phase.cos();
        features[11] += carrier.phase.sin();
        features[12] += carrier.phase.cos();
        features[13] += tick.trace.coherence;
        features[14] += tick.trace.spectral_entropy;
        features[15] += tick.snapshot.center_magnitude;
        final_phase = tick.snapshot.center_phase;
        final_carrier_phase = carrier.phase;
        count = 1.0;
    } else {
        for (offset, byte) in prompt.iter().copied().enumerate() {
            carrier = carrier.advance(byte, 1);
            carrier = trained_carrier_lock(carrier, prompt, bank, disabled_task_index);
            let tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
            let phase = tick.snapshot.center_phase;
            let delta = previous_phase
                .map(|previous| circular_delta(previous, phase))
                .unwrap_or(0.0);

            for (rank, cell_id) in tick.trace.active_cell_ids.iter().copied().enumerate() {
                let slot = 1 + (cell_id as usize % STAGE2_ORGAN_CELLS);
                features[slot] += (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
            }
            features[7] += phase.sin();
            features[8] += phase.cos();
            features[9] += delta.sin();
            features[10] += delta.cos();
            features[11] += carrier.phase.sin();
            features[12] += carrier.phase.cos();
            features[13] += tick.trace.coherence;
            features[14] += tick.trace.spectral_entropy;
            features[15] += tick.snapshot.center_magnitude;

            previous_phase = Some(phase);
            final_phase = phase;
            final_delta = delta;
            final_carrier_phase = carrier.phase;
            count += 1.0;
        }
    }

    let scale = count.max(1.0);
    for slot in &mut features[1..=15] {
        *slot /= scale;
    }
    features[16] = final_phase.sin();
    features[17] = final_phase.cos();
    features[18] = final_delta.sin();
    features[19] = final_delta.cos();
    features[20] = final_carrier_phase.sin();
    features[21] = final_carrier_phase.cos();
    features[22] = (prompt.len() as f32 / 64.0).clamp(0.0, 1.0);
    features[23] = f32::from(prompt.last().copied().unwrap_or(0)) / 255.0;

    features
}

fn prompt_snapshot(seed: u64, prompt: &[u8]) -> SpectrumSnapshot {
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick =
        run_stage2_tick_with_carrier(seed, prompt.last().copied().unwrap_or(0), carrier, None);
    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }
    snapshot_roundtrip(tick.snapshot)
}

fn prompt_cellular_carrier_snapshot(
    seed: u64,
    prompt: &[u8],
    disabled_task_index: Option<usize>,
) -> SpectrumSnapshot {
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick =
        run_stage2_tick_with_carrier(seed, prompt.last().copied().unwrap_or(0), carrier, None);
    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = cellular_carrier_lock(carrier, prompt, disabled_task_index);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }
    snapshot_roundtrip(tick.snapshot)
}

fn prompt_trained_carrier_snapshot(
    seed: u64,
    prompt: &[u8],
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
) -> SpectrumSnapshot {
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick =
        run_stage2_tick_with_carrier(seed, prompt.last().copied().unwrap_or(0), carrier, None);
    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = trained_carrier_lock(carrier, prompt, bank, disabled_task_index);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }
    snapshot_roundtrip(tick.snapshot)
}

fn prompt_lexical_carrier_snapshot(seed: u64, prompt: &[u8]) -> SpectrumSnapshot {
    let mut carrier = CarrierWave::from_seed(seed, prompt.first().copied().unwrap_or(0));
    let mut tick =
        run_stage2_tick_with_carrier(seed, prompt.last().copied().unwrap_or(0), carrier, None);
    for (offset, byte) in prompt.iter().copied().enumerate() {
        carrier = carrier.advance(byte, 1);
        carrier = lexical_carrier_lock(carrier, prompt);
        tick = run_stage2_tick_with_carrier(seed + offset as u64, byte, carrier, None);
    }
    snapshot_roundtrip(tick.snapshot)
}

fn lexical_carrier_lock(mut carrier: CarrierWave, prompt: &[u8]) -> CarrierWave {
    if let Some(phase) = lexical_task_phase(prompt) {
        carrier.phase = phase;
        carrier.amplitude = (carrier.amplitude * 0.50 + 0.50).clamp(0.0, 1.0);
        carrier.frequency = 1.0 + phase / std::f32::consts::TAU;
    }
    carrier
}

fn cellular_carrier_lock(
    mut carrier: CarrierWave,
    prompt: &[u8],
    disabled_task_index: Option<usize>,
) -> CarrierWave {
    if let Some((task_index, phase)) = cellular_task_phase(prompt, disabled_task_index) {
        carrier.phase = phase;
        carrier.amplitude = (carrier.amplitude * 0.50
            + cellular_task_amplitude(prompt, task_index))
        .clamp(0.0, 1.0);
        carrier.frequency = 1.0 + phase / std::f32::consts::TAU;
    }
    carrier
}

fn trained_carrier_lock(
    mut carrier: CarrierWave,
    prompt: &[u8],
    bank: TrainedCarrierLockBank,
    disabled_task_index: Option<usize>,
) -> CarrierWave {
    if let Some(task_index) = bank.predict_label(prompt, disabled_task_index) {
        let phase = trained_task_phase(task_index);
        carrier.phase = phase;
        carrier.amplitude = (carrier.amplitude * 0.50 + 0.50).clamp(0.0, 1.0);
        carrier.frequency = 1.0 + phase / std::f32::consts::TAU;
    }
    carrier
}

fn trained_task_phase(task_index: usize) -> f32 {
    ((task_index as f32 + 0.5) / BYTE_CONTEXT_TASKS.len() as f32 * std::f32::consts::TAU)
        .rem_euclid(std::f32::consts::TAU)
}

fn cellular_task_phase(prompt: &[u8], disabled_task_index: Option<usize>) -> Option<(usize, f32)> {
    let mut best_index = None;
    let mut best_score = 0.0;

    for (task_index, (task, _)) in BYTE_CONTEXT_TASKS.iter().enumerate() {
        if disabled_task_index == Some(task_index) {
            continue;
        }
        let score = task_cell_resonance(prompt, task.as_bytes());
        if score > best_score {
            best_index = Some(task_index);
            best_score = score;
        }
    }

    best_index.map(|index| {
        let phase = ((index as f32 + 0.5) / BYTE_CONTEXT_TASKS.len() as f32
            * std::f32::consts::TAU)
            .rem_euclid(std::f32::consts::TAU);
        (index, phase)
    })
}

fn cellular_task_amplitude(prompt: &[u8], task_index: usize) -> f32 {
    let score = task_cell_resonance(prompt, BYTE_CONTEXT_TASKS[task_index].0.as_bytes());
    (0.30 + score * 0.70).clamp(0.0, 1.0)
}

fn task_cell_resonance(prompt: &[u8], task: &[u8]) -> f32 {
    if task.is_empty() || prompt.len() < task.len() {
        return 0.0;
    }

    prompt
        .windows(task.len())
        .map(|window| {
            let matches = window
                .iter()
                .zip(task.iter())
                .filter(|(left, right)| left.eq_ignore_ascii_case(right))
                .count();
            matches as f32 / task.len() as f32
        })
        .fold(0.0, f32::max)
}

fn lexical_task_phase(prompt: &[u8]) -> Option<f32> {
    BYTE_CONTEXT_TASKS
        .iter()
        .position(|(task, _)| contains_ascii_token(prompt, task.as_bytes()))
        .map(|index| {
            ((index as f32 + 0.5) / BYTE_CONTEXT_TASKS.len() as f32 * std::f32::consts::TAU)
                .rem_euclid(std::f32::consts::TAU)
        })
}

fn contains_ascii_token(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack.windows(needle.len()).any(|window| {
            window
                .iter()
                .zip(needle.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
}

fn byte_snapshot_features(snapshot: SpectrumSnapshot) -> [f32; 6] {
    [
        1.0,
        circular_delta(snapshot.carrier.phase, snapshot.center_phase) / std::f32::consts::TAU,
        snapshot.coherence.clamp(0.0, 1.0),
        snapshot.center_magnitude.clamp(0.0, 1.0),
        snapshot.top_phases[0].sin(),
        snapshot.top_phases[1].cos(),
    ]
}

fn byte_snapshot_features_masked(snapshot: SpectrumSnapshot, mask: [bool; 6]) -> [f32; 6] {
    let mut features = byte_snapshot_features(snapshot);
    for (feature, keep) in features.iter_mut().zip(mask.iter()) {
        if !keep {
            *feature = 0.0;
        }
    }
    features
}

fn byte_snapshot_relative_features(snapshot: SpectrumSnapshot) -> [f32; 6] {
    let carrier_center = circular_delta(snapshot.carrier.phase, snapshot.center_phase);
    let top0_center = circular_delta(snapshot.top_phases[0], snapshot.center_phase);
    let top1_carrier = circular_delta(snapshot.top_phases[1], snapshot.carrier.phase);
    [
        1.0,
        carrier_center / std::f32::consts::TAU,
        snapshot.coherence.clamp(0.0, 1.0),
        snapshot.center_magnitude.clamp(0.0, 1.0),
        top0_center.sin(),
        top1_carrier.cos(),
    ]
}

fn byte_snapshot_carrier_state_features(snapshot: SpectrumSnapshot) -> [f32; 6] {
    [
        1.0,
        snapshot.carrier.phase.sin(),
        snapshot.carrier.phase.cos(),
        0.0,
        0.0,
        0.0,
    ]
}

fn mono_prompt_features(seed: u64, case_index: usize, prompt: &[u8]) -> [f32; 6] {
    let mut hash = seed ^ case_index as u64;
    for byte in prompt {
        hash = splitmix64(hash ^ u64::from(*byte));
    }
    let bytes = hash.to_le_bytes();
    [
        1.0,
        f32::from(bytes[0]) / 255.0,
        f32::from(bytes[1]) / 255.0,
        f32::from(bytes[2]) / 255.0,
        f32::from(bytes[3]) / 255.0,
        f32::from(bytes[4]) / 255.0,
    ]
}

fn no_snapshot_byte_features(current_byte: u8) -> [f32; 6] {
    [1.0, f32::from(current_byte) / 255.0, 0.0, 0.0, 0.0, 0.0]
}

#[cfg(test)]
#[path = "byte_context_tests.rs"]
mod tests;
