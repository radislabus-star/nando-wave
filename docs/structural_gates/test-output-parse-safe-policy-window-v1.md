# Test Output Parse Safe Policy Window V1 Structural Gate

## query

Check the narrow route: `test_output_parse` safe-policy proof must be counted
against the current 1000-call feedback window only by request-fingerprint
matches. The route-specific 97/104 proof must not be counted as 97 current
CPU80 accepts.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| window_builder | reads | base_trace_1000 | base_window_trace_path |
| window_builder | reads | route_trace_104 | promoted_route_trace_path |
| window_builder | joins_on | request_fingerprint | builder_join_key |
| window_builder | writes | isolated_window_trace | output_window_trace_path |
| window_inserted_rows | equals | 10 | window_report_inserted_rows |
| window_missing_matches | equals | 85 | window_report_missing_matches |
| isolated_window_trace | clears | other_shadow_requests | claim_boundary |
| route_specific_safe_policy | proves | 97_of_104_accepts | route_shadow_report |
| shadow_accepts | equals | 10 | full_window_shadow_accepts |
| shadow_false_accepts | equals | 0 | full_window_shadow_false_accepts |
| audit_verified_accepts | equals | 10 | audit_verified_accepts |
| audit_verified_false_events | equals | 0 | audit_verified_false_events |
| feedback_loop | includes | current_window_audit | feedback_report |
| feedback_loop | adds_route | test_output_parse | feedback_route_row |
| feedback_route_verified_accepts | equals | 10 | feedback_route_verified_metric |
| feedback_route_incremental_unique | equals | 10 | feedback_route_incremental_metric |
| feedback_route_false_accepts | equals | 0 | feedback_route_false_metric |
| feedback_total_verified_route_sum | equals | 42 | feedback_report_total_verified |
| feedback_total_incremental_unique | equals | 35 | feedback_report_incremental_unique |
| cpu80_claim | remains | not_proven | verified_gap_to_80_calls_758 |
| broad_answer_route | remains | not_promoted | claim_boundary |
| next_work | requires | business_value_gate | executor_review_notes |
