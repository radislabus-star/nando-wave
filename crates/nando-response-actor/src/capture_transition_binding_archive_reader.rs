//! Read-only verification of capture-owned transition bindings.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use nando_operator_learning::{
    CAPTURE_TRANSITION_BINDING_SCHEMA_V1, CaptureEvidenceReceipt, CaptureRecordCommitment,
    CaptureTransitionBinding,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const ARCHIVE_SCHEMA: &str = "nando.capture-transition-binding-archive.v1";
const DATA_FILE: &str = "capture-transition-binding-archive-v1.bin";
const CHECKPOINT_FILE: &str = "capture-transition-binding-archive-v1.cbor";
const RECORD_BYTES: u64 = 144;

#[derive(Deserialize)]
struct BindingArchiveCheckpoint {
    schema: String,
    next_sequence: u64,
    chain_root_sha256: String,
}

pub struct CaptureTransitionBindingArchiveReader {
    file: File,
    checkpoint: BindingArchiveCheckpoint,
}

impl CaptureTransitionBindingArchiveReader {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(directory.join(CHECKPOINT_FILE))
            .map_err(|error| format!("capture_binding_checkpoint_read:{error}"))?;
        let checkpoint: BindingArchiveCheckpoint = serde_cbor::from_slice(&bytes)
            .map_err(|error| format!("capture_binding_checkpoint_decode:{error}"))?;
        let mut file = File::open(directory.join(DATA_FILE))
            .map_err(|error| format!("capture_binding_archive_read_open:{error}"))?;
        validate_chain(&mut file, &checkpoint)?;
        Ok(Self { file, checkpoint })
    }

    pub fn verify(
        &mut self,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<(), String> {
        let binding = receipt
            .transition_binding
            .as_ref()
            .ok_or_else(|| "capture_transition_binding_missing".to_owned())?;
        if binding.frame_id_sha256 != frame_id_sha256 {
            return Err("capture_transition_binding_mismatch".to_owned());
        }
        self.verify_receipt(receipt)
    }

    pub fn verify_receipt(&mut self, receipt: &CaptureEvidenceReceipt) -> Result<(), String> {
        receipt.validate().map_err(str::to_owned)?;
        let binding = receipt
            .transition_binding
            .as_ref()
            .ok_or_else(|| "capture_transition_binding_missing".to_owned())?;
        if binding.sequence >= self.checkpoint.next_sequence {
            return Err("capture_transition_binding_mismatch".to_owned());
        }
        self.file
            .seek(SeekFrom::Start(
                binding.sequence.saturating_mul(RECORD_BYTES),
            ))
            .map_err(|error| format!("capture_binding_archive_verify_seek:{error}"))?;
        let archived = read_record(&mut self.file)?;
        if archived != *binding {
            return Err("capture_transition_binding_archive_mismatch".to_owned());
        }
        Ok(())
    }
}

fn validate_chain(file: &mut File, checkpoint: &BindingArchiveCheckpoint) -> Result<(), String> {
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
    let mut frames = BTreeSet::new();
    for sequence in 0..checkpoint.next_sequence {
        let binding = read_record(file)?;
        if binding.sequence != sequence {
            return Err("capture_binding_archive_sequence_mismatch".to_owned());
        }
        binding.verify_digest().map_err(str::to_owned)?;
        if !frames.insert(binding.frame_id_sha256.clone()) {
            return Err("capture_binding_archive_duplicate_frame".to_owned());
        }
        root = next_root(&root, &binding.record_sha256)?;
    }
    if root != checkpoint.chain_root_sha256 {
        return Err("capture_binding_archive_root_mismatch".to_owned());
    }
    Ok(())
}

fn read_record(file: &mut File) -> Result<CaptureTransitionBinding, String> {
    let mut bytes = [0_u8; RECORD_BYTES as usize];
    file.read_exact(&mut bytes)
        .map_err(|error| format!("capture_binding_archive_read:{error}"))?;
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
