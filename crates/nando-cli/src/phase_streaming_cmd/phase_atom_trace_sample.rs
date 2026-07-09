use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use super::write_json_file;

const DEFAULT_TRACE_SAMPLE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-phase-atom-trace-sample-v1.report.json";
const DEFAULT_TRACE_SAMPLE_OUTPUT: &str =
    "target/nando-wave/streaming/phase-stream-phase-atom-trace-sample-v1.jsonl";

#[derive(Serialize)]
struct TraceSampleReport {
    report_kind: &'static str,
    output_path: String,
    input_paths: Vec<String>,
    sample_modulus: usize,
    sample_remainder: usize,
    read_rows: usize,
    written_rows: usize,
    skipped_rows: usize,
    json_parse_for_sampling_used: bool,
    keep_verified_safe_rows: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_phase_atom_trace_sample_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TRACE_SAMPLE_REPORT));
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TRACE_SAMPLE_OUTPUT));
    let sample_modulus = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid sample_modulus '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(2);
    if sample_modulus == 0 {
        return Err("sample_modulus must be greater than zero".to_owned());
    }
    let sample_remainder = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid sample_remainder '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    if sample_remainder >= sample_modulus {
        return Err(format!(
            "sample_remainder {sample_remainder} must be smaller than sample_modulus {sample_modulus}"
        ));
    }
    let mut remaining_args = args.collect::<Vec<_>>();
    let keep_verified_safe_rows = if remaining_args
        .first()
        .is_some_and(|arg| arg == "--keep-verified-safe")
    {
        remaining_args.remove(0);
        true
    } else {
        false
    };
    let input_paths = remaining_args
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let output_file = std::fs::File::create(&output_path)
        .map_err(|error| format!("failed to create '{}': {error}", output_path.display()))?;
    let mut writer = std::io::BufWriter::new(output_file);
    let mut read_rows = 0usize;
    let mut written_rows = 0usize;

    for path in &input_paths {
        let input = std::fs::File::open(path)
            .map_err(|error| format!("failed to open '{}': {error}", path.display()))?;
        let reader = std::io::BufReader::new(input);
        for line in reader.lines() {
            let line =
                line.map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
            if line.trim().is_empty() {
                continue;
            }
            let row_index = read_rows;
            read_rows += 1;
            if row_index % sample_modulus == sample_remainder
                || (keep_verified_safe_rows && line_has_verified_safe_accept(&line))
            {
                writer
                    .write_all(line.as_bytes())
                    .and_then(|_| writer.write_all(b"\n"))
                    .map_err(|error| {
                        format!("failed to write '{}': {error}", output_path.display())
                    })?;
                written_rows += 1;
            }
        }
    }
    writer
        .flush()
        .map_err(|error| format!("failed to flush '{}': {error}", output_path.display()))?;

    let skipped_rows = read_rows.saturating_sub(written_rows);
    let verdict = if written_rows > 0 {
        "PHASE_STREAM_PHASE_ATOM_TRACE_SAMPLE_V1_PASS_SAMPLE_WRITTEN"
    } else {
        "PHASE_STREAM_PHASE_ATOM_TRACE_SAMPLE_V1_WATCH_EMPTY_SAMPLE"
    };
    let report = TraceSampleReport {
        report_kind: "phase_stream_phase_atom_trace_sample_v1",
        output_path: output_path.display().to_string(),
        input_paths: input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        sample_modulus,
        sample_remainder,
        read_rows,
        written_rows,
        skipped_rows,
        json_parse_for_sampling_used: keep_verified_safe_rows,
        keep_verified_safe_rows,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "cold deterministic trace sampler only: copies a modulo slice of append-only phase-atom JSONL for mini .nwpc survival diagnostics; optional verifier-positive oversampling is sampling-only and must be followed by full-trace quarantine; it does not score, compile, promote, serve, enable local_accept, or claim market money",
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_phase_atom_trace_sample_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_path: {}", output_path.display());
    println!("  sample_modulus: {sample_modulus}");
    println!("  sample_remainder: {sample_remainder}");
    println!("  keep_verified_safe_rows: {keep_verified_safe_rows}");
    println!("  read_rows: {read_rows}");
    println!("  written_rows: {written_rows}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn line_has_verified_safe_accept(line: &str) -> bool {
    serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|value| value.get("verified_safe_accept").and_then(Value::as_bool))
        .unwrap_or(false)
}
