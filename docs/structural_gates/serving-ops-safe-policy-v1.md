# Serving-Ops Safe-Policy V1

NANDA status: VETO for the full packet shape.

Current structural-gate result:

```text
nanda_structural_gate: VETO
complexity_score: 79
reason: candidate triads have weak composite-mode support
interpretation:
  proof-shape debt for the full promoted-route ladder, not a shadow/audit
  runtime failure
follow-up:
  route-core packet split out as serving-ops-safe-policy-core-v1.md and passed
```

This packet checks the narrow promoted serving-ops safe-policy claim: the
promoted shadow trace produced verified CPU accepts with provider cost,
false_accepts=0, unverified_shadow_accepts=0, and a feedback-loop increase from
8/1000 to 11/1000. It must not be used as CPU Routability 80 proof.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops safe policy promote command | cli_command | role-binding-real-traffic-serving-ops-safe-policy-promote-v1 | main.rs |
| serving ops promoted registry | output_path | profile-registry-serving-ops-safe-policy-v1.json | serving-ops-safe-policy-v1.report.json |
| serving ops promoted trace | output_path | serving-ops-safe-policy-v1.trace.jsonl | serving-ops-safe-policy-v1.report.json |
| serving ops selected policy | policy_name | market_safe_energy_margin_threshold | serving-ops-safe-policy-v1.report.json |
| serving ops selected policy | threshold | 1392640 | serving-ops-safe-policy-v1.report.json |
| serving ops selected policy | true_accepts | 3 | serving-ops-safe-policy-v1.report.json |
| serving ops selected policy | false_accepts | 0 | serving-ops-safe-policy-v1.report.json |
| serving ops selected policy | unverified_accepts | 0 | serving-ops-safe-policy-v1.report.json |
| serving ops promoted shadow | verdict | REAL_TRAFFIC_SHADOW_V1_PASS | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | nando_shadow_accepts | 3 | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | verified_safe_accepts | 3 | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | false_accepts | 0 | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | unverified_shadow_accepts | 0 | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | incremental_savings_over_exact_cache | 3 | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops promoted shadow | synthetic_trace_used | false | serving-ops-safe-policy-v1.shadow-report.json |
| serving ops verification audit | verified_cpu_accept_eligible_events | 3 | serving-ops-safe-policy-v1.verification-hook-audit.report.json |
| serving ops verification audit | market_claim_allowed | true | serving-ops-safe-policy-v1.verification-hook-audit.report.json |
| feedback loop after serving ops promote | verified_cpu_accept_eligible_events | 11 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops promote | verified_cpu_routability_milli | 11 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops promote | verified_gap_to_80_calls | 789 | cpu-route-feedback-loop-v1.report.json |
| serving ops route row | stage | verified_cpu_accept_eligible | cpu-route-feedback-loop-v1.report.json |
| serving ops route row | false_accepts | 0 | cpu-route-feedback-loop-v1.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops safe policy promote command | cli_command | role-binding-real-traffic-serving-ops-safe-policy-promote-v1 | candidate claim |
| serving ops promoted registry | output_path | profile-registry-serving-ops-safe-policy-v1.json | candidate claim |
| serving ops promoted trace | output_path | serving-ops-safe-policy-v1.trace.jsonl | candidate claim |
| serving ops selected policy | policy_name | market_safe_energy_margin_threshold | candidate claim |
| serving ops selected policy | threshold | 1392640 | candidate claim |
| serving ops selected policy | true_accepts | 3 | candidate claim |
| serving ops selected policy | false_accepts | 0 | candidate claim |
| serving ops selected policy | unverified_accepts | 0 | candidate claim |
| serving ops promoted shadow | verdict | REAL_TRAFFIC_SHADOW_V1_PASS | candidate claim |
| serving ops promoted shadow | nando_shadow_accepts | 3 | candidate claim |
| serving ops promoted shadow | verified_safe_accepts | 3 | candidate claim |
| serving ops promoted shadow | false_accepts | 0 | candidate claim |
| serving ops promoted shadow | unverified_shadow_accepts | 0 | candidate claim |
| serving ops promoted shadow | incremental_savings_over_exact_cache | 3 | candidate claim |
| serving ops promoted shadow | synthetic_trace_used | false | candidate claim |
| serving ops verification audit | verified_cpu_accept_eligible_events | 3 | candidate claim |
| serving ops verification audit | market_claim_allowed | true | candidate claim |
| feedback loop after serving ops promote | verified_cpu_accept_eligible_events | 11 | candidate claim |
| feedback loop after serving ops promote | verified_cpu_routability_milli | 11 | candidate claim |
| feedback loop after serving ops promote | verified_gap_to_80_calls | 789 | candidate claim |
| serving ops route row | stage | verified_cpu_accept_eligible | candidate claim |
| serving ops route row | false_accepts | 0 | candidate claim |
| CPU Routability 80 | claim_status | not_achieved | candidate claim |
