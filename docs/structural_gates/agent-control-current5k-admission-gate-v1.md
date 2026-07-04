# Agent-Control Current5k Admission Gate

Query:

```text
Verify that current5k agent_control catalog rows use the current5k admission
calibration evidence, not stale 1000-window v2 support, and that no CPU80 claim
or agent_control promotion is allowed.
```

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| old_agent_control_v2_admission_audit | best_robust_true_accepts | 11 | target/nando-wave/real-traffic-shadow/agent-control-admission-audit-v1.report.json |
| old_agent_control_v2_admission_audit | current_policy_event_support_exhausted | true | target/nando-wave/real-traffic-shadow/agent-control-admission-audit-v1.report.json |
| current5k_agent_control_admission_calibration | hook_ready_rows | 476 | target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1-5k.report.json |
| current5k_agent_control_admission_calibration | label_true_rows | 35 | target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1-5k.report.json |
| current5k_agent_control_admission_calibration | label_false_rows | 441 | target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1-5k.report.json |
| current5k_agent_control_admission_calibration | robust_safe_policy_found | false | target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1-5k.report.json |
| current5k_agent_control_admission_calibration | best_robust_true_accepts | 0 | target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1-5k.report.json |
| current5k_feedback_loop | uses_agent_control_admission_calibration | agent-control-admission-calibration-v1-5k.report.json | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| current5k_feedback_loop | incremental_cpu_accept_unique_request_fingerprints | 107 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| current5k_catalog_agent_control_row | current_status | WATCH | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_agent_control_seed0 |
| current5k_catalog_agent_control_row | agent_control_admission_best_robust_true_accepts | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_agent_control_seed0 |
| current5k_catalog_agent_control_row | agent_control_current_policy_event_support_exhausted | false | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_agent_control_seed0 |
| current5k_catalog_agent_control_row | expected_unique_cpu_accepts_over_exact_cache | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_agent_control_seed0 |
| current5k_catalog_agent_control_row | forbids_old_v2_authority | stale_v2_support_not_current5k_evidence | docs/EXECUTOR_REVIEW_NOTES.md |
| current5k_catalog_agent_control_row | forbids_cpu80_claim | cpu80_not_achieved | docs/EXECUTOR_REVIEW_NOTES.md |
| current5k_catalog_agent_control_stop_row | current_status | WATCH | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=agent_control_stop |
| current5k_catalog_agent_control_stop_row | agent_control_admission_best_robust_true_accepts | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=agent_control_stop |
| current5k_catalog_agent_control_stop_row | expected_unique_cpu_accepts_over_exact_cache | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=agent_control_stop |
