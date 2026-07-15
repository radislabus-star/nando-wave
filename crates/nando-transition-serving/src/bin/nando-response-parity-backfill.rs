use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nando_response_actor::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, OnlineResponseStream, OnlineResponseTailConfig,
    RelationFrame, RuntimeParityCase, teacher_action_symbol, teacher_program_signature,
    teacher_transition_from_completed,
};
use nando_transition_serving::verified_training_cases_from_session_tail;
use serde::Serialize;

const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_CASES_PER_TEACHER: usize = 64;

struct ParityCandidate {
    rank: String,
    frame: RelationFrame,
    parity: RuntimeParityCase,
}

#[derive(Serialize)]
struct ParityBackfillReceipt {
    schema: &'static str,
    sessions_root: String,
    files_scanned: usize,
    source_bytes_considered: u64,
    max_file_bytes: u64,
    cases_per_teacher_cap: usize,
    frames_seen: u64,
    parity_cases_seen: u64,
    selected_cases: usize,
    imported_rows: usize,
    selected_by_action: BTreeMap<String, usize>,
    work_slices: usize,
    exact_checks: usize,
    max_future_rows: usize,
    max_runtime_parity_rows: usize,
    replay_economics: bool,
    scan_millis: u128,
    checkpoint_open_millis: u128,
    apply_millis: u128,
    work_millis: u128,
    persist_millis: u128,
    elapsed_millis: u128,
}

fn main() -> Result<(), String> {
    let started = Instant::now();
    let mut arguments = std::env::args_os().skip(1);
    let sessions_root = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let miner_report_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let checkpoint_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let receipt_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let max_file_bytes = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_FILE_BYTES)
        .clamp(1, DEFAULT_MAX_FILE_BYTES);
    let cases_per_teacher = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
        .unwrap_or(DEFAULT_CASES_PER_TEACHER)
        .clamp(1, DEFAULT_CASES_PER_TEACHER);

    let mut files = Vec::new();
    collect_session_files(&sessions_root, &mut files)?;
    files.sort();
    let mut source_bytes_considered = 0_u64;
    let mut frames_seen = 0_u64;
    let mut parity_cases_seen = 0_u64;
    let mut seen_frames = BTreeSet::new();
    let mut pools = BTreeMap::<String, Vec<ParityCandidate>>::new();
    for path in &files {
        let length = fs::metadata(path)
            .map_err(|error| format!("parity_backfill_metadata:{}:{error}", path.display()))?
            .len();
        source_bytes_considered =
            source_bytes_considered.saturating_add(length.min(max_file_bytes));
        for (frame, parity) in verified_training_cases_from_session_tail(path, max_file_bytes)? {
            frames_seen = frames_seen.saturating_add(1);
            let Some(parity) = parity else { continue };
            parity_cases_seen = parity_cases_seen.saturating_add(1);
            if frame.verifier_label != Some(true)
                || !seen_frames.insert(frame.frame_id_sha256.clone())
            {
                continue;
            }
            let Some(signature) = teacher_program_signature(&frame) else {
                continue;
            };
            push_session_diverse(
                pools.entry(signature).or_default(),
                ParityCandidate {
                    rank: frame.frame_id_sha256.clone(),
                    frame,
                    parity,
                },
                cases_per_teacher,
            );
        }
    }
    let scan_millis = started.elapsed().as_millis();

    let mut selected = pools.into_values().flatten().collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        (
            left.frame.observed_at_unix_nanos,
            left.frame.session_id_sha256.as_str(),
            left.frame.frame_id_sha256.as_str(),
        )
            .cmp(&(
                right.frame.observed_at_unix_nanos,
                right.frame.session_id_sha256.as_str(),
                right.frame.frame_id_sha256.as_str(),
            ))
    });
    let mut selected_by_action = BTreeMap::<String, usize>::new();
    for candidate in &selected {
        *selected_by_action
            .entry(teacher_action_symbol(&candidate.frame))
            .or_default() += 1;
    }

    let selected_cases = selected.len();
    let checkpoint_open_started = Instant::now();
    let mut stream = OnlineResponseStream::open_streaming(OnlineResponseTailConfig {
        input_path: PathBuf::new(),
        report_path: miner_report_path,
        checkpoint_path: checkpoint_path.clone(),
        idle_sleep: Duration::from_millis(200),
    })?;
    let checkpoint_open_millis = checkpoint_open_started.elapsed().as_millis();
    let rows_before = stream.report().rows_seen;
    let apply_started = Instant::now();
    for candidate in selected {
        let economics = Some(EconomicsReceipt {
            schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
            exact_input_tokens: candidate.frame.estimated_input_tokens,
            ordinary: false,
            controlled: false,
            replay: true,
            dedupe_eligible: true,
            provider_evidence_ref_sha256: candidate.frame.evidence_ref_sha256.clone(),
        });
        let mut transition = teacher_transition_from_completed(&candidate.frame, economics)
            .map_err(|error| format!("parity_backfill_teacher_transition:{error:?}"))?;
        transition.runtime_parity_case = Some(candidate.parity);
        stream.apply_teacher_transition(transition)?;
    }
    let apply_millis = apply_started.elapsed().as_millis();
    let work_started = Instant::now();
    let mut work_slices = 0_usize;
    let mut exact_checks = 0_usize;
    loop {
        let checks = stream.run_self_training_work_slice();
        exact_checks = exact_checks.saturating_add(checks);
        work_slices = work_slices.saturating_add(1);
        if checks == 0 && !stream.has_self_training_work() {
            break;
        }
        if work_slices >= 4_096 {
            return Err("parity_backfill_work_budget_exhausted".to_owned());
        }
    }
    let work_millis = work_started.elapsed().as_millis();
    let persist_started = Instant::now();
    stream.persist_now()?;
    let persist_millis = persist_started.elapsed().as_millis();
    let report = stream.report();
    let imported_rows = report.rows_seen.saturating_sub(rows_before);
    let max_future_rows = report
        .self_training_v2
        .generations
        .iter()
        .map(|generation| generation.future_rows)
        .max()
        .unwrap_or(0);
    let max_runtime_parity_rows = report
        .self_training_v2
        .generations
        .iter()
        .map(|generation| generation.runtime_parity_rows)
        .max()
        .unwrap_or(0);
    let receipt = ParityBackfillReceipt {
        schema: "nando.response-parity-backfill.v1",
        sessions_root: sessions_root.display().to_string(),
        files_scanned: files.len(),
        source_bytes_considered,
        max_file_bytes,
        cases_per_teacher_cap: cases_per_teacher,
        frames_seen,
        parity_cases_seen,
        selected_cases,
        imported_rows,
        selected_by_action,
        work_slices,
        exact_checks,
        max_future_rows,
        max_runtime_parity_rows,
        replay_economics: true,
        scan_millis,
        checkpoint_open_millis,
        apply_millis,
        work_millis,
        persist_millis,
        elapsed_millis: started.elapsed().as_millis(),
    };
    persist_receipt(&receipt_path, &receipt)?;
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("parity_backfill_receipt_encode:{error}"))?
    );
    Ok(())
}

fn push_session_diverse(
    candidates: &mut Vec<ParityCandidate>,
    incoming: ParityCandidate,
    capacity: usize,
) {
    if candidates.len() < capacity {
        candidates.push(incoming);
        return;
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    for candidate in candidates.iter() {
        *counts
            .entry(candidate.frame.session_id_sha256.as_str())
            .or_default() += 1;
    }
    let incoming_count = counts
        .get(incoming.frame.session_id_sha256.as_str())
        .copied()
        .unwrap_or(0);
    let max_count = counts.values().copied().max().unwrap_or(0);
    let replace = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            let count = counts
                .get(candidate.frame.session_id_sha256.as_str())
                .copied()
                .unwrap_or(0);
            count > incoming_count || (count == incoming_count && count == max_count)
        })
        .max_by(|(_, left), (_, right)| left.rank.cmp(&right.rank))
        .map(|(index, candidate)| (index, candidate.rank.as_str()));
    if let Some((index, rank)) = replace
        && (incoming_count < max_count || incoming.rank.as_str() < rank)
    {
        candidates[index] = incoming;
    }
}

fn collect_session_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if path.is_file() {
            if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                output.push(path);
            }
            continue;
        }
        for entry in fs::read_dir(&path)
            .map_err(|error| format!("parity_backfill_read_dir:{}:{error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("parity_backfill_dir_entry:{error}"))?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                output.push(path);
            }
        }
    }
    Ok(())
}

fn persist_receipt(path: &Path, receipt: &ParityBackfillReceipt) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("parity_backfill_receipt_dir:{error}"))?;
    }
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("parity_backfill_receipt_encode:{error}"))?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("parity_backfill_receipt_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("parity_backfill_receipt_rename:{error}"))
}

fn usage() -> String {
    "usage: nando-response-parity-backfill <sessions-root> <miner-report> <miner-checkpoint> <receipt> [max-file-bytes<=67108864] [cases-per-teacher<=64]".to_owned()
}
