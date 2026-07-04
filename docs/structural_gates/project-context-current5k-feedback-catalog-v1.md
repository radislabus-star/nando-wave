# Project Context Current5k Feedback Catalog V1

## query

Verify that current5k artifact-backed project-context evidence is routed into
the CPU feedback loop and catalog without promoting broad `project_context_dialogue`,
without enabling local accepts, and without counting singleton evidence as CPU
savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| feedback_loop | reads_project_context_dry_run | project_context_payload_dry_run_v1_current5k | `cpu-route-feedback-loop-v1-current5k.combined.report.json:project_context_dry_run_report_path` |
| feedback_loop | reads_project_context_verification_audit | project_context_output_evidence_v1_current5k_audit | `cpu-route-feedback-loop-v1-current5k.combined.report.json:project_context_verification_audit_report_path` |
| feedback_loop | reads_project_context_local_calibration | project_context_local_accept_calibration_v1_current5k | `cpu-route-feedback-loop-v1-current5k.combined.report.json:project_context_local_accept_calibration_report_path` |
| project_context_dialogue | feedback_candidate_events | 1314 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].candidate_events` |
| project_context_dialogue | feedback_payload_ready_events | 15 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].payload_ready_events` |
| project_context_dialogue | feedback_scoreable_payload_events | 8 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].scoreable_payload_events` |
| project_context_dialogue | feedback_verification_hook_ready_events | 8 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].verification_hook_ready_events` |
| project_context_dialogue | feedback_verified_cpu_accept_eligible_events | 0 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].verified_cpu_accept_eligible_events` |
| project_context_dialogue | feedback_local_accept_safe_policy_found | true | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].local_accept_safe_policy_found` |
| project_context_dialogue | feedback_local_accept_best_safe_true_accepts | 1 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].local_accept_best_safe_true_accepts` |
| project_context_dialogue | feedback_local_accept_minimum_true_support | 3 | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].local_accept_minimum_true_support` |
| project_context_dialogue | feedback_local_accept_support_qualified | false | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].local_accept_support_qualified` |
| project_context_dialogue | feedback_stage | local_accept_calibration_support_insufficient | `cpu-route-feedback-loop-v1-current5k.combined.report.json:routes[route_key=project_context_dialogue].stage` |
| project_context_evidence | verified_true_events | 1 | `project-context-output-evidence-v1-current5k.report.json:verified_true_events` |
| project_context_evidence | verified_false_events | 7 | `project-context-output-evidence-v1-current5k.report.json:verified_false_events` |
| project_context_shadow | false_accepts | 0 | `project-context-output-evidence-v1-current5k.shadow-report.json:false_accepts` |
| project_context_shadow | nando_shadow_accepts | 0 | `project-context-output-evidence-v1-current5k.shadow-report.json:nando_shadow_accepts` |
| cpu_catalog_project_context_existing_row | current_status | REJECT_FOR_NOW | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[source_kind=existing_profile_route,route_or_family_key=project_context_dialogue].current_status` |
| cpu_catalog_project_context_existing_row | business_value_gate_failure_reason | expected_zero_broad_route_support_insufficient | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[source_kind=existing_profile_route,route_or_family_key=project_context_dialogue].business_value_gate_failure_reason` |
| cpu_catalog_project_context_existing_row | expected_unique_cpu_accepts_over_exact_cache | 0 | `cpu-operator-catalog-v1-current5k.combined.report.json:rows[source_kind=existing_profile_route,route_or_family_key=project_context_dialogue].expected_unique_cpu_accepts_over_exact_cache` |
| project_context_current5k_wiring | promotion_policy | broad_route_not_promoted | `docs/EXECUTOR_REVIEW_NOTES.md#2026-07-04---executor-integration-project-context-current5k-evidence-routed-into-feedback-loop` |
| project_context_current5k_wiring | local_accept_authority | none | `docs/EXECUTOR_REVIEW_NOTES.md#2026-07-04---executor-integration-project-context-current5k-evidence-routed-into-feedback-loop` |
