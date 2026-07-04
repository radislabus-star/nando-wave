# Agent-Control Admission Audit V1

NANDA status: PASS.

This packet checks only the rerank claim: agent-control current stop/control
support is exhausted, so the CPU catalog must move to the next route.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_agent_control_best_robust_true_accepts | value | 11 | field best_robust_true_accepts |
| metric_agent_control_verified_event_support | value | 11 | field verified_cpu_accept_eligible_events |
| metric_agent_control_incremental_unique_accepts | value | 5 | field incremental_verified_request_fingerprints |
| metric_agent_control_duplicate_hits | value | 5 | field duplicate_verified_route_hits |
| metric_agent_control_exact_cache_overlap | value | 1 | field exact_cache_overlap_verified_request_fingerprints |
| metric_agent_control_support_exhausted | value | true | field current_policy_event_support_exhausted |
| metric_catalog_top_row_after_audit | value | git_control | catalog report |
| metric_cpu80_unique_verified_accepts | value | 22 | catalog report |
| metric_market_claim_allowed | value | false | claim boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_agent_control_best_robust_true_accepts | value | 11 | field best_robust_true_accepts |
| metric_agent_control_verified_event_support | value | 11 | field verified_cpu_accept_eligible_events |
| metric_agent_control_incremental_unique_accepts | value | 5 | field incremental_verified_request_fingerprints |
| metric_agent_control_duplicate_hits | value | 5 | field duplicate_verified_route_hits |
| metric_agent_control_exact_cache_overlap | value | 1 | field exact_cache_overlap_verified_request_fingerprints |
| metric_agent_control_support_exhausted | value | true | field current_policy_event_support_exhausted |
| metric_catalog_top_row_after_audit | value | git_control | catalog report |
| metric_cpu80_unique_verified_accepts | value | 22 | catalog report |
| metric_market_claim_allowed | value | false | claim boundary |
