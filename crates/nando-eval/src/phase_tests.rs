use super::*;

#[test]
fn periodic_eval_has_required_baselines() {
    let report = periodic_eval(PeriodicTaskConfig {
        seed: 7,
        cases: 16,
        start: 11,
        step: 17,
    });
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.mono192.cases, 16);
    assert_eq!(report.no_bus.cases, 16);
    assert_eq!(report.voting.cases, 16);
    assert_eq!(report.wave_bus.cases, 16);
    assert_eq!(report.ablations.len(), STAGE2_ORGAN_CELLS);
    assert!(report.to_text().contains("mono192.accuracy"));
    assert!(report.to_text().contains("cell32_voting.accuracy"));
    assert!(report.to_text().contains("cell32_wave_bus.accuracy"));
    assert!(report.to_text().contains("ablation_drop"));
    assert!(
        report.mode_status == "not_found_stage_3_ablation"
            || report.mode_status == "candidate_needs_stronger_task"
    );
}

#[test]
fn phase_composition_eval_has_required_baselines() {
    let report = phase_composition_eval(PhaseCompositionConfig {
        seed: 13,
        cases: 16,
        start: 19,
        input_step: 23,
        phase_step: 5,
    });
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.mono192.cases, 16);
    assert_eq!(report.no_bus.cases, 16);
    assert_eq!(report.voting.cases, 16);
    assert_eq!(report.wave_bus.cases, 16);
    assert_eq!(report.ablations.len(), STAGE2_ORGAN_CELLS);
    assert!(report.to_text().contains("phase-composition eval"));
    assert!(report.to_text().contains("cell32_voting.accuracy"));
    assert!(report.to_text().contains("ablation_drop"));
    assert!(
        report.mode_status == "not_found_phase_composition"
            || report.mode_status == "candidate_needs_holdout"
    );
}

#[test]
fn phase_holdout_report_has_two_splits() {
    let report = phase_composition_holdout_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.train.config.seed, 13);
    assert_eq!(report.holdout.config.seed, 97);
    assert!(report.to_text().contains("wave_advantage_train"));
    assert!(
        report.mode_status == "candidate_holdout_passed_needs_carrier_test"
            || report.mode_status == "not_found_holdout_failed"
    );
}

#[test]
fn carrier_control_report_has_required_variants() {
    let report = carrier_control_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.correct_carrier.cases, 32);
    assert_eq!(report.no_carrier.cases, 32);
    assert_eq!(report.wrong_carrier.cases, 32);
    assert_eq!(report.corrupted_carrier.cases, 32);
    assert!(report.to_text().contains("correct_carrier_wave.accuracy"));
    assert!(report.to_text().contains("wrong_carrier_wave.accuracy"));
    assert!(report.to_text().contains("correct_over_no"));
    assert!(
        report.mode_status == "carrier_control_passed_candidate_mode"
            || report.mode_status == "not_found_carrier_control"
    );
}

#[test]
fn bus_transfer_report_has_required_controls() {
    let report = bus_transfer_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.correct_carrier_bus.cases, 32);
    assert_eq!(report.wrong_carrier_bus.cases, 32);
    assert_eq!(report.ablations.len(), STAGE2_ORGAN_CELLS);
    assert!(report.to_text().contains("delayed bus-transfer eval"));
    assert!(report.to_text().contains("correct_carrier_bus.accuracy"));
    assert!(report.to_text().contains("correct_over_best_baseline"));
    assert!(
        report.mode_status == "bus_transfer_passed_candidate_mode"
            || report.mode_status == "not_found_bus_transfer"
    );
}

#[test]
fn snapshot_memory_report_has_required_controls() {
    let report = snapshot_memory_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.warm_snapshot.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-memory eval"));
    assert!(report.to_text().contains("warm_snapshot.accuracy"));
    assert!(report.to_text().contains("warm_over_no_snapshot"));
    assert!(
        report.mode_status == "snapshot_memory_passed_state_replay"
            || report.mode_status == "not_found_snapshot_memory"
    );
}

#[test]
fn snapshot_transition_report_has_required_controls() {
    let report = snapshot_transition_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.warm_snapshot.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-transition eval"));
    assert!(
        report
            .to_text()
            .contains("warm_snapshot_transition.accuracy")
    );
    assert!(report.to_text().contains("warm_over_wrong_snapshot"));
    assert!(
        report.mode_status == "snapshot_transition_passed"
            || report.mode_status == "not_found_snapshot_transition"
    );
}

#[test]
fn snapshot_dynamics_report_has_required_controls() {
    let report = snapshot_dynamics_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.warm_snapshot.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-dynamics eval"));
    assert!(
        report
            .to_text()
            .contains("warm_snapshot_dynamics.mean_circular_error")
    );
    assert!(report.to_text().contains("warm_error_gain_over_no"));
    assert!(
        report.mode_status == "snapshot_dynamics_passed"
            || report.mode_status == "not_found_snapshot_dynamics"
    );
}

#[test]
fn snapshot_multitick_report_has_required_controls() {
    let report = snapshot_multitick_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert_eq!(report.warm_snapshot.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-multitick eval"));
    assert!(
        report
            .to_text()
            .contains("warm_snapshot_multitick.mean_circular_error")
    );
    assert!(report.to_text().contains("warm_error_gain_over_wrong"));
    assert!(
        report.mode_status == "snapshot_multitick_passed"
            || report.mode_status == "not_found_snapshot_multitick"
    );
}

#[test]
fn snapshot_adapt_report_has_required_controls() {
    let report = snapshot_adapt_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert!(report.learning_rate > 0.0);
    assert_eq!(report.adapted_snapshot.cases, 32);
    assert_eq!(report.adapted_no_snapshot.cases, 32);
    assert_eq!(report.adapted_wrong_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-adapt eval"));
    assert!(
        report
            .to_text()
            .contains("adapted_snapshot.mean_circular_error")
    );
    assert!(
        report
            .to_text()
            .contains("adapted_error_gain_over_wrong_adapt")
    );
    assert!(
        report.mode_status == "snapshot_adapt_passed"
            || report.mode_status == "not_found_snapshot_adapt"
    );
}

#[test]
fn snapshot_decoder_report_has_required_controls() {
    let report = snapshot_decoder_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert!(report.learning_rate > 0.0);
    assert_eq!(report.decoder_snapshot.cases, 32);
    assert_eq!(report.decoder_no_snapshot.cases, 32);
    assert_eq!(report.decoder_wrong_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-decoder eval"));
    assert!(
        report
            .to_text()
            .contains("decoder_snapshot.mean_circular_error")
    );
    assert!(
        report
            .to_text()
            .contains("decoder_error_gain_over_no_decoder")
    );
    assert!(
        report.mode_status == "snapshot_decoder_passed"
            || report.mode_status == "not_found_snapshot_decoder"
    );
}

#[test]
fn snapshot_keyed_report_has_required_controls() {
    let report = snapshot_keyed_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert_eq!(report.keyed_snapshot.cases, 32);
    assert_eq!(report.no_snapshot.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-keyed eval"));
    assert!(
        report
            .to_text()
            .contains("keyed_snapshot.mean_circular_error")
    );
    assert!(report.to_text().contains("keyed_error_gain_over_no"));
    assert!(
        report.mode_status == "snapshot_keyed_passed"
            || report.mode_status == "not_found_snapshot_keyed"
    );
}

#[test]
fn snapshot_keyed_transition_report_has_required_controls() {
    let report = snapshot_keyed_transition_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert_eq!(report.keyed_transition.cases, 32);
    assert_eq!(report.future_only.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(report.to_text().contains("snapshot-keyed-transition eval"));
    assert!(
        report
            .to_text()
            .contains("keyed_transition.mean_circular_error")
    );
    assert!(
        report
            .to_text()
            .contains("keyed_error_gain_over_future_only")
    );
    assert!(
        report.mode_status == "snapshot_keyed_transition_passed"
            || report.mode_status == "not_found_snapshot_keyed_transition"
    );
}

#[test]
fn snapshot_noisy_keyed_transition_report_has_required_controls() {
    let report = snapshot_noisy_keyed_transition_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.horizon, 4);
    assert_eq!(report.keyed_transition.cases, 32);
    assert_eq!(report.future_only.cases, 32);
    assert_eq!(report.wrong_snapshot.cases, 32);
    assert_eq!(report.corrupted_snapshot.cases, 32);
    assert!(
        report
            .to_text()
            .contains("snapshot-noisy-keyed-transition eval")
    );
    assert!(
        report
            .to_text()
            .contains("keyed_noisy_transition.mean_circular_error")
    );
    assert!(
        report
            .to_text()
            .contains("keyed_error_gain_over_future_only")
    );
    assert!(
        report.mode_status == "snapshot_noisy_keyed_transition_passed"
            || report.mode_status == "not_found_snapshot_noisy_keyed_transition"
    );
}

#[test]
fn snapshot_noisy_keyed_transition_sweep_report_has_required_controls() {
    let report = snapshot_noisy_keyed_transition_sweep_eval(
        PhaseCompositionConfig {
            seed: 13,
            cases: 16,
            start: 19,
            input_step: 23,
            phase_step: 5,
        },
        PhaseCompositionConfig {
            seed: 97,
            cases: 16,
            start: 31,
            input_step: 29,
            phase_step: 7,
        },
    );
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_count <= 4);
    assert!(
        report
            .to_text()
            .contains("snapshot-noisy-keyed-transition-sweep eval")
    );
    assert!(report.to_text().contains("horizon_1.keyed_accuracy"));
    assert!(report.to_text().contains("horizon_8.passed"));
    assert!(report.to_text().contains("min_error_gain_over_future_only"));
    assert!(
        report.mode_status == "snapshot_noisy_keyed_transition_sweep_passed"
            || report.mode_status == "not_found_snapshot_noisy_keyed_transition_sweep"
    );
}

#[test]
fn snapshot_noisy_keyed_transition_seed_sweep_report_has_required_controls() {
    let report = snapshot_noisy_keyed_transition_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("snapshot-noisy-keyed-transition-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.train_seed"));
    assert!(report.to_text().contains("seed_pair_3.passed"));
    assert!(
        report
            .to_text()
            .contains("min_error_gain_over_wrong_snapshot")
    );
    assert!(
        report.mode_status == "snapshot_noisy_keyed_transition_seed_sweep_passed"
            || report.mode_status == "not_found_snapshot_noisy_keyed_transition_seed_sweep"
    );
}
