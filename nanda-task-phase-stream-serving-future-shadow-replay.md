# NANDA Task: phase-stream-serving-future-shadow-replay

## query

Check that the .nwpc future-only shadow-serving replay excludes the admission
train window and scores only future heldout rows, while keeping product
local_accept disabled and legacy .nwrb forbidden.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_admission_report | allowed | tool_status_shadow_profile_candidate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| future_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_future_shadow_replay_v1 |
| future_shadow_replay_command | reconstructs | admission_time_split | crates/nando-cli/src/phase_streaming_cmd.rs#phase_atom_binary_time_split_indices |
| future_shadow_replay_report | excludes | training_overlap_events | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#excluded_training_overlap_events |
| future_shadow_replay_report | scores | future_heldout_events | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#routed_events |
| future_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#local_accept_enabled |
| future_shadow_replay_report | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#market_money_claim_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| serving_admission_report | allowed | tool_status_shadow_profile_candidate | target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json#serving_admission_candidate_allowed |
| future_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_future_shadow_replay_v1 |
| future_shadow_replay_command | reconstructs | admission_time_split | crates/nando-cli/src/phase_streaming_cmd.rs#phase_atom_binary_time_split_indices |
| future_shadow_replay_report | excludes | training_overlap_events | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#excluded_training_overlap_events |
| future_shadow_replay_report | scores | future_heldout_events | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#routed_events |
| future_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#local_accept_enabled |
| future_shadow_replay_report | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json#market_money_claim_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
