# NANDA Task: Generic Real-Traffic Online Discovery

## Query

Check that `phase-stream-real-traffic-online-discovery-v1` is a generic
real-agent-loop online shadow discovery rung. It may compile quarantine `.nwpc`
packages for non-legacy route-gap profiles and count calls on accepted buckets,
but it must not promote rejected buckets, make token/money claims without
evidence, enable product local accept, or revive `.nwrb` / role-binding backend.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| generic command | entrypoint | phase-stream-real-traffic-online-discovery-v1 | crates/nando-cli/src/main.rs:phase-stream-real-traffic-online-discovery-v1 |
| generic command | implementation | run_phase_stream_real_traffic_online_discovery_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:run_phase_stream_real_traffic_online_discovery_v1 |
| generic parser | requires_shadow_request | nando_shadow_request | crates/nando-cli/src/phase_streaming_cmd.rs:parse_generic_real_traffic_event.nando_shadow_request |
| generic parser | requires_verifier_label | verified_safe_accept boolean | crates/nando-cli/src/phase_streaming_cmd.rs:parse_generic_real_traffic_event.verified_safe_accept |
| generic parser | skips | legacy role_binding/nwrb profile names | crates/nando-cli/src/phase_streaming_cmd.rs:is_legacy_profile_name |
| generic compiler | package type | phase-center `.nwpc` | crates/nando-cli/src/phase_streaming_cmd.rs:compile_generic_bucket |
| generic scorer | uses | future events after compile | crates/nando-cli/src/phase_streaming_cmd.rs:score_generic_shadow_event |
| generic report | mode | online_shadow_discovery_only | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:mode |
| generic report | parsed_candidate_events | 323 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:parsed_candidate_events |
| generic report | skipped_legacy_profile_events | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:skipped_legacy_profile_events |
| generic report | accepted_bucket_count | 1 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:accepted_bucket_count |
| generic report | total_unique_cpu_accepts_over_exact_cache | 2 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| generic report | token_savings | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:total_nando_cpu_tokens_saved |
| generic report | cost_savings_microusd | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:total_nando_cpu_cost_saved_microusd |
| generic report | local_accept_enabled | false | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:local_accept_enabled |
| generic report | market_money_claim_allowed | false | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:market_money_claim_allowed |
| accepted bucket | profile | route_gap_agent_continue_execute_profile_v1 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:buckets[0].profile_id |
| accepted bucket | false_accepts | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:buckets[0].false_accepts |
| accepted bucket | unique_cpu_accepts_over_exact_cache | 2 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:buckets[0].unique_cpu_accepts_over_exact_cache |
| accepted bucket | token_cost_evidence_missing_events | 2 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:buckets[0].token_cost_evidence_missing_events |
| rejected serving bucket | reason | false_accepts_detected | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:buckets[3].rejection_reason |
| forbidden flags | legacy_backend_used | false | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json:forbidden_flags.legacy_backend_used |
| legacy guard | blocks | role-binding/nwrb command prefix | crates/nando-cli/src/main.rs:FORBIDDEN_LEGACY_NWRB_BACKEND |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| generic command | promotes | rejected serving_ops bucket | negative-contract:false_accept_bucket_must_not_promote |
| generic command | claims | token/money savings | negative-contract:token_cost_evidence_missing |
| generic command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| generic command | uses | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| generic command | uses | target/proof label authority | negative-contract:label_authority_forbidden |
