use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nando_response_actor::{
    OnlineResponseStream, OnlineResponseTailConfig, RelationAtom, RelationFrame,
    build_online_admission_snapshot, relation_frame_online_routing_atom_ids,
};

#[derive(Default)]
struct SplitStats {
    positive_rows: u64,
    positive_tokens: u64,
    negative_rows: u64,
    negative_tokens: u64,
}

struct PositiveSample {
    atom_ids: Vec<u64>,
    tokens: u64,
    session_id_sha256: String,
}

pub(super) fn main() -> Result<(), String> {
    let input = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| {
            "usage: nando-online-response-diagnose <relation-frames.jsonl>".to_owned()
        })?;
    if std::env::var_os("NANDO_DIAGNOSE_WAIT_SPLITS").is_some() {
        return diagnose_wait_splits(&input);
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nando-online-diagnose-{nonce}"));
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let report = root.join("report.json");
    let checkpoint = root.join("checkpoint.cbor");
    let checkpoint_supplied = if let Some(source_checkpoint) = std::env::args_os().nth(2) {
        std::fs::copy(source_checkpoint, &checkpoint).map_err(|error| error.to_string())?;
        true
    } else {
        false
    };
    let config = OnlineResponseTailConfig {
        input_path: input,
        report_path: report.clone(),
        checkpoint_path: checkpoint,
        idle_sleep: Duration::from_millis(10),
    };
    let mut result = if checkpoint_supplied {
        OnlineResponseStream::open_streaming(config)
    } else {
        OnlineResponseStream::open(config)
    };
    let diagnostic_signatures = std::env::var("NANDO_DIAGNOSE_SIGNATURES")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|signature| !signature.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .filter(|signatures| !signatures.is_empty());
    if let Ok(miner) = &mut result
        && std::env::var_os("NANDO_DIAGNOSE_DRAIN_WORK").is_some()
    {
        // Targeted mode audits one semantic law without replaying an unrelated
        // global synthesis backlog from the copied production checkpoint.
        for slice in 0..8_192 {
            let pending = diagnostic_signatures.as_ref().map_or_else(
                || miner.has_self_training_work(),
                |signatures| miner.has_self_training_work_for_signatures(signatures),
            );
            if !pending {
                break;
            }
            let checks = match &diagnostic_signatures {
                Some(signatures) => miner.run_self_training_work_slice_for_signatures(signatures),
                None => miner.run_self_training_work_slice(),
            };
            if slice == 0 || (slice + 1) % 128 == 0 {
                eprintln!(
                    "diagnose_work slices={} checks={} targeted_signatures={}",
                    slice + 1,
                    checks,
                    diagnostic_signatures.as_ref().map_or(0, BTreeSet::len),
                );
            }
        }
        let pending = diagnostic_signatures.as_ref().map_or_else(
            || miner.has_self_training_work(),
            |signatures| miner.has_self_training_work_for_signatures(signatures),
        );
        if pending {
            return Err(format!(
                "diagnose_self_training_work_exhausted:targeted_signatures={}",
                diagnostic_signatures.as_ref().map_or(0, BTreeSet::len),
            ));
        }
        miner.persist_now()?;
    }
    if let Ok(miner) = &result {
        if let Some(output_path) = std::env::var_os("NANDO_DIAGNOSE_EVIDENCE_AUDIT") {
            let signatures = diagnostic_signatures.as_ref().ok_or_else(|| {
                "NANDO_DIAGNOSE_EVIDENCE_AUDIT requires NANDO_DIAGNOSE_SIGNATURES".to_owned()
            })?;
            let audit = miner.semantic_law_evidence_audit(signatures);
            std::fs::write(
                output_path,
                serde_json::to_vec_pretty(&audit).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
        if let Some(output_path) = std::env::var_os("NANDO_DIAGNOSE_BINDING_EVIDENCE") {
            let signatures = diagnostic_signatures.as_ref().ok_or_else(|| {
                "NANDO_DIAGNOSE_BINDING_EVIDENCE requires NANDO_DIAGNOSE_SIGNATURES".to_owned()
            })?;
            let report = miner.semantic_law_binding_evidence_report(signatures)?;
            std::fs::write(
                output_path,
                serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
        if std::env::var_os("NANDO_DIAGNOSE_ADMISSION").is_some() {
            let candidates = miner.admission_candidates();
            for candidate in &candidates {
                eprintln!(
                    "online_subcenter bucket={} program={} required={:?} support={} future={} tokens={}",
                    candidate.candidate.bucket_id,
                    serde_json::to_string(&candidate.candidate.program.operation)
                        .unwrap_or_else(|_| "null".to_owned()),
                    candidate.required_routing_atom_ids,
                    candidate.support.len(),
                    candidate.future.len(),
                    candidate.candidate.positive_tokens,
                );
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let snapshot = build_online_admission_snapshot(
                &candidates,
                "nando-wave",
                1,
                now,
                30,
                &"a".repeat(64),
                &"b".repeat(64),
            )?;
            eprintln!(
                "online_admission candidates={} packages={}",
                candidates.len(),
                snapshot
                    .as_ref()
                    .map_or(0, |value| value.registry.packages.len())
            );
        }
        let output = std::fs::read_to_string(&report).map_err(|error| error.to_string())?;
        println!("{output}");
    }
    let _ = std::fs::remove_dir_all(root);
    result.map(|_| ())
}

fn diagnose_wait_splits(input: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(input).map_err(|error| error.to_string())?;
    let mut stats = BTreeMap::<u64, SplitStats>::new();
    let mut positives = Vec::<PositiveSample>::new();
    let mut positive_atom_frames = BTreeMap::<u64, Vec<usize>>::new();
    let mut negative_atom_frames = BTreeMap::<u64, Vec<usize>>::new();
    let mut positive_rows = 0_u64;
    let mut negative_rows = 0_u64;
    let mut routing_atoms_total = 0_u64;
    let mut routing_atoms_max = 0_usize;
    for line in BufReader::new(file).lines() {
        let frame =
            serde_json::from_str::<RelationFrame>(&line.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
        if frame.verifier_label != Some(true) {
            continue;
        }
        let wait = frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionFunction { value } if value == "wait"));
        let has_teacher = frame.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionCustomTool { .. }
                    | RelationAtom::ActionValueProjection { .. }
                    | RelationAtom::ActionStatusProjection { .. }
            )
        });
        if !has_teacher {
            continue;
        }
        let atom_ids = relation_frame_online_routing_atom_ids(&frame);
        routing_atoms_total = routing_atoms_total.saturating_add(atom_ids.len() as u64);
        routing_atoms_max = routing_atoms_max.max(atom_ids.len());
        if wait {
            positive_rows = positive_rows.saturating_add(1);
            let frame_index = positives.len();
            for atom in &atom_ids {
                positive_atom_frames
                    .entry(*atom)
                    .or_default()
                    .push(frame_index);
            }
            positives.push(PositiveSample {
                atom_ids: atom_ids.clone(),
                tokens: frame.estimated_input_tokens,
                session_id_sha256: frame.session_id_sha256.clone(),
            });
        } else {
            let frame_index = usize::try_from(negative_rows).unwrap_or(usize::MAX);
            for atom in &atom_ids {
                negative_atom_frames
                    .entry(*atom)
                    .or_default()
                    .push(frame_index);
            }
            negative_rows = negative_rows.saturating_add(1);
        }
        for atom in atom_ids {
            let entry = stats.entry(atom).or_default();
            if wait {
                entry.positive_rows = entry.positive_rows.saturating_add(1);
                entry.positive_tokens = entry
                    .positive_tokens
                    .saturating_add(frame.estimated_input_tokens);
            } else {
                entry.negative_rows = entry.negative_rows.saturating_add(1);
                entry.negative_tokens = entry
                    .negative_tokens
                    .saturating_add(frame.estimated_input_tokens);
            }
        }
    }
    let mut pair_counts = BTreeMap::<(u64, u64), usize>::new();
    for sample in &positives {
        let mut atoms = sample
            .atom_ids
            .iter()
            .copied()
            .filter(|atom| {
                stats
                    .get(atom)
                    .is_some_and(|value| value.positive_rows >= 32)
            })
            .collect::<Vec<_>>();
        if std::env::var_os("NANDO_DIAGNOSE_STATIC_PAIRS").is_some() {
            atoms.sort_unstable();
            atoms.truncate(16);
        } else {
            atoms.sort_by_key(|atom| (negative_atom_frames.get(atom).map_or(0, Vec::len), *atom));
            atoms.truncate(8);
        }
        for left in 0..atoms.len() {
            for right in left + 1..atoms.len() {
                let pair = if atoms[left] < atoms[right] {
                    (atoms[left], atoms[right])
                } else {
                    (atoms[right], atoms[left])
                };
                *pair_counts.entry(pair).or_default() += 1;
            }
        }
    }
    let mut clean_pairs = Vec::<((u64, u64), Vec<usize>)>::new();
    for (pair, rows) in pair_counts {
        if rows < 32
            || sorted_intersects(
                negative_atom_frames.get(&pair.0).map_or(&[], Vec::as_slice),
                negative_atom_frames.get(&pair.1).map_or(&[], Vec::as_slice),
            )
        {
            continue;
        }
        let positive_frames = sorted_intersection(
            positive_atom_frames.get(&pair.0).map_or(&[], Vec::as_slice),
            positive_atom_frames.get(&pair.1).map_or(&[], Vec::as_slice),
        );
        if positive_frames.len() >= 32 {
            clean_pairs.push((pair, positive_frames));
        }
    }
    let mut clean = stats
        .into_iter()
        .filter(|(_, value)| value.positive_rows >= 32 && value.negative_rows == 0)
        .collect::<Vec<_>>();
    clean.sort_by(|left, right| {
        right
            .1
            .positive_tokens
            .cmp(&left.1.positive_tokens)
            .then_with(|| right.1.positive_rows.cmp(&left.1.positive_rows))
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut candidates = clean
        .iter()
        .filter_map(|(atom, _)| {
            positive_atom_frames
                .get(atom)
                .cloned()
                .map(|frames| (format!("{atom}"), frames))
        })
        .collect::<Vec<_>>();
    candidates.extend(
        clean_pairs
            .iter()
            .map(|(pair, frames)| (format!("{}+{}", pair.0, pair.1), frames.clone())),
    );
    let mut covered = vec![false; positives.len()];
    let mut selected = BTreeSet::<String>::new();
    for _ in 0..64 {
        let next = candidates
            .iter()
            .filter(|(key, _)| !selected.contains(key))
            .filter_map(|(key, frames)| {
                let marginal_tokens = frames
                    .iter()
                    .filter(|index| !covered[**index])
                    .map(|index| positives[*index].tokens)
                    .sum::<u64>();
                (marginal_tokens > 0).then_some((key.clone(), frames, marginal_tokens))
            })
            .max_by(|left, right| left.2.cmp(&right.2).then_with(|| right.0.cmp(&left.0)));
        let Some((key, frames, marginal_tokens)) = next else {
            break;
        };
        selected.insert(key.clone());
        for index in frames {
            covered[*index] = true;
        }
        let covered_rows = covered.iter().filter(|value| **value).count();
        let covered_tokens = positives
            .iter()
            .zip(&covered)
            .filter(|(_, covered)| **covered)
            .map(|(sample, _)| sample.tokens)
            .sum::<u64>();
        let covered_sessions = positives
            .iter()
            .zip(&covered)
            .filter(|(_, covered)| **covered)
            .map(|(sample, _)| sample.session_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let step = serde_json::json!({
            "rank": selected.len(),
            "subcenter": key,
            "marginal_tokens": marginal_tokens,
            "covered_rows": covered_rows,
            "covered_tokens": covered_tokens,
            "covered_sessions": covered_sessions,
        });
        if selected.len() == 1
            || selected.len() == 4
            || selected.len() == 16
            || selected.len() == 64
        {
            eprintln!("greedy_checkpoint={step}");
        }
    }
    let covered_rows = covered.iter().filter(|value| **value).count();
    let covered_tokens = positives
        .iter()
        .zip(&covered)
        .filter(|(_, covered)| **covered)
        .map(|(sample, _)| sample.tokens)
        .sum::<u64>();
    let covered_sessions = positives
        .iter()
        .zip(&covered)
        .filter(|(_, covered)| **covered)
        .map(|(sample, _)| sample.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "schema": "nando.wait-preaction-split-diagnostic.v1",
            "positive_rows": positive_rows,
            "negative_rows": negative_rows,
            "routing_atoms_average": routing_atoms_total as f64 / (positive_rows + negative_rows).max(1) as f64,
            "routing_atoms_max": routing_atoms_max,
            "clean_atom_count": clean.len(),
            "clean_pair_count": clean_pairs.len(),
            "greedy_subcenters": selected.len(),
            "greedy_covered_rows": covered_rows,
            "greedy_covered_tokens": covered_tokens,
            "greedy_covered_sessions": covered_sessions,
            "top_clean_atoms": clean.into_iter().take(32).map(|(atom_id, value)| serde_json::json!({
                "atom_id": atom_id,
                "positive_rows": value.positive_rows,
                "positive_tokens": value.positive_tokens,
                "negative_rows": value.negative_rows,
            })).collect::<Vec<_>>(),
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn sorted_intersects(left: &[usize], right: &[usize]) -> bool {
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

fn sorted_intersection(left: &[usize], right: &[usize]) -> Vec<usize> {
    let mut output = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                output.push(left[left_index]);
                left_index += 1;
                right_index += 1;
            }
        }
    }
    output
}
