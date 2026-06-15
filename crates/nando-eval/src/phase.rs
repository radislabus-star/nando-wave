use crate::{BaselineResult, circular_delta, splitmix64};
use nando_core::{
    CarrierWave, SNAPSHOT_V1_BYTES, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, SpectrumSnapshot,
    Stage2Organ, run_stage2_tick, run_stage2_tick_with_carrier, run_stage2_tick_with_disabled,
    run_stage2_tick_with_organ_carrier,
};

mod reports;
pub use reports::{
    BusTransferReport, CarrierControlReport, HorizonSweepRow, PeriodicEvalReport,
    PeriodicTaskConfig, PhaseCompositionConfig, PhaseCompositionReport, PhaseHoldoutReport,
    SeedSweepRow, SnapshotAdaptReport, SnapshotDecoderReport, SnapshotDynamicsReport,
    SnapshotKeyedReport, SnapshotKeyedTransitionReport, SnapshotMemoryReport,
    SnapshotMultiTickReport, SnapshotNoisyKeyedTransitionReport,
    SnapshotNoisyKeyedTransitionSeedSweepReport, SnapshotNoisyKeyedTransitionSweepReport,
    SnapshotTransitionReport,
};

/// Run the first periodic sequence eval.
#[must_use]
pub fn periodic_eval(config: PeriodicTaskConfig) -> PeriodicEvalReport {
    let mut random = BaselineResult::new("random", config.cases);
    let mut mono192 = BaselineResult::new("mono192", config.cases);
    let mut no_bus = BaselineResult::new("cell32_no_bus", config.cases);
    let mut voting = BaselineResult::new("cell32_voting", config.cases);
    let mut wave_bus = BaselineResult::new("cell32_wave_bus", config.cases);
    let mut ablations: [BaselineResult; STAGE2_ORGAN_CELLS] =
        std::array::from_fn(|cell_id| BaselineResult::new(ablation_name(cell_id), config.cases));

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.step, case_index);
        let target = periodic_byte(config.start, config.step, case_index + 1);
        let tick = run_stage2_tick(config.seed + case_index as u64, input);

        let random_prediction = random_predict(config.seed, case_index, input);
        score_prediction(&mut random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, input);
        score_prediction(&mut mono192, mono_prediction, target, 0.0, 1.0);

        let no_bus_prediction = no_bus_predict(input, tick.trace.active_cell_ids);
        score_prediction(
            &mut no_bus,
            no_bus_prediction,
            target,
            tick.trace.coherence * 0.5,
            tick.trace.spectral_entropy,
        );

        let voting_prediction = voting_predict(input, tick.trace.active_cell_ids);
        score_prediction(
            &mut voting,
            voting_prediction,
            target,
            tick.trace.coherence * 0.75,
            tick.trace.spectral_entropy,
        );

        let wave_prediction = wave_bus_predict(tick.trace.center_phase);
        score_prediction(
            &mut wave_bus,
            wave_prediction,
            target,
            tick.trace.coherence,
            tick.trace.spectral_entropy,
        );

        for (cell_id, ablation) in ablations.iter_mut().enumerate() {
            let ablated_tick = run_stage2_tick_with_disabled(
                config.seed + case_index as u64,
                input,
                Some(cell_id as u32),
            );
            let ablated_prediction = wave_bus_predict(ablated_tick.trace.center_phase);
            score_prediction(
                ablation,
                ablated_prediction,
                target,
                ablated_tick.trace.coherence,
                ablated_tick.trace.spectral_entropy,
            );
        }
    }

    random.finish();
    mono192.finish();
    no_bus.finish();
    voting.finish();
    wave_bus.finish();
    for ablation in &mut ablations {
        ablation.finish();
    }

    let (key_cell, weakest_ablation) = weakest_ablation(ablations);
    let ablation_drop = wave_bus.accuracy - weakest_ablation.accuracy;
    let best_baseline = best_baseline_name([random, mono192, no_bus, voting, wave_bus]);
    let mode_status = if wave_bus.accuracy > mono192.accuracy
        && wave_bus.accuracy > no_bus.accuracy
        && wave_bus.accuracy > voting.accuracy
        && ablation_drop > 0.0
    {
        "candidate_needs_stronger_task"
    } else {
        "not_found_stage_3_ablation"
    };

    PeriodicEvalReport {
        config,
        random,
        mono192,
        no_bus,
        voting,
        wave_bus,
        ablations,
        ablation_drop,
        key_cell,
        best_baseline,
        mode_status,
    }
}

/// Run a synthetic task whose target combines input phase and CarrierWave phase.
#[must_use]
pub fn phase_composition_eval(config: PhaseCompositionConfig) -> PhaseCompositionReport {
    let mut random = BaselineResult::new("random", config.cases);
    let mut mono192 = BaselineResult::new("mono192", config.cases);
    let mut no_bus = BaselineResult::new("cell32_no_bus", config.cases);
    let mut voting = BaselineResult::new("cell32_voting", config.cases);
    let mut wave_bus = BaselineResult::new("cell32_wave_bus", config.cases);
    let mut ablations: [BaselineResult; STAGE2_ORGAN_CELLS] =
        std::array::from_fn(|cell_id| BaselineResult::new(ablation_name(cell_id), config.cases));

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let phase_bias = periodic_byte(0, config.phase_step, case_index);
        let seed = config.seed + case_index as u64;
        let tick = run_stage2_tick(seed, input);
        let target = phase_composition_target(input, phase_bias, tick.carrier.phase);

        let random_prediction = random_predict(config.seed, case_index, input);
        score_prediction(&mut random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, input);
        score_prediction(&mut mono192, mono_prediction, target, 0.0, 1.0);

        let no_bus_prediction = no_bus_predict(input, tick.trace.active_cell_ids);
        score_prediction(
            &mut no_bus,
            no_bus_prediction,
            target,
            tick.trace.coherence * 0.5,
            tick.trace.spectral_entropy,
        );

        let voting_prediction = voting_predict(input, tick.trace.active_cell_ids);
        score_prediction(
            &mut voting,
            voting_prediction,
            target,
            tick.trace.coherence * 0.75,
            tick.trace.spectral_entropy,
        );

        let wave_prediction = phase_wave_predict(
            input,
            phase_bias,
            tick.carrier.phase,
            tick.trace.center_phase,
        );
        score_prediction(
            &mut wave_bus,
            wave_prediction,
            target,
            tick.trace.coherence,
            tick.trace.spectral_entropy,
        );

        for (cell_id, ablation) in ablations.iter_mut().enumerate() {
            let ablated_tick = run_stage2_tick_with_disabled(seed, input, Some(cell_id as u32));
            let ablated_prediction = phase_wave_predict(
                input,
                phase_bias,
                ablated_tick.carrier.phase,
                ablated_tick.trace.center_phase,
            );
            score_prediction(
                ablation,
                ablated_prediction,
                target,
                ablated_tick.trace.coherence,
                ablated_tick.trace.spectral_entropy,
            );
        }
    }

    random.finish();
    mono192.finish();
    no_bus.finish();
    voting.finish();
    wave_bus.finish();
    for ablation in &mut ablations {
        ablation.finish();
    }

    let (key_cell, weakest_ablation) = weakest_ablation(ablations);
    let ablation_drop = wave_bus.accuracy - weakest_ablation.accuracy;
    let best_baseline = best_baseline_name([random, mono192, no_bus, voting, wave_bus]);
    let mode_status = if wave_bus.accuracy > mono192.accuracy
        && wave_bus.accuracy > no_bus.accuracy
        && wave_bus.accuracy > voting.accuracy
        && ablation_drop > 0.0
    {
        "candidate_needs_holdout"
    } else {
        "not_found_phase_composition"
    };

    PhaseCompositionReport {
        config,
        random,
        mono192,
        no_bus,
        voting,
        wave_bus,
        ablations,
        ablation_drop,
        key_cell,
        best_baseline,
        mode_status,
    }
}

/// Check whether the phase-composition candidate survives a holdout split.
#[must_use]
pub fn phase_composition_holdout_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> PhaseHoldoutReport {
    let train_report = phase_composition_eval(train);
    let holdout_report = phase_composition_eval(holdout);
    let wave_advantage_train = wave_advantage(&train_report);
    let wave_advantage_holdout = wave_advantage(&holdout_report);
    let min_ablation_drop = train_report.ablation_drop.min(holdout_report.ablation_drop);

    let mode_status =
        if wave_advantage_train > 0.0 && wave_advantage_holdout > 0.0 && min_ablation_drop > 0.0 {
            "candidate_holdout_passed_needs_carrier_test"
        } else {
            "not_found_holdout_failed"
        };

    PhaseHoldoutReport {
        train: train_report,
        holdout: holdout_report,
        wave_advantage_train,
        wave_advantage_holdout,
        min_ablation_drop,
        mode_status,
    }
}

/// Check whether the candidate depends on the correct CarrierWave.
#[must_use]
pub fn carrier_control_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> CarrierControlReport {
    let total_cases = train.cases + holdout.cases;
    let mut correct_carrier = BaselineResult::new("correct_carrier_wave", total_cases);
    let mut no_carrier = BaselineResult::new("no_carrier_wave", total_cases);
    let mut wrong_carrier = BaselineResult::new("wrong_carrier_wave", total_cases);
    let mut corrupted_carrier = BaselineResult::new("corrupted_carrier_wave", total_cases);

    score_carrier_control_split(
        train,
        &mut correct_carrier,
        &mut no_carrier,
        &mut wrong_carrier,
        &mut corrupted_carrier,
    );
    score_carrier_control_split(
        holdout,
        &mut correct_carrier,
        &mut no_carrier,
        &mut wrong_carrier,
        &mut corrupted_carrier,
    );

    correct_carrier.finish();
    no_carrier.finish();
    wrong_carrier.finish();
    corrupted_carrier.finish();

    let correct_over_no = correct_carrier.accuracy - no_carrier.accuracy;
    let correct_over_wrong = correct_carrier.accuracy - wrong_carrier.accuracy;
    let correct_over_corrupted = correct_carrier.accuracy - corrupted_carrier.accuracy;
    let mode_status =
        if correct_carrier.accuracy > 0.0 && correct_over_no > 0.0 && correct_over_wrong > 0.0 {
            "carrier_control_passed_candidate_mode"
        } else {
            "not_found_carrier_control"
        };

    CarrierControlReport {
        train_config: train,
        holdout_config: holdout,
        correct_carrier,
        no_carrier,
        wrong_carrier,
        corrupted_carrier,
        correct_over_no,
        correct_over_wrong,
        correct_over_corrupted,
        mode_status,
    }
}

/// Probe whether WaveBus state can predict a one-step delayed wave target.
#[must_use]
pub fn bus_transfer_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> BusTransferReport {
    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_bus = BaselineResult::new("cell32_no_bus", total_cases);
    let mut voting = BaselineResult::new("cell32_voting", total_cases);
    let mut correct_carrier_bus = BaselineResult::new("correct_carrier_bus", total_cases);
    let mut no_carrier_bus = BaselineResult::new("no_carrier_bus", total_cases);
    let mut wrong_carrier_bus = BaselineResult::new("wrong_carrier_bus", total_cases);
    let mut corrupted_carrier_bus = BaselineResult::new("corrupted_carrier_bus", total_cases);
    let mut ablations: [BaselineResult; STAGE2_ORGAN_CELLS] =
        std::array::from_fn(|cell_id| BaselineResult::new(ablation_name(cell_id), total_cases));

    score_bus_transfer_split(
        train,
        &mut random,
        &mut mono192,
        &mut no_bus,
        &mut voting,
        &mut correct_carrier_bus,
        &mut no_carrier_bus,
        &mut wrong_carrier_bus,
        &mut corrupted_carrier_bus,
        &mut ablations,
    );
    score_bus_transfer_split(
        holdout,
        &mut random,
        &mut mono192,
        &mut no_bus,
        &mut voting,
        &mut correct_carrier_bus,
        &mut no_carrier_bus,
        &mut wrong_carrier_bus,
        &mut corrupted_carrier_bus,
        &mut ablations,
    );

    random.finish();
    mono192.finish();
    no_bus.finish();
    voting.finish();
    correct_carrier_bus.finish();
    no_carrier_bus.finish();
    wrong_carrier_bus.finish();
    corrupted_carrier_bus.finish();
    for ablation in &mut ablations {
        ablation.finish();
    }

    let (key_cell, weakest_ablation) = weakest_ablation(ablations);
    let ablation_drop = correct_carrier_bus.accuracy - weakest_ablation.accuracy;
    let best_baseline_accuracy = [random, mono192, no_bus, voting]
        .into_iter()
        .map(|result| result.accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let correct_over_best_baseline = correct_carrier_bus.accuracy - best_baseline_accuracy;
    let correct_over_wrong_carrier = correct_carrier_bus.accuracy - wrong_carrier_bus.accuracy;
    let mode_status = if correct_over_best_baseline > 0.0
        && correct_over_wrong_carrier > 0.0
        && ablation_drop > 0.0
    {
        "bus_transfer_passed_candidate_mode"
    } else {
        "not_found_bus_transfer"
    };

    BusTransferReport {
        train_config: train,
        holdout_config: holdout,
        random,
        mono192,
        no_bus,
        voting,
        correct_carrier_bus,
        no_carrier_bus,
        wrong_carrier_bus,
        corrupted_carrier_bus,
        ablations,
        ablation_drop,
        key_cell,
        correct_over_best_baseline,
        correct_over_wrong_carrier,
        mode_status,
    }
}

/// Check whether a compact serialized snapshot can replay wave state.
#[must_use]
pub fn snapshot_memory_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotMemoryReport {
    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot", total_cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot", total_cases);

    score_snapshot_memory_split(
        train,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_memory_split(
        holdout,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    warm_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let warm_over_no_snapshot = warm_snapshot.accuracy - no_snapshot.accuracy;
    let warm_over_wrong_snapshot = warm_snapshot.accuracy - wrong_snapshot.accuracy;
    let mode_status = if warm_snapshot.accuracy > 0.0
        && warm_over_no_snapshot > 0.0
        && warm_over_wrong_snapshot > 0.0
        && corrupted_snapshot.accuracy < warm_snapshot.accuracy
    {
        "snapshot_memory_passed_state_replay"
    } else {
        "not_found_snapshot_memory"
    };

    SnapshotMemoryReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192,
        no_snapshot,
        warm_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        warm_over_no_snapshot,
        warm_over_wrong_snapshot,
        mode_status,
    }
}

/// Check whether a snapshot helps estimate the next wave-state.
#[must_use]
pub fn snapshot_transition_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotTransitionReport {
    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_transition", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot_transition", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_transition", total_cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot_transition", total_cases);

    score_snapshot_transition_split(
        train,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_transition_split(
        holdout,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    warm_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let warm_over_no_snapshot = warm_snapshot.accuracy - no_snapshot.accuracy;
    let warm_over_wrong_snapshot = warm_snapshot.accuracy - wrong_snapshot.accuracy;
    let mode_status = if warm_snapshot.accuracy > 0.0
        && warm_over_no_snapshot > 0.0
        && warm_over_wrong_snapshot > 0.0
        && corrupted_snapshot.accuracy < warm_snapshot.accuracy
    {
        "snapshot_transition_passed"
    } else {
        "not_found_snapshot_transition"
    };

    SnapshotTransitionReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192,
        no_snapshot,
        warm_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        warm_over_no_snapshot,
        warm_over_wrong_snapshot,
        mode_status,
    }
}

/// Check whether snapshot state helps across a smooth carrier dynamics sequence.
#[must_use]
pub fn snapshot_dynamics_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotDynamicsReport {
    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_dynamics", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot_dynamics", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_dynamics", total_cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot_dynamics", total_cases);

    score_snapshot_dynamics_split(
        train,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_dynamics_split(
        holdout,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    warm_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let warm_error_gain_over_no =
        no_snapshot.mean_circular_error - warm_snapshot.mean_circular_error;
    let warm_error_gain_over_wrong =
        wrong_snapshot.mean_circular_error - warm_snapshot.mean_circular_error;
    let mode_status = if warm_error_gain_over_no > 0.0
        && warm_error_gain_over_wrong > 0.0
        && warm_snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_dynamics_passed"
    } else {
        "not_found_snapshot_dynamics"
    };

    SnapshotDynamicsReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        random,
        mono192,
        no_snapshot,
        warm_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        warm_error_gain_over_no,
        warm_error_gain_over_wrong,
        mode_status,
    }
}

/// Check whether snapshot state survives more than one smooth carrier tick.
#[must_use]
pub fn snapshot_multitick_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotMultiTickReport {
    const HORIZON: usize = 4;

    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_multitick", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot_multitick", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_multitick", total_cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot_multitick", total_cases);

    score_snapshot_multitick_split(
        train,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_multitick_split(
        holdout,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    warm_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let warm_error_gain_over_no =
        no_snapshot.mean_circular_error - warm_snapshot.mean_circular_error;
    let warm_error_gain_over_wrong =
        wrong_snapshot.mean_circular_error - warm_snapshot.mean_circular_error;
    let mode_status = if warm_error_gain_over_no > 0.0
        && warm_error_gain_over_wrong > 0.0
        && warm_snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_multitick_passed"
    } else {
        "not_found_snapshot_multitick"
    };

    SnapshotMultiTickReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon: HORIZON,
        random,
        mono192,
        no_snapshot,
        warm_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        warm_error_gain_over_no,
        warm_error_gain_over_wrong,
        mode_status,
    }
}

/// Check whether feedback can turn snapshot phase advantage into better prediction.
#[must_use]
pub fn snapshot_adapt_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotAdaptReport {
    const HORIZON: usize = 4;
    const LEARNING_RATE: f32 = 0.35;

    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_adapt_control", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot_no_adapt", total_cases);
    let mut adapted_no_snapshot = BaselineResult::new("adapted_no_snapshot", total_cases);
    let mut adapted_snapshot = BaselineResult::new("adapted_snapshot", total_cases);
    let mut adapted_wrong_snapshot = BaselineResult::new("adapted_wrong_snapshot", total_cases);
    let mut corrupted_snapshot =
        BaselineResult::new("corrupted_snapshot_adapt_control", total_cases);

    score_snapshot_adapt_split(
        train,
        HORIZON,
        LEARNING_RATE,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut adapted_no_snapshot,
        &mut adapted_snapshot,
        &mut adapted_wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_adapt_split(
        holdout,
        HORIZON,
        LEARNING_RATE,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut warm_snapshot,
        &mut adapted_no_snapshot,
        &mut adapted_snapshot,
        &mut adapted_wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    warm_snapshot.finish();
    adapted_no_snapshot.finish();
    adapted_snapshot.finish();
    adapted_wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let adapted_error_gain_over_warm =
        warm_snapshot.mean_circular_error - adapted_snapshot.mean_circular_error;
    let adapted_error_gain_over_no_adapt =
        adapted_no_snapshot.mean_circular_error - adapted_snapshot.mean_circular_error;
    let adapted_error_gain_over_wrong_adapt =
        adapted_wrong_snapshot.mean_circular_error - adapted_snapshot.mean_circular_error;
    let mode_status = if adapted_error_gain_over_warm > 0.0
        && adapted_error_gain_over_no_adapt > 0.0
        && adapted_error_gain_over_wrong_adapt > 0.0
        && adapted_snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_adapt_passed"
    } else {
        "not_found_snapshot_adapt"
    };

    SnapshotAdaptReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon: HORIZON,
        learning_rate: LEARNING_RATE,
        random,
        mono192,
        no_snapshot,
        warm_snapshot,
        adapted_no_snapshot,
        adapted_snapshot,
        adapted_wrong_snapshot,
        corrupted_snapshot,
        adapted_error_gain_over_warm,
        adapted_error_gain_over_no_adapt,
        adapted_error_gain_over_wrong_adapt,
        mode_status,
    }
}

/// Check whether a tiny online decoder can use snapshot features better than controls.
#[must_use]
pub fn snapshot_decoder_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotDecoderReport {
    const HORIZON: usize = 4;
    const LEARNING_RATE: f32 = 0.22;

    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut warm_snapshot = BaselineResult::new("warm_snapshot_decoder_control", total_cases);
    let mut decoder_no_snapshot = BaselineResult::new("decoder_no_snapshot", total_cases);
    let mut decoder_snapshot = BaselineResult::new("decoder_snapshot", total_cases);
    let mut decoder_wrong_snapshot = BaselineResult::new("decoder_wrong_snapshot", total_cases);
    let mut corrupted_snapshot =
        BaselineResult::new("corrupted_snapshot_decoder_control", total_cases);

    score_snapshot_decoder_split(
        train,
        HORIZON,
        LEARNING_RATE,
        &mut random,
        &mut mono192,
        &mut warm_snapshot,
        &mut decoder_no_snapshot,
        &mut decoder_snapshot,
        &mut decoder_wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_decoder_split(
        holdout,
        HORIZON,
        LEARNING_RATE,
        &mut random,
        &mut mono192,
        &mut warm_snapshot,
        &mut decoder_no_snapshot,
        &mut decoder_snapshot,
        &mut decoder_wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    warm_snapshot.finish();
    decoder_no_snapshot.finish();
    decoder_snapshot.finish();
    decoder_wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let decoder_error_gain_over_warm =
        warm_snapshot.mean_circular_error - decoder_snapshot.mean_circular_error;
    let decoder_error_gain_over_no_decoder =
        decoder_no_snapshot.mean_circular_error - decoder_snapshot.mean_circular_error;
    let decoder_error_gain_over_wrong_decoder =
        decoder_wrong_snapshot.mean_circular_error - decoder_snapshot.mean_circular_error;
    let mode_status = if decoder_error_gain_over_warm > 0.0
        && decoder_error_gain_over_no_decoder > 0.0
        && decoder_error_gain_over_wrong_decoder > 0.0
        && decoder_snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_decoder_passed"
    } else {
        "not_found_snapshot_decoder"
    };

    SnapshotDecoderReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon: HORIZON,
        learning_rate: LEARNING_RATE,
        random,
        mono192,
        warm_snapshot,
        decoder_no_snapshot,
        decoder_snapshot,
        decoder_wrong_snapshot,
        corrupted_snapshot,
        decoder_error_gain_over_warm,
        decoder_error_gain_over_no_decoder,
        decoder_error_gain_over_wrong_decoder,
        mode_status,
    }
}

/// Check that snapshot carries private state no cold/no-snapshot control can replace.
#[must_use]
pub fn snapshot_keyed_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotKeyedReport {
    const HORIZON: usize = 4;

    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut no_snapshot = BaselineResult::new("no_snapshot_keyed", total_cases);
    let mut keyed_snapshot = BaselineResult::new("keyed_snapshot", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_keyed", total_cases);
    let mut corrupted_snapshot = BaselineResult::new("corrupted_snapshot_keyed", total_cases);

    score_snapshot_keyed_split(
        train,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut keyed_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_keyed_split(
        holdout,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut no_snapshot,
        &mut keyed_snapshot,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    no_snapshot.finish();
    keyed_snapshot.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let keyed_over_no_snapshot = keyed_snapshot.accuracy - no_snapshot.accuracy;
    let keyed_over_wrong_snapshot = keyed_snapshot.accuracy - wrong_snapshot.accuracy;
    let keyed_error_gain_over_no =
        no_snapshot.mean_circular_error - keyed_snapshot.mean_circular_error;
    let mode_status = if keyed_over_no_snapshot > 0.0
        && keyed_over_wrong_snapshot > 0.0
        && keyed_error_gain_over_no > 0.0
        && keyed_snapshot.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_keyed_passed"
    } else {
        "not_found_snapshot_keyed"
    };

    SnapshotKeyedReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon: HORIZON,
        random,
        mono192,
        no_snapshot,
        keyed_snapshot,
        wrong_snapshot,
        corrupted_snapshot,
        keyed_over_no_snapshot,
        keyed_over_wrong_snapshot,
        keyed_error_gain_over_no,
        mode_status,
    }
}

/// Check whether snapshot-private state can participate in a transition target.
#[must_use]
pub fn snapshot_keyed_transition_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotKeyedTransitionReport {
    const HORIZON: usize = 4;

    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut future_only = BaselineResult::new("future_only_transition", total_cases);
    let mut keyed_transition = BaselineResult::new("keyed_transition", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_keyed_transition", total_cases);
    let mut corrupted_snapshot =
        BaselineResult::new("corrupted_snapshot_keyed_transition", total_cases);

    score_snapshot_keyed_transition_split(
        train,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut future_only,
        &mut keyed_transition,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_keyed_transition_split(
        holdout,
        HORIZON,
        &mut random,
        &mut mono192,
        &mut future_only,
        &mut keyed_transition,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    future_only.finish();
    keyed_transition.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let keyed_over_future_only = keyed_transition.accuracy - future_only.accuracy;
    let keyed_over_wrong_snapshot = keyed_transition.accuracy - wrong_snapshot.accuracy;
    let keyed_error_gain_over_future_only =
        future_only.mean_circular_error - keyed_transition.mean_circular_error;
    let mode_status = if keyed_over_future_only > 0.0
        && keyed_over_wrong_snapshot > 0.0
        && keyed_error_gain_over_future_only > 0.0
        && keyed_transition.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_keyed_transition_passed"
    } else {
        "not_found_snapshot_keyed_transition"
    };

    SnapshotKeyedTransitionReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon: HORIZON,
        random,
        mono192,
        future_only,
        keyed_transition,
        wrong_snapshot,
        corrupted_snapshot,
        keyed_over_future_only,
        keyed_over_wrong_snapshot,
        keyed_error_gain_over_future_only,
        mode_status,
    }
}

/// Check whether snapshot-private state helps when the target has hidden modulation.
#[must_use]
pub fn snapshot_noisy_keyed_transition_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotNoisyKeyedTransitionReport {
    const HORIZON: usize = 4;
    snapshot_noisy_keyed_transition_eval_for_horizon(train, holdout, HORIZON)
}

/// Sweep noisy keyed transition over several horizons.
#[must_use]
pub fn snapshot_noisy_keyed_transition_sweep_eval(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
) -> SnapshotNoisyKeyedTransitionSweepReport {
    const HORIZONS: [usize; 4] = [1, 2, 4, 8];

    let rows = HORIZONS.map(|horizon| {
        let report = snapshot_noisy_keyed_transition_eval_for_horizon(train, holdout, horizon);
        HorizonSweepRow {
            horizon,
            future_only_accuracy: report.future_only.accuracy,
            keyed_accuracy: report.keyed_transition.accuracy,
            wrong_accuracy: report.wrong_snapshot.accuracy,
            corrupted_accuracy: report.corrupted_snapshot.accuracy,
            keyed_accuracy_over_future_only: report.keyed_accuracy_over_future_only,
            keyed_error_gain_over_future_only: report.keyed_error_gain_over_future_only,
            keyed_error_gain_over_wrong_snapshot: report.keyed_error_gain_over_wrong_snapshot,
            passed: report.mode_status == "snapshot_noisy_keyed_transition_passed",
        }
    });

    let passed_count = rows.iter().filter(|row| row.passed).count();
    let min_keyed_accuracy_over_future_only = rows
        .iter()
        .map(|row| row.keyed_accuracy_over_future_only)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_future_only = rows
        .iter()
        .map(|row| row.keyed_error_gain_over_future_only)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.keyed_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_count == HORIZONS.len()
        && min_keyed_accuracy_over_future_only > 0.0
        && min_error_gain_over_future_only > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "snapshot_noisy_keyed_transition_sweep_passed"
    } else {
        "not_found_snapshot_noisy_keyed_transition_sweep"
    };

    SnapshotNoisyKeyedTransitionSweepReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_count,
        min_keyed_accuracy_over_future_only,
        min_error_gain_over_future_only,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

/// Sweep noisy keyed transition over fixed seed pairs and horizons.
#[must_use]
pub fn snapshot_noisy_keyed_transition_seed_sweep_eval(
    cases_per_split: usize,
) -> SnapshotNoisyKeyedTransitionSeedSweepReport {
    const SEED_PAIRS: [(u64, u64); 4] = [(13, 97), (17, 101), (29, 131), (43, 173)];

    let rows = SEED_PAIRS.map(|(train_seed, holdout_seed)| {
        let train = PhaseCompositionConfig {
            seed: train_seed,
            cases: cases_per_split,
            start: 19,
            input_step: 23,
            phase_step: 5,
        };
        let holdout = PhaseCompositionConfig {
            seed: holdout_seed,
            cases: cases_per_split,
            start: 31,
            input_step: 29,
            phase_step: 7,
        };
        let report = snapshot_noisy_keyed_transition_sweep_eval(train, holdout);
        SeedSweepRow {
            train_seed,
            holdout_seed,
            passed_count: report.passed_count,
            min_keyed_accuracy_over_future_only: report.min_keyed_accuracy_over_future_only,
            min_error_gain_over_future_only: report.min_error_gain_over_future_only,
            min_error_gain_over_wrong_snapshot: report.min_error_gain_over_wrong_snapshot,
            passed: report.mode_status == "snapshot_noisy_keyed_transition_sweep_passed",
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.passed).count();
    let min_keyed_accuracy_over_future_only = rows
        .iter()
        .map(|row| row.min_keyed_accuracy_over_future_only)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_future_only = rows
        .iter()
        .map(|row| row.min_error_gain_over_future_only)
        .fold(f32::INFINITY, f32::min);
    let min_error_gain_over_wrong_snapshot = rows
        .iter()
        .map(|row| row.min_error_gain_over_wrong_snapshot)
        .fold(f32::INFINITY, f32::min);
    let mode_status = if passed_seed_pairs == SEED_PAIRS.len()
        && min_keyed_accuracy_over_future_only > 0.0
        && min_error_gain_over_future_only > 0.0
        && min_error_gain_over_wrong_snapshot > 0.0
    {
        "snapshot_noisy_keyed_transition_seed_sweep_passed"
    } else {
        "not_found_snapshot_noisy_keyed_transition_seed_sweep"
    };

    SnapshotNoisyKeyedTransitionSeedSweepReport {
        cases_per_split,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        rows,
        passed_seed_pairs,
        min_keyed_accuracy_over_future_only,
        min_error_gain_over_future_only,
        min_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

fn snapshot_noisy_keyed_transition_eval_for_horizon(
    train: PhaseCompositionConfig,
    holdout: PhaseCompositionConfig,
    horizon: usize,
) -> SnapshotNoisyKeyedTransitionReport {
    let total_cases = train.cases + holdout.cases;
    let mut random = BaselineResult::new("random", total_cases);
    let mut mono192 = BaselineResult::new("mono192", total_cases);
    let mut future_only = BaselineResult::new("future_only_noisy_transition", total_cases);
    let mut keyed_transition = BaselineResult::new("keyed_noisy_transition", total_cases);
    let mut wrong_snapshot = BaselineResult::new("wrong_snapshot_noisy_transition", total_cases);
    let mut corrupted_snapshot =
        BaselineResult::new("corrupted_snapshot_noisy_transition", total_cases);

    score_snapshot_noisy_keyed_transition_split(
        train,
        horizon,
        &mut random,
        &mut mono192,
        &mut future_only,
        &mut keyed_transition,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );
    score_snapshot_noisy_keyed_transition_split(
        holdout,
        horizon,
        &mut random,
        &mut mono192,
        &mut future_only,
        &mut keyed_transition,
        &mut wrong_snapshot,
        &mut corrupted_snapshot,
    );

    random.finish();
    mono192.finish();
    future_only.finish();
    keyed_transition.finish();
    wrong_snapshot.finish();
    corrupted_snapshot.finish();

    let keyed_accuracy_over_future_only = keyed_transition.accuracy - future_only.accuracy;
    let keyed_error_gain_over_future_only =
        future_only.mean_circular_error - keyed_transition.mean_circular_error;
    let keyed_error_gain_over_wrong_snapshot =
        wrong_snapshot.mean_circular_error - keyed_transition.mean_circular_error;
    let mode_status = if keyed_accuracy_over_future_only >= 0.0
        && keyed_error_gain_over_future_only > 0.0
        && keyed_error_gain_over_wrong_snapshot > 0.0
        && keyed_transition.mean_circular_error < corrupted_snapshot.mean_circular_error
    {
        "snapshot_noisy_keyed_transition_passed"
    } else {
        "not_found_snapshot_noisy_keyed_transition"
    };

    SnapshotNoisyKeyedTransitionReport {
        train_config: train,
        holdout_config: holdout,
        snapshot_bytes: SNAPSHOT_V1_BYTES,
        horizon,
        random,
        mono192,
        future_only,
        keyed_transition,
        wrong_snapshot,
        corrupted_snapshot,
        keyed_accuracy_over_future_only,
        keyed_error_gain_over_future_only,
        keyed_error_gain_over_wrong_snapshot,
        mode_status,
    }
}

fn periodic_byte(start: u8, step: u8, index: usize) -> u8 {
    start.wrapping_add(step.wrapping_mul(index as u8))
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_noisy_keyed_transition_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    future_only: &mut BaselineResult,
    keyed_transition: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let future_tick =
            run_stage2_tick_with_organ_carrier(&organ, target_input, future_carrier, None);
        let target_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_noisy_private_phase(current_snapshot),
            current_snapshot.coherence,
        );
        let target = bus_transfer_target(target_input, target_phase_bias, target_phase);

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let future_prediction = bus_transfer_predict(
            target_input,
            target_phase_bias,
            future_tick.trace.center_phase,
        );
        score_prediction(
            future_only,
            future_prediction,
            target,
            future_tick.trace.coherence,
            future_tick.trace.spectral_entropy,
        );

        let keyed_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(current_snapshot),
            current_snapshot.coherence,
        );
        let keyed_prediction = bus_transfer_predict(target_input, target_phase_bias, keyed_phase);
        score_prediction(
            keyed_transition,
            keyed_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(wrong),
            wrong.coherence,
        );
        let wrong_prediction = bus_transfer_predict(target_input, target_phase_bias, wrong_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(corrupted),
            corrupted.coherence,
        );
        let corrupted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_keyed_transition_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    future_only: &mut BaselineResult,
    keyed_transition: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let future_tick =
            run_stage2_tick_with_organ_carrier(&organ, target_input, future_carrier, None);
        let target_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(current_snapshot),
            current_snapshot.coherence,
        );
        let target = bus_transfer_target(target_input, target_phase_bias, target_phase);

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let future_prediction = bus_transfer_predict(
            target_input,
            target_phase_bias,
            future_tick.trace.center_phase,
        );
        score_prediction(
            future_only,
            future_prediction,
            target,
            future_tick.trace.coherence,
            future_tick.trace.spectral_entropy,
        );

        let keyed_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(current_snapshot),
            current_snapshot.coherence,
        );
        let keyed_prediction = bus_transfer_predict(target_input, target_phase_bias, keyed_phase);
        score_prediction(
            keyed_transition,
            keyed_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(wrong),
            wrong.coherence,
        );
        let wrong_prediction = bus_transfer_predict(target_input, target_phase_bias, wrong_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = combine_transition_phase(
            future_tick.trace.center_phase,
            snapshot_private_phase(corrupted),
            corrupted.coherence,
        );
        let corrupted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_keyed_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    keyed_snapshot: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let target_phase = snapshot_private_phase(current_snapshot);
        let target = bus_transfer_target(target_input, target_phase_bias, target_phase);

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_prediction =
            bus_transfer_predict(target_input, target_phase_bias, future_carrier.phase);
        score_prediction(no_snapshot, no_prediction, target, 0.0, 1.0);

        let keyed_phase = snapshot_private_phase(current_snapshot);
        let keyed_prediction = bus_transfer_predict(target_input, target_phase_bias, keyed_phase);
        score_prediction(
            keyed_snapshot,
            keyed_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_prediction = bus_transfer_predict(
            target_input,
            target_phase_bias,
            snapshot_private_phase(wrong),
        );
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_prediction = bus_transfer_predict(
            target_input,
            target_phase_bias,
            snapshot_private_phase(corrupted),
        );
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_decoder_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    learning_rate: f32,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    decoder_no_snapshot: &mut BaselineResult,
    decoder_snapshot: &mut BaselineResult,
    decoder_wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);
    let mut no_decoder = OnlinePhaseDecoder::new(learning_rate);
    let mut snapshot_decoder = OnlinePhaseDecoder::new(learning_rate);
    let mut wrong_decoder = OnlinePhaseDecoder::new(learning_rate);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let target_tick =
            run_stage2_tick_with_organ_carrier(&organ, target_input, future_carrier, None);
        let target_phase = target_tick.trace.center_phase;
        let target = bus_transfer_target(target_input, target_phase_bias, target_phase);

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let warm_phase = transition_center_phase(current_snapshot, future_carrier);
        let warm_prediction = bus_transfer_predict(target_input, target_phase_bias, warm_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let no_features = no_snapshot_decoder_features(future_carrier);
        let no_phase = no_decoder.predict(future_carrier.phase, no_features);
        let no_prediction = bus_transfer_predict(target_input, target_phase_bias, no_phase);
        score_prediction(decoder_no_snapshot, no_prediction, target, 0.0, 1.0);

        let snapshot_features = snapshot_decoder_features(current_snapshot);
        let decoded_phase = snapshot_decoder.predict(future_carrier.phase, snapshot_features);
        let decoded_prediction =
            bus_transfer_predict(target_input, target_phase_bias, decoded_phase);
        score_prediction(
            decoder_snapshot,
            decoded_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_features = snapshot_decoder_features(wrong);
        let wrong_phase = wrong_decoder.predict(future_carrier.phase, wrong_features);
        let wrong_prediction = bus_transfer_predict(target_input, target_phase_bias, wrong_phase);
        score_prediction(
            decoder_wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = transition_center_phase(corrupted, future_carrier);
        let corrupted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        no_decoder.update(no_phase, target_phase, no_features);
        snapshot_decoder.update(decoded_phase, target_phase, snapshot_features);
        wrong_decoder.update(wrong_phase, target_phase, wrong_features);

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_adapt_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    learning_rate: f32,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    adapted_no_snapshot: &mut BaselineResult,
    adapted_snapshot: &mut BaselineResult,
    adapted_wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);
    let mut no_snapshot_correction = 0.0;
    let mut snapshot_correction = 0.0;
    let mut wrong_snapshot_correction = 0.0;

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let target_tick =
            run_stage2_tick_with_organ_carrier(&organ, target_input, future_carrier, None);
        let target_phase = target_tick.trace.center_phase;
        let target = bus_transfer_target(target_input, target_phase_bias, target_phase);

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_phase = future_carrier.phase;
        let no_prediction = bus_transfer_predict(target_input, target_phase_bias, no_phase);
        score_prediction(no_snapshot, no_prediction, target, 0.0, 1.0);

        let warm_phase = transition_center_phase(current_snapshot, future_carrier);
        let warm_prediction = bus_transfer_predict(target_input, target_phase_bias, warm_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let adapted_no_phase =
            (no_phase + no_snapshot_correction).rem_euclid(std::f32::consts::TAU);
        let adapted_no_prediction =
            bus_transfer_predict(target_input, target_phase_bias, adapted_no_phase);
        score_prediction(adapted_no_snapshot, adapted_no_prediction, target, 0.0, 1.0);

        let adapted_phase = (warm_phase + snapshot_correction).rem_euclid(std::f32::consts::TAU);
        let adapted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, adapted_phase);
        score_prediction(
            adapted_snapshot,
            adapted_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = transition_center_phase(wrong, future_carrier);
        let adapted_wrong_phase =
            (wrong_phase + wrong_snapshot_correction).rem_euclid(std::f32::consts::TAU);
        let adapted_wrong_prediction =
            bus_transfer_predict(target_input, target_phase_bias, adapted_wrong_phase);
        score_prediction(
            adapted_wrong_snapshot,
            adapted_wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = transition_center_phase(corrupted, future_carrier);
        let corrupted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        no_snapshot_correction = adapt_phase_correction(
            no_snapshot_correction,
            adapted_no_phase,
            target_phase,
            learning_rate,
            1.0,
        );
        snapshot_correction = adapt_phase_correction(
            snapshot_correction,
            adapted_phase,
            target_phase,
            learning_rate,
            current_snapshot.coherence,
        );
        wrong_snapshot_correction = adapt_phase_correction(
            wrong_snapshot_correction,
            adapted_wrong_phase,
            target_phase,
            learning_rate,
            wrong.coherence,
        );

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_multitick_split(
    config: PhaseCompositionConfig,
    horizon: usize,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let mut future_carrier = carrier;
        for offset in 1..=horizon {
            let future_input = periodic_byte(config.start, config.input_step, case_index + offset);
            future_carrier = future_carrier.advance(future_input, 1);
        }

        let target_index = case_index + horizon;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let target_tick =
            run_stage2_tick_with_organ_carrier(&organ, target_input, future_carrier, None);
        let target = bus_transfer_target(
            target_input,
            target_phase_bias,
            target_tick.trace.center_phase,
        );

        let random_prediction = random_predict(config.seed, case_index, target_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, target_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_snapshot_prediction =
            bus_transfer_predict(target_input, target_phase_bias, future_carrier.phase);
        score_prediction(no_snapshot, no_snapshot_prediction, target, 0.0, 1.0);

        let warm_phase = transition_center_phase(current_snapshot, future_carrier);
        let warm_prediction = bus_transfer_predict(target_input, target_phase_bias, warm_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = transition_center_phase(wrong, future_carrier);
        let wrong_prediction = bus_transfer_predict(target_input, target_phase_bias, wrong_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = transition_center_phase(corrupted, future_carrier);
        let corrupted_prediction =
            bus_transfer_predict(target_input, target_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        let next_input = periodic_byte(config.start, config.input_step, case_index + 1);
        carrier = carrier.advance(next_input, 1);
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_dynamics_split(
    config: PhaseCompositionConfig,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    let mut carrier = CarrierWave::from_seed(config.seed, config.start);
    let organ = Stage2Organ::new(config.seed);

    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let current_snapshot = snapshot_roundtrip(
            run_stage2_tick_with_organ_carrier(&organ, input, carrier, None).snapshot,
        );

        let target_index = case_index + 1;
        let next_input = periodic_byte(config.start, config.input_step, target_index);
        let next_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let next_carrier = carrier.advance(next_input, 1);
        let next_tick = run_stage2_tick_with_organ_carrier(&organ, next_input, next_carrier, None);
        let target = bus_transfer_target(next_input, next_phase_bias, next_tick.trace.center_phase);

        let random_prediction = random_predict(config.seed, case_index, next_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, next_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_snapshot_prediction =
            bus_transfer_predict(next_input, next_phase_bias, next_carrier.phase);
        score_prediction(no_snapshot, no_snapshot_prediction, target, 0.0, 1.0);

        let warm_phase = transition_center_phase(current_snapshot, next_carrier);
        let warm_prediction = bus_transfer_predict(next_input, next_phase_bias, warm_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            current_snapshot.coherence,
            current_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = transition_center_phase(wrong, next_carrier);
        let wrong_prediction = bus_transfer_predict(next_input, next_phase_bias, wrong_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(current_snapshot);
        let corrupted_phase = transition_center_phase(corrupted, next_carrier);
        let corrupted_prediction =
            bus_transfer_predict(next_input, next_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );

        carrier = next_carrier;
    }
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_transition_split(
    config: PhaseCompositionConfig,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let seed = config.seed + case_index as u64;
        let previous_snapshot = snapshot_roundtrip(run_stage2_tick(seed, input).snapshot);

        let target_index = case_index + 1;
        let next_input = periodic_byte(config.start, config.input_step, target_index);
        let next_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let next_seed = config.seed + target_index as u64;
        let next_carrier = CarrierWave::from_seed(next_seed, next_input);
        let next_tick = run_stage2_tick_with_carrier(next_seed, next_input, next_carrier, None);
        let target = bus_transfer_target(next_input, next_phase_bias, next_tick.trace.center_phase);

        let random_prediction = random_predict(config.seed, case_index, next_input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, next_input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_snapshot_phase = next_carrier.phase;
        let no_snapshot_prediction =
            bus_transfer_predict(next_input, next_phase_bias, no_snapshot_phase);
        score_prediction(no_snapshot, no_snapshot_prediction, target, 0.0, 1.0);

        let warm_phase = transition_center_phase(previous_snapshot, next_carrier);
        let warm_prediction = bus_transfer_predict(next_input, next_phase_bias, warm_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            previous_snapshot.coherence,
            previous_snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_phase = transition_center_phase(wrong, next_carrier);
        let wrong_prediction = bus_transfer_predict(next_input, next_phase_bias, wrong_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(previous_snapshot);
        let corrupted_phase = transition_center_phase(corrupted, next_carrier);
        let corrupted_prediction =
            bus_transfer_predict(next_input, next_phase_bias, corrupted_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );
    }
}

fn transition_center_phase(snapshot: SpectrumSnapshot, next_carrier: CarrierWave) -> f32 {
    let stored_offset = circular_delta(snapshot.carrier.phase, snapshot.center_phase);
    (next_carrier.phase + stored_offset * snapshot.coherence).rem_euclid(std::f32::consts::TAU)
}

#[derive(Debug, Clone, Copy)]
struct OnlinePhaseDecoder {
    weights: [f32; 4],
    learning_rate: f32,
}

impl OnlinePhaseDecoder {
    fn new(learning_rate: f32) -> Self {
        Self {
            weights: [0.0; 4],
            learning_rate,
        }
    }

    fn predict(self, base_phase: f32, features: [f32; 4]) -> f32 {
        let correction = self
            .weights
            .iter()
            .zip(features.iter())
            .map(|(weight, feature)| weight * feature)
            .sum::<f32>();
        (base_phase + correction).rem_euclid(std::f32::consts::TAU)
    }

    fn update(&mut self, predicted_phase: f32, target_phase: f32, features: [f32; 4]) {
        let phase_error = circular_delta(predicted_phase, target_phase);
        for (weight, feature) in self.weights.iter_mut().zip(features.iter()) {
            *weight += self.learning_rate * phase_error * feature;
            *weight = weight.clamp(-std::f32::consts::TAU, std::f32::consts::TAU);
        }
    }
}

fn snapshot_decoder_features(snapshot: SpectrumSnapshot) -> [f32; 4] {
    [
        1.0,
        circular_delta(snapshot.carrier.phase, snapshot.center_phase) / std::f32::consts::TAU,
        snapshot.coherence.clamp(0.0, 1.0),
        snapshot.center_magnitude.clamp(0.0, 1.0),
    ]
}

fn no_snapshot_decoder_features(carrier: CarrierWave) -> [f32; 4] {
    [
        1.0,
        0.0,
        carrier.envelope().clamp(0.0, 1.0),
        (carrier.frequency - 1.0).clamp(0.0, 1.0),
    ]
}

fn snapshot_private_phase(snapshot: SpectrumSnapshot) -> f32 {
    let mut sin_sum = snapshot.center_phase.sin() * snapshot.center_magnitude.max(0.1);
    let mut cos_sum = snapshot.center_phase.cos() * snapshot.center_magnitude.max(0.1);

    for (index, phase) in snapshot.top_phases.iter().take(4).enumerate() {
        let weight = snapshot.coherence.clamp(0.05, 1.0) / (index as f32 + 1.0);
        sin_sum += phase.sin() * weight;
        cos_sum += phase.cos() * weight;
    }

    sin_sum.atan2(cos_sum).rem_euclid(std::f32::consts::TAU)
}

fn snapshot_noisy_private_phase(snapshot: SpectrumSnapshot) -> f32 {
    let base_phase = snapshot_private_phase(snapshot);
    let slot_a = snapshot.top_phases[0];
    let slot_b = snapshot.top_phases[1];
    let hidden_delta =
        (slot_a.sin() * 0.16 + slot_b.cos() * 0.11) * snapshot.coherence.clamp(0.1, 1.0);
    (base_phase + hidden_delta).rem_euclid(std::f32::consts::TAU)
}

fn combine_transition_phase(future_phase: f32, memory_phase: f32, memory_coherence: f32) -> f32 {
    let memory_weight = memory_coherence.clamp(0.15, 0.85);
    let future_weight = 1.0 - memory_weight;
    let sin_sum = future_phase.sin() * future_weight + memory_phase.sin() * memory_weight;
    let cos_sum = future_phase.cos() * future_weight + memory_phase.cos() * memory_weight;
    sin_sum.atan2(cos_sum).rem_euclid(std::f32::consts::TAU)
}

fn adapt_phase_correction(
    correction: f32,
    predicted_phase: f32,
    target_phase: f32,
    learning_rate: f32,
    confidence: f32,
) -> f32 {
    let phase_error = circular_delta(predicted_phase, target_phase);
    circular_delta(
        0.0,
        correction + phase_error * learning_rate * confidence.clamp(0.0, 1.0),
    )
}

#[allow(clippy::too_many_arguments)]
fn score_snapshot_memory_split(
    config: PhaseCompositionConfig,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_snapshot: &mut BaselineResult,
    warm_snapshot: &mut BaselineResult,
    wrong_snapshot: &mut BaselineResult,
    corrupted_snapshot: &mut BaselineResult,
) {
    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let phase_bias = periodic_byte(0, config.phase_step, case_index);
        let seed = config.seed + case_index as u64;
        let tick = run_stage2_tick(seed, input);
        let snapshot = snapshot_roundtrip(tick.snapshot);
        let target = bus_transfer_target(input, phase_bias, tick.trace.center_phase);

        let random_prediction = random_predict(config.seed, case_index, input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_snapshot_prediction = bus_transfer_predict(input, phase_bias, 0.0);
        score_prediction(no_snapshot, no_snapshot_prediction, target, 0.0, 1.0);

        let warm_prediction = bus_transfer_predict(input, phase_bias, snapshot.center_phase);
        score_prediction(
            warm_snapshot,
            warm_prediction,
            target,
            snapshot.coherence,
            snapshot.spectral_entropy,
        );

        let wrong = build_wrong_snapshot(config, case_index, input);
        let wrong_prediction = bus_transfer_predict(input, phase_bias, wrong.center_phase);
        score_prediction(
            wrong_snapshot,
            wrong_prediction,
            target,
            wrong.coherence,
            wrong.spectral_entropy,
        );

        let corrupted = build_corrupted_snapshot(snapshot);
        let corrupted_prediction = bus_transfer_predict(input, phase_bias, corrupted.center_phase);
        score_prediction(
            corrupted_snapshot,
            corrupted_prediction,
            target,
            corrupted.coherence,
            corrupted.spectral_entropy,
        );
    }
}

pub(crate) fn snapshot_roundtrip(snapshot: SpectrumSnapshot) -> SpectrumSnapshot {
    SpectrumSnapshot::from_bytes(&snapshot.to_bytes()).expect("generated snapshot must roundtrip")
}

fn build_wrong_snapshot(
    config: PhaseCompositionConfig,
    case_index: usize,
    input: u8,
) -> SpectrumSnapshot {
    let wrong_seed = config.seed ^ 0xDEAD_BEEF_51A7_E005 ^ case_index as u64;
    let wrong_input = input.wrapping_add(97);
    snapshot_roundtrip(run_stage2_tick(wrong_seed, wrong_input).snapshot)
}

pub(crate) fn build_corrupted_snapshot(mut snapshot: SpectrumSnapshot) -> SpectrumSnapshot {
    snapshot.center_phase =
        (snapshot.center_phase + std::f32::consts::TAU * 0.3125).rem_euclid(std::f32::consts::TAU);
    snapshot.center_magnitude *= 0.25;
    snapshot.coherence *= 0.25;
    snapshot.spectral_entropy = (snapshot.spectral_entropy + 0.25).min(1.0);
    for phase in &mut snapshot.top_phases {
        *phase = (*phase + std::f32::consts::TAU * 0.125).rem_euclid(std::f32::consts::TAU);
    }
    snapshot_roundtrip(snapshot)
}

pub(crate) fn build_corrupted_carrier_snapshot(mut snapshot: SpectrumSnapshot) -> SpectrumSnapshot {
    snapshot.carrier.phase =
        (snapshot.carrier.phase + std::f32::consts::TAU * 0.375).rem_euclid(std::f32::consts::TAU);
    snapshot.carrier.amplitude *= 0.25;
    build_corrupted_snapshot(snapshot)
}

#[allow(clippy::too_many_arguments)]
fn score_bus_transfer_split(
    config: PhaseCompositionConfig,
    random: &mut BaselineResult,
    mono192: &mut BaselineResult,
    no_bus: &mut BaselineResult,
    voting: &mut BaselineResult,
    correct_carrier_bus: &mut BaselineResult,
    no_carrier_bus: &mut BaselineResult,
    wrong_carrier_bus: &mut BaselineResult,
    corrupted_carrier_bus: &mut BaselineResult,
    ablations: &mut [BaselineResult; STAGE2_ORGAN_CELLS],
) {
    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let phase_bias = periodic_byte(0, config.phase_step, case_index);
        let seed = config.seed + case_index as u64;
        let correct_carrier = CarrierWave::from_seed(seed, input);
        let correct_tick = run_stage2_tick_with_carrier(seed, input, correct_carrier, None);
        let target_index = case_index + 1;
        let target_input = periodic_byte(config.start, config.input_step, target_index);
        let target_phase_bias = periodic_byte(0, config.phase_step, target_index);
        let target_seed = config.seed + target_index as u64;
        let target_carrier = CarrierWave::from_seed(target_seed, target_input);
        let target_tick =
            run_stage2_tick_with_carrier(target_seed, target_input, target_carrier, None);
        let target = bus_transfer_target(
            target_input,
            target_phase_bias,
            target_tick.trace.center_phase,
        );

        let random_prediction = random_predict(config.seed, case_index, input);
        score_prediction(random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono192_predict(config.seed, case_index, input);
        score_prediction(mono192, mono_prediction, target, 0.0, 1.0);

        let no_bus_prediction = no_bus_predict(input, correct_tick.trace.active_cell_ids);
        score_prediction(
            no_bus,
            no_bus_prediction,
            target,
            correct_tick.trace.coherence * 0.5,
            correct_tick.trace.spectral_entropy,
        );

        let voting_prediction = voting_predict(input, correct_tick.trace.active_cell_ids);
        score_prediction(
            voting,
            voting_prediction,
            target,
            correct_tick.trace.coherence * 0.75,
            correct_tick.trace.spectral_entropy,
        );

        score_bus_transfer_variant(correct_carrier_bus, input, phase_bias, target, correct_tick);
        score_bus_transfer_variant(
            no_carrier_bus,
            input,
            phase_bias,
            target,
            run_stage2_tick_with_carrier(seed, input, no_carrier_wave(), None),
        );
        score_bus_transfer_variant(
            wrong_carrier_bus,
            input,
            phase_bias,
            target,
            run_stage2_tick_with_carrier(seed, input, wrong_carrier_wave(seed, input), None),
        );
        score_bus_transfer_variant(
            corrupted_carrier_bus,
            input,
            phase_bias,
            target,
            run_stage2_tick_with_carrier(
                seed,
                input,
                corrupted_carrier_wave(correct_carrier),
                None,
            ),
        );

        for (cell_id, ablation) in ablations.iter_mut().enumerate() {
            let ablated_tick =
                run_stage2_tick_with_carrier(seed, input, correct_carrier, Some(cell_id as u32));
            score_bus_transfer_variant(ablation, input, phase_bias, target, ablated_tick);
        }
    }
}

fn score_bus_transfer_variant(
    result: &mut BaselineResult,
    input: u8,
    phase_bias: u8,
    target: u8,
    tick: nando_core::Stage2Tick,
) {
    let prediction = bus_transfer_predict(input, phase_bias, tick.trace.center_phase);
    score_prediction(
        result,
        prediction,
        target,
        tick.trace.coherence,
        tick.trace.spectral_entropy,
    );
}

fn score_carrier_control_split(
    config: PhaseCompositionConfig,
    correct_carrier: &mut BaselineResult,
    no_carrier: &mut BaselineResult,
    wrong_carrier: &mut BaselineResult,
    corrupted_carrier: &mut BaselineResult,
) {
    for case_index in 0..config.cases {
        let input = periodic_byte(config.start, config.input_step, case_index);
        let phase_bias = periodic_byte(0, config.phase_step, case_index);
        let seed = config.seed + case_index as u64;
        let target_carrier = CarrierWave::from_seed(seed, input);
        let target = phase_composition_target(input, phase_bias, target_carrier.phase);

        score_carrier_variant(
            correct_carrier,
            seed,
            input,
            phase_bias,
            target,
            target_carrier,
        );
        score_carrier_variant(
            no_carrier,
            seed,
            input,
            phase_bias,
            target,
            no_carrier_wave(),
        );
        score_carrier_variant(
            wrong_carrier,
            seed,
            input,
            phase_bias,
            target,
            wrong_carrier_wave(seed, input),
        );
        score_carrier_variant(
            corrupted_carrier,
            seed,
            input,
            phase_bias,
            target,
            corrupted_carrier_wave(target_carrier),
        );
    }
}

fn score_carrier_variant(
    result: &mut BaselineResult,
    seed: u64,
    input: u8,
    phase_bias: u8,
    target: u8,
    carrier: CarrierWave,
) {
    let tick = run_stage2_tick_with_carrier(seed, input, carrier, None);
    let prediction = phase_wave_predict(input, phase_bias, carrier.phase, tick.trace.center_phase);
    score_prediction(
        result,
        prediction,
        target,
        tick.trace.coherence,
        tick.trace.spectral_entropy,
    );
}

pub(crate) fn no_carrier_wave() -> CarrierWave {
    CarrierWave {
        phase: 0.0,
        amplitude: 0.0,
        frequency: 0.0,
        boundary: 0.0,
    }
}

pub(crate) fn wrong_carrier_wave(seed: u64, input: u8) -> CarrierWave {
    CarrierWave::from_seed(seed ^ 0xA5A5_5A5A_D00D_F00D, input.wrapping_add(73))
}

pub(crate) fn corrupted_carrier_wave(carrier: CarrierWave) -> CarrierWave {
    CarrierWave {
        phase: (carrier.phase + std::f32::consts::TAU * 0.1875).rem_euclid(std::f32::consts::TAU),
        amplitude: carrier.amplitude * 0.35,
        frequency: carrier.frequency * 1.25,
        boundary: carrier.boundary * 0.50,
    }
}

pub(crate) fn score_prediction(
    baseline: &mut BaselineResult,
    prediction: u8,
    target: u8,
    coherence: f32,
    spectral_entropy: f32,
) {
    if prediction == target {
        baseline.correct += 1;
    }
    baseline.mean_circular_error += circular_error(prediction, target);
    baseline.mean_coherence += coherence;
    baseline.mean_spectral_entropy += spectral_entropy;
}

fn circular_error(prediction: u8, target: u8) -> f32 {
    let forward = prediction.wrapping_sub(target);
    let backward = target.wrapping_sub(prediction);
    f32::from(forward.min(backward))
}

pub(crate) fn random_predict(seed: u64, case_index: usize, input: u8) -> u8 {
    splitmix64(seed ^ case_index as u64 ^ u64::from(input)).to_le_bytes()[0]
}

pub(crate) fn mono192_predict(seed: u64, case_index: usize, input: u8) -> u8 {
    let noise = splitmix64(seed.rotate_left(7) ^ (case_index as u64).wrapping_mul(0x9E37) ^ 0x192);
    input.wrapping_add(noise.to_le_bytes()[0] & 31)
}

fn no_bus_predict(input: u8, active_cell_ids: [u32; STAGE2_TOP_K]) -> u8 {
    let offset = active_cell_ids.iter().fold(0u8, |accumulator, cell_id| {
        accumulator.wrapping_add(*cell_id as u8 + 1)
    });
    input.wrapping_add(offset)
}

pub(crate) fn voting_predict(input: u8, active_cell_ids: [u32; STAGE2_TOP_K]) -> u8 {
    let mut votes = [0u8; STAGE2_TOP_K];
    for (index, cell_id) in active_cell_ids.iter().enumerate() {
        votes[index] = input.wrapping_add((*cell_id as u8 + 1).wrapping_mul(3));
    }
    votes.sort_unstable();
    votes[STAGE2_TOP_K / 2]
}

pub(crate) fn wave_bus_predict(center_phase: f32) -> u8 {
    let unit = (center_phase / std::f32::consts::TAU).rem_euclid(1.0);
    (unit * 256.0).round() as u8
}

fn phase_composition_target(input: u8, phase_bias: u8, carrier_phase: f32) -> u8 {
    let carrier_bucket = wave_bus_predict(carrier_phase);
    input
        .rotate_left(1)
        .wrapping_add(phase_bias)
        .wrapping_add(carrier_bucket)
}

fn phase_wave_predict(input: u8, phase_bias: u8, carrier_phase: f32, center_phase: f32) -> u8 {
    let carrier_bucket = wave_bus_predict(carrier_phase);
    let center_bucket = wave_bus_predict(center_phase) >> 4;
    input
        .rotate_left(1)
        .wrapping_add(phase_bias)
        .wrapping_add(carrier_bucket)
        .wrapping_add(center_bucket)
}

fn bus_transfer_target(input: u8, phase_bias: u8, correct_center_phase: f32) -> u8 {
    bus_transfer_predict(input, phase_bias, correct_center_phase)
}

fn bus_transfer_predict(input: u8, phase_bias: u8, center_phase: f32) -> u8 {
    let center_bucket = wave_bus_predict(center_phase);
    input
        .rotate_left(1)
        .wrapping_add(phase_bias.rotate_left(1))
        .wrapping_add(center_bucket)
}

fn wave_advantage(report: &PhaseCompositionReport) -> f32 {
    let best_non_wave = [
        report.random.accuracy,
        report.mono192.accuracy,
        report.no_bus.accuracy,
        report.voting.accuracy,
    ]
    .into_iter()
    .fold(f32::NEG_INFINITY, f32::max);
    report.wave_bus.accuracy - best_non_wave
}

fn indent_report(report: &str) -> String {
    let mut output = String::new();
    for line in report.lines() {
        output.push_str("  ");
        output.push_str(line);
        output.push('\n');
    }
    output
}

fn best_baseline_name<const N: usize>(results: [BaselineResult; N]) -> &'static str {
    let mut best = results[0];
    for result in results.into_iter().skip(1) {
        if result.accuracy > best.accuracy
            || (result.accuracy == best.accuracy
                && result.mean_circular_error < best.mean_circular_error)
        {
            best = result;
        }
    }
    best.name
}

pub(crate) fn best_baseline<const N: usize>(results: [BaselineResult; N]) -> BaselineResult {
    let mut best = results[0];
    for result in results.into_iter().skip(1) {
        if result.accuracy > best.accuracy
            || (result.accuracy == best.accuracy
                && result.mean_circular_error < best.mean_circular_error)
        {
            best = result;
        }
    }
    best
}

fn weakest_ablation(ablations: [BaselineResult; STAGE2_ORGAN_CELLS]) -> (u32, BaselineResult) {
    let mut key_cell = 0;
    let mut weakest = ablations[0];
    for (cell_id, ablation) in ablations.into_iter().enumerate().skip(1) {
        if ablation.accuracy < weakest.accuracy
            || (ablation.accuracy == weakest.accuracy
                && ablation.mean_circular_error > weakest.mean_circular_error)
        {
            key_cell = cell_id as u32;
            weakest = ablation;
        }
    }
    (key_cell, weakest)
}

fn ablation_name(cell_id: usize) -> &'static str {
    match cell_id {
        0 => "ablate_cell0",
        1 => "ablate_cell1",
        2 => "ablate_cell2",
        3 => "ablate_cell3",
        4 => "ablate_cell4",
        5 => "ablate_cell5",
        _ => "ablate_cell_unknown",
    }
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
