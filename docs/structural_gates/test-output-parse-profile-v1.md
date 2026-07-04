# Test Output Parse Profile V1 Structural Gate

## query

Check that `test_output_parse` profile compilation creates a disabled-threshold
`.nwrb` profile and shadow-scoring route, while keeping verified CPU accepts,
threshold lowering, and market savings claims blocked until explicit verifier
and admission evidence exist.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| profile_builder | reads | base_profile_registry | base_registry_path |
| profile_builder | reads | tool_state_payload_trace | dry_run_trace_path |
| profile_builder | writes | test_output_parse_nwrb_package | package_path |
| profile_builder | writes | overlay_profile_registry | registry_path |
| profile_builder | writes | profile_report | report_path |
| profile_builder | compiles | route_gap_test_output_parse_profile_v1 | profile_id |
| profile_builder | keeps_threshold | i32_max_disabled | threshold_field |
| profile_builder | excludes | raw_prompt_tool_response_text | raw_text_flags_false |
| profile_builder | excludes | target_or_proof_labels | label_flags_false |
| profile_builder | keeps | local_accepts_disabled | local_accepts_flag |
| profile_builder | keeps | market_claim_disabled | market_claim_flag |
| profile_report | verdict | profile_ready_accepts_disabled | verdict_field |
| profile_report | scoreable_payload_events | 97 | report_scoreable |
| profile_report | package_training_requests | 97 | report_training |
| profile_report | edge_count | 7 | report_edge_count |
| profile_report | changed_edges | 112 | report_changed_edges |
| profile_report | strict_ordered_pass_rows | 97 | report_strict_pass |
| profile_report | unexpected_local_accepts_under_disabled_threshold | 0 | report_disabled_guard |
| shadow_report | operator_candidate_calls | 97 | shadow_candidates |
| shadow_report | nando_shadow_accepts | 0 | shadow_accepts |
| shadow_report | nando_shadow_fallbacks | 97 | shadow_fallbacks |
| shadow_report | false_accepts | 0 | shadow_false_accepts |
| shadow_report | synthetic_trace_used | false | shadow_non_synthetic |
| audit_report | verdict | missing_hooks | audit_verdict |
| audit_report | verification_hook_ready_events | 0 | audit_hooks_missing |
| audit_report | verified_cpu_accept_eligible_events | 0 | audit_no_eligible_accepts |
| audit_report | candidates_missing_explicit_verification | 97 | audit_missing_verification |
| cpu80_claim | rejects | profile_margins_as_savings | business_value_gate |
| cpu80_claim | requires | verifier_admission_false_accepts_zero | claim_boundary |
| next_debt | requires | deterministic_verifier_and_admission_audit | next_engineering_debt |
