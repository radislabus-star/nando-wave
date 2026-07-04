# Metrics-Report Soak Safe-Policy Review V1

NANDA status: PASS.

This packet checks the 5000-row metrics_report safe-policy soak boundary:
the route produced 3 verified safe accepts and 0 false accepts, but 1
unverified shadow accept keeps the route in REVIEW and outside the default
CPU Routability 80 claim.

Current structural-gate result:

```text
nanda_structural_gate: PASS
complexity_score: 43
agent_action: SAFE_TO_EDIT
reason: candidate structure is coherent with source triads
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metrics report soak | trace_rows_written | 5000 | metrics-report-soak-v1/metrics-report-payload-dry-run-v1.report.json |
| metrics report soak | scoreable_payload_events | 63 | metrics-report-soak-v1/metrics-report-payload-dry-run-v1.report.json |
| metrics report calibration | best_safe_true_accepts | 3 | metrics-report-soak-v1/metrics-report-local-accept-calibration-v1.report.json |
| metrics report promoted shadow | verdict | REAL_TRAFFIC_SHADOW_V1_REVIEW | metrics-report-soak-v1/metrics-report-safe-policy-v1.shadow-report.json |
| metrics report promoted shadow | verified_safe_accepts | 3 | metrics-report-soak-v1/metrics-report-safe-policy-v1.shadow-report.json |
| metrics report promoted shadow | unverified_shadow_accepts | 1 | metrics-report-soak-v1/metrics-report-safe-policy-v1.shadow-report.json |
| metrics report promoted shadow | false_accepts | 0 | metrics-report-soak-v1/metrics-report-safe-policy-v1.shadow-report.json |
| metrics report verification audit | market_claim_allowed | false | metrics-report-soak-v1/metrics-report-safe-policy-v1.verification-hook-audit.report.json |
| default CPU feedback | verified_cpu_accept_eligible_events | 12 | cpu-route-feedback-loop-v1.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metrics report soak | trace_rows_written | 5000 | candidate claim |
| metrics report soak | scoreable_payload_events | 63 | candidate claim |
| metrics report calibration | best_safe_true_accepts | 3 | candidate claim |
| metrics report promoted shadow | verdict | REAL_TRAFFIC_SHADOW_V1_REVIEW | candidate claim |
| metrics report promoted shadow | verified_safe_accepts | 3 | candidate claim |
| metrics report promoted shadow | unverified_shadow_accepts | 1 | candidate claim |
| metrics report promoted shadow | false_accepts | 0 | candidate claim |
| metrics report verification audit | market_claim_allowed | false | candidate claim |
| default CPU feedback | verified_cpu_accept_eligible_events | 12 | candidate claim |
| CPU Routability 80 | claim_status | not_achieved | candidate claim |
