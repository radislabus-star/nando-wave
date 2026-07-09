# NANDA Task: Phase Stream Discovery Registry

## Query

Check that `phase-stream-discovery-v1` is an offline phase-center discovery
registry only. It may compile quarantine `.nwpc` candidates, but it must not
enable local accept, serving promotion, market claims, or the old `.nwrb`
role-binding backend.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| discovery command | entrypoint | phase-stream-discovery-v1 | crates/nando-cli/src/main.rs:phase-stream-discovery-v1 |
| discovery command | implementation | run_phase_stream_discovery_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:run_phase_stream_discovery_v1 |
| discovery command | trace input count | two trace JSONL files | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:trace_paths |
| discovery command | bucket route | proof_scope plus parse_test_output action | crates/nando-cli/src/phase_streaming_cmd.rs:discovery_bucket_key |
| discovery compiler | package type | phase-center `.nwpc` | crates/nando-cli/src/phase_streaming_cmd.rs:build_discovery_candidate |
| discovery report | mode | offline_shadow_discovery_only | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:mode |
| discovery report | parsed_events | 110 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:parsed_events |
| discovery report | bucket_count | 2 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:bucket_count |
| discovery report | candidate_count | 2 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidate_count |
| discovery report | accepted_candidate_count | 2 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:accepted_candidate_count |
| discovery report | unique_cpu_accepts_over_exact_cache | 25 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| discovery report | local_accept_enabled | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:local_accept_enabled |
| discovery report | product_runtime_changed | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:product_runtime_changed |
| discovery report | serving_runtime_changed | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:serving_runtime_changed |
| discovery report | market_money_claim_allowed | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:market_money_claim_allowed |
| raw-output candidate | proof_scope | raw_output_parse | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[0].proof_scope |
| raw-output candidate | false_accepts | 0 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[0].false_accepts |
| raw-output candidate | parity_mismatches | 0 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[0].runtime_margin_parity_mismatches |
| metadata candidate | proof_scope | tool_output_state_metadata_parse | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[1].proof_scope |
| metadata candidate | false_accepts | 0 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[1].false_accepts |
| metadata candidate | parity_mismatches | 0 | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:candidates[1].runtime_margin_parity_mismatches |
| forbidden flags | target_id_used | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:forbidden_flags.target_id_used |
| forbidden flags | proof_rule_id_authority_used | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:forbidden_flags.proof_rule_id_authority_used |
| forbidden flags | concrete_x_lookup_used | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:forbidden_flags.concrete_x_lookup_used |
| forbidden flags | manual_local_out_t_used | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:forbidden_flags.manual_local_out_t_used |
| forbidden flags | legacy_backend_used | false | target/nando-wave/streaming/online-phase-center-discovery-v1.report.json:forbidden_flags.legacy_backend_used |
| legacy guard | blocks | role-binding/nwrb command prefix | crates/nando-cli/src/main.rs:FORBIDDEN_LEGACY_NWRB_BACKEND |
| phase center core | runtime file | phase_center_runtime.rs | crates/nando-core/src/wave/phase_center_runtime.rs |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| discovery command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| discovery command | promotes | serving profile | negative-contract:serving_promotion_forbidden |
| discovery command | allows | market money claim | negative-contract:market_claim_forbidden |
| discovery command | uses | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| discovery command | uses | target/proof label authority | negative-contract:label_authority_forbidden |
