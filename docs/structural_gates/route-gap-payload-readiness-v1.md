# Route-Gap Payload Readiness V1

NANDA status: VETO.

This packet checks the narrow readiness claim: no-candidate real Codex prompts
can be counted by request-side payload-builder readiness without enabling local
accepts or market-savings claims.

This packet must not be used as a CPU Routability 80 proof.

Current structural-gate result:

```text
nanda_structural_gate: VETO
conflicts: 0
stable_source_triads: 21
weak_candidate_triads: 21
reason: at least one candidate triad has weak composite-mode support
interpretation: proof-shape debt, not a runtime safety failure
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| route-gap payload readiness command | cli_command | role-binding-real-traffic-route-gap-payload-readiness-v1 | main.rs |
| route-gap payload readiness command | input_history | /home/ubu/.codex/history.jsonl | route-gap-payload-readiness-v1.report.json |
| route-gap payload readiness command | input_registry | profile-registry-agent-control-v1.json | route-gap-payload-readiness-v1.report.json |
| route-gap payload readiness command | output_report | route-gap-payload-readiness-v1.report.json | role_binding_runtime_cmd.rs |
| route-gap payload readiness command | raw_text_written | false | route-gap-payload-readiness-v1.report.json |
| route-gap payload readiness command | response_text_used | false | route-gap-payload-readiness-v1.report.json |
| route-gap payload readiness command | local_accepts_enabled | false | route-gap-payload-readiness-v1.report.json |
| route-gap payload readiness command | market_claim_allowed | false | route-gap-payload-readiness-v1.report.json |
| current traffic sample | sampled_llm_calls | 1000 | route-gap-payload-readiness-v1.report.json |
| current routed zone | existing_route_candidate_events | 408 | route-gap-payload-readiness-v1.report.json |
| current route-gap zone | no_candidate_events | 592 | route-gap-payload-readiness-v1.report.json |
| route-gap readiness | payload_ready_events | 54 | route-gap-payload-readiness-v1.report.json |
| top ready route-gap family | family_key | planning_next_step | route-gap-payload-readiness-v1.report.json |
| planning_next_step | payload_ready_events | 19 | route-gap-payload-readiness-v1.report.json |
| read_inspect | payload_ready_events | 12 | route-gap-payload-readiness-v1.report.json |
| metrics_report_readout | payload_ready_events | 6 | route-gap-payload-readiness-v1.report.json |
| answer_or_explain | payload_ready_events | 0 | route-gap-payload-readiness-v1.report.json |
| answer_or_explain | blocker | missing_evidence_and_verifier_signal | route-gap-payload-readiness-v1.report.json |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | cpu-operator-catalog-v1.report.json |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | cpu-operator-catalog-v1.report.json |
| route-gap payload readiness | claim_status | readiness_not_savings | route-gap-payload-readiness-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| route-gap payload readiness command | cli_command | role-binding-real-traffic-route-gap-payload-readiness-v1 | candidate claim |
| route-gap payload readiness command | input_history | /home/ubu/.codex/history.jsonl | candidate claim |
| route-gap payload readiness command | input_registry | profile-registry-agent-control-v1.json | candidate claim |
| route-gap payload readiness command | output_report | route-gap-payload-readiness-v1.report.json | candidate claim |
| route-gap payload readiness command | raw_text_written | false | candidate claim |
| route-gap payload readiness command | response_text_used | false | candidate claim |
| route-gap payload readiness command | local_accepts_enabled | false | candidate claim |
| route-gap payload readiness command | market_claim_allowed | false | candidate claim |
| current traffic sample | sampled_llm_calls | 1000 | candidate claim |
| current routed zone | existing_route_candidate_events | 408 | candidate claim |
| current route-gap zone | no_candidate_events | 592 | candidate claim |
| route-gap readiness | payload_ready_events | 54 | candidate claim |
| top ready route-gap family | family_key | planning_next_step | candidate claim |
| planning_next_step | payload_ready_events | 19 | candidate claim |
| read_inspect | payload_ready_events | 12 | candidate claim |
| metrics_report_readout | payload_ready_events | 6 | candidate claim |
| answer_or_explain | payload_ready_events | 0 | candidate claim |
| answer_or_explain | blocker | missing_evidence_and_verifier_signal | candidate claim |
| CPU Routability 80 | current_verified_cpu_accepts | 8 | candidate claim |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | candidate claim |
| route-gap payload readiness | claim_status | readiness_not_savings | candidate claim |
