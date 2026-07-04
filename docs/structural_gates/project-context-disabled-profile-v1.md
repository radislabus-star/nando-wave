# Project-Context Disabled Profile V1

This packet checks one risky role swap only: a disabled project-context profile
is a scoreable serving path for artifact-backed rows, not a verified CPU accept
and not market savings. Broad project dialogue remains fallback-only.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| project_context_profile | profile_registered | true | project-context-payload-dry-run-v1.report.json |
| project_context_profile | local_accepts_enabled | false | project-context-profile-v1.report.json |
| project_context_profile | scoreable_payload_events | 2 | project-context-profile-v1.report.json |
| project_context_dry_run | project_context_candidate_events | 211 | project-context-payload-dry-run-v1.report.json |
| project_context_shadow | nando_shadow_accepts | 0 | project-context-profile-v1.shadow-report.json |
| project_context_audit | verification_hook_ready_events | 0 | project-context-profile-v1.verification-hook-audit.report.json |
| project_context_feedback | stage | scoreable_payload_missing_verification_hook | cpu-route-feedback-loop-v1.report.json |
| project_context_feedback | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_verified_cpu_accepts | 26 | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_gap_to_80 | 774 | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| project_context_profile | profile_registered | true | candidate claim |
| project_context_profile | local_accepts_enabled | false | candidate claim |
| project_context_profile | scoreable_payload_events | 2 | candidate claim |
| project_context_dry_run | project_context_candidate_events | 211 | candidate claim |
| project_context_shadow | nando_shadow_accepts | 0 | candidate claim |
| project_context_audit | verification_hook_ready_events | 0 | candidate claim |
| project_context_feedback | stage | scoreable_payload_missing_verification_hook | candidate claim |
| project_context_feedback | verified_cpu_accept_eligible_events | 0 | candidate claim |
| cpu80_overall | unique_verified_cpu_accepts | 26 | candidate claim |
| cpu80_overall | unique_gap_to_80 | 774 | candidate claim |
