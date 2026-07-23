//! Durable capture-owner commitments with bounded random-access verification.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CaptureEvidenceReceipt, CaptureRecordCommitment, write_atomic_cbor};

const SCHEMA: &str = "nando.capture-commitment-archive.v1";
const DATA_FILE: &str = "capture-commitment-archive-v1.bin";
const CHECKPOINT_FILE: &str = "capture-commitment-archive-v1.cbor";
const RECORD_BYTES: u64 = 40;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ArchiveCheckpoint {
    schema: String,
    base_sequence: u64,
    next_sequence: u64,
    chain_root_sha256: String,
}

pub struct CaptureCommitmentArchive {
    data_path: PathBuf,
    checkpoint_path: PathBuf,
    file: File,
    checkpoint: ArchiveCheckpoint,
}

pub struct CaptureCommitmentArchiveReader {
    file: File,
    checkpoint: ArchiveCheckpoint,
}

impl CaptureCommitmentArchive {
    pub fn open(directory: &Path, next_sequence_hint: u64) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("capture_archive_dir:{}:{error}", directory.display()))?;
        let data_path = directory.join(DATA_FILE);
        let checkpoint_path = directory.join(CHECKPOINT_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&data_path)
            .map_err(|error| format!("capture_archive_open:{}:{error}", data_path.display()))?;
        let checkpoint = match std::fs::read(&checkpoint_path) {
            Ok(bytes) => serde_cbor::from_slice(&bytes)
                .map_err(|error| format!("capture_archive_checkpoint_decode:{error}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ArchiveCheckpoint {
                schema: SCHEMA.to_owned(),
                base_sequence: next_sequence_hint,
                next_sequence: next_sequence_hint,
                chain_root_sha256: initial_root(next_sequence_hint),
            },
            Err(error) => return Err(format!("capture_archive_checkpoint_read:{error}")),
        };
        validate_checkpoint_chain(&mut file, &checkpoint)?;
        let committed_bytes = checkpoint
            .next_sequence
            .saturating_sub(checkpoint.base_sequence)
            .saturating_mul(RECORD_BYTES);
        if file
            .metadata()
            .map_err(|error| format!("capture_archive_metadata:{error}"))?
            .len()
            > committed_bytes
        {
            file.set_len(committed_bytes)
                .map_err(|error| format!("capture_archive_recover_tail:{error}"))?;
        }
        file.seek(SeekFrom::End(0))
            .map_err(|error| format!("capture_archive_seek:{error}"))?;
        Ok(Self {
            data_path,
            checkpoint_path,
            file,
            checkpoint,
        })
    }

    pub fn append(&mut self, record: &CaptureRecordCommitment) -> Result<(), String> {
        if record.sequence < self.checkpoint.next_sequence {
            self.verify_committed_record(record)?;
            self.file
                .seek(SeekFrom::End(0))
                .map_err(|error| format!("capture_archive_seek:{error}"))?;
            return Ok(());
        }
        if record.sequence > self.checkpoint.next_sequence {
            return Err("capture_archive_sequence_mismatch".to_owned());
        }
        let digest = decode_sha256(&record.record_sha256)?;
        self.file
            .write_all(&record.sequence.to_le_bytes())
            .and_then(|()| self.file.write_all(&digest))
            .map_err(|error| format!("capture_archive_append:{error}"))?;
        self.checkpoint.chain_root_sha256 =
            next_root(&self.checkpoint.chain_root_sha256, record.sequence, &digest)?;
        self.checkpoint.next_sequence = self.checkpoint.next_sequence.saturating_add(1);
        Ok(())
    }

    fn verify_committed_record(&mut self, record: &CaptureRecordCommitment) -> Result<(), String> {
        if record.sequence < self.checkpoint.base_sequence {
            return Err("capture_archive_record_unavailable".to_owned());
        }
        let offset = record
            .sequence
            .saturating_sub(self.checkpoint.base_sequence)
            .saturating_mul(RECORD_BYTES);
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|error| format!("capture_archive_replay_seek:{error}"))?;
        let mut bytes = [0_u8; RECORD_BYTES as usize];
        self.file
            .read_exact(&mut bytes)
            .map_err(|error| format!("capture_archive_replay_read:{error}"))?;
        let sequence = u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default());
        if sequence != record.sequence || bytes[8..] != decode_sha256(&record.record_sha256)? {
            return Err("capture_archive_replay_mismatch".to_owned());
        }
        Ok(())
    }

    pub fn seal(&mut self) -> Result<(), String> {
        self.file
            .sync_data()
            .map_err(|error| format!("capture_archive_sync:{error}"))?;
        write_atomic_cbor(&self.checkpoint_path, &self.checkpoint)?;
        Ok(())
    }

    #[must_use]
    pub fn data_path(&self) -> &Path {
        &self.data_path
    }
}

impl CaptureCommitmentArchiveReader {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let checkpoint_path = directory.join(CHECKPOINT_FILE);
        let bytes = std::fs::read(&checkpoint_path)
            .map_err(|error| format!("capture_archive_checkpoint_read:{error}"))?;
        let checkpoint: ArchiveCheckpoint = serde_cbor::from_slice(&bytes)
            .map_err(|error| format!("capture_archive_checkpoint_decode:{error}"))?;
        let file = File::open(directory.join(DATA_FILE))
            .map_err(|error| format!("capture_archive_read_open:{error}"))?;
        validate_checkpoint_metadata(&file, &checkpoint)?;
        Ok(Self { file, checkpoint })
    }

    pub fn verify_receipt(&mut self, receipt: &CaptureEvidenceReceipt) -> Result<(), String> {
        receipt.validate().map_err(str::to_owned)?;
        for record in &receipt.records {
            if record.sequence < self.checkpoint.base_sequence
                || record.sequence >= self.checkpoint.next_sequence
            {
                return Err("capture_archive_record_unavailable".to_owned());
            }
            let offset = record
                .sequence
                .saturating_sub(self.checkpoint.base_sequence)
                .saturating_mul(RECORD_BYTES);
            self.file
                .seek(SeekFrom::Start(offset))
                .map_err(|error| format!("capture_archive_verify_seek:{error}"))?;
            let mut bytes = [0_u8; RECORD_BYTES as usize];
            self.file
                .read_exact(&mut bytes)
                .map_err(|error| format!("capture_archive_verify_read:{error}"))?;
            let sequence = u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default());
            if sequence != record.sequence || bytes[8..] != decode_sha256(&record.record_sha256)? {
                return Err("capture_archive_record_mismatch".to_owned());
            }
        }
        Ok(())
    }
}

fn validate_checkpoint_metadata(file: &File, checkpoint: &ArchiveCheckpoint) -> Result<(), String> {
    if checkpoint.schema != SCHEMA || checkpoint.next_sequence < checkpoint.base_sequence {
        return Err("capture_archive_checkpoint_invalid".to_owned());
    }
    let committed = checkpoint
        .next_sequence
        .saturating_sub(checkpoint.base_sequence);
    let committed_bytes = committed.saturating_mul(RECORD_BYTES);
    if file
        .metadata()
        .map_err(|error| format!("capture_archive_metadata:{error}"))?
        .len()
        < committed_bytes
    {
        return Err("capture_archive_truncated".to_owned());
    }
    Ok(())
}

fn validate_checkpoint_chain(
    file: &mut File,
    checkpoint: &ArchiveCheckpoint,
) -> Result<(), String> {
    validate_checkpoint_metadata(file, checkpoint)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("capture_archive_seek:{error}"))?;
    let mut root = initial_root(checkpoint.base_sequence);
    for expected in checkpoint.base_sequence..checkpoint.next_sequence {
        let mut bytes = [0_u8; RECORD_BYTES as usize];
        file.read_exact(&mut bytes)
            .map_err(|error| format!("capture_archive_read:{error}"))?;
        let sequence = u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default());
        if sequence != expected {
            return Err("capture_archive_sequence_mismatch".to_owned());
        }
        let digest: &[u8; 32] = bytes[8..]
            .try_into()
            .map_err(|_| "capture_archive_record_invalid".to_owned())?;
        root = next_root(&root, sequence, digest)?;
    }
    if root != checkpoint.chain_root_sha256 {
        return Err("capture_archive_root_mismatch".to_owned());
    }
    Ok(())
}

fn initial_root(base_sequence: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.capture-commitment-archive.v1");
    hasher.update(base_sequence.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

fn next_root(previous: &str, sequence: u64, digest: &[u8; 32]) -> Result<String, String> {
    let previous = decode_sha256(previous)?;
    let mut hasher = Sha256::new();
    hasher.update(b"nando.capture-commitment-archive-link.v1");
    hasher.update(previous);
    hasher.update(sequence.to_le_bytes());
    hasher.update(digest);
    Ok(format!("{:x}", hasher.finalize()))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("capture_archive_digest_invalid".to_owned());
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err("capture_archive_digest_invalid".to_owned()),
    }
}

#[cfg(test)]
#[path = "capture_commitment_archive_tests.rs"]
mod tests;
