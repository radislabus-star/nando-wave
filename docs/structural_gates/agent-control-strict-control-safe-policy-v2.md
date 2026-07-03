# agent-control-strict-control-safe-policy-v2

## Claim

The agent-control route now has a stricter request-side safe policy that
raises verifier-backed local CPU accepts from 3 to 11 on the current real Codex
trace window while keeping broad agent-control blocked and keeping the global
CPU Routability 80 claim closed.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| agent_control_broad_route | has_label_true_rows | 12 | agent_control_admission_v2.label_true_rows |
| agent_control_broad_route | has_label_false_rows | 112 | agent_control_admission_v2.label_false_rows |
| previous_hard_stop_policy | admitted_true_rows | 3 | agent_control_admission_v1.best_robust_true_accepts |
| previous_hard_stop_policy | admitted_false_rows | 0 | agent_control_admission_v1.robust_policy_false_accepts |
| strict_control_stop_forms_policy | is_request_side_policy | true | role_binding_runtime_cmd.rs.policy_name |
| strict_control_stop_forms_policy | admits_true_rows | 11 | agent_control_admission_v2.policy.true_accepts |
| strict_control_stop_forms_policy | admits_false_rows | 0 | agent_control_admission_v2.policy.false_accepts |
| strict_control_stop_forms_policy | misses_true_rows | 1 | agent_control_admission_v2.policy.missed_true |
| strict_control_stop_forms_policy | uses_feature | one_token_lowercase_stop | role_binding_runtime_cmd.rs.feature.one_token_lowercase_stop |
| strict_control_stop_forms_policy | uses_feature | stop_uppercase_goal_control | role_binding_runtime_cmd.rs.feature.stop_uppercase_goal_control |
| agent_control_safe_policy_v2 | selected_policy | strict_control_stop_forms | agent_control_safe_policy_v2.selected_policy_name |
| agent_control_safe_policy_v2 | writes_trace | agent-control-safe-policy-v2.trace.jsonl | agent_control_safe_policy_v2.promoted_trace_path |
| agent_control_safe_policy_v2 | policy_accept_rows | 11 | agent_control_safe_policy_v2.policy_accept_rows |
| agent_control_safe_policy_v2 | policy_accept_verified_true_rows | 11 | agent_control_safe_policy_v2.policy_accept_verified_true_rows |
| agent_control_safe_policy_v2 | policy_accept_verified_false_rows | 0 | agent_control_safe_policy_v2.policy_accept_verified_false_rows |
| agent_control_safe_policy_v2 | policy_accept_unverified_rows | 0 | agent_control_safe_policy_v2.policy_accept_unverified_rows |
| agent_control_safe_policy_v2 | runtime_acceptance_mismatches | 0 | agent_control_safe_policy_v2.runtime_acceptance_mismatches |
| agent_control_shadow_v2 | shadow_accepts | 11 | agent_control_shadow_v2.nando_shadow_accepts |
| agent_control_shadow_v2 | verified_safe_accepts | 11 | agent_control_shadow_v2.verified_safe_accepts |
| agent_control_shadow_v2 | false_accepts | 0 | agent_control_shadow_v2.false_accepts |
| agent_control_shadow_v2 | unverified_shadow_accepts | 0 | agent_control_shadow_v2.unverified_shadow_accepts |
| agent_control_shadow_v2 | incremental_savings_over_exact_cache | 6 | agent_control_shadow_v2.incremental_savings |
| agent_control_shadow_v2 | p99_shadow_score_latency_ns | 23783 | agent_control_shadow_v2.p99 |
| agent_control_audit_v2 | verification_hook_ready_events | 11 | agent_control_audit_v2.verification_hook_ready_events |
| agent_control_audit_v2 | verified_cpu_accept_eligible_events | 11 | agent_control_audit_v2.verified_cpu_accept_eligible_events |
| agent_control_audit_v2 | false_accepts | 0 | agent_control_audit_v2.route_false_accepts |
| feedback_loop_v2 | verified_cpu_accept_eligible_events | 16 | feedback_v2.verified_cpu_accepts |
| feedback_loop_v2 | verified_cpu_routability_milli | 16 | feedback_v2.verified_cpu_routability_milli |
| feedback_loop_v2 | verified_gap_to_80_calls | 784 | feedback_v2.verified_gap_to_80 |
| feedback_loop_v2 | market_claim_allowed | false | feedback_v2.market_claim_allowed |
| cpu_operator_catalog_v2 | current_verified_cpu_accepts | 16 | catalog_v2.current_verified_cpu_accepts |
| cpu_operator_catalog_v2 | top_catalog_row | role_binding_agent_control_seed0 | catalog_v2.top_catalog_row |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| strict_control_stop_forms_policy | improves | agent_control_verified_cpu_accepts | 3_to_11_route_gain |
| broad_agent_control_route | remains | blocked | 112 false verifier labels |
| global_cpu_routability_80 | remains | not_achieved | 16_of_1000_verified |
| unified_feedback_v2 | does_not_allow | market_wide_savings_claim | verified_cpu_routability_milli_16 |
| next_repair | should_target | conditional_or_planning_support | largest remaining verified gap |
