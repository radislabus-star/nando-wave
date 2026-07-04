# Serving-Ops Route V1

NANDA status: VETO for the full packet shape.

Current structural-gate result:

```text
nanda_structural_gate: VETO
complexity_score: 133
reason:
  - candidate triads have weak composite-mode support
  - packet exceeds target size: entities 37>32, triads 78>64
interpretation:
  proof-shape debt for the full route ladder, not a runtime safety failure
follow-up:
  route-core packet split out as serving-ops-route-core-v1.md and passed
```

This packet checks the narrow serving-ops route claim:
real Codex prompts can be routed into a request-side service/daemon/HTTP metric
profile, scored with a registered `.nwrb` package, attached to deterministic
final-answer evidence, and calibrated for a future safe policy without enabling
local accepts or market-savings claims.

This packet must not be used as a CPU Routability 80 proof.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops route | route_key | serving_ops | role_binding_runtime_cmd.rs |
| serving ops route | profile_id | route_gap_serving_ops_profile_v1 | role_binding_runtime_cmd.rs |
| serving ops payload dry run command | cli_command | role-binding-real-traffic-serving-ops-payload-dry-run-v1 | main.rs |
| serving ops profile command | cli_command | role-binding-real-traffic-serving-ops-profile-v1 | main.rs |
| serving ops output evidence command | cli_command | role-binding-real-traffic-serving-ops-output-evidence-v1 | main.rs |
| serving ops local accept calibration command | cli_command | role-binding-real-traffic-serving-ops-local-accept-calibration-v1 | main.rs |
| serving ops payload dry run | candidate_events | 25 | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | payload_ready_events | 8 | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | scoreable_payload_events | 8 | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | raw_text_written | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | response_text_used | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | local_accepts_enabled | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops payload dry run | market_claim_allowed | false | serving-ops-payload-dry-run-v1.report.json |
| serving ops profile | edge_count | 8 | serving-ops-profile-v1.report.json |
| serving ops profile | package_bytes | 140 | serving-ops-profile-v1.report.json |
| serving ops profile | runtime_bytes_estimate | 33000 | serving-ops-profile-v1.report.json |
| serving ops profile | unexpected_local_accepts_under_disabled_threshold | 0 | serving-ops-profile-v1.report.json |
| serving ops output evidence | output_evidence_matched_events | 7 | serving-ops-output-evidence-v1.report.json |
| serving ops output evidence | verified_true_events | 5 | serving-ops-output-evidence-v1.report.json |
| serving ops output evidence | verified_false_events | 2 | serving-ops-output-evidence-v1.report.json |
| serving ops output evidence | raw_prompt_text_written | false | serving-ops-output-evidence-v1.report.json |
| serving ops output evidence | raw_response_text_written | false | serving-ops-output-evidence-v1.report.json |
| serving ops verification audit | verification_hook_ready_events | 7 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops verification audit | verified_cpu_accept_eligible_events | 0 | serving-ops-output-evidence-v1.verification-hook-audit.report.json |
| serving ops calibration | safe_policy_found | true | serving-ops-local-accept-calibration-v1.report.json |
| serving ops calibration | best_safe_true_accepts | 3 | serving-ops-local-accept-calibration-v1.report.json |
| serving ops calibration | local_accepts_enabled | false | serving-ops-local-accept-calibration-v1.report.json |
| serving ops calibration | market_claim_allowed | false | serving-ops-local-accept-calibration-v1.report.json |
| feedback loop after serving ops | operator_candidate_calls | 427 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops | scoreable_candidate_calls | 167 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops | verification_hook_ready_events | 130 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops | verified_cpu_accept_eligible_events | 8 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops | verified_cpu_routability_milli | 8 | cpu-route-feedback-loop-v1.report.json |
| feedback loop after serving ops | verified_gap_to_80_calls | 792 | cpu-route-feedback-loop-v1.report.json |
| serving ops route row | stage | local_accept_calibration_safe_policy_candidate | cpu-route-feedback-loop-v1.report.json |
| serving ops route row | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json |
| serving ops route row | false_accepts | 0 | cpu-route-feedback-loop-v1.report.json |
| CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-v1.report.json |
| serving ops route | claim_status | scoreable_calibrated_not_promoted | EXECUTOR_REVIEW_NOTES.md |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving ops route | route_key | serving_ops | candidate claim |
| serving ops route | profile_id | route_gap_serving_ops_profile_v1 | candidate claim |
| serving ops payload dry run command | cli_command | role-binding-real-traffic-serving-ops-payload-dry-run-v1 | candidate claim |
| serving ops profile command | cli_command | role-binding-real-traffic-serving-ops-profile-v1 | candidate claim |
| serving ops output evidence command | cli_command | role-binding-real-traffic-serving-ops-output-evidence-v1 | candidate claim |
| serving ops local accept calibration command | cli_command | role-binding-real-traffic-serving-ops-local-accept-calibration-v1 | candidate claim |
| serving ops payload dry run | candidate_events | 25 | candidate claim |
| serving ops payload dry run | payload_ready_events | 8 | candidate claim |
| serving ops payload dry run | scoreable_payload_events | 8 | candidate claim |
| serving ops payload dry run | raw_text_written | false | candidate claim |
| serving ops payload dry run | response_text_used | false | candidate claim |
| serving ops payload dry run | local_accepts_enabled | false | candidate claim |
| serving ops payload dry run | market_claim_allowed | false | candidate claim |
| serving ops profile | edge_count | 8 | candidate claim |
| serving ops profile | package_bytes | 140 | candidate claim |
| serving ops profile | runtime_bytes_estimate | 33000 | candidate claim |
| serving ops profile | unexpected_local_accepts_under_disabled_threshold | 0 | candidate claim |
| serving ops output evidence | output_evidence_matched_events | 7 | candidate claim |
| serving ops output evidence | verified_true_events | 5 | candidate claim |
| serving ops output evidence | verified_false_events | 2 | candidate claim |
| serving ops output evidence | raw_prompt_text_written | false | candidate claim |
| serving ops output evidence | raw_response_text_written | false | candidate claim |
| serving ops verification audit | verification_hook_ready_events | 7 | candidate claim |
| serving ops verification audit | verified_cpu_accept_eligible_events | 0 | candidate claim |
| serving ops calibration | safe_policy_found | true | candidate claim |
| serving ops calibration | best_safe_true_accepts | 3 | candidate claim |
| serving ops calibration | local_accepts_enabled | false | candidate claim |
| serving ops calibration | market_claim_allowed | false | candidate claim |
| feedback loop after serving ops | operator_candidate_calls | 427 | candidate claim |
| feedback loop after serving ops | scoreable_candidate_calls | 167 | candidate claim |
| feedback loop after serving ops | verification_hook_ready_events | 130 | candidate claim |
| feedback loop after serving ops | verified_cpu_accept_eligible_events | 8 | candidate claim |
| feedback loop after serving ops | verified_cpu_routability_milli | 8 | candidate claim |
| feedback loop after serving ops | verified_gap_to_80_calls | 792 | candidate claim |
| serving ops route row | stage | local_accept_calibration_safe_policy_candidate | candidate claim |
| serving ops route row | verified_cpu_accept_eligible_events | 0 | candidate claim |
| serving ops route row | false_accepts | 0 | candidate claim |
| CPU Routability 80 | claim_status | not_achieved | candidate claim |
| serving ops route | claim_status | scoreable_calibrated_not_promoted | candidate claim |
