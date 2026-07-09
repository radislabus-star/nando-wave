# NANDA Task: phase-stream-serving-append-shadow-replay

## query

Check that the .nwpc append-window shadow replay treats the watermark trace as
history, scores only events newer than the watermark, and refuses to count
savings when no new append events exist.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| append_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_append_shadow_replay_v1 |
| append_shadow_replay_command | treats | watermark_trace_as_history | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_trace_path |
| append_shadow_replay_command | filters | events_newer_than_watermark_timestamp | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_max_timestamp |
| append_shadow_replay_report | found | no_new_append_events_after_watermark | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#rejection_reason |
| append_shadow_replay_report | keeps_zero | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#unique_cpu_accepts_over_exact_cache |
| append_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#local_accept_enabled |
| append_shadow_replay_report | keeps_disabled | market_savings_count | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#market_savings_count_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| append_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_append_shadow_replay_v1 |
| append_shadow_replay_command | treats | watermark_trace_as_history | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_trace_path |
| append_shadow_replay_command | filters | events_newer_than_watermark_timestamp | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_max_timestamp |
| append_shadow_replay_report | found | no_new_append_events_after_watermark | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#rejection_reason |
| append_shadow_replay_report | keeps_zero | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#unique_cpu_accepts_over_exact_cache |
| append_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#local_accept_enabled |
| append_shadow_replay_report | keeps_disabled | market_savings_count | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json#market_savings_count_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
