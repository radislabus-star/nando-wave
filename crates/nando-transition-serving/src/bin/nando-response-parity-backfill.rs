use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nando_response_actor::{
    OnlineResponseStream, OnlineResponseTailConfig, RelationFrame, RuntimeParityCase,
    teacher_action_symbol, teacher_program_signature,
};
use nando_transition_serving::{
    verified_training_cases_from_session_head, verified_training_cases_from_session_tail,
    verified_write_stdin_training_cases_from_session_for_signatures,
};
use serde::Serialize;

const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_CASES_PER_TEACHER: usize = 64;
const DEFAULT_MAX_FILES: usize = 64;
const DEFAULT_TARGET_TEACHERS: usize = 4;
const REQUIRED_SUPPORT_ROWS: usize = 32;

struct ParityCandidate {
    rank: String,
    frame: RelationFrame,
    parity: RuntimeParityCase,
}

#[derive(Serialize)]
struct ParityBackfillReceipt {
    schema: &'static str,
    sessions_root: String,
    relation_frames_path: String,
    files_available: usize,
    files_scanned: usize,
    max_files: usize,
    source_bytes_considered: u64,
    max_file_bytes: u64,
    cases_per_teacher_cap: usize,
    target_teacher_cap: usize,
    scan_only: bool,
    custom_full_scan: bool,
    custom_files_scanned: usize,
    custom_prefilter_bytes: u64,
    custom_sparse_bytes: u64,
    target_teachers: usize,
    frames_seen: u64,
    parity_cases_seen: u64,
    parity_seen_by_action: BTreeMap<String, usize>,
    verified_parity_seen_by_action: BTreeMap<String, usize>,
    selected_cases: usize,
    imported_rows: usize,
    selected_by_action: BTreeMap<String, usize>,
    selected_by_signature: BTreeMap<String, usize>,
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
    let relation_frames_path = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
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
    let max_files = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_FILES)
        .clamp(1, 1_024);
    let target_teacher_cap = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
        .unwrap_or(DEFAULT_TARGET_TEACHERS)
        .clamp(1, 64);
    let mut scan_only = false;
    let mut custom_full_scan = false;
    for value in arguments {
        match value.to_string_lossy().as_ref() {
            "scan-only" => scan_only = true,
            "custom-full-scan" => custom_full_scan = true,
            _ => return Err(usage()),
        }
    }

    let checkpoint_open_started = Instant::now();
    let mut stream = OnlineResponseStream::open_streaming(OnlineResponseTailConfig {
        input_path: relation_frames_path.clone(),
        report_path: miner_report_path,
        checkpoint_path: checkpoint_path.clone(),
        idle_sleep: Duration::from_millis(200),
    })?;
    let checkpoint_open_millis = checkpoint_open_started.elapsed().as_millis();
    let baseline = stream.report();
    let rows_before = baseline.rows_seen;
    let mut matching_parity = BTreeMap::<String, usize>::new();
    for generation in &baseline.self_training_v2.generations {
        matching_parity
            .entry(generation.teacher_signature_sha256.clone())
            .and_modify(|rows| *rows = (*rows).max(generation.matching_runtime_parity_rows))
            .or_insert(generation.matching_runtime_parity_rows);
    }
    let mut target_rank = baseline
        .self_training_v2
        .discovery
        .teacher_pools
        .iter()
        .filter(|pool| {
            matching_parity
                .get(&pool.teacher_signature_sha256)
                .copied()
                .unwrap_or(0)
                < REQUIRED_SUPPORT_ROWS
        })
        .map(|pool| (pool.positive_tokens, pool.teacher_signature_sha256.clone()))
        .collect::<Vec<_>>();
    target_rank.sort_by(|left, right| right.cmp(left));
    let target_signatures = target_rank
        .into_iter()
        .take(target_teacher_cap)
        .map(|(_, signature)| signature)
        .collect::<BTreeSet<_>>();
    eprintln!(
        "nando-parity-backfill stage=checkpoint_open millis={checkpoint_open_millis} targets={}",
        target_signatures.len()
    );

    let mut files = Vec::new();
    collect_session_files(&sessions_root, &mut files)?;
    files.sort_by(|left, right| {
        let left_modified = fs::metadata(left).and_then(|value| value.modified()).ok();
        let right_modified = fs::metadata(right).and_then(|value| value.modified()).ok();
        right_modified
            .cmp(&left_modified)
            .then_with(|| right.cmp(left))
    });
    let files_available = files.len();
    files.truncate(max_files);
    let scan_started = Instant::now();
    let mut source_bytes_considered = 0_u64;
    let mut frames_seen = 0_u64;
    let mut parity_cases_seen = 0_u64;
    let mut parity_seen_by_action = BTreeMap::<String, usize>::new();
    let mut verified_parity_seen_by_action = BTreeMap::<String, usize>::new();
    let mut custom_files_scanned = 0_usize;
    let mut custom_prefilter_bytes = 0_u64;
    let mut custom_sparse_bytes = 0_u64;
    let mut seen_frames = BTreeSet::new();
    let mut pools = BTreeMap::<String, Vec<ParityCandidate>>::new();
    for path in &files {
        let length = fs::metadata(path)
            .map_err(|error| format!("parity_backfill_metadata:{}:{error}", path.display()))?
            .len();
        let bounded_bytes = length.min(max_file_bytes);
        source_bytes_considered = source_bytes_considered.saturating_add(bounded_bytes);
        let mut cases = verified_training_cases_from_session_head(path, max_file_bytes)?;
        if length > max_file_bytes {
            source_bytes_considered = source_bytes_considered.saturating_add(bounded_bytes);
            cases.extend(verified_training_cases_from_session_tail(
                path,
                max_file_bytes,
            )?);
        }
        if custom_full_scan {
            custom_prefilter_bytes = custom_prefilter_bytes.saturating_add(length);
            // The checkpoint already identifies deficient teacher laws. Keep a full-file
            // scan bounded to those laws instead of retaining unrelated tool traffic.
            let custom_cases = verified_write_stdin_training_cases_from_session_for_signatures(
                path,
                &target_signatures,
            )?;
            if !custom_cases.is_empty() {
                custom_files_scanned = custom_files_scanned.saturating_add(1);
                custom_sparse_bytes = custom_sparse_bytes.saturating_add(length);
                cases.extend(custom_cases);
            }
        }
        for (frame, parity) in cases {
            if !seen_frames.insert(frame.frame_id_sha256.clone()) {
                continue;
            }
            frames_seen = frames_seen.saturating_add(1);
            let Some(parity) = parity else { continue };
            parity_cases_seen = parity_cases_seen.saturating_add(1);
            *parity_seen_by_action
                .entry(teacher_action_symbol(&frame))
                .or_default() += 1;
            if frame.verifier_label != Some(true) {
                continue;
            }
            *verified_parity_seen_by_action
                .entry(teacher_action_symbol(&frame))
                .or_default() += 1;
            let Some(signature) = teacher_program_signature(&frame) else {
                continue;
            };
            if !target_signatures.contains(&signature) {
                continue;
            }
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
    let scan_millis = scan_started.elapsed().as_millis();
    eprintln!(
        "nando-parity-backfill stage=scan millis={scan_millis} files={} frames={frames_seen} parity={parity_cases_seen}",
        files.len()
    );

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
    let mut selected_by_signature = BTreeMap::<String, usize>::new();
    for candidate in &selected {
        *selected_by_action
            .entry(teacher_action_symbol(&candidate.frame))
            .or_default() += 1;
        if let Some(signature) = teacher_program_signature(&candidate.frame) {
            *selected_by_signature.entry(signature).or_default() += 1;
        }
    }

    let selected_cases = selected.len();
    let selected_signatures = selected
        .iter()
        .filter_map(|candidate| teacher_program_signature(&candidate.frame))
        .collect::<BTreeSet<_>>();
    let apply_started = Instant::now();
    if !scan_only && !selected_signatures.is_empty() {
        stream.train_replay_cases_batch_buffered(
            selected
                .into_iter()
                .map(|candidate| (candidate.frame, Some(candidate.parity))),
        )?;
    }
    let apply_millis = apply_started.elapsed().as_millis();
    eprintln!(
        "nando-parity-backfill stage=apply millis={apply_millis} selected={selected_cases} signatures={}",
        selected_signatures.len()
    );
    let work_started = Instant::now();
    let mut work_slices = 0_usize;
    let mut exact_checks = 0_usize;
    while !scan_only && !selected_signatures.is_empty() {
        let checks = stream.run_self_training_work_slice_for_signatures(&selected_signatures);
        exact_checks = exact_checks.saturating_add(checks);
        work_slices = work_slices.saturating_add(1);
        if checks == 0 && !stream.has_self_training_work_for_signatures(&selected_signatures) {
            break;
        }
        if work_slices >= 4_096 {
            return Err("parity_backfill_work_budget_exhausted".to_owned());
        }
    }
    let work_millis = work_started.elapsed().as_millis();
    eprintln!(
        "nando-parity-backfill stage=work millis={work_millis} slices={work_slices} checks={exact_checks}"
    );
    let persist_millis = if scan_only || selected_signatures.is_empty() {
        0
    } else {
        let persist_started = Instant::now();
        stream.persist_now()?;
        persist_started.elapsed().as_millis()
    };
    eprintln!("nando-parity-backfill stage=persist millis={persist_millis}");
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
        relation_frames_path: relation_frames_path.display().to_string(),
        files_available,
        files_scanned: files.len(),
        max_files,
        source_bytes_considered,
        max_file_bytes,
        cases_per_teacher_cap: cases_per_teacher,
        target_teacher_cap,
        scan_only,
        custom_full_scan,
        custom_files_scanned,
        custom_prefilter_bytes,
        custom_sparse_bytes,
        target_teachers: target_signatures.len(),
        frames_seen,
        parity_cases_seen,
        parity_seen_by_action,
        verified_parity_seen_by_action,
        selected_cases,
        imported_rows,
        selected_by_action,
        selected_by_signature,
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
    "usage: nando-response-parity-backfill <sessions-root> <relation-frames> <miner-report> <miner-checkpoint> <receipt> [max-file-bytes<=67108864] [cases-per-teacher<=64] [max-files<=1024] [target-teachers<=64] [scan-only] [custom-full-scan]".to_owned()
}
