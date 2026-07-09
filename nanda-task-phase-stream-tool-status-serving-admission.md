# NANDA Task: phase-stream-tool-status-serving-admission

## query

Check the route boundary for the new tool_status .nwpc serving-admission audit:
it is a runtime replay candidate gate, not product local_accept and not legacy
.nwrb/role-binding revival.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| tool_status_discovery | produced | quarantine_nwpc_candidate | target/nando-wave/streaming/phase-atom-tool-status-time-split-discovery-v1.report.json#package_path |
| tool_status_promotion_audit | accepted | quarantine_promotion_candidate | target/nando-wave/streaming/phase-atom-tool-status-time-split-promotion-audit-v1.report.json#promotion_candidate_allowed |
| serving_admission_command | loads | quarantine_nwpc_with_PhaseCenterOffloadRuntime | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_action_family_serving_admission_audit_v1 |
| serving_admission_command | replays | verifier_bound_heldout_window | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_action_family_serving_admission_audit_v1 |
| serving_admission_report | treated_as | runtime_replay_candidate_gate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| serving_admission_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#local_accept_enabled |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_admission_report | treated_as | runtime_replay_candidate_gate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| serving_admission_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#local_accept_enabled |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
