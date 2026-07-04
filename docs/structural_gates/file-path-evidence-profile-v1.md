# File Path Evidence Profile V1 Structural Gate

## query

Check that file-path evidence profile v1 is a disabled scoring profile and
shadow telemetry rung, not a local-accept promotion or CPU savings claim.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| file_path_evidence_profile_v1 | reads | file_path_evidence_payload_trace | target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl |
| file_path_evidence_profile_v1 | writes | nwrb_package | target/nando-wave/real-traffic-shadow/file-path-evidence-seed0.nwrb |
| file_path_evidence_profile_v1 | writes | registry_overlay | target/nando-wave/real-traffic-shadow/profile-registry-file-path-evidence-v1.json |
| file_path_evidence_profile_v1 | writes | profile_report | target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.report.json |
| file_path_evidence_profile_v1 | profile_id | split_file_path_evidence_answer_profile_v1 | report profile_id |
| file_path_evidence_profile_v1 | has_edge_count | 7 | report edge_count |
| file_path_evidence_profile_v1 | has_scoreable_payload_events | 44 | report scoreable_payload_events |
| file_path_evidence_profile_v1 | has_threshold | i32_max | threshold=2147483647 |
| file_path_evidence_profile_v1 | has_unexpected_accepts_under_disabled_threshold | zero | unexpected_local_accepts_under_disabled_threshold=0 |
| file_path_evidence_profile_v1 | keeps_disabled | local_accepts | local_accepts_enabled_on_real_traffic=false |
| file_path_evidence_profile_v1 | forbids | market_claim | market_claim_allowed=false |
| file_path_evidence_profile_v1 | does_not_use | response_text | response_text_used=false |
| file_path_evidence_profile_v1 | does_not_use | target_labels | target_labels_used=false |
| file_path_evidence_profile_v1 | does_not_use | proof_labels | proof_labels_used=false |
| file_path_evidence_shadow_v1 | reads | file_path_registry_overlay | registry_config_path=profile-registry-file-path-evidence-v1.json |
| file_path_evidence_shadow_v1 | reads | file_path_payload_trace | trace_path=file-path-evidence-payload-dry-run-v1.trace.jsonl |
| file_path_evidence_shadow_v1 | measured_operator_candidate_calls | 44 | shadow report |
| file_path_evidence_shadow_v1 | measured_shadow_accepts | zero | nando_shadow_accepts=0 |
| file_path_evidence_shadow_v1 | measured_shadow_fallbacks | 44 | nando_shadow_fallbacks=44 |
| file_path_evidence_shadow_v1 | measured_verified_safe_accepts | zero | verified_safe_accepts=0 |
| file_path_evidence_shadow_v1 | measured_unverified_shadow_accepts | zero | unverified_shadow_accepts=0 |
| file_path_evidence_shadow_v1 | measured_false_accepts | zero | false_accepts=0 |
| file_path_evidence_shadow_v1 | measured_incremental_reduction | zero | incremental_reduction_vs_exact_cache_milli=0 |
| file_path_evidence_shadow_v1 | measured_p99_latency_ns | 259065 | p99_shadow_score_latency_ns=259065 |
| file_path_evidence_audit_v1 | reads | file_path_shadow_report | file-path-evidence-profile-shadow-v1.report.json |
| file_path_evidence_audit_v1 | measured_verification_hook_ready_events | zero | verification_hook_ready_events=0 |
| file_path_evidence_audit_v1 | measured_verified_cpu_accept_eligible_events | zero | verified_cpu_accept_eligible_events=0 |
| file_path_evidence_audit_v1 | measured_candidates_missing_output_evidence | 44 | candidates_missing_output_evidence=44 |
| file_path_evidence_audit_v1 | measured_provider_cost_events | zero | provider_cost_events=0 |
| file_path_evidence_answer | status | candidate | profile exists but verifier evidence missing |
| file_path_evidence_answer | next_requires | source_path_or_url_presence_verifier_v1 | next_route_debt_attach_source_path_or_url_presence_verifier_v1 |
| file_path_evidence_answer | next_requires | admission_audit | next_route_debt_calibrate_request_side_admission |
| file_path_evidence_answer | next_requires | feedback_catalog_unique_value | next_route_debt_feed_verified_unique_accepts_back_into_cpu_call_catalog |
| cpu80_claim | rejects | disabled_profile_as_savings | project rule |
| cpu80_claim | requires | verified_accepts_false_accepts_zero | project rule |
| local_accept_promotion | remains_blocked_by | missing_verification_hooks | verification_hook_ready_events=0 |
