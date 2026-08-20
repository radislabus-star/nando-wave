use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLEANUP_EVENT_SCHEMA_V1, K2_UNCERTAINTY_CLEANUP_OWNER_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_CLEANUP_OWNER_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2UncertaintyCleanupAuthorizationReceiptV1, K2UncertaintyCleanupClassifiedPathV1,
    K2UncertaintyCleanupFileKindV1, denied_authority_v1, publish_control_bytes_v1,
    require_denied_authority_v1, uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
    validate_cleanup_relative_path_v1, validate_sibling_roots_v1,
};

const CLEANUP_EVENT_DIRECTORY_V1: &str = "cleanup-events";
const CLEANUP_OWNER_RECEIPT_FILE_V1: &str = "cleanup-owner-receipt.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum K2UncertaintyCleanupEventKindV1 {
    DeleteIntentFrozen,
    DeleteCompleteFrozen,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupEventV1 {
    pub schema: String,
    pub sequence: u64,
    pub previous_event_root_sha256: Option<String>,
    pub kind: K2UncertaintyCleanupEventKindV1,
    pub target: K2UncertaintyCleanupClassifiedPathV1,
    pub owner_executable_sha256: String,
    pub event_root_sha256: String,
}

impl K2UncertaintyCleanupEventV1 {
    pub fn seal(
        sequence: u64,
        previous_event_root_sha256: Option<String>,
        kind: K2UncertaintyCleanupEventKindV1,
        target: K2UncertaintyCleanupClassifiedPathV1,
        owner_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_EVENT_SCHEMA_V1.to_owned(),
            sequence,
            previous_event_root_sha256,
            kind,
            target,
            owner_executable_sha256,
            event_root_sha256: String::new(),
        };
        value.event_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if let Some(root) = &self.previous_event_root_sha256 {
            require_composition_root_v1(root)?;
        }
        self.target.validate()?;
        require_composition_root_v1(&self.owner_executable_sha256)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_EVENT_SCHEMA_V1
            || (self.sequence == 0) != self.previous_event_root_sha256.is_none()
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_event_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_EVENT_SCHEMA_V1,
            self.sequence,
            &self.previous_event_root_sha256,
            self.kind,
            &self.target,
            &self.owner_executable_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupOwnerRequestV1 {
    pub schema: String,
    pub governed_root: String,
    pub control_root: String,
    pub authorization: K2UncertaintyCleanupAuthorizationReceiptV1,
    pub owner_executable_sha256: String,
    pub request_root_sha256: String,
}

impl K2UncertaintyCleanupOwnerRequestV1 {
    pub fn seal(
        governed_root: String,
        control_root: String,
        authorization: K2UncertaintyCleanupAuthorizationReceiptV1,
        owner_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_OWNER_REQUEST_SCHEMA_V1.to_owned(),
            governed_root,
            control_root,
            authorization,
            owner_executable_sha256,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.authorization.validate()?;
        require_composition_root_v1(&self.owner_executable_sha256)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_OWNER_REQUEST_SCHEMA_V1
            || self.governed_root.is_empty()
            || self.control_root.is_empty()
            || self.governed_root == self.control_root
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_owner_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_OWNER_REQUEST_SCHEMA_V1,
            &self.governed_root,
            &self.control_root,
            &self.authorization.receipt_root_sha256,
            &self.owner_executable_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupOwnerReceiptV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub authorization_root_sha256: String,
    pub events: Vec<K2UncertaintyCleanupEventV1>,
    pub deleted_paths: u64,
    pub owner_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCleanupOwnerReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.authorization_root_sha256,
            &self.owner_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        validate_cleanup_event_chain_v1(&self.events)?;
        if self.schema != K2_UNCERTAINTY_CLEANUP_OWNER_RECEIPT_SCHEMA_V1
            || self.events.len() != self.deleted_paths as usize * 2
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_owner_receipt_invalid",
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
            K2_UNCERTAINTY_CLEANUP_OWNER_RECEIPT_SCHEMA_V1,
            &self.request_root_sha256,
            &self.authorization_root_sha256,
            &self.events,
            self.deleted_paths,
            &self.owner_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyCleanupFaultV1 {
    None,
    BeforeIntent { target: usize },
    AfterIntent { target: usize },
    AfterMutation { target: usize },
    AfterParentFsync { target: usize },
    AfterCompletion { target: usize },
}

pub fn execute_self_formed_cleanup_v1(
    request: &K2UncertaintyCleanupOwnerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupOwnerReceiptV1> {
    execute_self_formed_cleanup_with_fault_v1(request, K2UncertaintyCleanupFaultV1::None)
}

pub fn execute_self_formed_cleanup_with_fault_v1(
    request: &K2UncertaintyCleanupOwnerRequestV1,
    fault: K2UncertaintyCleanupFaultV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupOwnerReceiptV1> {
    request.validate()?;
    let governed_root = Path::new(&request.governed_root);
    let control_root = Path::new(&request.control_root);
    validate_sibling_roots_v1(governed_root, control_root)?;
    let event_root = control_root.join(CLEANUP_EVENT_DIRECTORY_V1);
    fs::create_dir_all(&event_root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_cleanup_event_root"))?;
    fs::set_permissions(&event_root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_cleanup_event_root"))?;
    let mut events = load_cleanup_events_v1(&event_root)?;
    validate_cleanup_event_prefix_v1(&events, &request.authorization.disposable_entries)?;

    for (target_index, target) in request.authorization.disposable_entries.iter().enumerate() {
        let intent_index = target_index * 2;
        let completion_index = intent_index + 1;
        if events.len() <= intent_index {
            fail_at_v1(
                fault,
                K2UncertaintyCleanupFaultV1::BeforeIntent {
                    target: target_index,
                },
            )?;
            let intent = K2UncertaintyCleanupEventV1::seal(
                intent_index as u64,
                events.last().map(|event| event.event_root_sha256.clone()),
                K2UncertaintyCleanupEventKindV1::DeleteIntentFrozen,
                target.clone(),
                request.owner_executable_sha256.clone(),
            )?;
            publish_cleanup_event_v1(&event_root, &intent)?;
            events.push(intent);
            fail_at_v1(
                fault,
                K2UncertaintyCleanupFaultV1::AfterIntent {
                    target: target_index,
                },
            )?;
        }
        let path = resolve_cleanup_target_v1(governed_root, &target.relative_path)?;
        if events.len() <= completion_index {
            if cleanup_path_exists_v1(&path)? {
                validate_cleanup_target_identity_v1(&path, target)?;
                match target.file_kind {
                    K2UncertaintyCleanupFileKindV1::Regular => fs::remove_file(&path)
                        .map_err(|_| K2CompositionErrorV1::Io("unlink_self_formed_cleanup_file"))?,
                    K2UncertaintyCleanupFileKindV1::Directory => {
                        fs::remove_dir(&path).map_err(|_| {
                            K2CompositionErrorV1::Io("rmdir_self_formed_cleanup_directory")
                        })?
                    }
                }
                fail_at_v1(
                    fault,
                    K2UncertaintyCleanupFaultV1::AfterMutation {
                        target: target_index,
                    },
                )?;
            }
            let parent = path.parent().ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_parent_missing",
            ))?;
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_cleanup_parent"))?;
            fail_at_v1(
                fault,
                K2UncertaintyCleanupFaultV1::AfterParentFsync {
                    target: target_index,
                },
            )?;
            let completion = K2UncertaintyCleanupEventV1::seal(
                completion_index as u64,
                events.last().map(|event| event.event_root_sha256.clone()),
                K2UncertaintyCleanupEventKindV1::DeleteCompleteFrozen,
                target.clone(),
                request.owner_executable_sha256.clone(),
            )?;
            publish_cleanup_event_v1(&event_root, &completion)?;
            events.push(completion);
            fail_at_v1(
                fault,
                K2UncertaintyCleanupFaultV1::AfterCompletion {
                    target: target_index,
                },
            )?;
        } else if cleanup_path_exists_v1(&path)? {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_completed_target_present",
            ));
        }
    }
    validate_cleanup_event_chain_v1(&events)?;
    let mut receipt = K2UncertaintyCleanupOwnerReceiptV1 {
        schema: K2_UNCERTAINTY_CLEANUP_OWNER_RECEIPT_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        authorization_root_sha256: request.authorization.receipt_root_sha256.clone(),
        events,
        deleted_paths: request.authorization.disposable_entries.len() as u64,
        owner_executable_sha256: request.owner_executable_sha256.clone(),
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    publish_control_bytes_v1(
        &control_root.join(CLEANUP_OWNER_RECEIPT_FILE_V1),
        &uncertainty_bytes_v1(&receipt)?,
    )?;
    Ok(receipt)
}

pub fn validate_cleanup_event_chain_v1(
    events: &[K2UncertaintyCleanupEventV1],
) -> K2CompositionResultV1<()> {
    if events.is_empty() || !events.len().is_multiple_of(2) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_event_denominator_invalid",
        ));
    }
    validate_cleanup_event_prefix_v1(events, &[])?;
    for pair in events.chunks_exact(2) {
        if pair[0].kind != K2UncertaintyCleanupEventKindV1::DeleteIntentFrozen
            || pair[1].kind != K2UncertaintyCleanupEventKindV1::DeleteCompleteFrozen
            || pair[0].target != pair[1].target
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_event_pair_invalid",
            ));
        }
    }
    Ok(())
}

fn validate_cleanup_event_prefix_v1(
    events: &[K2UncertaintyCleanupEventV1],
    expected_targets: &[K2UncertaintyCleanupClassifiedPathV1],
) -> K2CompositionResultV1<()> {
    let mut previous: Option<&str> = None;
    for (sequence, event) in events.iter().enumerate() {
        event.validate()?;
        if event.sequence != sequence as u64
            || event.previous_event_root_sha256.as_deref() != previous
            || (!expected_targets.is_empty()
                && expected_targets.get(sequence / 2) != Some(&event.target))
            || (sequence % 2 == 0
                && event.kind != K2UncertaintyCleanupEventKindV1::DeleteIntentFrozen)
            || (sequence % 2 == 1
                && event.kind != K2UncertaintyCleanupEventKindV1::DeleteCompleteFrozen)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_event_chain_invalid",
            ));
        }
        previous = Some(&event.event_root_sha256);
    }
    if events.len() > expected_targets.len() * 2 && !expected_targets.is_empty() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_event_prefix_oversized",
        ));
    }
    Ok(())
}

fn load_cleanup_events_v1(root: &Path) -> K2CompositionResultV1<Vec<K2UncertaintyCleanupEventV1>> {
    let mut paths = fs::read_dir(root)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_events"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_event_entry"))?;
    paths.sort_by_key(|entry| entry.file_name());
    let mut events = Vec::with_capacity(paths.len());
    for entry in paths {
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_cleanup_event"))?
            .is_file()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_event_residue_invalid",
            ));
        }
        let event: K2UncertaintyCleanupEventV1 = uncertainty_decode_v1(
            &fs::read(path)
                .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_event"))?,
        )?;
        events.push(event);
    }
    Ok(events)
}

fn publish_cleanup_event_v1(
    root: &Path,
    event: &K2UncertaintyCleanupEventV1,
) -> K2CompositionResultV1<()> {
    publish_control_bytes_v1(
        &root.join(format!("{:020}.json", event.sequence)),
        &uncertainty_bytes_v1(event)?,
    )
}

fn resolve_cleanup_target_v1(root: &Path, relative: &str) -> K2CompositionResultV1<PathBuf> {
    validate_cleanup_relative_path_v1(relative)?;
    let mut current = root.to_path_buf();
    for component in Path::new(relative).components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_symlink_rejected",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(K2CompositionErrorV1::Io(
                    "stat_self_formed_cleanup_component",
                ));
            }
        }
    }
    Ok(current)
}

fn cleanup_path_exists_v1(path: &Path) -> K2CompositionResultV1<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(K2CompositionErrorV1::Io(
            "stat_self_formed_cleanup_target_presence",
        )),
    }
}

fn validate_cleanup_target_identity_v1(
    path: &Path,
    target: &K2UncertaintyCleanupClassifiedPathV1,
) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_cleanup_target"))?;
    if metadata.file_type().is_symlink()
        || metadata.mode() & 0o7777 != target.mode
        || (metadata.is_file() && target.file_kind != K2UncertaintyCleanupFileKindV1::Regular)
        || (metadata.is_dir() && target.file_kind != K2UncertaintyCleanupFileKindV1::Directory)
        || (!metadata.is_file() && !metadata.is_dir())
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_target_identity_mismatch",
        ));
    }
    let content_sha256 = if metadata.is_file() {
        Some(composition_sha256_file_v1(path)?)
    } else {
        None
    };
    if metadata.is_file()
        && (metadata.len() != target.size_bytes || content_sha256 != target.content_sha256)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_target_content_mismatch",
        ));
    }
    Ok(())
}

fn fail_at_v1(
    actual: K2UncertaintyCleanupFaultV1,
    expected: K2UncertaintyCleanupFaultV1,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_injected_fault",
        ));
    }
    Ok(())
}

pub fn run_self_formed_cleanup_owner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_owner_stdin"))?;
    let request: K2UncertaintyCleanupOwnerRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_cleanup_owner"))?;
    if composition_sha256_file_v1(&executable)? != request.owner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_owner_executable_mismatch",
        ));
    }
    let receipt = execute_self_formed_cleanup_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_cleanup_owner_stdout"))
}
