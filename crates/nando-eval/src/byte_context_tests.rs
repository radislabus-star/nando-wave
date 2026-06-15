use super::*;
use crate::{Chat0FeedbackEntry, Chat0PromotedState, chat0_response_for_target};

#[test]
fn byte_context_report_has_required_controls() {
    let report = byte_context_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.mono192_prompt_decoder.cases, 16);
    assert_eq!(report.no_snapshot_decoder.cases, 16);
    assert_eq!(report.cell32_voting.cases, 16);
    assert_eq!(report.snapshot_decoder.cases, 16);
    assert_eq!(report.wrong_snapshot_decoder.cases, 16);
    assert_eq!(report.corrupted_snapshot_decoder.cases, 16);
    assert!(report.to_text().contains("byte-context eval"));
    assert!(report.to_text().contains("mono192_prompt_decoder.accuracy"));
    assert!(
        report
            .to_text()
            .contains("snapshot_accuracy_over_best_control")
    );
    assert!(
        report.mode_status == "byte_context_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context"
    );
}

#[test]
fn byte_context_centroid_report_has_required_controls() {
    let report = byte_context_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.mono192_prompt_centroid.cases, 16);
    assert_eq!(report.no_snapshot_centroid.cases, 16);
    assert_eq!(report.cell32_voting.cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(report.to_text().contains("byte-context-centroid eval"));
    assert!(
        report
            .to_text()
            .contains("mono192_prompt_centroid.accuracy")
    );
    assert!(
        report
            .to_text()
            .contains("snapshot_accuracy_over_best_control")
    );
    assert!(
        report.mode_status == "byte_context_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_centroid"
    );
}

#[test]
fn byte_context_offset_centroid_report_has_required_controls() {
    let report = byte_context_offset_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_offset_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_offset_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_offset_centroid"
    );
}

#[test]
fn byte_context_denoised_centroid_report_has_required_controls() {
    let report = byte_context_denoised_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_denoised_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_denoised_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_denoised_centroid"
    );
}

#[test]
fn byte_context_relative_centroid_report_has_required_controls() {
    let report = byte_context_relative_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_relative_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_relative_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_relative_centroid"
    );
}

#[test]
fn byte_context_lexical_carrier_centroid_report_has_required_controls() {
    let report = byte_context_lexical_carrier_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_lexical_carrier_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_lexical_carrier_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_lexical_carrier_centroid"
    );
}

#[test]
fn byte_context_cellular_carrier_centroid_report_has_required_controls() {
    let report = byte_context_cellular_carrier_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_cellular_carrier_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_cellular_carrier_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_cellular_carrier_centroid"
    );
}

#[test]
fn byte_context_trained_carrier_centroid_report_has_required_controls() {
    let report = byte_context_trained_carrier_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_trained_carrier_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_trained_carrier_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_trained_carrier_centroid"
    );
}

#[test]
fn byte_context_prompt_carrier_centroid_report_has_required_controls() {
    let report = byte_context_prompt_carrier_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_prompt_carrier_centroid.accuracy")
    );
    assert!(
        report.mode_status == "byte_context_prompt_carrier_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_prompt_carrier_centroid"
    );
}

#[test]
fn byte_context_prompt_carrier_diverse_centroid_report_has_required_controls() {
    let report = byte_context_prompt_carrier_diverse_centroid_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.train_cases, 16);
    assert_eq!(report.holdout_cases, 16);
    assert_eq!(report.snapshot_centroid.cases, 16);
    assert_eq!(report.wrong_snapshot_centroid.cases, 16);
    assert_eq!(report.corrupted_snapshot_centroid.cases, 16);
    assert!(
        report
            .to_text()
            .contains("snapshot_prompt_carrier_diverse_centroid.accuracy")
    );
    assert!(
        report.mode_status
            == "byte_context_prompt_carrier_diverse_centroid_candidate_needs_seed_sweep"
            || report.mode_status == "not_found_byte_context_prompt_carrier_diverse_centroid"
    );
}

#[test]
fn byte_context_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report
            .to_text()
            .contains("min_snapshot_accuracy_over_best_control")
    );
    assert!(
        report.mode_status == "byte_context_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_offset_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_offset_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_offset_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_offset_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_denoised_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_denoised_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_denoised_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_denoised_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_relative_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_relative_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_relative_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_relative_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_lexical_carrier_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_lexical_carrier_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_lexical_carrier_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_lexical_carrier_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_cellular_carrier_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_cellular_carrier_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_cellular_carrier_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_cellular_carrier_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_trained_carrier_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_trained_carrier_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_trained_carrier_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_trained_carrier_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_prompt_carrier_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_prompt_carrier_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_prompt_carrier_centroid_seed_sweep_passed"
            || report.mode_status == "not_found_byte_context_prompt_carrier_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_prompt_carrier_diverse_centroid_seed_sweep_report_has_required_controls() {
    let report = byte_context_prompt_carrier_diverse_centroid_seed_sweep_eval(16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.rows.len(), 4);
    assert!(report.passed_seed_pairs <= 4);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-seed-sweep eval")
    );
    assert!(report.to_text().contains("seed_pair_0.snapshot_accuracy"));
    assert!(
        report.mode_status == "byte_context_prompt_carrier_diverse_centroid_seed_sweep_passed"
            || report.mode_status
                == "not_found_byte_context_prompt_carrier_diverse_centroid_seed_sweep"
    );
}

#[test]
fn byte_context_centroid_ablation_report_has_required_controls() {
    let report = byte_context_centroid_ablation_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.full_snapshot.cases, 16);
    assert_eq!(report.ablations.len(), 5);
    assert!(
        report
            .to_text()
            .contains("byte-context-centroid-ablation eval")
    );
    assert!(report.to_text().contains("ablate_snapshot_offset.accuracy"));
    assert!(report.to_text().contains("key_feature"));
    assert!(report.to_text().contains("max_accuracy_drop"));
    assert!(
        report.mode_status == "byte_context_centroid_ablation_sensitive"
            || report.mode_status == "not_found_byte_context_centroid_ablation"
    );
}

#[test]
fn byte_context_cellular_carrier_ablation_report_has_required_controls() {
    let report = byte_context_cellular_carrier_ablation_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.full_snapshot.cases, 16);
    assert_eq!(report.ablations.len(), BYTE_CONTEXT_TASKS.len());
    assert!(
        report
            .to_text()
            .contains("byte-context-cellular-carrier-ablation eval")
    );
    assert!(report.to_text().contains("ablate_lock_ping.accuracy"));
    assert!(report.to_text().contains("min_accuracy_drop"));
    assert!(
        report.mode_status == "byte_context_cellular_carrier_ablation_sensitive"
            || report.mode_status == "not_found_byte_context_cellular_carrier_ablation"
    );
}

#[test]
fn byte_context_trained_carrier_ablation_report_has_required_controls() {
    let report = byte_context_trained_carrier_ablation_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.full_snapshot.cases, 16);
    assert_eq!(report.ablations.len(), BYTE_CONTEXT_TASKS.len());
    assert!(
        report
            .to_text()
            .contains("byte-context-trained-carrier-ablation eval")
    );
    assert!(
        report
            .to_text()
            .contains("ablate_trained_lock_ping.accuracy")
    );
    assert!(report.to_text().contains("min_accuracy_drop"));
    assert!(
        report.mode_status == "byte_context_trained_carrier_ablation_sensitive"
            || report.mode_status == "not_found_byte_context_trained_carrier_ablation"
    );
}

#[test]
fn byte_context_prompt_carrier_ablation_report_has_required_controls() {
    let report = byte_context_prompt_carrier_ablation_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.full_snapshot.cases, 16);
    assert_eq!(report.all_disabled.cases, 16);
    assert_eq!(report.ablations.len(), BYTE_CONTEXT_TASKS.len());
    assert!(
        report
            .to_text()
            .contains("byte-context-prompt-carrier-ablation eval")
    );
    assert!(
        report
            .to_text()
            .contains("ablate_prompt_lock_ping.accuracy")
    );
    assert!(report.to_text().contains("min_accuracy_drop"));
    assert!(report.to_text().contains("accuracy_over_all_disabled"));
    assert!(
        report.mode_status == "byte_context_prompt_carrier_bank_ablation_sensitive"
            || report.mode_status == "not_found_byte_context_prompt_carrier_ablation"
    );
}

#[test]
fn byte_context_prompt_carrier_diverse_ablation_report_has_required_controls() {
    let report = byte_context_prompt_carrier_diverse_ablation_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.full_snapshot.cases, 16);
    assert_eq!(report.all_disabled.cases, 16);
    assert_eq!(report.ablations.len(), BYTE_CONTEXT_TASKS.len());
    assert!(
        report
            .to_text()
            .contains("byte-context-prompt-carrier-ablation eval")
    );
    assert!(
        report
            .to_text()
            .contains("ablate_prompt_diverse_lock_all.accuracy")
    );
    assert!(report.to_text().contains("accuracy_over_all_disabled"));
    assert!(
        report.mode_status == "byte_context_prompt_carrier_diverse_bank_ablation_sensitive"
            || report.mode_status == "not_found_byte_context_prompt_carrier_diverse_ablation"
    );
}

#[test]
fn chat0_report_has_required_controls_and_feedback_log() {
    let report = chat0_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.mono192_prompt.cases, 16);
    assert_eq!(report.no_snapshot.cases, 16);
    assert_eq!(report.wrong_snapshot.cases, 16);
    assert_eq!(report.corrupted_snapshot.cases, 16);
    assert_eq!(report.prompt_cloud_snapshot.cases, 16);
    assert!(report.feedback_log_entries <= 16);
    assert!(report.to_text().contains("Nando Wave chat-0 eval"));
    assert!(
        report
            .to_text()
            .contains("prompt_cloud_snapshot_chat0.exact_accuracy")
    );
    assert!(report.to_text().contains("feedback_log_entries"));
    assert!(
        report.mode_status == "chat0_prompt_cloud_loop_passed"
            || report.mode_status == "not_found_chat0_prompt_cloud_loop"
    );
}

#[test]
fn chat0_route_eval_has_required_controls_and_route_counts() {
    let report = chat0_route_eval(13, 97, 16);
    assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert_eq!(report.cases_per_split, 16);
    assert_eq!(report.random.cases, 16);
    assert_eq!(report.snapshot_classifier.cases, 16);
    assert_eq!(report.prompt_cloud_lock_bank.cases, 16);
    assert_eq!(report.hybrid_route.cases, 16);
    assert!(report.lock_bank_route_count <= 16);
    assert!(report.feedback_log_entries <= 16);
    assert!(report.to_text().contains("Nando Wave chat-0 route eval"));
    assert!(
        report
            .to_text()
            .contains("prompt_cloud_lock_bank_chat0_route.exact_accuracy")
    );
    assert!(
        report.mode_status == "chat0_route_usable_snapshot_tied_or_better"
            || report.mode_status == "not_found_chat0_route_quality"
    );
}

#[test]
fn chat0_promote_eval_reports_no_corrections_without_mutation() {
    let feedback = [Chat0FeedbackEntry {
        prompt: String::from("manual:2: ping?"),
        response: String::from("pong"),
        expected: String::from("pong"),
        feedback_correct: true,
    }];

    let report = chat0_promote_eval(13, 97, 16, &feedback);

    assert_eq!(report.feedback_entries, 1);
    assert_eq!(report.correction_entries, 0);
    assert_eq!(report.replay_base.cases, 1);
    assert_eq!(report.replay_candidate.cases, 1);
    assert_eq!(report.mode_status, "not_found_chat0_promote_no_corrections");
    assert!(report.to_text().contains("Nando Wave chat-0 promote eval"));
}

#[test]
fn chat0_promote_eval_replay_candidate_improves_corrections() {
    let feedback = [Chat0FeedbackEntry {
        prompt: String::from("manual:2: ping?"),
        response: String::from("pong"),
        expected: String::from("help"),
        feedback_correct: false,
    }];

    let report = chat0_promote_eval(13, 97, 32, &feedback);

    assert_eq!(report.feedback_entries, 1);
    assert_eq!(report.correction_entries, 1);
    assert_eq!(report.replay_base.correct, 0);
    assert_eq!(report.replay_candidate.correct, 1);
    assert!(report.replay_improvement > 0.0);
    assert!(
        report.mode_status == "chat0_feedback_replay_promote_candidate_passed"
            || report.mode_status == "not_found_chat0_feedback_promote"
    );
}

#[test]
fn chat0_promoted_holdout_eval_separates_exact_overlay_from_task_hint() {
    let feedback: Vec<Chat0FeedbackEntry> = BYTE_CONTEXT_TASKS
        .iter()
        .map(|(task, target)| Chat0FeedbackEntry {
            prompt: format!("manual:2: {task}?"),
            response: String::from("?"),
            expected: String::from(chat0_response_for_target(*target)),
            feedback_correct: false,
        })
        .collect();

    let report = chat0_promoted_holdout_eval(13, 97, 32, &feedback);

    assert_eq!(report.feedback_entries, BYTE_CONTEXT_TASKS.len());
    assert_eq!(report.promoted_entries, BYTE_CONTEXT_TASKS.len());
    assert_eq!(report.exact_overlay_applied, 0);
    assert_eq!(report.harmonic_transfer_applied, 32);
    assert!(report.task_hint_overlay_applied > report.exact_overlay_applied);
    assert_eq!(report.exact_over_base, 0.0);
    assert!(report.harmonic_transfer_over_base <= 0.0);
    assert!(report.harmonic_transfer_ablation_max_drop > 0.0);
    assert!(
        report.harmonic_transfer_ablation_min_accuracy
            < report.harmonic_transfer_overlay.exact_accuracy
    );
    assert!(report.selective_harmonic_transfer_applied <= report.harmonic_transfer_applied);
    assert!(report.selective_harmonic_transfer_over_base <= 0.0);
    assert!(report.selective_harmonic_best_over_base <= report.task_hint_over_base);
    assert_eq!(report.cell_signature_transfer_applied, 32);
    assert!(report.cell_signature_transfer_over_base <= 0.0);
    assert!(report.cell_signature_ablation_max_drop >= 0.0);
    assert_eq!(report.trajectory_transfer_applied, 32);
    assert!(report.trajectory_transfer_over_base <= 0.0);
    assert!(report.trajectory_ablation_max_drop >= 0.0);
    assert!(report.task_hint_over_base > 0.0);
    assert_eq!(
        report.mode_status,
        "chat0_promoted_task_hint_holdout_candidate_not_mode"
    );
}

#[test]
fn chat0_promoted_state_roundtrips_corrections_only() {
    let feedback = [
        Chat0FeedbackEntry {
            prompt: String::from("manual:2: ping?"),
            response: String::from("pong"),
            expected: String::from("help"),
            feedback_correct: false,
        },
        Chat0FeedbackEntry {
            prompt: String::from("cmd ping #1 answer: "),
            response: String::from("pong"),
            expected: String::from("pong"),
            feedback_correct: true,
        },
    ];

    let state = Chat0PromotedState::from_feedback(13, 128, &feedback);
    let parsed = Chat0PromotedState::from_text(&state.to_text()).expect("state parses");

    assert_eq!(parsed.version, 1);
    assert_eq!(parsed.train_seed, 13);
    assert_eq!(parsed.cases_per_split, 128);
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].prompt, "manual:2: ping?");
    assert_eq!(parsed.entries[0].expected, "help");
    assert_eq!(parsed.entries[0].target, b'h');
}

#[test]
fn chat0_once_with_promoted_state_applies_exact_feedback_overlay() {
    let feedback = [Chat0FeedbackEntry {
        prompt: String::from("manual:2: ping?"),
        response: String::from("pong"),
        expected: String::from("help"),
        feedback_correct: false,
    }];
    let state = Chat0PromotedState::from_feedback(13, 128, &feedback);

    let base = chat0_once(13, 128, b"manual:2: ping?", Some("help"));
    let promoted =
        chat0_once_with_promoted_state(13, 128, b"manual:2: ping?", Some("help"), &state);

    assert_ne!(base.response, "help");
    assert_eq!(promoted.response, "help");
    assert_eq!(promoted.feedback_correct, Some(true));
    assert_eq!(promoted.route, "promoted_feedback_state");
    assert_eq!(promoted.mode_status, "chat0_once_answered_promoted_state");
}

#[test]
fn chat0_once_trace_exposes_answer_snapshot_and_feedback() {
    let trace = chat0_once(13, 32, b"cmd ping #1 answer: ", Some("pong"));
    assert_eq!(trace.train_seed, 13);
    assert_eq!(trace.cases_per_split, 32);
    assert!(!trace.response.is_empty());
    assert!(trace.predicted_task != "unknown");
    assert_eq!(trace.snapshot_bytes, SNAPSHOT_V1_BYTES);
    assert!(trace.coherence.is_finite());
    assert!(trace.spectral_entropy.is_finite());
    assert!(trace.feedback_correct.is_some());
    assert!(
        trace
            .to_text()
            .contains("mode_status: chat0_once_answered_eval_gated")
    );
}
