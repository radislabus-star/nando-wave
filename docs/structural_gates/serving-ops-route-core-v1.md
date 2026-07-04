# Serving-Ops Route Core V1

NANDA status: PASS.

This packet checks only the core safety boundary for the serving-ops route:
payloads are scoreable, the profile exists, evidence hooks attach, calibration
finds a safe-policy candidate, but local accepts remain disabled and CPU
Routability 80 remains unachieved.

This packet must not be used as a CPU Routability 80 proof or a market-savings
claim.

Current structural-gate result:

```text
nanda_structural_gate: PASS
complexity_score: 34
agent_action: SAFE_TO_EDIT
reason: candidate structure is coherent with source triads
```

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops payload dry run | scoreable_payload_events | 8 | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | local_accepts_enabled | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops profile | edge_count | 8 | serving-ops-profile-v1.report.json |
| serving ops output evidence | verification_hook_ready_events | 7 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops verification audit | verified_cpu_accept_eligible_events | 0 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops calibration | safe_policy_found | true | serving-ops-local-accept-calibration-v1.report.json |
| serving ops calibration | local_accepts_enabled | false | serving-ops-local-accept-calibration-v1.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops payload dry run | scoreable_payload_events | 8 | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | local_accepts_enabled | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops profile | edge_count | 8 | serving-ops-profile-v1.report.json |
| serving ops output evidence | verification_hook_ready_events | 7 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops verification audit | verified_cpu_accept_eligible_events | 0 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops calibration | safe_policy_found | true | serving-ops-local-accept-calibration-v1.report.json |
| serving ops calibration | local_accepts_enabled | false | serving-ops-local-accept-calibration-v1.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |
