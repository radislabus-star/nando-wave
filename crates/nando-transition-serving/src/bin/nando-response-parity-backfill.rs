use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nando_operator_kernel::RelationFrame;
use nando_operator_learning::{
    RuntimeParityCase, teacher_action_symbol, teacher_program_signature,
};
use nando_response_actor::{OnlineResponseStream, OnlineResponseTailConfig};
use nando_transition_serving::{
    verified_capture_bound_training_cases_from_sessions, verified_training_cases_from_session_head,
    verified_training_cases_from_session_tail,
    verified_write_stdin_training_cases_from_session_for_signatures,
};
use serde::Serialize;

const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_CASES_PER_TEACHER: usize = 64;
const DEFAULT_MAX_FILES: usize = 64;
const DEFAULT_TARGET_TEACHERS: usize = 4;
const MAX_CAPTURE_BOUND_FILES: usize = 8;
const MAX_CAPTURE_BOUND_SOURCE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BACKFILL_WORK_SLICES: usize = 256;

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
    capture_bound_files_planned: usize,
    capture_bound_files_scanned: usize,
    capture_bound_source_bytes_planned: u64,
    capture_bound_cases_seen: usize,
    capture_bound_selected_cases: usize,
    capture_bound_censored_session_identities: BTreeMap<String, String>,
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
    capture_bound_selected_by_action: BTreeMap<String, usize>,
    work_slices: usize,
    exact_checks: usize,
    work_budget_exhausted: bool,
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
    // This tool schedules high-value replay work only. Proof readiness belongs
    // to adaptive identification and must never be inferred from a row count.
    let transferred_actions = baseline
        .live_scalar_shadow
        .laws
        .iter()
        .filter(|law| law.future_rows > 0)
        .map(|law| law.teacher_action_symbol.as_str())
        .collect::<BTreeSet<_>>();
    let mut target_rank = baseline
        .self_training_v2
        .discovery
        .teacher_pools
        .iter()
        .filter(|pool| !transferred_actions.contains(pool.action_symbol.as_str()))
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
    let mut custom_candidate_paths = BTreeMap::<PathBuf, BTreeSet<String>>::new();
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
                custom_candidate_paths
                    .entry(path.clone())
                    .or_default()
                    .extend(
                        custom_cases
                            .iter()
                            .filter_map(|(frame, _)| teacher_program_signature(frame))
                            .filter(|signature| target_signatures.contains(signature)),
                    );
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
    let mut capture_bound_cases_seen = 0_usize;
    let mut capture_bound_files_scanned = 0_usize;
    let mut capture_bound_censored_session_identities = BTreeMap::new();
    let capture_bound_paths = select_capture_bound_paths(&custom_candidate_paths)?;
    let capture_bound_source_bytes_planned =
        capture_bound_paths.iter().try_fold(0_u64, |total, path| {
            fs::metadata(path)
                .map(|metadata| total.saturating_add(metadata.len()))
                .map_err(|error| format!("capture_bound_metadata:{}:{error}", path.display()))
        })?;
    if !scan_only && custom_full_scan && !custom_candidate_paths.is_empty() {
        let evidence_root = std::env::var_os("NANDO_STREAMING_EVIDENCE_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| "NANDO_STREAMING_EVIDENCE_DIR is required for apply".to_owned())?;
        let capture_bound = verified_capture_bound_training_cases_from_sessions(
            &capture_bound_paths,
            &evidence_root,
        )?;
        capture_bound_files_scanned = capture_bound.files_scanned;
        capture_bound_cases_seen = capture_bound.cases.len();
        capture_bound_censored_session_identities = capture_bound.censored_session_identities;
        let bound_signatures = capture_bound
            .cases
            .iter()
            .filter_map(|(frame, _)| teacher_program_signature(frame))
            .filter(|signature| target_signatures.contains(signature))
            .collect::<BTreeSet<_>>();
        for signature in &bound_signatures {
            // A target law must not mix durable capture receipts with the
            // earlier parity-only replay candidates.
            pools.remove(signature);
        }
        for (frame, parity) in capture_bound.cases {
            if frame.verifier_label != Some(true) {
                continue;
            }
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
    let mut capture_bound_selected_by_action = BTreeMap::<String, usize>::new();
    let mut capture_bound_selected_cases = 0_usize;
    for candidate in &selected {
        let action = teacher_action_symbol(&candidate.frame);
        *selected_by_action.entry(action.clone()).or_default() += 1;
        if candidate
            .parity
            .capture_receipt
            .as_ref()
            .and_then(|receipt| receipt.transition_binding.as_ref())
            .is_some()
        {
            capture_bound_selected_cases = capture_bound_selected_cases.saturating_add(1);
            *capture_bound_selected_by_action.entry(action).or_default() += 1;
        }
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
    let mut work_budget_exhausted = false;
    while !scan_only && !selected_signatures.is_empty() {
        let checks = stream.run_self_training_work_slice_for_signatures(&selected_signatures);
        exact_checks = exact_checks.saturating_add(checks);
        work_slices = work_slices.saturating_add(1);
        if checks == 0 && !stream.has_self_training_work_for_signatures(&selected_signatures) {
            break;
        }
        if work_slices >= MAX_BACKFILL_WORK_SLICES {
            work_budget_exhausted =
                stream.has_self_training_work_for_signatures(&selected_signatures);
            break;
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
        schema: "nando.response-parity-backfill.v2",
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
        capture_bound_files_planned: capture_bound_paths.len(),
        capture_bound_files_scanned,
        capture_bound_source_bytes_planned,
        capture_bound_cases_seen,
        capture_bound_selected_cases,
        capture_bound_censored_session_identities,
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
        capture_bound_selected_by_action,
        work_slices,
        exact_checks,
        work_budget_exhausted,
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

fn select_capture_bound_paths(
    candidates: &BTreeMap<PathBuf, BTreeSet<String>>,
) -> Result<Vec<PathBuf>, String> {
    let mut ranked = candidates
        .iter()
        .map(|(path, signatures)| {
            let bytes = fs::metadata(path)
                .map_err(|error| format!("capture_bound_metadata:{}:{error}", path.display()))?
                .len();
            Ok((bytes, path.clone(), signatures.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut uncovered = candidates
        .values()
        .flat_map(|signatures| signatures.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    let mut selected_bytes = 0_u64;
    for (bytes, path, signatures) in &ranked {
        if signatures.is_disjoint(&uncovered) {
            continue;
        }
        selected.push(path.clone());
        selected_bytes = selected_bytes.saturating_add(*bytes);
        uncovered.retain(|signature| !signatures.contains(signature));
        if uncovered.is_empty() || selected.len() >= MAX_CAPTURE_BOUND_FILES {
            break;
        }
    }
    if !uncovered.is_empty() {
        return Err("capture_bound_signature_coverage_incomplete".to_owned());
    }
    for (bytes, path, _) in ranked {
        if selected.len() >= MAX_CAPTURE_BOUND_FILES {
            break;
        }
        if selected.iter().any(|selected| selected == &path) {
            continue;
        }
        if selected_bytes.saturating_add(bytes) > MAX_CAPTURE_BOUND_SOURCE_BYTES {
            continue;
        }
        selected.push(path);
        selected_bytes = selected_bytes.saturating_add(bytes);
    }
    if selected_bytes > MAX_CAPTURE_BOUND_SOURCE_BYTES && selected.len() > 1 {
        return Err("capture_bound_source_budget_exhausted".to_owned());
    }
    Ok(selected)
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
    "usage: nando-response-parity-backfill <sessions-root> <relation-frames> <miner-report> <miner-checkpoint> <receipt> [max-file-bytes<=67108864] [cases-per-teacher<=64] [max-files<=1024] [target-teachers<=64] [scan-only] [custom-full-scan]; apply requires NANDO_STREAMING_EVIDENCE_DIR".to_owned()
}
