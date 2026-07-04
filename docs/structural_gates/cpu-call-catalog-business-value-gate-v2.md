# CPU Call Catalog Business Value Gate V2

## query

Verify the updated CPU call catalog gate: Nando profiles are selected by
real-runtime business value, not by technical interest. Candidate rows are not
savings. Online discovery may nominate profiles, but local accept still
requires a deterministic verifier and false_accepts=0.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| nando_product | sold_as | per_runtime_cpu_call_catalog | `docs/CPU_CALL_CATALOG.md#product-identity` |
| per_runtime_catalog | mined_from | real_runtime_traffic | `docs/CPU_CALL_CATALOG.md#market-trajectory` |
| business_value_gate | requires | complete_eight_field_gate | `docs/CPU_CALL_CATALOG.md#business-value-gate-fields` |
| candidate_rows | savings_policy | not_counted_as_savings | `docs/CPU_CALL_CATALOG.md#catalog-savings-policy` |
| broad_routes | promotion_policy | split_before_profile_work | `docs/CPU_CALL_CATALOG.md#current5k-discovery-triage` |
| online_operator_discovery | may_nominate | repeated_action_centers | `docs/CPU_CALL_CATALOG.md#online-operator-discovery` |
| online_operator_discovery | local_accept_policy | verifier_required | `docs/CPU_CALL_CATALOG.md#online-operator-discovery` |
| singleton_policy | cpu80_policy | watch_not_promote | `docs/CPU_CALL_CATALOG.md#current5k-discovery-triage` |
| next_profile_work | blocked_without | business_value_gate | `docs/CPU_CALL_CATALOG.md#hard-stop` |
