# IME Input-State Disabled Profile V1

This packet checks one risky role swap only: a disabled IME input-state profile
is a scoreable serving path, not a verified CPU accept and not market savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| ime_input_state_profile | profile_registered | true | ime-input-state-profile-v1.report.json |
| ime_input_state_profile | local_accepts_enabled | false | ime-input-state-profile-v1.report.json |
| ime_input_state_shadow | nando_shadow_accepts | 0 | ime-input-state-profile-shadow-v1.report.json |
| ime_input_state_audit | verification_hook_ready_events | 5 | ime-input-state-profile-v1.verification-hook-audit.report.json |
| ime_input_state_feedback | stage | scoreable_payload_profile_registered_accepts_disabled | docs/EXECUTOR_REVIEW_NOTES.md |
| ime_input_state_feedback | verified_cpu_accept_eligible_events | 0 | ime-input-state-profile-v1.verification-hook-audit.report.json |
| cpu80_overall | unique_verified_cpu_accepts | 26 | docs/EXECUTOR_REVIEW_NOTES.md |
| cpu80_overall | unique_gap_to_80 | 774 | docs/EXECUTOR_REVIEW_NOTES.md |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| ime_input_state_profile | profile_registered | true | candidate claim |
| ime_input_state_profile | local_accepts_enabled | false | candidate claim |
| ime_input_state_shadow | nando_shadow_accepts | 0 | candidate claim |
| ime_input_state_audit | verification_hook_ready_events | 5 | candidate claim |
| ime_input_state_feedback | stage | scoreable_payload_profile_registered_accepts_disabled | candidate claim |
| ime_input_state_feedback | verified_cpu_accept_eligible_events | 0 | candidate claim |
| cpu80_overall | unique_verified_cpu_accepts | 26 | candidate claim |
| cpu80_overall | unique_gap_to_80 | 774 | candidate claim |

