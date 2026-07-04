# Project-Context Workspace Evidence Hook V1

This packet checks one risky role swap only: a project-context workspace
evidence hook makes a scoreable row verification-hook-ready, not locally
accepted, not verified CPU savings, and not a market claim.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| project_context_evidence | artifact_evidence_matched_events | 2 | project-context-output-evidence-v1.report.json |
| project_context_evidence | verified_true_events | 1 | project-context-output-evidence-v1.report.json |
| project_context_evidence | verified_false_events | 1 | project-context-output-evidence-v1.report.json |
| project_context_shadow | nando_shadow_accepts | 0 | project-context-output-evidence-v1.shadow-report.json |
| project_context_audit | verification_hook_ready_events | 2 | project-context-output-evidence-v1.verification-hook-audit.report.json |
| project_context_calibration | best_safe_true_accepts | 1 | project-context-local-accept-calibration-v1.report.json |
| project_context_feedback | stage | local_accept_calibration_support_insufficient | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_verified_cpu_accepts | 26 | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_gap_to_80 | 774 | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| project_context_evidence | artifact_evidence_matched_events | 2 | candidate claim |
| project_context_evidence | verified_true_events | 1 | candidate claim |
| project_context_evidence | verified_false_events | 1 | candidate claim |
| project_context_shadow | nando_shadow_accepts | 0 | candidate claim |
| project_context_audit | verification_hook_ready_events | 2 | candidate claim |
| project_context_calibration | best_safe_true_accepts | 1 | candidate claim |
| project_context_feedback | stage | local_accept_calibration_support_insufficient | candidate claim |
| cpu80_overall | unique_verified_cpu_accepts | 26 | candidate claim |
| cpu80_overall | unique_gap_to_80 | 774 | candidate claim |
