# NANDA Task: Phase Stream Refined Online Discovery

## Query

Check that `phase-stream-real-traffic-refined-online-discovery-v1` is a
shadow-only request-shape refinement over the allowed phase-center path. It may
increase verifier-bound unique CPU accepts on real enriched traces, but it must
not enable product local accept, claim market savings, revive `.nwrb`, or hide
false accepts through bucket splitting.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| refined command | entrypoint | phase-stream-real-traffic-refined-online-discovery-v1 | crates/nando-cli/src/main.rs:486 |
| refined command | implementation | run_phase_stream_real_traffic_refined_online_discovery_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:1810 |
| refined command | bucket mode | request_shape_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:1811 |
| request shape key | built from | profile_id route_key slot_summary | crates/nando-cli/src/phase_streaming_cmd.rs:4017 |
| refined report | path | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json | report artifact |
| refined report | margin_threshold_micro | 100000 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:margin_threshold_micro |
| refined report | accepted_bucket_count | 2 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:accepted_bucket_count |
| refined report | stream_false_accepts | 0 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:stream_false_accepts |
| refined report | unique CPU accepts over exact cache | 4 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| refined report | token savings | 2010 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:total_nando_cpu_tokens_saved |
| refined report | cost savings microusd | 6030 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:total_nando_cpu_cost_saved_microusd |
| refined report | token evidence missing events | 0 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:token_evidence_missing_events |
| refined report | cost evidence missing events | 0 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:cost_evidence_missing_events |
| refined report | local_accept_enabled | false | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:local_accept_enabled |
| refined report | market_money_claim_allowed | false | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:market_money_claim_allowed |
| refined report | forbidden flags | all false | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:forbidden_flags |
| accepted agent bucket | unique CPU accepts over exact cache | 2 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:buckets agent_continue_execute |
| accepted agent bucket | false_accepts | 0 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:buckets agent_continue_execute |
| accepted metrics bucket | unique CPU accepts over exact cache | 2 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:buckets metrics_report_readout |
| accepted metrics bucket | false_accepts | 0 | target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json:buckets metrics_report_readout |
| low threshold 50000 | rejected because | stream_false_accepts 1 | target/nando-wave/streaming/refined-slot-threshold-50000.report.json:stream_false_accepts |
| safe threshold 100000 | selected because | false accepts zero and unique accepts four | target/nando-wave/streaming/refined-slot-threshold-100000.report.json |
| legacy guard | blocks | role-binding command prefix | crates/nando-cli/src/main.rs:506 |
| legacy skip helper | rejects | role_binding/nwrb profile names | crates/nando-cli/src/phase_streaming_cmd.rs:4055 |
| executor notes | records | refined phase-center discovery boundary | docs/EXECUTOR_REVIEW_NOTES.md:1 |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| refined command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| refined command | promotes | serving runtime | negative-contract:serving_runtime_unchanged |
| refined command | revives | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| refined command | uses | target_id authority | negative-contract:target_id_used_false |
| refined command | uses | proof_rule_id authority | negative-contract:proof_rule_id_authority_used_false |
| refined command | uses | concrete_x_lookup | negative-contract:concrete_x_lookup_used_false |
| refined command | uses | manual local_out_t | negative-contract:manual_local_out_t_used_false |
| low threshold 50000 | selected as | safe current threshold | negative-contract:threshold_50000_has_false_accepts_1 |
| refined report | claims | market money proof | negative-contract:market_money_claim_allowed_false |
| refined report | claims | full goal complete | negative-contract:4_unique_accepts_not_500_of_5000 |
