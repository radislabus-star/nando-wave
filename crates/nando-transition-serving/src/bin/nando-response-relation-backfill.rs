use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nando_operator_learning::teacher_action_symbol;
use nando_response_actor::{
    OnlineResponseMinerReport, OnlineResponseStream, OnlineResponseTailConfig,
};
use nando_transition_serving::{
    verified_training_cases_from_session, verified_training_cases_from_session_tail,
};

fn main() -> Result<(), String> {
    let mut arguments = std::env::args_os().skip(1);
    let live_ledger = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let report = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let checkpoint = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let root = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "sessions root missing".to_owned())?;
    let max_file_bytes = arguments
        .next()
        .and_then(|value| value.to_string_lossy().parse::<u64>().ok())
        .unwrap_or(64 * 1024 * 1024);

    let mut files = Vec::new();
    collect_session_files(&root, &mut files)?;
    files.sort();
    let mut seen_frames = BTreeSet::new();
    let mut seen_events = BTreeSet::new();
    let mut cases = Vec::new();
    for path in &files {
        let file_bytes = fs::metadata(path).map_err(|error| error.to_string())?.len();
        let extracted = if file_bytes > max_file_bytes {
            verified_training_cases_from_session_tail(path, max_file_bytes)?
        } else {
            verified_training_cases_from_session(path)?
        };
        for (frame, parity) in extracted {
            if !seen_frames.insert(frame.frame_id_sha256.clone())
                || !seen_events.insert(frame.event_id_sha256.clone())
            {
                continue;
            }
            cases.push((frame, parity));
        }
    }
    cases.sort_by(|(left, _), (right, _)| {
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
    let mut parity_by_action = BTreeMap::<String, usize>::new();
    for (frame, parity) in &cases {
        if parity.is_some() {
            *parity_by_action
                .entry(teacher_action_symbol(frame))
                .or_default() += 1;
        }
    }
    let parity_cases_total = parity_by_action.values().sum::<usize>();
    let parity_by_action_json =
        serde_json::to_string(&parity_by_action).map_err(|error| error.to_string())?;

    let emitted = cases.len();
    let mut miner = OnlineResponseStream::open_streaming(OnlineResponseTailConfig {
        input_path: live_ledger,
        report_path: report,
        checkpoint_path: checkpoint,
        idle_sleep: Duration::from_millis(50),
    })?;
    let future_frames_before = frozen_future_rows(&miner.report());
    let result = miner.train_replay_cases_batch(cases)?;
    let report = miner.report();
    let future_frames_after = frozen_future_rows(&report);
    let frozen_future_rows_claimed = future_frames_after.saturating_sub(future_frames_before);
    if frozen_future_rows_claimed != 0 {
        return Err("relation_backfill_claimed_frozen_future".to_owned());
    }
    println!(
        "{{\"files_scanned\":{},\"chronological_frames\":{},\"parity_cases_total\":{},\"parity_by_action\":{},\"support_only\":true,\"frozen_future_rows_before\":{},\"frozen_future_rows_after\":{},\"frozen_future_rows_claimed\":0,\"rows_learned\":{},\"buckets\":{},\"raw_text_persisted\":false}}",
        files.len(),
        emitted,
        parity_cases_total,
        parity_by_action_json,
        future_frames_before,
        future_frames_after,
        result.rows_learned,
        result.bucket_count
    );
    Ok(())
}

fn frozen_future_rows(report: &OnlineResponseMinerReport) -> usize {
    report
        .buckets
        .iter()
        .map(|bucket| bucket.frozen_future_rows)
        .sum::<usize>()
        .saturating_add(
            report
                .self_training_v2
                .generations
                .iter()
                .map(|generation| generation.future_rows)
                .sum::<usize>(),
        )
}

fn usage() -> String {
    "usage: nando-response-relation-backfill <live-ledger> <report> <checkpoint> <sessions-root> [max-file-bytes]".to_owned()
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
        for entry in fs::read_dir(&path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_dir() {
                pending.push(path);
            } else if (file_type.is_file() || (file_type.is_symlink() && path.is_file()))
                && path.extension().and_then(|value| value.to_str()) == Some("jsonl")
            {
                output.push(path);
            }
        }
    }
    Ok(())
}
