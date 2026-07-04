# agent-continue-admission-feedback-catalog-v1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| feedback_loop_command | reads | agent_continue_admission_calibration_report | feedback loader in `crates/nando-cli/src/role_binding_runtime_cmd.rs` |
| cpu_operator_catalog_command | reads | agent_continue_admission_calibration_report | catalog loader in `crates/nando-cli/src/role_binding_runtime_cmd.rs` |
| agent_continue_admission_calibration_report | found_robust_safe_policy | `false` | report robust_safe_policy_found |
| agent_continue_admission_calibration_report | found_singleton_safe_policy | `false` | report singleton_safe_policy_found |
| agent_continue_admission_calibration_report | best_robust_true_accepts | `0` | report best_robust_true_accepts |
| agent_continue_admission_calibration_report | best_singleton_true_accepts | `0` | report best_singleton_true_accepts |
| feedback_agent_continue_route | has_stage | `local_accept_calibration_failed` | `target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json` route row |
| feedback_agent_continue_route | found_local_accept_safe_policy | `false` | feedback route local_accept_safe_policy_found |
| feedback_agent_continue_route | best_safe_true_accepts | `0` | feedback route local_accept_best_safe_true_accepts |
| feedback_agent_continue_route | has_verification_hook_ready_events | `25` | feedback route verification_hook_ready_events |
| feedback_agent_continue_route | has_verified_cpu_accept_eligible_events | `0` | feedback route verified_cpu_accept_eligible_events |
| feedback_agent_continue_route | has_false_accepts | `0` | feedback route false_accepts |
| feedback_agent_continue_route | next_action | split_or_capture_richer_request_side_state | feedback route next_action |
| catalog_existing_agent_continue_route | has_agent_continue_admission_no_safe_policy | `true` | catalog existing_profile_route row |
| catalog_existing_agent_continue_route | best_robust_true_accepts | `0` | catalog row agent_continue_admission_best_robust_true_accepts |
| catalog_existing_agent_continue_route | best_singleton_true_accepts | `0` | catalog row agent_continue_admission_best_singleton_true_accepts |
| catalog_existing_agent_continue_route | next_action | split_or_capture_richer_request_side_state | catalog row next_action |
| catalog_route_gap_agent_continue_route | has_agent_continue_admission_no_safe_policy | `true` | catalog route_gap_family row |
| catalog_route_gap_agent_continue_route | next_action | do_not_promote_route_gap_row | catalog row next_action |
| feedback_report | counts_unique_verified_cpu_accepts | `26` | feedback report verified_cpu_accept_unique_request_fingerprints |
| feedback_report | counts_incremental_unique_cpu_accepts | `25` | feedback report incremental_cpu_accept_unique_request_fingerprints |
| feedback_report | allows_market_claim | `false` | feedback report market_claim_allowed |
| catalog_report | allows_market_claim | `false` | catalog report market_claim_allowed |
| integration_change | adds_verified_cpu_accepts | `false` | unique/incremental accepts unchanged after integration |
| integration_change | enables_local_accepts | `false` | report local accepts remain disabled/review-only |
| next_engineering_debt | requires | route_split_or_richer_request_side_state | executor review notes |
