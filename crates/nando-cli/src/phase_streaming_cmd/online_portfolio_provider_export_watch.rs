use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::online_portfolio_provider_export_autoscan::run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1;
use super::{json_bool, json_string, read_json_value, write_json_file};

const DEFAULT_PROVIDER_EXPORT_WATCH_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-watch-v1.report.json";
const DEFAULT_PROVIDER_EXPORT_SCAN_DIR: &str = "target/nando-wave/streaming";
const DEFAULT_PROVIDER_EXPORT_WATCH_WORK_DIR: &str =
    "target/nando-wave/streaming/online-miner-portfolio-provider-export-watch-v1";
const DEFAULT_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-run-check-parse-detail-multisplit-v5.jsonl";
const DEFAULT_BILLING_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1-run-check-parse-detail-multisplit-v5.report.json";
const DEFAULT_CYCLES: usize = 1;
const DEFAULT_SLEEP_MS: u64 = 0;
const DEFAULT_MAX_EVALUATED_CANDIDATES: usize = 8;

#[derive(Serialize)]
struct WatchCycle {
    cycle_index: usize,
    autoscan_report_path: String,
    verdict: String,
    matching_candidate_count: usize,
    evaluated_candidate_count: usize,
    provider_evidence_present_count: usize,
    provider_export_attestation_valid_count: usize,
    product_ready_count: usize,
    promotion_ready_count: usize,
    money_ready_count: usize,
    market_money_claim_allowed: bool,
    local_accept_enabled: bool,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_provider_export_watch_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_WATCH_REPORT));
    let scan_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_SCAN_DIR));
    let work_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_WATCH_WORK_DIR));
    let cycles = parse_optional_usize(args.next(), DEFAULT_CYCLES, "cycles")?.max(1);
    let sleep_ms = parse_optional_u64(args.next(), DEFAULT_SLEEP_MS, "sleep-ms")?;
    let max_evaluated_candidates = parse_optional_usize(
        args.next(),
        DEFAULT_MAX_EVALUATED_CANDIDATES,
        "max evaluated candidates",
    )?;
    let runtime_replay_report_path = next_path(&mut args, DEFAULT_RUNTIME_REPLAY_REPORT);
    let billing_request_report_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_REPORT);
    let billing_request_jsonl_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_JSONL);
    let billing_contract_report_path = next_path(&mut args, DEFAULT_BILLING_CONTRACT_REPORT);
    let provider_correlation_audit_report_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create provider export watch dir '{}': {error}",
            work_dir.display()
        )
    })?;
    let history_jsonl_path = work_dir.join("provider-export-watch.history.jsonl");
    let mut history = String::new();
    let mut cycle_reports = Vec::<WatchCycle>::new();

    for cycle_index in 0..cycles {
        let cycle_dir = work_dir.join(format!("cycle-{cycle_index:03}"));
        let cycle_report_path = cycle_dir.join("provider-export-autoscan.report.json");
        let mut autoscan_args = vec![
            cycle_report_path.display().to_string(),
            scan_dir.display().to_string(),
            cycle_dir.display().to_string(),
            max_evaluated_candidates.to_string(),
            runtime_replay_report_path.display().to_string(),
            billing_request_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            billing_contract_report_path.display().to_string(),
        ];
        if let Some(path) = &provider_correlation_audit_report_path {
            autoscan_args.push(path.display().to_string());
        }
        run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1(
            autoscan_args.into_iter(),
        )?;
        let autoscan = read_json_value(&cycle_report_path)?;
        let cycle = WatchCycle {
            cycle_index,
            autoscan_report_path: path_string(&cycle_report_path),
            verdict: json_string(&autoscan, &["verdict"]).unwrap_or_default(),
            matching_candidate_count: json_usize(&autoscan, &["matching_candidate_count"]),
            evaluated_candidate_count: json_usize(&autoscan, &["evaluated_candidate_count"]),
            provider_evidence_present_count: json_usize(
                &autoscan,
                &["provider_evidence_present_count"],
            ),
            provider_export_attestation_valid_count: json_usize(
                &autoscan,
                &["provider_export_attestation_valid_count"],
            ),
            product_ready_count: json_usize(&autoscan, &["product_ready_count"]),
            promotion_ready_count: json_usize(&autoscan, &["promotion_ready_count"]),
            money_ready_count: json_usize(&autoscan, &["money_ready_count"]),
            market_money_claim_allowed: json_bool(&autoscan, &["market_money_claim_allowed"])
                .unwrap_or(false),
            local_accept_enabled: json_bool(&autoscan, &["local_accept_enabled"]).unwrap_or(false),
        };
        history.push_str(
            &serde_json::to_string(&cycle)
                .map_err(|error| format!("failed to serialize watch cycle: {error}"))?,
        );
        history.push('\n');
        let product_ready = cycle.product_ready_count > 0
            && cycle.promotion_ready_count > 0
            && cycle.money_ready_count > 0
            && cycle.market_money_claim_allowed;
        cycle_reports.push(cycle);
        if product_ready {
            break;
        }
        if cycle_index + 1 < cycles && sleep_ms > 0 {
            std::thread::sleep(Duration::from_millis(sleep_ms));
        }
    }

    std::fs::write(&history_jsonl_path, history).map_err(|error| {
        format!(
            "failed to write provider export watch history '{}': {error}",
            history_jsonl_path.display()
        )
    })?;

    let cycles_completed = cycle_reports.len();
    let product_ready_cycles = cycle_reports
        .iter()
        .filter(|cycle| {
            cycle.product_ready_count > 0
                && cycle.promotion_ready_count > 0
                && cycle.money_ready_count > 0
                && cycle.market_money_claim_allowed
        })
        .count();
    let matching_candidate_cycles = cycle_reports
        .iter()
        .filter(|cycle| cycle.matching_candidate_count > 0)
        .count();
    let latest = cycle_reports.last();
    let verdict = if product_ready_cycles > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_WATCH_V1_FOUND_PRODUCT_READY_EVIDENCE"
    } else if matching_candidate_cycles > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_WATCH_V1_WATCH_MATCHING_BUT_BLOCKED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_WATCH_V1_WATCH_NO_MATCHING_CANDIDATE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_provider_export_watch_v1",
        "mode": "bounded_cold_provider_export_watch_no_local_accept",
        "scan_dir": path_string(&scan_dir),
        "work_dir": path_string(&work_dir),
        "history_jsonl_path": path_string(&history_jsonl_path),
        "runtime_replay_report_path": path_string(&runtime_replay_report_path),
        "billing_request_report_path": path_string(&billing_request_report_path),
        "billing_request_jsonl_path": path_string(&billing_request_jsonl_path),
        "billing_contract_report_path": path_string(&billing_contract_report_path),
        "provider_correlation_audit_report_path": provider_correlation_audit_report_path
            .as_ref()
            .map(|path| path_string(path)),
        "cycles_requested": cycles,
        "cycles_completed": cycles_completed,
        "cycle_sleep_ms": sleep_ms,
        "max_evaluated_candidates": max_evaluated_candidates,
        "matching_candidate_cycles": matching_candidate_cycles,
        "product_ready_cycles": product_ready_cycles,
        "latest_cycle": latest,
        "cycles": cycle_reports,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": product_ready_cycles > 0,
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
        "boundary": "bounded provider-export inbox watch only: repeatedly scans external provider-export candidates and runs autoscan/admission; does not mine, compile, serve, mutate registry, auto-promote, enable local_accept, estimate missing money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_provider_export_watch_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  history_jsonl_path: {}", history_jsonl_path.display());
    println!("  cycles_completed: {cycles_completed}");
    println!("  matching_candidate_cycles: {matching_candidate_cycles}");
    println!("  product_ready_cycles: {product_ready_cycles}");
    println!("  market_money_claim_allowed: {}", product_ready_cycles > 0);
    println!("  verdict: {verdict}");
    Ok(())
}

fn parse_optional_usize(
    value: Option<String>,
    default_value: usize,
    label: &str,
) -> Result<usize, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid {label} '{value}': {error}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default_value))
}

fn parse_optional_u64(
    value: Option<String>,
    default_value: u64,
    label: &str,
) -> Result<u64, String> {
    value
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid {label} '{value}': {error}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default_value))
}

fn next_path<I>(args: &mut I, default_path: &str) -> PathBuf
where
    I: Iterator<Item = String>,
{
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_path))
}

fn json_usize(value: &Value, path: &[&str]) -> usize {
    let mut current = value;
    for key in path {
        current = match current.get(*key) {
            Some(value) => value,
            None => return 0,
        };
    }
    current
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(0)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
