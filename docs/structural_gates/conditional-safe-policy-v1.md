# Conditional Safe Policy V1

NANDA status: `VETO` for the current numeric packet shape. The gate reports no
conflicts, no evidence gaps, no foreign pull, and route coherence 1.0, but most
exact-match candidate triads are weak under composite-mode support. Do not treat
this packet as a structural PASS until the proof shape is repaired.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| conditional safe-policy route | request_side_policy | conditional_gate_terms_prompt_len_ge_300 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_policy | energy_threshold_only | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | selected_threshold | 8192 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | request_side_policy_accept_rows | 55 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_policy_accept_rows | 2 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_policy_verified_true_rows | 2 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_policy_verified_false_rows | 0 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_policy_unverified_rows | 0 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | runtime_acceptance_mismatches | 0 | conditional-safe-policy-v1.report.json |
| conditional safe-policy route | shadow_accepts | 2 | conditional-safe-policy-v1.shadow-report.json |
| conditional safe-policy route | shadow_false_accepts | 0 | conditional-safe-policy-v1.shadow-report.json |
| conditional safe-policy route | shadow_unverified_accepts | 0 | conditional-safe-policy-v1.shadow-report.json |
| conditional safe-policy route | verified_cpu_accept_eligible_events | 2 | conditional-safe-policy-v1.verification-hook-audit.report.json |
| broad conditional route | calibration_verifier_true_rows | 17 | conditional-local-accept-calibration-v1.report.json |
| broad conditional route | calibration_verifier_false_rows | 46 | conditional-local-accept-calibration-v1.report.json |
| broad conditional route | promotion_status | not_promoted | conditional-safe-policy-v1.report.json |
| overall CPU Routability 80 | verified_cpu_accept_eligible_events | 8 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| overall CPU Routability 80 | required_verified_cpu_accepts | 800 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| overall CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| conditional safe-policy route | request_side_policy | conditional_gate_terms_prompt_len_ge_300 | candidate claim |
| conditional safe-policy route | runtime_policy | energy_threshold_only | candidate claim |
| conditional safe-policy route | selected_threshold | 8192 | candidate claim |
| conditional safe-policy route | request_side_policy_accept_rows | 55 | candidate claim |
| conditional safe-policy route | runtime_policy_accept_rows | 2 | candidate claim |
| conditional safe-policy route | runtime_policy_verified_true_rows | 2 | candidate claim |
| conditional safe-policy route | runtime_policy_verified_false_rows | 0 | candidate claim |
| conditional safe-policy route | runtime_policy_unverified_rows | 0 | candidate claim |
| conditional safe-policy route | runtime_acceptance_mismatches | 0 | candidate claim |
| conditional safe-policy route | shadow_accepts | 2 | candidate claim |
| conditional safe-policy route | shadow_false_accepts | 0 | candidate claim |
| conditional safe-policy route | shadow_unverified_accepts | 0 | candidate claim |
| conditional safe-policy route | verified_cpu_accept_eligible_events | 2 | candidate claim |
| broad conditional route | calibration_verifier_true_rows | 17 | candidate claim |
| broad conditional route | calibration_verifier_false_rows | 46 | candidate claim |
| broad conditional route | promotion_status | not_promoted | candidate claim |
| overall CPU Routability 80 | verified_cpu_accept_eligible_events | 8 | candidate claim |
| overall CPU Routability 80 | required_verified_cpu_accepts | 800 | candidate claim |
| overall CPU Routability 80 | claim_status | not_achieved | candidate claim |
