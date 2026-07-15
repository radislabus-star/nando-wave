use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

const SEGMENT_MAGIC: &[u8; 4] = b"NTF1";
const FRAME_HEADER_BYTES: u64 = 12;
const DEFAULT_SEGMENT_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_SYNC_EVERY_RECORDS: u32 = 64;
const MAX_FRAME_PAYLOAD_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FramedRecordRef {
    pub segment_id: u64,
    pub offset: u64,
    pub payload_bytes: u32,
    pub payload_digest64: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FramedLedgerStatus {
    pub active_segment_id: u64,
    pub active_segment_bytes: u64,
    pub records_appended: u64,
    pub recovered_tail_bytes: u64,
    pub fsync_count: u64,
}

pub struct FramedCborLedger {
    directory: PathBuf,
    prefix: String,
    max_segment_bytes: u64,
    sync_every_records: u32,
    file: File,
    status: FramedLedgerStatus,
    unsynced_records: u32,
}

impl FramedCborLedger {
    pub fn open(directory: &Path, prefix: &str) -> Result<Self, String> {
        Self::open_with_limits(
            directory,
            prefix,
            DEFAULT_SEGMENT_BYTES,
            DEFAULT_SYNC_EVERY_RECORDS,
        )
    }

    pub fn open_with_limits(
        directory: &Path,
        prefix: &str,
        max_segment_bytes: u64,
        sync_every_records: u32,
    ) -> Result<Self, String> {
        if prefix.is_empty()
            || !prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err("framed_ledger_invalid_prefix".to_owned());
        }
        fs::create_dir_all(directory)
            .map_err(|error| format!("framed_ledger_dir:{}:{error}", directory.display()))?;
        let segment_id = latest_segment_id(directory, prefix)?.unwrap_or(0);
        let path = segment_path(directory, prefix, segment_id);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| format!("framed_ledger_open:{}:{error}", path.display()))?;
        #[cfg(unix)]
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("framed_ledger_permissions:{}:{error}", path.display()))?;
        let recovered_tail_bytes = recover_segment_tail(&mut file)?;
        let bytes = file
            .metadata()
            .map_err(|error| format!("framed_ledger_metadata:{error}"))?
            .len();
        file.seek(SeekFrom::End(0))
            .map_err(|error| format!("framed_ledger_seek_end:{error}"))?;
        Ok(Self {
            directory: directory.to_owned(),
            prefix: prefix.to_owned(),
            max_segment_bytes: max_segment_bytes.max(1024 * 1024),
            sync_every_records: sync_every_records.max(1),
            file,
            status: FramedLedgerStatus {
                active_segment_id: segment_id,
                active_segment_bytes: bytes,
                recovered_tail_bytes,
                ..FramedLedgerStatus::default()
            },
            unsynced_records: 0,
        })
    }

    pub fn append<T: Serialize>(&mut self, value: &T) -> Result<FramedRecordRef, String> {
        let payload =
            serde_cbor::to_vec(value).map_err(|error| format!("framed_ledger_encode:{error}"))?;
        let payload_bytes = u32::try_from(payload.len())
            .map_err(|_| "framed_ledger_record_too_large".to_owned())?;
        if payload_bytes > MAX_FRAME_PAYLOAD_BYTES {
            return Err("framed_ledger_record_too_large".to_owned());
        }
        let record_bytes = FRAME_HEADER_BYTES.saturating_add(u64::from(payload_bytes));
        if self.status.active_segment_bytes > u64::from(SEGMENT_MAGIC.len() as u32)
            && self
                .status
                .active_segment_bytes
                .saturating_add(record_bytes)
                > self.max_segment_bytes
        {
            self.rotate()?;
        }
        let offset = self.status.active_segment_bytes;
        let payload_digest64 = digest64(&payload);
        self.file
            .write_all(&payload_bytes.to_le_bytes())
            .and_then(|()| self.file.write_all(&payload_digest64.to_le_bytes()))
            .and_then(|()| self.file.write_all(&payload))
            .map_err(|error| format!("framed_ledger_append:{error}"))?;
        self.status.active_segment_bytes = self
            .status
            .active_segment_bytes
            .saturating_add(record_bytes);
        self.status.records_appended = self.status.records_appended.saturating_add(1);
        self.unsynced_records = self.unsynced_records.saturating_add(1);
        if self.unsynced_records >= self.sync_every_records {
            self.sync()?;
        }
        Ok(FramedRecordRef {
            segment_id: self.status.active_segment_id,
            offset,
            payload_bytes,
            payload_digest64,
        })
    }

    pub fn sync(&mut self) -> Result<(), String> {
        if self.unsynced_records == 0 {
            return Ok(());
        }
        self.file
            .sync_data()
            .map_err(|error| format!("framed_ledger_sync:{error}"))?;
        self.unsynced_records = 0;
        self.status.fsync_count = self.status.fsync_count.saturating_add(1);
        Ok(())
    }

    /// Seals all records already represented by an external atomic state
    /// checkpoint. Call only after that checkpoint has been durably renamed.
    pub fn compact_after_checkpoint(&mut self) -> Result<(), String> {
        self.sync()?;
        let previous_segment = self.status.active_segment_id;
        if self.status.active_segment_bytes <= u64::try_from(SEGMENT_MAGIC.len()).unwrap_or(4) {
            return Ok(());
        }
        self.rotate()?;
        for segment_id in segment_ids(&self.directory, &self.prefix)? {
            if segment_id <= previous_segment {
                let path = segment_path(&self.directory, &self.prefix, segment_id);
                fs::remove_file(&path).map_err(|error| {
                    format!("framed_ledger_compact_remove:{}:{error}", path.display())
                })?;
            }
        }
        sync_directory(&self.directory)
    }

    #[must_use]
    pub fn status(&self) -> FramedLedgerStatus {
        self.status.clone()
    }

    fn rotate(&mut self) -> Result<(), String> {
        self.sync()?;
        self.status.active_segment_id = self.status.active_segment_id.saturating_add(1);
        let path = segment_path(&self.directory, &self.prefix, self.status.active_segment_id);
        self.file = create_segment(&path)?;
        self.status.active_segment_bytes = u64::try_from(SEGMENT_MAGIC.len()).unwrap_or(4);
        sync_directory(&self.directory)?;
        Ok(())
    }
}

impl Drop for FramedCborLedger {
    fn drop(&mut self) {
        let _ = self.sync();
    }
}

pub fn read_framed_cbor<T: DeserializeOwned>(
    directory: &Path,
    prefix: &str,
) -> Result<Vec<T>, String> {
    let mut output = Vec::new();
    for segment_id in segment_ids(directory, prefix)? {
        let path = segment_path(directory, prefix, segment_id);
        let mut file = File::open(&path)
            .map_err(|error| format!("framed_ledger_read_open:{}:{error}", path.display()))?;
        let mut magic = [0_u8; 4];
        file.read_exact(&mut magic)
            .map_err(|error| format!("framed_ledger_read_magic:{error}"))?;
        if &magic != SEGMENT_MAGIC {
            return Err("framed_ledger_bad_magic".to_owned());
        }
        loop {
            let mut length = [0_u8; 4];
            match file.read_exact(&mut length) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(format!("framed_ledger_read_length:{error}")),
            }
            let payload_bytes = u32::from_le_bytes(length);
            if payload_bytes > MAX_FRAME_PAYLOAD_BYTES {
                return Err("framed_ledger_record_too_large".to_owned());
            }
            let payload_bytes = usize::try_from(payload_bytes)
                .map_err(|_| "framed_ledger_record_too_large".to_owned())?;
            let mut expected_digest = [0_u8; 8];
            file.read_exact(&mut expected_digest)
                .map_err(|error| format!("framed_ledger_read_digest:{error}"))?;
            let mut payload = vec![0_u8; payload_bytes];
            file.read_exact(&mut payload)
                .map_err(|error| format!("framed_ledger_read_payload:{error}"))?;
            if digest64(&payload) != u64::from_le_bytes(expected_digest) {
                return Err("framed_ledger_digest_mismatch".to_owned());
            }
            output.push(
                serde_cbor::from_slice(&payload)
                    .map_err(|error| format!("framed_ledger_decode:{error}"))?,
            );
        }
    }
    Ok(output)
}

pub fn write_atomic_cbor<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "atomic_checkpoint_parent_missing".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("atomic_checkpoint_dir:{}:{error}", parent.display()))?;
    let temporary = path.with_extension("tmp");
    let bytes =
        serde_cbor::to_vec(value).map_err(|error| format!("atomic_checkpoint_encode:{error}"))?;
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| format!("atomic_checkpoint_create:{}:{error}", temporary.display()))?;
        file.write_all(&bytes)
            .map_err(|error| format!("atomic_checkpoint_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("atomic_checkpoint_sync:{error}"))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("atomic_checkpoint_rename:{}:{error}", path.display()))?;
    sync_directory(parent)
}

fn create_segment(path: &Path) -> Result<File, String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("framed_ledger_create:{}:{error}", path.display()))?;
    file.write_all(SEGMENT_MAGIC)
        .map_err(|error| format!("framed_ledger_write_magic:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("framed_ledger_sync_magic:{error}"))?;
    Ok(file)
}

fn recover_segment_tail(file: &mut File) -> Result<u64, String> {
    let length = file
        .metadata()
        .map_err(|error| format!("framed_ledger_recover_metadata:{error}"))?
        .len();
    if length == 0 {
        file.write_all(SEGMENT_MAGIC)
            .map_err(|error| format!("framed_ledger_initialize:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("framed_ledger_initialize_sync:{error}"))?;
        return Ok(0);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("framed_ledger_recover_seek:{error}"))?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)
        .map_err(|error| format!("framed_ledger_recover_magic:{error}"))?;
    if &magic != SEGMENT_MAGIC {
        return Err("framed_ledger_bad_magic".to_owned());
    }
    let mut valid_end = u64::try_from(SEGMENT_MAGIC.len()).unwrap_or(4);
    loop {
        let record_start = valid_end;
        file.seek(SeekFrom::Start(record_start))
            .map_err(|error| format!("framed_ledger_recover_record_seek:{error}"))?;
        let mut header = [0_u8; 12];
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(format!("framed_ledger_recover_header:{error}")),
        }
        let payload_bytes = u32::from_le_bytes(header[..4].try_into().unwrap_or([0; 4]));
        if payload_bytes > MAX_FRAME_PAYLOAD_BYTES {
            break;
        }
        let Ok(payload_bytes) = usize::try_from(payload_bytes) else {
            break;
        };
        let expected_digest = u64::from_le_bytes(header[4..12].try_into().unwrap_or([0; 8]));
        let mut payload = vec![0_u8; payload_bytes];
        if file.read_exact(&mut payload).is_err() || digest64(&payload) != expected_digest {
            break;
        }
        valid_end = record_start
            .saturating_add(FRAME_HEADER_BYTES)
            .saturating_add(u64::try_from(payload_bytes).unwrap_or(u64::MAX));
    }
    let recovered = length.saturating_sub(valid_end);
    if recovered > 0 {
        file.set_len(valid_end)
            .map_err(|error| format!("framed_ledger_recover_truncate:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("framed_ledger_recover_sync:{error}"))?;
    }
    Ok(recovered)
}

fn latest_segment_id(directory: &Path, prefix: &str) -> Result<Option<u64>, String> {
    Ok(segment_ids(directory, prefix)?.into_iter().max())
}

fn segment_ids(directory: &Path, prefix: &str) -> Result<Vec<u64>, String> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("framed_ledger_list:{}:{error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("framed_ledger_entry:{error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let expected_prefix = format!("{prefix}-");
        let Some(id) = name
            .strip_prefix(&expected_prefix)
            .and_then(|value| value.strip_suffix(".cbor"))
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        ids.push(id);
    }
    ids.sort_unstable();
    Ok(ids)
}

fn segment_path(directory: &Path, prefix: &str, segment_id: u64) -> PathBuf {
    directory.join(format!("{prefix}-{segment_id:020}.cbor"))
}

fn digest64(payload: &[u8]) -> u64 {
    let digest = Sha256::digest(payload);
    u64::from_le_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("directory_sync:{}:{error}", path.display()))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}
