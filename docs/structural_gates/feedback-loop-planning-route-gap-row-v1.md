# feedback-loop-planning-route-gap-row-v1

## Claim

The CPU route feedback loop now counts planning_next_step route-gap traffic in
the global dashboard without promoting its singleton safe-policy candidate into
verified CPU savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| cpu_route_feedback_loop_v1 | reads | cpu-route-forecast-v1.report.json | base forecast report |
| cpu_route_feedback_loop_v1 | reads | planning-next-step-payload-dry-run-v1.report.json | planning dry-run report |
| cpu_route_feedback_loop_v1 | reads | planning-next-step-local-accept-calibration-v1.report.json | planning calibration report |
| cpu_route_feedback_loop_v1 | reads | planning-next-step-artifact-progress-v1.verification-hook-audit.report.json | planning audit report |
| base_forecast_routes | include | conditional_edit_mixed | existing forecast routes |
| planning_next_step | lives_in | route_gap_side_channel | not present in base forecast route list |
| feedback_dashboard | appends | planning_next_step_route_row | route row in feedback report |
| planning_next_step_route_row | has_candidate_events | 14 | feedback report metrics |
| planning_next_step_route_row | has_scoreable_payload_events | 14 | feedback report metrics |
| planning_next_step_route_row | has_verification_hook_ready_events | 7 | feedback report metrics |
| planning_next_step_route_row | has_verified_cpu_accept_eligible_events | 0 | feedback report metrics |
| planning_next_step_route_row | has_false_accepts | 0 | feedback report metrics |
| planning_next_step_route_row | has_safe_policy_found | true | calibration report |
| planning_next_step_route_row | has_best_safe_true_accepts | 1 | calibration report |
| planning_next_step_route_row | requires_minimum_true_support | 3 | feedback support guard |
| planning_next_step_route_row | has_support_qualified | false | feedback report |
| planning_next_step_route_row | has_stage | local_accept_calibration_support_insufficient | feedback report |
| verified_cpu_routability | remains | 8_of_1000 | feedback report |
| market_claim | remains | disallowed | feedback claim boundary |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| planning_next_step_safe_policy_candidate | is_visible_in | feedback_dashboard | safe_policy_found=true |
| planning_next_step_safe_policy_candidate | is_blocked_by | minimum_true_support | 1 < 3 |
| planning_next_step_candidate_traffic | increases | operator_candidate_calls | 288 -> 302 |
| planning_next_step_scoreable_traffic | increases | scoreable_candidate_calls | 79 -> 93 |
| planning_next_step_route_row | does_not_increase | verified_cpu_accepts | verified accepts stay 8 |
| singleton_policy | does_not_prove | market_savings | support insufficient and provider cost missing |
