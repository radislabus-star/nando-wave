use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use ed25519_dalek::{SigningKey, VerifyingKey};
use nando_operator_admission::{
    ExactMemoryCleanupReceiptV1, K1VocabularyGateV1, OperatorCertificationAnchorV1,
    OperatorCertificationEntryV1, OperatorCertificationJournalEventV1,
    OperatorCertificationLedgerV1, RuntimePackageRevocationLedgerV1,
};
use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_response_actor::{
    ResponseOperation, ResponsePackage, ResponseRegistry, response_execution_payload_digest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::write_bytes_atomic;

const LEDGER_CACHE_FILE: &str = "operator-certification-ledger-v1.json";
const JOURNAL_DIR: &str = "operator-certification-journal-v1";
#[cfg(test)]
const CLEANUP_DIR: &str = "exact-memory-cleanup-receipts-v1";
const AUTHORITY_REQUEST_SCHEMA: &str = "nando.operator-certification-authority-request.v1";
const AUTHORITY_RESPONSE_SCHEMA: &str = "nando.operator-certification-authority-response.v1";
const ROLE_TOPOLOGY_SCHEMA_V1: &str = "nando.operator-role-topology.v1";

#[derive(Clone, Debug)]
pub struct CertificationAuthorityConfigV1 {
    pub root: PathBuf,
    pub cleanup_receipts_path: PathBuf,
    pub anchor_path: PathBuf,
    pub authority_socket_path: PathBuf,
    pub authority_public_key_path: PathBuf,
    pub cleanup_public_key_path: PathBuf,
    pub response_registry_path: PathBuf,
    pub runtime_revocations_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CertificationProjectionV1 {
    pub ledger_root_sha256: String,
    pub entry: OperatorCertificationEntryV1,
    pub k1_vocabulary_gate: K1VocabularyGateV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthorityRequestV1 {
    schema: String,
    entry: OperatorCertificationEntryV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthorityResponseV1 {
    schema: String,
    projection: Option<CertificationProjectionV1>,
    error: String,
}

pub fn append_entry(
    config: &CertificationAuthorityConfigV1,
    entry: OperatorCertificationEntryV1,
) -> Result<CertificationProjectionV1, String> {
    #[cfg(not(unix))]
    {
        let _ = (config, entry);
        return Err("operator_certification_authority_requires_unix".to_owned());
    }
    #[cfg(unix)]
    {
        let mut stream = UnixStream::connect(&config.authority_socket_path)
            .map_err(|error| format!("operator_certification_authority_connect:{error}"))?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|error| format!("operator_certification_authority_read_timeout:{error}"))?;
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|error| format!("operator_certification_authority_write_timeout:{error}"))?;
        let request = AuthorityRequestV1 {
            schema: AUTHORITY_REQUEST_SCHEMA.to_owned(),
            entry,
        };
        serde_json::to_writer(&mut stream, &request)
            .map_err(|error| format!("operator_certification_authority_encode:{error}"))?;
        stream
            .write_all(b"\n")
            .map_err(|error| format!("operator_certification_authority_write:{error}"))?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|error| format!("operator_certification_authority_shutdown:{error}"))?;
        let response: AuthorityResponseV1 = serde_json::from_reader(&mut stream)
            .map_err(|error| format!("operator_certification_authority_decode:{error}"))?;
        if response.schema != AUTHORITY_RESPONSE_SCHEMA || !response.error.is_empty() {
            return Err(if response.error.is_empty() {
                "operator_certification_authority_response_invalid".to_owned()
            } else {
                response.error
            });
        }
        let projection = response
            .projection
            .ok_or_else(|| "operator_certification_authority_projection_missing".to_owned())?;
        validate_projection(config, &projection)?;
        Ok(projection)
    }
}

pub fn validate_projection(
    config: &CertificationAuthorityConfigV1,
    projection: &CertificationProjectionV1,
) -> Result<(), String> {
    let current_ledger = restore_anchored_ledger(config)?;
    let ledger = if current_ledger.ledger_root_sha256 == projection.ledger_root_sha256 {
        current_ledger.clone()
    } else {
        restore_signed_journal_ledger_root(
            config,
            &projection.ledger_root_sha256,
            current_ledger.revision,
        )?
    };
    let persisted_entry = ledger
        .latest_entries()
        .into_iter()
        .find(|candidate| candidate.package_id == projection.entry.package_id)
        .ok_or_else(|| "operator_certification_projection_missing".to_owned())?;
    let persisted_gate = ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    if ledger.ledger_root_sha256 != projection.ledger_root_sha256
        || persisted_entry != &projection.entry
        || persisted_gate != projection.k1_vocabulary_gate
    {
        return Err("operator_certification_projection_binding_mismatch".to_owned());
    }
    let current_entry = current_ledger
        .latest_entries()
        .into_iter()
        .find(|candidate| candidate.package_id == projection.entry.package_id)
        .ok_or_else(|| "operator_certification_current_projection_missing".to_owned())?;
    let current_gate = current_ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    if (projection.entry.product_registry_member && !current_entry.product_registry_member)
        || (projection.entry.epistemic_registry_member && !current_entry.epistemic_registry_member)
        || (projection.entry.k1_unit_eligible && !current_entry.k1_unit_eligible)
        || (projection.k1_vocabulary_gate.open && !current_gate.open)
    {
        return Err("operator_certification_stale_safety_projection".to_owned());
    }
    Ok(())
}

pub fn restore_cleanup_receipt(
    config: &CertificationAuthorityConfigV1,
    bundle_id_sha256: &str,
    package_id: &str,
    candidate_root_sha256: &str,
) -> Result<Option<ExactMemoryCleanupReceiptV1>, String> {
    let path = config
        .cleanup_receipts_path
        .join(format!("{bundle_id_sha256}.json"));
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("exact_memory_cleanup_receipt_read:{error}")),
    };
    let receipt: ExactMemoryCleanupReceiptV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("exact_memory_cleanup_receipt_decode:{error}"))?;
    let public_key = read_verifying_key(&config.cleanup_public_key_path)?;
    receipt
        .validate_with_public_key(&public_key)
        .map_err(str::to_owned)?;
    if receipt.bundle_id_sha256 != bundle_id_sha256
        || receipt.package_id != package_id
        || receipt.candidate_root_sha256 != candidate_root_sha256
    {
        return Err("exact_memory_cleanup_receipt_binding_mismatch".to_owned());
    }
    Ok(Some(receipt))
}

#[cfg(unix)]
pub fn run_authority(
    config: CertificationAuthorityConfigV1,
    signing_key_path: &Path,
) -> Result<(), String> {
    let signing_key = read_signing_key(signing_key_path)?;
    let expected_public_key = read_verifying_key(&config.authority_public_key_path)?;
    if signing_key.verifying_key() != expected_public_key {
        return Err("operator_certification_authority_key_mismatch".to_owned());
    }
    if let Some(parent) = config.authority_socket_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("operator_certification_socket_parent:{error}"))?;
    }
    if config.authority_socket_path.exists() {
        fs::remove_file(&config.authority_socket_path)
            .map_err(|error| format!("operator_certification_socket_remove:{error}"))?;
    }
    let listener = UnixListener::bind(&config.authority_socket_path)
        .map_err(|error| format!("operator_certification_socket_bind:{error}"))?;
    fs::set_permissions(
        &config.authority_socket_path,
        fs::Permissions::from_mode(0o660),
    )
    .map_err(|error| format!("operator_certification_socket_permissions:{error}"))?;
    recover_anchor(&config, &signing_key)?;
    for connection in listener.incoming() {
        let mut stream = connection
            .map_err(|error| format!("operator_certification_authority_accept:{error}"))?;
        let payload = match handle_authority_request(&config, &signing_key, &mut stream) {
            Ok(projection) => AuthorityResponseV1 {
                schema: AUTHORITY_RESPONSE_SCHEMA.to_owned(),
                projection: Some(projection),
                error: String::new(),
            },
            Err(error) => AuthorityResponseV1 {
                schema: AUTHORITY_RESPONSE_SCHEMA.to_owned(),
                projection: None,
                error,
            },
        };
        serde_json::to_writer(&mut stream, &payload)
            .map_err(|error| format!("operator_certification_authority_response:{error}"))?;
    }
    Ok(())
}

#[cfg(unix)]
fn handle_authority_request(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    stream: &mut UnixStream,
) -> Result<CertificationProjectionV1, String> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("operator_certification_authority_read:{error}"))?;
    let request: AuthorityRequestV1 = serde_json::from_str(&line)
        .map_err(|error| format!("operator_certification_authority_request_decode:{error}"))?;
    if request.schema != AUTHORITY_REQUEST_SCHEMA {
        return Err("operator_certification_authority_request_schema_invalid".to_owned());
    }
    append_authoritative(config, signing_key, request.entry)
}

fn append_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    entry: OperatorCertificationEntryV1,
) -> Result<CertificationProjectionV1, String> {
    validate_external_evidence(config, &entry)?;
    let (mut ledger, last_event_root) = restore_signed_journal(config)?;
    let changed = match ledger.append(entry.clone()) {
        Ok(changed) => changed,
        Err("operator_certification_transition_invalid") => {
            let package = restore_registry_package(config, &entry.package_id)?;
            ledger
                .migrate_role_topology(
                    entry.clone(),
                    &legacy_role_topology_id(&package)?,
                    &canonical_role_topology_id(&package)?,
                )
                .map_err(str::to_owned)?
        }
        Err(error) => return Err(error.to_owned()),
    };
    let last_event_root = if changed {
        let event = OperatorCertificationJournalEventV1::seal(
            ledger.revision,
            &last_event_root,
            entry.clone(),
            &ledger.ledger_root_sha256,
            signing_key,
        )
        .map_err(str::to_owned)?;
        persist_event(&config.root, &event)?;
        persist_anchor(config, signing_key, &ledger, &event.event_root_sha256)?;
        persist_cache(&config.root, &ledger)?;
        event.event_root_sha256
    } else {
        last_event_root
    };
    let anchored = restore_anchored_ledger(config)?;
    if anchored != ledger || !valid_nonzero_sha256(&last_event_root) {
        return Err("operator_certification_authority_restart_parity".to_owned());
    }
    projection_for(&ledger, &entry.package_id)
}

fn validate_external_evidence(
    config: &CertificationAuthorityConfigV1,
    entry: &OperatorCertificationEntryV1,
) -> Result<(), String> {
    entry.validate().map_err(str::to_owned)?;
    if entry.law.status == nando_operator_admission::LawCertificateStatusV1::Pass {
        validate_cleanup_root_only(config, entry)?;
    }
    let live_false_bad_apply = durable_false_bad_apply(config, &entry.package_id)?;
    if entry.false_bad_apply != live_false_bad_apply
        || (live_false_bad_apply > 0
            && entry.execution.status
                != nando_operator_admission::ExecutionCertificateStatusV1::Revoked)
    {
        return Err("operator_certification_live_safety_binding_mismatch".to_owned());
    }
    Ok(())
}

fn validate_cleanup_root_only(
    config: &CertificationAuthorityConfigV1,
    entry: &OperatorCertificationEntryV1,
) -> Result<(), String> {
    let path = config
        .cleanup_receipts_path
        .join(format!("{}.json", entry.bundle_id_sha256));
    let receipt: ExactMemoryCleanupReceiptV1 = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("exact_memory_cleanup_receipt_read:{error}"))?,
    )
    .map_err(|error| format!("exact_memory_cleanup_receipt_decode:{error}"))?;
    receipt
        .validate_with_public_key(&read_verifying_key(&config.cleanup_public_key_path)?)
        .map_err(str::to_owned)?;
    let package = restore_registry_package(config, &entry.package_id)?;
    let execution_payload_sha256 =
        response_execution_payload_digest(&package).map_err(str::to_owned)?;
    if receipt.bundle_id_sha256 != entry.bundle_id_sha256
        || receipt.package_id != entry.package_id
        || receipt.execution_payload_sha256 != execution_payload_sha256
        || !entry
            .law
            .evidence_roots_sha256
            .contains(&receipt.candidate_root_sha256)
        || entry.law.cleanup_receipt_root_sha256.as_deref()
            != Some(receipt.receipt_root_sha256.as_str())
    {
        return Err("operator_certification_cleanup_root_mismatch".to_owned());
    }
    Ok(())
}

pub fn durable_false_bad_apply(
    config: &CertificationAuthorityConfigV1,
    package_id: &str,
) -> Result<u64, String> {
    durable_false_bad_apply_evidence(config, package_id).map(|(count, _)| count)
}

pub fn durable_false_bad_apply_evidence(
    config: &CertificationAuthorityConfigV1,
    package_id: &str,
) -> Result<(u64, Vec<String>), String> {
    let bytes = match fs::read(&config.runtime_revocations_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((0, Vec::new()));
        }
        Err(error) => return Err(format!("operator_certification_revocations_read:{error}")),
    };
    let revocations: RuntimePackageRevocationLedgerV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("operator_certification_revocations_decode:{error}"))?;
    revocations.validate().map_err(str::to_owned)?;
    let package = restore_registry_package(config, package_id)?;
    let execution_payload = response_execution_payload_digest(&package).map_err(str::to_owned)?;
    let evidence = revocations
        .revocations
        .iter()
        .filter(|revocation| {
            revocation.package_id == package_id
                && revocation.execution_payload_sha256 == execution_payload
        })
        .map(|revocation| revocation.request_sha256.clone())
        .collect::<Vec<_>>();
    let count = u64::try_from(evidence.len())
        .map_err(|_| "operator_certification_false_bad_apply_overflow".to_owned())?;
    Ok((count, evidence))
}

fn restore_registry_package(
    config: &CertificationAuthorityConfigV1,
    package_id: &str,
) -> Result<ResponsePackage, String> {
    let registry: ResponseRegistry = serde_json::from_slice(
        &fs::read(&config.response_registry_path)
            .map_err(|error| format!("operator_certification_registry_read:{error}"))?,
    )
    .map_err(|error| format!("operator_certification_registry_decode:{error}"))?;
    registry.validate().map_err(str::to_owned)?;
    registry
        .packages
        .into_iter()
        .find(|package| package.package_id == package_id)
        .ok_or_else(|| "operator_certification_registry_package_missing".to_owned())
}

fn canonical_role_topology_id(package: &ResponsePackage) -> Result<String, String> {
    let restored = package
        .crystallized_operator
        .as_ref()
        .ok_or_else(|| "operator_role_topology_bundle_missing".to_owned())?
        .restore_verified()
        .map_err(|_| "operator_role_topology_restore_failed".to_owned())?;
    canonical_json_sha256(&(
        ROLE_TOPOLOGY_SCHEMA_V1,
        restored.role_graph().topology_commitment_sha256(),
    ))
    .map_err(str::to_owned)
}

fn legacy_role_topology_id(package: &ResponsePackage) -> Result<String, String> {
    let operation_class = match &package.program.operation {
        ResponseOperation::UniqueConsensus { .. } => "unique_consensus",
        ResponseOperation::AdvancePlan { .. } => "advance_plan",
        ResponseOperation::FunctionCallFromRoles { .. } => "function_call_from_roles",
        ResponseOperation::CustomToolCallFromRoles { .. } => "custom_tool_call_from_roles",
        ResponseOperation::ProjectSelectedValue { .. } => "project_selected_value",
        ResponseOperation::ProjectStatus { .. } => "project_status",
        ResponseOperation::ComposeCollection { .. } => "compose_collection",
        ResponseOperation::CopyAfterPrefix { .. } => "copy_after_prefix",
        ResponseOperation::TestResultSummary { .. } => "test_result_summary",
        ResponseOperation::WaitOnYieldedCell { .. } => "wait_on_yielded_cell",
        ResponseOperation::WaitOnAnyYieldedCell { .. } => "wait_on_any_yielded_cell",
        ResponseOperation::WaitOnYieldedSurfaces { .. } => "wait_on_yielded_surfaces",
    };
    canonical_json_sha256(&(
        ROLE_TOPOLOGY_SCHEMA_V1,
        operation_class,
        &package.required_routing_atom_ids,
        package.proof.verifier_schema.as_str(),
    ))
    .map_err(str::to_owned)
}

fn restore_anchored_ledger(
    config: &CertificationAuthorityConfigV1,
) -> Result<OperatorCertificationLedgerV1, String> {
    let (ledger, last_event_root) = restore_signed_journal(config)?;
    if ledger.revision == 0 {
        if config.anchor_path.exists() {
            return Err("operator_certification_orphan_anchor".to_owned());
        }
        return Ok(ledger);
    }
    let anchor: OperatorCertificationAnchorV1 = serde_json::from_slice(
        &fs::read(&config.anchor_path)
            .map_err(|error| format!("operator_certification_anchor_read:{error}"))?,
    )
    .map_err(|error| format!("operator_certification_anchor_decode:{error}"))?;
    anchor
        .validate_with_public_key(&read_verifying_key(&config.authority_public_key_path)?)
        .map_err(str::to_owned)?;
    if anchor.revision != ledger.revision
        || anchor.journal_event_root_sha256 != last_event_root
        || anchor.ledger_root_sha256 != ledger.ledger_root_sha256
    {
        return Err("operator_certification_rollback_detected".to_owned());
    }
    Ok(ledger)
}

fn restore_signed_journal(
    config: &CertificationAuthorityConfigV1,
) -> Result<(OperatorCertificationLedgerV1, String), String> {
    let mut ledger = OperatorCertificationLedgerV1::empty().map_err(str::to_owned)?;
    let mut previous_root = journal_genesis_root();
    let directory = config.root.join(JOURNAL_DIR);
    let mut paths = match fs::read_dir(&directory) {
        Ok(entries) => entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("operator_certification_journal_list:{error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((ledger, previous_root));
        }
        Err(error) => return Err(format!("operator_certification_journal_open:{error}")),
    };
    paths.sort();
    let public_key = read_verifying_key(&config.authority_public_key_path)?;
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err("operator_certification_journal_unknown_file".to_owned());
        }
        let event: OperatorCertificationJournalEventV1 = serde_json::from_slice(
            &fs::read(&path)
                .map_err(|error| format!("operator_certification_journal_read:{error}"))?,
        )
        .map_err(|error| format!("operator_certification_journal_decode:{error}"))?;
        event
            .validate_with_public_key(&public_key)
            .map_err(str::to_owned)?;
        if event.sequence != ledger.revision.saturating_add(1)
            || event.previous_event_root_sha256 != previous_root
        {
            return Err("operator_certification_journal_chain_invalid".to_owned());
        }
        append_replayed_entry(&mut ledger, event.entry.clone())?;
        if event.resulting_ledger_root_sha256 != ledger.ledger_root_sha256 {
            return Err("operator_certification_journal_ledger_mismatch".to_owned());
        }
        previous_root = event.event_root_sha256;
    }
    Ok((ledger, previous_root))
}

fn recover_anchor(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
) -> Result<(), String> {
    let (mut ledger, mut last_event_root) = restore_signed_journal(config)?;
    if ledger.revision == 0 {
        let cache_path = config.root.join(LEDGER_CACHE_FILE);
        if !cache_path.is_file() {
            return Ok(());
        }
        let legacy: OperatorCertificationLedgerV1 = serde_json::from_slice(
            &fs::read(&cache_path)
                .map_err(|error| format!("operator_certification_cache_read:{error}"))?,
        )
        .map_err(|error| format!("operator_certification_cache_decode:{error}"))?;
        legacy.validate().map_err(str::to_owned)?;
        for entry in legacy.entries {
            ledger.append(entry.clone()).map_err(str::to_owned)?;
            let event = OperatorCertificationJournalEventV1::seal(
                ledger.revision,
                &last_event_root,
                entry,
                &ledger.ledger_root_sha256,
                signing_key,
            )
            .map_err(str::to_owned)?;
            persist_event(&config.root, &event)?;
            last_event_root = event.event_root_sha256;
        }
        persist_anchor(config, signing_key, &ledger, &last_event_root)?;
        return Ok(());
    }
    match restore_anchored_ledger(config) {
        Ok(restored) if restored == ledger => Ok(()),
        Err(error) if error.starts_with("operator_certification_anchor_read:") => {
            Err("operator_certification_anchor_missing_for_nonempty_journal".to_owned())
        }
        Err(error) if error == "operator_certification_rollback_detected" => {
            let anchor: OperatorCertificationAnchorV1 = serde_json::from_slice(
                &fs::read(&config.anchor_path)
                    .map_err(|error| format!("operator_certification_anchor_read:{error}"))?,
            )
            .map_err(|error| format!("operator_certification_anchor_decode:{error}"))?;
            anchor
                .validate_with_public_key(&signing_key.verifying_key())
                .map_err(str::to_owned)?;
            let (prefix, prefix_event_root) =
                restore_signed_journal_prefix(config, anchor.revision)?;
            if anchor.revision >= ledger.revision
                || anchor.ledger_root_sha256 != prefix.ledger_root_sha256
                || anchor.journal_event_root_sha256 != prefix_event_root
            {
                return Err("operator_certification_rollback_detected".to_owned());
            }
            persist_anchor(config, signing_key, &ledger, &last_event_root)
        }
        Err(error) => Err(error),
        Ok(_) => Err("operator_certification_anchor_recovery_mismatch".to_owned()),
    }
}

fn restore_signed_journal_prefix(
    config: &CertificationAuthorityConfigV1,
    revision: u64,
) -> Result<(OperatorCertificationLedgerV1, String), String> {
    let mut ledger = OperatorCertificationLedgerV1::empty().map_err(str::to_owned)?;
    let mut previous_root = journal_genesis_root();
    let public_key = read_verifying_key(&config.authority_public_key_path)?;
    for sequence in 1..=revision {
        let path = config
            .root
            .join(JOURNAL_DIR)
            .join(format!("{sequence:020}.json"));
        let event: OperatorCertificationJournalEventV1 = serde_json::from_slice(
            &fs::read(path)
                .map_err(|error| format!("operator_certification_journal_read:{error}"))?,
        )
        .map_err(|error| format!("operator_certification_journal_decode:{error}"))?;
        event
            .validate_with_public_key(&public_key)
            .map_err(str::to_owned)?;
        if event.sequence != sequence || event.previous_event_root_sha256 != previous_root {
            return Err("operator_certification_journal_chain_invalid".to_owned());
        }
        append_replayed_entry(&mut ledger, event.entry.clone())?;
        if event.resulting_ledger_root_sha256 != ledger.ledger_root_sha256 {
            return Err("operator_certification_journal_ledger_mismatch".to_owned());
        }
        previous_root = event.event_root_sha256;
    }
    Ok((ledger, previous_root))
}

fn restore_signed_journal_ledger_root(
    config: &CertificationAuthorityConfigV1,
    ledger_root_sha256: &str,
    maximum_revision: u64,
) -> Result<OperatorCertificationLedgerV1, String> {
    if !valid_nonzero_sha256(ledger_root_sha256) {
        return Err("operator_certification_projection_ledger_root_invalid".to_owned());
    }
    let mut ledger = OperatorCertificationLedgerV1::empty().map_err(str::to_owned)?;
    let mut previous_root = journal_genesis_root();
    let public_key = read_verifying_key(&config.authority_public_key_path)?;
    for sequence in 1..=maximum_revision {
        let path = config
            .root
            .join(JOURNAL_DIR)
            .join(format!("{sequence:020}.json"));
        let event: OperatorCertificationJournalEventV1 = serde_json::from_slice(
            &fs::read(path)
                .map_err(|error| format!("operator_certification_journal_read:{error}"))?,
        )
        .map_err(|error| format!("operator_certification_journal_decode:{error}"))?;
        event
            .validate_with_public_key(&public_key)
            .map_err(str::to_owned)?;
        if event.sequence != sequence || event.previous_event_root_sha256 != previous_root {
            return Err("operator_certification_journal_chain_invalid".to_owned());
        }
        append_replayed_entry(&mut ledger, event.entry.clone())?;
        if event.resulting_ledger_root_sha256 != ledger.ledger_root_sha256 {
            return Err("operator_certification_journal_ledger_mismatch".to_owned());
        }
        if ledger.ledger_root_sha256 == ledger_root_sha256 {
            return Ok(ledger);
        }
        previous_root = event.event_root_sha256;
    }
    Err("operator_certification_projection_not_in_anchored_journal".to_owned())
}

fn append_replayed_entry(
    ledger: &mut OperatorCertificationLedgerV1,
    entry: OperatorCertificationEntryV1,
) -> Result<(), String> {
    match ledger.append(entry.clone()) {
        Ok(_) => Ok(()),
        Err("operator_certification_transition_invalid") => {
            let previous = ledger
                .latest_entries()
                .into_iter()
                .find(|candidate| candidate.package_id == entry.package_id)
                .cloned()
                .ok_or_else(|| {
                    "operator_certification_topology_migration_predecessor_missing".to_owned()
                })?;
            let canonical_role_topology_id_sha256 = entry.role_topology_id_sha256.clone();
            ledger
                .migrate_role_topology(
                    entry,
                    &previous.role_topology_id_sha256,
                    &canonical_role_topology_id_sha256,
                )
                .map(|_| ())
                .map_err(str::to_owned)
        }
        Err(error) => Err(error.to_owned()),
    }
}

fn persist_event(root: &Path, event: &OperatorCertificationJournalEventV1) -> Result<(), String> {
    let directory = root.join(JOURNAL_DIR);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("operator_certification_journal_parent:{error}"))?;
    let path = directory.join(format!("{:020}.json", event.sequence));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o640);
    let mut file = options
        .open(&path)
        .map_err(|error| format!("operator_certification_journal_create:{error}"))?;
    let bytes = serde_json::to_vec(event)
        .map_err(|error| format!("operator_certification_journal_encode:{error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("operator_certification_journal_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("operator_certification_journal_sync:{error}"))?;
    File::open(&directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("operator_certification_journal_dir_sync:{error}"))
}

fn persist_anchor(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    ledger: &OperatorCertificationLedgerV1,
    last_event_root: &str,
) -> Result<(), String> {
    let anchor = OperatorCertificationAnchorV1::seal(
        ledger.revision,
        last_event_root,
        &ledger.ledger_root_sha256,
        signing_key,
    )
    .map_err(str::to_owned)?;
    let parent = config
        .anchor_path
        .parent()
        .ok_or_else(|| "operator_certification_anchor_parent_missing".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("operator_certification_anchor_parent:{error}"))?;
    write_bytes_atomic(
        &config.anchor_path,
        &serde_json::to_vec(&anchor)
            .map_err(|error| format!("operator_certification_anchor_encode:{error}"))?,
        "operator-certification-anchor",
    )
}

fn persist_cache(root: &Path, ledger: &OperatorCertificationLedgerV1) -> Result<(), String> {
    write_bytes_atomic(
        &root.join(LEDGER_CACHE_FILE),
        &serde_json::to_vec(ledger)
            .map_err(|error| format!("operator_certification_cache_encode:{error}"))?,
        "operator-certification-ledger-cache",
    )
}

fn projection_for(
    ledger: &OperatorCertificationLedgerV1,
    package_id: &str,
) -> Result<CertificationProjectionV1, String> {
    let entry = ledger
        .latest_entries()
        .into_iter()
        .find(|candidate| candidate.package_id == package_id)
        .cloned()
        .ok_or_else(|| "operator_certification_projection_missing".to_owned())?;
    Ok(CertificationProjectionV1 {
        ledger_root_sha256: ledger.ledger_root_sha256.clone(),
        entry,
        k1_vocabulary_gate: ledger.k1_vocabulary_gate().map_err(str::to_owned)?,
    })
}

fn journal_genesis_root() -> String {
    format!(
        "{:x}",
        Sha256::digest(b"nando.operator-certification-journal-genesis.v1")
    )
}

pub fn read_signing_key(path: &Path) -> Result<SigningKey, String> {
    let bytes = read_hex_file::<32>(path, "operator_certification_private_key")?;
    Ok(SigningKey::from_bytes(&bytes))
}

pub fn read_verifying_key(path: &Path) -> Result<VerifyingKey, String> {
    let bytes = read_hex_file::<32>(path, "operator_certification_public_key")?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|_| "operator_certification_public_key_invalid".to_owned())
}

fn read_hex_file<const N: usize>(path: &Path, label: &str) -> Result<[u8; N], String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("{label}_read:{error}"))?
        .trim()
        .to_owned();
    if value.len() != N * 2 {
        return Err(format!("{label}_length_invalid"));
    }
    let mut output = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_digit(pair[0]).ok_or_else(|| format!("{label}_encoding_invalid"))?;
        let low = hex_digit(pair[1]).ok_or_else(|| format!("{label}_encoding_invalid"))?;
        output[index] = high << 4 | low;
    }
    Ok(output)
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use nando_operator_admission::{
        ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1,
        LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
        OperatorMechanismClassV1,
    };

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn root(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-operator-certification-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn entry() -> OperatorCertificationEntryV1 {
        let bundle = root('a');
        let package = "package-one";
        OperatorCertificationEntryV1::seal(
            &bundle,
            package,
            &root('b'),
            &root('c'),
            ExecutionCertificateV1::seal(
                &bundle,
                package,
                ExecutionCertificateStatusV1::Pass,
                vec![root('d')],
                "",
            )
            .expect("execution"),
            LawCertificateV1::seal(
                &bundle,
                package,
                LawCertificateStatusV1::Partial,
                vec![root('e')],
                None,
                "exact_memory_cleanup_receipt_missing",
            )
            .expect("law"),
            MechanismCertificateV1::seal(
                &bundle,
                package,
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
                vec![root('f')],
                "post_center_holdout_collecting",
            )
            .expect("mechanism"),
            0,
        )
        .expect("entry")
    }

    fn failed_mechanism_entry(
        previous: &OperatorCertificationEntryV1,
    ) -> OperatorCertificationEntryV1 {
        OperatorCertificationEntryV1::seal(
            &previous.bundle_id_sha256,
            &previous.package_id,
            &previous.semantic_law_id_sha256,
            &previous.role_topology_id_sha256,
            previous.execution.clone(),
            previous.law.clone(),
            MechanismCertificateV1::seal(
                &previous.bundle_id_sha256,
                &previous.package_id,
                MechanismCertificateStatusV1::Fail,
                OperatorMechanismClassV1::Unresolved,
                vec![root('9')],
                "wave_causal_not_proven",
            )
            .expect("failed mechanism"),
            previous.false_bad_apply,
        )
        .expect("failed entry")
    }

    fn revoked_entry(previous: &OperatorCertificationEntryV1) -> OperatorCertificationEntryV1 {
        OperatorCertificationEntryV1::seal(
            &previous.bundle_id_sha256,
            &previous.package_id,
            &previous.semantic_law_id_sha256,
            &previous.role_topology_id_sha256,
            ExecutionCertificateV1::seal(
                &previous.bundle_id_sha256,
                &previous.package_id,
                ExecutionCertificateStatusV1::Revoked,
                vec![root('1')],
                "runtime_false_bad_apply",
            )
            .expect("revoked execution"),
            previous.law.clone(),
            previous.mechanism.clone(),
            1,
        )
        .expect("revoked entry")
    }

    #[test]
    fn signed_journal_detects_physical_rollback() {
        let root = test_root();
        fs::create_dir_all(&root).expect("root");
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_path = root.join("authority.pub");
        fs::write(
            &public_path,
            encode_hex(signing_key.verifying_key().as_bytes()),
        )
        .expect("public key");
        let config = CertificationAuthorityConfigV1 {
            root: root.join("state"),
            cleanup_receipts_path: root.join(CLEANUP_DIR),
            anchor_path: root.join("external-anchor/anchor.json"),
            authority_socket_path: root.join("authority.sock"),
            authority_public_key_path: public_path.clone(),
            cleanup_public_key_path: public_path,
            response_registry_path: root.join("registry.json"),
            runtime_revocations_path: root.join("revocations.json"),
        };
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("empty");
        let first = entry();
        ledger.append(first.clone()).expect("append");
        let event = OperatorCertificationJournalEventV1::seal(
            1,
            &journal_genesis_root(),
            first,
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("event");
        persist_event(&config.root, &event).expect("persist event");
        persist_anchor(&config, &signing_key, &ledger, &event.event_root_sha256)
            .expect("persist anchor");
        assert_eq!(restore_anchored_ledger(&config).expect("restore"), ledger);

        fs::remove_file(
            config
                .root
                .join(JOURNAL_DIR)
                .join("00000000000000000001.json"),
        )
        .expect("remove event");
        assert_eq!(
            restore_anchored_ledger(&config),
            Err("operator_certification_orphan_anchor".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn signed_tail_recovers_after_crash_but_old_anchor_cannot_return() {
        let root = test_root();
        fs::create_dir_all(&root).expect("root");
        let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
        let public_path = root.join("authority.pub");
        fs::write(
            &public_path,
            encode_hex(signing_key.verifying_key().as_bytes()),
        )
        .expect("public key");
        let config = CertificationAuthorityConfigV1 {
            root: root.join("state"),
            cleanup_receipts_path: root.join(CLEANUP_DIR),
            anchor_path: root.join("external-anchor/anchor.json"),
            authority_socket_path: root.join("authority.sock"),
            authority_public_key_path: public_path.clone(),
            cleanup_public_key_path: public_path,
            response_registry_path: root.join("registry.json"),
            runtime_revocations_path: root.join("revocations.json"),
        };
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("empty");
        let first = entry();
        ledger.append(first.clone()).expect("first append");
        let first_event = OperatorCertificationJournalEventV1::seal(
            1,
            &journal_genesis_root(),
            first.clone(),
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("first event");
        persist_event(&config.root, &first_event).expect("persist first");
        persist_anchor(
            &config,
            &signing_key,
            &ledger,
            &first_event.event_root_sha256,
        )
        .expect("first anchor");
        let old_anchor = fs::read(&config.anchor_path).expect("old anchor");

        let second = failed_mechanism_entry(&first);
        ledger.append(second.clone()).expect("second append");
        let second_event = OperatorCertificationJournalEventV1::seal(
            2,
            &first_event.event_root_sha256,
            second,
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("second event");
        persist_event(&config.root, &second_event).expect("persist crash tail");
        assert_eq!(
            restore_anchored_ledger(&config),
            Err("operator_certification_rollback_detected".to_owned())
        );
        recover_anchor(&config, &signing_key).expect("signed tail recovery");
        assert_eq!(restore_anchored_ledger(&config).expect("recovered"), ledger);

        fs::write(&config.anchor_path, old_anchor).expect("restore old anchor");
        assert_eq!(
            restore_anchored_ledger(&config),
            Err("operator_certification_rollback_detected".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn signed_topology_migration_replays_after_restart() {
        let temp_root = test_root();
        fs::create_dir_all(&temp_root).expect("root");
        let signing_key = SigningKey::from_bytes(&[11_u8; 32]);
        let public_path = temp_root.join("authority.pub");
        fs::write(
            &public_path,
            encode_hex(signing_key.verifying_key().as_bytes()),
        )
        .expect("public key");
        let config = CertificationAuthorityConfigV1 {
            root: temp_root.join("state"),
            cleanup_receipts_path: temp_root.join(CLEANUP_DIR),
            anchor_path: temp_root.join("external-anchor/anchor.json"),
            authority_socket_path: temp_root.join("authority.sock"),
            authority_public_key_path: public_path.clone(),
            cleanup_public_key_path: public_path,
            response_registry_path: temp_root.join("registry.json"),
            runtime_revocations_path: temp_root.join("revocations.json"),
        };
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("empty");
        let legacy = entry();
        ledger.append(legacy.clone()).expect("legacy append");
        let legacy_event = OperatorCertificationJournalEventV1::seal(
            1,
            &journal_genesis_root(),
            legacy.clone(),
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("legacy event");
        persist_event(&config.root, &legacy_event).expect("persist legacy");

        let canonical_topology = root('8');
        let migrated = OperatorCertificationEntryV1::seal(
            &legacy.bundle_id_sha256,
            &legacy.package_id,
            &legacy.semantic_law_id_sha256,
            &canonical_topology,
            legacy.execution.clone(),
            legacy.law.clone(),
            legacy.mechanism.clone(),
            legacy.false_bad_apply,
        )
        .expect("migrated entry");
        ledger
            .migrate_role_topology(
                migrated.clone(),
                &legacy.role_topology_id_sha256,
                &canonical_topology,
            )
            .expect("topology migration");
        let migrated_event = OperatorCertificationJournalEventV1::seal(
            2,
            &legacy_event.event_root_sha256,
            migrated,
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("migrated event");
        persist_event(&config.root, &migrated_event).expect("persist migration");
        persist_anchor(
            &config,
            &signing_key,
            &ledger,
            &migrated_event.event_root_sha256,
        )
        .expect("persist anchor");

        assert_eq!(restore_anchored_ledger(&config).expect("restart"), ledger);
        fs::remove_dir_all(temp_root).expect("cleanup");
    }

    #[test]
    fn nonempty_journal_without_external_anchor_fails_closed() {
        let root = test_root();
        fs::create_dir_all(&root).expect("root");
        let signing_key = SigningKey::from_bytes(&[13_u8; 32]);
        let public_path = root.join("authority.pub");
        fs::write(
            &public_path,
            encode_hex(signing_key.verifying_key().as_bytes()),
        )
        .expect("public key");
        let config = CertificationAuthorityConfigV1 {
            root: root.join("state"),
            cleanup_receipts_path: root.join(CLEANUP_DIR),
            anchor_path: root.join("external-anchor/anchor.json"),
            authority_socket_path: root.join("authority.sock"),
            authority_public_key_path: public_path.clone(),
            cleanup_public_key_path: public_path,
            response_registry_path: root.join("registry.json"),
            runtime_revocations_path: root.join("revocations.json"),
        };
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("empty");
        let first = entry();
        ledger.append(first.clone()).expect("append");
        let event = OperatorCertificationJournalEventV1::seal(
            1,
            &journal_genesis_root(),
            first,
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("event");
        persist_event(&config.root, &event).expect("persist event");

        assert_eq!(
            recover_anchor(&config, &signing_key),
            Err("operator_certification_anchor_missing_for_nonempty_journal".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn stale_projection_accepts_safe_append_but_rejects_later_revocation() {
        let root = test_root();
        fs::create_dir_all(&root).expect("root");
        let signing_key = SigningKey::from_bytes(&[15_u8; 32]);
        let public_path = root.join("authority.pub");
        fs::write(
            &public_path,
            encode_hex(signing_key.verifying_key().as_bytes()),
        )
        .expect("public key");
        let config = CertificationAuthorityConfigV1 {
            root: root.join("state"),
            cleanup_receipts_path: root.join(CLEANUP_DIR),
            anchor_path: root.join("external-anchor/anchor.json"),
            authority_socket_path: root.join("authority.sock"),
            authority_public_key_path: public_path.clone(),
            cleanup_public_key_path: public_path,
            response_registry_path: root.join("registry.json"),
            runtime_revocations_path: root.join("revocations.json"),
        };
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("empty");
        let first = entry();
        ledger.append(first.clone()).expect("first append");
        let first_projection = projection_for(&ledger, &first.package_id).expect("projection");
        let first_event = OperatorCertificationJournalEventV1::seal(
            1,
            &journal_genesis_root(),
            first.clone(),
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("first event");
        persist_event(&config.root, &first_event).expect("persist first");

        let second = failed_mechanism_entry(&first);
        ledger.append(second.clone()).expect("second append");
        let second_event = OperatorCertificationJournalEventV1::seal(
            2,
            &first_event.event_root_sha256,
            second.clone(),
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("second event");
        persist_event(&config.root, &second_event).expect("persist second");
        persist_anchor(
            &config,
            &signing_key,
            &ledger,
            &second_event.event_root_sha256,
        )
        .expect("second anchor");
        validate_projection(&config, &first_projection).expect("safe stale prefix");

        let revoked = revoked_entry(&second);
        ledger.append(revoked.clone()).expect("revocation append");
        let revoked_event = OperatorCertificationJournalEventV1::seal(
            3,
            &second_event.event_root_sha256,
            revoked,
            &ledger.ledger_root_sha256,
            &signing_key,
        )
        .expect("revocation event");
        persist_event(&config.root, &revoked_event).expect("persist revocation");
        persist_anchor(
            &config,
            &signing_key,
            &ledger,
            &revoked_event.event_root_sha256,
        )
        .expect("revocation anchor");
        assert_eq!(
            validate_projection(&config, &first_projection),
            Err("operator_certification_stale_safety_projection".to_owned())
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn encode_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
