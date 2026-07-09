use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use super::{json_string, write_json_file};

const DEFAULT_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-correlation-sidecar-v1.report.json";
const DEFAULT_SIDECAR_JSONL: &str = "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-correlation-sidecar-v1.jsonl";
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

struct TraceCandidate {
    route_id: u32,
    atom_ids: Vec<u64>,
    request_fingerprint: Option<String>,
    exact_cache_key: Option<String>,
    trace_id: Option<String>,
    external_provider_correlation_keys: Vec<String>,
    match_keys: Vec<String>,
}

pub(crate) fn run_phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let sidecar_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SIDECAR_JSONL));
    let prepared_hot_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_HOT_PACK));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

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

    if let Some(parent) = sidecar_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create prepared correlation sidecar dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&sidecar_jsonl_path).map_err(|error| {
        format!(
            "failed to create prepared correlation sidecar '{}': {error}",
            sidecar_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut pack_index = 0usize;
    let mut route_ordinals = BTreeMap::<u32, usize>::new();
    let mut total_trace_rows = 0usize;
    let mut parsed_trace_candidates = 0usize;
    let mut sidecar_rows = 0usize;
    let mut rows_with_request_fingerprint = 0usize;
    let mut rows_with_exact_cache_key = 0usize;
    let mut rows_with_trace_id = 0usize;
    let mut rows_with_provider_correlation_keys = 0usize;

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_trace_rows += 1;
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if row
                .get("verified_safe_accept")
                .and_then(Value::as_bool)
                .is_none()
            {
                continue;
            }
            let Some(candidate) = trace_candidate(&row) else {
                continue;
            };
            parsed_trace_candidates += 1;
            let Some(pack_row) = pack.rows.get(pack_index) else {
                continue;
            };
            if !same_prepared_row(pack_row, &candidate) {
                continue;
            }
            pack_index += 1;
            let route_row_ordinal = route_ordinals
                .entry(pack_row.route_id)
                .and_modify(|ordinal| *ordinal += 1)
                .or_insert(1);
            let provider_correlation_ready =
                !candidate.external_provider_correlation_keys.is_empty();
            rows_with_request_fingerprint += usize::from(candidate.request_fingerprint.is_some());
            rows_with_exact_cache_key += usize::from(candidate.exact_cache_key.is_some());
            rows_with_trace_id += usize::from(candidate.trace_id.is_some());
            rows_with_provider_correlation_keys += usize::from(provider_correlation_ready);
            sidecar_rows += 1;
            let sidecar = serde_json::json!({
                "schema_version": "phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1",
                "prepared_pack_path": prepared_hot_pack_path,
                "source_trace_path": trace_path,
                "pack_row_ordinal": pack_index,
                "route_row_ordinal": route_row_ordinal,
                "route_id": pack_row.route_id,
                "atom_fingerprint64": atom_fingerprint64(&pack_row.atom_ids),
                "request_fingerprint": candidate.request_fingerprint,
                "exact_cache_key": candidate.exact_cache_key,
                "trace_id": candidate.trace_id,
                "match_keys": candidate.match_keys,
                "external_provider_correlation_keys": candidate.external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "verified_safe_accept": pack_row.verified_safe_accept,
                "exact_cache_hit": pack_row.exact_cache_hit,
                "tokens": pack_row.tokens,
                "cost_microusd": pack_row.cost_microusd,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "cold correlation sidecar only: preserves request/trace/provider metadata outside the numeric hot pack; does not alter atoms, score, compile, serve, local-accept, or claim money"
            });
            serde_json::to_writer(&mut writer, &sidecar).map_err(|error| {
                format!(
                    "failed to serialize prepared correlation sidecar '{}': {error}",
                    sidecar_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write prepared correlation sidecar '{}': {error}",
                    sidecar_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush prepared correlation sidecar '{}': {error}",
            sidecar_jsonl_path.display()
        )
    })?;

    let pack_sidecar_parity = sidecar_rows == pack.rows.len();
    let verdict = if pack_sidecar_parity && rows_with_request_fingerprint == sidecar_rows {
        "PHASE_STREAM_LIVE_STORE_PREPARED_HOT_PACK_CORRELATION_SIDECAR_V1_PASS"
    } else if sidecar_rows > 0 {
        "PHASE_STREAM_LIVE_STORE_PREPARED_HOT_PACK_CORRELATION_SIDECAR_V1_WATCH_PARTIAL"
    } else {
        "PHASE_STREAM_LIVE_STORE_PREPARED_HOT_PACK_CORRELATION_SIDECAR_V1_WATCH_EMPTY"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1",
        "prepared_hot_pack_path": prepared_hot_pack_path,
        "sidecar_jsonl_path": sidecar_jsonl_path,
        "input_trace_paths": trace_paths,
        "cells": pack.cells,
        "pack_rows": pack.rows.len(),
        "total_trace_rows": total_trace_rows,
        "parsed_trace_candidates": parsed_trace_candidates,
        "sidecar_rows": sidecar_rows,
        "rows_with_request_fingerprint": rows_with_request_fingerprint,
        "rows_with_exact_cache_key": rows_with_exact_cache_key,
        "rows_with_trace_id": rows_with_trace_id,
        "rows_with_provider_correlation_keys": rows_with_provider_correlation_keys,
        "pack_sidecar_parity": pack_sidecar_parity,
        "pack_remains_numeric_only": true,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": verdict,
        "boundary": "prepared hot-pack correlation sidecar only: request/trace/provider metadata stays outside the numeric hot pack; no scoring, mining, package compile, serving mutation, local_accept, or market money claim"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  sidecar_jsonl_path: {}", sidecar_jsonl_path.display());
    println!("  sidecar_rows: {sidecar_rows}");
    println!("  pack_sidecar_parity: {pack_sidecar_parity}");
    println!("  rows_with_request_fingerprint: {rows_with_request_fingerprint}");
    println!("  rows_with_provider_correlation_keys: {rows_with_provider_correlation_keys}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn trace_candidate(row: &Value) -> Option<TraceCandidate> {
    let safe_atoms = live_store_safe_atoms(row);
    if safe_atoms.is_empty() {
        return None;
    }
    let route_key = live_store_route_key(row);
    let atom_ids = safe_atoms
        .iter()
        .map(|atom| super::stable_fingerprint(["live_store_atom", atom.as_str()]))
        .collect::<Vec<_>>();
    let route_id = live_store_hash_id(["live_store_route", route_key.as_str()]);
    let request_fingerprint = json_string(row, &["request_fingerprint"]);
    let exact_cache_key = json_string(row, &["exact_cache_key"]);
    let trace_id = json_string(row, &["trace_id"]);
    let external_provider_correlation_keys =
        super::phase_atom_external_provider_correlation_keys(row);
    let mut match_keys = Vec::new();
    push_match_key(
        &mut match_keys,
        "request_fingerprint",
        request_fingerprint.as_deref(),
    );
    push_match_key(
        &mut match_keys,
        "exact_cache_key",
        exact_cache_key.as_deref(),
    );
    push_match_key(&mut match_keys, "trace_id", trace_id.as_deref());
    match_keys.extend(external_provider_correlation_keys.iter().cloned());
    match_keys.sort();
    match_keys.dedup();
    Some(TraceCandidate {
        route_id,
        atom_ids,
        request_fingerprint,
        exact_cache_key,
        trace_id,
        external_provider_correlation_keys,
        match_keys,
    })
}

fn same_prepared_row(pack_row: &PreparedHotPackRow, candidate: &TraceCandidate) -> bool {
    pack_row.route_id == candidate.route_id && pack_row.atom_ids == candidate.atom_ids
}

fn live_store_safe_atoms(row: &Value) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for atom in super::phase_atom_string_vec(row, "request_atoms")
        .into_iter()
        .chain(super::phase_atom_string_vec(row, "state_atoms"))
        .chain(super::phase_atom_string_vec(row, "action_atoms"))
        .chain(super::phase_atom_string_vec(row, "tool_atoms"))
        .chain(super::phase_atom_string_vec(row, "route_hint_atoms"))
        .chain(super::phase_atom_string_vec(row, "shadow_payload_atoms"))
    {
        if !live_store_forbidden_atom(&atom) {
            atoms.insert(atom);
        }
    }
    atoms.into_iter().collect()
}

fn live_store_route_key(row: &Value) -> String {
    super::phase_atom_string_vec(row, "route_hint_atoms")
        .into_iter()
        .next()
        .or_else(|| {
            super::phase_atom_action_families(&super::phase_atom_string_vec(row, "action_atoms"))
                .into_iter()
                .next()
        })
        .or_else(|| json_string(row, &["traffic_source"]))
        .unwrap_or_else(|| "unknown_route".to_owned())
}

fn live_store_forbidden_atom(atom: &str) -> bool {
    [
        "output_hash64:",
        "verifier_label:",
        "verified_safe_accept:",
        "candidate_verified_safe_accept:",
        "candidate_result_label:",
        "exact_cache_key:",
        "request_fingerprint:",
        "trace_id:",
        "source_trace_id:",
        "target_id:",
        "proof_rule_id:",
        "concrete_x_lookup:",
        "manual_local_out_t:",
    ]
    .iter()
    .any(|prefix| atom.starts_with(prefix))
}

fn live_store_hash_id<'a, I>(parts: I) -> u32
where
    I: IntoIterator<Item = &'a str>,
{
    (super::stable_fingerprint(parts) & 0xffff_ffff) as u32
}

fn push_match_key(keys: &mut Vec<String>, prefix: &str, value: Option<&str>) {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return;
    };
    if value.starts_with(prefix) && value.contains(':') {
        keys.push(value.to_owned());
    } else {
        keys.push(format!("{prefix}:{value}"));
    }
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
