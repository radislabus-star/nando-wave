# CPU Call Catalog Business Value Gate V1

## query

Check the core relation: CPU call catalog separates verified CPU savings from
candidate/backlog rows and rejects broad routes as full-route promotions.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| cpu_call_catalog_business_value_v1 | reads | cpu-route-feedback-loop-v5.post-ime-current.report.json | command input |
| cpu_call_catalog_business_value_v1 | writes | cpu-call-catalog-business-value-v1.report.json | command output |
| cpu_call_catalog_business_value_v1 | current_incremental_unique_accepts | 25 | report summary |
| cpu_call_catalog_business_value_v1 | business_gate_passed_rows | 7 | report summary |
| business_value_gate | requires_expected | expected_unique_cpu_accepts_over_exact_cache_gt_zero | doc business gate expected unique rule |
| business_value_gate | requires_safety | false_accepts_eq_zero | doc business gate false accept rule |
| scoreable_rows | are_not_market_savings | true | doc scoreable boundary |
| candidate_rows | are_not_market_savings | true | doc candidate boundary |
| route_gap_family_rows | are_not_duplicate_proven_savings | true | route gap duplicate rule |
| answer_or_explain | status | REJECT_FOR_NOW | answer route report row |
| project_context_dialogue | status | REJECT_FOR_NOW | project context report row |
| agent_continue_execute | status | REJECT_FOR_NOW | agent continue report row |
