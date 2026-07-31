//! Authenticated cold-path transport for compact verified session evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use nando_client_evidence::NandoRouteReceiptV1;
use nando_operator_kernel::{
    RelationFrame, canonical_json_sha256, sha256_bytes, valid_nonzero_sha256,
};
use nando_operator_learning::{
    RuntimeParityCase, is_source_neutral_relation_frame, teacher_outcome_from_completed,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::multi_source_frame_archive::MultiSourceFrameArchive;

pub const REMOTE_EVIDENCE_ENDPOINT_V1: &str = "/_nando/evidence/v1/batches";
pub const REMOTE_EVIDENCE_BATCH_SCHEMA_V1: &str = "nando.remote-evidence-batch.v1";
pub const REMOTE_EVIDENCE_FRAME_SCHEMA_V1: &str = "nando.remote-evidence-frame.v1";
const REMOTE_EVIDENCE_HEAD_SCHEMA_V1: &str = "nando.remote-evidence-client-head.v1";
pub const REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1: usize = 8 * 1024 * 1024;
pub const REMOTE_EVIDENCE_MAX_FRAME_BYTES_V1: usize = 4 * 1024 * 1024;
pub const REMOTE_EVIDENCE_MAX_FRAMES_V1: usize = 32;
pub const REMOTE_EVIDENCE_AUTH_SKEW_SECONDS_V1: u64 = 300;
const REMOTE_EVIDENCE_MAX_CLIENTS_V1: usize = 1_024;
const REMOTE_EVIDENCE_MAX_KEY_BYTES_V1: usize = 256;
const HEAD_FILE: &str = "head-v1.cbor";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteEvidenceFrameV1 {
    pub schema: String,
    pub frame_root_sha256: String,
    pub verifier_receipt_root_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_receipt_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_receipt: Option<NandoRouteReceiptV1>,
    pub session_id_sha256: String,
    pub turn_intent_id_sha256: String,
    pub action_event_id_sha256: String,
    pub observed_at_unix_nanos: u64,
    pub frame: RelationFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_parity_case: Option<RuntimeParityCase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteBoundEvidenceFrameV1 {
    pub frame: RelationFrame,
    pub route_receipt: NandoRouteReceiptV1,
    pub runtime_parity_case: Option<RuntimeParityCase>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteEvidenceBatchV1 {
    pub schema: String,
    pub batch_root_sha256: String,
    pub client_id_sha256: String,
    pub sequence: u64,
    pub previous_batch_root_sha256: String,
    pub generated_at_unix: u64,
    pub frames: Vec<RemoteEvidenceFrameV1>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct RemoteEvidenceBatchDigestV1<'a> {
    schema: &'static str,
    client_id_sha256: &'a str,
    sequence: u64,
    previous_batch_root_sha256: &'a str,
    generated_at_unix: u64,
    frames: &'a [RemoteEvidenceFrameV1],
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RemoteEvidenceClientHeadV1 {
    schema: String,
    head_root_sha256: String,
    client_id_sha256: String,
    sequence: u64,
    batch_root_sha256: String,
    accepted_batches: u64,
    accepted_frames: u64,
    last_accepted_at_unix: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RemoteEvidenceSpoolStatusV1 {
    pub enabled: bool,
    pub transport_ready: bool,
    pub configured_clients: u64,
    pub active_clients: u64,
    pub accepted_batches: u64,
    pub accepted_frames: u64,
    pub route_bound_frames: u64,
    pub runtime_parity_frames: u64,
    pub duplicate_batches: u64,
    pub auth_failures: u64,
    pub rejected_batches: u64,
    pub last_accepted_at_unix: u64,
    pub learning_closed_loop_ready: bool,
    pub raw_session_payload_persisted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteEvidenceAckV1 {
    pub ok: bool,
    pub client_id_sha256: String,
    pub sequence: u64,
    pub batch_root_sha256: String,
    pub accepted_frames: u64,
    pub duplicate_idempotent: bool,
    pub ack_hmac_sha256: String,
}

pub(crate) struct RemoteEvidenceSpoolRuntime {
    root: PathBuf,
    keys_directory: PathBuf,
    heads: BTreeMap<String, RemoteEvidenceClientHeadV1>,
    configured_clients: u64,
    route_bound_frame_roots: BTreeSet<String>,
    route_receipt_by_frame_root: BTreeMap<String, NandoRouteReceiptV1>,
    runtime_parity_by_frame_root: BTreeMap<String, RuntimeParityCase>,
    duplicate_batches: u64,
    auth_failures: u64,
    rejected_batches: u64,
}

type AcceptedFrameEvidenceV1 = (
    BTreeSet<String>,
    BTreeMap<String, NandoRouteReceiptV1>,
    BTreeMap<String, RuntimeParityCase>,
);

impl RemoteEvidenceFrameV1 {
    pub fn seal(frame: RelationFrame) -> Result<Self, String> {
        Self::seal_with_route_receipt(frame, None, None)
    }

    pub fn seal_route_bound(
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
    ) -> Result<Self, String> {
        Self::seal_route_bound_with_parity(frame, route_receipt, None)
    }

    pub fn seal_route_bound_with_parity(
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<Self, String> {
        Self::seal_with_route_receipt(frame, Some(route_receipt), runtime_parity_case)
    }

    fn seal_with_route_receipt(
        frame: RelationFrame,
        route_receipt: Option<NandoRouteReceiptV1>,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<Self, String> {
        let outcome = teacher_outcome_from_completed(&frame)
            .map_err(|error| format!("remote_evidence_verifier:{error:?}"))?;
        if !outcome.verifier.accepted
            || frame.verifier_label != Some(true)
            || !is_source_neutral_relation_frame(&frame)
        {
            return Err("remote_evidence_frame_not_verified".to_owned());
        }
        let frame_root_sha256 = canonical_json_sha256(&frame)
            .map_err(|error| format!("remote_evidence_frame_root:{error}"))?;
        let verifier_receipt_root_sha256 = canonical_json_sha256(&outcome.verifier)
            .map_err(|error| format!("remote_evidence_verifier_root:{error}"))?;
        let route_receipt_root_sha256 = route_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_root_sha256.clone());
        let receipt = Self {
            schema: REMOTE_EVIDENCE_FRAME_SCHEMA_V1.to_owned(),
            frame_root_sha256,
            verifier_receipt_root_sha256,
            route_receipt_root_sha256,
            route_receipt,
            session_id_sha256: frame.session_id_sha256.clone(),
            turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
            action_event_id_sha256: frame.event_id_sha256.clone(),
            observed_at_unix_nanos: frame.observed_at_unix_nanos,
            frame,
            runtime_parity_case,
        };
        receipt
            .validate()
            .then_some(receipt)
            .ok_or_else(|| "remote_evidence_frame_invalid".to_owned())
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        if self.schema != REMOTE_EVIDENCE_FRAME_SCHEMA_V1
            || self.frame.verifier_label != Some(true)
            || !is_source_neutral_relation_frame(&self.frame)
            || self.observed_at_unix_nanos == 0
            || ![
                self.frame_root_sha256.as_str(),
                self.verifier_receipt_root_sha256.as_str(),
                self.session_id_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.action_event_id_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self
                .route_receipt_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || !self.route_binding_is_valid()
            || self.session_id_sha256 != self.frame.session_id_sha256
            || self.turn_intent_id_sha256 != self.frame.client_intent_id_sha256
            || self.action_event_id_sha256 != self.frame.event_id_sha256
            || self.observed_at_unix_nanos != self.frame.observed_at_unix_nanos
            || self.runtime_parity_case.as_ref().is_some_and(|parity| {
                parity.evidence_ref_sha256 != self.frame.frame_id_sha256
                    || parity.request_text.is_empty()
                    || parity.expected_response.is_empty()
                    || serde_cbor::to_vec(parity).map_or(true, |bytes| {
                        bytes.len() > REMOTE_EVIDENCE_MAX_FRAME_BYTES_V1
                    })
            })
            || !matches!(
                canonical_json_sha256(&self.frame),
                Ok(root) if root == self.frame_root_sha256
            )
        {
            return false;
        }
        teacher_outcome_from_completed(&self.frame).is_ok_and(|outcome| {
            outcome.verifier.accepted
                && canonical_json_sha256(&outcome.verifier)
                    .is_ok_and(|root| root == self.verifier_receipt_root_sha256)
        })
    }

    #[must_use]
    pub fn is_route_bound(&self) -> bool {
        self.route_receipt.is_some() && self.route_binding_is_valid()
    }

    fn route_binding_is_valid(&self) -> bool {
        match (
            self.route_receipt_root_sha256.as_deref(),
            self.route_receipt.as_ref(),
        ) {
            (None, None) => true,
            // Root-only V1 frames remain decodable but are not proven route bindings.
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(root), Some(receipt)) => {
                receipt.validate()
                    && root == receipt.receipt_root_sha256
                    && receipt.turn_intent_id_sha256 == self.frame.client_intent_id_sha256
                    && receipt.session_id_sha256 == self.frame.session_id_sha256
                    && receipt.request_observed_at_unix_nanos <= self.frame.observed_at_unix_nanos
                    && receipt.route_confirmed_at_unix_nanos <= self.frame.observed_at_unix_nanos
            }
        }
    }
}

impl RemoteEvidenceBatchV1 {
    pub fn seal(
        client_id_sha256: String,
        sequence: u64,
        previous_batch_root_sha256: String,
        generated_at_unix: u64,
        frames: Vec<RelationFrame>,
    ) -> Result<Self, String> {
        Self::seal_frames(
            client_id_sha256,
            sequence,
            previous_batch_root_sha256,
            generated_at_unix,
            frames
                .into_iter()
                .map(RemoteEvidenceFrameV1::seal)
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    pub fn seal_route_bound(
        client_id_sha256: String,
        sequence: u64,
        previous_batch_root_sha256: String,
        generated_at_unix: u64,
        frames: Vec<RouteBoundEvidenceFrameV1>,
    ) -> Result<Self, String> {
        Self::seal_frames(
            client_id_sha256,
            sequence,
            previous_batch_root_sha256,
            generated_at_unix,
            frames
                .into_iter()
                .map(|bound| {
                    RemoteEvidenceFrameV1::seal_route_bound_with_parity(
                        bound.frame,
                        bound.route_receipt,
                        bound.runtime_parity_case,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    fn seal_frames(
        client_id_sha256: String,
        sequence: u64,
        previous_batch_root_sha256: String,
        generated_at_unix: u64,
        mut frames: Vec<RemoteEvidenceFrameV1>,
    ) -> Result<Self, String> {
        if !valid_nonzero_sha256(&client_id_sha256)
            || !valid_nonzero_sha256(&previous_batch_root_sha256)
            || sequence == 0
            || generated_at_unix == 0
            || frames.is_empty()
            || frames.len() > REMOTE_EVIDENCE_MAX_FRAMES_V1
        {
            return Err("remote_evidence_batch_invalid".to_owned());
        }
        frames.sort_by(|left, right| left.frame_root_sha256.cmp(&right.frame_root_sha256));
        if frames
            .windows(2)
            .any(|pair| pair[0].frame_root_sha256 == pair[1].frame_root_sha256)
        {
            return Err("remote_evidence_batch_duplicate_frame".to_owned());
        }
        let mut batch = Self {
            schema: REMOTE_EVIDENCE_BATCH_SCHEMA_V1.to_owned(),
            batch_root_sha256: String::new(),
            client_id_sha256,
            sequence,
            previous_batch_root_sha256,
            generated_at_unix,
            frames,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        batch.batch_root_sha256 = batch.expected_root();
        if !batch.validate() {
            return Err("remote_evidence_batch_invalid".to_owned());
        }
        let bytes = serde_cbor::to_vec(&batch)
            .map_err(|error| format!("remote_evidence_batch_encode:{error}"))?;
        if bytes.len() > REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1 {
            return Err("remote_evidence_batch_budget".to_owned());
        }
        Ok(batch)
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&RemoteEvidenceBatchDigestV1 {
            schema: REMOTE_EVIDENCE_BATCH_SCHEMA_V1,
            client_id_sha256: &self.client_id_sha256,
            sequence: self.sequence,
            previous_batch_root_sha256: &self.previous_batch_root_sha256,
            generated_at_unix: self.generated_at_unix,
            frames: &self.frames,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .expect("remote evidence batch serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == REMOTE_EVIDENCE_BATCH_SCHEMA_V1
            && valid_nonzero_sha256(&self.batch_root_sha256)
            && valid_nonzero_sha256(&self.client_id_sha256)
            && valid_nonzero_sha256(&self.previous_batch_root_sha256)
            && self.sequence > 0
            && self.generated_at_unix > 0
            && !self.frames.is_empty()
            && self.frames.len() <= REMOTE_EVIDENCE_MAX_FRAMES_V1
            && self.frames.iter().all(RemoteEvidenceFrameV1::validate)
            && self
                .frames
                .windows(2)
                .all(|pair| pair[0].frame_root_sha256 < pair[1].frame_root_sha256)
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self.batch_root_sha256 == self.expected_root()
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        if !self.validate() {
            return Err("remote_evidence_batch_invalid".to_owned());
        }
        let bytes = serde_cbor::to_vec(self)
            .map_err(|error| format!("remote_evidence_batch_encode:{error}"))?;
        if bytes.is_empty() || bytes.len() > REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1 {
            return Err("remote_evidence_batch_budget".to_owned());
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1 {
            return Err("remote_evidence_batch_budget".to_owned());
        }
        let batch: Self = serde_cbor::from_slice(bytes)
            .map_err(|error| format!("remote_evidence_batch_decode:{error}"))?;
        if !batch.validate() || batch.canonical_bytes()? != bytes {
            return Err("remote_evidence_batch_noncanonical".to_owned());
        }
        Ok(batch)
    }
}

impl RemoteEvidenceAckV1 {
    fn seal(
        key: &[u8],
        client_id_sha256: String,
        sequence: u64,
        batch_root_sha256: String,
        accepted_frames: u64,
        duplicate_idempotent: bool,
    ) -> Result<Self, String> {
        let mut ack = Self {
            ok: true,
            client_id_sha256,
            sequence,
            batch_root_sha256,
            accepted_frames,
            duplicate_idempotent,
            ack_hmac_sha256: String::new(),
        };
        ack.ack_hmac_sha256 = ack.expected_hmac(key)?;
        ack.verify(key)
            .then_some(ack)
            .ok_or_else(|| "remote_evidence_ack_invalid".to_owned())
    }

    #[must_use]
    pub fn verify(&self, key: &[u8]) -> bool {
        if !self.ok
            || key.len() != 32
            || !valid_nonzero_sha256(&self.client_id_sha256)
            || self.sequence == 0
            || !valid_nonzero_sha256(&self.batch_root_sha256)
            || self.accepted_frames == 0
        {
            return false;
        }
        let Ok(signature) = hex_decode(&self.ack_hmac_sha256) else {
            return false;
        };
        let Ok(mut mac) = HmacSha256::new_from_slice(key) else {
            return false;
        };
        mac.update(self.auth_payload().as_bytes());
        mac.verify_slice(&signature).is_ok()
    }

    fn expected_hmac(&self, key: &[u8]) -> Result<String, String> {
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|_| "remote_evidence_key_invalid".to_owned())?;
        mac.update(self.auth_payload().as_bytes());
        Ok(hex_encode(&mac.finalize().into_bytes()))
    }

    fn auth_payload(&self) -> String {
        format!(
            "ACK\n{}\n{}\n{}\n{}\n{}",
            self.client_id_sha256,
            self.sequence,
            self.batch_root_sha256,
            self.accepted_frames,
            u8::from(self.duplicate_idempotent)
        )
    }
}

#[must_use]
pub fn remote_evidence_genesis_root(client_id_sha256: &str) -> String {
    canonical_json_sha256(&("nando.remote-evidence-client-genesis.v1", client_id_sha256))
        .expect("remote evidence genesis serializes")
}

pub fn sign_remote_evidence_request_v1(
    key: &[u8],
    timestamp_unix: u64,
    batch: &RemoteEvidenceBatchV1,
    body: &[u8],
) -> Result<String, String> {
    if key.len() != 32 || body != batch.canonical_bytes()? {
        return Err("remote_evidence_signing_input_invalid".to_owned());
    }
    let payload = auth_payload(timestamp_unix, batch, body);
    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|_| "remote_evidence_key_invalid".to_owned())?;
    mac.update(payload.as_bytes());
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

fn verify_remote_evidence_request_v1(
    key: &[u8],
    timestamp_unix: u64,
    signature_hex: &str,
    batch: &RemoteEvidenceBatchV1,
    body: &[u8],
) -> bool {
    let Ok(signature) = hex_decode(signature_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(key) else {
        return false;
    };
    mac.update(auth_payload(timestamp_unix, batch, body).as_bytes());
    mac.verify_slice(&signature).is_ok()
}

fn auth_payload(timestamp_unix: u64, batch: &RemoteEvidenceBatchV1, body: &[u8]) -> String {
    format!(
        "POST\n{}\n{}\n{}\n{}\n{}",
        REMOTE_EVIDENCE_ENDPOINT_V1,
        timestamp_unix,
        batch.sequence,
        batch.batch_root_sha256,
        sha256_bytes(body)
    )
}

impl RemoteEvidenceClientHeadV1 {
    fn genesis(client_id_sha256: &str) -> Self {
        let mut head = Self {
            schema: REMOTE_EVIDENCE_HEAD_SCHEMA_V1.to_owned(),
            head_root_sha256: String::new(),
            client_id_sha256: client_id_sha256.to_owned(),
            sequence: 0,
            batch_root_sha256: remote_evidence_genesis_root(client_id_sha256),
            accepted_batches: 0,
            accepted_frames: 0,
            last_accepted_at_unix: 0,
        };
        head.head_root_sha256 = head.expected_root();
        head
    }

    fn advance(&self, batch: &RemoteEvidenceBatchV1, accepted_at_unix: u64) -> Self {
        let mut head = Self {
            schema: REMOTE_EVIDENCE_HEAD_SCHEMA_V1.to_owned(),
            head_root_sha256: String::new(),
            client_id_sha256: self.client_id_sha256.clone(),
            sequence: batch.sequence,
            batch_root_sha256: batch.batch_root_sha256.clone(),
            accepted_batches: self.accepted_batches.saturating_add(1),
            accepted_frames: self
                .accepted_frames
                .saturating_add(u64::try_from(batch.frames.len()).unwrap_or(u64::MAX)),
            last_accepted_at_unix: accepted_at_unix,
        };
        head.head_root_sha256 = head.expected_root();
        head
    }

    fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            REMOTE_EVIDENCE_HEAD_SCHEMA_V1,
            self.client_id_sha256.as_str(),
            self.sequence,
            self.batch_root_sha256.as_str(),
            self.accepted_batches,
            self.accepted_frames,
            self.last_accepted_at_unix,
        ))
        .expect("remote evidence head serializes")
    }

    fn validate(&self) -> bool {
        self.schema == REMOTE_EVIDENCE_HEAD_SCHEMA_V1
            && valid_nonzero_sha256(&self.head_root_sha256)
            && valid_nonzero_sha256(&self.client_id_sha256)
            && valid_nonzero_sha256(&self.batch_root_sha256)
            && self.accepted_batches == self.sequence
            && (self.sequence == 0 && self.accepted_frames == 0 && self.last_accepted_at_unix == 0
                || self.sequence > 0 && self.accepted_frames > 0 && self.last_accepted_at_unix > 0)
            && self.head_root_sha256 == self.expected_root()
    }
}

impl RemoteEvidenceSpoolRuntime {
    pub(crate) fn open(root: PathBuf, keys_directory: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(root.join("clients"))
            .map_err(|error| format!("remote_evidence_spool_directory:{error}"))?;
        let mut heads = BTreeMap::new();
        let mut route_bound_frame_roots = BTreeSet::new();
        let mut route_receipt_by_frame_root = BTreeMap::new();
        let mut runtime_parity_by_frame_root = BTreeMap::new();
        for entry in fs::read_dir(root.join("clients"))
            .map_err(|error| format!("remote_evidence_spool_scan:{error}"))?
        {
            let entry = entry.map_err(|error| format!("remote_evidence_spool_scan:{error}"))?;
            if !entry
                .file_type()
                .map_err(|error| format!("remote_evidence_spool_scan:{error}"))?
                .is_dir()
            {
                continue;
            }
            let Some(client_id) = entry.file_name().to_str().map(str::to_owned) else {
                return Err("remote_evidence_client_id_invalid".to_owned());
            };
            if !valid_nonzero_sha256(&client_id) {
                return Err("remote_evidence_client_id_invalid".to_owned());
            }
            let Some(bytes) = read_optional_bounded(
                &entry.path().join(HEAD_FILE),
                REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1,
            )?
            else {
                continue;
            };
            let head: RemoteEvidenceClientHeadV1 = serde_cbor::from_slice(&bytes)
                .map_err(|error| format!("remote_evidence_head_decode:{error}"))?;
            if !head.validate() || head.client_id_sha256 != client_id {
                return Err("remote_evidence_head_invalid".to_owned());
            }
            let canonical = serde_cbor::to_vec(&head)
                .map_err(|error| format!("remote_evidence_head_encode:{error}"))?;
            if canonical != bytes {
                return Err("remote_evidence_head_invalid".to_owned());
            }
            let (client_route_bound_roots, client_route_receipts, client_runtime_parity) =
                load_accepted_frame_evidence(&entry.path(), &head)?;
            route_bound_frame_roots.extend(client_route_bound_roots);
            for (frame_root, receipt) in client_route_receipts {
                match route_receipt_by_frame_root.get(&frame_root) {
                    Some(existing) if existing == &receipt => {}
                    Some(_) => return Err("remote_evidence_route_receipt_rebound".to_owned()),
                    None => {
                        route_receipt_by_frame_root.insert(frame_root, receipt);
                    }
                }
            }
            for (frame_root, parity) in client_runtime_parity {
                match runtime_parity_by_frame_root.get(&frame_root) {
                    Some(existing) if existing == &parity => {}
                    Some(_) => return Err("remote_evidence_runtime_parity_rebound".to_owned()),
                    None => {
                        runtime_parity_by_frame_root.insert(frame_root, parity);
                    }
                }
            }
            heads.insert(client_id, head);
        }
        if heads.len() > REMOTE_EVIDENCE_MAX_CLIENTS_V1 {
            return Err("remote_evidence_client_budget".to_owned());
        }
        let configured_clients = count_configured_clients(&keys_directory)?;
        Ok(Self {
            root,
            keys_directory,
            heads,
            configured_clients,
            route_bound_frame_roots,
            route_receipt_by_frame_root,
            runtime_parity_by_frame_root,
            duplicate_batches: 0,
            auth_failures: 0,
            rejected_batches: 0,
        })
    }

    pub(crate) fn ingest(
        &mut self,
        client_id_sha256: &str,
        timestamp_unix: u64,
        signature_hex: &str,
        body: &[u8],
        now_unix: u64,
        frame_archive: &mut MultiSourceFrameArchive,
    ) -> Result<RemoteEvidenceAckV1, String> {
        let result = self.ingest_inner(
            client_id_sha256,
            timestamp_unix,
            signature_hex,
            body,
            now_unix,
            frame_archive,
        );
        if let Err(error) = &result {
            if error.starts_with("remote_evidence_auth") {
                self.auth_failures = self.auth_failures.saturating_add(1);
            } else {
                self.rejected_batches = self.rejected_batches.saturating_add(1);
            }
        }
        result
    }

    fn ingest_inner(
        &mut self,
        client_id_sha256: &str,
        timestamp_unix: u64,
        signature_hex: &str,
        body: &[u8],
        now_unix: u64,
        frame_archive: &mut MultiSourceFrameArchive,
    ) -> Result<RemoteEvidenceAckV1, String> {
        if !valid_nonzero_sha256(client_id_sha256)
            || timestamp_unix.abs_diff(now_unix) > REMOTE_EVIDENCE_AUTH_SKEW_SECONDS_V1
        {
            return Err("remote_evidence_auth_invalid".to_owned());
        }
        let batch = RemoteEvidenceBatchV1::from_canonical_bytes(body)?;
        if batch.client_id_sha256 != client_id_sha256
            || batch.generated_at_unix
                > now_unix.saturating_add(REMOTE_EVIDENCE_AUTH_SKEW_SECONDS_V1)
        {
            return Err("remote_evidence_auth_binding_invalid".to_owned());
        }
        let key = read_client_key(&self.keys_directory, client_id_sha256)?;
        if !verify_remote_evidence_request_v1(&key, timestamp_unix, signature_hex, &batch, body) {
            return Err("remote_evidence_auth_signature_invalid".to_owned());
        }
        let head = self
            .heads
            .get(client_id_sha256)
            .cloned()
            .unwrap_or_else(|| RemoteEvidenceClientHeadV1::genesis(client_id_sha256));
        if batch.sequence == head.sequence && batch.batch_root_sha256 == head.batch_root_sha256 {
            self.duplicate_batches = self.duplicate_batches.saturating_add(1);
            return RemoteEvidenceAckV1::seal(
                &key,
                client_id_sha256.to_owned(),
                batch.sequence,
                batch.batch_root_sha256,
                u64::try_from(batch.frames.len()).unwrap_or(u64::MAX),
                true,
            );
        }
        if batch.sequence != head.sequence.saturating_add(1)
            || batch.previous_batch_root_sha256 != head.batch_root_sha256
        {
            return Err("remote_evidence_sequence_conflict".to_owned());
        }
        for frame in &batch.frames {
            if let Some(receipt) = &frame.route_receipt
                && self
                    .route_receipt_by_frame_root
                    .get(&frame.frame_root_sha256)
                    .is_some_and(|existing| existing != receipt)
            {
                return Err("remote_evidence_route_receipt_rebound".to_owned());
            }
            if let Some(parity) = &frame.runtime_parity_case
                && self
                    .runtime_parity_by_frame_root
                    .get(&frame.frame_root_sha256)
                    .is_some_and(|existing| existing != parity)
            {
                return Err("remote_evidence_runtime_parity_rebound".to_owned());
            }
        }
        let client_directory = self.root.join("clients").join(client_id_sha256);
        fs::create_dir_all(&client_directory)
            .map_err(|error| format!("remote_evidence_client_directory:{error}"))?;
        let receipt_name = format!("{:020}-{}.cbor", batch.sequence, batch.batch_root_sha256);
        let receipt_path = client_directory.join(&receipt_name);
        let pending_path = client_directory.join(format!("{receipt_name}.pending"));
        match read_optional_bounded(&receipt_path, REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1)? {
            Some(existing) if existing != body => {
                return Err("remote_evidence_batch_rebound".to_owned());
            }
            Some(_) => {}
            None => match read_optional_bounded(&pending_path, REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1)?
            {
                Some(existing) if existing != body => {
                    return Err("remote_evidence_batch_rebound".to_owned());
                }
                Some(_) => {}
                None => write_atomic(&pending_path, body)?,
            },
        }

        let frames = batch
            .frames
            .iter()
            .map(|receipt| receipt.frame.clone())
            .collect::<Vec<_>>();
        frame_archive.append_batch(&frames)?;
        if !receipt_path.exists() {
            fs::rename(&pending_path, &receipt_path)
                .map_err(|error| format!("remote_evidence_receipt_publish:{error}"))?;
            sync_directory(&client_directory)?;
        }
        let next_head = head.advance(&batch, now_unix);
        let head_bytes = serde_cbor::to_vec(&next_head)
            .map_err(|error| format!("remote_evidence_head_encode:{error}"))?;
        write_atomic(&client_directory.join(HEAD_FILE), &head_bytes)?;
        self.route_bound_frame_roots.extend(
            batch
                .frames
                .iter()
                .filter(|frame| frame.is_route_bound())
                .map(|frame| frame.frame_root_sha256.clone()),
        );
        for frame in &batch.frames {
            if let Some(receipt) = &frame.route_receipt {
                self.route_receipt_by_frame_root
                    .entry(frame.frame_root_sha256.clone())
                    .or_insert_with(|| receipt.clone());
            }
            if let Some(parity) = &frame.runtime_parity_case {
                self.runtime_parity_by_frame_root
                    .entry(frame.frame_root_sha256.clone())
                    .or_insert_with(|| parity.clone());
            }
        }
        self.heads.insert(client_id_sha256.to_owned(), next_head);
        RemoteEvidenceAckV1::seal(
            &key,
            client_id_sha256.to_owned(),
            batch.sequence,
            batch.batch_root_sha256,
            u64::try_from(batch.frames.len()).unwrap_or(u64::MAX),
            false,
        )
    }

    pub(crate) fn status(&self, frame_archive_ready: bool) -> RemoteEvidenceSpoolStatusV1 {
        let accepted_batches = self.heads.values().map(|head| head.accepted_batches).sum();
        let accepted_frames = self.heads.values().map(|head| head.accepted_frames).sum();
        RemoteEvidenceSpoolStatusV1 {
            enabled: true,
            transport_ready: frame_archive_ready && self.configured_clients > 0,
            configured_clients: self.configured_clients,
            active_clients: u64::try_from(self.heads.len()).unwrap_or(u64::MAX),
            accepted_batches,
            accepted_frames,
            route_bound_frames: u64::try_from(self.route_bound_frame_roots.len())
                .unwrap_or(u64::MAX),
            runtime_parity_frames: u64::try_from(self.runtime_parity_by_frame_root.len())
                .unwrap_or(u64::MAX),
            duplicate_batches: self.duplicate_batches,
            auth_failures: self.auth_failures,
            rejected_batches: self.rejected_batches,
            last_accepted_at_unix: self
                .heads
                .values()
                .map(|head| head.last_accepted_at_unix)
                .max()
                .unwrap_or(0),
            learning_closed_loop_ready: false,
            raw_session_payload_persisted: false,
        }
    }

    pub(crate) fn route_bound_frame_roots(&self) -> BTreeSet<String> {
        self.route_bound_frame_roots.clone()
    }

    pub(crate) fn route_receipts_by_frame_root(&self) -> BTreeMap<String, NandoRouteReceiptV1> {
        self.route_receipt_by_frame_root.clone()
    }

    pub(crate) fn runtime_parity_for_frame(
        &self,
        frame_root_sha256: &str,
    ) -> Option<RuntimeParityCase> {
        self.runtime_parity_by_frame_root
            .get(frame_root_sha256)
            .cloned()
    }
}

fn load_accepted_frame_evidence(
    client_directory: &Path,
    head: &RemoteEvidenceClientHeadV1,
) -> Result<AcceptedFrameEvidenceV1, String> {
    let mut batches = BTreeMap::new();
    for entry in fs::read_dir(client_directory)
        .map_err(|error| format!("remote_evidence_spool_scan:{error}"))?
    {
        let entry = entry.map_err(|error| format!("remote_evidence_spool_scan:{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("remote_evidence_spool_scan:{error}"))?
            .is_file()
            || entry.file_name() == HEAD_FILE
            || entry.path().extension().and_then(|value| value.to_str()) != Some("cbor")
        {
            continue;
        }
        let Some(bytes) = read_optional_bounded(&entry.path(), REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1)?
        else {
            continue;
        };
        let batch = RemoteEvidenceBatchV1::from_canonical_bytes(&bytes)?;
        if batch.client_id_sha256 != head.client_id_sha256 {
            return Err("remote_evidence_receipt_client_invalid".to_owned());
        }
        if batch.sequence <= head.sequence && batches.insert(batch.sequence, batch).is_some() {
            return Err("remote_evidence_receipt_sequence_duplicate".to_owned());
        }
    }
    let mut previous_root = remote_evidence_genesis_root(&head.client_id_sha256);
    let mut accepted_frames = 0_u64;
    let mut route_bound_frame_roots = BTreeSet::new();
    let mut route_receipt_by_frame_root = BTreeMap::new();
    let mut runtime_parity_by_frame_root = BTreeMap::new();
    for sequence in 1..=head.sequence {
        let batch = batches
            .get(&sequence)
            .ok_or_else(|| "remote_evidence_receipt_missing".to_owned())?;
        if batch.previous_batch_root_sha256 != previous_root {
            return Err("remote_evidence_receipt_chain_invalid".to_owned());
        }
        previous_root.clone_from(&batch.batch_root_sha256);
        accepted_frames =
            accepted_frames.saturating_add(u64::try_from(batch.frames.len()).unwrap_or(u64::MAX));
        route_bound_frame_roots.extend(
            batch
                .frames
                .iter()
                .filter(|frame| frame.is_route_bound())
                .map(|frame| frame.frame_root_sha256.clone()),
        );
        for frame in &batch.frames {
            if let Some(receipt) = &frame.route_receipt {
                match route_receipt_by_frame_root.get(&frame.frame_root_sha256) {
                    Some(existing) if existing == receipt => {}
                    Some(_) => return Err("remote_evidence_route_receipt_rebound".to_owned()),
                    None => {
                        route_receipt_by_frame_root
                            .insert(frame.frame_root_sha256.clone(), receipt.clone());
                    }
                }
            }
            if let Some(parity) = &frame.runtime_parity_case {
                match runtime_parity_by_frame_root.get(&frame.frame_root_sha256) {
                    Some(existing) if existing == parity => {}
                    Some(_) => return Err("remote_evidence_runtime_parity_rebound".to_owned()),
                    None => {
                        runtime_parity_by_frame_root
                            .insert(frame.frame_root_sha256.clone(), parity.clone());
                    }
                }
            }
        }
    }
    if previous_root != head.batch_root_sha256 || accepted_frames != head.accepted_frames {
        return Err("remote_evidence_receipt_head_invalid".to_owned());
    }
    Ok((
        route_bound_frame_roots,
        route_receipt_by_frame_root,
        runtime_parity_by_frame_root,
    ))
}

fn count_configured_clients(directory: &Path) -> Result<u64, String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("remote_evidence_key_directory:{error}")),
    };
    let mut clients = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("remote_evidence_key_directory:{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("remote_evidence_key_directory:{error}"))?
            .is_file()
        {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(client_id) = name.strip_suffix(".key") else {
            continue;
        };
        if valid_nonzero_sha256(client_id) {
            clients.insert(client_id.to_owned());
        }
    }
    if clients.len() > REMOTE_EVIDENCE_MAX_CLIENTS_V1 {
        return Err("remote_evidence_client_budget".to_owned());
    }
    Ok(u64::try_from(clients.len()).unwrap_or(u64::MAX))
}

fn read_client_key(directory: &Path, client_id_sha256: &str) -> Result<Vec<u8>, String> {
    let path = directory.join(format!("{client_id_sha256}.key"));
    let metadata =
        fs::metadata(&path).map_err(|_| "remote_evidence_auth_client_unknown".to_owned())?;
    #[cfg(unix)]
    if metadata.mode() & 0o077 != 0 {
        return Err("remote_evidence_auth_key_permissions".to_owned());
    }
    if !metadata.is_file() || metadata.len() > REMOTE_EVIDENCE_MAX_KEY_BYTES_V1 as u64 {
        return Err("remote_evidence_auth_key_invalid".to_owned());
    }
    let bytes = fs::read(&path).map_err(|_| "remote_evidence_auth_key_read".to_owned())?;
    parse_key_bytes(&bytes).map_err(|_| "remote_evidence_auth_key_invalid".to_owned())
}

pub fn parse_remote_evidence_key_v1(bytes: &[u8]) -> Result<Vec<u8>, String> {
    parse_key_bytes(bytes)
}

fn parse_key_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() == 32 {
        return Ok(bytes.to_vec());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "remote_evidence_key_invalid".to_owned())?
        .trim();
    let key = hex_decode(text)?;
    (key.len() == 32)
        .then_some(key)
        .ok_or_else(|| "remote_evidence_key_invalid".to_owned())
}

fn read_optional_bounded(path: &Path, max_bytes: usize) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("remote_evidence_read:{}:{error}", path.display())),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(max_bytes.saturating_add(1)).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("remote_evidence_read:{}:{error}", path.display()))?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err("remote_evidence_state_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1 {
        return Err("remote_evidence_state_budget".to_owned());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "remote_evidence_parent_missing".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("remote_evidence_directory:{error}"))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("remote_evidence_write_open:{error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("remote_evidence_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("remote_evidence_rename:{error}"))?;
    sync_directory(parent)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("remote_evidence_directory_sync:{error}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    #[allow(clippy::manual_is_multiple_of)] // Keep the workspace Rust 1.95 MSRV.
    if value.len() % 2 != 0 {
        return Err("remote_evidence_hex_invalid".to_owned());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("remote_evidence_hex_invalid".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nando_client_evidence::{
        ClientRouteIdentityV1, NandoRouteReceiptV1, route_receipt_genesis_root,
        sha256_bytes as client_sha256_bytes,
    };
    use nando_operator_kernel::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame,
        ResponseValueSelector, sha256_bytes,
    };
    use nando_operator_learning::{RuntimeParityCase, SOURCE_NEUTRAL_EXTRACTOR_VERSION};
    use serde_json::json;

    use super::{
        REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1, RemoteEvidenceBatchV1, RemoteEvidenceSpoolRuntime,
        RouteBoundEvidenceFrameV1, remote_evidence_genesis_root, sign_remote_evidence_request_v1,
        write_atomic,
    };
    use crate::multi_source_frame_archive::MultiSourceFrameArchive;

    fn hash(value: &str) -> String {
        sha256_bytes(value.as_bytes())
    }

    fn completed_frame(label: &str) -> RelationFrame {
        let value_root = hash(&format!("value:{label}"));
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: hash(&format!("frame:{label}")),
            event_id_sha256: hash(&format!("event:{label}")),
            client_intent_id_sha256: hash(&format!("intent:{label}")),
            session_id_sha256: hash(&format!("session:{label}")),
            observed_at_unix_nanos: 1_700_000_000_000_000_000,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: value_root.clone(),
                },
                RelationAtom::UniqueSlot { slot_id: 7 },
                RelationAtom::ObservationSelector {
                    slot_id: 7,
                    selector: ResponseValueSelector::JsonField {
                        field: "opaque".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 11,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: value_root,
                },
                RelationAtom::SlotEquality {
                    left_slot: 7,
                    right_slot: 11,
                },
                RelationAtom::ActionFunction {
                    value: "transport_a".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: 11,
                    value_type: Some(AtomValueType::Integer),
                },
            ],
            evidence_ref_sha256: hash(&format!("evidence:{label}")),
        }
    }

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nando-remote-evidence-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn route_receipt_for_frame(
        frame: &RelationFrame,
        request_observed_at_unix_nanos: u64,
        route_confirmed_at_unix_nanos: u64,
    ) -> NandoRouteReceiptV1 {
        NandoRouteReceiptV1::seal(
            1,
            route_receipt_genesis_root(),
            &ClientRouteIdentityV1 {
                turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
                session_id_sha256: frame.session_id_sha256.clone(),
            },
            client_sha256_bytes(b"request"),
            418,
            request_observed_at_unix_nanos,
            route_confirmed_at_unix_nanos,
        )
        .expect("route receipt")
    }

    fn runtime_parity(frame: &RelationFrame, expected_response: &str) -> RuntimeParityCase {
        RuntimeParityCase {
            evidence_ref_sha256: frame.frame_id_sha256.clone(),
            capture_receipt: None,
            request_text: "Return opaque".to_owned(),
            provider_payload: json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"opaque\":7}"
                }]
            }),
            expected_response: expected_response.to_owned(),
        }
    }

    fn write_key(directory: &std::path::Path, client_id: &str, key: &[u8]) {
        fs::create_dir_all(directory).expect("key directory");
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(directory.join(format!("{client_id}.key")))
            .expect("key file");
        file.write_all(key).expect("key write");
        file.sync_all().expect("key sync");
    }

    fn signed_batch(
        client_id: &str,
        key: &[u8],
        label: &str,
    ) -> (RemoteEvidenceBatchV1, Vec<u8>, String) {
        let batch = RemoteEvidenceBatchV1::seal(
            client_id.to_owned(),
            1,
            remote_evidence_genesis_root(client_id),
            1_700_000_000,
            vec![completed_frame(label)],
        )
        .expect("batch");
        let body = batch.canonical_bytes().expect("body");
        let signature =
            sign_remote_evidence_request_v1(key, batch.generated_at_unix, &batch, &body)
                .expect("signature");
        (batch, body, signature)
    }

    #[test]
    fn authenticated_batch_is_durable_and_restart_idempotent() {
        let root = temporary_root("restart");
        let spool_root = root.join("spool");
        let key_root = root.join("keys");
        let archive_root = root.join("frames");
        let client_id = hash("client:restart");
        let key = [7_u8; 32];
        write_key(&key_root, &client_id, &key);
        let (batch, body, _) = signed_batch(&client_id, &key, "restart");
        let retry_at_unix = batch.generated_at_unix.saturating_add(1_000);
        let signature = sign_remote_evidence_request_v1(&key, retry_at_unix, &batch, &body)
            .expect("delayed retry signature");

        let mut archive = MultiSourceFrameArchive::open(&archive_root).expect("archive");
        let mut runtime =
            RemoteEvidenceSpoolRuntime::open(spool_root.clone(), key_root.clone()).expect("spool");
        let ack = runtime
            .ingest(
                &client_id,
                retry_at_unix,
                &signature,
                &body,
                retry_at_unix,
                &mut archive,
            )
            .expect("ingest");
        assert!(!ack.duplicate_idempotent);
        assert!(ack.verify(&key));
        let mut tampered_ack = ack.clone();
        tampered_ack.accepted_frames = tampered_ack.accepted_frames.saturating_add(1);
        assert!(!tampered_ack.verify(&key));
        assert_eq!(archive.len(), 1);
        assert!(runtime.status(true).transport_ready);
        assert_eq!(runtime.status(true).route_bound_frames, 0);
        assert!(!runtime.status(true).learning_closed_loop_ready);

        drop(runtime);
        drop(archive);
        let mut restored_archive =
            MultiSourceFrameArchive::open(&archive_root).expect("archive restore");
        let mut restored =
            RemoteEvidenceSpoolRuntime::open(spool_root, key_root).expect("spool restore");
        let replay = restored
            .ingest(
                &client_id,
                retry_at_unix,
                &signature,
                &body,
                retry_at_unix,
                &mut restored_archive,
            )
            .expect("idempotent replay");
        assert!(replay.duplicate_idempotent);
        assert!(replay.verify(&key));
        assert_eq!(restored_archive.len(), 1);
        assert_eq!(restored.status(true).accepted_frames, 1);
        assert_eq!(restored.status(true).route_bound_frames, 0);
        assert_eq!(restored.status(true).duplicate_batches, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn route_bound_frame_count_survives_restart_without_promoting_authority() {
        let root = temporary_root("route-bound");
        let spool_root = root.join("spool");
        let key_root = root.join("keys");
        let archive_root = root.join("frames");
        let client_id = hash("client:route-bound");
        let key = [13_u8; 32];
        write_key(&key_root, &client_id, &key);
        let frame = completed_frame("route-bound");
        let route_receipt = route_receipt_for_frame(
            &frame,
            frame.observed_at_unix_nanos.saturating_sub(2),
            frame.observed_at_unix_nanos.saturating_sub(1),
        );
        let batch = RemoteEvidenceBatchV1::seal_route_bound(
            client_id.clone(),
            1,
            remote_evidence_genesis_root(&client_id),
            1_700_000_000,
            vec![RouteBoundEvidenceFrameV1 {
                frame,
                route_receipt,
                runtime_parity_case: None,
            }],
        )
        .expect("route-bound batch");
        let body = batch.canonical_bytes().expect("body");
        let signature =
            sign_remote_evidence_request_v1(&key, batch.generated_at_unix, &batch, &body)
                .expect("signature");
        let mut archive = MultiSourceFrameArchive::open(&archive_root).expect("archive");
        let mut runtime =
            RemoteEvidenceSpoolRuntime::open(spool_root.clone(), key_root.clone()).expect("spool");
        runtime
            .ingest(
                &client_id,
                batch.generated_at_unix,
                &signature,
                &body,
                batch.generated_at_unix,
                &mut archive,
            )
            .expect("ingest");
        assert_eq!(runtime.status(true).route_bound_frames, 1);
        assert!(!runtime.status(true).learning_closed_loop_ready);
        drop(runtime);

        let restored =
            RemoteEvidenceSpoolRuntime::open(spool_root, key_root).expect("spool restore");
        assert_eq!(restored.status(true).route_bound_frames, 1);
        assert!(!restored.status(true).learning_closed_loop_ready);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runtime_parity_survives_restart_and_rebound_is_rejected() {
        let root = temporary_root("runtime-parity");
        let spool_root = root.join("spool");
        let key_root = root.join("keys");
        let archive_root = root.join("frames");
        let client_id = hash("client:runtime-parity");
        let key = [23_u8; 32];
        write_key(&key_root, &client_id, &key);
        let frame = completed_frame("runtime-parity");
        let route_receipt = route_receipt_for_frame(
            &frame,
            frame.observed_at_unix_nanos.saturating_sub(2),
            frame.observed_at_unix_nanos.saturating_sub(1),
        );
        let batch = RemoteEvidenceBatchV1::seal_route_bound(
            client_id.clone(),
            1,
            remote_evidence_genesis_root(&client_id),
            1_700_000_000,
            vec![RouteBoundEvidenceFrameV1 {
                frame: frame.clone(),
                route_receipt: route_receipt.clone(),
                runtime_parity_case: Some(runtime_parity(&frame, "7")),
            }],
        )
        .expect("parity batch");
        let body = batch.canonical_bytes().expect("body");
        let signature =
            sign_remote_evidence_request_v1(&key, batch.generated_at_unix, &batch, &body)
                .expect("signature");
        let mut archive = MultiSourceFrameArchive::open(&archive_root).expect("archive");
        let mut runtime =
            RemoteEvidenceSpoolRuntime::open(spool_root.clone(), key_root.clone()).expect("spool");
        runtime
            .ingest(
                &client_id,
                batch.generated_at_unix,
                &signature,
                &body,
                batch.generated_at_unix,
                &mut archive,
            )
            .expect("ingest");
        assert_eq!(runtime.status(true).runtime_parity_frames, 1);
        assert_eq!(
            runtime.runtime_parity_for_frame(&batch.frames[0].frame_root_sha256),
            batch.frames[0].runtime_parity_case
        );
        drop(runtime);

        let mut restored =
            RemoteEvidenceSpoolRuntime::open(spool_root, key_root).expect("spool restore");
        assert_eq!(restored.status(true).runtime_parity_frames, 1);
        assert_eq!(
            restored.runtime_parity_for_frame(&batch.frames[0].frame_root_sha256),
            batch.frames[0].runtime_parity_case
        );

        let rebound = RemoteEvidenceBatchV1::seal_route_bound(
            client_id.clone(),
            2,
            batch.batch_root_sha256.clone(),
            batch.generated_at_unix.saturating_add(1),
            vec![RouteBoundEvidenceFrameV1 {
                frame: frame.clone(),
                route_receipt,
                runtime_parity_case: Some(runtime_parity(&frame, "8")),
            }],
        )
        .expect("rebound batch");
        let rebound_body = rebound.canonical_bytes().expect("rebound body");
        let rebound_signature = sign_remote_evidence_request_v1(
            &key,
            rebound.generated_at_unix,
            &rebound,
            &rebound_body,
        )
        .expect("rebound signature");
        assert_eq!(
            restored
                .ingest(
                    &client_id,
                    rebound.generated_at_unix,
                    &rebound_signature,
                    &rebound_body,
                    rebound.generated_at_unix,
                    &mut archive,
                )
                .expect_err("parity rebound must fail"),
            "remote_evidence_runtime_parity_rebound"
        );
        assert_eq!(restored.status(true).accepted_frames, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn route_binding_requires_the_full_pre_action_receipt() {
        let frame = completed_frame("route-proof");
        let receipt = route_receipt_for_frame(
            &frame,
            frame.observed_at_unix_nanos.saturating_sub(2),
            frame.observed_at_unix_nanos.saturating_sub(1),
        );
        let sealed = super::RemoteEvidenceFrameV1::seal_route_bound(frame.clone(), receipt)
            .expect("route-bound frame");
        assert!(sealed.validate());
        assert!(sealed.is_route_bound());

        let mut legacy_root_only = sealed.clone();
        legacy_root_only.route_receipt = None;
        assert!(legacy_root_only.validate());
        assert!(!legacy_root_only.is_route_bound());

        let post_action_receipt = route_receipt_for_frame(
            &frame,
            frame.observed_at_unix_nanos.saturating_sub(1),
            frame.observed_at_unix_nanos.saturating_add(1),
        );
        assert!(
            super::RemoteEvidenceFrameV1::seal_route_bound(frame, post_action_receipt).is_err()
        );
    }

    #[test]
    fn invalid_signature_is_rejected_before_archive_mutation() {
        let root = temporary_root("signature");
        let key_root = root.join("keys");
        let client_id = hash("client:signature");
        let key = [11_u8; 32];
        write_key(&key_root, &client_id, &key);
        let (batch, body, _) = signed_batch(&client_id, &key, "signature");
        let mut archive = MultiSourceFrameArchive::open(&root.join("frames")).expect("archive");
        let mut runtime =
            RemoteEvidenceSpoolRuntime::open(root.join("spool"), key_root).expect("spool");
        let error = runtime
            .ingest(
                &client_id,
                batch.generated_at_unix,
                &"00".repeat(32),
                &body,
                batch.generated_at_unix,
                &mut archive,
            )
            .expect_err("signature must fail");
        assert_eq!(error, "remote_evidence_auth_signature_invalid");
        assert_eq!(archive.len(), 0);
        assert_eq!(runtime.status(true).auth_failures, 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn pending_receipt_recovers_after_archive_append_crash_boundary() {
        let root = temporary_root("pending");
        let spool_root = root.join("spool");
        let key_root = root.join("keys");
        let archive_root = root.join("frames");
        let client_id = hash("client:pending");
        let key = [19_u8; 32];
        write_key(&key_root, &client_id, &key);
        let (batch, body, signature) = signed_batch(&client_id, &key, "pending");
        let client_root = spool_root.join("clients").join(&client_id);
        let receipt_name = format!("{:020}-{}.cbor", batch.sequence, batch.batch_root_sha256);
        write_atomic(&client_root.join(format!("{receipt_name}.pending")), &body)
            .expect("pending receipt");

        let mut archive = MultiSourceFrameArchive::open(&archive_root).expect("archive");
        archive
            .append_batch(
                &batch
                    .frames
                    .iter()
                    .map(|receipt| receipt.frame.clone())
                    .collect::<Vec<_>>(),
            )
            .expect("pre-crash archive append");
        let mut runtime = RemoteEvidenceSpoolRuntime::open(spool_root, key_root).expect("spool");
        let ack = runtime
            .ingest(
                &client_id,
                batch.generated_at_unix,
                &signature,
                &body,
                batch.generated_at_unix,
                &mut archive,
            )
            .expect("pending recovery");
        assert!(!ack.duplicate_idempotent);
        assert_eq!(archive.len(), 1);
        assert!(client_root.join(receipt_name).is_file());
        assert!(
            body.len() <= REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1,
            "fixture stays within the wire budget"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
