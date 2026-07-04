# Test Output Parse Output Evidence V1 Structural Gate

## query

Check that `test_output_parse` output evidence attaches verifier labels to the
scoreable dry-run rows, but does not promote local accepts or count market
savings from those labels.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| output_evidence | reads | test_output_parse_dry_run_trace | input_trace |
| output_evidence | reads | codex_sessions | sessions_root |
| output_evidence | writes | enriched_trace | output_trace |
| output_evidence | writes | evidence_report | report |
| output_evidence | filters_profile | route_gap_test_output_parse_profile_v1 | profile_id |
| output_evidence | attaches | response_fingerprint | final_answer_fingerprint |
| output_evidence | attaches | test_status_verifier_label | deterministic_verifier |
| output_evidence | excludes | raw_prompt_text | raw_prompt_text_written_false |
| output_evidence | excludes | raw_response_text | raw_response_text_written_false |
| output_evidence | excludes | target_or_proof_labels | label_flags_false |
| output_evidence | keeps | local_accepts_disabled | local_accepts_flag |
| output_evidence | keeps | market_claim_disabled | market_claim_flag |
| evidence_report | verdict | evidence_attached | verdict_field |
| evidence_report | operator_candidate_calls | 3 | report_count_candidates |
| evidence_report | scoreable_candidate_calls | 3 | report_count_scoreable |
| evidence_report | output_evidence_matched_events | 3 | report_count_matched |
| evidence_report | deterministic_verification_events | 3 | report_count_verification |
| evidence_report | verified_true_events | 3 | report_count_true |
| evidence_report | verified_false_events | 0 | report_count_false |
| evidence_report | verifier_not_applicable_events | 0 | report_count_not_applicable |
| cpu80_claim | rejects | verifier_labels_as_savings | business_value_gate |
| cpu80_claim | requires | profile_shadow_and_admission | claim_boundary |
| next_debt | requires | disabled_profile_or_tool_output_state_capture | next_engineering_debt |

