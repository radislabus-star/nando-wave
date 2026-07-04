# Resource Pressure Payload Dry Run V1

This packet checks one narrow claim boundary: `resource_pressure_budget`
advanced from manual discovery to one request-side scoreable payload, but it
did not enable local accepts or market-savings claims.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | resource_pressure_payload_dry_run_report | verdict | RESOURCE_PRESSURE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | verdict | resource_pressure_budget | report |
| t2 | resource_pressure_payload_dry_run_report | resource_pressure_candidate_events | 3 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| t3 | resource_pressure_payload_dry_run_report | payload_ready_events | 1 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| t4 | resource_pressure_payload_dry_run_report | scoreable_payload_events | 1 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| t5 | resource_pressure_payload_dry_run_report | market_claim_allowed | false | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | claim guard | resource_pressure_budget | claim |
| t6 | cpu_operator_catalog_uncatalogued_row | scoreable_payload_events | 1 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | count | catalog | advisory |
| t7 | cpu_operator_catalog_uncatalogued_row | verified_cpu_accept_eligible_events | 0 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | claim guard | catalog | advisory |
| t8 | resource_pressure_payload_dry_run_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Resource Pressure Payload Dry Run V1 | 0.99 | engineering decision | claim guard | resource_pressure_budget | decision |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | resource_pressure_payload_dry_run_report | verdict | RESOURCE_PRESSURE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | verdict | resource_pressure_budget | report |
| c2 | resource_pressure_payload_dry_run_report | resource_pressure_candidate_events | 3 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| c3 | resource_pressure_payload_dry_run_report | payload_ready_events | 1 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| c4 | resource_pressure_payload_dry_run_report | scoreable_payload_events | 1 | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | count | resource_pressure_budget | metric |
| c5 | resource_pressure_payload_dry_run_report | market_claim_allowed | false | resource-pressure-payload-dry-run-v1.report.json | 0.99 | dry-run report | claim guard | resource_pressure_budget | claim |
| c6 | cpu_operator_catalog_uncatalogued_row | scoreable_payload_events | 1 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | count | catalog | advisory |
| c7 | cpu_operator_catalog_uncatalogued_row | verified_cpu_accept_eligible_events | 0 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | claim guard | catalog | advisory |
| c8 | resource_pressure_payload_dry_run_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Resource Pressure Payload Dry Run V1 | 0.99 | engineering decision | claim guard | resource_pressure_budget | decision |
