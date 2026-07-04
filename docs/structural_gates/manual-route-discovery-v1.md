# Manual Route Discovery V1

This packet checks one narrow claim boundary: manual route discovery identified
`resource_pressure_budget` as the top uncatalogued subfamily, but it did not
enable local accepts or market-savings claims.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | manual_route_discovery_report | top_subfamily | resource_pressure_budget | manual-route-discovery-v1.report.json | 0.99 | discovery report | route family | manual_route_discovery | report |
| t2 | manual_route_discovery_report | raw_text_written | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | privacy guard | manual_route_discovery | claim |
| t3 | manual_route_discovery_report | local_accepts_enabled | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | claim guard | manual_route_discovery | claim |
| t4 | manual_route_discovery_report | market_claim_allowed | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | claim guard | manual_route_discovery | claim |
| t5 | resource_pressure_budget | recommended_payload_builder | resource_pressure_payload_builder_v1 | manual-route-discovery-v1.report.json | 0.99 | discovered subfamily | builder id | resource_pressure_budget | next_route |
| t6 | resource_pressure_budget | recommended_verifier | write_rate_or_resource_budget_verifier_v1 | manual-route-discovery-v1.report.json | 0.99 | discovered subfamily | verifier id | resource_pressure_budget | next_route |
| t7 | cpu_operator_catalog_uncatalogued_row | manual_route_discovery_top_subfamily | resource_pressure_budget | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | route family | catalog | advisory |
| t8 | manual_route_discovery_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Manual Route Discovery V1 | 0.99 | engineering decision | claim guard | manual_route_discovery | decision |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | manual_route_discovery_report | top_subfamily | resource_pressure_budget | manual-route-discovery-v1.report.json | 0.99 | discovery report | route family | manual_route_discovery | report |
| c2 | manual_route_discovery_report | raw_text_written | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | privacy guard | manual_route_discovery | claim |
| c3 | manual_route_discovery_report | local_accepts_enabled | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | claim guard | manual_route_discovery | claim |
| c4 | manual_route_discovery_report | market_claim_allowed | false | manual-route-discovery-v1.report.json | 0.99 | discovery report | claim guard | manual_route_discovery | claim |
| c5 | resource_pressure_budget | recommended_payload_builder | resource_pressure_payload_builder_v1 | manual-route-discovery-v1.report.json | 0.99 | discovered subfamily | builder id | resource_pressure_budget | next_route |
| c6 | resource_pressure_budget | recommended_verifier | write_rate_or_resource_budget_verifier_v1 | manual-route-discovery-v1.report.json | 0.99 | discovered subfamily | verifier id | resource_pressure_budget | next_route |
| c7 | cpu_operator_catalog_uncatalogued_row | manual_route_discovery_top_subfamily | resource_pressure_budget | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | route family | catalog | advisory |
| c8 | manual_route_discovery_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Manual Route Discovery V1 | 0.99 | engineering decision | claim guard | manual_route_discovery | decision |
