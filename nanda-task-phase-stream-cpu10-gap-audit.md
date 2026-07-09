# NANDA Task: Phase Stream CPU10 Gap Audit

## Query

Check that `phase-stream-real-traffic-cpu10-gap-audit-v1` is an audit-only
capacity report. It may compare the current safe frontier against verifier-ready
trace ceiling and CPU10 target, but it must not treat the ceiling as achieved
accepts, enable local accept, promote serving runtime, claim market money, or
revive `.nwrb`.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| cpu10-gap | entrypoint | phase-stream-real-traffic-cpu10-gap-audit-v1 | crates/nando-cli/src/main.rs |
| cpu10-gap | implementation | run_phase_stream_real_traffic_cpu10_gap_audit_v1 | crates/nando-cli/src/phase_streaming_cmd.rs |
| cpu10-gap | report | real-traffic-phase-center-cpu10-gap-audit-v1.report.json | command output |
| cpu10-gap | target_accepts | 500 | target_cpu_accepts_over_exact_cache |
| cpu10-gap | current_safe_accepts | 68 | current_safe_accepts_over_exact_cache |
| cpu10-gap | remaining_gap | 432 | remaining_accept_gap_to_cpu10 |
| cpu10-gap | verifier_true_ceiling | 107 | verifier_true_over_exact_cache_ceiling |
| cpu10-gap | extra_true_needed | 393 | additional_verifier_true_over_exact_cache_needed_for_cpu10 |
| cpu10-gap | scoring_only_can_reach_target | false | current_trace_pool_can_reach_cpu10_by_scoring_only |
| cpu10-gap | current_tokens_saved | 55626 | current_safe_tokens_saved |
| cpu10-gap | current_cost_saved_microusd | 60098 | current_safe_cost_saved_microusd |
| route-agent-continue | true_ceiling | 68 | route report |
| route-metrics-report | true_ceiling | 31 | route report |
| cpu10-gap | local_accept | false | report boundary |
| cpu10-gap | market_claim | false | report boundary |
| cpu10-gap | forbidden_flags | all_false | report forbidden_flags |
| executor-notes | records | scoring_only_cannot_reach_cpu10 | docs/EXECUTOR_REVIEW_NOTES.md |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| cpu10-gap | treats | true_ceiling_as_achieved_accepts | negative-contract:ceiling_not_achieved |
| cpu10-gap | enables | product_local_accept | negative-contract:local_accept_false |
| cpu10-gap | promotes | serving_runtime | negative-contract:serving_runtime_false |
| cpu10-gap | revives | nwrb_role_binding_backend | negative-contract:legacy_backend_false |
| cpu10-gap | claims | market_money_proof | negative-contract:market_claim_false |
| cpu10-gap | claims | cpu10_complete | negative-contract:68_not_500 |
