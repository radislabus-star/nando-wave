use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use nando_operator_kernel::{RelationFrame, canonical_json_sha256};
use nando_operator_learning::is_source_neutral_relation_frame;

fn main() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: nando-merge-relation-ledgers OUTPUT INPUT...".to_owned())?;
    let inputs = args.map(PathBuf::from).collect::<Vec<_>>();
    if inputs.is_empty() {
        return Err("relation_ledger_inputs_missing".to_owned());
    }

    let mut frames = BTreeMap::<String, (String, RelationFrame)>::new();
    let mut conflicting_ids = BTreeSet::new();
    let mut rows_read = 0_u64;
    let mut filtered = 0_u64;
    let mut duplicates = 0_u64;
    for path in &inputs {
        let file = File::open(path).map_err(|error| format!("input_open:{path:?}:{error}"))?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|error| format!("input_read:{path:?}:{error}"))?;
            rows_read = rows_read.saturating_add(1);
            let frame = match serde_json::from_str::<RelationFrame>(&line) {
                Ok(frame) if is_source_neutral_relation_frame(&frame) => frame,
                Ok(_) => {
                    filtered = filtered.saturating_add(1);
                    continue;
                }
                Err(error) => return Err(format!("frame_parse:{path:?}:{rows_read}:{error}")),
            };
            let digest = canonical_json_sha256(&frame).map_err(str::to_owned)?;
            if conflicting_ids.contains(&frame.frame_id_sha256) {
                continue;
            }
            match frames.get(&frame.frame_id_sha256) {
                Some((existing, _)) if existing == &digest => {
                    duplicates = duplicates.saturating_add(1);
                }
                Some(_) => {
                    frames.remove(&frame.frame_id_sha256);
                    conflicting_ids.insert(frame.frame_id_sha256);
                }
                None => {
                    frames.insert(frame.frame_id_sha256.clone(), (digest, frame));
                }
            }
        }
    }

    let mut ordered = frames
        .into_values()
        .map(|(_, frame)| frame)
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.session_id_sha256.cmp(&right.session_id_sha256))
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });

    let parent = output
        .parent()
        .ok_or_else(|| "output_parent_missing".to_owned())?;
    let temporary = parent.join(format!(".relation-merge.{}.tmp", std::process::id()));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("output_create:{error}"))?;
    let mut writer = BufWriter::new(file);
    for frame in &ordered {
        serde_json::to_writer(&mut writer, frame)
            .map_err(|error| format!("output_encode:{error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("output_write:{error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("output_flush:{error}"))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| format!("output_sync:{error}"))?;
    fs::rename(&temporary, &output).map_err(|error| format!("output_rename:{error}"))?;

    println!(
        "{}",
        serde_json::json!({
            "rows_read": rows_read,
            "source_neutral_rows": ordered.len(),
            "filtered_rows": filtered,
            "duplicates": duplicates,
            "conflicting_ids_excluded": conflicting_ids.len(),
            "output_bytes": fs::metadata(&output).map(|m| m.len()).unwrap_or(0),
            "raw_text_persisted": false,
        })
    );
    Ok(())
}
