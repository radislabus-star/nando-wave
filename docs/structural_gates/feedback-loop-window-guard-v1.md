# Feedback-Loop Window Guard V1

NANDA status: PASS.

This packet checks the CPU feedback-loop aggregation boundary: route-specific
verification audits whose `total_llm_calls` differ from the forecast window are
reported as mismatches and excluded from CPU Routability aggregation.

Current structural-gate result:

```text
nanda_structural_gate: PASS
complexity_score: 27
agent_action: SAFE_TO_EDIT
reason: candidate structure is coherent with source triads
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback-loop-window-guard | forecast window | 1000 calls | cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json |
| feedback-loop-window-guard | metrics audit window | 5000 calls | metrics-report-soak-v1 verification audit |
| feedback-loop-window-guard | mismatch route | metrics_report_readout | cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json |
| feedback-loop-window-guard | mismatch handling | excluded from feedback | cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json |
| feedback-loop-window-guard | verified CPU accepts | 12 | cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json |
| feedback-loop-window-guard | CPU Routability 80 | not achieved | cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json |
| default-feedback-window | audit mismatches | none | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback-loop-window-guard | forecast window | 1000 calls | candidate claim |
| feedback-loop-window-guard | metrics audit window | 5000 calls | candidate claim |
| feedback-loop-window-guard | mismatch route | metrics_report_readout | candidate claim |
| feedback-loop-window-guard | mismatch handling | excluded from feedback | candidate claim |
| feedback-loop-window-guard | verified CPU accepts | 12 | candidate claim |
| feedback-loop-window-guard | CPU Routability 80 | not achieved | candidate claim |
| default-feedback-window | audit mismatches | none | candidate claim |
