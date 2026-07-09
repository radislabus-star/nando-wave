# NANDA Task: phase-stream-serving-shadow-replay

## query

Check that the .nwpc shadow-serving replay is a runtime-registry dry run only:
it loads admitted phase-center profiles and shadow-scores trace rows, but does
not compile, promote, enable product local_accept, or revive legacy .nwrb.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_admission_report | allowed | tool_status_shadow_profile_candidate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_shadow_replay_v1 |
| shadow_replay_command | routes | phase_atom_trace_rows | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_shadow_replay_v1 |
| shadow_replay_report | treated_as | full_trace_dry_run_capacity | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#full_trace_replay |
| shadow_replay_report | keeps_disabled | market_savings_count | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#market_savings_count_allowed |
| shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#local_accept_enabled |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_admission_report | allowed | tool_status_shadow_profile_candidate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_shadow_replay_v1 |
| shadow_replay_command | routes | phase_atom_trace_rows | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_shadow_replay_v1 |
| shadow_replay_report | treated_as | full_trace_dry_run_capacity | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#full_trace_replay |
| shadow_replay_report | keeps_disabled | market_savings_count | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#market_savings_count_allowed |
| shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json#local_accept_enabled |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
