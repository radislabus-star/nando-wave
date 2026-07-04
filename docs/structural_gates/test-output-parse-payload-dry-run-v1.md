# Test Output Parse Payload Dry Run V1 Structural Gate

## query

Check one route: `test_output_parse` advanced from broad-route split discovery
to request-side scoreable payloads, while local accepts and market claims stayed
disabled until verifier evidence exists.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| dry_run | reads | codex_history_jsonl | history_source |
| dry_run | reads | broad_split_report | split_source |
| dry_run | reads | profile_registry | registry_source |
| dry_run | writes | dry_run_trace | trace_output |
| dry_run | writes | dry_run_report | report_output |
| dry_run | selects | test_output_parse_split | split_key |
| dry_run | emits_route_key | test_output_parse | route_key |
| dry_run | emits_profile_id | route_gap_test_output_parse_profile_v1 | profile_id |
| dry_run | encodes_roles | command_status_artifact_boundary | request_shape |
| dry_run | excludes | raw_prompt_text | raw_text_written_false |
| dry_run | excludes | response_text | response_text_used_false |
| dry_run | excludes | target_or_proof_labels | label_flags_false |
| dry_run | keeps | local_accepts_disabled | local_accepts_flag |
| dry_run | keeps | market_claim_disabled | market_claim_flag |
| dry_run_report | verdict | scoreable_payloads_profile_missing | verdict_field |
| dry_run_report | candidate_events | 104 | report_count_candidate |
| dry_run_report | non_exact_candidate_events | 102 | report_count_non_exact |
| dry_run_report | exact_cache_overlap_events | 2 | report_count_overlap |
| dry_run_report | payload_ready_events | 3 | report_count_ready |
| dry_run_report | scoreable_payload_events | 3 | report_count_scoreable |
| dry_run_report | profile_registered | false | report_profile_flag |
| dry_run_report | expected_unique_cpu_accepts | 0 | report_expected_unique |
| dry_run_report | expected_savings_milli | 0 | report_expected_savings |
| dry_run_report | false_accepts | 0 | report_false_accepts |
| broad_routes | remain_blocked | answer_or_explain_and_project_context_dialogue | split_policy |
| parent_route_counts | aggregate | answer_or_explain_57_project_context_47 | parent_counts |
| next_debt | requires | tool_output_verifier_then_disabled_profile | next_engineering_debt |
| cpu80_claim | rejects | scoreable_payloads_as_savings | business_value_gate |
| cpu80_claim | requires | verifier_evidence_before_accept | no_auto_accept_rule |

