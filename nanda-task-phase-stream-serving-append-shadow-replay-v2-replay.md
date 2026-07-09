# NANDA Task: phase-stream-serving-append-shadow-replay-v2-replay

## query

Check that the .nwpc append-window shadow replay v2 uses the previous trace as
exact-cache watermark history, scores newer append rows through the phase-center
runtime registry, reports unique CPU accepts over exact-cache, and keeps
product accept plus money claims disabled.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| append_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_append_shadow_replay_v1 |
| append_shadow_replay_command | treats | watermark_trace_as_exact_cache_history | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_trace_path |
| append_shadow_replay_command | filters | events_newer_than_watermark_timestamp | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_max_timestamp |
| append_shadow_replay_report | scores | routed_append_events | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#routed_events |
| append_shadow_replay_report | counts | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#unique_cpu_accepts_over_exact_cache |
| append_shadow_replay_report | keeps_zero | false_accepts | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#false_accepts |
| append_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#local_accept_enabled |
| append_shadow_replay_report | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#market_money_claim_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| append_shadow_replay_command | loads | admitted_nwpc_runtime_registry | crates/nando-cli/src/phase_streaming_cmd.rs#run_phase_stream_phase_atom_serving_append_shadow_replay_v1 |
| append_shadow_replay_command | treats | watermark_trace_as_exact_cache_history | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_trace_path |
| append_shadow_replay_command | filters | events_newer_than_watermark_timestamp | crates/nando-cli/src/phase_streaming_cmd.rs#append_watermark_max_timestamp |
| append_shadow_replay_report | scores | routed_append_events | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#routed_events |
| append_shadow_replay_report | counts | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#unique_cpu_accepts_over_exact_cache |
| append_shadow_replay_report | keeps_zero | false_accepts | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#false_accepts |
| append_shadow_replay_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#local_accept_enabled |
| append_shadow_replay_report | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v2.report.json#market_money_claim_allowed |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
