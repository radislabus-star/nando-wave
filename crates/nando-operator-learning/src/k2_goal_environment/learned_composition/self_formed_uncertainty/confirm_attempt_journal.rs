use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, composition_decode_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1, K2UncertaintyConfirmAttemptDescriptorV1,
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptEventV1,
    K2UncertaintyConfirmAttemptModeV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

const ATTEMPT_JOURNAL_FILE_V1: &str = "attempt-journal.json";
const MAX_ATTEMPT_EVENTS_V1: usize = 256;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct K2UncertaintyConfirmAttemptJournalFileV1 {
    schema: String,
    descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
    events: Vec<K2UncertaintyConfirmAttemptEventV1>,
    authority: K2CompositionAuthorityBoundaryV1,
    journal_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyConfirmAttemptJournalFaultV1 {
    None,
    BeforeRename,
    AfterRename,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyConfirmAttemptPhaseV1 {
    AwaitingArtifacts,
    AwaitingNonce,
    AwaitingNonceCommit,
    ReadyForGeneratorDispatch,
    GeneratorDispatched,
    CasesGenerated,
    Downstream,
    NonceCreatedUncommitted,
    NonceCommittedUndispatched,
    GeneratorResultIndeterminate,
    Terminal,
    CleanupFrozen,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyConfirmAttemptProjectionV1 {
    pub attempt_root_sha256: String,
    pub mode: K2UncertaintyConfirmAttemptModeV1,
    pub event_count: u64,
    pub last_kind: Option<K2UncertaintyConfirmAttemptEventKindV1>,
    pub last_event_root_sha256: Option<String>,
    pub phase: K2UncertaintyConfirmAttemptPhaseV1,
    pub generator_dispatch_count: u64,
    pub cases_generated: bool,
    pub terminal: bool,
    pub cleanup_frozen: bool,
    pub sealed_attempts: u64,
}

pub struct K2UncertaintyConfirmAttemptJournalV1 {
    root: PathBuf,
    path: PathBuf,
    file: K2UncertaintyConfirmAttemptJournalFileV1,
}

impl K2UncertaintyConfirmAttemptJournalV1 {
    pub fn create_exclusive(
        root: &Path,
        descriptor: K2UncertaintyConfirmAttemptDescriptorV1,
    ) -> K2CompositionResultV1<Self> {
        descriptor.validate()?;
        DirBuilder::new()
            .mode(0o700)
            .create(root)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_attempt_directory"))?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_attempt_directory"))?;
        let path = root.join(ATTEMPT_JOURNAL_FILE_V1);
        let mut file = K2UncertaintyConfirmAttemptJournalFileV1 {
            schema: K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1.to_owned(),
            descriptor,
            events: Vec::new(),
            authority: denied_authority_v1(),
            journal_root_sha256: String::new(),
        };
        reseal_journal_file_v1(&mut file)?;
        atomic_write_attempt_journal_v1(
            root,
            &path,
            &composition_bytes_v1(&file)?,
            0,
            K2UncertaintyConfirmAttemptJournalFaultV1::None,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn open_existing(root: &Path) -> K2CompositionResultV1<Self> {
        let path = root.join(ATTEMPT_JOURNAL_FILE_V1);
        require_private_attempt_root_v1(root, &path)?;
        let bytes = fs::read(&path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_attempt_journal"))?;
        let file: K2UncertaintyConfirmAttemptJournalFileV1 = composition_decode_v1(&bytes)?;
        validate_journal_file_v1(&file)?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn descriptor(&self) -> &K2UncertaintyConfirmAttemptDescriptorV1 {
        &self.file.descriptor
    }

    pub fn events(&self) -> &[K2UncertaintyConfirmAttemptEventV1] {
        &self.file.events
    }

    pub fn projection(&self) -> K2UncertaintyConfirmAttemptProjectionV1 {
        let last_kind = self.file.events.last().map(|event| event.kind);
        let phase = phase_v1(self.file.descriptor.mode, last_kind);
        K2UncertaintyConfirmAttemptProjectionV1 {
            attempt_root_sha256: self.file.descriptor.attempt_root_sha256.clone(),
            mode: self.file.descriptor.mode,
            event_count: self.file.events.len() as u64,
            last_kind,
            last_event_root_sha256: self
                .file
                .events
                .last()
                .map(|event| event.event_root_sha256.clone()),
            phase,
            generator_dispatch_count: self
                .file
                .events
                .iter()
                .filter(|event| {
                    event.kind == K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched
                })
                .count() as u64,
            cases_generated: self
                .file
                .events
                .iter()
                .any(|event| event.kind == K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated),
            terminal: matches!(
                phase,
                K2UncertaintyConfirmAttemptPhaseV1::NonceCreatedUncommitted
                    | K2UncertaintyConfirmAttemptPhaseV1::NonceCommittedUndispatched
                    | K2UncertaintyConfirmAttemptPhaseV1::GeneratorResultIndeterminate
                    | K2UncertaintyConfirmAttemptPhaseV1::Terminal
                    | K2UncertaintyConfirmAttemptPhaseV1::CleanupFrozen
            ),
            cleanup_frozen: phase == K2UncertaintyConfirmAttemptPhaseV1::CleanupFrozen,
            sealed_attempts: self.file.descriptor.sealed_attempts,
        }
    }

    pub fn append(
        &mut self,
        kind: K2UncertaintyConfirmAttemptEventKindV1,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
    ) -> K2CompositionResultV1<K2UncertaintyConfirmAttemptEventV1> {
        self.append_with_fault(
            kind,
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
            K2UncertaintyConfirmAttemptJournalFaultV1::None,
        )
    }

    pub fn append_with_fault(
        &mut self,
        kind: K2UncertaintyConfirmAttemptEventKindV1,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
        fault: K2UncertaintyConfirmAttemptJournalFaultV1,
    ) -> K2CompositionResultV1<K2UncertaintyConfirmAttemptEventV1> {
        require_composition_root_v1(&owner_executable_sha256)?;
        require_composition_root_v1(&request_root_sha256)?;
        require_composition_root_v1(&payload_root_sha256)?;
        validate_next_kind_v1(
            self.file.descriptor.mode,
            self.file.events.last().map(|event| event.kind),
            kind,
        )?;
        let sequence = self.file.events.len() as u64;
        let event = K2UncertaintyConfirmAttemptEventV1::seal(
            self.file.descriptor.attempt_root_sha256.clone(),
            sequence,
            self.file
                .events
                .last()
                .map(|previous| previous.event_root_sha256.clone()),
            kind,
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
        )?;
        let mut candidate = self.file.clone();
        candidate.events.push(event.clone());
        reseal_journal_file_v1(&mut candidate)?;
        atomic_write_attempt_journal_v1(
            &self.root,
            &self.path,
            &composition_bytes_v1(&candidate)?,
            sequence + 1,
            fault,
        )?;
        self.file = candidate;
        Ok(event)
    }

    pub fn recover_after_restart(
        &mut self,
        owner_executable_sha256: String,
        recovery_request_root_sha256: String,
        retained_nonce_root_sha256: Option<String>,
        complete_split_receipt_root_sha256: Option<String>,
    ) -> K2CompositionResultV1<K2UncertaintyConfirmAttemptProjectionV1> {
        for root in [
            &retained_nonce_root_sha256,
            &complete_split_receipt_root_sha256,
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        let projection = self.projection();
        match projection.phase {
            K2UncertaintyConfirmAttemptPhaseV1::AwaitingNonce
                if retained_nonce_root_sha256.is_some() =>
            {
                self.append(
                    K2UncertaintyConfirmAttemptEventKindV1::NonceCreatedUncommitted,
                    owner_executable_sha256,
                    recovery_request_root_sha256,
                    retained_nonce_root_sha256.ok_or(K2CompositionErrorV1::Invalid(
                        "self_formed_retained_nonce_root_missing",
                    ))?,
                )?;
            }
            K2UncertaintyConfirmAttemptPhaseV1::AwaitingNonceCommit => {
                let payload = retained_nonce_root_sha256.or_else(|| {
                    self.file
                        .events
                        .last()
                        .map(|event| event.payload_root_sha256.clone())
                });
                self.append(
                    K2UncertaintyConfirmAttemptEventKindV1::NonceCreatedUncommitted,
                    owner_executable_sha256,
                    recovery_request_root_sha256,
                    payload.ok_or(K2CompositionErrorV1::Invalid(
                        "self_formed_retained_nonce_root_missing",
                    ))?,
                )?;
            }
            K2UncertaintyConfirmAttemptPhaseV1::ReadyForGeneratorDispatch
                if self.file.descriptor.mode == K2UncertaintyConfirmAttemptModeV1::Confirm =>
            {
                self.append(
                    K2UncertaintyConfirmAttemptEventKindV1::NonceCommittedUndispatched,
                    owner_executable_sha256,
                    recovery_request_root_sha256,
                    self.file
                        .events
                        .last()
                        .map(|event| event.payload_root_sha256.clone())
                        .ok_or(K2CompositionErrorV1::Invalid(
                            "self_formed_nonce_commit_event_missing",
                        ))?,
                )?;
            }
            K2UncertaintyConfirmAttemptPhaseV1::GeneratorDispatched => {
                let (kind, payload) = if let Some(root) = complete_split_receipt_root_sha256 {
                    (K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated, root)
                } else {
                    (
                        K2UncertaintyConfirmAttemptEventKindV1::GeneratorResultIndeterminate,
                        self.file
                            .events
                            .last()
                            .map(|event| event.payload_root_sha256.clone())
                            .ok_or(K2CompositionErrorV1::Invalid(
                                "self_formed_generator_dispatch_event_missing",
                            ))?,
                    )
                };
                self.append(
                    kind,
                    owner_executable_sha256,
                    recovery_request_root_sha256,
                    payload,
                )?;
            }
            _ => {}
        }
        Ok(self.projection())
    }
}

fn phase_v1(
    mode: K2UncertaintyConfirmAttemptModeV1,
    last: Option<K2UncertaintyConfirmAttemptEventKindV1>,
) -> K2UncertaintyConfirmAttemptPhaseV1 {
    use K2UncertaintyConfirmAttemptEventKindV1 as Event;
    match last {
        None => K2UncertaintyConfirmAttemptPhaseV1::AwaitingArtifacts,
        Some(Event::ArtifactsFrozen) => match mode {
            K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal => {
                K2UncertaintyConfirmAttemptPhaseV1::ReadyForGeneratorDispatch
            }
            K2UncertaintyConfirmAttemptModeV1::Confirm => {
                K2UncertaintyConfirmAttemptPhaseV1::AwaitingNonce
            }
        },
        Some(Event::NonceCreated) => K2UncertaintyConfirmAttemptPhaseV1::AwaitingNonceCommit,
        Some(Event::NonceCommitted) => {
            K2UncertaintyConfirmAttemptPhaseV1::ReadyForGeneratorDispatch
        }
        Some(Event::GeneratorDispatched) => K2UncertaintyConfirmAttemptPhaseV1::GeneratorDispatched,
        Some(Event::CasesGenerated) => K2UncertaintyConfirmAttemptPhaseV1::CasesGenerated,
        Some(Event::NonceCreatedUncommitted) => {
            K2UncertaintyConfirmAttemptPhaseV1::NonceCreatedUncommitted
        }
        Some(Event::NonceCommittedUndispatched) => {
            K2UncertaintyConfirmAttemptPhaseV1::NonceCommittedUndispatched
        }
        Some(Event::GeneratorResultIndeterminate) => {
            K2UncertaintyConfirmAttemptPhaseV1::GeneratorResultIndeterminate
        }
        Some(Event::DevelopmentRehearsalTerminalFrozen | Event::ScientificVerdictFrozen) => {
            K2UncertaintyConfirmAttemptPhaseV1::Terminal
        }
        Some(Event::CleanupFrozen) => K2UncertaintyConfirmAttemptPhaseV1::CleanupFrozen,
        Some(_) => K2UncertaintyConfirmAttemptPhaseV1::Downstream,
    }
}

fn validate_next_kind_v1(
    mode: K2UncertaintyConfirmAttemptModeV1,
    previous: Option<K2UncertaintyConfirmAttemptEventKindV1>,
    next: K2UncertaintyConfirmAttemptEventKindV1,
) -> K2CompositionResultV1<()> {
    use K2UncertaintyConfirmAttemptEventKindV1 as Event;
    let valid = match (previous, next) {
        (None, Event::ArtifactsFrozen) => true,
        (Some(Event::ArtifactsFrozen), Event::GeneratorDispatched) => {
            mode == K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal
        }
        (Some(Event::ArtifactsFrozen), Event::NonceCreated) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::ArtifactsFrozen), Event::NonceCreatedUncommitted) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::NonceCreated), Event::NonceCommitted) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::NonceCreated), Event::NonceCreatedUncommitted) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::NonceCommitted), Event::GeneratorDispatched) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::NonceCommitted), Event::NonceCommittedUndispatched) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (Some(Event::GeneratorDispatched), Event::CasesGenerated) => true,
        (Some(Event::GeneratorDispatched), Event::GeneratorResultIndeterminate) => true,
        (Some(Event::CasesGenerated), Event::ModelSetsFrozen) => true,
        (Some(Event::ModelSetsFrozen), Event::ProbeSetsFrozen) => true,
        (Some(Event::ProbeSetsFrozen), Event::SelectionsFrozen) => true,
        (Some(Event::SelectionsFrozen), Event::AllCasesPrecommitted) => true,
        (Some(Event::AllCasesPrecommitted | Event::ProbeObserved), Event::ProbeDispatched) => true,
        (Some(Event::ProbeDispatched), Event::ProbeObserved) => true,
        (Some(Event::AllCasesPrecommitted | Event::ProbeObserved), Event::ObservationsFrozen) => {
            true
        }
        (Some(Event::ObservationsFrozen), Event::ModelsUpdated) => true,
        (Some(Event::ModelsUpdated), Event::ControlsFrozen) => true,
        (Some(Event::ControlsFrozen), Event::DevelopmentRehearsalTerminalFrozen) => {
            mode == K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal
        }
        (Some(Event::ControlsFrozen), Event::ScientificVerdictFrozen) => {
            mode == K2UncertaintyConfirmAttemptModeV1::Confirm
        }
        (
            Some(
                Event::DevelopmentRehearsalTerminalFrozen
                | Event::ScientificVerdictFrozen
                | Event::NonceCreatedUncommitted
                | Event::NonceCommittedUndispatched
                | Event::GeneratorResultIndeterminate,
            ),
            Event::CleanupFrozen,
        ) => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_attempt_transition_invalid",
        ))
    }
}

fn require_private_attempt_root_v1(root: &Path, journal: &Path) -> K2CompositionResultV1<()> {
    let root_metadata = fs::symlink_metadata(root)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_attempt_directory"))?;
    let journal_metadata = fs::symlink_metadata(journal)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_attempt_journal"))?;
    if root_metadata.file_type().is_symlink()
        || !root_metadata.is_dir()
        || root_metadata.permissions().mode() & 0o777 != 0o700
        || journal_metadata.file_type().is_symlink()
        || !journal_metadata.is_file()
        || journal_metadata.permissions().mode() & 0o777 != 0o600
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_attempt_path_invalid",
        ));
    }
    Ok(())
}

fn validate_journal_file_v1(
    file: &K2UncertaintyConfirmAttemptJournalFileV1,
) -> K2CompositionResultV1<()> {
    file.descriptor.validate()?;
    require_denied_authority_v1(&file.authority)?;
    if file.schema != K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1
        || file.events.len() > MAX_ATTEMPT_EVENTS_V1
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_attempt_journal_invalid",
        ));
    }
    let mut previous_kind = None;
    let mut previous_root: Option<&str> = None;
    for (sequence, event) in file.events.iter().enumerate() {
        event.validate()?;
        if event.attempt_root_sha256 != file.descriptor.attempt_root_sha256
            || event.sequence != sequence as u64
            || event.previous_event_root_sha256.as_deref() != previous_root
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_attempt_journal_chain_invalid",
            ));
        }
        validate_next_kind_v1(file.descriptor.mode, previous_kind, event.kind)?;
        previous_kind = Some(event.kind);
        previous_root = Some(&event.event_root_sha256);
    }
    if file.journal_root_sha256
        != uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1,
            &file.descriptor,
            &file.events,
            &file.authority,
        ))?
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_attempt_journal_root_invalid",
        ));
    }
    Ok(())
}

fn reseal_journal_file_v1(
    file: &mut K2UncertaintyConfirmAttemptJournalFileV1,
) -> K2CompositionResultV1<()> {
    file.journal_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_CONFIRM_ATTEMPT_JOURNAL_SCHEMA_V1,
        &file.descriptor,
        &file.events,
        &file.authority,
    ))?;
    validate_journal_file_v1(file)
}

fn atomic_write_attempt_journal_v1(
    root: &Path,
    path: &Path,
    bytes: &[u8],
    sequence: u64,
    fault: K2UncertaintyConfirmAttemptJournalFaultV1,
) -> K2CompositionResultV1<()> {
    let temporary = root.join(format!(".{ATTEMPT_JOURNAL_FILE_V1}.{sequence}.tmp"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_attempt_journal_temp"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_attempt_journal_temp"))?;
    if fault == K2UncertaintyConfirmAttemptJournalFaultV1::BeforeRename {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "self_formed_attempt_journal_fault_before_rename",
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_attempt_journal"))?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_attempt_journal_directory"))?;
    if fault == K2UncertaintyConfirmAttemptJournalFaultV1::AfterRename {
        return Err(K2CompositionErrorV1::Io(
            "self_formed_attempt_journal_fault_after_rename",
        ));
    }
    Ok(())
}
