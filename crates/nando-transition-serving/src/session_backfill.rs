use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nando_response_actor::{
    CanonicalEventGraph, CollectionSynthesisExample, DeterministicEvidenceGraphStore,
    DeterministicEvidenceLedger, EvidenceGraphBuilder, EvidenceGraphPolicy, EvidencePolicyV1,
    OnlineCollectionMiner, OnlineCollectionObservation, RawEvidenceEnvelope, canonical_json_bytes,
    canonicalize_evidence_envelope, sha256_bytes,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const BACKFILL_SCHEMA_V1: &str = "nando.session-evidence-backfill.v2";
const MAX_TURN_EVENTS: usize = 64;
const MAX_TURN_NODES: usize = 8_192;
const MAX_RETAINED_TURN_BYTES: usize = 2 * 1024 * 1024;
const OVERFLOW_LEDGER_BATCH_ROWS: usize = 256;
const MAX_LEDGER_BATCH_ROWS: usize = 8;
const MAX_LEDGER_BATCH_BYTES: usize = 256 * 1024;
const MAX_SOURCE_BYTES_PER_PASS: u64 = 16 * 1024 * 1024;
const MAX_SOURCE_TURNS_PER_PASS: u32 = 128;
const MAX_INITIAL_COLLECTION_TAIL_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct BackfillCheckpoint {
    schema: String,
    sources: BTreeMap<String, BackfillSource>,
    #[serde(default)]
    evidence_covered_sources: BTreeMap<String, BackfillSource>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct BackfillSource {
    offset: u64,
    turn_index: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CollectionMigrationCheckpoint {
    schema: String,
    sources: BTreeMap<String, BackfillSource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollectionMigrationReport {
    pub schema: String,
    pub elapsed_millis: u128,
    pub deadline_reached: bool,
    pub files_seen: u64,
    pub bytes_scanned: u64,
    pub turns_scanned: u64,
    pub observations_before: u64,
    pub observations_after: u64,
    pub supported_observations: u64,
    pub unsupported_observations: u64,
    pub bucket_count: usize,
    pub support_rows: usize,
    pub support_tokens: u64,
    pub frozen_buckets: usize,
}

#[derive(Clone)]
struct BackfillRow {
    offset: u64,
    bytes: Vec<u8>,
    value: Value,
}

pub fn spawn_session_backfill(
    root: PathBuf,
    checkpoint_path: PathBuf,
    evidence: Arc<Mutex<DeterministicEvidenceLedger>>,
    evidence_graphs: Arc<Mutex<DeterministicEvidenceGraphStore>>,
    collection_miner: Arc<Mutex<OnlineCollectionMiner>>,
) -> Result<(), String> {
    thread::Builder::new()
        .name("nando-session-backfill".to_owned())
        .spawn(move || {
            loop {
                if let Err(error) = run_backfill(
                    &root,
                    &checkpoint_path,
                    &evidence,
                    &evidence_graphs,
                    &collection_miner,
                ) {
                    eprintln!("nando-session-backfill error: {error}");
                    thread::sleep(Duration::from_secs(1));
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        })
        .map_err(|error| format!("session_backfill_thread:{error}"))?;
    Ok(())
}

/// Replays historical turns into the compact collection version space once.
/// The production daemon never calls this path; normal learning stays live-tail only.
pub fn run_collection_migration_pass(
    root: &Path,
    migration_checkpoint_path: &Path,
    collection_checkpoint_path: &Path,
    config: nando_response_actor::OnlineCollectionConfig,
    max_duration: Duration,
) -> Result<CollectionMigrationReport, String> {
    const SCHEMA: &str = "nando.collection-migration.v1";
    let started = Instant::now();
    let mut checkpoint = load_collection_migration_checkpoint(migration_checkpoint_path, SCHEMA)?;
    let mut miner = OnlineCollectionMiner::open(collection_checkpoint_path, config)?;
    let observations_before = miner.status().observations_total;
    let mut files_seen = 0_u64;
    let mut bytes_scanned = 0_u64;
    let mut turns_scanned = 0_u64;
    let mut deadline_reached = false;
    let mut paths = session_files(root);
    paths.sort_by(|left, right| {
        let left_modified = fs::metadata(left).and_then(|value| value.modified()).ok();
        let right_modified = fs::metadata(right).and_then(|value| value.modified()).ok();
        right_modified
            .cmp(&left_modified)
            .then_with(|| left.cmp(right))
    });

    'files: for path in paths {
        if started.elapsed() >= max_duration {
            deadline_reached = true;
            break;
        }
        files_seen = files_seen.saturating_add(1);
        let source_id = path.to_string_lossy().into_owned();
        let source_sha256 = sha256_bytes(source_id.as_bytes());
        let is_new_source = !checkpoint.sources.contains_key(&source_sha256);
        let mut source = checkpoint
            .sources
            .get(&source_sha256)
            .copied()
            .unwrap_or_default();
        let mut file = File::open(&path)
            .map_err(|error| format!("collection_migration_open:{}:{error}", path.display()))?;
        let length = file
            .metadata()
            .map_err(|error| format!("collection_migration_metadata:{error}"))?
            .len();
        if is_new_source {
            source.offset = length.saturating_sub(MAX_INITIAL_COLLECTION_TAIL_BYTES);
        } else if source.offset > length {
            source = BackfillSource::default();
        }
        file.seek(SeekFrom::Start(source.offset))
            .map_err(|error| format!("collection_migration_seek:{error}"))?;
        let start_offset = source.offset;
        let mut reader = BufReader::new(file);
        let mut rows = Vec::<BackfillRow>::new();
        let mut retained_bytes = 0_usize;
        let mut line = Vec::new();
        let mut waiting_for_initial_boundary = is_new_source && source.offset > 0;
        loop {
            line.clear();
            let position = reader
                .stream_position()
                .map_err(|error| format!("collection_migration_position:{error}"))?;
            let bytes = reader
                .read_until(b'\n', &mut line)
                .map_err(|error| format!("collection_migration_read:{error}"))?;
            if bytes == 0 || !line.ends_with(b"\n") {
                if !rows.is_empty() {
                    observe_collection_turn(&source_id, source.turn_index, &rows, &mut miner)?;
                    turns_scanned = turns_scanned.saturating_add(1);
                }
                source.offset = position;
                break;
            }
            let raw = line
                .strip_suffix(b"\n")
                .and_then(|value| value.strip_suffix(b"\r").or(Some(value)))
                .unwrap_or(&line);
            let Ok(value) = serde_json::from_slice::<Value>(raw) else {
                source.offset = position.saturating_add(bytes as u64);
                continue;
            };
            if waiting_for_initial_boundary {
                source.offset = position.saturating_add(bytes as u64);
                if !is_turn_boundary(&value) {
                    continue;
                }
                waiting_for_initial_boundary = false;
            }
            if is_turn_boundary(&value) && !rows.is_empty() {
                observe_collection_turn(&source_id, source.turn_index, &rows, &mut miner)?;
                turns_scanned = turns_scanned.saturating_add(1);
                rows.clear();
                retained_bytes = 0;
                source.offset = position;
                checkpoint.sources.insert(source_sha256.clone(), source);
                if started.elapsed() >= max_duration {
                    deadline_reached = true;
                    bytes_scanned =
                        bytes_scanned.saturating_add(source.offset.saturating_sub(start_offset));
                    miner.flush()?;
                    persist_collection_migration_checkpoint(
                        migration_checkpoint_path,
                        &checkpoint,
                    )?;
                    break 'files;
                }
            }
            if is_turn_boundary(&value) {
                source.turn_index = source.turn_index.saturating_add(1);
            }
            let row = BackfillRow {
                offset: position,
                bytes: raw.to_vec(),
                value,
            };
            retained_bytes = retained_bytes.saturating_add(row.bytes.len());
            if retained_bytes <= MAX_RETAINED_TURN_BYTES
                || is_collection_migration_event(&row.value)
            {
                rows.push(row);
            }
            source.offset = position.saturating_add(bytes as u64);
        }
        bytes_scanned = bytes_scanned.saturating_add(source.offset.saturating_sub(start_offset));
        checkpoint.sources.insert(source_sha256, source);
        miner.flush()?;
        persist_collection_migration_checkpoint(migration_checkpoint_path, &checkpoint)?;
    }

    miner.flush()?;
    persist_collection_migration_checkpoint(migration_checkpoint_path, &checkpoint)?;
    let status = miner.status();
    Ok(CollectionMigrationReport {
        schema: SCHEMA.to_owned(),
        elapsed_millis: started.elapsed().as_millis(),
        deadline_reached,
        files_seen,
        bytes_scanned,
        turns_scanned,
        observations_before,
        observations_after: status.observations_total,
        supported_observations: status
            .observations_total
            .saturating_sub(status.unsupported_total),
        unsupported_observations: status.unsupported_total,
        bucket_count: status.buckets.len(),
        support_rows: status
            .buckets
            .iter()
            .map(|bucket| bucket.support_rows)
            .sum(),
        support_tokens: status
            .buckets
            .iter()
            .map(|bucket| bucket.support_tokens)
            .sum(),
        frozen_buckets: status.buckets.iter().filter(|bucket| bucket.frozen).count(),
    })
}

fn observe_collection_turn(
    source_id: &str,
    turn_index: u64,
    rows: &[BackfillRow],
    miner: &mut OnlineCollectionMiner,
) -> Result<(), String> {
    let Some((example, estimated_input_tokens)) = collection_example(rows, true) else {
        return Ok(());
    };
    let intent = format!("{source_id}:turn:{turn_index}");
    let event_material = rows
        .iter()
        .filter(|row| is_collection_migration_event(&row.value))
        .map(|row| (row.offset, event_id(&row.value), sha256_bytes(&row.bytes)))
        .collect::<Vec<_>>();
    let evidence_graph_sha256 = sha256_bytes(
        &canonical_json_bytes(&(source_id, turn_index, event_material))
            .map_err(|error| format!("collection_migration_digest:{error}"))?,
    );
    miner.observe_buffered(OnlineCollectionObservation {
        evidence_graph_sha256,
        client_intent_id_sha256: sha256_bytes(intent.as_bytes()),
        session_id_sha256: sha256_bytes(source_id.as_bytes()),
        event_time_unix_nanos: rows.iter().rev().find_map(|row| event_time(&row.value)),
        estimated_input_tokens,
        example,
    })
}

fn is_collection_migration_event(row: &Value) -> bool {
    is_minimal_collection_event(row) || is_tool_output(row) || is_tool_call(row)
}

fn is_tool_call(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("response_item")
        && matches!(
            row.get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str),
            Some("function_call" | "custom_tool_call")
        )
}

fn load_collection_migration_checkpoint(
    path: &Path,
    schema: &str,
) -> Result<CollectionMigrationCheckpoint, String> {
    if !path.exists() {
        return Ok(CollectionMigrationCheckpoint {
            schema: schema.to_owned(),
            sources: BTreeMap::new(),
        });
    }
    let checkpoint = serde_json::from_slice::<CollectionMigrationCheckpoint>(
        &fs::read(path).map_err(|error| format!("collection_migration_checkpoint_read:{error}"))?,
    )
    .map_err(|error| format!("collection_migration_checkpoint_decode:{error}"))?;
    if checkpoint.schema != schema {
        return Err("collection_migration_checkpoint_schema".to_owned());
    }
    Ok(checkpoint)
}

fn persist_collection_migration_checkpoint(
    path: &Path,
    checkpoint: &CollectionMigrationCheckpoint,
) -> Result<(), String> {
    persist_json_atomically(path, checkpoint, "collection_migration_checkpoint")
}

fn persist_json_atomically<T: Serialize>(
    path: &Path,
    value: &T,
    label: &str,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{label}_dir:{error}"))?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary).map_err(|error| format!("{label}_create:{error}"))?;
    file.write_all(
        &canonical_json_bytes(value).map_err(|error| format!("{label}_encode:{error}"))?,
    )
    .map_err(|error| format!("{label}_write:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("{label}_sync:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("{label}_rename:{error}"))?;
    Ok(())
}

fn run_backfill(
    root: &Path,
    checkpoint_path: &Path,
    evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
    evidence_graphs: &Arc<Mutex<DeterministicEvidenceGraphStore>>,
    collection_miner: &Arc<Mutex<OnlineCollectionMiner>>,
) -> Result<(), String> {
    let mut checkpoint = load_checkpoint(checkpoint_path)?;
    let mut paths = session_files(root);
    paths.sort_by(|left, right| {
        let left_modified = fs::metadata(left).and_then(|value| value.modified()).ok();
        let right_modified = fs::metadata(right).and_then(|value| value.modified()).ok();
        right_modified
            .cmp(&left_modified)
            .then_with(|| left.cmp(right))
    });
    let mut turns_since_checkpoint = 0_u32;
    for path in paths {
        let source_id = path.to_string_lossy().into_owned();
        let source_sha256 = sha256_bytes(source_id.as_bytes());
        let evidence_covered_offset = checkpoint
            .evidence_covered_sources
            .get(&source_sha256)
            .map(|source| source.offset);
        let mut source = checkpoint
            .sources
            .get(&source_sha256)
            .copied()
            .unwrap_or_default();
        let mut file = File::open(&path)
            .map_err(|error| format!("session_backfill_open:{}:{error}", path.display()))?;
        let length = file
            .metadata()
            .map_err(|error| format!("session_backfill_metadata:{error}"))?
            .len();
        if source.offset > length {
            source = BackfillSource::default();
        }
        file.seek(SeekFrom::Start(source.offset))
            .map_err(|error| format!("session_backfill_seek:{error}"))?;
        let pass_start_offset = source.offset;
        let mut source_turns_this_pass = 0_u32;
        let mut reader = BufReader::new(file);
        let mut rows = Vec::new();
        let mut retained_bytes = 0_usize;
        let mut graph_overflow = false;
        let mut overflow_output_ordinal = 0_u32;
        let mut line = String::new();
        loop {
            line.clear();
            let position = reader
                .stream_position()
                .map_err(|error| format!("session_backfill_position:{error}"))?;
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| format!("session_backfill_read:{error}"))?;
            if bytes == 0 || !line.ends_with('\n') {
                if !rows.is_empty() {
                    if graph_overflow {
                        ingest_rows_only(
                            &source_id,
                            source.turn_index,
                            &rows,
                            &mut overflow_output_ordinal,
                            evidence_covered_offset,
                            evidence,
                        )?;
                    } else {
                        process_rows(
                            &source_id,
                            source.turn_index,
                            &rows,
                            evidence_covered_offset,
                            evidence,
                            evidence_graphs,
                            collection_miner,
                        )?;
                    }
                    rows.clear();
                }
                source.offset = position;
                break;
            }
            let Ok(value) = serde_json::from_str::<Value>(line.trim_end()) else {
                ingest_unparsed_row(
                    &source_id,
                    source.turn_index,
                    position,
                    line.trim_end().as_bytes(),
                    evidence_covered_offset,
                    evidence,
                )?;
                source.offset = position.saturating_add(bytes as u64);
                continue;
            };
            if is_turn_boundary(&value) && (!rows.is_empty() || graph_overflow) {
                if graph_overflow && !rows.is_empty() {
                    ingest_rows_only(
                        &source_id,
                        source.turn_index,
                        &rows,
                        &mut overflow_output_ordinal,
                        evidence_covered_offset,
                        evidence,
                    )?;
                } else if !rows.is_empty() {
                    process_rows(
                        &source_id,
                        source.turn_index,
                        &rows,
                        evidence_covered_offset,
                        evidence,
                        evidence_graphs,
                        collection_miner,
                    )?;
                }
                rows.clear();
                retained_bytes = 0;
                graph_overflow = false;
                overflow_output_ordinal = 0;
                source.offset = position;
                checkpoint.sources.insert(source_sha256.clone(), source);
                turns_since_checkpoint = turns_since_checkpoint.saturating_add(1);
                source_turns_this_pass = source_turns_this_pass.saturating_add(1);
                if turns_since_checkpoint >= 32 {
                    flush_collection_miner(collection_miner)?;
                    persist_checkpoint(checkpoint_path, &checkpoint)?;
                    turns_since_checkpoint = 0;
                }
                thread::sleep(Duration::from_millis(2));
                if source.offset.saturating_sub(pass_start_offset) >= MAX_SOURCE_BYTES_PER_PASS
                    || source_turns_this_pass >= MAX_SOURCE_TURNS_PER_PASS
                {
                    break;
                }
            }
            if is_turn_boundary(&value) {
                source.turn_index = source.turn_index.saturating_add(1);
            }
            let row = BackfillRow {
                offset: position,
                bytes: line.trim_end().as_bytes().to_vec(),
                value,
            };
            retained_bytes = retained_bytes.saturating_add(row.bytes.len());
            rows.push(row);
            if retained_bytes > MAX_RETAINED_TURN_BYTES {
                graph_overflow = true;
            }
            if graph_overflow
                && (rows.len() >= OVERFLOW_LEDGER_BATCH_ROWS
                    || retained_bytes >= MAX_RETAINED_TURN_BYTES)
            {
                ingest_rows_only(
                    &source_id,
                    source.turn_index,
                    &rows,
                    &mut overflow_output_ordinal,
                    evidence_covered_offset,
                    evidence,
                )?;
                rows.clear();
                retained_bytes = 0;
            }
            source.offset = position.saturating_add(bytes as u64);
        }
        checkpoint.sources.insert(source_sha256, source);
        flush_collection_miner(collection_miner)?;
        persist_checkpoint(checkpoint_path, &checkpoint)?;
    }
    Ok(())
}

fn process_rows(
    source_id: &str,
    turn_index: u64,
    rows: &[BackfillRow],
    evidence_covered_offset: Option<u64>,
    evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
    evidence_graphs: &Arc<Mutex<DeterministicEvidenceGraphStore>>,
    collection_miner: &Arc<Mutex<OnlineCollectionMiner>>,
) -> Result<(), String> {
    let mut output_ordinal = 0_u32;
    let envelopes = build_envelopes(source_id, turn_index, rows, &mut output_ordinal);
    ingest_envelopes(evidence, &envelopes, evidence_covered_offset)?;
    let canonical = rows
        .iter()
        .zip(envelopes.iter())
        .filter(|(row, _)| is_evidence_graph_event(&row.value))
        .filter_map(|(_, envelope)| {
            canonicalize_evidence_envelope(envelope, EvidencePolicyV1::streaming_bounded()).ok()
        })
        .collect::<Vec<CanonicalEventGraph>>();
    let node_count = canonical
        .iter()
        .map(|graph| graph.nodes.len())
        .sum::<usize>();
    let graph_overflow = canonical.len() > MAX_TURN_EVENTS || node_count > MAX_TURN_NODES;
    let canonical = if graph_overflow {
        rows.iter()
            .zip(envelopes.iter())
            .filter(|(row, _)| is_minimal_collection_event(&row.value))
            .filter_map(|(_, envelope)| {
                canonicalize_evidence_envelope(envelope, EvidencePolicyV1::streaming_bounded()).ok()
            })
            .collect()
    } else {
        canonical
    };
    process_canonical_rows(
        canonical,
        rows,
        !graph_overflow,
        evidence_graphs,
        collection_miner,
    )
}

fn build_envelopes(
    source_id: &str,
    turn_index: u64,
    rows: &[BackfillRow],
    output_ordinal: &mut u32,
) -> Vec<RawEvidenceEnvelope> {
    rows.iter()
        .map(|row| {
            if is_tool_output(&row.value) {
                *output_ordinal = (*output_ordinal).saturating_add(1);
            }
            RawEvidenceEnvelope {
                source_stream_id: source_id.to_owned(),
                source_offset: row.offset,
                event_id: event_id(&row.value).unwrap_or_else(|| sha256_bytes(&row.bytes)),
                session_id: source_id.to_owned(),
                client_intent_id: (turn_index > 0)
                    .then(|| format!("{source_id}:turn:{turn_index}")),
                call_id: call_id(&row.value),
                output_ordinal: is_tool_output(&row.value).then_some(*output_ordinal),
                event_time_unix_nanos: event_time(&row.value),
                schema_version: 1,
                payload: row.bytes.clone(),
            }
        })
        .collect()
}

fn ingest_envelopes(
    evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
    envelopes: &[RawEvidenceEnvelope],
    evidence_covered_offset: Option<u64>,
) -> Result<(), String> {
    let mut start = envelopes.partition_point(|envelope| {
        evidence_covered_offset.is_some_and(|offset| envelope.source_offset < offset)
    });
    while start < envelopes.len() {
        let mut end = start;
        let mut bytes = 0_usize;
        while end < envelopes.len() && end.saturating_sub(start) < MAX_LEDGER_BATCH_ROWS {
            let next = envelopes[end].payload.len();
            if end > start && bytes.saturating_add(next) > MAX_LEDGER_BATCH_BYTES {
                break;
            }
            bytes = bytes.saturating_add(next);
            end = end.saturating_add(1);
            if bytes >= MAX_LEDGER_BATCH_BYTES {
                break;
            }
        }
        evidence
            .lock()
            .map_err(|_| "session_backfill_evidence_lock_poisoned".to_owned())?
            .ingest_unseen_batch(envelopes[start..end].to_vec())?;
        start = end;
    }
    Ok(())
}

fn ingest_rows_only(
    source_id: &str,
    turn_index: u64,
    rows: &[BackfillRow],
    output_ordinal: &mut u32,
    evidence_covered_offset: Option<u64>,
    evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
) -> Result<(), String> {
    let envelopes = build_envelopes(source_id, turn_index, rows, output_ordinal);
    ingest_envelopes(evidence, &envelopes, evidence_covered_offset)
}

fn ingest_unparsed_row(
    source_id: &str,
    turn_index: u64,
    source_offset: u64,
    payload: &[u8],
    evidence_covered_offset: Option<u64>,
    evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
) -> Result<(), String> {
    ingest_envelopes(
        evidence,
        &[RawEvidenceEnvelope {
            source_stream_id: source_id.to_owned(),
            source_offset,
            event_id: sha256_bytes(payload),
            session_id: source_id.to_owned(),
            client_intent_id: (turn_index > 0).then(|| format!("{source_id}:turn:{turn_index}")),
            call_id: None,
            output_ordinal: None,
            event_time_unix_nanos: None,
            schema_version: 1,
            payload: payload.to_vec(),
        }],
        evidence_covered_offset,
    )
}

fn process_canonical_rows(
    canonical: Vec<CanonicalEventGraph>,
    rows: &[BackfillRow],
    include_outputs: bool,
    evidence_graphs: &Arc<Mutex<DeterministicEvidenceGraphStore>>,
    collection_miner: &Arc<Mutex<OnlineCollectionMiner>>,
) -> Result<(), String> {
    let node_count = canonical
        .iter()
        .map(|graph| graph.nodes.len())
        .sum::<usize>();
    if canonical.is_empty() || canonical.len() > MAX_TURN_EVENTS || node_count > MAX_TURN_NODES {
        return Ok(());
    }
    let graph = EvidenceGraphBuilder::build(
        &canonical,
        EvidenceGraphPolicy {
            max_events: MAX_TURN_EVENTS,
            max_atoms: 32_768,
        },
    )
    .map_err(str::to_owned)?;
    let graph_sha256 = graph.graph_sha256.clone();
    evidence_graphs
        .lock()
        .map_err(|_| "session_backfill_graph_lock_poisoned".to_owned())?
        .append(graph)?;
    let Some((example, tokens)) = collection_example(rows, include_outputs) else {
        return Ok(());
    };
    let last = canonical
        .last()
        .ok_or_else(|| "session_backfill_graph_missing".to_owned())?;
    let Some(client_intent_id_sha256) = last.client_intent_id_sha256.clone() else {
        return Ok(());
    };
    let event_time_unix_nanos = canonical
        .iter()
        .rev()
        .find_map(|graph| match graph.event_time {
            nando_response_actor::EvidenceEventTime::Known { unix_nanos } => Some(unix_nanos),
            nando_response_actor::EvidenceEventTime::Unknown => None,
        });
    collection_miner
        .lock()
        .map_err(|_| "session_backfill_collection_lock_poisoned".to_owned())?
        .observe_buffered(OnlineCollectionObservation {
            evidence_graph_sha256: graph_sha256,
            client_intent_id_sha256,
            session_id_sha256: last.session_id_sha256.clone(),
            event_time_unix_nanos,
            estimated_input_tokens: tokens,
            example,
        })
}

fn flush_collection_miner(
    collection_miner: &Arc<Mutex<OnlineCollectionMiner>>,
) -> Result<(), String> {
    collection_miner
        .lock()
        .map_err(|_| "session_backfill_collection_lock_poisoned".to_owned())?
        .flush()
}

fn collection_example(
    rows: &[BackfillRow],
    include_outputs: bool,
) -> Option<(CollectionSynthesisExample, u64)> {
    let mut requests = Vec::<Value>::new();
    let mut outputs = Vec::<Value>::new();
    let mut expected = None::<String>;
    let mut tokens = 0_u64;
    for row in rows {
        let row_type = row.value.get("type").and_then(Value::as_str).unwrap_or("");
        let data = row.value.get("payload")?;
        let payload_type = data.get("type").and_then(Value::as_str).unwrap_or("");
        if row_type == "event_msg"
            && payload_type == "user_message"
            && let Some(message) = data.get("message").and_then(Value::as_str)
            && let Some(item) = request_item(message)
        {
            requests.push(item);
        }
        if row_type == "response_item"
            && payload_type == "message"
            && data.get("role").and_then(Value::as_str) == Some("user")
            && let Some(message) = message_text(data.get("content"))
            && let Some(item) = request_item(&message)
        {
            requests.push(item);
        }
        if row_type == "response_item"
            && matches!(payload_type, "function_call" | "custom_tool_call")
        {
            requests.push(data.clone());
        }
        if include_outputs
            && row_type == "response_item"
            && matches!(
                payload_type,
                "function_call_output" | "custom_tool_call_output"
            )
            && let Some(candidate) = data.get("output").and_then(collection_payload)
            && let Some(item) = candidate
                .get("input")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
        {
            outputs.push(item.clone());
        }
        if row_type == "event_msg"
            && payload_type == "agent_message"
            && data.get("phase").and_then(Value::as_str) == Some("final_answer")
        {
            expected = data
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        if row_type == "response_item"
            && payload_type == "message"
            && data.get("role").and_then(Value::as_str) == Some("assistant")
            && data.get("phase").and_then(Value::as_str) == Some("final_answer")
        {
            expected = message_text(data.get("content"));
        }
        if row_type == "event_msg" && payload_type == "token_count" {
            tokens = data
                .get("info")
                .and_then(|info| info.get("last_token_usage"))
                .and_then(|usage| usage.get("input_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
        }
    }
    if outputs.is_empty() && requests.is_empty() {
        return None;
    }
    let expected_response = expected.filter(|value| !value.is_empty() && value.len() <= 16_384)?;
    requests.extend(outputs);
    Some((
        CollectionSynthesisExample {
            provider_payload: serde_json::json!({"input": requests}),
            expected_response,
        },
        tokens,
    ))
}

fn request_item(message: &str) -> Option<Value> {
    (!message.is_empty() && message.len() <= 16_384).then(|| {
        serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{"type":"input_text", "text":message}],
        })
    })
}

fn collection_payload(output: &Value) -> Option<Value> {
    let text = output.as_str()?;
    if text.is_empty() || text.len() > 65_536 {
        return None;
    }
    Some(serde_json::json!({
        "input": [{"type":"function_call_output", "output":text}]
    }))
}

fn message_text(content: Option<&Value>) -> Option<String> {
    let text = content?
        .as_array()?
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

fn is_turn_boundary(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("turn_context")
}

fn is_evidence_graph_event(row: &Value) -> bool {
    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let payload_type = row
        .get("payload")
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    row_type == "turn_context"
        || (row_type == "response_item"
            && matches!(
                payload_type,
                "function_call"
                    | "custom_tool_call"
                    | "function_call_output"
                    | "custom_tool_call_output"
                    | "message"
            ))
        || (row_type == "event_msg"
            && matches!(
                payload_type,
                "agent_message" | "token_count" | "user_message"
            ))
}

fn is_minimal_collection_event(row: &Value) -> bool {
    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let payload = row.get("payload");
    let payload_type = payload
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    row_type == "turn_context"
        || (row_type == "event_msg"
            && matches!(
                payload_type,
                "user_message" | "agent_message" | "token_count"
            ))
        || (row_type == "response_item"
            && payload_type == "message"
            && matches!(
                payload
                    .and_then(|payload| payload.get("role"))
                    .and_then(Value::as_str),
                Some("user" | "assistant")
            ))
}

fn is_tool_output(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("response_item")
        && matches!(
            row.get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        )
}

fn event_id(row: &Value) -> Option<String> {
    row.get("payload")
        .and_then(|payload| payload.get("id").or_else(|| payload.get("call_id")))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn call_id(row: &Value) -> Option<String> {
    row.get("payload")
        .and_then(|payload| payload.get("call_id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn event_time(row: &Value) -> Option<u64> {
    let timestamp = row.get("timestamp")?.as_str()?;
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    u64::try_from(parsed.unix_timestamp_nanos()).ok()
}

fn session_files(root: &Path) -> Vec<PathBuf> {
    let mut output = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                output.push(path);
            }
        }
    }
    output
}

fn load_checkpoint(path: &Path) -> Result<BackfillCheckpoint, String> {
    if !path.exists() {
        let evidence_covered_sources = load_previous_generation(path)?
            .map(|checkpoint| {
                let mut coverage = checkpoint.evidence_covered_sources;
                for (source, state) in checkpoint.sources {
                    coverage
                        .entry(source)
                        .and_modify(|current| {
                            if state.offset > current.offset {
                                *current = state;
                            }
                        })
                        .or_insert(state);
                }
                coverage
            })
            .unwrap_or_default();
        return Ok(BackfillCheckpoint {
            schema: BACKFILL_SCHEMA_V1.to_owned(),
            sources: BTreeMap::new(),
            evidence_covered_sources,
        });
    }
    let checkpoint: BackfillCheckpoint = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("session_backfill_checkpoint_read:{error}"))?,
    )
    .map_err(|error| format!("session_backfill_checkpoint_decode:{error}"))?;
    if checkpoint.schema != BACKFILL_SCHEMA_V1 {
        return Err("session_backfill_checkpoint_schema".to_owned());
    }
    Ok(checkpoint)
}

fn load_previous_generation(path: &Path) -> Result<Option<BackfillCheckpoint>, String> {
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    let Some((prefix, version)) = stem.rsplit_once("-v") else {
        return Ok(None);
    };
    let Ok(version) = version.parse::<u32>() else {
        return Ok(None);
    };
    let Some(previous_version) = version.checked_sub(1).filter(|value| *value > 0) else {
        return Ok(None);
    };
    let previous = path.with_file_name(format!("{prefix}-v{previous_version}.json"));
    if !previous.exists() {
        return Ok(None);
    }
    let checkpoint: BackfillCheckpoint = serde_json::from_slice(
        &fs::read(&previous).map_err(|error| format!("session_backfill_previous_read:{error}"))?,
    )
    .map_err(|error| format!("session_backfill_previous_decode:{error}"))?;
    if checkpoint.schema != BACKFILL_SCHEMA_V1 {
        return Err("session_backfill_previous_schema".to_owned());
    }
    Ok(Some(checkpoint))
}

fn persist_checkpoint(path: &Path, checkpoint: &BackfillCheckpoint) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("session_backfill_checkpoint_dir:{error}"))?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)
        .map_err(|error| format!("session_backfill_checkpoint_create:{error}"))?;
    file.write_all(
        &canonical_json_bytes(checkpoint)
            .map_err(|error| format!("session_backfill_checkpoint_encode:{error}"))?,
    )
    .map_err(|error| format!("session_backfill_checkpoint_write:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("session_backfill_checkpoint_sync:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("session_backfill_checkpoint_rename:{error}"))?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("session_backfill_checkpoint_parent_sync:{error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_response_actor::{OnlineCollectionConfig, ResponsePackageState};
    use serde_json::json;

    #[test]
    fn evidence_prefix_offset_is_exclusive() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-prefix-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("ledger"),
        ));
        let envelope = RawEvidenceEnvelope {
            source_stream_id: "source".to_owned(),
            source_offset: 10,
            event_id: "event".to_owned(),
            session_id: "session".to_owned(),
            client_intent_id: None,
            call_id: None,
            output_ordinal: None,
            event_time_unix_nanos: None,
            schema_version: 1,
            payload: br#"{"type":"turn_context","payload":{}}"#.to_vec(),
        };

        ingest_envelopes(&evidence, &[envelope], Some(10)).expect("ingest boundary");

        assert_eq!(evidence.lock().expect("lock").accounting().ingress_total, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn overflow_projection_keeps_request_provenance_and_drops_outputs() {
        let rows = vec![
            BackfillRow {
                offset: 1,
                bytes: Vec::new(),
                value: json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Do the work. End only CAPTURE_COMPLETE."}]}}),
            },
            BackfillRow {
                offset: 2,
                bytes: Vec::new(),
                value: json!({"type":"response_item","payload":{"type":"function_call_output","output":"{\"private\":[1,2,3]}"}}),
            },
            BackfillRow {
                offset: 3,
                bytes: Vec::new(),
                value: json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"CAPTURE_COMPLETE"}}),
            },
        ];

        let (example, _) = collection_example(&rows, false).expect("request-only example");
        let input = example
            .provider_payload
            .get("input")
            .and_then(Value::as_array)
            .expect("input");
        assert_eq!(input.len(), 1);
        assert_eq!(example.expected_response, "CAPTURE_COMPLETE");
        assert_eq!(
            example.provider_payload,
            json!({
                "input":[{"type":"message","role":"user","content":[{
                    "type":"input_text",
                    "text":"Do the work. End only CAPTURE_COMPLETE."
                }]}]
            })
        );
        assert!(is_minimal_collection_event(&rows[0].value));
        assert!(!is_minimal_collection_event(&rows[1].value));
        assert!(is_minimal_collection_event(&rows[2].value));
        let space = nando_response_actor::enumerate_source_neutral_response_programs(&example)
            .expect("request-only synthesis");
        assert!(!space.programs.is_empty());
    }

    #[test]
    fn historical_backfill_is_resumable_and_builds_generic_quarantine() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-backfill-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let sessions = root.join("sessions");
        fs::create_dir_all(&sessions).expect("sessions");
        let session_path = sessions.join("history.jsonl");
        let mut file = File::create(&session_path).expect("session");
        serde_json::to_writer(
            &mut file,
            &json!({"timestamp":"2026-07-14T00:00:00Z","type":"session_meta","payload":{"id":"private-session"}}),
        )
        .expect("meta");
        file.write_all(b"\n").expect("newline");
        for index in 1..=8 {
            let rows = [
                json!({"timestamp":format!("2026-07-14T00:00:{index:02}Z"),"type":"turn_context","payload":{}}),
                json!({"timestamp":format!("2026-07-14T00:00:{index:02}Z"),"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("Count records for batch {index}")}]}}),
                json!({"timestamp":format!("2026-07-14T00:01:{index:02}Z"),"type":"response_item","payload":{"type":"function_call_output","call_id":format!("call-{index}"),"output":format!("{{\"surface_{index}\":[{{\"private\":{}}},{{\"private\":{}}},{{\"private\":{}}}]}}", index + 10, index + 20, index + 30)}}),
                json!({"timestamp":format!("2026-07-14T00:02:{index:02}Z"),"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"3"}}),
                json!({"timestamp":format!("2026-07-14T00:03:{index:02}Z"),"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100}}}}),
            ];
            for row in rows {
                serde_json::to_writer(&mut file, &row).expect("row");
                file.write_all(b"\n").expect("newline");
            }
        }
        file.sync_all().expect("sync");
        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("ledger"),
        ));
        let graphs = Arc::new(Mutex::new(
            DeterministicEvidenceGraphStore::open(root.join("graphs.jsonl")).expect("graphs"),
        ));
        let collection = Arc::new(Mutex::new(
            OnlineCollectionMiner::open(
                root.join("collection.json"),
                OnlineCollectionConfig {
                    support_rows: 4,
                    future_rows: 4,
                    max_buckets: 8,
                    max_receipts_per_bucket: 16,
                },
            )
            .expect("collection"),
        ));
        let checkpoint = root.join("session-evidence-backfill-v1.json");
        run_backfill(&sessions, &checkpoint, &evidence, &graphs, &collection).expect("backfill");
        let status = collection.lock().expect("lock").status();
        assert_eq!(status.observations_total, 8);
        let packages = collection
            .lock()
            .expect("lock")
            .quarantine_packages()
            .expect("packages");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].state, ResponsePackageState::Quarantine);
        assert!(packages[0].required_routing_atom_ids.len() > 2);
        run_backfill(&sessions, &checkpoint, &evidence, &graphs, &collection).expect("resume");
        assert_eq!(collection.lock().expect("lock").status(), status);
        let accounting_before_generation_replay = evidence.lock().expect("ledger").accounting();
        let second_collection = Arc::new(Mutex::new(
            OnlineCollectionMiner::open(
                root.join("collection-v2.json"),
                OnlineCollectionConfig {
                    support_rows: 4,
                    future_rows: 4,
                    max_buckets: 8,
                    max_receipts_per_bucket: 16,
                },
            )
            .expect("second collection"),
        ));
        run_backfill(
            &sessions,
            &root.join("session-evidence-backfill-v2.json"),
            &evidence,
            &graphs,
            &second_collection,
        )
        .expect("synthesis generation replay");
        assert_eq!(
            second_collection
                .lock()
                .expect("second collection lock")
                .status()
                .observations_total,
            8
        );
        assert_eq!(
            evidence.lock().expect("ledger").accounting(),
            accounting_before_generation_replay
        );
        for path in [root.join("collection.json"), root.join("graphs.jsonl")] {
            let durable = String::from_utf8(fs::read(path).expect("durable")).expect("utf8");
            for private in ["private-session", "surface_", "private", "Count records"] {
                assert!(!durable.contains(private), "durable leak: {private}");
            }
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
