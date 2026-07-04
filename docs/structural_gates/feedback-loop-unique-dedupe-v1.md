# Feedback-Loop Unique Dedupe V1

NANDA status: PASS.

This packet checks only two structural boundaries:

- CPU Routability feedback must expose unique accepted real requests separately
  from raw route-sum hits.
- Agent-control accepts must not be hidden in totals without a visible route
  row.

Current structural-gate result:

```text
nanda_structural_gate: PASS
complexity_score: 39
agent_action: SAFE_TO_EDIT
reason: candidate structure is coherent with source triads
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback unique dedupe | key | request_fingerprint | role_binding_runtime_cmd.rs |
| feedback unique dedupe | source rows | shadow report rows | role_binding_runtime_cmd.rs |
| default feedback | route-sum accepts | 20 | cpu-route-feedback-loop-v1.report.json |
| default feedback | unique accepts | 15 | cpu-route-feedback-loop-v1.report.json |
| default feedback | duplicate route hits | 5 | cpu-route-feedback-loop-v1.report.json |
| default feedback | exposes route row | role_binding_agent_control_seed0 | cpu-route-feedback-loop-v1.report.json |
| v3 dedupe check | route-sum accepts | 21 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| v3 dedupe check | unique accepts | 16 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| v3 dedupe check | duplicate route hits | 5 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback unique dedupe | key | request_fingerprint | role_binding_runtime_cmd.rs |
| feedback unique dedupe | source rows | shadow report rows | role_binding_runtime_cmd.rs |
| default feedback | route-sum accepts | 20 | cpu-route-feedback-loop-v1.report.json |
| default feedback | unique accepts | 15 | cpu-route-feedback-loop-v1.report.json |
| default feedback | duplicate route hits | 5 | cpu-route-feedback-loop-v1.report.json |
| default feedback | exposes route row | role_binding_agent_control_seed0 | cpu-route-feedback-loop-v1.report.json |
| v3 dedupe check | route-sum accepts | 21 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| v3 dedupe check | unique accepts | 16 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| v3 dedupe check | duplicate route hits | 5 | cpu-route-feedback-loop-v3.dedup-check.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |
