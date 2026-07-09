use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nando_core::{
    PhaseCenterFlatRecord, PhaseCenterFlatRuntime, PhaseCenterHotRouteTable, PhaseCenterHotRuntime,
};

use super::paths::{
    live_store_resolve_registry_relative_path, live_store_route_key_from_bucket_key,
};
use super::reports::PhaseStreamLiveStoreCleanPromotionManifestInput;
use super::source_events::{
    LiveStoreParsedAtomEvent, live_store_action_family_route_id_from_row, live_store_hash_id,
};
use super::state::{LiveStoreCleanManifestRuntimeBundle, LiveStoreProductHotRegistryRuntimeBundle};

pub(super) fn load_live_store_clean_manifest_runtime(
    manifest_path: &Path,
) -> Result<LiveStoreCleanManifestRuntimeBundle, String> {
    let manifest_text = std::fs::read_to_string(manifest_path).map_err(|error| {
        format!(
            "failed to read clean promotion manifest '{}': {error}",
            manifest_path.display()
        )
    })?;
    let manifest =
        serde_json::from_str::<PhaseStreamLiveStoreCleanPromotionManifestInput>(&manifest_text)
            .map_err(|error| {
                format!(
                    "failed to parse clean promotion manifest '{}': {error}",
                    manifest_path.display()
                )
            })?;
    if manifest.promoted_packages.is_empty() {
        return Err("clean promotion manifest has no promoted packages".to_owned());
    }
    if !manifest.allowed {
        return Err(format!(
            "clean promotion manifest is not allowed: {}",
            manifest.blocker
        ));
    }
    if manifest.false_accepts != 0 {
        return Err("clean promotion manifest has false_accepts".to_owned());
    }
    if manifest.runtime_parity_mismatches != 0 {
        return Err("clean promotion manifest has runtime parity mismatches".to_owned());
    }
    if !manifest.exact_cache_overlap_excluded {
        return Err("clean promotion manifest did not exclude exact-cache overlap".to_owned());
    }
    if manifest.routes.is_empty() {
        return Err("clean promotion manifest has no routes".to_owned());
    }
    if manifest.local_accept_enabled {
        return Err("clean promotion manifest unexpectedly enables local_accept".to_owned());
    }
    if manifest.market_money_claim_allowed {
        return Err("clean promotion manifest unexpectedly allows market money claim".to_owned());
    }

    let mut records = Vec::<PhaseCenterFlatRecord>::new();
    let mut profile_ids = Vec::<u32>::new();
    let mut thresholds = Vec::<i64>::new();
    let mut cells = None::<usize>;
    let mut loaded_record_count = 0usize;
    for package in &manifest.promoted_packages {
        let package_path = PathBuf::from(&package.package_path);
        let package_bytes = std::fs::read(&package_path).map_err(|error| {
            format!(
                "failed to read clean promotion package '{}': {error}",
                package_path.display()
            )
        })?;
        let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)
            .map_err(|error| format!("failed to inspect .nwpc package: {error:?}"))?;
        if package_info.fingerprint64 != package.package_fingerprint64 {
            return Err(format!(
                "clean promotion package fingerprint mismatch for '{}': manifest={} actual={}",
                package_path.display(),
                package.package_fingerprint64,
                package_info.fingerprint64
            ));
        }
        let flat = PhaseCenterFlatRuntime::from_bytes(&package_bytes)
            .map_err(|error| format!("failed to load .nwpc package: {error:?}"))?;
        if flat.record_count() != 1 {
            return Err(format!(
                "clean promotion package '{}' must contain one record, found {}",
                package_path.display(),
                flat.record_count()
            ));
        }
        if let Some(expected_cells) = cells {
            if expected_cells != flat.cells() {
                return Err("clean promotion packages have mixed cell widths".to_owned());
            }
        } else {
            cells = Some(flat.cells());
        }
        records.push(
            flat.record(0)
                .map_err(|error| format!("failed to read .nwpc record: {error:?}"))?
                .clone(),
        );
        profile_ids.push(package.profile_id);
        thresholds.push(package.threshold_micro);
        loaded_record_count += flat.record_count();
    }

    let cells = cells.ok_or_else(|| "clean promotion cells missing".to_owned())?;
    let flat_runtime = PhaseCenterFlatRuntime::new(cells, records)
        .map_err(|error| format!("failed to build clean manifest flat runtime: {error:?}"))?;
    let hot_runtime =
        PhaseCenterHotRuntime::from_flat_runtime(&flat_runtime, &profile_ids, &thresholds)
            .map_err(|error| format!("failed to build clean manifest hot runtime: {error:?}"))?;
    let mut route_profiles = BTreeMap::<u32, Vec<u32>>::new();
    for route in &manifest.routes {
        route_profiles
            .entry(route.route_id)
            .or_default()
            .push(route.profile_id);
    }

    let mut plans = Vec::with_capacity(route_profiles.len());
    for (route_id, profile_ids_for_route) in route_profiles {
        let plan = hot_runtime
            .route_plan_from_profile_ids(route_id, profile_ids_for_route)
            .map_err(|error| format!("failed to build clean manifest route plan: {error:?}"))?
            .ok_or_else(|| "clean manifest route plan unexpectedly empty".to_owned())?;
        plans.push(plan);
    }
    let route_table = PhaseCenterHotRouteTable::from_plans(plans)
        .map_err(|error| format!("failed to build clean manifest route table: {error:?}"))?;
    let mut route_manifest_index_mismatches = 0usize;
    for route in &manifest.routes {
        let resolved = route_table.resolve_route_index(route.route_id);
        if resolved != Some(route.route_index) {
            route_manifest_index_mismatches += 1;
        }
    }

    Ok(LiveStoreCleanManifestRuntimeBundle {
        manifest,
        flat_runtime,
        hot_runtime,
        route_table,
        profile_ids,
        thresholds,
        cells,
        loaded_record_count,
        route_manifest_index_mismatches,
    })
}

pub(super) fn load_live_store_product_hot_registry_runtime(
    registry_path: &Path,
    expected_cells: usize,
) -> Result<LiveStoreProductHotRegistryRuntimeBundle, String> {
    let registry = super::super::read_json_value(registry_path)?;
    if super::super::json_bool(&registry, &["local_accept_enabled"]).unwrap_or(true) {
        return Err("product-hot registry unexpectedly enables local_accept".to_owned());
    }
    if super::super::json_bool(&registry, &["market_money_claim_allowed"]).unwrap_or(true) {
        return Err("product-hot registry unexpectedly allows market money claim".to_owned());
    }
    if super::super::json_u64(&registry, &["false_accepts"]).unwrap_or(1) != 0 {
        return Err("product-hot registry has false_accepts".to_owned());
    }
    if !super::super::json_bool(&registry, &["shadow_registry_budget_passed"]).unwrap_or(false) {
        return Err("product-hot registry budget gate is not passed".to_owned());
    }
    let candidates = super::super::json_at(&registry, &["candidates"])
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "product-hot registry missing candidates".to_owned())?;
    if candidates.is_empty() {
        return Err("product-hot registry has no candidates".to_owned());
    }

    let mut records = Vec::<PhaseCenterFlatRecord>::new();
    let mut profile_ids = Vec::<u32>::new();
    let mut thresholds = Vec::<i64>::new();
    let mut route_profiles = BTreeMap::<u32, Vec<u32>>::new();
    let mut cells = None::<usize>;
    let mut package_bytes_total = 0usize;

    for candidate in candidates {
        if !super::super::json_bool(candidate, &["promotion_gate_passed"]).unwrap_or(false) {
            continue;
        }
        if !super::super::json_bool(candidate, &["verifier_bound"]).unwrap_or(false) {
            continue;
        }
        if super::super::json_bool(candidate, &["local_accept_enabled"]).unwrap_or(true) {
            return Err("product-hot candidate unexpectedly enables local_accept".to_owned());
        }
        if super::super::json_u64(candidate, &["false_accepts"]).unwrap_or(1) != 0 {
            return Err("product-hot candidate has false_accepts".to_owned());
        }
        let bucket_key = super::super::json_string(candidate, &["bucket_key"])
            .ok_or_else(|| "product-hot candidate missing bucket_key".to_owned())?;
        let route_key = live_store_route_key_from_bucket_key(&bucket_key);
        let route_id = live_store_hash_id(["live_store_route", route_key]);
        let profile_id = live_store_hash_id(["live_store_bucket", bucket_key.as_str()]);
        let threshold_micro =
            super::super::json_i64(candidate, &["safe_accept_margin_threshold_micro"])
                .or_else(|| {
                    super::super::json_i64(candidate, &["auto_calibrated_margin_threshold_micro"])
                })
                .ok_or_else(|| "product-hot candidate missing threshold".to_owned())?;
        if threshold_micro <= 0 {
            return Err("product-hot candidate threshold must be positive".to_owned());
        }
        let package_path = super::super::json_string(candidate, &["package_path"])
            .map(PathBuf::from)
            .ok_or_else(|| "product-hot candidate missing package_path".to_owned())?;
        let package_path = live_store_resolve_registry_relative_path(registry_path, &package_path);
        let package_fingerprint64 =
            super::super::json_u64(candidate, &["package_fingerprint64"]).unwrap_or_default();
        let package_bytes = std::fs::read(&package_path).map_err(|error| {
            format!(
                "failed to read product-hot .nwpc package '{}': {error}",
                package_path.display()
            )
        })?;
        let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)
            .map_err(|error| format!("failed to inspect product-hot .nwpc package: {error:?}"))?;
        if package_info.fingerprint64 != package_fingerprint64 {
            return Err(format!(
                "product-hot package fingerprint mismatch for '{}': registry={} actual={}",
                package_path.display(),
                package_fingerprint64,
                package_info.fingerprint64
            ));
        }
        if package_info.cells != expected_cells {
            return Err(format!(
                "product-hot package cell mismatch for '{}': expected={} actual={}",
                package_path.display(),
                expected_cells,
                package_info.cells
            ));
        }
        let flat = PhaseCenterFlatRuntime::from_bytes(&package_bytes)
            .map_err(|error| format!("failed to load product-hot .nwpc package: {error:?}"))?;
        if flat.record_count() != 1 {
            return Err(format!(
                "product-hot package '{}' must contain one record, found {}",
                package_path.display(),
                flat.record_count()
            ));
        }
        if let Some(expected) = cells {
            if expected != flat.cells() {
                return Err("product-hot packages have mixed cell widths".to_owned());
            }
        } else {
            cells = Some(flat.cells());
        }
        if profile_ids.contains(&profile_id) {
            return Err(format!(
                "product-hot profile id collision for bucket_key '{}'",
                bucket_key
            ));
        }
        records.push(
            flat.record(0)
                .map_err(|error| format!("failed to read product-hot .nwpc record: {error:?}"))?
                .clone(),
        );
        profile_ids.push(profile_id);
        thresholds.push(threshold_micro);
        route_profiles.entry(route_id).or_default().push(profile_id);
        package_bytes_total = package_bytes_total.saturating_add(package_bytes.len());
    }

    if records.is_empty() {
        return Err("product-hot registry has no loadable promoted candidates".to_owned());
    }
    let cells = cells.ok_or_else(|| "product-hot cells missing".to_owned())?;
    let flat_runtime = PhaseCenterFlatRuntime::new(cells, records)
        .map_err(|error| format!("failed to build product-hot flat runtime: {error:?}"))?;
    let hot_runtime =
        PhaseCenterHotRuntime::from_flat_runtime(&flat_runtime, &profile_ids, &thresholds)
            .map_err(|error| format!("failed to build product-hot hot runtime: {error:?}"))?;
    let mut plans = Vec::with_capacity(route_profiles.len());
    for (route_id, route_profile_ids) in route_profiles {
        let plan = hot_runtime
            .route_plan_from_profile_ids(route_id, route_profile_ids)
            .map_err(|error| format!("failed to build product-hot route plan: {error:?}"))?
            .ok_or_else(|| "product-hot route plan unexpectedly empty".to_owned())?;
        plans.push(plan);
    }
    let route_table = PhaseCenterHotRouteTable::from_plans(plans)
        .map_err(|error| format!("failed to build product-hot route table: {error:?}"))?;
    Ok(LiveStoreProductHotRegistryRuntimeBundle {
        registry_path: registry_path.to_path_buf(),
        hot_runtime,
        route_table,
        cells,
        package_bytes: package_bytes_total,
    })
}

pub(super) fn load_live_store_product_hot_runtime_from_clean_manifest(
    manifest_path: &Path,
    expected_cells: usize,
) -> Result<LiveStoreProductHotRegistryRuntimeBundle, String> {
    let bundle = load_live_store_clean_manifest_runtime(manifest_path)?;
    if bundle.cells != expected_cells {
        return Err(format!(
            "clean manifest cells mismatch: expected={} actual={}",
            expected_cells, bundle.cells
        ));
    }
    if bundle.route_manifest_index_mismatches != 0 {
        return Err("clean manifest route index mismatch".to_owned());
    }
    let package_bytes =
        bundle
            .manifest
            .promoted_packages
            .iter()
            .try_fold(0usize, |sum, package| {
                let metadata = std::fs::metadata(&package.package_path).map_err(|error| {
                    format!(
                        "failed to inspect clean manifest package '{}': {error}",
                        package.package_path
                    )
                })?;
                Ok::<_, String>(sum.saturating_add(metadata.len() as usize))
            })?;
    Ok(LiveStoreProductHotRegistryRuntimeBundle {
        registry_path: manifest_path.to_path_buf(),
        hot_runtime: bundle.hot_runtime,
        route_table: bundle.route_table,
        cells: bundle.cells,
        package_bytes,
    })
}

pub(super) fn try_load_live_store_allowed_call_token_runtime(
    manifest_path: &Path,
    expected_cells: usize,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> Result<Option<LiveStoreProductHotRegistryRuntimeBundle>, String> {
    if !manifest_path.exists() {
        return Ok(None);
    }
    let manifest = super::super::read_json_value(manifest_path)?;
    if !super::super::json_bool(&manifest, &["allowed"]).unwrap_or(false) {
        return Ok(None);
    }
    if live_store_call_token_manifest_promotes_quarantined_profile(
        &manifest,
        quarantined_profile_ids,
    ) {
        return Ok(None);
    }
    match load_live_store_product_hot_runtime_from_clean_manifest(manifest_path, expected_cells) {
        Ok(runtime) => Ok(Some(runtime)),
        Err(error) => {
            eprintln!(
                "phase-stream live-tail: ignored invalid call/token active manifest '{}': {error}",
                manifest_path.display()
            );
            Ok(None)
        }
    }
}

pub(super) fn live_store_allowed_call_token_manifest_profile_ids(
    manifest_path: &Path,
) -> Result<Vec<u32>, String> {
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }
    let manifest = super::super::read_json_value(manifest_path)?;
    if !super::super::json_bool(&manifest, &["allowed"]).unwrap_or(false)
        || super::super::json_u64(&manifest, &["false_accepts"]).unwrap_or(u64::MAX) != 0
        || super::super::json_u64(&manifest, &["runtime_parity_mismatches"]).unwrap_or(u64::MAX)
            != 0
    {
        return Ok(Vec::new());
    }
    let Some(packages) = manifest
        .get("promoted_packages")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut profile_ids = packages
        .iter()
        .filter_map(|package| {
            package
                .get("profile_id")
                .or_else(|| package.get("bucket_id"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
        })
        .collect::<Vec<_>>();
    profile_ids.sort_unstable();
    profile_ids.dedup();
    Ok(profile_ids)
}

pub(super) fn live_store_trusted_clean_report_profile_ids(
    report_path: &Path,
) -> Result<Vec<u32>, String> {
    if !report_path.exists() {
        return Ok(Vec::new());
    }
    let report = super::super::read_json_value(report_path)?;
    let Some(candidates) = report
        .get("clean_candidate_reports")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut profile_ids = candidates
        .iter()
        .filter(|candidate| {
            super::super::json_bool(candidate, &["candidate"]).unwrap_or(false)
                && super::super::json_bool(candidate, &["shadow_ready"]).unwrap_or(false)
                && !super::super::json_bool(candidate, &["rejected"]).unwrap_or(false)
                && super::super::json_u64(candidate, &["false_accepts"]).unwrap_or(u64::MAX) == 0
                && super::super::json_u64(candidate, &["trust_false_risk_micro"])
                    .unwrap_or(u64::MAX)
                    == 0
                && super::super::json_u64(candidate, &["tokens_saved"]).unwrap_or_default() > 0
                && super::super::json_u64(candidate, &["unique_cpu_accepts_over_exact_cache"])
                    .unwrap_or_default()
                    > 0
        })
        .filter_map(|candidate| {
            super::super::json_u64(candidate, &["profile_id"]).and_then(|id| u32::try_from(id).ok())
        })
        .collect::<Vec<_>>();
    profile_ids.sort_unstable();
    profile_ids.dedup();
    Ok(profile_ids)
}

pub(super) fn live_store_call_token_manifest_promotes_quarantined_profile(
    manifest: &serde_json::Value,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> bool {
    if quarantined_profile_ids.is_empty() {
        return false;
    }
    let Some(packages) = manifest
        .get("promoted_packages")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    packages.iter().any(|package| {
        ["profile_id", "bucket_id"].iter().any(|key| {
            package
                .get(*key)
                .and_then(serde_json::Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .is_some_and(|id| quarantined_profile_ids.contains(&id))
        })
    })
}

pub(super) fn live_store_product_hot_route_index(
    bundle: &LiveStoreProductHotRegistryRuntimeBundle,
    event: &LiveStoreParsedAtomEvent,
    row: &serde_json::Value,
) -> Option<usize> {
    bundle
        .route_table
        .resolve_route_index(event.route_id)
        .or_else(|| {
            live_store_action_family_route_id_from_row(row)
                .and_then(|route_id| bundle.route_table.resolve_route_index(route_id))
        })
}

pub(super) fn disable_live_store_call_token_active_manifest(
    manifest_path: &Path,
    reason: &str,
    false_accepts: usize,
    trace_id: &str,
    route_key: &str,
    bucket_key: &str,
    profile_ids: &BTreeSet<u32>,
) -> Result<(), String> {
    let mut manifest = if manifest_path.exists() {
        super::super::read_json_value(manifest_path)?
    } else {
        serde_json::json!({
            "report_kind": "phase_stream_live_store_call_token_promotion_manifest_v1",
            "manifest_kind": "disabled_missing_active_manifest_v1"
        })
    };
    let object = manifest
        .as_object_mut()
        .ok_or_else(|| "active call-token manifest is not a JSON object".to_owned())?;
    object.insert("allowed".to_owned(), serde_json::Value::Bool(false));
    object.insert(
        "blocker".to_owned(),
        serde_json::Value::String(reason.to_owned()),
    );
    object.insert(
        "local_accept_enabled".to_owned(),
        serde_json::Value::Bool(false),
    );
    object.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::Value::Bool(false),
    );
    object.insert(
        "live_score_only_disabled".to_owned(),
        serde_json::Value::Bool(true),
    );
    object.insert(
        "live_score_only_disable_reason".to_owned(),
        serde_json::Value::String(reason.to_owned()),
    );
    object.insert(
        "live_score_only_disable_false_accepts".to_owned(),
        serde_json::json!(false_accepts),
    );
    object.insert(
        "live_score_only_disable_trace_id".to_owned(),
        serde_json::Value::String(trace_id.to_owned()),
    );
    object.insert(
        "live_score_only_disable_route_key".to_owned(),
        serde_json::Value::String(route_key.to_owned()),
    );
    object.insert(
        "live_score_only_disable_bucket_key".to_owned(),
        serde_json::Value::String(bucket_key.to_owned()),
    );
    object.insert(
        "live_score_only_disable_profile_ids".to_owned(),
        serde_json::json!(profile_ids.iter().copied().collect::<Vec<_>>()),
    );
    object.insert(
        "boundary".to_owned(),
        serde_json::Value::String(
            "disabled by live score-only shadow false_accept; may not be reloaded until a fresh false_accept=0 manifest replaces it"
                .to_owned(),
        ),
    );
    super::super::write_json_file(manifest_path, &manifest)
}
