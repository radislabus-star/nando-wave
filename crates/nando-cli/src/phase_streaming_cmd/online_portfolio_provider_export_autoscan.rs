use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};

use super::online_portfolio_provider_export_admission::run_phase_stream_online_miner_portfolio_provider_export_admission_v1;
use super::{
    json_bool, json_string, json_u64, read_json_value, sanitize_file_stem, write_json_file,
};

const DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-autoscan-v1.report.json";
const DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_SCAN_DIR: &str = "target/nando-wave/streaming";
const DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_WORK_DIR: &str =
    "target/nando-wave/streaming/online-miner-portfolio-provider-export-autoscan-v1";
const DEFAULT_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-run-check-parse-detail-multisplit-v5.jsonl";
const DEFAULT_BILLING_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_MAX_EVALUATED_CANDIDATES: usize = 8;
const MAX_AUTOSCAN_CANDIDATE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone)]
struct ProviderExportCandidate {
    path: PathBuf,
    file_name: String,
    key_hit_count: usize,
    line_count: usize,
}

#[derive(Serialize)]
struct EvaluatedProviderExportCandidate {
    provider_export_jsonl_path: String,
    work_dir: String,
    report_path: String,
    key_hit_count: usize,
    line_count: usize,
    verdict: String,
    provider_billing_evidence_present: bool,
    provider_export_attestation_present: bool,
    provider_export_attestation_valid: bool,
    provider_export_attestation_blockers: Vec<String>,
    product_ready: bool,
    promotion_ready: bool,
    product_promotion_allowed: bool,
    market_money_claim_allowed: bool,
    request_rows: usize,
    rows_enriched_provider_cost: usize,
    rows_enriched_provider_tokens: usize,
    missing_billing_request_rows: usize,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_REPORT));
    let scan_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_SCAN_DIR));
    let work_dir = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_WORK_DIR)
    });
    let max_evaluated_candidates = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max evaluated candidates '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_MAX_EVALUATED_CANDIDATES);
    let runtime_replay_report_path = next_path(&mut args, DEFAULT_RUNTIME_REPLAY_REPORT);
    let billing_request_report_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_REPORT);
    let billing_request_jsonl_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_JSONL);
    let billing_contract_report_path = next_path(&mut args, DEFAULT_BILLING_CONTRACT_REPORT);
    let provider_correlation_audit_report_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let selected_keys = read_selected_billing_keys(&billing_request_jsonl_path)?;
    let mut candidate_file_count = 0usize;
    let mut skipped_non_file_count = 0usize;
    let mut skipped_extension_count = 0usize;
    let mut skipped_name_filter_count = 0usize;
    let mut skipped_size_count = 0usize;
    let mut candidates = Vec::<ProviderExportCandidate>::new();

    let mut entries = std::fs::read_dir(&scan_dir)
        .map_err(|error| format!("failed to read scan dir '{}': {error}", scan_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to list scan dir '{}': {error}", scan_dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            skipped_non_file_count += 1;
            continue;
        }
        if !is_provider_export_extension(&path) {
            skipped_extension_count += 1;
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_owned();
        if !is_likely_provider_export_name(&file_name) {
            skipped_name_filter_count += 1;
            continue;
        }
        let file_size = entry
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|error| {
                format!(
                    "failed to inspect provider export candidate '{}': {error}",
                    path.display()
                )
            })?;
        if file_size > MAX_AUTOSCAN_CANDIDATE_BYTES {
            skipped_size_count += 1;
            continue;
        }
        candidate_file_count += 1;
        let (key_hit_count, line_count) = count_selected_key_hits(&path, &selected_keys)?;
        if key_hit_count == 0 {
            continue;
        }
        candidates.push(ProviderExportCandidate {
            path,
            file_name,
            key_hit_count,
            line_count,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .key_hit_count
            .cmp(&left.key_hit_count)
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    let matching_candidate_count = candidates.len();

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create online portfolio provider export autoscan work dir '{}': {error}",
            work_dir.display()
        )
    })?;

    let mut evaluated = Vec::new();
    for (index, candidate) in candidates.iter().take(max_evaluated_candidates).enumerate() {
        let stem = sanitize_file_stem(&candidate.file_name);
        let candidate_work_dir = work_dir.join(format!("{index:03}-{stem}"));
        let candidate_report_path =
            candidate_work_dir.join("provider-export-admission.report.json");
        let mut admission_args = vec![
            candidate_report_path.display().to_string(),
            candidate.path.display().to_string(),
            candidate_work_dir.display().to_string(),
            runtime_replay_report_path.display().to_string(),
            billing_request_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            billing_contract_report_path.display().to_string(),
        ];
        if let Some(path) = &provider_correlation_audit_report_path {
            admission_args.push(path.display().to_string());
        }
        run_phase_stream_online_miner_portfolio_provider_export_admission_v1(
            admission_args.into_iter(),
        )?;
        let report = read_json_value(&candidate_report_path)?;
        evaluated.push(EvaluatedProviderExportCandidate {
            provider_export_jsonl_path: candidate.path.display().to_string(),
            work_dir: candidate_work_dir.display().to_string(),
            report_path: candidate_report_path.display().to_string(),
            key_hit_count: candidate.key_hit_count,
            line_count: candidate.line_count,
            verdict: json_string(&report, &["verdict"]).unwrap_or_default(),
            provider_billing_evidence_present: json_bool(
                &report,
                &["provider_billing_evidence_present"],
            )
            .unwrap_or(false),
            provider_export_attestation_present: json_bool(
                &report,
                &["provider_export_attestation", "present"],
            )
            .unwrap_or(false),
            provider_export_attestation_valid: json_bool(
                &report,
                &["provider_export_attestation", "valid"],
            )
            .unwrap_or(false),
            provider_export_attestation_blockers: json_string_vec_path(
                &report,
                &["provider_export_attestation", "blockers"],
            ),
            product_ready: json_bool(&report, &["product_ready"]).unwrap_or(false),
            promotion_ready: json_bool(&report, &["promotion_ready"]).unwrap_or(false),
            product_promotion_allowed: json_bool(&report, &["product_promotion_allowed"])
                .unwrap_or(false),
            market_money_claim_allowed: json_bool(&report, &["market_money_claim_allowed"])
                .unwrap_or(false),
            request_rows: json_usize_path(&report, &["request_rows"]).unwrap_or(0),
            rows_enriched_provider_cost: json_usize_path(&report, &["rows_enriched_provider_cost"])
                .unwrap_or(0),
            rows_enriched_provider_tokens: json_usize_path(
                &report,
                &["rows_enriched_provider_tokens"],
            )
            .unwrap_or(0),
            missing_billing_request_rows: json_usize_path(
                &report,
                &["missing_billing_request_rows"],
            )
            .unwrap_or(0),
        });
    }

    let provider_evidence_present_count = evaluated
        .iter()
        .filter(|candidate| candidate.provider_billing_evidence_present)
        .count();
    let provider_export_attestation_present_count = evaluated
        .iter()
        .filter(|candidate| candidate.provider_export_attestation_present)
        .count();
    let provider_export_attestation_valid_count = evaluated
        .iter()
        .filter(|candidate| candidate.provider_export_attestation_valid)
        .count();
    let product_ready_count = evaluated
        .iter()
        .filter(|candidate| candidate.product_ready)
        .count();
    let promotion_ready_count = evaluated
        .iter()
        .filter(|candidate| candidate.promotion_ready)
        .count();
    let money_ready_count = evaluated
        .iter()
        .filter(|candidate| candidate.market_money_claim_allowed)
        .count();
    let verdict = if product_ready_count > 0 && promotion_ready_count > 0 && money_ready_count > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_V1_FOUND_PRODUCT_READY_EVIDENCE"
    } else if matching_candidate_count > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_V1_NO_PRODUCT_READY_EVIDENCE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_AUTOSCAN_V1_NO_MATCHING_CANDIDATE"
    };

    let candidates_json = candidates
        .iter()
        .map(|candidate| {
            serde_json::json!({
                "provider_export_jsonl_path": candidate.path,
                "key_hit_count": candidate.key_hit_count,
                "line_count": candidate.line_count
            })
        })
        .collect::<Vec<_>>();
    let evaluated_json = serde_json::to_value(&evaluated)
        .map_err(|error| format!("failed to serialize autoscan evaluated candidates: {error}"))?;

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
    let mut report = Map::new();
    insert_json(
        &mut report,
        "report_kind",
        "phase_stream_online_miner_portfolio_provider_export_autoscan_v1",
    )?;
    insert_json(
        &mut report,
        "mode",
        "cold_provider_export_autoscan_no_local_accept",
    )?;
    insert_json(&mut report, "scan_dir", scan_dir)?;
    insert_json(&mut report, "work_dir", work_dir)?;
    insert_json(
        &mut report,
        "runtime_replay_report_path",
        runtime_replay_report_path,
    )?;
    insert_json(
        &mut report,
        "billing_request_report_path",
        billing_request_report_path,
    )?;
    insert_json(
        &mut report,
        "billing_request_jsonl_path",
        billing_request_jsonl_path,
    )?;
    insert_json(
        &mut report,
        "billing_contract_report_path",
        billing_contract_report_path,
    )?;
    insert_json(
        &mut report,
        "provider_correlation_audit_report_path",
        provider_correlation_audit_report_path,
    )?;
    insert_json(&mut report, "selected_key_count", selected_keys.len())?;
    insert_json(&mut report, "candidate_file_count", candidate_file_count)?;
    insert_json(
        &mut report,
        "skipped_non_file_count",
        skipped_non_file_count,
    )?;
    insert_json(
        &mut report,
        "skipped_extension_count",
        skipped_extension_count,
    )?;
    insert_json(
        &mut report,
        "skipped_name_filter_count",
        skipped_name_filter_count,
    )?;
    insert_json(&mut report, "skipped_size_count", skipped_size_count)?;
    insert_json(
        &mut report,
        "max_autoscan_candidate_bytes",
        MAX_AUTOSCAN_CANDIDATE_BYTES,
    )?;
    insert_json(
        &mut report,
        "matching_candidate_count",
        matching_candidate_count,
    )?;
    insert_json(
        &mut report,
        "max_evaluated_candidates",
        max_evaluated_candidates,
    )?;
    insert_json(&mut report, "evaluated_candidate_count", evaluated.len())?;
    insert_json(
        &mut report,
        "provider_evidence_present_count",
        provider_evidence_present_count,
    )?;
    insert_json(
        &mut report,
        "provider_export_attestation_required_for_money_claim",
        true,
    )?;
    insert_json(
        &mut report,
        "provider_export_attestation_present_count",
        provider_export_attestation_present_count,
    )?;
    insert_json(
        &mut report,
        "provider_export_attestation_valid_count",
        provider_export_attestation_valid_count,
    )?;
    insert_json(&mut report, "product_ready_count", product_ready_count)?;
    insert_json(&mut report, "promotion_ready_count", promotion_ready_count)?;
    insert_json(&mut report, "money_ready_count", money_ready_count)?;
    insert_json(&mut report, "candidates", candidates_json)?;
    insert_json(&mut report, "evaluated", evaluated_json)?;
    insert_json(&mut report, "local_accept_enabled", false)?;
    insert_json(&mut report, "auto_promote_enabled", false)?;
    insert_json(&mut report, "serving_registry_mutated", false)?;
    insert_json(&mut report, "product_runtime_changed", false)?;
    insert_json(&mut report, "serving_runtime_changed", false)?;
    insert_json(
        &mut report,
        "market_money_claim_allowed",
        money_ready_count > 0,
    )?;
    insert_json(&mut report, "forbidden_flags", forbidden_flags)?;
    insert_json(&mut report, "verdict", verdict)?;
    insert_json(
        &mut report,
        "boundary",
        "online-miner portfolio provider export autoscan only: scans local candidate export files for selected billing keys and runs provider-export admission on matching candidates; does not compile, mine, serve, mutate registry, enable local_accept, estimate missing money, or use legacy nwrb",
    )?;
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_provider_export_autoscan_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  candidate_file_count: {candidate_file_count}");
    println!("  matching_candidate_count: {matching_candidate_count}");
    println!("  evaluated_candidate_count: {}", evaluated.len());
    println!("  provider_evidence_present_count: {provider_evidence_present_count}");
    println!(
        "  provider_export_attestation_valid_count: {provider_export_attestation_valid_count}"
    );
    println!("  product_ready_count: {product_ready_count}");
    println!("  market_money_claim_allowed: {}", money_ready_count > 0);
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_selected_billing_keys(path: &Path) -> Result<Vec<String>, String> {
    let file = File::open(path).map_err(|error| {
        format!(
            "failed to read billing request '{}': {error}",
            path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let mut keys = Vec::<String>::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read billing request '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(&line).map_err(|error| {
            format!(
                "failed to parse billing request '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if let Some(value) =
            json_string(&row, &["request_fingerprint"]).filter(|value| !value.is_empty())
        {
            keys.push(value);
        }
        if let Some(value) =
            json_string(&row, &["exact_cache_key"]).filter(|value| !value.is_empty())
        {
            keys.push(value);
        }
        if let Some(values) = row.get("match_keys").and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                if !value.is_empty() {
                    keys.push(value.to_owned());
                }
            }
        }
    }
    keys.sort();
    keys.dedup();
    Ok(keys)
}

fn is_provider_export_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension, "jsonl" | "csv" | "tsv"))
}

fn is_likely_provider_export_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    let has_provider_signal = lower.contains("provider")
        || lower.contains("billing")
        || lower.contains("usage")
        || lower.contains("invoice")
        || lower.contains("cost");
    let is_known_proof_bulk = lower.contains("trace")
        || lower.contains("decision")
        || lower.contains("candidate")
        || lower.contains("denominator")
        || lower.contains("autoscan")
        || lower.contains("chain")
        || lower.contains("joined")
        || lower.contains("pack")
        || lower.contains("request")
        || lower.contains("template")
        || lower.contains("missing")
        || lower.contains("negative")
        || lower.contains("normalized")
        || lower.contains("report");
    has_provider_signal && !is_known_proof_bulk
}

fn count_selected_key_hits(
    path: &Path,
    selected_keys: &[String],
) -> Result<(usize, usize), String> {
    let file = File::open(path)
        .map_err(|error| format!("failed to read candidate '{}': {error}", path.display()))?;
    let reader = BufReader::new(file);
    let mut key_hit_count = 0usize;
    let mut line_count = 0usize;
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read candidate '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        line_count += 1;
        key_hit_count += selected_keys
            .iter()
            .filter(|key| line.contains(key.as_str()))
            .count();
    }
    Ok((key_hit_count, line_count))
}

fn next_path<I>(args: &mut I, default_path: &str) -> PathBuf
where
    I: Iterator<Item = String>,
{
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_path))
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_string_vec_path(value: &Value, path: &[&str]) -> Vec<String> {
    let mut current = value;
    for key in path {
        current = match current.get(*key) {
            Some(value) => value,
            None => return Vec::new(),
        };
    }
    current
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn insert_json<T>(report: &mut Map<String, Value>, key: &str, value: T) -> Result<(), String>
where
    T: Serialize,
{
    let value = serde_json::to_value(value)
        .map_err(|error| format!("failed to serialize autoscan report field '{key}': {error}"))?;
    report.insert(key.to_owned(), value);
    Ok(())
}
