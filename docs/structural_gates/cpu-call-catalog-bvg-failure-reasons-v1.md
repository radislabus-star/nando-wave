# CPU Call Catalog BVG Failure Reasons V1

## query

Verify the CPU call catalog failure-reason refresh: `business_value_gate_failure_reason`
is a diagnostic guardrail that explains why a route is blocked. It must not
enable local accepts, count candidate rows as savings, or override the
BUSINESS_VALUE_GATE.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| cpu_call_catalog | input_window | current5k_non_synthetic_codex_trace | `cpu-operator-catalog-v1-current5k.combined.report.json:schema_version` |
| cpu_call_catalog | current_verified_cpu_accepts | 106 | `cpu-operator-catalog-v1-current5k.combined.report.json:current_verified_cpu_accepts` |
| cpu_call_catalog | current_incremental_unique_accepts_over_exact_cache | 104 | `cpu-operator-catalog-v1-current5k.combined.report.json:current_incremental_unique_cpu_accepts_over_exact_cache` |
| cpu_call_catalog | business_value_gate_passed_rows | 5 | `cpu-operator-catalog-v1-current5k.combined.report.json:business_value_gate_passed_rows` |
| bvg_failure_reason | role | diagnostic_only | `role_binding_runtime_cmd.rs:RoleBindingCpuOperatorCatalogRow.business_value_gate_failure_reason` |
| bvg_failure_reason | local_accept_authority | none | `role_binding_runtime_cmd.rs:business_value_gate_failure_reason diagnostic field` |
| bvg_failure_reason | market_claim_authority | none | `docs/CPU_CALL_CATALOG.md#business-value-gate-reason-refresh` |
| role_binding_agent_control_seed0 | failure_reason | missing_deterministic_verifier_hook_current_support_exhausted | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[route_or_family_key=role_binding_agent_control_seed0].business_value_gate_failure_reason` |
| agent_control_stop | failure_reason | missing_verifier_expected_zero_exhausted_unknown_risk | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[route_or_family_key=agent_control_stop].business_value_gate_failure_reason` |
| git_control | failure_reason | expected_zero_no_safe_policy_missing_policy | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[route_or_family_key=git_control].business_value_gate_failure_reason` |
| edit_marker_length | failure_reason | expected_zero_no_safe_policy_missing_policy | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[route_or_family_key=role_binding_edit_marker_length_seed0].business_value_gate_failure_reason` |
| blocked_routes | promotion_policy | no_threshold_lowering_without_new_evidence_or_split | `docs/EXECUTOR_REVIEW_NOTES.md#2026-07-04---executor-integration-business-value-gate-failure-reasons` |
| candidate_rows | savings_policy | not_counted_as_savings | `docs/CPU_CALL_CATALOG.md#claim-boundary` |
| broad_routes | promotion_policy | split_before_profile_work | `docs/CPU_CALL_CATALOG.md#claim-boundary` |
