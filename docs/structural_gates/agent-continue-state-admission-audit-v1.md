# Agent Continue State Admission Audit V1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| agent_continue_execute_state_admission_audit_v1 | reads | artifact_progress_trace | target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1.trace.jsonl |
| agent_continue_execute_state_admission_audit_v1 | reads | codex_sessions_jsonl | /home/ubu/.codex/sessions |
| codex_sessions_jsonl | supplies | previous_assistant_state_before_user_message | build_codex_session_previous_agent_state_index |
| previous_assistant_state_before_user_message | transformed_into | boolean_state_features | extract_agent_continue_execute_state_admission_features |
| agent_continue_execute_state_admission_audit_v1 | writes | fingerprints_features_counts_only | RoleBindingAgentContinueExecuteStateAdmissionAuditReport |
| agent_continue_execute_state_admission_audit_v1 | does_not_write | raw_prompt_text | raw_prompt_text_written=false |
| agent_continue_execute_state_admission_audit_v1 | does_not_write | raw_response_text | raw_response_text_written=false |
| agent_continue_execute_state_admission_audit_v1 | does_not_enable | local_accepts | local_accepts_enabled=false |
| verifier_labels | used_for | offline_policy_evaluation_only | target_labels_used_for_runtime=false |
| state_admission_result | found | singleton_only_no_robust_policy | best_singleton_true_accepts=1 best_robust_true_accepts=0 |
| cpu_route_feedback_loop_v1 | records | state_admission_singleton_only_next_action | agent_continue_execute next_action |
| cpu_operator_catalog_v1 | records | state_admission_singleton_only_next_action | agent_continue_execute row |
| cpu_routability_80 | unchanged_by | state_admission_audit_v1 | current_verified_cpu_accepts=26 verified_gap_to_80_calls=774 |
| market_claim | remains | disallowed | market_claim_allowed=false |

