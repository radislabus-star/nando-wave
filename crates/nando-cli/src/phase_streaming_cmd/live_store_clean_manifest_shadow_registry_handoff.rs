use std::path::{Path, PathBuf};

use nando_core::PhaseCenterOffloadRuntime;
use serde::Serialize;
use serde_json::Value;

use super::{
    json_bool, json_string, json_u64, read_json_value, sanitize_file_stem, write_json_file,
};

const DEFAULT_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-handoff-v1.report.json";
const DEFAULT_SHADOW_REGISTRY_DIR: &str =
    "target/nando-wave/streaming/live-store-clean-manifest-shadow-registry-v1";
const DEFAULT_PREPARED_REVIEW_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-prepared-policy-shadow-review-v1-current.report.json";

#[derive(Serialize)]
struct CleanManifestShadowRegistryPackage {
    source_package_path: String,
    registry_package_path: String,
    route_id: u32,
    profile_id: u32,
    threshold_micro: i64,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    registry_copy_exact: bool,
    accepted_for_shadow_registry: bool,
    blockers: Vec<String>,
}

#[derive(Serialize)]
struct CleanManifestShadowRegistryHandoffReport {
    report_kind: &'static str,
    mode: &'static str,
    prepared_review_report_path: String,
    registry_dir: String,
    input_verdict: String,
    input_prepared_shadow_review_allowed: bool,
    input_prepared_shadow_safety_review_passed: bool,
    input_prepared_shadow_hot_latency_passed: bool,
    input_forbidden_flags_clear: bool,
    input_unique_accepts_over_exact_cache: usize,
    input_tokens_saved: u64,
    input_cost_saved_microusd: u64,
    input_false_accepts: usize,
    input_local_accept_events: usize,
    input_memory_worker_p99_score_latency_ns: u64,
    input_prepared_p99_score_latency_ns: u64,
    package_count: usize,
    promoted_package_count: usize,
    blocked_package_count: usize,
    packages: Vec<CleanManifestShadowRegistryPackage>,
    shadow_registry_handoff_allowed: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    shadow_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Copy)]
struct CleanManifestPackageSpec {
    route_id: u32,
    profile_id: u32,
    threshold_micro: i64,
    package_fingerprint64: u64,
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let registry_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SHADOW_REGISTRY_DIR));
    let prepared_review_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_REVIEW_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let prepared = read_json_value(&prepared_review_report_path)?;
    let input_verdict = json_string(&prepared, &["verdict"]).unwrap_or_default();
    let input_prepared_shadow_review_allowed =
        json_bool(&prepared, &["prepared_shadow_review_allowed"]).unwrap_or(false);
    let input_prepared_shadow_safety_review_passed =
        json_bool(&prepared, &["prepared_shadow_safety_review_passed"]).unwrap_or(false);
    let input_prepared_shadow_hot_latency_passed =
        json_bool(&prepared, &["prepared_shadow_hot_latency_passed"]).unwrap_or(false);
    let input_forbidden_flags_clear = prepared
        .get("forbidden_flags")
        .is_some_and(forbidden_flags_all_bool_false);
    let input_unique_accepts_over_exact_cache = json_usize(
        &prepared,
        &["metrics", "unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let input_tokens_saved = json_u64(&prepared, &["metrics", "tokens_saved"]).unwrap_or(0);
    let input_cost_saved_microusd =
        json_u64(&prepared, &["metrics", "cost_saved_microusd"]).unwrap_or(0);
    let input_false_accepts =
        json_usize(&prepared, &["metrics", "false_accepts"]).unwrap_or(usize::MAX);
    let input_local_accept_events =
        json_usize(&prepared, &["metrics", "local_accept_events"]).unwrap_or(usize::MAX);
    let input_memory_worker_p99_score_latency_ns = json_u64(
        &prepared,
        &["latency", "memory_worker_p99_score_latency_ns"],
    )
    .unwrap_or(u64::MAX);
    let input_prepared_p99_score_latency_ns =
        json_u64(&prepared, &["latency", "prepared_p99_score_latency_ns"]).unwrap_or(u64::MAX);
    let policy_path = json_string(&prepared, &["inputs", "policy_path"]).unwrap_or_default();
    let policy = if policy_path.is_empty() {
        Value::Null
    } else {
        read_json_value(Path::new(&policy_path))?
    };
    let source_manifest_path = json_string(&policy, &["source_manifest_path"]).unwrap_or_default();
    let source_manifest = if source_manifest_path.is_empty() {
        Value::Null
    } else {
        read_json_value(Path::new(&source_manifest_path))?
    };
    let package_paths = package_paths_from_prepared_review(&prepared);
    let input_gate_clear = json_string(&prepared, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1")
        && input_verdict
            == "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_V1_PASS"
        && input_prepared_shadow_review_allowed
        && input_prepared_shadow_safety_review_passed
        && input_prepared_shadow_hot_latency_passed
        && input_forbidden_flags_clear
        && input_unique_accepts_over_exact_cache > 0
        && input_tokens_saved > 0
        && input_false_accepts == 0
        && input_local_accept_events == 0
        && input_memory_worker_p99_score_latency_ns <= 1_000
        && input_prepared_p99_score_latency_ns <= 1_000
        && json_bool(&prepared, &["local_accept_enabled"]) == Some(false)
        && json_bool(&prepared, &["market_money_claim_allowed"]) == Some(false)
        && json_bool(&prepared, &["hot_loop", "json_used"]) == Some(false)
        && json_bool(&prepared, &["hot_loop", "btreemap_used"]) == Some(false)
        && json_bool(&prepared, &["hot_loop", "string_route_used"]) == Some(false)
        && json_bool(&prepared, &["hot_loop", "file_io_used"]) == Some(false)
        && json_bool(&prepared, &["hot_loop", "package_compile_used"]) == Some(false)
        && json_bool(&prepared, &["mutation_flags", "registry_mutation_enabled"]) == Some(false)
        && json_bool(
            &prepared,
            &["mutation_flags", "cpu_profile_registry_write_enabled"],
        ) == Some(false)
        && json_bool(
            &prepared,
            &["mutation_flags", "serving_profile_artifact_written"],
        ) == Some(false)
        && json_bool(&prepared, &["mutation_flags", "product_promotion_enabled"]) == Some(false);

    let packages = package_paths
        .iter()
        .map(|path| {
            let spec = clean_manifest_package_spec(path, &source_manifest);
            audit_clean_manifest_package(path, &registry_dir, input_gate_clear, spec)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let promoted_package_count = packages
        .iter()
        .filter(|package| package.accepted_for_shadow_registry)
        .count();
    let package_count = packages.len();
    let blocked_package_count = package_count.saturating_sub(promoted_package_count);
    let shadow_registry_handoff_allowed = input_gate_clear && promoted_package_count > 0;
    let verdict = if shadow_registry_handoff_allowed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_HANDOFF_V1_PASS"
    } else if input_gate_clear {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_HANDOFF_V1_WATCH_NO_PACKAGE"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_HANDOFF_V1_WATCH_INPUT_GATE"
    };

    let report = CleanManifestShadowRegistryHandoffReport {
        report_kind: "phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1",
        mode: "prepared_policy_shadow_review_to_shadow_registry_handoff",
        prepared_review_report_path: prepared_review_report_path.display().to_string(),
        registry_dir: registry_dir.display().to_string(),
        input_verdict,
        input_prepared_shadow_review_allowed,
        input_prepared_shadow_safety_review_passed,
        input_prepared_shadow_hot_latency_passed,
        input_forbidden_flags_clear,
        input_unique_accepts_over_exact_cache,
        input_tokens_saved,
        input_cost_saved_microusd,
        input_false_accepts,
        input_local_accept_events,
        input_memory_worker_p99_score_latency_ns,
        input_prepared_p99_score_latency_ns,
        package_count,
        promoted_package_count,
        blocked_package_count,
        packages,
        shadow_registry_handoff_allowed,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        shadow_registry_mutated: promoted_package_count > 0,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "shadow registry handoff only: copies prepared-review-approved verifier-bound .nwpc packages into a shadow registry; it does not mutate serving registry, enable local_accept, auto-promote runtime behavior, claim market money, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  registry_dir: {}", registry_dir.display());
    println!("  promoted_package_count: {promoted_package_count}");
    println!("  unique_cpu_accepts_over_exact_cache: {input_unique_accepts_over_exact_cache}");
    println!("  false_accepts: {input_false_accepts}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn package_paths_from_prepared_review(prepared: &Value) -> Vec<String> {
    prepared
        .get("profile_refs")
        .and_then(|refs| refs.get("package_paths"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn clean_manifest_package_spec(
    source_package_path: &str,
    source_manifest: &Value,
) -> Option<CleanManifestPackageSpec> {
    source_manifest
        .get("promoted_packages")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|package| {
            let package_path = json_string(package, &["package_path"])?;
            if package_path != source_package_path {
                return None;
            }
            Some(CleanManifestPackageSpec {
                route_id: json_u32(package, &["route_id"])?,
                profile_id: json_u32(package, &["profile_id"])?,
                threshold_micro: json_i64(package, &["threshold_micro"])?,
                package_fingerprint64: json_u64(package, &["package_fingerprint64"])?,
            })
        })
}

fn audit_clean_manifest_package(
    source_package_path: &str,
    registry_dir: &Path,
    input_gate_clear: bool,
    spec: Option<CleanManifestPackageSpec>,
) -> Result<CleanManifestShadowRegistryPackage, String> {
    let source_path = PathBuf::from(source_package_path);
    let source_file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clean-manifest-package.nwpc");
    let registry_file_name = format!(
        "{}.shadow-ready.nwpc",
        sanitize_file_stem(source_file_name.trim_end_matches(".nwpc"))
    );
    let registry_package_path = registry_dir.join(registry_file_name);
    let mut blockers = Vec::new();
    if !input_gate_clear {
        blockers.push("prepared_review_input_gate_not_clear".to_owned());
    }
    if source_package_path.is_empty() {
        blockers.push("missing_source_package_path".to_owned());
    }
    if source_path.extension().and_then(|value| value.to_str()) != Some("nwpc") {
        blockers.push("source_package_not_nwpc".to_owned());
    }
    if spec.is_none() {
        blockers.push("source_package_missing_clean_manifest_spec".to_owned());
    }

    let mut package_fingerprint64 = 0u64;
    let mut package_bytes_len = 0usize;
    let mut package_records = 0usize;
    let mut package_bytes = Vec::new();
    if blockers.is_empty() {
        package_bytes = std::fs::read(&source_path).map_err(|error| {
            format!(
                "failed to read clean manifest source package '{}': {error}",
                source_path.display()
            )
        })?;
        let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
            .map_err(|error| {
                format!(
                    "failed to inspect clean manifest source package '{}': {error:?}",
                    source_path.display()
                )
            })?;
        package_fingerprint64 = package_info.fingerprint64;
        package_bytes_len = package_bytes.len();
        package_records = package_info.record_count;
        if package_records == 0 {
            blockers.push("source_package_empty_records".to_owned());
        }
        if let Some(spec) = spec
            && package_fingerprint64 != spec.package_fingerprint64
        {
            blockers.push("source_package_fingerprint_mismatch".to_owned());
        }
    }

    blockers.sort();
    blockers.dedup();
    let accepted_for_shadow_registry = blockers.is_empty();
    let mut registry_copy_exact = false;
    if accepted_for_shadow_registry {
        std::fs::create_dir_all(registry_dir).map_err(|error| {
            format!(
                "failed to create clean manifest shadow registry '{}': {error}",
                registry_dir.display()
            )
        })?;
        std::fs::write(&registry_package_path, &package_bytes).map_err(|error| {
            format!(
                "failed to write clean manifest shadow package '{}': {error}",
                registry_package_path.display()
            )
        })?;
        let registry_bytes = std::fs::read(&registry_package_path).map_err(|error| {
            format!(
                "failed to read clean manifest shadow package '{}': {error}",
                registry_package_path.display()
            )
        })?;
        registry_copy_exact = registry_bytes == package_bytes;
        if !registry_copy_exact {
            return Err(format!(
                "clean manifest shadow package '{}' copy mismatch",
                registry_package_path.display()
            ));
        }
    }

    Ok(CleanManifestShadowRegistryPackage {
        source_package_path: source_package_path.to_owned(),
        registry_package_path: registry_package_path.display().to_string(),
        route_id: spec.map_or(0, |spec| spec.route_id),
        profile_id: spec.map_or(0, |spec| spec.profile_id),
        threshold_micro: spec.map_or(0, |spec| spec.threshold_micro),
        package_fingerprint64,
        package_bytes: package_bytes_len,
        package_records,
        registry_copy_exact,
        accepted_for_shadow_registry,
        blockers,
    })
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_u32(value: &Value, path: &[&str]) -> Option<u32> {
    json_u64(value, path).and_then(|number| u32::try_from(number).ok())
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    current.as_i64().or_else(|| {
        current
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
    })
}

fn forbidden_flags_all_bool_false(value: &Value) -> bool {
    let Some(flags) = value.as_object() else {
        return false;
    };
    !flags.is_empty() && flags.values().all(|value| value.as_bool() == Some(false))
}
