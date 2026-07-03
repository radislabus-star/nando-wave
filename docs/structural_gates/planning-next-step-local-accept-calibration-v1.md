# planning-next-step-local-accept-calibration-v1

## Claim

This packet checks that planning_next_step local-accept calibration stays a
review-only score-policy search and is not promoted into CPU savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| planning_next_step_calibration | reads | profile-registry-planning-next-step-v1.json | registry path in calibration report |
| planning_next_step_calibration | reads | planning-next-step-artifact-progress-v1.trace.jsonl | trace path in calibration report |
| planning_next_step_calibration | writes | planning-next-step-local-accept-calibration-v1.report.json | calibration report path |
| planning_next_step_calibration | evaluates | request_side_score_readout_policies | policy rows in report |
| request_side_score_readout_policies | compare_against | artifact_progress_labels | label_true_rows=1 label_false_rows=6 |
| artifact_progress_labels | come_from | tool_backed_artifact_progress_verifier | previous artifact-progress report |
| planning_next_step_calibration | does_not_enable | local_accepts | local_accepts_enabled=false |
| planning_next_step_calibration | does_not_allow | market_claim | market_claim_allowed=false |
| best_boundary_slot_margin_threshold_request_side_only | accepts | one_true_row | accepts=1 true_accepts=1 |
| best_boundary_slot_margin_threshold_request_side_only | rejects | six_false_rows | false_accepts=0 |
| singleton_true_support | blocks | production_promotion | true support is only one row |
| missing_provider_cost | blocks | savings_claim | provider_cost_microusd missing in trace/audit |
| disabled_profile_threshold | blocks | shadow_accept | registry threshold remains i32::MAX |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| calibration_report | proves | safe_policy_candidate | safe_policy_found=true |
| safe_policy_candidate | does_not_prove | CPU_savings | shadow accepts remain zero |
| safe_policy_candidate | requires_before_promotion | more_true_support | singleton calibration boundary |
| safe_policy_candidate | requires_before_promotion | provider_cost | market claim boundary |
| safe_policy_candidate | requires_before_promotion | promoted_shadow_audit | false_accepts=0 and unverified accepts=0 required |
