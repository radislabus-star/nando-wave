# Broad Route Split Discovery V1 Structural Gate

## query

Check that broad-route split discovery is a review-only business-value filter,
not a CPU savings claim or local-accept promotion.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| broad_route_split_discovery_v1 | reads | non_synthetic_codex_history_5k | target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json |
| broad_route_split_discovery_v1 | writes | fingerprints_features_counts_only | raw_text_written=false |
| broad_route_split_discovery_v1 | does_not_use | response_text | response_text_used=false |
| broad_route_split_discovery_v1 | does_not_use | target_labels | target_labels_used=false |
| broad_route_split_discovery_v1 | does_not_use | proof_labels | proof_labels_used=false |
| broad_route_split_discovery_v1 | keeps_disabled | local_accepts | local_accepts_enabled=false |
| broad_route_split_discovery_v1 | forbids | market_claim | market_claim_allowed=false |
| broad_route_split_discovery_v1 | measured_broad_candidate_events | 3330 | report measured result |
| broad_route_split_discovery_v1 | measured_non_exact_broad_candidate_events | 3089 | report measured result |
| broad_route_split_discovery_v1 | measured_business_value_gate_passed_rows | 0 | report measured result |
| answer_or_explain | split_into | file_path_evidence_answer | candidates=79 non_exact=79 payload=70 verifier=70 |
| project_context_dialogue | split_into | file_path_evidence_answer | candidates=65 non_exact=63 payload=51 verifier=52 |
| answer_or_explain | split_into | test_output_parse | candidates=57 non_exact=57 payload=3 verifier=35 |
| answer_or_explain | split_into | metric_from_report | candidates=27 non_exact=27 payload=14 verifier=14 |
| answer_or_explain | split_into | git_status_summary | candidates=10 non_exact=10 payload=5 verifier=10 |
| agent_continue_execute | split_into | git_status_summary | candidates=8 non_exact=8 payload=4 verifier=8 |
| project_context_dialogue | remains_blocked_as_whole | broad_reasoning_requires_llm | non_exact=1216 risk=HIGH_BROAD_UNSPLIT |
| answer_or_explain | remains_blocked_as_whole | broad_reasoning_requires_llm | non_exact=1029 risk=HIGH_BROAD_UNSPLIT |
| agent_continue_execute | remains_watch | artifact_progress | high stateful singleton risk |
| candidate_split | has_expected_unique_cpu_accepts | zero_until_shadow_audit | expected_unique_cpu_accepts_over_exact_cache=0 |
| cpu80_claim | requires | verified_accepts_false_accepts_zero | project rule |
| cpu80_claim | rejects | discovery_only_candidate_splits | discovery has no shadow audit |
| local_accept_promotion | requires | deterministic_verifier_and_shadow_audit | project rule |
