# CPU Operator Catalog Refresh After Test Output Parse

## query

Check that the CPU operator catalog refresh uses the canonical
`cpu-operator-catalog-v1.report.json`, includes the full-window
`test_output_parse` contribution as 10 unique expected accepts, and keeps CPU80
unproven.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| cpu_call_catalog_doc | source_report | cpu_operator_catalog_v1_report | docs/CPU_CALL_CATALOG.md |
| catalog_command | reads | cpu_route_feedback_loop_v1 | command_output |
| catalog_report | business_value_gate_passed_rows | 8 | cpu_operator_catalog_v1_report |
| catalog_report | proven_profile_rows | 8 | cpu_operator_catalog_v1_report |
| catalog_report | current_unique_verified_accepts | 36 | cpu_operator_catalog_v1_report |
| catalog_report | current_incremental_unique_accepts | 35 | cpu_operator_catalog_v1_report |
| test_output_parse_row | priority_rank | 1 | cpu_operator_catalog_v1_report |
| test_output_parse_row | current_status | PROVEN | cpu_operator_catalog_v1_report |
| test_output_parse_row | expected_unique_accepts | 10 | cpu_operator_catalog_v1_report |
| test_output_parse_row | false_accepts | 0 | cpu_operator_catalog_v1_report |
| test_output_parse_row | business_value_gate_passed | true | cpu_operator_catalog_v1_report |
| agent_loop_registry | top_next_profile_key | none | agent_loop_profile_registry_report |
| cpu80_claim | remains | not_proven | executor_review_notes |
| next_growth | requires | sibling_split_with_new_verified_uniques | executor_review_notes |
