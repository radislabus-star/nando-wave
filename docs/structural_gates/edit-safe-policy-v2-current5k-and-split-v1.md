# Edit Safe-Policy V2 Current5k And Runtime Command Split Gate

Query:

```text
Verify that edit safe-policy v2 is a tiny current5k PROVEN row, that the
runtime command split preserves the same edit-route behavior, and that CPU80
is still not achieved.
```

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| edit_safe_policy_v2_promote | verdict | EDIT_SAFE_POLICY_PROMOTE_V2_REVIEW_PROMOTED_TRACE_READY | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_promote | request_side_policy_name | code_diff_and_question_mark | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_promote | selected_policy_threshold | 7680 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_promote | policy_accept_rows | 3 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_promote | policy_accept_verified_true_rows | 3 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_promote | policy_accept_verified_false_rows | 0 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| edit_safe_policy_v2_shadow | verdict | REAL_TRAFFIC_SHADOW_V1_PASS | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.shadow-report.json |
| edit_safe_policy_v2_shadow | total_requests | 5000 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.shadow-report.json |
| edit_safe_policy_v2_shadow | verified_safe_accepts | 3 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.shadow-report.json |
| edit_safe_policy_v2_shadow | false_accepts | 0 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.shadow-report.json |
| edit_safe_policy_v2_shadow | synthetic_trace_used | false | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.shadow-report.json |
| edit_safe_policy_v2_audit | verified_cpu_accept_eligible_events | 3 | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.verification-hook-audit.report.json |
| current5k_feedback | incremental_unique_cpu_accepts | 110 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| current5k_feedback | incremental_unique_gap_to_80_calls | 3890 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| current5k_catalog_edit_row | current_status | PROVEN | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| current5k_catalog_edit_row | expected_unique_cpu_accepts_over_exact_cache | 3 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| current5k_catalog_edit_row | false_accept_risk | LOW_VERIFIED_POLICY_ZERO_FALSE_ACCEPTS | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=role_binding_edit_marker_length_seed0 |
| runtime_command_split | extracted_route_block | edit_safe_policy_admission_commands | crates/nando-cli/src/role_binding_runtime_cmd/edit_safe_policy.rs |
| runtime_command_split | preserves_module_scope | include_from_role_binding_runtime_cmd | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| runtime_command_split | public_command_registry_changed_for | edit_safe_policy_promote_v2 | crates/nando-cli/src/main.rs |
| runtime_command_split | target_labels_used_for_runtime | false | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| runtime_command_split | proof_labels_used_for_runtime | false | target/nando-wave/real-traffic-shadow/edit-safe-policy-v2-current5k.report.json |
| cpu80_status | achieved | false | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |

## Claim Boundary

This packet allows the narrow claim:

```text
edit_marker_length safe-policy v2 adds 3 incremental unique verified CPU
accepts over exact cache on current5k with false_accepts=0.
```

It does not allow:

```text
CPU80 achieved
broad edit route solved
global market savings claim
local accepts without shadow/audit/verifier
```
