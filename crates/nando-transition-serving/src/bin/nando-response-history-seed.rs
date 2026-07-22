use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nando_operator_kernel::RelationFrame;
use nando_operator_learning::{teacher_action_symbol, teacher_program_signature};
use nando_response_actor::{OnlineResponseStream, OnlineResponseTailConfig};
use serde::Serialize;
use sha2::{Digest, Sha256};

const DEFAULT_ROWS_PER_TEACHER: usize = 128;
const MAX_TEACHER_PROGRAMS: usize = 256;

#[derive(Debug)]
struct SeedCandidate {
    rank: String,
    frame: RelationFrame,
}

#[derive(Serialize)]
struct SeedReceipt {
    schema: &'static str,
    source_path: String,
    source_bytes: u64,
    source_sha256: String,
    checkpoint_path: String,
    rows_per_teacher_cap: usize,
    teacher_program_cap: usize,
    lines_seen: u64,
    parse_errors: u64,
    rows_without_teacher: u64,
    rejected_rows_skipped: u64,
    rows_skipped_by_program_cap: u64,
    eligible_rows: u64,
    selected_rows: usize,
    imported_rows: usize,
    teacher_programs: usize,
    selected_by_action: BTreeMap<String, usize>,
    support_only: bool,
    frozen_future_rows_claimed: u64,
    scan_millis: u128,
    checkpoint_open_millis: u128,
    train_persist_millis: u128,
    elapsed_millis: u128,
}

fn main() -> Result<(), String> {
    let started = Instant::now();
    let mut arguments = std::env::args_os().skip(1);
    let source_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let miner_report_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let checkpoint_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let receipt_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let rows_per_teacher = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
        .unwrap_or(DEFAULT_ROWS_PER_TEACHER)
        .clamp(1, DEFAULT_ROWS_PER_TEACHER);

    let source = File::open(&source_path)
        .map_err(|error| format!("history_seed_source_open:{}:{error}", source_path.display()))?;
    let source_bytes = source
        .metadata()
        .map_err(|error| format!("history_seed_source_metadata:{error}"))?
        .len();
    let mut reader = BufReader::new(source);
    let mut source_hasher = Sha256::new();
    let mut pools = BTreeMap::<String, Vec<SeedCandidate>>::new();
    let mut selected_by_action = BTreeMap::<String, usize>::new();
    let mut buffer = Vec::new();
    let mut lines_seen = 0_u64;
    let mut parse_errors = 0_u64;
    let mut rows_without_teacher = 0_u64;
    let mut rejected_rows_skipped = 0_u64;
    let mut rows_skipped_by_program_cap = 0_u64;
    let mut eligible_rows = 0_u64;

    loop {
        buffer.clear();
        let bytes = reader
            .read_until(b'\n', &mut buffer)
            .map_err(|error| format!("history_seed_source_read:{error}"))?;
        if bytes == 0 {
            break;
        }
        source_hasher.update(&buffer);
        lines_seen = lines_seen.saturating_add(1);
        let frame = match serde_json::from_slice::<RelationFrame>(&buffer) {
            Ok(frame) => frame,
            Err(_) => {
                parse_errors = parse_errors.saturating_add(1);
                continue;
            }
        };
        if frame.verifier_label != Some(true) {
            rejected_rows_skipped = rejected_rows_skipped.saturating_add(1);
            continue;
        }
        let Some(signature) = teacher_program_signature(&frame) else {
            rows_without_teacher = rows_without_teacher.saturating_add(1);
            continue;
        };
        eligible_rows = eligible_rows.saturating_add(1);
        if !pools.contains_key(&signature) && pools.len() >= MAX_TEACHER_PROGRAMS {
            rows_skipped_by_program_cap = rows_skipped_by_program_cap.saturating_add(1);
            continue;
        }
        let candidates = pools.entry(signature).or_default();
        let rank = frame.frame_id_sha256.clone();
        if candidates.len() < rows_per_teacher {
            candidates.push(SeedCandidate { rank, frame });
            continue;
        }
        let Some((replace_index, largest)) = candidates
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.rank.cmp(&right.rank))
        else {
            continue;
        };
        if rank < largest.rank {
            candidates[replace_index] = SeedCandidate { rank, frame };
        }
    }
    let scan_millis = started.elapsed().as_millis();

    let mut selected = pools
        .into_values()
        .flatten()
        .map(|candidate| candidate.frame)
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        (
            left.observed_at_unix_nanos,
            left.session_id_sha256.as_str(),
            left.frame_id_sha256.as_str(),
        )
            .cmp(&(
                right.observed_at_unix_nanos,
                right.session_id_sha256.as_str(),
                right.frame_id_sha256.as_str(),
            ))
    });
    for frame in &selected {
        *selected_by_action
            .entry(teacher_action_symbol(frame))
            .or_default() += 1;
    }

    let teacher_programs = selected
        .iter()
        .filter_map(teacher_program_signature)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let selected_rows = selected.len();
    let checkpoint_open_started = Instant::now();
    let mut stream = OnlineResponseStream::open_streaming(OnlineResponseTailConfig {
        input_path: source_path.clone(),
        report_path: miner_report_path,
        checkpoint_path: checkpoint_path.clone(),
        idle_sleep: Duration::from_millis(200),
    })?;
    let checkpoint_open_millis = checkpoint_open_started.elapsed().as_millis();
    let rows_before = stream.report().rows_seen;
    let train_started = Instant::now();
    let result = stream.train_replay_batch(selected)?;
    let train_persist_millis = train_started.elapsed().as_millis();
    let imported_rows = result.rows_seen.saturating_sub(rows_before);

    let receipt = SeedReceipt {
        schema: "nando.response-history-seed.v1",
        source_path: source_path.display().to_string(),
        source_bytes,
        source_sha256: format!("{:x}", source_hasher.finalize()),
        checkpoint_path: checkpoint_path.display().to_string(),
        rows_per_teacher_cap: rows_per_teacher,
        teacher_program_cap: MAX_TEACHER_PROGRAMS,
        lines_seen,
        parse_errors,
        rows_without_teacher,
        rejected_rows_skipped,
        rows_skipped_by_program_cap,
        eligible_rows,
        selected_rows,
        imported_rows,
        teacher_programs,
        selected_by_action,
        support_only: true,
        frozen_future_rows_claimed: 0,
        scan_millis,
        checkpoint_open_millis,
        train_persist_millis,
        elapsed_millis: started.elapsed().as_millis(),
    };
    persist_receipt(&receipt_path, &receipt)?;
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("history_seed_receipt_encode:{error}"))?
    );
    Ok(())
}

fn persist_receipt(path: &Path, receipt: &SeedReceipt) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("history_seed_receipt_dir:{error}"))?;
    }
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("history_seed_receipt_encode:{error}"))?;
    fs::write(&temporary, bytes).map_err(|error| format!("history_seed_receipt_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("history_seed_receipt_rename:{error}"))
}

fn usage() -> String {
    "usage: nando-response-history-seed <verified-relation-ledger> <miner-report> <miner-checkpoint> <seed-receipt> [rows-per-teacher<=128]".to_owned()
}
