# CPU Catalog Current5k Business Value Gate V1

## query

Verify the current5k CPU catalog claim boundary: use the current window as a
business-value filter, count only verified unique CPU accepts over exact cache,
keep weak or broad routes out of promotion, and do not claim CPU80.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| current5k_catalog | total_llm_calls | 5000 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| current5k_catalog | exact_cache_hits | 452 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| current5k_catalog | incremental_unique_accepts | 101 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| current5k_catalog | gap_to_cpu80_calls | 3899 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| current5k_catalog | proven_profile_rows | 4 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| business_value_gate | blocks_profile_without | real_traffic_and_verifier_and_unique_accepts | `docs/CPU_CALL_CATALOG.md` |
| current5k_catalog | claim_boundary | not_cpu80 | `docs/CPU_CALL_CATALOG.md` |
| current5k_catalog | savings_count_policy | verified_unique_only | `docs/CPU_CALL_CATALOG.md` |
| broad_routes | promotion_policy | split_required | `docs/CPU_CALL_CATALOG.md` |
| next_profile_selection | policy | business_value_gate_required | `docs/CPU_CALL_CATALOG.md` |
