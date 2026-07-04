# Answer Evidence Local-Accept Feedback V1

This packet checks a narrow claim boundary: `answer_or_explain` has now run
local-accept calibration and the result is a failed readout policy, not verified
CPU savings.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | answer_evidence_calibration | hook_ready_rows | 9 | answer-evidence-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | measured value | answer-evidence | calibration |
| t2 | answer_evidence_calibration | label_true_rows | 3 | answer-evidence-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | verifier label count | answer-evidence | calibration |
| t3 | answer_evidence_calibration | label_false_rows | 6 | answer-evidence-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | verifier label count | answer-evidence | calibration |
| t4 | answer_evidence_calibration | safe_policy_found | false | answer-evidence-local-accept-calibration-v1.report.json | 0.99 | calibration artifact | policy verdict | answer-evidence | calibration |
| t5 | answer_evidence_feedback_route | stage | local_accept_calibration_failed | cpu-route-feedback-loop-v1.report.json | 0.99 | feedback route row | route stage | answer-evidence | feedback |
| t6 | answer_evidence_feedback_route | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json | 0.99 | feedback route row | measured value | answer-evidence | feedback |
| t7 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | cpu-route-feedback-loop-v1.report.json | 0.99 | global CPU80 state | measured value | feedback-loop | current-state |
| t8 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | cpu-route-feedback-loop-v1.report.json | 0.99 | global CPU80 state | measured value | feedback-loop | current-state |
| t9 | market_claim_boundary | allows_market_claim | false | cpu-route-feedback-loop-v1.report.json | 0.99 | claim boundary | permission state | claim-boundary | boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | answer_evidence_calibration | hook_ready_rows | 9 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | calibration artifact | measured value | answer-evidence | calibration |
| c2 | answer_evidence_calibration | label_true_rows | 3 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | calibration artifact | verifier label count | answer-evidence | calibration |
| c3 | answer_evidence_calibration | label_false_rows | 6 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | calibration artifact | verifier label count | answer-evidence | calibration |
| c4 | answer_evidence_calibration | safe_policy_found | false | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | calibration artifact | policy verdict | answer-evidence | calibration |
| c5 | answer_evidence_feedback_route | stage | local_accept_calibration_failed | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | feedback route row | route stage | answer-evidence | feedback |
| c6 | answer_evidence_feedback_route | verified_cpu_accept_eligible_events | 0 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | feedback route row | measured value | answer-evidence | feedback |
| c7 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | global CPU80 state | measured value | feedback-loop | current-state |
| c8 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | global CPU80 state | measured value | feedback-loop | current-state |
| c9 | market_claim_boundary | allows_market_claim | false | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Local-Accept Calibration Feedback V1 | 0.95 | claim boundary | permission state | claim-boundary | boundary |
