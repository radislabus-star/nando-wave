use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nando_response_actor::{
    OnlineResponseStream, OnlineResponseTailConfig, build_online_admission_snapshot,
};
use nando_transition_serving::{
    verified_relation_frames_from_session, verified_relation_frames_from_session_tail,
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
    let mut frames = Vec::new();
    for path in &files {
        let file_bytes = fs::metadata(path).map_err(|error| error.to_string())?.len();
        let extracted = if file_bytes > max_file_bytes {
            verified_relation_frames_from_session_tail(path, max_file_bytes)?
        } else {
            verified_relation_frames_from_session(path)?
        };
        for frame in extracted {
            if !seen_frames.insert(frame.frame_id_sha256.clone())
                || !seen_events.insert(frame.event_id_sha256.clone())
            {
                continue;
            }
            frames.push(frame);
        }
    }
    frames.sort_by(|left, right| {
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

    let emitted = frames.len();
    let mut miner = OnlineResponseStream::open(OnlineResponseTailConfig {
        input_path: live_ledger,
        report_path: report,
        checkpoint_path: checkpoint,
        idle_sleep: Duration::from_millis(50),
    })?;
    let result = miner.ingest_batch(frames)?;
    let admission_candidates = miner.admission_candidates();
    let snapshot_present = build_online_admission_snapshot(
        &admission_candidates,
        "nando-wave",
        1,
        1,
        30,
        &"0".repeat(64),
        &"0".repeat(64),
    )
    .map_err(str::to_owned)?
    .is_some();
    let report = miner.report();
    let future_frames = report
        .buckets
        .iter()
        .map(|bucket| bucket.frozen_future_rows)
        .sum::<usize>();
    println!(
        "{{\"files_scanned\":{},\"chronological_frames\":{},\"frozen_future_frames\":{},\"rows_learned\":{},\"buckets\":{},\"admission_candidates\":{},\"snapshot_present\":{},\"raw_text_persisted\":false}}",
        files.len(),
        emitted,
        future_frames,
        result.rows_learned,
        result.bucket_count,
        admission_candidates.len(),
        snapshot_present
    );
    Ok(())
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
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("jsonl")
            {
                output.push(path);
            }
        }
    }
    Ok(())
}
