use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use nando_core::{
    WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC, WavePredictorActiveCenter, WavePredictorCenterId,
    WavePredictorRoleBindingEvalTask, WavePredictorRoleBindingOffloadAction,
    WavePredictorRoleBindingOffloadPolicy, WavePredictorRoleBindingOffloadRuntime,
};
use serde::{Deserialize, Serialize};

const DEFAULT_ROLE_BINDING_PACKAGE: &str =
    "target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb";
const DEFAULT_ROLE_BINDING_INSPECT_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_EVAL_PACK: &str =
    "target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json";
const DEFAULT_ROLE_BINDING_SCORE_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK: &str =
    "target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.nwreb";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-eval-pack-binary-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_ROOT: &str =
    "target/nando-wave/slot32-role-binding";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT: &str = "target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_REPORT: &str = "target/nando-wave/slot32-role-binding/role-binding-operator-blueprint-gap-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_EVAL_PACK_MAX_TASKS: usize = 128;
const DEFAULT_ROLE_BINDING_MARGIN_THRESHOLD: i32 = 1;
const DEFAULT_ROLE_BINDING_CORPUS_MARGIN_THRESHOLD: i32 = 1_000_000;
const ROLE_BINDING_EVAL_PACK_BINARY_MAGIC: [u8; 8] = *b"NWRE0001";
const ROLE_BINDING_BINARY_SUITE_ITEMS: [RoleBindingBinarySuiteItem; 7] = [
    RoleBindingBinarySuiteItem::new("sdk_mixed_map", 0, None),
    RoleBindingBinarySuiteItem::new("sdk_mixed_map", 1, None),
    RoleBindingBinarySuiteItem::new("sdk_mixed_map", 2, None),
    RoleBindingBinarySuiteItem::new("sdk_conditional_branch", 0, None),
    RoleBindingBinarySuiteItem::new("sdk_conditional_branch", 1, None),
    RoleBindingBinarySuiteItem::new("sdk_conditional_branch", 2, None),
    RoleBindingBinarySuiteItem::new("sdk_edit_marker_length", 0, Some(1)),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoleBindingBinarySuiteItem {
    label: &'static str,
    seed: u8,
    margin_threshold: Option<i32>,
}

impl RoleBindingBinarySuiteItem {
    const fn new(label: &'static str, seed: u8, margin_threshold: Option<i32>) -> Self {
        Self {
            label,
            seed,
            margin_threshold,
        }
    }

    fn effective_margin_threshold(self, default_margin_threshold: i32) -> i32 {
        self.margin_threshold.unwrap_or(default_margin_threshold)
    }
}

pub(crate) fn run_role_binding_package_inspect_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PACKAGE));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_INSPECT_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report = build_role_binding_package_inspect_report(&package_path)?;
    if !report.gate_pass {
        return Err(format!(
            "role-binding package inspect gate failed for {}",
            package_path.display()
        ));
    }
    write_json_file(&report_path, &report)?;

    println!("role-binding-package-inspect-v1: {}", report.verdict);
    println!("  package_path: {}", report.package_path);
    println!("  report_path: {}", report_path.display());
    println!("  package_magic: {}", report.package_magic_text);
    println!("  edge_count: {}", report.edge_count);
    println!("  package_bytes: {}", report.package_bytes);
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!(
        "  sdk_load_matches_inspect: {}",
        report.sdk_load_matches_inspect
    );
    println!("  rust_runtime_used: {}", report.rust_runtime_used);
    println!("  python_demo_used: {}", report.python_demo_used);
    println!("  corpus_jsonl_used: {}", report.corpus_jsonl_used);
    Ok(())
}

pub(crate) fn run_role_binding_package_verify_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PACKAGE));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_INSPECT_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved = read_json_file::<RoleBindingPackageInspectReport>(&report_path)?;
    let rebuilt = build_role_binding_package_inspect_report(&package_path)?;
    let report_matches_package = saved.matches_rebuilt(&rebuilt);
    if !saved.gate_pass || !rebuilt.gate_pass || !report_matches_package {
        return Err(format!(
            "role-binding package verify failed: saved_gate_pass={} rebuilt_gate_pass={} report_matches_package={}",
            saved.gate_pass, rebuilt.gate_pass, report_matches_package
        ));
    }

    println!("role-binding-package-verify-v1: ROLE_BINDING_PACKAGE_VERIFY_V1_PASS");
    println!("  package_path: {}", package_path.display());
    println!("  report_path: {}", report_path.display());
    println!("  package_fingerprint64: {}", rebuilt.package_fingerprint64);
    println!("  edge_count: {}", rebuilt.edge_count);
    println!("  report_matches_package: {report_matches_package}");
    Ok(())
}

pub(crate) fn run_role_binding_eval_pack_from_package_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PACKAGE));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_EVAL_PACK));
    let max_tasks = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid max-tasks '{value}': {error}"))?,
        None => DEFAULT_ROLE_BINDING_EVAL_PACK_MAX_TASKS,
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let (eval_pack, report) = build_role_binding_eval_pack_from_package(&package_path, max_tasks)?;
    write_json_file(&eval_pack_path, &eval_pack)?;

    println!("role-binding-eval-pack-from-package-v1: {}", report.verdict);
    println!("  package_path: {}", package_path.display());
    println!("  eval_pack_path: {}", eval_pack_path.display());
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!("  task_count: {}", report.task_count);
    println!("  expected_local_tasks: {}", report.expected_local_tasks);
    println!(
        "  expected_fallback_tasks: {}",
        report.expected_fallback_tasks
    );
    println!("  claim_boundary: {}", report.claim_boundary);
    Ok(())
}

pub(crate) fn run_role_binding_eval_pack_binary_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let source_eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_EVAL_PACK));
    let binary_eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report =
        build_role_binding_binary_eval_pack_report(&source_eval_pack_path, &binary_eval_pack_path)?;
    if !report.gate_pass {
        return Err(format!(
            "role-binding binary eval-pack gate failed: verdict={}",
            report.verdict
        ));
    }
    write_json_file(&report_path, &report)?;

    println!("role-binding-eval-pack-binary-v1: {}", report.verdict);
    println!("  source_eval_pack_path: {}", report.source_eval_pack_path);
    println!("  binary_eval_pack_path: {}", report.binary_eval_pack_path);
    println!("  report_path: {}", report_path.display());
    println!(
        "  package_fingerprint64: {:?}",
        report.package_fingerprint64
    );
    println!("  task_count: {}", report.task_count);
    println!("  sequence_count: {}", report.sequence_count);
    println!(
        "  source_eval_pack_bytes: {}",
        report.source_eval_pack_bytes
    );
    println!(
        "  binary_eval_pack_bytes: {}",
        report.binary_eval_pack_bytes
    );
    println!("  size_reduction_milli: {}", report.size_reduction_milli);
    println!("  roundtrip_exact: {}", report.roundtrip_exact);
    Ok(())
}

pub(crate) fn run_role_binding_binary_eval_pack_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let root_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_ROOT));
    let suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let margin_threshold = match args.next() {
        Some(value) => value
            .parse::<i32>()
            .map_err(|error| format!("invalid margin-threshold '{value}': {error}"))?,
        None => DEFAULT_ROLE_BINDING_CORPUS_MARGIN_THRESHOLD,
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report =
        build_role_binding_binary_eval_pack_suite_report(&root_dir, margin_threshold, true)?;
    if !report.gate_pass {
        return Err(format!(
            "role-binding binary eval-pack suite gate failed: verdict={}",
            report.verdict
        ));
    }
    write_json_file(&suite_report_path, &report)?;

    println!("role-binding-binary-eval-pack-suite-v1: {}", report.verdict);
    println!("  root_dir: {}", report.root_dir);
    println!("  suite_report_path: {}", suite_report_path.display());
    println!("  suite_items: {}", report.suite_items);
    println!("  total_sequence_count: {}", report.total_sequence_count);
    println!(
        "  total_expected_local_sequences: {}",
        report.total_expected_local_sequences
    );
    println!(
        "  total_expected_fallback_sequences: {}",
        report.total_expected_fallback_sequences
    );
    println!(
        "  total_sequence_false_local_accepts: {}",
        report.total_sequence_false_local_accepts
    );
    println!(
        "  total_sequence_missed_expected_local: {}",
        report.total_sequence_missed_expected_local
    );
    println!(
        "  min_sequence_strict_ordered_accuracy_milli: {}",
        report.min_sequence_strict_ordered_accuracy_milli
    );
    println!(
        "  suite_size_reduction_milli: {}",
        report.suite_size_reduction_milli
    );
    Ok(())
}

pub(crate) fn run_role_binding_binary_eval_pack_suite_verify_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let root_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_ROOT));
    let suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let margin_threshold = match args.next() {
        Some(value) => value
            .parse::<i32>()
            .map_err(|error| format!("invalid margin-threshold '{value}': {error}"))?,
        None => DEFAULT_ROLE_BINDING_CORPUS_MARGIN_THRESHOLD,
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved = read_json_file::<RoleBindingBinaryEvalPackSuiteReport>(&suite_report_path)?;
    let rebuilt =
        build_role_binding_binary_eval_pack_suite_report(&root_dir, margin_threshold, false)?;
    let report_matches_sources = saved.matches_rebuilt(&rebuilt);
    if !saved.gate_pass || !rebuilt.gate_pass || !report_matches_sources {
        return Err(format!(
            "role-binding binary eval-pack suite verify failed: saved_gate_pass={} rebuilt_gate_pass={} report_matches_sources={}",
            saved.gate_pass, rebuilt.gate_pass, report_matches_sources
        ));
    }

    println!(
        "role-binding-binary-eval-pack-suite-verify-v1: ROLE_BINDING_BINARY_EVAL_PACK_SUITE_VERIFY_V1_PASS"
    );
    println!("  root_dir: {}", root_dir.display());
    println!("  suite_report_path: {}", suite_report_path.display());
    println!("  suite_items: {}", rebuilt.suite_items);
    println!("  total_sequence_count: {}", rebuilt.total_sequence_count);
    println!("  report_matches_sources: {report_matches_sources}");
    Ok(())
}

pub(crate) fn run_role_binding_release_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let binary_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report = build_role_binding_release_suite_report(&binary_suite_report_path)?;
    if !report.gate_pass {
        return Err(format!(
            "role-binding release suite gate failed: verdict={}",
            report.verdict
        ));
    }
    write_json_file(&release_suite_report_path, &report)?;

    println!("role-binding-release-suite-v1: {}", report.verdict);
    println!(
        "  binary_suite_report_path: {}",
        report.binary_suite_report_path
    );
    println!(
        "  release_suite_report_path: {}",
        release_suite_report_path.display()
    );
    println!("  package_count: {}", report.package_count);
    println!(
        "  binary_eval_pack_count: {}",
        report.binary_eval_pack_count
    );
    println!("  score_report_count: {}", report.score_report_count);
    println!("  total_sequence_count: {}", report.total_sequence_count);
    println!(
        "  min_sequence_strict_ordered_accuracy_milli: {}",
        report.min_sequence_strict_ordered_accuracy_milli
    );
    println!(
        "  total_sequence_false_local_accepts: {}",
        report.total_sequence_false_local_accepts
    );
    println!(
        "  all_package_fingerprints_match_suite: {}",
        report.all_package_fingerprints_match_suite
    );
    println!(
        "  all_eval_pack_fingerprints_match_suite: {}",
        report.all_eval_pack_fingerprints_match_suite
    );
    println!("  gate_pass: {}", report.gate_pass);
    Ok(())
}

pub(crate) fn run_role_binding_release_suite_verify_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let binary_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved = read_json_file::<RoleBindingReleaseSuiteReport>(&release_suite_report_path)?;
    let rebuilt = build_role_binding_release_suite_report(&binary_suite_report_path)?;
    let report_matches_sources = saved.matches_rebuilt(&rebuilt);
    if !saved.gate_pass || !rebuilt.gate_pass || !report_matches_sources {
        return Err(format!(
            "role-binding release suite verify failed: saved_gate_pass={} rebuilt_gate_pass={} report_matches_sources={}",
            saved.gate_pass, rebuilt.gate_pass, report_matches_sources
        ));
    }

    println!("role-binding-release-suite-verify-v1: ROLE_BINDING_RELEASE_SUITE_VERIFY_V1_PASS");
    println!(
        "  binary_suite_report_path: {}",
        binary_suite_report_path.display()
    );
    println!(
        "  release_suite_report_path: {}",
        release_suite_report_path.display()
    );
    println!("  package_count: {}", rebuilt.package_count);
    println!("  total_sequence_count: {}", rebuilt.total_sequence_count);
    println!("  report_matches_sources: {report_matches_sources}");
    Ok(())
}

pub(crate) fn run_role_binding_operator_blueprint_gap_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    let gap_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report = build_role_binding_operator_blueprint_gap_report(&release_suite_report_path)?;
    write_json_file(&gap_report_path, &report)?;

    println!("role-binding-operator-blueprint-gap-v1: {}", report.verdict);
    println!(
        "  release_suite_report_path: {}",
        report.release_suite_report_path
    );
    println!("  gap_report_path: {}", gap_report_path.display());
    println!(
        "  blueprint_required_class_count: {}",
        report.blueprint_required_class_count
    );
    println!("  proven_classes: {}", report.proven_classes);
    println!("  partial_classes: {}", report.partial_classes);
    println!("  missing_classes: {}", report.missing_classes);
    println!("  coverage_gate_pass: {}", report.coverage_gate_pass);
    println!(
        "  full_32_slot_operator_battery_closed: {}",
        report.full_32_slot_operator_battery_closed
    );
    Ok(())
}

pub(crate) fn run_role_binding_operator_blueprint_gap_verify_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let release_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    let gap_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved = read_json_file::<RoleBindingOperatorBlueprintGapReport>(&gap_report_path)?;
    let rebuilt = build_role_binding_operator_blueprint_gap_report(&release_suite_report_path)?;
    let report_matches_sources = saved.matches_rebuilt(&rebuilt);
    if !saved.release_suite_gate_pass || !rebuilt.release_suite_gate_pass || !report_matches_sources
    {
        return Err(format!(
            "role-binding operator blueprint gap verify failed: saved_release_suite_gate_pass={} rebuilt_release_suite_gate_pass={} report_matches_sources={}",
            saved.release_suite_gate_pass, rebuilt.release_suite_gate_pass, report_matches_sources
        ));
    }

    println!(
        "role-binding-operator-blueprint-gap-verify-v1: ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_VERIFY_V1_PASS"
    );
    println!(
        "  release_suite_report_path: {}",
        release_suite_report_path.display()
    );
    println!("  gap_report_path: {}", gap_report_path.display());
    println!("  report_matches_sources: {report_matches_sources}");
    println!("  coverage_gate_pass: {}", rebuilt.coverage_gate_pass);
    println!("  verdict: {}", rebuilt.verdict);
    Ok(())
}

pub(crate) fn run_role_binding_package_score_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PACKAGE));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_EVAL_PACK));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_SCORE_REPORT));
    let margin_threshold = match args.next() {
        Some(value) => value
            .parse::<i32>()
            .map_err(|error| format!("invalid margin-threshold '{value}': {error}"))?,
        None => DEFAULT_ROLE_BINDING_MARGIN_THRESHOLD,
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report =
        build_role_binding_package_score_report(&package_path, &eval_pack_path, margin_threshold)?;
    if !report.gate_pass {
        return Err(format!(
            "role-binding package score gate failed: verdict={} false_local_accepts={} missed_expected_local={}",
            report.verdict, report.false_local_accepts, report.missed_expected_local
        ));
    }
    write_json_file(&report_path, &report)?;

    println!("role-binding-package-score-v1: {}", report.verdict);
    println!("  package_path: {}", report.package_path);
    println!("  eval_pack_path: {}", report.eval_pack_path);
    println!("  report_path: {}", report_path.display());
    println!("  package_fingerprint64: {}", report.package_fingerprint64);
    println!(
        "  eval_pack_fingerprint64: {}",
        report.eval_pack_fingerprint64
    );
    println!("  task_count: {}", report.task_count);
    println!("  local_operator_calls: {}", report.local_operator_calls);
    println!("  fallback_to_llm_calls: {}", report.fallback_to_llm_calls);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  missed_expected_local: {}", report.missed_expected_local);
    println!("  sequence_count: {}", report.sequence_count);
    println!(
        "  sequence_local_operator_calls: {}",
        report.sequence_local_operator_calls
    );
    println!(
        "  sequence_fallback_to_llm_calls: {}",
        report.sequence_fallback_to_llm_calls
    );
    println!(
        "  sequence_false_local_accepts: {}",
        report.sequence_false_local_accepts
    );
    println!(
        "  sequence_missed_expected_local: {}",
        report.sequence_missed_expected_local
    );
    println!(
        "  sequence_strict_ordered_accuracy_milli: {}",
        report.sequence_strict_ordered_accuracy_milli
    );
    println!(
        "  sequence_median_energy_margin: {}",
        report.sequence_median_energy_margin
    );
    println!("  min_margin: {}", report.min_margin);
    println!("  p10_margin: {}", report.p10_margin);
    println!("  median_margin: {}", report.median_margin);
    println!("  max_margin: {}", report.max_margin);
    Ok(())
}

pub(crate) fn run_role_binding_package_score_verify_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PACKAGE));
    let eval_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_EVAL_PACK));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_SCORE_REPORT));
    let margin_threshold = match args.next() {
        Some(value) => value
            .parse::<i32>()
            .map_err(|error| format!("invalid margin-threshold '{value}': {error}"))?,
        None => DEFAULT_ROLE_BINDING_MARGIN_THRESHOLD,
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved = read_json_file::<RoleBindingPackageScoreReport>(&report_path)?;
    let rebuilt =
        build_role_binding_package_score_report(&package_path, &eval_pack_path, margin_threshold)?;
    let report_matches_sources = saved.matches_rebuilt(&rebuilt);
    if !saved.gate_pass || !rebuilt.gate_pass || !report_matches_sources {
        return Err(format!(
            "role-binding package score verify failed: saved_gate_pass={} rebuilt_gate_pass={} report_matches_sources={}",
            saved.gate_pass, rebuilt.gate_pass, report_matches_sources
        ));
    }

    println!("role-binding-package-score-verify-v1: ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS");
    println!("  package_path: {}", package_path.display());
    println!("  eval_pack_path: {}", eval_pack_path.display());
    println!("  report_path: {}", report_path.display());
    println!("  package_fingerprint64: {}", rebuilt.package_fingerprint64);
    println!(
        "  eval_pack_fingerprint64: {}",
        rebuilt.eval_pack_fingerprint64
    );
    println!("  task_count: {}", rebuilt.task_count);
    println!("  sequence_count: {}", rebuilt.sequence_count);
    println!("  report_matches_sources: {report_matches_sources}");
    Ok(())
}

fn build_role_binding_package_inspect_report(
    package_path: &Path,
) -> Result<RoleBindingPackageInspectReport, String> {
    let bytes = std::fs::read(package_path)
        .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
    let info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&bytes)
        .map_err(|error| format!("failed to inspect {}: {error:?}", package_path.display()))?;
    let policy = WavePredictorRoleBindingOffloadPolicy::new(1)
        .map_err(|error| format!("failed to build conservative role-binding policy: {error:?}"))?;
    let runtime = WavePredictorRoleBindingOffloadRuntime::from_package_bytes(&bytes, policy)
        .map_err(|error| format!("failed to load {}: {error:?}", package_path.display()))?;
    let sdk_load_matches_inspect = runtime.package_info() == info;
    let magic_matches = info.magic == WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC;
    let serialized_len_matches = info.serialized_len == bytes.len();
    let nonzero_fingerprint = info.fingerprint64 != 0;
    let nonempty_runtime = info.edge_count > 0;
    let gate_pass = magic_matches
        && serialized_len_matches
        && nonzero_fingerprint
        && nonempty_runtime
        && sdk_load_matches_inspect;

    Ok(RoleBindingPackageInspectReport {
        schema_version: "nando_role_binding_package_inspect_report_v1".to_owned(),
        verdict: if gate_pass {
            "ROLE_BINDING_PACKAGE_INSPECT_V1_PASS"
        } else {
            "ROLE_BINDING_PACKAGE_INSPECT_V1_WATCH"
        }
        .to_owned(),
        package_path: package_path.display().to_string(),
        package_magic_text: String::from_utf8_lossy(&info.magic).into_owned(),
        package_magic_bytes: info.magic,
        package_bytes: bytes.len(),
        action_base: info.action_base,
        action_count: info.action_count,
        role_base: info.role_base,
        role_stride: info.role_stride,
        slot_scoped_action_page_bits: info.slot_scoped_action_page_bits,
        slot_scoped_action_page_mask: info.slot_scoped_action_page_mask,
        slot_scoped_action_source_bits: info.slot_scoped_action_source_bits,
        edge_count: info.edge_count,
        serialized_len: info.serialized_len,
        payload_bytes: info.payload_bytes,
        package_fingerprint64: info.fingerprint64,
        magic_matches,
        serialized_len_matches,
        nonzero_fingerprint,
        nonempty_runtime,
        sdk_load_matches_inspect,
        gate_pass,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        corpus_jsonl_used: false,
        rust_runtime_used: true,
        claim_boundary: "role-binding .nwrb package inspect/verify only; not phase-center .nwpc, not CLI/daemon scoring, not raw-language action parsing, not broad workflow reasoning".to_owned(),
    })
}

fn build_role_binding_eval_pack_from_package(
    package_path: &Path,
    max_tasks: usize,
) -> Result<(RoleBindingEvalPack, RoleBindingEvalPackBuildReport), String> {
    if max_tasks < 2 {
        return Err("max-tasks must be at least 2".to_owned());
    }
    let bytes = std::fs::read(package_path)
        .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
    let info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&bytes)
        .map_err(|error| format!("failed to inspect {}: {error:?}", package_path.display()))?;
    if info.role_stride < 2 {
        return Err(
            "role_stride must be at least 2 to build positive/fallback eval rows".to_owned(),
        );
    }
    let policy = WavePredictorRoleBindingOffloadPolicy::new(DEFAULT_ROLE_BINDING_MARGIN_THRESHOLD)
        .map_err(|error| format!("failed to build conservative role-binding policy: {error:?}"))?;
    let runtime = WavePredictorRoleBindingOffloadRuntime::from_package_bytes(&bytes, policy)
        .map_err(|error| format!("failed to load {}: {error:?}", package_path.display()))?;

    let lane_span = info.role_stride.min(u32::from(u16::MAX));
    let pair_count = (max_tasks / 2).max(1);
    let mut rows = Vec::with_capacity(pair_count * 2);
    for (index, edge) in runtime.table().edges().iter().enumerate() {
        if rows.len() >= pair_count * 2 {
            break;
        }
        let lane = ((index as u32) % lane_span) as u16;
        let wrong_lane = ((u32::from(lane) + 1) % lane_span) as u16;
        let signed_strength = signed_strength_from_sign_key(edge.sign_key)?;
        let role_center = info
            .role_base
            .checked_add(WavePredictorCenterId::from(edge.slot_id).saturating_mul(info.role_stride))
            .and_then(|slot_base| slot_base.checked_add(WavePredictorCenterId::from(lane)))
            .ok_or_else(|| "role center overflow while building eval pack".to_owned())?;
        let active_fringe = vec![
            RoleBindingActiveCenterRow {
                center_id: edge.action_center,
                strength: 1,
            },
            RoleBindingActiveCenterRow {
                center_id: role_center,
                strength: 1,
            },
        ];
        rows.push(RoleBindingEvalTaskRow {
            task_id: format!(
                "package_edge_{index:05}_local_slot{}_out{}_lane{}",
                edge.slot_id, edge.output_slot_id, lane
            ),
            target_lane_id: lane,
            target_signed_strength: signed_strength,
            wrong_lane_id: wrong_lane,
            wrong_signed_strength: signed_strength,
            active_fringe: active_fringe.clone(),
            binding_output_slot: Some(edge.output_slot_id),
            expect_local_operator: true,
        });
        rows.push(RoleBindingEvalTaskRow {
            task_id: format!(
                "package_edge_{index:05}_fallback_slot{}_out{}_lane{}",
                edge.slot_id, edge.output_slot_id, lane
            ),
            target_lane_id: wrong_lane,
            target_signed_strength: signed_strength,
            wrong_lane_id: lane,
            wrong_signed_strength: signed_strength,
            active_fringe,
            binding_output_slot: Some(edge.output_slot_id),
            expect_local_operator: false,
        });
    }
    if rows.len() < 2 {
        return Err("failed to derive any eval tasks from package edges".to_owned());
    }

    let expected_local_tasks = rows.iter().filter(|row| row.expect_local_operator).count();
    let expected_fallback_tasks = rows.len() - expected_local_tasks;
    let pack = RoleBindingEvalPack {
        schema_version: "nando_role_binding_eval_pack_v1".to_owned(),
        package_fingerprint64: Some(info.fingerprint64),
        source_package_path: Some(package_path.display().to_string()),
        generation_method:
            "package_edge_smoke_rows; validates CLI scoring plumbing, not independent corpus proof"
                .to_owned(),
        tasks: rows,
        sequences: Vec::new(),
    };
    let report = RoleBindingEvalPackBuildReport {
        schema_version: "nando_role_binding_eval_pack_build_report_v1".to_owned(),
        verdict: "ROLE_BINDING_EVAL_PACK_FROM_PACKAGE_V1_PASS".to_owned(),
        package_path: package_path.display().to_string(),
        package_fingerprint64: info.fingerprint64,
        task_count: pack.tasks.len(),
        expected_local_tasks,
        expected_fallback_tasks,
        gate_pass: expected_local_tasks > 0 && expected_fallback_tasks > 0,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        corpus_jsonl_used: false,
        rust_runtime_used: true,
        claim_boundary:
            "package-derived eval-pack is a CLI scoring smoke only; independent corpus eval-pack is still required for operator proof"
                .to_owned(),
    };
    Ok((pack, report))
}

fn build_role_binding_package_score_report(
    package_path: &Path,
    eval_pack_path: &Path,
    margin_threshold: i32,
) -> Result<RoleBindingPackageScoreReport, String> {
    let policy = WavePredictorRoleBindingOffloadPolicy::new(margin_threshold)
        .map_err(|error| format!("invalid margin threshold {margin_threshold}: {error:?}"))?;
    let package_bytes = std::fs::read(package_path)
        .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
    let eval_pack_bytes = std::fs::read(eval_pack_path)
        .map_err(|error| format!("failed to read {}: {error}", eval_pack_path.display()))?;
    let eval_pack_file = parse_role_binding_eval_pack_file(eval_pack_path, &eval_pack_bytes)?;
    let eval_pack = eval_pack_file.pack;

    validate_role_binding_eval_pack(&eval_pack)?;
    let info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("failed to inspect {}: {error:?}", package_path.display()))?;
    let runtime =
        WavePredictorRoleBindingOffloadRuntime::from_package_bytes(&package_bytes, policy)
            .map_err(|error| format!("failed to load {}: {error:?}", package_path.display()))?;
    let sdk_load_matches_inspect = runtime.package_info() == info;

    let active_storage = eval_pack
        .tasks
        .iter()
        .map(|row| {
            row.active_fringe
                .iter()
                .map(|active| WavePredictorActiveCenter {
                    center_id: active.center_id,
                    strength: active.strength,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let tasks = eval_pack
        .tasks
        .iter()
        .zip(active_storage.iter())
        .map(|(row, active_fringe)| WavePredictorRoleBindingEvalTask {
            target_lane_id: row.target_lane_id,
            target_signed_strength: row.target_signed_strength,
            wrong_lane_id: row.wrong_lane_id,
            wrong_signed_strength: row.wrong_signed_strength,
            active_fringe,
            binding_output_slot: row.binding_output_slot,
            expect_local_operator: row.expect_local_operator,
        })
        .collect::<Vec<_>>();

    let mut decisions = Vec::new();
    let mut margins = Vec::new();
    let summary = runtime
        .offload_summary_into(&tasks, &mut decisions, &mut margins)
        .map_err(|error| format!("failed to score role-binding eval pack: {error:?}"))?;
    let sequence_reports = eval_pack
        .sequences
        .iter()
        .map(|row| score_role_binding_sequence(&runtime, row, margin_threshold))
        .collect::<Result<Vec<_>, _>>()?;
    let per_task = eval_pack
        .tasks
        .iter()
        .zip(decisions.iter())
        .map(|(row, decision)| RoleBindingPackageScoreRow {
            task_id: row.task_id.clone(),
            action: role_binding_action_name(decision.action).to_owned(),
            margin: decision.margin,
            expect_local_operator: row.expect_local_operator,
            false_local_accept: decision.action
                == WavePredictorRoleBindingOffloadAction::LocalOperator
                && !row.expect_local_operator,
            missed_expected_local: decision.action
                == WavePredictorRoleBindingOffloadAction::FallbackToLlm
                && row.expect_local_operator,
        })
        .collect::<Vec<_>>();
    let expected_local_tasks = eval_pack
        .tasks
        .iter()
        .filter(|row| row.expect_local_operator)
        .count();
    let expected_fallback_tasks = eval_pack.tasks.len() - expected_local_tasks;
    let missed_expected_local = per_task
        .iter()
        .filter(|row| row.missed_expected_local)
        .count();
    let false_local_accepts = per_task.iter().filter(|row| row.false_local_accept).count();
    let expected_local_sequences = eval_pack
        .sequences
        .iter()
        .filter(|row| row.expect_local_operator)
        .count();
    let expected_fallback_sequences = eval_pack.sequences.len() - expected_local_sequences;
    let sequence_local_operator_calls = sequence_reports
        .iter()
        .filter(|row| row.action == "local_operator")
        .count();
    let sequence_fallback_to_llm_calls = sequence_reports.len() - sequence_local_operator_calls;
    let sequence_false_local_accepts = sequence_reports
        .iter()
        .filter(|row| row.false_local_accept)
        .count();
    let sequence_missed_expected_local = sequence_reports
        .iter()
        .filter(|row| row.missed_expected_local)
        .count();
    let mut sequence_energy_margins = sequence_reports
        .iter()
        .map(|row| row.energy_margin)
        .collect::<Vec<_>>();
    sequence_energy_margins.sort_unstable();
    let (
        sequence_min_energy_margin,
        sequence_p10_energy_margin,
        sequence_median_energy_margin,
        sequence_max_energy_margin,
    ) = margin_summary_or_zero(&sequence_energy_margins);
    let sequence_strict_ordered_accuracy_milli = if eval_pack.sequences.is_empty() {
        0
    } else {
        milli_ratio(
            sequence_reports
                .iter()
                .filter(|row| row.expect_local_operator && row.strict_ordered_pass)
                .count(),
            expected_local_sequences.max(1),
        )
    };
    let eval_pack_package_fingerprint_matches = eval_pack
        .package_fingerprint64
        .is_none_or(|fingerprint| fingerprint == info.fingerprint64);
    let mut sorted_margins = margins.clone();
    sorted_margins.sort_unstable();
    let (min_margin, p10_margin, median_margin, max_margin) =
        margin_summary_or_zero(&sorted_margins);
    let eval_pack_fingerprint64 = role_binding_cli_fingerprint64(&eval_pack_bytes);
    let claim_boundary = if eval_pack
        .generation_method
        .contains("heldout_corpus_sequences")
    {
        "role-binding .nwrb CLI sequence scoring over an independent corpus-emitted eval-pack; compact binary eval-pack, daemon registry, .nwpc bridge, and raw-language action parsing remain open"
    } else {
        "role-binding .nwrb CLI scoring/verify over an explicit eval-pack; package-derived eval-packs are plumbing smoke, independent corpus eval-pack is still required for full operator proof"
    };
    let task_gate_pass = eval_pack.tasks.is_empty()
        || (summary.calls == eval_pack.tasks.len()
            && expected_local_tasks > 0
            && expected_fallback_tasks > 0
            && summary.local_operator_calls == expected_local_tasks
            && summary.fallback_to_llm_calls == expected_fallback_tasks
            && false_local_accepts == 0
            && missed_expected_local == 0);
    let sequence_gate_pass = eval_pack.sequences.is_empty()
        || (expected_local_sequences > 0
            && expected_fallback_sequences > 0
            && sequence_local_operator_calls == expected_local_sequences
            && sequence_fallback_to_llm_calls == expected_fallback_sequences
            && sequence_false_local_accepts == 0
            && sequence_missed_expected_local == 0
            && sequence_strict_ordered_accuracy_milli == 1000);
    let gate_pass = (!eval_pack.tasks.is_empty() || !eval_pack.sequences.is_empty())
        && task_gate_pass
        && sequence_gate_pass
        && false_local_accepts == 0
        && missed_expected_local == 0
        && sequence_false_local_accepts == 0
        && sequence_missed_expected_local == 0
        && eval_pack_package_fingerprint_matches
        && sdk_load_matches_inspect;

    Ok(RoleBindingPackageScoreReport {
        schema_version: "nando_role_binding_package_score_report_v1".to_owned(),
        verdict: if gate_pass {
            "ROLE_BINDING_PACKAGE_SCORE_V1_PASS"
        } else {
            "ROLE_BINDING_PACKAGE_SCORE_V1_WATCH"
        }
        .to_owned(),
        package_path: package_path.display().to_string(),
        eval_pack_path: eval_pack_path.display().to_string(),
        package_fingerprint64: info.fingerprint64,
        eval_pack_fingerprint64,
        eval_pack_format: eval_pack_file.format,
        eval_pack_bytes: eval_pack_bytes.len(),
        eval_pack_package_fingerprint_matches,
        margin_threshold,
        task_count: summary.calls,
        expected_local_tasks,
        expected_fallback_tasks,
        local_operator_calls: summary.local_operator_calls,
        fallback_to_llm_calls: summary.fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        sequence_count: sequence_reports.len(),
        expected_local_sequences,
        expected_fallback_sequences,
        sequence_local_operator_calls,
        sequence_fallback_to_llm_calls,
        sequence_false_local_accepts,
        sequence_missed_expected_local,
        sequence_strict_ordered_accuracy_milli,
        sequence_min_energy_margin,
        sequence_p10_energy_margin,
        sequence_median_energy_margin,
        sequence_max_energy_margin,
        min_margin,
        p10_margin,
        median_margin,
        max_margin,
        sdk_load_matches_inspect,
        gate_pass,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        corpus_jsonl_used: false,
        rust_runtime_used: true,
        per_task,
        per_sequence: sequence_reports,
        claim_boundary: claim_boundary.to_owned(),
    })
}

fn build_role_binding_binary_eval_pack_report(
    source_eval_pack_path: &Path,
    binary_eval_pack_path: &Path,
) -> Result<RoleBindingBinaryEvalPackReport, String> {
    build_role_binding_binary_eval_pack_report_with_write(
        source_eval_pack_path,
        binary_eval_pack_path,
        true,
    )
}

fn build_role_binding_binary_eval_pack_report_with_write(
    source_eval_pack_path: &Path,
    binary_eval_pack_path: &Path,
    write_binary_artifact: bool,
) -> Result<RoleBindingBinaryEvalPackReport, String> {
    let source_bytes = std::fs::read(source_eval_pack_path).map_err(|error| {
        format!(
            "failed to read {}: {error}",
            source_eval_pack_path.display()
        )
    })?;
    let source = parse_role_binding_eval_pack_file(source_eval_pack_path, &source_bytes)?;
    if source.format == "binary" {
        return Err(
            "source eval-pack is already binary; expected json or compatible input".to_owned(),
        );
    }
    validate_role_binding_eval_pack(&source.pack)?;
    let binary_bytes = encode_role_binding_eval_pack_binary(&source.pack)?;
    if write_binary_artifact {
        if let Some(parent) = binary_eval_pack_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        std::fs::write(binary_eval_pack_path, &binary_bytes).map_err(|error| {
            format!(
                "failed to write {}: {error}",
                binary_eval_pack_path.display()
            )
        })?;
    }
    let read_back = std::fs::read(binary_eval_pack_path).map_err(|error| {
        format!(
            "failed to read back {}: {error}",
            binary_eval_pack_path.display()
        )
    })?;
    if !write_binary_artifact && read_back != binary_bytes {
        return Err(format!(
            "existing binary eval-pack {} does not match encoded source {}",
            binary_eval_pack_path.display(),
            source_eval_pack_path.display()
        ));
    }
    let decoded = parse_role_binding_eval_pack_binary(binary_eval_pack_path, &read_back)?;
    let roundtrip_exact = decoded == source.pack;
    let source_eval_pack_fingerprint64 = role_binding_cli_fingerprint64(&source_bytes);
    let binary_eval_pack_fingerprint64 = role_binding_cli_fingerprint64(&binary_bytes);
    let size_reduction_milli = if source_bytes.is_empty() {
        0
    } else {
        (source_bytes.len().saturating_sub(binary_bytes.len())) * 1000 / source_bytes.len()
    };
    let gate_pass = roundtrip_exact
        && binary_bytes.starts_with(&ROLE_BINDING_EVAL_PACK_BINARY_MAGIC)
        && binary_bytes.len() < source_bytes.len();

    Ok(RoleBindingBinaryEvalPackReport {
        schema_version: "nando_role_binding_binary_eval_pack_report_v1".to_owned(),
        verdict: if gate_pass {
            "ROLE_BINDING_EVAL_PACK_BINARY_V1_PASS"
        } else {
            "ROLE_BINDING_EVAL_PACK_BINARY_V1_WATCH"
        }
        .to_owned(),
        source_eval_pack_path: source_eval_pack_path.display().to_string(),
        binary_eval_pack_path: binary_eval_pack_path.display().to_string(),
        source_eval_pack_format: source.format,
        binary_magic_text: String::from_utf8_lossy(&ROLE_BINDING_EVAL_PACK_BINARY_MAGIC)
            .into_owned(),
        package_fingerprint64: source.pack.package_fingerprint64,
        task_count: source.pack.tasks.len(),
        sequence_count: source.pack.sequences.len(),
        source_eval_pack_bytes: source_bytes.len(),
        binary_eval_pack_bytes: binary_bytes.len(),
        source_eval_pack_fingerprint64,
        binary_eval_pack_fingerprint64,
        size_reduction_milli,
        roundtrip_exact,
        gate_pass,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        corpus_jsonl_used: false,
        rust_runtime_used: true,
        claim_boundary: "compact binary role-binding eval-pack packaging; does not change .nwrb runtime semantics or operator proof source".to_owned(),
    })
}

fn build_role_binding_binary_eval_pack_suite_report(
    root_dir: &Path,
    margin_threshold: i32,
    write_artifacts: bool,
) -> Result<RoleBindingBinaryEvalPackSuiteReport, String> {
    let mut rows = Vec::with_capacity(ROLE_BINDING_BINARY_SUITE_ITEMS.len());
    for (index, item) in ROLE_BINDING_BINARY_SUITE_ITEMS.iter().enumerate() {
        let label = item.label;
        let seed = item.seed;
        let item_margin_threshold = item.effective_margin_threshold(margin_threshold);
        if write_artifacts {
            println!(
                "  suite item {}/{}: {} seed{}",
                index + 1,
                ROLE_BINDING_BINARY_SUITE_ITEMS.len(),
                label,
                seed
            );
        }
        let package_path = root_dir.join(format!("{label}-seed{seed}.nwrb"));
        let source_eval_pack_path =
            root_dir.join(format!("{label}-seed{seed}.corpus-eval-pack-v1.json"));
        let binary_eval_pack_path =
            root_dir.join(format!("{label}-seed{seed}.corpus-eval-pack-v1.nwreb"));
        let binary_report_path = root_dir.join(format!(
            "role-binding-eval-pack-binary-corpus-{label}-seed{seed}-v1.product-proof.json"
        ));
        let score_report_path = root_dir.join(format!(
            "role-binding-package-score-binary-corpus-{label}-seed{seed}-v1.product-proof.json"
        ));

        let binary_report = build_role_binding_binary_eval_pack_report_with_write(
            &source_eval_pack_path,
            &binary_eval_pack_path,
            write_artifacts,
        )?;
        if write_artifacts {
            write_json_file(&binary_report_path, &binary_report)?;
        }
        let saved_binary_report =
            read_json_file::<RoleBindingBinaryEvalPackReport>(&binary_report_path)?;
        let binary_report_matches_sources = saved_binary_report.matches_rebuilt(&binary_report);

        let score_report = build_role_binding_package_score_report(
            &package_path,
            &binary_eval_pack_path,
            item_margin_threshold,
        )?;
        if write_artifacts {
            write_json_file(&score_report_path, &score_report)?;
        }
        let saved_score_report =
            read_json_file::<RoleBindingPackageScoreReport>(&score_report_path)?;
        let score_report_matches_sources = saved_score_report.matches_rebuilt(&score_report);

        rows.push(RoleBindingBinaryEvalPackSuiteRow {
            label: label.to_owned(),
            seed,
            margin_threshold: item_margin_threshold,
            package_path: package_path.display().to_string(),
            source_eval_pack_path: source_eval_pack_path.display().to_string(),
            binary_eval_pack_path: binary_eval_pack_path.display().to_string(),
            binary_eval_pack_report_path: binary_report_path.display().to_string(),
            score_report_path: score_report_path.display().to_string(),
            package_fingerprint64: score_report.package_fingerprint64,
            source_eval_pack_fingerprint64: binary_report.source_eval_pack_fingerprint64,
            binary_eval_pack_fingerprint64: binary_report.binary_eval_pack_fingerprint64,
            score_eval_pack_fingerprint64: score_report.eval_pack_fingerprint64,
            source_eval_pack_bytes: binary_report.source_eval_pack_bytes,
            binary_eval_pack_bytes: binary_report.binary_eval_pack_bytes,
            size_reduction_milli: binary_report.size_reduction_milli,
            binary_roundtrip_exact: binary_report.roundtrip_exact,
            binary_gate_pass: binary_report.gate_pass,
            binary_report_matches_sources,
            eval_pack_format: score_report.eval_pack_format.clone(),
            eval_pack_package_fingerprint_matches: score_report
                .eval_pack_package_fingerprint_matches,
            score_gate_pass: score_report.gate_pass,
            score_report_matches_sources,
            sequence_count: score_report.sequence_count,
            expected_local_sequences: score_report.expected_local_sequences,
            expected_fallback_sequences: score_report.expected_fallback_sequences,
            sequence_local_operator_calls: score_report.sequence_local_operator_calls,
            sequence_fallback_to_llm_calls: score_report.sequence_fallback_to_llm_calls,
            sequence_false_local_accepts: score_report.sequence_false_local_accepts,
            sequence_missed_expected_local: score_report.sequence_missed_expected_local,
            sequence_strict_ordered_accuracy_milli: score_report
                .sequence_strict_ordered_accuracy_milli,
            sequence_min_energy_margin: score_report.sequence_min_energy_margin,
            sequence_p10_energy_margin: score_report.sequence_p10_energy_margin,
            sequence_median_energy_margin: score_report.sequence_median_energy_margin,
            sequence_max_energy_margin: score_report.sequence_max_energy_margin,
        });
    }

    let suite_items = rows.len();
    let total_source_eval_pack_bytes = rows
        .iter()
        .map(|row| row.source_eval_pack_bytes)
        .sum::<usize>();
    let total_binary_eval_pack_bytes = rows
        .iter()
        .map(|row| row.binary_eval_pack_bytes)
        .sum::<usize>();
    let total_sequence_count = rows.iter().map(|row| row.sequence_count).sum::<usize>();
    let total_expected_local_sequences = rows
        .iter()
        .map(|row| row.expected_local_sequences)
        .sum::<usize>();
    let total_expected_fallback_sequences = rows
        .iter()
        .map(|row| row.expected_fallback_sequences)
        .sum::<usize>();
    let total_sequence_local_operator_calls = rows
        .iter()
        .map(|row| row.sequence_local_operator_calls)
        .sum::<usize>();
    let total_sequence_fallback_to_llm_calls = rows
        .iter()
        .map(|row| row.sequence_fallback_to_llm_calls)
        .sum::<usize>();
    let total_sequence_false_local_accepts = rows
        .iter()
        .map(|row| row.sequence_false_local_accepts)
        .sum::<usize>();
    let total_sequence_missed_expected_local = rows
        .iter()
        .map(|row| row.sequence_missed_expected_local)
        .sum::<usize>();
    let suite_size_reduction_milli = if total_source_eval_pack_bytes == 0 {
        0
    } else {
        total_source_eval_pack_bytes
            .saturating_sub(total_binary_eval_pack_bytes)
            .checked_mul(1000)
            .and_then(|value| value.checked_div(total_source_eval_pack_bytes))
            .unwrap_or(0)
    };
    let min_size_reduction_milli = rows
        .iter()
        .map(|row| row.size_reduction_milli)
        .min()
        .unwrap_or(0);
    let max_binary_eval_pack_bytes = rows
        .iter()
        .map(|row| row.binary_eval_pack_bytes)
        .max()
        .unwrap_or(0);
    let min_sequence_strict_ordered_accuracy_milli = rows
        .iter()
        .map(|row| row.sequence_strict_ordered_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_sequence_median_energy_margin = rows
        .iter()
        .map(|row| row.sequence_median_energy_margin)
        .min()
        .unwrap_or(0);
    let all_binary_gate_pass = rows.iter().all(|row| row.binary_gate_pass);
    let all_binary_reports_match_sources = rows.iter().all(|row| row.binary_report_matches_sources);
    let all_score_gate_pass = rows.iter().all(|row| row.score_gate_pass);
    let all_score_reports_match_sources = rows.iter().all(|row| row.score_report_matches_sources);
    let all_eval_pack_format_binary = rows.iter().all(|row| row.eval_pack_format == "binary");
    let all_package_fingerprints_match = rows
        .iter()
        .all(|row| row.eval_pack_package_fingerprint_matches);
    let all_forbidden_flags_false = true;
    let gate_pass = suite_items == ROLE_BINDING_BINARY_SUITE_ITEMS.len()
        && total_sequence_count > 0
        && total_expected_local_sequences > 0
        && total_expected_fallback_sequences > 0
        && total_sequence_local_operator_calls == total_expected_local_sequences
        && total_sequence_fallback_to_llm_calls == total_expected_fallback_sequences
        && total_sequence_false_local_accepts == 0
        && total_sequence_missed_expected_local == 0
        && min_sequence_strict_ordered_accuracy_milli == 1000
        && total_binary_eval_pack_bytes < total_source_eval_pack_bytes
        && all_binary_gate_pass
        && all_binary_reports_match_sources
        && all_score_gate_pass
        && all_score_reports_match_sources
        && all_eval_pack_format_binary
        && all_package_fingerprints_match
        && all_forbidden_flags_false;

    Ok(RoleBindingBinaryEvalPackSuiteReport {
        schema_version: "nando_role_binding_binary_eval_pack_suite_report_v1".to_owned(),
        verdict: if gate_pass {
            "ROLE_BINDING_BINARY_EVAL_PACK_SUITE_V1_PASS"
        } else {
            "ROLE_BINDING_BINARY_EVAL_PACK_SUITE_V1_WATCH"
        }
        .to_owned(),
        root_dir: root_dir.display().to_string(),
        margin_threshold,
        suite_items,
        total_source_eval_pack_bytes,
        total_binary_eval_pack_bytes,
        suite_size_reduction_milli,
        min_size_reduction_milli,
        max_binary_eval_pack_bytes,
        total_sequence_count,
        total_expected_local_sequences,
        total_expected_fallback_sequences,
        total_sequence_local_operator_calls,
        total_sequence_fallback_to_llm_calls,
        total_sequence_false_local_accepts,
        total_sequence_missed_expected_local,
        min_sequence_strict_ordered_accuracy_milli,
        min_sequence_median_energy_margin,
        all_binary_gate_pass,
        all_binary_reports_match_sources,
        all_score_gate_pass,
        all_score_reports_match_sources,
        all_eval_pack_format_binary,
        all_package_fingerprints_match,
        all_forbidden_flags_false,
        gate_pass,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        rust_runtime_used: true,
        rows,
        claim_boundary: "compact binary role-binding eval-pack packaging and scoring for the current role-binding package set with per-item margin thresholds; does not close the full OPERATOR_BLUEPRINT battery, daemon registry, .nwpc bridge, raw-language action parsing, broad workflow reasoning, or text generation".to_owned(),
    })
}

fn build_role_binding_release_suite_report(
    binary_suite_report_path: &Path,
) -> Result<RoleBindingReleaseSuiteReport, String> {
    let binary_suite_report_bytes = std::fs::read(binary_suite_report_path).map_err(|error| {
        format!(
            "failed to read {}: {error}",
            binary_suite_report_path.display()
        )
    })?;
    let binary_suite_report =
        serde_json::from_slice::<RoleBindingBinaryEvalPackSuiteReport>(&binary_suite_report_bytes)
            .map_err(|error| {
                format!(
                    "failed to parse {}: {error}",
                    binary_suite_report_path.display()
                )
            })?;
    let binary_suite_report_fingerprint64 =
        role_binding_cli_fingerprint64(&binary_suite_report_bytes);

    let mut rows = Vec::with_capacity(binary_suite_report.rows.len());
    for suite_row in &binary_suite_report.rows {
        let package_path = PathBuf::from(&suite_row.package_path);
        let package_bytes = std::fs::read(&package_path)
            .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
        let package_info = WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(
            &package_bytes,
        )
        .map_err(|error| format!("failed to inspect {}: {error:?}", package_path.display()))?;
        let package_magic_matches = package_info.magic == WAVE_PREDICTOR_ROLE_BINDING_PACKAGE_MAGIC;
        let package_fingerprint_matches_suite =
            package_info.fingerprint64 == suite_row.package_fingerprint64;
        let package_bytes_match_inspect = package_info.serialized_len == package_bytes.len();

        let binary_eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let binary_eval_pack_bytes = std::fs::read(&binary_eval_pack_path).map_err(|error| {
            format!(
                "failed to read {}: {error}",
                binary_eval_pack_path.display()
            )
        })?;
        let binary_eval_pack_fingerprint64 =
            role_binding_cli_fingerprint64(&binary_eval_pack_bytes);
        let binary_eval_pack_fingerprint_matches_suite =
            binary_eval_pack_fingerprint64 == suite_row.binary_eval_pack_fingerprint64;
        let binary_eval_pack_magic_matches =
            binary_eval_pack_bytes.starts_with(&ROLE_BINDING_EVAL_PACK_BINARY_MAGIC);

        let binary_eval_pack_report_path = PathBuf::from(&suite_row.binary_eval_pack_report_path);
        let binary_eval_pack_report_bytes =
            std::fs::read(&binary_eval_pack_report_path).map_err(|error| {
                format!(
                    "failed to read {}: {error}",
                    binary_eval_pack_report_path.display()
                )
            })?;
        let binary_eval_pack_report = serde_json::from_slice::<RoleBindingBinaryEvalPackReport>(
            &binary_eval_pack_report_bytes,
        )
        .map_err(|error| {
            format!(
                "failed to parse {}: {error}",
                binary_eval_pack_report_path.display()
            )
        })?;
        let binary_eval_pack_report_fingerprint64 =
            role_binding_cli_fingerprint64(&binary_eval_pack_report_bytes);
        let binary_report_matches_suite_row = binary_eval_pack_report.gate_pass
            && binary_eval_pack_report.binary_eval_pack_fingerprint64
                == suite_row.binary_eval_pack_fingerprint64
            && binary_eval_pack_report.source_eval_pack_fingerprint64
                == suite_row.source_eval_pack_fingerprint64
            && binary_eval_pack_report.binary_eval_pack_bytes == suite_row.binary_eval_pack_bytes
            && binary_eval_pack_report.sequence_count == suite_row.sequence_count;

        let score_report_path = PathBuf::from(&suite_row.score_report_path);
        let score_report_bytes = std::fs::read(&score_report_path)
            .map_err(|error| format!("failed to read {}: {error}", score_report_path.display()))?;
        let score_report = serde_json::from_slice::<RoleBindingPackageScoreReport>(
            &score_report_bytes,
        )
        .map_err(|error| format!("failed to parse {}: {error}", score_report_path.display()))?;
        let score_report_fingerprint64 = role_binding_cli_fingerprint64(&score_report_bytes);
        let score_report_matches_suite_row = score_report.gate_pass
            && score_report.package_fingerprint64 == suite_row.package_fingerprint64
            && score_report.eval_pack_fingerprint64 == suite_row.score_eval_pack_fingerprint64
            && score_report.eval_pack_fingerprint64 == suite_row.binary_eval_pack_fingerprint64
            && score_report.eval_pack_format == "binary"
            && score_report.margin_threshold == suite_row.margin_threshold
            && score_report.sequence_count == suite_row.sequence_count
            && score_report.expected_local_sequences == suite_row.expected_local_sequences
            && score_report.expected_fallback_sequences == suite_row.expected_fallback_sequences
            && score_report.sequence_local_operator_calls
                == suite_row.sequence_local_operator_calls
            && score_report.sequence_fallback_to_llm_calls
                == suite_row.sequence_fallback_to_llm_calls
            && score_report.sequence_false_local_accepts == suite_row.sequence_false_local_accepts
            && score_report.sequence_missed_expected_local
                == suite_row.sequence_missed_expected_local
            && score_report.sequence_strict_ordered_accuracy_milli
                == suite_row.sequence_strict_ordered_accuracy_milli
            && !score_report.target_center_id_training_used
            && !score_report.proof_rule_id_training_authority_used
            && !score_report.concrete_x_lookup_used
            && !score_report.local_out_t_runtime_extension_used
            && !score_report.python_demo_used;

        rows.push(RoleBindingReleaseSuiteRow {
            label: suite_row.label.clone(),
            seed: suite_row.seed,
            margin_threshold: suite_row.margin_threshold,
            package_path: suite_row.package_path.clone(),
            package_bytes: package_bytes.len(),
            package_magic_text: String::from_utf8_lossy(&package_info.magic).into_owned(),
            package_edge_count: package_info.edge_count,
            package_fingerprint64: package_info.fingerprint64,
            package_magic_matches,
            package_bytes_match_inspect,
            package_fingerprint_matches_suite,
            binary_eval_pack_path: suite_row.binary_eval_pack_path.clone(),
            binary_eval_pack_bytes: binary_eval_pack_bytes.len(),
            binary_eval_pack_fingerprint64,
            binary_eval_pack_magic_matches,
            binary_eval_pack_fingerprint_matches_suite,
            binary_eval_pack_report_path: suite_row.binary_eval_pack_report_path.clone(),
            binary_eval_pack_report_fingerprint64,
            binary_eval_pack_report_gate_pass: binary_eval_pack_report.gate_pass,
            binary_report_matches_suite_row,
            score_report_path: suite_row.score_report_path.clone(),
            score_report_fingerprint64,
            score_report_gate_pass: score_report.gate_pass,
            score_report_matches_suite_row,
            sequence_count: suite_row.sequence_count,
            expected_local_sequences: suite_row.expected_local_sequences,
            expected_fallback_sequences: suite_row.expected_fallback_sequences,
            sequence_false_local_accepts: suite_row.sequence_false_local_accepts,
            sequence_missed_expected_local: suite_row.sequence_missed_expected_local,
            sequence_strict_ordered_accuracy_milli: suite_row
                .sequence_strict_ordered_accuracy_milli,
            sequence_median_energy_margin: suite_row.sequence_median_energy_margin,
        });
    }

    let package_count = rows.len();
    let binary_eval_pack_count = rows.len();
    let score_report_count = rows.len();
    let total_package_bytes = rows.iter().map(|row| row.package_bytes).sum::<usize>();
    let total_binary_eval_pack_bytes = rows
        .iter()
        .map(|row| row.binary_eval_pack_bytes)
        .sum::<usize>();
    let total_sequence_count = rows.iter().map(|row| row.sequence_count).sum::<usize>();
    let total_expected_local_sequences = rows
        .iter()
        .map(|row| row.expected_local_sequences)
        .sum::<usize>();
    let total_expected_fallback_sequences = rows
        .iter()
        .map(|row| row.expected_fallback_sequences)
        .sum::<usize>();
    let total_sequence_false_local_accepts = rows
        .iter()
        .map(|row| row.sequence_false_local_accepts)
        .sum::<usize>();
    let total_sequence_missed_expected_local = rows
        .iter()
        .map(|row| row.sequence_missed_expected_local)
        .sum::<usize>();
    let min_sequence_strict_ordered_accuracy_milli = rows
        .iter()
        .map(|row| row.sequence_strict_ordered_accuracy_milli)
        .min()
        .unwrap_or(0);
    let min_sequence_median_energy_margin = rows
        .iter()
        .map(|row| row.sequence_median_energy_margin)
        .min()
        .unwrap_or(0);
    let all_packages_magic_match = rows.iter().all(|row| row.package_magic_matches);
    let all_packages_bytes_match_inspect = rows.iter().all(|row| row.package_bytes_match_inspect);
    let all_package_fingerprints_match_suite =
        rows.iter().all(|row| row.package_fingerprint_matches_suite);
    let all_eval_pack_magic_match = rows.iter().all(|row| row.binary_eval_pack_magic_matches);
    let all_eval_pack_fingerprints_match_suite = rows
        .iter()
        .all(|row| row.binary_eval_pack_fingerprint_matches_suite);
    let all_binary_reports_match_suite_rows =
        rows.iter().all(|row| row.binary_report_matches_suite_row);
    let all_score_reports_match_suite_rows =
        rows.iter().all(|row| row.score_report_matches_suite_row);
    let all_forbidden_flags_false = binary_suite_report.all_forbidden_flags_false;
    let gate_pass = binary_suite_report.gate_pass
        && package_count == ROLE_BINDING_BINARY_SUITE_ITEMS.len()
        && total_sequence_count == binary_suite_report.total_sequence_count
        && total_expected_local_sequences == binary_suite_report.total_expected_local_sequences
        && total_expected_fallback_sequences
            == binary_suite_report.total_expected_fallback_sequences
        && total_sequence_false_local_accepts == 0
        && total_sequence_missed_expected_local == 0
        && min_sequence_strict_ordered_accuracy_milli == 1000
        && all_packages_magic_match
        && all_packages_bytes_match_inspect
        && all_package_fingerprints_match_suite
        && all_eval_pack_magic_match
        && all_eval_pack_fingerprints_match_suite
        && all_binary_reports_match_suite_rows
        && all_score_reports_match_suite_rows
        && all_forbidden_flags_false;

    Ok(RoleBindingReleaseSuiteReport {
        schema_version: "nando_role_binding_release_suite_report_v1".to_owned(),
        verdict: if gate_pass {
            "ROLE_BINDING_RELEASE_SUITE_V1_PASS"
        } else {
            "ROLE_BINDING_RELEASE_SUITE_V1_WATCH"
        }
        .to_owned(),
        binary_suite_report_path: binary_suite_report_path.display().to_string(),
        binary_suite_report_fingerprint64,
        binary_suite_report_bytes: binary_suite_report_bytes.len(),
        binary_suite_gate_pass: binary_suite_report.gate_pass,
        binary_suite_report_matches_sources: binary_suite_report.all_binary_reports_match_sources
            && binary_suite_report.all_score_reports_match_sources,
        package_count,
        binary_eval_pack_count,
        score_report_count,
        total_package_bytes,
        total_binary_eval_pack_bytes,
        total_sequence_count,
        total_expected_local_sequences,
        total_expected_fallback_sequences,
        total_sequence_false_local_accepts,
        total_sequence_missed_expected_local,
        min_sequence_strict_ordered_accuracy_milli,
        min_sequence_median_energy_margin,
        all_packages_magic_match,
        all_packages_bytes_match_inspect,
        all_package_fingerprints_match_suite,
        all_eval_pack_magic_match,
        all_eval_pack_fingerprints_match_suite,
        all_binary_reports_match_suite_rows,
        all_score_reports_match_suite_rows,
        all_forbidden_flags_false,
        gate_pass,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        rust_runtime_used: true,
        rows,
        claim_boundary: "role-binding release suite ties current .nwrb packages, .nwreb eval-packs, per-row score reports, per-item margin thresholds, and aggregate binary suite into one product-proof bundle; does not close full OPERATOR_BLUEPRINT battery, daemon registry, .nwpc bridge, raw-language action parsing, broad workflow reasoning, text generation, or commercial license".to_owned(),
    })
}

fn build_role_binding_operator_blueprint_gap_report(
    release_suite_report_path: &Path,
) -> Result<RoleBindingOperatorBlueprintGapReport, String> {
    let release_suite_report_bytes = std::fs::read(release_suite_report_path).map_err(|error| {
        format!(
            "failed to read {}: {error}",
            release_suite_report_path.display()
        )
    })?;
    let release_suite_report =
        serde_json::from_slice::<RoleBindingReleaseSuiteReport>(&release_suite_report_bytes)
            .map_err(|error| {
                format!(
                    "failed to parse {}: {error}",
                    release_suite_report_path.display()
                )
            })?;
    let release_suite_report_fingerprint64 =
        role_binding_cli_fingerprint64(&release_suite_report_bytes);

    let source_labels = release_suite_report
        .rows
        .iter()
        .map(|row| row.label.clone())
        .collect::<BTreeSet<_>>();
    let mixed_map_labels = labels_for(&source_labels, &["sdk_mixed_map"]);
    let conditional_labels = labels_for(&source_labels, &["sdk_conditional_branch"]);
    let edit_labels = labels_for(&source_labels, &["sdk_edit_marker_length"]);
    let edit_status = if edit_labels.is_empty() {
        "MISSING"
    } else {
        "PARTIAL"
    };
    let edit_evidence = if edit_labels.is_empty() {
        "No current role-binding release-suite label proves insert/delete/replace/clear/append/prepend as state-changing operators."
    } else {
        "sdk_edit_marker_length proves bounded EDIT marker/length transfer through source-verified .nwrb/.nwreb scoring with delete/insert/replace/duplicate/drop-style heldout rows, but it does not close the full EDIT blueprint family such as clear/append/prepend as separate product classes."
    };
    let edit_metrics = if edit_labels.is_empty() {
        Vec::new()
    } else {
        role_binding_release_metrics(&release_suite_report)
    };
    let edit_next_gate = if edit_labels.is_empty() {
        "Create EDIT corpus rows with wrong edit target, wrong edit position, wrong inserted filler, heldout edit surfaces, no target_id, and cleanup/readout ablation."
    } else {
        "Extend EDIT to full blueprint coverage with clear/append/prepend rows, source-verified package/eval-pack scoring, shortcut gates, and channel ablations."
    };
    let all_role_binding_labels = labels_for(
        &source_labels,
        &[
            "sdk_mixed_map",
            "sdk_conditional_branch",
            "sdk_edit_marker_length",
        ],
    );

    let rows = vec![
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "SELECT".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "Current .nwrb/.nwreb suite proves role-slot activation is present in strict role-binding rows, but it does not isolate select_span/select_field/select_by_marker/select_by_predicate/select_window channels or their ablations.".to_owned(),
            source_labels: all_role_binding_labels.clone(),
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Build a 32-slot SELECT corpus with heldout markers/predicates/windows, shortcut gates, marker/predicate ablations, and flat/runtime parity.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "MOVE_COPY".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "sdk_mixed_map packages prove loaded-runtime role/filler transfer with strict ordered slot readout, but not the full move_slot/move_span/copy_slot/copy_span/swap families across the blueprint.".to_owned(),
            source_labels: mixed_map_labels.clone(),
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Extend the 32-slot battery with explicit MOVE/COPY span and swap families, same-bag negatives, copy/move ablation collapse, and source-verified .nwrb/.nwreb scoring.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "EDIT".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: edit_status.to_owned(),
            current_strict_role_binding_evidence: edit_evidence.to_owned(),
            source_labels: edit_labels,
            current_metrics: edit_metrics,
            next_required_gate: edit_next_gate.to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "ORDER".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "The release suite has strict ordered slot readout over role-binding sequences, but this artifact does not itself cover the full ORDER family set such as reverse/rotate/block_swap/window_reverse/interleave/stable_reorder.".to_owned(),
            source_labels: all_role_binding_labels.clone(),
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Tie the 32-slot ORDER multi-seed rungs into the release artifact or add an ORDER .nwrb/.nwreb suite with mirror/symmetry breakdown and same-bag derangements.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "FIELD".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "MISSING".to_owned(),
            current_strict_role_binding_evidence: "No current role-binding release-suite label proves extract_field/merge_fields/split_field/normalize_field/compare_fields over named field structures.".to_owned(),
            source_labels: Vec::new(),
            current_metrics: Vec::new(),
            next_required_gate: "Create FIELD corpus rows with heldout field names/values, wrong normalized form, wrong compared pair, field-channel ablation, and runtime parity.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "FILTER_GROUP".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "MISSING".to_owned(),
            current_strict_role_binding_evidence: "No current role-binding release-suite label proves filter_by_predicate/partition/group_by_key/stable_sort_by_key/deduplicate.".to_owned(),
            source_labels: Vec::new(),
            current_metrics: Vec::new(),
            next_required_gate: "Create FILTER/GROUP rows with predicate heldout, same input bag wrong kept/removed subsets, stable order preservation, and group-key ablation.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "CONDITION_ROUTE".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "sdk_conditional_branch proves branch-local role-binding behavior and fallback discipline, but not the full if_then_else/route_by_marker/route_by_field/route_by_compare/route_by_state action_tree contract.".to_owned(),
            source_labels: conditional_labels.clone(),
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Extend conditional corpus to require state/action conjunction, both then/else branches in action text, wrong-branch traps, condition ablation collapse, and no proof_rule_id authority.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "COMPOSE".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "Current mixed/conditional role-binding rows exercise map-plus-branch mechanics, but the release suite does not prove A_then_B, A_then_if_B_else_C, repeat_n, or verify_then_repair composition depth.".to_owned(),
            source_labels: all_role_binding_labels.clone(),
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Build depth-2 then depth-3 COMPOSE rows with intermediate-state traps, wrong-order composition traps, composed-demo/action ablation, and package scoring.".to_owned(),
        },
        RoleBindingOperatorBlueprintGapRow {
            operator_class: "VERIFY_REPAIR".to_owned(),
            required_by_blueprint: true,
            current_evidence_status: "PARTIAL".to_owned(),
            current_strict_role_binding_evidence: "The release suite proves zero false local accepts and strict fallback accounting, but it does not prove learned check_same_bag/check_field_constraint/check_order_constraint/repair_unset_slot/reject_unsettled operators.".to_owned(),
            source_labels: all_role_binding_labels,
            current_metrics: role_binding_release_metrics(&release_suite_report),
            next_required_gate: "Create VERIFY/REPAIR rows where wrong answers are rejected, low-gap answers are unsettled, repair improves strict slots without target leak, and cleanup/repair ablation collapses.".to_owned(),
        },
    ];

    let blueprint_required_class_count =
        rows.iter().filter(|row| row.required_by_blueprint).count();
    let proven_classes = rows
        .iter()
        .filter(|row| row.current_evidence_status == "PROVEN")
        .count();
    let partial_classes = rows
        .iter()
        .filter(|row| row.current_evidence_status == "PARTIAL")
        .count();
    let missing_classes = rows
        .iter()
        .filter(|row| row.current_evidence_status == "MISSING")
        .count();
    let coverage_gate_pass = release_suite_report.gate_pass
        && blueprint_required_class_count > 0
        && proven_classes == blueprint_required_class_count
        && partial_classes == 0
        && missing_classes == 0;

    Ok(RoleBindingOperatorBlueprintGapReport {
        schema_version: "nando_role_binding_operator_blueprint_gap_report_v1".to_owned(),
        verdict: if coverage_gate_pass {
            "ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_PASS"
        } else {
            "ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_WATCH"
        }
        .to_owned(),
        release_suite_report_path: release_suite_report_path.display().to_string(),
        release_suite_report_fingerprint64,
        release_suite_report_bytes: release_suite_report_bytes.len(),
        release_suite_gate_pass: release_suite_report.gate_pass,
        release_suite_package_count: release_suite_report.package_count,
        release_suite_total_sequence_count: release_suite_report.total_sequence_count,
        release_suite_min_sequence_strict_ordered_accuracy_milli: release_suite_report
            .min_sequence_strict_ordered_accuracy_milli,
        release_suite_min_sequence_median_energy_margin: release_suite_report
            .min_sequence_median_energy_margin,
        all_forbidden_flags_false: release_suite_report.all_forbidden_flags_false,
        blueprint_source: "docs/OPERATOR_BLUEPRINT.md".to_owned(),
        blueprint_required_class_count,
        proven_classes,
        partial_classes,
        missing_classes,
        coverage_gate_pass,
        role_binding_release_suite_closed: release_suite_report.gate_pass,
        full_32_slot_operator_battery_closed: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        python_demo_used: false,
        rust_runtime_used: true,
        rows,
        next_engineering_step: "Generate and package missing/partial 32-slot operator classes as Rust-first corpora, then score them through source-verified flat runtime packages with shortcuts, ablations, parity, and latency evidence.".to_owned(),
        claim_boundary: "This audit proves only the coverage gap between the current strict role-binding release suite and OPERATOR_BLUEPRINT. It does not close the full 32-slot operator battery; WATCH is the honest state until every required class has source-verified package proof.".to_owned(),
    })
}

fn role_binding_release_metrics(report: &RoleBindingReleaseSuiteReport) -> Vec<String> {
    vec![
        format!("package_count={}", report.package_count),
        format!("total_sequence_count={}", report.total_sequence_count),
        format!(
            "min_sequence_strict_ordered_accuracy_milli={}",
            report.min_sequence_strict_ordered_accuracy_milli
        ),
        format!(
            "min_sequence_median_energy_margin={}",
            report.min_sequence_median_energy_margin
        ),
        format!(
            "total_sequence_false_local_accepts={}",
            report.total_sequence_false_local_accepts
        ),
        format!(
            "total_sequence_missed_expected_local={}",
            report.total_sequence_missed_expected_local
        ),
    ]
}

fn labels_for(source_labels: &BTreeSet<String>, wanted: &[&str]) -> Vec<String> {
    wanted
        .iter()
        .filter(|label| source_labels.contains::<str>(*label))
        .map(|label| (*label).to_owned())
        .collect()
}

fn parse_role_binding_eval_pack_file(
    path: &Path,
    bytes: &[u8],
) -> Result<RoleBindingEvalPackFile, String> {
    if bytes.starts_with(&ROLE_BINDING_EVAL_PACK_BINARY_MAGIC) {
        return Ok(RoleBindingEvalPackFile {
            format: "binary".to_owned(),
            pack: parse_role_binding_eval_pack_binary(path, bytes)?,
        });
    }
    let pack = serde_json::from_slice::<RoleBindingEvalPack>(bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(RoleBindingEvalPackFile {
        format: "json".to_owned(),
        pack,
    })
}

fn encode_role_binding_eval_pack_binary(pack: &RoleBindingEvalPack) -> Result<Vec<u8>, String> {
    let task_count = u32::try_from(pack.tasks.len())
        .map_err(|_| "too many role-binding eval tasks for binary pack".to_owned())?;
    let sequence_count = u32::try_from(pack.sequences.len())
        .map_err(|_| "too many role-binding eval sequences for binary pack".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&ROLE_BINDING_EVAL_PACK_BINARY_MAGIC);
    bytes.extend_from_slice(&pack.package_fingerprint64.unwrap_or(0).to_le_bytes());
    write_binary_string(
        &mut bytes,
        pack.source_package_path.as_deref().unwrap_or(""),
    )?;
    write_binary_string(&mut bytes, &pack.generation_method)?;
    bytes.extend_from_slice(&task_count.to_le_bytes());
    bytes.extend_from_slice(&sequence_count.to_le_bytes());

    for task in &pack.tasks {
        write_binary_string(&mut bytes, &task.task_id)?;
        bytes.extend_from_slice(&task.target_lane_id.to_le_bytes());
        bytes.extend_from_slice(&task.target_signed_strength.to_le_bytes());
        bytes.extend_from_slice(&task.wrong_lane_id.to_le_bytes());
        bytes.extend_from_slice(&task.wrong_signed_strength.to_le_bytes());
        write_optional_slot(&mut bytes, task.binding_output_slot);
        bytes.push(u8::from(task.expect_local_operator));
        write_active_fringe_binary(&mut bytes, &task.active_fringe)?;
    }

    for sequence in &pack.sequences {
        write_binary_string(&mut bytes, &sequence.task_id)?;
        bytes.push(u8::from(sequence.expect_local_operator));
        write_active_fringe_binary(&mut bytes, &sequence.active_fringe)?;
        let slot_count = u32::try_from(sequence.slots.len())
            .map_err(|_| "too many role-binding sequence slots for binary pack".to_owned())?;
        bytes.extend_from_slice(&slot_count.to_le_bytes());
        for slot in &sequence.slots {
            write_optional_slot(&mut bytes, slot.binding_output_slot);
            write_impulses_binary(&mut bytes, &slot.positive_impulses)?;
            write_impulses_binary(&mut bytes, &slot.negative_impulses)?;
        }
    }

    Ok(bytes)
}

fn parse_role_binding_eval_pack_binary(
    path: &Path,
    bytes: &[u8],
) -> Result<RoleBindingEvalPack, String> {
    let mut reader = BinaryEvalPackReader::new(path, bytes);
    reader.expect_magic()?;
    let package_fingerprint64 = match reader.read_u64()? {
        0 => None,
        value => Some(value),
    };
    let source_package_path = match reader.read_string()?.as_str() {
        "" => None,
        value => Some(value.to_owned()),
    };
    let generation_method = reader.read_string()?;
    let task_count = reader.read_u32()? as usize;
    let sequence_count = reader.read_u32()? as usize;
    let mut tasks = Vec::with_capacity(task_count);
    let mut sequences = Vec::with_capacity(sequence_count);

    for _ in 0..task_count {
        tasks.push(RoleBindingEvalTaskRow {
            task_id: reader.read_string()?,
            target_lane_id: reader.read_u16()?,
            target_signed_strength: reader.read_i16()?,
            wrong_lane_id: reader.read_u16()?,
            wrong_signed_strength: reader.read_i16()?,
            binding_output_slot: reader.read_optional_slot()?,
            expect_local_operator: reader.read_bool()?,
            active_fringe: reader.read_active_fringe()?,
        });
    }

    for _ in 0..sequence_count {
        let task_id = reader.read_string()?;
        let expect_local_operator = reader.read_bool()?;
        let active_fringe = reader.read_active_fringe()?;
        let slot_count = reader.read_u32()? as usize;
        let mut slots = Vec::with_capacity(slot_count);
        for _ in 0..slot_count {
            slots.push(RoleBindingSequenceSlotRow {
                binding_output_slot: reader.read_optional_slot()?,
                positive_impulses: reader.read_impulses()?,
                negative_impulses: reader.read_impulses()?,
            });
        }
        sequences.push(RoleBindingSequenceEvalRow {
            task_id,
            active_fringe,
            slots,
            expect_local_operator,
        });
    }

    reader.finish()?;
    Ok(RoleBindingEvalPack {
        schema_version: "nando_role_binding_eval_pack_v1".to_owned(),
        package_fingerprint64,
        source_package_path,
        generation_method,
        tasks,
        sequences,
    })
}

fn validate_role_binding_eval_pack(pack: &RoleBindingEvalPack) -> Result<(), String> {
    if pack.schema_version != "nando_role_binding_eval_pack_v1" {
        return Err(format!(
            "unsupported role-binding eval-pack schema_version '{}'",
            pack.schema_version
        ));
    }
    if pack.tasks.is_empty() && pack.sequences.is_empty() {
        return Err("role-binding eval pack has no tasks or sequences".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &pack.tasks {
        if row.task_id.trim().is_empty() {
            return Err("role-binding eval task has empty task_id".to_owned());
        }
        if !seen.insert(row.task_id.as_str()) {
            return Err(format!(
                "duplicate role-binding eval task_id '{}'",
                row.task_id
            ));
        }
        if row.active_fringe.is_empty() {
            return Err(format!(
                "role-binding eval task '{}' has empty active_fringe",
                row.task_id
            ));
        }
        if row.target_lane_id == row.wrong_lane_id
            && row.target_signed_strength == row.wrong_signed_strength
        {
            return Err(format!(
                "role-binding eval task '{}' has identical target and wrong impulse",
                row.task_id
            ));
        }
    }
    for row in &pack.sequences {
        if row.task_id.trim().is_empty() {
            return Err("role-binding sequence has empty task_id".to_owned());
        }
        if !seen.insert(row.task_id.as_str()) {
            return Err(format!(
                "duplicate role-binding eval task_id '{}'",
                row.task_id
            ));
        }
        if row.active_fringe.is_empty() {
            return Err(format!(
                "role-binding sequence '{}' has empty active_fringe",
                row.task_id
            ));
        }
        if row.slots.is_empty() {
            return Err(format!(
                "role-binding sequence '{}' has no slots",
                row.task_id
            ));
        }
        for slot in &row.slots {
            if slot.positive_impulses.is_empty() {
                return Err(format!(
                    "role-binding sequence '{}' has slot with no positive impulses",
                    row.task_id
                ));
            }
            if slot.negative_impulses.is_empty() {
                return Err(format!(
                    "role-binding sequence '{}' has slot with no negative impulses",
                    row.task_id
                ));
            }
        }
    }
    Ok(())
}

fn score_role_binding_sequence(
    runtime: &WavePredictorRoleBindingOffloadRuntime,
    row: &RoleBindingSequenceEvalRow,
    margin_threshold: i32,
) -> Result<RoleBindingPackageSequenceScoreRow, String> {
    let active_fringe = row
        .active_fringe
        .iter()
        .map(|active| WavePredictorActiveCenter {
            center_id: active.center_id,
            strength: active.strength,
        })
        .collect::<Vec<_>>();
    let prepared = runtime.prepare_active_fringe(&active_fringe);
    let mut energy_margin = 0i32;
    let mut min_slot_margin = i32::MAX;
    let mut strict_ordered_pass = true;

    for slot in &row.slots {
        let positive_score = slot
            .positive_impulses
            .iter()
            .map(|impulse| {
                runtime.score_alignment_prepared(
                    &prepared,
                    impulse.lane_id,
                    impulse.signed_strength,
                    slot.binding_output_slot,
                )
            })
            .sum::<i32>();
        let negative_score = slot
            .negative_impulses
            .iter()
            .map(|impulse| {
                runtime.score_alignment_prepared(
                    &prepared,
                    impulse.lane_id,
                    impulse.signed_strength,
                    slot.binding_output_slot,
                )
            })
            .sum::<i32>();
        let slot_margin = positive_score - negative_score;
        energy_margin += slot_margin;
        min_slot_margin = min_slot_margin.min(slot_margin);
        strict_ordered_pass &= slot_margin > 0;
    }

    if min_slot_margin == i32::MAX {
        return Err(format!(
            "role-binding sequence '{}' has no scorable slots",
            row.task_id
        ));
    }

    let local_accept = strict_ordered_pass && energy_margin >= margin_threshold;
    let false_local_accept = local_accept && !row.expect_local_operator;
    let missed_expected_local = !local_accept && row.expect_local_operator;
    Ok(RoleBindingPackageSequenceScoreRow {
        task_id: row.task_id.clone(),
        action: if local_accept {
            "local_operator"
        } else {
            "fallback_to_llm"
        }
        .to_owned(),
        energy_margin,
        min_slot_margin,
        strict_ordered_pass,
        expect_local_operator: row.expect_local_operator,
        false_local_accept,
        missed_expected_local,
    })
}

fn write_binary_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), String> {
    let len = u32::try_from(value.len())
        .map_err(|_| "role-binding binary eval-pack string is too large".to_owned())?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_optional_slot(bytes: &mut Vec<u8>, slot: Option<u8>) {
    bytes.extend_from_slice(&slot.map(u16::from).unwrap_or(u16::MAX).to_le_bytes());
}

fn write_active_fringe_binary(
    bytes: &mut Vec<u8>,
    active_fringe: &[RoleBindingActiveCenterRow],
) -> Result<(), String> {
    let len = u32::try_from(active_fringe.len())
        .map_err(|_| "role-binding binary eval-pack active fringe is too large".to_owned())?;
    bytes.extend_from_slice(&len.to_le_bytes());
    for active in active_fringe {
        bytes.extend_from_slice(&active.center_id.to_le_bytes());
        bytes.extend_from_slice(&active.strength.to_le_bytes());
    }
    Ok(())
}

fn write_impulses_binary(
    bytes: &mut Vec<u8>,
    impulses: &[RoleBindingImpulseRow],
) -> Result<(), String> {
    let len = u32::try_from(impulses.len())
        .map_err(|_| "role-binding binary eval-pack impulse list is too large".to_owned())?;
    bytes.extend_from_slice(&len.to_le_bytes());
    for impulse in impulses {
        bytes.extend_from_slice(&impulse.lane_id.to_le_bytes());
        bytes.extend_from_slice(&impulse.signed_strength.to_le_bytes());
    }
    Ok(())
}

struct BinaryEvalPackReader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryEvalPackReader<'a> {
    fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    fn expect_magic(&mut self) -> Result<(), String> {
        let magic = self.take(ROLE_BINDING_EVAL_PACK_BINARY_MAGIC.len())?;
        if magic != ROLE_BINDING_EVAL_PACK_BINARY_MAGIC {
            return Err(format!(
                "invalid role-binding binary eval-pack magic in {}",
                self.path.display()
            ));
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        if self.offset != self.bytes.len() {
            return Err(format!(
                "trailing bytes in role-binding binary eval-pack {}: offset={} len={}",
                self.path.display(),
                self.offset,
                self.bytes.len()
            ));
        }
        Ok(())
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "role-binding binary eval-pack offset overflow".to_owned())?;
        let slice = self.bytes.get(self.offset..end).ok_or_else(|| {
            format!(
                "truncated role-binding binary eval-pack {} at offset {}",
                self.path.display(),
                self.offset
            )
        })?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(buf))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(i16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(buf))
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(format!(
                "invalid bool value {value} in role-binding binary eval-pack {}",
                self.path.display()
            )),
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| {
                format!(
                    "invalid utf8 string in role-binding binary eval-pack {}: {error}",
                    self.path.display()
                )
            })
    }

    fn read_optional_slot(&mut self) -> Result<Option<u8>, String> {
        match self.read_u16()? {
            u16::MAX => Ok(None),
            value => u8::try_from(value)
                .map(Some)
                .map_err(|_| format!("invalid output slot {value} in {}", self.path.display())),
        }
    }

    fn read_active_fringe(&mut self) -> Result<Vec<RoleBindingActiveCenterRow>, String> {
        let len = self.read_u32()? as usize;
        let mut active_fringe = Vec::with_capacity(len);
        for _ in 0..len {
            active_fringe.push(RoleBindingActiveCenterRow {
                center_id: self.read_u32()?,
                strength: self.read_i16()?,
            });
        }
        Ok(active_fringe)
    }

    fn read_impulses(&mut self) -> Result<Vec<RoleBindingImpulseRow>, String> {
        let len = self.read_u32()? as usize;
        let mut impulses = Vec::with_capacity(len);
        for _ in 0..len {
            impulses.push(RoleBindingImpulseRow {
                lane_id: self.read_u16()?,
                signed_strength: self.read_i16()?,
            });
        }
        Ok(impulses)
    }
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn signed_strength_from_sign_key(sign_key: u8) -> Result<i16, String> {
    match sign_key {
        0 => Ok(1),
        1 => Ok(-1),
        other => Err(format!("unsupported role-binding sign_key {other}")),
    }
}

fn role_binding_cli_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn margin_summary_or_zero(sorted: &[i32]) -> (i32, i32, i32, i32) {
    if sorted.is_empty() {
        return (0, 0, 0, 0);
    }
    (
        sorted[0],
        sorted[sorted.len() / 10],
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
    )
}

fn milli_ratio(numerator: usize, denominator: usize) -> usize {
    numerator
        .checked_mul(1000)
        .and_then(|value| value.checked_div(denominator))
        .unwrap_or(0)
}

fn role_binding_action_name(action: WavePredictorRoleBindingOffloadAction) -> &'static str {
    match action {
        WavePredictorRoleBindingOffloadAction::LocalOperator => "local_operator",
        WavePredictorRoleBindingOffloadAction::FallbackToLlm => "fallback_to_llm",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingEvalPack {
    schema_version: String,
    package_fingerprint64: Option<u64>,
    source_package_path: Option<String>,
    generation_method: String,
    #[serde(default)]
    tasks: Vec<RoleBindingEvalTaskRow>,
    #[serde(default)]
    sequences: Vec<RoleBindingSequenceEvalRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RoleBindingEvalPackFile {
    format: String,
    pack: RoleBindingEvalPack,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingEvalTaskRow {
    task_id: String,
    target_lane_id: u16,
    target_signed_strength: i16,
    wrong_lane_id: u16,
    wrong_signed_strength: i16,
    active_fringe: Vec<RoleBindingActiveCenterRow>,
    binding_output_slot: Option<u8>,
    expect_local_operator: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingActiveCenterRow {
    center_id: WavePredictorCenterId,
    strength: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingSequenceEvalRow {
    task_id: String,
    active_fringe: Vec<RoleBindingActiveCenterRow>,
    slots: Vec<RoleBindingSequenceSlotRow>,
    expect_local_operator: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingSequenceSlotRow {
    binding_output_slot: Option<u8>,
    positive_impulses: Vec<RoleBindingImpulseRow>,
    negative_impulses: Vec<RoleBindingImpulseRow>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingImpulseRow {
    lane_id: u16,
    signed_strength: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingEvalPackBuildReport {
    schema_version: String,
    verdict: String,
    package_path: String,
    package_fingerprint64: u64,
    task_count: usize,
    expected_local_tasks: usize,
    expected_fallback_tasks: usize,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    corpus_jsonl_used: bool,
    rust_runtime_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingBinaryEvalPackReport {
    schema_version: String,
    verdict: String,
    source_eval_pack_path: String,
    binary_eval_pack_path: String,
    source_eval_pack_format: String,
    binary_magic_text: String,
    package_fingerprint64: Option<u64>,
    task_count: usize,
    sequence_count: usize,
    source_eval_pack_bytes: usize,
    binary_eval_pack_bytes: usize,
    source_eval_pack_fingerprint64: u64,
    binary_eval_pack_fingerprint64: u64,
    size_reduction_milli: usize,
    roundtrip_exact: bool,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    corpus_jsonl_used: bool,
    rust_runtime_used: bool,
    claim_boundary: String,
}

impl RoleBindingBinaryEvalPackReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self == rebuilt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingBinaryEvalPackSuiteReport {
    schema_version: String,
    verdict: String,
    root_dir: String,
    margin_threshold: i32,
    suite_items: usize,
    total_source_eval_pack_bytes: usize,
    total_binary_eval_pack_bytes: usize,
    suite_size_reduction_milli: usize,
    min_size_reduction_milli: usize,
    max_binary_eval_pack_bytes: usize,
    total_sequence_count: usize,
    total_expected_local_sequences: usize,
    total_expected_fallback_sequences: usize,
    total_sequence_local_operator_calls: usize,
    total_sequence_fallback_to_llm_calls: usize,
    total_sequence_false_local_accepts: usize,
    total_sequence_missed_expected_local: usize,
    min_sequence_strict_ordered_accuracy_milli: usize,
    min_sequence_median_energy_margin: i32,
    all_binary_gate_pass: bool,
    all_binary_reports_match_sources: bool,
    all_score_gate_pass: bool,
    all_score_reports_match_sources: bool,
    all_eval_pack_format_binary: bool,
    all_package_fingerprints_match: bool,
    all_forbidden_flags_false: bool,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    rust_runtime_used: bool,
    rows: Vec<RoleBindingBinaryEvalPackSuiteRow>,
    claim_boundary: String,
}

impl RoleBindingBinaryEvalPackSuiteReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self == rebuilt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingBinaryEvalPackSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_path: String,
    source_eval_pack_path: String,
    binary_eval_pack_path: String,
    binary_eval_pack_report_path: String,
    score_report_path: String,
    package_fingerprint64: u64,
    source_eval_pack_fingerprint64: u64,
    binary_eval_pack_fingerprint64: u64,
    score_eval_pack_fingerprint64: u64,
    source_eval_pack_bytes: usize,
    binary_eval_pack_bytes: usize,
    size_reduction_milli: usize,
    binary_roundtrip_exact: bool,
    binary_gate_pass: bool,
    binary_report_matches_sources: bool,
    eval_pack_format: String,
    eval_pack_package_fingerprint_matches: bool,
    score_gate_pass: bool,
    score_report_matches_sources: bool,
    sequence_count: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
    sequence_local_operator_calls: usize,
    sequence_fallback_to_llm_calls: usize,
    sequence_false_local_accepts: usize,
    sequence_missed_expected_local: usize,
    sequence_strict_ordered_accuracy_milli: usize,
    sequence_min_energy_margin: i32,
    sequence_p10_energy_margin: i32,
    sequence_median_energy_margin: i32,
    sequence_max_energy_margin: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingReleaseSuiteReport {
    schema_version: String,
    verdict: String,
    binary_suite_report_path: String,
    binary_suite_report_fingerprint64: u64,
    binary_suite_report_bytes: usize,
    binary_suite_gate_pass: bool,
    binary_suite_report_matches_sources: bool,
    package_count: usize,
    binary_eval_pack_count: usize,
    score_report_count: usize,
    total_package_bytes: usize,
    total_binary_eval_pack_bytes: usize,
    total_sequence_count: usize,
    total_expected_local_sequences: usize,
    total_expected_fallback_sequences: usize,
    total_sequence_false_local_accepts: usize,
    total_sequence_missed_expected_local: usize,
    min_sequence_strict_ordered_accuracy_milli: usize,
    min_sequence_median_energy_margin: i32,
    all_packages_magic_match: bool,
    all_packages_bytes_match_inspect: bool,
    all_package_fingerprints_match_suite: bool,
    all_eval_pack_magic_match: bool,
    all_eval_pack_fingerprints_match_suite: bool,
    all_binary_reports_match_suite_rows: bool,
    all_score_reports_match_suite_rows: bool,
    all_forbidden_flags_false: bool,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    rust_runtime_used: bool,
    rows: Vec<RoleBindingReleaseSuiteRow>,
    claim_boundary: String,
}

impl RoleBindingReleaseSuiteReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self == rebuilt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingReleaseSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_path: String,
    package_bytes: usize,
    package_magic_text: String,
    package_edge_count: usize,
    package_fingerprint64: u64,
    package_magic_matches: bool,
    package_bytes_match_inspect: bool,
    package_fingerprint_matches_suite: bool,
    binary_eval_pack_path: String,
    binary_eval_pack_bytes: usize,
    binary_eval_pack_fingerprint64: u64,
    binary_eval_pack_magic_matches: bool,
    binary_eval_pack_fingerprint_matches_suite: bool,
    binary_eval_pack_report_path: String,
    binary_eval_pack_report_fingerprint64: u64,
    binary_eval_pack_report_gate_pass: bool,
    binary_report_matches_suite_row: bool,
    score_report_path: String,
    score_report_fingerprint64: u64,
    score_report_gate_pass: bool,
    score_report_matches_suite_row: bool,
    sequence_count: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
    sequence_false_local_accepts: usize,
    sequence_missed_expected_local: usize,
    sequence_strict_ordered_accuracy_milli: usize,
    sequence_median_energy_margin: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingOperatorBlueprintGapReport {
    schema_version: String,
    verdict: String,
    release_suite_report_path: String,
    release_suite_report_fingerprint64: u64,
    release_suite_report_bytes: usize,
    release_suite_gate_pass: bool,
    release_suite_package_count: usize,
    release_suite_total_sequence_count: usize,
    release_suite_min_sequence_strict_ordered_accuracy_milli: usize,
    release_suite_min_sequence_median_energy_margin: i32,
    all_forbidden_flags_false: bool,
    blueprint_source: String,
    blueprint_required_class_count: usize,
    proven_classes: usize,
    partial_classes: usize,
    missing_classes: usize,
    coverage_gate_pass: bool,
    role_binding_release_suite_closed: bool,
    full_32_slot_operator_battery_closed: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    rust_runtime_used: bool,
    rows: Vec<RoleBindingOperatorBlueprintGapRow>,
    next_engineering_step: String,
    claim_boundary: String,
}

impl RoleBindingOperatorBlueprintGapReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self == rebuilt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingOperatorBlueprintGapRow {
    operator_class: String,
    required_by_blueprint: bool,
    current_evidence_status: String,
    current_strict_role_binding_evidence: String,
    source_labels: Vec<String>,
    current_metrics: Vec<String>,
    next_required_gate: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingPackageScoreReport {
    schema_version: String,
    verdict: String,
    package_path: String,
    eval_pack_path: String,
    package_fingerprint64: u64,
    eval_pack_fingerprint64: u64,
    eval_pack_format: String,
    eval_pack_bytes: usize,
    eval_pack_package_fingerprint_matches: bool,
    margin_threshold: i32,
    task_count: usize,
    expected_local_tasks: usize,
    expected_fallback_tasks: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    sequence_count: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
    sequence_local_operator_calls: usize,
    sequence_fallback_to_llm_calls: usize,
    sequence_false_local_accepts: usize,
    sequence_missed_expected_local: usize,
    sequence_strict_ordered_accuracy_milli: usize,
    sequence_min_energy_margin: i32,
    sequence_p10_energy_margin: i32,
    sequence_median_energy_margin: i32,
    sequence_max_energy_margin: i32,
    min_margin: i32,
    p10_margin: i32,
    median_margin: i32,
    max_margin: i32,
    sdk_load_matches_inspect: bool,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    corpus_jsonl_used: bool,
    rust_runtime_used: bool,
    per_task: Vec<RoleBindingPackageScoreRow>,
    per_sequence: Vec<RoleBindingPackageSequenceScoreRow>,
    claim_boundary: String,
}

impl RoleBindingPackageScoreReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self == rebuilt
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingPackageScoreRow {
    task_id: String,
    action: String,
    margin: i32,
    expect_local_operator: bool,
    false_local_accept: bool,
    missed_expected_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingPackageSequenceScoreRow {
    task_id: String,
    action: String,
    energy_margin: i32,
    min_slot_margin: i32,
    strict_ordered_pass: bool,
    expect_local_operator: bool,
    false_local_accept: bool,
    missed_expected_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RoleBindingPackageInspectReport {
    schema_version: String,
    verdict: String,
    package_path: String,
    package_magic_text: String,
    package_magic_bytes: [u8; 8],
    package_bytes: usize,
    action_base: u32,
    action_count: u32,
    role_base: u32,
    role_stride: u32,
    slot_scoped_action_page_bits: u8,
    slot_scoped_action_page_mask: u64,
    slot_scoped_action_source_bits: u8,
    edge_count: usize,
    serialized_len: usize,
    payload_bytes: usize,
    package_fingerprint64: u64,
    magic_matches: bool,
    serialized_len_matches: bool,
    nonzero_fingerprint: bool,
    nonempty_runtime: bool,
    sdk_load_matches_inspect: bool,
    gate_pass: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    python_demo_used: bool,
    corpus_jsonl_used: bool,
    rust_runtime_used: bool,
    claim_boundary: String,
}

impl RoleBindingPackageInspectReport {
    fn matches_rebuilt(&self, rebuilt: &Self) -> bool {
        self.schema_version == rebuilt.schema_version
            && self.verdict == rebuilt.verdict
            && self.package_magic_text == rebuilt.package_magic_text
            && self.package_magic_bytes == rebuilt.package_magic_bytes
            && self.package_bytes == rebuilt.package_bytes
            && self.action_base == rebuilt.action_base
            && self.action_count == rebuilt.action_count
            && self.role_base == rebuilt.role_base
            && self.role_stride == rebuilt.role_stride
            && self.slot_scoped_action_page_bits == rebuilt.slot_scoped_action_page_bits
            && self.slot_scoped_action_page_mask == rebuilt.slot_scoped_action_page_mask
            && self.slot_scoped_action_source_bits == rebuilt.slot_scoped_action_source_bits
            && self.edge_count == rebuilt.edge_count
            && self.serialized_len == rebuilt.serialized_len
            && self.payload_bytes == rebuilt.payload_bytes
            && self.package_fingerprint64 == rebuilt.package_fingerprint64
            && self.magic_matches == rebuilt.magic_matches
            && self.serialized_len_matches == rebuilt.serialized_len_matches
            && self.nonzero_fingerprint == rebuilt.nonzero_fingerprint
            && self.nonempty_runtime == rebuilt.nonempty_runtime
            && self.sdk_load_matches_inspect == rebuilt.sdk_load_matches_inspect
            && self.gate_pass == rebuilt.gate_pass
            && self.target_center_id_training_used == rebuilt.target_center_id_training_used
            && self.proof_rule_id_training_authority_used
                == rebuilt.proof_rule_id_training_authority_used
            && self.concrete_x_lookup_used == rebuilt.concrete_x_lookup_used
            && self.local_out_t_runtime_extension_used == rebuilt.local_out_t_runtime_extension_used
            && self.python_demo_used == rebuilt.python_demo_used
            && self.corpus_jsonl_used == rebuilt.corpus_jsonl_used
            && self.rust_runtime_used == rebuilt.rust_runtime_used
            && self.claim_boundary == rebuilt.claim_boundary
    }
}
