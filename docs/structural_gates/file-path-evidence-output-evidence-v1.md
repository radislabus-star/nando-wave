# File Path Evidence Output Evidence V1 Structural Gate

## query

Check that file-path evidence output evidence v1 attaches verifier labels for
the narrow split, while still forbidding local accepts and CPU savings claims.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| file_path_evidence_output_evidence_v1 | reads | file_path_evidence_payload_trace | file-path-evidence-payload-dry-run-v1.trace.jsonl |
| file_path_evidence_output_evidence_v1 | reads | codex_sessions | /home/ubu/.codex/sessions |
| file_path_evidence_output_evidence_v1 | writes | output_evidence_trace | file-path-evidence-output-evidence-v1.trace.jsonl |
| file_path_evidence_output_evidence_v1 | writes | output_evidence_report | file-path-evidence-output-evidence-v1.report.json |
| file_path_evidence_output_evidence_v1 | uses_verifier | source_path_or_url_presence_verifier_v1 | verification_source |
| file_path_evidence_output_evidence_v1 | measured_operator_candidate_calls | 44 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_scoreable_candidate_calls | 44 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_session_ids_requested | 9 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_session_files_scanned | 9 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_output_evidence_matched_events | 39 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_no_session_output_match_events | 5 | output evidence report |
| file_path_evidence_output_evidence_v1 | measured_verifier_true | 15 | verified_true_events=15 |
| file_path_evidence_output_evidence_v1 | measured_verifier_false | 24 | verified_false_events=24 |
| file_path_evidence_output_evidence_v1 | writes_no | raw_prompt_text | raw_prompt_text_written=false |
| file_path_evidence_output_evidence_v1 | writes_no | raw_response_text | raw_response_text_written=false |
| file_path_evidence_output_evidence_v1 | uses | response_text_for_verification_only | response_text_used_for_verification=true |
| file_path_evidence_output_evidence_v1 | does_not_use | target_labels | target_labels_used=false |
| file_path_evidence_output_evidence_v1 | does_not_use | proof_labels | proof_labels_used=false |
| file_path_evidence_output_evidence_v1 | keeps_disabled | local_accepts | local_accepts_enabled=false |
| file_path_evidence_output_evidence_v1 | forbids | market_claim | market_claim_allowed=false |
| file_path_evidence_output_shadow_v1 | measured_shadow_accepts | zero | nando_shadow_accepts=0 |
| file_path_evidence_output_shadow_v1 | measured_shadow_fallbacks | 44 | nando_shadow_fallbacks=44 |
| file_path_evidence_output_shadow_v1 | measured_verified_safe_accepts | zero | verified_safe_accepts=0 |
| file_path_evidence_output_shadow_v1 | measured_unverified_shadow_accepts | zero | unverified_shadow_accepts=0 |
| file_path_evidence_output_shadow_v1 | measured_false_accepts | zero | false_accepts=0 |
| file_path_evidence_output_shadow_v1 | measured_incremental_reduction | zero | incremental_reduction_vs_exact_cache_milli=0 |
| file_path_evidence_output_audit_v1 | measured_verification_hook_ready_events | 39 | verification_hook_ready_events=39 |
| file_path_evidence_output_audit_v1 | measured_verified_cpu_accept_eligible_events | zero | verified_cpu_accept_eligible_events=0 |
| file_path_evidence_output_audit_v1 | measured_candidates_missing_output_evidence | 5 | candidates_missing_output_evidence=5 |
| file_path_evidence_output_audit_v1 | measured_provider_cost_events | zero | provider_cost_events=0 |
| file_path_evidence_answer | status | candidate | verifier labels exist but no accepted rows |
| file_path_evidence_answer | next_requires | request_side_admission_calibration | next_step_request_side_admission_calibration |
| file_path_evidence_answer | next_requires | promoted_shadow_with_false_accepts_zero | cpu80_rule_promoted_shadow_false_accepts_zero |
| file_path_evidence_answer | next_requires | feedback_catalog_unique_value | cpu80_rule_feedback_catalog_unique_value |
| cpu80_claim | rejects | verifier_labels_as_savings | project_rule_verifier_labels_not_savings |
| cpu80_claim | rejects | disabled_shadow_as_savings | project_rule_disabled_shadow_not_savings |
| cpu80_claim | requires | verified_accepts_false_accepts_zero | project_rule_verified_accepts_false_accepts_zero |
