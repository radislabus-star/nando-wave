# File Path Evidence Admission Calibration V1 Structural Gate

## query

Check that file-path evidence admission calibration uses verifier labels only
to evaluate request-side admission policies, keeps local accepts disabled, and
does not turn singleton-only separation into CPU savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| file_path_evidence_admission_calibration_v1 | reads | file_path_evidence_output_evidence_trace | file-path-evidence-output-evidence-v1.trace.jsonl |
| file_path_evidence_admission_calibration_v1 | reads | codex_history_jsonl | /home/ubu/.codex/history.jsonl |
| file_path_evidence_admission_calibration_v1 | writes | admission_calibration_report | file-path-evidence-admission-calibration-v1.report.json |
| file_path_evidence_admission_calibration_v1 | filters_profile | split_file_path_evidence_answer_profile_v1 | request.profile_id |
| file_path_evidence_admission_calibration_v1 | uses_labels_from | source_path_or_url_presence_verifier_v1 | verified_safe_accept |
| file_path_evidence_admission_calibration_v1 | extracts | request_side_prompt_features | history prompt by request fingerprint |
| file_path_evidence_admission_calibration_v1 | writes_no | raw_prompt_text | raw_prompt_text_written=false |
| file_path_evidence_admission_calibration_v1 | writes_no | raw_response_text | raw_response_text_written=false |
| file_path_evidence_admission_calibration_v1 | does_not_use | response_text_for_features | response_text_used_for_features=false |
| file_path_evidence_admission_calibration_v1 | does_not_use | target_labels_for_runtime | target_labels_used_for_runtime=false |
| file_path_evidence_admission_calibration_v1 | does_not_use | proof_labels_for_runtime | proof_labels_used_for_runtime=false |
| file_path_evidence_admission_calibration_v1 | keeps_disabled | local_accepts | local_accepts_enabled=false |
| file_path_evidence_admission_calibration_v1 | forbids | market_claim | market_claim_allowed=false |
| file_path_evidence_admission_calibration_v1 | measured_hook_ready_rows | 39 | admission report |
| file_path_evidence_admission_calibration_v1 | measured_rows_with_prompt_features | 39 | admission report |
| file_path_evidence_admission_calibration_v1 | measured_history_prompt_missing_rows | zero | admission report |
| file_path_evidence_admission_calibration_v1 | measured_label_true_rows | 15 | admission report |
| file_path_evidence_admission_calibration_v1 | measured_label_false_rows | 24 | admission report |
| file_path_evidence_admission_calibration_v1 | requires_minimum_true_support | 3 | minimum_true_support=3 |
| file_path_evidence_admission_calibration_v1 | measured_robust_safe_policy_found | false | robust_safe_policy_found=false |
| file_path_evidence_admission_calibration_v1 | measured_singleton_safe_policy_found | true | singleton_safe_policy_found=true |
| file_path_evidence_admission_calibration_v1 | measured_best_robust_true_accepts | zero | best_robust_true_accepts=0 |
| file_path_evidence_admission_calibration_v1 | measured_best_singleton_true_accepts | 1 | best_singleton_true_accepts=1 |
| file_path_evidence_admission_calibration_v1 | verdict | singleton_only_no_robust_policy | FILE_PATH_EVIDENCE_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY |
| file_path_evidence_answer | status_after_admission | WATCH | singleton-only separation |
| file_path_evidence_answer | not_ready_for | safe_policy_promotion | robust_safe_policy_found=false |
| file_path_evidence_answer | not_ready_for | verified_cpu_accepts | local_accepts_enabled=false |
| file_path_evidence_answer | next_requires | more_verifier_true_non_synthetic_rows | next_engineering_debt |
| file_path_evidence_answer | next_allows | narrower_artifact_backed_split | next_engineering_debt |
| cpu80_claim | rejects | singleton_only_as_savings | project_rule_business_value_gate |
| cpu80_claim | rejects | verifier_labels_as_savings | project_rule_verifier_labels_not_savings |
| cpu80_claim | requires | robust_false_accepts_zero_shadow | project_rule_promoted_shadow_false_accepts_zero |

