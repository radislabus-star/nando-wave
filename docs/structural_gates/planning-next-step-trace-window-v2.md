# planning-next-step-trace-window-v2

## Claim

The wider real Codex history window improves planning_next_step evidence
density, but it still must not promote local CPU accepts because the safe policy
support remains below the minimum promotion threshold.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| planning_next_step_v2 | reads | codex_history_window_5000 | dry-run command max-events |
| planning_next_step_v1 | read | codex_history_window_1000 | v1 baseline |
| planning_next_step_v2 | writes | planning-next-step-payload-dry-run-v2.report.json | v2 dry-run artifact path |
| planning_next_step_v2 | writes | planning-next-step-artifact-progress-v2.report.json | v2 artifact-progress artifact path |
| planning_next_step_v2 | writes | planning-next-step-local-accept-calibration-v2.report.json | v2 calibration artifact path |
| planning_candidates | increase | 54_to_170 | v1/v2 dry-run reports |
| scoreable_payload_events | increase | 14_to_31 | v1/v2 dry-run reports |
| artifact_evidence_matches | increase | 7_to_22 | v1/v2 artifact reports |
| verified_true_events | increase | 1_to_2 | v1/v2 artifact reports |
| verified_false_events | increase | 6_to_20 | v1/v2 artifact reports |
| verification_hook_ready_events | increase | 7_to_22 | v1/v2 audit reports |
| safe_policy_found | remains | true | v1/v2 calibration reports |
| best_safe_true_accepts | remains | 1 | v1/v2 calibration reports |
| margin_only_all_true_safe_accept | remains | false | v2 calibration margin diagnostics |
| end_slot_margin_false_collision | equals | 8 | v2 calibration margin diagnostics |
| min_slot_margin_false_collision | equals | 18 | v2 calibration margin diagnostics |
| energy_margin_false_collision | equals | 2 | v2 calibration margin diagnostics |
| minimum_true_support | remains | 3 | promotion guard |
| verified_cpu_accept_eligible_events | remains | 0 | v2 audit field verified_cpu_accept_eligible_events |
| shadow_false_accepts | remains | 0 | v2 audit field shadow_false_accepts |
| market_claim_allowed | remains | false | v2 audit field market_claim_allowed |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| wider_trace_window | proves | more_planning_route_evidence | candidates and hook-ready rows increased |
| wider_trace_window | does_not_prove | cpu_savings | verified CPU accepts stay zero |
| second_true_label | is_missed_by | current_safe_policy | v2 calibration missed_true=1 |
| all_current_margin_thresholds | cannot_accept | both_true_rows_without_false | v2 margin collision diagnostics |
| current_safe_policy | is_blocked_by | minimum_true_support | best safe true accepts 1 < 3 |
| planning_next_step | should_remain | review_only | market claim disallowed |
| next_repair | should_target | admission_feature_split | need a new request-side feature, not lower threshold |
