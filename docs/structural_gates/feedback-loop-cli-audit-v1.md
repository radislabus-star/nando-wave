# Feedback Loop CLI Audit V1

NANDA status: `VETO` for the current numeric packet shape. The gate reports
weak composite-mode support for exact numeric candidate triads. Treat this as
proof-shape debt, not as a feedback-loop runtime failure and not as a structural
PASS.

This packet checks the narrow CLI integration claim: the feedback-loop command
is wired, route-specific reports are auto-loaded when present, and CPU
Routability 80 remains unachieved.

This packet must not be used as a market-savings claim.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback loop command | stage_function | feedback_route_stage | role_binding_runtime_cmd.rs |
| feedback loop command | next_action_function | feedback_route_next_action | role_binding_runtime_cmd.rs |
| feedback loop command | cli_dispatch | main.rs | main.rs |
| feedback loop command | help_listing | help.rs | help.rs |
| feedback loop command | auto_loads_route_specific_reports | true | role_binding_runtime_cmd.rs |
| feedback loop help | mentions_route_specific_autoload | true | help.rs |
| current feedback report | total_llm_calls | 1000 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | exact_cache_hits | 53 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | operator_candidate_calls | 408 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | scoreable_candidate_calls | 82 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | verification_hook_ready_events | 65 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | verified_cpu_accept_eligible_events | 8 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | verified_gap_to_80_calls | 792 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| current feedback report | false_accepts | 0 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| overall CPU Routability 80 | required_verified_cpu_accepts | 800 | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |
| overall CPU Routability 80 | claim_status | not_achieved | cpu-route-feedback-loop-conditional-agent-control-v1.report.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback loop command | stage_function | feedback_route_stage | candidate claim |
| feedback loop command | next_action_function | feedback_route_next_action | candidate claim |
| feedback loop command | cli_dispatch | main.rs | candidate claim |
| feedback loop command | help_listing | help.rs | candidate claim |
| feedback loop command | auto_loads_route_specific_reports | true | candidate claim |
| feedback loop help | mentions_route_specific_autoload | true | candidate claim |
| current feedback report | total_llm_calls | 1000 | candidate claim |
| current feedback report | exact_cache_hits | 53 | candidate claim |
| current feedback report | operator_candidate_calls | 408 | candidate claim |
| current feedback report | scoreable_candidate_calls | 82 | candidate claim |
| current feedback report | verification_hook_ready_events | 65 | candidate claim |
| current feedback report | verified_cpu_accept_eligible_events | 8 | candidate claim |
| current feedback report | verified_gap_to_80_calls | 792 | candidate claim |
| current feedback report | false_accepts | 0 | candidate claim |
| overall CPU Routability 80 | required_verified_cpu_accepts | 800 | candidate claim |
| overall CPU Routability 80 | claim_status | not_achieved | candidate claim |
