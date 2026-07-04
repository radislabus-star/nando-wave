# Manual Route Discovery 5k Taxonomy V1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| codex_history_route_candidates_5k | samples | 5000_real_codex_calls | codex-history-route-candidates-v1-5k.report.json:events_written=5000 |
| codex_history_route_candidates_5k | finds | 1457_existing_route_candidates | codex-history-route-candidates-v1-5k.report.json:candidate_events=1457 |
| codex_history_route_candidates_5k | finds | 3543_no_candidate_events | codex-history-route-candidates-v1-5k.report.json:no_candidate_events=3543 |
| route_gap_payload_readiness_5k | finds | 803_payload_ready_events | route-gap-payload-readiness-v1-5k.report.json:payload_ready_events=803 |
| manual_route_discovery_5k | reads | route_gap_payload_readiness_5k | target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-5k.report.json |
| manual_route_discovery_5k | writes | manual_route_discovery_5k_report | target/nando-wave/real-traffic-shadow/manual-route-discovery-v1-5k.report.json |
| manual_route_discovery_5k | keeps | local_accepts_disabled | manual-route-discovery-v1-5k.report.json:local_accepts_enabled=false |
| manual_route_discovery_5k | keeps | market_claim_disallowed | manual-route-discovery-v1-5k.report.json:market_claim_allowed=false |
| manual_route_discovery_5k | does_not_write | raw_prompt_text | manual-route-discovery-v1-5k.report.json:raw_text_written=false |
| manual_route_discovery_5k | finds | 116_uncatalogued_events | manual-route-discovery-v1-5k.report.json:uncatalogued_events=116 |
| manual_route_discovery_5k | finds | 27_payload_ready_uncatalogued_events | manual-route-discovery-v1-5k.report.json:payload_ready_events=27 |
| manual_review_required | reduced_to | 39_events_2_payload_ready | manual-route-discovery-v1-5k.report.json:subfamilies[manual_review_required] |
| top_manual_subfamily | is | ime_input_state_debug | manual-route-discovery-v1-5k.report.json:top_subfamily=ime_input_state_debug |
| ime_input_state_debug | candidate_events | 16 | manual-route-discovery-v1-5k.report.json:subfamilies[ime_input_state_debug].candidate_events=16 |
| ime_input_state_debug | payload_ready_events | 6 | manual-route-discovery-v1-5k.report.json:subfamilies[ime_input_state_debug].payload_ready_events=6 |
| document_stamp_layout_edit | payload_ready_events | 4 | manual-route-discovery-v1-5k.report.json:subfamilies[document_stamp_layout_edit].payload_ready_events=4 |
| product_price_certification_table | payload_ready_events | 3 | manual-route-discovery-v1-5k.report.json:subfamilies[product_price_certification_table].payload_ready_events=3 |
| business_party_identity_address | payload_ready_events | 3 | manual-route-discovery-v1-5k.report.json:subfamilies[business_party_identity_address].payload_ready_events=3 |
| route_visibility_result | adds | zero_verified_cpu_accepts | docs/EXECUTOR_REVIEW_NOTES.md:Manual Route Discovery 5k Taxonomy Split |
| cpu80_state | remains | 26_verified_accepts_774_gap | docs/EXECUTOR_REVIEW_NOTES.md:Manual Route Discovery 5k Taxonomy Split |
