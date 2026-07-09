# NANDA Task: Phase Stream Token/Cost Enrichment

## Query

Check that `phase-stream-real-traffic-token-cost-enrich-v1` only enriches
real-traffic trace rows with readiness-report token/cost estimates, then the
phase-center online discovery uses that evidence for shadow-only calls/tokens/
money accounting. It must not promote rejected buckets, enable local accept,
revive `.nwrb`, or claim market proof while false accepts remain in rejected
buckets and total savings are tiny.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| enrichment command | entrypoint | phase-stream-real-traffic-token-cost-enrich-v1 | crates/nando-cli/src/main.rs:489 |
| enrichment command | implementation | run_phase_stream_real_traffic_token_cost_enrich_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:2317 |
| enrichment command | source evidence | route-gap-payload-readiness-v1-current5k.report.json | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:readiness_report_path |
| enrichment command | join key | request_fingerprint | crates/nando-cli/src/phase_streaming_cmd.rs:3732 |
| enrichment command | copies | estimated_total_tokens | crates/nando-cli/src/phase_streaming_cmd.rs:3834 |
| enrichment command | copies | estimated_total_cost_microusd | crates/nando-cli/src/phase_streaming_cmd.rs:3848 |
| enrichment command | writes marker | token_cost_evidence_source | crates/nando-cli/src/phase_streaming_cmd.rs:3860 |
| enrichment report | mode | trace_enrichment_only | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:mode |
| enrichment report | rows_with_shadow_request | 405 | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:rows_with_shadow_request |
| enrichment report | matched_rows | 11967 | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:matched_rows |
| enrichment report | rows_enriched_tokens | 11967 | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:rows_enriched_tokens |
| enrichment report | rows_enriched_cost | 11967 | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:rows_enriched_cost |
| enrichment report | local_accept_enabled | false | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:local_accept_enabled |
| enrichment report | market_money_claim_allowed | false | target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json:market_money_claim_allowed |
| enriched audit | nonlegacy_candidate_rows | 405 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-enriched-v1.report.json:nonlegacy_candidate_rows |
| enriched audit | verifier_bound_token_or_cost_events | 363 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-enriched-v1.report.json:verifier_bound_token_or_cost_events |
| enriched audit | compile_ready_bucket_count | 5 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-enriched-v1.report.json:compile_ready_bucket_count |
| enriched audit | money_proof_candidate_bucket_count | 4 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-enriched-v1.report.json:money_proof_candidate_bucket_count |
| enriched discovery | accepted_bucket_count | 1 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:accepted_bucket_count |
| enriched discovery | stream_false_accepts | 6 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:stream_false_accepts |
| enriched discovery | unique CPU accepts over exact cache | 2 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| enriched discovery | token savings | 16 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:total_nando_cpu_tokens_saved |
| enriched discovery | cost savings microusd | 48 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:total_nando_cpu_cost_saved_microusd |
| enriched discovery | token evidence missing events | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:token_evidence_missing_events |
| enriched discovery | cost evidence missing events | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:cost_evidence_missing_events |
| enriched discovery | local_accept_enabled | false | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:local_accept_enabled |
| enriched discovery | market_money_claim_allowed | false | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:market_money_claim_allowed |
| accepted bucket | route | agent_continue_execute | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:buckets[0].route_key |
| accepted bucket | false_accepts | 0 | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:buckets[0].false_accepts |
| rejected metrics bucket | blocker | false_accepts_detected | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:buckets[2].rejection_reason |
| rejected serving bucket | blocker | false_accepts_detected | target/nando-wave/streaming/real-traffic-phase-center-online-discovery-enriched-v1.report.json:buckets[4].rejection_reason |
| legacy guard | blocks | role-binding command prefix | crates/nando-cli/src/main.rs:501 |
| legacy skip helper | rejects | role_binding/nwrb profile names | crates/nando-cli/src/phase_streaming_cmd.rs:4005 |
| executor notes | records | token/cost enrichment and remaining false-accept blockers | docs/EXECUTOR_REVIEW_NOTES.md:1 |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| enrichment command | compiles | `.nwpc` package | negative-contract:enrichment_only_no_compile |
| enrichment command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| enrichment command | revives | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| enriched discovery | promotes | metrics_report_readout bucket | negative-contract:metrics_report_false_accepts_detected |
| enriched discovery | promotes | serving_ops bucket | negative-contract:serving_ops_false_accepts_detected |
| enriched discovery | claims | market money proof | negative-contract:market_money_claim_allowed_false |
| enriched discovery | claims | goal complete | negative-contract:only_2_unique_accepts_not_500_of_5000 |
