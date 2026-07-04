# Edit Admission Code-Diff Question-Mark Current5k Gate

Query:

```text
Verify that the current5k edit_marker_length code_diff_and_question_mark
admission policy is only a request-side candidate, not a CPU savings claim or
local-accept promotion.
```

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| current5k_edit_admission_calibration | verdict | EDIT_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | hook_ready_rows | 42 | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | label_true_rows | 14 | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | label_false_rows | 28 | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| code_diff_and_question_mark_policy | true_accepts | 4 | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json#policy=code_diff_and_question_mark |
| code_diff_and_question_mark_policy | false_accepts | 0 | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json#policy=code_diff_and_question_mark |
| code_diff_and_question_mark_policy | uses_prompt_side_feature_pair | has_code_diff_lines_and_has_question_mark | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| current5k_edit_admission_calibration | response_text_used_for_features | false | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | target_labels_used_for_runtime | false | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | proof_labels_used_for_runtime | false | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | local_accepts_enabled | false | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_edit_admission_calibration | market_claim_allowed | false | target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json |
| current5k_catalog_edit_marker_row | current_status | WATCH | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| current5k_catalog_edit_marker_row | edit_admission_best_robust_true_accepts | 4 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| current5k_catalog_edit_marker_row | expected_unique_cpu_accepts_over_exact_cache | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| current5k_catalog_edit_marker_row | next_valid_step | separate_promoted_shadow_audit | docs/EXECUTOR_REVIEW_NOTES.md |
| cpu80_status | achieved | false | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
