# Test Output Parse Safe Policy V1 Structural Gate

## query

Check that `test_output_parse` safe-policy promotion uses request-side previous
tool-output state plus `.nwrb` score as runtime authority, and uses
`verified_safe_accept` only as offline audit evidence. It must not widen into
`answer_or_explain`, must not read final answers as serving authority, and must
keep the CPU80 claim bounded to a route-specific rung until full feedback
attribution is regenerated.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| safe_policy_promoter | reads | disabled_test_output_parse_registry | base_registry_config_path |
| safe_policy_promoter | reads | tool_state_payload_trace | tool_state_trace_path |
| safe_policy_promoter | writes | promoted_registry | promoted_registry_config_path |
| safe_policy_promoter | writes | promoted_trace | promoted_trace_path |
| safe_policy_promoter | writes | promote_report | report_path |
| promoted_registry | changes | threshold_to_1196032 | selected_policy_threshold |
| promoted_registry | uses | strict_ordered_energy_threshold | selected_acceptance_policy |
| runtime_authority | includes | request_side_payload_score | nwrb_score_path |
| runtime_authority | includes | known_previous_tool_output_status | request_side_policy_name |
| runtime_authority | excludes | verified_safe_accept_label | claim_boundary |
| audit_evidence | includes | verified_safe_accept_true | promoted_trace |
| audit_evidence | includes | provider_cost_estimate | provider_cost_microusd |
| audit_evidence | excludes | raw_prompt_or_response_text | raw_text_flags_false |
| safe_policy_promote_report | policy_accept_rows | 97 | report_policy_accept_rows |
| safe_policy_promote_report | runtime_acceptance_mismatches | 0 | report_runtime_mismatch |
| shadow_report | total_llm_calls | 104 | shadow_total |
| shadow_report | exact_cache_hits | 2 | shadow_exact_cache |
| shadow_report | nando_shadow_accepts | 97 | shadow_accepts |
| shadow_report | verified_safe_accepts | 97 | shadow_verified |
| shadow_report | unverified_shadow_accepts | 0 | shadow_unverified |
| shadow_report | false_accepts | 0 | shadow_false_accepts |
| shadow_report | incremental_savings_over_exact_cache | 95 | shadow_incremental |
| shadow_report | synthetic_trace_used | false | shadow_real_trace |
| verification_audit | verification_hook_ready_events | 97 | audit_hooks_ready |
| verification_audit | verified_cpu_accept_eligible_events | 97 | audit_cpu_eligible |
| verification_audit | verified_false_events | 0 | audit_no_false_labels |
| broad_answer_route | remains | not_promoted | decision_boundary |
| cpu80_claim | remains | not_proven_by_route_specific_rung | decision_boundary |
| next_debt | requires | full_feedback_unique_attribution | executor_review_notes |
