# Mixed Map Current5k Safe Policy V1

## query

Verify the current5k mixed_map boundary: a request-side constrained safe policy
adds verified CPU accepts with false_accepts=0, but remains a small support row
and must not be widened into a broad mapping claim.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| mixed_map_current5k | candidate_events | 478 | `mixed-payload-dry-run-v1-current5k.report.json` |
| mixed_map_current5k | scoreable_payload_events | 37 | `mixed-payload-dry-run-v1-current5k.report.json` |
| mixed_map_current5k | output_evidence_matched_events | 34 | `mixed-output-evidence-v1-current5k.report.json` |
| mixed_map_current5k | verified_true_events | 25 | `mixed-output-evidence-v1-current5k.report.json` |
| mixed_map_current5k | verified_false_events | 9 | `mixed-output-evidence-v1-current5k.report.json` |
| mixed_map_current5k | request_admission_safe_policy_found | true | `mixed-admission-audit-v1-current5k.report.json` |
| mixed_map_current5k | promoted_policy_accepts | 6 | `mixed-safe-policy-v3-current5k.report.json` |
| mixed_map_current5k | promoted_false_accepts | 0 | `mixed-safe-policy-v3-current5k.report.json` |
| mixed_map_current5k | shadow_verified_safe_accepts | 6 | `mixed-safe-policy-v3-current5k.shadow-report.json` |
| mixed_map_current5k | shadow_false_accepts | 0 | `mixed-safe-policy-v3-current5k.shadow-report.json` |
| mixed_map_current5k | shelf | PROVEN_SMALL_SUPPORT | `docs/CPU_CALL_CATALOG.md` |
| mixed_map_current5k | widening_policy | forbidden_without_new_gate | `docs/CPU_CALL_CATALOG.md` |
