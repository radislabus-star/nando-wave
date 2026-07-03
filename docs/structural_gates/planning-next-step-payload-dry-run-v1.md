# Planning Next-Step Payload Dry-Run V1

NANDA status: VETO.

This packet checks the narrow dry-run claim: no-candidate real Codex prompts in
the planning_next_step route-gap family can be converted into request-side
active_fringe/slot payloads without enabling local accepts or market-savings
claims.

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
| planning-next-step dry-run command | cli_command | role-binding-real-traffic-planning-next-step-payload-dry-run-v1 | main.rs |
| planning-next-step dry-run command | input_history | /home/ubu/.codex/history.jsonl | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | input_registry | profile-registry-agent-control-v1.json | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | output_trace | planning-next-step-payload-dry-run-v1.trace.jsonl | role_binding_runtime_cmd.rs |
| planning-next-step dry-run command | output_report | planning-next-step-payload-dry-run-v1.report.json | role_binding_runtime_cmd.rs |
| planning-next-step dry-run command | raw_text_written | false | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | response_text_used | false | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | target_labels_used | false | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | proof_labels_used | false | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | local_accepts_enabled | false | planning-next-step-payload-dry-run-v1.report.json |
| planning-next-step dry-run command | market_claim_allowed | false | planning-next-step-payload-dry-run-v1.report.json |
| current traffic sample | sampled_llm_calls | 1000 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step route-gap | candidate_events | 54 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step route-gap | payload_ready_events | 19 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step route-gap | payload_built_events | 14 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step route-gap | scoreable_payload_events | 14 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step route-gap | builder_rejected_events | 5 | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step profile | profile_registered | false | planning-next-step-payload-dry-run-v1.report.json |
| planning_next_step shadow | nando_shadow_accepts | 0 | planning-next-step-payload-dry-run-v1.shadow-report.json |
| planning_next_step shadow | verified_safe_accepts | 0 | planning-next-step-payload-dry-run-v1.shadow-report.json |
| planning_next_step shadow | false_accepts | 0 | planning-next-step-payload-dry-run-v1.shadow-report.json |
| planning_next_step shadow | incremental_reduction_vs_exact_cache_milli | 0 | planning-next-step-payload-dry-run-v1.shadow-report.json |
| planning_next_step audit | verification_hook_ready_events | 0 | planning-next-step-payload-dry-run-v1.verification-hook-audit.report.json |
| planning_next_step audit | verified_cpu_accept_eligible_events | 0 | planning-next-step-payload-dry-run-v1.verification-hook-audit.report.json |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | cpu-operator-catalog-v1.report.json |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | cpu-operator-catalog-v1.report.json |
| planning-next-step dry-run | claim_status | payload_not_savings | planning-next-step-payload-dry-run-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| planning-next-step dry-run command | cli_command | role-binding-real-traffic-planning-next-step-payload-dry-run-v1 | candidate claim |
| planning-next-step dry-run command | input_history | /home/ubu/.codex/history.jsonl | candidate claim |
| planning-next-step dry-run command | input_registry | profile-registry-agent-control-v1.json | candidate claim |
| planning-next-step dry-run command | output_trace | planning-next-step-payload-dry-run-v1.trace.jsonl | candidate claim |
| planning-next-step dry-run command | output_report | planning-next-step-payload-dry-run-v1.report.json | candidate claim |
| planning-next-step dry-run command | raw_text_written | false | candidate claim |
| planning-next-step dry-run command | response_text_used | false | candidate claim |
| planning-next-step dry-run command | target_labels_used | false | candidate claim |
| planning-next-step dry-run command | proof_labels_used | false | candidate claim |
| planning-next-step dry-run command | local_accepts_enabled | false | candidate claim |
| planning-next-step dry-run command | market_claim_allowed | false | candidate claim |
| current traffic sample | sampled_llm_calls | 1000 | candidate claim |
| planning_next_step route-gap | candidate_events | 54 | candidate claim |
| planning_next_step route-gap | payload_ready_events | 19 | candidate claim |
| planning_next_step route-gap | payload_built_events | 14 | candidate claim |
| planning_next_step route-gap | scoreable_payload_events | 14 | candidate claim |
| planning_next_step route-gap | builder_rejected_events | 5 | candidate claim |
| planning_next_step profile | profile_registered | false | candidate claim |
| planning_next_step shadow | nando_shadow_accepts | 0 | candidate claim |
| planning_next_step shadow | verified_safe_accepts | 0 | candidate claim |
| planning_next_step shadow | false_accepts | 0 | candidate claim |
| planning_next_step shadow | incremental_reduction_vs_exact_cache_milli | 0 | candidate claim |
| planning_next_step audit | verification_hook_ready_events | 0 | candidate claim |
| planning_next_step audit | verified_cpu_accept_eligible_events | 0 | candidate claim |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | candidate claim |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | candidate claim |
| planning-next-step dry-run | claim_status | payload_not_savings | candidate claim |
