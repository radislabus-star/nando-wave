# IME Input-State Payload Evidence V1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| ime_input_state_stage_v1 | starts_from | manual_route_discovery_5k | manual-route-discovery-v1-5k.report.json:top_subfamily=ime_input_state_debug |
| ime_input_state_stage_v1 | sees | 16_candidate_events | ime-input-state-payload-dry-run-v1.report.json:ime_input_state_candidate_events=16 |
| ime_input_state_stage_v1 | sees | 6_payload_ready_events | ime-input-state-payload-dry-run-v1.report.json:payload_ready_events=6 |
| ime_payload_builder_v1 | builds | 6_scoreable_payloads | ime-input-state-payload-dry-run-v1.report.json:scoreable_payload_events=6 |
| ime_payload_builder_v1 | uses | prompt_side_only | role_binding_runtime_cmd.rs:run_role_binding_real_traffic_ime_input_state_payload_dry_run_v1 |
| ime_payload_builder_v1 | keeps | local_accepts_disabled | build_ime_input_state_dry_run_request:expect_local_operator=false |
| ime_payload_builder_v1 | forbids | raw_text_and_labels | raw_text_written=false target_labels_used=false proof_labels_used=false |
| ime_payload_builder_v1 | reports | profile_missing | ime-input-state-payload-dry-run-v1.report.json:profile_registered=false |
| ime_output_evidence_v1 | matches | 5_session_outputs | ime-input-state-output-evidence-v1.report.json:output_evidence_matched_events=5 |
| ime_output_evidence_v1 | verifies | 3_true_2_false | ime-input-state-output-evidence-v1.report.json:verified_true_events=3 verified_false_events=2 |
| ime_output_evidence_v1 | writes | fingerprints_only | ime-input-state-output-evidence-v1.report.json:raw_response_text_written=false |
| ime_shadow_v1 | scores | 6_operator_candidates | ime-input-state-shadow-v1.report.json:operator_candidate_calls=6 |
| ime_shadow_v1 | accepts | zero_local_requests | ime-input-state-shadow-v1.report.json:nando_shadow_accepts=0 |
| ime_shadow_v1 | records | zero_false_accepts | ime-input-state-shadow-v1.report.json:false_accepts=0 |
| ime_verification_audit_v1 | finds | 5_ready_hooks | ime-input-state-output-evidence-v1.verification-hook-audit.report.json:verification_hook_ready_events=5 |
| ime_verification_audit_v1 | finds | zero_cpu_accept_eligible | ime-input-state-output-evidence-v1.verification-hook-audit.report.json:verified_cpu_accept_eligible_events=0 |
| ime_verification_audit_v1 | blocks | market_claim | ime-input-state-output-evidence-v1.verification-hook-audit.report.json:market_claim_allowed=false |
| cpu80_state | remains | 26_accepts_774_gap | docs/EXECUTOR_REVIEW_NOTES.md:IME Input-State Payload + Evidence V1 |
| ime_next_step | requires | disabled_threshold_profile_then_admission_audit | docs/EXECUTOR_REVIEW_NOTES.md:IME Input-State Payload + Evidence V1 |
