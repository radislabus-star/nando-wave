use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

use nando_operator_learning::{OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1, OpportunityBridgeEventV1};

#[cfg(test)]
use super::MAX_CONSUMER_INFLIGHT_EVENTS;
use super::{BridgeInner, record_event, record_timing, set_last_error};

const EVENT_SUFFIX: &str = ".cbor";
const MAX_SPOOL_FILES: u64 = 131_072;
pub(super) const MAX_SPOOL_BYTES: u64 = 1024 * 1024 * 1024;

pub(super) struct PendingBridgeEvent {
    pub(super) path: PathBuf,
    pub(super) sequence: u64,
    pub(super) event: OpportunityBridgeEventV1,
}

pub(super) fn persist_event(
    inner: &BridgeInner,
    event: &OpportunityBridgeEventV1,
) -> Result<(), String> {
    let bytes = event.canonical_cbor()?;
    let digest = event.canonical_sha256()?;
    let _guard = inner
        .persist_lock
        .lock()
        .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
    let event_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut spool_files = inner.spool_files.load(Ordering::Acquire);
    let mut spool_bytes = inner.spool_bytes.load(Ordering::Acquire);
    let mut next_bytes = spool_bytes.saturating_add(event_bytes);
    if spool_files >= MAX_SPOOL_FILES || next_bytes > MAX_SPOOL_BYTES {
        refresh_spool_counters(inner)?;
        spool_files = inner.spool_files.load(Ordering::Acquire);
        spool_bytes = inner.spool_bytes.load(Ordering::Acquire);
        next_bytes = spool_bytes.saturating_add(event_bytes);
    }
    if spool_files >= MAX_SPOOL_FILES || next_bytes > MAX_SPOOL_BYTES {
        return Err(format!(
            "opportunity_bridge_spool_capacity_exceeded:files={spool_files}:bytes={spool_bytes}"
        ));
    }
    let sequence = inner.next_sequence.fetch_add(1, Ordering::AcqRel);
    let file_name = event_file_name(sequence, &digest);
    let final_path = inner.pending_dir.join(&file_name);
    let temporary_path = inner.staging_dir.join(format!("{file_name}.tmp"));
    if final_path.exists() {
        inner.producer.duplicates.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary_path)
        .map_err(|error| format!("opportunity_bridge_temp_open:{error}"))?;
    let persisted = file
        .write_all(&bytes)
        .map_err(|error| format!("opportunity_bridge_temp_write:{error}"))
        .and_then(|()| {
            fs::rename(&temporary_path, &final_path)
                .map_err(|error| format!("opportunity_bridge_publish:{error}"))
        });
    if persisted.is_err() {
        let _ = fs::remove_file(&temporary_path);
        return persisted;
    }
    inner.spool_files.fetch_add(1, Ordering::AcqRel);
    inner.spool_bytes.fetch_add(event_bytes, Ordering::AcqRel);
    inner.pending_events.fetch_add(1, Ordering::AcqRel);
    inner.pending_bytes.fetch_add(event_bytes, Ordering::AcqRel);
    record_event(&inner.producer, event, sequence);
    inner.producer_sync_requested.store(true, Ordering::Release);
    Ok(())
}

pub(super) fn sync_pending_spool(
    directory: &Path,
    after_sequence: u64,
    through_sequence: u64,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("opportunity_bridge_pending_read_dir:{error}")),
    };
    for entry in entries {
        let path = entry
            .map_err(|error| format!("opportunity_bridge_pending_entry:{error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("cbor") {
            continue;
        }
        let Ok((sequence, _)) = pending_identity(&path) else {
            continue;
        };
        if sequence <= after_sequence || sequence > through_sequence {
            continue;
        }
        match File::open(&path).and_then(|file| file.sync_data()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("opportunity_bridge_pending_sync:{error}")),
        }
    }
    sync_directory(directory)
}

#[cfg(test)]
pub(super) fn drain_pending_once<F>(inner: &BridgeInner, mut deliver: F) -> Result<(), String>
where
    F: FnMut(OpportunityBridgeEventV1) -> Result<(), String>,
{
    loop {
        let batch = pending_batch(inner, MAX_CONSUMER_INFLIGHT_EVENTS, &BTreeSet::new())?;
        if batch.is_empty() {
            return Ok(());
        }
        let started = Instant::now();
        for pending in &batch {
            if let Err(error) = deliver(pending.event.clone()) {
                return Err(format!("opportunity_bridge_delivery:{error}"));
            }
        }
        acknowledge_pending_batch(inner, batch, started)?;
    }
}

pub(super) fn pending_batch(
    inner: &BridgeInner,
    limit: usize,
    excluded: &BTreeSet<PathBuf>,
) -> Result<Vec<PendingBridgeEvent>, String> {
    let mut pending = Vec::new();
    for path in pending_event_paths_bounded(&inner.pending_dir, limit, excluded)? {
        let (sequence, expected_digest) = match pending_identity(&path) {
            Ok(identity) => identity,
            Err(error) => {
                quarantine_event(inner, &path, &error)?;
                continue;
            }
        };
        let event = match read_pending_event(&path, &expected_digest) {
            Ok(event) => event,
            Err(error) => {
                quarantine_event(inner, &path, &error)?;
                continue;
            }
        };
        pending.push(PendingBridgeEvent {
            path,
            sequence,
            event,
        });
    }
    Ok(pending)
}

pub(super) fn acknowledge_pending_batch(
    inner: &BridgeInner,
    pending: Vec<PendingBridgeEvent>,
    started: Instant,
) -> Result<(), String> {
    if pending.is_empty() {
        return Err("opportunity_bridge_ack_batch_empty".to_owned());
    }
    let _guard = inner
        .persist_lock
        .lock()
        .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
    for row in pending {
        let bytes = row.path.metadata().map_or(0, |metadata| metadata.len());
        if let Err(error) = fs::remove_file(&row.path) {
            refresh_pending_counters(inner);
            return Err(format!("opportunity_bridge_ack_remove:{error}"));
        }
        saturating_atomic_sub(&inner.spool_files, 1);
        saturating_atomic_sub(&inner.spool_bytes, bytes);
        saturating_atomic_sub(&inner.pending_events, 1);
        saturating_atomic_sub(&inner.pending_bytes, bytes);
        // Count only an event whose spool file was actually removed. If a
        // later unlink fails, the durable worker ledger remains authoritative
        // and the remaining suffix is retried without losing this prefix.
        record_event(&inner.consumer, &row.event, row.sequence);
    }
    if let Err(error) = sync_directory(&inner.pending_dir) {
        refresh_pending_counters(inner);
        return Err(error);
    }
    record_timing(&inner.consumer, started);
    Ok(())
}

fn read_pending_event(
    path: &Path,
    expected_digest: &str,
) -> Result<OpportunityBridgeEventV1, String> {
    let mut file =
        File::open(path).map_err(|error| format!("opportunity_bridge_event_open:{error}"))?;
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(u64::try_from(OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1 + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("opportunity_bridge_event_read:{error}"))?;
    let event = OpportunityBridgeEventV1::from_canonical_cbor(&bytes)?;
    if event.canonical_sha256()? != expected_digest {
        return Err("opportunity_bridge_filename_digest_mismatch".to_owned());
    }
    Ok(event)
}

fn quarantine_event(inner: &BridgeInner, path: &Path, reason: &str) -> Result<(), String> {
    let _guard = inner
        .persist_lock
        .lock()
        .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
    inner
        .consumer
        .invalid_events
        .fetch_add(1, Ordering::Relaxed);
    set_last_error(&inner.consumer, reason);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "opportunity_bridge_invalid_pending_name".to_owned())?;
    let destination = inner.rejected_dir.join(format!("{file_name}.invalid"));
    let bytes = path.metadata().map_or(0, |metadata| metadata.len());
    fs::rename(path, destination)
        .map_err(|error| format!("opportunity_bridge_quarantine:{error}"))?;
    saturating_atomic_sub(&inner.pending_events, 1);
    saturating_atomic_sub(&inner.pending_bytes, bytes);
    sync_directory(&inner.pending_dir)?;
    sync_directory(&inner.rejected_dir)
}

pub(super) fn recover_temporary_events(
    staging_dir: &Path,
    pending_dir: &Path,
    rejected_dir: &Path,
) -> Result<(), String> {
    let entries = match fs::read_dir(staging_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("opportunity_bridge_recover_read_dir:{error}")),
    };
    for entry in entries {
        let path = entry
            .map_err(|error| format!("opportunity_bridge_recover_entry:{error}"))?
            .path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(final_name) = name.strip_suffix(".tmp") else {
            continue;
        };
        if !final_name.ends_with(EVENT_SUFFIX) {
            continue;
        }
        let final_path = pending_dir.join(final_name);
        if final_path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("opportunity_bridge_recover_remove:{error}"))?;
            continue;
        }
        let expected_digest = final_name
            .strip_suffix(EVENT_SUFFIX)
            .and_then(|stem| stem.split_once('-').map(|(_, digest)| digest));
        if let Some(expected_digest) = expected_digest
            && read_pending_event(&path, expected_digest).is_ok()
        {
            fs::rename(&path, &final_path)
                .map_err(|error| format!("opportunity_bridge_recover_publish:{error}"))?;
            continue;
        }
        let destination = rejected_dir.join(format!("{name}.invalid"));
        fs::rename(&path, destination)
            .map_err(|error| format!("opportunity_bridge_recover_quarantine:{error}"))?;
    }
    sync_directory(staging_dir)?;
    sync_directory(pending_dir)?;
    sync_directory(rejected_dir)
}

fn pending_event_paths_bounded(
    directory: &Path,
    limit: usize,
    excluded: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("opportunity_bridge_pending_read_dir:{error}")),
    };
    let mut paths = BTreeSet::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("opportunity_bridge_pending_entry:{error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("cbor")
            && !excluded.contains(&path)
        {
            paths.insert(path);
            if paths.len() > limit {
                paths.pop_last();
            }
        }
    }
    Ok(paths.into_iter().collect())
}

fn pending_identity(path: &Path) -> Result<(u64, String), String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_suffix(EVENT_SUFFIX))
        .ok_or_else(|| "opportunity_bridge_invalid_pending_name".to_owned())?;
    let (sequence, digest) = name
        .split_once('-')
        .ok_or_else(|| "opportunity_bridge_invalid_pending_name".to_owned())?;
    if sequence.len() != 20
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err("opportunity_bridge_invalid_pending_name".to_owned());
    }
    let sequence = sequence
        .parse::<u64>()
        .map_err(|_| "opportunity_bridge_invalid_sequence".to_owned())?;
    Ok((sequence, digest.to_owned()))
}

pub(super) fn next_pending_sequence(directories: &[&Path]) -> Result<u64, String> {
    let mut maximum = 0_u64;
    for directory in directories {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("opportunity_bridge_sequence_read_dir:{error}")),
        };
        for entry in entries {
            let path = entry
                .map_err(|error| format!("opportunity_bridge_sequence_entry:{error}"))?
                .path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let name = name.strip_suffix(".tmp").unwrap_or(name);
            let Some(stem) = name.strip_suffix(EVENT_SUFFIX) else {
                continue;
            };
            let Some((sequence, _)) = stem.split_once('-') else {
                continue;
            };
            if let Ok(sequence) = sequence.parse::<u64>() {
                maximum = maximum.max(sequence);
            }
        }
    }
    Ok(maximum.saturating_add(1).max(1))
}

pub(super) fn first_pending_sequence(directory: &Path) -> Result<Option<u64>, String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("opportunity_bridge_sequence_read_dir:{error}")),
    };
    let mut minimum = None::<u64>;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("opportunity_bridge_sequence_entry:{error}"))?
            .path();
        let Ok((sequence, _)) = pending_identity(&path) else {
            continue;
        };
        minimum = Some(minimum.map_or(sequence, |current| current.min(sequence)));
    }
    Ok(minimum)
}

pub(super) fn event_file_name(sequence: u64, digest: &str) -> String {
    format!("{sequence:020}-{digest}{EVENT_SUFFIX}")
}

pub(super) fn pending_stats(directory: &Path) -> (u64, u64) {
    let Ok(entries) = fs::read_dir(directory) else {
        return (0, 0);
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("cbor"))
        .fold((0_u64, 0_u64), |(count, bytes), path| {
            (
                count.saturating_add(1),
                bytes.saturating_add(path.metadata().map_or(0, |metadata| metadata.len())),
            )
        })
}

pub(super) fn spool_stats(directories: &[&Path]) -> Result<(u64, u64), String> {
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    for directory in directories {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("opportunity_bridge_spool_read_dir:{error}")),
        };
        for entry in entries {
            let entry = entry.map_err(|error| format!("opportunity_bridge_spool_entry:{error}"))?;
            let metadata = entry
                .metadata()
                .map_err(|error| format!("opportunity_bridge_spool_metadata:{error}"))?;
            if metadata.is_file() {
                files = files.saturating_add(1);
                bytes = bytes.saturating_add(metadata.len());
                if files > MAX_SPOOL_FILES || bytes > MAX_SPOOL_BYTES {
                    return Ok((files, bytes));
                }
            }
        }
    }
    Ok((files, bytes))
}

pub(super) fn refresh_spool_counters(inner: &BridgeInner) -> Result<(), String> {
    let (spool_files, spool_bytes) =
        spool_stats(&[&inner.staging_dir, &inner.pending_dir, &inner.rejected_dir])?;
    let (pending_events, pending_bytes) = pending_stats(&inner.pending_dir);
    inner.spool_files.store(spool_files, Ordering::Release);
    inner.spool_bytes.store(spool_bytes, Ordering::Release);
    inner
        .pending_events
        .store(pending_events, Ordering::Release);
    inner.pending_bytes.store(pending_bytes, Ordering::Release);
    Ok(())
}

pub(super) fn refresh_pending_counters(inner: &BridgeInner) {
    let (pending_events, pending_bytes) = pending_stats(&inner.pending_dir);
    inner
        .pending_events
        .store(pending_events, Ordering::Release);
    inner.pending_bytes.store(pending_bytes, Ordering::Release);
}

fn saturating_atomic_sub(value: &std::sync::atomic::AtomicU64, amount: u64) {
    let _ = value.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        Some(current.saturating_sub(amount))
    });
}

pub(super) fn create_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("opportunity_bridge_create_dir:{}:{error}", path.display()))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        format!(
            "opportunity_bridge_directory_permissions:{}:{error}",
            path.display()
        )
    })?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!(
                "opportunity_bridge_directory_sync:{}:{error}",
                path.display()
            )
        })
}
