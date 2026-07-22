use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

use nando_operator_learning::{OPPORTUNITY_BRIDGE_MAX_EVENT_BYTES_V1, OpportunityBridgeEventV1};

use super::{BridgeInner, record_event, record_timing, set_last_error};

const EVENT_SUFFIX: &str = ".cbor";

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
    record_event(&inner.producer, event, sequence);
    inner.producer_sync_requested.store(true, Ordering::Release);
    Ok(())
}

pub(super) fn sync_pending_spool(
    directory: &Path,
    after_sequence: u64,
    through_sequence: u64,
) -> Result<(), String> {
    for path in pending_event_paths(directory)? {
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
    for pending in pending_batch(inner)? {
        let started = Instant::now();
        if let Err(error) = deliver(pending.event.clone()) {
            return Err(format!("opportunity_bridge_delivery:{error}"));
        }
        acknowledge_pending(inner, pending, started)?;
    }
    Ok(())
}

pub(super) fn pending_batch(inner: &BridgeInner) -> Result<Vec<PendingBridgeEvent>, String> {
    let mut pending = Vec::new();
    for path in pending_event_paths(&inner.pending_dir)? {
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

pub(super) fn acknowledge_pending(
    inner: &BridgeInner,
    pending: PendingBridgeEvent,
    started: Instant,
) -> Result<(), String> {
    fs::remove_file(&pending.path)
        .map_err(|error| format!("opportunity_bridge_ack_remove:{error}"))?;
    sync_directory(&inner.pending_dir)?;
    record_event(&inner.consumer, &pending.event, pending.sequence);
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
    fs::rename(path, destination)
        .map_err(|error| format!("opportunity_bridge_quarantine:{error}"))?;
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

fn pending_event_paths(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("opportunity_bridge_pending_read_dir:{error}")),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("opportunity_bridge_pending_entry:{error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("cbor") {
            paths.push(path);
        }
    }
    paths.sort_unstable();
    Ok(paths)
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

pub(super) fn event_file_name(sequence: u64, digest: &str) -> String {
    format!("{sequence:020}-{digest}{EVENT_SUFFIX}")
}

pub(super) fn pending_stats(directory: &Path) -> (u64, u64) {
    pending_event_paths(directory).map_or((0, 0), |paths| {
        paths.iter().fold((0_u64, 0_u64), |(count, bytes), path| {
            (
                count.saturating_add(1),
                bytes.saturating_add(path.metadata().map_or(0, |metadata| metadata.len())),
            )
        })
    })
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
