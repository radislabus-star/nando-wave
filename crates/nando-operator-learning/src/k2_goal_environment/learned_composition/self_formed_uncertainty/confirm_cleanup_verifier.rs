use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    CLEANUP_AFTER_CENSUS_PAGE_DIRECTORY_V1, K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_CLEANUP_VERIFY_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2UncertaintyCleanupAfterCensusEntryV1, K2UncertaintyCleanupAfterCensusPageV1,
    K2UncertaintyCleanupAuthorizationReceiptV1, K2UncertaintyCleanupClassifiedPathV1,
    K2UncertaintyCleanupFileKindV1, K2UncertaintyCleanupManifestV1,
    K2UncertaintyCleanupOwnerReceiptV1, K2UncertaintyCleanupVerifierFaultV1,
    K2UncertaintyRetentionClassV1, denied_authority_v1, load_self_formed_cleanup_manifest_pages_v1,
    paginate_cleanup_after_census_v1, publish_control_bytes_v1, require_denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
    validate_cleanup_event_chain_v1, validate_cleanup_manifest_pages_v1, validate_sibling_roots_v1,
    walk_governed_root_v1,
};

const CLEANUP_MANIFEST_FILE_V1: &str = "cleanup-manifest.json";
const CLEANUP_PAGE_DIRECTORY_V1: &str = "cleanup-manifest-pages";
const CLEANUP_AUTHORIZATION_FILE_V1: &str = "cleanup-authorization.json";
const CLEANUP_EVENT_DIRECTORY_V1: &str = "cleanup-events";
const CLEANUP_OWNER_RECEIPT_FILE_V1: &str = "cleanup-owner-receipt.json";
const CLEANUP_RECEIPT_FILE_V1: &str = "cleanup-frozen.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupVerifyRequestV1 {
    pub schema: String,
    pub governed_root: String,
    pub control_root: String,
    pub before_manifest: K2UncertaintyCleanupManifestV1,
    pub owner_receipt: K2UncertaintyCleanupOwnerReceiptV1,
    pub verifier_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyCleanupVerifyRequestV1 {
    pub fn seal(
        governed_root: String,
        control_root: String,
        before_manifest: K2UncertaintyCleanupManifestV1,
        owner_receipt: K2UncertaintyCleanupOwnerReceiptV1,
        verifier_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_VERIFY_REQUEST_SCHEMA_V1.to_owned(),
            governed_root,
            control_root,
            before_manifest,
            owner_receipt,
            verifier_executable_sha256,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.before_manifest.validate()?;
        self.owner_receipt.validate()?;
        require_composition_root_v1(&self.verifier_executable_sha256)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_VERIFY_REQUEST_SCHEMA_V1
            || self.governed_root.is_empty()
            || self.control_root.is_empty()
            || self.governed_root == self.control_root
            || self.owner_receipt.authorization_root_sha256.is_empty()
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_verify_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_VERIFY_REQUEST_SCHEMA_V1,
            &self.governed_root,
            &self.control_root,
            &self.before_manifest.manifest_root_sha256,
            &self.owner_receipt.receipt_root_sha256,
            &self.verifier_executable_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupReceiptV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub before_manifest_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub after_census_root_sha256: String,
    pub control_manifest_root_sha256: String,
    pub owner_receipt_root_sha256: String,
    pub retained_paths: u64,
    pub deleted_paths: u64,
    pub unexpected_residue: u64,
    pub cleanup_frozen: bool,
    pub verifier_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCleanupReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.before_manifest_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.after_census_root_sha256,
            &self.control_manifest_root_sha256,
            &self.owner_receipt_root_sha256,
            &self.verifier_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1
            || !self.cleanup_frozen
            || self.deleted_paths == 0
            || self.unexpected_residue != 0
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_receipt_invalid",
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
            K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1,
            &self.request_root_sha256,
            &self.before_manifest_root_sha256,
            &self.terminal_receipt_root_sha256,
            &self.after_census_root_sha256,
            &self.control_manifest_root_sha256,
            &self.owner_receipt_root_sha256,
            self.retained_paths,
            self.deleted_paths,
            self.unexpected_residue,
            self.cleanup_frozen,
            &self.verifier_executable_sha256,
            &self.authority,
        ))
    }
}

pub fn verify_self_formed_cleanup_v1(
    request: &K2UncertaintyCleanupVerifyRequestV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupReceiptV1> {
    verify_self_formed_cleanup_with_fault_v1(request, K2UncertaintyCleanupVerifierFaultV1::None)
}

pub(crate) fn verify_self_formed_cleanup_with_fault_v1(
    request: &K2UncertaintyCleanupVerifyRequestV1,
    fault: K2UncertaintyCleanupVerifierFaultV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupReceiptV1> {
    request.validate()?;
    let governed_root = Path::new(&request.governed_root);
    let control_root = Path::new(&request.control_root);
    validate_sibling_roots_v1(governed_root, control_root)?;
    let pages = load_self_formed_cleanup_manifest_pages_v1(control_root, &request.before_manifest)?;
    validate_cleanup_manifest_pages_v1(&request.before_manifest, &pages)?;
    validate_cleanup_event_chain_v1(&request.owner_receipt.events)?;

    let before_entries = pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let observed = walk_governed_root_v1(governed_root)?;
    let mut after_projection = Vec::with_capacity(observed.len());
    for (relative_path, path, metadata) in observed {
        let expected = before_entries
            .get(&relative_path)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_unexpected_residue",
            ))?;
        if expected.retention == K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_disposable_residue_present",
            ));
        }
        validate_retained_identity_v1(&path, &metadata, expected)?;
        after_projection.push(K2UncertaintyCleanupAfterCensusEntryV1::seal(
            relative_path,
            expected.file_kind,
            expected.content_sha256.clone(),
            metadata.mode() & 0o7777,
            if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
        )?);
    }
    let observed_paths = after_projection
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect::<BTreeSet<_>>();
    let mut retained_paths = 0_u64;
    let mut deleted_paths = 0_u64;
    for entry in before_entries.values() {
        match entry.retention {
            K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync => {
                deleted_paths += 1;
                if observed_paths.contains(entry.relative_path.as_str()) {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_cleanup_disposable_residue_present",
                    ));
                }
            }
            _ => {
                retained_paths += 1;
                if !observed_paths.contains(entry.relative_path.as_str()) {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_cleanup_retained_path_missing",
                    ));
                }
            }
        }
    }
    if deleted_paths != request.owner_receipt.deleted_paths {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_deleted_denominator_invalid",
        ));
    }
    let after_pages = paginate_cleanup_after_census_v1(after_projection)?;
    let after_page_root = control_root.join(CLEANUP_AFTER_CENSUS_PAGE_DIRECTORY_V1);
    fs::create_dir_all(&after_page_root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_after_census_page_root"))?;
    fs::set_permissions(&after_page_root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_after_census_page_root"))?;
    for (page_index, page) in after_pages.iter().enumerate() {
        publish_control_bytes_v1(
            &after_page_root.join(format!("{}.json", page.page_root_sha256)),
            &uncertainty_bytes_v1(page)?,
        )?;
        fail_cleanup_verifier_at_v1(
            fault,
            K2UncertaintyCleanupVerifierFaultV1::AfterCensusPage { page: page_index },
        )?;
    }
    let after_census_root_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-cleanup-after-census.v1",
        after_pages
            .iter()
            .map(|page| &page.page_root_sha256)
            .collect::<Vec<_>>(),
        after_pages
            .iter()
            .map(|page| page.entries.len() as u64)
            .sum::<u64>(),
    ))?;
    let (control_manifest_root_sha256, terminal_receipt_root_sha256) =
        validate_control_root_v1(control_root, request, &pages, &after_pages)?;
    let mut receipt = K2UncertaintyCleanupReceiptV1 {
        schema: K2_UNCERTAINTY_CLEANUP_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        before_manifest_root_sha256: request.before_manifest.manifest_root_sha256.clone(),
        terminal_receipt_root_sha256,
        after_census_root_sha256,
        control_manifest_root_sha256,
        owner_receipt_root_sha256: request.owner_receipt.receipt_root_sha256.clone(),
        retained_paths,
        deleted_paths,
        unexpected_residue: 0,
        cleanup_frozen: true,
        verifier_executable_sha256: request.verifier_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    fail_cleanup_verifier_at_v1(fault, K2UncertaintyCleanupVerifierFaultV1::BeforeReceipt)?;
    publish_control_bytes_v1(
        &control_root.join(CLEANUP_RECEIPT_FILE_V1),
        &uncertainty_bytes_v1(&receipt)?,
    )?;
    fail_cleanup_verifier_at_v1(fault, K2UncertaintyCleanupVerifierFaultV1::AfterReceipt)?;
    Ok(receipt)
}

fn fail_cleanup_verifier_at_v1(
    actual: K2UncertaintyCleanupVerifierFaultV1,
    expected: K2UncertaintyCleanupVerifierFaultV1,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_verifier_injected_fault",
        ));
    }
    Ok(())
}

fn validate_retained_identity_v1(
    path: &Path,
    metadata: &fs::Metadata,
    expected: &K2UncertaintyCleanupClassifiedPathV1,
) -> K2CompositionResultV1<()> {
    let kind_matches = match expected.file_kind {
        K2UncertaintyCleanupFileKindV1::Regular => metadata.is_file(),
        K2UncertaintyCleanupFileKindV1::Directory => metadata.is_dir(),
    };
    if !kind_matches || metadata.mode() & 0o7777 != expected.mode {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_retained_identity_mismatch",
        ));
    }
    if metadata.is_file()
        && (metadata.len() != expected.size_bytes
            || Some(composition_sha256_file_v1(path)?) != expected.content_sha256)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_retained_content_mismatch",
        ));
    }
    Ok(())
}

fn validate_control_root_v1(
    control_root: &Path,
    request: &K2UncertaintyCleanupVerifyRequestV1,
    pages: &[super::K2UncertaintyCleanupManifestPageV1],
    after_pages: &[K2UncertaintyCleanupAfterCensusPageV1],
) -> K2CompositionResultV1<(String, String)> {
    let manifest: K2UncertaintyCleanupManifestV1 = decode_exact_file_v1(
        &control_root.join(CLEANUP_MANIFEST_FILE_V1),
        &request.before_manifest,
    )?;
    let authorization: K2UncertaintyCleanupAuthorizationReceiptV1 = uncertainty_decode_v1(
        &fs::read(control_root.join(CLEANUP_AUTHORIZATION_FILE_V1))
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_authorization"))?,
    )?;
    authorization.validate()?;
    if authorization.receipt_root_sha256 != request.owner_receipt.authorization_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_authorization_root_mismatch",
        ));
    }
    if authorization.before_manifest_root_sha256 != request.before_manifest.manifest_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_before_manifest_root_mismatch",
        ));
    }
    let owner: K2UncertaintyCleanupOwnerReceiptV1 = decode_exact_file_v1(
        &control_root.join(CLEANUP_OWNER_RECEIPT_FILE_V1),
        &request.owner_receipt,
    )?;
    let mut expected_files = BTreeSet::from([
        CLEANUP_MANIFEST_FILE_V1.to_owned(),
        CLEANUP_AUTHORIZATION_FILE_V1.to_owned(),
        CLEANUP_OWNER_RECEIPT_FILE_V1.to_owned(),
    ]);
    for page in pages {
        expected_files.insert(format!(
            "{CLEANUP_PAGE_DIRECTORY_V1}/{}.json",
            page.page_root_sha256
        ));
    }
    for page in after_pages {
        let relative_path = format!(
            "{CLEANUP_AFTER_CENSUS_PAGE_DIRECTORY_V1}/{}.json",
            page.page_root_sha256
        );
        let _: K2UncertaintyCleanupAfterCensusPageV1 =
            decode_exact_file_v1(&control_root.join(&relative_path), page)?;
        expected_files.insert(relative_path);
    }
    for event in &owner.events {
        expected_files.insert(format!(
            "{CLEANUP_EVENT_DIRECTORY_V1}/{:020}.json",
            event.sequence
        ));
    }
    let mut actual_files = walk_control_root_v1(control_root)?;
    actual_files.remove(CLEANUP_RECEIPT_FILE_V1);
    if actual_files != expected_files {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_control_manifest_incomplete",
        ));
    }
    let control_manifest_root = uncertainty_root_v1(&(
        "nando.k2-self-formed-cleanup-control-manifest.v1",
        &manifest.manifest_root_sha256,
        &authorization.receipt_root_sha256,
        &owner.receipt_root_sha256,
        actual_files,
    ))?;
    Ok((
        control_manifest_root,
        authorization.terminal_receipt_root_sha256,
    ))
}

fn decode_exact_file_v1<T>(path: &Path, expected: &T) -> K2CompositionResultV1<T>
where
    T: serde::de::DeserializeOwned + Serialize + Clone + Eq,
{
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_cleanup_control_artifact"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.mode() & 0o7777 != 0o400
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_control_artifact_mode_invalid",
        ));
    }
    let decoded: T = uncertainty_decode_v1(
        &fs::read(path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_control_artifact"))?,
    )?;
    if &decoded != expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_control_artifact_mismatch",
        ));
    }
    Ok(decoded)
}

fn walk_control_root_v1(root: &Path) -> K2CompositionResultV1<BTreeSet<String>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_cleanup_control_directory"))?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.mode() & 0o7777 != 0o700
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_control_directory_invalid",
            ));
        }
        for entry in fs::read_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_control_directory"))?
        {
            let entry = entry
                .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_control_entry"))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_cleanup_control_entry"))?;
            if metadata.file_type().is_symlink() {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_control_symlink_rejected",
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() && metadata.mode() & 0o7777 == 0o400 {
                files.insert(
                    path.strip_prefix(root)
                        .map_err(|_| {
                            K2CompositionErrorV1::Invalid("self_formed_cleanup_control_escape")
                        })?
                        .to_str()
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_cleanup_control_path_not_utf8",
                        ))?
                        .to_owned(),
                );
            } else {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_control_residue_invalid",
                ));
            }
        }
    }
    Ok(files)
}

pub fn run_self_formed_cleanup_verifier_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_verifier_stdin"))?;
    let request: K2UncertaintyCleanupVerifyRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_cleanup_verifier"))?;
    if composition_sha256_file_v1(&executable)? != request.verifier_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_verifier_executable_mismatch",
        ));
    }
    let receipt = verify_self_formed_cleanup_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_cleanup_verifier_stdout"))
}
