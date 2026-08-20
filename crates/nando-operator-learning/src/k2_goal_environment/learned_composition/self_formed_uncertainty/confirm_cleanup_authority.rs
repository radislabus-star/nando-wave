use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLEANUP_AUTH_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_CLEANUP_AUTH_REQUEST_SCHEMA_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyCleanupClassifiedPathV1,
    K2UncertaintyCleanupFileKindV1, K2UncertaintyCleanupManifestV1, K2UncertaintyRetentionClassV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalModeV1, denied_authority_v1, load_self_formed_cleanup_manifest_pages_v1,
    publish_control_bytes_v1, require_denied_authority_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1, validate_cleanup_manifest_pages_v1,
};

const CLEANUP_AUTHORIZATION_FILE_V1: &str = "cleanup-authorization.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K2UncertaintyCleanupAuthorizationFaultV1 {
    None,
    BeforeReceipt,
    AfterReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupAuthorizationRequestV1 {
    pub schema: String,
    pub control_root: String,
    pub experiment_root_sha256: String,
    pub terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1,
    pub before_manifest: K2UncertaintyCleanupManifestV1,
    pub journal_projection_root_sha256: String,
    pub observer_durable_event_root_sha256: String,
    pub terminal_durable_event_root_sha256: String,
    pub sealed_attempts: u64,
    pub authorization_slots: u64,
    pub confirm_nonce_commitment_sha256: Option<String>,
    pub authorizer_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyCleanupAuthorizationRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        control_root: String,
        experiment_root_sha256: String,
        terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1,
        before_manifest: K2UncertaintyCleanupManifestV1,
        journal_projection_root_sha256: String,
        observer_durable_event_root_sha256: String,
        terminal_durable_event_root_sha256: String,
        authorizer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_AUTH_REQUEST_SCHEMA_V1.to_owned(),
            control_root,
            experiment_root_sha256,
            terminal_receipt,
            before_manifest,
            journal_projection_root_sha256,
            observer_durable_event_root_sha256,
            terminal_durable_event_root_sha256,
            sealed_attempts: 0,
            authorization_slots: 0,
            confirm_nonce_commitment_sha256: None,
            authorizer_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.terminal_receipt.validate()?;
        self.before_manifest.validate()?;
        for root in [
            &self.experiment_root_sha256,
            &self.journal_projection_root_sha256,
            &self.observer_durable_event_root_sha256,
            &self.terminal_durable_event_root_sha256,
            &self.authorizer_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_AUTH_REQUEST_SCHEMA_V1
            || self.control_root.is_empty()
            || self.experiment_root_sha256 != self.before_manifest.experiment_root_sha256
            || self.terminal_receipt.mode != K2UncertaintyTerminalModeV1::DevelopmentRehearsal
            || self.terminal_receipt.disposition
                != K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass
            || self.sealed_attempts != 0
            || self.authorization_slots != 0
            || self.confirm_nonce_commitment_sha256.is_some()
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_authorization_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_AUTH_REQUEST_SCHEMA_V1,
            &self.control_root,
            &self.experiment_root_sha256,
            &self.terminal_receipt.receipt_root_sha256,
            &self.before_manifest.manifest_root_sha256,
            &self.journal_projection_root_sha256,
            &self.observer_durable_event_root_sha256,
            &self.terminal_durable_event_root_sha256,
            self.sealed_attempts,
            self.authorization_slots,
            &self.confirm_nonce_commitment_sha256,
            &self.authorizer_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupAuthorizationReceiptV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub experiment_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub before_manifest_root_sha256: String,
    pub disposable_entries: Vec<K2UncertaintyCleanupClassifiedPathV1>,
    pub authorizer_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCleanupAuthorizationReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.experiment_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.before_manifest_root_sha256,
            &self.authorizer_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let mut paths = BTreeSet::new();
        for entry in &self.disposable_entries {
            entry.validate()?;
            if entry.retention != K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync
                || !paths.insert(&entry.relative_path)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_authorization_target_invalid",
                ));
            }
        }
        if self.schema != K2_UNCERTAINTY_CLEANUP_AUTH_RECEIPT_SCHEMA_V1
            || self.disposable_entries.is_empty()
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_authorization_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_AUTH_RECEIPT_SCHEMA_V1,
            &self.request_root_sha256,
            &self.experiment_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.before_manifest_root_sha256,
            &self.disposable_entries,
            &self.authorizer_executable_sha256,
            &self.authority,
        ))
    }
}

pub fn authorize_self_formed_cleanup_v1(
    request: &K2UncertaintyCleanupAuthorizationRequestV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupAuthorizationReceiptV1> {
    authorize_self_formed_cleanup_with_fault_v1(
        request,
        K2UncertaintyCleanupAuthorizationFaultV1::None,
    )
}

pub(crate) fn authorize_self_formed_cleanup_with_fault_v1(
    request: &K2UncertaintyCleanupAuthorizationRequestV1,
    fault: K2UncertaintyCleanupAuthorizationFaultV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupAuthorizationReceiptV1> {
    request.validate()?;
    let pages = load_self_formed_cleanup_manifest_pages_v1(
        Path::new(&request.control_root),
        &request.before_manifest,
    )?;
    validate_cleanup_manifest_pages_v1(&request.before_manifest, &pages)?;
    let mut disposable_entries = pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .filter(|entry| {
            entry.retention == K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync
        })
        .cloned()
        .collect::<Vec<_>>();
    disposable_entries.sort_by(|left, right| match (left.file_kind, right.file_kind) {
        (K2UncertaintyCleanupFileKindV1::Regular, K2UncertaintyCleanupFileKindV1::Directory) => {
            std::cmp::Ordering::Less
        }
        (K2UncertaintyCleanupFileKindV1::Directory, K2UncertaintyCleanupFileKindV1::Regular) => {
            std::cmp::Ordering::Greater
        }
        (K2UncertaintyCleanupFileKindV1::Directory, K2UncertaintyCleanupFileKindV1::Directory) => {
            right
                .relative_path
                .matches('/')
                .count()
                .cmp(&left.relative_path.matches('/').count())
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        }
        _ => left.relative_path.cmp(&right.relative_path),
    });
    let mut receipt = K2UncertaintyCleanupAuthorizationReceiptV1 {
        schema: K2_UNCERTAINTY_CLEANUP_AUTH_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        experiment_root_sha256: request.experiment_root_sha256.clone(),
        terminal_receipt_root_sha256: request.terminal_receipt.receipt_root_sha256.clone(),
        before_manifest_root_sha256: request.before_manifest.manifest_root_sha256.clone(),
        disposable_entries,
        authorizer_executable_sha256: request.authorizer_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    fail_cleanup_authorization_at_v1(
        fault,
        K2UncertaintyCleanupAuthorizationFaultV1::BeforeReceipt,
    )?;
    publish_control_bytes_v1(
        &Path::new(&request.control_root).join(CLEANUP_AUTHORIZATION_FILE_V1),
        &uncertainty_bytes_v1(&receipt)?,
    )?;
    fail_cleanup_authorization_at_v1(
        fault,
        K2UncertaintyCleanupAuthorizationFaultV1::AfterReceipt,
    )?;
    Ok(receipt)
}

fn fail_cleanup_authorization_at_v1(
    actual: K2UncertaintyCleanupAuthorizationFaultV1,
    expected: K2UncertaintyCleanupAuthorizationFaultV1,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_authorization_injected_fault",
        ));
    }
    Ok(())
}

pub fn run_self_formed_cleanup_authorizer_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_authorizer_stdin"))?;
    let request: K2UncertaintyCleanupAuthorizationRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_cleanup_authorizer"))?;
    if composition_sha256_file_v1(&executable)? != request.authorizer_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_authorizer_executable_mismatch",
        ));
    }
    let receipt = authorize_self_formed_cleanup_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_cleanup_authorizer_stdout"))
}
