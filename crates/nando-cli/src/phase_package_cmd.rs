use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use nando_core::{
    PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO, PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
    PhaseCenterCell, PhaseCenterCompiler, PhaseCenterEvalTask, PhaseCenterFlatRuntime,
    PhaseCenterOffloadAction, PhaseCenterOffloadDecision, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, PhaseCenterOffloadSummary, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};

const DEFAULT_CELLS: usize = 32;
const DEFAULT_CORPUS: &str = "data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl";
const DEFAULT_ACTION_CONTRACT: &str =
    "data/rule_logic_operator_battery_v4/action_contract_v1/sample_action_contract_v1.jsonl";
const DEFAULT_ACTION_CORPUS: &str =
    "data/rule_logic_operator_battery_v4/action_contract_v1/generated_action_contract_v1.jsonl";
const DEFAULT_ACTION_DOMAIN_CORPUS: &str = "data/rule_logic_operator_battery_v4/action_contract_v1/generated_domain_action_contract_v1.jsonl";
const DEFAULT_ACTION_COVERAGE_CORPUS: &str = "data/rule_logic_operator_battery_v5/action_contract_v1/generated_coverage_action_contract_v1.jsonl";
const DEFAULT_PACKAGE: &str = "target/nando-wave/phase-center-v4-c32.nwpc";
const DEFAULT_ACTION_PACKAGE: &str = "target/nando-wave/action-runtime-v1-c32.nwpc";
const DEFAULT_ACTION_BENCH_ITERATIONS: usize = 1000;
const ACTION_BENCH_P99_NS_GATE: u128 = 50_000;
const ACTION_PRODUCT_PROOF_KIND: &str = "phase_action_product_proof_v1";
const ACTION_RELEASE_SUITE_KIND: &str = "phase_action_release_suite_v1";
const ACTION_LICENSE_PACKAGE_KIND: &str = "phase_action_noncommercial_license_package_v1";
const ACTION_OFFLOAD_AUDIT_KIND: &str = "phase_action_offload_audit_v1";
const ACTION_CACHE_OFFLOAD_BENCH_KIND: &str = "phase_action_cache_offload_bench_v1";
const ACTION_WORKFLOW_BENCH_KIND: &str = "phase_action_workflow_bench_v1";
const ACTION_WORKFLOW_REPLAY_KIND: &str = "phase_action_workflow_replay_v1";
const ACTION_REGRESSION_KIND: &str = "phase_action_regression_v1";
const ACTION_REGRESSION_FREEZE_KIND: &str = "phase_action_regression_freeze_v1";
const ACTION_OPTIMIZED_BUILD: bool = !cfg!(debug_assertions);
const PHASE_EVAL_TASK_PACKAGE_MAGIC: [u8; 8] = *b"NWPCT001";
const PHASE_EVAL_TASK_PACKAGE_HEADER_BYTES: usize = 44;
const DEFAULT_ACTION_GENERATED_PACKAGE: &str =
    "target/nando-wave/action-runtime-v1-generated-c32.nwpc";
const DEFAULT_ACTION_DOMAIN_PACKAGE: &str =
    "target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc";
const DEFAULT_ACTION_COVERAGE_PACKAGE: &str =
    "target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc";
const DEFAULT_ACTION_RELEASE_SUITE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-release-suite.product-proof.json";
const DEFAULT_ACTION_LICENSE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-license-package.product-proof.json";
const DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT: &str =
    "target/nando-wave/action-runtime-v1-offload-audit.product-proof.json";
const DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT: &str =
    "target/nando-wave/action-runtime-v1-cache-offload-bench.product-proof.json";
const DEFAULT_ACTION_WORKFLOW_BENCH_REPORT: &str =
    "target/nando-wave/action-runtime-v1-workflow-bench.product-proof.json";
const DEFAULT_ACTION_WORKFLOW_REPLAY_REPORT: &str =
    "target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json";
const DEFAULT_ACTION_REGRESSION_REPORT: &str =
    "target/nando-wave/action-runtime-v1-regression.product-proof.json";
const DEFAULT_ACTION_REGRESSION_FREEZE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-regression-freeze.product-proof.json";
const DEFAULT_STRICT_MULTI_SEED_DIAGNOSTICS_ROOT: &str =
    "data/rule_logic_operator_battery_v4/diagnostics/multiseed";
const DEFAULT_STRICT_MULTI_SEED_AUDIT_REPORT: &str =
    "target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json";
const STRICT_MULTI_SEED_AUDIT_KIND: &str = "strict_multiseed_rust_audit_v1";
const DEFAULT_OPERATOR_BLUEPRINT: &str = "docs/OPERATOR_BLUEPRINT.md";
const DEFAULT_NONCOMMERCIAL_LICENSE_FILE: &str = "LICENSE-NONCOMMERCIAL.md";
const NONCOMMERCIAL_LICENSE_NAME: &str = "Nando Wave Non-Commercial Source License v1.0";
const ACTION_STATE_TRANSITION_FORMULA: &str = "state_t + action_tree -> state_t+1";
const DEFAULT_ACTION_OFFLOAD_MARGIN_THRESHOLD_MICRO: i64 =
    PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO;
const DEFAULT_ACTION_OFFLOAD_SIMULATED_CALLS: usize = 1000;
const DEFAULT_ACTION_WORKFLOW_REPLAY_SESSIONS: usize = 256;
const DEFAULT_ACTION_WORKFLOW_REPLAY_STEPS_PER_SESSION: usize = 12;
const MIN_ACTION_CONTRACT_KEY_COVERAGE: usize = 6;

#[derive(Clone, Debug, Deserialize)]
struct PhaseOperatorRow {
    source_group: String,
    operator_class: String,
    condition_flag: Option<String>,
    sequence_length: usize,
    surface_family: String,
    #[serde(default = "default_noise_type")]
    noise_type: String,
    #[serde(rename = "rule_action_example")]
    action: String,
    source_tokens: Vec<String>,
    correct_tokens: Vec<String>,
    wrong_tokens: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionContractRow {
    schema_version: String,
    task_id: String,
    split: String,
    state_before: String,
    action_tree: PhaseActionTree,
    state_after_correct: String,
    state_after_wrong: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionTree {
    select: String,
    transform: String,
    write: String,
    condition: String,
    check: String,
}

#[derive(Clone, Debug)]
struct PreparedTask {
    task: PhaseCenterEvalTask,
}

#[derive(Clone, Debug)]
struct PreparedEval {
    tasks: Vec<PreparedTask>,
    action_ablation_tasks: Vec<PreparedTask>,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_missing_centers: usize,
    heldout_surface_groups: usize,
    heldout_noise_groups: usize,
}

pub(crate) fn run_phase_package_v4(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_package_args(args)?;
    let rows = load_phase_operator_rows(&config.corpus_path)?;
    let train_rows = rows
        .iter()
        .filter(|row| phase_split(row) == Some("train"))
        .count();
    let heldout_rows = rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
        .count();

    let (runtime, key_to_index, skipped_train_rows) = compile_runtime(&rows, config.cells)?;
    let package_bytes = runtime.to_bytes().map_err(format_runtime_error)?;
    write_package(&config.package_path, &package_bytes)?;

    let loaded_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&loaded_bytes).map_err(format_runtime_error)?;
    let loaded_runtime =
        PhaseCenterFlatRuntime::from_bytes(&loaded_bytes).map_err(format_runtime_error)?;
    let prepared = prepare_eval_tasks(&rows, config.cells, &key_to_index);
    let eval = eval_loaded_runtime(&loaded_runtime, &prepared.tasks)?;
    let action_ablation_eval =
        eval_loaded_runtime(&loaded_runtime, &prepared.action_ablation_tasks)?;
    let manifest = PhasePackageManifest::from_run(PhasePackageManifestInput {
        corpus_path: &config.corpus_path,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        rows: rows.len(),
        train_rows,
        heldout_rows,
        cells: config.cells,
        skipped_train_rows,
        prepared: &prepared,
        key_to_index: &key_to_index,
        loaded_runtime: &loaded_runtime,
        package_info,
        package_bytes_len: loaded_bytes.len(),
        eval,
        action_ablation_eval,
    });
    write_manifest(&config.manifest_path, &manifest)?;

    let gate_pass = package_v4_gate_pass(
        &eval,
        &prepared,
        skipped_train_rows,
        &action_ablation_eval,
        PackageGateMeta {
            package_fingerprint64: package_info.fingerprint64,
            operator_key_count: manifest.operator_keys.len(),
            record_count: loaded_runtime.record_count(),
            has_empty_operator_key: manifest.operator_keys.iter().any(|key| key.is_empty()),
        },
    );

    println!("nando_phase_package_v4:");
    println!("  verdict: {}", phase_package_v4_verdict(gate_pass));
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  rows: {}", rows.len());
    println!("  train_rows: {train_rows}");
    println!("  heldout_rows: {heldout_rows}");
    println!("  cells: {}", config.cells);
    println!("  flat_records: {}", loaded_runtime.record_count());
    println!("  operator_key_count: {}", manifest.operator_keys.len());
    println!("  skipped_train_rows: {skipped_train_rows}");
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        prepared.action_ablation_tasks.len()
    );
    println!(
        "  action_ablation_missing_centers: {}",
        prepared.action_ablation_missing_centers
    );
    println!(
        "  heldout_surface_groups: {}",
        prepared.heldout_surface_groups
    );
    println!("  heldout_noise_groups: {}", prepared.heldout_noise_groups);
    println!("  package_magic: {:?}", PHASE_CENTER_RUNTIME_PACKAGE_MAGIC);
    println!("  inspected_cells: {}", package_info.cells);
    println!("  inspected_records: {}", package_info.record_count);
    println!("  inspected_payload_bytes: {}", package_info.payload_bytes);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", loaded_bytes.len());
    println!("  serialized_len: {}", loaded_runtime.serialized_len());
    println!(
        "  runtime_bytes_estimate: {}",
        loaded_runtime.bytes_estimate()
    );
    println!("  accuracy_milli: {}", eval.accuracy_milli);
    println!("  wrong_wins: {}", eval.wrong_wins);
    println!("  median_margin: {:.6}", eval.median_margin);
    println!("  p10_margin: {:.6}", eval.p10_margin);
    println!("  p50_latency_ns: {}", eval.p50_latency_ns);
    println!("  p99_latency_ns: {}", eval.p99_latency_ns);
    println!("  total_eval_us: {}", eval.total_eval_us);
    println!("  rows_per_second: {:.2}", eval.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        action_ablation_eval.accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        action_ablation_eval.wrong_wins
    );
    println!(
        "  action_ablation_median_margin: {:.6}",
        action_ablation_eval.median_margin
    );
    println!(
        "  action_ablation_p10_margin: {:.6}",
        action_ablation_eval.p10_margin
    );
    println!("  compiler_path: nando_core::PhaseCenterCompiler");
    println!("  package_path_api: nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    println!("  epoch_repair_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    if !gate_pass {
        return Err(String::from("phase package v4 gate failed"));
    }

    Ok(())
}

pub(crate) fn run_phase_package_inspect(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_package_inspect_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_manifest(&config.manifest_path)?;

    let manifest_matches =
        validate_manifest_package_match(&manifest, &package_info, package_bytes.len()).is_ok();
    let forbidden_used = manifest.forbidden_flags.any_forbidden_used();
    let inspect_pass = manifest_matches && !forbidden_used;

    println!("nando_phase_package_inspect:");
    println!(
        "  verdict: {}",
        if inspect_pass {
            "PHASE_PACKAGE_INSPECT_PASS"
        } else {
            "PHASE_PACKAGE_INSPECT_WATCH"
        }
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  package_magic: {:?}", package_info.magic);
    println!("  cells: {}", package_info.cells);
    println!("  flat_records: {}", package_info.record_count);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  inspected_payload_bytes: {}", package_info.payload_bytes);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  runtime_bytes_estimate: {}", runtime.bytes_estimate());
    println!("  manifest_schema_version: {}", manifest.schema_version);
    println!("  manifest_verdict: {}", manifest.verdict);
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  claim_boundary: {}", manifest.claim_boundary);
    println!(
        "  target_center_id_training_used: {}",
        manifest.forbidden_flags.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        manifest
            .forbidden_flags
            .proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        manifest.forbidden_flags.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        manifest.forbidden_flags.local_out_t_runtime_extension_used
    );

    validate_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if manifest.forbidden_flags.any_forbidden_used() {
        return Err(String::from(
            "phase package manifest reports forbidden shortcut usage",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_inspect_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_inspect_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let gate_pass = manifest.gate_pass() && manifest_matches;

    println!("nando_phase_action_package_inspect_v1:");
    println!(
        "  verdict: {}",
        phase_action_package_inspect_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  package_magic: {:?}", package_info.magic);
    println!("  cells: {}", package_info.cells);
    println!("  flat_records: {}", package_info.record_count);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  inspected_payload_bytes: {}", package_info.payload_bytes);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  runtime_bytes_estimate: {}", runtime.bytes_estimate());
    println!("  manifest_schema_version: {}", manifest.schema_version);
    println!("  manifest_verdict: {}", manifest.verdict);
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  manifest_matches_package: {manifest_matches}");
    println!(
        "  source_contract_fingerprint64: {}",
        manifest.source_contract_fingerprint64
    );
    println!(
        "  source_contract_bytes: {}",
        manifest.source_contract_bytes
    );
    println!("  accuracy_milli: {}", manifest.accuracy_milli);
    println!("  wrong_wins: {}", manifest.wrong_wins);
    println!(
        "  action_ablation_accuracy_milli: {}",
        manifest.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        manifest.action_ablation_wrong_wins
    );
    println!("  python_demo_used: {}", manifest.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        manifest.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        manifest.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        manifest.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        manifest.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", manifest.claim_boundary);

    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if !manifest.gate_pass() {
        return Err(String::from(
            "phase action package manifest does not pass its gate",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_source_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_source_verify_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let source_rebuild = rebuild_action_package_from_source(&manifest, &package_bytes)?;
    let report = PhaseActionSourceVerifyReport::from_inputs(PhaseActionSourceVerifyReportInput {
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        manifest: &manifest,
        package_info,
        package_bytes_len: package_bytes.len(),
        runtime_bytes_estimate: runtime.bytes_estimate(),
        manifest_matches_package: manifest_matches,
        source_rebuild: &source_rebuild,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }
    let gate_pass = report.gate_pass();

    println!("nando_phase_action_source_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_source_verify_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  corpus_path: {}", report.corpus_path);
    println!(
        "  source_contract_fingerprint64: {}",
        report.source_contract_fingerprint64
    );
    println!("  source_contract_bytes: {}", report.source_contract_bytes);
    println!(
        "  source_rebuild_matches_package: {}",
        report.source_rebuild_matches_package
    );
    println!(
        "  source_rebuild_package_fingerprint64: {}",
        report.source_rebuild_package_fingerprint64
    );
    println!(
        "  source_rebuild_package_bytes: {}",
        report.source_rebuild_package_bytes
    );
    println!(
        "  source_rebuild_operator_keys_match: {}",
        report.source_rebuild_operator_keys_match
    );
    println!(
        "  source_rebuild_contract_verdict: {}",
        report.source_rebuild_contract_verdict
    );
    println!(
        "  source_rebuild_skipped_train_rows: {}",
        report.source_rebuild_skipped_train_rows
    );
    println!(
        "  source_rebuild_action_tree_key_count: {}",
        report.source_rebuild_action_tree_key_count
    );
    println!(
        "  source_rebuild_train_action_tree_key_count: {}",
        report.source_rebuild_train_action_tree_key_count
    );
    println!(
        "  source_rebuild_heldout_action_tree_key_count: {}",
        report.source_rebuild_heldout_action_tree_key_count
    );
    println!(
        "  source_rebuild_min_train_rows_per_action_tree: {}",
        report.source_rebuild_min_train_rows_per_action_tree
    );
    println!(
        "  source_rebuild_min_heldout_rows_per_action_tree: {}",
        report.source_rebuild_min_heldout_rows_per_action_tree
    );
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!("  package_bytes: {}", report.package_bytes);
    println!("  cells: {}", report.cells);
    println!("  flat_records: {}", report.flat_records);
    println!(
        "  manifest_matches_package: {}",
        report.manifest_matches_package
    );
    println!("  manifest_gate_pass: {}", report.manifest_gate_pass);
    println!("  compiler_path: {}", report.compiler_path);
    println!("  package_path_api: {}", report.package_path_api);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used());
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action source verify v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_score_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_score_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if !manifest.gate_pass() {
        return Err(String::from(
            "phase action package score v1 refused manifest that does not pass its gate",
        ));
    }

    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    if !contract_report.gate_pass() {
        return Err(String::from(
            "phase action package score v1 refused dirty action contract",
        ));
    }
    let key_to_index = action_manifest_key_to_index(&manifest)?;
    let prepared = prepare_action_contract_eval(&rows, manifest.cells, &key_to_index);
    let eval = eval_loaded_runtime(&runtime, &prepared.tasks)?;
    let action_ablation_eval = eval_loaded_runtime(&runtime, &prepared.action_ablation_tasks)?;
    let forbidden_used = manifest_forbidden_used(&manifest);
    let gate_pass = action_score_report_gate_inputs_pass(
        &eval,
        &prepared,
        &action_ablation_eval,
        forbidden_used,
        &contract_report.verdict,
        manifest.gate_pass(),
    ) && ACTION_OPTIMIZED_BUILD;
    let verdict = phase_action_package_score_v1_verdict(gate_pass);
    let report = PhaseActionPackageScoreReport::from_score(PhaseActionPackageScoreReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        corpus_path: &config.corpus_path,
        eval_task_package_path: None,
        package_info,
        package_bytes_len: package_bytes.len(),
        runtime: &runtime,
        manifest: &manifest,
        rows: rows.len(),
        prepared: &prepared,
        eval,
        action_ablation_eval,
        compiler_used: false,
        contract_verdict: &contract_report.verdict,
        eval_task_package_used: false,
        corpus_jsonl_used_in_score_loop: false,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_action_package_score_v1:");
    println!("  verdict: {verdict}");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  cells: {}", manifest.cells);
    println!("  flat_records: {}", runtime.record_count());
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  rows: {}", rows.len());
    println!("  heldout_eval_rows: {}", prepared.tasks.len());
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        prepared.action_ablation_tasks.len()
    );
    println!(
        "  action_ablation_missing_centers: {}",
        prepared.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", report.accuracy_milli);
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  median_margin: {:.6}", report.median_margin);
    println!("  p10_margin: {:.6}", report.p10_margin);
    println!("  p50_latency_ns: {}", report.p50_latency_ns);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  total_eval_us: {}", report.total_eval_us);
    println!("  rows_per_second: {:.2}", report.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  optimized_build: {}", report.optimized_build);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        format_optional_bool(report.corpus_jsonl_used_in_score_loop)
    );
    println!("  contract_verdict: {}", report.contract_verdict);
    println!("  manifest_verdict: {}", report.manifest_verdict);
    println!("  runtime_path: {}", report.runtime_path);
    if let Some(report_path) = &config.report_path {
        println!("  score_report_path: {}", report_path.display());
    }
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action package score v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_eval_pack_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_eval_pack_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if !manifest.gate_pass() {
        return Err(String::from(
            "phase action eval-pack v1 refused manifest that does not pass its gate",
        ));
    }

    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    if !contract_report.gate_pass() {
        return Err(String::from(
            "phase action eval-pack v1 refused dirty action contract",
        ));
    }
    let key_to_index = action_manifest_key_to_index(&manifest)?;
    let prepared = prepare_action_contract_eval(&rows, manifest.cells, &key_to_index);
    let eval_package = PhaseEvalTaskPackage::from_prepared(
        manifest.cells,
        package_info.fingerprint64,
        rows.len(),
        prepared,
    );
    let bytes = eval_package.to_bytes()?;
    write_package(&config.eval_pack_path, &bytes)?;
    let loaded = read_eval_task_package(&config.eval_pack_path)?;
    let gate_pass =
        action_eval_pack_v1_gate_pass(&loaded, &package_info, &manifest, &contract_report.verdict);

    println!("nando_phase_action_eval_pack_v1:");
    println!(
        "  verdict: {}",
        phase_action_eval_pack_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  eval_pack_magic: {:?}", PHASE_EVAL_TASK_PACKAGE_MAGIC);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!(
        "  eval_pack_package_fingerprint64: {}",
        loaded.package_fingerprint64
    );
    println!("  cells: {}", loaded.cells);
    println!("  rows: {}", loaded.rows);
    println!("  contract_verdict: {}", contract_report.verdict);
    println!("  manifest_verdict: {}", manifest.verdict);
    println!("  heldout_eval_rows: {}", loaded.prepared.tasks.len());
    println!(
        "  action_ablation_eval_rows: {}",
        loaded.prepared.action_ablation_tasks.len()
    );
    println!("  missing_centers: {}", loaded.prepared.missing_centers);
    println!("  skipped_rows: {}", loaded.prepared.skipped_rows);
    println!(
        "  action_ablation_missing_centers: {}",
        loaded.prepared.action_ablation_missing_centers
    );
    println!("  eval_pack_bytes: {}", bytes.len());
    println!("  compiler_used: false");
    println!("  jsonl_used_after_pack_build: false");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    println!("  python_demo_used: false");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    if !gate_pass {
        return Err(String::from("phase action eval-pack v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_score_pack_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_score_pack_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if !manifest.gate_pass() {
        return Err(String::from(
            "phase action package score-pack v1 refused manifest that does not pass its gate",
        ));
    }
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    validate_action_eval_task_package_match(&eval_package, &manifest, &package_info)?;
    let eval = eval_loaded_runtime(&runtime, &eval_package.prepared.tasks)?;
    let action_ablation_eval =
        eval_loaded_runtime(&runtime, &eval_package.prepared.action_ablation_tasks)?;
    let forbidden_used = manifest_forbidden_used(&manifest);
    let gate_pass = action_score_report_gate_inputs_pass(
        &eval,
        &eval_package.prepared,
        &action_ablation_eval,
        forbidden_used,
        &manifest.contract_verdict,
        manifest.gate_pass(),
    ) && ACTION_OPTIMIZED_BUILD;
    let verdict = phase_action_package_score_pack_v1_verdict(gate_pass);
    let report = PhaseActionPackageScoreReport::from_score(PhaseActionPackageScoreReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        corpus_path: Path::new(&manifest.corpus_path),
        eval_task_package_path: Some(&config.eval_pack_path),
        package_info,
        package_bytes_len: package_bytes.len(),
        runtime: &runtime,
        manifest: &manifest,
        rows: eval_package.rows,
        prepared: &eval_package.prepared,
        eval,
        action_ablation_eval,
        compiler_used: false,
        contract_verdict: &manifest.contract_verdict,
        eval_task_package_used: true,
        corpus_jsonl_used_in_score_loop: false,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_action_package_score_pack_v1:");
    println!("  verdict: {verdict}");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  cells: {}", manifest.cells);
    println!("  flat_records: {}", runtime.record_count());
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  eval_pack_rows: {}", eval_package.rows);
    println!("  eval_pack_bytes: {}", eval_package.serialized_len());
    println!("  heldout_eval_rows: {}", eval_package.prepared.tasks.len());
    println!(
        "  missing_centers: {}",
        eval_package.prepared.missing_centers
    );
    println!("  skipped_rows: {}", eval_package.prepared.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        eval_package.prepared.action_ablation_tasks.len()
    );
    println!(
        "  action_ablation_missing_centers: {}",
        eval_package.prepared.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", report.accuracy_milli);
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  median_margin: {:.6}", report.median_margin);
    println!("  p10_margin: {:.6}", report.p10_margin);
    println!("  p50_latency_ns: {}", report.p50_latency_ns);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  total_eval_us: {}", report.total_eval_us);
    println!("  rows_per_second: {:.2}", report.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  optimized_build: {}", report.optimized_build);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        format_optional_bool(report.corpus_jsonl_used_in_score_loop)
    );
    println!("  contract_verdict: {}", report.contract_verdict);
    println!("  manifest_verdict: {}", report.manifest_verdict);
    println!("  runtime_path: {}", report.runtime_path);
    if let Some(report_path) = &config.report_path {
        println!("  score_report_path: {}", report_path.display());
    }
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from(
            "phase action package score-pack v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_bench_pack_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_bench_pack_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    if !manifest.gate_pass() {
        return Err(String::from(
            "phase action package bench-pack v1 refused manifest that does not pass its gate",
        ));
    }
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    validate_action_eval_task_package_match(&eval_package, &manifest, &package_info)?;

    println!("nando_phase_action_package_bench_pack_v1:");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  bench_iterations: {}", config.iterations);
    println!(
        "  planned_bench_samples: {}",
        eval_package
            .prepared
            .tasks
            .len()
            .saturating_mul(config.iterations)
    );
    println!(
        "  planned_action_ablation_bench_samples: {}",
        eval_package
            .prepared
            .action_ablation_tasks
            .len()
            .saturating_mul(config.iterations)
    );

    let eval = bench_loaded_runtime(&runtime, &eval_package.prepared.tasks, config.iterations)?;
    let action_ablation_eval = bench_loaded_runtime(
        &runtime,
        &eval_package.prepared.action_ablation_tasks,
        config.iterations,
    )?;
    let gate_pass = action_score_report_gate_inputs_pass(
        &eval,
        &eval_package.prepared,
        &action_ablation_eval,
        manifest_forbidden_used(&manifest),
        &manifest.contract_verdict,
        manifest.gate_pass(),
    ) && eval.p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
        && ACTION_OPTIMIZED_BUILD;
    let verdict = phase_action_package_bench_pack_v1_verdict(gate_pass);
    let report = PhaseActionPackageBenchReport::from_bench(PhaseActionPackageBenchReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        eval_task_package_path: &config.eval_pack_path,
        package_info,
        package_bytes_len: package_bytes.len(),
        eval_package: &eval_package,
        runtime: &runtime,
        manifest: &manifest,
        bench_iterations: config.iterations,
        eval,
        action_ablation_eval,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("  verdict: {verdict}");
    println!("  cells: {}", report.cells);
    println!("  flat_records: {}", report.flat_records);
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!("  package_bytes: {}", report.package_bytes);
    println!("  eval_pack_bytes: {}", report.eval_pack_bytes);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    println!("  rows: {}", report.rows);
    println!("  heldout_eval_rows: {}", report.heldout_eval_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        report.action_ablation_eval_rows
    );
    println!("  bench_samples: {}", report.bench_samples);
    println!(
        "  action_ablation_bench_samples: {}",
        report.action_ablation_bench_samples
    );
    println!("  accuracy_milli: {}", report.accuracy_milli);
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  median_margin: {:.6}", report.median_margin);
    println!("  p10_margin: {:.6}", report.p10_margin);
    println!("  p50_latency_ns: {}", report.p50_latency_ns);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  p99_latency_gate_ns: {}", report.p99_latency_gate_ns);
    println!("  total_eval_us: {}", report.total_eval_us);
    println!("  rows_per_second: {:.2}", report.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  optimized_build: {}", report.optimized_build);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_bench_loop: {}",
        report.corpus_jsonl_used_in_bench_loop
    );
    println!("  contract_verdict: {}", report.contract_verdict);
    println!("  manifest_verdict: {}", report.manifest_verdict);
    println!("  runtime_path: {}", report.runtime_path);
    if let Some(report_path) = &config.report_path {
        println!("  bench_report_path: {}", report_path.display());
    }
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from(
            "phase action package bench-pack v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_bench_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_bench_verify_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    let bench_report = read_action_bench_report(&config.report_path)?;

    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let eval_pack_matches =
        validate_action_eval_task_package_match(&eval_package, &manifest, &package_info).is_ok();
    let report_matches = validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let gate_pass = manifest_matches
        && eval_pack_matches
        && report_matches
        && manifest.gate_pass()
        && bench_report.gate_pass();

    println!("nando_phase_action_package_bench_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_package_bench_verify_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  bench_report_path: {}", config.report_path.display());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  cells: {}", package_info.cells);
    println!("  flat_records: {}", package_info.record_count);
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!(
        "  bench_report_schema_version: {}",
        bench_report.schema_version
    );
    println!("  bench_report_verdict: {}", bench_report.verdict);
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  eval_pack_matches_package: {eval_pack_matches}");
    println!("  bench_report_matches_package: {report_matches}");
    println!("  bench_iterations: {}", bench_report.bench_iterations);
    println!("  bench_samples: {}", bench_report.bench_samples);
    println!("  accuracy_milli: {}", bench_report.accuracy_milli);
    println!("  wrong_wins: {}", bench_report.wrong_wins);
    println!("  p50_latency_ns: {}", bench_report.p50_latency_ns);
    println!("  p99_latency_ns: {}", bench_report.p99_latency_ns);
    println!(
        "  p99_latency_gate_ns: {}",
        bench_report.p99_latency_gate_ns
    );
    println!(
        "  action_ablation_accuracy_milli: {}",
        bench_report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        bench_report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", bench_report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        bench_report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_bench_loop: {}",
        bench_report.corpus_jsonl_used_in_bench_loop
    );
    println!("  contract_verdict: {}", bench_report.contract_verdict);
    println!("  manifest_verdict: {}", bench_report.manifest_verdict);
    println!(
        "  manifest_forbidden_used: {}",
        manifest_forbidden_used(&manifest)
    );
    println!(
        "  bench_report_forbidden_used: {}",
        bench_report.forbidden_used()
    );
    println!("  claim_boundary: {}", bench_report.claim_boundary);

    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    validate_action_eval_task_package_match(&eval_package, &manifest, &package_info)?;
    validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )?;
    if !gate_pass {
        return Err(String::from(
            "phase action package bench verify v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_product_proof_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_product_proof_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    let score_report = read_action_score_report(&config.score_report_path)?;
    let bench_report = read_action_bench_report(&config.bench_report_path)?;
    let source_rebuild = rebuild_action_package_from_source(&manifest, &package_bytes)?;

    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let eval_pack_matches =
        validate_action_eval_task_package_match(&eval_package, &manifest, &package_info).is_ok();
    let score_report_matches = validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let bench_report_matches = validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let score_report_gate_pass = score_report.gate_pass();
    let bench_report_gate_pass = bench_report.gate_pass();
    let input_gate_pass = manifest_matches
        && eval_pack_matches
        && score_report_matches
        && bench_report_matches
        && manifest.gate_pass()
        && source_rebuild.gate_pass()
        && score_report_gate_pass
        && bench_report_gate_pass
        && score_report.verdict == "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS"
        && bench_report.verdict == "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS";
    let verdict = phase_action_product_proof_v1_verdict(input_gate_pass);
    let report = PhaseActionProductProofReport::from_inputs(PhaseActionProductProofReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        eval_task_package_path: &config.eval_pack_path,
        score_report_path: &config.score_report_path,
        bench_report_path: &config.bench_report_path,
        package_info,
        package_bytes_len: package_bytes.len(),
        manifest: &manifest,
        eval_package: &eval_package,
        score_report: &score_report,
        bench_report: &bench_report,
        source_rebuild: &source_rebuild,
    });
    let gate_pass = input_gate_pass && report.gate_pass();
    let proof_report_path = config
        .proof_report_path
        .unwrap_or_else(|| default_product_proof_report_path(&config.package_path));
    write_json_file(&proof_report_path, &report)?;

    println!("nando_phase_action_product_proof_v1:");
    println!("  verdict: {verdict}");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!(
        "  score_report_path: {}",
        config.score_report_path.display()
    );
    println!(
        "  bench_report_path: {}",
        config.bench_report_path.display()
    );
    println!("  product_proof_path: {}", proof_report_path.display());
    println!("  product_proof_kind: {}", report.product_proof_kind);
    println!(
        "  source_contract_fingerprint64: {}",
        report.source_contract_fingerprint64
    );
    println!("  source_contract_bytes: {}", report.source_contract_bytes);
    println!(
        "  source_rebuild_matches_package: {}",
        report.source_rebuild_matches_package
    );
    println!(
        "  source_rebuild_package_fingerprint64: {}",
        report.source_rebuild_package_fingerprint64
    );
    println!(
        "  source_rebuild_package_bytes: {}",
        report.source_rebuild_package_bytes
    );
    println!(
        "  source_rebuild_operator_keys_match: {}",
        report.source_rebuild_operator_keys_match
    );
    println!(
        "  source_rebuild_contract_verdict: {}",
        report.source_rebuild_contract_verdict
    );
    println!(
        "  source_rebuild_skipped_train_rows: {}",
        report.source_rebuild_skipped_train_rows
    );
    println!(
        "  source_rebuild_action_tree_key_count: {}",
        report.source_rebuild_action_tree_key_count
    );
    println!(
        "  source_rebuild_train_action_tree_key_count: {}",
        report.source_rebuild_train_action_tree_key_count
    );
    println!(
        "  source_rebuild_heldout_action_tree_key_count: {}",
        report.source_rebuild_heldout_action_tree_key_count
    );
    println!(
        "  source_rebuild_min_train_rows_per_action_tree: {}",
        report.source_rebuild_min_train_rows_per_action_tree
    );
    println!(
        "  source_rebuild_min_heldout_rows_per_action_tree: {}",
        report.source_rebuild_min_heldout_rows_per_action_tree
    );
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!("  package_bytes: {}", report.package_bytes);
    println!("  eval_pack_bytes: {}", report.eval_pack_bytes);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  eval_pack_matches_package: {eval_pack_matches}");
    println!("  score_report_matches_package: {score_report_matches}");
    println!("  bench_report_matches_package: {bench_report_matches}");
    println!("  score_report_gate_pass: {score_report_gate_pass}");
    println!("  bench_report_gate_pass: {bench_report_gate_pass}");
    println!(
        "  input_score_forbidden_used: {}",
        score_report.forbidden_used()
    );
    println!(
        "  input_bench_forbidden_used: {}",
        bench_report.forbidden_used()
    );
    println!("  score_report_verdict: {}", report.score_report_verdict);
    println!("  bench_report_verdict: {}", report.bench_report_verdict);
    println!("  rows: {}", report.rows);
    println!("  heldout_eval_rows: {}", report.heldout_eval_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        report.action_ablation_eval_rows
    );
    println!("  score_accuracy_milli: {}", report.score_accuracy_milli);
    println!("  score_wrong_wins: {}", report.score_wrong_wins);
    println!("  score_p99_latency_ns: {}", report.score_p99_latency_ns);
    println!(
        "  score_action_ablation_accuracy_milli: {}",
        report.score_action_ablation_accuracy_milli
    );
    println!("  bench_iterations: {}", report.bench_iterations);
    println!("  bench_samples: {}", report.bench_samples);
    println!("  bench_accuracy_milli: {}", report.bench_accuracy_milli);
    println!("  bench_wrong_wins: {}", report.bench_wrong_wins);
    println!("  bench_p99_latency_ns: {}", report.bench_p99_latency_ns);
    println!(
        "  bench_p99_latency_gate_ns: {}",
        report.bench_p99_latency_gate_ns
    );
    println!(
        "  bench_action_ablation_accuracy_milli: {}",
        report.bench_action_ablation_accuracy_milli
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  optimized_build: {}", report.optimized_build);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        report.corpus_jsonl_used_in_score_loop
    );
    println!(
        "  corpus_jsonl_used_in_bench_loop: {}",
        report.corpus_jsonl_used_in_bench_loop
    );
    println!("  forbidden_used: {}", report.forbidden_used());
    println!("  claim_boundary: {}", report.claim_boundary);
    println!("  license_boundary: {}", report.license_boundary);
    println!("  product_boundary: {}", report.product_boundary);

    if !gate_pass {
        return Err(String::from("phase action product proof v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_product_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_product_verify_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    let score_report = read_action_score_report(&config.score_report_path)?;
    let bench_report = read_action_bench_report(&config.bench_report_path)?;
    let product_report = read_action_product_proof_report(&config.proof_report_path)?;
    let source_rebuild = rebuild_action_package_from_source(&manifest, &package_bytes)?;

    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let eval_pack_matches =
        validate_action_eval_task_package_match(&eval_package, &manifest, &package_info).is_ok();
    let score_report_matches = validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let bench_report_matches = validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let product_report_matches =
        validate_action_product_proof_report_match(PhaseActionProductProofValidationInput {
            report: &product_report,
            manifest: &manifest,
            eval_package: &eval_package,
            score_report: &score_report,
            bench_report: &bench_report,
            package_info: &package_info,
            package_bytes_len: package_bytes.len(),
            source_rebuild: &source_rebuild,
        })
        .is_ok();
    let score_report_gate_pass = score_report.gate_pass();
    let bench_report_gate_pass = bench_report.gate_pass();
    let product_report_gate_pass = product_report.gate_pass();
    let gate_pass = manifest_matches
        && eval_pack_matches
        && score_report_matches
        && bench_report_matches
        && product_report_matches
        && manifest.gate_pass()
        && score_report_gate_pass
        && bench_report_gate_pass
        && product_report_gate_pass;

    println!("nando_phase_action_product_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_product_verify_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!(
        "  score_report_path: {}",
        config.score_report_path.display()
    );
    println!(
        "  bench_report_path: {}",
        config.bench_report_path.display()
    );
    println!(
        "  product_proof_path: {}",
        config.proof_report_path.display()
    );
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!(
        "  product_proof_kind: {}",
        product_report.product_proof_kind
    );
    println!(
        "  source_contract_fingerprint64: {}",
        product_report.source_contract_fingerprint64
    );
    println!(
        "  source_contract_bytes: {}",
        product_report.source_contract_bytes
    );
    println!(
        "  source_rebuild_matches_package: {}",
        product_report.source_rebuild_matches_package
    );
    println!(
        "  source_rebuild_package_fingerprint64: {}",
        product_report.source_rebuild_package_fingerprint64
    );
    println!(
        "  source_rebuild_package_bytes: {}",
        product_report.source_rebuild_package_bytes
    );
    println!(
        "  source_rebuild_operator_keys_match: {}",
        product_report.source_rebuild_operator_keys_match
    );
    println!(
        "  source_rebuild_action_tree_key_count: {}",
        product_report.source_rebuild_action_tree_key_count
    );
    println!(
        "  source_rebuild_train_action_tree_key_count: {}",
        product_report.source_rebuild_train_action_tree_key_count
    );
    println!(
        "  source_rebuild_heldout_action_tree_key_count: {}",
        product_report.source_rebuild_heldout_action_tree_key_count
    );
    println!(
        "  source_rebuild_min_train_rows_per_action_tree: {}",
        product_report.source_rebuild_min_train_rows_per_action_tree
    );
    println!(
        "  source_rebuild_min_heldout_rows_per_action_tree: {}",
        product_report.source_rebuild_min_heldout_rows_per_action_tree
    );
    println!("  product_report_verdict: {}", product_report.verdict);
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  eval_pack_matches_package: {eval_pack_matches}");
    println!("  score_report_matches_package: {score_report_matches}");
    println!("  bench_report_matches_package: {bench_report_matches}");
    println!("  product_report_matches_package: {product_report_matches}");
    println!("  score_report_gate_pass: {score_report_gate_pass}");
    println!("  bench_report_gate_pass: {bench_report_gate_pass}");
    println!("  product_report_gate_pass: {product_report_gate_pass}");
    println!(
        "  input_score_forbidden_used: {}",
        score_report.forbidden_used()
    );
    println!(
        "  input_bench_forbidden_used: {}",
        bench_report.forbidden_used()
    );
    println!("  score_report_verdict: {}", score_report.verdict);
    println!("  bench_report_verdict: {}", bench_report.verdict);
    println!(
        "  score_accuracy_milli: {}",
        product_report.score_accuracy_milli
    );
    println!("  score_wrong_wins: {}", product_report.score_wrong_wins);
    println!("  bench_iterations: {}", product_report.bench_iterations);
    println!("  bench_samples: {}", product_report.bench_samples);
    println!(
        "  bench_p99_latency_ns: {}",
        product_report.bench_p99_latency_ns
    );
    println!(
        "  bench_p99_latency_gate_ns: {}",
        product_report.bench_p99_latency_gate_ns
    );
    println!("  compiler_used: {}", product_report.compiler_used);
    println!("  optimized_build: {}", product_report.optimized_build);
    println!(
        "  eval_task_package_used: {}",
        product_report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        product_report.corpus_jsonl_used_in_score_loop
    );
    println!(
        "  corpus_jsonl_used_in_bench_loop: {}",
        product_report.corpus_jsonl_used_in_bench_loop
    );
    println!("  forbidden_used: {}", product_report.forbidden_used());
    println!("  product_boundary: {}", product_report.product_boundary);

    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    validate_action_eval_task_package_match(&eval_package, &manifest, &package_info)?;
    validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )?;
    validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )?;
    validate_action_product_proof_report_match(PhaseActionProductProofValidationInput {
        report: &product_report,
        manifest: &manifest,
        eval_package: &eval_package,
        score_report: &score_report,
        bench_report: &bench_report,
        package_info: &package_info,
        package_bytes_len: package_bytes.len(),
        source_rebuild: &source_rebuild,
    })?;
    if !gate_pass {
        return Err(String::from("phase action product verify v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_release_suite_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_release_suite_args(args)?;
    let artifacts = default_action_release_suite_bundles()
        .iter()
        .map(build_action_release_suite_artifact_report)
        .collect::<Result<Vec<_>, _>>()?;
    let report = PhaseActionReleaseSuiteReport::from_artifacts(artifacts);
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_release_suite_v1:");
    println!("  verdict: {}", report.verdict);
    println!(
        "  release_suite_report_path: {}",
        config.report_path.display()
    );
    println!("  release_suite_kind: {}", report.release_suite_kind);
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  distinct_package_fingerprints: {}",
        report.distinct_package_fingerprints
    );
    println!("  total_package_bytes: {}", report.total_package_bytes);
    println!("  total_eval_pack_bytes: {}", report.total_eval_pack_bytes);
    println!(
        "  total_source_verify_report_bytes: {}",
        report.total_source_verify_report_bytes
    );
    println!(
        "  total_shortcut_report_bytes: {}",
        report.total_shortcut_report_bytes
    );
    println!(
        "  total_operator_coverage_report_bytes: {}",
        report.total_operator_coverage_report_bytes
    );
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!("  total_bench_samples: {}", report.total_bench_samples);
    println!(
        "  max_score_p99_latency_ns: {}",
        report.max_score_p99_latency_ns
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!(
        "  all_score_accuracy_milli_1000: {}",
        report.all_score_accuracy_milli_1000
    );
    println!(
        "  all_bench_accuracy_milli_1000: {}",
        report.all_bench_accuracy_milli_1000
    );
    println!(
        "  all_source_verify_reports_pass: {}",
        report.all_source_verify_reports_pass
    );
    println!(
        "  all_shortcut_reports_pass: {}",
        report.all_shortcut_reports_pass
    );
    println!(
        "  all_operator_coverage_reports_match_sources: {}",
        report.all_operator_coverage_reports_match_sources
    );
    println!(
        "  operator_dimension_coverage_artifact_count: {}",
        report.operator_dimension_coverage_artifact_count
    );
    println!(
        "  release_operator_dimension_coverage_pass: {}",
        report.release_operator_dimension_coverage_pass
    );
    println!(
        "  max_operator_coverage_min_dimension_value_count: {}",
        report.max_operator_coverage_min_dimension_value_count
    );
    println!(
        "  max_operator_coverage_wide_dimension_count: {}",
        report.max_operator_coverage_wide_dimension_count
    );
    println!(
        "  all_action_ablation_collapses: {}",
        report.all_action_ablation_collapses
    );
    println!(
        "  all_action_contract_source_rebuild_clean: {}",
        report.all_action_contract_source_rebuild_clean
    );
    println!(
        "  all_optimized_build_reports_pass: {}",
        report.all_optimized_build_reports_pass
    );
    println!(
        "  total_source_rebuild_accepted_action_tree_rows: {}",
        report.total_source_rebuild_accepted_action_tree_rows
    );
    println!(
        "  total_source_rebuild_rejected_action_tree_rows: {}",
        report.total_source_rebuild_rejected_action_tree_rows
    );
    println!(
        "  total_source_rebuild_forbidden_contract_rows: {}",
        report.total_source_rebuild_forbidden_contract_rows
    );
    println!(
        "  total_source_rebuild_action_tree_key_count: {}",
        report.total_source_rebuild_action_tree_key_count
    );
    println!(
        "  min_source_rebuild_action_tree_key_count: {}",
        report.min_source_rebuild_action_tree_key_count
    );
    println!(
        "  all_action_tree_key_coverage_pass: {}",
        report.all_action_tree_key_coverage_pass
    );
    println!(
        "  all_package_report_parity_pass: {}",
        report.all_package_report_parity_pass
    );
    println!(
        "  all_manifest_package_parity_pass: {}",
        report.all_manifest_package_parity_pass
    );
    println!(
        "  all_eval_pack_package_parity_pass: {}",
        report.all_eval_pack_package_parity_pass
    );
    println!(
        "  all_score_report_package_parity_pass: {}",
        report.all_score_report_package_parity_pass
    );
    println!(
        "  all_bench_report_package_parity_pass: {}",
        report.all_bench_report_package_parity_pass
    );
    println!(
        "  all_product_report_package_parity_pass: {}",
        report.all_product_report_package_parity_pass
    );
    println!(
        "  all_source_rebuild_package_parity_pass: {}",
        report.all_source_rebuild_package_parity_pass
    );
    println!(
        "  all_source_verify_report_package_parity_pass: {}",
        report.all_source_verify_report_package_parity_pass
    );
    println!(
        "  max_score_action_ablation_accuracy_milli: {}",
        report.max_score_action_ablation_accuracy_milli
    );
    println!(
        "  max_bench_action_ablation_accuracy_milli: {}",
        report.max_bench_action_ablation_accuracy_milli
    );
    println!(
        "  total_score_action_ablation_wrong_wins: {}",
        report.total_score_action_ablation_wrong_wins
    );
    println!(
        "  total_bench_action_ablation_wrong_wins: {}",
        report.total_bench_action_ablation_wrong_wins
    );
    println!(
        "  total_score_wrong_wins: {}",
        report.total_score_wrong_wins
    );
    println!(
        "  total_bench_wrong_wins: {}",
        report.total_bench_wrong_wins
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!(
        "  commercial_license_closed: {}",
        report.commercial_license_closed
    );
    for artifact in &report.artifacts {
        println!(
            "  artifact.{}.product_verify_pass: {}",
            artifact.label, artifact.product_verify_pass
        );
        println!(
            "  artifact.{}.package_fingerprint64: {}",
            artifact.label, artifact.package_fingerprint64
        );
        println!(
            "  artifact.{}.source_contract_fingerprint64: {}",
            artifact.label, artifact.source_contract_fingerprint64
        );
        println!(
            "  artifact.{}.source_contract_bytes: {}",
            artifact.label, artifact.source_contract_bytes
        );
        println!(
            "  artifact.{}.source_rebuild_matches_package: {}",
            artifact.label, artifact.source_rebuild_matches_package
        );
        println!(
            "  artifact.{}.source_rebuild_package_fingerprint64: {}",
            artifact.label, artifact.source_rebuild_package_fingerprint64
        );
        println!(
            "  artifact.{}.source_verify_report_gate_pass: {}",
            artifact.label, artifact.source_verify_report_gate_pass
        );
        println!(
            "  artifact.{}.source_verify_report_matches_package: {}",
            artifact.label, artifact.source_verify_report_matches_package
        );
        println!(
            "  artifact.{}.shortcut_report_gate_pass: {}",
            artifact.label, artifact.shortcut_report_gate_pass
        );
        println!(
            "  artifact.{}.shortcut_report_matches_corpus: {}",
            artifact.label, artifact.shortcut_report_matches_corpus
        );
        println!(
            "  artifact.{}.score_accuracy_milli: {}",
            artifact.label, artifact.score_accuracy_milli
        );
        println!(
            "  artifact.{}.bench_p99_latency_ns: {}",
            artifact.label, artifact.bench_p99_latency_ns
        );
    }
    println!("  suite_boundary: {}", report.suite_boundary);
    println!("  product_boundary: {}", report.product_boundary);
    println!("  license_boundary: {}", report.license_boundary);

    if !report.gate_pass() {
        return Err(String::from("phase action release suite v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_release_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_release_suite_args(args)?;
    let report = read_action_release_suite_report(&config.report_path)?;
    let rebuilt_artifacts = report
        .artifacts
        .iter()
        .map(action_product_bundle_paths_from_artifact)
        .map(|paths| build_action_release_suite_artifact_report(&paths))
        .collect::<Result<Vec<_>, _>>()?;
    let rebuilt_report = PhaseActionReleaseSuiteReport::from_artifacts(rebuilt_artifacts);
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_release_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_release_verify_v1_verdict(gate_pass)
    );
    println!(
        "  release_suite_report_path: {}",
        config.report_path.display()
    );
    println!("  release_suite_kind: {}", report.release_suite_kind);
    println!("  artifact_count: {}", report.artifact_count);
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    println!(
        "  distinct_package_fingerprints: {}",
        report.distinct_package_fingerprints
    );
    println!("  total_package_bytes: {}", report.total_package_bytes);
    println!("  total_eval_pack_bytes: {}", report.total_eval_pack_bytes);
    println!(
        "  total_source_verify_report_bytes: {}",
        report.total_source_verify_report_bytes
    );
    println!(
        "  total_shortcut_report_bytes: {}",
        report.total_shortcut_report_bytes
    );
    println!(
        "  total_operator_coverage_report_bytes: {}",
        report.total_operator_coverage_report_bytes
    );
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!("  total_bench_samples: {}", report.total_bench_samples);
    println!(
        "  max_score_p99_latency_ns: {}",
        report.max_score_p99_latency_ns
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!(
        "  all_source_verify_reports_pass: {}",
        report.all_source_verify_reports_pass
    );
    println!(
        "  all_shortcut_reports_pass: {}",
        report.all_shortcut_reports_pass
    );
    println!(
        "  all_operator_coverage_reports_match_sources: {}",
        report.all_operator_coverage_reports_match_sources
    );
    println!(
        "  operator_dimension_coverage_artifact_count: {}",
        report.operator_dimension_coverage_artifact_count
    );
    println!(
        "  release_operator_dimension_coverage_pass: {}",
        report.release_operator_dimension_coverage_pass
    );
    println!(
        "  max_operator_coverage_min_dimension_value_count: {}",
        report.max_operator_coverage_min_dimension_value_count
    );
    println!(
        "  max_operator_coverage_wide_dimension_count: {}",
        report.max_operator_coverage_wide_dimension_count
    );
    println!(
        "  all_action_ablation_collapses: {}",
        report.all_action_ablation_collapses
    );
    println!(
        "  all_action_contract_source_rebuild_clean: {}",
        report.all_action_contract_source_rebuild_clean
    );
    println!(
        "  all_optimized_build_reports_pass: {}",
        report.all_optimized_build_reports_pass
    );
    println!(
        "  total_source_rebuild_accepted_action_tree_rows: {}",
        report.total_source_rebuild_accepted_action_tree_rows
    );
    println!(
        "  total_source_rebuild_rejected_action_tree_rows: {}",
        report.total_source_rebuild_rejected_action_tree_rows
    );
    println!(
        "  total_source_rebuild_forbidden_contract_rows: {}",
        report.total_source_rebuild_forbidden_contract_rows
    );
    println!(
        "  total_source_rebuild_action_tree_key_count: {}",
        report.total_source_rebuild_action_tree_key_count
    );
    println!(
        "  min_source_rebuild_action_tree_key_count: {}",
        report.min_source_rebuild_action_tree_key_count
    );
    println!(
        "  all_action_tree_key_coverage_pass: {}",
        report.all_action_tree_key_coverage_pass
    );
    println!(
        "  all_package_report_parity_pass: {}",
        report.all_package_report_parity_pass
    );
    println!(
        "  all_manifest_package_parity_pass: {}",
        report.all_manifest_package_parity_pass
    );
    println!(
        "  all_eval_pack_package_parity_pass: {}",
        report.all_eval_pack_package_parity_pass
    );
    println!(
        "  all_score_report_package_parity_pass: {}",
        report.all_score_report_package_parity_pass
    );
    println!(
        "  all_bench_report_package_parity_pass: {}",
        report.all_bench_report_package_parity_pass
    );
    println!(
        "  all_product_report_package_parity_pass: {}",
        report.all_product_report_package_parity_pass
    );
    println!(
        "  all_source_rebuild_package_parity_pass: {}",
        report.all_source_rebuild_package_parity_pass
    );
    println!(
        "  all_source_verify_report_package_parity_pass: {}",
        report.all_source_verify_report_package_parity_pass
    );
    println!(
        "  max_score_action_ablation_accuracy_milli: {}",
        report.max_score_action_ablation_accuracy_milli
    );
    println!(
        "  max_bench_action_ablation_accuracy_milli: {}",
        report.max_bench_action_ablation_accuracy_milli
    );
    println!(
        "  total_score_action_ablation_wrong_wins: {}",
        report.total_score_action_ablation_wrong_wins
    );
    println!(
        "  total_bench_action_ablation_wrong_wins: {}",
        report.total_bench_action_ablation_wrong_wins
    );
    println!(
        "  commercial_license_closed: {}",
        report.commercial_license_closed
    );
    for artifact in &report.artifacts {
        println!(
            "  artifact.{}.gate_pass: {}",
            artifact.label,
            artifact.gate_pass()
        );
        println!(
            "  artifact.{}.product_verify_pass: {}",
            artifact.label, artifact.product_verify_pass
        );
        println!(
            "  artifact.{}.source_rebuild_matches_package: {}",
            artifact.label, artifact.source_rebuild_matches_package
        );
        println!(
            "  artifact.{}.source_verify_report_gate_pass: {}",
            artifact.label, artifact.source_verify_report_gate_pass
        );
        println!(
            "  artifact.{}.source_verify_report_matches_package: {}",
            artifact.label, artifact.source_verify_report_matches_package
        );
        println!(
            "  artifact.{}.score_report_gate_pass: {}",
            artifact.label, artifact.score_report_gate_pass
        );
        println!(
            "  artifact.{}.bench_report_gate_pass: {}",
            artifact.label, artifact.bench_report_gate_pass
        );
        println!(
            "  artifact.{}.product_report_gate_pass: {}",
            artifact.label, artifact.product_report_gate_pass
        );
    }
    println!("  suite_boundary: {}", report.suite_boundary);
    println!("  product_boundary: {}", report.product_boundary);
    println!("  license_boundary: {}", report.license_boundary);

    if !gate_pass {
        return Err(String::from("phase action release verify v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_license_package_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_license_package_args(args)?;
    let report = build_action_license_package_report(
        &config.release_suite_report_path,
        &config.license_file_path,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_license_package_v1:");
    println!("  verdict: {}", report.verdict);
    println!(
        "  license_package_report_path: {}",
        config.report_path.display()
    );
    println!(
        "  release_suite_report_path: {}",
        config.release_suite_report_path.display()
    );
    println!(
        "  license_file_path: {}",
        config.license_file_path.display()
    );
    println!("  license_package_kind: {}", report.license_package_kind);
    println!("  license_name: {}", report.license_name);
    println!(
        "  license_file_fingerprint64: {}",
        report.license_file_fingerprint64
    );
    println!("  license_file_bytes: {}", report.license_file_bytes);
    println!(
        "  license_file_contains_noncommercial_grant: {}",
        report.license_file_contains_noncommercial_grant
    );
    println!(
        "  license_file_contains_commercial_restriction: {}",
        report.license_file_contains_commercial_restriction
    );
    println!(
        "  license_file_contains_no_warranty: {}",
        report.license_file_contains_no_warranty
    );
    println!(
        "  cargo_workspace_license_file_declared: {}",
        report.cargo_workspace_license_file_declared
    );
    println!(
        "  cargo_workspace_mit_license_declared: {}",
        report.cargo_workspace_mit_license_declared
    );
    println!(
        "  cargo_crate_license_file_workspace_declared: {}",
        report.cargo_crate_license_file_workspace_declared
    );
    println!(
        "  cargo_crate_license_workspace_declared: {}",
        report.cargo_crate_license_workspace_declared
    );
    println!("  release_suite_verdict: {}", report.release_suite_verdict);
    println!("  release_suite_kind: {}", report.release_suite_kind);
    println!(
        "  release_suite_gate_pass: {}",
        report.release_suite_gate_pass
    );
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  release_suite_artifact_count: {}",
        report.release_suite_artifact_count
    );
    println!(
        "  release_suite_total_runtime_bytes_estimate: {}",
        report.release_suite_total_runtime_bytes_estimate
    );
    println!(
        "  release_suite_total_bench_samples: {}",
        report.release_suite_total_bench_samples
    );
    println!(
        "  release_suite_max_bench_p99_latency_ns: {}",
        report.release_suite_max_bench_p99_latency_ns
    );
    println!(
        "  release_suite_license_boundary_mentions_mit: {}",
        report.release_suite_license_boundary_mentions_mit
    );
    println!(
        "  commercial_use_allowed: {}",
        report.commercial_use_allowed
    );
    println!(
        "  noncommercial_use_allowed: {}",
        report.noncommercial_use_allowed
    );
    println!(
        "  commercial_license_closed: {}",
        report.commercial_license_closed
    );
    println!(
        "  non_commercial_license_closed: {}",
        report.non_commercial_license_closed
    );
    println!("  package_boundary: {}", report.package_boundary);
    println!("  license_boundary: {}", report.license_boundary);

    if !report.gate_pass() {
        return Err(String::from("phase action license package v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_license_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_license_package_args(args)?;
    let report = read_action_license_package_report(&config.report_path)?;
    let rebuilt_report = build_action_license_package_report(
        &config.release_suite_report_path,
        &config.license_file_path,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_license_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_license_verify_v1_verdict(gate_pass)
    );
    println!(
        "  license_package_report_path: {}",
        config.report_path.display()
    );
    println!(
        "  release_suite_report_path: {}",
        config.release_suite_report_path.display()
    );
    println!(
        "  license_file_path: {}",
        config.license_file_path.display()
    );
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    println!("  license_package_kind: {}", report.license_package_kind);
    println!("  license_name: {}", report.license_name);
    println!(
        "  license_file_fingerprint64: {}",
        report.license_file_fingerprint64
    );
    println!(
        "  cargo_workspace_license_file_declared: {}",
        report.cargo_workspace_license_file_declared
    );
    println!(
        "  cargo_workspace_mit_license_declared: {}",
        report.cargo_workspace_mit_license_declared
    );
    println!(
        "  cargo_crate_license_file_workspace_declared: {}",
        report.cargo_crate_license_file_workspace_declared
    );
    println!(
        "  cargo_crate_license_workspace_declared: {}",
        report.cargo_crate_license_workspace_declared
    );
    println!(
        "  release_suite_gate_pass: {}",
        report.release_suite_gate_pass
    );
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  release_suite_artifact_count: {}",
        report.release_suite_artifact_count
    );
    println!(
        "  release_suite_total_runtime_bytes_estimate: {}",
        report.release_suite_total_runtime_bytes_estimate
    );
    println!(
        "  release_suite_max_bench_p99_latency_ns: {}",
        report.release_suite_max_bench_p99_latency_ns
    );
    println!(
        "  release_suite_license_boundary_mentions_mit: {}",
        report.release_suite_license_boundary_mentions_mit
    );
    println!(
        "  commercial_use_allowed: {}",
        report.commercial_use_allowed
    );
    println!(
        "  noncommercial_use_allowed: {}",
        report.noncommercial_use_allowed
    );
    println!(
        "  commercial_license_closed: {}",
        report.commercial_license_closed
    );
    println!(
        "  non_commercial_license_closed: {}",
        report.non_commercial_license_closed
    );
    println!("  package_boundary: {}", report.package_boundary);
    println!("  license_boundary: {}", report.license_boundary);

    if !gate_pass {
        return Err(String::from("phase action license verify v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_offload_audit_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_offload_audit_args(args)?;
    let report = build_action_offload_audit_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        config.margin_threshold_micro,
        config.simulated_calls,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_offload_audit_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  offload_audit_kind: {}", report.offload_audit_kind);
    println!("  report_path: {}", config.report_path.display());
    println!(
        "  release_suite_report_path: {}",
        config.release_suite_report_path.display()
    );
    println!(
        "  license_package_report_path: {}",
        config.license_report_path.display()
    );
    println!(
        "  license_file_path: {}",
        config.license_file_path.display()
    );
    println!(
        "  margin_threshold_micro: {}",
        report.margin_threshold_micro
    );
    println!("  simulated_calls: {}", report.simulated_calls);
    println!("  local_operator_calls: {}", report.local_operator_calls);
    println!("  fallback_to_llm_calls: {}", report.fallback_to_llm_calls);
    println!("  offload_rate_milli: {}", report.offload_rate_milli);
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  total_unique_eval_rows: {}",
        report.total_unique_eval_rows
    );
    println!(
        "  unique_local_operator_rows: {}",
        report.unique_local_operator_rows
    );
    println!("  unique_fallback_rows: {}", report.unique_fallback_rows);
    println!(
        "  unique_offload_rate_milli: {}",
        report.unique_offload_rate_milli
    );
    println!("  median_margin_micro: {}", report.median_margin_micro);
    println!("  p10_margin_micro: {}", report.p10_margin_micro);
    println!(
        "  release_suite_gate_pass: {}",
        report.release_suite_gate_pass
    );
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  license_package_gate_pass: {}",
        report.license_package_gate_pass
    );
    println!(
        "  license_report_matches_sources: {}",
        report.license_report_matches_sources
    );
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!(
        "  commercial_use_allowed: {}",
        report.commercial_use_allowed
    );
    println!(
        "  noncommercial_use_allowed: {}",
        report.noncommercial_use_allowed
    );
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!(
        "  offload_sdk_inspect_api: {}",
        report.offload_sdk_inspect_api
    );
    println!("  offload_policy_api: {}", report.offload_policy_api);
    println!("  offload_batch_api: {}", report.offload_batch_api);
    println!("  offload_summary_api: {}", report.offload_summary_api);
    println!("  offload_buffer_api: {}", report.offload_buffer_api);
    println!(
        "  offload_summary_buffer_api: {}",
        report.offload_summary_buffer_api
    );
    println!(
        "  offload_runtime_summary_api: {}",
        report.offload_runtime_summary_api
    );
    for artifact in &report.artifacts {
        println!(
            "  artifact.{}.unique_offload_rate_milli: {}",
            artifact.label, artifact.unique_offload_rate_milli
        );
        println!(
            "  artifact.{}.sdk_inspected_fingerprint64: {}",
            artifact.label, artifact.sdk_inspected_fingerprint64
        );
        println!(
            "  artifact.{}.sdk_inspect_matches_package: {}",
            artifact.label, artifact.sdk_inspect_matches_package
        );
        println!(
            "  artifact.{}.sdk_inspect_matches_eval_pack: {}",
            artifact.label, artifact.sdk_inspect_matches_eval_pack
        );
        println!(
            "  artifact.{}.simulated_local_operator_calls: {}",
            artifact.label, artifact.simulated_local_operator_calls
        );
        println!(
            "  artifact.{}.simulated_fallback_to_llm_calls: {}",
            artifact.label, artifact.simulated_fallback_to_llm_calls
        );
    }
    println!("  fallback_policy: {}", report.fallback_policy);
    println!("  audit_boundary: {}", report.audit_boundary);
    println!("  license_boundary: {}", report.license_boundary);

    if !report.gate_pass() {
        return Err(String::from("phase action offload audit v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_offload_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_offload_verify_args(args)?;
    let report = read_action_offload_audit_report(&config.report_path)?;
    let rebuilt_report = build_action_offload_audit_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        report.margin_threshold_micro,
        report.simulated_calls,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_offload_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_offload_verify_v1_verdict(gate_pass)
    );
    println!("  report_path: {}", config.report_path.display());
    println!(
        "  release_suite_report_path: {}",
        config.release_suite_report_path.display()
    );
    println!(
        "  license_package_report_path: {}",
        config.license_report_path.display()
    );
    println!(
        "  license_file_path: {}",
        config.license_file_path.display()
    );
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    println!(
        "  margin_threshold_micro: {}",
        report.margin_threshold_micro
    );
    println!("  simulated_calls: {}", report.simulated_calls);
    println!("  local_operator_calls: {}", report.local_operator_calls);
    println!("  fallback_to_llm_calls: {}", report.fallback_to_llm_calls);
    println!("  offload_rate_milli: {}", report.offload_rate_milli);
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  release_suite_gate_pass: {}",
        report.release_suite_gate_pass
    );
    println!(
        "  license_package_gate_pass: {}",
        report.license_package_gate_pass
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!(
        "  offload_sdk_inspect_api: {}",
        report.offload_sdk_inspect_api
    );
    println!("  offload_policy_api: {}", report.offload_policy_api);
    println!("  offload_batch_api: {}", report.offload_batch_api);
    println!("  offload_summary_api: {}", report.offload_summary_api);
    println!("  offload_buffer_api: {}", report.offload_buffer_api);
    println!(
        "  offload_summary_buffer_api: {}",
        report.offload_summary_buffer_api
    );
    println!(
        "  offload_runtime_summary_api: {}",
        report.offload_runtime_summary_api
    );
    println!("  fallback_policy: {}", report.fallback_policy);
    println!("  audit_boundary: {}", report.audit_boundary);

    if !gate_pass {
        return Err(String::from("phase action offload verify v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_cache_offload_bench_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_cache_offload_bench_args(args)?;
    let report = build_action_cache_offload_bench_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        config.margin_threshold_micro,
        config.simulated_calls,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_cache_offload_bench_v1:");
    println!("  verdict: {}", report.verdict);
    println!(
        "  cache_offload_bench_kind: {}",
        report.cache_offload_bench_kind
    );
    println!("  report_path: {}", config.report_path.display());
    print_phase_action_cache_offload_bench_report(&report);

    if !report.gate_pass() {
        return Err(String::from(
            "phase action cache offload bench v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_cache_offload_bench_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_cache_offload_bench_args(args)?;
    let report = read_action_cache_offload_bench_report(&config.report_path)?;
    let rebuilt_report = build_action_cache_offload_bench_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        report.margin_threshold_micro,
        report.simulated_calls,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_cache_offload_bench_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_cache_offload_bench_verify_v1_verdict(gate_pass)
    );
    println!("  report_path: {}", config.report_path.display());
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    print_phase_action_cache_offload_bench_report(&report);

    if !gate_pass {
        return Err(String::from(
            "phase action cache offload bench verify v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_workflow_bench_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_workflow_bench_args(args)?;
    let report = build_action_workflow_bench_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        &config.cache_bench_report_path,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_workflow_bench_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  workflow_bench_kind: {}", report.workflow_bench_kind);
    println!("  report_path: {}", config.report_path.display());
    print_phase_action_workflow_bench_report(&report);

    if !report.gate_pass() {
        return Err(String::from("phase action workflow bench v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_workflow_bench_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_workflow_bench_args(args)?;
    let report = read_action_workflow_bench_report(&config.report_path)?;
    let rebuilt_report = build_action_workflow_bench_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        &config.cache_bench_report_path,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_workflow_bench_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_workflow_bench_verify_v1_verdict(gate_pass)
    );
    println!("  report_path: {}", config.report_path.display());
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    print_phase_action_workflow_bench_report(&report);

    if !gate_pass {
        return Err(String::from(
            "phase action workflow bench verify v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_workflow_replay_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_workflow_replay_args(args)?;
    let report = build_action_workflow_replay_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        config.margin_threshold_micro,
        config.workflow_sessions,
        config.steps_per_session,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_workflow_replay_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  workflow_replay_kind: {}", report.workflow_replay_kind);
    println!("  report_path: {}", config.report_path.display());
    print_phase_action_workflow_replay_report(&report);

    if !report.gate_pass() {
        return Err(String::from("phase action workflow replay v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_workflow_replay_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_workflow_replay_args(args)?;
    let report = read_action_workflow_replay_report(&config.report_path)?;
    let rebuilt_report = build_action_workflow_replay_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        report.margin_threshold_micro,
        report.workflow_sessions,
        report.steps_per_session,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_workflow_replay_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_workflow_replay_verify_v1_verdict(gate_pass)
    );
    println!("  report_path: {}", config.report_path.display());
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    print_phase_action_workflow_replay_report(&report);

    if !gate_pass {
        return Err(String::from(
            "phase action workflow replay verify v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_strict_multiseed_rust_audit_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_strict_multiseed_rust_audit_args(args)?;
    let report = build_strict_multiseed_rust_audit_report(&config.diagnostics_root_path)?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_strict_multiseed_rust_audit_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  audit_kind: {}", report.audit_kind);
    println!("  report_path: {}", config.report_path.display());
    print_strict_multiseed_rust_audit_report(&report);
    Ok(())
}

pub(crate) fn run_strict_multiseed_rust_audit_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_strict_multiseed_rust_audit_args(args)?;
    let report = read_strict_multiseed_rust_audit_report(&config.report_path)?;
    let rebuilt_report = build_strict_multiseed_rust_audit_report(&config.diagnostics_root_path)?;
    let report_matches_sources = report == rebuilt_report;

    println!("nando_strict_multiseed_rust_audit_verify_v1:");
    println!(
        "  verdict: {}",
        strict_multiseed_rust_audit_verify_v1_verdict(report_matches_sources)
    );
    println!("  report_path: {}", config.report_path.display());
    println!("  report_matches_sources: {report_matches_sources}");
    print_strict_multiseed_rust_audit_report(&report);

    if !report_matches_sources {
        return Err(String::from(
            "strict multiseed rust audit report does not match current logs",
        ));
    }
    Ok(())
}

fn print_phase_action_workflow_bench_report(report: &PhaseActionWorkflowBenchReport) {
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  license_report_matches_sources: {}",
        report.license_report_matches_sources
    );
    println!(
        "  cache_bench_report_matches_sources: {}",
        report.cache_bench_report_matches_sources
    );
    println!(
        "  workflow_artifact_label: {}",
        report.workflow_artifact_label
    );
    println!(
        "  workflow_artifact_found: {}",
        report.workflow_artifact_found
    );
    println!(
        "  workflow_source_rebuild_action_tree_key_count: {}",
        report.workflow_source_rebuild_action_tree_key_count
    );
    println!(
        "  workflow_unique_eval_rows: {}",
        report.workflow_unique_eval_rows
    );
    println!(
        "  workflow_simulated_calls: {}",
        report.workflow_simulated_calls
    );
    println!(
        "  workflow_exact_cache_llm_calls: {}",
        report.workflow_exact_cache_llm_calls
    );
    println!(
        "  workflow_exact_cache_plus_nando_llm_calls: {}",
        report.workflow_exact_cache_plus_nando_llm_calls
    );
    println!(
        "  workflow_nando_local_operator_calls: {}",
        report.workflow_nando_local_operator_calls
    );
    println!(
        "  workflow_incremental_llm_calls_removed_vs_cache: {}",
        report.workflow_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_incremental_llm_call_reduction_vs_cache_milli: {}",
        report.workflow_incremental_llm_call_reduction_vs_cache_milli
    );
    println!(
        "  workflow_local_accuracy_milli: {}",
        report.workflow_local_accuracy_milli
    );
    println!(
        "  workflow_false_local_accepts: {}",
        report.workflow_false_local_accepts
    );
    println!(
        "  workflow_bench_p99_latency_ns: {}",
        report.workflow_bench_p99_latency_ns
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!("  workflow_boundary: {}", report.workflow_boundary);
}

fn print_phase_action_workflow_replay_report(report: &PhaseActionWorkflowReplayReport) {
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  license_report_matches_sources: {}",
        report.license_report_matches_sources
    );
    println!(
        "  margin_threshold_micro: {}",
        report.margin_threshold_micro
    );
    println!("  workflow_sessions: {}", report.workflow_sessions);
    println!("  steps_per_session: {}", report.steps_per_session);
    println!("  workflow_trace_calls: {}", report.workflow_trace_calls);
    println!("  package_aliases: {}", report.package_aliases.join(", "));
    println!("  package_count: {}", report.package_count);
    println!("  all_packages_observed: {}", report.all_packages_observed);
    println!(
        "  sessions_cover_all_packages: {}",
        report.sessions_cover_all_packages
    );
    println!(
        "  total_unique_eval_rows: {}",
        report.total_unique_eval_rows
    );
    println!("  replay_unique_rows: {}", report.replay_unique_rows);
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  nando_local_operator_calls: {}",
        report.nando_local_operator_calls
    );
    println!("  nando_fallback_events: {}", report.nando_fallback_events);
    println!(
        "  incremental_llm_calls_removed_vs_cache: {}",
        report.incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  incremental_llm_call_reduction_vs_cache_milli: {}",
        report.incremental_llm_call_reduction_vs_cache_milli
    );
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    for artifact in &report.artifacts {
        println!(
            "  artifact.{}.trace_calls: {}",
            artifact.label, artifact.trace_calls
        );
        println!(
            "  artifact.{}.unique_replayed_rows: {}",
            artifact.label, artifact.unique_replayed_rows
        );
        println!(
            "  artifact.{}.local_operator_calls: {}",
            artifact.label, artifact.local_operator_calls
        );
        println!(
            "  artifact.{}.fallback_to_llm_calls: {}",
            artifact.label, artifact.fallback_to_llm_calls
        );
    }
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!("  workflow_boundary: {}", report.workflow_boundary);
}

fn print_strict_multiseed_rust_audit_report(report: &StrictMultiSeedRustAuditReport) {
    println!("  gate_pass: {}", report.gate_pass);
    println!("  diagnostics_root_path: {}", report.diagnostics_root_path);
    println!("  expected_seeds: {:?}", report.expected_seeds);
    println!("  expected_classes: {:?}", report.expected_classes);
    println!("  observed_logs: {}", report.observed_logs);
    println!("  missing_logs: {}", report.missing_logs.len());
    println!(
        "  strict_runtime_issues: {}",
        report.strict_runtime_issues.len()
    );
    for issue in report.strict_runtime_issues.iter().take(12) {
        println!("    issue: {issue}");
    }
    if report.strict_runtime_issues.len() > 12 {
        println!(
            "    issue: ... {} more",
            report.strict_runtime_issues.len() - 12
        );
    }
    println!("  evidence_warnings: {}", report.evidence_warnings.len());
    for warning in report.evidence_warnings.iter().take(12) {
        println!("    warning: {warning}");
    }
    if report.evidence_warnings.len() > 12 {
        println!(
            "    warning: ... {} more",
            report.evidence_warnings.len() - 12
        );
    }
    println!("  logs_fingerprint64: {}", report.logs_fingerprint64);
    println!("  logs_total_bytes: {}", report.logs_total_bytes);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!(
        "  rust_runtime_logs_used: {}",
        report.rust_runtime_logs_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);
}

fn print_phase_action_cache_offload_bench_report(report: &PhaseActionCacheOffloadBenchReport) {
    println!(
        "  margin_threshold_micro: {}",
        report.margin_threshold_micro
    );
    println!("  simulated_calls: {}", report.simulated_calls);
    println!("  no_cache_llm_calls: {}", report.no_cache_llm_calls);
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!(
        "  exact_cache_hit_rate_milli: {}",
        report.exact_cache_hit_rate_milli
    );
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  nando_local_operator_calls: {}",
        report.nando_local_operator_calls
    );
    println!("  nando_fallback_events: {}", report.nando_fallback_events);
    println!(
        "  nando_operator_hit_rate_milli: {}",
        report.nando_operator_hit_rate_milli
    );
    println!(
        "  incremental_llm_calls_removed_vs_cache: {}",
        report.incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  incremental_llm_call_reduction_vs_cache_milli: {}",
        report.incremental_llm_call_reduction_vs_cache_milli
    );
    println!(
        "  token_units_removed_vs_cache: {}",
        report.token_units_removed_vs_cache
    );
    println!(
        "  cost_units_removed_vs_cache: {}",
        report.cost_units_removed_vs_cache
    );
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  release_suite_gate_pass: {}",
        report.release_suite_gate_pass
    );
    println!(
        "  release_suite_matches_sources: {}",
        report.release_suite_matches_sources
    );
    println!(
        "  license_package_gate_pass: {}",
        report.license_package_gate_pass
    );
    println!(
        "  license_report_matches_sources: {}",
        report.license_report_matches_sources
    );
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        report.eval_task_package_used
    );
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!("  offload_policy_api: {}", report.offload_policy_api);
    println!("  cache_baseline_policy: {}", report.cache_baseline_policy);
    for artifact in &report.artifacts {
        println!(
            "  artifact.{}.exact_cache_llm_calls: {}",
            artifact.label, artifact.exact_cache_llm_calls
        );
        println!(
            "  artifact.{}.nando_plus_cache_llm_calls: {}",
            artifact.label, artifact.nando_plus_cache_llm_calls
        );
        println!(
            "  artifact.{}.incremental_llm_calls_removed_vs_cache: {}",
            artifact.label, artifact.incremental_llm_calls_removed_vs_cache
        );
    }
    println!("  product_boundary: {}", report.product_boundary);
}

pub(crate) fn run_phase_action_regression_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_regression_args(args)?;
    let report = build_action_regression_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        &config.offload_report_path,
        &config.cache_bench_report_path,
        &config.workflow_bench_report_path,
        &config.workflow_replay_report_path,
    )?;
    write_json_file(&config.report_path, &report)?;

    println!("nando_phase_action_regression_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  regression_kind: {}", report.regression_kind);
    println!("  report_path: {}", config.report_path.display());
    println!(
        "  release_suite_report_path: {}",
        config.release_suite_report_path.display()
    );
    println!(
        "  release_suite_report_fingerprint64: {}",
        report.release_suite_report_fingerprint64
    );
    println!(
        "  release_suite_report_bytes: {}",
        report.release_suite_report_bytes
    );
    println!(
        "  license_package_report_path: {}",
        config.license_report_path.display()
    );
    println!(
        "  license_package_report_fingerprint64: {}",
        report.license_package_report_fingerprint64
    );
    println!(
        "  license_package_report_bytes: {}",
        report.license_package_report_bytes
    );
    println!(
        "  offload_audit_report_path: {}",
        config.offload_report_path.display()
    );
    println!(
        "  offload_audit_report_fingerprint64: {}",
        report.offload_audit_report_fingerprint64
    );
    println!(
        "  offload_audit_report_bytes: {}",
        report.offload_audit_report_bytes
    );
    println!(
        "  cache_offload_bench_report_path: {}",
        config.cache_bench_report_path.display()
    );
    println!(
        "  cache_offload_bench_report_fingerprint64: {}",
        report.cache_offload_bench_report_fingerprint64
    );
    println!(
        "  cache_offload_bench_report_bytes: {}",
        report.cache_offload_bench_report_bytes
    );
    println!(
        "  workflow_bench_report_path: {}",
        config.workflow_bench_report_path.display()
    );
    println!(
        "  workflow_bench_report_fingerprint64: {}",
        report.workflow_bench_report_fingerprint64
    );
    println!(
        "  workflow_bench_report_bytes: {}",
        report.workflow_bench_report_bytes
    );
    println!(
        "  workflow_replay_report_path: {}",
        config.workflow_replay_report_path.display()
    );
    println!(
        "  workflow_replay_report_fingerprint64: {}",
        report.workflow_replay_report_fingerprint64
    );
    println!(
        "  workflow_replay_report_bytes: {}",
        report.workflow_replay_report_bytes
    );
    println!("  release_verify_pass: {}", report.release_verify_pass);
    println!("  license_verify_pass: {}", report.license_verify_pass);
    println!("  offload_verify_pass: {}", report.offload_verify_pass);
    println!(
        "  cache_bench_verify_pass: {}",
        report.cache_bench_verify_pass
    );
    println!(
        "  workflow_bench_verify_pass: {}",
        report.workflow_bench_verify_pass
    );
    println!(
        "  workflow_replay_verify_pass: {}",
        report.workflow_replay_verify_pass
    );
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!("  total_bench_samples: {}", report.total_bench_samples);
    println!(
        "  total_source_verify_report_bytes: {}",
        report.total_source_verify_report_bytes
    );
    println!(
        "  total_shortcut_report_bytes: {}",
        report.total_shortcut_report_bytes
    );
    println!(
        "  all_source_verify_reports_pass: {}",
        report.all_source_verify_reports_pass
    );
    println!(
        "  all_shortcut_reports_pass: {}",
        report.all_shortcut_reports_pass
    );
    println!(
        "  all_action_ablation_collapses: {}",
        report.all_action_ablation_collapses
    );
    println!(
        "  all_action_contract_source_rebuild_clean: {}",
        report.all_action_contract_source_rebuild_clean
    );
    println!(
        "  all_optimized_build_reports_pass: {}",
        report.all_optimized_build_reports_pass
    );
    println!(
        "  total_source_rebuild_accepted_action_tree_rows: {}",
        report.total_source_rebuild_accepted_action_tree_rows
    );
    println!(
        "  total_source_rebuild_rejected_action_tree_rows: {}",
        report.total_source_rebuild_rejected_action_tree_rows
    );
    println!(
        "  total_source_rebuild_forbidden_contract_rows: {}",
        report.total_source_rebuild_forbidden_contract_rows
    );
    println!(
        "  total_source_rebuild_action_tree_key_count: {}",
        report.total_source_rebuild_action_tree_key_count
    );
    println!(
        "  min_source_rebuild_action_tree_key_count: {}",
        report.min_source_rebuild_action_tree_key_count
    );
    println!(
        "  all_action_tree_key_coverage_pass: {}",
        report.all_action_tree_key_coverage_pass
    );
    println!(
        "  all_package_report_parity_pass: {}",
        report.all_package_report_parity_pass
    );
    println!(
        "  all_manifest_package_parity_pass: {}",
        report.all_manifest_package_parity_pass
    );
    println!(
        "  all_eval_pack_package_parity_pass: {}",
        report.all_eval_pack_package_parity_pass
    );
    println!(
        "  all_score_report_package_parity_pass: {}",
        report.all_score_report_package_parity_pass
    );
    println!(
        "  all_bench_report_package_parity_pass: {}",
        report.all_bench_report_package_parity_pass
    );
    println!(
        "  all_product_report_package_parity_pass: {}",
        report.all_product_report_package_parity_pass
    );
    println!(
        "  all_source_rebuild_package_parity_pass: {}",
        report.all_source_rebuild_package_parity_pass
    );
    println!(
        "  all_source_verify_report_package_parity_pass: {}",
        report.all_source_verify_report_package_parity_pass
    );
    println!(
        "  max_score_action_ablation_accuracy_milli: {}",
        report.max_score_action_ablation_accuracy_milli
    );
    println!(
        "  max_bench_action_ablation_accuracy_milli: {}",
        report.max_bench_action_ablation_accuracy_milli
    );
    println!(
        "  total_score_action_ablation_wrong_wins: {}",
        report.total_score_action_ablation_wrong_wins
    );
    println!(
        "  total_bench_action_ablation_wrong_wins: {}",
        report.total_bench_action_ablation_wrong_wins
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  offload_rate_milli: {}", report.offload_rate_milli);
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  cache_bench_verdict: {}", report.cache_bench_verdict);
    println!(
        "  cache_incremental_llm_calls_removed_vs_cache: {}",
        report.cache_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  cache_exact_cache_llm_calls: {}",
        report.cache_exact_cache_llm_calls
    );
    println!(
        "  cache_exact_cache_plus_nando_llm_calls: {}",
        report.cache_exact_cache_plus_nando_llm_calls
    );
    println!(
        "  workflow_incremental_llm_calls_removed_vs_cache: {}",
        report.workflow_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_exact_cache_llm_calls: {}",
        report.workflow_exact_cache_llm_calls
    );
    println!(
        "  workflow_exact_cache_plus_nando_llm_calls: {}",
        report.workflow_exact_cache_plus_nando_llm_calls
    );
    println!(
        "  workflow_local_accuracy_milli: {}",
        report.workflow_local_accuracy_milli
    );
    println!(
        "  workflow_false_local_accepts: {}",
        report.workflow_false_local_accepts
    );
    println!(
        "  workflow_replay_incremental_llm_calls_removed_vs_cache: {}",
        report.workflow_replay_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_replay_exact_cache_llm_calls: {}",
        report.workflow_replay_exact_cache_llm_calls
    );
    println!(
        "  workflow_replay_exact_cache_plus_nando_llm_calls: {}",
        report.workflow_replay_exact_cache_plus_nando_llm_calls
    );
    println!(
        "  workflow_replay_local_accuracy_milli: {}",
        report.workflow_replay_local_accuracy_milli
    );
    println!(
        "  workflow_replay_false_local_accepts: {}",
        report.workflow_replay_false_local_accepts
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!(
        "  offload_sdk_inspect_api: {}",
        report.offload_sdk_inspect_api
    );
    println!(
        "  offload_runtime_summary_api: {}",
        report.offload_runtime_summary_api
    );
    println!(
        "  operator_blueprint_path: {}",
        report.operator_blueprint_path
    );
    println!(
        "  operator_blueprint_fingerprint64: {}",
        report.operator_blueprint_fingerprint64
    );
    println!(
        "  operator_blueprint_formula_present: {}",
        report.operator_blueprint_formula_present
    );
    println!(
        "  operator_blueprint_runtime_package_contract_present: {}",
        report.operator_blueprint_runtime_package_contract_present
    );
    println!(
        "  operator_blueprint_source_verify_contract_present: {}",
        report.operator_blueprint_source_verify_contract_present
    );
    println!(
        "  operator_blueprint_shortcut_report_contract_present: {}",
        report.operator_blueprint_shortcut_report_contract_present
    );
    println!(
        "  operator_blueprint_rust_proof_path_present: {}",
        report.operator_blueprint_rust_proof_path_present
    );
    println!(
        "  operator_blueprint_forbidden_invariants_present: {}",
        report.operator_blueprint_forbidden_invariants_present
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !report.gate_pass() {
        return Err(String::from("phase action regression v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_regression_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_regression_verify_args(args)?;
    let report = read_action_regression_report(&config.report_path)?;
    let rebuilt_report = build_action_regression_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        &config.offload_report_path,
        &config.cache_bench_report_path,
        &config.workflow_bench_report_path,
        &config.workflow_replay_report_path,
    )?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_regression_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_regression_verify_v1_verdict(gate_pass)
    );
    println!("  report_path: {}", config.report_path.display());
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    println!(
        "  release_suite_report_fingerprint64: {}",
        report.release_suite_report_fingerprint64
    );
    println!(
        "  release_suite_report_bytes: {}",
        report.release_suite_report_bytes
    );
    println!(
        "  license_package_report_fingerprint64: {}",
        report.license_package_report_fingerprint64
    );
    println!(
        "  license_package_report_bytes: {}",
        report.license_package_report_bytes
    );
    println!(
        "  offload_audit_report_fingerprint64: {}",
        report.offload_audit_report_fingerprint64
    );
    println!(
        "  offload_audit_report_bytes: {}",
        report.offload_audit_report_bytes
    );
    println!(
        "  cache_offload_bench_report_fingerprint64: {}",
        report.cache_offload_bench_report_fingerprint64
    );
    println!(
        "  cache_offload_bench_report_bytes: {}",
        report.cache_offload_bench_report_bytes
    );
    println!(
        "  workflow_bench_report_fingerprint64: {}",
        report.workflow_bench_report_fingerprint64
    );
    println!(
        "  workflow_bench_report_bytes: {}",
        report.workflow_bench_report_bytes
    );
    println!(
        "  workflow_replay_report_fingerprint64: {}",
        report.workflow_replay_report_fingerprint64
    );
    println!(
        "  workflow_replay_report_bytes: {}",
        report.workflow_replay_report_bytes
    );
    println!("  release_verify_pass: {}", report.release_verify_pass);
    println!("  license_verify_pass: {}", report.license_verify_pass);
    println!("  offload_verify_pass: {}", report.offload_verify_pass);
    println!(
        "  cache_bench_verify_pass: {}",
        report.cache_bench_verify_pass
    );
    println!(
        "  workflow_bench_verify_pass: {}",
        report.workflow_bench_verify_pass
    );
    println!(
        "  workflow_replay_verify_pass: {}",
        report.workflow_replay_verify_pass
    );
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!("  total_bench_samples: {}", report.total_bench_samples);
    println!(
        "  total_source_verify_report_bytes: {}",
        report.total_source_verify_report_bytes
    );
    println!(
        "  total_shortcut_report_bytes: {}",
        report.total_shortcut_report_bytes
    );
    println!(
        "  all_source_verify_reports_pass: {}",
        report.all_source_verify_reports_pass
    );
    println!(
        "  all_shortcut_reports_pass: {}",
        report.all_shortcut_reports_pass
    );
    println!(
        "  all_action_ablation_collapses: {}",
        report.all_action_ablation_collapses
    );
    println!(
        "  all_action_contract_source_rebuild_clean: {}",
        report.all_action_contract_source_rebuild_clean
    );
    println!(
        "  all_optimized_build_reports_pass: {}",
        report.all_optimized_build_reports_pass
    );
    println!(
        "  total_source_rebuild_accepted_action_tree_rows: {}",
        report.total_source_rebuild_accepted_action_tree_rows
    );
    println!(
        "  total_source_rebuild_rejected_action_tree_rows: {}",
        report.total_source_rebuild_rejected_action_tree_rows
    );
    println!(
        "  total_source_rebuild_forbidden_contract_rows: {}",
        report.total_source_rebuild_forbidden_contract_rows
    );
    println!(
        "  total_source_rebuild_action_tree_key_count: {}",
        report.total_source_rebuild_action_tree_key_count
    );
    println!(
        "  min_source_rebuild_action_tree_key_count: {}",
        report.min_source_rebuild_action_tree_key_count
    );
    println!(
        "  all_action_tree_key_coverage_pass: {}",
        report.all_action_tree_key_coverage_pass
    );
    println!(
        "  all_package_report_parity_pass: {}",
        report.all_package_report_parity_pass
    );
    println!(
        "  all_manifest_package_parity_pass: {}",
        report.all_manifest_package_parity_pass
    );
    println!(
        "  all_eval_pack_package_parity_pass: {}",
        report.all_eval_pack_package_parity_pass
    );
    println!(
        "  all_score_report_package_parity_pass: {}",
        report.all_score_report_package_parity_pass
    );
    println!(
        "  all_bench_report_package_parity_pass: {}",
        report.all_bench_report_package_parity_pass
    );
    println!(
        "  all_product_report_package_parity_pass: {}",
        report.all_product_report_package_parity_pass
    );
    println!(
        "  all_source_rebuild_package_parity_pass: {}",
        report.all_source_rebuild_package_parity_pass
    );
    println!(
        "  all_source_verify_report_package_parity_pass: {}",
        report.all_source_verify_report_package_parity_pass
    );
    println!(
        "  max_score_action_ablation_accuracy_milli: {}",
        report.max_score_action_ablation_accuracy_milli
    );
    println!(
        "  max_bench_action_ablation_accuracy_milli: {}",
        report.max_bench_action_ablation_accuracy_milli
    );
    println!(
        "  total_score_action_ablation_wrong_wins: {}",
        report.total_score_action_ablation_wrong_wins
    );
    println!(
        "  total_bench_action_ablation_wrong_wins: {}",
        report.total_bench_action_ablation_wrong_wins
    );
    println!("  offload_rate_milli: {}", report.offload_rate_milli);
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  cache_bench_verdict: {}", report.cache_bench_verdict);
    println!(
        "  cache_incremental_llm_calls_removed_vs_cache: {}",
        report.cache_incremental_llm_calls_removed_vs_cache
    );
    println!("  compiler_used: {}", report.compiler_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!("  offload_sdk_api: {}", report.offload_sdk_api);
    println!(
        "  offload_sdk_inspect_api: {}",
        report.offload_sdk_inspect_api
    );
    println!(
        "  offload_runtime_summary_api: {}",
        report.offload_runtime_summary_api
    );
    println!(
        "  operator_blueprint_path: {}",
        report.operator_blueprint_path
    );
    println!(
        "  operator_blueprint_fingerprint64: {}",
        report.operator_blueprint_fingerprint64
    );
    println!(
        "  operator_blueprint_formula_present: {}",
        report.operator_blueprint_formula_present
    );
    println!(
        "  operator_blueprint_runtime_package_contract_present: {}",
        report.operator_blueprint_runtime_package_contract_present
    );
    println!(
        "  operator_blueprint_source_verify_contract_present: {}",
        report.operator_blueprint_source_verify_contract_present
    );
    println!(
        "  operator_blueprint_shortcut_report_contract_present: {}",
        report.operator_blueprint_shortcut_report_contract_present
    );
    println!(
        "  operator_blueprint_rust_proof_path_present: {}",
        report.operator_blueprint_rust_proof_path_present
    );
    println!(
        "  operator_blueprint_forbidden_invariants_present: {}",
        report.operator_blueprint_forbidden_invariants_present
    );

    if !gate_pass {
        return Err(String::from(
            "phase action regression verify v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_regression_freeze_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_regression_freeze_args(args)?;
    let report = build_action_regression_freeze_report(&config)?;
    write_json_file(&config.freeze_report_path, &report)?;

    println!("nando_phase_action_regression_freeze_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  freeze_kind: {}", report.freeze_kind);
    println!(
        "  freeze_report_path: {}",
        config.freeze_report_path.display()
    );
    print_phase_action_regression_freeze_report(&report);

    if !report.gate_pass() {
        return Err(String::from(
            "phase action regression freeze v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_regression_freeze_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_regression_freeze_args(args)?;
    let report = read_action_regression_freeze_report(&config.freeze_report_path)?;
    let rebuilt_report = build_action_regression_freeze_report(&config)?;
    let report_matches_sources = report == rebuilt_report;
    let gate_pass = report.gate_pass() && report_matches_sources;

    println!("nando_phase_action_regression_freeze_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_regression_freeze_verify_v1_verdict(gate_pass)
    );
    println!(
        "  freeze_report_path: {}",
        config.freeze_report_path.display()
    );
    println!("  report_gate_pass: {}", report.gate_pass());
    println!("  report_matches_sources: {report_matches_sources}");
    print_phase_action_regression_freeze_report(&report);

    if !gate_pass {
        return Err(String::from(
            "phase action regression freeze verify v1 gate failed",
        ));
    }
    Ok(())
}

fn print_phase_action_regression_freeze_report(report: &PhaseActionRegressionFreezeReport) {
    println!(
        "  regression_report_path: {}",
        report.regression_report_path
    );
    println!(
        "  regression_report_fingerprint64: {}",
        report.regression_report_fingerprint64
    );
    println!(
        "  regression_report_bytes: {}",
        report.regression_report_bytes
    );
    println!("  regression_verdict: {}", report.regression_verdict);
    println!("  regression_gate_pass: {}", report.regression_gate_pass);
    println!(
        "  regression_matches_sources: {}",
        report.regression_matches_sources
    );
    println!(
        "  release_suite_report_fingerprint64: {}",
        report.release_suite_report_fingerprint64
    );
    println!(
        "  license_package_report_fingerprint64: {}",
        report.license_package_report_fingerprint64
    );
    println!(
        "  offload_audit_report_fingerprint64: {}",
        report.offload_audit_report_fingerprint64
    );
    println!(
        "  cache_offload_bench_report_fingerprint64: {}",
        report.cache_offload_bench_report_fingerprint64
    );
    println!(
        "  cache_offload_bench_report_bytes: {}",
        report.cache_offload_bench_report_bytes
    );
    println!(
        "  workflow_bench_report_fingerprint64: {}",
        report.workflow_bench_report_fingerprint64
    );
    println!(
        "  workflow_bench_report_bytes: {}",
        report.workflow_bench_report_bytes
    );
    println!(
        "  cache_bench_verify_pass: {}",
        report.cache_bench_verify_pass
    );
    println!(
        "  cache_bench_report_matches_sources: {}",
        report.cache_bench_report_matches_sources
    );
    println!(
        "  workflow_bench_verify_pass: {}",
        report.workflow_bench_verify_pass
    );
    println!(
        "  workflow_bench_report_matches_sources: {}",
        report.workflow_bench_report_matches_sources
    );
    println!(
        "  workflow_replay_verify_pass: {}",
        report.workflow_replay_verify_pass
    );
    println!(
        "  workflow_replay_report_matches_sources: {}",
        report.workflow_replay_report_matches_sources
    );
    println!(
        "  cache_incremental_llm_calls_removed_vs_cache: {}",
        report.cache_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_incremental_llm_calls_removed_vs_cache: {}",
        report.workflow_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_replay_incremental_llm_calls_removed_vs_cache: {}",
        report.workflow_replay_incremental_llm_calls_removed_vs_cache
    );
    println!(
        "  workflow_replay_exact_cache_llm_calls: {}",
        report.workflow_replay_exact_cache_llm_calls
    );
    println!(
        "  workflow_replay_exact_cache_plus_nando_llm_calls: {}",
        report.workflow_replay_exact_cache_plus_nando_llm_calls
    );
    println!(
        "  workflow_replay_local_accuracy_milli: {}",
        report.workflow_replay_local_accuracy_milli
    );
    println!(
        "  workflow_replay_false_local_accepts: {}",
        report.workflow_replay_false_local_accepts
    );
    println!(
        "  operator_blueprint_fingerprint64: {}",
        report.operator_blueprint_fingerprint64
    );
    println!("  artifact_count: {}", report.artifact_count);
    println!(
        "  total_runtime_bytes_estimate: {}",
        report.total_runtime_bytes_estimate
    );
    println!("  total_bench_samples: {}", report.total_bench_samples);
    println!(
        "  all_package_report_parity_pass: {}",
        report.all_package_report_parity_pass
    );
    println!(
        "  all_action_contract_source_rebuild_clean: {}",
        report.all_action_contract_source_rebuild_clean
    );
    println!(
        "  total_source_rebuild_action_tree_key_count: {}",
        report.total_source_rebuild_action_tree_key_count
    );
    println!(
        "  min_source_rebuild_action_tree_key_count: {}",
        report.min_source_rebuild_action_tree_key_count
    );
    println!(
        "  all_action_tree_key_coverage_pass: {}",
        report.all_action_tree_key_coverage_pass
    );
    println!(
        "  all_optimized_build_reports_pass: {}",
        report.all_optimized_build_reports_pass
    );
    println!(
        "  max_bench_p99_latency_ns: {}",
        report.max_bench_p99_latency_ns
    );
    println!("  offload_rate_milli: {}", report.offload_rate_milli);
    println!("  local_accuracy_milli: {}", report.local_accuracy_milli);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  compiler_used: {}", report.compiler_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  forbidden_used: {}", report.forbidden_used);
    println!(
        "  state_transition_formula: {}",
        report.state_transition_formula
    );
    println!("  claim_boundary: {}", report.claim_boundary);
}

pub(crate) fn run_phase_package_score_v4(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_package_score_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_manifest(&config.manifest_path)?;
    validate_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    let key_to_index = manifest_key_to_index(&manifest)?;
    let rows = load_phase_operator_rows(&config.corpus_path)?;
    let prepared = prepare_eval_tasks(&rows, manifest.cells, &key_to_index);
    let eval = eval_loaded_runtime(&runtime, &prepared.tasks)?;
    let action_ablation_eval = eval_loaded_runtime(&runtime, &prepared.action_ablation_tasks)?;
    let forbidden_used = manifest.forbidden_flags.any_forbidden_used();
    let gate_pass = score_v4_gate_pass(&eval, &prepared, &action_ablation_eval, forbidden_used);
    let verdict = phase_package_score_v4_verdict(gate_pass);
    let report = PhasePackageScoreReport::from_score(PhasePackageScoreReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        corpus_path: &config.corpus_path,
        eval_task_package_path: None,
        package_info,
        package_bytes_len: package_bytes.len(),
        runtime: &runtime,
        manifest: &manifest,
        rows: rows.len(),
        prepared: &prepared,
        eval,
        action_ablation_eval,
        compiler_used: false,
        eval_task_package_used: false,
        corpus_jsonl_used_in_score_loop: false,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_package_score_v4:");
    println!("  verdict: {verdict}");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  cells: {}", manifest.cells);
    println!("  flat_records: {}", runtime.record_count());
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  rows: {}", rows.len());
    println!("  heldout_eval_rows: {}", prepared.tasks.len());
    println!("  missing_centers: {}", prepared.missing_centers);
    println!("  skipped_rows: {}", prepared.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        prepared.action_ablation_tasks.len()
    );
    println!(
        "  action_ablation_missing_centers: {}",
        prepared.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", eval.accuracy_milli);
    println!("  wrong_wins: {}", eval.wrong_wins);
    println!("  median_margin: {:.6}", eval.median_margin);
    println!("  p10_margin: {:.6}", eval.p10_margin);
    println!("  p50_latency_ns: {}", eval.p50_latency_ns);
    println!("  p99_latency_ns: {}", eval.p99_latency_ns);
    println!("  total_eval_us: {}", eval.total_eval_us);
    println!("  rows_per_second: {:.2}", eval.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        action_ablation_eval.accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        action_ablation_eval.wrong_wins
    );
    println!(
        "  action_ablation_median_margin: {:.6}",
        action_ablation_eval.median_margin
    );
    println!(
        "  action_ablation_p10_margin: {:.6}",
        action_ablation_eval.p10_margin
    );
    println!("  compiler_used: false");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    if let Some(report_path) = &config.report_path {
        println!("  score_report_path: {}", report_path.display());
    }
    println!(
        "  target_center_id_training_used: {}",
        manifest.forbidden_flags.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        manifest
            .forbidden_flags
            .proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        manifest.forbidden_flags.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        manifest.forbidden_flags.local_out_t_runtime_extension_used
    );

    if !gate_pass {
        return Err(String::from("phase package score v4 gate failed"));
    }

    Ok(())
}

pub(crate) fn run_phase_eval_pack_v4(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_eval_pack_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_manifest(&config.manifest_path)?;
    validate_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    let key_to_index = manifest_key_to_index(&manifest)?;
    let rows = load_phase_operator_rows(&config.corpus_path)?;
    let prepared = prepare_eval_tasks(&rows, manifest.cells, &key_to_index);
    let eval_package = PhaseEvalTaskPackage::from_prepared(
        manifest.cells,
        package_info.fingerprint64,
        rows.len(),
        prepared,
    );
    let bytes = eval_package.to_bytes()?;
    write_package(&config.eval_pack_path, &bytes)?;
    let loaded = read_eval_task_package(&config.eval_pack_path)?;
    let gate_pass = eval_pack_v4_gate_pass(&loaded, &package_info, &manifest);

    println!("nando_phase_eval_pack_v4:");
    println!("  verdict: {}", phase_eval_pack_v4_verdict(gate_pass));
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  eval_pack_magic: {:?}", PHASE_EVAL_TASK_PACKAGE_MAGIC);
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!(
        "  eval_pack_package_fingerprint64: {}",
        loaded.package_fingerprint64
    );
    println!("  cells: {}", loaded.cells);
    println!("  rows: {}", loaded.rows);
    println!("  heldout_eval_rows: {}", loaded.prepared.tasks.len());
    println!(
        "  action_ablation_eval_rows: {}",
        loaded.prepared.action_ablation_tasks.len()
    );
    println!("  missing_centers: {}", loaded.prepared.missing_centers);
    println!("  skipped_rows: {}", loaded.prepared.skipped_rows);
    println!(
        "  action_ablation_missing_centers: {}",
        loaded.prepared.action_ablation_missing_centers
    );
    println!("  eval_pack_bytes: {}", bytes.len());
    println!("  compiler_used: false");
    println!("  jsonl_used_after_pack_build: false");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");

    if !gate_pass {
        return Err(String::from("phase eval pack v4 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_package_score_pack_v4(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_package_score_pack_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_manifest(&config.manifest_path)?;
    validate_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    let eval_package = read_eval_task_package(&config.eval_pack_path)?;
    validate_eval_task_package_match(&eval_package, &manifest, &package_info)?;
    let eval = eval_loaded_runtime(&runtime, &eval_package.prepared.tasks)?;
    let action_ablation_eval =
        eval_loaded_runtime(&runtime, &eval_package.prepared.action_ablation_tasks)?;
    let forbidden_used = manifest.forbidden_flags.any_forbidden_used();
    let gate_pass = score_v4_gate_pass(
        &eval,
        &eval_package.prepared,
        &action_ablation_eval,
        forbidden_used,
    );
    let verdict = phase_package_score_pack_v4_verdict(gate_pass);
    let report = PhasePackageScoreReport::from_score(PhasePackageScoreReportInput {
        verdict,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        corpus_path: Path::new(&manifest.corpus_path),
        eval_task_package_path: Some(&config.eval_pack_path),
        package_info,
        package_bytes_len: package_bytes.len(),
        runtime: &runtime,
        manifest: &manifest,
        rows: eval_package.rows,
        prepared: &eval_package.prepared,
        eval,
        action_ablation_eval,
        compiler_used: false,
        eval_task_package_used: true,
        corpus_jsonl_used_in_score_loop: false,
    });
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_package_score_pack_v4:");
    println!("  verdict: {verdict}");
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  eval_pack_path: {}", config.eval_pack_path.display());
    println!("  cells: {}", manifest.cells);
    println!("  flat_records: {}", runtime.record_count());
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  eval_pack_rows: {}", eval_package.rows);
    println!("  eval_pack_bytes: {}", eval_package.serialized_len());
    println!("  heldout_eval_rows: {}", eval_package.prepared.tasks.len());
    println!(
        "  missing_centers: {}",
        eval_package.prepared.missing_centers
    );
    println!("  skipped_rows: {}", eval_package.prepared.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        eval_package.prepared.action_ablation_tasks.len()
    );
    println!(
        "  action_ablation_missing_centers: {}",
        eval_package.prepared.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", eval.accuracy_milli);
    println!("  wrong_wins: {}", eval.wrong_wins);
    println!("  median_margin: {:.6}", eval.median_margin);
    println!("  p10_margin: {:.6}", eval.p10_margin);
    println!("  p50_latency_ns: {}", eval.p50_latency_ns);
    println!("  p99_latency_ns: {}", eval.p99_latency_ns);
    println!("  total_eval_us: {}", eval.total_eval_us);
    println!("  rows_per_second: {:.2}", eval.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        action_ablation_eval.accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        action_ablation_eval.wrong_wins
    );
    println!("  compiler_used: false");
    println!("  eval_task_package_used: true");
    println!("  corpus_jsonl_used_in_score_loop: false");
    println!("  runtime_path: nando_core::PhaseCenterFlatRuntime");
    if let Some(report_path) = &config.report_path {
        println!("  score_report_path: {}", report_path.display());
    }
    println!(
        "  target_center_id_training_used: {}",
        manifest.forbidden_flags.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        manifest
            .forbidden_flags
            .proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        manifest.forbidden_flags.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        manifest.forbidden_flags.local_out_t_runtime_extension_used
    );

    if !gate_pass {
        return Err(String::from("phase package score-pack v4 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_boundary_v4(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_boundary_args(args)?;
    let rows = load_phase_operator_rows(&config.corpus_path)?;
    let report = PhaseActionBoundaryReport::from_rows(&rows);
    let gate_pass = report.router_gate_pass();

    println!("nando_phase_action_boundary_v4:");
    println!("  verdict: {}", phase_action_boundary_v4_verdict(gate_pass));
    println!("  corpus_path: {}", config.corpus_path.display());
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!(
        "  explicit_operator_class_label_rows: {}",
        report.explicit_operator_class_label_rows
    );
    println!(
        "  explicit_operator_family_label_rows: {}",
        report.explicit_operator_family_label_rows
    );
    println!(
        "  explicit_order_slot_map_rows: {}",
        report.explicit_order_slot_map_rows
    );
    println!(
        "  explicit_branch_slot_map_rows: {}",
        report.explicit_branch_slot_map_rows
    );
    println!(
        "  explicit_source_slot_token_rows: {}",
        report.explicit_source_slot_token_rows
    );
    println!(
        "  literal_marker_parameter_rows: {}",
        report.literal_marker_parameter_rows
    );
    println!(
        "  action_demo_arrow_rows: {}",
        report.action_demo_arrow_rows
    );
    println!(
        "  proof_rule_id_literal_rows: {}",
        report.proof_rule_id_literal_rows
    );
    println!(
        "  target_answer_literal_rows: {}",
        report.target_answer_literal_rows
    );
    println!("  scorer_claim_allowed: true");
    println!("  autonomous_action_router_claim_allowed: {gate_pass}");
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!(
        "  boundary: current v4 action text is an operator-key contract for the scorer, not a green raw action-router corpus"
    );

    if !gate_pass {
        return Err(String::from(
            "phase action boundary v4 gate is WATCH: action labels/slot maps are present",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_corpus_v1(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_action_corpus_args(args)?;
    let rows = generate_action_contract_corpus_v1();
    write_action_contract_jsonl(&config.output_path, &rows)?;
    let report = PhaseActionCorpusReport::from_rows(&config.output_path, &rows);
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }
    let gate_pass = report.gate_pass();

    println!("nando_phase_action_corpus_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  output_path: {}", report.output_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  operator_key_count: {}", report.operator_key_count);
    println!("  same_bag_rows: {}", report.same_bag_rows);
    println!(
        "  duplicate_task_id_rows: {}",
        report.duplicate_task_id_rows
    );
    println!("  min_sequence_len: {}", report.min_sequence_len);
    println!("  max_sequence_len: {}", report.max_sequence_len);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action corpus v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_domain_corpus_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_domain_corpus_args(args)?;
    let rows = generate_domain_action_contract_corpus_v1();
    write_action_contract_jsonl(&config.output_path, &rows)?;
    let mut report = PhaseActionCorpusReport::from_rows(&config.output_path, &rows);
    report.claim_boundary =
        "deterministic workflow-shaped action_contract_v1 corpus factory; not a domain reasoning proof"
            .to_string();
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }
    let gate_pass = report.gate_pass();

    println!("nando_phase_action_domain_corpus_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  output_path: {}", report.output_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  operator_key_count: {}", report.operator_key_count);
    println!("  same_bag_rows: {}", report.same_bag_rows);
    println!(
        "  duplicate_task_id_rows: {}",
        report.duplicate_task_id_rows
    );
    println!("  min_sequence_len: {}", report.min_sequence_len);
    println!("  max_sequence_len: {}", report.max_sequence_len);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: workflow-shaped corpus factory; not a domain reasoning proof");

    if !gate_pass {
        return Err(String::from("phase action domain corpus v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_coverage_corpus_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_coverage_corpus_args(args)?;
    let rows = generate_coverage_action_contract_corpus_v1();
    write_action_contract_jsonl(&config.output_path, &rows)?;
    let mut report = PhaseActionCorpusReport::from_rows(&config.output_path, &rows);
    report.claim_boundary =
        "V5 operator-dimension coverage action_contract_v1 corpus factory; not a runtime proof"
            .to_string();
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }
    let gate_pass = report.gate_pass();

    println!("nando_phase_action_coverage_corpus_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  output_path: {}", report.output_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  operator_key_count: {}", report.operator_key_count);
    println!("  same_bag_rows: {}", report.same_bag_rows);
    println!(
        "  duplicate_task_id_rows: {}",
        report.duplicate_task_id_rows
    );
    println!("  min_sequence_len: {}", report.min_sequence_len);
    println!("  max_sequence_len: {}", report.max_sequence_len);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action coverage corpus v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_contract_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_contract_args(args)?;
    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let report = PhaseActionContractReport::from_rows(&rows);
    let gate_pass = report.gate_pass();

    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_action_contract_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  corpus_path: {}", config.corpus_path.display());
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!(
        "  accepted_action_tree_rows: {}",
        report.accepted_action_tree_rows
    );
    println!("  schema_mismatch_rows: {}", report.schema_mismatch_rows);
    println!("  invalid_split_rows: {}", report.invalid_split_rows);
    println!("  empty_task_id_rows: {}", report.empty_task_id_rows);
    println!(
        "  empty_state_before_rows: {}",
        report.empty_state_before_rows
    );
    println!(
        "  empty_action_select_rows: {}",
        report.empty_action_select_rows
    );
    println!(
        "  empty_action_transform_rows: {}",
        report.empty_action_transform_rows
    );
    println!(
        "  empty_action_write_rows: {}",
        report.empty_action_write_rows
    );
    println!(
        "  empty_action_condition_rows: {}",
        report.empty_action_condition_rows
    );
    println!(
        "  empty_action_check_rows: {}",
        report.empty_action_check_rows
    );
    println!("  empty_correct_rows: {}", report.empty_correct_rows);
    println!("  empty_wrong_rows: {}", report.empty_wrong_rows);
    println!(
        "  identical_correct_wrong_rows: {}",
        report.identical_correct_wrong_rows
    );
    println!(
        "  forbidden_operator_label_rows: {}",
        report.forbidden_operator_label_rows
    );
    println!(
        "  forbidden_slot_map_rows: {}",
        report.forbidden_slot_map_rows
    );
    println!(
        "  forbidden_target_leak_rows: {}",
        report.forbidden_target_leak_rows
    );
    println!(
        "  forbidden_lookup_authority_rows: {}",
        report.forbidden_lookup_authority_rows
    );
    println!(
        "  forbidden_local_out_t_rows: {}",
        report.forbidden_local_out_t_rows
    );
    println!(
        "  forbidden_arrow_demo_rows: {}",
        report.forbidden_arrow_demo_rows
    );
    println!(
        "  concrete_output_token_leak_rows: {}",
        report.concrete_output_token_leak_rows
    );
    println!("  claim_boundary: {}", report.claim_boundary);
    println!("  runtime_or_training_proof: false");
    println!("  python_demo_used: false");

    if !gate_pass {
        return Err(String::from("phase action contract v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_operator_coverage_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_contract_args(args)?;
    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    let report =
        PhaseActionOperatorCoverageReport::from_rows(&config.corpus_path, &rows, &contract_report);
    let gate_pass = report.gate_pass();

    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_action_operator_coverage_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  corpus_path: {}", report.corpus_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  action_tree_key_count: {}", report.action_tree_key_count);
    println!("  select_value_count: {}", report.select_value_count);
    println!("  transform_value_count: {}", report.transform_value_count);
    println!("  write_value_count: {}", report.write_value_count);
    println!("  condition_value_count: {}", report.condition_value_count);
    println!("  check_value_count: {}", report.check_value_count);
    println!(
        "  min_dimension_value_count: {}",
        report.min_dimension_value_count
    );
    println!("  wide_dimension_count: {}", report.wide_dimension_count);
    println!(
        "  train_dimension_coverage_pass: {}",
        report.train_dimension_coverage_pass
    );
    println!(
        "  heldout_dimension_coverage_pass: {}",
        report.heldout_dimension_coverage_pass
    );
    println!(
        "  full_operator_dimension_coverage_pass: {}",
        report.full_operator_dimension_coverage_pass
    );
    println!("  contract_gate_pass: {}", report.contract_gate_pass);
    println!("  label_authority_used: {}", report.label_authority_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from(
            "phase action operator coverage v1 gate failed",
        ));
    }
    Ok(())
}

pub(crate) fn run_phase_action_shortcut_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_shortcut_args(args)?;
    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    if !contract_report.gate_pass() {
        return Err(String::from(
            "phase action shortcut v1 refused dirty action contract",
        ));
    }
    let report = PhaseActionShortcutReport::from_rows(&config.corpus_path, &rows);
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }
    let gate_pass = report.gate_pass();

    println!("nando_phase_action_shortcut_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  corpus_path: {}", report.corpus_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  operator_key_count: {}", report.operator_key_count);
    println!(
        "  heldout_operator_keys_seen_in_train_rows: {}",
        report.heldout_operator_keys_seen_in_train_rows
    );
    println!(
        "  heldout_operator_key_missing_rows: {}",
        report.heldout_operator_key_missing_rows
    );
    println!(
        "  exact_state_lookup_hits: {}",
        report.exact_state_lookup_hits
    );
    println!(
        "  exact_transition_lookup_hits: {}",
        report.exact_transition_lookup_hits
    );
    println!(
        "  heldout_token_overlap_rows: {}",
        report.heldout_token_overlap_rows
    );
    println!(
        "  heldout_length_seen_in_train_rows: {}",
        report.heldout_length_seen_in_train_rows
    );
    println!("  non_same_bag_rows: {}", report.non_same_bag_rows);
    println!(
        "  correct_wrong_identical_rows: {}",
        report.correct_wrong_identical_rows
    );
    println!(
        "  source_bigram_correct_wins: {}",
        report.source_bigram_correct_wins
    );
    println!(
        "  source_bigram_wrong_wins: {}",
        report.source_bigram_wrong_wins
    );
    println!("  source_bigram_ties: {}", report.source_bigram_ties);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        report.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        report.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        report.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action shortcut v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_runtime_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_runtime_args(args)?;
    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    if !contract_report.gate_pass() {
        return Err(String::from(
            "phase action runtime v1 refused dirty action contract",
        ));
    }

    let (runtime, key_to_index, skipped_train_rows) =
        compile_action_contract_runtime(&rows, config.cells)?;
    let prepared = prepare_action_contract_eval(&rows, config.cells, &key_to_index);
    let eval = eval_loaded_runtime(&runtime, &prepared.tasks)?;
    let action_ablation_eval = eval_loaded_runtime(&runtime, &prepared.action_ablation_tasks)?;
    let report = PhaseActionRuntimeReport::from_run(PhaseActionRuntimeReportInput {
        corpus_path: &config.corpus_path,
        cells: config.cells,
        rows: rows.len(),
        train_rows: contract_report.train_rows,
        heldout_rows: contract_report.heldout_rows,
        contract_report,
        runtime: &runtime,
        operator_key_count: key_to_index.len(),
        skipped_train_rows,
        prepared: &prepared,
        eval,
        action_ablation_eval,
    });
    let gate_pass = report.gate_pass();
    if let Some(report_path) = &config.report_path {
        write_json_file(report_path, &report)?;
    }

    println!("nando_phase_action_runtime_v1:");
    println!("  verdict: {}", report.verdict);
    println!("  corpus_path: {}", report.corpus_path);
    if let Some(report_path) = &config.report_path {
        println!("  report_path: {}", report_path.display());
    }
    println!("  rows: {}", report.rows);
    println!("  train_rows: {}", report.train_rows);
    println!("  heldout_rows: {}", report.heldout_rows);
    println!("  cells: {}", report.cells);
    println!("  operator_key_count: {}", report.operator_key_count);
    println!("  flat_records: {}", report.flat_records);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    println!("  skipped_train_rows: {}", report.skipped_train_rows);
    println!("  missing_centers: {}", report.missing_centers);
    println!("  skipped_rows: {}", report.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        report.action_ablation_eval_rows
    );
    println!(
        "  action_ablation_missing_centers: {}",
        report.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", report.accuracy_milli);
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  median_margin: {:.6}", report.median_margin);
    println!("  p10_margin: {:.6}", report.p10_margin);
    println!("  p50_latency_ns: {}", report.p50_latency_ns);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  rows_per_second: {:.2}", report.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        report.action_ablation_wrong_wins
    );
    println!("  contract_verdict: {}", report.contract_verdict);
    println!("  compiler_path: {}", report.compiler_path);
    println!("  runtime_path: {}", report.runtime_path);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  target_center_id_training_used: false");
    println!("  proof_rule_id_training_authority_used: false");
    println!("  concrete_x_lookup_used: false");
    println!("  local_out_t_runtime_extension_used: false");
    println!("  claim_boundary: {}", report.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action runtime v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_action_package_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_args(args)?;
    let (source_contract_fingerprint64, source_contract_bytes) =
        inspect_report_file(&config.corpus_path)?;
    let rows = load_phase_action_contract_rows(&config.corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    if !contract_report.gate_pass() {
        return Err(String::from(
            "phase action package v1 refused dirty action contract",
        ));
    }

    let (runtime, key_to_index, skipped_train_rows) =
        compile_action_contract_runtime(&rows, config.cells)?;
    let package_bytes = runtime.to_bytes().map_err(format_runtime_error)?;
    write_package(&config.package_path, &package_bytes)?;

    let loaded_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&loaded_bytes).map_err(format_runtime_error)?;
    let loaded_runtime =
        PhaseCenterFlatRuntime::from_bytes(&loaded_bytes).map_err(format_runtime_error)?;
    let prepared = prepare_action_contract_eval(&rows, config.cells, &key_to_index);
    let eval = eval_loaded_runtime(&loaded_runtime, &prepared.tasks)?;
    let action_ablation_eval =
        eval_loaded_runtime(&loaded_runtime, &prepared.action_ablation_tasks)?;
    let manifest = PhaseActionPackageManifest::from_run(PhaseActionPackageManifestInput {
        corpus_path: &config.corpus_path,
        source_contract_fingerprint64,
        source_contract_bytes,
        package_path: &config.package_path,
        manifest_path: &config.manifest_path,
        rows: rows.len(),
        train_rows: contract_report.train_rows,
        heldout_rows: contract_report.heldout_rows,
        cells: config.cells,
        key_to_index: &key_to_index,
        skipped_train_rows,
        prepared: &prepared,
        loaded_runtime: &loaded_runtime,
        package_info,
        package_bytes_len: loaded_bytes.len(),
        eval,
        action_ablation_eval,
        contract_verdict: &contract_report.verdict,
    });
    write_json_file(&config.manifest_path, &manifest)?;
    let gate_pass = manifest.gate_pass();

    println!("nando_phase_action_package_v1:");
    println!("  verdict: {}", manifest.verdict);
    println!("  corpus_path: {}", manifest.corpus_path);
    println!(
        "  source_contract_fingerprint64: {}",
        manifest.source_contract_fingerprint64
    );
    println!(
        "  source_contract_bytes: {}",
        manifest.source_contract_bytes
    );
    println!("  package_path: {}", manifest.package_path);
    println!("  manifest_path: {}", manifest.manifest_path);
    println!("  rows: {}", manifest.rows);
    println!("  train_rows: {}", manifest.train_rows);
    println!("  heldout_rows: {}", manifest.heldout_rows);
    println!("  cells: {}", manifest.cells);
    println!("  flat_records: {}", manifest.flat_records);
    println!("  operator_key_count: {}", manifest.operator_keys.len());
    println!("  package_bytes: {}", manifest.package_bytes);
    println!(
        "  inspected_payload_bytes: {}",
        manifest.inspected_payload_bytes
    );
    println!(
        "  package_fingerprint64: {}",
        manifest.package_fingerprint64
    );
    println!(
        "  runtime_bytes_estimate: {}",
        manifest.runtime_bytes_estimate
    );
    println!("  skipped_train_rows: {}", manifest.skipped_train_rows);
    println!("  missing_centers: {}", manifest.missing_centers);
    println!("  skipped_rows: {}", manifest.skipped_rows);
    println!(
        "  action_ablation_eval_rows: {}",
        manifest.action_ablation_eval_rows
    );
    println!(
        "  action_ablation_missing_centers: {}",
        manifest.action_ablation_missing_centers
    );
    println!("  accuracy_milli: {}", manifest.accuracy_milli);
    println!("  wrong_wins: {}", manifest.wrong_wins);
    println!("  median_margin: {:.6}", manifest.median_margin);
    println!("  p10_margin: {:.6}", manifest.p10_margin);
    println!("  p50_latency_ns: {}", manifest.p50_latency_ns);
    println!("  p99_latency_ns: {}", manifest.p99_latency_ns);
    println!("  rows_per_second: {:.2}", manifest.rows_per_second);
    println!(
        "  action_ablation_accuracy_milli: {}",
        manifest.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        manifest.action_ablation_wrong_wins
    );
    println!("  contract_verdict: {}", manifest.contract_verdict);
    println!("  compiler_path: {}", manifest.compiler_path);
    println!("  package_path_api: {}", manifest.package_path_api);
    println!("  runtime_path: {}", manifest.runtime_path);
    println!("  python_demo_used: {}", manifest.python_demo_used);
    println!(
        "  target_center_id_training_used: {}",
        manifest.target_center_id_training_used
    );
    println!(
        "  proof_rule_id_training_authority_used: {}",
        manifest.proof_rule_id_training_authority_used
    );
    println!(
        "  concrete_x_lookup_used: {}",
        manifest.concrete_x_lookup_used
    );
    println!(
        "  local_out_t_runtime_extension_used: {}",
        manifest.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", manifest.claim_boundary);

    if !gate_pass {
        return Err(String::from("phase action package v1 gate failed"));
    }
    Ok(())
}

pub(crate) fn run_phase_package_verify(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_phase_package_verify_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_manifest(&config.manifest_path)?;
    let score_report = read_score_report(&config.report_path)?;

    let manifest_matches =
        validate_manifest_package_match(&manifest, &package_info, package_bytes.len()).is_ok();
    let report_matches = validate_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let gate_pass = manifest_matches
        && report_matches
        && !manifest.forbidden_flags.any_forbidden_used()
        && !score_report.forbidden_flags.any_forbidden_used()
        && score_report_gate_pass(&score_report);

    println!("nando_phase_package_verify:");
    println!("  verdict: {}", phase_package_verify_verdict(gate_pass));
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  score_report_path: {}", config.report_path.display());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  cells: {}", package_info.cells);
    println!("  flat_records: {}", package_info.record_count);
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!(
        "  score_report_schema_version: {}",
        score_report.schema_version
    );
    println!("  score_report_verdict: {}", score_report.verdict);
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  score_report_matches_package: {report_matches}");
    println!("  accuracy_milli: {}", score_report.accuracy_milli);
    println!("  wrong_wins: {}", score_report.wrong_wins);
    println!(
        "  action_ablation_accuracy_milli: {}",
        score_report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        score_report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", score_report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        score_report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        format_optional_bool(score_report.corpus_jsonl_used_in_score_loop)
    );
    if !score_report.eval_task_package_path.is_empty() {
        println!(
            "  eval_task_package_path: {}",
            score_report.eval_task_package_path
        );
    }
    println!(
        "  manifest_forbidden_used: {}",
        manifest.forbidden_flags.any_forbidden_used()
    );
    println!(
        "  score_report_forbidden_used: {}",
        score_report.forbidden_flags.any_forbidden_used()
    );
    println!(
        "  manifest_target_center_id_training_used: {}",
        manifest.forbidden_flags.target_center_id_training_used
    );
    println!(
        "  score_report_target_center_id_training_used: {}",
        score_report.forbidden_flags.target_center_id_training_used
    );
    println!(
        "  manifest_proof_rule_id_training_authority_used: {}",
        manifest
            .forbidden_flags
            .proof_rule_id_training_authority_used
    );
    println!(
        "  score_report_proof_rule_id_training_authority_used: {}",
        score_report
            .forbidden_flags
            .proof_rule_id_training_authority_used
    );
    println!(
        "  manifest_concrete_x_lookup_used: {}",
        manifest.forbidden_flags.concrete_x_lookup_used
    );
    println!(
        "  score_report_concrete_x_lookup_used: {}",
        score_report.forbidden_flags.concrete_x_lookup_used
    );
    println!(
        "  manifest_local_out_t_runtime_extension_used: {}",
        manifest.forbidden_flags.local_out_t_runtime_extension_used
    );
    println!(
        "  score_report_local_out_t_runtime_extension_used: {}",
        score_report
            .forbidden_flags
            .local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", score_report.claim_boundary);

    validate_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    validate_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )?;
    if !gate_pass {
        return Err(String::from("phase package verify gate failed"));
    }

    Ok(())
}

pub(crate) fn run_phase_action_package_verify_v1(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let config = parse_phase_action_package_verify_args(args)?;
    let package_bytes = std::fs::read(&config.package_path).map_err(|error| {
        format!(
            "failed to read '{}': {error}",
            config.package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&config.manifest_path)?;
    let score_report = read_action_score_report(&config.report_path)?;

    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let report_matches = validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )
    .is_ok();
    let gate_pass =
        manifest_matches && report_matches && manifest.gate_pass() && score_report.gate_pass();

    println!("nando_phase_action_package_verify_v1:");
    println!(
        "  verdict: {}",
        phase_action_package_verify_v1_verdict(gate_pass)
    );
    println!("  package_path: {}", config.package_path.display());
    println!("  manifest_path: {}", config.manifest_path.display());
    println!("  score_report_path: {}", config.report_path.display());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_bytes: {}", package_bytes.len());
    println!("  cells: {}", package_info.cells);
    println!("  flat_records: {}", package_info.record_count);
    println!("  manifest_operator_keys: {}", manifest.operator_keys.len());
    println!(
        "  score_report_schema_version: {}",
        score_report.schema_version
    );
    println!("  score_report_verdict: {}", score_report.verdict);
    println!("  manifest_matches_package: {manifest_matches}");
    println!("  score_report_matches_package: {report_matches}");
    println!("  accuracy_milli: {}", score_report.accuracy_milli);
    println!("  wrong_wins: {}", score_report.wrong_wins);
    println!(
        "  action_ablation_accuracy_milli: {}",
        score_report.action_ablation_accuracy_milli
    );
    println!(
        "  action_ablation_wrong_wins: {}",
        score_report.action_ablation_wrong_wins
    );
    println!("  compiler_used: {}", score_report.compiler_used);
    println!(
        "  eval_task_package_used: {}",
        score_report.eval_task_package_used
    );
    println!(
        "  corpus_jsonl_used_in_score_loop: {}",
        format_optional_bool(score_report.corpus_jsonl_used_in_score_loop)
    );
    if !score_report.eval_task_package_path.is_empty() {
        println!(
            "  eval_task_package_path: {}",
            score_report.eval_task_package_path
        );
    }
    println!("  contract_verdict: {}", score_report.contract_verdict);
    println!("  manifest_verdict: {}", score_report.manifest_verdict);
    println!(
        "  manifest_forbidden_used: {}",
        manifest_forbidden_used(&manifest)
    );
    println!(
        "  score_report_forbidden_used: {}",
        score_report.forbidden_used()
    );
    println!(
        "  manifest_target_center_id_training_used: {}",
        manifest.target_center_id_training_used
    );
    println!(
        "  score_report_target_center_id_training_used: {}",
        score_report.target_center_id_training_used
    );
    println!(
        "  manifest_proof_rule_id_training_authority_used: {}",
        manifest.proof_rule_id_training_authority_used
    );
    println!(
        "  score_report_proof_rule_id_training_authority_used: {}",
        score_report.proof_rule_id_training_authority_used
    );
    println!(
        "  manifest_concrete_x_lookup_used: {}",
        manifest.concrete_x_lookup_used
    );
    println!(
        "  score_report_concrete_x_lookup_used: {}",
        score_report.concrete_x_lookup_used
    );
    println!(
        "  manifest_local_out_t_runtime_extension_used: {}",
        manifest.local_out_t_runtime_extension_used
    );
    println!(
        "  score_report_local_out_t_runtime_extension_used: {}",
        score_report.local_out_t_runtime_extension_used
    );
    println!("  claim_boundary: {}", score_report.claim_boundary);

    validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())?;
    validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime.bytes_estimate(),
    )?;
    if !gate_pass {
        return Err(String::from("phase action package verify v1 gate failed"));
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct PhasePackageConfig {
    corpus_path: PathBuf,
    package_path: PathBuf,
    manifest_path: PathBuf,
    cells: usize,
}

#[derive(Clone, Debug)]
struct PhasePackageInspectConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageInspectConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionSourceVerifyConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageScoreConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    corpus_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionEvalPackConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    corpus_path: PathBuf,
    eval_pack_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageScorePackConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageBenchPackConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    iterations: usize,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageBenchVerifyConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionProductProofConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    score_report_path: PathBuf,
    bench_report_path: PathBuf,
    proof_report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionProductVerifyConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    score_report_path: PathBuf,
    bench_report_path: PathBuf,
    proof_report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionReleaseSuiteConfig {
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionLicensePackageConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionOffloadAuditConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    margin_threshold_micro: i64,
    simulated_calls: usize,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionOffloadVerifyConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionCacheOffloadBenchConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    margin_threshold_micro: i64,
    simulated_calls: usize,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionWorkflowBenchConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    cache_bench_report_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionWorkflowReplayConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    margin_threshold_micro: i64,
    workflow_sessions: usize,
    steps_per_session: usize,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionRegressionConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    offload_report_path: PathBuf,
    report_path: PathBuf,
    cache_bench_report_path: PathBuf,
    workflow_bench_report_path: PathBuf,
    workflow_replay_report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionRegressionVerifyConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    offload_report_path: PathBuf,
    report_path: PathBuf,
    cache_bench_report_path: PathBuf,
    workflow_bench_report_path: PathBuf,
    workflow_replay_report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionRegressionFreezeConfig {
    release_suite_report_path: PathBuf,
    license_file_path: PathBuf,
    license_report_path: PathBuf,
    offload_report_path: PathBuf,
    regression_report_path: PathBuf,
    freeze_report_path: PathBuf,
    cache_bench_report_path: PathBuf,
    workflow_bench_report_path: PathBuf,
    workflow_replay_report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionProductBundlePaths {
    label: String,
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    score_report_path: PathBuf,
    bench_report_path: PathBuf,
    proof_report_path: PathBuf,
    source_verify_report_path: PathBuf,
    shortcut_report_path: PathBuf,
    operator_coverage_report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageVerifyConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhasePackageScoreConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    corpus_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseEvalPackConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    corpus_path: PathBuf,
    eval_pack_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhasePackageScorePackConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    eval_pack_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionBoundaryConfig {
    corpus_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionCorpusConfig {
    output_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionContractConfig {
    corpus_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionShortcutConfig {
    corpus_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct StrictMultiSeedRustAuditConfig {
    diagnostics_root_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhaseActionRuntimeConfig {
    corpus_path: PathBuf,
    cells: usize,
    report_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct PhaseActionPackageConfig {
    corpus_path: PathBuf,
    package_path: PathBuf,
    cells: usize,
    manifest_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PhasePackageVerifyConfig {
    package_path: PathBuf,
    manifest_path: PathBuf,
    report_path: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct PackageGateMeta {
    package_fingerprint64: u64,
    operator_key_count: usize,
    record_count: usize,
    has_empty_operator_key: bool,
}

#[derive(Clone, Copy, Debug)]
struct RuntimeEval {
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhasePackageManifest {
    schema_version: String,
    package_kind: String,
    verdict: String,
    corpus_path: String,
    package_path: String,
    manifest_path: String,
    command: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    cells: usize,
    flat_records: usize,
    operator_keys: Vec<String>,
    skipped_train_rows: usize,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_eval_rows: usize,
    action_ablation_missing_centers: usize,
    heldout_surface_groups: usize,
    heldout_noise_groups: usize,
    package_magic: Vec<u8>,
    inspected_payload_bytes: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    serialized_len: usize,
    runtime_bytes_estimate: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    compiler_path: String,
    package_path_api: String,
    runtime_path: String,
    forbidden_flags: ForbiddenFlags,
    claim_boundary: String,
    license_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhasePackageScoreReport {
    schema_version: String,
    package_kind: String,
    verdict: String,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    #[serde(default)]
    eval_task_package_path: String,
    #[serde(default)]
    eval_task_package_used: bool,
    corpus_jsonl_used_in_score_loop: Option<bool>,
    cells: usize,
    flat_records: usize,
    manifest_operator_keys: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_payload_bytes: usize,
    runtime_bytes_estimate: usize,
    rows: usize,
    heldout_eval_rows: usize,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_eval_rows: usize,
    action_ablation_missing_centers: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    compiler_used: bool,
    runtime_path: String,
    forbidden_flags: ForbiddenFlags,
    claim_boundary: String,
    license_boundary: String,
}

struct PhasePackageScoreReportInput<'a> {
    verdict: &'static str,
    package_path: &'a Path,
    manifest_path: &'a Path,
    corpus_path: &'a Path,
    eval_task_package_path: Option<&'a Path>,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime: &'a PhaseCenterFlatRuntime,
    manifest: &'a PhasePackageManifest,
    rows: usize,
    prepared: &'a PreparedEval,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used_in_score_loop: bool,
}

impl PhasePackageScoreReport {
    fn from_score(input: PhasePackageScoreReportInput<'_>) -> Self {
        Self {
            schema_version: "nando_phase_package_score_report_v1".to_string(),
            package_kind: input.manifest.package_kind.clone(),
            verdict: input.verdict.to_string(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            corpus_path: input.corpus_path.display().to_string(),
            eval_task_package_path: input
                .eval_task_package_path
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            eval_task_package_used: input.eval_task_package_used,
            corpus_jsonl_used_in_score_loop: Some(input.corpus_jsonl_used_in_score_loop),
            cells: input.manifest.cells,
            flat_records: input.runtime.record_count(),
            manifest_operator_keys: input.manifest.operator_keys.len(),
            package_fingerprint64: input.package_info.fingerprint64,
            package_bytes: input.package_bytes_len,
            inspected_payload_bytes: input.package_info.payload_bytes,
            runtime_bytes_estimate: input.runtime.bytes_estimate(),
            rows: input.rows,
            heldout_eval_rows: input.prepared.tasks.len(),
            missing_centers: input.prepared.missing_centers,
            skipped_rows: input.prepared.skipped_rows,
            action_ablation_eval_rows: input.prepared.action_ablation_tasks.len(),
            action_ablation_missing_centers: input.prepared.action_ablation_missing_centers,
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            compiler_used: input.compiler_used,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            forbidden_flags: input.manifest.forbidden_flags.clone(),
            claim_boundary: input.manifest.claim_boundary.clone(),
            license_boundary: input.manifest.license_boundary.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct PhaseEvalTaskPackage {
    cells: usize,
    package_fingerprint64: u64,
    rows: usize,
    prepared: PreparedEval,
}

#[derive(Clone, Debug, Default)]
struct PhaseActionBoundaryReport {
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    explicit_operator_class_label_rows: usize,
    explicit_operator_family_label_rows: usize,
    explicit_order_slot_map_rows: usize,
    explicit_branch_slot_map_rows: usize,
    explicit_source_slot_token_rows: usize,
    literal_marker_parameter_rows: usize,
    action_demo_arrow_rows: usize,
    proof_rule_id_literal_rows: usize,
    target_answer_literal_rows: usize,
}

impl PhaseActionBoundaryReport {
    fn from_rows(rows: &[PhaseOperatorRow]) -> Self {
        let mut report = Self {
            rows: rows.len(),
            train_rows: rows
                .iter()
                .filter(|row| phase_split(row) == Some("train"))
                .count(),
            heldout_rows: rows
                .iter()
                .filter(|row| phase_split(row) == Some("heldout"))
                .count(),
            ..Self::default()
        };

        for row in rows {
            let action = row.action.as_str();
            report.explicit_operator_class_label_rows +=
                usize::from(action.contains("operator_class:"));
            report.explicit_operator_family_label_rows +=
                usize::from(action.contains("operator_family:"));
            report.explicit_order_slot_map_rows += usize::from(action.contains("operator_slots:"));
            report.explicit_branch_slot_map_rows +=
                usize::from(action.contains("then_slots:") || action.contains("else_slots:"));
            report.explicit_source_slot_token_rows += usize::from(action.contains("src"));
            report.literal_marker_parameter_rows += usize::from(action.contains("marker:"));
            report.action_demo_arrow_rows +=
                usize::from(action.contains("demo:") && action.contains("->"));
            report.proof_rule_id_literal_rows += usize::from(action.contains("proof_rule_id"));
            report.target_answer_literal_rows += usize::from(
                action.contains("state_after")
                    || action.contains("correct_tokens")
                    || action.contains("wrong_tokens"),
            );
        }

        report
    }

    const fn router_gate_pass(&self) -> bool {
        self.rows > 0
            && self.explicit_operator_class_label_rows == 0
            && self.explicit_operator_family_label_rows == 0
            && self.explicit_order_slot_map_rows == 0
            && self.explicit_branch_slot_map_rows == 0
            && self.proof_rule_id_literal_rows == 0
            && self.target_answer_literal_rows == 0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct StrictMultiSeedRustAuditReport {
    schema_version: String,
    audit_kind: String,
    verdict: String,
    gate_pass: bool,
    diagnostics_root_path: String,
    expected_seeds: Vec<u8>,
    expected_classes: Vec<String>,
    observed_logs: usize,
    missing_logs: Vec<String>,
    strict_runtime_issues: Vec<String>,
    evidence_warnings: Vec<String>,
    logs_fingerprint64: u64,
    logs_total_bytes: usize,
    log_reports: Vec<StrictMultiSeedRuntimeLogReport>,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    corpus_jsonl_used: bool,
    rust_runtime_logs_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct StrictMultiSeedRuntimeLogReport {
    seed: u8,
    operator_class: String,
    log_path: String,
    log_fingerprint64: u64,
    log_bytes: usize,
    test_result_ok: bool,
    test_result_failed: bool,
    slot_accuracy_milli: Option<usize>,
    flat_slot_accuracy_milli: Option<usize>,
    sequence_energy_accuracy_milli: Option<usize>,
    energy_pass_slot_fail: Option<usize>,
    output_slot_cleanup_failed_slots: Option<usize>,
    slot_failure_total: Option<usize>,
    flat_gap_parity_mismatches: Option<usize>,
    flat_sequence_energy_parity_mismatches: Option<usize>,
    state_delta_edges: Option<usize>,
    role_binding_edges: Option<usize>,
    target_center_id_training_used: Option<bool>,
    proof_rule_id_training_authority_used: Option<bool>,
    concrete_x_lookup_used: Option<bool>,
    local_out_t_runtime_extension_used: Option<bool>,
    issues: Vec<String>,
    evidence_warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionCorpusReport {
    schema_version: String,
    verdict: String,
    output_path: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    operator_key_count: usize,
    same_bag_rows: usize,
    duplicate_task_id_rows: usize,
    min_sequence_len: usize,
    max_sequence_len: usize,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

impl PhaseActionCorpusReport {
    fn from_rows(output_path: &Path, rows: &[PhaseActionContractRow]) -> Self {
        let mut task_ids = BTreeSet::new();
        let mut duplicate_task_id_rows = 0usize;
        let mut operator_keys = BTreeSet::new();
        let mut same_bag_rows = 0usize;
        let mut min_sequence_len = usize::MAX;
        let mut max_sequence_len = 0usize;

        for row in rows {
            if !task_ids.insert(row.task_id.as_str()) {
                duplicate_task_id_rows += 1;
            }
            operator_keys.insert(action_contract_key(row));
            if same_token_bag(&row.state_after_correct, &row.state_after_wrong) {
                same_bag_rows += 1;
            }
            let len = row.state_before.split_whitespace().count();
            min_sequence_len = min_sequence_len.min(len);
            max_sequence_len = max_sequence_len.max(len);
        }
        if rows.is_empty() {
            min_sequence_len = 0;
        }

        let mut report = Self {
            schema_version: "nando_phase_action_corpus_report_v1".to_string(),
            verdict: String::new(),
            output_path: output_path.display().to_string(),
            rows: rows.len(),
            train_rows: rows.iter().filter(|row| row.split == "train").count(),
            heldout_rows: rows.iter().filter(|row| row.split == "heldout").count(),
            operator_key_count: operator_keys.len(),
            same_bag_rows,
            duplicate_task_id_rows,
            min_sequence_len,
            max_sequence_len,
            python_demo_used: false,
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            claim_boundary: String::from(
                "deterministic clean action_contract_v1 corpus factory; not a runtime proof",
            ),
        };
        report.verdict = phase_action_corpus_v1_verdict(report.gate_pass()).to_string();
        report
    }

    const fn gate_pass(&self) -> bool {
        self.rows > 0
            && self.train_rows > 0
            && self.heldout_rows > 0
            && self.operator_key_count > 1
            && self.same_bag_rows == self.rows
            && self.duplicate_task_id_rows == 0
            && self.min_sequence_len >= 4
            && self.max_sequence_len >= self.min_sequence_len
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
    }
}

#[derive(Clone, Debug, Default, Serialize)]
struct PhaseActionContractReport {
    schema_version: String,
    verdict: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    schema_mismatch_rows: usize,
    invalid_split_rows: usize,
    empty_task_id_rows: usize,
    empty_state_before_rows: usize,
    empty_action_select_rows: usize,
    empty_action_transform_rows: usize,
    empty_action_write_rows: usize,
    empty_action_condition_rows: usize,
    empty_action_check_rows: usize,
    empty_correct_rows: usize,
    empty_wrong_rows: usize,
    identical_correct_wrong_rows: usize,
    forbidden_operator_label_rows: usize,
    forbidden_slot_map_rows: usize,
    forbidden_target_leak_rows: usize,
    forbidden_lookup_authority_rows: usize,
    forbidden_local_out_t_rows: usize,
    forbidden_arrow_demo_rows: usize,
    concrete_output_token_leak_rows: usize,
    action_tree_key_count: usize,
    train_action_tree_key_count: usize,
    heldout_action_tree_key_count: usize,
    min_train_rows_per_action_tree: usize,
    min_heldout_rows_per_action_tree: usize,
    accepted_action_tree_rows: usize,
    claim_boundary: String,
}

impl PhaseActionContractReport {
    fn from_rows(rows: &[PhaseActionContractRow]) -> Self {
        let mut report = Self {
            schema_version: "nando_phase_action_contract_report_v1".to_string(),
            rows: rows.len(),
            train_rows: rows.iter().filter(|row| row.split == "train").count(),
            heldout_rows: rows.iter().filter(|row| row.split == "heldout").count(),
            claim_boundary: String::from(
                "contract gate only: clean action_tree schema, not a model/runtime proof",
            ),
            ..Self::default()
        };
        let mut action_tree_counts = BTreeMap::<String, usize>::new();
        let mut train_action_tree_counts = BTreeMap::<String, usize>::new();
        let mut heldout_action_tree_counts = BTreeMap::<String, usize>::new();

        for row in rows {
            let action_text = action_contract_text(row);
            let action_lower = action_text.to_ascii_lowercase();
            let action_key = action_contract_key(row);
            *action_tree_counts.entry(action_key.clone()).or_default() += 1;
            match row.split.as_str() {
                "train" => {
                    *train_action_tree_counts.entry(action_key).or_default() += 1;
                }
                "heldout" => {
                    *heldout_action_tree_counts.entry(action_key).or_default() += 1;
                }
                _ => {}
            }
            report.schema_mismatch_rows +=
                usize::from(row.schema_version != "nando_action_contract_v1");
            report.invalid_split_rows +=
                usize::from(row.split != "train" && row.split != "heldout");
            report.empty_task_id_rows += usize::from(row.task_id.trim().is_empty());
            report.empty_state_before_rows += usize::from(row.state_before.trim().is_empty());
            report.empty_action_select_rows +=
                usize::from(row.action_tree.select.trim().is_empty());
            report.empty_action_transform_rows +=
                usize::from(row.action_tree.transform.trim().is_empty());
            report.empty_action_write_rows += usize::from(row.action_tree.write.trim().is_empty());
            report.empty_action_condition_rows +=
                usize::from(row.action_tree.condition.trim().is_empty());
            report.empty_action_check_rows += usize::from(row.action_tree.check.trim().is_empty());
            report.empty_correct_rows += usize::from(row.state_after_correct.trim().is_empty());
            report.empty_wrong_rows += usize::from(row.state_after_wrong.trim().is_empty());
            report.identical_correct_wrong_rows += usize::from(
                collapse_whitespace(&row.state_after_correct)
                    == collapse_whitespace(&row.state_after_wrong),
            );
            report.forbidden_operator_label_rows += usize::from(contains_any(
                &action_lower,
                &[
                    "operator_class",
                    "operator_family",
                    "operator_slots",
                    "rule_action_example",
                    "proof_rule",
                    "rule_id",
                ],
            ));
            report.forbidden_slot_map_rows += usize::from(
                contains_any(
                    &action_lower,
                    &["then_slots", "else_slots", "slot_map", "target_slot"],
                ) || contains_numbered_slot_token(&action_lower),
            );
            report.forbidden_target_leak_rows += usize::from(contains_any(
                &action_lower,
                &[
                    "target_id",
                    "target_center",
                    "state_after",
                    "correct_tokens",
                    "wrong_tokens",
                    "answer",
                    "gold",
                ],
            ));
            report.forbidden_lookup_authority_rows += usize::from(contains_any(
                &action_lower,
                &[
                    "exact_lookup",
                    "concrete_x_lookup",
                    "lookup table",
                    "memorize",
                ],
            ));
            report.forbidden_local_out_t_rows += usize::from(action_lower.contains("local_out_t"));
            report.forbidden_arrow_demo_rows +=
                usize::from(action_lower.contains("->") || action_lower.contains("=>"));
            report.concrete_output_token_leak_rows +=
                usize::from(action_contains_output_token(row, &action_lower));
        }
        report.action_tree_key_count = action_tree_counts.len();
        report.train_action_tree_key_count = train_action_tree_counts.len();
        report.heldout_action_tree_key_count = heldout_action_tree_counts.len();
        report.min_train_rows_per_action_tree = action_tree_counts
            .keys()
            .map(|key| train_action_tree_counts.get(key).copied().unwrap_or(0))
            .min()
            .unwrap_or(0);
        report.min_heldout_rows_per_action_tree = action_tree_counts
            .keys()
            .map(|key| heldout_action_tree_counts.get(key).copied().unwrap_or(0))
            .min()
            .unwrap_or(0);
        report.accepted_action_tree_rows = report.rows.saturating_sub(report.rejected_rows());
        report.verdict = phase_action_contract_v1_verdict(report.gate_pass()).to_string();
        report
    }

    const fn gate_pass(&self) -> bool {
        self.rows > 0
            && self.train_rows > 0
            && self.heldout_rows > 0
            && self.schema_mismatch_rows == 0
            && self.invalid_split_rows == 0
            && self.empty_task_id_rows == 0
            && self.empty_state_before_rows == 0
            && self.empty_action_select_rows == 0
            && self.empty_action_transform_rows == 0
            && self.empty_action_write_rows == 0
            && self.empty_action_condition_rows == 0
            && self.empty_action_check_rows == 0
            && self.empty_correct_rows == 0
            && self.empty_wrong_rows == 0
            && self.identical_correct_wrong_rows == 0
            && self.forbidden_operator_label_rows == 0
            && self.forbidden_slot_map_rows == 0
            && self.forbidden_target_leak_rows == 0
            && self.forbidden_lookup_authority_rows == 0
            && self.forbidden_local_out_t_rows == 0
            && self.forbidden_arrow_demo_rows == 0
            && self.concrete_output_token_leak_rows == 0
            && self.action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.train_action_tree_key_count == self.action_tree_key_count
            && self.heldout_action_tree_key_count == self.action_tree_key_count
            && self.min_train_rows_per_action_tree > 0
            && self.min_heldout_rows_per_action_tree > 0
    }

    const fn rejected_rows(&self) -> usize {
        self.schema_mismatch_rows
            + self.invalid_split_rows
            + self.empty_task_id_rows
            + self.empty_state_before_rows
            + self.empty_action_select_rows
            + self.empty_action_transform_rows
            + self.empty_action_write_rows
            + self.empty_action_condition_rows
            + self.empty_action_check_rows
            + self.empty_correct_rows
            + self.empty_wrong_rows
            + self.identical_correct_wrong_rows
            + self.forbidden_operator_label_rows
            + self.forbidden_slot_map_rows
            + self.forbidden_target_leak_rows
            + self.forbidden_lookup_authority_rows
            + self.forbidden_local_out_t_rows
            + self.forbidden_arrow_demo_rows
            + self.concrete_output_token_leak_rows
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
struct PhaseActionOperatorCoverageReport {
    schema_version: String,
    verdict: String,
    corpus_path: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    action_tree_key_count: usize,
    select_value_count: usize,
    transform_value_count: usize,
    write_value_count: usize,
    condition_value_count: usize,
    check_value_count: usize,
    train_select_value_count: usize,
    train_transform_value_count: usize,
    train_write_value_count: usize,
    train_condition_value_count: usize,
    train_check_value_count: usize,
    heldout_select_value_count: usize,
    heldout_transform_value_count: usize,
    heldout_write_value_count: usize,
    heldout_condition_value_count: usize,
    heldout_check_value_count: usize,
    min_dimension_value_count: usize,
    wide_dimension_count: usize,
    train_dimension_coverage_pass: bool,
    heldout_dimension_coverage_pass: bool,
    full_operator_dimension_coverage_pass: bool,
    contract_gate_pass: bool,
    label_authority_used: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

impl PhaseActionOperatorCoverageReport {
    fn from_rows(
        corpus_path: &Path,
        rows: &[PhaseActionContractRow],
        contract_report: &PhaseActionContractReport,
    ) -> Self {
        let mut select_values = BTreeSet::new();
        let mut transform_values = BTreeSet::new();
        let mut write_values = BTreeSet::new();
        let mut condition_values = BTreeSet::new();
        let mut check_values = BTreeSet::new();
        let mut train_select_values = BTreeSet::new();
        let mut train_transform_values = BTreeSet::new();
        let mut train_write_values = BTreeSet::new();
        let mut train_condition_values = BTreeSet::new();
        let mut train_check_values = BTreeSet::new();
        let mut heldout_select_values = BTreeSet::new();
        let mut heldout_transform_values = BTreeSet::new();
        let mut heldout_write_values = BTreeSet::new();
        let mut heldout_condition_values = BTreeSet::new();
        let mut heldout_check_values = BTreeSet::new();

        for row in rows {
            select_values.insert(row.action_tree.select.as_str());
            transform_values.insert(row.action_tree.transform.as_str());
            write_values.insert(row.action_tree.write.as_str());
            condition_values.insert(row.action_tree.condition.as_str());
            check_values.insert(row.action_tree.check.as_str());

            match row.split.as_str() {
                "train" => {
                    train_select_values.insert(row.action_tree.select.as_str());
                    train_transform_values.insert(row.action_tree.transform.as_str());
                    train_write_values.insert(row.action_tree.write.as_str());
                    train_condition_values.insert(row.action_tree.condition.as_str());
                    train_check_values.insert(row.action_tree.check.as_str());
                }
                "heldout" => {
                    heldout_select_values.insert(row.action_tree.select.as_str());
                    heldout_transform_values.insert(row.action_tree.transform.as_str());
                    heldout_write_values.insert(row.action_tree.write.as_str());
                    heldout_condition_values.insert(row.action_tree.condition.as_str());
                    heldout_check_values.insert(row.action_tree.check.as_str());
                }
                _ => {}
            }
        }

        let dimension_counts = [
            select_values.len(),
            transform_values.len(),
            write_values.len(),
            condition_values.len(),
            check_values.len(),
        ];
        let min_dimension_value_count = dimension_counts.into_iter().min().unwrap_or(0);
        let wide_dimension_count = dimension_counts
            .into_iter()
            .filter(|count| *count >= 2)
            .count();
        let train_dimension_coverage_pass = train_select_values.len() == select_values.len()
            && train_transform_values.len() == transform_values.len()
            && train_write_values.len() == write_values.len()
            && train_condition_values.len() == condition_values.len()
            && train_check_values.len() == check_values.len();
        let heldout_dimension_coverage_pass = heldout_select_values.len() == select_values.len()
            && heldout_transform_values.len() == transform_values.len()
            && heldout_write_values.len() == write_values.len()
            && heldout_condition_values.len() == condition_values.len()
            && heldout_check_values.len() == check_values.len();
        let full_operator_dimension_coverage_pass = min_dimension_value_count >= 2
            && wide_dimension_count == 5
            && train_dimension_coverage_pass
            && heldout_dimension_coverage_pass;

        let mut report = Self {
            schema_version: "nando_phase_action_operator_coverage_report_v1".to_string(),
            verdict: String::new(),
            corpus_path: corpus_path.display().to_string(),
            rows: rows.len(),
            train_rows: contract_report.train_rows,
            heldout_rows: contract_report.heldout_rows,
            action_tree_key_count: contract_report.action_tree_key_count,
            select_value_count: select_values.len(),
            transform_value_count: transform_values.len(),
            write_value_count: write_values.len(),
            condition_value_count: condition_values.len(),
            check_value_count: check_values.len(),
            train_select_value_count: train_select_values.len(),
            train_transform_value_count: train_transform_values.len(),
            train_write_value_count: train_write_values.len(),
            train_condition_value_count: train_condition_values.len(),
            train_check_value_count: train_check_values.len(),
            heldout_select_value_count: heldout_select_values.len(),
            heldout_transform_value_count: heldout_transform_values.len(),
            heldout_write_value_count: heldout_write_values.len(),
            heldout_condition_value_count: heldout_condition_values.len(),
            heldout_check_value_count: heldout_check_values.len(),
            min_dimension_value_count,
            wide_dimension_count,
            train_dimension_coverage_pass,
            heldout_dimension_coverage_pass,
            full_operator_dimension_coverage_pass,
            contract_gate_pass: contract_report.gate_pass(),
            label_authority_used: false,
            python_demo_used: false,
            claim_boundary: String::from(
                "operator-dimension coverage audit only; not runtime proof, not target_id/proof_rule_id authority, and not a Python demo",
            ),
        };
        report.verdict = phase_action_operator_coverage_v1_verdict(report.gate_pass()).to_string();
        report
    }

    const fn gate_pass(&self) -> bool {
        self.rows > 0
            && self.train_rows > 0
            && self.heldout_rows > 0
            && self.action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.contract_gate_pass
            && self.full_operator_dimension_coverage_pass
            && !self.label_authority_used
            && !self.python_demo_used
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionShortcutReport {
    schema_version: String,
    verdict: String,
    corpus_path: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    operator_key_count: usize,
    heldout_operator_keys_seen_in_train_rows: usize,
    heldout_operator_key_missing_rows: usize,
    exact_state_lookup_hits: usize,
    exact_transition_lookup_hits: usize,
    heldout_token_overlap_rows: usize,
    heldout_length_seen_in_train_rows: usize,
    non_same_bag_rows: usize,
    correct_wrong_identical_rows: usize,
    source_bigram_correct_wins: usize,
    source_bigram_wrong_wins: usize,
    source_bigram_ties: usize,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

impl PhaseActionShortcutReport {
    fn from_rows(corpus_path: &Path, rows: &[PhaseActionContractRow]) -> Self {
        let train_rows = rows
            .iter()
            .filter(|row| row.split == "train")
            .collect::<Vec<_>>();
        let heldout_rows = rows
            .iter()
            .filter(|row| row.split == "heldout")
            .collect::<Vec<_>>();
        let train_states = train_rows
            .iter()
            .map(|row| collapse_whitespace(&row.state_before))
            .collect::<BTreeSet<_>>();
        let train_transitions = train_rows
            .iter()
            .map(|row| {
                format!(
                    "{}\n{}",
                    action_contract_key(row),
                    collapse_whitespace(&row.state_before)
                )
            })
            .collect::<BTreeSet<_>>();
        let train_action_keys = train_rows
            .iter()
            .map(|row| action_contract_key(row))
            .collect::<BTreeSet<_>>();
        let all_action_keys = rows
            .iter()
            .map(action_contract_key)
            .collect::<BTreeSet<_>>();
        let train_tokens = train_rows
            .iter()
            .flat_map(|row| row.state_before.split_whitespace())
            .collect::<BTreeSet<_>>();
        let train_lengths = train_rows
            .iter()
            .map(|row| row.state_before.split_whitespace().count())
            .collect::<BTreeSet<_>>();

        let mut report = Self {
            schema_version: "nando_phase_action_shortcut_report_v1".to_string(),
            verdict: String::new(),
            corpus_path: corpus_path.display().to_string(),
            rows: rows.len(),
            train_rows: train_rows.len(),
            heldout_rows: heldout_rows.len(),
            operator_key_count: all_action_keys.len(),
            heldout_operator_keys_seen_in_train_rows: 0,
            heldout_operator_key_missing_rows: 0,
            exact_state_lookup_hits: 0,
            exact_transition_lookup_hits: 0,
            heldout_token_overlap_rows: 0,
            heldout_length_seen_in_train_rows: 0,
            non_same_bag_rows: 0,
            correct_wrong_identical_rows: 0,
            source_bigram_correct_wins: 0,
            source_bigram_wrong_wins: 0,
            source_bigram_ties: 0,
            python_demo_used: false,
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            claim_boundary: String::from(
                "shortcut gate only: exact/token/length/bag/source-bigram baselines, not runtime proof",
            ),
        };

        for row in heldout_rows {
            let state = collapse_whitespace(&row.state_before);
            let action_key = action_contract_key(row);
            report.heldout_operator_keys_seen_in_train_rows +=
                usize::from(train_action_keys.contains(&action_key));
            report.heldout_operator_key_missing_rows +=
                usize::from(!train_action_keys.contains(&action_key));
            report.exact_state_lookup_hits += usize::from(train_states.contains(&state));
            report.exact_transition_lookup_hits +=
                usize::from(train_transitions.contains(&format!("{action_key}\n{state}")));
            report.heldout_token_overlap_rows += usize::from(
                row.state_before
                    .split_whitespace()
                    .any(|token| train_tokens.contains(token)),
            );
            report.heldout_length_seen_in_train_rows +=
                usize::from(train_lengths.contains(&row.state_before.split_whitespace().count()));
            report.non_same_bag_rows += usize::from(!same_token_bag(
                &row.state_after_correct,
                &row.state_after_wrong,
            ));
            report.correct_wrong_identical_rows += usize::from(
                collapse_whitespace(&row.state_after_correct)
                    == collapse_whitespace(&row.state_after_wrong),
            );
            let correct_bigram_score =
                source_bigram_overlap_score(&row.state_before, &row.state_after_correct);
            let wrong_bigram_score =
                source_bigram_overlap_score(&row.state_before, &row.state_after_wrong);
            if correct_bigram_score > wrong_bigram_score {
                report.source_bigram_correct_wins += 1;
            } else if wrong_bigram_score > correct_bigram_score {
                report.source_bigram_wrong_wins += 1;
            } else {
                report.source_bigram_ties += 1;
            }
        }

        report.verdict = phase_action_shortcut_v1_verdict(report.gate_pass()).to_string();
        report
    }

    const fn gate_pass(&self) -> bool {
        self.rows > 0
            && self.train_rows > 0
            && self.heldout_rows > 0
            && self.operator_key_count > 1
            && self.heldout_operator_keys_seen_in_train_rows == self.heldout_rows
            && self.heldout_operator_key_missing_rows == 0
            && self.exact_state_lookup_hits == 0
            && self.exact_transition_lookup_hits == 0
            && self.heldout_token_overlap_rows == 0
            && self.heldout_length_seen_in_train_rows == 0
            && self.non_same_bag_rows == 0
            && self.correct_wrong_identical_rows == 0
            && self.source_bigram_correct_wins == 0
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
    }
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionRuntimeReport {
    schema_version: String,
    verdict: String,
    corpus_path: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    cells: usize,
    operator_key_count: usize,
    flat_records: usize,
    runtime_bytes_estimate: usize,
    skipped_train_rows: usize,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_eval_rows: usize,
    action_ablation_missing_centers: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    contract_verdict: String,
    compiler_path: String,
    runtime_path: String,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

struct PhaseActionRuntimeReportInput<'a> {
    corpus_path: &'a Path,
    cells: usize,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    contract_report: PhaseActionContractReport,
    runtime: &'a PhaseCenterFlatRuntime,
    operator_key_count: usize,
    skipped_train_rows: usize,
    prepared: &'a PreparedEval,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
}

impl PhaseActionRuntimeReport {
    fn from_run(input: PhaseActionRuntimeReportInput<'_>) -> Self {
        let mut report = Self {
            schema_version: "nando_phase_action_runtime_report_v1".to_string(),
            verdict: String::new(),
            corpus_path: input.corpus_path.display().to_string(),
            rows: input.rows,
            train_rows: input.train_rows,
            heldout_rows: input.heldout_rows,
            cells: input.cells,
            operator_key_count: input.operator_key_count,
            flat_records: input.runtime.record_count(),
            runtime_bytes_estimate: input.runtime.bytes_estimate(),
            skipped_train_rows: input.skipped_train_rows,
            missing_centers: input.prepared.missing_centers,
            skipped_rows: input.prepared.skipped_rows,
            action_ablation_eval_rows: input.prepared.action_ablation_tasks.len(),
            action_ablation_missing_centers: input.prepared.action_ablation_missing_centers,
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            contract_verdict: input.contract_report.verdict,
            compiler_path: "nando_core::PhaseCenterCompiler".to_string(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            python_demo_used: false,
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            claim_boundary: String::from(
                "clean action_contract_v1 compiler/runtime smoke; not a broad action-router proof",
            ),
        };
        report.verdict = phase_action_runtime_v1_verdict(report.gate_pass()).to_string();
        report
    }

    const fn gate_pass(&self) -> bool {
        self.rows > 0
            && self.train_rows > 0
            && self.heldout_rows > 0
            && self.operator_key_count > 1
            && self.operator_key_count == self.flat_records
            && self.skipped_train_rows == 0
            && self.missing_centers == 0
            && self.skipped_rows == 0
            && self.action_ablation_eval_rows > 0
            && self.action_ablation_missing_centers == 0
            && self.accuracy_milli == 1000
            && self.wrong_wins == 0
            && self.action_ablation_accuracy_milli < self.accuracy_milli
            && self.action_ablation_wrong_wins > 0
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionPackageManifest {
    schema_version: String,
    package_kind: String,
    verdict: String,
    corpus_path: String,
    #[serde(default)]
    source_contract_fingerprint64: u64,
    #[serde(default)]
    source_contract_bytes: usize,
    package_path: String,
    manifest_path: String,
    command: String,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    cells: usize,
    flat_records: usize,
    operator_keys: Vec<String>,
    skipped_train_rows: usize,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_eval_rows: usize,
    action_ablation_missing_centers: usize,
    package_magic: Vec<u8>,
    inspected_payload_bytes: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    serialized_len: usize,
    runtime_bytes_estimate: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    contract_verdict: String,
    compiler_path: String,
    package_path_api: String,
    runtime_path: String,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
    license_boundary: String,
}

struct PhaseActionPackageManifestInput<'a> {
    corpus_path: &'a Path,
    source_contract_fingerprint64: u64,
    source_contract_bytes: usize,
    package_path: &'a Path,
    manifest_path: &'a Path,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    cells: usize,
    key_to_index: &'a BTreeMap<String, usize>,
    skipped_train_rows: usize,
    prepared: &'a PreparedEval,
    loaded_runtime: &'a PhaseCenterFlatRuntime,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
    contract_verdict: &'a str,
}

impl PhaseActionPackageManifest {
    fn from_run(input: PhaseActionPackageManifestInput<'_>) -> Self {
        let operator_keys = operator_keys_by_index(input.key_to_index);
        let gate_pass = action_package_v1_gate_pass(
            &input.eval,
            input.prepared,
            input.skipped_train_rows,
            &input.action_ablation_eval,
            PackageGateMeta {
                package_fingerprint64: input.package_info.fingerprint64,
                operator_key_count: operator_keys.len(),
                record_count: input.loaded_runtime.record_count(),
                has_empty_operator_key: operator_keys.iter().any(|key| key.is_empty()),
            },
            input.contract_verdict,
        );
        Self {
            schema_version: "nando_phase_action_package_manifest_v1".to_string(),
            package_kind: "phase_action_contract_v1_c32_smoke".to_string(),
            verdict: phase_action_package_v1_verdict(gate_pass).to_string(),
            corpus_path: input.corpus_path.display().to_string(),
            source_contract_fingerprint64: input.source_contract_fingerprint64,
            source_contract_bytes: input.source_contract_bytes,
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            command: "cargo run -p nando-cli --release -- phase-action-package-v1".to_string(),
            rows: input.rows,
            train_rows: input.train_rows,
            heldout_rows: input.heldout_rows,
            cells: input.cells,
            flat_records: input.loaded_runtime.record_count(),
            operator_keys,
            skipped_train_rows: input.skipped_train_rows,
            missing_centers: input.prepared.missing_centers,
            skipped_rows: input.prepared.skipped_rows,
            action_ablation_eval_rows: input.prepared.action_ablation_tasks.len(),
            action_ablation_missing_centers: input.prepared.action_ablation_missing_centers,
            package_magic: input.package_info.magic.to_vec(),
            inspected_payload_bytes: input.package_info.payload_bytes,
            package_fingerprint64: input.package_info.fingerprint64,
            package_bytes: input.package_bytes_len,
            serialized_len: input.loaded_runtime.serialized_len(),
            runtime_bytes_estimate: input.loaded_runtime.bytes_estimate(),
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            contract_verdict: input.contract_verdict.to_string(),
            compiler_path: "nando_core::PhaseCenterCompiler".to_string(),
            package_path_api: "nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes".to_string(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            python_demo_used: false,
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            claim_boundary: "clean action_tree package smoke; not a broad action-router proof"
                .to_string(),
            license_boundary: "non-commercial license-file metadata is declared; commercial license package is not closed by this action smoke gate".to_string(),
        }
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_PACKAGE_V1_PASS"
            && self.contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.source_contract_fingerprint64 != 0
            && self.source_contract_bytes > 0
            && self.package_fingerprint64 != 0
            && self.operator_keys.len() == self.flat_records
            && self.operator_keys.iter().all(|key| !key.is_empty())
            && self.skipped_train_rows == 0
            && self.missing_centers == 0
            && self.skipped_rows == 0
            && self.action_ablation_eval_rows > 0
            && self.action_ablation_missing_centers == 0
            && self.accuracy_milli == 1000
            && self.wrong_wins == 0
            && self.action_ablation_accuracy_milli < self.accuracy_milli
            && self.action_ablation_wrong_wins > 0
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CoverageTransform {
    Reverse,
    RotateLeft,
    RotateRight,
    AdjacentPairSwap,
    SwapHalves,
    EvenThenOdd,
    OddThenEven,
    InnerReverse,
    MoveSecondToTail,
    MovePenultimateToFront,
}

#[derive(Clone, Copy, Debug)]
struct CoverageActionSpec {
    key: &'static str,
    token_prefix: &'static str,
    select: &'static str,
    transform: &'static str,
    write: &'static str,
    condition: &'static str,
    check: &'static str,
    transform_kind: CoverageTransform,
}

impl CoverageActionSpec {
    const fn all() -> [Self; 30] {
        [
            Self {
                key: "select_span_reverse_replace_always_bag",
                token_prefix: "span",
                select: "the complete ordered sequence span",
                transform: "reverse the selected span order",
                write: "replace the selected span with the transformed span",
                condition: "always",
                check: "verify the same token bag and reversed adjacency",
                transform_kind: CoverageTransform::Reverse,
            },
            Self {
                key: "select_window_rotate_left_buffer_guard_shift",
                token_prefix: "win",
                select: "the marker-bounded record window",
                transform: "move the leading element after the final element",
                write: "write the transformed span into the active response buffer",
                condition: "when the guard marker is present",
                check: "verify the same token bag and one-step shift",
                transform_kind: CoverageTransform::RotateLeft,
            },
            Self {
                key: "select_field_rotate_right_pending_compare_boundary",
                token_prefix: "field",
                select: "the field-value record span",
                transform: "move the final element before the leading element",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the compare gate accepts the selected span",
                check: "verify the boundary item moved to the front",
                transform_kind: CoverageTransform::RotateRight,
            },
            Self {
                key: "select_predicate_pair_swap_normalize_evidence_pairs",
                token_prefix: "pred",
                select: "the predicate-matching stable span",
                transform: "exchange each adjacent pair while preserving pair boundaries",
                write: "overwrite the selected window with the normalized span",
                condition: "when the evidence channel is complete",
                check: "verify adjacent pairs changed and no token disappeared",
                transform_kind: CoverageTransform::AdjacentPairSwap,
            },
            Self {
                key: "select_buffer_swap_halves_next_route_blocks",
                token_prefix: "buf",
                select: "the current workflow buffer span",
                transform: "place the later block before the earlier block",
                write: "write the verified span into the next-state buffer",
                condition: "when the route branch is active",
                check: "verify two contiguous blocks exchanged",
                transform_kind: CoverageTransform::SwapHalves,
            },
            Self {
                key: "select_audit_even_odd_replace_always_groups",
                token_prefix: "audit",
                select: "the audit evidence window",
                transform: "emit even-positioned elements before odd-positioned elements",
                write: "replace the selected span with the transformed span",
                condition: "always",
                check: "verify stable order inside both parity groups",
                transform_kind: CoverageTransform::EvenThenOdd,
            },
            Self {
                key: "select_complete_odd_even_buffer_guard_groups",
                token_prefix: "span",
                select: "the complete ordered sequence span",
                transform: "emit odd-positioned elements before even-positioned elements",
                write: "write the transformed span into the active response buffer",
                condition: "when the guard marker is present",
                check: "verify stable order inside both inverse parity groups",
                transform_kind: CoverageTransform::OddThenEven,
            },
            Self {
                key: "select_window_inner_reverse_pending_compare_interior",
                token_prefix: "win",
                select: "the marker-bounded record window",
                transform: "keep boundary elements fixed and reverse the interior span",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the compare gate accepts the selected span",
                check: "verify boundaries stayed fixed and the interior changed",
                transform_kind: CoverageTransform::InnerReverse,
            },
            Self {
                key: "select_field_second_tail_normalize_evidence_relocate",
                token_prefix: "field",
                select: "the field-value record span",
                transform: "move the second element after the final element",
                write: "overwrite the selected window with the normalized span",
                condition: "when the evidence channel is complete",
                check: "verify one early element relocated to the tail",
                transform_kind: CoverageTransform::MoveSecondToTail,
            },
            Self {
                key: "select_predicate_penultimate_front_next_route_relocate",
                token_prefix: "pred",
                select: "the predicate-matching stable span",
                transform: "move the element before the final element before the leading element",
                write: "write the verified span into the next-state buffer",
                condition: "when the route branch is active",
                check: "verify one late element relocated to the front",
                transform_kind: CoverageTransform::MovePenultimateToFront,
            },
            Self {
                key: "select_buffer_reverse_replace_guard_bag",
                token_prefix: "buf",
                select: "the current workflow buffer span",
                transform: "reverse the selected span order",
                write: "replace the selected span with the transformed span",
                condition: "when the guard marker is present",
                check: "verify the same token bag and reversed adjacency",
                transform_kind: CoverageTransform::Reverse,
            },
            Self {
                key: "select_audit_rotate_left_buffer_compare_shift",
                token_prefix: "audit",
                select: "the audit evidence window",
                transform: "move the leading element after the final element",
                write: "write the transformed span into the active response buffer",
                condition: "when the compare gate accepts the selected span",
                check: "verify the same token bag and one-step shift",
                transform_kind: CoverageTransform::RotateLeft,
            },
            Self {
                key: "select_complete_rotate_right_pending_evidence_boundary",
                token_prefix: "span",
                select: "the complete ordered sequence span",
                transform: "move the final element before the leading element",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the evidence channel is complete",
                check: "verify the boundary item moved to the front",
                transform_kind: CoverageTransform::RotateRight,
            },
            Self {
                key: "select_window_pair_swap_normalize_route_pairs",
                token_prefix: "win",
                select: "the marker-bounded record window",
                transform: "exchange each adjacent pair while preserving pair boundaries",
                write: "overwrite the selected window with the normalized span",
                condition: "when the route branch is active",
                check: "verify adjacent pairs changed and no token disappeared",
                transform_kind: CoverageTransform::AdjacentPairSwap,
            },
            Self {
                key: "select_field_swap_halves_next_always_blocks",
                token_prefix: "field",
                select: "the field-value record span",
                transform: "place the later block before the earlier block",
                write: "write the verified span into the next-state buffer",
                condition: "always",
                check: "verify two contiguous blocks exchanged",
                transform_kind: CoverageTransform::SwapHalves,
            },
            Self {
                key: "select_predicate_even_odd_replace_compare_groups",
                token_prefix: "pred",
                select: "the predicate-matching stable span",
                transform: "emit even-positioned elements before odd-positioned elements",
                write: "replace the selected span with the transformed span",
                condition: "when the compare gate accepts the selected span",
                check: "verify stable order inside both parity groups",
                transform_kind: CoverageTransform::EvenThenOdd,
            },
            Self {
                key: "select_buffer_odd_even_buffer_evidence_groups",
                token_prefix: "buf",
                select: "the current workflow buffer span",
                transform: "emit odd-positioned elements before even-positioned elements",
                write: "write the transformed span into the active response buffer",
                condition: "when the evidence channel is complete",
                check: "verify stable order inside both inverse parity groups",
                transform_kind: CoverageTransform::OddThenEven,
            },
            Self {
                key: "select_audit_inner_reverse_pending_route_interior",
                token_prefix: "audit",
                select: "the audit evidence window",
                transform: "keep boundary elements fixed and reverse the interior span",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the route branch is active",
                check: "verify boundaries stayed fixed and the interior changed",
                transform_kind: CoverageTransform::InnerReverse,
            },
            Self {
                key: "select_complete_second_tail_normalize_always_relocate",
                token_prefix: "span",
                select: "the complete ordered sequence span",
                transform: "move the second element after the final element",
                write: "overwrite the selected window with the normalized span",
                condition: "always",
                check: "verify one early element relocated to the tail",
                transform_kind: CoverageTransform::MoveSecondToTail,
            },
            Self {
                key: "select_window_penultimate_front_next_guard_relocate",
                token_prefix: "win",
                select: "the marker-bounded record window",
                transform: "move the element before the final element before the leading element",
                write: "write the verified span into the next-state buffer",
                condition: "when the guard marker is present",
                check: "verify one late element relocated to the front",
                transform_kind: CoverageTransform::MovePenultimateToFront,
            },
            Self {
                key: "select_field_reverse_replace_route_bag",
                token_prefix: "field",
                select: "the field-value record span",
                transform: "reverse the selected span order",
                write: "replace the selected span with the transformed span",
                condition: "when the route branch is active",
                check: "verify the same token bag and reversed adjacency",
                transform_kind: CoverageTransform::Reverse,
            },
            Self {
                key: "select_predicate_rotate_left_buffer_always_shift",
                token_prefix: "pred",
                select: "the predicate-matching stable span",
                transform: "move the leading element after the final element",
                write: "write the transformed span into the active response buffer",
                condition: "always",
                check: "verify the same token bag and one-step shift",
                transform_kind: CoverageTransform::RotateLeft,
            },
            Self {
                key: "select_buffer_rotate_right_pending_guard_boundary",
                token_prefix: "buf",
                select: "the current workflow buffer span",
                transform: "move the final element before the leading element",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the guard marker is present",
                check: "verify the boundary item moved to the front",
                transform_kind: CoverageTransform::RotateRight,
            },
            Self {
                key: "select_audit_pair_swap_normalize_compare_pairs",
                token_prefix: "audit",
                select: "the audit evidence window",
                transform: "exchange each adjacent pair while preserving pair boundaries",
                write: "overwrite the selected window with the normalized span",
                condition: "when the compare gate accepts the selected span",
                check: "verify adjacent pairs changed and no token disappeared",
                transform_kind: CoverageTransform::AdjacentPairSwap,
            },
            Self {
                key: "select_complete_swap_halves_next_evidence_blocks",
                token_prefix: "span",
                select: "the complete ordered sequence span",
                transform: "place the later block before the earlier block",
                write: "write the verified span into the next-state buffer",
                condition: "when the evidence channel is complete",
                check: "verify two contiguous blocks exchanged",
                transform_kind: CoverageTransform::SwapHalves,
            },
            Self {
                key: "select_window_even_odd_replace_route_groups",
                token_prefix: "win",
                select: "the marker-bounded record window",
                transform: "emit even-positioned elements before odd-positioned elements",
                write: "replace the selected span with the transformed span",
                condition: "when the route branch is active",
                check: "verify stable order inside both parity groups",
                transform_kind: CoverageTransform::EvenThenOdd,
            },
            Self {
                key: "select_field_odd_even_buffer_always_groups",
                token_prefix: "field",
                select: "the field-value record span",
                transform: "emit odd-positioned elements before even-positioned elements",
                write: "write the transformed span into the active response buffer",
                condition: "always",
                check: "verify stable order inside both inverse parity groups",
                transform_kind: CoverageTransform::OddThenEven,
            },
            Self {
                key: "select_predicate_inner_reverse_pending_guard_interior",
                token_prefix: "pred",
                select: "the predicate-matching stable span",
                transform: "keep boundary elements fixed and reverse the interior span",
                write: "commit the transformed span to the pending workflow state",
                condition: "when the guard marker is present",
                check: "verify boundaries stayed fixed and the interior changed",
                transform_kind: CoverageTransform::InnerReverse,
            },
            Self {
                key: "select_buffer_second_tail_normalize_compare_relocate",
                token_prefix: "buf",
                select: "the current workflow buffer span",
                transform: "move the second element after the final element",
                write: "overwrite the selected window with the normalized span",
                condition: "when the compare gate accepts the selected span",
                check: "verify one early element relocated to the tail",
                transform_kind: CoverageTransform::MoveSecondToTail,
            },
            Self {
                key: "select_audit_penultimate_front_next_evidence_relocate",
                token_prefix: "audit",
                select: "the audit evidence window",
                transform: "move the element before the final element before the leading element",
                write: "write the verified span into the next-state buffer",
                condition: "when the evidence channel is complete",
                check: "verify one late element relocated to the front",
                transform_kind: CoverageTransform::MovePenultimateToFront,
            },
        ]
    }

    fn action_tree(self) -> PhaseActionTree {
        PhaseActionTree {
            select: self.select.to_string(),
            transform: self.transform.to_string(),
            write: self.write.to_string(),
            condition: self.condition.to_string(),
            check: self.check.to_string(),
        }
    }

    fn apply(self, source: &[String]) -> Vec<String> {
        match self.transform_kind {
            CoverageTransform::Reverse => reversed(source),
            CoverageTransform::RotateLeft => rotate_left(source),
            CoverageTransform::RotateRight => rotate_right(source),
            CoverageTransform::AdjacentPairSwap => adjacent_pair_swap(source),
            CoverageTransform::SwapHalves => swap_halves(source),
            CoverageTransform::EvenThenOdd => even_then_odd(source),
            CoverageTransform::OddThenEven => odd_then_even(source),
            CoverageTransform::InnerReverse => inner_reverse(source),
            CoverageTransform::MoveSecondToTail => move_second_to_tail(source),
            CoverageTransform::MovePenultimateToFront => move_penultimate_to_front(source),
        }
    }

    fn wrong(self, source: &[String]) -> Vec<String> {
        let mut wrong = match self.transform_kind {
            CoverageTransform::Reverse => adjacent_pair_swap(source),
            CoverageTransform::RotateLeft => rotate_right(source),
            CoverageTransform::RotateRight => rotate_left(source),
            CoverageTransform::AdjacentPairSwap => reversed(source),
            CoverageTransform::SwapHalves => rotate_left(source),
            CoverageTransform::EvenThenOdd => odd_then_even(source),
            CoverageTransform::OddThenEven => even_then_odd(source),
            CoverageTransform::InnerReverse => reversed(source),
            CoverageTransform::MoveSecondToTail => move_penultimate_to_front(source),
            CoverageTransform::MovePenultimateToFront => move_second_to_tail(source),
        };
        let correct = self.apply(source);
        if wrong == correct {
            wrong = rotate_left(&correct);
        }
        wrong
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActionCorpusOperator {
    Reverse,
    RotateLeft,
    RotateRight,
    AdjacentPairSwap,
    SwapHalves,
    EvenThenOdd,
    OddThenEven,
    InnerReverse,
    MoveSecondToTail,
    MovePenultimateToFront,
}

impl ActionCorpusOperator {
    const fn all() -> [Self; 10] {
        [
            Self::Reverse,
            Self::RotateLeft,
            Self::RotateRight,
            Self::AdjacentPairSwap,
            Self::SwapHalves,
            Self::EvenThenOdd,
            Self::OddThenEven,
            Self::InnerReverse,
            Self::MoveSecondToTail,
            Self::MovePenultimateToFront,
        ]
    }

    const fn key(self) -> &'static str {
        match self {
            Self::Reverse => "reverse",
            Self::RotateLeft => "rotate_left",
            Self::RotateRight => "rotate_right",
            Self::AdjacentPairSwap => "adjacent_pair_swap",
            Self::SwapHalves => "swap_halves",
            Self::EvenThenOdd => "even_then_odd",
            Self::OddThenEven => "odd_then_even",
            Self::InnerReverse => "inner_reverse",
            Self::MoveSecondToTail => "move_second_to_tail",
            Self::MovePenultimateToFront => "move_penultimate_to_front",
        }
    }

    fn action_tree(self) -> PhaseActionTree {
        let (transform, check) = match self {
            Self::Reverse => (
                "reverse the selected span order",
                "the result preserves the same token bag and reverses adjacency",
            ),
            Self::RotateLeft => (
                "move the leading element after the final element",
                "the result preserves the same token bag and shifts every element once",
            ),
            Self::RotateRight => (
                "move the final element before the leading element",
                "the result preserves the same token bag and shifts every element once",
            ),
            Self::AdjacentPairSwap => (
                "exchange each adjacent pair while preserving pair boundaries",
                "the result preserves the same token bag and swaps only within pairs",
            ),
            Self::SwapHalves => (
                "move the back block before the front block",
                "the result preserves the same token bag and exchanges the two contiguous blocks",
            ),
            Self::EvenThenOdd => (
                "emit the even-positioned elements before the odd-positioned elements",
                "the result preserves the same token bag and keeps stable order inside both groups",
            ),
            Self::OddThenEven => (
                "emit the odd-positioned elements before the even-positioned elements",
                "the result preserves the same token bag and keeps stable order inside both groups",
            ),
            Self::InnerReverse => (
                "keep the boundary elements fixed and reverse the interior span",
                "the result preserves the same token bag and changes only the interior order",
            ),
            Self::MoveSecondToTail => (
                "move the second element after the final element",
                "the result preserves the same token bag and relocates one early element",
            ),
            Self::MovePenultimateToFront => (
                "move the element before the final element before the leading element",
                "the result preserves the same token bag and relocates one late element",
            ),
        };
        PhaseActionTree {
            select: "the complete ordered sequence span".to_string(),
            transform: transform.to_string(),
            write: "replace the selected span with the transformed span".to_string(),
            condition: "always".to_string(),
            check: check.to_string(),
        }
    }

    fn apply(self, source: &[String]) -> Vec<String> {
        match self {
            Self::Reverse => reversed(source),
            Self::RotateLeft => rotate_left(source),
            Self::RotateRight => rotate_right(source),
            Self::AdjacentPairSwap => adjacent_pair_swap(source),
            Self::SwapHalves => swap_halves(source),
            Self::EvenThenOdd => even_then_odd(source),
            Self::OddThenEven => odd_then_even(source),
            Self::InnerReverse => inner_reverse(source),
            Self::MoveSecondToTail => move_second_to_tail(source),
            Self::MovePenultimateToFront => move_penultimate_to_front(source),
        }
    }

    fn wrong(self, source: &[String]) -> Vec<String> {
        let mut wrong = match self {
            Self::Reverse => adjacent_pair_swap(source),
            Self::RotateLeft => rotate_right(source),
            Self::RotateRight => rotate_left(source),
            Self::AdjacentPairSwap => reversed(source),
            Self::SwapHalves => rotate_left(source),
            Self::EvenThenOdd => odd_then_even(source),
            Self::OddThenEven => even_then_odd(source),
            Self::InnerReverse => reversed(source),
            Self::MoveSecondToTail => move_penultimate_to_front(source),
            Self::MovePenultimateToFront => move_second_to_tail(source),
        };
        let correct = self.apply(source);
        if wrong == correct {
            wrong = rotate_left(&correct);
        }
        wrong
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum WorkflowActionOperator {
    EscalateEvidence,
    AuditTrailFirst,
    ReverseHandoff,
    StableGroupRoute,
    SwapWorkBlocks,
    PairReview,
}

impl WorkflowActionOperator {
    const fn all() -> [Self; 6] {
        [
            Self::EscalateEvidence,
            Self::AuditTrailFirst,
            Self::ReverseHandoff,
            Self::StableGroupRoute,
            Self::SwapWorkBlocks,
            Self::PairReview,
        ]
    }

    const fn key(self) -> &'static str {
        match self {
            Self::EscalateEvidence => "escalate_evidence",
            Self::AuditTrailFirst => "audit_trail_first",
            Self::ReverseHandoff => "reverse_handoff",
            Self::StableGroupRoute => "stable_group_route",
            Self::SwapWorkBlocks => "swap_work_blocks",
            Self::PairReview => "pair_review",
        }
    }

    const fn token_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::EscalateEvidence => &[
                "case", "status", "owner", "priority", "evidence", "deadline", "queue", "risk",
            ],
            Self::AuditTrailFirst => &[
                "ticket", "snapshot", "operator", "change", "comment", "approval", "record",
                "trace",
            ],
            Self::ReverseHandoff => &[
                "intake", "triage", "analyst", "review", "handoff", "control", "closure", "archive",
            ],
            Self::StableGroupRoute => &[
                "client",
                "invoice",
                "route",
                "carrier",
                "customs",
                "broker",
                "warehouse",
                "release",
            ],
            Self::SwapWorkBlocks => &[
                "plan", "prepare", "verify", "dispatch", "receive", "inspect", "confirm", "report",
            ],
            Self::PairReview => &[
                "request",
                "answer",
                "claim",
                "evidence",
                "risk",
                "mitigation",
                "owner",
                "deadline",
            ],
        }
    }

    fn action_tree(self) -> PhaseActionTree {
        let (select, transform, check) = match self {
            Self::EscalateEvidence => (
                "the complete operational record span",
                "promote the late evidence-bearing record before the current front record",
                "the transformed span preserves all records and makes the late evidence item leading",
            ),
            Self::AuditTrailFirst => (
                "the complete audit record span",
                "place the final audit trace before the rest of the span",
                "the transformed span preserves all records and rotates exactly one boundary item",
            ),
            Self::ReverseHandoff => (
                "the complete handoff chain span",
                "read the handoff chain from final responsibility back to intake",
                "the transformed span preserves all records and reverses responsibility order",
            ),
            Self::StableGroupRoute => (
                "the complete route record span",
                "emit primary-position records before secondary-position records without disturbing group order",
                "the transformed span preserves all records and keeps stable order inside both groups",
            ),
            Self::SwapWorkBlocks => (
                "the complete two-block work span",
                "place the later work block before the earlier work block",
                "the transformed span preserves all records and exchanges two contiguous work blocks",
            ),
            Self::PairReview => (
                "the complete review pair span",
                "exchange each request-like record with its following response-like record",
                "the transformed span preserves all records and swaps only adjacent review pairs",
            ),
        };
        PhaseActionTree {
            select: select.to_string(),
            transform: transform.to_string(),
            write: "replace the record span with the transformed workflow span".to_string(),
            condition: "always".to_string(),
            check: check.to_string(),
        }
    }

    fn apply(self, source: &[String]) -> Vec<String> {
        match self {
            Self::EscalateEvidence => move_penultimate_to_front(source),
            Self::AuditTrailFirst => rotate_right(source),
            Self::ReverseHandoff => reversed(source),
            Self::StableGroupRoute => even_then_odd(source),
            Self::SwapWorkBlocks => swap_halves(source),
            Self::PairReview => adjacent_pair_swap(source),
        }
    }

    fn wrong(self, source: &[String]) -> Vec<String> {
        let mut wrong = match self {
            Self::EscalateEvidence => move_second_to_tail(source),
            Self::AuditTrailFirst => rotate_left(source),
            Self::ReverseHandoff => adjacent_pair_swap(source),
            Self::StableGroupRoute => odd_then_even(source),
            Self::SwapWorkBlocks => rotate_left(source),
            Self::PairReview => reversed(source),
        };
        let correct = self.apply(source);
        if wrong == correct {
            wrong = rotate_left(&correct);
        }
        wrong
    }
}

impl PhaseEvalTaskPackage {
    fn from_prepared(
        cells: usize,
        package_fingerprint64: u64,
        rows: usize,
        prepared: PreparedEval,
    ) -> Self {
        Self {
            cells,
            package_fingerprint64,
            rows,
            prepared,
        }
    }

    fn serialized_len(&self) -> usize {
        eval_task_package_len(
            self.cells,
            self.prepared.tasks.len(),
            self.prepared.action_ablation_tasks.len(),
        )
        .unwrap_or(usize::MAX)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let serialized_len = eval_task_package_len(
            self.cells,
            self.prepared.tasks.len(),
            self.prepared.action_ablation_tasks.len(),
        )
        .ok_or_else(|| String::from("phase eval task package is too large"))?;
        let cells = u32::try_from(self.cells)
            .map_err(|_| String::from("phase eval task cells exceed u32"))?;
        let rows =
            u32::try_from(self.rows).map_err(|_| String::from("phase eval rows exceed u32"))?;
        let task_count = u32::try_from(self.prepared.tasks.len())
            .map_err(|_| String::from("phase eval task count exceeds u32"))?;
        let action_task_count = u32::try_from(self.prepared.action_ablation_tasks.len())
            .map_err(|_| String::from("phase eval action task count exceeds u32"))?;
        let missing_centers = u32::try_from(self.prepared.missing_centers)
            .map_err(|_| String::from("phase eval missing centers exceed u32"))?;
        let skipped_rows = u32::try_from(self.prepared.skipped_rows)
            .map_err(|_| String::from("phase eval skipped rows exceed u32"))?;
        let action_missing = u32::try_from(self.prepared.action_ablation_missing_centers)
            .map_err(|_| String::from("phase eval action missing centers exceed u32"))?;

        let mut bytes = Vec::with_capacity(serialized_len);
        bytes.extend_from_slice(&PHASE_EVAL_TASK_PACKAGE_MAGIC);
        bytes.extend_from_slice(&cells.to_le_bytes());
        bytes.extend_from_slice(&self.package_fingerprint64.to_le_bytes());
        bytes.extend_from_slice(&rows.to_le_bytes());
        bytes.extend_from_slice(&task_count.to_le_bytes());
        bytes.extend_from_slice(&action_task_count.to_le_bytes());
        bytes.extend_from_slice(&missing_centers.to_le_bytes());
        bytes.extend_from_slice(&skipped_rows.to_le_bytes());
        bytes.extend_from_slice(&action_missing.to_le_bytes());
        write_eval_task_list(&mut bytes, &self.prepared.tasks)?;
        write_eval_task_list(&mut bytes, &self.prepared.action_ablation_tasks)?;
        Ok(bytes)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ForbiddenFlags {
    epoch_repair_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

impl ForbiddenFlags {
    const fn none() -> Self {
        Self {
            epoch_repair_used: false,
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
        }
    }

    const fn any_forbidden_used(&self) -> bool {
        self.epoch_repair_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }
}

struct PhasePackageManifestInput<'a> {
    corpus_path: &'a Path,
    package_path: &'a Path,
    manifest_path: &'a Path,
    rows: usize,
    train_rows: usize,
    heldout_rows: usize,
    cells: usize,
    skipped_train_rows: usize,
    prepared: &'a PreparedEval,
    key_to_index: &'a BTreeMap<String, usize>,
    loaded_runtime: &'a PhaseCenterFlatRuntime,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
}

impl PhasePackageManifest {
    fn from_run(input: PhasePackageManifestInput<'_>) -> Self {
        let operator_keys = operator_keys_by_index(input.key_to_index);
        let gate_pass = package_v4_gate_pass(
            &input.eval,
            input.prepared,
            input.skipped_train_rows,
            &input.action_ablation_eval,
            PackageGateMeta {
                package_fingerprint64: input.package_info.fingerprint64,
                operator_key_count: operator_keys.len(),
                record_count: input.loaded_runtime.record_count(),
                has_empty_operator_key: operator_keys.iter().any(|key| key.is_empty()),
            },
        );
        Self {
            schema_version: "nando_phase_package_manifest_v1".to_string(),
            package_kind: "phase_center_v4_c32_scorer".to_string(),
            verdict: phase_package_v4_verdict(gate_pass).to_string(),
            corpus_path: input.corpus_path.display().to_string(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            command: "cargo run -p nando-cli --release -- phase-package-v4".to_string(),
            rows: input.rows,
            train_rows: input.train_rows,
            heldout_rows: input.heldout_rows,
            cells: input.cells,
            flat_records: input.loaded_runtime.record_count(),
            operator_keys,
            skipped_train_rows: input.skipped_train_rows,
            missing_centers: input.prepared.missing_centers,
            skipped_rows: input.prepared.skipped_rows,
            action_ablation_eval_rows: input.prepared.action_ablation_tasks.len(),
            action_ablation_missing_centers: input.prepared.action_ablation_missing_centers,
            heldout_surface_groups: input.prepared.heldout_surface_groups,
            heldout_noise_groups: input.prepared.heldout_noise_groups,
            package_magic: input.package_info.magic.to_vec(),
            inspected_payload_bytes: input.package_info.payload_bytes,
            package_fingerprint64: input.package_info.fingerprint64,
            package_bytes: input.package_bytes_len,
            serialized_len: input.loaded_runtime.serialized_len(),
            runtime_bytes_estimate: input.loaded_runtime.bytes_estimate(),
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            compiler_path: "nando_core::PhaseCenterCompiler".to_string(),
            package_path_api: "nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes".to_string(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            forbidden_flags: ForbiddenFlags::none(),
            claim_boundary: "phase-center scorer package; not strict ordered decoder, text generation, multi-step reasoning, or multi-seed strict readout robustness".to_string(),
            license_boundary: "non-commercial license-file metadata is declared; commercial license package is not closed by this scorer gate".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionSourceVerifyReport {
    schema_version: String,
    verdict: String,
    package_kind: String,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    source_contract_fingerprint64: u64,
    source_contract_bytes: usize,
    source_rebuild_matches_package: bool,
    source_rebuild_package_fingerprint64: u64,
    source_rebuild_package_bytes: usize,
    source_rebuild_flat_records: usize,
    source_rebuild_operator_keys_match: bool,
    source_rebuild_contract_verdict: String,
    #[serde(default)]
    source_rebuild_contract_gate_pass: bool,
    #[serde(default)]
    source_rebuild_accepted_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_rejected_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_operator_label_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_slot_map_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_target_leak_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_lookup_authority_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_local_out_t_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_arrow_demo_rows: usize,
    #[serde(default)]
    source_rebuild_concrete_output_token_leak_rows: usize,
    #[serde(default)]
    source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_train_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_heldout_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_min_train_rows_per_action_tree: usize,
    #[serde(default)]
    source_rebuild_min_heldout_rows_per_action_tree: usize,
    source_rebuild_skipped_train_rows: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    cells: usize,
    flat_records: usize,
    runtime_bytes_estimate: usize,
    manifest_matches_package: bool,
    manifest_gate_pass: bool,
    compiler_path: String,
    package_path_api: String,
    runtime_path: String,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

struct PhaseActionSourceVerifyReportInput<'a> {
    package_path: &'a Path,
    manifest_path: &'a Path,
    manifest: &'a PhaseActionPackageManifest,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime_bytes_estimate: usize,
    manifest_matches_package: bool,
    source_rebuild: &'a PhaseActionSourceRebuildAudit,
}

impl PhaseActionSourceVerifyReport {
    fn from_inputs(input: PhaseActionSourceVerifyReportInput<'_>) -> Self {
        let mut report = Self {
            schema_version: "nando_phase_action_source_verify_report_v1".to_string(),
            verdict: "PHASE_ACTION_SOURCE_VERIFY_V1_WATCH".to_string(),
            package_kind: input.manifest.package_kind.clone(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            corpus_path: input.manifest.corpus_path.clone(),
            source_contract_fingerprint64: input.source_rebuild.source_contract_fingerprint64,
            source_contract_bytes: input.source_rebuild.source_contract_bytes,
            source_rebuild_matches_package: input.source_rebuild.source_rebuild_matches_package,
            source_rebuild_package_fingerprint64: input
                .source_rebuild
                .source_rebuild_package_fingerprint64,
            source_rebuild_package_bytes: input.source_rebuild.source_rebuild_package_bytes,
            source_rebuild_flat_records: input.source_rebuild.source_rebuild_flat_records,
            source_rebuild_operator_keys_match: input
                .source_rebuild
                .source_rebuild_operator_keys_match,
            source_rebuild_contract_verdict: input
                .source_rebuild
                .source_rebuild_contract_verdict
                .clone(),
            source_rebuild_contract_gate_pass: input.source_rebuild.source_rebuild_contract_gate_pass,
            source_rebuild_accepted_action_tree_rows: input
                .source_rebuild
                .source_rebuild_accepted_action_tree_rows,
            source_rebuild_rejected_action_tree_rows: input
                .source_rebuild
                .source_rebuild_rejected_action_tree_rows,
            source_rebuild_forbidden_operator_label_rows: input
                .source_rebuild
                .source_rebuild_forbidden_operator_label_rows,
            source_rebuild_forbidden_slot_map_rows: input
                .source_rebuild
                .source_rebuild_forbidden_slot_map_rows,
            source_rebuild_forbidden_target_leak_rows: input
                .source_rebuild
                .source_rebuild_forbidden_target_leak_rows,
            source_rebuild_forbidden_lookup_authority_rows: input
                .source_rebuild
                .source_rebuild_forbidden_lookup_authority_rows,
            source_rebuild_forbidden_local_out_t_rows: input
                .source_rebuild
                .source_rebuild_forbidden_local_out_t_rows,
            source_rebuild_forbidden_arrow_demo_rows: input
                .source_rebuild
                .source_rebuild_forbidden_arrow_demo_rows,
            source_rebuild_concrete_output_token_leak_rows: input
                .source_rebuild
                .source_rebuild_concrete_output_token_leak_rows,
            source_rebuild_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_action_tree_key_count,
            source_rebuild_train_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_train_action_tree_key_count,
            source_rebuild_heldout_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_heldout_action_tree_key_count,
            source_rebuild_min_train_rows_per_action_tree: input
                .source_rebuild
                .source_rebuild_min_train_rows_per_action_tree,
            source_rebuild_min_heldout_rows_per_action_tree: input
                .source_rebuild
                .source_rebuild_min_heldout_rows_per_action_tree,
            source_rebuild_skipped_train_rows: input
                .source_rebuild
                .source_rebuild_skipped_train_rows,
            package_fingerprint64: input.package_info.fingerprint64,
            package_bytes: input.package_bytes_len,
            cells: input.package_info.cells,
            flat_records: input.package_info.record_count,
            runtime_bytes_estimate: input.runtime_bytes_estimate,
            manifest_matches_package: input.manifest_matches_package,
            manifest_gate_pass: input.manifest.gate_pass(),
            compiler_path: "nando_core::PhaseCenterCompiler".to_string(),
            package_path_api: "nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes".to_string(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            python_demo_used: input.manifest.python_demo_used,
            target_center_id_training_used: input.manifest.target_center_id_training_used,
            proof_rule_id_training_authority_used: input
                .manifest
                .proof_rule_id_training_authority_used,
            concrete_x_lookup_used: input.manifest.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: input
                .manifest
                .local_out_t_runtime_extension_used,
            claim_boundary:
                "source-rebuild verifier for packaged flat action scorer; not score/bench/offload, strict ordered decoder, text generation, or autonomous raw action-router proof"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_SOURCE_VERIFY_V1_PASS".to_string();
        }
        report
    }

    const fn forbidden_used(&self) -> bool {
        self.python_demo_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_source_verify_report_v1"
            && self.package_kind == "phase_action_contract_v1_c32_smoke"
            && self.source_contract_fingerprint64 != 0
            && self.source_contract_bytes > 0
            && self.source_rebuild_matches_package
            && self.source_rebuild_package_fingerprint64 == self.package_fingerprint64
            && self.source_rebuild_package_bytes == self.package_bytes
            && self.source_rebuild_flat_records == self.flat_records
            && self.source_rebuild_operator_keys_match
            && self.source_rebuild_contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.source_rebuild_contract_gate_pass
            && self.source_rebuild_accepted_action_tree_rows > 0
            && self.source_rebuild_rejected_action_tree_rows == 0
            && self.source_rebuild_forbidden_operator_label_rows == 0
            && self.source_rebuild_forbidden_slot_map_rows == 0
            && self.source_rebuild_forbidden_target_leak_rows == 0
            && self.source_rebuild_forbidden_lookup_authority_rows == 0
            && self.source_rebuild_forbidden_local_out_t_rows == 0
            && self.source_rebuild_forbidden_arrow_demo_rows == 0
            && self.source_rebuild_concrete_output_token_leak_rows == 0
            && self.source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.source_rebuild_train_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_heldout_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_min_train_rows_per_action_tree > 0
            && self.source_rebuild_min_heldout_rows_per_action_tree > 0
            && self.source_rebuild_skipped_train_rows == 0
            && self.package_fingerprint64 != 0
            && self.package_bytes > 0
            && self.cells > 0
            && self.flat_records > 0
            && self.runtime_bytes_estimate > 0
            && self.manifest_matches_package
            && self.manifest_gate_pass
            && self.compiler_path == "nando_core::PhaseCenterCompiler"
            && self.package_path_api == "nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes"
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && !self.forbidden_used()
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_SOURCE_VERIFY_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionPackageScoreReport {
    schema_version: String,
    package_kind: String,
    verdict: String,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    #[serde(default)]
    eval_task_package_path: String,
    #[serde(default)]
    eval_task_package_used: bool,
    #[serde(default)]
    corpus_jsonl_used_in_score_loop: Option<bool>,
    cells: usize,
    flat_records: usize,
    manifest_operator_keys: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_payload_bytes: usize,
    runtime_bytes_estimate: usize,
    rows: usize,
    heldout_eval_rows: usize,
    missing_centers: usize,
    skipped_rows: usize,
    action_ablation_eval_rows: usize,
    action_ablation_missing_centers: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    compiler_used: bool,
    #[serde(default)]
    optimized_build: bool,
    contract_verdict: String,
    manifest_verdict: String,
    runtime_path: String,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
    license_boundary: String,
}

struct PhaseActionPackageScoreReportInput<'a> {
    verdict: &'static str,
    package_path: &'a Path,
    manifest_path: &'a Path,
    corpus_path: &'a Path,
    eval_task_package_path: Option<&'a Path>,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime: &'a PhaseCenterFlatRuntime,
    manifest: &'a PhaseActionPackageManifest,
    rows: usize,
    prepared: &'a PreparedEval,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
    compiler_used: bool,
    contract_verdict: &'a str,
    eval_task_package_used: bool,
    corpus_jsonl_used_in_score_loop: bool,
}

impl PhaseActionPackageScoreReport {
    fn from_score(input: PhaseActionPackageScoreReportInput<'_>) -> Self {
        Self {
            schema_version: "nando_phase_action_package_score_report_v1".to_string(),
            package_kind: input.manifest.package_kind.clone(),
            verdict: input.verdict.to_string(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            corpus_path: input.corpus_path.display().to_string(),
            eval_task_package_path: input
                .eval_task_package_path
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            eval_task_package_used: input.eval_task_package_used,
            corpus_jsonl_used_in_score_loop: Some(input.corpus_jsonl_used_in_score_loop),
            cells: input.manifest.cells,
            flat_records: input.runtime.record_count(),
            manifest_operator_keys: input.manifest.operator_keys.len(),
            package_fingerprint64: input.package_info.fingerprint64,
            package_bytes: input.package_bytes_len,
            inspected_payload_bytes: input.package_info.payload_bytes,
            runtime_bytes_estimate: input.runtime.bytes_estimate(),
            rows: input.rows,
            heldout_eval_rows: input.prepared.tasks.len(),
            missing_centers: input.prepared.missing_centers,
            skipped_rows: input.prepared.skipped_rows,
            action_ablation_eval_rows: input.prepared.action_ablation_tasks.len(),
            action_ablation_missing_centers: input.prepared.action_ablation_missing_centers,
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            compiler_used: input.compiler_used,
            optimized_build: ACTION_OPTIMIZED_BUILD,
            contract_verdict: input.contract_verdict.to_string(),
            manifest_verdict: input.manifest.verdict.clone(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            python_demo_used: false,
            target_center_id_training_used: input.manifest.target_center_id_training_used,
            proof_rule_id_training_authority_used: input
                .manifest
                .proof_rule_id_training_authority_used,
            concrete_x_lookup_used: input.manifest.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: input.manifest.local_out_t_runtime_extension_used,
            claim_boundary: input.manifest.claim_boundary.clone(),
            license_boundary: input.manifest.license_boundary.clone(),
        }
    }

    const fn forbidden_used(&self) -> bool {
        self.python_demo_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }

    fn gate_pass(&self) -> bool {
        action_score_report_verdict_gate_pass(self)
            && self.contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.manifest_verdict == "PHASE_ACTION_PACKAGE_V1_PASS"
            && self.package_fingerprint64 != 0
            && self.manifest_operator_keys == self.flat_records
            && self.missing_centers == 0
            && self.skipped_rows == 0
            && self.action_ablation_missing_centers == 0
            && self.heldout_eval_rows > 0
            && self.action_ablation_eval_rows > 0
            && self.accuracy_milli == 1000
            && self.wrong_wins == 0
            && self.action_ablation_accuracy_milli < self.accuracy_milli
            && self.action_ablation_wrong_wins > 0
            && !self.compiler_used
            && self.optimized_build
            && !self.forbidden_used()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionPackageBenchReport {
    schema_version: String,
    package_kind: String,
    verdict: String,
    package_path: String,
    manifest_path: String,
    eval_task_package_path: String,
    cells: usize,
    flat_records: usize,
    manifest_operator_keys: usize,
    package_fingerprint64: u64,
    eval_pack_package_fingerprint64: u64,
    package_bytes: usize,
    eval_pack_bytes: usize,
    inspected_payload_bytes: usize,
    runtime_bytes_estimate: usize,
    rows: usize,
    heldout_eval_rows: usize,
    action_ablation_eval_rows: usize,
    bench_iterations: usize,
    bench_samples: usize,
    action_ablation_bench_samples: usize,
    accuracy_milli: usize,
    wrong_wins: usize,
    median_margin: f64,
    p10_margin: f64,
    p50_latency_ns: u128,
    p99_latency_ns: u128,
    total_eval_us: u128,
    rows_per_second: f64,
    action_ablation_accuracy_milli: usize,
    action_ablation_wrong_wins: usize,
    action_ablation_median_margin: f64,
    action_ablation_p10_margin: f64,
    action_ablation_p50_latency_ns: u128,
    action_ablation_p99_latency_ns: u128,
    compiler_used: bool,
    #[serde(default)]
    optimized_build: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used_in_bench_loop: bool,
    contract_verdict: String,
    manifest_verdict: String,
    runtime_path: String,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    p99_latency_gate_ns: u128,
    claim_boundary: String,
    license_boundary: String,
}

struct PhaseActionPackageBenchReportInput<'a> {
    verdict: &'static str,
    package_path: &'a Path,
    manifest_path: &'a Path,
    eval_task_package_path: &'a Path,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    eval_package: &'a PhaseEvalTaskPackage,
    runtime: &'a PhaseCenterFlatRuntime,
    manifest: &'a PhaseActionPackageManifest,
    bench_iterations: usize,
    eval: RuntimeEval,
    action_ablation_eval: RuntimeEval,
}

impl PhaseActionPackageBenchReport {
    fn from_bench(input: PhaseActionPackageBenchReportInput<'_>) -> Self {
        let bench_samples = input
            .eval_package
            .prepared
            .tasks
            .len()
            .saturating_mul(input.bench_iterations);
        let action_ablation_bench_samples = input
            .eval_package
            .prepared
            .action_ablation_tasks
            .len()
            .saturating_mul(input.bench_iterations);
        Self {
            schema_version: "nando_phase_action_package_bench_report_v1".to_string(),
            package_kind: input.manifest.package_kind.clone(),
            verdict: input.verdict.to_string(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            eval_task_package_path: input.eval_task_package_path.display().to_string(),
            cells: input.manifest.cells,
            flat_records: input.runtime.record_count(),
            manifest_operator_keys: input.manifest.operator_keys.len(),
            package_fingerprint64: input.package_info.fingerprint64,
            eval_pack_package_fingerprint64: input.eval_package.package_fingerprint64,
            package_bytes: input.package_bytes_len,
            eval_pack_bytes: input.eval_package.serialized_len(),
            inspected_payload_bytes: input.package_info.payload_bytes,
            runtime_bytes_estimate: input.runtime.bytes_estimate(),
            rows: input.eval_package.rows,
            heldout_eval_rows: input.eval_package.prepared.tasks.len(),
            action_ablation_eval_rows: input.eval_package.prepared.action_ablation_tasks.len(),
            bench_iterations: input.bench_iterations,
            bench_samples,
            action_ablation_bench_samples,
            accuracy_milli: input.eval.accuracy_milli,
            wrong_wins: input.eval.wrong_wins,
            median_margin: input.eval.median_margin,
            p10_margin: input.eval.p10_margin,
            p50_latency_ns: input.eval.p50_latency_ns,
            p99_latency_ns: input.eval.p99_latency_ns,
            total_eval_us: input.eval.total_eval_us,
            rows_per_second: input.eval.rows_per_second,
            action_ablation_accuracy_milli: input.action_ablation_eval.accuracy_milli,
            action_ablation_wrong_wins: input.action_ablation_eval.wrong_wins,
            action_ablation_median_margin: input.action_ablation_eval.median_margin,
            action_ablation_p10_margin: input.action_ablation_eval.p10_margin,
            action_ablation_p50_latency_ns: input.action_ablation_eval.p50_latency_ns,
            action_ablation_p99_latency_ns: input.action_ablation_eval.p99_latency_ns,
            compiler_used: false,
            optimized_build: ACTION_OPTIMIZED_BUILD,
            eval_task_package_used: true,
            corpus_jsonl_used_in_bench_loop: false,
            contract_verdict: input.manifest.contract_verdict.clone(),
            manifest_verdict: input.manifest.verdict.clone(),
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            python_demo_used: false,
            target_center_id_training_used: input.manifest.target_center_id_training_used,
            proof_rule_id_training_authority_used: input
                .manifest
                .proof_rule_id_training_authority_used,
            concrete_x_lookup_used: input.manifest.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: input.manifest.local_out_t_runtime_extension_used,
            p99_latency_gate_ns: ACTION_BENCH_P99_NS_GATE,
            claim_boundary: input.manifest.claim_boundary.clone(),
            license_boundary: input.manifest.license_boundary.clone(),
        }
    }

    const fn forbidden_used(&self) -> bool {
        self.python_demo_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS"
            && self.contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.manifest_verdict == "PHASE_ACTION_PACKAGE_V1_PASS"
            && self.package_fingerprint64 != 0
            && self.package_fingerprint64 == self.eval_pack_package_fingerprint64
            && self.manifest_operator_keys == self.flat_records
            && self.heldout_eval_rows > 0
            && self.action_ablation_eval_rows > 0
            && self.bench_iterations > 0
            && self.bench_samples == self.heldout_eval_rows.saturating_mul(self.bench_iterations)
            && self.action_ablation_bench_samples
                == self
                    .action_ablation_eval_rows
                    .saturating_mul(self.bench_iterations)
            && self.accuracy_milli == 1000
            && self.wrong_wins == 0
            && self.p99_latency_ns <= self.p99_latency_gate_ns
            && self.action_ablation_accuracy_milli < self.accuracy_milli
            && self.action_ablation_wrong_wins > 0
            && !self.compiler_used
            && self.optimized_build
            && self.eval_task_package_used
            && !self.corpus_jsonl_used_in_bench_loop
            && !self.forbidden_used()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionProductProofReport {
    schema_version: String,
    package_kind: String,
    verdict: String,
    product_proof_kind: String,
    package_path: String,
    manifest_path: String,
    eval_task_package_path: String,
    score_report_path: String,
    bench_report_path: String,
    #[serde(default)]
    source_contract_fingerprint64: u64,
    #[serde(default)]
    source_contract_bytes: usize,
    #[serde(default)]
    source_rebuild_matches_package: bool,
    #[serde(default)]
    source_rebuild_package_fingerprint64: u64,
    #[serde(default)]
    source_rebuild_package_bytes: usize,
    #[serde(default)]
    source_rebuild_flat_records: usize,
    #[serde(default)]
    source_rebuild_operator_keys_match: bool,
    #[serde(default)]
    source_rebuild_contract_verdict: String,
    #[serde(default)]
    source_rebuild_contract_gate_pass: bool,
    #[serde(default)]
    source_rebuild_accepted_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_rejected_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_operator_label_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_slot_map_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_target_leak_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_lookup_authority_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_local_out_t_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_arrow_demo_rows: usize,
    #[serde(default)]
    source_rebuild_concrete_output_token_leak_rows: usize,
    #[serde(default)]
    source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_train_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_heldout_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_min_train_rows_per_action_tree: usize,
    #[serde(default)]
    source_rebuild_min_heldout_rows_per_action_tree: usize,
    #[serde(default)]
    source_rebuild_skipped_train_rows: usize,
    cells: usize,
    flat_records: usize,
    manifest_operator_keys: usize,
    package_fingerprint64: u64,
    eval_pack_package_fingerprint64: u64,
    package_bytes: usize,
    eval_pack_bytes: usize,
    runtime_bytes_estimate: usize,
    score_report_verdict: String,
    bench_report_verdict: String,
    contract_verdict: String,
    manifest_verdict: String,
    rows: usize,
    heldout_eval_rows: usize,
    action_ablation_eval_rows: usize,
    score_accuracy_milli: usize,
    score_wrong_wins: usize,
    score_p99_latency_ns: u128,
    score_action_ablation_accuracy_milli: usize,
    score_action_ablation_wrong_wins: usize,
    bench_iterations: usize,
    bench_samples: usize,
    bench_accuracy_milli: usize,
    bench_wrong_wins: usize,
    bench_p99_latency_ns: u128,
    bench_p99_latency_gate_ns: u128,
    bench_action_ablation_accuracy_milli: usize,
    bench_action_ablation_wrong_wins: usize,
    compiler_used: bool,
    #[serde(default)]
    optimized_build: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used_in_score_loop: bool,
    corpus_jsonl_used_in_bench_loop: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    runtime_path: String,
    claim_boundary: String,
    license_boundary: String,
    product_boundary: String,
}

struct PhaseActionProductProofReportInput<'a> {
    verdict: &'static str,
    package_path: &'a Path,
    manifest_path: &'a Path,
    eval_task_package_path: &'a Path,
    score_report_path: &'a Path,
    bench_report_path: &'a Path,
    package_info: nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    manifest: &'a PhaseActionPackageManifest,
    eval_package: &'a PhaseEvalTaskPackage,
    score_report: &'a PhaseActionPackageScoreReport,
    bench_report: &'a PhaseActionPackageBenchReport,
    source_rebuild: &'a PhaseActionSourceRebuildAudit,
}

impl PhaseActionProductProofReport {
    fn from_inputs(input: PhaseActionProductProofReportInput<'_>) -> Self {
        Self {
            schema_version: "nando_phase_action_product_proof_report_v1".to_string(),
            package_kind: input.manifest.package_kind.clone(),
            verdict: input.verdict.to_string(),
            product_proof_kind: ACTION_PRODUCT_PROOF_KIND.to_string(),
            package_path: input.package_path.display().to_string(),
            manifest_path: input.manifest_path.display().to_string(),
            eval_task_package_path: input.eval_task_package_path.display().to_string(),
            score_report_path: input.score_report_path.display().to_string(),
            bench_report_path: input.bench_report_path.display().to_string(),
            source_contract_fingerprint64: input.manifest.source_contract_fingerprint64,
            source_contract_bytes: input.manifest.source_contract_bytes,
            source_rebuild_matches_package: input.source_rebuild.source_rebuild_matches_package,
            source_rebuild_package_fingerprint64: input
                .source_rebuild
                .source_rebuild_package_fingerprint64,
            source_rebuild_package_bytes: input.source_rebuild.source_rebuild_package_bytes,
            source_rebuild_flat_records: input.source_rebuild.source_rebuild_flat_records,
            source_rebuild_operator_keys_match: input
                .source_rebuild
                .source_rebuild_operator_keys_match,
            source_rebuild_contract_verdict: input
                .source_rebuild
                .source_rebuild_contract_verdict
                .clone(),
            source_rebuild_contract_gate_pass: input.source_rebuild.source_rebuild_contract_gate_pass,
            source_rebuild_accepted_action_tree_rows: input
                .source_rebuild
                .source_rebuild_accepted_action_tree_rows,
            source_rebuild_rejected_action_tree_rows: input
                .source_rebuild
                .source_rebuild_rejected_action_tree_rows,
            source_rebuild_forbidden_operator_label_rows: input
                .source_rebuild
                .source_rebuild_forbidden_operator_label_rows,
            source_rebuild_forbidden_slot_map_rows: input
                .source_rebuild
                .source_rebuild_forbidden_slot_map_rows,
            source_rebuild_forbidden_target_leak_rows: input
                .source_rebuild
                .source_rebuild_forbidden_target_leak_rows,
            source_rebuild_forbidden_lookup_authority_rows: input
                .source_rebuild
                .source_rebuild_forbidden_lookup_authority_rows,
            source_rebuild_forbidden_local_out_t_rows: input
                .source_rebuild
                .source_rebuild_forbidden_local_out_t_rows,
            source_rebuild_forbidden_arrow_demo_rows: input
                .source_rebuild
                .source_rebuild_forbidden_arrow_demo_rows,
            source_rebuild_concrete_output_token_leak_rows: input
                .source_rebuild
                .source_rebuild_concrete_output_token_leak_rows,
            source_rebuild_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_action_tree_key_count,
            source_rebuild_train_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_train_action_tree_key_count,
            source_rebuild_heldout_action_tree_key_count: input
                .source_rebuild
                .source_rebuild_heldout_action_tree_key_count,
            source_rebuild_min_train_rows_per_action_tree: input
                .source_rebuild
                .source_rebuild_min_train_rows_per_action_tree,
            source_rebuild_min_heldout_rows_per_action_tree: input
                .source_rebuild
                .source_rebuild_min_heldout_rows_per_action_tree,
            source_rebuild_skipped_train_rows: input
                .source_rebuild
                .source_rebuild_skipped_train_rows,
            cells: input.manifest.cells,
            flat_records: input.manifest.flat_records,
            manifest_operator_keys: input.manifest.operator_keys.len(),
            package_fingerprint64: input.package_info.fingerprint64,
            eval_pack_package_fingerprint64: input.eval_package.package_fingerprint64,
            package_bytes: input.package_bytes_len,
            eval_pack_bytes: input.eval_package.serialized_len(),
            runtime_bytes_estimate: input.manifest.runtime_bytes_estimate,
            score_report_verdict: input.score_report.verdict.clone(),
            bench_report_verdict: input.bench_report.verdict.clone(),
            contract_verdict: input.manifest.contract_verdict.clone(),
            manifest_verdict: input.manifest.verdict.clone(),
            rows: input.manifest.rows,
            heldout_eval_rows: input.eval_package.prepared.tasks.len(),
            action_ablation_eval_rows: input.eval_package.prepared.action_ablation_tasks.len(),
            score_accuracy_milli: input.score_report.accuracy_milli,
            score_wrong_wins: input.score_report.wrong_wins,
            score_p99_latency_ns: input.score_report.p99_latency_ns,
            score_action_ablation_accuracy_milli: input
                .score_report
                .action_ablation_accuracy_milli,
            score_action_ablation_wrong_wins: input.score_report.action_ablation_wrong_wins,
            bench_iterations: input.bench_report.bench_iterations,
            bench_samples: input.bench_report.bench_samples,
            bench_accuracy_milli: input.bench_report.accuracy_milli,
            bench_wrong_wins: input.bench_report.wrong_wins,
            bench_p99_latency_ns: input.bench_report.p99_latency_ns,
            bench_p99_latency_gate_ns: input.bench_report.p99_latency_gate_ns,
            bench_action_ablation_accuracy_milli: input
                .bench_report
                .action_ablation_accuracy_milli,
            bench_action_ablation_wrong_wins: input.bench_report.action_ablation_wrong_wins,
            compiler_used: input.score_report.compiler_used || input.bench_report.compiler_used,
            optimized_build: input.score_report.optimized_build && input.bench_report.optimized_build,
            eval_task_package_used: input.score_report.eval_task_package_used
                && input.bench_report.eval_task_package_used,
            corpus_jsonl_used_in_score_loop: input
                .score_report
                .corpus_jsonl_used_in_score_loop
                .unwrap_or(true),
            corpus_jsonl_used_in_bench_loop: input.bench_report.corpus_jsonl_used_in_bench_loop,
            python_demo_used: input.score_report.python_demo_used
                || input.bench_report.python_demo_used
                || input.manifest.python_demo_used,
            target_center_id_training_used: input.score_report.target_center_id_training_used
                || input.bench_report.target_center_id_training_used
                || input.manifest.target_center_id_training_used,
            proof_rule_id_training_authority_used: input
                .score_report
                .proof_rule_id_training_authority_used
                || input.bench_report.proof_rule_id_training_authority_used
                || input.manifest.proof_rule_id_training_authority_used,
            concrete_x_lookup_used: input.score_report.concrete_x_lookup_used
                || input.bench_report.concrete_x_lookup_used
                || input.manifest.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: input
                .score_report
                .local_out_t_runtime_extension_used
                || input.bench_report.local_out_t_runtime_extension_used
                || input.manifest.local_out_t_runtime_extension_used,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            claim_boundary: input.manifest.claim_boundary.clone(),
            license_boundary: input.manifest.license_boundary.clone(),
            product_boundary:
                "product-proof bundle for packaged flat scorer only; not a commercial license closure, strict ordered decoder, text generation, or autonomous raw action-router proof"
                    .to_string(),
        }
    }

    const fn forbidden_used(&self) -> bool {
        self.python_demo_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_PRODUCT_PROOF_V1_PASS"
            && self.product_proof_kind == ACTION_PRODUCT_PROOF_KIND
            && self.contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.manifest_verdict == "PHASE_ACTION_PACKAGE_V1_PASS"
            && self.score_report_verdict == "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS"
            && self.bench_report_verdict == "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS"
            && self.source_contract_fingerprint64 != 0
            && self.source_contract_bytes > 0
            && self.source_rebuild_matches_package
            && self.source_rebuild_package_fingerprint64 == self.package_fingerprint64
            && self.source_rebuild_package_bytes == self.package_bytes
            && self.source_rebuild_flat_records == self.flat_records
            && self.source_rebuild_operator_keys_match
            && self.source_rebuild_contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.source_rebuild_contract_gate_pass
            && self.source_rebuild_accepted_action_tree_rows > 0
            && self.source_rebuild_rejected_action_tree_rows == 0
            && self.source_rebuild_forbidden_operator_label_rows == 0
            && self.source_rebuild_forbidden_slot_map_rows == 0
            && self.source_rebuild_forbidden_target_leak_rows == 0
            && self.source_rebuild_forbidden_lookup_authority_rows == 0
            && self.source_rebuild_forbidden_local_out_t_rows == 0
            && self.source_rebuild_forbidden_arrow_demo_rows == 0
            && self.source_rebuild_concrete_output_token_leak_rows == 0
            && self.source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.source_rebuild_train_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_heldout_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_min_train_rows_per_action_tree > 0
            && self.source_rebuild_min_heldout_rows_per_action_tree > 0
            && self.source_rebuild_skipped_train_rows == 0
            && self.package_fingerprint64 != 0
            && self.package_fingerprint64 == self.eval_pack_package_fingerprint64
            && self.manifest_operator_keys == self.flat_records
            && self.heldout_eval_rows > 0
            && self.action_ablation_eval_rows > 0
            && self.score_accuracy_milli == 1000
            && self.score_wrong_wins == 0
            && self.score_action_ablation_accuracy_milli < self.score_accuracy_milli
            && self.score_action_ablation_wrong_wins > 0
            && self.bench_iterations > 0
            && self.bench_samples == self.heldout_eval_rows.saturating_mul(self.bench_iterations)
            && self.bench_accuracy_milli == 1000
            && self.bench_wrong_wins == 0
            && self.bench_p99_latency_ns <= self.bench_p99_latency_gate_ns
            && self.bench_action_ablation_accuracy_milli < self.bench_accuracy_milli
            && self.bench_action_ablation_wrong_wins > 0
            && !self.compiler_used
            && self.optimized_build
            && self.eval_task_package_used
            && !self.corpus_jsonl_used_in_score_loop
            && !self.corpus_jsonl_used_in_bench_loop
            && !self.forbidden_used()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionReleaseSuiteArtifactReport {
    label: String,
    package_kind: String,
    package_path: String,
    manifest_path: String,
    eval_task_package_path: String,
    score_report_path: String,
    bench_report_path: String,
    product_proof_path: String,
    #[serde(default)]
    source_verify_report_path: String,
    #[serde(default)]
    shortcut_report_path: String,
    #[serde(default)]
    operator_coverage_report_path: String,
    #[serde(default)]
    source_verify_report_fingerprint64: u64,
    #[serde(default)]
    source_verify_report_bytes: usize,
    #[serde(default)]
    source_verify_report_verdict: String,
    #[serde(default)]
    source_verify_report_matches_package: bool,
    #[serde(default)]
    source_verify_report_gate_pass: bool,
    #[serde(default)]
    shortcut_report_fingerprint64: u64,
    #[serde(default)]
    shortcut_report_bytes: usize,
    #[serde(default)]
    shortcut_report_verdict: String,
    #[serde(default)]
    shortcut_report_matches_corpus: bool,
    #[serde(default)]
    shortcut_report_gate_pass: bool,
    #[serde(default)]
    operator_coverage_report_fingerprint64: u64,
    #[serde(default)]
    operator_coverage_report_bytes: usize,
    #[serde(default)]
    operator_coverage_report_verdict: String,
    #[serde(default)]
    operator_coverage_report_matches_corpus: bool,
    #[serde(default)]
    operator_coverage_report_gate_pass: bool,
    #[serde(default)]
    operator_coverage_full_operator_dimension_coverage_pass: bool,
    #[serde(default)]
    operator_coverage_min_dimension_value_count: usize,
    #[serde(default)]
    operator_coverage_wide_dimension_count: usize,
    #[serde(default)]
    operator_coverage_select_value_count: usize,
    #[serde(default)]
    operator_coverage_transform_value_count: usize,
    #[serde(default)]
    operator_coverage_write_value_count: usize,
    #[serde(default)]
    operator_coverage_condition_value_count: usize,
    #[serde(default)]
    operator_coverage_check_value_count: usize,
    #[serde(default)]
    source_contract_fingerprint64: u64,
    #[serde(default)]
    source_contract_bytes: usize,
    #[serde(default)]
    source_rebuild_matches_package: bool,
    #[serde(default)]
    source_rebuild_package_fingerprint64: u64,
    #[serde(default)]
    source_rebuild_package_bytes: usize,
    #[serde(default)]
    source_rebuild_flat_records: usize,
    #[serde(default)]
    source_rebuild_operator_keys_match: bool,
    #[serde(default)]
    source_rebuild_contract_gate_pass: bool,
    #[serde(default)]
    source_rebuild_accepted_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_rejected_action_tree_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_operator_label_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_slot_map_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_target_leak_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_lookup_authority_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_local_out_t_rows: usize,
    #[serde(default)]
    source_rebuild_forbidden_arrow_demo_rows: usize,
    #[serde(default)]
    source_rebuild_concrete_output_token_leak_rows: usize,
    #[serde(default)]
    source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_train_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_heldout_action_tree_key_count: usize,
    #[serde(default)]
    source_rebuild_min_train_rows_per_action_tree: usize,
    #[serde(default)]
    source_rebuild_min_heldout_rows_per_action_tree: usize,
    #[serde(default)]
    source_rebuild_skipped_train_rows: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    eval_pack_bytes: usize,
    runtime_bytes_estimate: usize,
    rows: usize,
    heldout_eval_rows: usize,
    action_ablation_eval_rows: usize,
    score_accuracy_milli: usize,
    score_wrong_wins: usize,
    score_p99_latency_ns: u128,
    score_action_ablation_accuracy_milli: usize,
    score_action_ablation_wrong_wins: usize,
    bench_iterations: usize,
    bench_samples: usize,
    bench_accuracy_milli: usize,
    bench_wrong_wins: usize,
    bench_p99_latency_ns: u128,
    bench_p99_latency_gate_ns: u128,
    bench_action_ablation_accuracy_milli: usize,
    bench_action_ablation_wrong_wins: usize,
    score_report_verdict: String,
    bench_report_verdict: String,
    product_report_verdict: String,
    manifest_gate_pass: bool,
    manifest_matches_package: bool,
    eval_pack_matches_package: bool,
    score_report_matches_package: bool,
    bench_report_matches_package: bool,
    product_report_matches_package: bool,
    score_report_gate_pass: bool,
    bench_report_gate_pass: bool,
    product_report_gate_pass: bool,
    product_verify_pass: bool,
    compiler_used: bool,
    #[serde(default)]
    optimized_build: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used_in_score_loop: bool,
    corpus_jsonl_used_in_bench_loop: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    product_boundary: String,
    license_boundary: String,
}

impl PhaseActionReleaseSuiteArtifactReport {
    const fn forbidden_used(&self) -> bool {
        self.python_demo_used
            || self.target_center_id_training_used
            || self.proof_rule_id_training_authority_used
            || self.concrete_x_lookup_used
            || self.local_out_t_runtime_extension_used
    }

    fn gate_pass(&self) -> bool {
        self.product_verify_pass
            && self.manifest_gate_pass
            && self.manifest_matches_package
            && self.eval_pack_matches_package
            && self.score_report_matches_package
            && self.bench_report_matches_package
            && self.product_report_matches_package
            && self.source_verify_report_matches_package
            && self.shortcut_report_matches_corpus
            && self.operator_coverage_report_matches_corpus
            && self.score_report_gate_pass
            && self.bench_report_gate_pass
            && self.product_report_gate_pass
            && self.source_verify_report_gate_pass
            && self.shortcut_report_gate_pass
            && self.score_report_verdict == "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS"
            && self.bench_report_verdict == "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS"
            && self.product_report_verdict == "PHASE_ACTION_PRODUCT_PROOF_V1_PASS"
            && self.source_verify_report_verdict == "PHASE_ACTION_SOURCE_VERIFY_V1_PASS"
            && self.shortcut_report_verdict == "PHASE_ACTION_SHORTCUT_V1_PASS"
            && self.source_verify_report_fingerprint64 != 0
            && self.source_verify_report_bytes > 0
            && self.shortcut_report_fingerprint64 != 0
            && self.shortcut_report_bytes > 0
            && self.operator_coverage_report_fingerprint64 != 0
            && self.operator_coverage_report_bytes > 0
            && self.source_contract_fingerprint64 != 0
            && self.source_contract_bytes > 0
            && self.source_rebuild_matches_package
            && self.source_rebuild_package_fingerprint64 == self.package_fingerprint64
            && self.source_rebuild_package_bytes == self.package_bytes
            && self.source_rebuild_flat_records > 0
            && self.source_rebuild_operator_keys_match
            && self.source_rebuild_contract_gate_pass
            && self.source_rebuild_accepted_action_tree_rows > 0
            && self.source_rebuild_rejected_action_tree_rows == 0
            && self.source_rebuild_forbidden_operator_label_rows == 0
            && self.source_rebuild_forbidden_slot_map_rows == 0
            && self.source_rebuild_forbidden_target_leak_rows == 0
            && self.source_rebuild_forbidden_lookup_authority_rows == 0
            && self.source_rebuild_forbidden_local_out_t_rows == 0
            && self.source_rebuild_forbidden_arrow_demo_rows == 0
            && self.source_rebuild_concrete_output_token_leak_rows == 0
            && self.source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.source_rebuild_train_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_heldout_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_min_train_rows_per_action_tree > 0
            && self.source_rebuild_min_heldout_rows_per_action_tree > 0
            && self.source_rebuild_skipped_train_rows == 0
            && self.package_fingerprint64 != 0
            && self.heldout_eval_rows > 0
            && self.action_ablation_eval_rows > 0
            && self.score_accuracy_milli == 1000
            && self.score_wrong_wins == 0
            && self.score_action_ablation_accuracy_milli < self.score_accuracy_milli
            && self.score_action_ablation_wrong_wins > 0
            && self.bench_iterations > 0
            && self.bench_samples == self.heldout_eval_rows.saturating_mul(self.bench_iterations)
            && self.bench_accuracy_milli == 1000
            && self.bench_wrong_wins == 0
            && self.bench_p99_latency_ns <= self.bench_p99_latency_gate_ns
            && self.bench_action_ablation_accuracy_milli < self.bench_accuracy_milli
            && self.bench_action_ablation_wrong_wins > 0
            && !self.compiler_used
            && self.optimized_build
            && self.eval_task_package_used
            && !self.corpus_jsonl_used_in_score_loop
            && !self.corpus_jsonl_used_in_bench_loop
            && !self.forbidden_used()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionReleaseSuiteReport {
    schema_version: String,
    verdict: String,
    release_suite_kind: String,
    artifact_count: usize,
    artifacts: Vec<PhaseActionReleaseSuiteArtifactReport>,
    distinct_package_fingerprints: bool,
    total_package_bytes: usize,
    total_eval_pack_bytes: usize,
    #[serde(default)]
    total_source_verify_report_bytes: usize,
    #[serde(default)]
    total_shortcut_report_bytes: usize,
    #[serde(default)]
    total_operator_coverage_report_bytes: usize,
    total_runtime_bytes_estimate: usize,
    total_bench_samples: usize,
    max_score_p99_latency_ns: u128,
    max_bench_p99_latency_ns: u128,
    all_score_accuracy_milli_1000: bool,
    all_bench_accuracy_milli_1000: bool,
    #[serde(default)]
    all_source_verify_reports_pass: bool,
    #[serde(default)]
    all_shortcut_reports_pass: bool,
    #[serde(default)]
    all_operator_coverage_reports_match_sources: bool,
    #[serde(default)]
    operator_dimension_coverage_artifact_count: usize,
    #[serde(default)]
    release_operator_dimension_coverage_pass: bool,
    #[serde(default)]
    max_operator_coverage_min_dimension_value_count: usize,
    #[serde(default)]
    max_operator_coverage_wide_dimension_count: usize,
    #[serde(default)]
    all_action_ablation_collapses: bool,
    #[serde(default)]
    all_action_contract_source_rebuild_clean: bool,
    #[serde(default)]
    all_optimized_build_reports_pass: bool,
    #[serde(default)]
    total_source_rebuild_accepted_action_tree_rows: usize,
    #[serde(default)]
    total_source_rebuild_rejected_action_tree_rows: usize,
    #[serde(default)]
    total_source_rebuild_forbidden_contract_rows: usize,
    #[serde(default)]
    total_source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    min_source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    all_action_tree_key_coverage_pass: bool,
    #[serde(default)]
    all_manifest_package_parity_pass: bool,
    #[serde(default)]
    all_eval_pack_package_parity_pass: bool,
    #[serde(default)]
    all_score_report_package_parity_pass: bool,
    #[serde(default)]
    all_bench_report_package_parity_pass: bool,
    #[serde(default)]
    all_product_report_package_parity_pass: bool,
    #[serde(default)]
    all_source_rebuild_package_parity_pass: bool,
    #[serde(default)]
    all_source_verify_report_package_parity_pass: bool,
    #[serde(default)]
    all_package_report_parity_pass: bool,
    #[serde(default)]
    max_score_action_ablation_accuracy_milli: usize,
    #[serde(default)]
    max_bench_action_ablation_accuracy_milli: usize,
    #[serde(default)]
    total_score_action_ablation_wrong_wins: usize,
    #[serde(default)]
    total_bench_action_ablation_wrong_wins: usize,
    total_score_wrong_wins: usize,
    total_bench_wrong_wins: usize,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    forbidden_used: bool,
    commercial_license_closed: bool,
    runtime_path: String,
    suite_boundary: String,
    product_boundary: String,
    license_boundary: String,
}

impl PhaseActionReleaseSuiteReport {
    fn from_artifacts(artifacts: Vec<PhaseActionReleaseSuiteArtifactReport>) -> Self {
        let artifact_count = artifacts.len();
        let mut fingerprints = BTreeSet::new();
        let distinct_package_fingerprints = artifacts
            .iter()
            .all(|artifact| fingerprints.insert(artifact.package_fingerprint64));
        let total_package_bytes = artifacts
            .iter()
            .map(|artifact| artifact.package_bytes)
            .sum();
        let total_eval_pack_bytes = artifacts
            .iter()
            .map(|artifact| artifact.eval_pack_bytes)
            .sum();
        let total_source_verify_report_bytes = artifacts
            .iter()
            .map(|artifact| artifact.source_verify_report_bytes)
            .sum();
        let total_shortcut_report_bytes = artifacts
            .iter()
            .map(|artifact| artifact.shortcut_report_bytes)
            .sum();
        let total_operator_coverage_report_bytes = artifacts
            .iter()
            .map(|artifact| artifact.operator_coverage_report_bytes)
            .sum();
        let total_runtime_bytes_estimate = artifacts
            .iter()
            .map(|artifact| artifact.runtime_bytes_estimate)
            .sum();
        let total_bench_samples = artifacts
            .iter()
            .map(|artifact| artifact.bench_samples)
            .sum();
        let max_score_p99_latency_ns = artifacts
            .iter()
            .map(|artifact| artifact.score_p99_latency_ns)
            .max()
            .unwrap_or(0);
        let max_bench_p99_latency_ns = artifacts
            .iter()
            .map(|artifact| artifact.bench_p99_latency_ns)
            .max()
            .unwrap_or(0);
        let all_score_accuracy_milli_1000 = artifacts
            .iter()
            .all(|artifact| artifact.score_accuracy_milli == 1000);
        let all_bench_accuracy_milli_1000 = artifacts
            .iter()
            .all(|artifact| artifact.bench_accuracy_milli == 1000);
        let all_source_verify_reports_pass = artifacts.iter().all(|artifact| {
            artifact.source_verify_report_gate_pass
                && artifact.source_verify_report_matches_package
                && artifact.source_verify_report_verdict == "PHASE_ACTION_SOURCE_VERIFY_V1_PASS"
        });
        let all_shortcut_reports_pass = artifacts.iter().all(|artifact| {
            artifact.shortcut_report_gate_pass
                && artifact.shortcut_report_matches_corpus
                && artifact.shortcut_report_verdict == "PHASE_ACTION_SHORTCUT_V1_PASS"
        });
        let all_operator_coverage_reports_match_sources = artifacts
            .iter()
            .all(|artifact| artifact.operator_coverage_report_matches_corpus);
        let operator_dimension_coverage_artifact_count = artifacts
            .iter()
            .filter(|artifact| artifact.operator_coverage_full_operator_dimension_coverage_pass)
            .count();
        let release_operator_dimension_coverage_pass =
            operator_dimension_coverage_artifact_count > 0;
        let max_operator_coverage_min_dimension_value_count = artifacts
            .iter()
            .map(|artifact| artifact.operator_coverage_min_dimension_value_count)
            .max()
            .unwrap_or(0);
        let max_operator_coverage_wide_dimension_count = artifacts
            .iter()
            .map(|artifact| artifact.operator_coverage_wide_dimension_count)
            .max()
            .unwrap_or(0);
        let all_action_ablation_collapses = artifacts.iter().all(|artifact| {
            artifact.score_action_ablation_accuracy_milli < artifact.score_accuracy_milli
                && artifact.score_action_ablation_wrong_wins > 0
                && artifact.bench_action_ablation_accuracy_milli < artifact.bench_accuracy_milli
                && artifact.bench_action_ablation_wrong_wins > 0
        });
        let total_source_rebuild_accepted_action_tree_rows = artifacts
            .iter()
            .map(|artifact| artifact.source_rebuild_accepted_action_tree_rows)
            .sum();
        let total_source_rebuild_rejected_action_tree_rows = artifacts
            .iter()
            .map(|artifact| artifact.source_rebuild_rejected_action_tree_rows)
            .sum();
        let total_source_rebuild_forbidden_contract_rows = artifacts
            .iter()
            .map(|artifact| {
                artifact.source_rebuild_forbidden_operator_label_rows
                    + artifact.source_rebuild_forbidden_slot_map_rows
                    + artifact.source_rebuild_forbidden_target_leak_rows
                    + artifact.source_rebuild_forbidden_lookup_authority_rows
                    + artifact.source_rebuild_forbidden_local_out_t_rows
                    + artifact.source_rebuild_forbidden_arrow_demo_rows
                    + artifact.source_rebuild_concrete_output_token_leak_rows
            })
            .sum();
        let all_action_contract_source_rebuild_clean = artifacts.iter().all(|artifact| {
            artifact.source_rebuild_contract_gate_pass
                && artifact.source_rebuild_accepted_action_tree_rows > 0
                && artifact.source_rebuild_rejected_action_tree_rows == 0
                && artifact.source_rebuild_forbidden_operator_label_rows == 0
                && artifact.source_rebuild_forbidden_slot_map_rows == 0
                && artifact.source_rebuild_forbidden_target_leak_rows == 0
                && artifact.source_rebuild_forbidden_lookup_authority_rows == 0
                && artifact.source_rebuild_forbidden_local_out_t_rows == 0
                && artifact.source_rebuild_forbidden_arrow_demo_rows == 0
                && artifact.source_rebuild_concrete_output_token_leak_rows == 0
        });
        let total_source_rebuild_action_tree_key_count = artifacts
            .iter()
            .map(|artifact| artifact.source_rebuild_action_tree_key_count)
            .sum();
        let min_source_rebuild_action_tree_key_count = artifacts
            .iter()
            .map(|artifact| artifact.source_rebuild_action_tree_key_count)
            .min()
            .unwrap_or(0);
        let all_action_tree_key_coverage_pass = artifacts.iter().all(|artifact| {
            artifact.source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
                && artifact.source_rebuild_train_action_tree_key_count
                    == artifact.source_rebuild_action_tree_key_count
                && artifact.source_rebuild_heldout_action_tree_key_count
                    == artifact.source_rebuild_action_tree_key_count
                && artifact.source_rebuild_min_train_rows_per_action_tree > 0
                && artifact.source_rebuild_min_heldout_rows_per_action_tree > 0
        });
        let all_optimized_build_reports_pass =
            artifacts.iter().all(|artifact| artifact.optimized_build);
        let all_manifest_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.manifest_matches_package);
        let all_eval_pack_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.eval_pack_matches_package);
        let all_score_report_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.score_report_matches_package);
        let all_bench_report_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.bench_report_matches_package);
        let all_product_report_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.product_report_matches_package);
        let all_source_rebuild_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.source_rebuild_matches_package);
        let all_source_verify_report_package_parity_pass = artifacts
            .iter()
            .all(|artifact| artifact.source_verify_report_matches_package);
        let all_package_report_parity_pass = all_manifest_package_parity_pass
            && all_eval_pack_package_parity_pass
            && all_score_report_package_parity_pass
            && all_bench_report_package_parity_pass
            && all_product_report_package_parity_pass
            && all_source_rebuild_package_parity_pass
            && all_source_verify_report_package_parity_pass;
        let max_score_action_ablation_accuracy_milli = artifacts
            .iter()
            .map(|artifact| artifact.score_action_ablation_accuracy_milli)
            .max()
            .unwrap_or(0);
        let max_bench_action_ablation_accuracy_milli = artifacts
            .iter()
            .map(|artifact| artifact.bench_action_ablation_accuracy_milli)
            .max()
            .unwrap_or(0);
        let total_score_action_ablation_wrong_wins = artifacts
            .iter()
            .map(|artifact| artifact.score_action_ablation_wrong_wins)
            .sum();
        let total_bench_action_ablation_wrong_wins = artifacts
            .iter()
            .map(|artifact| artifact.bench_action_ablation_wrong_wins)
            .sum();
        let total_score_wrong_wins = artifacts
            .iter()
            .map(|artifact| artifact.score_wrong_wins)
            .sum();
        let total_bench_wrong_wins = artifacts
            .iter()
            .map(|artifact| artifact.bench_wrong_wins)
            .sum();
        let compiler_used = artifacts.iter().any(|artifact| artifact.compiler_used);
        let eval_task_package_used = artifacts
            .iter()
            .all(|artifact| artifact.eval_task_package_used);
        let corpus_jsonl_used = artifacts.iter().any(|artifact| {
            artifact.corpus_jsonl_used_in_score_loop || artifact.corpus_jsonl_used_in_bench_loop
        });
        let forbidden_used = artifacts.iter().any(|artifact| artifact.forbidden_used());
        let license_boundary = artifacts
            .first()
            .map(|artifact| artifact.license_boundary.clone())
            .unwrap_or_else(|| {
                "non-commercial proof package only; commercial license closure is not claimed"
                    .to_string()
            });
        let mut report = Self {
            schema_version: "nando_phase_action_release_suite_report_v1".to_string(),
            verdict: "PHASE_ACTION_RELEASE_SUITE_V1_WATCH".to_string(),
            release_suite_kind: ACTION_RELEASE_SUITE_KIND.to_string(),
            artifact_count,
            artifacts,
            distinct_package_fingerprints,
            total_package_bytes,
            total_eval_pack_bytes,
            total_source_verify_report_bytes,
            total_shortcut_report_bytes,
            total_operator_coverage_report_bytes,
            total_runtime_bytes_estimate,
            total_bench_samples,
            max_score_p99_latency_ns,
            max_bench_p99_latency_ns,
            all_score_accuracy_milli_1000,
            all_bench_accuracy_milli_1000,
            all_source_verify_reports_pass,
            all_shortcut_reports_pass,
            all_operator_coverage_reports_match_sources,
            operator_dimension_coverage_artifact_count,
            release_operator_dimension_coverage_pass,
            max_operator_coverage_min_dimension_value_count,
            max_operator_coverage_wide_dimension_count,
            all_action_ablation_collapses,
            all_action_contract_source_rebuild_clean,
            all_optimized_build_reports_pass,
            total_source_rebuild_accepted_action_tree_rows,
            total_source_rebuild_rejected_action_tree_rows,
            total_source_rebuild_forbidden_contract_rows,
            total_source_rebuild_action_tree_key_count,
            min_source_rebuild_action_tree_key_count,
            all_action_tree_key_coverage_pass,
            all_manifest_package_parity_pass,
            all_eval_pack_package_parity_pass,
            all_score_report_package_parity_pass,
            all_bench_report_package_parity_pass,
            all_product_report_package_parity_pass,
            all_source_rebuild_package_parity_pass,
            all_source_verify_report_package_parity_pass,
            all_package_report_parity_pass,
            max_score_action_ablation_accuracy_milli,
            max_bench_action_ablation_accuracy_milli,
            total_score_action_ablation_wrong_wins,
            total_bench_action_ablation_wrong_wins,
            total_score_wrong_wins,
            total_bench_wrong_wins,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            forbidden_used,
            commercial_license_closed: false,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            suite_boundary:
                "release-suite over packaged flat action scorer product proofs; no Python demos, no JSONL score/bench loop, no compiler in scoring"
                    .to_string(),
            product_boundary:
                "packaged flat scorer release candidate only; not strict ordered decoder, text generation, autonomous raw action-router proof, or broad workflow reasoning"
                    .to_string(),
            license_boundary,
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_RELEASE_SUITE_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_release_suite_report_v1"
            && self.release_suite_kind == ACTION_RELEASE_SUITE_KIND
            && self.artifact_count == self.artifacts.len()
            && self.artifact_count >= 2
            && self.distinct_package_fingerprints
            && self.artifacts.iter().all(|artifact| artifact.gate_pass())
            && self.total_source_verify_report_bytes > 0
            && self.total_shortcut_report_bytes > 0
            && self.all_score_accuracy_milli_1000
            && self.all_bench_accuracy_milli_1000
            && self.all_source_verify_reports_pass
            && self.all_shortcut_reports_pass
            && self.all_operator_coverage_reports_match_sources
            && self.release_operator_dimension_coverage_pass
            && self.max_operator_coverage_min_dimension_value_count >= 2
            && self.max_operator_coverage_wide_dimension_count == 5
            && self.all_action_ablation_collapses
            && self.all_action_contract_source_rebuild_clean
            && self.all_action_tree_key_coverage_pass
            && self.all_optimized_build_reports_pass
            && self.total_source_rebuild_accepted_action_tree_rows > 0
            && self.total_source_rebuild_rejected_action_tree_rows == 0
            && self.total_source_rebuild_forbidden_contract_rows == 0
            && self.total_source_rebuild_action_tree_key_count
                >= self
                    .artifact_count
                    .saturating_mul(MIN_ACTION_CONTRACT_KEY_COVERAGE)
            && self.min_source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.all_manifest_package_parity_pass
            && self.all_eval_pack_package_parity_pass
            && self.all_score_report_package_parity_pass
            && self.all_bench_report_package_parity_pass
            && self.all_product_report_package_parity_pass
            && self.all_source_rebuild_package_parity_pass
            && self.all_source_verify_report_package_parity_pass
            && self.all_package_report_parity_pass
            && self.total_score_action_ablation_wrong_wins > 0
            && self.total_bench_action_ablation_wrong_wins > 0
            && self.total_score_wrong_wins == 0
            && self.total_bench_wrong_wins == 0
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.forbidden_used
            && !self.commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug)]
struct CargoLicenseAudit {
    workspace_license_file_declared: bool,
    workspace_mit_license_declared: bool,
    crate_license_file_workspace_declared: bool,
    crate_license_workspace_declared: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionLicensePackageReport {
    schema_version: String,
    verdict: String,
    license_package_kind: String,
    license_name: String,
    license_file_path: String,
    license_file_fingerprint64: u64,
    license_file_bytes: usize,
    license_file_contains_noncommercial_grant: bool,
    license_file_contains_commercial_restriction: bool,
    license_file_contains_no_warranty: bool,
    cargo_workspace_license_file_declared: bool,
    cargo_workspace_mit_license_declared: bool,
    cargo_crate_license_file_workspace_declared: bool,
    cargo_crate_license_workspace_declared: bool,
    release_suite_report_path: String,
    release_suite_verdict: String,
    release_suite_kind: String,
    release_suite_gate_pass: bool,
    release_suite_matches_sources: bool,
    release_suite_artifact_count: usize,
    release_suite_total_runtime_bytes_estimate: usize,
    release_suite_total_bench_samples: usize,
    release_suite_max_bench_p99_latency_ns: u128,
    release_suite_compiler_used: bool,
    release_suite_corpus_jsonl_used: bool,
    release_suite_forbidden_used: bool,
    release_suite_license_boundary: String,
    release_suite_license_boundary_mentions_mit: bool,
    commercial_use_allowed: bool,
    noncommercial_use_allowed: bool,
    commercial_license_closed: bool,
    non_commercial_license_closed: bool,
    runtime_path: String,
    package_boundary: String,
    license_boundary: String,
}

impl PhaseActionLicensePackageReport {
    fn from_inputs(
        release_suite_report_path: &Path,
        release_suite: &PhaseActionReleaseSuiteReport,
        release_suite_matches_sources: bool,
        license_file_path: &Path,
        license_text: &str,
        license_fingerprint64: u64,
        cargo_audit: CargoLicenseAudit,
    ) -> Self {
        let license_lower = license_text.to_ascii_lowercase();
        let mut report = Self {
            schema_version: "nando_phase_action_license_package_report_v1".to_string(),
            verdict: "PHASE_ACTION_LICENSE_PACKAGE_V1_WATCH".to_string(),
            license_package_kind: ACTION_LICENSE_PACKAGE_KIND.to_string(),
            license_name: NONCOMMERCIAL_LICENSE_NAME.to_string(),
            license_file_path: license_file_path.display().to_string(),
            license_file_fingerprint64: license_fingerprint64,
            license_file_bytes: license_text.len(),
            license_file_contains_noncommercial_grant: license_lower.contains("non-commercial")
                && license_lower.contains("permission"),
            license_file_contains_commercial_restriction: license_lower.contains("commercial use")
                && license_lower.contains("separate written"),
            license_file_contains_no_warranty: license_lower.contains("no warranty")
                || license_lower.contains("without warranty"),
            cargo_workspace_license_file_declared: cargo_audit.workspace_license_file_declared,
            cargo_workspace_mit_license_declared: cargo_audit.workspace_mit_license_declared,
            cargo_crate_license_file_workspace_declared: cargo_audit
                .crate_license_file_workspace_declared,
            cargo_crate_license_workspace_declared: cargo_audit.crate_license_workspace_declared,
            release_suite_report_path: release_suite_report_path.display().to_string(),
            release_suite_verdict: release_suite.verdict.clone(),
            release_suite_kind: release_suite.release_suite_kind.clone(),
            release_suite_gate_pass: release_suite.gate_pass(),
            release_suite_matches_sources,
            release_suite_artifact_count: release_suite.artifact_count,
            release_suite_total_runtime_bytes_estimate: release_suite
                .total_runtime_bytes_estimate,
            release_suite_total_bench_samples: release_suite.total_bench_samples,
            release_suite_max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            release_suite_compiler_used: release_suite.compiler_used,
            release_suite_corpus_jsonl_used: release_suite.corpus_jsonl_used,
            release_suite_forbidden_used: release_suite.forbidden_used,
            release_suite_license_boundary: release_suite.license_boundary.clone(),
            release_suite_license_boundary_mentions_mit: release_suite.license_boundary.contains("MIT"),
            commercial_use_allowed: false,
            noncommercial_use_allowed: true,
            commercial_license_closed: false,
            non_commercial_license_closed: true,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            package_boundary:
                "non-commercial proof package over the verified action release-suite; not a commercial license or broader reasoning claim"
                    .to_string(),
            license_boundary:
                "non-commercial source/proof use only; commercial use requires a separate written agreement"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_license_package_report_v1"
            && self.license_package_kind == ACTION_LICENSE_PACKAGE_KIND
            && self.license_name == NONCOMMERCIAL_LICENSE_NAME
            && self.license_file_fingerprint64 != 0
            && self.license_file_bytes > 512
            && self.license_file_contains_noncommercial_grant
            && self.license_file_contains_commercial_restriction
            && self.license_file_contains_no_warranty
            && self.cargo_workspace_license_file_declared
            && !self.cargo_workspace_mit_license_declared
            && self.cargo_crate_license_file_workspace_declared
            && !self.cargo_crate_license_workspace_declared
            && self.release_suite_verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS"
            && self.release_suite_kind == ACTION_RELEASE_SUITE_KIND
            && self.release_suite_gate_pass
            && self.release_suite_matches_sources
            && self.release_suite_artifact_count >= 2
            && self.release_suite_total_runtime_bytes_estimate > 0
            && self.release_suite_total_bench_samples > 0
            && !self.release_suite_compiler_used
            && !self.release_suite_corpus_jsonl_used
            && !self.release_suite_forbidden_used
            && !self.release_suite_license_boundary_mentions_mit
            && !self.commercial_use_allowed
            && self.noncommercial_use_allowed
            && !self.commercial_license_closed
            && self.non_commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Copy, Debug)]
struct ActionOffloadSample {
    artifact_index: usize,
    row_index: usize,
    decision: PhaseCenterOffloadDecision,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionOffloadArtifactReport {
    label: String,
    package_path: String,
    eval_task_package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    #[serde(default)]
    sdk_inspected_cells: usize,
    #[serde(default)]
    sdk_inspected_record_count: usize,
    #[serde(default)]
    sdk_inspected_serialized_len: usize,
    #[serde(default)]
    sdk_inspected_payload_bytes: usize,
    #[serde(default)]
    sdk_inspected_fingerprint64: u64,
    #[serde(default)]
    sdk_inspect_matches_package: bool,
    #[serde(default)]
    sdk_inspect_matches_eval_pack: bool,
    eval_pack_bytes: usize,
    runtime_bytes_estimate: usize,
    unique_eval_rows: usize,
    unique_local_operator_rows: usize,
    unique_fallback_rows: usize,
    unique_offload_rate_milli: usize,
    unique_local_accuracy_milli: usize,
    unique_false_local_accepts: usize,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    simulated_calls: usize,
    simulated_local_operator_calls: usize,
    simulated_fallback_to_llm_calls: usize,
    simulated_offload_rate_milli: usize,
    simulated_local_accuracy_milli: usize,
    simulated_false_local_accepts: usize,
    release_artifact_gate_pass: bool,
    product_verify_pass: bool,
    score_accuracy_milli: usize,
    score_wrong_wins: usize,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    forbidden_used: bool,
}

impl PhaseActionOffloadArtifactReport {
    fn gate_pass(&self) -> bool {
        self.release_artifact_gate_pass
            && self.product_verify_pass
            && self.package_fingerprint64 != 0
            && self.package_bytes > 0
            && self.sdk_inspected_cells > 0
            && self.sdk_inspected_record_count > 0
            && self.sdk_inspected_serialized_len == self.package_bytes
            && self.sdk_inspected_payload_bytes > 0
            && self.sdk_inspected_fingerprint64 == self.package_fingerprint64
            && self.sdk_inspect_matches_package
            && self.sdk_inspect_matches_eval_pack
            && self.eval_pack_bytes > 0
            && self.runtime_bytes_estimate > 0
            && self.unique_eval_rows > 0
            && self.unique_local_operator_rows > 0
            && self.unique_local_accuracy_milli == 1000
            && self.unique_false_local_accepts == 0
            && self.simulated_calls > 0
            && self.simulated_local_operator_calls > 0
            && self.simulated_local_accuracy_milli == 1000
            && self.simulated_false_local_accepts == 0
            && self.score_accuracy_milli == 1000
            && self.score_wrong_wins == 0
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.forbidden_used
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionOffloadAuditReport {
    schema_version: String,
    verdict: String,
    offload_audit_kind: String,
    release_suite_report_path: String,
    license_file_path: String,
    license_package_report_path: String,
    margin_threshold_micro: i64,
    simulated_calls: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    offload_rate_milli: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    total_unique_eval_rows: usize,
    unique_local_operator_rows: usize,
    unique_fallback_rows: usize,
    unique_offload_rate_milli: usize,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    artifact_count: usize,
    artifacts: Vec<PhaseActionOffloadArtifactReport>,
    release_suite_verdict: String,
    release_suite_kind: String,
    release_suite_gate_pass: bool,
    release_suite_matches_sources: bool,
    total_runtime_bytes_estimate: usize,
    total_bench_samples: usize,
    max_bench_p99_latency_ns: u128,
    license_package_verdict: String,
    license_package_kind: String,
    license_package_gate_pass: bool,
    license_report_matches_sources: bool,
    commercial_use_allowed: bool,
    noncommercial_use_allowed: bool,
    commercial_license_closed: bool,
    non_commercial_license_closed: bool,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    forbidden_used: bool,
    runtime_path: String,
    #[serde(default)]
    offload_sdk_api: String,
    #[serde(default)]
    offload_sdk_inspect_api: String,
    offload_policy_api: String,
    offload_batch_api: String,
    offload_summary_api: String,
    offload_buffer_api: String,
    offload_summary_buffer_api: String,
    offload_runtime_summary_api: String,
    fallback_policy: String,
    audit_boundary: String,
    license_boundary: String,
}

impl PhaseActionOffloadAuditReport {
    #[allow(clippy::too_many_arguments)]
    fn from_inputs(
        release_suite_report_path: &Path,
        release_suite: &PhaseActionReleaseSuiteReport,
        release_suite_matches_sources: bool,
        license_file_path: &Path,
        license_package_report_path: &Path,
        license_report: &PhaseActionLicensePackageReport,
        license_report_matches_sources: bool,
        margin_threshold_micro: i64,
        simulated_calls: usize,
        mut artifacts: Vec<PhaseActionOffloadArtifactReport>,
        samples: &[ActionOffloadSample],
        mut margins: Vec<i64>,
    ) -> Self {
        let mut simulation_margin_scratch = Vec::with_capacity(simulated_calls);
        let simulation_summary = PhaseCenterOffloadSummary::from_repeated_decision_fn_into(
            samples.len(),
            simulated_calls,
            |index| samples[index].decision,
            &mut simulation_margin_scratch,
        );

        if !samples.is_empty() {
            for call_index in 0..simulated_calls {
                let sample = samples[call_index % samples.len()];
                let Some(artifact) = artifacts.get_mut(sample.artifact_index) else {
                    continue;
                };
                artifact.simulated_calls += 1;
                if sample.decision.action == PhaseCenterOffloadAction::LocalOperator {
                    artifact.simulated_local_operator_calls += 1;
                    if sample.decision.is_false_local_accept() {
                        artifact.simulated_false_local_accepts += 1;
                    }
                } else {
                    artifact.simulated_fallback_to_llm_calls += 1;
                }
            }
        }

        for artifact in &mut artifacts {
            artifact.simulated_offload_rate_milli = milli_ratio(
                artifact.simulated_local_operator_calls,
                artifact.simulated_calls,
            );
            let simulated_local_correct = artifact
                .simulated_local_operator_calls
                .saturating_sub(artifact.simulated_false_local_accepts);
            artifact.simulated_local_accuracy_milli = milli_ratio(
                simulated_local_correct,
                artifact.simulated_local_operator_calls,
            );
        }

        margins.sort_unstable();
        let total_unique_eval_rows = artifacts
            .iter()
            .map(|artifact| artifact.unique_eval_rows)
            .sum();
        let unique_local_operator_rows = artifacts
            .iter()
            .map(|artifact| artifact.unique_local_operator_rows)
            .sum();
        let unique_fallback_rows = artifacts
            .iter()
            .map(|artifact| artifact.unique_fallback_rows)
            .sum();
        let compiler_used =
            release_suite.compiler_used || artifacts.iter().any(|artifact| artifact.compiler_used);
        let eval_task_package_used = release_suite.eval_task_package_used
            && artifacts
                .iter()
                .all(|artifact| artifact.eval_task_package_used);
        let corpus_jsonl_used = release_suite.corpus_jsonl_used
            || artifacts.iter().any(|artifact| artifact.corpus_jsonl_used);
        let python_demo_used = artifacts.iter().any(|artifact| artifact.python_demo_used);
        let target_center_id_training_used = release_suite
            .artifacts
            .iter()
            .any(|source| source.target_center_id_training_used);
        let proof_rule_id_training_authority_used = release_suite
            .artifacts
            .iter()
            .any(|source| source.proof_rule_id_training_authority_used);
        let concrete_x_lookup_used = release_suite
            .artifacts
            .iter()
            .any(|source| source.concrete_x_lookup_used);
        let local_out_t_runtime_extension_used = release_suite
            .artifacts
            .iter()
            .any(|source| source.local_out_t_runtime_extension_used);
        let forbidden_used = release_suite.forbidden_used
            || python_demo_used
            || target_center_id_training_used
            || proof_rule_id_training_authority_used
            || concrete_x_lookup_used
            || local_out_t_runtime_extension_used;

        let mut report = Self {
            schema_version: "nando_phase_action_offload_audit_report_v1".to_string(),
            verdict: "PHASE_ACTION_OFFLOAD_AUDIT_V1_WATCH".to_string(),
            offload_audit_kind: ACTION_OFFLOAD_AUDIT_KIND.to_string(),
            release_suite_report_path: release_suite_report_path.display().to_string(),
            license_file_path: license_file_path.display().to_string(),
            license_package_report_path: license_package_report_path.display().to_string(),
            margin_threshold_micro,
            simulated_calls,
            local_operator_calls: simulation_summary.local_operator_calls,
            fallback_to_llm_calls: simulation_summary.fallback_to_llm_calls,
            offload_rate_milli: simulation_summary.offload_rate_milli,
            local_accuracy_milli: simulation_summary.local_accuracy_milli,
            false_local_accepts: simulation_summary.false_local_accepts,
            total_unique_eval_rows,
            unique_local_operator_rows,
            unique_fallback_rows,
            unique_offload_rate_milli: milli_ratio(unique_local_operator_rows, total_unique_eval_rows),
            median_margin_micro: percentile_i64(&margins, 50),
            p10_margin_micro: percentile_i64(&margins, 10),
            artifact_count: artifacts.len(),
            artifacts,
            release_suite_verdict: release_suite.verdict.clone(),
            release_suite_kind: release_suite.release_suite_kind.clone(),
            release_suite_gate_pass: release_suite.gate_pass(),
            release_suite_matches_sources,
            total_runtime_bytes_estimate: release_suite.total_runtime_bytes_estimate,
            total_bench_samples: release_suite.total_bench_samples,
            max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            license_package_verdict: license_report.verdict.clone(),
            license_package_kind: license_report.license_package_kind.clone(),
            license_package_gate_pass: license_report.gate_pass(),
            license_report_matches_sources,
            commercial_use_allowed: license_report.commercial_use_allowed,
            noncommercial_use_allowed: license_report.noncommercial_use_allowed,
            commercial_license_closed: license_report.commercial_license_closed,
            non_commercial_license_closed: license_report.non_commercial_license_closed,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            python_demo_used,
            target_center_id_training_used,
            proof_rule_id_training_authority_used,
            concrete_x_lookup_used,
            local_out_t_runtime_extension_used,
            forbidden_used,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            offload_sdk_api: "nando_core::PhaseCenterOffloadRuntime".to_string(),
            offload_sdk_inspect_api:
                "nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes".to_string(),
            offload_policy_api: "nando_core::PhaseCenterOffloadPolicy".to_string(),
            offload_batch_api: "nando_core::PhaseCenterFlatRuntime::offload_decisions".to_string(),
            offload_summary_api: "nando_core::PhaseCenterOffloadSummary".to_string(),
            offload_buffer_api: "nando_core::PhaseCenterFlatRuntime::offload_decisions_into"
                .to_string(),
            offload_summary_buffer_api:
                "nando_core::PhaseCenterOffloadSummary::from_repeated_decision_fn_into"
                    .to_string(),
            offload_runtime_summary_api: "nando_core::PhaseCenterOffloadRuntime::offload_summary_into"
                .to_string(),
            fallback_policy:
                "local operator only when packaged margin_micro >= threshold; otherwise fallback_to_llm"
                    .to_string(),
            audit_boundary:
                "LLM offload audit over packaged flat action scorers; not a text generator, autonomous raw action parser, or commercial license"
                    .to_string(),
            license_boundary: license_report.license_boundary.clone(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_OFFLOAD_AUDIT_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_offload_audit_report_v1"
            && self.offload_audit_kind == ACTION_OFFLOAD_AUDIT_KIND
            && self.margin_threshold_micro > 0
            && self.simulated_calls >= DEFAULT_ACTION_OFFLOAD_SIMULATED_CALLS
            && self.local_operator_calls > 0
            && self.fallback_to_llm_calls > 0
            && self.local_operator_calls + self.fallback_to_llm_calls == self.simulated_calls
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.total_unique_eval_rows > 0
            && self.unique_local_operator_rows > 0
            && self.artifact_count == self.artifacts.len()
            && self.artifact_count >= 2
            && self.artifacts.iter().all(|artifact| artifact.gate_pass())
            && self.release_suite_verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS"
            && self.release_suite_kind == ACTION_RELEASE_SUITE_KIND
            && self.release_suite_gate_pass
            && self.release_suite_matches_sources
            && self.total_runtime_bytes_estimate > 0
            && self.total_bench_samples > 0
            && self.license_package_verdict == "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS"
            && self.license_package_kind == ACTION_LICENSE_PACKAGE_KIND
            && self.license_package_gate_pass
            && self.license_report_matches_sources
            && !self.commercial_use_allowed
            && self.noncommercial_use_allowed
            && !self.commercial_license_closed
            && self.non_commercial_license_closed
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.forbidden_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
            && self.offload_sdk_inspect_api
                == "nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes"
            && self.offload_policy_api == "nando_core::PhaseCenterOffloadPolicy"
            && self.offload_batch_api == "nando_core::PhaseCenterFlatRuntime::offload_decisions"
            && self.offload_summary_api == "nando_core::PhaseCenterOffloadSummary"
            && self.offload_buffer_api
                == "nando_core::PhaseCenterFlatRuntime::offload_decisions_into"
            && self.offload_summary_buffer_api
                == "nando_core::PhaseCenterOffloadSummary::from_repeated_decision_fn_into"
            && self.offload_runtime_summary_api
                == "nando_core::PhaseCenterOffloadRuntime::offload_summary_into"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_OFFLOAD_AUDIT_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionCacheOffloadArtifactReport {
    label: String,
    unique_eval_rows: usize,
    simulated_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_hit_rate_milli: usize,
    nando_local_operator_calls: usize,
    nando_fallback_events: usize,
    nando_plus_cache_llm_calls: usize,
    nando_plus_cache_hits: usize,
    nando_operator_hit_rate_milli: usize,
    incremental_llm_calls_removed_vs_cache: usize,
    incremental_llm_call_reduction_vs_cache_milli: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    eval_pack_bytes: usize,
    runtime_bytes_estimate: usize,
    release_artifact_gate_pass: bool,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    forbidden_used: bool,
}

impl PhaseActionCacheOffloadArtifactReport {
    fn gate_pass(&self) -> bool {
        self.unique_eval_rows > 0
            && self.simulated_calls > 0
            && self.exact_cache_llm_calls > 0
            && self.exact_cache_hits > 0
            && self.exact_cache_llm_calls + self.exact_cache_hits == self.simulated_calls
            && self.nando_local_operator_calls > 0
            && self.nando_plus_cache_llm_calls + self.nando_plus_cache_hits
                == self.nando_fallback_events
            && self.nando_local_operator_calls + self.nando_fallback_events == self.simulated_calls
            && self.incremental_llm_calls_removed_vs_cache > 0
            && self.exact_cache_llm_calls
                == self
                    .nando_plus_cache_llm_calls
                    .saturating_add(self.incremental_llm_calls_removed_vs_cache)
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.package_fingerprint64 != 0
            && self.package_bytes > 0
            && self.eval_pack_bytes > 0
            && self.runtime_bytes_estimate > 0
            && self.release_artifact_gate_pass
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.forbidden_used
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionCacheOffloadBenchReport {
    schema_version: String,
    verdict: String,
    cache_offload_bench_kind: String,
    release_suite_report_path: String,
    license_file_path: String,
    license_package_report_path: String,
    margin_threshold_micro: i64,
    simulated_calls: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_hit_rate_milli: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_plus_nando_cache_hits: usize,
    nando_local_operator_calls: usize,
    nando_fallback_events: usize,
    nando_operator_hit_rate_milli: usize,
    incremental_llm_calls_removed_vs_cache: usize,
    incremental_llm_call_reduction_vs_cache_milli: usize,
    token_units_per_llm_call: usize,
    no_cache_token_units: usize,
    exact_cache_token_units: usize,
    exact_cache_plus_nando_token_units: usize,
    token_units_removed_vs_cache: usize,
    cost_units_per_llm_call: usize,
    no_cache_cost_units: usize,
    exact_cache_cost_units: usize,
    exact_cache_plus_nando_cost_units: usize,
    cost_units_removed_vs_cache: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    artifact_count: usize,
    artifacts: Vec<PhaseActionCacheOffloadArtifactReport>,
    release_suite_verdict: String,
    release_suite_gate_pass: bool,
    release_suite_matches_sources: bool,
    license_package_verdict: String,
    license_package_gate_pass: bool,
    license_report_matches_sources: bool,
    total_runtime_bytes_estimate: usize,
    max_bench_p99_latency_ns: u128,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    forbidden_used: bool,
    runtime_path: String,
    offload_sdk_api: String,
    offload_policy_api: String,
    cache_baseline_policy: String,
    product_boundary: String,
}

impl PhaseActionCacheOffloadBenchReport {
    #[allow(clippy::too_many_arguments)]
    fn from_inputs(
        release_suite_report_path: &Path,
        release_suite: &PhaseActionReleaseSuiteReport,
        release_suite_matches_sources: bool,
        license_file_path: &Path,
        license_package_report_path: &Path,
        license_report: &PhaseActionLicensePackageReport,
        license_report_matches_sources: bool,
        margin_threshold_micro: i64,
        simulated_calls: usize,
        mut artifacts: Vec<PhaseActionCacheOffloadArtifactReport>,
        samples: &[ActionOffloadSample],
    ) -> Self {
        let mut exact_seen = BTreeSet::<(usize, usize)>::new();
        let mut fallback_seen = BTreeSet::<(usize, usize)>::new();
        let mut artifact_exact_seen = vec![BTreeSet::<usize>::new(); artifacts.len()];
        let mut artifact_fallback_seen = vec![BTreeSet::<usize>::new(); artifacts.len()];
        let mut exact_cache_llm_calls = 0usize;
        let mut exact_cache_hits = 0usize;
        let mut nando_plus_cache_llm_calls = 0usize;
        let mut nando_plus_cache_hits = 0usize;
        let mut nando_local_operator_calls = 0usize;
        let mut nando_fallback_events = 0usize;
        let mut false_local_accepts = 0usize;

        if !samples.is_empty() {
            for call_index in 0..simulated_calls {
                let sample = samples[call_index % samples.len()];
                let key = (sample.artifact_index, sample.row_index);
                let Some(artifact) = artifacts.get_mut(sample.artifact_index) else {
                    continue;
                };
                artifact.simulated_calls += 1;
                if exact_seen.insert(key) {
                    exact_cache_llm_calls += 1;
                    artifact.exact_cache_llm_calls += 1;
                } else {
                    exact_cache_hits += 1;
                    artifact.exact_cache_hits += 1;
                }

                if sample.decision.is_local_operator() {
                    nando_local_operator_calls += 1;
                    artifact.nando_local_operator_calls += 1;
                    if sample.decision.is_false_local_accept() {
                        false_local_accepts += 1;
                        artifact.false_local_accepts += 1;
                    }
                } else {
                    nando_fallback_events += 1;
                    artifact.nando_fallback_events += 1;
                    if fallback_seen.insert(key) {
                        nando_plus_cache_llm_calls += 1;
                        artifact.nando_plus_cache_llm_calls += 1;
                    } else {
                        nando_plus_cache_hits += 1;
                        artifact.nando_plus_cache_hits += 1;
                    }
                    if let Some(seen) = artifact_fallback_seen.get_mut(sample.artifact_index) {
                        seen.insert(sample.row_index);
                    }
                }
                if let Some(seen) = artifact_exact_seen.get_mut(sample.artifact_index) {
                    seen.insert(sample.row_index);
                }
            }
        }

        for (artifact_index, artifact) in artifacts.iter_mut().enumerate() {
            let exact_unique = artifact_exact_seen
                .get(artifact_index)
                .map_or(0, BTreeSet::len);
            let fallback_unique = artifact_fallback_seen
                .get(artifact_index)
                .map_or(0, BTreeSet::len);
            artifact.exact_cache_llm_calls = exact_unique;
            artifact.nando_plus_cache_llm_calls = fallback_unique;
            artifact.exact_cache_hits = artifact.simulated_calls.saturating_sub(exact_unique);
            artifact.nando_plus_cache_hits = artifact
                .nando_fallback_events
                .saturating_sub(fallback_unique);
            artifact.exact_cache_hit_rate_milli =
                milli_ratio(artifact.exact_cache_hits, artifact.simulated_calls);
            artifact.nando_operator_hit_rate_milli = milli_ratio(
                artifact.nando_local_operator_calls,
                artifact.simulated_calls,
            );
            artifact.incremental_llm_calls_removed_vs_cache =
                exact_unique.saturating_sub(fallback_unique);
            artifact.incremental_llm_call_reduction_vs_cache_milli = milli_ratio(
                artifact.incremental_llm_calls_removed_vs_cache,
                exact_unique,
            );
            let local_correct = artifact
                .nando_local_operator_calls
                .saturating_sub(artifact.false_local_accepts);
            artifact.local_accuracy_milli =
                milli_ratio(local_correct, artifact.nando_local_operator_calls);
        }

        let token_units_per_llm_call = 1usize;
        let cost_units_per_llm_call = 1usize;
        let exact_cache_plus_nando_token_units =
            nando_plus_cache_llm_calls.saturating_mul(token_units_per_llm_call);
        let exact_cache_token_units =
            exact_cache_llm_calls.saturating_mul(token_units_per_llm_call);
        let no_cache_token_units = simulated_calls.saturating_mul(token_units_per_llm_call);
        let exact_cache_plus_nando_cost_units =
            nando_plus_cache_llm_calls.saturating_mul(cost_units_per_llm_call);
        let exact_cache_cost_units = exact_cache_llm_calls.saturating_mul(cost_units_per_llm_call);
        let no_cache_cost_units = simulated_calls.saturating_mul(cost_units_per_llm_call);
        let incremental_llm_calls_removed_vs_cache =
            exact_cache_llm_calls.saturating_sub(nando_plus_cache_llm_calls);
        let compiler_used =
            release_suite.compiler_used || artifacts.iter().any(|artifact| artifact.compiler_used);
        let eval_task_package_used = release_suite.eval_task_package_used
            && artifacts
                .iter()
                .all(|artifact| artifact.eval_task_package_used);
        let corpus_jsonl_used = release_suite.corpus_jsonl_used
            || artifacts.iter().any(|artifact| artifact.corpus_jsonl_used);
        let python_demo_used = artifacts.iter().any(|artifact| artifact.python_demo_used);
        let forbidden_used = release_suite.forbidden_used
            || artifacts.iter().any(|artifact| artifact.forbidden_used)
            || python_demo_used;

        let mut report = Self {
            schema_version: "nando_phase_action_cache_offload_bench_report_v1".to_string(),
            verdict: "PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_WATCH".to_string(),
            cache_offload_bench_kind: ACTION_CACHE_OFFLOAD_BENCH_KIND.to_string(),
            release_suite_report_path: release_suite_report_path.display().to_string(),
            license_file_path: license_file_path.display().to_string(),
            license_package_report_path: license_package_report_path.display().to_string(),
            margin_threshold_micro,
            simulated_calls,
            no_cache_llm_calls: simulated_calls,
            exact_cache_llm_calls,
            exact_cache_hits,
            exact_cache_hit_rate_milli: milli_ratio(exact_cache_hits, simulated_calls),
            exact_cache_plus_nando_llm_calls: nando_plus_cache_llm_calls,
            exact_cache_plus_nando_cache_hits: nando_plus_cache_hits,
            nando_local_operator_calls,
            nando_fallback_events,
            nando_operator_hit_rate_milli: milli_ratio(nando_local_operator_calls, simulated_calls),
            incremental_llm_calls_removed_vs_cache,
            incremental_llm_call_reduction_vs_cache_milli: milli_ratio(
                incremental_llm_calls_removed_vs_cache,
                exact_cache_llm_calls,
            ),
            token_units_per_llm_call,
            no_cache_token_units,
            exact_cache_token_units,
            exact_cache_plus_nando_token_units,
            token_units_removed_vs_cache: exact_cache_token_units
                .saturating_sub(exact_cache_plus_nando_token_units),
            cost_units_per_llm_call,
            no_cache_cost_units,
            exact_cache_cost_units,
            exact_cache_plus_nando_cost_units,
            cost_units_removed_vs_cache: exact_cache_cost_units
                .saturating_sub(exact_cache_plus_nando_cost_units),
            local_accuracy_milli: milli_ratio(
                nando_local_operator_calls.saturating_sub(false_local_accepts),
                nando_local_operator_calls,
            ),
            false_local_accepts,
            artifact_count: artifacts.len(),
            artifacts,
            release_suite_verdict: release_suite.verdict.clone(),
            release_suite_gate_pass: release_suite.gate_pass(),
            release_suite_matches_sources,
            license_package_verdict: license_report.verdict.clone(),
            license_package_gate_pass: license_report.gate_pass(),
            license_report_matches_sources,
            total_runtime_bytes_estimate: release_suite.total_runtime_bytes_estimate,
            max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            python_demo_used,
            forbidden_used,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            offload_sdk_api: "nando_core::PhaseCenterOffloadRuntime".to_string(),
            offload_policy_api: "nando_core::PhaseCenterOffloadPolicy".to_string(),
            cache_baseline_policy:
                "exact cache baseline: first unique eval row calls LLM, repeated exact row is cache hit"
                    .to_string(),
            product_boundary:
                "cache-enabled offload benchmark over packaged flat action scorers; not a text generator, autonomous raw action parser, broad workflow reasoning, or commercial license"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_cache_offload_bench_report_v1"
            && self.cache_offload_bench_kind == ACTION_CACHE_OFFLOAD_BENCH_KIND
            && self.margin_threshold_micro > 0
            && self.simulated_calls >= DEFAULT_ACTION_OFFLOAD_SIMULATED_CALLS
            && self.no_cache_llm_calls == self.simulated_calls
            && self.exact_cache_llm_calls > 0
            && self.exact_cache_hits > 0
            && self.exact_cache_llm_calls + self.exact_cache_hits == self.simulated_calls
            && self.exact_cache_plus_nando_llm_calls > 0
            && self.exact_cache_plus_nando_llm_calls < self.exact_cache_llm_calls
            && self.exact_cache_plus_nando_llm_calls + self.exact_cache_plus_nando_cache_hits
                == self.nando_fallback_events
            && self.nando_local_operator_calls > 0
            && self.nando_fallback_events > 0
            && self.nando_local_operator_calls + self.nando_fallback_events == self.simulated_calls
            && self.incremental_llm_calls_removed_vs_cache > 0
            && self.exact_cache_llm_calls
                == self
                    .exact_cache_plus_nando_llm_calls
                    .saturating_add(self.incremental_llm_calls_removed_vs_cache)
            && self.token_units_removed_vs_cache == self.incremental_llm_calls_removed_vs_cache
            && self.cost_units_removed_vs_cache == self.incremental_llm_calls_removed_vs_cache
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.artifact_count == self.artifacts.len()
            && self.artifact_count >= 2
            && self.artifacts.iter().all(|artifact| artifact.gate_pass())
            && self.release_suite_verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS"
            && self.release_suite_gate_pass
            && self.release_suite_matches_sources
            && self.license_package_verdict == "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS"
            && self.license_package_gate_pass
            && self.license_report_matches_sources
            && self.total_runtime_bytes_estimate > 0
            && self.max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.forbidden_used
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
            && self.offload_policy_api == "nando_core::PhaseCenterOffloadPolicy"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionWorkflowBenchReport {
    schema_version: String,
    verdict: String,
    workflow_bench_kind: String,
    release_suite_report_path: String,
    license_file_path: String,
    license_package_report_path: String,
    cache_offload_bench_report_path: String,
    release_suite_verdict: String,
    release_suite_gate_pass: bool,
    release_suite_matches_sources: bool,
    license_package_verdict: String,
    license_package_gate_pass: bool,
    license_report_matches_sources: bool,
    cache_bench_verdict: String,
    cache_bench_gate_pass: bool,
    cache_bench_report_matches_sources: bool,
    workflow_artifact_label: String,
    workflow_artifact_found: bool,
    workflow_package_fingerprint64: u64,
    workflow_package_bytes: usize,
    workflow_eval_pack_bytes: usize,
    workflow_runtime_bytes_estimate: usize,
    workflow_source_rebuild_accepted_action_tree_rows: usize,
    workflow_source_rebuild_action_tree_key_count: usize,
    workflow_source_rebuild_min_train_rows_per_action_tree: usize,
    workflow_source_rebuild_min_heldout_rows_per_action_tree: usize,
    workflow_shortcut_report_gate_pass: bool,
    workflow_operator_coverage_report_matches_corpus: bool,
    workflow_operator_coverage_report_verdict: String,
    workflow_operator_coverage_full_operator_dimension_coverage_pass: bool,
    workflow_unique_eval_rows: usize,
    workflow_simulated_calls: usize,
    workflow_exact_cache_llm_calls: usize,
    workflow_exact_cache_hits: usize,
    workflow_exact_cache_plus_nando_llm_calls: usize,
    workflow_nando_local_operator_calls: usize,
    workflow_nando_fallback_events: usize,
    workflow_incremental_llm_calls_removed_vs_cache: usize,
    workflow_incremental_llm_call_reduction_vs_cache_milli: usize,
    workflow_local_accuracy_milli: usize,
    workflow_false_local_accepts: usize,
    workflow_nando_operator_hit_rate_milli: usize,
    workflow_score_accuracy_milli: usize,
    workflow_bench_p99_latency_ns: u128,
    release_max_bench_p99_latency_ns: u128,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    forbidden_used: bool,
    non_commercial_license_closed: bool,
    commercial_license_closed: bool,
    runtime_path: String,
    offload_sdk_api: String,
    cache_baseline_policy: String,
    workflow_boundary: String,
}

impl PhaseActionWorkflowBenchReport {
    #[allow(clippy::too_many_arguments)]
    fn from_inputs(
        release_suite_report_path: &Path,
        release_suite: &PhaseActionReleaseSuiteReport,
        release_suite_matches_sources: bool,
        license_file_path: &Path,
        license_package_report_path: &Path,
        license_report: &PhaseActionLicensePackageReport,
        license_report_matches_sources: bool,
        cache_bench_report_path: &Path,
        cache_bench: &PhaseActionCacheOffloadBenchReport,
        cache_bench_report_matches_sources: bool,
        release_domain_artifact: &PhaseActionReleaseSuiteArtifactReport,
        cache_domain_artifact: &PhaseActionCacheOffloadArtifactReport,
    ) -> Self {
        let compiler_used = release_suite.compiler_used
            || cache_bench.compiler_used
            || release_domain_artifact.compiler_used
            || cache_domain_artifact.compiler_used;
        let eval_task_package_used = release_suite.eval_task_package_used
            && cache_bench.eval_task_package_used
            && release_domain_artifact.eval_task_package_used
            && cache_domain_artifact.eval_task_package_used;
        let corpus_jsonl_used = release_suite.corpus_jsonl_used
            || cache_bench.corpus_jsonl_used
            || release_domain_artifact.corpus_jsonl_used_in_score_loop
            || release_domain_artifact.corpus_jsonl_used_in_bench_loop
            || cache_domain_artifact.corpus_jsonl_used;
        let python_demo_used = cache_bench.python_demo_used
            || release_domain_artifact.python_demo_used
            || cache_domain_artifact.python_demo_used;
        let target_center_id_training_used = release_domain_artifact.target_center_id_training_used;
        let proof_rule_id_training_authority_used =
            release_domain_artifact.proof_rule_id_training_authority_used;
        let concrete_x_lookup_used = release_domain_artifact.concrete_x_lookup_used;
        let local_out_t_runtime_extension_used =
            release_domain_artifact.local_out_t_runtime_extension_used;
        let forbidden_used = release_suite.forbidden_used
            || cache_bench.forbidden_used
            || release_domain_artifact.forbidden_used()
            || cache_domain_artifact.forbidden_used
            || python_demo_used;

        let mut report = Self {
            schema_version: "nando_phase_action_workflow_bench_report_v1".to_string(),
            verdict: "PHASE_ACTION_WORKFLOW_BENCH_V1_WATCH".to_string(),
            workflow_bench_kind: ACTION_WORKFLOW_BENCH_KIND.to_string(),
            release_suite_report_path: release_suite_report_path.display().to_string(),
            license_file_path: license_file_path.display().to_string(),
            license_package_report_path: license_package_report_path.display().to_string(),
            cache_offload_bench_report_path: cache_bench_report_path.display().to_string(),
            release_suite_verdict: release_suite.verdict.clone(),
            release_suite_gate_pass: release_suite.gate_pass(),
            release_suite_matches_sources,
            license_package_verdict: license_report.verdict.clone(),
            license_package_gate_pass: license_report.gate_pass(),
            license_report_matches_sources,
            cache_bench_verdict: cache_bench.verdict.clone(),
            cache_bench_gate_pass: cache_bench.gate_pass(),
            cache_bench_report_matches_sources,
            workflow_artifact_label: release_domain_artifact.label.clone(),
            workflow_artifact_found: release_domain_artifact.label == "domain_action"
                && cache_domain_artifact.label == "domain_action",
            workflow_package_fingerprint64: release_domain_artifact.package_fingerprint64,
            workflow_package_bytes: release_domain_artifact.package_bytes,
            workflow_eval_pack_bytes: release_domain_artifact.eval_pack_bytes,
            workflow_runtime_bytes_estimate: release_domain_artifact.runtime_bytes_estimate,
            workflow_source_rebuild_accepted_action_tree_rows: release_domain_artifact
                .source_rebuild_accepted_action_tree_rows,
            workflow_source_rebuild_action_tree_key_count: release_domain_artifact
                .source_rebuild_action_tree_key_count,
            workflow_source_rebuild_min_train_rows_per_action_tree: release_domain_artifact
                .source_rebuild_min_train_rows_per_action_tree,
            workflow_source_rebuild_min_heldout_rows_per_action_tree: release_domain_artifact
                .source_rebuild_min_heldout_rows_per_action_tree,
            workflow_shortcut_report_gate_pass: release_domain_artifact.shortcut_report_gate_pass,
            workflow_operator_coverage_report_matches_corpus: release_domain_artifact
                .operator_coverage_report_matches_corpus,
            workflow_operator_coverage_report_verdict: release_domain_artifact
                .operator_coverage_report_verdict
                .clone(),
            workflow_operator_coverage_full_operator_dimension_coverage_pass:
                release_domain_artifact.operator_coverage_full_operator_dimension_coverage_pass,
            workflow_unique_eval_rows: cache_domain_artifact.unique_eval_rows,
            workflow_simulated_calls: cache_domain_artifact.simulated_calls,
            workflow_exact_cache_llm_calls: cache_domain_artifact.exact_cache_llm_calls,
            workflow_exact_cache_hits: cache_domain_artifact.exact_cache_hits,
            workflow_exact_cache_plus_nando_llm_calls: cache_domain_artifact
                .nando_plus_cache_llm_calls,
            workflow_nando_local_operator_calls: cache_domain_artifact.nando_local_operator_calls,
            workflow_nando_fallback_events: cache_domain_artifact.nando_fallback_events,
            workflow_incremental_llm_calls_removed_vs_cache: cache_domain_artifact
                .incremental_llm_calls_removed_vs_cache,
            workflow_incremental_llm_call_reduction_vs_cache_milli: cache_domain_artifact
                .incremental_llm_call_reduction_vs_cache_milli,
            workflow_local_accuracy_milli: cache_domain_artifact.local_accuracy_milli,
            workflow_false_local_accepts: cache_domain_artifact.false_local_accepts,
            workflow_nando_operator_hit_rate_milli: cache_domain_artifact
                .nando_operator_hit_rate_milli,
            workflow_score_accuracy_milli: release_domain_artifact.score_accuracy_milli,
            workflow_bench_p99_latency_ns: release_domain_artifact.bench_p99_latency_ns,
            release_max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            python_demo_used,
            target_center_id_training_used,
            proof_rule_id_training_authority_used,
            concrete_x_lookup_used,
            local_out_t_runtime_extension_used,
            forbidden_used,
            non_commercial_license_closed: license_report.non_commercial_license_closed,
            commercial_license_closed: license_report.commercial_license_closed,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            offload_sdk_api: "nando_core::PhaseCenterOffloadRuntime".to_string(),
            cache_baseline_policy:
                "domain_action workflow-shaped exact-cache baseline vs packaged Nando local operator"
                    .to_string(),
            workflow_boundary:
                "workflow-shaped domain_action offload proof over frozen packaged scorer; not broad workflow reasoning, autonomous raw action parsing, text generation, or commercial license"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_WORKFLOW_BENCH_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_workflow_bench_report_v1"
            && self.workflow_bench_kind == ACTION_WORKFLOW_BENCH_KIND
            && self.workflow_artifact_label == "domain_action"
            && self.workflow_artifact_found
            && self.release_suite_verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS"
            && self.release_suite_gate_pass
            && self.release_suite_matches_sources
            && self.license_package_verdict == "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS"
            && self.license_package_gate_pass
            && self.license_report_matches_sources
            && self.cache_bench_verdict == "PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_PASS"
            && self.cache_bench_gate_pass
            && self.cache_bench_report_matches_sources
            && self.workflow_package_fingerprint64 != 0
            && self.workflow_package_bytes > 0
            && self.workflow_eval_pack_bytes > 0
            && self.workflow_runtime_bytes_estimate > 0
            && self.workflow_source_rebuild_accepted_action_tree_rows > 0
            && self.workflow_source_rebuild_action_tree_key_count
                >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.workflow_source_rebuild_min_train_rows_per_action_tree > 0
            && self.workflow_source_rebuild_min_heldout_rows_per_action_tree > 0
            && self.workflow_shortcut_report_gate_pass
            && self.workflow_operator_coverage_report_matches_corpus
            && !self.workflow_operator_coverage_report_verdict.is_empty()
            && self.workflow_unique_eval_rows > 0
            && self.workflow_simulated_calls >= self.workflow_unique_eval_rows
            && self.workflow_exact_cache_llm_calls > 0
            && self.workflow_exact_cache_hits > 0
            && self.workflow_exact_cache_plus_nando_llm_calls < self.workflow_exact_cache_llm_calls
            && self.workflow_nando_local_operator_calls > 0
            && self
                .workflow_nando_local_operator_calls
                .saturating_add(self.workflow_nando_fallback_events)
                == self.workflow_simulated_calls
            && self.workflow_incremental_llm_calls_removed_vs_cache > 0
            && self.workflow_local_accuracy_milli == 1000
            && self.workflow_false_local_accepts == 0
            && self.workflow_score_accuracy_milli == 1000
            && self.workflow_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && self.release_max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
            && !self.forbidden_used
            && self.non_commercial_license_closed
            && !self.commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_WORKFLOW_BENCH_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionWorkflowReplayArtifactReport {
    label: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    eval_pack_bytes: usize,
    runtime_bytes_estimate: usize,
    unique_eval_rows: usize,
    trace_calls: usize,
    unique_replayed_rows: usize,
    exact_cache_llm_calls: usize,
    exact_cache_hits: usize,
    nando_plus_cache_llm_calls: usize,
    nando_plus_cache_hits: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    release_artifact_gate_pass: bool,
    product_verify_pass: bool,
    score_accuracy_milli: usize,
    score_wrong_wins: usize,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    forbidden_used: bool,
}

impl PhaseActionWorkflowReplayArtifactReport {
    fn gate_pass(&self) -> bool {
        self.release_artifact_gate_pass
            && self.product_verify_pass
            && self.package_fingerprint64 != 0
            && self.package_bytes > 0
            && self.eval_pack_bytes > 0
            && self.runtime_bytes_estimate > 0
            && self.unique_eval_rows > 0
            && self.trace_calls > 0
            && self.unique_replayed_rows > 0
            && self.exact_cache_llm_calls > 0
            && self.exact_cache_hits > 0
            && self.exact_cache_llm_calls + self.exact_cache_hits == self.trace_calls
            && self.local_operator_calls > 0
            && self.local_operator_calls + self.fallback_to_llm_calls == self.trace_calls
            && self.nando_plus_cache_llm_calls + self.nando_plus_cache_hits
                == self.fallback_to_llm_calls
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.score_accuracy_milli == 1000
            && self.score_wrong_wins == 0
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.forbidden_used
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionWorkflowReplayReport {
    schema_version: String,
    verdict: String,
    workflow_replay_kind: String,
    release_suite_report_path: String,
    license_file_path: String,
    license_package_report_path: String,
    release_suite_verdict: String,
    release_suite_gate_pass: bool,
    release_suite_matches_sources: bool,
    license_package_verdict: String,
    license_package_gate_pass: bool,
    license_report_matches_sources: bool,
    margin_threshold_micro: i64,
    workflow_sessions: usize,
    steps_per_session: usize,
    workflow_trace_calls: usize,
    package_count: usize,
    package_aliases: Vec<String>,
    all_packages_observed: bool,
    sessions_cover_all_packages: bool,
    total_unique_eval_rows: usize,
    replay_unique_rows: usize,
    exact_cache_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_hit_rate_milli: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_plus_nando_cache_hits: usize,
    nando_local_operator_calls: usize,
    nando_fallback_events: usize,
    nando_operator_hit_rate_milli: usize,
    incremental_llm_calls_removed_vs_cache: usize,
    incremental_llm_call_reduction_vs_cache_milli: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    artifact_count: usize,
    artifacts: Vec<PhaseActionWorkflowReplayArtifactReport>,
    total_runtime_bytes_estimate: usize,
    max_bench_p99_latency_ns: u128,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    forbidden_used: bool,
    non_commercial_license_closed: bool,
    commercial_license_closed: bool,
    runtime_path: String,
    offload_sdk_api: String,
    offload_policy_api: String,
    cache_baseline_policy: String,
    workflow_boundary: String,
}

impl PhaseActionWorkflowReplayReport {
    #[allow(clippy::too_many_arguments)]
    fn from_inputs(
        release_suite_report_path: &Path,
        release_suite: &PhaseActionReleaseSuiteReport,
        release_suite_matches_sources: bool,
        license_file_path: &Path,
        license_package_report_path: &Path,
        license_report: &PhaseActionLicensePackageReport,
        license_report_matches_sources: bool,
        margin_threshold_micro: i64,
        workflow_sessions: usize,
        steps_per_session: usize,
        mut artifacts: Vec<PhaseActionWorkflowReplayArtifactReport>,
        artifact_reports: &[PhaseActionOffloadArtifactReport],
        samples_by_artifact: &[Vec<ActionOffloadSample>],
    ) -> Self {
        let workflow_trace_calls = workflow_sessions.saturating_mul(steps_per_session);
        let mut exact_seen = BTreeSet::<(usize, usize)>::new();
        let mut fallback_seen = BTreeSet::<(usize, usize)>::new();
        let mut observed_artifacts = BTreeSet::<usize>::new();
        let mut sessions_cover_all_count = 0usize;
        let mut exact_cache_llm_calls = 0usize;
        let mut exact_cache_hits = 0usize;
        let mut exact_cache_plus_nando_llm_calls = 0usize;
        let mut exact_cache_plus_nando_cache_hits = 0usize;
        let mut nando_local_operator_calls = 0usize;
        let mut nando_fallback_events = 0usize;
        let mut false_local_accepts = 0usize;
        let artifact_count = artifacts.len();
        let mut artifact_call_offsets = vec![0usize; artifact_count];

        if artifact_count > 0 && workflow_sessions > 0 && steps_per_session > 0 {
            for session_index in 0..workflow_sessions {
                let mut session_artifacts = BTreeSet::<usize>::new();
                for step_index in 0..steps_per_session {
                    let artifact_index = (session_index + step_index) % artifact_count;
                    let Some(samples) = samples_by_artifact.get(artifact_index) else {
                        continue;
                    };
                    if samples.is_empty() {
                        continue;
                    }
                    let sample_index = artifact_call_offsets[artifact_index] % samples.len();
                    artifact_call_offsets[artifact_index] =
                        artifact_call_offsets[artifact_index].wrapping_add(1);
                    let sample = samples[sample_index];
                    let key = (artifact_index, sample.row_index);
                    observed_artifacts.insert(artifact_index);
                    session_artifacts.insert(artifact_index);
                    let Some(artifact) = artifacts.get_mut(artifact_index) else {
                        continue;
                    };
                    artifact.trace_calls += 1;
                    if exact_seen.insert(key) {
                        exact_cache_llm_calls += 1;
                        artifact.exact_cache_llm_calls += 1;
                        artifact.unique_replayed_rows += 1;
                    } else {
                        exact_cache_hits += 1;
                        artifact.exact_cache_hits += 1;
                    }

                    if sample.decision.is_local_operator() {
                        nando_local_operator_calls += 1;
                        artifact.local_operator_calls += 1;
                        if sample.decision.is_false_local_accept() {
                            false_local_accepts += 1;
                            artifact.false_local_accepts += 1;
                        }
                    } else {
                        nando_fallback_events += 1;
                        artifact.fallback_to_llm_calls += 1;
                        if fallback_seen.insert(key) {
                            exact_cache_plus_nando_llm_calls += 1;
                            artifact.nando_plus_cache_llm_calls += 1;
                        } else {
                            exact_cache_plus_nando_cache_hits += 1;
                            artifact.nando_plus_cache_hits += 1;
                        }
                    }
                }
                if session_artifacts.len() == artifact_count {
                    sessions_cover_all_count += 1;
                }
            }
        }

        for artifact in &mut artifacts {
            let local_correct = artifact
                .local_operator_calls
                .saturating_sub(artifact.false_local_accepts);
            artifact.local_accuracy_milli =
                milli_ratio(local_correct, artifact.local_operator_calls);
        }

        let total_unique_eval_rows = artifact_reports
            .iter()
            .map(|artifact| artifact.unique_eval_rows)
            .sum();
        let replay_unique_rows = artifacts
            .iter()
            .map(|artifact| artifact.unique_replayed_rows)
            .sum();
        let package_aliases = artifacts
            .iter()
            .map(|artifact| artifact.label.clone())
            .collect::<Vec<_>>();
        let compiler_used = release_suite.compiler_used
            || artifact_reports
                .iter()
                .any(|artifact| artifact.compiler_used);
        let eval_task_package_used = release_suite.eval_task_package_used
            && artifact_reports
                .iter()
                .all(|artifact| artifact.eval_task_package_used);
        let corpus_jsonl_used = release_suite.corpus_jsonl_used
            || artifact_reports
                .iter()
                .any(|artifact| artifact.corpus_jsonl_used);
        let python_demo_used = artifact_reports
            .iter()
            .any(|artifact| artifact.python_demo_used);
        let target_center_id_training_used = release_suite
            .artifacts
            .iter()
            .any(|artifact| artifact.target_center_id_training_used);
        let proof_rule_id_training_authority_used = release_suite
            .artifacts
            .iter()
            .any(|artifact| artifact.proof_rule_id_training_authority_used);
        let concrete_x_lookup_used = release_suite
            .artifacts
            .iter()
            .any(|artifact| artifact.concrete_x_lookup_used);
        let local_out_t_runtime_extension_used = release_suite
            .artifacts
            .iter()
            .any(|artifact| artifact.local_out_t_runtime_extension_used);
        let forbidden_used = release_suite.forbidden_used
            || artifact_reports
                .iter()
                .any(|artifact| artifact.forbidden_used)
            || python_demo_used
            || target_center_id_training_used
            || proof_rule_id_training_authority_used
            || concrete_x_lookup_used
            || local_out_t_runtime_extension_used;
        let local_correct = nando_local_operator_calls.saturating_sub(false_local_accepts);
        let incremental_llm_calls_removed_vs_cache =
            exact_cache_llm_calls.saturating_sub(exact_cache_plus_nando_llm_calls);

        let mut report = Self {
            schema_version: "nando_phase_action_workflow_replay_report_v1".to_string(),
            verdict: "PHASE_ACTION_WORKFLOW_REPLAY_V1_WATCH".to_string(),
            workflow_replay_kind: ACTION_WORKFLOW_REPLAY_KIND.to_string(),
            release_suite_report_path: release_suite_report_path.display().to_string(),
            license_file_path: license_file_path.display().to_string(),
            license_package_report_path: license_package_report_path.display().to_string(),
            release_suite_verdict: release_suite.verdict.clone(),
            release_suite_gate_pass: release_suite.gate_pass(),
            release_suite_matches_sources,
            license_package_verdict: license_report.verdict.clone(),
            license_package_gate_pass: license_report.gate_pass(),
            license_report_matches_sources,
            margin_threshold_micro,
            workflow_sessions,
            steps_per_session,
            workflow_trace_calls,
            package_count: artifact_count,
            package_aliases,
            all_packages_observed: observed_artifacts.len() == artifact_count,
            sessions_cover_all_packages: sessions_cover_all_count == workflow_sessions,
            total_unique_eval_rows,
            replay_unique_rows,
            exact_cache_llm_calls,
            exact_cache_hits,
            exact_cache_hit_rate_milli: milli_ratio(exact_cache_hits, workflow_trace_calls),
            exact_cache_plus_nando_llm_calls,
            exact_cache_plus_nando_cache_hits,
            nando_local_operator_calls,
            nando_fallback_events,
            nando_operator_hit_rate_milli: milli_ratio(nando_local_operator_calls, workflow_trace_calls),
            incremental_llm_calls_removed_vs_cache,
            incremental_llm_call_reduction_vs_cache_milli: milli_ratio(
                incremental_llm_calls_removed_vs_cache,
                exact_cache_llm_calls,
            ),
            local_accuracy_milli: milli_ratio(local_correct, nando_local_operator_calls),
            false_local_accepts,
            artifact_count,
            artifacts,
            total_runtime_bytes_estimate: release_suite.total_runtime_bytes_estimate,
            max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            python_demo_used,
            target_center_id_training_used,
            proof_rule_id_training_authority_used,
            concrete_x_lookup_used,
            local_out_t_runtime_extension_used,
            forbidden_used,
            non_commercial_license_closed: license_report.non_commercial_license_closed,
            commercial_license_closed: license_report.commercial_license_closed,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            offload_sdk_api: "nando_core::PhaseCenterOffloadRuntime".to_string(),
            offload_policy_api: "nando_core::PhaseCenterOffloadPolicy".to_string(),
            cache_baseline_policy:
                "workflow replay exact-cache baseline: first unique package,row calls LLM; packaged Nando handles high-margin repeats locally"
                    .to_string(),
            workflow_boundary:
                "deterministic multi-package workflow replay over frozen .nwpc packages and binary eval-packs; not raw action parsing, text generation, dynamic real pilot traffic, or commercial license"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_workflow_replay_report_v1"
            && self.workflow_replay_kind == ACTION_WORKFLOW_REPLAY_KIND
            && self.margin_threshold_micro > 0
            && self.workflow_sessions >= 128
            && self.steps_per_session >= 6
            && self.workflow_trace_calls == self.workflow_sessions * self.steps_per_session
            && self.package_count == self.artifacts.len()
            && self.package_count >= 3
            && self.package_aliases.len() == self.package_count
            && self.all_packages_observed
            && self.sessions_cover_all_packages
            && self.artifacts.iter().all(|artifact| artifact.gate_pass())
            && self.release_suite_verdict == "PHASE_ACTION_RELEASE_SUITE_V1_PASS"
            && self.release_suite_gate_pass
            && self.release_suite_matches_sources
            && self.license_package_verdict == "PHASE_ACTION_LICENSE_PACKAGE_V1_PASS"
            && self.license_package_gate_pass
            && self.license_report_matches_sources
            && self.total_unique_eval_rows >= 300
            && self.replay_unique_rows == self.total_unique_eval_rows
            && self.exact_cache_llm_calls == self.replay_unique_rows
            && self.exact_cache_hits > 0
            && self.exact_cache_llm_calls + self.exact_cache_hits == self.workflow_trace_calls
            && self.exact_cache_plus_nando_llm_calls > 0
            && self.exact_cache_plus_nando_llm_calls < self.exact_cache_llm_calls
            && self.exact_cache_plus_nando_llm_calls + self.exact_cache_plus_nando_cache_hits
                == self.nando_fallback_events
            && self.nando_local_operator_calls > 0
            && self.nando_fallback_events > 0
            && self.nando_local_operator_calls + self.nando_fallback_events
                == self.workflow_trace_calls
            && self.incremental_llm_calls_removed_vs_cache > 0
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.total_runtime_bytes_estimate > 0
            && self.max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
            && !self.forbidden_used
            && self.non_commercial_license_closed
            && !self.commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
            && self.offload_policy_api == "nando_core::PhaseCenterOffloadPolicy"
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS" && self.gate_body_pass()
    }
}

struct PhaseActionRegressionReportInput<'a> {
    release_suite_report_path: &'a Path,
    release_suite_report_fingerprint64: u64,
    release_suite_report_bytes: usize,
    release_suite: &'a PhaseActionReleaseSuiteReport,
    release_suite_matches_sources: bool,
    license_file_path: &'a Path,
    license_package_report_path: &'a Path,
    license_package_report_fingerprint64: u64,
    license_package_report_bytes: usize,
    license_report: &'a PhaseActionLicensePackageReport,
    license_report_matches_sources: bool,
    offload_audit_report_path: &'a Path,
    offload_audit_report_fingerprint64: u64,
    offload_audit_report_bytes: usize,
    offload_report: &'a PhaseActionOffloadAuditReport,
    offload_report_matches_sources: bool,
    cache_bench_report_path: &'a Path,
    cache_bench_report_fingerprint64: u64,
    cache_bench_report_bytes: usize,
    cache_bench_report: &'a PhaseActionCacheOffloadBenchReport,
    cache_bench_report_matches_sources: bool,
    workflow_bench_report_path: &'a Path,
    workflow_bench_report_fingerprint64: u64,
    workflow_bench_report_bytes: usize,
    workflow_bench_report: &'a PhaseActionWorkflowBenchReport,
    workflow_bench_report_matches_sources: bool,
    workflow_replay_report_path: &'a Path,
    workflow_replay_report_fingerprint64: u64,
    workflow_replay_report_bytes: usize,
    workflow_replay_report: &'a PhaseActionWorkflowReplayReport,
    workflow_replay_report_matches_sources: bool,
    operator_blueprint: &'a OperatorBlueprintContract,
}

#[derive(Clone, Debug)]
struct OperatorBlueprintContract {
    path: String,
    fingerprint64: u64,
    bytes: usize,
    formula_present: bool,
    runtime_package_contract_present: bool,
    source_verify_contract_present: bool,
    shortcut_report_contract_present: bool,
    rust_proof_path_present: bool,
    proof_invariants_present: bool,
    forbidden_invariants_present: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionRegressionReport {
    schema_version: String,
    verdict: String,
    regression_kind: String,
    release_suite_report_path: String,
    #[serde(default)]
    release_suite_report_fingerprint64: u64,
    #[serde(default)]
    release_suite_report_bytes: usize,
    license_file_path: String,
    license_package_report_path: String,
    #[serde(default)]
    license_package_report_fingerprint64: u64,
    #[serde(default)]
    license_package_report_bytes: usize,
    offload_audit_report_path: String,
    #[serde(default)]
    offload_audit_report_fingerprint64: u64,
    #[serde(default)]
    offload_audit_report_bytes: usize,
    #[serde(default)]
    cache_offload_bench_report_path: String,
    #[serde(default)]
    cache_offload_bench_report_fingerprint64: u64,
    #[serde(default)]
    cache_offload_bench_report_bytes: usize,
    #[serde(default)]
    workflow_bench_report_path: String,
    #[serde(default)]
    workflow_bench_report_fingerprint64: u64,
    #[serde(default)]
    workflow_bench_report_bytes: usize,
    #[serde(default)]
    workflow_replay_report_path: String,
    #[serde(default)]
    workflow_replay_report_fingerprint64: u64,
    #[serde(default)]
    workflow_replay_report_bytes: usize,
    release_verify_pass: bool,
    license_verify_pass: bool,
    offload_verify_pass: bool,
    #[serde(default)]
    cache_bench_verify_pass: bool,
    #[serde(default)]
    workflow_bench_verify_pass: bool,
    #[serde(default)]
    workflow_replay_verify_pass: bool,
    release_suite_matches_sources: bool,
    license_report_matches_sources: bool,
    offload_report_matches_sources: bool,
    #[serde(default)]
    cache_bench_report_matches_sources: bool,
    #[serde(default)]
    workflow_bench_report_matches_sources: bool,
    #[serde(default)]
    workflow_replay_report_matches_sources: bool,
    artifact_count: usize,
    total_runtime_bytes_estimate: usize,
    total_bench_samples: usize,
    #[serde(default)]
    total_source_verify_report_bytes: usize,
    #[serde(default)]
    total_shortcut_report_bytes: usize,
    #[serde(default)]
    all_source_verify_reports_pass: bool,
    #[serde(default)]
    all_shortcut_reports_pass: bool,
    #[serde(default)]
    all_action_ablation_collapses: bool,
    #[serde(default)]
    all_action_contract_source_rebuild_clean: bool,
    #[serde(default)]
    all_optimized_build_reports_pass: bool,
    #[serde(default)]
    total_source_rebuild_accepted_action_tree_rows: usize,
    #[serde(default)]
    total_source_rebuild_rejected_action_tree_rows: usize,
    #[serde(default)]
    total_source_rebuild_forbidden_contract_rows: usize,
    #[serde(default)]
    total_source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    min_source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    all_action_tree_key_coverage_pass: bool,
    #[serde(default)]
    all_manifest_package_parity_pass: bool,
    #[serde(default)]
    all_eval_pack_package_parity_pass: bool,
    #[serde(default)]
    all_score_report_package_parity_pass: bool,
    #[serde(default)]
    all_bench_report_package_parity_pass: bool,
    #[serde(default)]
    all_product_report_package_parity_pass: bool,
    #[serde(default)]
    all_source_rebuild_package_parity_pass: bool,
    #[serde(default)]
    all_source_verify_report_package_parity_pass: bool,
    #[serde(default)]
    all_package_report_parity_pass: bool,
    #[serde(default)]
    max_score_action_ablation_accuracy_milli: usize,
    #[serde(default)]
    max_bench_action_ablation_accuracy_milli: usize,
    #[serde(default)]
    total_score_action_ablation_wrong_wins: usize,
    #[serde(default)]
    total_bench_action_ablation_wrong_wins: usize,
    max_score_p99_latency_ns: u128,
    max_bench_p99_latency_ns: u128,
    offload_rate_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    #[serde(default)]
    cache_bench_verdict: String,
    #[serde(default)]
    cache_exact_cache_llm_calls: usize,
    #[serde(default)]
    cache_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    cache_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    cache_local_accuracy_milli: usize,
    #[serde(default)]
    cache_false_local_accepts: usize,
    #[serde(default)]
    workflow_bench_verdict: String,
    #[serde(default)]
    workflow_artifact_label: String,
    #[serde(default)]
    workflow_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    workflow_exact_cache_llm_calls: usize,
    #[serde(default)]
    workflow_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    workflow_local_accuracy_milli: usize,
    #[serde(default)]
    workflow_false_local_accepts: usize,
    #[serde(default)]
    workflow_bench_p99_latency_ns: u128,
    #[serde(default)]
    workflow_replay_verdict: String,
    #[serde(default)]
    workflow_replay_package_count: usize,
    #[serde(default)]
    workflow_replay_trace_calls: usize,
    #[serde(default)]
    workflow_replay_total_unique_eval_rows: usize,
    #[serde(default)]
    workflow_replay_unique_rows: usize,
    #[serde(default)]
    workflow_replay_exact_cache_llm_calls: usize,
    #[serde(default)]
    workflow_replay_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    workflow_replay_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    workflow_replay_incremental_llm_call_reduction_vs_cache_milli: usize,
    #[serde(default)]
    workflow_replay_local_accuracy_milli: usize,
    #[serde(default)]
    workflow_replay_false_local_accepts: usize,
    #[serde(default)]
    workflow_replay_max_bench_p99_latency_ns: u128,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    forbidden_used: bool,
    commercial_use_allowed: bool,
    noncommercial_use_allowed: bool,
    commercial_license_closed: bool,
    non_commercial_license_closed: bool,
    runtime_path: String,
    #[serde(default)]
    offload_sdk_api: String,
    #[serde(default)]
    offload_sdk_inspect_api: String,
    offload_runtime_summary_api: String,
    #[serde(default)]
    operator_blueprint_path: String,
    #[serde(default)]
    operator_blueprint_fingerprint64: u64,
    #[serde(default)]
    operator_blueprint_bytes: usize,
    #[serde(default)]
    operator_blueprint_formula_present: bool,
    #[serde(default)]
    operator_blueprint_runtime_package_contract_present: bool,
    #[serde(default)]
    operator_blueprint_source_verify_contract_present: bool,
    #[serde(default)]
    operator_blueprint_shortcut_report_contract_present: bool,
    #[serde(default)]
    operator_blueprint_rust_proof_path_present: bool,
    #[serde(default)]
    operator_blueprint_proof_invariants_present: bool,
    #[serde(default)]
    operator_blueprint_forbidden_invariants_present: bool,
    state_transition_formula: String,
    claim_boundary: String,
}

impl PhaseActionRegressionReport {
    fn from_inputs(input: PhaseActionRegressionReportInput<'_>) -> Self {
        let release_suite = input.release_suite;
        let license_report = input.license_report;
        let offload_report = input.offload_report;
        let cache_bench_report = input.cache_bench_report;
        let workflow_bench_report = input.workflow_bench_report;
        let workflow_replay_report = input.workflow_replay_report;
        let release_suite_matches_sources = input.release_suite_matches_sources;
        let license_report_matches_sources = input.license_report_matches_sources;
        let offload_report_matches_sources = input.offload_report_matches_sources;
        let cache_bench_report_matches_sources = input.cache_bench_report_matches_sources;
        let workflow_bench_report_matches_sources = input.workflow_bench_report_matches_sources;
        let workflow_replay_report_matches_sources = input.workflow_replay_report_matches_sources;
        let release_verify_pass = release_suite.gate_pass() && release_suite_matches_sources;
        let license_verify_pass = license_report.gate_pass() && license_report_matches_sources;
        let offload_verify_pass = offload_report.gate_pass() && offload_report_matches_sources;
        let cache_bench_verify_pass =
            cache_bench_report.gate_pass() && cache_bench_report_matches_sources;
        let workflow_bench_verify_pass =
            workflow_bench_report.gate_pass() && workflow_bench_report_matches_sources;
        let workflow_replay_verify_pass =
            workflow_replay_report.gate_pass() && workflow_replay_report_matches_sources;
        let compiler_used = release_suite.compiler_used
            || offload_report.compiler_used
            || cache_bench_report.compiler_used
            || workflow_bench_report.compiler_used
            || workflow_replay_report.compiler_used;
        let eval_task_package_used = release_suite.eval_task_package_used
            && offload_report.eval_task_package_used
            && cache_bench_report.eval_task_package_used
            && workflow_bench_report.eval_task_package_used
            && workflow_replay_report.eval_task_package_used;
        let corpus_jsonl_used = release_suite.corpus_jsonl_used
            || offload_report.corpus_jsonl_used
            || cache_bench_report.corpus_jsonl_used
            || workflow_bench_report.corpus_jsonl_used
            || workflow_replay_report.corpus_jsonl_used;
        let python_demo_used = offload_report.python_demo_used
            || cache_bench_report.python_demo_used
            || workflow_bench_report.python_demo_used
            || workflow_replay_report.python_demo_used;
        let forbidden_used = release_suite.forbidden_used
            || offload_report.forbidden_used
            || cache_bench_report.forbidden_used
            || workflow_bench_report.forbidden_used
            || workflow_replay_report.forbidden_used
            || python_demo_used
            || offload_report.target_center_id_training_used
            || offload_report.proof_rule_id_training_authority_used
            || offload_report.concrete_x_lookup_used
            || offload_report.local_out_t_runtime_extension_used
            || workflow_bench_report.target_center_id_training_used
            || workflow_bench_report.proof_rule_id_training_authority_used
            || workflow_bench_report.concrete_x_lookup_used
            || workflow_bench_report.local_out_t_runtime_extension_used
            || workflow_replay_report.target_center_id_training_used
            || workflow_replay_report.proof_rule_id_training_authority_used
            || workflow_replay_report.concrete_x_lookup_used
            || workflow_replay_report.local_out_t_runtime_extension_used;
        let mut report = Self {
            schema_version: "nando_phase_action_regression_report_v1".to_string(),
            verdict: "PHASE_ACTION_REGRESSION_V1_WATCH".to_string(),
            regression_kind: ACTION_REGRESSION_KIND.to_string(),
            release_suite_report_path: input.release_suite_report_path.display().to_string(),
            release_suite_report_fingerprint64: input.release_suite_report_fingerprint64,
            release_suite_report_bytes: input.release_suite_report_bytes,
            license_file_path: input.license_file_path.display().to_string(),
            license_package_report_path: input.license_package_report_path.display().to_string(),
            license_package_report_fingerprint64: input.license_package_report_fingerprint64,
            license_package_report_bytes: input.license_package_report_bytes,
            offload_audit_report_path: input.offload_audit_report_path.display().to_string(),
            offload_audit_report_fingerprint64: input.offload_audit_report_fingerprint64,
            offload_audit_report_bytes: input.offload_audit_report_bytes,
            cache_offload_bench_report_path: input.cache_bench_report_path.display().to_string(),
            cache_offload_bench_report_fingerprint64: input.cache_bench_report_fingerprint64,
            cache_offload_bench_report_bytes: input.cache_bench_report_bytes,
            workflow_bench_report_path: input.workflow_bench_report_path.display().to_string(),
            workflow_bench_report_fingerprint64: input.workflow_bench_report_fingerprint64,
            workflow_bench_report_bytes: input.workflow_bench_report_bytes,
            workflow_replay_report_path: input.workflow_replay_report_path.display().to_string(),
            workflow_replay_report_fingerprint64: input.workflow_replay_report_fingerprint64,
            workflow_replay_report_bytes: input.workflow_replay_report_bytes,
            release_verify_pass,
            license_verify_pass,
            offload_verify_pass,
            cache_bench_verify_pass,
            workflow_bench_verify_pass,
            workflow_replay_verify_pass,
            release_suite_matches_sources,
            license_report_matches_sources,
            offload_report_matches_sources,
            cache_bench_report_matches_sources,
            workflow_bench_report_matches_sources,
            workflow_replay_report_matches_sources,
            artifact_count: release_suite.artifact_count,
            total_runtime_bytes_estimate: release_suite.total_runtime_bytes_estimate,
            total_bench_samples: release_suite.total_bench_samples,
            total_source_verify_report_bytes: release_suite.total_source_verify_report_bytes,
            total_shortcut_report_bytes: release_suite.total_shortcut_report_bytes,
            all_source_verify_reports_pass: release_suite.all_source_verify_reports_pass,
            all_shortcut_reports_pass: release_suite.all_shortcut_reports_pass,
            all_action_ablation_collapses: release_suite.all_action_ablation_collapses,
            all_action_contract_source_rebuild_clean: release_suite
                .all_action_contract_source_rebuild_clean,
            all_optimized_build_reports_pass: release_suite.all_optimized_build_reports_pass,
            total_source_rebuild_accepted_action_tree_rows: release_suite
                .total_source_rebuild_accepted_action_tree_rows,
            total_source_rebuild_rejected_action_tree_rows: release_suite
                .total_source_rebuild_rejected_action_tree_rows,
            total_source_rebuild_forbidden_contract_rows: release_suite
                .total_source_rebuild_forbidden_contract_rows,
            total_source_rebuild_action_tree_key_count: release_suite
                .total_source_rebuild_action_tree_key_count,
            min_source_rebuild_action_tree_key_count: release_suite
                .min_source_rebuild_action_tree_key_count,
            all_action_tree_key_coverage_pass: release_suite.all_action_tree_key_coverage_pass,
            all_manifest_package_parity_pass: release_suite.all_manifest_package_parity_pass,
            all_eval_pack_package_parity_pass: release_suite.all_eval_pack_package_parity_pass,
            all_score_report_package_parity_pass: release_suite
                .all_score_report_package_parity_pass,
            all_bench_report_package_parity_pass: release_suite
                .all_bench_report_package_parity_pass,
            all_product_report_package_parity_pass: release_suite
                .all_product_report_package_parity_pass,
            all_source_rebuild_package_parity_pass: release_suite
                .all_source_rebuild_package_parity_pass,
            all_source_verify_report_package_parity_pass: release_suite
                .all_source_verify_report_package_parity_pass,
            all_package_report_parity_pass: release_suite.all_package_report_parity_pass,
            max_score_action_ablation_accuracy_milli: release_suite
                .max_score_action_ablation_accuracy_milli,
            max_bench_action_ablation_accuracy_milli: release_suite
                .max_bench_action_ablation_accuracy_milli,
            total_score_action_ablation_wrong_wins: release_suite
                .total_score_action_ablation_wrong_wins,
            total_bench_action_ablation_wrong_wins: release_suite
                .total_bench_action_ablation_wrong_wins,
            max_score_p99_latency_ns: release_suite.max_score_p99_latency_ns,
            max_bench_p99_latency_ns: release_suite.max_bench_p99_latency_ns,
            offload_rate_milli: offload_report.offload_rate_milli,
            local_operator_calls: offload_report.local_operator_calls,
            fallback_to_llm_calls: offload_report.fallback_to_llm_calls,
            local_accuracy_milli: offload_report.local_accuracy_milli,
            false_local_accepts: offload_report.false_local_accepts,
            cache_bench_verdict: cache_bench_report.verdict.clone(),
            cache_exact_cache_llm_calls: cache_bench_report.exact_cache_llm_calls,
            cache_exact_cache_plus_nando_llm_calls: cache_bench_report
                .exact_cache_plus_nando_llm_calls,
            cache_incremental_llm_calls_removed_vs_cache: cache_bench_report
                .incremental_llm_calls_removed_vs_cache,
            cache_local_accuracy_milli: cache_bench_report.local_accuracy_milli,
            cache_false_local_accepts: cache_bench_report.false_local_accepts,
            workflow_bench_verdict: workflow_bench_report.verdict.clone(),
            workflow_artifact_label: workflow_bench_report.workflow_artifact_label.clone(),
            workflow_incremental_llm_calls_removed_vs_cache: workflow_bench_report
                .workflow_incremental_llm_calls_removed_vs_cache,
            workflow_exact_cache_llm_calls: workflow_bench_report.workflow_exact_cache_llm_calls,
            workflow_exact_cache_plus_nando_llm_calls: workflow_bench_report
                .workflow_exact_cache_plus_nando_llm_calls,
            workflow_local_accuracy_milli: workflow_bench_report.workflow_local_accuracy_milli,
            workflow_false_local_accepts: workflow_bench_report.workflow_false_local_accepts,
            workflow_bench_p99_latency_ns: workflow_bench_report.workflow_bench_p99_latency_ns,
            workflow_replay_verdict: workflow_replay_report.verdict.clone(),
            workflow_replay_package_count: workflow_replay_report.package_count,
            workflow_replay_trace_calls: workflow_replay_report.workflow_trace_calls,
            workflow_replay_total_unique_eval_rows: workflow_replay_report.total_unique_eval_rows,
            workflow_replay_unique_rows: workflow_replay_report.replay_unique_rows,
            workflow_replay_exact_cache_llm_calls: workflow_replay_report.exact_cache_llm_calls,
            workflow_replay_exact_cache_plus_nando_llm_calls: workflow_replay_report
                .exact_cache_plus_nando_llm_calls,
            workflow_replay_incremental_llm_calls_removed_vs_cache: workflow_replay_report
                .incremental_llm_calls_removed_vs_cache,
            workflow_replay_incremental_llm_call_reduction_vs_cache_milli: workflow_replay_report
                .incremental_llm_call_reduction_vs_cache_milli,
            workflow_replay_local_accuracy_milli: workflow_replay_report.local_accuracy_milli,
            workflow_replay_false_local_accepts: workflow_replay_report.false_local_accepts,
            workflow_replay_max_bench_p99_latency_ns: workflow_replay_report.max_bench_p99_latency_ns,
            compiler_used,
            eval_task_package_used,
            corpus_jsonl_used,
            python_demo_used,
            target_center_id_training_used: offload_report.target_center_id_training_used,
            proof_rule_id_training_authority_used: offload_report
                .proof_rule_id_training_authority_used,
            concrete_x_lookup_used: offload_report.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: offload_report.local_out_t_runtime_extension_used,
            forbidden_used,
            commercial_use_allowed: license_report.commercial_use_allowed,
            noncommercial_use_allowed: license_report.noncommercial_use_allowed,
            commercial_license_closed: license_report.commercial_license_closed,
            non_commercial_license_closed: license_report.non_commercial_license_closed,
            runtime_path: "nando_core::PhaseCenterFlatRuntime".to_string(),
            offload_sdk_api: offload_report.offload_sdk_api.clone(),
            offload_sdk_inspect_api: offload_report.offload_sdk_inspect_api.clone(),
            offload_runtime_summary_api: offload_report.offload_runtime_summary_api.clone(),
            operator_blueprint_path: input.operator_blueprint.path.clone(),
            operator_blueprint_fingerprint64: input.operator_blueprint.fingerprint64,
            operator_blueprint_bytes: input.operator_blueprint.bytes,
            operator_blueprint_formula_present: input.operator_blueprint.formula_present,
            operator_blueprint_runtime_package_contract_present: input
                .operator_blueprint
                .runtime_package_contract_present,
            operator_blueprint_source_verify_contract_present: input
                .operator_blueprint
                .source_verify_contract_present,
            operator_blueprint_shortcut_report_contract_present: input
                .operator_blueprint
                .shortcut_report_contract_present,
            operator_blueprint_rust_proof_path_present: input
                .operator_blueprint
                .rust_proof_path_present,
            operator_blueprint_proof_invariants_present: input
                .operator_blueprint
                .proof_invariants_present,
            operator_blueprint_forbidden_invariants_present: input
                .operator_blueprint
                .forbidden_invariants_present,
            state_transition_formula: ACTION_STATE_TRANSITION_FORMULA.to_string(),
            claim_boundary:
                "green regression over packaged flat action scorer release/license/offload/cache/workflow-benchmark proofs anchored to OPERATOR_BLUEPRINT; not strict ordered decoder, text generation, autonomous raw action parser, or broad workflow reasoning"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_REGRESSION_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_regression_report_v1"
            && self.regression_kind == ACTION_REGRESSION_KIND
            && self.release_verify_pass
            && self.license_verify_pass
            && self.offload_verify_pass
            && self.cache_bench_verify_pass
            && self.workflow_bench_verify_pass
            && self.workflow_replay_verify_pass
            && self.release_suite_report_fingerprint64 != 0
            && self.release_suite_report_bytes > 0
            && self.license_package_report_fingerprint64 != 0
            && self.license_package_report_bytes > 0
            && self.offload_audit_report_fingerprint64 != 0
            && self.offload_audit_report_bytes > 0
            && self.cache_offload_bench_report_fingerprint64 != 0
            && self.cache_offload_bench_report_bytes > 0
            && self.workflow_bench_report_fingerprint64 != 0
            && self.workflow_bench_report_bytes > 0
            && self.workflow_replay_report_fingerprint64 != 0
            && self.workflow_replay_report_bytes > 0
            && self.release_suite_matches_sources
            && self.license_report_matches_sources
            && self.offload_report_matches_sources
            && self.cache_bench_report_matches_sources
            && self.workflow_bench_report_matches_sources
            && self.workflow_replay_report_matches_sources
            && self.artifact_count >= 2
            && self.total_runtime_bytes_estimate > 0
            && self.total_bench_samples > 0
            && self.total_source_verify_report_bytes > 0
            && self.total_shortcut_report_bytes > 0
            && self.all_source_verify_reports_pass
            && self.all_shortcut_reports_pass
            && self.all_action_ablation_collapses
            && self.all_action_contract_source_rebuild_clean
            && self.all_action_tree_key_coverage_pass
            && self.all_optimized_build_reports_pass
            && self.total_source_rebuild_accepted_action_tree_rows > 0
            && self.total_source_rebuild_rejected_action_tree_rows == 0
            && self.total_source_rebuild_forbidden_contract_rows == 0
            && self.total_source_rebuild_action_tree_key_count
                >= self
                    .artifact_count
                    .saturating_mul(MIN_ACTION_CONTRACT_KEY_COVERAGE)
            && self.min_source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.all_manifest_package_parity_pass
            && self.all_eval_pack_package_parity_pass
            && self.all_score_report_package_parity_pass
            && self.all_bench_report_package_parity_pass
            && self.all_product_report_package_parity_pass
            && self.all_source_rebuild_package_parity_pass
            && self.all_source_verify_report_package_parity_pass
            && self.all_package_report_parity_pass
            && self.total_score_action_ablation_wrong_wins > 0
            && self.total_bench_action_ablation_wrong_wins > 0
            && self.max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && self.offload_rate_milli > 0
            && self.local_operator_calls > 0
            && self.fallback_to_llm_calls > 0
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && self.cache_bench_verdict == "PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_PASS"
            && self.cache_exact_cache_llm_calls > self.cache_exact_cache_plus_nando_llm_calls
            && self.cache_incremental_llm_calls_removed_vs_cache > 0
            && self.cache_exact_cache_llm_calls
                == self
                    .cache_exact_cache_plus_nando_llm_calls
                    .saturating_add(self.cache_incremental_llm_calls_removed_vs_cache)
            && self.cache_local_accuracy_milli == 1000
            && self.cache_false_local_accepts == 0
            && self.workflow_bench_verdict == "PHASE_ACTION_WORKFLOW_BENCH_V1_PASS"
            && self.workflow_artifact_label == "domain_action"
            && self.workflow_exact_cache_llm_calls > self.workflow_exact_cache_plus_nando_llm_calls
            && self.workflow_incremental_llm_calls_removed_vs_cache > 0
            && self.workflow_exact_cache_llm_calls
                == self
                    .workflow_exact_cache_plus_nando_llm_calls
                    .saturating_add(self.workflow_incremental_llm_calls_removed_vs_cache)
            && self.workflow_local_accuracy_milli == 1000
            && self.workflow_false_local_accepts == 0
            && self.workflow_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && self.workflow_replay_verdict == "PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS"
            && self.workflow_replay_package_count >= self.artifact_count
            && self.workflow_replay_trace_calls > 0
            && self.workflow_replay_total_unique_eval_rows > 0
            && self.workflow_replay_unique_rows == self.workflow_replay_total_unique_eval_rows
            && self.workflow_replay_exact_cache_llm_calls
                > self.workflow_replay_exact_cache_plus_nando_llm_calls
            && self.workflow_replay_incremental_llm_calls_removed_vs_cache > 0
            && self.workflow_replay_exact_cache_llm_calls
                == self
                    .workflow_replay_exact_cache_plus_nando_llm_calls
                    .saturating_add(self.workflow_replay_incremental_llm_calls_removed_vs_cache)
            && self.workflow_replay_incremental_llm_call_reduction_vs_cache_milli > 0
            && self.workflow_replay_local_accuracy_milli == 1000
            && self.workflow_replay_false_local_accepts == 0
            && self.workflow_replay_max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
            && !self.forbidden_used
            && !self.commercial_use_allowed
            && self.noncommercial_use_allowed
            && !self.commercial_license_closed
            && self.non_commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
            && self.offload_sdk_inspect_api
                == "nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes"
            && self.offload_runtime_summary_api
                == "nando_core::PhaseCenterOffloadRuntime::offload_summary_into"
            && self.operator_blueprint_path == DEFAULT_OPERATOR_BLUEPRINT
            && self.operator_blueprint_fingerprint64 != 0
            && self.operator_blueprint_bytes > 0
            && self.operator_blueprint_formula_present
            && self.operator_blueprint_runtime_package_contract_present
            && self.operator_blueprint_source_verify_contract_present
            && self.operator_blueprint_shortcut_report_contract_present
            && self.operator_blueprint_rust_proof_path_present
            && self.operator_blueprint_proof_invariants_present
            && self.operator_blueprint_forbidden_invariants_present
            && self.state_transition_formula == ACTION_STATE_TRANSITION_FORMULA
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_REGRESSION_V1_PASS" && self.gate_body_pass()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PhaseActionRegressionFreezeReport {
    schema_version: String,
    verdict: String,
    freeze_kind: String,
    regression_report_path: String,
    regression_report_fingerprint64: u64,
    regression_report_bytes: usize,
    regression_verdict: String,
    regression_gate_pass: bool,
    regression_matches_sources: bool,
    release_suite_report_fingerprint64: u64,
    license_package_report_fingerprint64: u64,
    offload_audit_report_fingerprint64: u64,
    #[serde(default)]
    cache_offload_bench_report_fingerprint64: u64,
    #[serde(default)]
    cache_offload_bench_report_bytes: usize,
    #[serde(default)]
    workflow_bench_report_fingerprint64: u64,
    #[serde(default)]
    workflow_bench_report_bytes: usize,
    #[serde(default)]
    workflow_replay_report_fingerprint64: u64,
    #[serde(default)]
    workflow_replay_report_bytes: usize,
    #[serde(default)]
    cache_bench_verify_pass: bool,
    #[serde(default)]
    cache_bench_report_matches_sources: bool,
    #[serde(default)]
    workflow_bench_verify_pass: bool,
    #[serde(default)]
    workflow_bench_report_matches_sources: bool,
    #[serde(default)]
    workflow_replay_verify_pass: bool,
    #[serde(default)]
    workflow_replay_report_matches_sources: bool,
    #[serde(default)]
    cache_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    cache_exact_cache_llm_calls: usize,
    #[serde(default)]
    cache_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    workflow_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    workflow_exact_cache_llm_calls: usize,
    #[serde(default)]
    workflow_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    workflow_local_accuracy_milli: usize,
    #[serde(default)]
    workflow_false_local_accepts: usize,
    #[serde(default)]
    workflow_replay_incremental_llm_calls_removed_vs_cache: usize,
    #[serde(default)]
    workflow_replay_exact_cache_llm_calls: usize,
    #[serde(default)]
    workflow_replay_exact_cache_plus_nando_llm_calls: usize,
    #[serde(default)]
    workflow_replay_local_accuracy_milli: usize,
    #[serde(default)]
    workflow_replay_false_local_accepts: usize,
    #[serde(default)]
    workflow_replay_unique_rows: usize,
    #[serde(default)]
    workflow_replay_total_unique_eval_rows: usize,
    #[serde(default)]
    workflow_replay_package_count: usize,
    operator_blueprint_fingerprint64: u64,
    artifact_count: usize,
    total_runtime_bytes_estimate: usize,
    total_bench_samples: usize,
    all_package_report_parity_pass: bool,
    all_action_ablation_collapses: bool,
    all_action_contract_source_rebuild_clean: bool,
    #[serde(default)]
    all_action_tree_key_coverage_pass: bool,
    #[serde(default)]
    total_source_rebuild_action_tree_key_count: usize,
    #[serde(default)]
    min_source_rebuild_action_tree_key_count: usize,
    all_optimized_build_reports_pass: bool,
    max_score_p99_latency_ns: u128,
    max_bench_p99_latency_ns: u128,
    offload_rate_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    local_accuracy_milli: usize,
    false_local_accepts: usize,
    compiler_used: bool,
    eval_task_package_used: bool,
    corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    forbidden_used: bool,
    commercial_license_closed: bool,
    non_commercial_license_closed: bool,
    runtime_path: String,
    offload_sdk_api: String,
    offload_sdk_inspect_api: String,
    offload_runtime_summary_api: String,
    state_transition_formula: String,
    claim_boundary: String,
}

impl PhaseActionRegressionFreezeReport {
    fn from_regression(
        regression_report_path: &Path,
        regression_report_fingerprint64: u64,
        regression_report_bytes: usize,
        regression: &PhaseActionRegressionReport,
        regression_matches_sources: bool,
    ) -> Self {
        let mut report = Self {
            schema_version: "nando_phase_action_regression_freeze_report_v1".to_string(),
            verdict: "PHASE_ACTION_REGRESSION_FREEZE_V1_WATCH".to_string(),
            freeze_kind: ACTION_REGRESSION_FREEZE_KIND.to_string(),
            regression_report_path: regression_report_path.display().to_string(),
            regression_report_fingerprint64,
            regression_report_bytes,
            regression_verdict: regression.verdict.clone(),
            regression_gate_pass: regression.gate_pass(),
            regression_matches_sources,
            release_suite_report_fingerprint64: regression.release_suite_report_fingerprint64,
            license_package_report_fingerprint64: regression.license_package_report_fingerprint64,
            offload_audit_report_fingerprint64: regression.offload_audit_report_fingerprint64,
            cache_offload_bench_report_fingerprint64: regression
                .cache_offload_bench_report_fingerprint64,
            cache_offload_bench_report_bytes: regression.cache_offload_bench_report_bytes,
            workflow_bench_report_fingerprint64: regression.workflow_bench_report_fingerprint64,
            workflow_bench_report_bytes: regression.workflow_bench_report_bytes,
            workflow_replay_report_fingerprint64: regression.workflow_replay_report_fingerprint64,
            workflow_replay_report_bytes: regression.workflow_replay_report_bytes,
            cache_bench_verify_pass: regression.cache_bench_verify_pass,
            cache_bench_report_matches_sources: regression.cache_bench_report_matches_sources,
            workflow_bench_verify_pass: regression.workflow_bench_verify_pass,
            workflow_bench_report_matches_sources: regression.workflow_bench_report_matches_sources,
            workflow_replay_verify_pass: regression.workflow_replay_verify_pass,
            workflow_replay_report_matches_sources: regression
                .workflow_replay_report_matches_sources,
            cache_incremental_llm_calls_removed_vs_cache: regression
                .cache_incremental_llm_calls_removed_vs_cache,
            cache_exact_cache_llm_calls: regression.cache_exact_cache_llm_calls,
            cache_exact_cache_plus_nando_llm_calls: regression
                .cache_exact_cache_plus_nando_llm_calls,
            workflow_incremental_llm_calls_removed_vs_cache: regression
                .workflow_incremental_llm_calls_removed_vs_cache,
            workflow_exact_cache_llm_calls: regression.workflow_exact_cache_llm_calls,
            workflow_exact_cache_plus_nando_llm_calls: regression
                .workflow_exact_cache_plus_nando_llm_calls,
            workflow_local_accuracy_milli: regression.workflow_local_accuracy_milli,
            workflow_false_local_accepts: regression.workflow_false_local_accepts,
            workflow_replay_incremental_llm_calls_removed_vs_cache: regression
                .workflow_replay_incremental_llm_calls_removed_vs_cache,
            workflow_replay_exact_cache_llm_calls: regression
                .workflow_replay_exact_cache_llm_calls,
            workflow_replay_exact_cache_plus_nando_llm_calls: regression
                .workflow_replay_exact_cache_plus_nando_llm_calls,
            workflow_replay_local_accuracy_milli: regression.workflow_replay_local_accuracy_milli,
            workflow_replay_false_local_accepts: regression.workflow_replay_false_local_accepts,
            workflow_replay_unique_rows: regression.workflow_replay_unique_rows,
            workflow_replay_total_unique_eval_rows: regression.workflow_replay_total_unique_eval_rows,
            workflow_replay_package_count: regression.workflow_replay_package_count,
            operator_blueprint_fingerprint64: regression.operator_blueprint_fingerprint64,
            artifact_count: regression.artifact_count,
            total_runtime_bytes_estimate: regression.total_runtime_bytes_estimate,
            total_bench_samples: regression.total_bench_samples,
            all_package_report_parity_pass: regression.all_package_report_parity_pass,
            all_action_ablation_collapses: regression.all_action_ablation_collapses,
            all_action_contract_source_rebuild_clean: regression
                .all_action_contract_source_rebuild_clean,
            all_action_tree_key_coverage_pass: regression.all_action_tree_key_coverage_pass,
            total_source_rebuild_action_tree_key_count: regression
                .total_source_rebuild_action_tree_key_count,
            min_source_rebuild_action_tree_key_count: regression
                .min_source_rebuild_action_tree_key_count,
            all_optimized_build_reports_pass: regression.all_optimized_build_reports_pass,
            max_score_p99_latency_ns: regression.max_score_p99_latency_ns,
            max_bench_p99_latency_ns: regression.max_bench_p99_latency_ns,
            offload_rate_milli: regression.offload_rate_milli,
            local_operator_calls: regression.local_operator_calls,
            fallback_to_llm_calls: regression.fallback_to_llm_calls,
            local_accuracy_milli: regression.local_accuracy_milli,
            false_local_accepts: regression.false_local_accepts,
            compiler_used: regression.compiler_used,
            eval_task_package_used: regression.eval_task_package_used,
            corpus_jsonl_used: regression.corpus_jsonl_used,
            python_demo_used: regression.python_demo_used,
            target_center_id_training_used: regression.target_center_id_training_used,
            proof_rule_id_training_authority_used: regression
                .proof_rule_id_training_authority_used,
            concrete_x_lookup_used: regression.concrete_x_lookup_used,
            local_out_t_runtime_extension_used: regression.local_out_t_runtime_extension_used,
            forbidden_used: regression.forbidden_used,
            commercial_license_closed: regression.commercial_license_closed,
            non_commercial_license_closed: regression.non_commercial_license_closed,
            runtime_path: regression.runtime_path.clone(),
            offload_sdk_api: regression.offload_sdk_api.clone(),
            offload_sdk_inspect_api: regression.offload_sdk_inspect_api.clone(),
            offload_runtime_summary_api: regression.offload_runtime_summary_api.clone(),
            state_transition_formula: regression.state_transition_formula.clone(),
            claim_boundary:
                "frozen green regression checkpoint over packaged flat action scorer release/license/offload/cache/workflow-benchmark proofs; not strict ordered decoder, text generation, autonomous raw action parser, broad workflow reasoning, or commercial license"
                    .to_string(),
        };
        if report.gate_body_pass() {
            report.verdict = "PHASE_ACTION_REGRESSION_FREEZE_V1_PASS".to_string();
        }
        report
    }

    fn gate_body_pass(&self) -> bool {
        self.schema_version == "nando_phase_action_regression_freeze_report_v1"
            && self.freeze_kind == ACTION_REGRESSION_FREEZE_KIND
            && self.regression_report_fingerprint64 != 0
            && self.regression_report_bytes > 0
            && self.regression_verdict == "PHASE_ACTION_REGRESSION_V1_PASS"
            && self.regression_gate_pass
            && self.regression_matches_sources
            && self.release_suite_report_fingerprint64 != 0
            && self.license_package_report_fingerprint64 != 0
            && self.offload_audit_report_fingerprint64 != 0
            && self.cache_offload_bench_report_fingerprint64 != 0
            && self.cache_offload_bench_report_bytes > 0
            && self.workflow_bench_report_fingerprint64 != 0
            && self.workflow_bench_report_bytes > 0
            && self.workflow_replay_report_fingerprint64 != 0
            && self.workflow_replay_report_bytes > 0
            && self.cache_bench_verify_pass
            && self.cache_bench_report_matches_sources
            && self.workflow_bench_verify_pass
            && self.workflow_bench_report_matches_sources
            && self.workflow_replay_verify_pass
            && self.workflow_replay_report_matches_sources
            && self.cache_incremental_llm_calls_removed_vs_cache > 0
            && self.cache_exact_cache_llm_calls > self.cache_exact_cache_plus_nando_llm_calls
            && self.workflow_incremental_llm_calls_removed_vs_cache > 0
            && self.workflow_exact_cache_llm_calls > self.workflow_exact_cache_plus_nando_llm_calls
            && self.workflow_local_accuracy_milli == 1000
            && self.workflow_false_local_accepts == 0
            && self.workflow_replay_incremental_llm_calls_removed_vs_cache > 0
            && self.workflow_replay_exact_cache_llm_calls
                > self.workflow_replay_exact_cache_plus_nando_llm_calls
            && self.workflow_replay_exact_cache_llm_calls
                == self
                    .workflow_replay_exact_cache_plus_nando_llm_calls
                    .saturating_add(self.workflow_replay_incremental_llm_calls_removed_vs_cache)
            && self.workflow_replay_local_accuracy_milli == 1000
            && self.workflow_replay_false_local_accepts == 0
            && self.workflow_replay_unique_rows == self.workflow_replay_total_unique_eval_rows
            && self.workflow_replay_unique_rows > 0
            && self.workflow_replay_package_count >= self.artifact_count
            && self.operator_blueprint_fingerprint64 != 0
            && self.artifact_count >= 2
            && self.total_runtime_bytes_estimate > 0
            && self.total_bench_samples > 0
            && self.all_package_report_parity_pass
            && self.all_action_ablation_collapses
            && self.all_action_contract_source_rebuild_clean
            && self.all_action_tree_key_coverage_pass
            && self.total_source_rebuild_action_tree_key_count
                >= self
                    .artifact_count
                    .saturating_mul(MIN_ACTION_CONTRACT_KEY_COVERAGE)
            && self.min_source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.all_optimized_build_reports_pass
            && self.max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
            && self.offload_rate_milli > 0
            && self.local_operator_calls > 0
            && self.fallback_to_llm_calls > 0
            && self.local_accuracy_milli == 1000
            && self.false_local_accepts == 0
            && !self.compiler_used
            && self.eval_task_package_used
            && !self.corpus_jsonl_used
            && !self.python_demo_used
            && !self.target_center_id_training_used
            && !self.proof_rule_id_training_authority_used
            && !self.concrete_x_lookup_used
            && !self.local_out_t_runtime_extension_used
            && !self.forbidden_used
            && !self.commercial_license_closed
            && self.non_commercial_license_closed
            && self.runtime_path == "nando_core::PhaseCenterFlatRuntime"
            && self.offload_sdk_api == "nando_core::PhaseCenterOffloadRuntime"
            && self.offload_sdk_inspect_api
                == "nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes"
            && self.offload_runtime_summary_api
                == "nando_core::PhaseCenterOffloadRuntime::offload_summary_into"
            && self.state_transition_formula == ACTION_STATE_TRANSITION_FORMULA
    }

    fn gate_pass(&self) -> bool {
        self.verdict == "PHASE_ACTION_REGRESSION_FREEZE_V1_PASS" && self.gate_body_pass()
    }
}

fn parse_phase_package_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhasePackageConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_CORPUS));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let cells = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid cells '{value}': {error}"))?,
        None => DEFAULT_CELLS,
    };
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cells == 0 {
        return Err(String::from("cells must be greater than zero"));
    }
    Ok(PhasePackageConfig {
        corpus_path,
        package_path,
        manifest_path,
        cells,
    })
}

fn parse_phase_package_inspect_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhasePackageInspectConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhasePackageInspectConfig {
        package_path,
        manifest_path,
    })
}

fn parse_phase_action_package_inspect_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageInspectConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageInspectConfig {
        package_path,
        manifest_path,
    })
}

fn parse_phase_action_source_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionSourceVerifyConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionSourceVerifyConfig {
        package_path,
        manifest_path,
        report_path,
    })
}

fn parse_phase_action_package_score_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageScoreConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CONTRACT));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageScoreConfig {
        package_path,
        manifest_path,
        corpus_path,
        report_path,
    })
}

fn parse_phase_action_eval_pack_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionEvalPackConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CONTRACT));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionEvalPackConfig {
        package_path,
        manifest_path,
        corpus_path,
        eval_pack_path,
    })
}

fn parse_phase_action_package_score_pack_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageScorePackConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageScorePackConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        report_path,
    })
}

fn parse_phase_action_package_bench_pack_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageBenchPackConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let iterations = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid benchmark iterations '{value}': {error}"))?,
        None => DEFAULT_ACTION_BENCH_ITERATIONS,
    };
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if iterations == 0 {
        return Err(String::from(
            "benchmark iterations must be greater than zero",
        ));
    }
    Ok(PhaseActionPackageBenchPackConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        iterations,
        report_path,
    })
}

fn parse_phase_action_package_bench_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageBenchVerifyConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bench_report_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageBenchVerifyConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        report_path,
    })
}

fn parse_phase_action_product_proof_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionProductProofConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let score_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_score_report_path(&package_path));
    let bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bench_report_path(&package_path));
    let proof_report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionProductProofConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        score_report_path,
        bench_report_path,
        proof_report_path,
    })
}

fn parse_phase_action_product_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionProductVerifyConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let score_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_score_report_path(&package_path));
    let bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bench_report_path(&package_path));
    let proof_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_product_proof_report_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionProductVerifyConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        score_report_path,
        bench_report_path,
        proof_report_path,
    })
}

fn parse_phase_action_release_suite_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionReleaseSuiteConfig, String> {
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionReleaseSuiteConfig { report_path })
}

fn parse_phase_action_license_package_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionLicensePackageConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionLicensePackageConfig {
        release_suite_report_path,
        license_file_path,
        report_path,
    })
}

fn parse_phase_action_offload_audit_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionOffloadAuditConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let margin_threshold_micro = match args.next() {
        Some(value) => value
            .parse::<i64>()
            .map_err(|error| format!("invalid margin threshold micro '{value}': {error}"))?,
        None => DEFAULT_ACTION_OFFLOAD_MARGIN_THRESHOLD_MICRO,
    };
    let simulated_calls = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid simulated calls '{value}': {error}"))?,
        None => DEFAULT_ACTION_OFFLOAD_SIMULATED_CALLS,
    };
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if margin_threshold_micro <= 0 {
        return Err(String::from(
            "margin threshold micro must be greater than zero",
        ));
    }
    if simulated_calls == 0 {
        return Err(String::from("simulated calls must be greater than zero"));
    }
    Ok(PhaseActionOffloadAuditConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        margin_threshold_micro,
        simulated_calls,
        report_path,
    })
}

fn parse_phase_action_offload_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionOffloadVerifyConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionOffloadVerifyConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        report_path,
    })
}

fn parse_phase_action_cache_offload_bench_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionCacheOffloadBenchConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let margin_threshold_micro = match args.next() {
        Some(value) => value
            .parse::<i64>()
            .map_err(|error| format!("invalid margin threshold '{value}': {error}"))?,
        None => DEFAULT_ACTION_OFFLOAD_MARGIN_THRESHOLD_MICRO,
    };
    let simulated_calls = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid simulated calls '{value}': {error}"))?,
        None => DEFAULT_ACTION_OFFLOAD_SIMULATED_CALLS,
    };
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if margin_threshold_micro <= 0 {
        return Err(String::from("margin threshold must be greater than zero"));
    }
    if simulated_calls == 0 {
        return Err(String::from("simulated calls must be greater than zero"));
    }
    Ok(PhaseActionCacheOffloadBenchConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        margin_threshold_micro,
        simulated_calls,
        report_path,
    })
}

fn parse_phase_action_workflow_bench_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionWorkflowBenchConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let cache_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_BENCH_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionWorkflowBenchConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        cache_bench_report_path,
        report_path,
    })
}

fn parse_phase_action_workflow_replay_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionWorkflowReplayConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let margin_threshold_micro = match args.next() {
        Some(value) => value
            .parse::<i64>()
            .map_err(|error| format!("invalid margin threshold micro '{value}': {error}"))?,
        None => DEFAULT_ACTION_OFFLOAD_MARGIN_THRESHOLD_MICRO,
    };
    let workflow_sessions = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid workflow sessions '{value}': {error}"))?,
        None => DEFAULT_ACTION_WORKFLOW_REPLAY_SESSIONS,
    };
    let steps_per_session = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid steps per session '{value}': {error}"))?,
        None => DEFAULT_ACTION_WORKFLOW_REPLAY_STEPS_PER_SESSION,
    };
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_REPLAY_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if margin_threshold_micro <= 0 {
        return Err(String::from(
            "margin threshold micro must be greater than zero",
        ));
    }
    if workflow_sessions == 0 {
        return Err(String::from("workflow sessions must be greater than zero"));
    }
    if steps_per_session == 0 {
        return Err(String::from("steps per session must be greater than zero"));
    }
    Ok(PhaseActionWorkflowReplayConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        margin_threshold_micro,
        workflow_sessions,
        steps_per_session,
        report_path,
    })
}

fn parse_strict_multiseed_rust_audit_args(
    mut args: impl Iterator<Item = String>,
) -> Result<StrictMultiSeedRustAuditConfig, String> {
    let diagnostics_root_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STRICT_MULTI_SEED_DIAGNOSTICS_ROOT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STRICT_MULTI_SEED_AUDIT_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(StrictMultiSeedRustAuditConfig {
        diagnostics_root_path,
        report_path,
    })
}

fn parse_phase_action_regression_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionRegressionConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let offload_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_REGRESSION_REPORT));
    let cache_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT));
    let workflow_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_BENCH_REPORT));
    let workflow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_REPLAY_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionRegressionConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        offload_report_path,
        report_path,
        cache_bench_report_path,
        workflow_bench_report_path,
        workflow_replay_report_path,
    })
}

fn parse_phase_action_regression_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionRegressionVerifyConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let offload_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_REGRESSION_REPORT));
    let cache_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT));
    let workflow_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_BENCH_REPORT));
    let workflow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_REPLAY_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionRegressionVerifyConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        offload_report_path,
        report_path,
        cache_bench_report_path,
        workflow_bench_report_path,
        workflow_replay_report_path,
    })
}

fn parse_phase_action_regression_freeze_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionRegressionFreezeConfig, String> {
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_RELEASE_SUITE_REPORT));
    let license_file_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NONCOMMERCIAL_LICENSE_FILE));
    let license_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_LICENSE_REPORT));
    let offload_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_OFFLOAD_AUDIT_REPORT));
    let regression_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_REGRESSION_REPORT));
    let freeze_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_REGRESSION_FREEZE_REPORT));
    let cache_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_CACHE_OFFLOAD_BENCH_REPORT));
    let workflow_bench_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_BENCH_REPORT));
    let workflow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ACTION_WORKFLOW_REPLAY_REPORT));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionRegressionFreezeConfig {
        release_suite_report_path,
        license_file_path,
        license_report_path,
        offload_report_path,
        regression_report_path,
        freeze_report_path,
        cache_bench_report_path,
        workflow_bench_report_path,
        workflow_replay_report_path,
    })
}

fn parse_phase_action_package_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageVerifyConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_score_report_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageVerifyConfig {
        package_path,
        manifest_path,
        report_path,
    })
}

fn parse_phase_package_score_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhasePackageScoreConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_CORPUS));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhasePackageScoreConfig {
        package_path,
        manifest_path,
        corpus_path,
        report_path,
    })
}

fn parse_phase_eval_pack_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseEvalPackConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_CORPUS));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseEvalPackConfig {
        package_path,
        manifest_path,
        corpus_path,
        eval_pack_path,
    })
}

fn parse_phase_package_score_pack_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhasePackageScorePackConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_eval_task_package_path(&package_path));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhasePackageScorePackConfig {
        package_path,
        manifest_path,
        eval_pack_path,
        report_path,
    })
}

fn parse_phase_action_boundary_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionBoundaryConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_CORPUS));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionBoundaryConfig { corpus_path })
}

fn parse_phase_action_corpus_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionCorpusConfig, String> {
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CORPUS));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionCorpusConfig {
        output_path,
        report_path,
    })
}

fn parse_phase_action_domain_corpus_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionCorpusConfig, String> {
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_DOMAIN_CORPUS));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionCorpusConfig {
        output_path,
        report_path,
    })
}

fn parse_phase_action_coverage_corpus_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionCorpusConfig, String> {
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_COVERAGE_CORPUS));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionCorpusConfig {
        output_path,
        report_path,
    })
}

fn parse_phase_action_contract_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionContractConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CONTRACT));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionContractConfig {
        corpus_path,
        report_path,
    })
}

fn parse_phase_action_shortcut_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionShortcutConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CORPUS));
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionShortcutConfig {
        corpus_path,
        report_path,
    })
}

fn parse_phase_action_runtime_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionRuntimeConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CONTRACT));
    let cells = match args.next() {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|error| format!("invalid cells '{raw}': {error}"))?,
        None => DEFAULT_CELLS,
    };
    let report_path = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionRuntimeConfig {
        corpus_path,
        cells,
        report_path,
    })
}

fn parse_phase_action_package_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhaseActionPackageConfig, String> {
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_CONTRACT));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_ACTION_PACKAGE));
    let cells = match args.next() {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|error| format!("invalid cells '{raw}': {error}"))?,
        None => DEFAULT_CELLS,
    };
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhaseActionPackageConfig {
        corpus_path,
        package_path,
        cells,
        manifest_path,
    })
}

fn parse_phase_package_verify_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PhasePackageVerifyConfig, String> {
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path(DEFAULT_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_manifest_path(&package_path));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_score_report_path(&package_path));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(PhasePackageVerifyConfig {
        package_path,
        manifest_path,
        report_path,
    })
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn default_manifest_path(package_path: &Path) -> PathBuf {
    let mut manifest = package_path.as_os_str().to_os_string();
    manifest.push(".manifest.json");
    PathBuf::from(manifest)
}

fn default_score_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.score-report.json"))
}

fn default_action_score_pack_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.score-pack-report.json"))
}

fn default_bench_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.bench-report.json"))
}

fn default_action_bench_pack_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.bench-pack-report.json"))
}

fn default_product_proof_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.product-proof.json"))
}

fn default_action_source_verify_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.source-verify-report.json"))
}

fn default_action_shortcut_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.shortcut-report.json"))
}

fn default_action_operator_coverage_report_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.operator-coverage-report.json"))
}

fn default_eval_task_package_path(package_path: &Path) -> PathBuf {
    let file_stem = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".nwpc"))
        .unwrap_or("phase-center-package");
    package_path.with_file_name(format!("{file_stem}.eval-pack"))
}

fn default_action_product_bundle_paths(
    label: &str,
    package_relative: &str,
) -> PhaseActionProductBundlePaths {
    let package_path = repo_path(package_relative);
    PhaseActionProductBundlePaths {
        label: label.to_string(),
        manifest_path: default_manifest_path(&package_path),
        eval_pack_path: default_eval_task_package_path(&package_path),
        score_report_path: default_action_score_pack_report_path(&package_path),
        bench_report_path: default_action_bench_pack_report_path(&package_path),
        proof_report_path: default_product_proof_report_path(&package_path),
        source_verify_report_path: default_action_source_verify_report_path(&package_path),
        shortcut_report_path: default_action_shortcut_report_path(&package_path),
        operator_coverage_report_path: default_action_operator_coverage_report_path(&package_path),
        package_path,
    }
}

fn default_action_release_suite_bundles() -> Vec<PhaseActionProductBundlePaths> {
    vec![
        default_action_product_bundle_paths("generated_action", DEFAULT_ACTION_GENERATED_PACKAGE),
        default_action_product_bundle_paths("domain_action", DEFAULT_ACTION_DOMAIN_PACKAGE),
        default_action_product_bundle_paths("coverage_action", DEFAULT_ACTION_COVERAGE_PACKAGE),
    ]
}

fn action_product_bundle_paths_from_artifact(
    artifact: &PhaseActionReleaseSuiteArtifactReport,
) -> PhaseActionProductBundlePaths {
    PhaseActionProductBundlePaths {
        label: artifact.label.clone(),
        package_path: PathBuf::from(&artifact.package_path),
        manifest_path: PathBuf::from(&artifact.manifest_path),
        eval_pack_path: PathBuf::from(&artifact.eval_task_package_path),
        score_report_path: PathBuf::from(&artifact.score_report_path),
        bench_report_path: PathBuf::from(&artifact.bench_report_path),
        proof_report_path: PathBuf::from(&artifact.product_proof_path),
        source_verify_report_path: if artifact.source_verify_report_path.is_empty() {
            default_action_source_verify_report_path(Path::new(&artifact.package_path))
        } else {
            PathBuf::from(&artifact.source_verify_report_path)
        },
        shortcut_report_path: if artifact.shortcut_report_path.is_empty() {
            default_action_shortcut_report_path(Path::new(&artifact.package_path))
        } else {
            PathBuf::from(&artifact.shortcut_report_path)
        },
        operator_coverage_report_path: if artifact.operator_coverage_report_path.is_empty() {
            default_action_operator_coverage_report_path(Path::new(&artifact.package_path))
        } else {
            PathBuf::from(&artifact.operator_coverage_report_path)
        },
    }
}

fn load_phase_operator_rows(path: &Path) -> Result<Vec<PhaseOperatorRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<PhaseOperatorRow>(line).map_err(|error| {
            format!(
                "failed to parse '{}' line {}: {error}",
                path.display(),
                index + 1
            )
        })?;
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(format!("corpus '{}' has no rows", path.display()));
    }
    Ok(rows)
}

fn load_phase_action_contract_rows(path: &Path) -> Result<Vec<PhaseActionContractRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<PhaseActionContractRow>(line).map_err(|error| {
            format!(
                "failed to parse action contract '{}' line {}: {error}",
                path.display(),
                index + 1
            )
        })?;
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(format!(
            "action contract corpus '{}' has no rows",
            path.display()
        ));
    }
    Ok(rows)
}

fn write_action_contract_jsonl(path: &Path, rows: &[PhaseActionContractRow]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize action contract row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn generate_action_contract_corpus_v1() -> Vec<PhaseActionContractRow> {
    let operators = ActionCorpusOperator::all();
    let train_lengths = [4usize, 6, 8];
    let heldout_lengths = [5usize, 7];
    let samples_per_cell = 4usize;
    let mut rows = Vec::new();
    let mut token_seed = 0usize;

    for operator in operators {
        for &len in &train_lengths {
            for sample in 0..samples_per_cell {
                rows.push(action_corpus_row(
                    operator,
                    "train",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
        for &len in &heldout_lengths {
            for sample in 0..samples_per_cell {
                rows.push(action_corpus_row(
                    operator,
                    "heldout",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
    }

    rows
}

fn generate_domain_action_contract_corpus_v1() -> Vec<PhaseActionContractRow> {
    let operators = WorkflowActionOperator::all();
    let train_lengths = [6usize, 8, 10];
    let heldout_lengths = [7usize, 9];
    let samples_per_cell = 4usize;
    let mut rows = Vec::new();
    let mut token_seed = 0usize;

    for operator in operators {
        for &len in &train_lengths {
            for sample in 0..samples_per_cell {
                rows.push(domain_action_corpus_row(
                    operator,
                    "train",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
        for &len in &heldout_lengths {
            for sample in 0..samples_per_cell {
                rows.push(domain_action_corpus_row(
                    operator,
                    "heldout",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
    }

    rows
}

fn generate_coverage_action_contract_corpus_v1() -> Vec<PhaseActionContractRow> {
    let specs = CoverageActionSpec::all();
    let train_lengths = [6usize, 8, 10];
    let heldout_lengths = [5usize, 7, 9];
    let samples_per_cell = 2usize;
    let mut rows = Vec::new();
    let mut token_seed = 0usize;

    for spec in specs {
        for &len in &train_lengths {
            for sample in 0..samples_per_cell {
                rows.push(coverage_action_corpus_row(
                    spec,
                    "train",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
        for &len in &heldout_lengths {
            for sample in 0..samples_per_cell {
                rows.push(coverage_action_corpus_row(
                    spec,
                    "heldout",
                    len,
                    sample,
                    &mut token_seed,
                ));
            }
        }
    }

    rows
}

fn action_corpus_row(
    operator: ActionCorpusOperator,
    split: &str,
    len: usize,
    sample: usize,
    token_seed: &mut usize,
) -> PhaseActionContractRow {
    let source_tokens = action_corpus_tokens(len, token_seed);
    let correct_tokens = operator.apply(&source_tokens);
    let wrong_tokens = operator.wrong(&source_tokens);
    let action_tree = operator.action_tree();
    PhaseActionContractRow {
        schema_version: "nando_action_contract_v1".to_string(),
        task_id: format!(
            "generated_contract_v1_{}_len{}_{}_{}",
            split,
            len,
            operator.key(),
            sample
        ),
        split: split.to_string(),
        state_before: source_tokens.join(" "),
        action_tree,
        state_after_correct: correct_tokens.join(" "),
        state_after_wrong: wrong_tokens.join(" "),
    }
}

fn domain_action_corpus_row(
    operator: WorkflowActionOperator,
    split: &str,
    len: usize,
    sample: usize,
    token_seed: &mut usize,
) -> PhaseActionContractRow {
    let source_tokens = domain_action_corpus_tokens(operator, len, token_seed);
    let correct_tokens = operator.apply(&source_tokens);
    let wrong_tokens = operator.wrong(&source_tokens);
    PhaseActionContractRow {
        schema_version: "nando_action_contract_v1".to_string(),
        task_id: format!(
            "generated_domain_contract_v1_{}_len{}_{}_{}",
            split,
            len,
            operator.key(),
            sample
        ),
        split: split.to_string(),
        state_before: source_tokens.join(" "),
        action_tree: operator.action_tree(),
        state_after_correct: correct_tokens.join(" "),
        state_after_wrong: wrong_tokens.join(" "),
    }
}

fn coverage_action_corpus_row(
    spec: CoverageActionSpec,
    split: &str,
    len: usize,
    sample: usize,
    token_seed: &mut usize,
) -> PhaseActionContractRow {
    let source_tokens = coverage_action_corpus_tokens(spec, len, token_seed);
    let correct_tokens = spec.apply(&source_tokens);
    let wrong_tokens = spec.wrong(&source_tokens);
    PhaseActionContractRow {
        schema_version: "nando_action_contract_v1".to_string(),
        task_id: format!(
            "generated_coverage_contract_v1_{}_len{}_{}_{}",
            split, len, spec.key, sample
        ),
        split: split.to_string(),
        state_before: source_tokens.join(" "),
        action_tree: spec.action_tree(),
        state_after_correct: correct_tokens.join(" "),
        state_after_wrong: wrong_tokens.join(" "),
    }
}

fn action_corpus_tokens(len: usize, token_seed: &mut usize) -> Vec<String> {
    let mut tokens = Vec::with_capacity(len);
    for _ in 0..len {
        tokens.push(format!("x{:05}", *token_seed));
        *token_seed += 1;
    }
    tokens
}

fn coverage_action_corpus_tokens(
    spec: CoverageActionSpec,
    len: usize,
    token_seed: &mut usize,
) -> Vec<String> {
    let mut tokens = Vec::with_capacity(len);
    for index in 0..len {
        tokens.push(format!(
            "v5{}_{}_{:05}",
            spec.token_prefix,
            index % 7,
            *token_seed
        ));
        *token_seed += 1;
    }
    tokens
}

fn domain_action_corpus_tokens(
    operator: WorkflowActionOperator,
    len: usize,
    token_seed: &mut usize,
) -> Vec<String> {
    let prefixes = operator.token_prefixes();
    let mut tokens = Vec::with_capacity(len);
    for index in 0..len {
        let prefix = prefixes[index % prefixes.len()];
        tokens.push(format!("{prefix}_{:05}", *token_seed));
        *token_seed += 1;
    }
    tokens
}

fn compile_runtime(
    rows: &[PhaseOperatorRow],
    cells: usize,
) -> Result<(PhaseCenterFlatRuntime, BTreeMap<String, usize>, usize), String> {
    let mut train_items = Vec::new();
    let mut keys = BTreeSet::new();
    let mut skipped_train_rows = 0usize;

    for row in rows.iter().filter(|row| phase_split(row) == Some("train")) {
        let key = phase_operator_key(row);
        let Some(positive_atoms) = phase_transition_atoms(row, &row.correct_tokens) else {
            skipped_train_rows += 1;
            continue;
        };
        let Some(negative_atoms) = phase_transition_atoms(row, &row.wrong_tokens) else {
            skipped_train_rows += 1;
            continue;
        };
        keys.insert(key.clone());
        train_items.push((key, positive_atoms, negative_atoms));
    }

    if keys.is_empty() {
        return Err(String::from("no train operator keys found"));
    }

    let key_to_index = keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<BTreeMap<_, _>>();
    let mut compiler =
        PhaseCenterCompiler::new(cells, key_to_index.len()).map_err(format_runtime_error)?;

    for (key, positive_atoms, negative_atoms) in train_items {
        let Some(program_index) = key_to_index.get(&key).copied() else {
            return Err(format!("missing compiler index for key '{key}'"));
        };
        compiler
            .add_positive_atoms(program_index, positive_atoms.iter().map(String::as_str))
            .map_err(format_runtime_error)?;
        compiler
            .add_negative_atoms(program_index, negative_atoms.iter().map(String::as_str))
            .map_err(format_runtime_error)?;
    }

    let runtime = compiler.compile().map_err(format_runtime_error)?;
    Ok((runtime, key_to_index, skipped_train_rows))
}

fn compile_action_contract_runtime(
    rows: &[PhaseActionContractRow],
    cells: usize,
) -> Result<(PhaseCenterFlatRuntime, BTreeMap<String, usize>, usize), String> {
    let mut train_items = Vec::new();
    let mut keys = BTreeSet::new();
    let mut skipped_train_rows = 0usize;

    for row in rows.iter().filter(|row| row.split == "train") {
        let key = action_contract_key(row);
        let Some(positive_atoms) = action_contract_transition_atoms(row, &row.state_after_correct)
        else {
            skipped_train_rows += 1;
            continue;
        };
        let Some(negative_atoms) = action_contract_transition_atoms(row, &row.state_after_wrong)
        else {
            skipped_train_rows += 1;
            continue;
        };
        keys.insert(key.clone());
        train_items.push((key, positive_atoms, negative_atoms));
    }

    if keys.is_empty() {
        return Err(String::from("no train action contract keys found"));
    }

    let key_to_index = keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<BTreeMap<_, _>>();
    let mut compiler =
        PhaseCenterCompiler::new(cells, key_to_index.len()).map_err(format_runtime_error)?;

    for (key, positive_atoms, negative_atoms) in train_items {
        let Some(program_index) = key_to_index.get(&key).copied() else {
            return Err(format!(
                "missing action contract compiler index for key '{key}'"
            ));
        };
        compiler
            .add_positive_atoms(program_index, positive_atoms.iter().map(String::as_str))
            .map_err(format_runtime_error)?;
        compiler
            .add_negative_atoms(program_index, negative_atoms.iter().map(String::as_str))
            .map_err(format_runtime_error)?;
    }

    let runtime = compiler.compile().map_err(format_runtime_error)?;
    Ok((runtime, key_to_index, skipped_train_rows))
}

fn write_package(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn operator_keys_by_index(key_to_index: &BTreeMap<String, usize>) -> Vec<String> {
    let mut keys = vec![String::new(); key_to_index.len()];
    for (key, index) in key_to_index {
        if let Some(slot) = keys.get_mut(*index) {
            *slot = key.clone();
        }
    }
    keys
}

fn write_manifest(path: &Path, manifest: &PhasePackageManifest) -> Result<(), String> {
    write_json_file(path, manifest)
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn read_manifest(path: &Path) -> Result<PhasePackageManifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse manifest '{}': {error}", path.display()))
}

fn read_action_package_manifest(path: &Path) -> Result<PhaseActionPackageManifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action package manifest '{}': {error}",
            path.display()
        )
    })
}

fn read_score_report(path: &Path) -> Result<PhasePackageScoreReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse score report '{}': {error}", path.display()))
}

fn read_action_score_report(path: &Path) -> Result<PhaseActionPackageScoreReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action score report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_bench_report(path: &Path) -> Result<PhaseActionPackageBenchReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action bench report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_product_proof_report(path: &Path) -> Result<PhaseActionProductProofReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action product proof report '{}': {error}",
            path.display()
        )
    })
}

fn read_strict_multiseed_rust_audit_report(
    path: &Path,
) -> Result<StrictMultiSeedRustAuditReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse strict multiseed rust audit report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_source_verify_report(path: &Path) -> Result<PhaseActionSourceVerifyReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action source verify report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_shortcut_report(path: &Path) -> Result<PhaseActionShortcutReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action shortcut report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_operator_coverage_report(
    path: &Path,
) -> Result<PhaseActionOperatorCoverageReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action operator coverage report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_release_suite_report(path: &Path) -> Result<PhaseActionReleaseSuiteReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action release suite report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_license_package_report(
    path: &Path,
) -> Result<PhaseActionLicensePackageReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action license package report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_offload_audit_report(path: &Path) -> Result<PhaseActionOffloadAuditReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action offload audit report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_cache_offload_bench_report(
    path: &Path,
) -> Result<PhaseActionCacheOffloadBenchReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action cache offload bench report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_workflow_bench_report(
    path: &Path,
) -> Result<PhaseActionWorkflowBenchReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action workflow bench report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_workflow_replay_report(
    path: &Path,
) -> Result<PhaseActionWorkflowReplayReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action workflow replay report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_regression_report(path: &Path) -> Result<PhaseActionRegressionReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action regression report '{}': {error}",
            path.display()
        )
    })
}

fn read_action_regression_freeze_report(
    path: &Path,
) -> Result<PhaseActionRegressionFreezeReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse action regression freeze report '{}': {error}",
            path.display()
        )
    })
}

fn read_eval_task_package(path: &Path) -> Result<PhaseEvalTaskPackage, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    if bytes.len() < PHASE_EVAL_TASK_PACKAGE_HEADER_BYTES {
        return Err(format!(
            "phase eval task package '{}' is too short",
            path.display()
        ));
    }
    if bytes[..PHASE_EVAL_TASK_PACKAGE_MAGIC.len()] != PHASE_EVAL_TASK_PACKAGE_MAGIC {
        return Err(format!(
            "phase eval task package '{}' has invalid magic",
            path.display()
        ));
    }

    let mut offset = PHASE_EVAL_TASK_PACKAGE_MAGIC.len();
    let cells = read_u32(&bytes, &mut offset, path)? as usize;
    let package_fingerprint64 = read_u64(&bytes, &mut offset, path)?;
    let rows = read_u32(&bytes, &mut offset, path)? as usize;
    let task_count = read_u32(&bytes, &mut offset, path)? as usize;
    let action_task_count = read_u32(&bytes, &mut offset, path)? as usize;
    let missing_centers = read_u32(&bytes, &mut offset, path)? as usize;
    let skipped_rows = read_u32(&bytes, &mut offset, path)? as usize;
    let action_ablation_missing_centers = read_u32(&bytes, &mut offset, path)? as usize;
    if cells == 0 || task_count == 0 || action_task_count == 0 {
        return Err(format!(
            "phase eval task package '{}' has empty cells/tasks",
            path.display()
        ));
    }

    let tasks = read_eval_task_list(&bytes, &mut offset, cells, task_count, path)?;
    let action_ablation_tasks =
        read_eval_task_list(&bytes, &mut offset, cells, action_task_count, path)?;
    if offset != bytes.len() {
        return Err(format!(
            "phase eval task package '{}' has trailing bytes",
            path.display()
        ));
    }

    Ok(PhaseEvalTaskPackage {
        cells,
        package_fingerprint64,
        rows,
        prepared: PreparedEval {
            tasks,
            action_ablation_tasks,
            missing_centers,
            skipped_rows,
            action_ablation_missing_centers,
            heldout_surface_groups: 0,
            heldout_noise_groups: 0,
        },
    })
}

fn validate_manifest_package_match(
    manifest: &PhasePackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
) -> Result<(), String> {
    let matches = manifest.package_fingerprint64 == package_info.fingerprint64
        && manifest.package_bytes == package_bytes_len
        && manifest.cells == package_info.cells
        && manifest.flat_records == package_info.record_count
        && manifest.operator_keys.len() == package_info.record_count
        && manifest.operator_keys.iter().all(|key| !key.is_empty());
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase package manifest does not match package",
        ))
    }
}

fn validate_action_manifest_package_match(
    manifest: &PhaseActionPackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
) -> Result<(), String> {
    let (source_contract_fingerprint64, source_contract_bytes) =
        inspect_report_file(Path::new(&manifest.corpus_path))?;
    let matches = manifest.schema_version == "nando_phase_action_package_manifest_v1"
        && manifest.package_kind == "phase_action_contract_v1_c32_smoke"
        && manifest.source_contract_fingerprint64 == source_contract_fingerprint64
        && manifest.source_contract_bytes == source_contract_bytes
        && manifest.package_fingerprint64 == package_info.fingerprint64
        && manifest.package_bytes == package_bytes_len
        && manifest.cells == package_info.cells
        && manifest.flat_records == package_info.record_count
        && manifest.operator_keys.len() == package_info.record_count
        && manifest.operator_keys.iter().all(|key| !key.is_empty())
        && manifest.inspected_payload_bytes == package_info.payload_bytes
        && manifest.package_magic == package_info.magic.to_vec();
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action package manifest does not match package",
        ))
    }
}

#[derive(Clone, Debug)]
struct PhaseActionSourceRebuildAudit {
    source_contract_fingerprint64: u64,
    source_contract_bytes: usize,
    source_rebuild_matches_package: bool,
    source_rebuild_package_fingerprint64: u64,
    source_rebuild_package_bytes: usize,
    source_rebuild_flat_records: usize,
    source_rebuild_operator_keys_match: bool,
    source_rebuild_contract_verdict: String,
    source_rebuild_contract_gate_pass: bool,
    source_rebuild_accepted_action_tree_rows: usize,
    source_rebuild_rejected_action_tree_rows: usize,
    source_rebuild_forbidden_operator_label_rows: usize,
    source_rebuild_forbidden_slot_map_rows: usize,
    source_rebuild_forbidden_target_leak_rows: usize,
    source_rebuild_forbidden_lookup_authority_rows: usize,
    source_rebuild_forbidden_local_out_t_rows: usize,
    source_rebuild_forbidden_arrow_demo_rows: usize,
    source_rebuild_concrete_output_token_leak_rows: usize,
    source_rebuild_action_tree_key_count: usize,
    source_rebuild_train_action_tree_key_count: usize,
    source_rebuild_heldout_action_tree_key_count: usize,
    source_rebuild_min_train_rows_per_action_tree: usize,
    source_rebuild_min_heldout_rows_per_action_tree: usize,
    source_rebuild_skipped_train_rows: usize,
}

impl PhaseActionSourceRebuildAudit {
    fn gate_pass(&self) -> bool {
        self.source_contract_fingerprint64 != 0
            && self.source_contract_bytes > 0
            && self.source_rebuild_matches_package
            && self.source_rebuild_package_fingerprint64 != 0
            && self.source_rebuild_package_bytes > 0
            && self.source_rebuild_flat_records > 0
            && self.source_rebuild_operator_keys_match
            && self.source_rebuild_contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
            && self.source_rebuild_contract_gate_pass
            && self.source_rebuild_accepted_action_tree_rows > 0
            && self.source_rebuild_rejected_action_tree_rows == 0
            && self.source_rebuild_forbidden_operator_label_rows == 0
            && self.source_rebuild_forbidden_slot_map_rows == 0
            && self.source_rebuild_forbidden_target_leak_rows == 0
            && self.source_rebuild_forbidden_lookup_authority_rows == 0
            && self.source_rebuild_forbidden_local_out_t_rows == 0
            && self.source_rebuild_forbidden_arrow_demo_rows == 0
            && self.source_rebuild_concrete_output_token_leak_rows == 0
            && self.source_rebuild_action_tree_key_count >= MIN_ACTION_CONTRACT_KEY_COVERAGE
            && self.source_rebuild_train_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_heldout_action_tree_key_count
                == self.source_rebuild_action_tree_key_count
            && self.source_rebuild_min_train_rows_per_action_tree > 0
            && self.source_rebuild_min_heldout_rows_per_action_tree > 0
            && self.source_rebuild_skipped_train_rows == 0
    }
}

fn rebuild_action_package_from_source(
    manifest: &PhaseActionPackageManifest,
    package_bytes: &[u8],
) -> Result<PhaseActionSourceRebuildAudit, String> {
    let corpus_path = Path::new(&manifest.corpus_path);
    let (source_contract_fingerprint64, source_contract_bytes) = inspect_report_file(corpus_path)?;
    let rows = load_phase_action_contract_rows(corpus_path)?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    let (rebuilt_runtime, rebuilt_key_to_index, source_rebuild_skipped_train_rows) =
        compile_action_contract_runtime(&rows, manifest.cells)?;
    let rebuilt_bytes = rebuilt_runtime.to_bytes().map_err(format_runtime_error)?;
    let rebuilt_info =
        PhaseCenterFlatRuntime::inspect_bytes(&rebuilt_bytes).map_err(format_runtime_error)?;
    let rebuilt_operator_keys = operator_keys_by_index(&rebuilt_key_to_index);
    let source_rebuild_operator_keys_match = rebuilt_operator_keys == manifest.operator_keys;
    let source_anchor_matches_manifest = manifest.source_contract_fingerprint64
        == source_contract_fingerprint64
        && manifest.source_contract_bytes == source_contract_bytes;
    let source_rebuild_matches_package = source_anchor_matches_manifest
        && contract_report.gate_pass()
        && contract_report.verdict == manifest.contract_verdict
        && rows.len() == manifest.rows
        && contract_report.train_rows == manifest.train_rows
        && contract_report.heldout_rows == manifest.heldout_rows
        && source_rebuild_skipped_train_rows == manifest.skipped_train_rows
        && rebuilt_bytes == package_bytes
        && rebuilt_bytes.len() == manifest.package_bytes
        && rebuilt_info.fingerprint64 == manifest.package_fingerprint64
        && rebuilt_info.record_count == manifest.flat_records
        && rebuilt_info.cells == manifest.cells
        && rebuilt_info.payload_bytes == manifest.inspected_payload_bytes
        && rebuilt_runtime.bytes_estimate() == manifest.runtime_bytes_estimate
        && source_rebuild_operator_keys_match;

    Ok(PhaseActionSourceRebuildAudit {
        source_contract_fingerprint64,
        source_contract_bytes,
        source_rebuild_matches_package,
        source_rebuild_package_fingerprint64: rebuilt_info.fingerprint64,
        source_rebuild_package_bytes: rebuilt_bytes.len(),
        source_rebuild_flat_records: rebuilt_info.record_count,
        source_rebuild_operator_keys_match,
        source_rebuild_contract_verdict: contract_report.verdict.clone(),
        source_rebuild_contract_gate_pass: contract_report.gate_pass(),
        source_rebuild_accepted_action_tree_rows: contract_report.accepted_action_tree_rows,
        source_rebuild_rejected_action_tree_rows: contract_report.rejected_rows(),
        source_rebuild_forbidden_operator_label_rows: contract_report.forbidden_operator_label_rows,
        source_rebuild_forbidden_slot_map_rows: contract_report.forbidden_slot_map_rows,
        source_rebuild_forbidden_target_leak_rows: contract_report.forbidden_target_leak_rows,
        source_rebuild_forbidden_lookup_authority_rows: contract_report
            .forbidden_lookup_authority_rows,
        source_rebuild_forbidden_local_out_t_rows: contract_report.forbidden_local_out_t_rows,
        source_rebuild_forbidden_arrow_demo_rows: contract_report.forbidden_arrow_demo_rows,
        source_rebuild_concrete_output_token_leak_rows: contract_report
            .concrete_output_token_leak_rows,
        source_rebuild_action_tree_key_count: contract_report.action_tree_key_count,
        source_rebuild_train_action_tree_key_count: contract_report.train_action_tree_key_count,
        source_rebuild_heldout_action_tree_key_count: contract_report.heldout_action_tree_key_count,
        source_rebuild_min_train_rows_per_action_tree: contract_report
            .min_train_rows_per_action_tree,
        source_rebuild_min_heldout_rows_per_action_tree: contract_report
            .min_heldout_rows_per_action_tree,
        source_rebuild_skipped_train_rows,
    })
}

fn validate_action_score_report_match(
    report: &PhaseActionPackageScoreReport,
    manifest: &PhaseActionPackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime_bytes_estimate: usize,
) -> Result<(), String> {
    let matches = report.schema_version == "nando_phase_action_package_score_report_v1"
        && report.package_kind == manifest.package_kind
        && report.cells == manifest.cells
        && report.flat_records == package_info.record_count
        && report.manifest_operator_keys == manifest.operator_keys.len()
        && report.package_fingerprint64 == package_info.fingerprint64
        && report.package_bytes == package_bytes_len
        && report.inspected_payload_bytes == package_info.payload_bytes
        && report.runtime_bytes_estimate == runtime_bytes_estimate
        && report.manifest_verdict == manifest.verdict
        && report.claim_boundary == manifest.claim_boundary
        && report.license_boundary == manifest.license_boundary;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action score report does not match package/manifest",
        ))
    }
}

fn validate_action_bench_report_match(
    report: &PhaseActionPackageBenchReport,
    manifest: &PhaseActionPackageManifest,
    eval_package: &PhaseEvalTaskPackage,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime_bytes_estimate: usize,
) -> Result<(), String> {
    let matches = report.schema_version == "nando_phase_action_package_bench_report_v1"
        && report.package_kind == manifest.package_kind
        && report.cells == manifest.cells
        && report.flat_records == package_info.record_count
        && report.manifest_operator_keys == manifest.operator_keys.len()
        && report.package_fingerprint64 == package_info.fingerprint64
        && report.eval_pack_package_fingerprint64 == eval_package.package_fingerprint64
        && report.package_bytes == package_bytes_len
        && report.eval_pack_bytes == eval_package.serialized_len()
        && report.inspected_payload_bytes == package_info.payload_bytes
        && report.runtime_bytes_estimate == runtime_bytes_estimate
        && report.rows == eval_package.rows
        && report.heldout_eval_rows == eval_package.prepared.tasks.len()
        && report.action_ablation_eval_rows == eval_package.prepared.action_ablation_tasks.len()
        && report.manifest_verdict == manifest.verdict
        && report.contract_verdict == manifest.contract_verdict
        && report.claim_boundary == manifest.claim_boundary
        && report.license_boundary == manifest.license_boundary;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action bench report does not match package/manifest/eval-pack",
        ))
    }
}

struct PhaseActionProductProofValidationInput<'a> {
    report: &'a PhaseActionProductProofReport,
    manifest: &'a PhaseActionPackageManifest,
    eval_package: &'a PhaseEvalTaskPackage,
    score_report: &'a PhaseActionPackageScoreReport,
    bench_report: &'a PhaseActionPackageBenchReport,
    package_info: &'a nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    source_rebuild: &'a PhaseActionSourceRebuildAudit,
}

fn validate_action_product_proof_report_match(
    input: PhaseActionProductProofValidationInput<'_>,
) -> Result<(), String> {
    let report = input.report;
    let manifest = input.manifest;
    let eval_package = input.eval_package;
    let score_report = input.score_report;
    let bench_report = input.bench_report;
    let package_info = input.package_info;
    let package_bytes_len = input.package_bytes_len;
    let source_rebuild = input.source_rebuild;
    let matches = report.schema_version == "nando_phase_action_product_proof_report_v1"
        && report.product_proof_kind == ACTION_PRODUCT_PROOF_KIND
        && report.package_kind == manifest.package_kind
        && report.source_contract_fingerprint64 == manifest.source_contract_fingerprint64
        && report.source_contract_bytes == manifest.source_contract_bytes
        && report.source_rebuild_matches_package == source_rebuild.source_rebuild_matches_package
        && report.source_rebuild_package_fingerprint64
            == source_rebuild.source_rebuild_package_fingerprint64
        && report.source_rebuild_package_bytes == source_rebuild.source_rebuild_package_bytes
        && report.source_rebuild_flat_records == source_rebuild.source_rebuild_flat_records
        && report.source_rebuild_operator_keys_match
            == source_rebuild.source_rebuild_operator_keys_match
        && report.source_rebuild_contract_verdict == source_rebuild.source_rebuild_contract_verdict
        && report.source_rebuild_contract_gate_pass
            == source_rebuild.source_rebuild_contract_gate_pass
        && report.source_rebuild_accepted_action_tree_rows
            == source_rebuild.source_rebuild_accepted_action_tree_rows
        && report.source_rebuild_rejected_action_tree_rows
            == source_rebuild.source_rebuild_rejected_action_tree_rows
        && report.source_rebuild_forbidden_operator_label_rows
            == source_rebuild.source_rebuild_forbidden_operator_label_rows
        && report.source_rebuild_forbidden_slot_map_rows
            == source_rebuild.source_rebuild_forbidden_slot_map_rows
        && report.source_rebuild_forbidden_target_leak_rows
            == source_rebuild.source_rebuild_forbidden_target_leak_rows
        && report.source_rebuild_forbidden_lookup_authority_rows
            == source_rebuild.source_rebuild_forbidden_lookup_authority_rows
        && report.source_rebuild_forbidden_local_out_t_rows
            == source_rebuild.source_rebuild_forbidden_local_out_t_rows
        && report.source_rebuild_forbidden_arrow_demo_rows
            == source_rebuild.source_rebuild_forbidden_arrow_demo_rows
        && report.source_rebuild_concrete_output_token_leak_rows
            == source_rebuild.source_rebuild_concrete_output_token_leak_rows
        && report.source_rebuild_action_tree_key_count
            == source_rebuild.source_rebuild_action_tree_key_count
        && report.source_rebuild_train_action_tree_key_count
            == source_rebuild.source_rebuild_train_action_tree_key_count
        && report.source_rebuild_heldout_action_tree_key_count
            == source_rebuild.source_rebuild_heldout_action_tree_key_count
        && report.source_rebuild_min_train_rows_per_action_tree
            == source_rebuild.source_rebuild_min_train_rows_per_action_tree
        && report.source_rebuild_min_heldout_rows_per_action_tree
            == source_rebuild.source_rebuild_min_heldout_rows_per_action_tree
        && report.source_rebuild_skipped_train_rows
            == source_rebuild.source_rebuild_skipped_train_rows
        && report.cells == manifest.cells
        && report.flat_records == package_info.record_count
        && report.manifest_operator_keys == manifest.operator_keys.len()
        && report.package_fingerprint64 == package_info.fingerprint64
        && report.eval_pack_package_fingerprint64 == eval_package.package_fingerprint64
        && report.package_bytes == package_bytes_len
        && report.eval_pack_bytes == eval_package.serialized_len()
        && report.runtime_bytes_estimate == manifest.runtime_bytes_estimate
        && report.score_report_verdict == score_report.verdict
        && report.bench_report_verdict == bench_report.verdict
        && report.contract_verdict == manifest.contract_verdict
        && report.manifest_verdict == manifest.verdict
        && report.rows == manifest.rows
        && report.heldout_eval_rows == eval_package.prepared.tasks.len()
        && report.action_ablation_eval_rows == eval_package.prepared.action_ablation_tasks.len()
        && report.score_accuracy_milli == score_report.accuracy_milli
        && report.score_wrong_wins == score_report.wrong_wins
        && report.score_p99_latency_ns == score_report.p99_latency_ns
        && report.score_action_ablation_accuracy_milli
            == score_report.action_ablation_accuracy_milli
        && report.score_action_ablation_wrong_wins == score_report.action_ablation_wrong_wins
        && report.bench_iterations == bench_report.bench_iterations
        && report.bench_samples == bench_report.bench_samples
        && report.bench_accuracy_milli == bench_report.accuracy_milli
        && report.bench_wrong_wins == bench_report.wrong_wins
        && report.bench_p99_latency_ns == bench_report.p99_latency_ns
        && report.bench_p99_latency_gate_ns == bench_report.p99_latency_gate_ns
        && report.bench_action_ablation_accuracy_milli
            == bench_report.action_ablation_accuracy_milli
        && report.bench_action_ablation_wrong_wins == bench_report.action_ablation_wrong_wins
        && report.optimized_build == (score_report.optimized_build && bench_report.optimized_build)
        && report.claim_boundary == manifest.claim_boundary
        && report.license_boundary == manifest.license_boundary;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action product proof report does not match package/manifest/eval-pack/reports",
        ))
    }
}

struct PhaseActionSourceVerifyValidationInput<'a> {
    report: &'a PhaseActionSourceVerifyReport,
    package_path: &'a Path,
    manifest_path: &'a Path,
    manifest: &'a PhaseActionPackageManifest,
    package_info: &'a nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime_bytes_estimate: usize,
    manifest_matches_package: bool,
    source_rebuild: &'a PhaseActionSourceRebuildAudit,
}

fn report_path_points_to_same_file(report_path: &str, expected_path: &Path) -> bool {
    if report_path == expected_path.display().to_string() {
        return true;
    }

    let report_path = Path::new(report_path);
    let mut report_candidates = vec![report_path.to_path_buf()];
    if !report_path.is_absolute() {
        report_candidates.push(repo_path(report_path.to_string_lossy().as_ref()));
    }

    let mut expected_candidates = vec![expected_path.to_path_buf()];
    if !expected_path.is_absolute() {
        expected_candidates.push(repo_path(expected_path.to_string_lossy().as_ref()));
    }

    let report_canonical = report_candidates
        .iter()
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .collect::<Vec<_>>();
    let expected_canonical = expected_candidates
        .iter()
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .collect::<Vec<_>>();

    report_canonical
        .iter()
        .any(|report| expected_canonical.iter().any(|expected| report == expected))
}

fn validate_action_source_verify_report_match(
    input: PhaseActionSourceVerifyValidationInput<'_>,
) -> Result<(), String> {
    let report = input.report;
    let manifest = input.manifest;
    let package_info = input.package_info;
    let source_rebuild = input.source_rebuild;
    let matches = report.gate_pass()
        && report.package_kind == manifest.package_kind
        && report_path_points_to_same_file(&report.package_path, input.package_path)
        && report_path_points_to_same_file(&report.manifest_path, input.manifest_path)
        && report.corpus_path == manifest.corpus_path
        && report.source_contract_fingerprint64 == manifest.source_contract_fingerprint64
        && report.source_contract_bytes == manifest.source_contract_bytes
        && report.source_contract_fingerprint64 == source_rebuild.source_contract_fingerprint64
        && report.source_contract_bytes == source_rebuild.source_contract_bytes
        && report.source_rebuild_matches_package == source_rebuild.source_rebuild_matches_package
        && report.source_rebuild_package_fingerprint64
            == source_rebuild.source_rebuild_package_fingerprint64
        && report.source_rebuild_package_bytes == source_rebuild.source_rebuild_package_bytes
        && report.source_rebuild_flat_records == source_rebuild.source_rebuild_flat_records
        && report.source_rebuild_operator_keys_match
            == source_rebuild.source_rebuild_operator_keys_match
        && report.source_rebuild_contract_verdict == source_rebuild.source_rebuild_contract_verdict
        && report.source_rebuild_contract_gate_pass
            == source_rebuild.source_rebuild_contract_gate_pass
        && report.source_rebuild_accepted_action_tree_rows
            == source_rebuild.source_rebuild_accepted_action_tree_rows
        && report.source_rebuild_rejected_action_tree_rows
            == source_rebuild.source_rebuild_rejected_action_tree_rows
        && report.source_rebuild_forbidden_operator_label_rows
            == source_rebuild.source_rebuild_forbidden_operator_label_rows
        && report.source_rebuild_forbidden_slot_map_rows
            == source_rebuild.source_rebuild_forbidden_slot_map_rows
        && report.source_rebuild_forbidden_target_leak_rows
            == source_rebuild.source_rebuild_forbidden_target_leak_rows
        && report.source_rebuild_forbidden_lookup_authority_rows
            == source_rebuild.source_rebuild_forbidden_lookup_authority_rows
        && report.source_rebuild_forbidden_local_out_t_rows
            == source_rebuild.source_rebuild_forbidden_local_out_t_rows
        && report.source_rebuild_forbidden_arrow_demo_rows
            == source_rebuild.source_rebuild_forbidden_arrow_demo_rows
        && report.source_rebuild_concrete_output_token_leak_rows
            == source_rebuild.source_rebuild_concrete_output_token_leak_rows
        && report.source_rebuild_action_tree_key_count
            == source_rebuild.source_rebuild_action_tree_key_count
        && report.source_rebuild_train_action_tree_key_count
            == source_rebuild.source_rebuild_train_action_tree_key_count
        && report.source_rebuild_heldout_action_tree_key_count
            == source_rebuild.source_rebuild_heldout_action_tree_key_count
        && report.source_rebuild_min_train_rows_per_action_tree
            == source_rebuild.source_rebuild_min_train_rows_per_action_tree
        && report.source_rebuild_min_heldout_rows_per_action_tree
            == source_rebuild.source_rebuild_min_heldout_rows_per_action_tree
        && report.source_rebuild_skipped_train_rows
            == source_rebuild.source_rebuild_skipped_train_rows
        && report.package_fingerprint64 == package_info.fingerprint64
        && report.package_bytes == input.package_bytes_len
        && report.cells == package_info.cells
        && report.flat_records == package_info.record_count
        && report.runtime_bytes_estimate == input.runtime_bytes_estimate
        && report.manifest_matches_package == input.manifest_matches_package
        && report.manifest_gate_pass == manifest.gate_pass()
        && report.compiler_path == "nando_core::PhaseCenterCompiler"
        && report.package_path_api == "nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes"
        && report.runtime_path == "nando_core::PhaseCenterFlatRuntime"
        && report.python_demo_used == manifest.python_demo_used
        && report.target_center_id_training_used == manifest.target_center_id_training_used
        && report.proof_rule_id_training_authority_used
            == manifest.proof_rule_id_training_authority_used
        && report.concrete_x_lookup_used == manifest.concrete_x_lookup_used
        && report.local_out_t_runtime_extension_used == manifest.local_out_t_runtime_extension_used;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action source verify report does not match package/manifest/source rebuild",
        ))
    }
}

fn validate_action_shortcut_report_match(
    report: &PhaseActionShortcutReport,
    manifest: &PhaseActionPackageManifest,
) -> Result<(), String> {
    let matches = report.gate_pass()
        && report.schema_version == "nando_phase_action_shortcut_report_v1"
        && report.verdict == "PHASE_ACTION_SHORTCUT_V1_PASS"
        && report_path_points_to_same_file(&report.corpus_path, Path::new(&manifest.corpus_path))
        && report.rows == manifest.rows
        && report.train_rows == manifest.train_rows
        && report.heldout_rows == manifest.heldout_rows
        && report.operator_key_count == manifest.operator_keys.len()
        && report.heldout_operator_keys_seen_in_train_rows == report.heldout_rows
        && report.heldout_operator_key_missing_rows == 0
        && report.exact_state_lookup_hits == 0
        && report.exact_transition_lookup_hits == 0
        && report.heldout_token_overlap_rows == 0
        && report.heldout_length_seen_in_train_rows == 0
        && report.non_same_bag_rows == 0
        && report.correct_wrong_identical_rows == 0
        && report.source_bigram_correct_wins == 0
        && report.python_demo_used == manifest.python_demo_used
        && report.target_center_id_training_used == manifest.target_center_id_training_used
        && report.proof_rule_id_training_authority_used
            == manifest.proof_rule_id_training_authority_used
        && report.concrete_x_lookup_used == manifest.concrete_x_lookup_used
        && report.local_out_t_runtime_extension_used == manifest.local_out_t_runtime_extension_used;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action shortcut report does not match package manifest/corpus gate",
        ))
    }
}

fn validate_action_operator_coverage_report_match(
    report: &PhaseActionOperatorCoverageReport,
    manifest: &PhaseActionPackageManifest,
) -> Result<(), String> {
    let rows = load_phase_action_contract_rows(Path::new(&manifest.corpus_path))?;
    let contract_report = PhaseActionContractReport::from_rows(&rows);
    let rebuilt = PhaseActionOperatorCoverageReport::from_rows(
        Path::new(&manifest.corpus_path),
        &rows,
        &contract_report,
    );
    let matches = report == &rebuilt
        && report.schema_version == "nando_phase_action_operator_coverage_report_v1"
        && report_path_points_to_same_file(&report.corpus_path, Path::new(&manifest.corpus_path))
        && report.rows == manifest.rows
        && report.train_rows == manifest.train_rows
        && report.heldout_rows == manifest.heldout_rows
        && report.action_tree_key_count == manifest.operator_keys.len()
        && report.contract_gate_pass
        && !report.label_authority_used
        && !report.python_demo_used;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action operator coverage report does not match package manifest/corpus gate",
        ))
    }
}

fn build_action_release_suite_artifact_report(
    paths: &PhaseActionProductBundlePaths,
) -> Result<PhaseActionReleaseSuiteArtifactReport, String> {
    let package_bytes = std::fs::read(&paths.package_path)
        .map_err(|error| format!("failed to read '{}': {error}", paths.package_path.display()))?;
    let package_info =
        PhaseCenterFlatRuntime::inspect_bytes(&package_bytes).map_err(format_runtime_error)?;
    let runtime =
        PhaseCenterFlatRuntime::from_bytes(&package_bytes).map_err(format_runtime_error)?;
    let manifest = read_action_package_manifest(&paths.manifest_path)?;
    let eval_package = read_eval_task_package(&paths.eval_pack_path)?;
    let score_report = read_action_score_report(&paths.score_report_path)?;
    let bench_report = read_action_bench_report(&paths.bench_report_path)?;
    let product_report = read_action_product_proof_report(&paths.proof_report_path)?;
    let source_verify_report = read_action_source_verify_report(&paths.source_verify_report_path)?;
    let (source_verify_report_fingerprint64, source_verify_report_bytes) =
        inspect_report_file(&paths.source_verify_report_path)?;
    let shortcut_report = read_action_shortcut_report(&paths.shortcut_report_path)?;
    let (shortcut_report_fingerprint64, shortcut_report_bytes) =
        inspect_report_file(&paths.shortcut_report_path)?;
    let operator_coverage_report =
        read_action_operator_coverage_report(&paths.operator_coverage_report_path)?;
    let (operator_coverage_report_fingerprint64, operator_coverage_report_bytes) =
        inspect_report_file(&paths.operator_coverage_report_path)?;
    let source_rebuild = rebuild_action_package_from_source(&manifest, &package_bytes)?;
    let runtime_bytes_estimate = runtime.bytes_estimate();

    let manifest_matches =
        validate_action_manifest_package_match(&manifest, &package_info, package_bytes.len())
            .is_ok();
    let eval_pack_matches =
        validate_action_eval_task_package_match(&eval_package, &manifest, &package_info).is_ok();
    let score_report_matches = validate_action_score_report_match(
        &score_report,
        &manifest,
        &package_info,
        package_bytes.len(),
        runtime_bytes_estimate,
    )
    .is_ok();
    let bench_report_matches = validate_action_bench_report_match(
        &bench_report,
        &manifest,
        &eval_package,
        &package_info,
        package_bytes.len(),
        runtime_bytes_estimate,
    )
    .is_ok();
    let product_report_matches =
        validate_action_product_proof_report_match(PhaseActionProductProofValidationInput {
            report: &product_report,
            manifest: &manifest,
            eval_package: &eval_package,
            score_report: &score_report,
            bench_report: &bench_report,
            package_info: &package_info,
            package_bytes_len: package_bytes.len(),
            source_rebuild: &source_rebuild,
        })
        .is_ok();
    let source_verify_report_matches =
        validate_action_source_verify_report_match(PhaseActionSourceVerifyValidationInput {
            report: &source_verify_report,
            package_path: &paths.package_path,
            manifest_path: &paths.manifest_path,
            manifest: &manifest,
            package_info: &package_info,
            package_bytes_len: package_bytes.len(),
            runtime_bytes_estimate,
            manifest_matches_package: manifest_matches,
            source_rebuild: &source_rebuild,
        })
        .is_ok();
    let shortcut_report_matches =
        validate_action_shortcut_report_match(&shortcut_report, &manifest).is_ok();
    let operator_coverage_report_matches =
        validate_action_operator_coverage_report_match(&operator_coverage_report, &manifest)
            .is_ok();
    let manifest_gate_pass = manifest.gate_pass();
    let score_report_gate_pass = score_report.gate_pass();
    let bench_report_gate_pass = bench_report.gate_pass();
    let product_report_gate_pass = product_report.gate_pass();
    let source_verify_report_gate_pass = source_verify_report.gate_pass();
    let shortcut_report_gate_pass = shortcut_report.gate_pass();
    let product_verify_pass = manifest_matches
        && eval_pack_matches
        && score_report_matches
        && bench_report_matches
        && product_report_matches
        && source_verify_report_matches
        && shortcut_report_matches
        && operator_coverage_report_matches
        && manifest_gate_pass
        && score_report_gate_pass
        && bench_report_gate_pass
        && product_report_gate_pass
        && source_verify_report_gate_pass
        && shortcut_report_gate_pass;

    Ok(PhaseActionReleaseSuiteArtifactReport {
        label: paths.label.clone(),
        package_kind: manifest.package_kind.clone(),
        package_path: paths.package_path.display().to_string(),
        manifest_path: paths.manifest_path.display().to_string(),
        eval_task_package_path: paths.eval_pack_path.display().to_string(),
        score_report_path: paths.score_report_path.display().to_string(),
        bench_report_path: paths.bench_report_path.display().to_string(),
        product_proof_path: paths.proof_report_path.display().to_string(),
        source_verify_report_path: paths.source_verify_report_path.display().to_string(),
        shortcut_report_path: paths.shortcut_report_path.display().to_string(),
        operator_coverage_report_path: paths.operator_coverage_report_path.display().to_string(),
        source_verify_report_fingerprint64,
        source_verify_report_bytes,
        source_verify_report_verdict: source_verify_report.verdict.clone(),
        source_verify_report_matches_package: source_verify_report_matches,
        source_verify_report_gate_pass,
        shortcut_report_fingerprint64,
        shortcut_report_bytes,
        shortcut_report_verdict: shortcut_report.verdict.clone(),
        shortcut_report_matches_corpus: shortcut_report_matches,
        shortcut_report_gate_pass,
        operator_coverage_report_fingerprint64,
        operator_coverage_report_bytes,
        operator_coverage_report_verdict: operator_coverage_report.verdict.clone(),
        operator_coverage_report_matches_corpus: operator_coverage_report_matches,
        operator_coverage_report_gate_pass: operator_coverage_report.gate_pass(),
        operator_coverage_full_operator_dimension_coverage_pass: operator_coverage_report
            .full_operator_dimension_coverage_pass,
        operator_coverage_min_dimension_value_count: operator_coverage_report
            .min_dimension_value_count,
        operator_coverage_wide_dimension_count: operator_coverage_report.wide_dimension_count,
        operator_coverage_select_value_count: operator_coverage_report.select_value_count,
        operator_coverage_transform_value_count: operator_coverage_report.transform_value_count,
        operator_coverage_write_value_count: operator_coverage_report.write_value_count,
        operator_coverage_condition_value_count: operator_coverage_report.condition_value_count,
        operator_coverage_check_value_count: operator_coverage_report.check_value_count,
        source_contract_fingerprint64: product_report.source_contract_fingerprint64,
        source_contract_bytes: product_report.source_contract_bytes,
        source_rebuild_matches_package: product_report.source_rebuild_matches_package,
        source_rebuild_package_fingerprint64: product_report.source_rebuild_package_fingerprint64,
        source_rebuild_package_bytes: product_report.source_rebuild_package_bytes,
        source_rebuild_flat_records: product_report.source_rebuild_flat_records,
        source_rebuild_operator_keys_match: product_report.source_rebuild_operator_keys_match,
        source_rebuild_contract_gate_pass: product_report.source_rebuild_contract_gate_pass,
        source_rebuild_accepted_action_tree_rows: product_report
            .source_rebuild_accepted_action_tree_rows,
        source_rebuild_rejected_action_tree_rows: product_report
            .source_rebuild_rejected_action_tree_rows,
        source_rebuild_forbidden_operator_label_rows: product_report
            .source_rebuild_forbidden_operator_label_rows,
        source_rebuild_forbidden_slot_map_rows: product_report
            .source_rebuild_forbidden_slot_map_rows,
        source_rebuild_forbidden_target_leak_rows: product_report
            .source_rebuild_forbidden_target_leak_rows,
        source_rebuild_forbidden_lookup_authority_rows: product_report
            .source_rebuild_forbidden_lookup_authority_rows,
        source_rebuild_forbidden_local_out_t_rows: product_report
            .source_rebuild_forbidden_local_out_t_rows,
        source_rebuild_forbidden_arrow_demo_rows: product_report
            .source_rebuild_forbidden_arrow_demo_rows,
        source_rebuild_concrete_output_token_leak_rows: product_report
            .source_rebuild_concrete_output_token_leak_rows,
        source_rebuild_action_tree_key_count: product_report.source_rebuild_action_tree_key_count,
        source_rebuild_train_action_tree_key_count: product_report
            .source_rebuild_train_action_tree_key_count,
        source_rebuild_heldout_action_tree_key_count: product_report
            .source_rebuild_heldout_action_tree_key_count,
        source_rebuild_min_train_rows_per_action_tree: product_report
            .source_rebuild_min_train_rows_per_action_tree,
        source_rebuild_min_heldout_rows_per_action_tree: product_report
            .source_rebuild_min_heldout_rows_per_action_tree,
        source_rebuild_skipped_train_rows: product_report.source_rebuild_skipped_train_rows,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: package_bytes.len(),
        eval_pack_bytes: eval_package.serialized_len(),
        runtime_bytes_estimate,
        rows: manifest.rows,
        heldout_eval_rows: eval_package.prepared.tasks.len(),
        action_ablation_eval_rows: eval_package.prepared.action_ablation_tasks.len(),
        score_accuracy_milli: score_report.accuracy_milli,
        score_wrong_wins: score_report.wrong_wins,
        score_p99_latency_ns: score_report.p99_latency_ns,
        score_action_ablation_accuracy_milli: score_report.action_ablation_accuracy_milli,
        score_action_ablation_wrong_wins: score_report.action_ablation_wrong_wins,
        bench_iterations: bench_report.bench_iterations,
        bench_samples: bench_report.bench_samples,
        bench_accuracy_milli: bench_report.accuracy_milli,
        bench_wrong_wins: bench_report.wrong_wins,
        bench_p99_latency_ns: bench_report.p99_latency_ns,
        bench_p99_latency_gate_ns: bench_report.p99_latency_gate_ns,
        bench_action_ablation_accuracy_milli: bench_report.action_ablation_accuracy_milli,
        bench_action_ablation_wrong_wins: bench_report.action_ablation_wrong_wins,
        score_report_verdict: score_report.verdict.clone(),
        bench_report_verdict: bench_report.verdict.clone(),
        product_report_verdict: product_report.verdict.clone(),
        manifest_gate_pass,
        manifest_matches_package: manifest_matches,
        eval_pack_matches_package: eval_pack_matches,
        score_report_matches_package: score_report_matches,
        bench_report_matches_package: bench_report_matches,
        product_report_matches_package: product_report_matches,
        score_report_gate_pass,
        bench_report_gate_pass,
        product_report_gate_pass,
        product_verify_pass,
        compiler_used: product_report.compiler_used,
        optimized_build: product_report.optimized_build,
        eval_task_package_used: product_report.eval_task_package_used,
        corpus_jsonl_used_in_score_loop: product_report.corpus_jsonl_used_in_score_loop,
        corpus_jsonl_used_in_bench_loop: product_report.corpus_jsonl_used_in_bench_loop,
        python_demo_used: product_report.python_demo_used,
        target_center_id_training_used: product_report.target_center_id_training_used,
        proof_rule_id_training_authority_used: product_report.proof_rule_id_training_authority_used,
        concrete_x_lookup_used: product_report.concrete_x_lookup_used,
        local_out_t_runtime_extension_used: product_report.local_out_t_runtime_extension_used,
        product_boundary: product_report.product_boundary.clone(),
        license_boundary: product_report.license_boundary.clone(),
    })
}

fn rebuild_action_release_suite_report(
    report: &PhaseActionReleaseSuiteReport,
) -> Result<PhaseActionReleaseSuiteReport, String> {
    let rebuilt_artifacts = report
        .artifacts
        .iter()
        .map(action_product_bundle_paths_from_artifact)
        .map(|paths| build_action_release_suite_artifact_report(&paths))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PhaseActionReleaseSuiteReport::from_artifacts(
        rebuilt_artifacts,
    ))
}

fn audit_cargo_license_files() -> Result<CargoLicenseAudit, String> {
    let root_cargo_path = repo_path("Cargo.toml");
    let cli_cargo_path = repo_path("crates/nando-cli/Cargo.toml");
    let core_cargo_path = repo_path("crates/nando-core/Cargo.toml");
    let eval_cargo_path = repo_path("crates/nando-eval/Cargo.toml");
    let root_cargo = std::fs::read_to_string(&root_cargo_path)
        .map_err(|error| format!("failed to read '{}': {error}", root_cargo_path.display()))?;
    let crate_cargos = [&cli_cargo_path, &core_cargo_path, &eval_cargo_path]
        .into_iter()
        .map(|path| {
            std::fs::read_to_string(path)
                .map_err(|error| format!("failed to read '{}': {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CargoLicenseAudit {
        workspace_license_file_declared: root_cargo
            .contains("license-file = \"LICENSE-NONCOMMERCIAL.md\""),
        workspace_mit_license_declared: root_cargo.contains("license = \"MIT\""),
        crate_license_file_workspace_declared: crate_cargos
            .iter()
            .all(|text| text.contains("license-file.workspace = true")),
        crate_license_workspace_declared: crate_cargos
            .iter()
            .any(|text| text.contains("license.workspace = true")),
    })
}

fn file_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn inspect_report_file(path: &Path) -> Result<(u64, usize), String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    Ok((file_fingerprint64(&bytes), bytes.len()))
}

fn inspect_operator_blueprint_contract(path: &Path) -> Result<OperatorBlueprintContract, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        format!(
            "operator blueprint '{}' is not valid UTF-8: {error}",
            path.display()
        )
    })?;
    let forbidden_invariants_present = [
        "no exact lookup",
        "no target_id",
        "no proof_rule_id authority",
        "no concrete_x_lookup",
        "no fixed frame_id",
        "no manual local_out_t",
        "no hand-coded bind(X)",
    ]
    .iter()
    .all(|needle| text.contains(needle));

    Ok(OperatorBlueprintContract {
        path: path.display().to_string(),
        fingerprint64: file_fingerprint64(&bytes),
        bytes: bytes.len(),
        formula_present: text.contains(ACTION_STATE_TRANSITION_FORMULA),
        runtime_package_contract_present: text.contains("## Runtime Package Contract")
            && text.contains("contract JSONL")
            && text.contains("-> .nwpc flat runtime package")
            && text.contains("-> verify bundle"),
        source_verify_contract_present: text.contains("phase-action-source-verify-v1")
            && text.contains("source-verify command")
            && text.contains("source_verify_report_gate_pass")
            && text.contains("all_source_verify_reports_pass"),
        shortcut_report_contract_present: text.contains("phase-action-shortcut-v1")
            && text.contains("shortcut report")
            && text.contains("shortcut_report_gate_pass")
            && text.contains("all_shortcut_reports_pass"),
        rust_proof_path_present: text.contains("Python demos are no longer proof artifacts")
            && text.contains("Accepted proof path")
            && text.contains("Rust compiler/package")
            && text.contains("flat CPU score-pack"),
        proof_invariants_present: text.contains("## Proof invariants")
            && text.contains("shortcut gates clean")
            && text.contains("flat/runtime parity exact")
            && text.contains("all_package_report_parity_pass")
            && text.contains("ablation of required channel collapses")
            && text.contains("all_action_ablation_collapses"),
        forbidden_invariants_present,
    })
}

fn build_action_license_package_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
) -> Result<PhaseActionLicensePackageReport, String> {
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let license_bytes = std::fs::read(license_file_path)
        .map_err(|error| format!("failed to read '{}': {error}", license_file_path.display()))?;
    let license_text = std::str::from_utf8(&license_bytes).map_err(|error| {
        format!(
            "license file '{}' is not valid UTF-8: {error}",
            license_file_path.display()
        )
    })?;
    let license_fingerprint64 = file_fingerprint64(&license_bytes);
    let cargo_audit = audit_cargo_license_files()?;
    Ok(PhaseActionLicensePackageReport::from_inputs(
        release_suite_report_path,
        &release_suite,
        release_suite_matches_sources,
        license_file_path,
        license_text,
        license_fingerprint64,
        cargo_audit,
    ))
}

fn build_action_offload_audit_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
    license_report_path: &Path,
    margin_threshold_micro: i64,
    simulated_calls: usize,
) -> Result<PhaseActionOffloadAuditReport, String> {
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let license_report = read_action_license_package_report(license_report_path)?;
    let rebuilt_license_report =
        build_action_license_package_report(release_suite_report_path, license_file_path)?;
    let license_report_matches_sources = license_report == rebuilt_license_report;

    let mut artifacts = Vec::with_capacity(release_suite.artifacts.len());
    let mut samples = Vec::new();
    let mut margins = Vec::new();
    for (artifact_index, artifact) in release_suite.artifacts.iter().enumerate() {
        let (report, mut artifact_samples, mut artifact_margins) =
            build_action_offload_artifact_report(artifact_index, artifact, margin_threshold_micro)?;
        artifacts.push(report);
        samples.append(&mut artifact_samples);
        margins.append(&mut artifact_margins);
    }

    Ok(PhaseActionOffloadAuditReport::from_inputs(
        release_suite_report_path,
        &release_suite,
        release_suite_matches_sources,
        license_file_path,
        license_report_path,
        &license_report,
        license_report_matches_sources,
        margin_threshold_micro,
        simulated_calls,
        artifacts,
        &samples,
        margins,
    ))
}

fn build_action_cache_offload_bench_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
    license_report_path: &Path,
    margin_threshold_micro: i64,
    simulated_calls: usize,
) -> Result<PhaseActionCacheOffloadBenchReport, String> {
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let license_report = read_action_license_package_report(license_report_path)?;
    let rebuilt_license_report =
        build_action_license_package_report(release_suite_report_path, license_file_path)?;
    let license_report_matches_sources = license_report == rebuilt_license_report;

    let mut artifacts = Vec::with_capacity(release_suite.artifacts.len());
    let mut samples = Vec::new();
    for (artifact_index, artifact) in release_suite.artifacts.iter().enumerate() {
        let (offload_artifact, mut artifact_samples, _) =
            build_action_offload_artifact_report(artifact_index, artifact, margin_threshold_micro)?;
        artifacts.push(PhaseActionCacheOffloadArtifactReport {
            label: offload_artifact.label,
            unique_eval_rows: offload_artifact.unique_eval_rows,
            simulated_calls: 0,
            exact_cache_llm_calls: 0,
            exact_cache_hits: 0,
            exact_cache_hit_rate_milli: 0,
            nando_local_operator_calls: 0,
            nando_fallback_events: 0,
            nando_plus_cache_llm_calls: 0,
            nando_plus_cache_hits: 0,
            nando_operator_hit_rate_milli: 0,
            incremental_llm_calls_removed_vs_cache: 0,
            incremental_llm_call_reduction_vs_cache_milli: 0,
            local_accuracy_milli: 0,
            false_local_accepts: 0,
            package_fingerprint64: offload_artifact.package_fingerprint64,
            package_bytes: offload_artifact.package_bytes,
            eval_pack_bytes: offload_artifact.eval_pack_bytes,
            runtime_bytes_estimate: offload_artifact.runtime_bytes_estimate,
            release_artifact_gate_pass: offload_artifact.release_artifact_gate_pass,
            compiler_used: offload_artifact.compiler_used,
            eval_task_package_used: offload_artifact.eval_task_package_used,
            corpus_jsonl_used: offload_artifact.corpus_jsonl_used,
            python_demo_used: offload_artifact.python_demo_used,
            forbidden_used: offload_artifact.forbidden_used,
        });
        samples.append(&mut artifact_samples);
    }

    Ok(PhaseActionCacheOffloadBenchReport::from_inputs(
        release_suite_report_path,
        &release_suite,
        release_suite_matches_sources,
        license_file_path,
        license_report_path,
        &license_report,
        license_report_matches_sources,
        margin_threshold_micro,
        simulated_calls,
        artifacts,
        &samples,
    ))
}

fn build_action_workflow_bench_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
    license_report_path: &Path,
    cache_bench_report_path: &Path,
) -> Result<PhaseActionWorkflowBenchReport, String> {
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let license_report = read_action_license_package_report(license_report_path)?;
    let rebuilt_license_report =
        build_action_license_package_report(release_suite_report_path, license_file_path)?;
    let license_report_matches_sources = license_report == rebuilt_license_report;
    let cache_bench_report = read_action_cache_offload_bench_report(cache_bench_report_path)?;
    let rebuilt_cache_bench_report = build_action_cache_offload_bench_report(
        release_suite_report_path,
        license_file_path,
        license_report_path,
        cache_bench_report.margin_threshold_micro,
        cache_bench_report.simulated_calls,
    )?;
    let cache_bench_report_matches_sources = cache_bench_report == rebuilt_cache_bench_report;
    let release_domain_artifact = release_suite
        .artifacts
        .iter()
        .find(|artifact| artifact.label == "domain_action")
        .ok_or_else(|| String::from("release suite does not contain domain_action artifact"))?;
    let cache_domain_artifact = cache_bench_report
        .artifacts
        .iter()
        .find(|artifact| artifact.label == "domain_action")
        .ok_or_else(|| {
            String::from("cache bench report does not contain domain_action artifact")
        })?;
    Ok(PhaseActionWorkflowBenchReport::from_inputs(
        release_suite_report_path,
        &release_suite,
        release_suite_matches_sources,
        license_file_path,
        license_report_path,
        &license_report,
        license_report_matches_sources,
        cache_bench_report_path,
        &cache_bench_report,
        cache_bench_report_matches_sources,
        release_domain_artifact,
        cache_domain_artifact,
    ))
}

fn build_action_workflow_replay_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
    license_report_path: &Path,
    margin_threshold_micro: i64,
    workflow_sessions: usize,
    steps_per_session: usize,
) -> Result<PhaseActionWorkflowReplayReport, String> {
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let license_report = read_action_license_package_report(license_report_path)?;
    let rebuilt_license_report =
        build_action_license_package_report(release_suite_report_path, license_file_path)?;
    let license_report_matches_sources = license_report == rebuilt_license_report;

    let mut artifact_reports = Vec::with_capacity(release_suite.artifacts.len());
    let mut replay_artifacts = Vec::with_capacity(release_suite.artifacts.len());
    let mut samples_by_artifact =
        vec![Vec::<ActionOffloadSample>::new(); release_suite.artifacts.len()];

    for (artifact_index, artifact) in release_suite.artifacts.iter().enumerate() {
        let (offload_artifact, artifact_samples, _artifact_margins) =
            build_action_offload_artifact_report(artifact_index, artifact, margin_threshold_micro)?;
        let replay_artifact = PhaseActionWorkflowReplayArtifactReport {
            label: offload_artifact.label.clone(),
            package_fingerprint64: offload_artifact.package_fingerprint64,
            package_bytes: offload_artifact.package_bytes,
            eval_pack_bytes: offload_artifact.eval_pack_bytes,
            runtime_bytes_estimate: offload_artifact.runtime_bytes_estimate,
            unique_eval_rows: offload_artifact.unique_eval_rows,
            trace_calls: 0,
            unique_replayed_rows: 0,
            exact_cache_llm_calls: 0,
            exact_cache_hits: 0,
            nando_plus_cache_llm_calls: 0,
            nando_plus_cache_hits: 0,
            local_operator_calls: 0,
            fallback_to_llm_calls: 0,
            local_accuracy_milli: 0,
            false_local_accepts: 0,
            release_artifact_gate_pass: offload_artifact.release_artifact_gate_pass,
            product_verify_pass: offload_artifact.product_verify_pass,
            score_accuracy_milli: offload_artifact.score_accuracy_milli,
            score_wrong_wins: offload_artifact.score_wrong_wins,
            compiler_used: offload_artifact.compiler_used,
            eval_task_package_used: offload_artifact.eval_task_package_used,
            corpus_jsonl_used: offload_artifact.corpus_jsonl_used,
            python_demo_used: offload_artifact.python_demo_used,
            forbidden_used: offload_artifact.forbidden_used,
        };
        if let Some(slot) = samples_by_artifact.get_mut(artifact_index) {
            *slot = artifact_samples;
        }
        artifact_reports.push(offload_artifact);
        replay_artifacts.push(replay_artifact);
    }

    Ok(PhaseActionWorkflowReplayReport::from_inputs(
        release_suite_report_path,
        &release_suite,
        release_suite_matches_sources,
        license_file_path,
        license_report_path,
        &license_report,
        license_report_matches_sources,
        margin_threshold_micro,
        workflow_sessions,
        steps_per_session,
        replay_artifacts,
        &artifact_reports,
        &samples_by_artifact,
    ))
}

fn build_action_regression_report(
    release_suite_report_path: &Path,
    license_file_path: &Path,
    license_report_path: &Path,
    offload_report_path: &Path,
    cache_bench_report_path: &Path,
    workflow_bench_report_path: &Path,
    workflow_replay_report_path: &Path,
) -> Result<PhaseActionRegressionReport, String> {
    let (release_suite_report_fingerprint64, release_suite_report_bytes) =
        inspect_report_file(release_suite_report_path)?;
    let release_suite = read_action_release_suite_report(release_suite_report_path)?;
    let rebuilt_release_suite = rebuild_action_release_suite_report(&release_suite)?;
    let release_suite_matches_sources = release_suite == rebuilt_release_suite;
    let (license_package_report_fingerprint64, license_package_report_bytes) =
        inspect_report_file(license_report_path)?;
    let license_report = read_action_license_package_report(license_report_path)?;
    let rebuilt_license_report =
        build_action_license_package_report(release_suite_report_path, license_file_path)?;
    let license_report_matches_sources = license_report == rebuilt_license_report;
    let (offload_audit_report_fingerprint64, offload_audit_report_bytes) =
        inspect_report_file(offload_report_path)?;
    let offload_report = read_action_offload_audit_report(offload_report_path)?;
    let rebuilt_offload_report = build_action_offload_audit_report(
        release_suite_report_path,
        license_file_path,
        license_report_path,
        offload_report.margin_threshold_micro,
        offload_report.simulated_calls,
    )?;
    let offload_report_matches_sources = offload_report == rebuilt_offload_report;
    let (cache_bench_report_fingerprint64, cache_bench_report_bytes) =
        inspect_report_file(cache_bench_report_path)?;
    let cache_bench_report = read_action_cache_offload_bench_report(cache_bench_report_path)?;
    let rebuilt_cache_bench_report = build_action_cache_offload_bench_report(
        release_suite_report_path,
        license_file_path,
        license_report_path,
        cache_bench_report.margin_threshold_micro,
        cache_bench_report.simulated_calls,
    )?;
    let cache_bench_report_matches_sources = cache_bench_report == rebuilt_cache_bench_report;
    let (workflow_bench_report_fingerprint64, workflow_bench_report_bytes) =
        inspect_report_file(workflow_bench_report_path)?;
    let workflow_bench_report = read_action_workflow_bench_report(workflow_bench_report_path)?;
    let rebuilt_workflow_bench_report = build_action_workflow_bench_report(
        release_suite_report_path,
        license_file_path,
        license_report_path,
        cache_bench_report_path,
    )?;
    let workflow_bench_report_matches_sources =
        workflow_bench_report == rebuilt_workflow_bench_report;
    let (workflow_replay_report_fingerprint64, workflow_replay_report_bytes) =
        inspect_report_file(workflow_replay_report_path)?;
    let workflow_replay_report = read_action_workflow_replay_report(workflow_replay_report_path)?;
    let rebuilt_workflow_replay_report = build_action_workflow_replay_report(
        release_suite_report_path,
        license_file_path,
        license_report_path,
        workflow_replay_report.margin_threshold_micro,
        workflow_replay_report.workflow_sessions,
        workflow_replay_report.steps_per_session,
    )?;
    let workflow_replay_report_matches_sources =
        workflow_replay_report == rebuilt_workflow_replay_report;
    let operator_blueprint =
        inspect_operator_blueprint_contract(Path::new(DEFAULT_OPERATOR_BLUEPRINT))?;

    Ok(PhaseActionRegressionReport::from_inputs(
        PhaseActionRegressionReportInput {
            release_suite_report_path,
            release_suite_report_fingerprint64,
            release_suite_report_bytes,
            release_suite: &release_suite,
            release_suite_matches_sources,
            license_file_path,
            license_package_report_path: license_report_path,
            license_package_report_fingerprint64,
            license_package_report_bytes,
            license_report: &license_report,
            license_report_matches_sources,
            offload_audit_report_path: offload_report_path,
            offload_audit_report_fingerprint64,
            offload_audit_report_bytes,
            offload_report: &offload_report,
            offload_report_matches_sources,
            cache_bench_report_path,
            cache_bench_report_fingerprint64,
            cache_bench_report_bytes,
            cache_bench_report: &cache_bench_report,
            cache_bench_report_matches_sources,
            workflow_bench_report_path,
            workflow_bench_report_fingerprint64,
            workflow_bench_report_bytes,
            workflow_bench_report: &workflow_bench_report,
            workflow_bench_report_matches_sources,
            workflow_replay_report_path,
            workflow_replay_report_fingerprint64,
            workflow_replay_report_bytes,
            workflow_replay_report: &workflow_replay_report,
            workflow_replay_report_matches_sources,
            operator_blueprint: &operator_blueprint,
        },
    ))
}

fn build_action_regression_freeze_report(
    config: &PhaseActionRegressionFreezeConfig,
) -> Result<PhaseActionRegressionFreezeReport, String> {
    let regression_report_path = &config.regression_report_path;
    let (regression_report_fingerprint64, regression_report_bytes) =
        inspect_report_file(regression_report_path)?;
    let regression_report = read_action_regression_report(regression_report_path)?;
    let rebuilt_regression_report = build_action_regression_report(
        &config.release_suite_report_path,
        &config.license_file_path,
        &config.license_report_path,
        &config.offload_report_path,
        &config.cache_bench_report_path,
        &config.workflow_bench_report_path,
        &config.workflow_replay_report_path,
    )?;
    let regression_matches_sources = regression_report == rebuilt_regression_report;
    Ok(PhaseActionRegressionFreezeReport::from_regression(
        regression_report_path,
        regression_report_fingerprint64,
        regression_report_bytes,
        &regression_report,
        regression_matches_sources,
    ))
}

fn build_action_offload_artifact_report(
    artifact_index: usize,
    release_artifact: &PhaseActionReleaseSuiteArtifactReport,
    margin_threshold_micro: i64,
) -> Result<
    (
        PhaseActionOffloadArtifactReport,
        Vec<ActionOffloadSample>,
        Vec<i64>,
    ),
    String,
> {
    let paths = action_product_bundle_paths_from_artifact(release_artifact);
    let source_artifact = build_action_release_suite_artifact_report(&paths)?;
    let package_bytes = std::fs::read(&paths.package_path)
        .map_err(|error| format!("failed to read '{}': {error}", paths.package_path.display()))?;
    let eval_package = read_eval_task_package(&paths.eval_pack_path)?;
    let policy =
        PhaseCenterOffloadPolicy::new(margin_threshold_micro).map_err(format_runtime_error)?;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(format_runtime_error)?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(&package_bytes, policy)
        .map_err(format_runtime_error)?;
    if offload_runtime.package_info() != package_info {
        return Err(String::from(
            "offload runtime package info mismatch after SDK inspect",
        ));
    }

    let mut decisions = Vec::with_capacity(eval_package.prepared.tasks.len());
    let mut unique_margin_scratch = Vec::with_capacity(eval_package.prepared.tasks.len());
    let unique_summary = offload_runtime
        .offload_summary_into(
            eval_package
                .prepared
                .tasks
                .iter()
                .map(|prepared| &prepared.task),
            &mut decisions,
            &mut unique_margin_scratch,
        )
        .map_err(format_runtime_error)?;
    let mut samples = Vec::with_capacity(decisions.len());
    let mut margins = Vec::with_capacity(decisions.len());
    for (row_index, decision) in decisions.into_iter().enumerate() {
        samples.push(ActionOffloadSample {
            artifact_index,
            row_index,
            decision,
        });
        margins.push(decision.margin_micro);
    }

    let unique_eval_rows = unique_summary.calls;
    let unique_local_operator_rows = unique_summary.local_operator_calls;
    let unique_fallback_rows = unique_summary.fallback_to_llm_calls;
    let corpus_jsonl_used = source_artifact.corpus_jsonl_used_in_score_loop
        || source_artifact.corpus_jsonl_used_in_bench_loop;
    let release_artifact_gate_pass = source_artifact.gate_pass();
    let forbidden_used = source_artifact.forbidden_used();
    let report = PhaseActionOffloadArtifactReport {
        label: source_artifact.label,
        package_path: source_artifact.package_path,
        eval_task_package_path: source_artifact.eval_task_package_path,
        package_fingerprint64: source_artifact.package_fingerprint64,
        package_bytes: source_artifact.package_bytes,
        sdk_inspected_cells: package_info.cells,
        sdk_inspected_record_count: package_info.record_count,
        sdk_inspected_serialized_len: package_info.serialized_len,
        sdk_inspected_payload_bytes: package_info.payload_bytes,
        sdk_inspected_fingerprint64: package_info.fingerprint64,
        sdk_inspect_matches_package: package_info.fingerprint64
            == source_artifact.package_fingerprint64
            && package_info.serialized_len == source_artifact.package_bytes,
        sdk_inspect_matches_eval_pack: package_info.fingerprint64
            == eval_package.package_fingerprint64,
        eval_pack_bytes: source_artifact.eval_pack_bytes,
        runtime_bytes_estimate: source_artifact.runtime_bytes_estimate,
        unique_eval_rows,
        unique_local_operator_rows,
        unique_fallback_rows,
        unique_offload_rate_milli: unique_summary.offload_rate_milli,
        unique_local_accuracy_milli: unique_summary.local_accuracy_milli,
        unique_false_local_accepts: unique_summary.false_local_accepts,
        median_margin_micro: unique_summary.median_margin_micro,
        p10_margin_micro: unique_summary.p10_margin_micro,
        simulated_calls: 0,
        simulated_local_operator_calls: 0,
        simulated_fallback_to_llm_calls: 0,
        simulated_offload_rate_milli: 0,
        simulated_local_accuracy_milli: 0,
        simulated_false_local_accepts: 0,
        release_artifact_gate_pass,
        product_verify_pass: source_artifact.product_verify_pass,
        score_accuracy_milli: source_artifact.score_accuracy_milli,
        score_wrong_wins: source_artifact.score_wrong_wins,
        compiler_used: source_artifact.compiler_used,
        eval_task_package_used: source_artifact.eval_task_package_used,
        corpus_jsonl_used,
        python_demo_used: source_artifact.python_demo_used,
        forbidden_used,
    };
    Ok((report, samples, margins))
}

fn build_strict_multiseed_rust_audit_report(
    diagnostics_root_path: &Path,
) -> Result<StrictMultiSeedRustAuditReport, String> {
    let expected_seeds = vec![1_u8, 2, 3];
    let expected_classes = ["order", "edit", "conditional", "composed"]
        .iter()
        .map(|class| (*class).to_string())
        .collect::<Vec<_>>();
    let mut log_reports = Vec::new();
    let mut missing_logs = Vec::new();
    let mut strict_runtime_issues = Vec::new();
    let mut evidence_warnings = Vec::new();
    let mut fingerprint_input = Vec::new();
    let mut logs_total_bytes = 0usize;

    for seed in &expected_seeds {
        for operator_class in &expected_classes {
            let log_path =
                strict_multiseed_runtime_log_path(diagnostics_root_path, *seed, operator_class);
            if !log_path.exists() {
                let missing = format!(
                    "missing log seed={} class={} path={}",
                    seed,
                    operator_class,
                    log_path.display()
                );
                missing_logs.push(missing.clone());
                strict_runtime_issues.push(missing);
                continue;
            }

            let log_report = parse_strict_multiseed_runtime_log(*seed, operator_class, &log_path)?;
            logs_total_bytes += log_report.log_bytes;
            fingerprint_input.extend_from_slice(log_report.log_path.as_bytes());
            fingerprint_input.extend_from_slice(&log_report.log_fingerprint64.to_le_bytes());
            fingerprint_input.extend_from_slice(&log_report.log_bytes.to_le_bytes());
            for issue in &log_report.issues {
                strict_runtime_issues
                    .push(format!("seed={} class={}: {}", seed, operator_class, issue));
            }
            for warning in &log_report.evidence_warnings {
                evidence_warnings.push(format!(
                    "seed={} class={}: {}",
                    seed, operator_class, warning
                ));
            }
            log_reports.push(log_report);
        }
    }

    let target_center_id_training_used = log_reports
        .iter()
        .any(|report| report.target_center_id_training_used == Some(true));
    let proof_rule_id_training_authority_used = log_reports
        .iter()
        .any(|report| report.proof_rule_id_training_authority_used == Some(true));
    let concrete_x_lookup_used = log_reports
        .iter()
        .any(|report| report.concrete_x_lookup_used == Some(true));
    let local_out_t_runtime_extension_used = log_reports
        .iter()
        .any(|report| report.local_out_t_runtime_extension_used == Some(true));
    if target_center_id_training_used {
        strict_runtime_issues.push(String::from("target_center_id_training_used=true"));
    }
    if proof_rule_id_training_authority_used {
        strict_runtime_issues.push(String::from("proof_rule_id_training_authority_used=true"));
    }
    if concrete_x_lookup_used {
        strict_runtime_issues.push(String::from("concrete_x_lookup_used=true"));
    }
    if local_out_t_runtime_extension_used {
        strict_runtime_issues.push(String::from("local_out_t_runtime_extension_used=true"));
    }

    let observed_logs = log_reports.len();
    let gate_pass = observed_logs == expected_seeds.len() * expected_classes.len()
        && missing_logs.is_empty()
        && strict_runtime_issues.is_empty()
        && evidence_warnings.is_empty()
        && !target_center_id_training_used
        && !proof_rule_id_training_authority_used
        && !concrete_x_lookup_used
        && !local_out_t_runtime_extension_used;
    let verdict = if gate_pass {
        "STRICT_MULTI_SEED_RUST_AUDIT_PASS"
    } else if strict_runtime_issues.is_empty() {
        "STRICT_MULTI_SEED_RUST_AUDIT_WATCH"
    } else {
        "STRICT_MULTI_SEED_RUST_AUDIT_RED"
    }
    .to_string();

    Ok(StrictMultiSeedRustAuditReport {
        schema_version: "strict_multiseed_rust_audit_report_v1".to_string(),
        audit_kind: STRICT_MULTI_SEED_AUDIT_KIND.to_string(),
        verdict,
        gate_pass,
        diagnostics_root_path: diagnostics_root_path.display().to_string(),
        expected_seeds,
        expected_classes,
        observed_logs,
        missing_logs,
        strict_runtime_issues,
        evidence_warnings,
        logs_fingerprint64: file_fingerprint64(&fingerprint_input),
        logs_total_bytes,
        log_reports,
        target_center_id_training_used,
        proof_rule_id_training_authority_used,
        concrete_x_lookup_used,
        local_out_t_runtime_extension_used,
        python_demo_used: false,
        corpus_jsonl_used: false,
        rust_runtime_logs_used: true,
        claim_boundary:
            "Rust log audit of v4 strict multi-seed robustness over canonical release logs; not Python demo authority, not corpus JSONL authority, and not a broad product claim"
                .to_string(),
    })
}

fn strict_multiseed_runtime_log_path(
    diagnostics_root_path: &Path,
    seed: u8,
    operator_class: &str,
) -> PathBuf {
    let class_dir = diagnostics_root_path
        .join(format!("seed_{seed:03}"))
        .join(operator_class);
    let release_path = class_dir.join(format!("{operator_class}_runtime_gate_release.log"));
    if release_path.exists() {
        return release_path;
    }
    let strict_red_path = class_dir.join(format!(
        "{operator_class}_runtime_gate_release_strict_red_repro.log"
    ));
    if strict_red_path.exists() {
        strict_red_path
    } else {
        release_path
    }
}

fn parse_strict_multiseed_runtime_log(
    seed: u8,
    operator_class: &str,
    log_path: &Path,
) -> Result<StrictMultiSeedRuntimeLogReport, String> {
    let bytes = std::fs::read(log_path)
        .map_err(|error| format!("failed to read '{}': {error}", log_path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        format!(
            "strict multiseed log '{}' is not UTF-8: {error}",
            log_path.display()
        )
    })?;
    let metrics = strict_log_metrics(text);
    let test_result_failed = text.contains("test result: FAILED") || text.contains("panicked at ");
    let test_result_ok = !test_result_failed && text.contains("test result: ok");

    let slot_accuracy_milli = strict_metric_usize(
        &metrics,
        &[&format!(
            "{operator_class}_slot_ordered_sequence_accuracy_milli"
        )],
    );
    let flat_slot_accuracy_milli = strict_metric_usize(
        &metrics,
        &[&format!(
            "{operator_class}_flat_slot_ordered_sequence_accuracy_milli"
        )],
    );
    let sequence_energy_accuracy_milli = strict_metric_usize(
        &metrics,
        &[&format!("{operator_class}_sequence_energy_accuracy_milli")],
    );
    let energy_pass_slot_fail = strict_metric_usize(
        &metrics,
        &[&format!("{operator_class}_energy_pass_slot_fail")],
    );
    let output_slot_cleanup_failed_slots = strict_metric_usize(
        &metrics,
        &[&format!(
            "{operator_class}_output_slot_cleanup_failed_slots"
        )],
    );
    let slot_failure_total = strict_metric_usize(
        &metrics,
        &[
            &format!("{operator_class}_slot_failure_total"),
            "slot_failure_total",
        ],
    );
    let flat_gap_parity_mismatches = strict_metric_usize(&metrics, &["flat_gap_parity_mismatches"]);
    let flat_sequence_energy_parity_mismatches =
        strict_metric_usize(&metrics, &["flat_sequence_energy_parity_mismatches"]);
    let state_delta_edges = strict_metric_usize(&metrics, &["state_delta_edges"]);
    let role_binding_edges = strict_metric_usize(&metrics, &["role_binding_edges"]);
    let target_center_id_training_used =
        strict_metric_bool(&metrics, &["target_center_id_training_used"]);
    let proof_rule_id_training_authority_used =
        strict_metric_bool(&metrics, &["proof_rule_id_training_authority_used"]);
    let concrete_x_lookup_used = strict_metric_bool(&metrics, &["concrete_x_lookup_used"]);
    let local_out_t_runtime_extension_used =
        strict_metric_bool(&metrics, &["local_out_t_runtime_extension_used"]);

    let mut issues = Vec::new();
    let mut evidence_warnings = Vec::new();
    if !test_result_ok {
        issues.push(String::from(
            "rust test did not finish with test result: ok",
        ));
    }
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "slot_accuracy_milli",
        slot_accuracy_milli,
        1000,
    );
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "sequence_energy_accuracy_milli",
        sequence_energy_accuracy_milli,
        1000,
    );
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "flat_gap_parity_mismatches",
        flat_gap_parity_mismatches,
        0,
    );
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "flat_sequence_energy_parity_mismatches",
        flat_sequence_energy_parity_mismatches,
        0,
    );
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "state_delta_edges",
        state_delta_edges,
        0,
    );
    strict_require_metric_eq(
        &mut issues,
        &mut evidence_warnings,
        "energy_pass_slot_fail",
        energy_pass_slot_fail,
        0,
    );
    if let Some(failed_slots) = output_slot_cleanup_failed_slots {
        if failed_slots != 0 {
            issues.push(format!(
                "output_slot_cleanup_failed_slots expected 0 got {failed_slots}"
            ));
        }
    } else {
        evidence_warnings.push(String::from(
            "output_slot_cleanup_failed_slots metric missing",
        ));
    }
    if let Some(failure_total) = slot_failure_total
        && failure_total != 0
    {
        issues.push(format!("slot_failure_total expected 0 got {failure_total}"));
    }
    strict_require_flag_false(
        &mut issues,
        &mut evidence_warnings,
        "target_center_id_training_used",
        target_center_id_training_used,
    );
    strict_require_flag_false(
        &mut issues,
        &mut evidence_warnings,
        "proof_rule_id_training_authority_used",
        proof_rule_id_training_authority_used,
    );
    strict_require_flag_false(
        &mut issues,
        &mut evidence_warnings,
        "concrete_x_lookup_used",
        concrete_x_lookup_used,
    );
    strict_require_flag_false(
        &mut issues,
        &mut evidence_warnings,
        "local_out_t_runtime_extension_used",
        local_out_t_runtime_extension_used,
    );
    if flat_slot_accuracy_milli != slot_accuracy_milli {
        issues.push(format!(
            "flat slot accuracy mismatch field={slot_accuracy_milli:?} flat={flat_slot_accuracy_milli:?}"
        ));
    }
    if role_binding_edges.is_none() {
        evidence_warnings.push(String::from("role_binding_edges metric missing"));
    }

    Ok(StrictMultiSeedRuntimeLogReport {
        seed,
        operator_class: operator_class.to_string(),
        log_path: log_path.display().to_string(),
        log_fingerprint64: file_fingerprint64(&bytes),
        log_bytes: bytes.len(),
        test_result_ok,
        test_result_failed,
        slot_accuracy_milli,
        flat_slot_accuracy_milli,
        sequence_energy_accuracy_milli,
        energy_pass_slot_fail,
        output_slot_cleanup_failed_slots,
        slot_failure_total,
        flat_gap_parity_mismatches,
        flat_sequence_energy_parity_mismatches,
        state_delta_edges,
        role_binding_edges,
        target_center_id_training_used,
        proof_rule_id_training_authority_used,
        concrete_x_lookup_used,
        local_out_t_runtime_extension_used,
        issues,
        evidence_warnings,
    })
}

fn strict_log_metrics(text: &str) -> BTreeMap<String, String> {
    let mut metrics = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = strict_key_value(trimmed) {
            metrics.insert(key, value);
        }
        if let Some((_, rest)) = trimmed.split_once(": ")
            && let Some((key, value)) = strict_key_value(rest.trim())
        {
            metrics.insert(key, value);
        }
    }
    metrics
}

fn strict_key_value(text: &str) -> Option<(String, String)> {
    let (key, value) = text.split_once(": ").or_else(|| text.split_once('='))?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let value = value
        .trim()
        .trim_end_matches(',')
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    Some((key.to_string(), value))
}

fn strict_metric_usize(metrics: &BTreeMap<String, String>, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|key| metrics.get(*key).and_then(|value| value.parse().ok()))
}

fn strict_metric_bool(metrics: &BTreeMap<String, String>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| metrics.get(*key).and_then(|value| value.parse().ok()))
}

fn strict_require_metric_eq(
    issues: &mut Vec<String>,
    evidence_warnings: &mut Vec<String>,
    name: &str,
    value: Option<usize>,
    expected: usize,
) {
    match value {
        Some(actual) if actual == expected => {}
        Some(actual) => issues.push(format!("{name} expected {expected} got {actual}")),
        None => evidence_warnings.push(format!("{name} metric missing")),
    }
}

fn strict_require_flag_false(
    issues: &mut Vec<String>,
    evidence_warnings: &mut Vec<String>,
    name: &str,
    value: Option<bool>,
) {
    match value {
        Some(false) => {}
        Some(true) => issues.push(format!("{name}=true")),
        None => evidence_warnings.push(format!("{name} flag missing")),
    }
}

fn strict_multiseed_rust_audit_verify_v1_verdict(report_matches_sources: bool) -> &'static str {
    if report_matches_sources {
        "STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS"
    } else {
        "STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_WATCH"
    }
}

fn validate_eval_task_package_match(
    eval_package: &PhaseEvalTaskPackage,
    manifest: &PhasePackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
) -> Result<(), String> {
    let matches = eval_package.cells == manifest.cells
        && eval_package.cells == package_info.cells
        && eval_package.package_fingerprint64 == package_info.fingerprint64
        && eval_package.rows == manifest.rows;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase eval task package does not match package/manifest",
        ))
    }
}

fn validate_action_eval_task_package_match(
    eval_package: &PhaseEvalTaskPackage,
    manifest: &PhaseActionPackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
) -> Result<(), String> {
    let matches = eval_package.cells == manifest.cells
        && eval_package.cells == package_info.cells
        && eval_package.package_fingerprint64 == package_info.fingerprint64
        && eval_package.rows == manifest.rows;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase action eval task package does not match package/manifest",
        ))
    }
}

fn validate_score_report_match(
    report: &PhasePackageScoreReport,
    manifest: &PhasePackageManifest,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    package_bytes_len: usize,
    runtime_bytes_estimate: usize,
) -> Result<(), String> {
    let matches = report.schema_version == "nando_phase_package_score_report_v1"
        && report.package_kind == manifest.package_kind
        && report.cells == manifest.cells
        && report.flat_records == package_info.record_count
        && report.manifest_operator_keys == manifest.operator_keys.len()
        && report.package_fingerprint64 == package_info.fingerprint64
        && report.package_bytes == package_bytes_len
        && report.inspected_payload_bytes == package_info.payload_bytes
        && report.runtime_bytes_estimate == runtime_bytes_estimate
        && report.claim_boundary == manifest.claim_boundary
        && report.license_boundary == manifest.license_boundary;
    if matches {
        Ok(())
    } else {
        Err(String::from(
            "phase package score report does not match package/manifest",
        ))
    }
}

fn score_report_gate_pass(report: &PhasePackageScoreReport) -> bool {
    score_report_verdict_gate_pass(report)
        && report.accuracy_milli == 1000
        && report.wrong_wins == 0
        && report.missing_centers == 0
        && report.skipped_rows == 0
        && report.action_ablation_missing_centers == 0
        && report.action_ablation_eval_rows > 0
        && report.action_ablation_accuracy_milli < report.accuracy_milli
        && report.action_ablation_wrong_wins > 0
        && !report.compiler_used
        && !report.forbidden_flags.any_forbidden_used()
}

fn score_report_verdict_gate_pass(report: &PhasePackageScoreReport) -> bool {
    match report.verdict.as_str() {
        "PHASE_PACKAGE_SCORE_V4_PASS" => true,
        "PHASE_PACKAGE_SCORE_PACK_V4_PASS" => {
            report.eval_task_package_used
                && !report.eval_task_package_path.is_empty()
                && report.corpus_jsonl_used_in_score_loop == Some(false)
        }
        _ => false,
    }
}

fn action_eval_pack_v1_gate_pass(
    eval_package: &PhaseEvalTaskPackage,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    manifest: &PhaseActionPackageManifest,
    contract_verdict: &str,
) -> bool {
    validate_action_eval_task_package_match(eval_package, manifest, package_info).is_ok()
        && contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
        && manifest.gate_pass()
        && eval_package.prepared.missing_centers == 0
        && eval_package.prepared.skipped_rows == 0
        && eval_package.prepared.action_ablation_missing_centers == 0
        && !eval_package.prepared.tasks.is_empty()
        && !eval_package.prepared.action_ablation_tasks.is_empty()
        && !manifest_forbidden_used(manifest)
}

fn action_score_report_verdict_gate_pass(report: &PhaseActionPackageScoreReport) -> bool {
    match report.verdict.as_str() {
        "PHASE_ACTION_PACKAGE_SCORE_V1_PASS" => {
            !report.eval_task_package_used
                && report.eval_task_package_path.is_empty()
                && report.corpus_jsonl_used_in_score_loop != Some(true)
        }
        "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS" => {
            report.eval_task_package_used
                && !report.eval_task_package_path.is_empty()
                && report.corpus_jsonl_used_in_score_loop == Some(false)
        }
        _ => false,
    }
}

const fn manifest_forbidden_used(manifest: &PhaseActionPackageManifest) -> bool {
    manifest.python_demo_used
        || manifest.target_center_id_training_used
        || manifest.proof_rule_id_training_authority_used
        || manifest.concrete_x_lookup_used
        || manifest.local_out_t_runtime_extension_used
}

fn format_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn manifest_key_to_index(
    manifest: &PhasePackageManifest,
) -> Result<BTreeMap<String, usize>, String> {
    let mut key_to_index = BTreeMap::new();
    for (index, key) in manifest.operator_keys.iter().enumerate() {
        if key.is_empty() {
            return Err(format!("manifest operator key at index {index} is empty"));
        }
        if key_to_index.insert(key.clone(), index).is_some() {
            return Err(format!("manifest operator key is duplicated: {key}"));
        }
    }
    if key_to_index.len() != manifest.flat_records {
        return Err(format!(
            "manifest operator key count {} does not match flat_records {}",
            key_to_index.len(),
            manifest.flat_records
        ));
    }
    Ok(key_to_index)
}

fn action_manifest_key_to_index(
    manifest: &PhaseActionPackageManifest,
) -> Result<BTreeMap<String, usize>, String> {
    let mut key_to_index = BTreeMap::new();
    for (index, key) in manifest.operator_keys.iter().enumerate() {
        if key.is_empty() {
            return Err(format!(
                "action manifest operator key at index {index} is empty"
            ));
        }
        if key_to_index.insert(key.clone(), index).is_some() {
            return Err(format!("action manifest operator key is duplicated: {key}"));
        }
    }
    if key_to_index.len() != manifest.flat_records {
        return Err(format!(
            "action manifest operator key count {} does not match flat_records {}",
            key_to_index.len(),
            manifest.flat_records
        ));
    }
    Ok(key_to_index)
}

fn prepare_eval_tasks(
    rows: &[PhaseOperatorRow],
    cells: usize,
    key_to_index: &BTreeMap<String, usize>,
) -> PreparedEval {
    let mut tasks = Vec::new();
    let mut action_ablation_tasks = Vec::new();
    let mut missing_centers = 0usize;
    let mut skipped_rows = 0usize;
    let mut action_ablation_missing_centers = 0usize;
    let mut heldout_surface_groups = BTreeSet::new();
    let mut heldout_noise_groups = BTreeSet::new();
    let action_ablation_buckets = action_ablation_buckets(rows, key_to_index);

    for row in rows
        .iter()
        .filter(|row| phase_split(row) == Some("heldout"))
    {
        heldout_surface_groups.insert(row.surface_family.as_str());
        heldout_noise_groups.insert(row.noise_type.as_str());
        let key = phase_operator_key(row);
        let Some(center_index) = key_to_index.get(&key).copied() else {
            missing_centers += 1;
            continue;
        };
        let Some(correct_atoms) = phase_transition_atoms(row, &row.correct_tokens) else {
            skipped_rows += 1;
            continue;
        };
        let Some(wrong_atoms) = phase_transition_atoms(row, &row.wrong_tokens) else {
            skipped_rows += 1;
            continue;
        };
        tasks.push(PreparedTask {
            task: PhaseCenterEvalTask {
                center_index,
                correct_vec: phase_vector_from_atoms(
                    correct_atoms.iter().map(String::as_str),
                    cells,
                )
                .into_boxed_slice(),
                wrong_vec: phase_vector_from_atoms(wrong_atoms.iter().map(String::as_str), cells)
                    .into_boxed_slice(),
            },
        });
        let bucket = phase_operator_bucket_key(row);
        let Some(ablation_center_index) = action_ablation_buckets
            .get(&bucket)
            .and_then(|indices| indices.iter().copied().find(|index| *index != center_index))
        else {
            action_ablation_missing_centers += 1;
            continue;
        };
        action_ablation_tasks.push(PreparedTask {
            task: PhaseCenterEvalTask {
                center_index: ablation_center_index,
                correct_vec: phase_vector_from_atoms(
                    correct_atoms.iter().map(String::as_str),
                    cells,
                )
                .into_boxed_slice(),
                wrong_vec: phase_vector_from_atoms(wrong_atoms.iter().map(String::as_str), cells)
                    .into_boxed_slice(),
            },
        });
    }

    PreparedEval {
        tasks,
        action_ablation_tasks,
        missing_centers,
        skipped_rows,
        action_ablation_missing_centers,
        heldout_surface_groups: heldout_surface_groups.len(),
        heldout_noise_groups: heldout_noise_groups.len(),
    }
}

fn action_ablation_buckets(
    rows: &[PhaseOperatorRow],
    key_to_index: &BTreeMap<String, usize>,
) -> BTreeMap<String, BTreeSet<usize>> {
    let mut buckets = BTreeMap::<String, BTreeSet<usize>>::new();
    for row in rows {
        let key = phase_operator_key(row);
        if let Some(index) = key_to_index.get(&key).copied() {
            buckets
                .entry(phase_operator_bucket_key(row))
                .or_default()
                .insert(index);
        }
    }
    buckets
}

fn prepare_action_contract_eval(
    rows: &[PhaseActionContractRow],
    cells: usize,
    key_to_index: &BTreeMap<String, usize>,
) -> PreparedEval {
    let mut tasks = Vec::new();
    let mut action_ablation_tasks = Vec::new();
    let mut missing_centers = 0usize;
    let mut skipped_rows = 0usize;
    let mut action_ablation_missing_centers = 0usize;

    for row in rows.iter().filter(|row| row.split == "heldout") {
        let key = action_contract_key(row);
        let Some(center_index) = key_to_index.get(&key).copied() else {
            missing_centers += 1;
            continue;
        };
        let Some(correct_atoms) = action_contract_transition_atoms(row, &row.state_after_correct)
        else {
            skipped_rows += 1;
            continue;
        };
        let Some(wrong_atoms) = action_contract_transition_atoms(row, &row.state_after_wrong)
        else {
            skipped_rows += 1;
            continue;
        };
        let correct_vec = phase_vector_from_atoms(correct_atoms.iter().map(String::as_str), cells)
            .into_boxed_slice();
        let wrong_vec = phase_vector_from_atoms(wrong_atoms.iter().map(String::as_str), cells)
            .into_boxed_slice();
        tasks.push(PreparedTask {
            task: PhaseCenterEvalTask {
                center_index,
                correct_vec: correct_vec.clone(),
                wrong_vec: wrong_vec.clone(),
            },
        });

        let mut competing_centers = 0usize;
        for ablation_center_index in key_to_index
            .values()
            .copied()
            .filter(|index| *index != center_index)
        {
            competing_centers += 1;
            action_ablation_tasks.push(PreparedTask {
                task: PhaseCenterEvalTask {
                    center_index: ablation_center_index,
                    correct_vec: correct_vec.clone(),
                    wrong_vec: wrong_vec.clone(),
                },
            });
        }
        if competing_centers == 0 {
            action_ablation_missing_centers += 1;
        }
    }

    PreparedEval {
        tasks,
        action_ablation_tasks,
        missing_centers,
        skipped_rows,
        action_ablation_missing_centers,
        heldout_surface_groups: 0,
        heldout_noise_groups: 0,
    }
}

fn eval_loaded_runtime(
    runtime: &PhaseCenterFlatRuntime,
    tasks: &[PreparedTask],
) -> Result<RuntimeEval, String> {
    if tasks.is_empty() {
        return Err(String::from("no heldout eval tasks prepared"));
    }
    let mut margins = Vec::with_capacity(tasks.len());
    let mut latencies = Vec::with_capacity(tasks.len());
    let mut correct = 0usize;
    let total_start = Instant::now();
    for prepared in tasks {
        let start = Instant::now();
        let margin = runtime
            .margin(&prepared.task)
            .map_err(format_runtime_error)?;
        latencies.push(start.elapsed().as_nanos());
        margins.push(margin);
        correct += usize::from(margin > 0.0);
    }
    let total_eval_us = total_start.elapsed().as_micros();
    margins.sort_by(f64::total_cmp);
    latencies.sort_unstable();
    let seconds = total_start.elapsed().as_secs_f64();
    Ok(RuntimeEval {
        accuracy_milli: milli_ratio(correct, tasks.len()),
        wrong_wins: tasks.len().saturating_sub(correct),
        median_margin: percentile_f64(&margins, 50),
        p10_margin: percentile_f64(&margins, 10),
        p50_latency_ns: percentile_u128(&latencies, 50),
        p99_latency_ns: percentile_u128(&latencies, 99),
        total_eval_us,
        rows_per_second: tasks.len() as f64 / seconds.max(f64::EPSILON),
    })
}

fn bench_loaded_runtime(
    runtime: &PhaseCenterFlatRuntime,
    tasks: &[PreparedTask],
    iterations: usize,
) -> Result<RuntimeEval, String> {
    if iterations == 0 {
        return Err(String::from(
            "benchmark iterations must be greater than zero",
        ));
    }
    if tasks.is_empty() {
        return Err(String::from("no heldout eval tasks prepared"));
    }
    let sample_count = tasks
        .len()
        .checked_mul(iterations)
        .ok_or_else(|| String::from("benchmark sample count overflow"))?;
    let mut margins = Vec::with_capacity(sample_count);
    let mut latencies = Vec::with_capacity(sample_count);
    let mut correct = 0usize;
    let total_start = Instant::now();
    for _ in 0..iterations {
        for prepared in tasks {
            let start = Instant::now();
            let margin = runtime
                .margin(&prepared.task)
                .map_err(format_runtime_error)?;
            latencies.push(start.elapsed().as_nanos());
            margins.push(margin);
            correct += usize::from(margin > 0.0);
        }
    }
    let total_eval_us = total_start.elapsed().as_micros();
    margins.sort_by(f64::total_cmp);
    latencies.sort_unstable();
    let seconds = total_start.elapsed().as_secs_f64();
    Ok(RuntimeEval {
        accuracy_milli: milli_ratio(correct, sample_count),
        wrong_wins: sample_count.saturating_sub(correct),
        median_margin: percentile_f64(&margins, 50),
        p10_margin: percentile_f64(&margins, 10),
        p50_latency_ns: percentile_u128(&latencies, 50),
        p99_latency_ns: percentile_u128(&latencies, 99),
        total_eval_us,
        rows_per_second: sample_count as f64 / seconds.max(f64::EPSILON),
    })
}

fn eval_task_package_len(
    cells: usize,
    task_count: usize,
    action_task_count: usize,
) -> Option<usize> {
    let cells_bytes = cells
        .checked_mul(2)?
        .checked_mul(std::mem::size_of::<f64>())?;
    let task_bytes = std::mem::size_of::<u32>().checked_add(cells_bytes.checked_mul(2)?)?;
    PHASE_EVAL_TASK_PACKAGE_HEADER_BYTES
        .checked_add(task_bytes.checked_mul(task_count.checked_add(action_task_count)?)?)
}

fn write_eval_task_list(bytes: &mut Vec<u8>, tasks: &[PreparedTask]) -> Result<(), String> {
    for prepared in tasks {
        let center_index = u32::try_from(prepared.task.center_index)
            .map_err(|_| String::from("phase eval task center index exceeds u32"))?;
        bytes.extend_from_slice(&center_index.to_le_bytes());
        write_phase_cells(bytes, &prepared.task.correct_vec);
        write_phase_cells(bytes, &prepared.task.wrong_vec);
    }
    Ok(())
}

fn write_phase_cells(bytes: &mut Vec<u8>, cells: &[PhaseCenterCell]) {
    for cell in cells {
        bytes.extend_from_slice(&cell.re.to_le_bytes());
        bytes.extend_from_slice(&cell.im.to_le_bytes());
    }
}

fn read_eval_task_list(
    bytes: &[u8],
    offset: &mut usize,
    cells: usize,
    count: usize,
    path: &Path,
) -> Result<Vec<PreparedTask>, String> {
    let mut tasks = Vec::with_capacity(count);
    for _ in 0..count {
        let center_index = read_u32(bytes, offset, path)? as usize;
        let correct_vec = read_phase_cells(bytes, offset, cells, path)?.into_boxed_slice();
        let wrong_vec = read_phase_cells(bytes, offset, cells, path)?.into_boxed_slice();
        tasks.push(PreparedTask {
            task: PhaseCenterEvalTask {
                center_index,
                correct_vec,
                wrong_vec,
            },
        });
    }
    Ok(tasks)
}

fn read_phase_cells(
    bytes: &[u8],
    offset: &mut usize,
    cells: usize,
    path: &Path,
) -> Result<Vec<PhaseCenterCell>, String> {
    let mut values = Vec::with_capacity(cells);
    for _ in 0..cells {
        let re = read_f64(bytes, offset, path)?;
        let im = read_f64(bytes, offset, path)?;
        values.push(PhaseCenterCell { re, im });
    }
    Ok(values)
}

fn read_u32(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u32, String> {
    let raw = read_exact_array::<4>(bytes, offset, path)?;
    Ok(u32::from_le_bytes(raw))
}

fn read_u64(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u64, String> {
    let raw = read_exact_array::<8>(bytes, offset, path)?;
    Ok(u64::from_le_bytes(raw))
}

fn read_f64(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<f64, String> {
    let raw = read_exact_array::<8>(bytes, offset, path)?;
    Ok(f64::from_le_bytes(raw))
}

fn read_exact_array<const N: usize>(
    bytes: &[u8],
    offset: &mut usize,
    path: &Path,
) -> Result<[u8; N], String> {
    let end = offset.checked_add(N).ok_or_else(|| {
        format!(
            "phase eval task package '{}' offset overflow",
            path.display()
        )
    })?;
    let Some(slice) = bytes.get(*offset..end) else {
        return Err(format!(
            "phase eval task package '{}' ended unexpectedly",
            path.display()
        ));
    };
    let mut raw = [0u8; N];
    raw.copy_from_slice(slice);
    *offset = end;
    Ok(raw)
}

fn eval_pack_v4_gate_pass(
    eval_package: &PhaseEvalTaskPackage,
    package_info: &nando_core::PhaseCenterRuntimePackageInfo,
    manifest: &PhasePackageManifest,
) -> bool {
    validate_eval_task_package_match(eval_package, manifest, package_info).is_ok()
        && eval_package.prepared.missing_centers == 0
        && eval_package.prepared.skipped_rows == 0
        && eval_package.prepared.action_ablation_missing_centers == 0
        && !eval_package.prepared.tasks.is_empty()
        && !eval_package.prepared.action_ablation_tasks.is_empty()
        && !manifest.forbidden_flags.any_forbidden_used()
}

fn package_v4_gate_pass(
    eval: &RuntimeEval,
    prepared: &PreparedEval,
    skipped_train_rows: usize,
    action_ablation_eval: &RuntimeEval,
    package: PackageGateMeta,
) -> bool {
    score_v4_gate_pass(eval, prepared, action_ablation_eval, false)
        && skipped_train_rows == 0
        && package.package_fingerprint64 != 0
        && package.operator_key_count == package.record_count
        && !package.has_empty_operator_key
}

fn action_package_v1_gate_pass(
    eval: &RuntimeEval,
    prepared: &PreparedEval,
    skipped_train_rows: usize,
    action_ablation_eval: &RuntimeEval,
    package: PackageGateMeta,
    contract_verdict: &str,
) -> bool {
    contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
        && score_v4_gate_pass(eval, prepared, action_ablation_eval, false)
        && skipped_train_rows == 0
        && package.package_fingerprint64 != 0
        && package.operator_key_count == package.record_count
        && !package.has_empty_operator_key
}

fn action_score_report_gate_inputs_pass(
    eval: &RuntimeEval,
    prepared: &PreparedEval,
    action_ablation_eval: &RuntimeEval,
    forbidden_used: bool,
    contract_verdict: &str,
    manifest_gate_pass: bool,
) -> bool {
    contract_verdict == "PHASE_ACTION_CONTRACT_V1_PASS"
        && manifest_gate_pass
        && eval.accuracy_milli == 1000
        && eval.wrong_wins == 0
        && prepared.missing_centers == 0
        && prepared.skipped_rows == 0
        && prepared.action_ablation_missing_centers == 0
        && !prepared.action_ablation_tasks.is_empty()
        && action_ablation_eval.accuracy_milli < eval.accuracy_milli
        && action_ablation_eval.wrong_wins > 0
        && !forbidden_used
}

fn score_v4_gate_pass(
    eval: &RuntimeEval,
    prepared: &PreparedEval,
    action_ablation_eval: &RuntimeEval,
    forbidden_used: bool,
) -> bool {
    eval.accuracy_milli == 1000
        && eval.wrong_wins == 0
        && prepared.missing_centers == 0
        && prepared.skipped_rows == 0
        && prepared.action_ablation_missing_centers == 0
        && !prepared.action_ablation_tasks.is_empty()
        && action_ablation_eval.accuracy_milli < eval.accuracy_milli
        && action_ablation_eval.wrong_wins > 0
        && !forbidden_used
}

fn phase_package_v4_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_PACKAGE_V4_PASS"
    } else {
        "PHASE_PACKAGE_V4_WATCH"
    }
}

fn phase_package_score_v4_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_PACKAGE_SCORE_V4_PASS"
    } else {
        "PHASE_PACKAGE_SCORE_V4_WATCH"
    }
}

fn phase_eval_pack_v4_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_EVAL_PACK_V4_PASS"
    } else {
        "PHASE_EVAL_PACK_V4_WATCH"
    }
}

fn phase_package_score_pack_v4_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_PACKAGE_SCORE_PACK_V4_PASS"
    } else {
        "PHASE_PACKAGE_SCORE_PACK_V4_WATCH"
    }
}

fn phase_action_boundary_v4_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_BOUNDARY_V4_PASS"
    } else {
        "PHASE_ACTION_BOUNDARY_V4_WATCH"
    }
}

fn phase_action_corpus_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_CORPUS_V1_PASS"
    } else {
        "PHASE_ACTION_CORPUS_V1_WATCH"
    }
}

fn phase_action_contract_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_CONTRACT_V1_PASS"
    } else {
        "PHASE_ACTION_CONTRACT_V1_WATCH"
    }
}

fn phase_action_operator_coverage_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_OPERATOR_COVERAGE_V1_PASS"
    } else {
        "PHASE_ACTION_OPERATOR_COVERAGE_V1_WATCH"
    }
}

fn phase_action_shortcut_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_SHORTCUT_V1_PASS"
    } else {
        "PHASE_ACTION_SHORTCUT_V1_WATCH"
    }
}

fn phase_action_runtime_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_RUNTIME_V1_PASS"
    } else {
        "PHASE_ACTION_RUNTIME_V1_WATCH"
    }
}

fn phase_action_package_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_V1_WATCH"
    }
}

fn phase_action_package_inspect_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_INSPECT_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_INSPECT_V1_WATCH"
    }
}

fn phase_action_source_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_SOURCE_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_SOURCE_VERIFY_V1_WATCH"
    }
}

fn phase_action_package_score_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_SCORE_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_SCORE_V1_WATCH"
    }
}

fn phase_action_eval_pack_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_EVAL_PACK_V1_PASS"
    } else {
        "PHASE_ACTION_EVAL_PACK_V1_WATCH"
    }
}

fn phase_action_package_score_pack_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_SCORE_PACK_V1_WATCH"
    }
}

fn phase_action_package_bench_pack_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_BENCH_PACK_V1_WATCH"
    }
}

fn phase_action_package_bench_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_BENCH_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_BENCH_VERIFY_V1_WATCH"
    }
}

fn phase_action_product_proof_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PRODUCT_PROOF_V1_PASS"
    } else {
        "PHASE_ACTION_PRODUCT_PROOF_V1_WATCH"
    }
}

fn phase_action_product_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PRODUCT_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_PRODUCT_VERIFY_V1_WATCH"
    }
}

fn phase_action_release_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_RELEASE_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_RELEASE_VERIFY_V1_WATCH"
    }
}

fn phase_action_license_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_LICENSE_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_LICENSE_VERIFY_V1_WATCH"
    }
}

fn phase_action_offload_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_OFFLOAD_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_OFFLOAD_VERIFY_V1_WATCH"
    }
}

fn phase_action_cache_offload_bench_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_CACHE_OFFLOAD_BENCH_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_CACHE_OFFLOAD_BENCH_VERIFY_V1_WATCH"
    }
}

fn phase_action_workflow_bench_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_WORKFLOW_BENCH_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_WORKFLOW_BENCH_VERIFY_V1_WATCH"
    }
}

fn phase_action_workflow_replay_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_WATCH"
    }
}

fn phase_action_regression_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_REGRESSION_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_REGRESSION_VERIFY_V1_WATCH"
    }
}

fn phase_action_regression_freeze_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_REGRESSION_FREEZE_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_REGRESSION_FREEZE_VERIFY_V1_WATCH"
    }
}

fn phase_action_package_verify_v1_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_ACTION_PACKAGE_VERIFY_V1_PASS"
    } else {
        "PHASE_ACTION_PACKAGE_VERIFY_V1_WATCH"
    }
}

fn phase_package_verify_verdict(gate_pass: bool) -> &'static str {
    if gate_pass {
        "PHASE_PACKAGE_VERIFY_PASS"
    } else {
        "PHASE_PACKAGE_VERIFY_WATCH"
    }
}

fn phase_operator_key(row: &PhaseOperatorRow) -> String {
    let condition = row
        .condition_flag
        .as_deref()
        .map(|value| format!("condition={value}"))
        .unwrap_or_else(|| "condition=<none>".to_string());
    format!(
        "class={}|length={}|{}|action={}",
        row.operator_class,
        row.sequence_length,
        condition,
        normalized_phase_action(row)
    )
}

fn phase_operator_bucket_key(row: &PhaseOperatorRow) -> String {
    let condition = row
        .condition_flag
        .as_deref()
        .map(|value| format!("condition={value}"))
        .unwrap_or_else(|| "condition=<none>".to_string());
    format!(
        "class={}|length={}|{}",
        row.operator_class, row.sequence_length, condition
    )
}

fn normalized_phase_action(row: &PhaseOperatorRow) -> String {
    let action = if let Some(marker) = extract_marker_value(&row.action) {
        row.action.replace(&marker, "<MARKER>")
    } else {
        row.action.clone()
    };
    collapse_whitespace(&action)
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn action_contract_text(row: &PhaseActionContractRow) -> String {
    collapse_whitespace(&format!(
        "{} {} {} {} {}",
        row.action_tree.select,
        row.action_tree.transform,
        row.action_tree.write,
        row.action_tree.condition,
        row.action_tree.check
    ))
}

fn action_contract_key(row: &PhaseActionContractRow) -> String {
    collapse_whitespace(&format!(
        "select={}|transform={}|write={}|condition={}|check={}",
        row.action_tree.select,
        row.action_tree.transform,
        row.action_tree.write,
        row.action_tree.condition,
        row.action_tree.check
    ))
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_numbered_slot_token(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|token| {
            let token = token.trim_matches('_');
            token
                .strip_prefix("src")
                .or_else(|| token.strip_prefix("out"))
                .or_else(|| token.strip_prefix("slot"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
                })
        })
}

fn action_contains_output_token(row: &PhaseActionContractRow, action_lower: &str) -> bool {
    row.state_after_correct
        .split_whitespace()
        .chain(row.state_after_wrong.split_whitespace())
        .map(|token| {
            token
                .trim_matches(|character: char| !character.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .any(|token| token.len() > 2 && action_lower.contains(&token))
}

fn same_token_bag(left: &str, right: &str) -> bool {
    let mut left_tokens = left.split_whitespace().collect::<Vec<_>>();
    let mut right_tokens = right.split_whitespace().collect::<Vec<_>>();
    left_tokens.sort_unstable();
    right_tokens.sort_unstable();
    left_tokens == right_tokens
}

fn source_bigram_overlap_score(source: &str, candidate: &str) -> usize {
    let source_tokens = source.split_whitespace().collect::<Vec<_>>();
    let candidate_tokens = candidate.split_whitespace().collect::<Vec<_>>();
    let source_bigrams = source_tokens
        .windows(2)
        .map(|window| format!("{}\0{}", window[0], window[1]))
        .collect::<BTreeSet<_>>();
    candidate_tokens
        .windows(2)
        .filter(|window| source_bigrams.contains(&format!("{}\0{}", window[0], window[1])))
        .count()
}

fn action_contract_transition_atoms(
    row: &PhaseActionContractRow,
    candidate_state: &str,
) -> Option<Vec<String>> {
    let source_tokens = row.state_before.split_whitespace().collect::<Vec<_>>();
    let candidate_tokens = candidate_state.split_whitespace().collect::<Vec<_>>();
    if source_tokens.is_empty() || candidate_tokens.is_empty() {
        return None;
    }

    let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, token) in source_tokens.iter().enumerate() {
        positions.entry(*token).or_default().push(index);
    }

    let mut atoms = vec![
        format!("src_len:{}", source_tokens.len()),
        format!("out_len:{}", candidate_tokens.len()),
    ];
    for (out_slot, token) in candidate_tokens.iter().enumerate() {
        if let Some(source_slots) = positions.get(*token) {
            if source_slots.len() != 1 {
                return None;
            }
            let src_slot = source_slots[0];
            atoms.push(format!("rel:o{out_slot}:s{src_slot}"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push(format!("src:s{src_slot}"));
            atoms.push(format!("delta:{}", out_slot as isize - src_slot as isize));
        } else {
            atoms.push(format!("insert:o{out_slot}"));
            atoms.push(format!("insert_shape:{}", token_shape(token)));
        }
    }
    Some(atoms)
}

fn reversed(source: &[String]) -> Vec<String> {
    source.iter().cloned().rev().collect()
}

fn rotate_left(source: &[String]) -> Vec<String> {
    let mut out = source.to_vec();
    if !out.is_empty() {
        out.rotate_left(1);
    }
    out
}

fn rotate_right(source: &[String]) -> Vec<String> {
    let mut out = source.to_vec();
    if !out.is_empty() {
        out.rotate_right(1);
    }
    out
}

fn adjacent_pair_swap(source: &[String]) -> Vec<String> {
    let mut out = source.to_vec();
    for chunk in out.chunks_mut(2) {
        if chunk.len() == 2 {
            chunk.swap(0, 1);
        }
    }
    out
}

fn swap_halves(source: &[String]) -> Vec<String> {
    let pivot = source.len() / 2;
    source[pivot..]
        .iter()
        .chain(source[..pivot].iter())
        .cloned()
        .collect()
}

fn even_then_odd(source: &[String]) -> Vec<String> {
    source
        .iter()
        .step_by(2)
        .chain(source.iter().skip(1).step_by(2))
        .cloned()
        .collect()
}

fn odd_then_even(source: &[String]) -> Vec<String> {
    source
        .iter()
        .skip(1)
        .step_by(2)
        .chain(source.iter().step_by(2))
        .cloned()
        .collect()
}

fn inner_reverse(source: &[String]) -> Vec<String> {
    if source.len() <= 2 {
        return source.to_vec();
    }
    let mut out = Vec::with_capacity(source.len());
    out.push(source[0].clone());
    out.extend(source[1..source.len() - 1].iter().cloned().rev());
    out.push(source[source.len() - 1].clone());
    out
}

fn move_second_to_tail(source: &[String]) -> Vec<String> {
    if source.len() <= 2 {
        return rotate_left(source);
    }
    let mut out = Vec::with_capacity(source.len());
    out.push(source[0].clone());
    out.extend(source[2..].iter().cloned());
    out.push(source[1].clone());
    out
}

fn move_penultimate_to_front(source: &[String]) -> Vec<String> {
    if source.len() <= 2 {
        return rotate_right(source);
    }
    let penultimate = source.len() - 2;
    let mut out = Vec::with_capacity(source.len());
    out.push(source[penultimate].clone());
    out.extend(source[..penultimate].iter().cloned());
    out.push(source[source.len() - 1].clone());
    out
}

fn token_shape(token: &str) -> String {
    token
        .chars()
        .map(|character| {
            if character.is_ascii_digit() {
                '0'
            } else if character.is_ascii_uppercase() {
                'A'
            } else if character.is_ascii_lowercase() {
                'a'
            } else {
                '_'
            }
        })
        .collect()
}

fn phase_split(row: &PhaseOperatorRow) -> Option<&'static str> {
    if row.source_group.contains("_train_") {
        Some("train")
    } else if row.source_group.contains("_heldout_") {
        Some("heldout")
    } else {
        None
    }
}

fn phase_transition_atoms(
    row: &PhaseOperatorRow,
    candidate_tokens: &[String],
) -> Option<Vec<String>> {
    let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, token) in row.source_tokens.iter().enumerate() {
        positions.entry(token.as_str()).or_default().push(index);
    }

    let marker = extract_marker_value(&row.action);
    let mut atoms = vec![
        format!("class:{}", row.operator_class),
        format!("src_len:{}", row.source_tokens.len()),
        format!("out_len:{}", candidate_tokens.len()),
    ];
    for (out_slot, token) in candidate_tokens.iter().enumerate() {
        if let Some(source_slots) = positions.get(token.as_str()) {
            if source_slots.len() != 1 {
                return None;
            }
            let src_slot = source_slots[0];
            atoms.push(format!("rel:o{out_slot}:s{src_slot}"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push(format!("src:s{src_slot}"));
            atoms.push(format!("delta:{}", out_slot as isize - src_slot as isize));
        } else if marker.as_deref() == Some(token.as_str()) {
            atoms.push(format!("rel:o{out_slot}:marker"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push("src:marker".to_string());
        } else {
            return None;
        }
    }
    Some(atoms)
}

fn extract_marker_value(action: &str) -> Option<String> {
    let start = action.find("marker:")? + "marker:".len();
    let value = action[start..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn percentile_f64(values: &[f64], percentile: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values[percentile_index(values.len(), percentile)]
}

fn percentile_u128(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values[percentile_index(values.len(), percentile)]
}

fn percentile_i64(values: &[i64], percentile: usize) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values[percentile_index(values.len(), percentile)]
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let clamped = percentile.min(100);
    ((len - 1) * clamped) / 100
}

fn milli_ratio(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (1000 * numerator + denominator / 2) / denominator
}

fn format_runtime_error(error: nando_core::PhaseCenterRuntimeError) -> String {
    format!("phase-center runtime error: {error:?}")
}

fn default_noise_type() -> String {
    "v4_noise".to_string()
}
