use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{LearningRequestStructureV2, PreActionTopologyCommitV1};
use nando_operator_kernel::{sha256_bytes, valid_nonzero_sha256};
use nando_operator_learning::{
    LEARNING_STRUCTURE_RECORD_MAX_BYTES_V3, LearningRequestStructureV1, LearningStructureRecordV2,
    LearningStructureRecordV3, ProviderRequestCaptureReceiptV3,
};
use serde::{Deserialize, Serialize};

use crate::request_learning::{
    REQUEST_LEARNING_CHECKPOINT_MAX_BYTES_V2, RequestLearningIndex, RequestLearningWatermarkV2,
};

const BRIDGE_STATUS_SCHEMA_V2: &str = "nando.learning-structure-bridge-status.v2";
const BRIDGE_META_SCHEMA_V2: &str = "nando.learning-structure-bridge-meta.v2";
const MAX_PENDING_BYTES_V2: u64 = 64 * 1024 * 1024;
const RECORD_SUFFIX: &str = ".cbor";
const PRODUCER_DURABILITY_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct LearningStructureEndpointStatusV2 {
    pub(crate) enabled: bool,
    pub(crate) ready: bool,
    pub(crate) records: u64,
    pub(crate) duplicates: u64,
    pub(crate) failures: u64,
    pub(crate) last_sequence: u64,
    pub(crate) durable_sequence: u64,
    pub(crate) durability_syncs: u64,
    pub(crate) last_micros: u64,
    pub(crate) max_micros: u64,
    pub(crate) last_error: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct LearningStructureBridgeStatusV2 {
    pub(crate) schema: String,
    pub(crate) bridge_epoch_sha256: String,
    pub(crate) pending_records: u64,
    pub(crate) pending_bytes: u64,
    pub(crate) sequence_gaps: u64,
    pub(crate) checkpoint_restored: bool,
    pub(crate) checkpoint_bytes: u64,
    pub(crate) producer: LearningStructureEndpointStatusV2,
    pub(crate) consumer: LearningStructureEndpointStatusV2,
}

#[derive(Default)]
struct EndpointCounters {
    records: AtomicU64,
    duplicates: AtomicU64,
    failures: AtomicU64,
    last_sequence: AtomicU64,
    durable_sequence: AtomicU64,
    durability_syncs: AtomicU64,
    last_micros: AtomicU64,
    max_micros: AtomicU64,
    last_error: RwLock<String>,
}

#[derive(Clone)]
pub(crate) struct LearningStructureBridgeRuntimeV2 {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    staging_dir: PathBuf,
    pending_dir: PathBuf,
    rejected_dir: PathBuf,
    checkpoint_path: PathBuf,
    epoch_sha256: String,
    producer_enabled: bool,
    consumer_enabled: bool,
    consumer_poll: Duration,
    next_sequence: AtomicU64,
    producer_lock: Mutex<()>,
    watermark: Mutex<RequestLearningWatermarkV2>,
    producer: EndpointCounters,
    consumer: EndpointCounters,
    consumer_started: AtomicBool,
    sequence_gaps: AtomicU64,
    checkpoint_restored: bool,
    producer_sync_requested: AtomicBool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BridgeMetaV2 {
    schema: String,
    bridge_epoch_sha256: String,
    next_sequence: u64,
}

impl LearningStructureBridgeRuntimeV2 {
    pub(crate) fn open(
        root: PathBuf,
        producer_enabled: bool,
        consumer_enabled: bool,
        consumer_poll: Duration,
    ) -> Result<(Self, Arc<RequestLearningIndex>), String> {
        if producer_enabled && consumer_enabled {
            return Err("learning_structure_bridge_roles_overlap".to_owned());
        }
        create_private_directory(&root)?;
        let staging_dir = root.join("staging");
        let pending_dir = root.join("pending");
        let rejected_dir = root.join("rejected");
        create_private_directory(&staging_dir)?;
        create_private_directory(&pending_dir)?;
        create_private_directory(&rejected_dir)?;
        recover_staging(&staging_dir, &pending_dir, &rejected_dir)?;

        let meta_path = root.join("bridge-meta-v2.cbor");
        let mut meta = load_or_create_meta(&meta_path, &root)?;
        meta.next_sequence = meta
            .next_sequence
            .max(next_pending_sequence(&pending_dir)?.saturating_add(1))
            .max(1);

        let checkpoint_path = root.join("request-learning-v2.checkpoint.cbor");
        let (request_learning, watermark, checkpoint_restored) =
            load_request_learning_checkpoint(&checkpoint_path, &meta.bridge_epoch_sha256)?;
        meta.next_sequence = meta
            .next_sequence
            .max(watermark.last_sequence.saturating_add(1));
        if producer_enabled {
            persist_cbor_atomic(&meta_path, &meta)?;
        }
        let producer = EndpointCounters::default();
        producer
            .last_sequence
            .store(meta.next_sequence.saturating_sub(1), Ordering::Release);
        producer
            .durable_sequence
            .store(watermark.last_sequence, Ordering::Release);
        let consumer = EndpointCounters::default();
        consumer
            .last_sequence
            .store(watermark.last_sequence, Ordering::Release);
        consumer
            .durable_sequence
            .store(watermark.last_sequence, Ordering::Release);
        let runtime = Self {
            inner: Arc::new(BridgeInner {
                staging_dir,
                pending_dir,
                rejected_dir,
                checkpoint_path,
                epoch_sha256: meta.bridge_epoch_sha256,
                producer_enabled,
                consumer_enabled,
                consumer_poll: consumer_poll.max(Duration::from_millis(10)),
                next_sequence: AtomicU64::new(meta.next_sequence),
                producer_lock: Mutex::new(()),
                watermark: Mutex::new(watermark),
                producer,
                consumer,
                consumer_started: AtomicBool::new(false),
                sequence_gaps: AtomicU64::new(0),
                checkpoint_restored,
                producer_sync_requested: AtomicBool::new(false),
            }),
        };
        if producer_enabled {
            runtime.start_producer_durability_worker()?;
        }
        Ok((runtime, Arc::new(request_learning)))
    }

    pub(crate) fn producer_enabled(&self) -> bool {
        self.inner.producer_enabled
    }

    pub(crate) fn submit(
        &self,
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure: LearningRequestStructureV1,
    ) -> Result<(), String> {
        if !self.inner.producer_enabled {
            return Err("learning_structure_bridge_producer_disabled".to_owned());
        }
        let started = Instant::now();
        let _guard = self
            .inner
            .producer_lock
            .lock()
            .map_err(|_| "learning_structure_bridge_producer_lock_poisoned".to_owned())?;
        let (_, pending_bytes) = pending_stats(&self.inner.pending_dir);
        if pending_bytes >= MAX_PENDING_BYTES_V2 {
            return self.producer_failure("learning_structure_bridge_spool_budget");
        }
        let sequence = self.inner.next_sequence.load(Ordering::Acquire);
        let record = LearningStructureRecordV2::new(
            self.inner.epoch_sha256.clone(),
            sequence,
            capture_receipt,
            structure,
        )
        .map_err(|error| format!("learning_structure_bridge_record:{error:?}"))?;
        let bytes = record
            .canonical_cbor()
            .map_err(|error| format!("learning_structure_bridge_encode:{error:?}"))?;
        let name = record_file_name(sequence, record.record_sha256());
        let final_path = self.inner.pending_dir.join(&name);
        let temporary_path = self.inner.staging_dir.join(format!("{name}.tmp"));
        // Hot publication is process-durable; the cold checkpoint is the durable ACK boundary.
        write_private_file_buffered(&temporary_path, &bytes)?;
        fs::rename(&temporary_path, &final_path)
            .map_err(|error| format!("learning_structure_bridge_publish:{error}"))?;

        let next_sequence = sequence.saturating_add(1);
        self.inner
            .next_sequence
            .store(next_sequence, Ordering::Release);
        record_success(&self.inner.producer, sequence);
        self.inner
            .producer_sync_requested
            .store(true, Ordering::Release);
        record_timing(&self.inner.producer, started);
        Ok(())
    }

    pub(crate) fn submit_v3(
        &self,
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure_v1: LearningRequestStructureV1,
        structure_v2: LearningRequestStructureV2,
        topology_commit: PreActionTopologyCommitV1,
    ) -> Result<(), String> {
        if !self.inner.producer_enabled {
            return Err("learning_structure_bridge_producer_disabled".to_owned());
        }
        let started = Instant::now();
        let _guard = self
            .inner
            .producer_lock
            .lock()
            .map_err(|_| "learning_structure_bridge_producer_lock_poisoned".to_owned())?;
        let (_, pending_bytes) = pending_stats(&self.inner.pending_dir);
        if pending_bytes >= MAX_PENDING_BYTES_V2 {
            return self.producer_failure("learning_structure_bridge_spool_budget");
        }
        let sequence = self.inner.next_sequence.load(Ordering::Acquire);
        let record = LearningStructureRecordV3::new(
            self.inner.epoch_sha256.clone(),
            sequence,
            capture_receipt,
            structure_v1,
            structure_v2,
            topology_commit,
        )
        .map_err(|error| format!("learning_structure_bridge_record_v3:{error:?}"))?;
        self.publish_record(
            sequence,
            record.record_sha256(),
            &record
                .canonical_cbor()
                .map_err(|error| format!("learning_structure_bridge_encode_v3:{error:?}"))?,
            started,
        )
    }

    fn publish_record(
        &self,
        sequence: u64,
        digest: &str,
        bytes: &[u8],
        started: Instant,
    ) -> Result<(), String> {
        let name = record_file_name(sequence, digest);
        let final_path = self.inner.pending_dir.join(&name);
        let temporary_path = self.inner.staging_dir.join(format!("{name}.tmp"));
        write_private_file_buffered(&temporary_path, bytes)?;
        fs::rename(&temporary_path, &final_path)
            .map_err(|error| format!("learning_structure_bridge_publish:{error}"))?;
        self.inner
            .next_sequence
            .store(sequence.saturating_add(1), Ordering::Release);
        record_success(&self.inner.producer, sequence);
        self.inner
            .producer_sync_requested
            .store(true, Ordering::Release);
        record_timing(&self.inner.producer, started);
        Ok(())
    }

    pub(crate) fn start_consumer(
        &self,
        request_learning: Arc<RequestLearningIndex>,
    ) -> Result<(), String> {
        if !self.inner.consumer_enabled {
            return Ok(());
        }
        if self
            .inner
            .consumer_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("learning_structure_bridge_consumer_already_started".to_owned());
        }
        let inner = Arc::downgrade(&self.inner);
        thread::Builder::new()
            .name("nando-learning-structure-v2".to_owned())
            .spawn(move || {
                while let Some(inner) = inner.upgrade() {
                    if let Err(error) = drain_pending(&inner, &request_learning) {
                        record_failure(&inner.consumer, &error);
                    }
                    let poll = inner.consumer_poll;
                    drop(inner);
                    thread::sleep(poll);
                }
            })
            .map(|_| ())
            .map_err(|error| format!("learning_structure_bridge_consumer_spawn:{error}"))
    }

    pub(crate) fn status(&self) -> LearningStructureBridgeStatusV2 {
        let (pending_records, pending_bytes) = pending_stats(&self.inner.pending_dir);
        LearningStructureBridgeStatusV2 {
            schema: BRIDGE_STATUS_SCHEMA_V2.to_owned(),
            bridge_epoch_sha256: self.inner.epoch_sha256.clone(),
            pending_records,
            pending_bytes,
            sequence_gaps: self.inner.sequence_gaps.load(Ordering::Relaxed),
            checkpoint_restored: self.inner.checkpoint_restored,
            checkpoint_bytes: self
                .inner
                .checkpoint_path
                .metadata()
                .map_or(0, |metadata| metadata.len()),
            producer: endpoint_status(
                &self.inner.producer,
                self.inner.producer_enabled,
                self.inner.producer_enabled,
            ),
            consumer: endpoint_status(
                &self.inner.consumer,
                self.inner.consumer_enabled,
                self.inner.consumer_started.load(Ordering::Acquire),
            ),
        }
    }

    fn producer_failure<T>(&self, error: &str) -> Result<T, String> {
        record_failure(&self.inner.producer, error);
        Err(error.to_owned())
    }

    fn start_producer_durability_worker(&self) -> Result<(), String> {
        let inner = Arc::downgrade(&self.inner);
        thread::Builder::new()
            .name("nando-learning-structure-sync-v2".to_owned())
            .spawn(move || {
                thread::sleep(PRODUCER_DURABILITY_INTERVAL);
                while let Some(inner) = inner.upgrade() {
                    if inner.producer_sync_requested.swap(false, Ordering::AcqRel) {
                        let previous = inner.producer.durable_sequence.load(Ordering::Acquire);
                        let target = inner.producer.last_sequence.load(Ordering::Acquire);
                        match sync_pending_records(&inner.pending_dir, previous, target) {
                            Ok(()) => {
                                inner
                                    .producer
                                    .durable_sequence
                                    .store(target, Ordering::Release);
                                inner
                                    .producer
                                    .durability_syncs
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Err(error) => {
                                inner.producer_sync_requested.store(true, Ordering::Release);
                                record_failure(&inner.producer, &error);
                            }
                        }
                    }
                    drop(inner);
                    thread::sleep(PRODUCER_DURABILITY_INTERVAL);
                }
            })
            .map(|_| ())
            .map_err(|error| format!("learning_structure_bridge_durability_spawn:{error}"))
    }
}

fn drain_pending(inner: &BridgeInner, index: &RequestLearningIndex) -> Result<(), String> {
    for path in pending_paths(&inner.pending_dir)? {
        let started = Instant::now();
        let (sequence, expected_digest) = pending_identity(&path)?;
        let record = read_record(&path, &expected_digest)?;
        if record.bridge_epoch_sha256() != inner.epoch_sha256 {
            quarantine(inner, &path, "learning_structure_bridge_epoch_mismatch")?;
            continue;
        }
        let watermark = inner
            .watermark
            .lock()
            .map_err(|_| "learning_structure_bridge_watermark_lock_poisoned".to_owned())?
            .clone();
        if sequence <= watermark.last_sequence {
            if sequence == watermark.last_sequence
                && record.record_sha256() != watermark.last_record_sha256
            {
                quarantine(
                    inner,
                    &path,
                    "learning_structure_bridge_ack_digest_mismatch",
                )?;
                continue;
            }
            acknowledge_path(&path, &inner.pending_dir)?;
            inner.consumer.duplicates.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let expected_sequence = watermark.last_sequence.saturating_add(1);
        if sequence != expected_sequence {
            inner.sequence_gaps.fetch_add(1, Ordering::Relaxed);
            return Err(format!(
                "learning_structure_bridge_sequence_gap:{expected_sequence}:{sequence}"
            ));
        }
        match &record {
            LearningStructureRecord::V2(_) => index
                .observe_structure(record.structure_v1())
                .map_err(str::to_owned)?,
            LearningStructureRecord::V3(record) => {
                index.observe_structure_v3(record).map_err(str::to_owned)?
            }
        }
        let next_watermark = RequestLearningWatermarkV2 {
            bridge_epoch_sha256: inner.epoch_sha256.clone(),
            last_sequence: sequence,
            last_record_sha256: record.record_sha256().to_owned(),
        };
        let checkpoint = index.checkpoint_cbor(&next_watermark)?;
        write_private_file_atomic(&inner.checkpoint_path, &checkpoint)?;
        *inner
            .watermark
            .lock()
            .map_err(|_| "learning_structure_bridge_watermark_lock_poisoned".to_owned())? =
            next_watermark;
        acknowledge_path(&path, &inner.pending_dir)?;
        record_success(&inner.consumer, sequence);
        inner
            .consumer
            .durable_sequence
            .store(sequence, Ordering::Release);
        record_timing(&inner.consumer, started);
    }
    Ok(())
}

fn load_request_learning_checkpoint(
    path: &Path,
    epoch_sha256: &str,
) -> Result<(RequestLearningIndex, RequestLearningWatermarkV2, bool), String> {
    let bytes = match read_bounded(path, REQUEST_LEARNING_CHECKPOINT_MAX_BYTES_V2)? {
        Some(bytes) => bytes,
        None => {
            return Ok((
                RequestLearningIndex::default(),
                RequestLearningWatermarkV2 {
                    bridge_epoch_sha256: epoch_sha256.to_owned(),
                    last_sequence: 0,
                    last_record_sha256: String::new(),
                },
                false,
            ));
        }
    };
    let (index, watermark) = RequestLearningIndex::from_checkpoint_cbor(&bytes)?;
    if watermark.bridge_epoch_sha256 != epoch_sha256 {
        return Err("learning_structure_bridge_checkpoint_epoch_mismatch".to_owned());
    }
    Ok((index, watermark, true))
}

fn load_or_create_meta(path: &Path, root: &Path) -> Result<BridgeMetaV2, String> {
    if let Some(bytes) = read_bounded(path, 4 * 1024)? {
        return decode_meta(&bytes);
    }
    let material = format!("{}:{}:{}", root.display(), process::id(), unix_now_nanos());
    let meta = BridgeMetaV2 {
        schema: BRIDGE_META_SCHEMA_V2.to_owned(),
        bridge_epoch_sha256: sha256_bytes(material.as_bytes()),
        next_sequence: 1,
    };
    match create_cbor_exclusive(path, &meta) {
        Ok(()) => Ok(meta),
        Err(error) if error.contains("File exists") => {
            let bytes = read_bounded(path, 4 * 1024)?
                .ok_or_else(|| "learning_structure_bridge_meta_race".to_owned())?;
            decode_meta(&bytes)
        }
        Err(error) => Err(error),
    }
}

fn decode_meta(bytes: &[u8]) -> Result<BridgeMetaV2, String> {
    let meta: BridgeMetaV2 = serde_cbor::from_slice(bytes)
        .map_err(|error| format!("learning_structure_bridge_meta_decode:{error}"))?;
    if meta.schema != BRIDGE_META_SCHEMA_V2
        || !valid_nonzero_sha256(&meta.bridge_epoch_sha256)
        || meta.next_sequence == 0
    {
        return Err("learning_structure_bridge_meta_invalid".to_owned());
    }
    Ok(meta)
}

enum LearningStructureRecord {
    V2(Box<LearningStructureRecordV2>),
    V3(Box<LearningStructureRecordV3>),
}

impl LearningStructureRecord {
    fn bridge_epoch_sha256(&self) -> &str {
        match self {
            Self::V2(record) => record.bridge_epoch_sha256(),
            Self::V3(record) => record.bridge_epoch_sha256(),
        }
    }

    fn record_sha256(&self) -> &str {
        match self {
            Self::V2(record) => record.record_sha256(),
            Self::V3(record) => record.record_sha256(),
        }
    }

    fn structure_v1(&self) -> &LearningRequestStructureV1 {
        match self {
            Self::V2(record) => record.structure(),
            Self::V3(record) => record.structure_v1(),
        }
    }
}

fn read_record(path: &Path, expected_digest: &str) -> Result<LearningStructureRecord, String> {
    let bytes = read_bounded(path, LEARNING_STRUCTURE_RECORD_MAX_BYTES_V3)?
        .ok_or_else(|| "learning_structure_bridge_record_missing".to_owned())?;
    let record = LearningStructureRecordV3::from_canonical_cbor(&bytes)
        .map(|record| LearningStructureRecord::V3(Box::new(record)))
        .or_else(|_| {
            LearningStructureRecordV2::from_canonical_cbor(&bytes)
                .map(|record| LearningStructureRecord::V2(Box::new(record)))
        })
        .map_err(|error| format!("learning_structure_bridge_record_decode:{error:?}"))?;
    if record.record_sha256() != expected_digest {
        return Err("learning_structure_bridge_filename_digest_mismatch".to_owned());
    }
    Ok(record)
}

fn pending_paths(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = fs::read_dir(directory)
        .map_err(|error| format!("learning_structure_bridge_pending_read:{error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("cbor"))
        .collect::<Vec<_>>();
    paths.sort_unstable();
    Ok(paths)
}

fn pending_identity(path: &Path) -> Result<(u64, String), String> {
    let stem = path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_suffix(RECORD_SUFFIX))
        .ok_or_else(|| "learning_structure_bridge_pending_name_invalid".to_owned())?;
    let (sequence, digest) = stem
        .split_once('-')
        .ok_or_else(|| "learning_structure_bridge_pending_name_invalid".to_owned())?;
    if sequence.len() != 20 || !valid_nonzero_sha256(digest) {
        return Err("learning_structure_bridge_pending_name_invalid".to_owned());
    }
    Ok((
        sequence
            .parse()
            .map_err(|_| "learning_structure_bridge_sequence_invalid".to_owned())?,
        digest.to_owned(),
    ))
}

fn next_pending_sequence(directory: &Path) -> Result<u64, String> {
    pending_paths(directory)?
        .iter()
        .try_fold(0_u64, |maximum, path| {
            pending_identity(path).map(|(sequence, _)| maximum.max(sequence))
        })
}

fn pending_stats(directory: &Path) -> (u64, u64) {
    pending_paths(directory).map_or((0, 0), |paths| {
        paths.iter().fold((0_u64, 0_u64), |(count, bytes), path| {
            (
                count.saturating_add(1),
                bytes.saturating_add(path.metadata().map_or(0, |metadata| metadata.len())),
            )
        })
    })
}

fn sync_pending_records(
    directory: &Path,
    after_sequence: u64,
    through_sequence: u64,
) -> Result<(), String> {
    for path in pending_paths(directory)? {
        let Ok((sequence, _)) = pending_identity(&path) else {
            continue;
        };
        if sequence <= after_sequence || sequence > through_sequence {
            continue;
        }
        match File::open(&path).and_then(|file| file.sync_data()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("learning_structure_bridge_pending_sync:{error}"));
            }
        }
    }
    sync_directory(directory)
}

fn recover_staging(staging: &Path, pending: &Path, rejected: &Path) -> Result<(), String> {
    for entry in fs::read_dir(staging)
        .map_err(|error| format!("learning_structure_bridge_recover_read:{error}"))?
    {
        let path = entry
            .map_err(|error| format!("learning_structure_bridge_recover_entry:{error}"))?
            .path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(final_name) = name.strip_suffix(".tmp") else {
            continue;
        };
        let final_path = pending.join(final_name);
        if final_path.exists() {
            fs::remove_file(path)
                .map_err(|error| format!("learning_structure_bridge_recover_remove:{error}"))?;
        } else {
            fs::rename(&path, rejected.join(format!("{name}.incomplete")))
                .map_err(|error| format!("learning_structure_bridge_recover_reject:{error}"))?;
        }
    }
    Ok(())
}

fn quarantine(inner: &BridgeInner, path: &Path, reason: &str) -> Result<(), String> {
    let name = path
        .file_name()
        .ok_or_else(|| "learning_structure_bridge_quarantine_name".to_owned())?;
    fs::rename(path, inner.rejected_dir.join(name))
        .map_err(|error| format!("learning_structure_bridge_quarantine:{reason}:{error}"))?;
    sync_directory(&inner.pending_dir)?;
    sync_directory(&inner.rejected_dir)
}

fn acknowledge_path(path: &Path, pending_dir: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|error| format!("learning_structure_bridge_ack:{error}"))?;
    sync_directory(pending_dir)
}

fn record_file_name(sequence: u64, digest: &str) -> String {
    format!("{sequence:020}-{digest}{RECORD_SUFFIX}")
}

fn endpoint_status(
    counters: &EndpointCounters,
    enabled: bool,
    ready: bool,
) -> LearningStructureEndpointStatusV2 {
    LearningStructureEndpointStatusV2 {
        enabled,
        ready,
        records: counters.records.load(Ordering::Relaxed),
        duplicates: counters.duplicates.load(Ordering::Relaxed),
        failures: counters.failures.load(Ordering::Relaxed),
        last_sequence: counters.last_sequence.load(Ordering::Acquire),
        durable_sequence: counters.durable_sequence.load(Ordering::Acquire),
        durability_syncs: counters.durability_syncs.load(Ordering::Relaxed),
        last_micros: counters.last_micros.load(Ordering::Relaxed),
        max_micros: counters.max_micros.load(Ordering::Relaxed),
        last_error: counters
            .last_error
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "learning_structure_bridge_status_lock_poisoned".to_owned()),
    }
}

fn record_success(counters: &EndpointCounters, sequence: u64) {
    counters.records.fetch_add(1, Ordering::Relaxed);
    counters.last_sequence.store(sequence, Ordering::Release);
}

fn record_failure(counters: &EndpointCounters, error: &str) {
    counters.failures.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut last_error) = counters.last_error.write() {
        *last_error = error.to_owned();
    }
}

fn record_timing(counters: &EndpointCounters, started: Instant) {
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    counters.last_micros.store(micros, Ordering::Relaxed);
    counters.max_micros.fetch_max(micros, Ordering::Relaxed);
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("learning_structure_bridge_mkdir:{}:{error}", path.display()))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        format!(
            "learning_structure_bridge_directory_mode:{}:{error}",
            path.display()
        )
    })?;
    Ok(())
}

fn create_cbor_exclusive<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_cbor::to_vec(value)
        .map_err(|error| format!("learning_structure_bridge_encode:{error}"))?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(|error| format!("learning_structure_bridge_create:{error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("learning_structure_bridge_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("learning_structure_bridge_sync:{error}"))?;
    sync_directory(
        path.parent()
            .ok_or_else(|| "learning_structure_bridge_parent_missing".to_owned())?,
    )
}

fn persist_cbor_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_cbor::to_vec(value)
        .map_err(|error| format!("learning_structure_bridge_encode:{error}"))?;
    write_private_file_atomic(path, &bytes)
}

fn write_private_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    write_private_file(&temporary, bytes)?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("learning_structure_bridge_atomic_publish:{error}"))?;
    sync_directory(
        path.parent()
            .ok_or_else(|| "learning_structure_bridge_parent_missing".to_owned())?,
    )
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(|error| format!("learning_structure_bridge_file_open:{error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("learning_structure_bridge_file_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("learning_structure_bridge_file_sync:{error}"))
}

fn write_private_file_buffered(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(|error| format!("learning_structure_bridge_file_open:{error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("learning_structure_bridge_file_write:{error}"))
}

fn read_bounded(path: &Path, max_bytes: usize) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("learning_structure_bridge_file_open:{error}")),
    };
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(u64::try_from(max_bytes + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("learning_structure_bridge_file_read:{error}"))?;
    if bytes.len() > max_bytes {
        return Err("learning_structure_bridge_file_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("learning_structure_bridge_directory_sync:{error}"))
}

fn unix_now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
        MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1, PreActionMultiSourceTopologyV1,
        PreActionTopologyCommitV1, RuntimeProjectionV3, Sha256CommitmentV3, sha256_bytes,
    };
    use nando_operator_learning::{
        LearningRequestStructureInputV1, ProviderRequestCaptureInputV3,
        seal_provider_request_capture_v3,
    };

    use super::*;

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-learning-structure-{label}-{}-{}",
            process::id(),
            unix_now_nanos()
        ))
    }

    fn evidence(sequence: u64) -> (ProviderRequestCaptureReceiptV3, LearningRequestStructureV1) {
        let payload = format!("request-{sequence}");
        let request_root = Sha256CommitmentV3::digest_bytes(payload.as_bytes());
        let receipt = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: sequence,
            capture_epoch_root: Sha256CommitmentV3::digest_bytes(b"capture-epoch"),
            lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"lineage"),
            request_root_sha256: request_root,
            projection: RuntimeProjectionV3::Responses,
            streaming: true,
            observed_at_unix_ms: sequence,
        })
        .expect("receipt");
        let structure = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
            client_intent_id_sha256: sha256_bytes(format!("turn-{sequence}").as_bytes()),
            session_identity_sha256s: vec![sha256_bytes(b"session")],
            request_phase_atom_ids: vec![sequence],
            pre_action_context_atom_ids: vec![sequence.saturating_add(100)],
            capability_atom_ids: vec![7],
            provider_bound_turn_identity: true,
            estimated_input_tokens: 1,
            provider_payload_bytes: u64::try_from(payload.len()).expect("payload bytes"),
        })
        .expect("structure");
        (receipt, structure)
    }

    fn topology(
        receipt: &ProviderRequestCaptureReceiptV3,
        v1: &LearningRequestStructureV1,
    ) -> (LearningRequestStructureV2, PreActionTopologyCommitV1) {
        let v2 = LearningRequestStructureV2 {
            schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
            turn_intent_id_sha256: v1.client_intent_id_sha256().to_owned(),
            request_event_id_sha256: sha256_bytes(
                format!("event-{}", receipt.capture_sequence()).as_bytes(),
            ),
            provider_bound_turn_identity: true,
            session_lineage_roots_sha256: v1.session_identity_sha256s().to_vec(),
            request_phase_atom_ids: v1.request_phase_atom_ids().to_vec(),
            pre_action_context_atom_ids: v1.pre_action_context_atom_ids().to_vec(),
            capability_atom_ids: v1.capability_atom_ids().to_vec(),
            estimated_input_tokens: v1.estimated_input_tokens(),
            provider_payload_bytes: v1.provider_payload_bytes(),
            provider_capture_request_root_sha256: receipt.request_root_sha256().to_hex(),
            decidability_reason_code: "pre_action_pending".to_owned(),
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 0,
                output_part_count: 0,
                roles: Vec::new(),
                role_witnesses: Vec::new(),
                relations: Vec::new(),
            },
        };
        let commit = PreActionTopologyCommitV1::seal(
            &v2,
            MultiSourceEvidenceOriginV1::FreshLive,
            sha256_bytes(b"extractor"),
            sha256_bytes(b"config"),
            receipt.capture_sequence(),
        )
        .expect("commit");
        (v2, commit)
    }

    #[test]
    fn v3_record_restarts_through_the_existing_single_consumer() {
        let root = test_root("v3-restart");
        let (producer, _) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), true, false, Duration::ZERO)
                .expect("producer");
        let (receipt, v1) = evidence(1);
        let (v2, commit) = topology(&receipt, &v1);
        producer
            .submit_v3(receipt, v1, v2, commit)
            .expect("submit v3");
        let (consumer, index) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("consumer");
        drain_pending(&consumer.inner, &index).expect("drain");
        assert_eq!(index.status().structures_applied, 1);
        assert_eq!(consumer.status().consumer.last_sequence, 1);
        drop(producer);
        drop(consumer);
        let (restarted, restored) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("restart");
        assert!(restarted.status().checkpoint_restored);
        assert_eq!(restored.status().structures_applied, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn v4_checkpoint_keeps_each_commitment_when_turn_identity_repeats() {
        let root = test_root("v4-immutable-ledger");
        let (producer, _) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), true, false, Duration::ZERO)
                .expect("producer");
        let (receipt_a, v1) = evidence(1);
        let (v2_a, commit_a) = topology(&receipt_a, &v1);
        producer
            .submit_v3(receipt_a, v1.clone(), v2_a, commit_a)
            .expect("submit first");
        let (receipt_b, _) = evidence(2);
        let (v2_b, commit_b) = topology(&receipt_b, &v1);
        producer
            .submit_v3(receipt_b, v1, v2_b, commit_b)
            .expect("submit second");

        let (consumer, index) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("consumer");
        drain_pending(&consumer.inner, &index).expect("drain");
        let snapshot = index.audit_snapshot_v1().expect("snapshot");
        assert_eq!(snapshot.stored_turns, 1);
        assert_eq!(snapshot.stored_topologies, 2);
        assert_eq!(snapshot.topologies.len(), 2);
        assert!(
            snapshot
                .topologies
                .iter()
                .all(|row| row.physical_order_proven)
        );

        drop(producer);
        drop(consumer);
        let (_, restored) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("restart");
        assert_eq!(restored.status().stored_topologies, 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn durable_structure_restores_checkpoint_and_contiguous_sequence() {
        let root = test_root("restart");
        let (producer, _) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), true, false, Duration::ZERO)
                .expect("producer");
        for sequence in 1..=3 {
            let (receipt, structure) = evidence(sequence);
            producer.submit(receipt, structure).expect("submit");
        }
        assert_eq!(producer.status().pending_records, 3);

        let (consumer, index) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("consumer");
        drain_pending(&consumer.inner, &index).expect("first drain");
        assert_eq!(consumer.status().consumer.last_sequence, 3);
        assert_eq!(consumer.status().pending_records, 0);
        assert_eq!(index.status().structures_applied, 3);
        drop(producer);

        let (restarted, restored_index) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), false, true, Duration::ZERO)
                .expect("restart");
        assert!(restarted.status().checkpoint_restored);
        assert_eq!(restarted.status().consumer.last_sequence, 3);
        assert_eq!(restored_index.status().structures_applied, 3);

        let (restarted_producer, _) =
            LearningStructureBridgeRuntimeV2::open(root.clone(), true, false, Duration::ZERO)
                .expect("producer restart");
        for sequence in 4..=5 {
            let (receipt, structure) = evidence(sequence);
            restarted_producer
                .submit(receipt, structure)
                .expect("submit");
        }
        drain_pending(&restarted.inner, &restored_index).expect("second drain");
        assert_eq!(restarted.status().consumer.last_sequence, 5);
        assert_eq!(restored_index.status().structures_applied, 5);
        assert_eq!(restarted.status().sequence_gaps, 0);
        assert_eq!(restarted.status().pending_records, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
