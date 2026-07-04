# Agent Loop Profile Registry V1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| agent_loop_profile_registry_v1 | reads | cpu_route_feedback_loop_v1_report | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json |
| agent_loop_profile_registry_v1 | reads | cpu_operator_catalog_v1_report | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json |
| agent_loop_profile_registry_v1 | reads | read_inspect_admission_audit_v1_report | target/nando-wave/real-traffic-shadow/read-inspect-admission-audit-v1.report.json |
| agent_loop_profile_registry_v1 | reads | resource_pressure_output_evidence_v1_report | target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.report.json |
| agent_loop_profile_registry_v1 | reads | resource_pressure_verification_audit_v1_report | target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.verification-hook-audit.report.json |
| agent_loop_profile_registry_v1 | writes | microprofile_worklist_report | target/nando-wave/real-traffic-shadow/agent-loop-profile-registry-v1.report.json |
| microprofile_worklist_report | contains | 16_microprofiles | agent-loop-profile-registry-v1.report.json:microprofile_count=16 |
| microprofile_worklist_report | observes | 15_microprofiles_in_current_trace | agent-loop-profile-registry-v1.report.json:microprofiles_observed=15 |
| microprofile_worklist_report | keeps | market_claim_disallowed | agent-loop-profile-registry-v1.report.json:market_claim_allowed=false |
| microprofile_worklist_report | does_not_enable | local_accepts | agent-loop-profile-registry-v1.report.json:local_accepts_enabled=false |
| microprofile_worklist_report | does_not_write | raw_prompt_or_response_text | agent-loop-profile-registry-v1.report.json:raw_text_written=false |
| current_cpu80_state | unique_verified_accepts | 26 | cpu-route-feedback-loop-v1.report.json:verified_cpu_accept_unique_request_fingerprints=26 |
| current_cpu80_state | unique_gap_to_80 | 774 | cpu-route-feedback-loop-v1.report.json:unique_verified_gap_to_80_calls=774 |
| wide_routes | remain | quarantined | agent-loop-profile-registry-v1.report.json:blocked_wide_route_profiles=3 |
| read_inspect_admission_audit_v1 | finds | no_robust_safe_policy | read-inspect-admission-audit-v1.report.json:robust_safe_policy_found=false |
| read_inspect_admission_audit_v1 | finds | no_singleton_safe_policy | read-inspect-admission-audit-v1.report.json:singleton_safe_policy_found=false |
| read_inspect_admission_audit_v1 | observes | one_true_eight_false_hook_ready_rows | read-inspect-admission-audit-v1.report.json:label_true_rows=1,label_false_rows=8 |
| read_context_path | source_route | read_inspect | agent-loop-profile-registry-v1.report.json:rows[read_context_path].source_route_key=read_inspect |
| read_context_path | verifier_hooks | 9 | agent-loop-profile-registry-v1.report.json:rows[read_context_path].verification_hook_ready_events=9 |
| read_context_path | unique_verified_accepts | 0 | agent-loop-profile-registry-v1.report.json:rows[read_context_path].unique_verified_request_fingerprints=0 |
| read_context_path | readiness_state | admission_audit_no_safe_policy | agent-loop-profile-registry-v1.report.json:rows[read_context_path].readiness_state=admission_audit_no_safe_policy |
| read_context_path | blocked_by | read_inspect_admission_audit_v1 | agent-loop-profile-registry-v1.report.json:rows[read_context_path].admission_blocked_by_audit=true |
| response_shape_brevity | readiness_state | support_exhausted_or_singleton_only | cpu-operator-catalog-v1.report.json:rows[style_brevity].style_brevity_verifier_true_support_zero=true |
| resource_pressure_output_evidence_v1 | finds | zero_true_one_false | resource-pressure-output-evidence-v1.report.json:verified_true_events=0,verified_false_events=1 |
| resource_pressure_verification_audit_v1 | finds | zero_eligible_accepts | resource-pressure-output-evidence-v1.verification-hook-audit.report.json:verified_cpu_accept_eligible_events=0 |
| resource_budget_extract | source_route | resource_pressure_budget | agent-loop-profile-registry-v1.report.json:rows[resource_budget_extract].source_route_key=resource_pressure_budget |
| resource_budget_extract | verifier_hooks | 1 | agent-loop-profile-registry-v1.report.json:rows[resource_budget_extract].verification_hook_ready_events=1 |
| resource_budget_extract | unique_verified_accepts | 0 | agent-loop-profile-registry-v1.report.json:rows[resource_budget_extract].unique_verified_request_fingerprints=0 |
| resource_budget_extract | readiness_state | verification_audit_no_true_resource_budget_rows | agent-loop-profile-registry-v1.report.json:rows[resource_budget_extract].readiness_state=verification_audit_no_true_resource_budget_rows |
| resource_budget_extract | blocked_by | resource_pressure_verification_audit_v1 | agent-loop-profile-registry-v1.report.json:rows[resource_budget_extract].admission_blocked_by_audit=true |
| top_next_microprofile | is | none | agent-loop-profile-registry-v1.report.json:top_next_profile_key=null |
