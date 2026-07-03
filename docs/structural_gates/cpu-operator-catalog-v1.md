# CPU Operator Catalog V1

NANDA status: VETO.

This packet checks the narrow catalog claim:
existing routed profiles and no-candidate route-gap families are merged into a
CPU operator worklist, but no new local accepts or market savings are claimed.

This packet must not be used as a CPU Routability 80 proof.

Current structural-gate result:

```text
nanda_structural_gate: VETO
conflicts: 0
stable_triads: c2, c16
weak_triads: 17
reason: at least one candidate triad has weak composite-mode support
interpretation: proof-shape debt, not a runtime safety failure
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| CPU operator catalog command | cli_command | role-binding-real-traffic-cpu-operator-catalog-v1 | main.rs |
| CPU operator catalog command | feedback_source_report | cpu-route-feedback-loop-conditional-agent-control-v1.report.json | role_binding_runtime_cmd.rs:feedback_report_input |
| CPU operator catalog command | route_gap_source_report | route-gap-catalog-agent-control-v1.report.json | role_binding_runtime_cmd.rs:route_gap_input |
| CPU operator catalog command | route_gap_payload_readiness_report | route-gap-payload-readiness-v1.report.json | role_binding_runtime_cmd.rs:readiness_input |
| CPU operator catalog command | catalog_output_report | cpu-operator-catalog-v1.report.json | role_binding_runtime_cmd.rs:catalog_output |
| CPU operator catalog command | raw_text_written | false | cpu-operator-catalog-v1.report.json |
| CPU operator catalog command | local_accepts_enabled | false | cpu-operator-catalog-v1.report.json |
| CPU operator catalog command | market_claim_allowed | false | cpu-operator-catalog-v1.report.json |
| current traffic sample | total_llm_calls | 1000 | cpu-operator-catalog-v1.report.json |
| current traffic sample | exact_cache_hits | 53 | cpu-operator-catalog-v1.report.json |
| current routed zone | existing_operator_candidate_calls | 408 | cpu-operator-catalog-v1.report.json |
| current unrouted zone | no_candidate_calls | 592 | cpu-operator-catalog-v1.report.json |
| current route-gap readiness | route_gap_payload_ready_events | 54 | cpu-operator-catalog-v1.report.json |
| current CPU accepts | verified_cpu_accepts | 8 | cpu-operator-catalog-v1.report.json |
| CPU Routability 80 | target_verified_cpu_accepts | 800 | cpu-operator-catalog-v1.report.json |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | cpu-operator-catalog-v1.report.json |
| top no-candidate family | family_key | answer_or_explain | cpu-operator-catalog-v1.report.json |
| answer_or_explain | readiness | low_needs_knowledge_evidence | cpu-operator-catalog-v1.report.json |
| first medium backlog family | family_key | planning_next_step | cpu-operator-catalog-v1.report.json |
| planning_next_step | readiness | medium_state_transition_candidate | cpu-operator-catalog-v1.report.json |
| planning_next_step | payload_ready_events | 19 | cpu-operator-catalog-v1.report.json |
| CPU operator catalog | claim_status | worklist_not_savings | cpu-operator-catalog-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| CPU operator catalog command | cli_command | role-binding-real-traffic-cpu-operator-catalog-v1 | candidate claim |
| CPU operator catalog command | feedback_source_report | cpu-route-feedback-loop-conditional-agent-control-v1.report.json | candidate claim |
| CPU operator catalog command | route_gap_source_report | route-gap-catalog-agent-control-v1.report.json | candidate claim |
| CPU operator catalog command | route_gap_payload_readiness_report | route-gap-payload-readiness-v1.report.json | candidate claim |
| CPU operator catalog command | catalog_output_report | cpu-operator-catalog-v1.report.json | candidate claim |
| CPU operator catalog command | raw_text_written | false | candidate claim |
| CPU operator catalog command | local_accepts_enabled | false | candidate claim |
| CPU operator catalog command | market_claim_allowed | false | candidate claim |
| current traffic sample | total_llm_calls | 1000 | candidate claim |
| current traffic sample | exact_cache_hits | 53 | candidate claim |
| current routed zone | existing_operator_candidate_calls | 408 | candidate claim |
| current unrouted zone | no_candidate_calls | 592 | candidate claim |
| current route-gap readiness | route_gap_payload_ready_events | 54 | candidate claim |
| current CPU accepts | verified_cpu_accepts | 8 | candidate claim |
| CPU Routability 80 | target_verified_cpu_accepts | 800 | candidate claim |
| CPU Routability 80 | verified_gap_to_80_calls | 792 | candidate claim |
| top no-candidate family | family_key | answer_or_explain | candidate claim |
| answer_or_explain | readiness | low_needs_knowledge_evidence | candidate claim |
| first medium backlog family | family_key | planning_next_step | candidate claim |
| planning_next_step | readiness | medium_state_transition_candidate | candidate claim |
| planning_next_step | payload_ready_events | 19 | candidate claim |
| CPU operator catalog | claim_status | worklist_not_savings | candidate claim |
