# Test Output Parse 5k Window V1

## query

Check that the 5k `test_output_parse` attribution uses the 5k Codex
route-candidate window, counts only verified CPU accepts with output evidence,
keeps exact-cache overlap separate, and does not promote CPU80 or replace the
canonical 1000-window catalog.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| base_window | source_trace | codex_history_route_candidates_v1_5k_events | test-output-parse-safe-policy-window-v1-5k.report.json |
| base_window | total_llm_calls | 5000 | test-output-parse-safe-policy-window-shadow-v1-5k.report.json |
| base_window | exact_cache_hits | 459 | test-output-parse-safe-policy-window-shadow-v1-5k.report.json |
| trace_reader | accepts_alias | event_id_as_trace_id | role_binding_runtime_cmd.rs |
| trace_reader | normalizes_schema | event_v1_to_trace_v1 | role_binding_runtime_cmd.rs |
| promoted_route | route_key | test_output_parse | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| promoted_route | promoted_route_rows | 104 | test-output-parse-safe-policy-window-v1-5k.report.json |
| promoted_route | promoted_rows_inserted | 97 | test-output-parse-safe-policy-window-v1-5k.report.json |
| promoted_route | missing_base_match_rows | 0 | test-output-parse-safe-policy-window-v1-5k.report.json |
| promoted_route | exact_cache_overlap_promoted_rows | 2 | test-output-parse-safe-policy-window-v1-5k.report.json |
| shadow_report | verified_safe_accepts | 97 | test-output-parse-safe-policy-window-shadow-v1-5k.report.json |
| shadow_report | false_accepts | 0 | test-output-parse-safe-policy-window-shadow-v1-5k.report.json |
| shadow_report | incremental_savings_over_exact_cache | 95 | test-output-parse-safe-policy-window-shadow-v1-5k.report.json |
| verification_audit | scoreable_candidate_calls | 97 | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| verification_audit | verification_hook_ready_events | 97 | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| verification_audit | verified_cpu_accept_eligible_events | 97 | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| verification_audit | verified_true_events | 97 | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| verification_audit | verified_false_events | 0 | test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json |
| feedback_report | verified_cpu_accept_unique_request_fingerprints | 95 | cpu-route-feedback-loop-v1-5k.test-output-window.report.json |
| feedback_report | incremental_cpu_accept_unique_request_fingerprints | 93 | cpu-route-feedback-loop-v1-5k.test-output-window.report.json |
| feedback_report | exact_cache_overlap_verified_cpu_accepts | 2 | cpu-route-feedback-loop-v1-5k.test-output-window.report.json |
| feedback_report | incremental_cpu_accept_unique_reduction_milli | 18 | cpu-route-feedback-loop-v1-5k.test-output-window.report.json |
| feedback_report | incremental_unique_gap_to_80_calls | 3907 | cpu-route-feedback-loop-v1-5k.test-output-window.report.json |
| canonical_catalog | remains | 1000_window_current_snapshot | CPU_CALL_CATALOG.md |
| sidecar_5k | status | evidence_not_catalog_replacement | CPU_CALL_CATALOG.md |
| cpu80_claim | remains | not_proven | EXECUTOR_REVIEW_NOTES.md |
