//! Compact, payload-free proof that a Codex turn traversed the Nando route.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const NANDO_ROUTE_RECEIPT_SCHEMA_V1: &str = "nando.client-route-receipt.v1";
pub const DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_ROUTE_RECEIPTS: usize = 131_072;

const MAX_PROVIDER_ID_BYTES: usize = 256;
const MAX_ROUTE_RECEIPT_BYTES: usize = 2 * 1024;
const ROUTE_RECEIPT_GENESIS_DOMAIN_V1: &str = "nando.client-route-receipt-genesis.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRouteIdentityV1 {
    pub turn_intent_id_sha256: String,
    pub session_id_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NandoRouteReceiptV1 {
    pub schema: String,
    pub sequence: u64,
    pub previous_receipt_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub request_body_sha256: String,
    pub remote_status: u16,
    pub observed_at_unix_nanos: u64,
    pub receipt_root_sha256: String,
}

#[derive(Serialize)]
struct RouteReceiptDigestV1<'a> {
    schema: &'static str,
    sequence: u64,
    previous_receipt_root_sha256: &'a str,
    turn_intent_id_sha256: &'a str,
    session_id_sha256: &'a str,
    request_body_sha256: &'a str,
    remote_status: u16,
    observed_at_unix_nanos: u64,
}

pub struct NandoRouteReceiptLedger {
    path: PathBuf,
    file: File,
    max_bytes: u64,
    bytes: u64,
    last_sequence: u64,
    previous_receipt_root_sha256: String,
}

pub struct NandoRouteReceiptIndex {
    path: PathBuf,
    max_bytes: u64,
    offset: u64,
    last_sequence: u64,
    previous_receipt_root_sha256: String,
    receipts: BTreeMap<(String, String), Vec<NandoRouteReceiptV1>>,
}

impl ClientRouteIdentityV1 {
    #[must_use]
    pub fn from_payload(payload: &Value) -> Option<Self> {
        let metadata = payload.get("client_metadata")?;
        let turn_id = provider_id(metadata, "turn_id")?;
        let session_id = provider_id(metadata, "session_id")?;
        Some(Self {
            turn_intent_id_sha256: evidence_client_intent_id_sha256(turn_id),
            session_id_sha256: evidence_session_id_sha256(session_id),
        })
    }
}

impl NandoRouteReceiptV1 {
    pub fn seal(
        sequence: u64,
        previous_receipt_root_sha256: String,
        identity: &ClientRouteIdentityV1,
        request_body_sha256: String,
        remote_status: u16,
        observed_at_unix_nanos: u64,
    ) -> Result<Self, String> {
        let mut receipt = Self {
            schema: NANDO_ROUTE_RECEIPT_SCHEMA_V1.to_owned(),
            sequence,
            previous_receipt_root_sha256,
            turn_intent_id_sha256: identity.turn_intent_id_sha256.clone(),
            session_id_sha256: identity.session_id_sha256.clone(),
            request_body_sha256,
            remote_status,
            observed_at_unix_nanos,
            receipt_root_sha256: String::new(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt
            .validate()
            .then_some(receipt)
            .ok_or_else(|| "route_receipt_invalid".to_owned())
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == NANDO_ROUTE_RECEIPT_SCHEMA_V1
            && self.sequence > 0
            && self.observed_at_unix_nanos > 0
            && matches!(self.remote_status, 200 | 418)
            && [
                self.previous_receipt_root_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.session_id_sha256.as_str(),
                self.request_body_sha256.as_str(),
                self.receipt_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_sha256)
            && self
                .expected_root()
                .is_ok_and(|root| root == self.receipt_root_sha256)
    }

    fn expected_root(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(&RouteReceiptDigestV1 {
            schema: NANDO_ROUTE_RECEIPT_SCHEMA_V1,
            sequence: self.sequence,
            previous_receipt_root_sha256: &self.previous_receipt_root_sha256,
            turn_intent_id_sha256: &self.turn_intent_id_sha256,
            session_id_sha256: &self.session_id_sha256,
            request_body_sha256: &self.request_body_sha256,
            remote_status: self.remote_status,
            observed_at_unix_nanos: self.observed_at_unix_nanos,
        })
        .map_err(|error| format!("route_receipt_encode:{error}"))?;
        Ok(sha256_bytes(&bytes))
    }
}

impl NandoRouteReceiptLedger {
    pub fn open(path: &Path, max_bytes: u64) -> Result<Self, String> {
        validate_ledger_path(path, max_bytes)?;
        if let Some(parent) = path.parent() {
            let parent_existed = parent.exists();
            fs::create_dir_all(parent)
                .map_err(|error| format!("route_receipt_directory:{error}"))?;
            #[cfg(unix)]
            if !parent_existed {
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                    .map_err(|error| format!("route_receipt_directory_permissions:{error}"))?;
            }
        }
        let index = NandoRouteReceiptIndex::open(path, max_bytes)?;
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(path)
            .map_err(|error| format!("route_receipt_open:{error}"))?;
        #[cfg(unix)]
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("route_receipt_permissions:{error}"))?;
        let disk_bytes = file
            .metadata()
            .map_err(|error| format!("route_receipt_metadata:{error}"))?
            .len();
        if disk_bytes != index.offset {
            file.set_len(index.offset)
                .map_err(|error| format!("route_receipt_tail_recover:{error}"))?;
        }
        file.seek(SeekFrom::End(0))
            .map_err(|error| format!("route_receipt_seek:{error}"))?;
        Ok(Self {
            path: path.to_owned(),
            file,
            max_bytes,
            bytes: index.offset,
            last_sequence: index.last_sequence,
            previous_receipt_root_sha256: index.previous_receipt_root_sha256,
        })
    }

    pub fn append(
        &mut self,
        identity: &ClientRouteIdentityV1,
        request_body_sha256: String,
        remote_status: u16,
        observed_at_unix_nanos: u64,
    ) -> Result<NandoRouteReceiptV1, String> {
        let receipt = NandoRouteReceiptV1::seal(
            self.last_sequence.saturating_add(1),
            self.previous_receipt_root_sha256.clone(),
            identity,
            request_body_sha256,
            remote_status,
            observed_at_unix_nanos,
        )?;
        let mut bytes = serde_json::to_vec(&receipt)
            .map_err(|error| format!("route_receipt_encode:{error}"))?;
        if bytes.len() > MAX_ROUTE_RECEIPT_BYTES {
            return Err("route_receipt_record_budget".to_owned());
        }
        bytes.push(b'\n');
        let next_bytes = self
            .bytes
            .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| "route_receipt_ledger_budget".to_owned())?;
        if next_bytes > self.max_bytes {
            return Err("route_receipt_ledger_budget".to_owned());
        }
        self.file
            .write_all(&bytes)
            .and_then(|()| self.file.flush())
            .map_err(|error| format!("route_receipt_append:{error}"))?;
        self.bytes = next_bytes;
        self.last_sequence = receipt.sequence;
        self.previous_receipt_root_sha256 = receipt.receipt_root_sha256.clone();
        Ok(receipt)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl NandoRouteReceiptIndex {
    pub fn open(path: &Path, max_bytes: u64) -> Result<Self, String> {
        validate_ledger_path(path, max_bytes)?;
        let mut index = Self {
            path: path.to_owned(),
            max_bytes,
            offset: 0,
            last_sequence: 0,
            previous_receipt_root_sha256: route_receipt_genesis_root(),
            receipts: BTreeMap::new(),
        };
        index.refresh()?;
        Ok(index)
    }

    pub fn refresh(&mut self) -> Result<usize, String> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(format!("route_receipt_read_open:{error}")),
        };
        let file_bytes = file
            .metadata()
            .map_err(|error| format!("route_receipt_metadata:{error}"))?
            .len();
        if file_bytes > self.max_bytes {
            return Err("route_receipt_ledger_budget".to_owned());
        }
        if file_bytes < self.offset {
            return Err("route_receipt_ledger_truncated".to_owned());
        }
        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(self.offset))
            .map_err(|error| format!("route_receipt_seek:{error}"))?;
        let mut accepted = 0_usize;
        loop {
            let record_offset = self.offset;
            let mut line = Vec::new();
            let read = reader
                .read_until(b'\n', &mut line)
                .map_err(|error| format!("route_receipt_read:{error}"))?;
            if read == 0 {
                break;
            }
            if line.last() != Some(&b'\n') {
                self.offset = record_offset;
                break;
            }
            self.offset = self
                .offset
                .saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
            line.pop();
            if line.is_empty() || line.len() > MAX_ROUTE_RECEIPT_BYTES {
                return Err("route_receipt_record_budget".to_owned());
            }
            let receipt: NandoRouteReceiptV1 = serde_json::from_slice(&line)
                .map_err(|error| format!("route_receipt_decode:{error}"))?;
            if !receipt.validate()
                || receipt.sequence != self.last_sequence.saturating_add(1)
                || receipt.previous_receipt_root_sha256 != self.previous_receipt_root_sha256
                || serde_json::to_vec(&receipt)
                    .map_err(|error| format!("route_receipt_encode:{error}"))?
                    != line
            {
                return Err("route_receipt_chain_invalid".to_owned());
            }
            if usize::try_from(receipt.sequence)
                .map_or(true, |sequence| sequence > MAX_ROUTE_RECEIPTS)
            {
                return Err("route_receipt_index_budget".to_owned());
            }
            self.last_sequence = receipt.sequence;
            self.previous_receipt_root_sha256 = receipt.receipt_root_sha256.clone();
            self.receipts
                .entry((
                    receipt.turn_intent_id_sha256.clone(),
                    receipt.session_id_sha256.clone(),
                ))
                .or_default()
                .push(receipt);
            accepted = accepted.saturating_add(1);
        }
        Ok(accepted)
    }

    #[must_use]
    pub fn receipt_for_frame(
        &self,
        turn_intent_id_sha256: &str,
        session_id_sha256: &str,
        frame_observed_at_unix_nanos: u64,
    ) -> Option<&NandoRouteReceiptV1> {
        self.receipts
            .get(&(
                turn_intent_id_sha256.to_owned(),
                session_id_sha256.to_owned(),
            ))
            .and_then(|receipts| {
                receipts.iter().rev().find(|receipt| {
                    receipt.observed_at_unix_nanos <= frame_observed_at_unix_nanos
                        && frame_observed_at_unix_nanos > 0
                })
            })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.last_sequence).unwrap_or(usize::MAX)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}

#[must_use]
pub fn evidence_session_id_sha256(session_id: &str) -> String {
    domain_digest("nando.session-id.v1", session_id.as_bytes())
}

#[must_use]
pub fn evidence_client_intent_id_sha256(client_intent_id: &str) -> String {
    domain_digest("nando.client-intent-id.v1", client_intent_id.as_bytes())
}

#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[must_use]
pub fn route_receipt_genesis_root() -> String {
    domain_digest(ROUTE_RECEIPT_GENESIS_DOMAIN_V1, b"")
}

fn provider_id<'a>(metadata: &'a Value, key: &str) -> Option<&'a str> {
    metadata.get(key).and_then(Value::as_str).filter(|value| {
        !value.is_empty()
            && value.len() <= MAX_PROVIDER_ID_BYTES
            && value.bytes().all(|byte| !byte.is_ascii_control())
    })
}

fn domain_digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && !value.bytes().all(|byte| byte == b'0')
}

fn validate_ledger_path(path: &Path, max_bytes: u64) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.parent().is_none() {
        return Err("route_receipt_path_invalid".to_owned());
    }
    if max_bytes < 1024 * 1024 {
        return Err("route_receipt_ledger_budget".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-client-evidence-{}-{}-{label}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn identity() -> ClientRouteIdentityV1 {
        ClientRouteIdentityV1 {
            turn_intent_id_sha256: evidence_client_intent_id_sha256("turn-a"),
            session_id_sha256: evidence_session_id_sha256("session-a"),
        }
    }

    #[test]
    fn payload_identity_keeps_turn_and_session_domains_separate() {
        let parsed = ClientRouteIdentityV1::from_payload(&serde_json::json!({
            "client_metadata": {
                "turn_id": "turn-a",
                "session_id": "session-a"
            }
        }))
        .expect("identity");
        assert_eq!(parsed, identity());
        assert_ne!(parsed.turn_intent_id_sha256, parsed.session_id_sha256);
        assert!(ClientRouteIdentityV1::from_payload(&serde_json::json!({})).is_none());
    }

    #[test]
    fn receipt_rejects_unconfirmed_remote_status_and_tampering() {
        assert!(
            NandoRouteReceiptV1::seal(
                1,
                route_receipt_genesis_root(),
                &identity(),
                sha256_bytes(b"body"),
                502,
                10,
            )
            .is_err()
        );
        let mut receipt = NandoRouteReceiptV1::seal(
            1,
            route_receipt_genesis_root(),
            &identity(),
            sha256_bytes(b"body"),
            418,
            10,
        )
        .expect("receipt");
        receipt.remote_status = 200;
        assert!(!receipt.validate());
    }

    #[test]
    fn append_restart_and_partial_tail_recovery_are_fail_closed() {
        let root = temporary_root("restart");
        let path = root.join("route-receipts-v1.jsonl");
        let mut ledger =
            NandoRouteReceiptLedger::open(&path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .expect("ledger");
        let first = ledger
            .append(&identity(), sha256_bytes(b"one"), 418, 100)
            .expect("first");
        let second = ledger
            .append(&identity(), sha256_bytes(b"two"), 200, 200)
            .expect("second");
        drop(ledger);

        let mut raw = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append partial");
        raw.write_all(br#"{"schema":"partial""#)
            .expect("write partial");
        raw.flush().expect("flush partial");
        drop(raw);

        let ledger = NandoRouteReceiptLedger::open(&path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
            .expect("recovered ledger");
        assert_eq!(ledger.last_sequence, second.sequence);
        assert_eq!(
            ledger.previous_receipt_root_sha256,
            second.receipt_root_sha256
        );
        drop(ledger);

        let index = NandoRouteReceiptIndex::open(&path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
            .expect("index");
        assert_eq!(index.len(), 2);
        assert_eq!(
            index
                .receipt_for_frame(&first.turn_intent_id_sha256, &first.session_id_sha256, 300)
                .map(|receipt| receipt.sequence),
            Some(second.sequence)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
