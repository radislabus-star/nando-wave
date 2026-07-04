# Project Context Current5k Singleton Watch V1

## query

Verify the current5k project_context boundary: artifact_backed_project_state is
a valid narrow split candidate, but current verifier support is singleton-only
and must not be promoted as broad project_context CPU routability.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| project_context_current5k | candidate_events | 1313 | `project-context-subfamily-audit-v1-current5k.report.json` |
| project_context_current5k | top_actionable_subfamily | artifact_backed_project_state | `project-context-subfamily-audit-v1-current5k.report.json` |
| artifact_backed_project_state | candidate_events | 14 | `project-context-subfamily-audit-v1-current5k.report.json` |
| artifact_backed_project_state | payload_ready_events | 14 | `project-context-subfamily-audit-v1-current5k.report.json` |
| project_context_payload | payload_built_events | 7 | `project-context-payload-dry-run-v1-current5k.report.json` |
| project_context_payload | scoreable_payload_events | 7 | `project-context-payload-dry-run-v1-current5k.report.json` |
| project_context_profile | edge_count | 8 | `project-context-profile-v1-current5k.report.json` |
| project_context_profile | disabled_threshold_accepts | 0 | `project-context-profile-v1-current5k.report.json` |
| project_context_shadow | verified_safe_accepts | 0 | `project-context-profile-v1-current5k.shadow-report.json` |
| project_context_shadow | false_accepts | 0 | `project-context-profile-v1-current5k.shadow-report.json` |
| project_context_evidence | artifact_evidence_matched_events | 7 | `project-context-output-evidence-v1-current5k.report.json` |
| project_context_evidence | verified_true_events | 1 | `project-context-output-evidence-v1-current5k.report.json` |
| project_context_evidence | verified_false_events | 6 | `project-context-output-evidence-v1-current5k.report.json` |
| project_context_calibration | safe_policy_found | true | `project-context-local-accept-calibration-v1-current5k.report.json` |
| project_context_calibration | best_safe_true_accepts | 1 | `project-context-local-accept-calibration-v1-current5k.report.json` |
| project_context_audit | verified_cpu_accept_eligible_events | 0 | `project-context-output-evidence-v1-current5k.verification-hook-audit.report.json` |
| project_context_current5k | shelf | WATCH_SINGLETON_ONLY | `docs/CPU_CALL_CATALOG.md` |
| project_context_current5k | broad_route_promote_policy | forbidden_without_min_support | `docs/CPU_CALL_CATALOG.md` |
