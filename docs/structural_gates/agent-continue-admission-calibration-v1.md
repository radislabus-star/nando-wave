# agent-continue-admission-calibration-v1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| agent_continue_admission_command | reads | agent_continue_artifact_progress_trace | `target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1.trace.jsonl` |
| agent_continue_admission_command | reads | codex_history_prompts | `/home/ubu/.codex/history.jsonl` |
| agent_continue_admission_command | writes | admission_calibration_report | `target/nando-wave/real-traffic-shadow/agent-continue-execute-admission-calibration-v1.report.json` |
| admission_calibration_report | has_schema | `nando_role_binding_agent_continue_execute_admission_calibration_v1` | report schema field |
| admission_calibration_report | has_verdict | `AGENT_CONTINUE_EXECUTE_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY` | report verdict field |
| admission_calibration_report | counts_hook_ready_rows | `25` | report hook_ready_rows |
| admission_calibration_report | counts_prompt_feature_rows | `25` | report rows_with_prompt_features |
| admission_calibration_report | counts_missing_history_prompts | `0` | report history_prompt_missing_rows |
| admission_calibration_report | counts_true_labels | `6` | report label_true_rows |
| admission_calibration_report | counts_false_labels | `19` | report label_false_rows |
| admission_calibration_report | requires_minimum_true_support | `3` | report minimum_true_support |
| admission_calibration_report | found_robust_safe_policy | `false` | report robust_safe_policy_found |
| admission_calibration_report | found_singleton_safe_policy | `false` | report singleton_safe_policy_found |
| admission_calibration_report | best_robust_true_accepts | `0` | report best_robust_true_accepts |
| admission_calibration_report | best_singleton_true_accepts | `0` | report best_singleton_true_accepts |
| all_hook_ready_policy | accepts_true_rows | `6` | report policies.all_hook_ready_rows.true_accepts |
| all_hook_ready_policy | accepts_false_rows | `19` | report policies.all_hook_ready_rows.false_accepts |
| direct_action_words_policy | accepts_true_rows | `6` | report policies.direct_action_words.true_accepts |
| direct_action_words_policy | accepts_false_rows | `19` | report policies.direct_action_words.false_accepts |
| agent_continue_admission_command | writes_raw_prompt_text | `false` | report raw_prompt_text_written |
| agent_continue_admission_command | writes_raw_response_text | `false` | report raw_response_text_written |
| agent_continue_admission_command | uses_response_text_for_features | `false` | report response_text_used_for_features |
| agent_continue_admission_command | uses_target_labels_for_runtime | `false` | report target_labels_used_for_runtime |
| agent_continue_admission_command | uses_proof_labels_for_runtime | `false` | report proof_labels_used_for_runtime |
| agent_continue_admission_command | enables_local_accepts | `false` | report local_accepts_enabled |
| agent_continue_admission_command | allows_market_claim | `false` | report market_claim_allowed |
| agent_continue_route | remains | candidate_zone_without_accepts | no safe admission policy was found |
| next_engineering_debt | requires | split_route_or_richer_request_side_state | report next_engineering_debt |
