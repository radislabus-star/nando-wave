# Nando Wave Project Progress Tree

Дата среза: 2026-07-04

Назначение: короткая карта текущего положения проекта без stale PASS.

```text
Nando Wave / Operator Layer
|
+-- 1. Claim Boundary                                [ACTIVE]
|   |
|   +-- цель: компактный переносимый оператор
|   +-- формула: state_t + action_tree -> state_t+1
|   +-- запреты: lookup / target_id / proof_rule_id / concrete lookup / local_out_t
|   +-- не засчитываем: stale report / Python demo / hidden hardcode
|
+-- 2. Current Product Line                          [ACTIVE DIRECTION]
|   |
|   +-- цель линейки                                 [CPU profile offload поверх exact cache]
|   +-- serving formula                              [route -> L2-sized `.nwrb` profile shard -> score -> fallback]
|   +-- serving worker loads                         [`.nwrb` only]
|   +-- serving worker does not load                 [`.nwreb` eval packs / corpus JSONL / compiler / Python demo]
|   +-- endpoints                                    [/health /profiles /score /replay /metrics]
|   +-- profile runtime smoke                        [PASS]
|   |   |
|   |   +-- profile_count                            [7]
|   |   +-- exact_cache_llm_calls                    [2]
|   |   +-- exact_cache_plus_nando_llm_calls         [1]
|   |   +-- incremental_reduction_vs_exact_cache     [500 milli]
|   |   +-- false_local_accepts                      [0]
|   |   +-- p99_latency_ns                           [21436]
|   |   +-- runtime_bytes_estimate                   [790020]
|   |   +-- rss_bytes                                [10805248]
|   +-- profile replay suite                         [PASS, default CLI path]
|   |   |
|   |   +-- unique_sequences_replayed                [896]
|   |   +-- http_replay_batches                      [224]
|   |   +-- exact_cache_llm_calls                    [896]
|   |   +-- exact_cache_plus_nando_llm_calls         [448]
|   |   +-- incremental_reduction_vs_exact_cache     [500 milli]
|   |   +-- false_local_accepts                      [0]
|   |   +-- missed_expected_local                    [0]
|   |   +-- p99_latency_ns                           [213509 release HTTP replay]
|   |   +-- runtime_bytes_estimate                   [790020]
|   |   +-- serving_eval_packs_loaded                [false]
|   |   +-- replay_client_eval_packs_used            [true]
|   +-- default replay boundary                      [PASS]
|   |   |
|   |   +-- DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE [128]
|   |   +-- DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES    [4]
|   |   +-- command                                  [cargo run --release -p nando-cli -- role-binding-profile-replay-suite-v1]
|   |   +-- live default rerun                       [PASS]
|   +-- profile fallback smoke                       [PASS]
|   |   |
|   |   +-- command                                  [cargo run -p nando-cli -- role-binding-profile-fallback-smoke-v1]
|   |   +-- local_accept_pass                        [true]
|   |   +-- bad_route_fallback_pass                  [true, profile_not_found]
|   |   +-- low_margin_fallback_pass                 [true, margin_below_threshold]
|   |   +-- local_operator_calls / fallback_calls    [1 / 2]
|   |   +-- false_local_accepts                      [0]
|   |   +-- p99_latency_ns                           [24312]
|   +-- profile worker scaling                       [PASS]
|   |   |
|   |   +-- worker_count                             [2]
|   |   +-- total_profile_count                      [7]
|   |   +-- shard profile split                      [4 / 3]
|   |   +-- total_local_operator_calls               [7]
|   |   +-- wrong_worker_route_fallbacks             [2]
|   |   +-- false_local_accepts                      [0]
|   |   +-- max_worker_runtime_bytes_estimate        [398456]
|   |   +-- max_worker_rss_bytes                     [6557696]
|   |   +-- max_worker_p99_latency_ns                [6286]
|   +-- profile worker replay                        [PASS]
|   |   |
|   |   +-- worker_count                             [2]
|   |   +-- total_profile_count                      [7]
|   |   +-- unique_sequences_replayed                [896]
|   |   +-- exact_cache_llm_calls                    [896]
|   |   +-- exact_cache_plus_nando_llm_calls         [448]
|   |   +-- incremental_reduction_vs_exact_cache     [500 milli]
|   |   +-- false_local_accepts                      [0]
|   |   +-- missed_expected_local                    [0]
|   |   +-- max_worker_runtime_bytes_estimate        [398456]
|   |   +-- max_worker_p99_latency_ns                [265277]
|   +-- profile local load-balancer replay           [PASS]
|   |   |
|   |   +-- worker_count                             [2]
|   |   +-- total_profile_count                      [7]
|   |   +-- unique_sequences_replayed                [896]
|   |   +-- exact_cache_llm_calls                    [896]
|   |   +-- exact_cache_plus_nando_llm_calls         [448]
|   |   +-- incremental_reduction_vs_exact_cache     [500 milli]
|   |   +-- false_local_accepts                      [0]
|   |   +-- missed_expected_local                    [0]
|   |   +-- load_balancer_p99_latency_ns             [736030]
|   |   +-- core_score_p99_latency_ns                [78902]
|   |   +-- worker_score_p99_latency_ns              [167663]
|   |   +-- lb_upstream_roundtrip_p99_latency_ns     [735692]
|   |   +-- replay_client_wall_p99_latency_ns        [5489536]
|   |   +-- packed_score_parity_mismatches           [0 / 647928]
|   |   +-- max_worker_runtime_bytes_estimate        [492792]
|   |   +-- max_worker_p99_latency_ns                [167663]
|   +-- profile deployed cheap-VPS LB replay         [PASS, close to 3 ms envelope]
|   |   |
|   |   +-- host_alias                               [hostworld-ee]
|   |   +-- worker_count                             [2]
|   |   +-- total_profile_count                      [7]
|   |   +-- unique_sequences_replayed                [896]
|   |   +-- exact_cache_llm_calls                    [896]
|   |   +-- exact_cache_plus_nando_llm_calls         [448]
|   |   +-- incremental_reduction_vs_exact_cache     [500 milli]
|   |   +-- false_local_accepts                      [0]
|   |   +-- missed_expected_local                    [0]
|   |   +-- load_balancer_p99_latency_ns             [2993688]
|   |   +-- core_score_p99_latency_ns                [187721]
|   |   +-- worker_score_p99_latency_ns              [545095]
|   |   +-- lb_upstream_roundtrip_p99_latency_ns     [2993349]
|   |   +-- replay_client_wall_p99_latency_ns        [22311949]
|   |   +-- packed_score_parity_mismatches           [0 / 647928]
|   |   +-- max_worker_runtime_bytes_estimate        [492792]
|   |   +-- max_worker_p99_latency_ns                [545095]
|   +-- profile local bounded POST /score throughput [PASS]
|   |   |
|   |   +-- client_threads                           [4]
|   |   +-- score_requests                           [896]
|   |   +-- local_operator_calls                     [448]
|   |   +-- fallback_to_llm_calls                    [448]
|   |   +-- false_local_accepts                      [0]
|   |   +-- client_errors                            [0]
|   |   +-- load_balancer_p99_latency_ns             [1409548]
|   |   +-- core_score_p99_latency_ns                [119908]
|   |   +-- worker_score_p99_latency_ns              [305953]
|   +-- profile deployed bounded POST /score throughput [FAIL]
|   |   |
|   |   +-- host_alias                               [hostworld-ee]
|   |   +-- client_threads                           [4]
|   |   +-- score_requests                           [896]
|   |   +-- false_local_accepts                      [0]
|   |   +-- client_errors                            [0]
|   |   +-- load_balancer_p99_latency_ns             [3611864]
|   |   +-- lb_upstream_roundtrip_p99_latency_ns     [3610931]
|   |   +-- worker_score_p99_latency_ns              [577626]
|   |   +-- core_score_p99_latency_ns                [221243]
|   +-- real traffic routability / savings claim     [REVIEW, current default unique 15/1000; v3 diagnostic unique 16/1000]
|   |   |
|   |   +-- source                                  [non-synthetic Codex history trace]
|   |   +-- current default verified CPU route-sum accepts [20 / 1000]
|   |   +-- current default unique verified CPU accepts [15 / 1000]
|   |   +-- historical v3 diagnostic route-sum accepts [21 / 1000]
|   |   +-- historical v3 diagnostic unique accepts  [16 / 1000]
|   |   +-- historical v3 duplicate route hits       [5]
|   |   +-- exact_cache_hits                         [53]
|   |   +-- current default operator_candidate_calls [570]
|   |   +-- current default no_candidate_calls       [430]
|   |   +-- current default scoreable_candidate_calls [178]
|   |   +-- current default verification_hook_ready_events [138]
|   |   +-- false_accepts                            [0]
|   |   +-- current default unique_gap_to_80_calls   [785]
|   |   +-- current default agent-control route      [v2 visible row: 143 candidates, 11 route-sum verified]
|   |   +-- strongest diagnostic route               [agent-control strict forms: 11 verified in v3 check]
|   |   +-- mixed_map v2 route                       [3 verified, request-side goal/control fallback]
|   |   +-- git_control tool-output route            [1 verified, no git execution]
|   |   +-- market claim boundary                    [not achieved; current default unique 15 milli != 800 milli]
|   +-- current route-gap mining                     [REVIEW, measurement only]
|   |   |
|   |   +-- report                                  [route-gap-catalog-current-v1.report.json]
|   |   +-- registry                                [profile-registry-planning-next-step-v3.json]
|   |   +-- routed_candidate_events                  [462 / 1000]
|   |   +-- no_candidate_events                      [538 / 1000]
|   |   +-- readiness_report                         [route-gap-payload-readiness-current-v1.report.json]
|   |   +-- payload_ready_gap_events                 [35]
|   |   +-- pre_read_inspect_top_payload_ready_gap   [read_inspect: 12 ready / 27 candidates]
|   |   +-- read_inspect_route_status                [payload/profile/evidence integrated; accepts disabled]
|   |   +-- git_control_route_status                 [payload/profile/evidence integrated; accepts disabled]
|   |   +-- next_payload_ready_gaps                  [serving_ops, retrieval_lookup]
|   |   +-- market claim boundary                    [not savings; local_accepts_enabled=false]
|   +-- read_inspect route/profile/evidence rung     [REVIEW, hooks ready, accepts disabled]
|   |   |
|   |   +-- dry_run_report                           [read-inspect-payload-dry-run-v1.report.json]
|   |   +-- profile_report                           [read-inspect-profile-v1.report.json]
|   |   +-- registry                                 [profile-registry-read-inspect-v1.json]
|   |   +-- output_evidence_report                   [read-inspect-output-evidence-v1.report.json]
|   |   +-- output_evidence_audit                    [read-inspect-output-evidence-v1.verification-hook-audit.report.json]
|   |   +-- local_accept_calibration_report          [read-inspect-local-accept-calibration-v1.report.json]
|   |   +-- dry_run_candidate_events                 [27]
|   |   +-- dry_run_payload_ready_events             [12]
|   |   +-- scoreable_payload_events                 [12]
|   |   +-- edge_count                               [8]
|   |   +-- runtime_bytes_estimate                   [33000]
|   |   +-- output_evidence_matched_events           [9]
|   |   +-- deterministic_verification_events        [8]
|   |   +-- verifier_true_events                     [1]
|   |   +-- verifier_false_events                    [8]
|   |   +-- verification_hook_ready_events           [9]
|   |   +-- candidates_missing_output_evidence       [3]
|   |   +-- local_accept_safe_policy_found           [false]
|   |   +-- local_accept_best_safe_true_accepts      [0]
|   |   +-- local_accept_minimum_true_support        [3]
|   |   +-- local_accept_support_qualified           [false]
|   |   +-- route_stage                              [local_accept_calibration_failed]
|   |   +-- shadow_accepts                           [0]
|   |   +-- verified_cpu_accept_eligible_events      [0]
|   |   +-- false_accepts                            [0]
|   |   +-- market claim boundary                    [not savings; local_accepts_enabled=false; no safe readout policy]
|   +-- metrics_report route/profile/evidence rung  [REVIEW, hooks ready, accepts disabled]
|   |   |
|   |   +-- dry_run_report                           [metrics-report-payload-dry-run-v1.report.json]
|   |   +-- profile_report                           [metrics-report-profile-v1.report.json]
|   |   +-- registry                                 [profile-registry-metrics-report-v1.json]
|   |   +-- output_evidence_report                   [metrics-report-output-evidence-v1.report.json]
|   |   +-- output_evidence_audit                    [metrics-report-output-evidence-v1.verification-hook-audit.report.json]
|   |   +-- local_accept_calibration_report          [metrics-report-local-accept-calibration-v1.report.json]
|   |   +-- dry_run_candidate_events                 [55]
|   |   +-- dry_run_payload_ready_events             [42]
|   |   +-- scoreable_payload_events                 [42]
|   |   +-- edge_count                               [8]
|   |   +-- runtime_bytes_estimate                   [33000]
|   |   +-- output_evidence_matched_events           [32]
|   |   +-- deterministic_verification_events        [32]
|   |   +-- verifier_true_events                     [18]
|   |   +-- verifier_false_events                    [14]
|   |   +-- verification_hook_ready_events           [32]
|   |   +-- candidates_missing_output_evidence       [10]
|   |   +-- local_accept_safe_policy_found           [true]
|   |   +-- local_accept_best_safe_true_accepts      [2]
|   |   +-- local_accept_minimum_true_support        [3]
|   |   +-- local_accept_support_qualified           [false]
|   |   +-- route_stage                              [local_accept_calibration_support_insufficient]
|   |   +-- shadow_accepts                           [0]
|   |   +-- verified_cpu_accept_eligible_events      [0]
|   |   +-- market claim boundary                    [not savings; support 2 < 3; local_accepts_enabled=false]
|   +-- metrics_report 5000-row safe-policy soak    [PASS narrow shadow, 3/5000 separate window]
|   |   |
|   |   +-- artifact_root                            [target/nando-wave/real-traffic-shadow/metrics-report-soak-v1]
|   |   +-- trace_rows_written                       [5000]
|   |   +-- candidate_events                         [98]
|   |   +-- scoreable_payload_events                 [63]
|   |   +-- output_evidence_matched_events           [51]
|   |   +-- selected_acceptance_policy               [first_slot_threshold_active_fringe_min_114]
|   |   +-- request_side_policy_name                 [metrics_report_active_fringe_min_114]
|   |   +-- selected_policy_threshold                [393216]
|   |   +-- request_side_policy_evaluated_rows       [63]
|   |   +-- request_side_policy_accept_rows          [11]
|   |   +-- request_side_policy_reject_rows          [52]
|   |   +-- nando_shadow_accepts                     [3]
|   |   +-- verified_safe_accepts                    [3]
|   |   +-- unverified_shadow_accepts                [0]
|   |   +-- false_accepts                            [0]
|   |   +-- market_claim_allowed                     [true]
|   |   +-- boundary                                 [separate 5000-row soak; not added to default 12/1000 until unified feedback regeneration]
|   |   +-- current_feedback_impact                  [none; separate 5000-row denominator]
|   |   +-- next_debt                                [regenerate one unified feedback window before counting this route beside default 1000-row routes]
|   +-- git_control route/profile/evidence/promoted-safe-policy rung [PASS narrow shadow, 1/1000]
|   |   |
|   |   +-- dry_run_report                           [git-control-payload-dry-run-v1.report.json]
|   |   +-- profile_report                           [git-control-profile-v1.report.json]
|   |   +-- base_registry                            [profile-registry-git-control-v1.json]
|   |   +-- promoted_registry                        [profile-registry-git-control-safe-policy-v1.json]
|   |   +-- promoted_trace                           [git-control-safe-policy-v1.trace.jsonl]
|   |   +-- output_evidence_report                   [git-control-output-evidence-v1.report.json]
|   |   +-- output_evidence_audit                    [git-control-output-evidence-v1.verification-hook-audit.report.json]
|   |   +-- promoted_audit                           [git-control-safe-policy-v1.verification-hook-audit.report.json]
|   |   +-- local_accept_calibration_report          [git-control-local-accept-calibration-v1.report.json]
|   |   +-- dry_run_candidate_events                 [18]
|   |   +-- dry_run_payload_ready_events             [12]
|   |   +-- scoreable_payload_events                 [12]
|   |   +-- edge_count                               [8]
|   |   +-- runtime_bytes_estimate                   [33000]
|   |   +-- median_energy_margin                     [1240064]
|   |   +-- p10_energy_margin                        [906240]
|   |   +-- output_evidence_matched_events           [10]
|   |   +-- deterministic_verification_events        [10]
|   |   +-- verifier_true_events                     [6]
|   |   +-- verifier_false_events                    [4]
|   |   +-- tool_call_fingerprint_events            [5]
|   |   +-- verification_hook_ready_events           [10]
|   |   +-- candidates_missing_output_evidence       [2]
|   |   +-- local_accept_safe_policy_found           [true]
|   |   +-- local_accept_best_safe_true_accepts      [5]
|   |   +-- local_accept_minimum_true_support        [3]
|   |   +-- local_accept_support_qualified           [true]
|   |   +-- selected_policy_threshold                [1505280]
|   |   +-- nando_shadow_accepts                     [1]
|   |   +-- verified_safe_accepts                    [1]
|   |   +-- unverified_shadow_accepts                [0]
|   |   +-- workspace_mutation_enabled               [false]
|   |   +-- local_accepts_enabled_in_live_daemon     [false]
|   |   +-- route_stage                              [verified_cpu_accept_eligible]
|   |   +-- verified_cpu_accept_eligible_events      [1]
|   |   +-- false_accepts                            [0]
|   |   +-- market claim boundary                    [narrow route shadow PASS; tool-output fingerprints only; no git execution or workspace mutation; not CPU Routability 80]
|   +-- route-gap after git_control registry        [REVIEW, next serving_ops]
|   |   |
|   |   +-- route_gap_catalog                        [route-gap-catalog-git-control-v1.report.json]
|   |   +-- readiness_report                         [route-gap-payload-readiness-git-control-v1.report.json]
|   |   +-- existing_route_candidate_events          [507 / 1000]
|   |   +-- no_candidate_events                      [493 / 1000]
|   |   +-- payload_ready_events                     [13]
|   |   +-- top_payload_ready_family                 [serving_ops]
|   +-- serving_ops route/profile/evidence/promoted-safe-policy rung [PASS narrow shadow, 3/1000]
|   |   |
|   |   +-- registry                                  [profile-registry-serving-ops-safe-policy-v1.json]
|   |   +-- promoted_trace                           [serving-ops-safe-policy-v1.trace.jsonl]
|   |   +-- selected_policy                          [market_safe_energy_margin_threshold]
|   |   +-- selected_policy_threshold                [1392640]
|   |   +-- candidate_events                         [25 / 1000]
|   |   +-- scoreable_payload_events                 [8]
|   |   +-- verification_hook_ready_events           [7]
|   |   +-- nando_shadow_accepts                     [3]
|   |   +-- verified_safe_accepts                    [3]
|   |   +-- false_accepts                            [0]
|   |   +-- unverified_shadow_accepts                [0]
|   |   +-- incremental_savings_over_exact_cache     [3]
|   |   +-- p99_shadow_score_latency_ns              [156653]
|   |   +-- server_mutation_enabled                  [false]
|   |   +-- route_stage                              [verified_cpu_accept_eligible]
|   |   +-- market claim boundary                    [narrow route shadow PASS; not CPU Routability 80]
|   +-- route-gap after serving_ops registry        [REVIEW, uncatalogued next]
|   |   |
|   |   +-- route_gap_catalog                        [route-gap-catalog-serving-ops-v1.report.json]
|   |   +-- readiness_report                         [route-gap-payload-readiness-serving-ops-v1.report.json]
|   |   +-- existing_route_candidate_events          [520 / 1000]
|   |   +-- no_candidate_events                      [480 / 1000]
|   |   +-- payload_ready_events                     [10]
|   |   +-- top_payload_ready_family                 [uncatalogued]
|   +-- current default feedback after read_inspect + metrics_report calibration + agent-control v2 safe-policy + git_control safe-policy + serving_ops safe-policy [REVIEW, unique 15/1000]
|   |   |
|   |   +-- feedback_report                          [cpu-route-feedback-loop-v1.report.json]
|   |   +-- operator_candidate_calls                 [570 / 1000]
|   |   +-- no_candidate_calls                       [430 / 1000]
|   |   +-- scoreable_candidate_calls                [178 / 1000]
|   |   +-- verification_hook_ready_events           [138]
|   |   +-- verified_cpu_accept_eligible_events      [20]
|   |   +-- verified_cpu_accept_route_sum_events     [20]
|   |   +-- unique_verified_cpu_accepts              [15]
|   |   +-- incremental_unique_cpu_accepts           [15]
|   |   +-- duplicate_verified_route_hits            [5]
|   |   +-- exact_cache_overlap_verified_cpu_accepts [1]
|   |   +-- verified_gap_to_80_calls                 [780 route-sum, not scoreboard]
|   |   +-- unique_verified_gap_to_80_calls          [785]
|   |   +-- audit_window_mismatches                  [[]]
|   |   +-- window_guard_negative_test               [cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json excludes 5000-row metrics audit from 1000-row forecast]
|   |   +-- v3_dedup_check                           [21 route-sum -> 16 unique accepted requests; 5 duplicate route hits]
|   |   +-- agent_control_stage                      [verified_cpu_accept_eligible; v2 143 candidates, 11 route-sum accepts]
|   |   +-- git_control_stage                        [verified_cpu_accept_eligible]
|   |   +-- serving_ops_stage                        [verified_cpu_accept_eligible]
|   |   +-- boundary                                 [current scoreboard is unique request-fingerprint accepts, not route-sum]
|   +-- current CPU operator catalog                [REVIEW, unique scoreboard]
|   |   |
|   |   +-- catalog_report                           [cpu-operator-catalog-current-feedback-v1.report.json]
|   |   +-- current_verified_cpu_accepts             [15 unique]
|   |   +-- verified_cpu_accept_route_sum_events     [20]
|   |   +-- verified_cpu_accept_duplicate_route_hits [5]
|   |   +-- route_gap_feedback_no_candidate_mismatch [true]
|   |   +-- top_catalog_row                          [role_binding_agent_control_seed0]
|   +-- role-binding release suite                   [PASS]
|   |   |
|   |   +-- package_count                            [7]
|   |   +-- total_sequence_count                     [27648]
|   |   +-- min_strict_ordered_accuracy              [1000/1000]
|   |   +-- false_local_accepts                      [0]
|   +-- OPERATOR_BLUEPRINT coverage                  [WATCH]
|   |   |
|   |   +-- proven_classes                           [0]
|   |   +-- partial_classes                          [7]
|   |   +-- missing_classes                          [2]
|   |   +-- missing                                  [FIELD, FILTER_GROUP]
|   +-- next required product work                   [OPEN]
|       |
|       +-- real Codex/API/agent traffic shadow recorder
|       +-- real traffic routability / savings report
|       +-- per-score HostWorld throughput recovery
|       +-- persistent or binary LB -> worker upstream
|       +-- production-like daemon/watchdog/metrics on server
|
+-- 3. Research Rungs                                [STRONG, NOT FINAL]
|   |
|   +-- v2 ordered sequence length 3-6               [PASS 1000/1000]
|   +-- paged u32 / 16-slot rung                     [PASS 1000/1000]
|   |   |
|   |   +-- PAGE_COUNT = 32 means memory pages, not 32 output slots
|   |   +-- closed: 16 slots, lengths 13..16
|   |   +-- closed: 32-slot order corpus multi-seed rung, lengths 17..32
|   |   +-- closed: 32-slot mixed map rung, order/edit-map/composed-map
|   |   +-- closed: 32-slot conditional branch rung, symbolic branch-map inputs
|   |   +-- closed: 32-slot mixed+conditional multi-seed combined gate
|   |   +-- closed: 32-slot mixed+conditional cache/offload benchmark
|   |   +-- closed: serialized 32-slot role-binding `.nwrb` package proof
|   |   +-- closed: public Rust SDK smoke for role-binding `.nwrb` package loading
|   |   +-- closed: public SDK package runtime gate over loaded `.nwrb`
|   |   +-- closed: CLI inspect/verify for loaded `.nwrb` package artifact
|   |   +-- closed: CLI score/verify over explicit `.nwrb` eval-pack interface
|   |   +-- closed: independent corpus-emitted `.nwrb` CLI sequence scoring for representative 32-slot conditional package
|   |   +-- closed: compact binary `.nwreb` eval-pack scoring for representative 32-slot conditional package
|   |   +-- closed: all-seed compact binary `.nwreb` eval-pack suite for current 32-slot role-binding package set
|   |   +-- closed: role-binding release-suite product-proof bundle for current `.nwrb/.nwreb` set
|   |   +-- closed: EDIT marker/length `.nwrb/.nwreb` release-suite integration [PARTIAL OPERATOR_BLUEPRINT]
|   |   +-- closed: serving-only `.nwrb` profile registry/runtime smoke
|   |   |   |
|   |   |   +-- endpoints: /health /profiles /score /replay /metrics
|   |   |   +-- profile_count: 7
|   |   |   +-- runtime_bytes_estimate: 790020
|   |   |   +-- exact_cache_plus_nando reduction: 500 milli
|   |   |   +-- false_local_accepts: 0
|   |   |   +-- p99_latency_ns: 21436
|   |   |   +-- eval_packs_loaded / corpus_jsonl_loaded / compiler_used: false / false / false
|   |   +-- closed: serving-only `.nwrb` profile replay suite against exact cache
|   |   |   |
|   |   |   +-- unique_sequences_replayed: 896
|   |   |   +-- exact_cache_plus_nando reduction: 500 milli
|   |   |   +-- false_local_accepts: 0
|   |   |   +-- release p99_latency_ns: 213509
|   |   |   +-- eval packs used by replay client only, not serving worker
|   |   +-- closed: OPERATOR_BLUEPRINT gap audit over current role-binding release suite [WATCH, source-verified]
|   |   +-- not closed: full 32-slot operator battery
|   |   |   |
|   |   |   +-- partial: SELECT / MOVE_COPY / EDIT / ORDER / CONDITION_ROUTE / COMPOSE / VERIFY_REPAIR
|   |   |   +-- missing in release-suite battery: FIELD / FILTER_GROUP
|   |   +-- not closed: phase-center `.nwpc` bridge/product package for this strict role-binding path
|   |   +-- closed: deployed cheap-VPS replay for sampled release-suite profile runtime
|   |   +-- not closed: real Codex/API production traffic for `.nwrb` profile runtime
|   |   +-- not closed: raw-language action parsing
|   |
|   +-- v4 multiseed strict                          [PASS CURRENT SOURCE]
|   |   |
|   |   +-- strict-multiseed-rust-audit-v1           [PASS current-source log audit]
|   |   +-- strict-multiseed-rust-audit-verify-v1    [PASS, report_matches_sources true]
|   |   +-- observed logs                            [12]
|   |   +-- strict runtime issues                    [0]
|   |   +-- evidence warnings                        [0]
|   |   +-- order/edit/conditional/composed logs     [12/12 ok]
|   |   +-- strict slot / flat / energy              [1000 across logs]
|   |   +-- energy_pass_slot_fail                    [0]
|   |   +-- output_slot_cleanup_failed_slots         [0]
|   |   +-- flat parity mismatches                   [0]
|   |   +-- forbidden flags                          [false]
|   |   +-- fresh logs vs latest hebbian source      [12/12]
|   |   +-- fresh logs vs latest test source         [12/12]
|   |   +-- fresh logs vs phase package source       [12/12 after 23:05 CLI edit]
|   |   +-- current-source boundary                  [closed for 16-slot v4 strict rung]
|   |
|   +-- V5 operator-dimension coverage               [RELEASE-INTEGRATED PASS]
|       |
|       +-- persisted corpus path                    [exists]
|       |   data/rule_logic_operator_battery_v5/action_contract_v1/generated_coverage_action_contract_v1.jsonl
|       +-- corpus rows                              [360]
|       +-- train / heldout                          [180 / 180]
|       +-- operator_key_count                       [30]
|       +-- select/transform/write/condition/check   [6 / 10 / 5 / 5 / 10]
|       +-- same_bag_rows                            [360]
|       +-- shortcut gate                            [PASS]
|       +-- runtime smoke C32                        [PASS 1000/1000]
|       +-- action ablation                          [486/1000, wrong_wins 2682]
|
+-- 4. Flat CPU Runtime Package                      [FROZEN GREEN SNAPSHOT]
|   |
|   +-- release suite                                [PASS]
|   |   |
|   |   +-- artifact_count                           [3]
|   |   +-- release_suite_report_fingerprint64       [9827723825761118426]
|   |   +-- regression_report_fingerprint64          [2002304595771295125]
|   |   +-- workflow_replay_report_fingerprint64     [16637049491119000274]
|   |   +-- workflow_bench_report_fingerprint64      [7479237649753576261]
|   |   +-- operator_blueprint_fingerprint64         [9874423192353457577]
|   |   +-- generated_action                         [PASS package, coverage WATCH]
|   |   +-- domain_action                            [PASS package, coverage WATCH]
|   |   +-- coverage_action                          [PASS package, full coverage PASS]
|   |   +-- total_runtime_bytes_estimate             [48576]
|   |   +-- total_bench_samples                      [308000]
|   |   +-- max_bench_p99_latency_ns                 [117]
|   |   +-- all_package_report_parity_pass           [true]
|   |   +-- all_shortcut_reports_pass                [true]
|   |   +-- all_action_ablation_collapses            [true]
|   |   +-- compiler_used / corpus_jsonl_used        [false / false]
|   |   +-- forbidden_used                           [false]
|   |
|   +-- operator coverage at release level           [PASS]
|       |
|       +-- all_operator_coverage_reports_match_sources [true]
|       +-- operator_dimension_coverage_artifact_count  [1]
|       +-- release_operator_dimension_coverage_pass    [true]
|       +-- max_min_dimension_value_count               [5]
|       +-- max_wide_dimension_count                    [5]
|
+-- 5. Product Proof Chain                           [PASS, STRICT CURRENT-SOURCE GREEN]
|   |
|   +-- cargo fmt --check                            [PASS]
|   +-- cargo check -p nando-cli                     [PASS]
|   +-- phase-action-release-suite-v1                [PASS]
|   +-- phase-action-release-verify-v1               [PASS, report_matches_sources true]
|   +-- phase-action-license-verify-v1               [PASS, report_matches_sources true]
|   +-- phase-action-offload-verify-v1               [PASS, report_matches_sources true]
|   +-- phase-action-cache-offload-bench-verify-v1   [PASS, report_matches_sources true]
|   +-- phase-action-workflow-bench-v1               [PASS]
|   |   |
|   |   +-- report_matches_sources                   [true]
|   |   +-- workflow_simulated_calls                 [144]
|   |   +-- workflow_nando_fallback_events           [0]
|   |   +-- reduction vs cache                       [1000 milli]
|   |   +-- local accuracy / false accepts           [1000 / 0]
|   |   +-- p99 latency ns                           [85]
|   |   +-- forbidden_used                           [false]
|   +-- phase-action-regression-v1                   [PASS, workflow replay anchored]
|   +-- phase-action-regression-verify-v1            [PASS, report_matches_sources true]
|   +-- phase-action-regression-freeze-v1            [PASS, workflow replay anchored]
|   +-- phase-action-regression-freeze-verify-v1     [PASS, report_matches_sources true]
|   +-- strict-multiseed-rust-audit-v1               [PASS current-source log audit]
|   |   |
|   |   +-- observed_logs                            [12]
|   |   +-- strict_runtime_issues                    [0]
|   |   +-- logs_fingerprint64                       [2847134219208477714]
|   |   +-- freshness                                [PASS: logs newer than 23:05 source edit]
|   +-- strict-multiseed-rust-audit-verify-v1        [PASS, report_matches_sources true]
|   +-- canonical runtime rerun                      [DONE, GREEN LOGS AFTER 23:05 CLI EDIT]
|   |   |
|   |   +-- runtime logs                             [12/12 ok]
|   |   +-- first runtime log timestamp              [2026-07-02 23:24:45]
|   |   +-- last runtime log timestamp               [2026-07-03 00:08:10]
|   |   +-- wavepredictor_hebbian.rs timestamp       [2026-07-02 17:28:01]
|   |   +-- phase_package_cmd.rs timestamp at rerun  [2026-07-02 23:05:01]
|   |   +-- phase_package_cmd.rs latest timestamp    [2026-07-02 23:05:01]
|   |   +-- wavepredictor_binding_pressure_l3.rs timestamp [2026-07-02 18:11:21]
|   |   +-- logs newer than latest source/test       [12/12]
|   +-- workflow replay regression/freeze anchor     [PASS]
|   |   |
|   |   +-- workflow_replay_report_fingerprint64     [16637049491119000274]
|   |   +-- workflow_replay_report_bytes             [5274]
|   |   +-- workflow_replay_verify_pass              [true]
|   |   +-- workflow_replay_report_matches_sources   [true]
|   |   +-- workflow_replay_trace_calls              [3072]
|   |   +-- workflow_replay_total_unique_eval_rows   [308]
|   |   +-- workflow_replay_unique_rows              [308]
|   |   +-- exact_cache / exact_cache_plus_nando     [308 / 36]
|   |   +-- incremental_llm_calls_removed_vs_cache   [272]
|   |   +-- reduction_vs_cache_milli                 [883]
|   |   +-- local_accuracy / false_accepts           [1000 / 0]
|   +-- stale blueprint/freeze recheck               [caught and refreshed]
|   +-- cargo clippy nando-cli/nando-core            [PASS]
|   +-- RUSTFLAGS=-Dwarnings phase_center_runtime    [PASS 22 tests]
|
+-- 6. Offload / Cache-Enabled Benchmark             [PASS]
|   |
|   +-- offload_rate_milli                           [880]
|   +-- local_accuracy_milli                         [1000]
|   +-- false_local_accepts                          [0]
|   +-- public Rust SDK test                         [PASS]
|   +-- offload audit verify                         [PASS, report_matches_sources true]
|   +-- regression freeze verify                     [PASS, report_matches_sources true]
|   +-- offload_sdk_api                              [nando_core::PhaseCenterOffloadRuntime]
|   +-- offload_runtime_summary_api                  [PhaseCenterOffloadRuntime::offload_summary_into]
|   +-- loopback HTTP service smoke                  [PASS]
|   |   +-- command                                  [phase-action-daemon-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-smoke.product-proof.json]
|   |   +-- local/fallback HTTP requests             [2/2 status 200]
|   |   +-- false_local_accepts                      [0]
|   +-- existing package HTTP service smoke          [PASS]
|   |   +-- command                                  [phase-action-daemon-package-smoke-v1]
|   |   +-- serve command                            [phase-action-daemon-serve-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-package-smoke.product-proof.json]
|   |   +-- package                                  [action-runtime-v1-generated-coverage-c32.nwpc]
|   |   +-- package_fingerprint64                    [11103824464258352074]
|   |   +-- local/fallback HTTP requests             [2/2 status 200]
|   |   +-- local_margin_micro                       [791009]
|   |   +-- fallback_margin_micro                    [-791009]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP hardening smoke                         [PASS]
|   |   +-- command                                  [phase-action-daemon-hardening-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-hardening-smoke.product-proof.json]
|   |   +-- endpoints                                [/health /stats /score]
|   |   +-- bad_route_status_code                    [404]
|   |   +-- http_max_request_bytes                   [65536]
|   |   +-- http_requests_handled                    [4]
|   |   +-- http_bad_requests                        [1]
|   |   +-- local/fallback calls                     [1/1]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP bearer-auth smoke                       [PASS]
|   |   +-- command                                  [phase-action-daemon-auth-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-auth-smoke.product-proof.json]
|   |   +-- protected endpoints                      [/score /stats]
|   |   +-- public endpoint                          [/health]
|   |   +-- unauthorized_score_status_code           [401]
|   |   +-- authorized_score_status_code             [200]
|   |   +-- authorized_stats_status_code             [200]
|   |   +-- local/fallback calls                     [1/1]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP multi-package registry smoke            [PASS]
|   |   +-- command                                  [phase-action-daemon-registry-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-registry-smoke.product-proof.json]
|   |   +-- package_aliases                          [generated_action, domain_action, coverage_action]
|   |   +-- package_count                            [3]
|   |   +-- generated/domain/coverage status          [200/200/200]
|   |   +-- missing_alias_status_code                [404]
|   |   +-- packages_status_code                     [200]
|   |   +-- local_operator_calls                     [3]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP registry config-file smoke              [PASS]
|   |   +-- command                                  [phase-action-daemon-registry-config-smoke-v1]
|   |   +-- config                                   [target/nando-wave/action-runtime-v1-daemon-registry.config.json]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-registry-config-smoke.product-proof.json]
|   |   +-- package_aliases                          [generated_action, domain_action, coverage_action]
|   |   +-- package_count                            [3]
|   |   +-- generated/domain/coverage status          [200/200/200]
|   |   +-- missing_alias_status_code                [404]
|   |   +-- packages_status_code                     [200]
|   |   +-- stats_status_code                        [200]
|   |   +-- server_runtime_config_used                [true]
|   |   +-- server_runtime_compiler_used              [false]
|   |   +-- server_runtime_corpus_jsonl_used          [false]
|   |   +-- local_operator_calls                     [3]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP score rate-limit smoke                  [PASS]
|   |   +-- command                                  [phase-action-daemon-rate-limit-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-rate-limit-smoke.product-proof.json]
|   |   +-- max_score_requests                       [1]
|   |   +-- allowed_score_status_code                [200]
|   |   +-- rate_limited_score_status_code           [429]
|   |   +-- http_score_requests                      [1]
|   |   +-- http_rate_limited_requests               [1]
|   |   +-- local_operator_calls                     [1]
|   |   +-- fallback_to_llm_calls                    [0]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP structured observability smoke          [PASS]
|   |   +-- command                                  [phase-action-daemon-observability-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-observability-smoke.product-proof.json]
|   |   +-- package_aliases                          [generated_action, domain_action, coverage_action]
|   |   +-- max_score_requests                       [1]
|   |   +-- stats counters                           [score=1, bad=2, rate_limited=1]
|   |   +-- runtime provenance                       [config=true, compiler=false, corpus=false, python=false]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP structured audit-log smoke              [PASS]
|   |   +-- command                                  [phase-action-daemon-audit-log-smoke-v1]
|   |   +-- event_log                                [target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.events.jsonl]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.product-proof.json]
|   |   +-- event_count                              [6]
|   |   +-- statuses                                 [200, 200, 404, 200, 429, 200]
|   |   +-- request_kinds                            [health, packages, error, score, error, stats]
|   |   +-- audit_flags_pass                         [true]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP error-taxonomy smoke                    [PASS]
|   |   +-- command                                  [phase-action-daemon-error-taxonomy-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-error-taxonomy-smoke.product-proof.json]
|   |   +-- statuses                                 [400, 404, 413, 413, 400, 405, 413]
|   |   +-- error_messages_pass                      [true]
|   |   +-- score_requests                           [0]
|   |   +-- bad_requests                             [7]
|   |   +-- false_local_accepts                      [0]
|   +-- HTTP registry config validation smoke        [PASS]
|   |   +-- command                                  [phase-action-daemon-config-validation-smoke-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-config-validation-smoke.product-proof.json]
|   |   +-- valid_package_count                      [3]
|   |   +-- invalid_case_count                       [5]
|   |   +-- invalid_reject_count                     [5]
|   |   +-- invalid_error_messages_pass              [true]
|   |   +-- server_started_for_invalid_configs       [false]
|   +-- HTTP daemon proof suite                      [PASS]
|   |   +-- command                                  [phase-action-daemon-proof-suite-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-proof-suite.product-proof.json]
|   |   +-- artifact_count                           [12]
|   |   +-- pass_count                               [12]
|   |   +-- all_forbidden_flags_false                [true]
|   |   +-- all_python_demo_false                    [true]
|   |   +-- all_server_runtime_hot_path_clean        [true]
|   |   +-- all_false_local_accepts_zero             [true]
|   +-- HTTP daemon live proof suite                 [PASS]
|   |   +-- command                                  [phase-action-daemon-live-proof-suite-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json]
|   |   +-- live_rerun_performed                     [true]
|   |   +-- live_rerun_step_count                    [12]
|   |   +-- artifact_count                           [12]
|   |   +-- pass_count                               [12]
|   |   +-- all_server_runtime_hot_path_clean        [true]
|   |   +-- all_false_local_accepts_zero             [true]
|   +-- HTTP daemon systemd packaging smoke          [PASS]
|   |   +-- command                                  [phase-action-daemon-systemd-smoke-v1]
|   |   +-- service                                  [target/nando-wave/nando-wave-action-daemon.service]
|   |   +-- env                                      [target/nando-wave/nando-wave-action-daemon.env]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json]
|   |   +-- package_count                            [3]
|   |   +-- service_manager_artifacts_written        [true]
|   |   +-- service_exec_serve_registry              [true]
|   |   +-- service_hardening_pass                   [true]
|   |   +-- installed_to_systemd                     [false]
|   |   +-- systemctl_invoked                        [false]
|   +-- HTTP daemon deployment package              [PASS]
|   |   +-- command                                  [phase-action-daemon-deployment-package-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json]
|   |   +-- live_suite_artifact_count                [12]
|   |   +-- live_suite_step_count                    [12]
|   |   +-- live_suite_contains_systemd              [true]
|   |   +-- live_suite_hot_path_clean                [true]
|   |   +-- systemd_smoke_pass                       [true]
|   |   +-- systemd_hardening_pass                   [true]
|   |   +-- service_unit_exec_matches                [true]
|   |   +-- service_unit_env_matches                 [true]
|   |   +-- env_file_config_matches                  [true]
|   |   +-- registry_config_package_count            [3]
|   |   +-- deployment_artifacts_present             [true]
|   |   +-- installed_to_systemd                     [false]
|   |   +-- systemctl_invoked                        [false]
|   +-- HTTP daemon deployment verify               [PASS]
|   |   +-- command                                  [phase-action-daemon-deployment-verify-v1]
|   |   +-- report                                   [target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json]
|   |   +-- report_gate_pass                         [true]
|   |   +-- rebuilt_gate_pass                        [true]
|   |   +-- report_matches_sources                   [true]
|   +-- exact_cache_llm_calls                        [308]
|   +-- exact_cache_plus_nando_llm_calls             [36]
|   +-- incremental_llm_calls_removed_vs_cache       [272]
|   +-- incremental_llm_call_reduction_vs_cache_milli [883]
|
+-- 7. Product / License                             [OPEN]
    |
    +-- non-commercial license package               [PASS]
    +-- commercial_license_closed                    [false]
    +-- installable package                          [not closed]
    +-- real pilot workflow                          [not closed]
    +-- multi-package workflow replay                [PASS]
    |   +-- command                                  [phase-action-workflow-replay-v1]
    |   +-- verify_command                           [phase-action-workflow-replay-verify-v1]
    |   +-- report                                   [target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json]
    |   +-- workflow_trace_calls                     [3072]
    |   +-- total_unique_eval_rows                   [308]
    |   +-- replay_unique_rows                       [308]
    |   +-- package_aliases                          [generated_action, domain_action, coverage_action]
    |   +-- exact_cache_llm_calls                    [308]
    |   +-- exact_cache_plus_nando_llm_calls         [36]
    |   +-- incremental_llm_calls_removed_vs_cache   [272]
    |   +-- local_accuracy_milli                     [1000]
    |   +-- false_local_accepts                      [0]
    |   +-- report_matches_sources                   [true]
    |   +-- tamper_replay_unique_rows_307            [WATCH]
    +-- public Rust SDK surface                      [PASS]
    +-- single-package HTTP service surface          [PASS smoke]
    +-- first HTTP hardening smoke                   [PASS]
    +-- HTTP bearer-auth smoke                       [PASS]
    +-- HTTP multi-package registry smoke            [PASS]
    +-- HTTP registry config-file smoke              [PASS]
    +-- HTTP score rate-limit smoke                  [PASS]
    +-- HTTP structured observability smoke          [PASS]
    +-- HTTP structured audit-log smoke              [PASS]
    +-- HTTP error-taxonomy smoke                    [PASS]
    +-- HTTP registry config validation smoke        [PASS]
    +-- HTTP daemon proof suite                      [PASS]
    +-- HTTP daemon systemd packaging smoke          [PASS]
    +-- HTTP daemon deployment package               [PASS]
    +-- HTTP daemon deployment verify                [PASS]
    +-- production HTTP daemon hardening             [not closed]
```

Текущий ближайший доказательный долг:

```text
Product-chain coverage plumbing is closed in the current frozen reports, and
the cached/log audit is green over the current fresh 12 runtime logs.

Immediate blocker:
  cargo fmt --check: PASS
  cargo check -p nando-cli: PASS
  cargo clippy -p nando-cli -p nando-core -- -D warnings: PASS

  strict-multiseed audit helper wiring exists and verifies.
  The canonical log audit is now PASS:
    observed_logs: 12;
    strict_runtime_issues: 0;
    evidence_warnings: 0;
    logs_fingerprint64: 2847134219208477714.

  Full 12-log rerun evidence:
    logs_total: 12;
    test_ok: 12;
    target/proof/concrete/local_out forbidden flags: false in all 12;
    order/edit/conditional/composed strict slot/flat/energy: 1000;
    parity mismatches: 0;
    hard ablations collapse for binding/action/role/active_fringe.

  Diagnostic subchannel caveat:
    edit marker_role ablation leaves energy high while strict accuracy drops
    to 500 milli;
    conditional condition_action ablation leaves partial energy and seed_003
    has 3 milli strict accuracy.
    Do not claim isolated subchannel collapse beyond the hard proof gates.

  Current-source freshness:
    The strict 12-log chain was rerun after the 2026-07-02 23:05:01
    phase_package_cmd.rs source edit.

    Fresh runtime window:
      first canonical runtime log: 2026-07-02 23:24:45;
      last canonical runtime log: 2026-07-03 00:08:10;
      stale logs vs latest source: 0.

    Therefore the v4 16-slot strict behavior is current-source green.

  Evidence warnings:
    strict_multiseed_evidence_warnings: 0

Boundary:
  legacy workflow bench is still a small synthetic domain_action benchmark:
    unique eval rows 48;
    simulated calls 144;
    fallback events 0.

  multi-package workflow replay is now PASS and is anchored into
  regression/freeze:
    workflow trace calls 3072;
    all 308 unique eval rows replayed;
    generated_action/domain_action/coverage_action all observed;
    exact-cache LLM calls 308;
    exact-cache+Nando LLM calls 36;
    incremental LLM calls removed vs cache 272;
    regression/freeze workflow replay verify true;
    regression/freeze workflow replay report_matches_sources true;
    false_local_accepts 0.

  Do not call either one broad workflow reasoning or a real pilot workflow.

Open proof-debt:
  real external pilot workflow beyond deterministic frozen-package replay;
  full 32-slot operator battery beyond current mixed/conditional multi-seed rung;
  EDIT package/eval-pack integration beyond current-source runtime gate;
  phase-center .nwpc bridge/product package beyond serialized .nwrb role-binding scorer and public SDK smoke;
  production HTTP daemon hardening beyond local smoke/proof suite;
  text/domain prototype;
  raw-language action parsing;
  commercial license package.
```

Что сказать исполнителю:

```text
Stop spending time on release-suite coverage plumbing unless a new stale report appears.
It is green with coverage_action included.

Next best work:
  1. keep v4 16-slot strict audit frozen as behavioral regression;
  2. bridge public `.nwrb` role-binding SDK evidence into a product-facing `.nwpc`/registry path, or keep it explicitly separate;
  3. extend from symbolic branch-map inputs to clean action_tree/raw-action parsing;
  4. production HTTP daemon hardening if product integration needs it;
  5. real external pilot workflow beyond the synthetic domain_action bench.
```

Короткая оценка:

```text
Research core:          70-80%
CPU runtime proof:      82-88%
Product-proof package:  80-88%
LLM offload product:    35-45%
Продаваемая лицензия:   20-30%
```

Самое важное:

```text
V5 coverage_action теперь не просто scratch proof.
Он включён в release/regression/freeze цепочку packaged flat action scorer.

Граница claim остаётся:
это не text generation, не autonomous raw action parser и не broad workflow reasoning.
```
