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
| current5k_catalog | incremental_unique_accepts | 94 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| current5k_catalog | gap_to_cpu80_calls | 3906 | `cpu-route-feedback-loop-v1-current5k.combined.report.json` |
| current5k_catalog | proven_profile_rows | 2 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| test_output_parse_profile | shelf | PROVEN | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| test_output_parse_profile | incremental_unique_accepts | 91 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| metrics_report_readout_profile | shelf | PROVEN_SMALL_SUPPORT | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| metrics_report_readout_profile | incremental_unique_accepts | 3 | `cpu-operator-catalog-v1-current5k.combined.report.json` |
| agent_control_profile | shelf | WATCH_NO_SAFE_POLICY | `agent-control-admission-calibration-v1-5k.report.json` |
| agent_control_profile | robust_safe_policy_found | false | `agent-control-admission-calibration-v1-5k.report.json` |
| broad_routes | shelf | REJECT_FOR_NOW_UNTIL_SPLIT | `docs/CPU_CALL_CATALOG.md` |
| business_value_gate | blocks_profile_without | real_traffic_and_verifier_and_unique_accepts | `docs/CPU_CALL_CATALOG.md` |
| current5k_catalog | claim_boundary | not_cpu80 | `docs/CPU_CALL_CATALOG.md` |
| current5k_catalog | savings_count_policy | verified_unique_only | `docs/CPU_CALL_CATALOG.md` |
| agent_control_profile | promotion_policy | blocked_until_safe_policy | `docs/CPU_CALL_CATALOG.md` |
| metrics_report_readout_profile | claim_boundary | small_support_only | `docs/CPU_CALL_CATALOG.md` |
| broad_routes | promotion_policy | split_required | `docs/CPU_CALL_CATALOG.md` |
| next_profile_selection | policy | business_value_gate_required | `docs/CPU_CALL_CATALOG.md` |
