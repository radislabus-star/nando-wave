# Resource Pressure Feedback Loop V1 Structural Gate

## triads

| subject | relation | object | evidence | subject_role | object_role |
|---|---|---|---|---|---|
| feedback-loop-v1 | exposes route row | resource_pressure_budget | cpu-route-feedback-loop-v1.report.json routes includes route_key=resource_pressure_budget; route_count=15 | report_builder | route_visibility |
| feedback-loop-v1 | loads dry-run evidence | resource-pressure-payload-dry-run-v1.report.json | crates/nando-cli/src/role_binding_runtime_cmd.rs reads DEFAULT_RESOURCE_PRESSURE_PAYLOAD_DRY_RUN_REPORT; cpu-route-feedback-loop-v1.report.json includes resource_pressure_payload_dry_run_report_path | report_builder | dry_run_artifact |
| feedback-loop-v1 | loads verification evidence | resource-pressure-output-evidence-v1.verification-hook-audit.report.json | crates/nando-cli/src/role_binding_runtime_cmd.rs reads DEFAULT_RESOURCE_PRESSURE_OUTPUT_EVIDENCE_AUDIT_REPORT; cpu-route-feedback-loop-v1.report.json includes resource_pressure_verification_audit_report_path | report_builder | audit_artifact |
| resource_pressure_budget | candidate_events | 3 | cpu-route-feedback-loop-v1.report.json route_key resource_pressure_budget candidate_events=3 | route_row | candidate_count |
| resource_pressure_budget | scoreable_payload_events | 1 | cpu-route-feedback-loop-v1.report.json route_key resource_pressure_budget scoreable_payload_events=1 | route_row | scoreable_count |
| resource_pressure_budget | verification_hook_ready_events | 1 | cpu-route-feedback-loop-v1.report.json route_key resource_pressure_budget verification_hook_ready_events=1 | route_row | hook_count |
| resource_pressure_budget | verified_cpu_accept_eligible_events | 0 | cpu-route-feedback-loop-v1.report.json route_key resource_pressure_budget verified_cpu_accept_eligible_events=0 | route_row | verified_accept_count |
| market_claim | stays blocked | market_claim_allowed_false | cpu-route-feedback-loop-v1.report.json verified_gap_to_80_calls=768; unique_verified_gap_to_80_calls=774; market_claim_allowed=false | claim_boundary | market_boundary |
