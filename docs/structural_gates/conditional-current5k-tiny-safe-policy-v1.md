# Conditional Current5k Tiny Safe Policy V1

## query

Verify the current5k conditional_branch boundary: a tiny request-side v2 safe
policy adds verified CPU accepts with false_accepts=0, but the broad
conditional route remains unsafe and must not be widened.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| conditional_current5k | candidate_events | 456 | `conditional-payload-dry-run-v1-current5k.report.json` |
| conditional_current5k | payload_ready_events | 147 | `conditional-payload-readiness-v1-current5k.report.json` |
| conditional_current5k | scoreable_payload_events | 147 | `conditional-payload-dry-run-v1-current5k.report.json` |
| conditional_current5k | output_evidence_matched_events | 134 | `conditional-output-evidence-v1-current5k.report.json` |
| conditional_current5k | verified_true_events | 36 | `conditional-output-evidence-v1-current5k.report.json` |
| conditional_current5k | verified_false_events | 98 | `conditional-output-evidence-v1-current5k.report.json` |
| conditional_current5k | local_readout_safe_policy_found | false | `conditional-local-accept-calibration-v1-current5k.report.json` |
| conditional_current5k | request_admission_safe_policy_found | true | `conditional-admission-audit-v1-current5k.report.json` |
| conditional_current5k | promoted_runtime_accepts | 3 | `conditional-safe-policy-v2-current5k.report.json` |
| conditional_current5k | promoted_false_accepts | 0 | `conditional-safe-policy-v2-current5k.report.json` |
| conditional_current5k | promoted_unverified_accepts | 0 | `conditional-safe-policy-v2-current5k.report.json` |
| conditional_current5k | shadow_verified_safe_accepts | 3 | `conditional-safe-policy-v2-current5k.shadow-report.json` |
| conditional_current5k | shadow_false_accepts | 0 | `conditional-safe-policy-v2-current5k.shadow-report.json` |
| conditional_current5k | incremental_unique_accepts | 3 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| conditional_current5k | shelf | PROVEN_TINY_SUBSET | `docs/CPU_CALL_CATALOG.md` |
| conditional_current5k | broad_route_widening_policy | forbidden_without_new_gate | `docs/CPU_CALL_CATALOG.md` |
