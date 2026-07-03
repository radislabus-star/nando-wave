# mixed-safe-policy-v2

## Claim

The mixed-map route now has a v2 request-side safe policy that admits verified
local CPU accepts only after rejecting goal/control/meta prompts before energy
threshold selection. This raises the mixed route from 2 verified accepts in v1
to 3 verified accepts in v2 on the current real Codex trace window.

This packet checks route coherence only. Exact numeric metrics live in the JSON
reports and in `docs/EXECUTOR_REVIEW_NOTES.md`. This packet must not be used as
a global CPU Routability 80 or market-wide savings claim.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| mixed_safe_policy_v2 | route | role_binding_mixed_map_seed0 | mixed-safe-policy-v2.report.json |
| mixed_safe_policy_v2 | request_side_policy | mixed_no_goal_control_prompt | mixed-safe-policy-v2.report.json |
| mixed_no_goal_control_prompt | rejects | goal_control_meta_prompts | role_binding_runtime_cmd.rs |
| mixed_safe_policy_v2 | threshold | 393216 | mixed-safe-policy-v2.report.json |
| mixed_safe_policy_v2 | verified_true_accepts | 3 | mixed-safe-policy-v2.report.json |
| mixed_safe_policy_v2 | verified_false_accepts | 0 | mixed-safe-policy-v2.report.json |
| mixed_safe_policy_v2 | unverified_accepts | 0 | mixed-safe-policy-v2.report.json |
| mixed_safe_policy_v2 | runtime_mismatches | 0 | mixed-safe-policy-v2.report.json |
| mixed_shadow_v2 | accepts | 3 | mixed-safe-policy-v2.shadow-report.json |
| mixed_shadow_v2 | verified_safe_accepts | 3 | mixed-safe-policy-v2.shadow-report.json |
| mixed_shadow_v2 | false_accepts | 0 | mixed-safe-policy-v2.shadow-report.json |
| mixed_audit_v2 | hook_ready | 11 | mixed-safe-policy-v2.verification-hook-audit.report.json |
| mixed_audit_v2 | verified_cpu_accepts | 3 | mixed-safe-policy-v2.verification-hook-audit.report.json |
| feedback_loop_v3 | verified_cpu_accepts | 17 | cpu-route-feedback-loop-v3.mixed-v2-agent-control-planning-v3.report.json |
| feedback_loop_v3 | gap_to_80 | 783 | cpu-route-feedback-loop-v3.mixed-v2-agent-control-planning-v3.report.json |
| feedback_loop_v3 | market_claim_allowed | false | cpu-route-feedback-loop-v3.mixed-v2-agent-control-planning-v3.report.json |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| mixed_safe_policy_v2 | route | role_binding_mixed_map_seed0 | candidate claim |
| mixed_safe_policy_v2 | request_side_policy | mixed_no_goal_control_prompt | candidate claim |
| mixed_no_goal_control_prompt | rejects | goal_control_meta_prompts | candidate claim |
| mixed_safe_policy_v2 | threshold | 393216 | candidate claim |
| mixed_safe_policy_v2 | verified_true_accepts | 3 | candidate claim |
| mixed_safe_policy_v2 | verified_false_accepts | 0 | candidate claim |
| mixed_safe_policy_v2 | unverified_accepts | 0 | candidate claim |
| mixed_safe_policy_v2 | runtime_mismatches | 0 | candidate claim |
| mixed_shadow_v2 | accepts | 3 | candidate claim |
| mixed_shadow_v2 | verified_safe_accepts | 3 | candidate claim |
| mixed_shadow_v2 | false_accepts | 0 | candidate claim |
| mixed_audit_v2 | hook_ready | 11 | candidate claim |
| mixed_audit_v2 | verified_cpu_accepts | 3 | candidate claim |
| feedback_loop_v3 | verified_cpu_accepts | 17 | candidate claim |
| feedback_loop_v3 | gap_to_80 | 783 | candidate claim |
| feedback_loop_v3 | market_claim_allowed | false | candidate claim |
