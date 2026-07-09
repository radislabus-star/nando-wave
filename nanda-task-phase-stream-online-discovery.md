# NANDA Task: Phase Stream Online Discovery

## Query

Check that `phase-stream-online-discovery-v1` is an online-order shadow
discovery rung. It may compile quarantine `.nwpc` packages after enough
verifier-bound examples and score later events, but it must not enable product
local accept, serving promotion, market claims, or the old `.nwrb` backend.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| online discovery command | entrypoint | phase-stream-online-discovery-v1 | crates/nando-cli/src/main.rs:phase-stream-online-discovery-v1 |
| online discovery command | implementation | run_phase_stream_online_discovery_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:run_phase_stream_online_discovery_v1 |
| online discovery command | trace processing | stream order | crates/nando-cli/src/phase_streaming_cmd.rs:run_phase_stream_online_discovery_v1 |
| online discovery command | compile condition | min_bucket_events and at least two labels | crates/nando-cli/src/phase_streaming_cmd.rs:labels_for_indices |
| online discovery command | package type | phase-center `.nwpc` | crates/nando-cli/src/phase_streaming_cmd.rs:compile_online_bucket |
| online discovery command | scoring target | future events after compile | crates/nando-cli/src/phase_streaming_cmd.rs:score_online_shadow_event |
| online report | mode | online_shadow_discovery_only | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:mode |
| online report | parsed_events | 110 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:parsed_events |
| online report | compiled_bucket_count | 2 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:compiled_bucket_count |
| online report | accepted_bucket_count | 1 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:accepted_bucket_count |
| online report | stream_shadow_events | 79 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:stream_shadow_events |
| online report | stream_shadow_accepts | 69 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:stream_shadow_accepts |
| online report | stream_false_accepts | 0 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:stream_false_accepts |
| online report | unique_cpu_accepts_over_exact_cache | 68 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| online report | token_savings | 6942 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:total_nando_cpu_tokens_saved |
| online report | cost_savings_microusd | 7214 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:total_nando_cpu_cost_saved_microusd |
| online report | local_accept_enabled | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:local_accept_enabled |
| online report | product_runtime_changed | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:product_runtime_changed |
| online report | serving_runtime_changed | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:serving_runtime_changed |
| online report | market_money_claim_allowed | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:market_money_claim_allowed |
| accepted bucket | proof_scope | tool_output_state_metadata_parse | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[1].proof_scope |
| accepted bucket | unique_cpu_accepts_over_exact_cache | 68 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[1].unique_cpu_accepts_over_exact_cache |
| accepted bucket | false_accepts | 0 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[1].false_accepts |
| accepted bucket | parity_mismatches | 0 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[1].runtime_margin_parity_mismatches |
| rejected bucket | proof_scope | raw_output_parse | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[0].proof_scope |
| rejected bucket | reason | no_unique_accepts_over_exact_cache | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[0].rejection_reason |
| rejected bucket | false_accepts | 0 | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:buckets[0].false_accepts |
| forbidden flags | target_id_used | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:forbidden_flags.target_id_used |
| forbidden flags | proof_rule_id_authority_used | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:forbidden_flags.proof_rule_id_authority_used |
| forbidden flags | concrete_x_lookup_used | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:forbidden_flags.concrete_x_lookup_used |
| forbidden flags | manual_local_out_t_used | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:forbidden_flags.manual_local_out_t_used |
| forbidden flags | legacy_backend_used | false | target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json:forbidden_flags.legacy_backend_used |
| legacy guard | blocks | role-binding/nwrb command prefix | crates/nando-cli/src/main.rs:FORBIDDEN_LEGACY_NWRB_BACKEND |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| online discovery command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| online discovery command | promotes | serving profile | negative-contract:serving_promotion_forbidden |
| online discovery command | allows | market money claim | negative-contract:market_claim_forbidden |
| online discovery command | uses | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| online discovery command | uses | target/proof label authority | negative-contract:label_authority_forbidden |
