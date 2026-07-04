# Answer-Evidence Local Accept Calibration V1

This packet checks one narrow route: answer-evidence now has a local-accept
calibration command, but the current score geometry does not separate verifier
true rows from verifier false rows. Local accept must remain disabled.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| answer_evidence_calibration | hook_ready_rows | 9 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | label_true_rows | 3 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | label_false_rows | 6 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | safe_policy_found | false | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | best_safe_true_accepts | 0 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_promote | promotion_result | rejected_no_supported_policy | role-binding-real-traffic-answer-evidence-safe-policy-promote-v1 |
| answer_evidence_route | local_accept_status | disabled | claim boundary |
| answer_evidence_route | market_claim_allowed | false | claim boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| answer_evidence_calibration | hook_ready_rows | 9 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | label_true_rows | 3 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | label_false_rows | 6 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | safe_policy_found | false | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_calibration | best_safe_true_accepts | 0 | answer-evidence-local-accept-calibration-v1.report.json |
| answer_evidence_promote | promotion_result | rejected_no_supported_policy | role-binding-real-traffic-answer-evidence-safe-policy-promote-v1 |
| answer_evidence_route | local_accept_status | disabled | claim boundary |
| answer_evidence_route | market_claim_allowed | false | claim boundary |
