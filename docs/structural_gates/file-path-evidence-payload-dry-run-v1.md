# File Path Evidence Payload Dry-Run V1 Structural Gate

## query

Check that file-path evidence payload dry-run is a narrow review-only split
profile candidate, not a broad-route local-accept promotion or CPU savings
claim.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| file_path_evidence_payload_dry_run_v1 | reads | non_synthetic_codex_history_5k | target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.report.json |
| file_path_evidence_payload_dry_run_v1 | reads | broad_route_split_discovery_v1 | broad_split_report_path |
| file_path_evidence_payload_dry_run_v1 | filters_by | file_path_evidence_answer | split_key=file_path_evidence_answer |
| file_path_evidence_payload_dry_run_v1 | writes | fingerprints_features_counts_and_trace_only | raw_text_written=false |
| file_path_evidence_payload_dry_run_v1 | does_not_use | response_text | response_text_used=false |
| file_path_evidence_payload_dry_run_v1 | does_not_use | target_labels | target_labels_used=false |
| file_path_evidence_payload_dry_run_v1 | does_not_use | proof_labels | proof_labels_used=false |
| file_path_evidence_payload_dry_run_v1 | keeps_disabled | local_accepts | local_accepts_enabled=false |
| file_path_evidence_payload_dry_run_v1 | forbids | market_claim | market_claim_allowed=false |
| file_path_evidence_payload_dry_run_v1 | measured_candidate_events | 146 | report measured result |
| file_path_evidence_payload_dry_run_v1 | measured_non_exact_candidate_events | 144 | report measured result |
| file_path_evidence_payload_dry_run_v1 | measured_exact_cache_overlap_events | 2 | report measured result |
| file_path_evidence_payload_dry_run_v1 | measured_payload_ready_events | 122 | report measured result |
| file_path_evidence_payload_dry_run_v1 | measured_scoreable_payload_events | 44 | report measured result |
| file_path_evidence_payload_dry_run_v1 | has_expected_unique_cpu_accepts | zero | expected_unique_cpu_accepts_over_exact_cache=0 |
| file_path_evidence_payload_dry_run_v1 | has_expected_savings | zero | expected_savings_milli=0 |
| file_path_evidence_payload_dry_run_v1 | has_false_accepts | zero | false_accepts=0 because accepts are disabled |
| answer_or_explain | parent_of | file_path_evidence_answer | parent_route_counts answer_or_explain=79 |
| project_context_dialogue | parent_of | file_path_evidence_answer | parent_route_counts project_context_dialogue=65 |
| agent_continue_execute | parent_of | file_path_evidence_answer | parent_route_counts agent_continue_execute=2 |
| answer_or_explain | remains_blocked_as_whole | broad_route | CPU_CALL_CATALOG blocked_for_now answer_or_explain_as_a_whole |
| project_context_dialogue | remains_blocked_as_whole | broad_route | CPU_CALL_CATALOG blocked_for_now project_context_dialogue_as_a_whole |
| agent_continue_execute | remains_blocked_as_whole | broad_route | CPU_CALL_CATALOG blocked_for_now agent_continue_execute_as_a_whole |
| file_path_evidence_answer | status | candidate | payload exists but profile/verifier/shadow absent |
| cpu80_claim | rejects | scoreable_payloads_as_savings | project rule |
| cpu80_claim | requires | verified_accepts_false_accepts_zero | project rule |
| file_path_evidence_answer | next_requires | disabled_threshold_profile | next_engineering_debt compile_disabled_threshold_profile |
| file_path_evidence_answer | next_requires | source_path_or_url_presence_verifier_v1 | next_engineering_debt attach_source_path_or_url_presence_verifier_v1 |
| file_path_evidence_answer | next_requires | admission_audit | next_engineering_debt calibrate_request_side_admission |
| file_path_evidence_answer | next_requires | shadow_and_cpu_catalog | next_engineering_debt run_shadow_audit_feedback_cpu_catalog |
