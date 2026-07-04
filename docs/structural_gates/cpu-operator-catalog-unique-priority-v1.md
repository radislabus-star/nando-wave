# CPU Operator Catalog Unique Priority V1

NANDA status: PASS.

This packet checks the claim boundary for the CPU operator catalog rerank. It
uses one metric-value route so the structural gate verifies the bindings
between route-sum, unique, incremental, duplicate, and priority fields.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_incremental_unique_cpu_accepts | value | 21 | catalog field incremental_cpu_accept_unique_request_fingerprints |
| metric_verified_gap_to_80_calls | value | 778 | catalog field verified_gap_to_80_calls |
| metric_catalog_top_row | value | role_binding_conditional_branch_seed0 | catalog row priority_rank=1 |
| metric_agent_control_route_sum | value | 11 | catalog agent_control row verified_cpu_accept_eligible_events |
| metric_agent_control_incremental_unique | value | 5 | catalog agent_control row incremental_cpu_accept_unique_request_fingerprints |
| metric_agent_control_duplicate_route_hits | value | 5 | catalog agent_control row duplicate_verified_route_hits |
| metric_agent_control_exact_cache_overlap | value | 1 | catalog agent_control row exact_cache_overlap_verified_cpu_accepts |
| metric_priority_basis | value | incremental unique verified accepts, not route-sum duplicate accepts | catalog claim_boundary |
| metric_market_claim_allowed | value | false | catalog claim boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_incremental_unique_cpu_accepts | value | 21 | catalog field incremental_cpu_accept_unique_request_fingerprints |
| metric_verified_gap_to_80_calls | value | 778 | catalog field verified_gap_to_80_calls |
| metric_catalog_top_row | value | role_binding_conditional_branch_seed0 | catalog row priority_rank=1 |
| metric_agent_control_route_sum | value | 11 | catalog agent_control row verified_cpu_accept_eligible_events |
| metric_agent_control_incremental_unique | value | 5 | catalog agent_control row incremental_cpu_accept_unique_request_fingerprints |
| metric_agent_control_duplicate_route_hits | value | 5 | catalog agent_control row duplicate_verified_route_hits |
| metric_agent_control_exact_cache_overlap | value | 1 | catalog agent_control row exact_cache_overlap_verified_cpu_accepts |
| metric_priority_basis | value | incremental unique verified accepts, not route-sum duplicate accepts | catalog claim_boundary |
| metric_market_claim_allowed | value | false | catalog claim boundary |
