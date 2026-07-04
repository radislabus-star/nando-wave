# Agent Loop Profile Registry V1

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| agent_loop_profile_registry_v1 | reads | cpu_route_feedback_loop_v1_report | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json |
| agent_loop_profile_registry_v1 | reads | cpu_operator_catalog_v1_report | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json |
| agent_loop_profile_registry_v1 | writes | microprofile_worklist_report | target/nando-wave/real-traffic-shadow/agent-loop-profile-registry-v1.report.json |
| microprofile_worklist_report | contains | 16_microprofiles | microprofile_count=16 |
| microprofile_worklist_report | observes | 15_microprofiles_in_current_trace | microprofiles_observed=15 |
| microprofile_worklist_report | keeps | market_claim_disallowed | market_claim_allowed=false |
| microprofile_worklist_report | does_not_enable | local_accepts | local_accepts_enabled=false |
| microprofile_worklist_report | does_not_write | raw_prompt_or_response_text | raw_text_written=false |
| current_cpu80_state | unique_verified_accepts | 26 | cpu-route-feedback-loop-v1.report.json |
| current_cpu80_state | unique_gap_to_80 | 774 | cpu-route-feedback-loop-v1.report.json |
| wide_routes | remain | quarantined | blocked_wide_route_profiles=3 |
| top_next_microprofile | is | read_context_path | top_next_profile_key=read_context_path |
| read_context_path | source_route | read_inspect | agent-loop-profile-registry-v1.report.json |
| read_context_path | verifier_hooks | 9 | agent-loop-profile-registry-v1.report.json |
| read_context_path | unique_verified_accepts | 0 | agent-loop-profile-registry-v1.report.json |
| read_context_path | next_requirement | safe_admission_false_accepts_zero_provider_cost_unique_attribution | agent-loop-profile-registry-v1.report.json |

