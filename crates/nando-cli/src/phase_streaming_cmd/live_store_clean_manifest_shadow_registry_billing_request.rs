use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use nando_core::{PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, phase_vector_from_atom_ids};
use serde::Deserialize;
use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-billing-request-v1.report.json";
const DEFAULT_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-billing-request-v1.jsonl";
const DEFAULT_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-replay-v1-current.report.json";
const DEFAULT_PREPARED_HOT_PACK: &str =
    "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-v1.json";

#[derive(Clone, Deserialize)]
struct PreparedHotPack {
    cells: usize,
    rows: Vec<PreparedHotPackRow>,
}

#[derive(Clone, Deserialize)]
struct PreparedHotPackRow {
    route_id: u32,
    atom_ids: Vec<u64>,
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    tokens: u64,
    cost_microusd: u64,
}

#[derive(Clone)]
struct ReplayPackageSpec {
    registry_package_path: String,
    route_id: u32,
    profile_id: u32,
    threshold_micro: i64,
    package_fingerprint64: u64,
}

#[derive(Clone, Deserialize)]
struct PreparedCorrelationSidecarRow {
    route_id: u32,
    route_row_ordinal: usize,
    request_fingerprint: Option<String>,
    exact_cache_key: Option<String>,
    trace_id: Option<String>,
    match_keys: Vec<String>,
    external_provider_correlation_keys: Vec<String>,
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REQUEST_JSONL));
    let replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPLAY_REPORT));
    let prepared_hot_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_HOT_PACK));
    let sidecar_jsonl_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let replay = read_json_value(&replay_report_path)?;
    let replay_allowed = json_bool(&replay, &["shadow_registry_replay_allowed"]).unwrap_or(false)
        && json_string(&replay, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_PASS")
        && json_bool(&replay, &["local_accept_enabled"]) == Some(false)
        && json_bool(&replay, &["market_money_claim_allowed"]) == Some(false)
        && json_bool(&replay, &["serving_registry_mutated"]) == Some(false)
        && replay
            .get("forbidden_flags")
            .is_some_and(forbidden_flags_all_bool_false);
    let replay_unique_accepts =
        json_usize(&replay, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let replay_tokens = json_u64(&replay, &["tokens_saved"]).unwrap_or(0);
    let replay_cost = json_u64(&replay, &["cost_saved_microusd"]).unwrap_or(0);
    let replay_false_accepts = json_usize(&replay, &["false_accepts"]).unwrap_or(usize::MAX);
    let replay_local_accept_events =
        json_usize(&replay, &["local_accept_events"]).unwrap_or(usize::MAX);
    let specs = replay_package_specs(&replay);

    let pack_text = std::fs::read_to_string(&prepared_hot_pack_path).map_err(|error| {
        format!(
            "failed to read prepared hot pack '{}': {error}",
            prepared_hot_pack_path.display()
        )
    })?;
    let pack = serde_json::from_str::<PreparedHotPack>(&pack_text).map_err(|error| {
        format!(
            "failed to parse prepared hot pack '{}': {error}",
            prepared_hot_pack_path.display()
        )
    })?;
    let sidecar_rows = if let Some(path) = &sidecar_jsonl_path {
        read_sidecar_rows(path)?
    } else {
        BTreeMap::new()
    };

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create clean registry billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create clean registry billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut score_events = 0usize;
    let mut prepared_row_ordinal = 0usize;
    let mut route_row_ordinals = BTreeMap::<u32, usize>::new();
    let mut request_rows = 0usize;
    let mut skipped_not_verified_safe = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_below_threshold = 0usize;
    let mut rows_with_sidecar = 0usize;
    let mut rows_with_request_fingerprint = 0usize;
    let mut rows_with_exact_cache_key = 0usize;
    let mut rows_with_trace_id = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut total_tokens_requiring_billing = 0u64;
    let mut current_known_cost_microusd = 0u64;

    for spec in &specs {
        let package_bytes = std::fs::read(&spec.registry_package_path).map_err(|error| {
            format!(
                "failed to read clean registry package '{}': {error}",
                spec.registry_package_path
            )
        })?;
        let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
            &package_bytes,
            PhaseCenterOffloadPolicy::new(spec.threshold_micro)
                .map_err(|error| format!("invalid clean registry threshold: {error:?}"))?,
        )
        .map_err(|error| {
            format!(
                "failed to load clean registry package '{}': {error:?}",
                spec.registry_package_path
            )
        })?;

        for row in pack.rows.iter().filter(|row| row.route_id == spec.route_id) {
            score_events += 1;
            prepared_row_ordinal += 1;
            let route_row_ordinal = route_row_ordinals
                .entry(row.route_id)
                .and_modify(|ordinal| *ordinal += 1)
                .or_insert(1);
            let phase_vector = phase_vector_from_atom_ids(row.atom_ids.iter().copied(), pack.cells);
            let margin_micro = runtime
                .runtime()
                .score_vector_margin_micro(0, &phase_vector)
                .map_err(|error| format!("clean registry billing score error: {error:?}"))?;
            if margin_micro < spec.threshold_micro {
                skipped_below_threshold += 1;
                continue;
            }
            if !row.verified_safe_accept {
                skipped_not_verified_safe += 1;
                continue;
            }
            if row.exact_cache_hit {
                skipped_exact_cache_hit += 1;
                continue;
            }

            let atom_fingerprint64 = atom_fingerprint64(&row.atom_ids);
            let sidecar = sidecar_rows.get(&(row.route_id, *route_row_ordinal));
            rows_with_sidecar += usize::from(sidecar.is_some());
            rows_with_request_fingerprint += usize::from(
                sidecar
                    .and_then(|row| row.request_fingerprint.as_deref())
                    .is_some_and(|value| !value.is_empty()),
            );
            rows_with_exact_cache_key += usize::from(
                sidecar
                    .and_then(|row| row.exact_cache_key.as_deref())
                    .is_some_and(|value| !value.is_empty()),
            );
            rows_with_trace_id += usize::from(
                sidecar
                    .and_then(|row| row.trace_id.as_deref())
                    .is_some_and(|value| !value.is_empty()),
            );
            let external_provider_correlation_keys = sidecar
                .map(|row| row.external_provider_correlation_keys.clone())
                .unwrap_or_default();
            let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
            provider_correlation_ready_rows += usize::from(provider_correlation_ready);
            request_rows += 1;
            total_tokens_requiring_billing =
                total_tokens_requiring_billing.saturating_add(row.tokens);
            current_known_cost_microusd =
                current_known_cost_microusd.saturating_add(row.cost_microusd);
            let mut match_keys = vec![
                format!("phase_atom_fingerprint64:{atom_fingerprint64}"),
                format!("route_id:{}", spec.route_id),
                format!("profile_id:{}", spec.profile_id),
                format!("package_fingerprint64:{}", spec.package_fingerprint64),
            ];
            if let Some(sidecar) = sidecar {
                match_keys.extend(sidecar.match_keys.iter().cloned());
            }
            match_keys.sort();
            match_keys.dedup();
            let request = serde_json::json!({
                "schema_version": "phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1",
                "billing_request_id": format!(
                    "clean-shadow-registry-nwpc-{}-{}-{}",
                    spec.route_id,
                    prepared_row_ordinal,
                    atom_fingerprint64
                ),
                "route_id": spec.route_id,
                "profile_id": spec.profile_id,
                "package_fingerprint64": spec.package_fingerprint64,
                "prepared_row_ordinal": prepared_row_ordinal,
                "route_row_ordinal": route_row_ordinal,
                "atom_fingerprint64": atom_fingerprint64,
                "request_fingerprint": sidecar.and_then(|row| row.request_fingerprint.clone()),
                "exact_cache_key": sidecar.and_then(|row| row.exact_cache_key.clone()),
                "trace_id": sidecar.and_then(|row| row.trace_id.clone()),
                "external_provider_correlation_keys": external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "provider_correlation_blocker": if provider_correlation_ready {
                    serde_json::Value::Null
                } else {
                    serde_json::json!("prepared numeric pack has no external provider request id/call id; attach provider-boundary correlation before money claim")
                },
                "match_keys": match_keys,
                "margin_micro": margin_micro,
                "threshold_micro": spec.threshold_micro,
                "estimated_total_tokens": row.tokens,
                "current_total_cost_microusd": row.cost_microusd,
                "token_cost_estimate_used": true,
                "provider_billing_evidence_present": false,
                "unique_cpu_accept_over_exact_cache": true,
                "verified_safe_accept": true,
                "false_accept": false,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "clean shadow registry billing request only: asks external provider billing evidence to attach real costs to verifier-bound .nwpc shadow accepts; does not estimate missing money, promote, serve, or enable local_accept"
            });
            serde_json::to_writer(&mut writer, &request).map_err(|error| {
                format!(
                    "failed to serialize clean registry billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write clean registry billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush clean registry billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let accept_parity = request_rows == replay_unique_accepts;
    let token_parity = total_tokens_requiring_billing == replay_tokens;
    let cost_parity = current_known_cost_microusd == replay_cost;
    let provider_correlation_ready =
        request_rows > 0 && provider_correlation_ready_rows == request_rows;
    let billing_request_ready = replay_allowed
        && replay_false_accepts == 0
        && replay_local_accept_events == 0
        && request_rows > 0
        && accept_parity
        && token_parity
        && cost_parity;
    let ready_for_external_provider_evidence = billing_request_ready && provider_correlation_ready;
    let verdict = if ready_for_external_provider_evidence {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_BILLING_REQUEST_V1_READY_FOR_EXTERNAL_EVIDENCE"
    } else if billing_request_ready {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_BILLING_REQUEST_V1_WATCH_PROVIDER_CORRELATION_MISSING"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_BILLING_REQUEST_V1_WATCH"
    };

    let blockers: Vec<&str> = if ready_for_external_provider_evidence {
        Vec::new()
    } else if billing_request_ready {
        vec!["provider_correlation_missing"]
    } else {
        vec!["billing_request_not_ready"]
    };
    let forbidden_flags = serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "manual_class_list_used": false,
        "manual_threshold_selection_used": false,
        "local_accept_without_verifier_used": false
    });
    let mut report_map = serde_json::Map::new();
    report_map.insert(
        "report_kind".to_string(),
        serde_json::json!(
            "phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1"
        ),
    );
    report_map.insert(
        "replay_report_path".to_string(),
        serde_json::json!(replay_report_path.display().to_string()),
    );
    report_map.insert(
        "prepared_hot_pack_path".to_string(),
        serde_json::json!(prepared_hot_pack_path.display().to_string()),
    );
    report_map.insert(
        "correlation_sidecar_jsonl_path".to_string(),
        serde_json::json!(
            sidecar_jsonl_path
                .as_ref()
                .map(|path| path.display().to_string())
        ),
    );
    report_map.insert(
        "billing_request_jsonl_path".to_string(),
        serde_json::json!(request_jsonl_path.display().to_string()),
    );
    report_map.insert(
        "replay_allowed".to_string(),
        serde_json::json!(replay_allowed),
    );
    report_map.insert("score_events".to_string(), serde_json::json!(score_events));
    report_map.insert(
        "billing_request_rows".to_string(),
        serde_json::json!(request_rows),
    );
    report_map.insert(
        "skipped_not_verified_safe".to_string(),
        serde_json::json!(skipped_not_verified_safe),
    );
    report_map.insert(
        "skipped_exact_cache_hit".to_string(),
        serde_json::json!(skipped_exact_cache_hit),
    );
    report_map.insert(
        "skipped_below_threshold".to_string(),
        serde_json::json!(skipped_below_threshold),
    );
    report_map.insert(
        "deduplicated_by_atom_fingerprint".to_string(),
        serde_json::json!(false),
    );
    report_map.insert(
        "sidecar_rows_available".to_string(),
        serde_json::json!(sidecar_rows.len()),
    );
    report_map.insert(
        "billing_rows_with_sidecar".to_string(),
        serde_json::json!(rows_with_sidecar),
    );
    report_map.insert(
        "rows_with_request_fingerprint".to_string(),
        serde_json::json!(rows_with_request_fingerprint),
    );
    report_map.insert(
        "rows_with_exact_cache_key".to_string(),
        serde_json::json!(rows_with_exact_cache_key),
    );
    report_map.insert(
        "rows_with_trace_id".to_string(),
        serde_json::json!(rows_with_trace_id),
    );
    report_map.insert(
        "provider_correlation_ready_rows".to_string(),
        serde_json::json!(provider_correlation_ready_rows),
    );
    report_map.insert(
        "provider_correlation_missing_rows".to_string(),
        serde_json::json!(request_rows.saturating_sub(provider_correlation_ready_rows)),
    );
    report_map.insert(
        "total_tokens_requiring_billing".to_string(),
        serde_json::json!(total_tokens_requiring_billing),
    );
    report_map.insert(
        "current_known_cost_microusd".to_string(),
        serde_json::json!(current_known_cost_microusd),
    );
    report_map.insert(
        "shadow_replay_unique_accepts_over_exact_cache".to_string(),
        serde_json::json!(replay_unique_accepts),
    );
    report_map.insert(
        "shadow_replay_tokens_saved".to_string(),
        serde_json::json!(replay_tokens),
    );
    report_map.insert(
        "shadow_replay_cost_saved_microusd".to_string(),
        serde_json::json!(replay_cost),
    );
    report_map.insert(
        "accept_parity".to_string(),
        serde_json::json!(accept_parity),
    );
    report_map.insert("token_parity".to_string(), serde_json::json!(token_parity));
    report_map.insert("cost_parity".to_string(), serde_json::json!(cost_parity));
    report_map.insert(
        "billing_request_ready".to_string(),
        serde_json::json!(billing_request_ready),
    );
    report_map.insert(
        "ready_for_external_provider_evidence".to_string(),
        serde_json::json!(ready_for_external_provider_evidence),
    );
    report_map.insert(
        "provider_billing_evidence_present".to_string(),
        serde_json::json!(false),
    );
    report_map.insert("billing_request_only".to_string(), serde_json::json!(true));
    report_map.insert("local_accept_enabled".to_string(), serde_json::json!(false));
    report_map.insert("auto_promote_enabled".to_string(), serde_json::json!(false));
    report_map.insert(
        "serving_registry_mutated".to_string(),
        serde_json::json!(false),
    );
    report_map.insert(
        "market_money_claim_allowed".to_string(),
        serde_json::json!(false),
    );
    report_map.insert("forbidden_flags".to_string(), forbidden_flags);
    report_map.insert("blockers".to_string(), serde_json::json!(blockers));
    report_map.insert("verdict".to_string(), serde_json::json!(verdict));
    report_map.insert(
        "boundary".to_string(),
        serde_json::json!("billing request export only: emits verifier-bound .nwpc shadow accepts that need external provider billing evidence; it does not create evidence, estimate missing money, promote, serve, enable local_accept, or revive legacy nwrb"),
    );
    let report = Value::Object(report_map);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  cost_parity: {cost_parity}");
    println!("  provider_correlation_ready_rows: {provider_correlation_ready_rows}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn replay_package_specs(replay: &Value) -> Vec<ReplayPackageSpec> {
    replay
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|package| {
            package
                .get("blockers")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        })
        .filter_map(|package| {
            Some(ReplayPackageSpec {
                registry_package_path: json_string(package, &["registry_package_path"])?,
                route_id: json_u32(package, &["route_id"])?,
                profile_id: json_u32(package, &["profile_id"])?,
                threshold_micro: json_i64(package, &["threshold_micro"])?,
                package_fingerprint64: json_u64(package, &["package_fingerprint64"])?,
            })
        })
        .collect()
}

fn read_sidecar_rows(
    path: &PathBuf,
) -> Result<BTreeMap<(u32, usize), PreparedCorrelationSidecarRow>, String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read correlation sidecar '{}': {error}",
            path.display()
        )
    })?;
    let mut rows = BTreeMap::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row =
            serde_json::from_str::<PreparedCorrelationSidecarRow>(trimmed).map_err(|error| {
                format!(
                    "failed to parse correlation sidecar '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
        rows.insert((row.route_id, row.route_row_ordinal), row);
    }
    Ok(rows)
}

fn atom_fingerprint64(atom_ids: &[u64]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for atom_id in atom_ids {
        for byte in atom_id.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
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
