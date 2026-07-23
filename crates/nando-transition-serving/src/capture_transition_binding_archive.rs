//! Capture-owned writer for immutable frame-to-receipt bindings.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    CaptureEvidenceReceipt, CaptureTransitionBinding, write_atomic_cbor,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ARCHIVE_SCHEMA: &str = "nando.capture-transition-binding-archive.v1";
const DATA_FILE: &str = "capture-transition-binding-archive-v1.bin";
const CHECKPOINT_FILE: &str = "capture-transition-binding-archive-v1.cbor";
const RECORD_BYTES: u64 = 144;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct BindingArchiveCheckpoint {
    schema: String,
    next_sequence: u64,
    chain_root_sha256: String,
}

pub(crate) struct CaptureTransitionBindingArchive {
    checkpoint_path: PathBuf,
    file: File,
    checkpoint: BindingArchiveCheckpoint,
    by_frame: BTreeMap<String, CaptureTransitionBinding>,
}

impl CaptureTransitionBindingArchive {
    pub(crate) fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("capture_binding_archive_dir:{error}"))?;
        let checkpoint_path = directory.join(CHECKPOINT_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(directory.join(DATA_FILE))
            .map_err(|error| format!("capture_binding_archive_open:{error}"))?;
        let checkpoint = match std::fs::read(&checkpoint_path) {
            Ok(bytes) => serde_cbor::from_slice(&bytes)
                .map_err(|error| format!("capture_binding_checkpoint_decode:{error}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                BindingArchiveCheckpoint {
                    schema: ARCHIVE_SCHEMA.to_owned(),
                    next_sequence: 0,
                    chain_root_sha256: initial_root(),
                }
            }
            Err(error) => return Err(format!("capture_binding_checkpoint_read:{error}")),
        };
        let by_frame = validate_chain(&mut file, &checkpoint)?;
        let committed_bytes = checkpoint.next_sequence.saturating_mul(RECORD_BYTES);
        if file
            .metadata()
            .map_err(|error| format!("capture_binding_archive_metadata:{error}"))?
            .len()
            > committed_bytes
        {
            file.set_len(committed_bytes)
                .map_err(|error| format!("capture_binding_archive_recover_tail:{error}"))?;
        }
        file.seek(SeekFrom::End(0))
            .map_err(|error| format!("capture_binding_archive_seek:{error}"))?;
        Ok(Self {
            checkpoint_path,
            file,
            checkpoint,
            by_frame,
        })
    }

    pub(crate) fn append(
        &mut self,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<CaptureTransitionBinding, String> {
        receipt.validate().map_err(str::to_owned)?;
        if let Some(existing) = self.by_frame.get(frame_id_sha256) {
            if existing.records_root_sha256 != receipt.records_root_sha256
                || !receipt.records.contains(&existing.source_record)
            {
                return Err("capture_transition_binding_frame_rebound".to_owned());
            }
            return Ok(existing.clone());
        }
        let sequence = self.checkpoint.next_sequence;
        let binding = CaptureTransitionBinding::new(sequence, frame_id_sha256, receipt)
            .map_err(str::to_owned)?;
        self.file
            .write_all(&encode_record(&binding)?)
            .map_err(|error| format!("capture_binding_archive_append:{error}"))?;
        self.checkpoint.chain_root_sha256 =
            next_root(&self.checkpoint.chain_root_sha256, &binding.record_sha256)?;
        self.checkpoint.next_sequence = self.checkpoint.next_sequence.saturating_add(1);
        self.by_frame
            .insert(frame_id_sha256.to_owned(), binding.clone());
        Ok(binding)
    }

    pub(crate) fn seal(&mut self) -> Result<(), String> {
        self.file
            .sync_data()
            .map_err(|error| format!("capture_binding_archive_sync:{error}"))?;
        write_atomic_cbor(&self.checkpoint_path, &self.checkpoint)
    }
}

fn validate_chain(
    file: &mut File,
    checkpoint: &BindingArchiveCheckpoint,
) -> Result<BTreeMap<String, CaptureTransitionBinding>, String> {
    if checkpoint.schema != ARCHIVE_SCHEMA
        || file
            .metadata()
            .map_err(|error| format!("capture_binding_archive_metadata:{error}"))?
            .len()
            < checkpoint.next_sequence.saturating_mul(RECORD_BYTES)
    {
        return Err("capture_binding_checkpoint_invalid".to_owned());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("capture_binding_archive_seek:{error}"))?;
    let mut root = initial_root();
    let mut by_frame = BTreeMap::new();
    for sequence in 0..checkpoint.next_sequence {
        let mut bytes = [0_u8; RECORD_BYTES as usize];
        file.read_exact(&mut bytes)
            .map_err(|error| format!("capture_binding_archive_read:{error}"))?;
        if u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default()) != sequence {
            return Err("capture_binding_archive_sequence_mismatch".to_owned());
        }
        let binding = decode_record(&bytes)?;
        binding.verify_digest().map_err(str::to_owned)?;
        if by_frame
            .insert(binding.frame_id_sha256.clone(), binding.clone())
            .is_some()
        {
            return Err("capture_binding_archive_duplicate_frame".to_owned());
        }
        root = next_root(&root, &binding.record_sha256)?;
    }
    if root != checkpoint.chain_root_sha256 {
        return Err("capture_binding_archive_root_mismatch".to_owned());
    }
    Ok(by_frame)
}

fn decode_record(bytes: &[u8; RECORD_BYTES as usize]) -> Result<CaptureTransitionBinding, String> {
    use nando_operator_learning::{CAPTURE_TRANSITION_BINDING_SCHEMA_V1, CaptureRecordCommitment};

    Ok(CaptureTransitionBinding {
        schema: CAPTURE_TRANSITION_BINDING_SCHEMA_V1.to_owned(),
        sequence: u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default()),
        frame_id_sha256: hex_digest(&bytes[8..40]),
        records_root_sha256: hex_digest(&bytes[40..72]),
        source_record: CaptureRecordCommitment {
            sequence: u64::from_le_bytes(bytes[72..80].try_into().unwrap_or_default()),
            record_sha256: hex_digest(&bytes[80..112]),
        },
        record_sha256: hex_digest(&bytes[112..144]),
    })
}

fn encode_record(
    binding: &CaptureTransitionBinding,
) -> Result<[u8; RECORD_BYTES as usize], String> {
    let mut output = [0_u8; RECORD_BYTES as usize];
    output[..8].copy_from_slice(&binding.sequence.to_le_bytes());
    output[8..40].copy_from_slice(&decode_digest(&binding.frame_id_sha256)?);
    output[40..72].copy_from_slice(&decode_digest(&binding.records_root_sha256)?);
    output[72..80].copy_from_slice(&binding.source_record.sequence.to_le_bytes());
    output[80..112].copy_from_slice(&decode_digest(&binding.source_record.record_sha256)?);
    output[112..144].copy_from_slice(&decode_digest(&binding.record_sha256)?);
    Ok(output)
}

fn initial_root() -> String {
    format!("{:x}", Sha256::digest(ARCHIVE_SCHEMA.as_bytes()))
}

fn next_root(previous: &str, record_sha256: &str) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.capture-transition-binding-archive-link.v1");
    hasher.update(decode_digest(previous)?);
    hasher.update(decode_digest(record_sha256)?);
    Ok(format!("{:x}", hasher.finalize()))
}

fn decode_digest(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("capture_binding_digest_invalid".to_owned());
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(output)
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err("capture_binding_digest_invalid".to_owned()),
    }
}

#[cfg(test)]
#[path = "capture_transition_binding_archive_tests.rs"]
mod tests;
