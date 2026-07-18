use std::process::ExitCode;

mod args;
mod artifact_budget;
mod bench;
mod chat0_cmd;
mod help;
mod live;
mod modadd_cmd;
mod organ128_cmd;
mod phase_daemon_cmd;
mod phase_package_cmd;
mod phase_streaming_cmd;
mod snapshot_io;
mod status;
use args::{
    parse_bench_stage2_tick_args, parse_cases_only_args, parse_live_byte_train_args,
    parse_live_grok_sweep_args, parse_live_grok_trace_args, parse_optional_seed_arg,
    parse_periodic_args, parse_phase_composition_args, parse_phase_holdout_args,
    parse_seed_pair_cases_args, parse_snapshot_save_args, parse_wave_tick_args,
};
use bench::{
    print_link_tissue_bench, print_stage2_tick_bench, print_symbol_l3_bench,
    print_wave_layer_metrics,
};
use chat0_cmd::{
    run_chat0_once, run_chat0_once_promoted, run_chat0_promote_save, run_chat0_shell,
    run_eval_chat0_promote, run_eval_chat0_promoted_holdout,
};
use help::print_help;
use live::{
    print_live_architecture_compare, print_live_byte_holdout, print_live_byte_holdout_seed_sweep,
    print_live_byte_holdout_suite, print_live_byte_learn, print_live_byte_train,
    print_live_cell_promote, print_live_grok_sweep, print_live_grok_trace,
    print_live_tissue_diagnose,
};
use modadd_cmd::{run_organ128_modadd_eval, run_organ128_modadd_seed_sweep};
use organ128_cmd::{
    run_organ128_dialog_generate, run_organ128_response_gate_eval, run_organ128_settle_dialog,
    run_organ128_thought_probe_eval, run_organ128_train_generate, run_organ128_wave_scorer_eval,
};
use phase_daemon_cmd::{
    run_phase_action_daemon_audit_log_smoke_v1, run_phase_action_daemon_auth_smoke_v1,
    run_phase_action_daemon_config_validation_smoke_v1,
    run_phase_action_daemon_deployment_package_v1, run_phase_action_daemon_deployment_verify_v1,
    run_phase_action_daemon_error_taxonomy_smoke_v1, run_phase_action_daemon_hardening_smoke_v1,
    run_phase_action_daemon_live_proof_suite_v1, run_phase_action_daemon_observability_smoke_v1,
    run_phase_action_daemon_package_smoke_v1, run_phase_action_daemon_proof_suite_v1,
    run_phase_action_daemon_rate_limit_smoke_v1, run_phase_action_daemon_registry_config_smoke_v1,
    run_phase_action_daemon_registry_smoke_v1, run_phase_action_daemon_serve_registry_v1,
    run_phase_action_daemon_serve_v1, run_phase_action_daemon_smoke_v1,
    run_phase_action_daemon_systemd_smoke_v1,
};
use phase_package_cmd::{
    run_phase_action_boundary_v4, run_phase_action_cache_offload_bench_v1,
    run_phase_action_cache_offload_bench_verify_v1, run_phase_action_contract_v1,
    run_phase_action_corpus_v1, run_phase_action_coverage_corpus_v1,
    run_phase_action_domain_corpus_v1, run_phase_action_eval_pack_v1,
    run_phase_action_license_package_v1, run_phase_action_license_verify_v1,
    run_phase_action_offload_audit_v1, run_phase_action_offload_verify_v1,
    run_phase_action_operator_coverage_v1, run_phase_action_package_bench_pack_v1,
    run_phase_action_package_bench_verify_v1, run_phase_action_package_inspect_v1,
    run_phase_action_package_score_pack_v1, run_phase_action_package_score_v1,
    run_phase_action_package_v1, run_phase_action_package_verify_v1,
    run_phase_action_product_proof_v1, run_phase_action_product_verify_v1,
    run_phase_action_regression_freeze_v1, run_phase_action_regression_freeze_verify_v1,
    run_phase_action_regression_v1, run_phase_action_regression_verify_v1,
    run_phase_action_release_suite_v1, run_phase_action_release_verify_v1,
    run_phase_action_runtime_v1, run_phase_action_shortcut_v1, run_phase_action_source_verify_v1,
    run_phase_action_workflow_bench_v1, run_phase_action_workflow_bench_verify_v1,
    run_phase_action_workflow_replay_v1, run_phase_action_workflow_replay_verify_v1,
    run_phase_eval_pack_v4, run_phase_package_inspect, run_phase_package_score_pack_v4,
    run_phase_package_score_v4, run_phase_package_v4, run_phase_package_verify,
    run_strict_multiseed_rust_audit_v1, run_strict_multiseed_rust_audit_verify_v1,
};
use phase_streaming_cmd::{
    run_phase_stream_agent_continue_active_turn_state_v1,
    run_phase_stream_agent_continue_command_result_followup_pack_v1,
    run_phase_stream_agent_continue_subroute_scoreboard_v1,
    run_phase_stream_auto_subcenter_discovery_v1, run_phase_stream_automatic_continuation_split_v1,
    run_phase_stream_automatic_discovery_chain_gate_v1,
    run_phase_stream_codex_history_phase_atom_trace_v1,
    run_phase_stream_codex_session_live_append_v1,
    run_phase_stream_codex_session_planning_verifier_trace_v1,
    run_phase_stream_codex_session_run_check_verifier_trace_v1,
    run_phase_stream_codex_session_tool_status_verifier_trace_v1,
    run_phase_stream_codex_sessions_live_append_v1, run_phase_stream_constrained_split_miner_v1,
    run_phase_stream_discovery_v1, run_phase_stream_global_denominator_compressibility_audit_v1,
    run_phase_stream_hot_path_benchmark_v1,
    run_phase_stream_hot_path_daemon_admission_policy_smoke_v1,
    run_phase_stream_hot_path_daemon_append_live_loop_smoke_v1,
    run_phase_stream_hot_path_daemon_append_live_tail_v1,
    run_phase_stream_hot_path_daemon_append_shadow_gate_v1,
    run_phase_stream_hot_path_daemon_live_loop_budget_smoke_v1,
    run_phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1,
    run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1,
    run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1,
    run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1,
    run_phase_stream_hot_path_daemon_numeric_future_package_audit_v1,
    run_phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1,
    run_phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1,
    run_phase_stream_hot_path_daemon_shadow_gate_v1,
    run_phase_stream_live_source_adapter_worker_v1, run_phase_stream_live_store_adapter_smoke_v1,
    run_phase_stream_live_store_clean_manifest_admission_gate_v1,
    run_phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1,
    run_phase_stream_live_store_clean_manifest_live_policy_stage_v1,
    run_phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1,
    run_phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1,
    run_phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1,
    run_phase_stream_live_store_clean_manifest_shadow_registry_replay_v1,
    run_phase_stream_live_store_clean_manifest_shadow_v1,
    run_phase_stream_live_store_direct_batch_thread_smoke_v1,
    run_phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1,
    run_phase_stream_live_store_prepared_hot_pack_v1,
    run_phase_stream_live_worker_batch_thread_smoke_v1,
    run_phase_stream_live_worker_memory_smoke_v1, run_phase_stream_live_worker_queue_smoke_v1,
    run_phase_stream_live_worker_thread_smoke_v1, run_phase_stream_online_discovery_v1,
    run_phase_stream_online_miner_daemon_v1,
    run_phase_stream_online_miner_portfolio_admission_gate_v1,
    run_phase_stream_online_miner_portfolio_billing_evidence_contract_v1,
    run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1,
    run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1,
    run_phase_stream_online_miner_portfolio_billing_request_v1,
    run_phase_stream_online_miner_portfolio_clean_subset_manifest_v1,
    run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1,
    run_phase_stream_online_miner_portfolio_future_tail_billing_request_v1,
    run_phase_stream_online_miner_portfolio_future_tail_replay_v1,
    run_phase_stream_online_miner_portfolio_live_tail_billing_request_v1,
    run_phase_stream_online_miner_portfolio_live_tail_score_only_v1,
    run_phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1,
    run_phase_stream_online_miner_portfolio_np_rescue_v1,
    run_phase_stream_online_miner_portfolio_promotion_manifest_v1,
    run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1,
    run_phase_stream_online_miner_portfolio_provider_export_admission_v1,
    run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1,
    run_phase_stream_online_miner_portfolio_provider_export_normalize_v1,
    run_phase_stream_online_miner_portfolio_provider_export_watch_v1,
    run_phase_stream_online_miner_portfolio_runtime_replay_v1,
    run_phase_stream_online_miner_portfolio_selector_billing_request_v1,
    run_phase_stream_online_miner_portfolio_selector_v1,
    run_phase_stream_online_miner_promotion_billing_request_v1,
    run_phase_stream_online_miner_promotion_provider_capture_request_v1,
    run_phase_stream_online_miner_promotion_registry_gate_v1,
    run_phase_stream_online_miner_targeted_admission_gate_v1,
    run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1,
    run_phase_stream_online_miner_targeted_aggregate_billing_request_v1,
    run_phase_stream_online_miner_targeted_aggregate_gate_v1,
    run_phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1,
    run_phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1,
    run_phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1,
    run_phase_stream_online_miner_targeted_aggregate_provider_export_autoscan_v1,
    run_phase_stream_online_miner_targeted_billing_request_v1,
    run_phase_stream_online_miner_targeted_rejection_drilldown_v1,
    run_phase_stream_online_miner_targeted_shadow_v1,
    run_phase_stream_online_miner_targeted_split_refinement_v1,
    run_phase_stream_online_miner_value_pass_v1, run_phase_stream_opportunity_board_v1,
    run_phase_stream_phase_atom_action_family_separability_audit_v1,
    run_phase_stream_phase_atom_action_family_serving_admission_audit_v1,
    run_phase_stream_phase_atom_action_family_time_split_discovery_v1,
    run_phase_stream_phase_atom_compatible_denominator_shadow_v1,
    run_phase_stream_phase_atom_diversity_backlog_v1,
    run_phase_stream_phase_atom_frontier_billing_request_v1,
    run_phase_stream_phase_atom_frontier_claim_audit_v1,
    run_phase_stream_phase_atom_frontier_shadow_replay_v1,
    run_phase_stream_phase_atom_live_admission_manifest_v1,
    run_phase_stream_phase_atom_live_admission_policy_smoke_v1,
    run_phase_stream_phase_atom_live_capture_readiness_v1,
    run_phase_stream_phase_atom_live_daemon_shadow_gate_v1,
    run_phase_stream_phase_atom_live_self_mining_loop_v1,
    run_phase_stream_phase_atom_market_money_claim_gate_v1,
    run_phase_stream_phase_atom_run_check_discovery_v1,
    run_phase_stream_phase_atom_run_check_time_split_discovery_v1,
    run_phase_stream_phase_atom_run_check_time_split_promotion_audit_v1,
    run_phase_stream_phase_atom_serving_append_shadow_replay_v1,
    run_phase_stream_phase_atom_serving_future_shadow_replay_v1,
    run_phase_stream_phase_atom_serving_shadow_replay_v1,
    run_phase_stream_phase_atom_trace_sample_v1,
    run_phase_stream_phase_atom_verifier_needed_ranking_v1,
    run_phase_stream_provider_billing_evidence_join_v1,
    run_phase_stream_provider_boundary_append_sink_v1,
    run_phase_stream_provider_boundary_billing_capture_chain_v1,
    run_phase_stream_provider_boundary_billing_capture_contract_v1,
    run_phase_stream_provider_boundary_billing_capture_evidence_gate_v1,
    run_phase_stream_provider_boundary_capture_coverage_gate_v1,
    run_phase_stream_provider_boundary_capture_request_v1,
    run_phase_stream_provider_boundary_codex_token_backfill_v1,
    run_phase_stream_provider_boundary_correlation_join_v1,
    run_phase_stream_provider_boundary_export_ingest_v1,
    run_phase_stream_provider_boundary_live_chain_v1,
    run_phase_stream_provider_boundary_live_np_chain_v1,
    run_phase_stream_provider_boundary_match_readiness_v1,
    run_phase_stream_provider_boundary_np_chain_from_phase_trace_v1,
    run_phase_stream_provider_boundary_np_chain_v1,
    run_phase_stream_provider_boundary_phase_atom_trace_v1,
    run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1,
    run_phase_stream_provider_export_acquisition_pack_v1,
    run_phase_stream_provider_export_evidence_chain_v1,
    run_phase_stream_real_traffic_action_family_online_discovery_v1,
    run_phase_stream_real_traffic_cost_evidence_audit_v1,
    run_phase_stream_real_traffic_cpu10_gap_audit_v1,
    run_phase_stream_real_traffic_frontier_union_v1,
    run_phase_stream_real_traffic_guarded_separator_calibrated_split_shadow_v1,
    run_phase_stream_real_traffic_guarded_separator_shadow_v1,
    run_phase_stream_real_traffic_guarded_separator_split_shadow_v1,
    run_phase_stream_real_traffic_mining_input_readiness_v1,
    run_phase_stream_real_traffic_online_discovery_v1,
    run_phase_stream_real_traffic_phase_atom_trace_v1,
    run_phase_stream_real_traffic_refined_online_discovery_v1,
    run_phase_stream_real_traffic_separator_audit_v1,
    run_phase_stream_real_traffic_shadow_request_gap_audit_v1,
    run_phase_stream_real_traffic_state_action_online_discovery_v1,
    run_phase_stream_real_traffic_token_cost_enrich_v1,
    run_phase_stream_selected_split_nwpc_admission_gate_v1,
    run_phase_stream_selected_split_nwpc_billing_request_v1,
    run_phase_stream_selected_split_nwpc_evidence_chain_audit_v1,
    run_phase_stream_selected_split_nwpc_loss_audit_v1,
    run_phase_stream_selected_split_nwpc_portfolio_select_v1,
    run_phase_stream_selected_split_nwpc_promotion_gate_v1,
    run_phase_stream_selected_split_nwpc_provider_export_admission_v1,
    run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1,
    run_phase_stream_selected_split_nwpc_provider_export_autoscan_v1,
    run_phase_stream_selected_split_nwpc_quarantine_v1,
    run_phase_stream_selected_split_nwpc_shadow_replay_v1,
    run_phase_stream_selected_split_nwpc_stage_filter_v1,
    run_phase_stream_test_output_parse_promotion_audit_v1, run_phase_stream_test_output_parse_v1,
    run_phase_stream_test_output_raw_log_trace_v1, run_phase_stream_verifier_evidence_join_v1,
};
use snapshot_io::{read_snapshot, save_snapshot};
use status::{print_organ128_plan, print_status, print_wave_tick};

fn main() -> ExitCode {
    let _artifact_budget = artifact_budget::ArtifactBudgetGuard::start();
    let mut args = std::env::args();
    let _bin = args.next();
    let command = args.next();

    match command.as_deref() {
        None | Some("status") => {
            print_status();
            ExitCode::SUCCESS
        }
        Some("organ128-plan") => {
            print_organ128_plan();
            ExitCode::SUCCESS
        }
        Some("organ128-train-generate") => match run_organ128_train_generate(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli organ128-train-generate [seed] [epochs] [prompt] [generate-len]"
                );
                ExitCode::FAILURE
            }
        },
        Some("organ128-dialog-generate") => match run_organ128_dialog_generate(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli organ128-dialog-generate [seed] [prompt]");
                ExitCode::FAILURE
            }
        },
        Some("organ128-settle-dialog") => match run_organ128_settle_dialog(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli organ128-settle-dialog [seed] [prompt] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("organ128-wave-scorer-eval") => match run_organ128_wave_scorer_eval(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli organ128-wave-scorer-eval [seed] [epochs] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("organ128-response-gate-eval") => match run_organ128_response_gate_eval(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli organ128-response-gate-eval [seed] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("organ128-thought-probe-eval") => match run_organ128_thought_probe_eval(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli organ128-thought-probe-eval [seed] [ticks] [epochs]");
                ExitCode::FAILURE
            }
        },
        Some("organ128-modadd-eval") => exit_for_result(
            run_organ128_modadd_eval(args),
            "try: nando-cli organ128-modadd-eval [seed] [modulus] [train-cases] [holdout-cases]",
        ),
        Some("organ128-modadd-seed-sweep") => exit_for_result(
            run_organ128_modadd_seed_sweep(args),
            "try: nando-cli organ128-modadd-seed-sweep [modulus] [train-cases] [holdout-cases]",
        ),
        Some("wave-tick") => match parse_wave_tick_args(args) {
            Ok((seed, input_byte)) => {
                print_wave_tick(seed, input_byte);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli wave-tick <input-byte> [seed]");
                ExitCode::FAILURE
            }
        },
        Some("snapshot-save") => match parse_snapshot_save_args(args) {
            Ok((seed, input_byte, path)) => match save_snapshot(seed, input_byte, &path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            },
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli snapshot-save <input-byte> [seed] [path]");
                ExitCode::FAILURE
            }
        },
        Some("snapshot-read") => match args.next() {
            Some(path) => match read_snapshot(&path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("missing snapshot path");
                eprintln!("try: nando-cli snapshot-read <path>");
                ExitCode::FAILURE
            }
        },
        Some("bench-stage2-tick") => match parse_bench_stage2_tick_args(args) {
            Ok((seed, ticks)) => {
                print_stage2_tick_bench(seed, ticks);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli bench-stage2-tick [seed] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("bench-link-tissue") => match parse_bench_stage2_tick_args(args) {
            Ok((seed, ticks)) => {
                print_link_tissue_bench(seed, ticks);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli bench-link-tissue [seed] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("bench-symbol-l3") => match parse_bench_stage2_tick_args(args) {
            Ok((seed, ticks)) => {
                print_symbol_l3_bench(seed, ticks);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli bench-symbol-l3 [seed] [ticks]");
                ExitCode::FAILURE
            }
        },
        Some("bench-wave-layers") => match print_wave_layer_metrics() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
        Some("phase-package-v4") => exit_for_result(
            run_phase_package_v4(args),
            "try: nando-cli phase-package-v4 [corpus-jsonl] [package-path] [cells] [manifest-path]",
        ),
        Some("phase-package-inspect") => exit_for_result(
            run_phase_package_inspect(args),
            "try: nando-cli phase-package-inspect [package-path] [manifest-path]",
        ),
        Some("phase-package-score-v4") => exit_for_result(
            run_phase_package_score_v4(args),
            "try: nando-cli phase-package-score-v4 [package-path] [manifest-path] [corpus-jsonl] [score-report-json]",
        ),
        Some("phase-eval-pack-v4") => exit_for_result(
            run_phase_eval_pack_v4(args),
            "try: nando-cli phase-eval-pack-v4 [package-path] [manifest-path] [corpus-jsonl] [eval-pack-path]",
        ),
        Some("phase-package-score-pack-v4") => exit_for_result(
            run_phase_package_score_pack_v4(args),
            "try: nando-cli phase-package-score-pack-v4 [package-path] [manifest-path] [eval-pack-path] [score-report-json]",
        ),
        Some("phase-action-boundary-v4") => exit_for_result(
            run_phase_action_boundary_v4(args),
            "try: nando-cli phase-action-boundary-v4 [corpus-jsonl]",
        ),
        Some("phase-action-corpus-v1") => exit_for_result(
            run_phase_action_corpus_v1(args),
            "try: nando-cli phase-action-corpus-v1 [output-jsonl] [report-json]",
        ),
        Some("phase-action-domain-corpus-v1") => exit_for_result(
            run_phase_action_domain_corpus_v1(args),
            "try: nando-cli phase-action-domain-corpus-v1 [output-jsonl] [report-json]",
        ),
        Some("phase-action-coverage-corpus-v1") => exit_for_result(
            run_phase_action_coverage_corpus_v1(args),
            "try: nando-cli phase-action-coverage-corpus-v1 [output-jsonl] [report-json]",
        ),
        Some("phase-action-contract-v1") => exit_for_result(
            run_phase_action_contract_v1(args),
            "try: nando-cli phase-action-contract-v1 [contract-jsonl] [report-json]",
        ),
        Some("phase-action-operator-coverage-v1") => exit_for_result(
            run_phase_action_operator_coverage_v1(args),
            "try: nando-cli phase-action-operator-coverage-v1 [contract-jsonl] [report-json]",
        ),
        Some("phase-action-shortcut-v1") => exit_for_result(
            run_phase_action_shortcut_v1(args),
            "try: nando-cli phase-action-shortcut-v1 [contract-jsonl] [report-json]",
        ),
        Some("phase-action-runtime-v1") => exit_for_result(
            run_phase_action_runtime_v1(args),
            "try: nando-cli phase-action-runtime-v1 [contract-jsonl] [cells] [report-json]",
        ),
        Some("phase-action-package-v1") => exit_for_result(
            run_phase_action_package_v1(args),
            "try: nando-cli phase-action-package-v1 [contract-jsonl] [package-path] [cells] [manifest-path]",
        ),
        Some("phase-action-package-inspect-v1") => exit_for_result(
            run_phase_action_package_inspect_v1(args),
            "try: nando-cli phase-action-package-inspect-v1 [package-path] [manifest-path]",
        ),
        Some("phase-action-source-verify-v1") => exit_for_result(
            run_phase_action_source_verify_v1(args),
            "try: nando-cli phase-action-source-verify-v1 [package-path] [manifest-path] [source-verify-report-json]",
        ),
        Some("phase-action-package-score-v1") => exit_for_result(
            run_phase_action_package_score_v1(args),
            "try: nando-cli phase-action-package-score-v1 [package-path] [manifest-path] [contract-jsonl] [score-report-json]",
        ),
        Some("phase-action-eval-pack-v1") => exit_for_result(
            run_phase_action_eval_pack_v1(args),
            "try: nando-cli phase-action-eval-pack-v1 [package-path] [manifest-path] [contract-jsonl] [eval-pack-path]",
        ),
        Some("phase-action-package-score-pack-v1") => exit_for_result(
            run_phase_action_package_score_pack_v1(args),
            "try: nando-cli phase-action-package-score-pack-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json]",
        ),
        Some("phase-action-package-bench-pack-v1") => exit_for_result(
            run_phase_action_package_bench_pack_v1(args),
            "try: nando-cli phase-action-package-bench-pack-v1 [package-path] [manifest-path] [eval-pack-path] [iterations] [bench-report-json]",
        ),
        Some("phase-action-package-bench-verify-v1") => exit_for_result(
            run_phase_action_package_bench_verify_v1(args),
            "try: nando-cli phase-action-package-bench-verify-v1 [package-path] [manifest-path] [eval-pack-path] [bench-report-json]",
        ),
        Some("phase-action-product-proof-v1") => exit_for_result(
            run_phase_action_product_proof_v1(args),
            "try: nando-cli phase-action-product-proof-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json] [bench-report-json] [product-proof-json]",
        ),
        Some("phase-action-product-verify-v1") => exit_for_result(
            run_phase_action_product_verify_v1(args),
            "try: nando-cli phase-action-product-verify-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json] [bench-report-json] [product-proof-json]",
        ),
        Some("phase-action-release-suite-v1") => exit_for_result(
            run_phase_action_release_suite_v1(args),
            "try: nando-cli phase-action-release-suite-v1 [release-suite-report-json]",
        ),
        Some("phase-action-release-verify-v1") => exit_for_result(
            run_phase_action_release_verify_v1(args),
            "try: nando-cli phase-action-release-verify-v1 [release-suite-report-json]",
        ),
        Some("phase-action-license-package-v1") => exit_for_result(
            run_phase_action_license_package_v1(args),
            "try: nando-cli phase-action-license-package-v1 [release-suite-report-json] [license-file] [license-package-report-json]",
        ),
        Some("phase-action-license-verify-v1") => exit_for_result(
            run_phase_action_license_verify_v1(args),
            "try: nando-cli phase-action-license-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json]",
        ),
        Some("phase-action-offload-audit-v1") => exit_for_result(
            run_phase_action_offload_audit_v1(args),
            "try: nando-cli phase-action-offload-audit-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [offload-audit-report-json]",
        ),
        Some("phase-action-offload-verify-v1") => exit_for_result(
            run_phase_action_offload_verify_v1(args),
            "try: nando-cli phase-action-offload-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json]",
        ),
        Some("phase-action-cache-offload-bench-v1") => exit_for_result(
            run_phase_action_cache_offload_bench_v1(args),
            "try: nando-cli phase-action-cache-offload-bench-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [cache-offload-bench-report-json]",
        ),
        Some("phase-action-cache-offload-bench-verify-v1") => exit_for_result(
            run_phase_action_cache_offload_bench_verify_v1(args),
            "try: nando-cli phase-action-cache-offload-bench-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [cache-offload-bench-report-json]",
        ),
        Some("phase-action-daemon-smoke-v1") => exit_for_result(
            run_phase_action_daemon_smoke_v1(args),
            "try: nando-cli phase-action-daemon-smoke-v1 [daemon-smoke-report-json]",
        ),
        Some("phase-action-daemon-package-smoke-v1") => exit_for_result(
            run_phase_action_daemon_package_smoke_v1(args),
            "try: nando-cli phase-action-daemon-package-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-package-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-hardening-smoke-v1") => exit_for_result(
            run_phase_action_daemon_hardening_smoke_v1(args),
            "try: nando-cli phase-action-daemon-hardening-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-hardening-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-auth-smoke-v1") => exit_for_result(
            run_phase_action_daemon_auth_smoke_v1(args),
            "try: nando-cli phase-action-daemon-auth-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-auth-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-registry-smoke-v1") => exit_for_result(
            run_phase_action_daemon_registry_smoke_v1(args),
            "try: nando-cli phase-action-daemon-registry-smoke-v1 [daemon-registry-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-registry-config-smoke-v1") => exit_for_result(
            run_phase_action_daemon_registry_config_smoke_v1(args),
            "try: nando-cli phase-action-daemon-registry-config-smoke-v1 [registry-config-json] [daemon-registry-config-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-config-validation-smoke-v1") => exit_for_result(
            run_phase_action_daemon_config_validation_smoke_v1(args),
            "try: nando-cli phase-action-daemon-config-validation-smoke-v1 [registry-config-json] [daemon-config-validation-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-rate-limit-smoke-v1") => exit_for_result(
            run_phase_action_daemon_rate_limit_smoke_v1(args),
            "try: nando-cli phase-action-daemon-rate-limit-smoke-v1 [registry-config-json] [daemon-rate-limit-smoke-report-json] [margin-threshold-micro] [max-score-requests]",
        ),
        Some("phase-action-daemon-observability-smoke-v1") => exit_for_result(
            run_phase_action_daemon_observability_smoke_v1(args),
            "try: nando-cli phase-action-daemon-observability-smoke-v1 [registry-config-json] [daemon-observability-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-audit-log-smoke-v1") => exit_for_result(
            run_phase_action_daemon_audit_log_smoke_v1(args),
            "try: nando-cli phase-action-daemon-audit-log-smoke-v1 [registry-config-json] [audit-log-jsonl] [daemon-audit-log-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-error-taxonomy-smoke-v1") => exit_for_result(
            run_phase_action_daemon_error_taxonomy_smoke_v1(args),
            "try: nando-cli phase-action-daemon-error-taxonomy-smoke-v1 [registry-config-json] [daemon-error-taxonomy-smoke-report-json] [margin-threshold-micro]",
        ),
        Some("phase-action-daemon-proof-suite-v1") => exit_for_result(
            run_phase_action_daemon_proof_suite_v1(args),
            "try: nando-cli phase-action-daemon-proof-suite-v1 [daemon-proof-suite-report-json]",
        ),
        Some("phase-action-daemon-live-proof-suite-v1") => exit_for_result(
            run_phase_action_daemon_live_proof_suite_v1(args),
            "try: nando-cli phase-action-daemon-live-proof-suite-v1 [daemon-live-proof-suite-report-json]",
        ),
        Some("phase-action-daemon-systemd-smoke-v1") => exit_for_result(
            run_phase_action_daemon_systemd_smoke_v1(args),
            "try: nando-cli phase-action-daemon-systemd-smoke-v1 [service-unit] [env-file] [registry-config-json] [daemon-systemd-smoke-report-json]",
        ),
        Some("phase-action-daemon-deployment-package-v1") => exit_for_result(
            run_phase_action_daemon_deployment_package_v1(args),
            "try: nando-cli phase-action-daemon-deployment-package-v1 [daemon-live-proof-suite-report-json] [daemon-systemd-smoke-report-json] [daemon-deployment-package-report-json]",
        ),
        Some("phase-action-daemon-deployment-verify-v1") => exit_for_result(
            run_phase_action_daemon_deployment_verify_v1(args),
            "try: nando-cli phase-action-daemon-deployment-verify-v1 [daemon-live-proof-suite-report-json] [daemon-systemd-smoke-report-json] [daemon-deployment-package-report-json]",
        ),
        Some("phase-action-daemon-serve-registry-v1") => exit_for_result(
            run_phase_action_daemon_serve_registry_v1(args),
            "try: nando-cli phase-action-daemon-serve-registry-v1 [registry-config-json] [bind-addr] [margin-threshold-micro] [auth-token] [max-score-requests] [audit-log-jsonl]",
        ),
        Some("phase-action-daemon-serve-v1") => exit_for_result(
            run_phase_action_daemon_serve_v1(args),
            "try: nando-cli phase-action-daemon-serve-v1 [package-path] [bind-addr] [margin-threshold-micro] [auth-token] [max-score-requests] [audit-log-jsonl]",
        ),
        Some("phase-action-workflow-bench-v1") => exit_for_result(
            run_phase_action_workflow_bench_v1(args),
            "try: nando-cli phase-action-workflow-bench-v1 [release-suite-report-json] [license-file] [license-package-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json]",
        ),
        Some("phase-action-workflow-bench-verify-v1") => exit_for_result(
            run_phase_action_workflow_bench_verify_v1(args),
            "try: nando-cli phase-action-workflow-bench-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json]",
        ),
        Some("phase-action-workflow-replay-v1") => exit_for_result(
            run_phase_action_workflow_replay_v1(args),
            "try: nando-cli phase-action-workflow-replay-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [workflow-sessions] [steps-per-session] [workflow-replay-report-json]",
        ),
        Some("phase-action-workflow-replay-verify-v1") => exit_for_result(
            run_phase_action_workflow_replay_verify_v1(args),
            "try: nando-cli phase-action-workflow-replay-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [workflow-sessions] [steps-per-session] [workflow-replay-report-json]",
        ),
        Some("phase-action-regression-v1") => exit_for_result(
            run_phase_action_regression_v1(args),
            "try: nando-cli phase-action-regression-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]",
        ),
        Some("phase-action-regression-verify-v1") => exit_for_result(
            run_phase_action_regression_verify_v1(args),
            "try: nando-cli phase-action-regression-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]",
        ),
        Some("phase-action-regression-freeze-v1") => exit_for_result(
            run_phase_action_regression_freeze_v1(args),
            "try: nando-cli phase-action-regression-freeze-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [regression-freeze-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]",
        ),
        Some("phase-action-regression-freeze-verify-v1") => exit_for_result(
            run_phase_action_regression_freeze_verify_v1(args),
            "try: nando-cli phase-action-regression-freeze-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [regression-freeze-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]",
        ),
        Some("phase-stream-test-output-parse-v1") => exit_for_result(
            run_phase_stream_test_output_parse_v1(args),
            "try: nando-cli phase-stream-test-output-parse-v1 [trace-jsonl] [shadow-report-json] [cells] [candidate-package-path]",
        ),
        Some("phase-stream-test-output-raw-log-trace-v1") => exit_for_result(
            run_phase_stream_test_output_raw_log_trace_v1(args),
            "try: nando-cli phase-stream-test-output-raw-log-trace-v1 [trace-jsonl] [trace-report-json] [raw-log ...]",
        ),
        Some("phase-stream-discovery-v1") => exit_for_result(
            run_phase_stream_discovery_v1(args),
            "try: nando-cli phase-stream-discovery-v1 [report-json] [candidate-dir] [cells] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-online-discovery-v1") => exit_for_result(
            run_phase_stream_online_discovery_v1(args),
            "try: nando-cli phase-stream-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-daemon-v1") => exit_for_result(
            run_phase_stream_online_miner_daemon_v1(args),
            "try: nando-cli phase-stream-online-miner-daemon-v1 [report-json] [checkpoint-dir] [decision-log-jsonl] [cells] [min-bucket-events] [base-margin-floor-micro] [compile-every-rows] [max-active-buckets] [reservoir-per-label] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-value-pass-v1") => exit_for_result(
            run_phase_stream_online_miner_value_pass_v1(args),
            "try: nando-cli phase-stream-online-miner-value-pass-v1 [report-json] [top-k] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-targeted-shadow-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_shadow_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-shadow-v1 [report-json] [checkpoint-dir] [cells] [top-k] [train-permille] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-targeted-rejection-drilldown-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_rejection_drilldown_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-rejection-drilldown-v1 [report-json] [value-pass-report-json] [targeted-shadow-report-json] [promotion-registry-gate-report-json]",
        ),
        Some("phase-stream-online-miner-targeted-split-refinement-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_split_refinement_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-split-refinement-v1 [report-json] [candidate-jsonl] [rejection-drilldown-report-json] [targeted-shadow-report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-targeted-aggregate-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_aggregate_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-aggregate-gate-v1 [report-json] [accepted-events-jsonl] [targeted-shadow-report-json] [promotion-registry-gate-report-json] [split-shadow-replay-report-json]",
        ),
        Some("phase-stream-online-miner-targeted-aggregate-billing-request-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_aggregate_billing_request_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-aggregate-billing-request-v1 [report-json] [billing-request-jsonl] [targeted-aggregate-report-json]",
        ),
        Some("phase-stream-online-miner-targeted-aggregate-admission-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-aggregate-admission-gate-v1 [report-json] [targeted-aggregate-report-json] [aggregate-billing-request-report-json] [billing-evidence-gate-report-json]",
        ),
        Some("phase-stream-online-miner-targeted-aggregate-provider-export-acquisition-pack-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1(args),
                "try: nando-cli phase-stream-online-miner-targeted-aggregate-provider-export-acquisition-pack-v1 [report-json] [output-dir] [targeted-aggregate-report-json]",
            )
        }
        Some("phase-stream-online-miner-targeted-aggregate-provider-export-admission-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1(args),
                "try: nando-cli phase-stream-online-miner-targeted-aggregate-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [targeted-aggregate-report-json]",
            )
        }
        Some("phase-stream-online-miner-targeted-aggregate-provider-export-attestation-contract-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1(args),
                "try: nando-cli phase-stream-online-miner-targeted-aggregate-provider-export-attestation-contract-v1 [report-json] <provider-export-jsonl> [attestation-template-json]",
            )
        }
        Some("phase-stream-online-miner-targeted-aggregate-provider-export-autoscan-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_targeted_aggregate_provider_export_autoscan_v1(args),
                "try: nando-cli phase-stream-online-miner-targeted-aggregate-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [targeted-aggregate-report-json]",
            )
        }
        Some("phase-stream-online-miner-promotion-registry-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_promotion_registry_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-promotion-registry-gate-v1 [report-json] [shadow-registry-dir] [product-hot-promotion-registry-json]",
        ),
        Some("phase-stream-online-miner-promotion-billing-request-v1") => exit_for_result(
            run_phase_stream_online_miner_promotion_billing_request_v1(args),
            "try: nando-cli phase-stream-online-miner-promotion-billing-request-v1 [report-json] [billing-request-jsonl] [promotion-registry-gate-report-json] [decision-log-jsonl]",
        ),
        Some("phase-stream-online-miner-targeted-billing-request-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_billing_request_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-billing-request-v1 [report-json] [billing-request-jsonl] [targeted-shadow-report-json] [targeted-decision-log-jsonl]",
        ),
        Some("phase-stream-online-miner-targeted-admission-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_targeted_admission_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-targeted-admission-gate-v1 [report-json] [targeted-shadow-report-json] [promotion-registry-gate-report-json] [billing-evidence-gate-report-json] [provider-coverage-report-json]",
        ),
        Some("phase-stream-online-miner-promotion-provider-capture-request-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_promotion_provider_capture_request_v1(args),
                "try: nando-cli phase-stream-online-miner-promotion-provider-capture-request-v1 [report-json] [capture-request-jsonl] [billing-request-jsonl]",
            )
        }
        Some("phase-stream-opportunity-board-v1") => exit_for_result(
            run_phase_stream_opportunity_board_v1(args),
            "try: nando-cli phase-stream-opportunity-board-v1 [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-constrained-split-miner-v1") => exit_for_result(
            run_phase_stream_constrained_split_miner_v1(args),
            "try: nando-cli phase-stream-constrained-split-miner-v1 [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-automatic-continuation-split-v1") => exit_for_result(
            run_phase_stream_automatic_continuation_split_v1(args),
            "try: nando-cli phase-stream-automatic-continuation-split-v1 [report-json] [selected-split-report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-verifier-evidence-join-v1") => exit_for_result(
            run_phase_stream_verifier_evidence_join_v1(args),
            "try: nando-cli phase-stream-verifier-evidence-join-v1 [report-json] [output-jsonl] [base-phase-atom-trace-jsonl] [verifier-evidence-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-trace-sample-v1") => exit_for_result(
            run_phase_stream_phase_atom_trace_sample_v1(args),
            "try: nando-cli phase-stream-phase-atom-trace-sample-v1 [report-json] [output-jsonl] [sample-modulus] [sample-remainder] [--keep-verified-safe] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-selected-split-nwpc-quarantine-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_quarantine_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-quarantine-v1 [report-json] [package-dir] [cells] [selected-split-report-json] [--hash-train-future] [--auto-multi-split] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-selected-split-nwpc-promotion-gate-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_promotion_gate_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-promotion-gate-v1 [report-json] [shadow-registry-dir] [quarantine-report-json]",
        ),
        Some("phase-stream-selected-split-nwpc-shadow-replay-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_shadow_replay_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-shadow-replay-v1 [report-json] [promotion-gate-report-json] [--hash-train-future] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-selected-split-nwpc-portfolio-select-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_portfolio_select_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-portfolio-select-v1 [report-json] [portfolio-promotion-report-json] [shadow-replay-report-json ...]",
        ),
        Some("phase-stream-selected-split-nwpc-billing-request-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_billing_request_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-billing-request-v1 [report-json] [billing-request-jsonl] [shadow-replay-report-json]",
        ),
        Some("phase-stream-selected-split-nwpc-admission-gate-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_admission_gate_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-admission-gate-v1 [report-json] [shadow-replay-report-json] [billing-request-report-json] [billing-evidence-gate-report-json]",
        ),
        Some("phase-stream-selected-split-nwpc-provider-export-admission-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_provider_export_admission_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [shadow-replay-report-json] [billing-request-report-json] [billing-request-jsonl]",
        ),
        Some("phase-stream-selected-split-nwpc-provider-export-attestation-contract-v1") => {
            exit_for_result(
                run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1(args),
                "try: nando-cli phase-stream-selected-split-nwpc-provider-export-attestation-contract-v1 [report-json] <provider-export-jsonl> [attestation-template-json]",
            )
        }
        Some("phase-stream-selected-split-nwpc-provider-export-autoscan-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_provider_export_autoscan_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [shadow-replay-report-json] [billing-request-report-json] [billing-request-jsonl]",
        ),
        Some("phase-stream-selected-split-nwpc-evidence-chain-audit-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_evidence_chain_audit_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-evidence-chain-audit-v1 [report-json] [quarantine-report-json] [promotion-report-json] [shadow-replay-report-json] [billing-request-report-json] [admission-report-json] [provider-export-admission-report-json]",
        ),
        Some("phase-stream-selected-split-nwpc-loss-audit-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_loss_audit_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-loss-audit-v1 [report-json] [selected-split-report-json] [quarantine-report-json] [shadow-replay-report-json]",
        ),
        Some("phase-stream-selected-split-nwpc-stage-filter-v1") => exit_for_result(
            run_phase_stream_selected_split_nwpc_stage_filter_v1(args),
            "try: nando-cli phase-stream-selected-split-nwpc-stage-filter-v1 [report-json] [filtered-selected-split-report-json] [selected-split-report-json] [quarantine-report-json ...]",
        ),
        Some("phase-stream-live-store-adapter-smoke-v1") => exit_for_result(
            run_phase_stream_live_store_adapter_smoke_v1(args),
            "try: nando-cli phase-stream-live-store-adapter-smoke-v1 [report-json] [cells] [min-bucket-events] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-live-store-clean-manifest-shadow-v1") => exit_for_result(
            run_phase_stream_live_store_clean_manifest_shadow_v1(args),
            "try: nando-cli phase-stream-live-store-clean-manifest-shadow-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-live-store-clean-manifest-admission-gate-v1") => exit_for_result(
            run_phase_stream_live_store_clean_manifest_admission_gate_v1(args),
            "try: nando-cli phase-stream-live-store-clean-manifest-admission-gate-v1 [report-json] [manifest-json] [shadow-report-json] [prepared-hot-pack-report-json]",
        ),
        Some("phase-stream-live-store-clean-manifest-live-policy-stage-v1") => exit_for_result(
            run_phase_stream_live_store_clean_manifest_live_policy_stage_v1(args),
            "try: nando-cli phase-stream-live-store-clean-manifest-live-policy-stage-v1 [report-json] [policy-json] [clean-manifest-admission-report-json]",
        ),
        Some("phase-stream-live-store-clean-manifest-live-policy-shadow-review-v1") => {
            exit_for_result(
                run_phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1(args),
                "try: nando-cli phase-stream-live-store-clean-manifest-live-policy-shadow-review-v1 [report-json] [stage-report-json] [policy-json] [live-source-worker-report-json]",
            )
        }
        Some("phase-stream-live-store-clean-manifest-prepared-policy-shadow-review-v1") => {
            exit_for_result(
                run_phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1(args),
                "try: nando-cli phase-stream-live-store-clean-manifest-prepared-policy-shadow-review-v1 [report-json] [stage-report-json] [policy-json] [prepared-hot-pack-report-json] [memory-worker-report-json]",
            )
        }
        Some("phase-stream-live-store-clean-manifest-shadow-registry-handoff-v1") => {
            exit_for_result(
                run_phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1(args),
                "try: nando-cli phase-stream-live-store-clean-manifest-shadow-registry-handoff-v1 [report-json] [shadow-registry-dir] [prepared-policy-shadow-review-report-json]",
            )
        }
        Some("phase-stream-live-store-clean-manifest-shadow-registry-replay-v1") => {
            exit_for_result(
                run_phase_stream_live_store_clean_manifest_shadow_registry_replay_v1(args),
                "try: nando-cli phase-stream-live-store-clean-manifest-shadow-registry-replay-v1 [report-json] [shadow-registry-handoff-report-json] [prepared-hot-pack-json]",
            )
        }
        Some("phase-stream-live-store-clean-manifest-shadow-registry-billing-request-v1") => {
            exit_for_result(
                run_phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1(args),
                "try: nando-cli phase-stream-live-store-clean-manifest-shadow-registry-billing-request-v1 [report-json] [billing-request-jsonl] [shadow-registry-replay-report-json] [prepared-hot-pack-json] [correlation-sidecar-jsonl]",
            )
        }
        Some("phase-stream-live-store-prepared-hot-pack-v1") => exit_for_result(
            run_phase_stream_live_store_prepared_hot_pack_v1(args),
            "try: nando-cli phase-stream-live-store-prepared-hot-pack-v1 [manifest-json] [pack-json] [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-live-store-prepared-hot-pack-correlation-sidecar-v1") => {
            exit_for_result(
                run_phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1(args),
                "try: nando-cli phase-stream-live-store-prepared-hot-pack-correlation-sidecar-v1 [report-json] [sidecar-jsonl] [prepared-hot-pack-json] [phase-atom-trace-jsonl ...]",
            )
        }
        Some("phase-stream-live-worker-memory-smoke-v1") => exit_for_result(
            run_phase_stream_live_worker_memory_smoke_v1(args),
            "try: nando-cli phase-stream-live-worker-memory-smoke-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-live-source-adapter-worker-v1") => exit_for_result(
            run_phase_stream_live_source_adapter_worker_v1(args),
            "try: nando-cli phase-stream-live-source-adapter-worker-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-live-worker-queue-smoke-v1") => exit_for_result(
            run_phase_stream_live_worker_queue_smoke_v1(args),
            "try: nando-cli phase-stream-live-worker-queue-smoke-v1 [manifest-json] [report-json] [queue-batch-capacity] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-live-worker-thread-smoke-v1") => exit_for_result(
            run_phase_stream_live_worker_thread_smoke_v1(args),
            "try: nando-cli phase-stream-live-worker-thread-smoke-v1 [manifest-json] [report-json] [channel-capacity] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-live-worker-batch-thread-smoke-v1") => exit_for_result(
            run_phase_stream_live_worker_batch_thread_smoke_v1(args),
            "try: nando-cli phase-stream-live-worker-batch-thread-smoke-v1 [manifest-json] [report-json] [channel-capacity] [source-batch-capacity] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-live-store-direct-batch-thread-smoke-v1") => exit_for_result(
            run_phase_stream_live_store_direct_batch_thread_smoke_v1(args),
            "try: nando-cli phase-stream-live-store-direct-batch-thread-smoke-v1 [report-json] [channel-capacity] [source-batch-capacity] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-benchmark-v1") => exit_for_result(
            run_phase_stream_hot_path_benchmark_v1(args),
            "try: nando-cli phase-stream-hot-path-benchmark-v1 [report-json] [timed-score-iterations] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-admission-policy-smoke-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_admission_policy_smoke_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-admission-policy-smoke-v1 [daemon-policy-json] [policy-smoke-report-json]",
        ),
        Some("phase-stream-hot-path-daemon-shadow-gate-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_shadow_gate_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-shadow-gate-v1 [policy-smoke-json] [shadow-report-json] [decision-log-jsonl] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-append-shadow-gate-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_append_shadow_gate_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-append-shadow-gate-v1 [policy-smoke-json] [append-shadow-report-json] [decision-log-jsonl] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-live-loop-budget-smoke-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_live_loop_budget_smoke_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-live-loop-budget-smoke-v1 [report-json] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-append-live-loop-smoke-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_append_live_loop_smoke_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-append-live-loop-smoke-v1 [report-json] [decision-log-jsonl] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-append-live-tail-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_append_live_tail_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-append-live-tail-v1 [report-json] [decision-log-jsonl] [cells] [min-bucket-events] [idle-sleep-ms] [max-idle-ms] [max-append-events] [watermark-trace-jsonl] [append-tail-jsonl] [product-hot-registry-json]",
        ),
        Some("phase-stream-hot-path-daemon-live-loop-numeric-benchmark-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-live-loop-numeric-benchmark-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-numeric-package-shadow-audit-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-numeric-package-shadow-audit-v1 [numeric-report-json] [audit-report-json] [candidate-index]",
        ),
        Some("phase-stream-hot-path-daemon-numeric-future-package-audit-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_numeric_future_package_audit_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-numeric-future-package-audit-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1") => exit_for_result(
            run_phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1(args),
            "try: nando-cli phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]",
        ),
        Some("phase-stream-hot-path-daemon-numeric-admission-portfolio-gate-v1") => {
            exit_for_result(
                run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1(args),
                "try: nando-cli phase-stream-hot-path-daemon-numeric-admission-portfolio-gate-v1 [portfolio-report-json] [future-audit-report-json ...]",
            )
        }
        Some("phase-stream-hot-path-daemon-numeric-admission-portfolio-runtime-replay-v1") => {
            exit_for_result(
                run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1(
                    args,
                ),
                "try: nando-cli phase-stream-hot-path-daemon-numeric-admission-portfolio-runtime-replay-v1 [portfolio-gate-report-json] [runtime-replay-report-json]",
            )
        }
        Some("phase-stream-hot-path-daemon-numeric-false-accept-split-audit-v1") => {
            exit_for_result(
                run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1(args),
                "try: nando-cli phase-stream-hot-path-daemon-numeric-false-accept-split-audit-v1 [future-audit-report-json] [split-report-json] [top-k]",
            )
        }
        Some("phase-stream-real-traffic-online-discovery-v1") => exit_for_result(
            run_phase_stream_real_traffic_online_discovery_v1(args),
            "try: nando-cli phase-stream-real-traffic-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-refined-online-discovery-v1") => exit_for_result(
            run_phase_stream_real_traffic_refined_online_discovery_v1(args),
            "try: nando-cli phase-stream-real-traffic-refined-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-action-family-online-discovery-v1") => exit_for_result(
            run_phase_stream_real_traffic_action_family_online_discovery_v1(args),
            "try: nando-cli phase-stream-real-traffic-action-family-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-state-action-online-discovery-v1") => exit_for_result(
            run_phase_stream_real_traffic_state_action_online_discovery_v1(args),
            "try: nando-cli phase-stream-real-traffic-state-action-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-frontier-union-v1") => exit_for_result(
            run_phase_stream_real_traffic_frontier_union_v1(args),
            "try: nando-cli phase-stream-real-traffic-frontier-union-v1 [union-report-json] [online-discovery-report-json ...]",
        ),
        Some("phase-stream-real-traffic-cpu10-gap-audit-v1") => exit_for_result(
            run_phase_stream_real_traffic_cpu10_gap_audit_v1(args),
            "try: nando-cli phase-stream-real-traffic-cpu10-gap-audit-v1 [gap-report-json] [frontier-union-report-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-shadow-request-gap-audit-v1") => exit_for_result(
            run_phase_stream_real_traffic_shadow_request_gap_audit_v1(args),
            "try: nando-cli phase-stream-real-traffic-shadow-request-gap-audit-v1 [gap-report-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-mining-input-readiness-v1") => exit_for_result(
            run_phase_stream_real_traffic_mining_input_readiness_v1(args),
            "try: nando-cli phase-stream-real-traffic-mining-input-readiness-v1 [readiness-report-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-phase-atom-trace-v1") => exit_for_result(
            run_phase_stream_real_traffic_phase_atom_trace_v1(args),
            "try: nando-cli phase-stream-real-traffic-phase-atom-trace-v1 [report-json] [output-jsonl] [trace-jsonl ...]",
        ),
        Some("phase-stream-codex-history-phase-atom-trace-v1") => exit_for_result(
            run_phase_stream_codex_history_phase_atom_trace_v1(args),
            "try: nando-cli phase-stream-codex-history-phase-atom-trace-v1 [report-json] [output-jsonl] [history-jsonl] [max-rows]",
        ),
        Some("phase-stream-phase-atom-verifier-needed-ranking-v1") => exit_for_result(
            run_phase_stream_phase_atom_verifier_needed_ranking_v1(args),
            "try: nando-cli phase-stream-phase-atom-verifier-needed-ranking-v1 [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-agent-continue-active-turn-state-v1") => exit_for_result(
            run_phase_stream_agent_continue_active_turn_state_v1(args),
            "try: nando-cli phase-stream-agent-continue-active-turn-state-v1 [report-json] [output-jsonl] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-agent-continue-command-result-followup-pack-v1") => exit_for_result(
            run_phase_stream_agent_continue_command_result_followup_pack_v1(args),
            "try: nando-cli phase-stream-agent-continue-command-result-followup-pack-v1 [report-json] [output-jsonl] [tool-status-phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-agent-continue-subroute-scoreboard-v1") => exit_for_result(
            run_phase_stream_agent_continue_subroute_scoreboard_v1(args),
            "try: nando-cli phase-stream-agent-continue-subroute-scoreboard-v1 [report-json] [agent-continue-active-turn-jsonl]",
        ),
        Some("phase-stream-auto-subcenter-discovery-v1") => exit_for_result(
            run_phase_stream_auto_subcenter_discovery_v1(args),
            "try: nando-cli phase-stream-auto-subcenter-discovery-v1 [report-json] [candidate-trace-jsonl] [rejections-jsonl] [max-selected-candidates] [max-positive-rows-per-candidate] [background-rows-per-positive] [agent-continue-phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-codex-session-run-check-verifier-trace-v1") => exit_for_result(
            run_phase_stream_codex_session_run_check_verifier_trace_v1(args),
            "try: nando-cli phase-stream-codex-session-run-check-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]",
        ),
        Some("phase-stream-codex-session-planning-verifier-trace-v1") => exit_for_result(
            run_phase_stream_codex_session_planning_verifier_trace_v1(args),
            "try: nando-cli phase-stream-codex-session-planning-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]",
        ),
        Some("phase-stream-codex-session-tool-status-verifier-trace-v1") => exit_for_result(
            run_phase_stream_codex_session_tool_status_verifier_trace_v1(args),
            "try: nando-cli phase-stream-codex-session-tool-status-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]",
        ),
        Some("phase-stream-codex-session-live-append-v1") => exit_for_result(
            run_phase_stream_codex_session_live_append_v1(args),
            "try: nando-cli phase-stream-codex-session-live-append-v1 [report-json] [append-jsonl] [session-jsonl] [poll-ms] [max-idle-ms] [max-rows]",
        ),
        Some("phase-stream-codex-sessions-live-append-v1") => exit_for_result(
            run_phase_stream_codex_sessions_live_append_v1(args),
            "try: nando-cli phase-stream-codex-sessions-live-append-v1 [report-json] [append-jsonl] [sessions-dir] [poll-ms] [max-idle-ms] [max-rows] [max-recent-files]",
        ),
        Some("phase-stream-phase-atom-run-check-discovery-v1") => exit_for_result(
            run_phase_stream_phase_atom_run_check_discovery_v1(args),
            "try: nando-cli phase-stream-phase-atom-run-check-discovery-v1 [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-run-check-time-split-discovery-v1") => exit_for_result(
            run_phase_stream_phase_atom_run_check_time_split_discovery_v1(args),
            "try: nando-cli phase-stream-phase-atom-run-check-time-split-discovery-v1 [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [train-permille] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-action-family-time-split-discovery-v1") => exit_for_result(
            run_phase_stream_phase_atom_action_family_time_split_discovery_v1(args),
            "try: nando-cli phase-stream-phase-atom-action-family-time-split-discovery-v1 [action-family] [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [train-permille] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-action-family-separability-audit-v1") => exit_for_result(
            run_phase_stream_phase_atom_action_family_separability_audit_v1(args),
            "try: nando-cli phase-stream-phase-atom-action-family-separability-audit-v1 [action-family-or-bucket] [report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-run-check-time-split-promotion-audit-v1") => exit_for_result(
            run_phase_stream_phase_atom_run_check_time_split_promotion_audit_v1(args),
            "try: nando-cli phase-stream-phase-atom-run-check-time-split-promotion-audit-v1 [discovery-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]",
        ),
        Some("phase-stream-phase-atom-action-family-time-split-promotion-audit-v1") => {
            exit_for_result(
                run_phase_stream_phase_atom_run_check_time_split_promotion_audit_v1(args),
                "try: nando-cli phase-stream-phase-atom-action-family-time-split-promotion-audit-v1 [discovery-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]",
            )
        }
        Some("phase-stream-phase-atom-action-family-serving-admission-audit-v1") => {
            exit_for_result(
                run_phase_stream_phase_atom_action_family_serving_admission_audit_v1(args),
                "try: nando-cli phase-stream-phase-atom-action-family-serving-admission-audit-v1 [promotion-audit-json] [admission-report-json] [candidate-package-path] [margin-threshold-micro] [model-price-config-json] [phase-atom-trace-jsonl ...]",
            )
        }
        Some("phase-stream-phase-atom-serving-shadow-replay-v1") => exit_for_result(
            run_phase_stream_phase_atom_serving_shadow_replay_v1(args),
            "try: nando-cli phase-stream-phase-atom-serving-shadow-replay-v1 [shadow-report-json] [phase-atom-trace-jsonl] [serving-admission-report-json ...]",
        ),
        Some("phase-stream-phase-atom-serving-future-shadow-replay-v1") => exit_for_result(
            run_phase_stream_phase_atom_serving_future_shadow_replay_v1(args),
            "try: nando-cli phase-stream-phase-atom-serving-future-shadow-replay-v1 [shadow-report-json] [phase-atom-trace-jsonl] [serving-admission-report-json ...]",
        ),
        Some("phase-stream-phase-atom-serving-append-shadow-replay-v1") => exit_for_result(
            run_phase_stream_phase_atom_serving_append_shadow_replay_v1(args),
            "try: nando-cli phase-stream-phase-atom-serving-append-shadow-replay-v1 [shadow-report-json] [watermark-trace-jsonl] [append-trace-jsonl] [serving-admission-report-json ...]",
        ),
        Some("phase-stream-phase-atom-live-admission-manifest-v1") => exit_for_result(
            run_phase_stream_phase_atom_live_admission_manifest_v1(args),
            "try: nando-cli phase-stream-phase-atom-live-admission-manifest-v1 [serving-admission-report-json] [shadow-replay-report-json] [manifest-report-json]",
        ),
        Some("phase-stream-phase-atom-live-admission-policy-smoke-v1") => exit_for_result(
            run_phase_stream_phase_atom_live_admission_policy_smoke_v1(args),
            "try: nando-cli phase-stream-phase-atom-live-admission-policy-smoke-v1 [manifest-report-json] [policy-smoke-report-json]",
        ),
        Some("phase-stream-phase-atom-live-daemon-shadow-gate-v1") => exit_for_result(
            run_phase_stream_phase_atom_live_daemon_shadow_gate_v1(args),
            "try: nando-cli phase-stream-phase-atom-live-daemon-shadow-gate-v1 [policy-smoke-report-json] [live-trace-jsonl] [decision-log-jsonl] [gate-report-json] [exact-cache-watermark-trace-jsonl]",
        ),
        Some("phase-stream-phase-atom-live-self-mining-loop-v1") => exit_for_result(
            run_phase_stream_phase_atom_live_self_mining_loop_v1(args),
            "try: nando-cli phase-stream-phase-atom-live-self-mining-loop-v1 [report-json] [candidate-dir] [cells] [min-class-events] [margin-threshold-micro] [train-permille] [top-n] [model-price-config-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-global-denominator-compressibility-audit-v1") => exit_for_result(
            run_phase_stream_global_denominator_compressibility_audit_v1(args),
            "try: nando-cli phase-stream-global-denominator-compressibility-audit-v1 [report-json] [current5k-feedback-report-json] [phase-center-self-mining-report-json] [global-phase-atom-trace-jsonl] [mining-phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-compatible-denominator-shadow-v1") => exit_for_result(
            run_phase_stream_phase_atom_compatible_denominator_shadow_v1(args),
            "try: nando-cli phase-stream-phase-atom-compatible-denominator-shadow-v1 [report-json] [decision-log-jsonl] [self-mining-report-json] [compatible-phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-market-money-claim-gate-v1") => exit_for_result(
            run_phase_stream_phase_atom_market_money_claim_gate_v1(args),
            "try: nando-cli phase-stream-phase-atom-market-money-claim-gate-v1 [report-json] [compatible-shadow-report-json] [cost-audit-report-json] [model-price-config-json] [provider-billing-evidence-json]",
        ),
        Some("phase-stream-provider-billing-evidence-join-v1") => exit_for_result(
            run_phase_stream_provider_billing_evidence_join_v1(args),
            "try: nando-cli phase-stream-provider-billing-evidence-join-v1 [report-json] <provider-billing-jsonl> [output-dir] [trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-portfolio-selector-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_selector_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-selector-v1 [report-json] [online-miner-report-json] [decision-log-jsonl] [max-selected-buckets]  # legacy baseline/debug only",
        ),
        Some("phase-stream-online-miner-portfolio-np-rescue-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_np_rescue_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-np-rescue-v1 [report-json] [selector-report-json] [decision-log-jsonl] [max-selected-subcenters] [trace-jsonl ...]",
        ),
        Some("phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1 [report-json] [np-rescue-report-json]",
        ),
        Some("phase-stream-online-miner-portfolio-runtime-replay-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_runtime_replay_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-runtime-replay-v1 [report-json] [portfolio-selector-report-json]  # legacy baseline review only",
        ),
        Some("phase-stream-online-miner-portfolio-future-tail-replay-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_future_tail_replay_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-future-tail-replay-v1 [report-json] [portfolio-selector-report-json] [future-trace-jsonl] [min-future-row-index]",
        ),
        Some("phase-stream-online-miner-portfolio-live-tail-score-only-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_live_tail_score_only_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-live-tail-score-only-v1 [report-json] [decision-log-jsonl] [product-hot-registry-json] [append-tail-jsonl] [idle-sleep-ms] [max-idle-ms] [max-append-events]",
        ),
        Some("phase-stream-online-miner-portfolio-live-tail-billing-request-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_live_tail_billing_request_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-live-tail-billing-request-v1 [report-json] [billing-request-jsonl] [live-score-report-json] [decision-log-jsonl]",
            )
        }
        Some("phase-stream-online-miner-portfolio-clean-subset-manifest-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_clean_subset_manifest_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-clean-subset-manifest-v1 [report-json] [clean-selector-report-json] <future-tail-report-json>",
        ),
        Some("phase-stream-online-miner-portfolio-future-tail-billing-request-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_future_tail_billing_request_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-future-tail-billing-request-v1 [report-json] [billing-request-jsonl] <clean-future-tail-report-json>",
            )
        }
        Some("phase-stream-online-miner-portfolio-admission-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_admission_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-admission-gate-v1 [report-json] [runtime-replay-report-json] [provider-billing-evidence-join-report-json]",
        ),
        Some("phase-stream-online-miner-portfolio-billing-request-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_billing_request_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-billing-request-v1 [report-json] [billing-request-jsonl] [runtime-replay-report-json]",
        ),
        Some("phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1 [report-json] [output-billing-request-jsonl] <billing-request-jsonl> <provider-boundary-jsonl ...>",
        ),
        Some("phase-stream-online-miner-portfolio-selector-billing-request-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_selector_billing_request_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-selector-billing-request-v1 [report-json] [billing-request-jsonl] <selector-report-json>",
        ),
        Some("phase-stream-online-miner-portfolio-billing-evidence-gate-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-billing-evidence-gate-v1 [report-json] [billing-request-jsonl] <provider-billing-evidence-jsonl> [missing-request-jsonl]",
        ),
        Some("phase-stream-online-miner-portfolio-billing-evidence-contract-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_billing_evidence_contract_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-billing-evidence-contract-v1 [report-json] [billing-request-report-json] [template-jsonl]",
            )
        }
        Some("phase-stream-online-miner-portfolio-evidence-chain-audit-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-evidence-chain-audit-v1 [report-json] [runtime-replay-report-json] [billing-request-report-json] [billing-contract-report-json] [provider-normalize-report-json] [billing-evidence-gate-report-json] [admission-report-json] [promotion-report-json] [provider-correlation-audit-report-json]",
        ),
        Some("phase-stream-online-miner-portfolio-provider-export-admission-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_provider_export_admission_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]",
            )
        }
        Some("phase-stream-online-miner-portfolio-provider-export-autoscan-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]",
            )
        }
        Some("phase-stream-online-miner-portfolio-provider-export-watch-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_provider_export_watch_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-provider-export-watch-v1 [report-json] [scan-dir] [work-dir] [cycles] [sleep-ms] [max-evaluated-candidates] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]",
            )
        }
        Some("phase-stream-online-miner-portfolio-provider-correlation-audit-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-provider-correlation-audit-v1 [report-json] <jsonl ...>",
            )
        }
        Some("phase-stream-automatic-discovery-chain-gate-v1") => exit_for_result(
            run_phase_stream_automatic_discovery_chain_gate_v1(args),
            "try: nando-cli phase-stream-automatic-discovery-chain-gate-v1 [report-json] [capture-readiness-report-json] [selector-report-json] [runtime-replay-report-json]",
        ),
        Some("phase-stream-phase-atom-live-capture-readiness-v1") => exit_for_result(
            run_phase_stream_phase_atom_live_capture_readiness_v1(args),
            "try: nando-cli phase-stream-phase-atom-live-capture-readiness-v1 [report-json] <phase-atom-trace-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-phase-atom-trace-v1") => exit_for_result(
            run_phase_stream_provider_boundary_phase_atom_trace_v1(args),
            "try: nando-cli phase-stream-provider-boundary-phase-atom-trace-v1 [report-json] [output-jsonl] <provider-boundary-event-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-correlation-join-v1") => exit_for_result(
            run_phase_stream_provider_boundary_correlation_join_v1(args),
            "try: nando-cli phase-stream-provider-boundary-correlation-join-v1 [report-json] [output-jsonl] <phase-atom-trace-jsonl> <provider-boundary-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-match-readiness-v1") => exit_for_result(
            run_phase_stream_provider_boundary_match_readiness_v1(args),
            "try: nando-cli phase-stream-provider-boundary-match-readiness-v1 [report-json] <phase-atom-trace-jsonl ...> --provider <provider-boundary-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-capture-request-v1") => exit_for_result(
            run_phase_stream_provider_boundary_capture_request_v1(args),
            "try: nando-cli phase-stream-provider-boundary-capture-request-v1 [report-json] [output-jsonl] <phase-atom-trace-jsonl ...> [--provider <provider-boundary-jsonl ...>]",
        ),
        Some("phase-stream-provider-boundary-billing-capture-contract-v1") => exit_for_result(
            run_phase_stream_provider_boundary_billing_capture_contract_v1(args),
            "try: nando-cli phase-stream-provider-boundary-billing-capture-contract-v1 [report-json] [template-jsonl] [template-csv] <capture-request-jsonl>",
        ),
        Some("phase-stream-provider-boundary-billing-capture-evidence-gate-v1") => {
            exit_for_result(
                run_phase_stream_provider_boundary_billing_capture_evidence_gate_v1(args),
                "try: nando-cli phase-stream-provider-boundary-billing-capture-evidence-gate-v1 [report-json] <capture-request-jsonl> <filled-provider-evidence-jsonl> [missing-jsonl]",
            )
        }
        Some("phase-stream-provider-boundary-billing-capture-chain-v1") => exit_for_result(
            run_phase_stream_provider_boundary_billing_capture_chain_v1(args),
            "try: nando-cli phase-stream-provider-boundary-billing-capture-chain-v1 [report-json] [artifact-prefix] <capture-request-jsonl> <phase-atom-trace-jsonl ...> --provider-evidence <filled-provider-evidence-jsonl>",
        ),
        Some("phase-stream-provider-boundary-codex-token-backfill-v1") => exit_for_result(
            run_phase_stream_provider_boundary_codex_token_backfill_v1(args),
            "try: nando-cli phase-stream-provider-boundary-codex-token-backfill-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <phase-atom-trace-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-realtrace-token-cost-backfill-v1") => exit_for_result(
            run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1(args),
            "try: nando-cli phase-stream-provider-boundary-realtrace-token-cost-backfill-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <phase-atom-trace-jsonl ...>",
        ),
        Some("phase-stream-provider-export-acquisition-pack-v1") => exit_for_result(
            run_phase_stream_provider_export_acquisition_pack_v1(args),
            "try: nando-cli phase-stream-provider-export-acquisition-pack-v1 [report-json] [output-dir] [billing-request-jsonl]",
        ),
        Some("phase-stream-provider-export-evidence-chain-v1") => exit_for_result(
            run_phase_stream_provider_export_evidence_chain_v1(args),
            "try: nando-cli phase-stream-provider-export-evidence-chain-v1 [report-json] [work-dir] [billing-request-jsonl] [provider-boundary-capture-request-jsonl] [provider-export-jsonl]",
        ),
        Some("phase-stream-provider-boundary-capture-coverage-gate-v1") => exit_for_result(
            run_phase_stream_provider_boundary_capture_coverage_gate_v1(args),
            "try: nando-cli phase-stream-provider-boundary-capture-coverage-gate-v1 [report-json] <capture-request-jsonl> --provider <provider-boundary-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-export-ingest-v1") => exit_for_result(
            run_phase_stream_provider_boundary_export_ingest_v1(args),
            "try: nando-cli phase-stream-provider-boundary-export-ingest-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <provider-export-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-append-sink-v1") => exit_for_result(
            run_phase_stream_provider_boundary_append_sink_v1(args),
            "try: nando-cli phase-stream-provider-boundary-append-sink-v1 [report-json] [append-provider-boundary-jsonl] <provider-event-jsonl|- ...>",
        ),
        Some("phase-stream-provider-boundary-live-chain-v1") => exit_for_result(
            run_phase_stream_provider_boundary_live_chain_v1(args),
            "try: nando-cli phase-stream-provider-boundary-live-chain-v1 [report-json] [artifact-prefix] <capture-request-jsonl> <phase-atom-trace-jsonl ...> --provider-events <provider-event-jsonl|- ...>",
        ),
        Some("phase-stream-provider-boundary-live-np-chain-v1") => exit_for_result(
            run_phase_stream_provider_boundary_live_np_chain_v1(args),
            "try: nando-cli phase-stream-provider-boundary-live-np-chain-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <capture-request-jsonl> <score-ready-phase-atom-trace-jsonl> --provider-events <provider-event-jsonl|- ...>",
        ),
        Some("phase-stream-provider-boundary-np-chain-v1") => exit_for_result(
            run_phase_stream_provider_boundary_np_chain_v1(args),
            "try: nando-cli phase-stream-provider-boundary-np-chain-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <provider-boundary-event-jsonl ...>",
        ),
        Some("phase-stream-provider-boundary-np-chain-from-phase-trace-v1") => exit_for_result(
            run_phase_stream_provider_boundary_np_chain_from_phase_trace_v1(args),
            "try: nando-cli phase-stream-provider-boundary-np-chain-from-phase-trace-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <score-ready-phase-atom-trace-jsonl> <provider-boundary-event-jsonl ...>",
        ),
        Some("phase-stream-online-miner-portfolio-provider-export-normalize-v1") => {
            exit_for_result(
                run_phase_stream_online_miner_portfolio_provider_export_normalize_v1(args),
                "try: nando-cli phase-stream-online-miner-portfolio-provider-export-normalize-v1 [report-json] [billing-request-jsonl] <provider-export-jsonl> [normalized-evidence-jsonl]",
            )
        }
        Some("phase-stream-online-miner-portfolio-promotion-manifest-v1") => exit_for_result(
            run_phase_stream_online_miner_portfolio_promotion_manifest_v1(args),
            "try: nando-cli phase-stream-online-miner-portfolio-promotion-manifest-v1 [report-json] [admission-gate-report-json] [billing-contract-report-json]",
        ),
        Some("phase-stream-phase-atom-frontier-billing-request-v1") => exit_for_result(
            run_phase_stream_phase_atom_frontier_billing_request_v1(args),
            "try: nando-cli phase-stream-phase-atom-frontier-billing-request-v1 [report-json] [billing-request-jsonl] [frontier-shadow-replay-report-json]",
        ),
        Some("phase-stream-phase-atom-frontier-shadow-replay-v1") => exit_for_result(
            run_phase_stream_phase_atom_frontier_shadow_replay_v1(args),
            "try: nando-cli phase-stream-phase-atom-frontier-shadow-replay-v1 [report-json] [decision-log-jsonl] [frontier-union-report-json] [phase-atom-trace-jsonl ...]",
        ),
        Some("phase-stream-phase-atom-frontier-claim-audit-v1") => exit_for_result(
            run_phase_stream_phase_atom_frontier_claim_audit_v1(args),
            "try: nando-cli phase-stream-phase-atom-frontier-claim-audit-v1 [claim-audit-report-json] [frontier-shadow-replay-report-json]",
        ),
        Some("phase-stream-phase-atom-diversity-backlog-v1") => exit_for_result(
            run_phase_stream_phase_atom_diversity_backlog_v1(args),
            "try: nando-cli phase-stream-phase-atom-diversity-backlog-v1 [backlog-report-json] [claim-audit-report-json] [verifier-needed-ranking-json]",
        ),
        Some("phase-stream-real-traffic-separator-audit-v1") => exit_for_result(
            run_phase_stream_real_traffic_separator_audit_v1(args),
            "try: nando-cli phase-stream-real-traffic-separator-audit-v1 [report-json] [min-true-over-exact] [top-n] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-guarded-separator-shadow-v1") => exit_for_result(
            run_phase_stream_real_traffic_guarded_separator_shadow_v1(args),
            "try: nando-cli phase-stream-real-traffic-guarded-separator-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [max-guards] [separator-report-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-guarded-separator-split-shadow-v1") => exit_for_result(
            run_phase_stream_real_traffic_guarded_separator_split_shadow_v1(args),
            "try: nando-cli phase-stream-real-traffic-guarded-separator-split-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [max-guards] [selector-permille] [train-permille] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-guarded-separator-calibrated-split-shadow-v1") => {
            exit_for_result(
                run_phase_stream_real_traffic_guarded_separator_calibrated_split_shadow_v1(args),
                "try: nando-cli phase-stream-real-traffic-guarded-separator-calibrated-split-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [calibration-floor-micro] [calibration-guard-micro] [max-guards] [selector-permille] [compile-permille] [calibration-permille] [trace-jsonl ...]",
            )
        }
        Some("phase-stream-real-traffic-cost-evidence-audit-v1") => exit_for_result(
            run_phase_stream_real_traffic_cost_evidence_audit_v1(args),
            "try: nando-cli phase-stream-real-traffic-cost-evidence-audit-v1 [report-json] [trace-jsonl ...]",
        ),
        Some("phase-stream-real-traffic-token-cost-enrich-v1") => exit_for_result(
            run_phase_stream_real_traffic_token_cost_enrich_v1(args),
            "try: nando-cli phase-stream-real-traffic-token-cost-enrich-v1 [report-json] [readiness-report-json] [output-dir] [trace-jsonl ...]",
        ),
        Some("phase-stream-test-output-parse-promotion-audit-v1") => exit_for_result(
            run_phase_stream_test_output_parse_promotion_audit_v1(args),
            "try: nando-cli phase-stream-test-output-parse-promotion-audit-v1 [trace-jsonl] [shadow-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]",
        ),
        Some("phase-action-package-verify-v1") => exit_for_result(
            run_phase_action_package_verify_v1(args),
            "try: nando-cli phase-action-package-verify-v1 [package-path] [manifest-path] [score-report-json]",
        ),
        Some(command) if command.starts_with("role-binding-") => {
            eprintln!(
                "FORBIDDEN_LEGACY_NWRB_BACKEND: role-binding/nwrb has been removed from Nando Wave. Use phase-action / phase-center runtime."
            );
            ExitCode::FAILURE
        }
        Some("strict-multiseed-rust-audit-v1") => exit_for_result(
            run_strict_multiseed_rust_audit_v1(args),
            "try: nando-cli strict-multiseed-rust-audit-v1 [diagnostics-root] [audit-report-json]",
        ),
        Some("strict-multiseed-rust-audit-verify-v1") => exit_for_result(
            run_strict_multiseed_rust_audit_verify_v1(args),
            "try: nando-cli strict-multiseed-rust-audit-verify-v1 [diagnostics-root] [audit-report-json]",
        ),
        Some("phase-package-verify") => exit_for_result(
            run_phase_package_verify(args),
            "try: nando-cli phase-package-verify [package-path] [manifest-path] [score-report-json]",
        ),
        Some("live-byte-train") => match parse_live_byte_train_args(args) {
            Ok((seed, text)) => {
                print_live_byte_train(seed, &text);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-byte-train [seed] <text...>");
                ExitCode::FAILURE
            }
        },
        Some("live-byte-learn") => match parse_live_byte_train_args(args) {
            Ok((seed, text)) => {
                print_live_byte_learn(seed, &text);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-byte-learn [seed] <text...>");
                ExitCode::FAILURE
            }
        },
        Some("live-byte-holdout") => match parse_live_byte_train_args(args) {
            Ok((seed, text)) => {
                print_live_byte_holdout(seed, &text);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-byte-holdout [seed] <text...>");
                ExitCode::FAILURE
            }
        },
        Some("live-byte-holdout-suite") => match parse_optional_seed_arg(args) {
            Ok(seed) => {
                print_live_byte_holdout_suite(seed);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-byte-holdout-suite [seed]");
                ExitCode::FAILURE
            }
        },
        Some("live-byte-holdout-seed-sweep") => {
            print_live_byte_holdout_seed_sweep();
            ExitCode::SUCCESS
        }
        Some("live-cell-promote") => match parse_live_byte_train_args(args) {
            Ok((seed, text)) => {
                print_live_cell_promote(seed, &text);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-cell-promote [seed] <text...>");
                ExitCode::FAILURE
            }
        },
        Some("live-architecture-compare") => match parse_optional_seed_arg(args) {
            Ok(seed) => {
                print_live_architecture_compare(seed);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-architecture-compare [seed]");
                ExitCode::FAILURE
            }
        },
        Some("live-tissue-diagnose") => match parse_optional_seed_arg(args) {
            Ok(seed) => {
                print_live_tissue_diagnose(seed);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-tissue-diagnose [seed]");
                ExitCode::FAILURE
            }
        },
        Some("live-grok-trace") => match parse_live_grok_trace_args(args) {
            Ok((seed, epochs, interval)) => {
                print_live_grok_trace(seed, epochs, interval);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-grok-trace [seed] [epochs] [interval]");
                ExitCode::FAILURE
            }
        },
        Some("live-grok-sweep") => match parse_live_grok_sweep_args(args) {
            Ok((epochs, interval)) => {
                print_live_grok_sweep(epochs, interval);
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli live-grok-sweep [epochs] [interval]");
                ExitCode::FAILURE
            }
        },
        Some("eval-symbol-l3") => {
            print!("{}", nando_eval::symbol_l3_eval().to_text());
            ExitCode::SUCCESS
        }
        Some("eval-symbol-understanding") => {
            print!("{}", nando_eval::symbol_understanding0_eval().to_text());
            ExitCode::SUCCESS
        }
        Some("eval-symbol-retrieval") => {
            print!("{}", nando_eval::symbol_retrieval0_eval().to_text());
            ExitCode::SUCCESS
        }
        Some("eval-symbol-retrieval-sweep") => {
            print!(
                "{}",
                nando_eval::symbol_retrieval_stability_sweep().to_text()
            );
            ExitCode::SUCCESS
        }
        Some("eval-symbol-retrieval-capacity") => {
            print!("{}", nando_eval::symbol_retrieval_capacity_eval().to_text());
            ExitCode::SUCCESS
        }
        Some("eval-symbol-retrieval-capacity-scale") => {
            print!(
                "{}",
                nando_eval::symbol_retrieval_capacity_scale_eval().to_text()
            );
            ExitCode::SUCCESS
        }
        Some("eval-one-tick") => match parse_wave_tick_args(args) {
            Ok((seed, input_byte)) => {
                print!(
                    "{}",
                    nando_eval::one_tick_report(seed, input_byte).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-one-tick <input-byte> [seed]");
                ExitCode::FAILURE
            }
        },
        Some("eval-periodic") => match parse_periodic_args(args) {
            Ok(config) => {
                print!("{}", nando_eval::periodic_eval(config).to_text());
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-periodic [seed] [cases] [start] [step]");
                ExitCode::FAILURE
            }
        },
        Some("eval-phase-composition") => match parse_phase_composition_args(args) {
            Ok(config) => {
                print!("{}", nando_eval::phase_composition_eval(config).to_text());
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-phase-composition [seed] [cases] [start] [input-step] [phase-step]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-phase-holdout") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::phase_composition_holdout_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-phase-holdout [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-carrier-control") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::carrier_control_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-carrier-control [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-bus-transfer") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::bus_transfer_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-bus-transfer [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-memory") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_memory_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-memory [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-transition") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_transition_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-transition [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-dynamics") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_dynamics_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-dynamics [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-multitick") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_multitick_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-multitick [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-adapt") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_adapt_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-snapshot-adapt [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-decoder") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_decoder_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-decoder [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-keyed") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_keyed_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-snapshot-keyed [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-keyed-transition") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_keyed_transition_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-keyed-transition [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-noisy-keyed-transition") => match parse_phase_holdout_args(args) {
            Ok((train, holdout)) => {
                print!(
                    "{}",
                    nando_eval::snapshot_noisy_keyed_transition_eval(train, holdout).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-snapshot-noisy-keyed-transition [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-snapshot-noisy-keyed-transition-sweep") => {
            match parse_phase_holdout_args(args) {
                Ok((train, holdout)) => {
                    print!(
                        "{}",
                        nando_eval::snapshot_noisy_keyed_transition_sweep_eval(train, holdout)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-snapshot-noisy-keyed-transition-sweep [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-snapshot-noisy-keyed-transition-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::snapshot_noisy_keyed_transition_seed_sweep_eval(cases)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-snapshot-noisy-keyed-transition-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_eval(train_seed, holdout_seed, cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-byte-context [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-centroid") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_centroid_eval(train_seed, holdout_seed, cases)
                        .to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-byte-context-centroid [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-offset-centroid") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_offset_centroid_eval(train_seed, holdout_seed, cases)
                        .to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-byte-context-offset-centroid [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-denoised-centroid") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_denoised_centroid_eval(
                        train_seed,
                        holdout_seed,
                        cases
                    )
                    .to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-byte-context-denoised-centroid [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-relative-centroid") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_relative_centroid_eval(
                        train_seed,
                        holdout_seed,
                        cases
                    )
                    .to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-byte-context-relative-centroid [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-lexical-carrier-centroid") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_lexical_carrier_centroid_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-lexical-carrier-centroid [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-cellular-carrier-centroid") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_cellular_carrier_centroid_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-cellular-carrier-centroid [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-trained-carrier-centroid") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_trained_carrier_centroid_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-trained-carrier-centroid [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-centroid") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_centroid_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-centroid [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-diverse-centroid") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_diverse_centroid_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-diverse-centroid [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-centroid-seed-sweep") => match parse_cases_only_args(args) {
            Ok(cases) => {
                print!(
                    "{}",
                    nando_eval::byte_context_centroid_seed_sweep_eval(cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-byte-context-centroid-seed-sweep [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-offset-centroid-seed-sweep") => match parse_cases_only_args(args) {
            Ok(cases) => {
                print!(
                    "{}",
                    nando_eval::byte_context_offset_centroid_seed_sweep_eval(cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-byte-context-offset-centroid-seed-sweep [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-denoised-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_denoised_centroid_seed_sweep_eval(cases).to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-denoised-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-relative-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_relative_centroid_seed_sweep_eval(cases).to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-relative-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-lexical-carrier-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_lexical_carrier_centroid_seed_sweep_eval(cases)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-lexical-carrier-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-cellular-carrier-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_cellular_carrier_centroid_seed_sweep_eval(cases)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-cellular-carrier-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-trained-carrier-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_trained_carrier_centroid_seed_sweep_eval(cases)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-trained-carrier-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_centroid_seed_sweep_eval(cases)
                            .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep") => {
            match parse_cases_only_args(args) {
                Ok(cases) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_diverse_centroid_seed_sweep_eval(
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-centroid-ablation") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::byte_context_centroid_ablation_eval(
                        train_seed,
                        holdout_seed,
                        cases
                    )
                    .to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-byte-context-centroid-ablation [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-byte-context-cellular-carrier-ablation") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_cellular_carrier_ablation_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-cellular-carrier-ablation [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-trained-carrier-ablation") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_trained_carrier_ablation_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-trained-carrier-ablation [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-ablation") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_ablation_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-ablation [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-byte-context-prompt-carrier-diverse-ablation") => {
            match parse_seed_pair_cases_args(args) {
                Ok((train_seed, holdout_seed, cases)) => {
                    print!(
                        "{}",
                        nando_eval::byte_context_prompt_carrier_diverse_ablation_eval(
                            train_seed,
                            holdout_seed,
                            cases
                        )
                        .to_text()
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "try: nando-cli eval-byte-context-prompt-carrier-diverse-ablation [train-seed] [holdout-seed] [cases]"
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Some("eval-chat0") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::chat0_eval(train_seed, holdout_seed, cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-chat0 [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-settle-word") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::settle_word_eval(train_seed, holdout_seed, cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-settle-word [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-settle-word-seed-sweep") => match parse_cases_only_args(args) {
            Ok(cases) => {
                print!(
                    "{}",
                    nando_eval::settle_word_seed_sweep_eval(cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-settle-word-seed-sweep [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-chat0-route") => match parse_seed_pair_cases_args(args) {
            Ok((train_seed, holdout_seed, cases)) => {
                print!(
                    "{}",
                    nando_eval::chat0_route_eval(train_seed, holdout_seed, cases).to_text()
                );
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli eval-chat0-route [train-seed] [holdout-seed] [cases]");
                ExitCode::FAILURE
            }
        },
        Some("eval-chat0-promote") => match run_eval_chat0_promote(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-chat0-promote [feedback-log] [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("eval-chat0-promoted-holdout") => match run_eval_chat0_promoted_holdout(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli eval-chat0-promoted-holdout [feedback-log] [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("chat0-promote-save") => match run_chat0_promote_save(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli chat0-promote-save <feedback-log> <state-path> [train-seed] [holdout-seed] [cases]"
                );
                ExitCode::FAILURE
            }
        },
        Some("chat0-once") => match run_chat0_once(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli chat0-once <prompt> [expected] [trace-path]");
                ExitCode::FAILURE
            }
        },
        Some("chat0-once-promoted") => match run_chat0_once_promoted(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!(
                    "try: nando-cli chat0-once-promoted <state-path> <prompt> [expected] [trace-path]"
                );
                ExitCode::FAILURE
            }
        },
        Some("chat0-shell") => match run_chat0_shell(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                eprintln!("try: nando-cli chat0-shell [trace-dir] [feedback-log]");
                ExitCode::FAILURE
            }
        },
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("try: nando-cli --help");
            ExitCode::FAILURE
        }
    }
}

fn exit_for_result(result: Result<(), String>, usage: &str) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{usage}");
            ExitCode::FAILURE
        }
    }
}
