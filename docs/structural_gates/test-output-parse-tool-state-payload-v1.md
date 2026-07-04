# Test Output Parse Tool-State Payload V1 Structural Gate

## query

Check that `test_output_parse` tool-state payload generation turns previous
tool-output state into scoreable Nando shadow requests, while keeping local
accepts, market claims, final-answer evidence, target labels, and proof labels
disabled.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| tool_state_payload | reads | tool_output_state_report | report_source |
| tool_state_payload | reads | profile_registry | registry_source |
| tool_state_payload | writes | payload_trace | trace_output |
| tool_state_payload | writes | payload_report | report_output |
| tool_state_payload | selects | test_output_parse_rows | split_key |
| tool_state_payload | uses | previous_tool_output_fingerprint | request_time_state |
| tool_state_payload | uses | command_signal | command_signal_feature |
| tool_state_payload | uses | command_status | command_status_feature |
| tool_state_payload | builds | nando_shadow_request | score_request |
| tool_state_payload | emits_route_key | test_output_parse | route_key |
| tool_state_payload | emits_profile_id | route_gap_test_output_parse_profile_v1 | profile_id |
| tool_state_payload | encodes_roles | command_status_artifact_boundary | role_slots |
| tool_state_payload | excludes | raw_prompt_tool_response_text | raw_text_flags_false |
| tool_state_payload | excludes | final_answer_target_proof_labels | no_leak_flags_false |
| tool_state_payload | keeps | local_accepts_disabled | local_accepts_flag |
| tool_state_payload | keeps | market_claim_disabled | market_claim_flag |
| payload_report | verdict | scoreable_payloads_profile_missing | verdict_field |
| payload_report | operator_candidate_calls | 104 | report_count_candidate |
| payload_report | tool_output_state_matched_events | 104 | report_count_state_matched |
| payload_report | command_status_detected_events | 97 | report_count_status_detected |
| payload_report | payload_ready_events | 97 | report_count_ready |
| payload_report | payload_built_events | 97 | report_count_built |
| payload_report | scoreable_payload_events | 97 | report_count_scoreable |
| payload_report | builder_rejected_events | 7 | report_count_rejected |
| payload_report | profile_and_shadow_ready | false | report_profile_shadow_flag |
| payload_report | expected_unique_and_savings | 0 | report_no_savings_claim |
| payload_report | false_accepts | 0 | report_false_accepts_disabled |
| cpu80_claim | rejects | scoreable_payload_count_as_savings | business_value_gate |
| cpu80_claim | requires | disabled_profile_shadow_admission | claim_boundary |
| next_debt | requires | test_output_parse_profile_plus_admission_audit | next_engineering_debt |
