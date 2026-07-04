# Test Output Parse Tool-Output State V1 Structural Gate

## query

Check that `test_output_parse` tool-output state capture links real candidate
requests to previous agent-loop command-output state, without using final
answers, target/proof labels, local accepts, or market savings claims.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| tool_output_state | reads | codex_history_jsonl | history_source |
| tool_output_state | reads | broad_split_report | split_source |
| tool_output_state | reads | codex_sessions | sessions_root |
| tool_output_state | writes | state_trace | trace_output |
| tool_output_state | writes | state_report | report_output |
| tool_output_state | selects | test_output_parse_split | split_key |
| tool_output_state | associates | request_fingerprint | codex_user_message_fingerprint |
| tool_output_state | associates | previous_tool_output_fingerprint | request_time_previous_state |
| tool_output_state | classifies | command_signal | deterministic_command_signal_classifier |
| tool_output_state | classifies | command_status | deterministic_pass_fail_unknown_classifier |
| tool_output_state | excludes | future_tool_output | request_time_state_only |
| tool_output_state | excludes | final_answer_text | response_text_used_false |
| tool_output_state | excludes | raw_prompt_text | raw_prompt_text_written_false |
| tool_output_state | excludes | raw_tool_output_text | raw_tool_output_text_written_false |
| tool_output_state | excludes | raw_response_text | raw_response_text_written_false |
| tool_output_state | excludes | target_or_proof_labels | label_flags_false |
| tool_output_state | keeps | local_accepts_disabled | local_accepts_flag |
| tool_output_state | keeps | market_claim_disabled | market_claim_flag |
| state_report | verdict | tool_state_attached | verdict_field |
| state_report | candidate_events | 104 | report_count_candidate |
| state_report | non_exact_candidate_events | 102 | report_count_non_exact |
| state_report | exact_cache_overlap_events | 2 | report_count_overlap |
| state_report | session_files_scanned | 9 | report_count_sessions |
| state_report | codex_turns_indexed | 104 | report_count_turns |
| state_report | tool_outputs_indexed | 145578 | report_count_tool_outputs |
| state_report | tool_output_state_matched_events | 104 | report_count_matched |
| state_report | command_status_detected_events | 97 | report_count_status_detected |
| state_report | pass_status_events | 90 | report_count_pass |
| state_report | fail_status_events | 7 | report_count_fail |
| state_report | unknown_status_events | 7 | report_count_unknown |
| cpu80_claim | rejects | tool_state_matches_as_savings | business_value_gate |
| cpu80_claim | requires | payload_profile_admission_shadow | claim_boundary |
| next_debt | requires | scoreable_payloads_from_previous_tool_state | next_engineering_debt |
