//! Durable transport facts used by MS3 proof and diagnostics.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use nando_operator_learning::multi_source::TransportTerminalReceiptV1;
use nando_operator_learning::write_atomic_cbor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::nginx_terminal::parse_terminal_line;

const ARCHIVE_SCHEMA: &str = "nando.transport-terminal-receipt-archive.v1";
const DATA_FILE: &str = "terminal-receipts-v1.bin";
const CHECKPOINT_FILE: &str = "terminal-receipts-v1.cbor";
const RECORD_BYTES: u64 = 82;
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SOURCE_LINE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ArchiveCheckpoint {
    schema: String,
    next_sequence: u64,
    chain_root_sha256: String,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
}

pub(super) struct TerminalReceiptArchive {
    checkpoint_path: PathBuf,
    file: File,
    checkpoint: ArchiveCheckpoint,
    by_request: BTreeMap<String, TransportTerminalReceiptV1>,
}

impl TerminalReceiptArchive {
    pub(super) fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("terminal_archive_dir:{}:{error}", directory.display()))?;
        let checkpoint_path = directory.join(CHECKPOINT_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(directory.join(DATA_FILE))
            .map_err(|error| format!("terminal_archive_open:{error}"))?;
        let checkpoint = match std::fs::read(&checkpoint_path) {
            Ok(bytes) => serde_cbor::from_slice(&bytes)
                .map_err(|error| format!("terminal_archive_checkpoint_decode:{error}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ArchiveCheckpoint {
                schema: ARCHIVE_SCHEMA.to_owned(),
                next_sequence: 0,
                chain_root_sha256: initial_root(),
                source_device: 0,
                source_inode: 0,
                source_offset: 0,
            },
            Err(error) => return Err(format!("terminal_archive_checkpoint_read:{error}")),
        };
        let by_request = validate_chain(&mut file, &checkpoint)?;
        let committed_bytes = checkpoint.next_sequence.saturating_mul(RECORD_BYTES);
        if file
            .metadata()
            .map_err(|error| format!("terminal_archive_metadata:{error}"))?
            .len()
            > committed_bytes
        {
            file.set_len(committed_bytes)
                .map_err(|error| format!("terminal_archive_recover_tail:{error}"))?;
        }
        file.seek(SeekFrom::End(0))
            .map_err(|error| format!("terminal_archive_seek:{error}"))?;
        Ok(Self {
            checkpoint_path,
            file,
            checkpoint,
            by_request,
        })
    }

    pub(super) fn sync_source(&mut self, source_path: &Path) -> Result<(), String> {
        let mut source = File::open(source_path)
            .map_err(|error| format!("terminal_archive_source_open:{error}"))?;
        let metadata = source
            .metadata()
            .map_err(|error| format!("terminal_archive_source_metadata:{error}"))?;
        #[cfg(unix)]
        let (device, inode) = (metadata.dev(), metadata.ino());
        #[cfg(not(unix))]
        let (device, inode) = (0, 0);
        if self.checkpoint.source_device != device
            || self.checkpoint.source_inode != inode
            || metadata.len() < self.checkpoint.source_offset
        {
            self.checkpoint.source_device = device;
            self.checkpoint.source_inode = inode;
            self.checkpoint.source_offset = 0;
        }

        source
            .seek(SeekFrom::Start(self.checkpoint.source_offset))
            .map_err(|error| format!("terminal_archive_source_seek:{error}"))?;
        let mut reader = BufReader::new(source);
        let mut committed_offset = self.checkpoint.source_offset;
        loop {
            let line_offset = committed_offset;
            let mut bytes = Vec::new();
            let read = reader
                .read_until(b'\n', &mut bytes)
                .map_err(|error| format!("terminal_archive_source_read:{error}"))?;
            if read == 0 {
                break;
            }
            if bytes.last() != Some(&b'\n') {
                committed_offset = line_offset;
                break;
            }
            committed_offset = committed_offset.saturating_add(read as u64);
            if bytes.len() > MAX_SOURCE_LINE_BYTES {
                continue;
            }
            let Ok(line) = std::str::from_utf8(&bytes) else {
                continue;
            };
            let Some(receipt) = parse_terminal_line(line.trim_end()) else {
                continue;
            };
            self.append(receipt)?;
        }
        self.checkpoint.source_offset = committed_offset;
        self.file
            .sync_data()
            .map_err(|error| format!("terminal_archive_sync:{error}"))?;
        write_atomic_cbor(&self.checkpoint_path, &self.checkpoint)
    }

    pub(super) fn receipts_for_requests(
        &self,
        request_ids: &BTreeSet<String>,
    ) -> Vec<TransportTerminalReceiptV1> {
        request_ids
            .iter()
            .filter_map(|request| self.by_request.get(request).cloned())
            .collect()
    }

    pub(super) fn receipt_for_request(
        &self,
        request_event_id_sha256: &str,
    ) -> Option<TransportTerminalReceiptV1> {
        self.by_request.get(request_event_id_sha256).cloned()
    }

    pub(super) fn len(&self) -> usize {
        self.by_request.len()
    }

    fn append(&mut self, receipt: TransportTerminalReceiptV1) -> Result<(), String> {
        if !receipt.validate() {
            return Err("terminal_archive_receipt_invalid".to_owned());
        }
        if let Some(existing) = self.by_request.get(&receipt.request_event_id_sha256) {
            return if existing == &receipt {
                Ok(())
            } else {
                Err("terminal_archive_request_rebound".to_owned())
            };
        }
        let next_bytes = self
            .checkpoint
            .next_sequence
            .saturating_add(1)
            .saturating_mul(RECORD_BYTES);
        if next_bytes > MAX_ARCHIVE_BYTES {
            return Err("terminal_archive_budget_exhausted".to_owned());
        }
        let record = encode_record(&receipt)?;
        self.file
            .write_all(&record)
            .map_err(|error| format!("terminal_archive_append:{error}"))?;
        self.checkpoint.chain_root_sha256 = next_root(&self.checkpoint.chain_root_sha256, &record);
        self.checkpoint.next_sequence = self.checkpoint.next_sequence.saturating_add(1);
        self.by_request
            .insert(receipt.request_event_id_sha256.clone(), receipt);
        Ok(())
    }
}

fn validate_chain(
    file: &mut File,
    checkpoint: &ArchiveCheckpoint,
) -> Result<BTreeMap<String, TransportTerminalReceiptV1>, String> {
    let committed_bytes = checkpoint.next_sequence.saturating_mul(RECORD_BYTES);
    if checkpoint.schema != ARCHIVE_SCHEMA
        || checkpoint.chain_root_sha256.len() != 64
        || committed_bytes > MAX_ARCHIVE_BYTES
        || file
            .metadata()
            .map_err(|error| format!("terminal_archive_metadata:{error}"))?
            .len()
            < committed_bytes
    {
        return Err("terminal_archive_checkpoint_invalid".to_owned());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("terminal_archive_seek:{error}"))?;
    let mut root = initial_root();
    let mut by_request = BTreeMap::new();
    for _ in 0..checkpoint.next_sequence {
        let mut bytes = [0_u8; RECORD_BYTES as usize];
        file.read_exact(&mut bytes)
            .map_err(|error| format!("terminal_archive_read:{error}"))?;
        let receipt = decode_record(&bytes)?;
        if by_request
            .insert(receipt.request_event_id_sha256.clone(), receipt)
            .is_some()
        {
            return Err("terminal_archive_duplicate_request".to_owned());
        }
        root = next_root(&root, &bytes);
    }
    if root != checkpoint.chain_root_sha256 {
        return Err("terminal_archive_root_mismatch".to_owned());
    }
    Ok(by_request)
}

fn encode_record(
    receipt: &TransportTerminalReceiptV1,
) -> Result<[u8; RECORD_BYTES as usize], String> {
    let mut bytes = [0_u8; RECORD_BYTES as usize];
    bytes[..32].copy_from_slice(&decode_digest(&receipt.request_event_id_sha256)?);
    bytes[32..64].copy_from_slice(&decode_digest(&receipt.receipt_root_sha256)?);
    bytes[64..72].copy_from_slice(&receipt.started_at_unix_nanos.to_le_bytes());
    bytes[72..80].copy_from_slice(&receipt.completed_at_unix_nanos.to_le_bytes());
    bytes[80..82].copy_from_slice(&receipt.status.to_le_bytes());
    Ok(bytes)
}

fn decode_record(
    bytes: &[u8; RECORD_BYTES as usize],
) -> Result<TransportTerminalReceiptV1, String> {
    let request = hex_digest(&bytes[..32]);
    let expected_root = hex_digest(&bytes[32..64]);
    let receipt = TransportTerminalReceiptV1::seal(
        request,
        u64::from_le_bytes(bytes[64..72].try_into().unwrap_or_default()),
        u64::from_le_bytes(bytes[72..80].try_into().unwrap_or_default()),
        u16::from_le_bytes(bytes[80..82].try_into().unwrap_or_default()),
    )
    .map_err(str::to_owned)?;
    if receipt.receipt_root_sha256 != expected_root {
        return Err("terminal_archive_record_digest_mismatch".to_owned());
    }
    Ok(receipt)
}

fn initial_root() -> String {
    format!("{:x}", Sha256::digest(ARCHIVE_SCHEMA.as_bytes()))
}

fn next_root(previous: &str, record: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.transport-terminal-receipt-archive-link.v1");
    hasher.update(previous.as_bytes());
    hasher.update(record);
    format!("{:x}", hasher.finalize())
}

fn decode_digest(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("terminal_archive_digest_invalid".to_owned());
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
        _ => Err("terminal_archive_digest_invalid".to_owned()),
    }
}

#[cfg(test)]
#[path = "terminal_receipt_archive_tests.rs"]
mod tests;
