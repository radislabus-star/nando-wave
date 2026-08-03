use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_sha256, sha256_bytes, valid_nonzero_sha256};
use nando_operator_learning::multi_source::PreActionTopologyAuditRowV1;
use nando_operator_proof::independent_verifier_v3::F6_MAX_RAW_REQUEST_BYTES_V3;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::multi_source_topology_archive::read_topology_row_by_root;
use crate::operator_certification::CertificationAuthorityConfigV1;

const EVIDENCE_SCHEMA_V1: &str = "nando.k1-pre-action-authority-evidence.v1";
const EVIDENCE_DIR: &str = "k1-pre-action-authority-evidence-v1";
const MAX_PROVIDER_PAYLOAD_BYTES: usize = F6_MAX_RAW_REQUEST_BYTES_V3;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct K1PreActionAuthorityEvidenceV1 {
    schema: String,
    evidence_root_sha256: String,
    topology_commitment_root_sha256: String,
    provider_capture_request_root_sha256: String,
    provider_payload_json: String,
}

pub(super) struct RestoredPreActionEvidenceV1 {
    pub topology: PreActionTopologyAuditRowV1,
    pub provider_payload_json: String,
}

pub(super) fn archive(
    config: &CertificationAuthorityConfigV1,
    topology_commitment_root_sha256: &str,
    provider_capture_request_root_sha256: &str,
    provider_payload_json: String,
) -> Result<K1PreActionAuthorityEvidenceV1, String> {
    let topology = restore_topology(
        config,
        topology_commitment_root_sha256,
        provider_capture_request_root_sha256,
    )?;
    validate_payload(&provider_payload_json, provider_capture_request_root_sha256)?;
    if topology.commit.provider_capture_request_root_sha256 != provider_capture_request_root_sha256
    {
        return Err("k1_pre_action_evidence_capture_rebound".to_owned());
    }
    let evidence = K1PreActionAuthorityEvidenceV1::seal(
        topology_commitment_root_sha256.to_owned(),
        provider_capture_request_root_sha256.to_owned(),
        provider_payload_json,
    )?;
    let path = evidence_path(config, provider_capture_request_root_sha256)?;
    if path.exists() {
        let existing = read_evidence(&path)?;
        return if existing == evidence {
            Ok(existing)
        } else {
            Err("k1_pre_action_evidence_replacement_forbidden".to_owned())
        };
    }
    let bytes = serde_cbor::to_vec(&evidence)
        .map_err(|error| format!("k1_pre_action_evidence_encode:{error}"))?;
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| "k1_pre_action_evidence_parent_missing".to_owned())?,
    )
    .map_err(|error| format!("k1_pre_action_evidence_dir:{error}"))?;
    crate::write_bytes_atomic(&path, &bytes, "k1-pre-action-evidence")?;
    let restored = read_evidence(&path)?;
    if restored != evidence {
        return Err("k1_pre_action_evidence_restart_parity_failed".to_owned());
    }
    Ok(restored)
}

pub(super) fn restore(
    config: &CertificationAuthorityConfigV1,
    topology_commitment_root_sha256: &str,
    provider_capture_request_root_sha256: &str,
) -> Result<RestoredPreActionEvidenceV1, String> {
    let topology = restore_topology(
        config,
        topology_commitment_root_sha256,
        provider_capture_request_root_sha256,
    )?;
    let evidence = read_evidence(&evidence_path(
        config,
        provider_capture_request_root_sha256,
    )?)?;
    evidence.validate()?;
    if evidence.topology_commitment_root_sha256 != topology_commitment_root_sha256
        || evidence.provider_capture_request_root_sha256 != provider_capture_request_root_sha256
    {
        return Err("k1_pre_action_evidence_root_mismatch".to_owned());
    }
    Ok(RestoredPreActionEvidenceV1 {
        topology,
        provider_payload_json: evidence.provider_payload_json,
    })
}

pub(super) fn restore_topology(
    config: &CertificationAuthorityConfigV1,
    topology_commitment_root_sha256: &str,
    provider_capture_request_root_sha256: &str,
) -> Result<PreActionTopologyAuditRowV1, String> {
    if !valid_nonzero_sha256(topology_commitment_root_sha256)
        || !valid_nonzero_sha256(provider_capture_request_root_sha256)
    {
        return Err("k1_pre_action_evidence_root_invalid".to_owned());
    }
    let row = read_topology_row_by_root(
        &topology_archive_path(config)?,
        topology_commitment_root_sha256,
    )
    .map_err(|error| format!("k1_pre_action_topology_archive:{error}"))?;
    if row.commit.provider_capture_request_root_sha256 != provider_capture_request_root_sha256 {
        return Err("k1_pre_action_topology_capture_mismatch".to_owned());
    }
    Ok(row)
}

impl K1PreActionAuthorityEvidenceV1 {
    fn seal(
        topology_commitment_root_sha256: String,
        provider_capture_request_root_sha256: String,
        provider_payload_json: String,
    ) -> Result<Self, String> {
        let evidence_root_sha256 = evidence_root(
            &topology_commitment_root_sha256,
            &provider_capture_request_root_sha256,
            &provider_payload_json,
        )?;
        let evidence = Self {
            schema: EVIDENCE_SCHEMA_V1.to_owned(),
            evidence_root_sha256,
            topology_commitment_root_sha256,
            provider_capture_request_root_sha256,
            provider_payload_json,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != EVIDENCE_SCHEMA_V1
            || !valid_nonzero_sha256(&self.evidence_root_sha256)
            || !valid_nonzero_sha256(&self.topology_commitment_root_sha256)
            || !valid_nonzero_sha256(&self.provider_capture_request_root_sha256)
            || validate_payload(
                &self.provider_payload_json,
                &self.provider_capture_request_root_sha256,
            )
            .is_err()
            || evidence_root(
                &self.topology_commitment_root_sha256,
                &self.provider_capture_request_root_sha256,
                &self.provider_payload_json,
            )? != self.evidence_root_sha256
        {
            return Err("k1_pre_action_evidence_invalid".to_owned());
        }
        Ok(())
    }
}

fn validate_payload(payload: &str, expected_root_sha256: &str) -> Result<(), String> {
    if payload.is_empty()
        || payload.len() > MAX_PROVIDER_PAYLOAD_BYTES
        || sha256_bytes(payload.as_bytes()) != expected_root_sha256
        || serde_json::from_str::<Value>(payload).is_err()
    {
        return Err("k1_pre_action_evidence_payload_invalid".to_owned());
    }
    Ok(())
}

fn evidence_root(
    topology_commitment_root_sha256: &str,
    provider_capture_request_root_sha256: &str,
    provider_payload_json: &str,
) -> Result<String, String> {
    canonical_json_sha256(&(
        EVIDENCE_SCHEMA_V1,
        topology_commitment_root_sha256,
        provider_capture_request_root_sha256,
        sha256_bytes(provider_payload_json.as_bytes()),
    ))
    .map_err(|error| format!("k1_pre_action_evidence_root:{error}"))
}

fn topology_archive_path(config: &CertificationAuthorityConfigV1) -> Result<PathBuf, String> {
    Ok(config
        .root
        .parent()
        .ok_or_else(|| "k1_pre_action_topology_archive_parent_missing".to_owned())?
        .join("pre-action-topology-archive-v1"))
}

fn evidence_path(
    config: &CertificationAuthorityConfigV1,
    provider_capture_request_root_sha256: &str,
) -> Result<PathBuf, String> {
    if !valid_nonzero_sha256(provider_capture_request_root_sha256) {
        return Err("k1_pre_action_evidence_capture_root_invalid".to_owned());
    }
    Ok(config
        .root
        .join(EVIDENCE_DIR)
        .join(format!("{provider_capture_request_root_sha256}.cbor")))
}

fn read_evidence(path: &Path) -> Result<K1PreActionAuthorityEvidenceV1, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("k1_pre_action_evidence_metadata:{error}"))?;
    if metadata.len() > u64::try_from(MAX_PROVIDER_PAYLOAD_BYTES + 16_384).unwrap_or(u64::MAX) {
        return Err("k1_pre_action_evidence_budget".to_owned());
    }
    let bytes = fs::read(path).map_err(|error| format!("k1_pre_action_evidence_read:{error}"))?;
    let evidence: K1PreActionAuthorityEvidenceV1 = serde_cbor::from_slice(&bytes)
        .map_err(|error| format!("k1_pre_action_evidence_decode:{error}"))?;
    evidence.validate()?;
    Ok(evidence)
}
