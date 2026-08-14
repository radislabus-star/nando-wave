use std::io::Read;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use nando_operator_kernel::valid_nonzero_sha256;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::*;

pub(super) const K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2: usize = 3 * 1024 * 1024;
pub(super) const K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2: usize = 16 * 1024 * 1024;
const K1_CANDIDATE_FREEZE_ENCODING_V2: &str = "zstd-base64";
const K1_CANDIDATE_FREEZE_WINDOW_LOG_MAX_V2: u32 = 24;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1CandidateFreezeAuthorityRequestV2 {
    pub schema: String,
    pub logical_schema: String,
    pub encoding: String,
    pub(crate) logical_bytes: u64,
    pub logical_sha256: String,
    pub(crate) compressed_bytes: u64,
    pub payload_base64: String,
    pub scheduler_ledger_revision: u64,
    pub scheduler_ledger_root_sha256: String,
    pub scheduler_projection_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct K1SchedulerCasV2 {
    pub ledger_revision: u64,
    pub ledger_root_sha256: String,
    pub projection_root_sha256: String,
}

pub(crate) fn encode_candidate_freeze_v2(
    request: &K1CandidateFreezeAuthorityRequestV1,
    projection: &K1SchedulerProjectionV1,
) -> Result<K1CandidateFreezeAuthorityRequestV2, String> {
    request.catalog.validate().map_err(str::to_owned)?;
    projection.validate()?;
    let logical = serde_json::to_vec(request)
        .map_err(|error| format!("k1_candidate_freeze_v2_logical_encode:{error}"))?;
    if logical.len() > K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2 {
        return Err("k1_candidate_freeze_v2_logical_budget".to_owned());
    }
    let compressed = zstd::stream::encode_all(logical.as_slice(), 1)
        .map_err(|error| format!("k1_candidate_freeze_v2_compress:{error}"))?;
    if compressed.len() > K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2 {
        return Err("k1_candidate_freeze_v2_compressed_budget".to_owned());
    }
    let envelope = K1CandidateFreezeAuthorityRequestV2 {
        schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V2.to_owned(),
        logical_schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        encoding: K1_CANDIDATE_FREEZE_ENCODING_V2.to_owned(),
        logical_bytes: usize_to_u64(logical.len())?,
        logical_sha256: sha256_bytes(&logical),
        compressed_bytes: usize_to_u64(compressed.len())?,
        payload_base64: BASE64_STANDARD.encode(compressed),
        scheduler_ledger_revision: projection.ledger_revision,
        scheduler_ledger_root_sha256: projection.ledger_root_sha256.clone(),
        scheduler_projection_root_sha256: projection.projection_root_sha256.clone(),
    };
    let outer_bytes = serde_json::to_vec(&envelope)
        .map_err(|error| format!("k1_candidate_freeze_v2_envelope_encode:{error}"))?;
    if outer_bytes.len() > K1_SCHEDULER_MAX_REQUEST_BYTES {
        return Err("k1_candidate_freeze_v2_outer_budget".to_owned());
    }
    Ok(envelope)
}

pub(super) fn decode_candidate_freeze_v2(
    envelope: K1CandidateFreezeAuthorityRequestV2,
) -> Result<(K1CandidateFreezeAuthorityRequestV1, K1SchedulerCasV2), String> {
    if envelope.schema != K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V2
        || envelope.logical_schema != K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1
        || envelope.encoding != K1_CANDIDATE_FREEZE_ENCODING_V2
        || envelope.logical_bytes == 0
        || envelope.logical_bytes > usize_to_u64(K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2)?
        || envelope.compressed_bytes == 0
        || envelope.compressed_bytes > usize_to_u64(K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2)?
        || !valid_nonzero_sha256(&envelope.logical_sha256)
        || !valid_nonzero_sha256(&envelope.scheduler_ledger_root_sha256)
        || !valid_nonzero_sha256(&envelope.scheduler_projection_root_sha256)
    {
        return Err("k1_candidate_freeze_v2_envelope_invalid".to_owned());
    }
    let compressed = BASE64_STANDARD
        .decode(envelope.payload_base64.as_bytes())
        .map_err(|_| "k1_candidate_freeze_v2_base64_invalid".to_owned())?;
    if usize_to_u64(compressed.len())? != envelope.compressed_bytes {
        return Err("k1_candidate_freeze_v2_compressed_length_mismatch".to_owned());
    }
    let frame_bytes = zstd::zstd_safe::find_frame_compressed_size(&compressed)
        .map_err(|_| "k1_candidate_freeze_v2_frame_invalid".to_owned())?;
    if frame_bytes != compressed.len() {
        return Err("k1_candidate_freeze_v2_trailing_bytes".to_owned());
    }
    let mut decoder = zstd::stream::read::Decoder::new(compressed.as_slice())
        .map_err(|_| "k1_candidate_freeze_v2_frame_invalid".to_owned())?;
    decoder
        .window_log_max(K1_CANDIDATE_FREEZE_WINDOW_LOG_MAX_V2)
        .map_err(|_| "k1_candidate_freeze_v2_window_budget".to_owned())?;
    let decoder = decoder.single_frame();
    let mut logical = Vec::with_capacity(
        usize::try_from(envelope.logical_bytes)
            .unwrap_or(K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2)
            .min(K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2),
    );
    decoder
        .take(usize_to_u64(
            K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2.saturating_add(1),
        )?)
        .read_to_end(&mut logical)
        .map_err(|_| "k1_candidate_freeze_v2_decompress_failed".to_owned())?;
    if logical.len() > K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2 {
        return Err("k1_candidate_freeze_v2_logical_budget".to_owned());
    }
    if usize_to_u64(logical.len())? != envelope.logical_bytes {
        return Err("k1_candidate_freeze_v2_logical_length_mismatch".to_owned());
    }
    if sha256_bytes(&logical) != envelope.logical_sha256 {
        return Err("k1_candidate_freeze_v2_logical_checksum_mismatch".to_owned());
    }
    let request: K1CandidateFreezeAuthorityRequestV1 = serde_json::from_slice(&logical)
        .map_err(|error| format!("k1_candidate_freeze_v2_logical_decode:{error}"))?;
    let canonical = serde_json::to_vec(&request)
        .map_err(|error| format!("k1_candidate_freeze_v2_logical_encode:{error}"))?;
    if canonical != logical {
        return Err("k1_candidate_freeze_v2_logical_noncanonical".to_owned());
    }
    Ok((
        request,
        K1SchedulerCasV2 {
            ledger_revision: envelope.scheduler_ledger_revision,
            ledger_root_sha256: envelope.scheduler_ledger_root_sha256,
            projection_root_sha256: envelope.scheduler_projection_root_sha256,
        },
    ))
}

fn usize_to_u64(value: usize) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "k1_candidate_freeze_v2_size_overflow".to_owned())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
