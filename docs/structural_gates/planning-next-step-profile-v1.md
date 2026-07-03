# Planning Next-Step Profile V1

NANDA status: VETO.

This packet checks the narrow profile claim: the planning_next_step dry-run
payloads can be compiled into a real `.nwrb` package and registry overlay while
local accepts remain disabled by threshold and market-savings claims stay off.

This packet must not be used as a CPU Routability 80 proof.

Current structural-gate result:

```text
nanda_structural_gate: VETO
conflicts: 0
reason: at least one candidate triad has weak composite-mode support
interpretation: proof-shape debt, not a runtime safety failure
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| planning-next-step profile command | cli_command | role-binding-real-traffic-planning-next-step-profile-v1 | main.rs |
| planning-next-step profile command | input_trace | planning-next-step-payload-dry-run-v1.trace.jsonl | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | input_base_registry | profile-registry-agent-control-v1.json | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | output_package | planning-next-step-seed0.nwrb | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | output_registry | profile-registry-planning-next-step-v1.json | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | output_report | planning-next-step-profile-v1.report.json | role_binding_runtime_cmd.rs |
| planning-next-step profile command | raw_text_written | false | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | response_text_used | false | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | target_labels_used | false | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | proof_labels_used | false | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | local_accepts_enabled_on_real_traffic | false | planning-next-step-profile-v1.report.json |
| planning-next-step profile command | market_claim_allowed | false | planning-next-step-profile-v1.report.json |
| planning-next-step package | edge_count | 8 | planning-next-step-profile-v1.report.json |
| planning-next-step package | package_bytes | 140 | planning-next-step-profile-v1.report.json |
| planning-next-step package | package_training_requests | 14 | planning-next-step-profile-v1.report.json |
| planning-next-step profile | threshold | 2147483647 | planning-next-step-profile-v1.report.json |
| planning-next-step profile | unexpected_local_accepts_under_disabled_threshold | 0 | planning-next-step-profile-v1.report.json |
| planning-next-step profile | median_energy_margin | 1106944 | planning-next-step-profile-v1.report.json |
| planning-next-step profile | min_energy_margin | 873472 | planning-next-step-profile-v1.report.json |
| planning-next-step shadow | operator_candidate_calls | 14 | planning-next-step-profile-v1.shadow-report.json |
| planning-next-step shadow | nando_shadow_accepts | 0 | planning-next-step-profile-v1.shadow-report.json |
| planning-next-step shadow | verified_safe_accepts | 0 | planning-next-step-profile-v1.shadow-report.json |
| planning-next-step shadow | false_accepts | 0 | planning-next-step-profile-v1.shadow-report.json |
| planning-next-step shadow | incremental_reduction_vs_exact_cache_milli | 0 | planning-next-step-profile-v1.shadow-report.json |
| planning-next-step audit | verification_hook_ready_events | 0 | planning-next-step-profile-v1.verification-hook-audit.report.json |
| planning-next-step audit | verified_cpu_accept_eligible_events | 0 | planning-next-step-profile-v1.verification-hook-audit.report.json |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | cpu-operator-catalog-v1.report.json |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | cpu-operator-catalog-v1.report.json |
| planning-next-step profile | claim_status | profile_ready_not_verified_cpu | planning-next-step-profile-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| planning-next-step profile command | cli_command | role-binding-real-traffic-planning-next-step-profile-v1 | candidate claim |
| planning-next-step profile command | input_trace | planning-next-step-payload-dry-run-v1.trace.jsonl | candidate claim |
| planning-next-step profile command | input_base_registry | profile-registry-agent-control-v1.json | candidate claim |
| planning-next-step profile command | output_package | planning-next-step-seed0.nwrb | candidate claim |
| planning-next-step profile command | output_registry | profile-registry-planning-next-step-v1.json | candidate claim |
| planning-next-step profile command | output_report | planning-next-step-profile-v1.report.json | candidate claim |
| planning-next-step profile command | raw_text_written | false | candidate claim |
| planning-next-step profile command | response_text_used | false | candidate claim |
| planning-next-step profile command | target_labels_used | false | candidate claim |
| planning-next-step profile command | proof_labels_used | false | candidate claim |
| planning-next-step profile command | local_accepts_enabled_on_real_traffic | false | candidate claim |
| planning-next-step profile command | market_claim_allowed | false | candidate claim |
| planning-next-step package | edge_count | 8 | candidate claim |
| planning-next-step package | package_bytes | 140 | candidate claim |
| planning-next-step package | package_training_requests | 14 | candidate claim |
| planning-next-step profile | threshold | 2147483647 | candidate claim |
| planning-next-step profile | unexpected_local_accepts_under_disabled_threshold | 0 | candidate claim |
| planning-next-step profile | median_energy_margin | 1106944 | candidate claim |
| planning-next-step profile | min_energy_margin | 873472 | candidate claim |
| planning-next-step shadow | operator_candidate_calls | 14 | candidate claim |
| planning-next-step shadow | nando_shadow_accepts | 0 | candidate claim |
| planning-next-step shadow | verified_safe_accepts | 0 | candidate claim |
| planning-next-step shadow | false_accepts | 0 | candidate claim |
| planning-next-step shadow | incremental_reduction_vs_exact_cache_milli | 0 | candidate claim |
| planning-next-step audit | verification_hook_ready_events | 0 | candidate claim |
| planning-next-step audit | verified_cpu_accept_eligible_events | 0 | candidate claim |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | candidate claim |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | candidate claim |
| planning-next-step profile | claim_status | profile_ready_not_verified_cpu | candidate claim |
