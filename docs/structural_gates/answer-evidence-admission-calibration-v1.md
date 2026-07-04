# Answer Evidence Admission Calibration V1

This packet checks a narrow claim boundary: `answer_or_explain` now has
request-side admission calibration over grounded evidence rows, but it found
only singleton support and therefore must not enable local accepts.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | answer_evidence_admission_command | reads | answer_evidence_output_evidence_trace | `run_role_binding_real_traffic_answer_evidence_admission_calibration_v1` evidence_trace_path | 0.99 | CLI command | input artifact | answer-evidence | admission |
| t2 | answer_evidence_admission_command | reads | codex_history_prompts | `run_role_binding_real_traffic_answer_evidence_admission_calibration_v1` history_path | 0.99 | CLI command | request-side prompt source | answer-evidence | admission |
| t3 | answer_evidence_admission_command | writes | answer_evidence_admission_report | `run_role_binding_real_traffic_answer_evidence_admission_calibration_v1` report_path | 0.99 | CLI command | output artifact | answer-evidence | admission |
| t4 | answer_evidence_admission_report | has_verdict | ANSWER_EVIDENCE_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY | answer-evidence-admission-calibration-v1.report.json verdict | 0.99 | calibration artifact | verdict | answer-evidence | admission |
| t5 | answer_evidence_admission_report | hook_ready_rows | 9 | answer-evidence-admission-calibration-v1.report.json hook_ready_rows | 0.99 | calibration artifact | measured value | answer-evidence | admission |
| t6 | answer_evidence_admission_report | rows_with_prompt_features | 9 | answer-evidence-admission-calibration-v1.report.json rows_with_prompt_features | 0.99 | calibration artifact | measured value | answer-evidence | admission |
| t7 | answer_evidence_admission_report | label_true_rows | 3 | answer-evidence-admission-calibration-v1.report.json label_true_rows | 0.99 | calibration artifact | verifier label count | answer-evidence | admission |
| t8 | answer_evidence_admission_report | label_false_rows | 6 | answer-evidence-admission-calibration-v1.report.json label_false_rows | 0.99 | calibration artifact | verifier label count | answer-evidence | admission |
| t9 | answer_evidence_admission_report | minimum_true_support | 3 | answer-evidence-admission-calibration-v1.report.json minimum_true_support | 0.99 | calibration artifact | support threshold | answer-evidence | admission |
| t10 | answer_evidence_admission_report | robust_safe_policy_found | false | answer-evidence-admission-calibration-v1.report.json robust_safe_policy_found | 0.99 | calibration artifact | policy verdict | answer-evidence | admission |
| t11 | answer_evidence_admission_report | singleton_safe_policy_found | true | answer-evidence-admission-calibration-v1.report.json singleton_safe_policy_found | 0.99 | calibration artifact | policy verdict | answer-evidence | admission |
| t12 | answer_evidence_admission_report | best_robust_true_accepts | 0 | answer-evidence-admission-calibration-v1.report.json best_robust_true_accepts | 0.99 | calibration artifact | measured value | answer-evidence | admission |
| t13 | answer_evidence_admission_report | best_singleton_true_accepts | 1 | answer-evidence-admission-calibration-v1.report.json best_singleton_true_accepts | 0.99 | calibration artifact | measured value | answer-evidence | admission |
| t14 | concise_grounded_question_policy | true_accepts | 1 | answer-evidence-admission-calibration-v1.report.json policy concise_grounded_question | 0.99 | policy row | verifier true count | answer-evidence | policy |
| t15 | concise_grounded_question_policy | false_accepts | 0 | answer-evidence-admission-calibration-v1.report.json policy concise_grounded_question | 0.99 | policy row | verifier false count | answer-evidence | policy |
| t16 | concise_grounded_question_policy | robust_safe | false | answer-evidence-admission-calibration-v1.report.json policy concise_grounded_question | 0.99 | policy row | policy verdict | answer-evidence | policy |
| t17 | cpu_operator_catalog_command | reads | answer_evidence_admission_report | cpu operator catalog loader in `crates/nando-cli/src/role_binding_runtime_cmd.rs` | 0.99 | CLI command | calibration artifact | catalog | admission |
| t18 | catalog_route_gap_answer_or_explain | answer_evidence_admission_singleton_only | true | cpu-operator-catalog-v1.report.json route_gap_family answer_or_explain row | 0.99 | catalog row | route status | answer-evidence | catalog |
| t19 | catalog_route_gap_answer_or_explain | best_robust_true_accepts | 0 | cpu-operator-catalog-v1.report.json route_gap_family answer_or_explain row | 0.99 | catalog row | measured value | answer-evidence | catalog |
| t20 | catalog_route_gap_answer_or_explain | best_singleton_true_accepts | 1 | cpu-operator-catalog-v1.report.json route_gap_family answer_or_explain row | 0.99 | catalog row | measured value | answer-evidence | catalog |
| t21 | catalog_existing_answer_or_explain_profile | answer_evidence_admission_singleton_only | true | cpu-operator-catalog-v1.report.json existing_profile_route answer_or_explain row | 0.99 | catalog row | profile status | answer-evidence | catalog |
| t22 | catalog_existing_answer_or_explain_profile | verified_cpu_accept_eligible_events | 0 | cpu-operator-catalog-v1.report.json existing_profile_route answer_or_explain row | 0.99 | catalog row | measured value | answer-evidence | catalog |
| t23 | cpu_operator_catalog_report | current_verified_cpu_accepts | 26 | cpu-operator-catalog-v1.report.json current_verified_cpu_accepts | 0.99 | catalog report | measured value | catalog | claim-boundary |
| t24 | cpu_operator_catalog_report | verified_gap_to_80_calls | 774 | cpu-operator-catalog-v1.report.json verified_gap_to_80_calls | 0.99 | catalog report | measured value | catalog | claim-boundary |
| t25 | cpu_operator_catalog_report | allows_market_claim | false | cpu-operator-catalog-v1.report.json market_claim_allowed | 0.99 | catalog report | claim permission | catalog | claim-boundary |
| t26 | answer_evidence_admission_change | enables_local_accepts | false | answer-evidence-admission-calibration-v1.report.json local_accepts_enabled | 0.99 | integration change | permission state | answer-evidence | claim-boundary |
| t27 | answer_evidence_admission_change | writes_raw_prompt_text | false | answer-evidence-admission-calibration-v1.report.json raw_prompt_text_written | 0.99 | integration change | privacy boundary | answer-evidence | claim-boundary |
| t28 | answer_evidence_admission_change | writes_raw_response_text | false | answer-evidence-admission-calibration-v1.report.json raw_response_text_written | 0.99 | integration change | privacy boundary | answer-evidence | claim-boundary |
| t29 | answer_evidence_admission_change | uses_response_text_for_features | false | answer-evidence-admission-calibration-v1.report.json response_text_used_for_features | 0.99 | integration change | anti-leak boundary | answer-evidence | claim-boundary |
| t30 | answer_evidence_admission_change | uses_target_labels_for_runtime | false | answer-evidence-admission-calibration-v1.report.json target_labels_used_for_runtime | 0.99 | integration change | anti-leak boundary | answer-evidence | claim-boundary |
| t31 | answer_evidence_admission_change | uses_proof_labels_for_runtime | false | answer-evidence-admission-calibration-v1.report.json proof_labels_used_for_runtime | 0.99 | integration change | anti-leak boundary | answer-evidence | claim-boundary |
| t32 | next_engineering_debt | requires | collect_more_verifier_true_grounded_rows_or_split_subfamily | docs/EXECUTOR_REVIEW_NOTES.md Answer Evidence Admission Calibration V1 | 0.95 | next step | required work | answer-evidence | debt |
