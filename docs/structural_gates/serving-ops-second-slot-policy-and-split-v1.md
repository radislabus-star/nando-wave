# Serving Ops Second-Slot Policy And Command Split Gate

Query:

```text
Verify that serving_ops was split into a route-owned command include and that
the candidate second-slot admission policy was rejected when it failed the
real-traffic safety gate. Preserve current safe serving_ops fallback and CPU80
claim boundary.
```

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_ops_command_split | extracted_route_block | serving_ops_real_traffic_public_commands | crates/nando-cli/src/role_binding_runtime_cmd/serving_ops.rs |
| role_binding_runtime_cmd | includes | role_binding_runtime_cmd/serving_ops.rs | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| serving_ops_command_split | preserves_module_scope | include_from_role_binding_runtime_cmd | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| serving_ops_command_split | moved_lines | 1084 | mechanical function-boundary extraction |
| serving_ops_command_split | changed_cli_dispatch | false | crates/nando-cli/src/main.rs unchanged |
| serving_ops_command_split | changed_help_surface | false | crates/nando-cli/src/help.rs unchanged |
| serving_ops_second_slot_policy | added_runtime_policy | second_slot_threshold | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| serving_ops_second_slot_policy | accepted_without_second_slot | false | profile_accepts_score uses None and rejects second_slot_threshold |
| serving_ops_runtime_score | computes_second_slot_margin | true | score_role_binding_profile_request records slot index 1 |
| serving_ops_calibration_current5k | best_metric_slot_margin_threshold_true_accepts | 6 | target/nando-wave/real-traffic-shadow/serving-ops-local-accept-calibration-v1-current5k.report.json |
| serving_ops_calibration_current5k | best_metric_slot_margin_threshold_false_accepts | 0 | target/nando-wave/real-traffic-shadow/serving-ops-local-accept-calibration-v1-current5k.report.json |
| serving_ops_calibration_current5k | best_metric_slot_margin_threshold | 409600 | target/nando-wave/real-traffic-shadow/serving-ops-local-accept-calibration-v1-current5k.report.json |
| serving_ops_second_slot_probe | registry_path | target/nando-wave/real-traffic-shadow/profile-registry-serving-ops-second-slot-probe-v1-current5k.json | probe artifact |
| serving_ops_second_slot_probe | nando_shadow_accepts | 7 | target/nando-wave/real-traffic-shadow/serving-ops-second-slot-probe-v1-current5k.shadow-report.json |
| serving_ops_second_slot_probe | verified_safe_accepts | 6 | target/nando-wave/real-traffic-shadow/serving-ops-second-slot-probe-v1-current5k.shadow-report.json |
| serving_ops_second_slot_probe | false_accepts | 1 | target/nando-wave/real-traffic-shadow/serving-ops-second-slot-probe-v1-current5k.shadow-report.json |
| serving_ops_second_slot_probe | market_claim_allowed | false | target/nando-wave/real-traffic-shadow/serving-ops-second-slot-probe-v1-current5k.verification-hook-audit.report.json |
| serving_ops_second_slot_probe | promoted | false | false_accepts=1 blocks promotion |
| serving_ops_safe_policy_current5k | selected_acceptance_policy | energy_threshold_only | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.report.json |
| serving_ops_safe_policy_current5k | selected_policy_name | market_safe_energy_margin_threshold | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.report.json |
| serving_ops_safe_policy_current5k | nando_shadow_accepts | 1 | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.shadow-report.json |
| serving_ops_safe_policy_current5k | verified_safe_accepts | 1 | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.shadow-report.json |
| serving_ops_safe_policy_current5k | false_accepts | 0 | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.shadow-report.json |
| serving_ops_safe_policy_current5k | market_claim_allowed | true | target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.verification-hook-audit.report.json |
| feedback_loop_current5k | total_llm_calls | 5000 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| feedback_loop_current5k | verified_cpu_accept_unique_request_fingerprints | 112 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| feedback_loop_current5k | incremental_cpu_accept_unique_request_fingerprints | 110 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| feedback_loop_current5k | verified_gap_to_80_calls | 3886 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| feedback_loop_current5k | unique_verified_gap_to_80_calls | 3888 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json |
| feedback_loop_current5k | current_window_companion_paths_fixed | conditional_and_mixed | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| catalog_current5k | current_verified_cpu_accepts | 112 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| catalog_current5k | current_incremental_unique_cpu_accepts_over_exact_cache | 110 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| catalog_current5k | market_claim_allowed | false | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| cpu80_status | achieved | false | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |

## Claim Boundary

Allowed:

```text
serving_ops public real-traffic commands were moved into a route-owned include
file; a second-slot runtime policy was implemented but rejected for promotion
because the reproducible probe produced false_accepts=1. The current safe
serving_ops profile remains the energy-threshold fallback with 1 verified
accept and 0 false accepts on current5k.
```

Not allowed:

```text
CPU80 achieved
second-slot policy promoted
market savings from the rejected probe
daemon/server mutation
target/proof label authority
manual local_out_t or concrete lookup
```
