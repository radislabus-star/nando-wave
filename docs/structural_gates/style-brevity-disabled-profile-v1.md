# Style-Brevity Disabled Profile V1

This packet checks one risky role swap only: a disabled style-brevity profile is
a scoreable serving path, not a verified CPU accept and not market savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| style_brevity_profile | profile_registered | true | style-brevity-payload-dry-run-v1.report.json |
| style_brevity_profile | local_accepts_enabled | false | style-brevity-profile-v1.report.json |
| style_brevity_shadow | nando_shadow_accepts | 0 | style-brevity-profile-v1.shadow-report.json |
| style_brevity_audit | verification_hook_ready_events | 0 | style-brevity-profile-v1.verification-hook-audit.report.json |
| style_brevity_feedback | stage | scoreable_payload_missing_verification_hook | cpu-route-feedback-loop-v1.report.json |
| style_brevity_feedback | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_verified_cpu_accepts | 26 | cpu-route-feedback-loop-v1.report.json |
| cpu80_overall | unique_gap_to_80 | 774 | cpu-route-feedback-loop-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| style_brevity_profile | profile_registered | true | candidate claim |
| style_brevity_profile | local_accepts_enabled | false | candidate claim |
| style_brevity_shadow | nando_shadow_accepts | 0 | candidate claim |
| style_brevity_audit | verification_hook_ready_events | 0 | candidate claim |
| style_brevity_feedback | stage | scoreable_payload_missing_verification_hook | candidate claim |
| style_brevity_feedback | verified_cpu_accept_eligible_events | 0 | candidate claim |
| cpu80_overall | unique_verified_cpu_accepts | 26 | candidate claim |
| cpu80_overall | unique_gap_to_80 | 774 | candidate claim |
