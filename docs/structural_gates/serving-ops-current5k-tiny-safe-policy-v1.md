# Serving Ops Current5k Tiny Safe Policy V1

## query

Verify the current5k serving_ops boundary: one tiny safe-policy accept is
allowed to count as verified CPU support, but it must not become a broad server
operation claim or daemon mutation path.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| serving_ops_current5k | candidate_events | 74 | `serving-ops-payload-dry-run-v1-current5k.report.json` |
| serving_ops_current5k | scoreable_payload_events | 40 | `serving-ops-payload-dry-run-v1-current5k.report.json` |
| serving_ops_current5k | output_evidence_matched_events | 33 | `serving-ops-output-evidence-v1-current5k.report.json` |
| serving_ops_current5k | verified_true_events | 21 | `serving-ops-output-evidence-v1-current5k.report.json` |
| serving_ops_current5k | verified_false_events | 12 | `serving-ops-output-evidence-v1-current5k.report.json` |
| serving_ops_current5k | local_readout_safe_policy_found | true | `serving-ops-local-accept-calibration-v1-current5k.report.json` |
| serving_ops_current5k | promoted_accept_rows | 1 | `serving-ops-safe-policy-v1-current5k.report.json` |
| serving_ops_current5k | promoted_false_accepts | 0 | `serving-ops-safe-policy-v1-current5k.report.json` |
| serving_ops_current5k | shadow_verified_safe_accepts | 1 | `serving-ops-safe-policy-v1-current5k.shadow-report.json` |
| serving_ops_current5k | shadow_false_accepts | 0 | `serving-ops-safe-policy-v1-current5k.shadow-report.json` |
| serving_ops_current5k | shelf | PROVEN_TINY_SUPPORT | `docs/CPU_CALL_CATALOG.md` |
| serving_ops_current5k | daemon_mutation_policy | disabled | `docs/CPU_CALL_CATALOG.md` |
