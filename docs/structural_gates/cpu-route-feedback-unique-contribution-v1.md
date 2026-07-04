# CPU Route Feedback Unique Contribution V1

NANDA status: pending.

This packet checks the claim boundary for the CPU route feedback report. It
uses one metric-value route so the structural gate verifies the bindings
between metric names and numbers, not a prose claim.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_changed_file | value | crates/nando-cli/src/role_binding_runtime_cmd.rs | git diff |
| metric_route_sum_verified_cpu_eligible_hits | value | 28 | feedback field verified_cpu_accept_route_sum_events |
| metric_route_local_unique_fingerprint_sum | value | 23 | feedback field verified_cpu_accept_route_unique_sum_request_fingerprints |
| metric_global_unique_verified_cpu_accepts | value | 22 | feedback field verified_cpu_accept_unique_request_fingerprints |
| metric_conservative_incremental_unique_cpu_accepts | value | 21 | feedback field incremental_cpu_accept_unique_request_fingerprints |
| metric_within_route_duplicate_hits | value | 5 | feedback field verified_cpu_accept_duplicate_within_route_hits |
| metric_cross_route_overlap_fingerprints | value | 1 | feedback field verified_cpu_accept_cross_route_overlap_request_fingerprints |
| metric_retrieval_window5000_safe_policy_found | value | false | retrieval window5000 field safe_policy_found |
| metric_cpu80_remaining_debt | value | not achieved / unique gap to 80 is 778 calls | feedback field unique_verified_gap_to_80_calls |
| metric_market_claim_allowed | value | false | claim boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_changed_file | value | crates/nando-cli/src/role_binding_runtime_cmd.rs | git diff |
| metric_route_sum_verified_cpu_eligible_hits | value | 28 | feedback field verified_cpu_accept_route_sum_events |
| metric_route_local_unique_fingerprint_sum | value | 23 | feedback field verified_cpu_accept_route_unique_sum_request_fingerprints |
| metric_global_unique_verified_cpu_accepts | value | 22 | feedback field verified_cpu_accept_unique_request_fingerprints |
| metric_conservative_incremental_unique_cpu_accepts | value | 21 | feedback field incremental_cpu_accept_unique_request_fingerprints |
| metric_within_route_duplicate_hits | value | 5 | feedback field verified_cpu_accept_duplicate_within_route_hits |
| metric_cross_route_overlap_fingerprints | value | 1 | feedback field verified_cpu_accept_cross_route_overlap_request_fingerprints |
| metric_retrieval_window5000_safe_policy_found | value | false | retrieval window5000 field safe_policy_found |
| metric_cpu80_remaining_debt | value | not achieved / unique gap to 80 is 778 calls | feedback field unique_verified_gap_to_80_calls |
| metric_market_claim_allowed | value | false | claim boundary |
