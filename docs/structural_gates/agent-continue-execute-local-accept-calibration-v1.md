# Agent Continue Execute Local-Accept Calibration V1

This packet checks a narrow claim boundary: `agent_continue_execute` now has
request-side local-accept calibration, but the support is too weak for
promotion and it is not verified CPU savings.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | agent_continue_execute_calibration | hook_ready_rows | 25 | agent-continue-execute-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | measured value | agent-continue | calibration |
| t2 | agent_continue_execute_calibration | label_true_rows | 6 | agent-continue-execute-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | verifier label count | agent-continue | calibration |
| t3 | agent_continue_execute_calibration | label_false_rows | 19 | agent-continue-execute-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | verifier label count | agent-continue | calibration |
| t4 | agent_continue_execute_calibration | best_safe_true_accepts | 1 | agent-continue-execute-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | weak support count | agent-continue | calibration |
| t5 | agent_continue_execute_feedback_route | stage | local_accept_calibration_support_insufficient | cpu-route-feedback-loop-v1.report.json | 0.99 | feedback route row | route stage | agent-continue | feedback |
| t6 | agent_continue_execute_feedback_route | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json | 0.99 | feedback route row | measured value | agent-continue | feedback |
| t7 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | cpu-route-feedback-loop-v1.report.json | 0.99 | global CPU80 state | measured value | feedback-loop | current-state |
| t8 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | cpu-route-feedback-loop-v1.report.json | 0.99 | global CPU80 state | measured value | feedback-loop | current-state |
| t9 | market_claim_boundary | allows_market_claim | false | cpu-route-feedback-loop-v1.report.json | 0.99 | claim boundary | permission state | claim-boundary | boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | agent_continue_execute_calibration | hook_ready_rows | 25 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | calibration artifact | measured value | agent-continue | calibration |
| c2 | agent_continue_execute_calibration | label_true_rows | 6 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | calibration artifact | verifier label count | agent-continue | calibration |
| c3 | agent_continue_execute_calibration | label_false_rows | 19 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | calibration artifact | verifier label count | agent-continue | calibration |
| c4 | agent_continue_execute_calibration | best_safe_true_accepts | 1 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | calibration artifact | weak support count | agent-continue | calibration |
| c5 | agent_continue_execute_feedback_route | stage | local_accept_calibration_support_insufficient | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | feedback route row | route stage | agent-continue | feedback |
| c6 | agent_continue_execute_feedback_route | verified_cpu_accept_eligible_events | 0 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | feedback route row | measured value | agent-continue | feedback |
| c7 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | global CPU80 state | measured value | feedback-loop | current-state |
| c8 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | global CPU80 state | measured value | feedback-loop | current-state |
| c9 | market_claim_boundary | allows_market_claim | false | docs/EXECUTOR_REVIEW_NOTES.md Agent Continue Execute Local-Accept Calibration V1 | 0.95 | claim boundary | permission state | claim-boundary | boundary |
