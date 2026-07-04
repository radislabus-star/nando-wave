# Resource Pressure Output Evidence V1

This packet checks one narrow claim boundary: `resource_pressure_budget`
advanced from scoreable-only to verifier-hook-ready, but it still has zero
verified CPU accepts and no market-savings claim.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | resource_pressure_output_evidence_report | verdict | RESOURCE_PRESSURE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verdict | resource_pressure_budget | report |
| t2 | resource_pressure_output_evidence_report | output_evidence_matched_events | 1 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | count | resource_pressure_budget | metric |
| t3 | resource_pressure_output_evidence_report | verified_true_events | 0 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verifier count | resource_pressure_budget | verifier |
| t4 | resource_pressure_output_evidence_report | verified_false_events | 1 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verifier count | resource_pressure_budget | verifier |
| t5 | resource_pressure_audit_report | verification_hook_ready_events | 1 | resource-pressure-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | count | resource_pressure_budget | audit |
| t6 | resource_pressure_audit_report | verified_cpu_accept_eligible_events | 0 | resource-pressure-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | claim guard | resource_pressure_budget | audit |
| t7 | cpu_operator_catalog_uncatalogued_row | verification_hook_ready_events | 1 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | count | catalog | advisory |
| t8 | resource_pressure_output_evidence_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Resource Pressure Output Evidence V1 | 0.99 | engineering decision | claim guard | resource_pressure_budget | decision |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | resource_pressure_output_evidence_report | verdict | RESOURCE_PRESSURE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verdict | resource_pressure_budget | report |
| c2 | resource_pressure_output_evidence_report | output_evidence_matched_events | 1 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | count | resource_pressure_budget | metric |
| c3 | resource_pressure_output_evidence_report | verified_true_events | 0 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verifier count | resource_pressure_budget | verifier |
| c4 | resource_pressure_output_evidence_report | verified_false_events | 1 | resource-pressure-output-evidence-v1.report.json | 0.99 | output evidence report | verifier count | resource_pressure_budget | verifier |
| c5 | resource_pressure_audit_report | verification_hook_ready_events | 1 | resource-pressure-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | count | resource_pressure_budget | audit |
| c6 | resource_pressure_audit_report | verified_cpu_accept_eligible_events | 0 | resource-pressure-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | claim guard | resource_pressure_budget | audit |
| c7 | cpu_operator_catalog_uncatalogued_row | verification_hook_ready_events | 1 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | count | catalog | advisory |
| c8 | resource_pressure_output_evidence_decision | may_count_as_cpu_savings | false | docs/EXECUTOR_REVIEW_NOTES.md Resource Pressure Output Evidence V1 | 0.99 | engineering decision | claim guard | resource_pressure_budget | decision |
