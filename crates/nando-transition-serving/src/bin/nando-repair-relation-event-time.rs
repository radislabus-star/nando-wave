use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use nando_operator_kernel::{RelationFrame, sha256_bytes};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let input = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let output = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let sessions = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut frames = Vec::new();
    let mut targets = BTreeSet::new();
    for line in BufReader::new(File::open(&input).map_err(|error| error.to_string())?).lines() {
        let line = line.map_err(|error| error.to_string())?;
        let frame = serde_json::from_str::<RelationFrame>(&line)
            .map_err(|error| format!("relation_frame_parse:{error}"))?;
        if frame.observed_at_unix_nanos == 0 {
            targets.insert(frame.event_id_sha256.clone());
        }
        frames.push(frame);
    }

    let mut files = Vec::new();
    collect_jsonl(&sessions, &mut files)?;
    files.sort();
    let mut repaired = BTreeMap::<String, u64>::new();
    let mut lines_read = 0_u64;
    let mut candidate_lines = 0_u64;
    'files: for path in &files {
        let reader = BufReader::with_capacity(
            1024 * 1024,
            File::open(path).map_err(|error| format!("session_open:{path:?}:{error}"))?,
        );
        for line in reader.lines() {
            let line = line.map_err(|error| format!("session_read:{path:?}:{error}"))?;
            lines_read = lines_read.saturating_add(1);
            if !(line.contains("function_call")
                || line.contains("custom_tool_call")
                || line.contains("final_answer"))
            {
                continue;
            }
            candidate_lines = candidate_lines.saturating_add(1);
            let Ok(row) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let event_id = sha256_bytes(serde_json::to_vec(&row).unwrap_or_default().as_slice());
            if !targets.contains(&event_id) {
                continue;
            }
            let Some(timestamp) = row
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
                .and_then(|value| u64::try_from(value.unix_timestamp_nanos()).ok())
            else {
                continue;
            };
            repaired.insert(event_id, timestamp);
            if repaired.len() == targets.len() {
                break 'files;
            }
        }
    }

    let mut repaired_rows = 0_usize;
    for frame in &mut frames {
        if frame.observed_at_unix_nanos == 0
            && let Some(timestamp) = repaired.get(&frame.event_id_sha256)
        {
            frame.observed_at_unix_nanos = *timestamp;
            repaired_rows = repaired_rows.saturating_add(1);
        }
    }
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.session_id_sha256.cmp(&right.session_id_sha256))
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    let parent = output
        .parent()
        .ok_or_else(|| "output_parent_missing".to_owned())?;
    let temporary = parent.join(format!(".event-time-repair.{}.tmp", std::process::id()));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    for frame in &frames {
        serde_json::to_writer(&mut writer, frame).map_err(|error| error.to_string())?;
        writer.write_all(b"\n").map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| error.to_string())?;
    fs::rename(&temporary, &output).map_err(|error| error.to_string())?;

    println!(
        "{}",
        serde_json::json!({
            "schema":"nando.relation-event-time-repair.v1",
            "frames":frames.len(),
            "zero_time_targets":targets.len(),
            "matched_events":repaired.len(),
            "repaired_rows":repaired_rows,
            "remaining_zero_time_rows":targets.len().saturating_sub(repaired.len()),
            "session_files":files.len(),
            "session_lines_read":lines_read,
            "candidate_lines_parsed":candidate_lines,
            "raw_text_persisted":false
        })
    );
    Ok(())
}

fn collect_jsonl(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
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

fn usage() -> String {
    "usage: nando-repair-relation-event-time INPUT OUTPUT SESSIONS_ROOT".to_owned()
}
