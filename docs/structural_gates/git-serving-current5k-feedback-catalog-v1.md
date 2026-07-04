# Git Serving Current5k Feedback Catalog V1

## query

Verify that the current5k feedback loop now reads git_control and serving_ops
companion evidence from the same current5k window, and that the CPU catalog
keeps the correct business-value boundary: git_control remains blocked, while
serving_ops is only tiny proven support.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| feedback_loop_current5k | reads_git_dry_run_report | git-control-payload-dry-run-v1-current5k.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.git_control_dry_run_report_path` |
| feedback_loop_current5k | reads_git_verification_audit | git-control-output-evidence-v1-current5k.verification-hook-audit.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.git_control_verification_audit_report_path` |
| feedback_loop_current5k | reads_git_local_accept_calibration | git-control-local-accept-calibration-v1-current5k.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.git_control_local_accept_calibration_report_path` |
| feedback_loop_current5k | reads_serving_dry_run_report | serving-ops-payload-dry-run-v1-current5k.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.serving_ops_dry_run_report_path` |
| feedback_loop_current5k | reads_serving_verification_audit | serving-ops-output-evidence-v1-current5k.verification-hook-audit.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.serving_ops_verification_audit_report_path` |
| feedback_loop_current5k | reads_serving_local_accept_calibration | serving-ops-local-accept-calibration-v1-current5k.report.json | `cpu-route-feedback-loop-v1-current5k.combined.report.json.serving_ops_local_accept_calibration_report_path` |
| feedback_loop_current5k | total_llm_calls | 5000 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| feedback_loop_current5k | scoreable_candidate_calls | 599 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| feedback_loop_current5k | verification_hook_ready_events | 392 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| feedback_loop_current5k | verified_cpu_accept_unique_request_fingerprints | 106 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| feedback_loop_current5k | incremental_cpu_accept_unique_request_fingerprints | 104 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| git_control_feedback_route | candidate_events | 123 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_feedback_route | scoreable_payload_events | 90 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_feedback_route | verification_hook_ready_events | 74 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_feedback_route | verified_cpu_accept_eligible_events | 0 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_feedback_route | false_accepts | 0 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_feedback_route | stage | local_accept_calibration_failed | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.git_control` |
| git_control_catalog_existing_profile | status | CANDIDATE | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.git_control.existing_profile_route` |
| git_control_catalog_existing_profile | expected_unique_cpu_accepts_over_exact_cache | 0 | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.git_control.existing_profile_route` |
| git_control_catalog_existing_profile | business_value_gate_failure_reason | expected_unique_cpu_accepts_zero,no_safe_local_accept_policy,safe_policy_missing | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.git_control.existing_profile_route` |
| serving_ops_feedback_route | candidate_events | 74 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_feedback_route | scoreable_payload_events | 40 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_feedback_route | verification_hook_ready_events | 33 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_feedback_route | verified_cpu_accept_eligible_events | 1 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_feedback_route | false_accepts | 0 | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_feedback_route | stage | verified_cpu_accept_eligible | `cpu-route-feedback-loop-v1-current5k.combined.report.json.routes.serving_ops` |
| serving_ops_catalog_existing_profile | status | PROVEN | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.serving_ops.existing_profile_route` |
| serving_ops_catalog_existing_profile | expected_unique_cpu_accepts_over_exact_cache | 1 | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.serving_ops.existing_profile_route` |
| serving_ops_catalog_existing_profile | business_value_gate_failure_reason | PASSED | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.serving_ops.existing_profile_route` |
| serving_ops_catalog_existing_profile | next_action | non_synthetic_soak_before_market_claim | `cpu-operator-catalog-v1-current5k.combined.report.json.rows.serving_ops.existing_profile_route` |
| current5k_promotion_policy | git_control_local_accepts | disabled | `docs/CPU_CALL_CATALOG.md` |
| current5k_promotion_policy | serving_ops_daemon_mutations | disabled | `docs/CPU_CALL_CATALOG.md` |
| current5k_promotion_policy | new_cpu_accepts_promoted_by_this_patch | 0 | `docs/EXECUTOR_REVIEW_NOTES.md` |
