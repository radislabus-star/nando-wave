# NANDA Task: Phase Stream State/Action Online Discovery

## Query

Check that `phase-stream-real-traffic-state-action-online-discovery-v1` is an
allowed shadow-only phase-center discovery mode. It may use coarse request-side
state/action buckets and report a deduped safe frontier with request-shape
discovery, but it must not enable local accept, claim market proof, revive
`.nwrb`, or count duplicate requests twice.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| state_action command | entrypoint | phase-stream-real-traffic-state-action-online-discovery-v1 | crates/nando-cli/src/main.rs:492 |
| state_action command | implementation | run_phase_stream_real_traffic_state_action_online_discovery_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:1815 |
| state_action command | bucket mode | state_action_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:1819 |
| state_action signature | source fields | tool band active band slot count impulse bands | crates/nando-cli/src/phase_streaming_cmd.rs:4047 |
| state_action report | path | target/nando-wave/streaming/real-traffic-phase-center-state-action-online-discovery-v1.report.json | report artifact |
| state_action report | cells | 64 | state_action report aggregate |
| state_action report | margin_threshold_micro | 150000 | state_action report aggregate |
| state_action report | accepted_bucket_count | 2 | state_action report aggregate |
| state_action report | stream_false_accepts | 0 | state_action report aggregate |
| state_action report | unique CPU accepts over exact cache | 8 | state_action report aggregate |
| state_action report | token savings | 90 | state_action report aggregate |
| state_action report | cost savings microusd | 270 | state_action report aggregate |
| state_action report | local_accept_enabled | false | state_action report boundary |
| state_action report | market_money_claim_allowed | false | state_action report boundary |
| state_action report | forbidden flags | all false | state_action report forbidden_flags |
| unsafe threshold report | rejected because | threshold100000 false_accepts 1 | target/nando-wave/streaming/state-action-c64-threshold-100000.report.json |
| safe threshold report | selected because | threshold150000 false_accepts zero and accepts eight | target/nando-wave/streaming/state-action-c64-threshold-150000.report.json |
| request_shape report | unique CPU accepts over exact cache | 4 | request_shape current report aggregate |
| request_shape report | token savings | 2010 | request_shape current report aggregate |
| combined frontier | dedupe key | request_fingerprint | unique_accepts arrays |
| combined frontier | unique CPU accepts over exact cache | 12 | jq union over unique_accepts |
| combined frontier | token savings | 2100 | jq union over unique_accepts |
| combined frontier | cost savings microusd | 6300 | jq union over unique_accepts |
| legacy guard | blocks | role-binding command prefix | crates/nando-cli/src/main.rs:510 |
| legacy skip helper | rejects | role_binding/nwrb profile names | crates/nando-cli/src/phase_streaming_cmd.rs:4090 |
| executor notes | records | state/action phase-center boundary | docs/EXECUTOR_REVIEW_NOTES.md:1 |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| state_action command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| state_action command | promotes | serving runtime | negative-contract:serving_runtime_unchanged |
| state_action command | revives | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| state_action command | uses | target_id authority | negative-contract:target_id_used_false |
| state_action command | uses | proof_rule_id authority | negative-contract:proof_rule_id_authority_used_false |
| state_action command | uses | concrete_x_lookup | negative-contract:concrete_x_lookup_used_false |
| state_action command | uses | manual local_out_t | negative-contract:manual_local_out_t_used_false |
| unsafe threshold report | selected as | safe current threshold | negative-contract:threshold100000_has_false_accepts_1 |
| combined frontier | double counts | duplicate request_fingerprint | negative-contract:union_uses_request_fingerprint_dedupe |
| combined frontier | claims | full goal complete | negative-contract:12_unique_accepts_not_500_of_5000 |
| combined frontier | claims | market money proof | negative-contract:market_money_claim_allowed_false |
