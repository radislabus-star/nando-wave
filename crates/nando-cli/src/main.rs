use std::process::ExitCode;

mod args;
mod bench;
mod chat0_cmd;
mod help;
mod live;
mod modadd_cmd;
mod organ128_cmd;
mod phase_daemon_cmd;
mod phase_package_cmd;
mod role_binding_package_cmd;
mod role_binding_runtime_cmd;
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
use role_binding_package_cmd::{
    run_role_binding_binary_eval_pack_suite_v1, run_role_binding_binary_eval_pack_suite_verify_v1,
    run_role_binding_eval_pack_binary_v1, run_role_binding_eval_pack_from_package_v1,
    run_role_binding_operator_blueprint_gap_v1, run_role_binding_operator_blueprint_gap_verify_v1,
    run_role_binding_package_inspect_v1, run_role_binding_package_score_v1,
    run_role_binding_package_score_verify_v1, run_role_binding_package_verify_v1,
    run_role_binding_release_suite_v1, run_role_binding_release_suite_verify_v1,
};
use role_binding_runtime_cmd::{
    run_role_binding_profile_fallback_smoke_v1, run_role_binding_profile_lb_replay_v1,
    run_role_binding_profile_lb_serve_v1, run_role_binding_profile_lb_throughput_v1,
    run_role_binding_profile_registry_from_release_v1, run_role_binding_profile_replay_suite_v1,
    run_role_binding_profile_runtime_smoke_v1, run_role_binding_profile_serve_v1,
    run_role_binding_profile_worker_replay_v1, run_role_binding_profile_worker_scaling_v1,
    run_role_binding_real_traffic_agent_control_admission_calibration_v1,
    run_role_binding_real_traffic_agent_control_output_evidence_v1,
    run_role_binding_real_traffic_agent_control_payload_dry_run_v1,
    run_role_binding_real_traffic_agent_control_profile_v1,
    run_role_binding_real_traffic_codex_history_ingest_v1,
    run_role_binding_real_traffic_codex_history_route_candidates_v1,
    run_role_binding_real_traffic_conditional_local_accept_calibration_v1,
    run_role_binding_real_traffic_conditional_output_evidence_v1,
    run_role_binding_real_traffic_conditional_payload_dry_run_v1,
    run_role_binding_real_traffic_conditional_payload_readiness_v1,
    run_role_binding_real_traffic_cpu_route_forecast_v1,
    run_role_binding_real_traffic_edit_admission_calibration_v1,
    run_role_binding_real_traffic_edit_local_accept_calibration_v1,
    run_role_binding_real_traffic_edit_output_evidence_v1,
    run_role_binding_real_traffic_edit_payload_dry_run_v1,
    run_role_binding_real_traffic_edit_payload_readiness_v1,
    run_role_binding_real_traffic_edit_safe_policy_promote_v1,
    run_role_binding_real_traffic_feedback_loop_v1, run_role_binding_real_traffic_ingest_events_v1,
    run_role_binding_real_traffic_mixed_local_accept_calibration_v1,
    run_role_binding_real_traffic_mixed_output_evidence_v1,
    run_role_binding_real_traffic_mixed_payload_dry_run_v1,
    run_role_binding_real_traffic_mixed_payload_readiness_v1,
    run_role_binding_real_traffic_mixed_safe_policy_promote_v1,
    run_role_binding_real_traffic_record_serve_v1, run_role_binding_real_traffic_record_v1,
    run_role_binding_real_traffic_route_gap_catalog_v1,
    run_role_binding_real_traffic_shadow_smoke_v1, run_role_binding_real_traffic_shadow_v1,
    run_role_binding_real_traffic_verification_hook_audit_v1,
};
use snapshot_io::{read_snapshot, save_snapshot};
use status::{print_organ128_plan, print_status, print_wave_tick};

fn main() -> ExitCode {
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
        Some("phase-action-package-verify-v1") => exit_for_result(
            run_phase_action_package_verify_v1(args),
            "try: nando-cli phase-action-package-verify-v1 [package-path] [manifest-path] [score-report-json]",
        ),
        Some("role-binding-package-inspect-v1") => exit_for_result(
            run_role_binding_package_inspect_v1(args),
            "try: nando-cli role-binding-package-inspect-v1 [package-path] [report-json]",
        ),
        Some("role-binding-package-verify-v1") => exit_for_result(
            run_role_binding_package_verify_v1(args),
            "try: nando-cli role-binding-package-verify-v1 [package-path] [report-json]",
        ),
        Some("role-binding-eval-pack-from-package-v1") => exit_for_result(
            run_role_binding_eval_pack_from_package_v1(args),
            "try: nando-cli role-binding-eval-pack-from-package-v1 [package-path] [eval-pack-json] [max-tasks]",
        ),
        Some("role-binding-eval-pack-binary-v1") => exit_for_result(
            run_role_binding_eval_pack_binary_v1(args),
            "try: nando-cli role-binding-eval-pack-binary-v1 [source-eval-pack-json] [binary-eval-pack] [report-json]",
        ),
        Some("role-binding-binary-eval-pack-suite-v1") => exit_for_result(
            run_role_binding_binary_eval_pack_suite_v1(args),
            "try: nando-cli role-binding-binary-eval-pack-suite-v1 [root-dir] [suite-report-json] [margin-threshold]",
        ),
        Some("role-binding-binary-eval-pack-suite-verify-v1") => exit_for_result(
            run_role_binding_binary_eval_pack_suite_verify_v1(args),
            "try: nando-cli role-binding-binary-eval-pack-suite-verify-v1 [root-dir] [suite-report-json] [margin-threshold]",
        ),
        Some("role-binding-package-score-v1") => exit_for_result(
            run_role_binding_package_score_v1(args),
            "try: nando-cli role-binding-package-score-v1 [package-path] [eval-pack-json] [score-report-json] [margin-threshold]",
        ),
        Some("role-binding-package-score-verify-v1") => exit_for_result(
            run_role_binding_package_score_verify_v1(args),
            "try: nando-cli role-binding-package-score-verify-v1 [package-path] [eval-pack-json] [score-report-json] [margin-threshold]",
        ),
        Some("role-binding-release-suite-v1") => exit_for_result(
            run_role_binding_release_suite_v1(args),
            "try: nando-cli role-binding-release-suite-v1 [binary-suite-report-json] [release-suite-report-json]",
        ),
        Some("role-binding-release-suite-verify-v1") => exit_for_result(
            run_role_binding_release_suite_verify_v1(args),
            "try: nando-cli role-binding-release-suite-verify-v1 [binary-suite-report-json] [release-suite-report-json]",
        ),
        Some("role-binding-operator-blueprint-gap-v1") => exit_for_result(
            run_role_binding_operator_blueprint_gap_v1(args),
            "try: nando-cli role-binding-operator-blueprint-gap-v1 [release-suite-report-json] [gap-report-json]",
        ),
        Some("role-binding-operator-blueprint-gap-verify-v1") => exit_for_result(
            run_role_binding_operator_blueprint_gap_verify_v1(args),
            "try: nando-cli role-binding-operator-blueprint-gap-verify-v1 [release-suite-report-json] [gap-report-json]",
        ),
        Some("role-binding-profile-registry-from-release-v1") => exit_for_result(
            run_role_binding_profile_registry_from_release_v1(args),
            "try: nando-cli role-binding-profile-registry-from-release-v1 [release-suite-report-json] [registry-config-json]",
        ),
        Some("role-binding-profile-serve-v1") => exit_for_result(
            run_role_binding_profile_serve_v1(args),
            "try: nando-cli role-binding-profile-serve-v1 [registry-config-json] [bind-addr] [request-limit]",
        ),
        Some("role-binding-profile-runtime-smoke-v1") => exit_for_result(
            run_role_binding_profile_runtime_smoke_v1(args),
            "try: nando-cli role-binding-profile-runtime-smoke-v1 [registry-config-json] [runtime-smoke-report-json]",
        ),
        Some("role-binding-profile-replay-suite-v1") => exit_for_result(
            run_role_binding_profile_replay_suite_v1(args),
            "try: nando-cli role-binding-profile-replay-suite-v1 [registry-config-json] [binary-suite-report-json] [replay-suite-report-json] [max-unique-sequences-per-profile] [batch-unique-sequences]",
        ),
        Some("role-binding-profile-fallback-smoke-v1") => exit_for_result(
            run_role_binding_profile_fallback_smoke_v1(args),
            "try: nando-cli role-binding-profile-fallback-smoke-v1 [registry-config-json] [fallback-smoke-report-json]",
        ),
        Some("role-binding-profile-worker-scaling-v1") => exit_for_result(
            run_role_binding_profile_worker_scaling_v1(args),
            "try: nando-cli role-binding-profile-worker-scaling-v1 [registry-config-json] [worker-scaling-report-json] [worker-count]",
        ),
        Some("role-binding-profile-worker-replay-v1") => exit_for_result(
            run_role_binding_profile_worker_replay_v1(args),
            "try: nando-cli role-binding-profile-worker-replay-v1 [registry-config-json] [binary-suite-report-json] [worker-replay-report-json] [worker-count] [max-unique-sequences-per-profile] [batch-unique-sequences]",
        ),
        Some("role-binding-profile-lb-serve-v1") => exit_for_result(
            run_role_binding_profile_lb_serve_v1(args),
            "try: nando-cli role-binding-profile-lb-serve-v1 <lb-config-json> [bind-addr] [request-limit]",
        ),
        Some("role-binding-profile-lb-replay-v1") => exit_for_result(
            run_role_binding_profile_lb_replay_v1(args),
            "try: nando-cli role-binding-profile-lb-replay-v1 [registry-config-json] [binary-suite-report-json] [lb-replay-report-json] [worker-count] [max-unique-sequences-per-profile] [batch-unique-sequences]",
        ),
        Some("role-binding-profile-lb-throughput-v1") => exit_for_result(
            run_role_binding_profile_lb_throughput_v1(args),
            "try: nando-cli role-binding-profile-lb-throughput-v1 [registry-config-json] [binary-suite-report-json] [throughput-report-json] [worker-count] [max-unique-sequences-per-profile] [client-threads] [sequence-repetitions]",
        ),
        Some("role-binding-real-traffic-record-v1") => exit_for_result(
            run_role_binding_real_traffic_record_v1(args),
            "try: nando-cli role-binding-real-traffic-record-v1 [trace-jsonl] <record-json>",
        ),
        Some("role-binding-real-traffic-record-serve-v1") => exit_for_result(
            run_role_binding_real_traffic_record_serve_v1(args),
            "try: nando-cli role-binding-real-traffic-record-serve-v1 [trace-jsonl] [bind-addr] [request-limit]",
        ),
        Some("role-binding-real-traffic-ingest-events-v1") => exit_for_result(
            run_role_binding_real_traffic_ingest_events_v1(args),
            "try: nando-cli role-binding-real-traffic-ingest-events-v1 <events-jsonl> [trace-jsonl] [ingest-report-json]",
        ),
        Some("role-binding-real-traffic-codex-history-ingest-v1") => exit_for_result(
            run_role_binding_real_traffic_codex_history_ingest_v1(args),
            "try: nando-cli role-binding-real-traffic-codex-history-ingest-v1 [history-jsonl] [events-jsonl] [ingest-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-codex-history-route-candidates-v1") => exit_for_result(
            run_role_binding_real_traffic_codex_history_route_candidates_v1(args),
            "try: nando-cli role-binding-real-traffic-codex-history-route-candidates-v1 [history-jsonl] [registry-config-json] [events-jsonl] [route-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-shadow-v1") => exit_for_result(
            run_role_binding_real_traffic_shadow_v1(args),
            "try: nando-cli role-binding-real-traffic-shadow-v1 [registry-config-json] [trace-jsonl] [shadow-report-json]",
        ),
        Some("role-binding-real-traffic-cpu-route-forecast-v1") => exit_for_result(
            run_role_binding_real_traffic_cpu_route_forecast_v1(args),
            "try: nando-cli role-binding-real-traffic-cpu-route-forecast-v1 [route-report-json] [shadow-report-json] [forecast-report-json]",
        ),
        Some("role-binding-real-traffic-route-gap-catalog-v1") => exit_for_result(
            run_role_binding_real_traffic_route_gap_catalog_v1(args),
            "try: nando-cli role-binding-real-traffic-route-gap-catalog-v1 [history-jsonl] [registry-config-json] [route-gap-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-agent-control-profile-v1") => exit_for_result(
            run_role_binding_real_traffic_agent_control_profile_v1(args),
            "try: nando-cli role-binding-real-traffic-agent-control-profile-v1 [base-registry-json] [agent-control-package-nwrb] [overlay-registry-json] [profile-report-json]",
        ),
        Some("role-binding-real-traffic-agent-control-payload-dry-run-v1") => exit_for_result(
            run_role_binding_real_traffic_agent_control_payload_dry_run_v1(args),
            "try: nando-cli role-binding-real-traffic-agent-control-payload-dry-run-v1 [history-jsonl] [agent-control-registry-json] [trace-jsonl] [dry-run-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-agent-control-output-evidence-v1") => exit_for_result(
            run_role_binding_real_traffic_agent_control_output_evidence_v1(args),
            "try: nando-cli role-binding-real-traffic-agent-control-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]",
        ),
        Some("role-binding-real-traffic-agent-control-admission-calibration-v1") => {
            exit_for_result(
                run_role_binding_real_traffic_agent_control_admission_calibration_v1(args),
                "try: nando-cli role-binding-real-traffic-agent-control-admission-calibration-v1 [evidence-trace-jsonl] [history-jsonl] [admission-report-json]",
            )
        }
        Some("role-binding-real-traffic-edit-payload-readiness-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_payload_readiness_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-edit-payload-dry-run-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_payload_dry_run_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-conditional-payload-readiness-v1") => exit_for_result(
            run_role_binding_real_traffic_conditional_payload_readiness_v1(args),
            "try: nando-cli role-binding-real-traffic-conditional-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-conditional-payload-dry-run-v1") => exit_for_result(
            run_role_binding_real_traffic_conditional_payload_dry_run_v1(args),
            "try: nando-cli role-binding-real-traffic-conditional-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-mixed-payload-readiness-v1") => exit_for_result(
            run_role_binding_real_traffic_mixed_payload_readiness_v1(args),
            "try: nando-cli role-binding-real-traffic-mixed-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-mixed-payload-dry-run-v1") => exit_for_result(
            run_role_binding_real_traffic_mixed_payload_dry_run_v1(args),
            "try: nando-cli role-binding-real-traffic-mixed-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]",
        ),
        Some("role-binding-real-traffic-edit-output-evidence-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_output_evidence_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]",
        ),
        Some("role-binding-real-traffic-conditional-output-evidence-v1") => exit_for_result(
            run_role_binding_real_traffic_conditional_output_evidence_v1(args),
            "try: nando-cli role-binding-real-traffic-conditional-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]",
        ),
        Some("role-binding-real-traffic-mixed-output-evidence-v1") => exit_for_result(
            run_role_binding_real_traffic_mixed_output_evidence_v1(args),
            "try: nando-cli role-binding-real-traffic-mixed-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]",
        ),
        Some("role-binding-real-traffic-edit-local-accept-calibration-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_local_accept_calibration_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]",
        ),
        Some("role-binding-real-traffic-conditional-local-accept-calibration-v1") => {
            exit_for_result(
                run_role_binding_real_traffic_conditional_local_accept_calibration_v1(args),
                "try: nando-cli role-binding-real-traffic-conditional-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]",
            )
        }
        Some("role-binding-real-traffic-mixed-local-accept-calibration-v1") => exit_for_result(
            run_role_binding_real_traffic_mixed_local_accept_calibration_v1(args),
            "try: nando-cli role-binding-real-traffic-mixed-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]",
        ),
        Some("role-binding-real-traffic-mixed-safe-policy-promote-v1") => exit_for_result(
            run_role_binding_real_traffic_mixed_safe_policy_promote_v1(args),
            "try: nando-cli role-binding-real-traffic-mixed-safe-policy-promote-v1 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd]",
        ),
        Some("role-binding-real-traffic-edit-safe-policy-promote-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_safe_policy_promote_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-safe-policy-promote-v1 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd]",
        ),
        Some("role-binding-real-traffic-edit-admission-calibration-v1") => exit_for_result(
            run_role_binding_real_traffic_edit_admission_calibration_v1(args),
            "try: nando-cli role-binding-real-traffic-edit-admission-calibration-v1 [evidence-trace-jsonl] [history-jsonl] [admission-report-json]",
        ),
        Some("role-binding-real-traffic-verification-hook-audit-v1") => exit_for_result(
            run_role_binding_real_traffic_verification_hook_audit_v1(args),
            "try: nando-cli role-binding-real-traffic-verification-hook-audit-v1 [trace-jsonl] [shadow-report-json] [audit-report-json]",
        ),
        Some("role-binding-real-traffic-feedback-loop-v1") => exit_for_result(
            run_role_binding_real_traffic_feedback_loop_v1(args),
            "try: nando-cli role-binding-real-traffic-feedback-loop-v1 [forecast-report-json] [edit-dry-run-report-json] [verification-audit-report-json] [feedback-report-json]",
        ),
        Some("role-binding-real-traffic-shadow-smoke-v1") => exit_for_result(
            run_role_binding_real_traffic_shadow_smoke_v1(args),
            "try: nando-cli role-binding-real-traffic-shadow-smoke-v1 [binary-suite-report-json] [trace-jsonl] [max-unique-sequences-per-profile]",
        ),
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
