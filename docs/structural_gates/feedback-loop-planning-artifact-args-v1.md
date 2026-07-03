# feedback-loop-planning-artifact-args-v1

## Claim

The CPU route feedback loop can now read an explicit planning_next_step artifact
bundle without overwriting the default v1 dashboard baseline, and the explicit
v2 bundle increases planning coverage while still blocking local accept
promotion.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| feedback_loop_command | keeps | four_existing_positional_arguments | CLI compatibility |
| feedback_loop_command | adds | optional_planning_dry_run_report_path | CLI argument 5 |
| feedback_loop_command | adds | optional_planning_calibration_report_path | CLI argument 6 |
| feedback_loop_command | adds | optional_planning_audit_report_path | CLI argument 7 |
| omitted_planning_args | use | default_v1_planning_artifacts | default dashboard check |
| explicit_planning_args | use | v2_planning_artifacts | explicit v2 dashboard check |
| default_v1_feedback | has_operator_candidate_calls | 302 | default-check report |
| default_v1_feedback | has_scoreable_candidate_calls | 93 | default-check report |
| default_v1_feedback | has_verification_hook_ready_events | 72 | default-check report |
| default_v1_feedback | has_verified_cpu_accepts | 8 | default-check report |
| explicit_v2_feedback | has_operator_candidate_calls | 319 | planning-v2 report |
| explicit_v2_feedback | has_scoreable_candidate_calls | 110 | planning-v2 report |
| explicit_v2_feedback | has_verification_hook_ready_events | 87 | planning-v2 report |
| explicit_v2_feedback | has_verified_cpu_accepts | 8 | planning-v2 report |
| explicit_v2_planning_row | has_candidate_events | 31 | planning-v2 report |
| explicit_v2_planning_row | has_scoreable_payload_events | 31 | planning-v2 report |
| explicit_v2_planning_row | has_verification_hook_ready_events | 22 | planning-v2 report |
| explicit_v2_planning_row | has_support_qualified | false | planning-v2 report |
| explicit_v2_planning_row | has_verified_cpu_accepts | 0 | planning-v2 report |
| market_claim | remains | disallowed | verified CPU remains 8/1000 |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| planning_artifact_args | improve | dashboard_artifact_comparison | v1 and v2 reports can coexist |
| planning_artifact_args | do_not_enable | local_accept_promotion | support_qualified=false |
| explicit_v2_bundle | increases | planning_coverage | candidate and hook-ready rows increase |
| explicit_v2_bundle | does_not_increase | verified_cpu_accepts | verified stays 8 |
| next_repair | should_target | planning_admission_split | new feature needed before promotion |
