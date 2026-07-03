# planning-next-step-admission-calibration-v1

## Claim

The planning_next_step route now has a request-side admission diagnostic that
can separate the two known artifact-progress true rows from goal-control false
collisions on the v3 real Codex trace window, while keeping local accepts and
market claims disabled.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| admission_command | is_named | role-binding-real-traffic-planning-next-step-admission-calibration-v1 | CLI command |
| admission_command | reads | planning-next-step-artifact-progress-v3.trace.jsonl | v3 calibration run |
| admission_command | reads | codex_history_jsonl | history argument |
| admission_command | writes | planning-next-step-admission-calibration-v3.report.json | v3 calibration report |
| admission_command | does_not_write | raw_prompt_text | report stores fingerprints and features |
| admission_command | sets | response_text_used_for_features_false | v3_report.response_text_used_for_features |
| admission_command | sets | target_labels_used_for_runtime_false | v3_report.target_labels_used_for_runtime |
| admission_command | sets | proof_labels_used_for_runtime_false | v3_report.proof_labels_used_for_runtime |
| admission_command | sets | local_accepts_enabled_false | v3_report.local_accepts_enabled |
| admission_command | sets | market_claim_allowed_false | v3_report.market_claim_allowed |
| v3_admission_window | has_hook_ready_rows | 29 | v3_report.hook_ready_rows |
| v3_admission_window | has_label_true_rows | 2 | v3_report.label_true_rows |
| v3_admission_window | has_label_false_rows | 27 | v3_report.label_false_rows |
| direct_action_project_policy | admits_true_rows | 2 | v3_policy.direct_action_project.true_accepts |
| direct_action_project_policy | admits_false_rows | 2 | v3_policy.direct_action_project.false_accepts |
| goal_control_feature | marks_false_rows | 4 | v3_feature_counts.has_goal_control.false |
| goal_control_feature | marks_true_rows | 0 | v3_feature_counts.has_goal_control.true |
| direct_action_project_no_goal_control_policy | admits_true_rows | 2 | v3_policy.no_goal_control.true_accepts |
| direct_action_project_no_goal_control_policy | admits_false_rows | 0 | v3_policy.no_goal_control.false_accepts |
| direct_action_project_no_goal_control_policy | misses_true_rows | 0 | v3_policy.no_goal_control.missed_true |
| direct_action_project_no_goal_control_policy | is_robust_safe | true | v3_policy.no_goal_control.robust_safe |
| feedback_loop_v3 | has_operator_candidate_calls | 337 | planning_v3_feedback.operator_candidate_calls |
| feedback_loop_v3 | has_scoreable_candidate_calls | 128 | planning_v3_feedback.scoreable_candidate_calls |
| feedback_loop_v3 | has_verification_hook_ready_events | 94 | planning_v3_feedback.verification_hook_ready_events |
| feedback_loop_v3 | has_verified_cpu_accepts | 8 | planning_v3_feedback.verified_cpu_accepts |
| planning_next_step_feedback_row | has_verified_cpu_accepts | 0 | planning_v3_feedback.planning_row_verified |
| planning_next_step_feedback_row | has_support_qualified | false | planning_v3_feedback.planning_row_support |
| cpu_routability_goal | has_gap_to_80_calls | 792 | planning_v3_feedback.gap_to_80 |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| admission_split | improves | request_side_route_quality | separates goal-control false collisions |
| admission_split | does_not_enable | local_cpu_accept | local_accepts_enabled=false |
| robust_policy_candidate | requires | larger_support_window | true support is only two rows |
| feedback_loop_v3 | remains | review_only | verified CPU stays 8/1000 |
| next_repair | should_target | promoted_shadow_audit | need verified accepts to increase with false_accepts=0 |
| market_claim | remains | disallowed | market_claim_allowed=false |
