use std::process::ExitCode;

mod args;
mod bench;
mod chat0_cmd;
mod help;
mod live;
mod organ128_cmd;
mod snapshot_io;
mod status;
use args::{
    parse_bench_stage2_tick_args, parse_cases_only_args, parse_live_byte_train_args,
    parse_live_grok_sweep_args, parse_live_grok_trace_args, parse_optional_seed_arg,
    parse_periodic_args, parse_phase_composition_args, parse_phase_holdout_args,
    parse_seed_pair_cases_args, parse_snapshot_save_args, parse_wave_tick_args,
};
use bench::{print_link_tissue_bench, print_stage2_tick_bench};
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
use organ128_cmd::{
    run_organ128_dialog_generate, run_organ128_response_gate_eval, run_organ128_settle_dialog,
    run_organ128_thought_probe_eval, run_organ128_train_generate, run_organ128_wave_scorer_eval,
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
