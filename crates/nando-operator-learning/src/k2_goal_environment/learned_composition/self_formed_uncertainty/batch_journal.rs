use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, composition_decode_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_BATCH_JOURNAL_EVENT_SCHEMA_V1, K2_UNCERTAINTY_BATCH_JOURNAL_EVENTS_V1,
    K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1, denied_authority_v1,
    require_denied_authority_v1, require_sorted_unique_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyBatchJournalEventKindV1 {
    BatchFrozen,
    CasesGenerated,
    ModelSetsFrozen,
    ProbeSetsFrozen,
    SelectionsFrozen,
    AllCasesPrecommitted,
    ProbeDispatched,
    ProbeObserved,
    ModelsUpdated,
    ControlsFrozen,
    TerminalFrozen,
    CleanupFrozen,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyBatchJournalEventV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub sequence: u64,
    pub kind: K2UncertaintyBatchJournalEventKindV1,
    pub case_id_sha256: Option<String>,
    pub previous_event_root_sha256: Option<String>,
    pub owner_executable_sha256: String,
    pub request_root_sha256: String,
    pub payload_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub event_root_sha256: String,
}

impl K2UncertaintyBatchJournalEventV1 {
    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_BATCH_JOURNAL_EVENT_SCHEMA_V1,
            &self.experiment_id_sha256,
            self.sequence,
            self.kind,
            &self.case_id_sha256,
            &self.previous_event_root_sha256,
            &self.owner_executable_sha256,
            &self.request_root_sha256,
            &self.payload_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct K2UncertaintyBatchJournalFileV1 {
    schema: String,
    experiment_id_sha256: String,
    execution_order_case_roots_sha256: Vec<String>,
    events: Vec<K2UncertaintyBatchJournalEventV1>,
    authority: K2CompositionAuthorityBoundaryV1,
    file_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyBatchJournalFaultV1 {
    None,
    BeforeRename,
    AfterRename,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyBatchJournalProjectionV1 {
    pub experiment_id_sha256: String,
    pub execution_order_case_roots_sha256: Vec<String>,
    pub event_count: u64,
    pub last_kind: Option<K2UncertaintyBatchJournalEventKindV1>,
    pub last_event_root_sha256: Option<String>,
    pub all_cases_precommitted: bool,
    pub all_cases_precommitted_payload_root_sha256: Option<String>,
    pub completed_cases: u64,
    pub indeterminate_dispatch_case_id_sha256: Option<String>,
    pub terminal: bool,
    pub cleanup_frozen: bool,
}

pub struct K2UncertaintyBatchJournalV1 {
    root: PathBuf,
    path: PathBuf,
    file: K2UncertaintyBatchJournalFileV1,
}

impl K2UncertaintyBatchJournalV1 {
    pub fn create(
        root: &Path,
        experiment_id_sha256: String,
        execution_order_case_roots_sha256: Vec<String>,
    ) -> K2CompositionResultV1<Self> {
        require_composition_root_v1(&experiment_id_sha256)?;
        if execution_order_case_roots_sha256.len() != K2_UNCERTAINTY_CONFIRM_CASES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_case_count_invalid",
            ));
        }
        require_sorted_unique_v1(
            &{
                let mut roots = execution_order_case_roots_sha256.clone();
                roots.sort();
                roots
            },
            "self_formed_batch_journal_case_roots_invalid",
        )?;
        for case in &execution_order_case_roots_sha256 {
            require_composition_root_v1(case)?;
        }
        fs::create_dir_all(root)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_batch_journal_root"))?;
        let path = root.join(format!("{experiment_id_sha256}.json"));
        if path.exists() {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_identity_exists",
            ));
        }
        let mut file = K2UncertaintyBatchJournalFileV1 {
            schema: K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            execution_order_case_roots_sha256,
            events: Vec::new(),
            authority: denied_authority_v1(),
            file_root_sha256: String::new(),
        };
        reseal_file_v1(&mut file)?;
        atomic_write_v1(
            root,
            &path,
            &composition_bytes_v1(&file)?,
            K2UncertaintyBatchJournalFaultV1::None,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn open_existing(root: &Path, experiment_id_sha256: String) -> K2CompositionResultV1<Self> {
        let path = root.join(format!("{experiment_id_sha256}.json"));
        let bytes = fs::read(&path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_batch_journal"))?;
        let file: K2UncertaintyBatchJournalFileV1 = composition_decode_v1(&bytes)?;
        validate_file_v1(&file, &experiment_id_sha256)?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn append(
        &mut self,
        kind: K2UncertaintyBatchJournalEventKindV1,
        case_id_sha256: Option<String>,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
    ) -> K2CompositionResultV1<String> {
        self.append_with_fault(
            kind,
            case_id_sha256,
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
            K2UncertaintyBatchJournalFaultV1::None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append_with_fault(
        &mut self,
        kind: K2UncertaintyBatchJournalEventKindV1,
        case_id_sha256: Option<String>,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
        fault: K2UncertaintyBatchJournalFaultV1,
    ) -> K2CompositionResultV1<String> {
        for root in [
            &owner_executable_sha256,
            &request_root_sha256,
            &payload_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if let Some(case) = &case_id_sha256 {
            require_composition_root_v1(case)?;
        }
        let sequence = self.file.events.len() as u64;
        let (expected_kind, expected_case) =
            expected_event_v1(sequence, &self.file.execution_order_case_roots_sha256)?;
        if kind != expected_kind || case_id_sha256.as_deref() != expected_case {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_event_order_invalid",
            ));
        }
        let previous_event_root_sha256 = self
            .file
            .events
            .last()
            .map(|event| event.event_root_sha256.clone());
        let authority = denied_authority_v1();
        let mut event = K2UncertaintyBatchJournalEventV1 {
            schema: K2_UNCERTAINTY_BATCH_JOURNAL_EVENT_SCHEMA_V1.to_owned(),
            experiment_id_sha256: self.file.experiment_id_sha256.clone(),
            sequence,
            kind,
            case_id_sha256,
            previous_event_root_sha256,
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
            authority,
            event_root_sha256: String::new(),
        };
        event.event_root_sha256 = event.expected_root()?;
        let event_root = event.event_root_sha256.clone();
        let mut candidate = self.file.clone();
        candidate.events.push(event);
        reseal_file_v1(&mut candidate)?;
        atomic_write_v1(
            &self.root,
            &self.path,
            &composition_bytes_v1(&candidate)?,
            fault,
        )?;
        self.file = candidate;
        Ok(event_root)
    }

    #[must_use]
    pub fn projection(&self) -> K2UncertaintyBatchJournalProjectionV1 {
        let count = self.file.events.len();
        let completed_cases = count
            .saturating_sub(6)
            .min(K2_UNCERTAINTY_CONFIRM_CASES_V1 * 3)
            / 3;
        let indeterminate_dispatch_case_id_sha256 = self.file.events.last().and_then(|event| {
            (event.kind == K2UncertaintyBatchJournalEventKindV1::ProbeDispatched)
                .then(|| event.case_id_sha256.clone())
                .flatten()
        });
        K2UncertaintyBatchJournalProjectionV1 {
            experiment_id_sha256: self.file.experiment_id_sha256.clone(),
            execution_order_case_roots_sha256: self.file.execution_order_case_roots_sha256.clone(),
            event_count: count as u64,
            last_kind: self.file.events.last().map(|event| event.kind),
            last_event_root_sha256: self
                .file
                .events
                .last()
                .map(|event| event.event_root_sha256.clone()),
            all_cases_precommitted: count >= 6,
            all_cases_precommitted_payload_root_sha256: self
                .file
                .events
                .get(5)
                .filter(|event| {
                    event.kind == K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted
                })
                .map(|event| event.payload_root_sha256.clone()),
            completed_cases: completed_cases as u64,
            indeterminate_dispatch_case_id_sha256,
            terminal: count >= K2_UNCERTAINTY_BATCH_JOURNAL_EVENTS_V1 - 1,
            cleanup_frozen: count == K2_UNCERTAINTY_BATCH_JOURNAL_EVENTS_V1,
        }
    }
}

fn expected_event_v1(
    sequence: u64,
    execution_order: &[String],
) -> K2CompositionResultV1<(K2UncertaintyBatchJournalEventKindV1, Option<&str>)> {
    use K2UncertaintyBatchJournalEventKindV1::*;
    let sequence = usize::try_from(sequence).map_err(|_| {
        K2CompositionErrorV1::Invalid("self_formed_batch_journal_sequence_overflow")
    })?;
    let fixed = [
        BatchFrozen,
        CasesGenerated,
        ModelSetsFrozen,
        ProbeSetsFrozen,
        SelectionsFrozen,
        AllCasesPrecommitted,
    ];
    if let Some(kind) = fixed.get(sequence) {
        return Ok((*kind, None));
    }
    let case_events = K2_UNCERTAINTY_CONFIRM_CASES_V1 * 3;
    if sequence < 6 + case_events {
        let offset = sequence - 6;
        let kind = [ProbeDispatched, ProbeObserved, ModelsUpdated][offset % 3];
        return Ok((kind, execution_order.get(offset / 3).map(String::as_str)));
    }
    let kind = match sequence - 6 - case_events {
        0 => ControlsFrozen,
        1 => TerminalFrozen,
        2 => CleanupFrozen,
        _ => {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_terminal_transition",
            ));
        }
    };
    Ok((kind, None))
}

fn validate_file_v1(
    file: &K2UncertaintyBatchJournalFileV1,
    experiment_id_sha256: &str,
) -> K2CompositionResultV1<()> {
    require_denied_authority_v1(&file.authority)?;
    if file.schema != K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1
        || file.experiment_id_sha256 != experiment_id_sha256
        || file.execution_order_case_roots_sha256.len() != K2_UNCERTAINTY_CONFIRM_CASES_V1
        || file.events.len() > K2_UNCERTAINTY_BATCH_JOURNAL_EVENTS_V1
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_batch_journal_file_invalid",
        ));
    }
    let mut sorted_cases = file.execution_order_case_roots_sha256.clone();
    sorted_cases.sort();
    require_sorted_unique_v1(
        &sorted_cases,
        "self_formed_batch_journal_case_roots_invalid",
    )?;
    let mut previous = None;
    for (sequence, event) in file.events.iter().enumerate() {
        require_denied_authority_v1(&event.authority)?;
        let (kind, case) =
            expected_event_v1(sequence as u64, &file.execution_order_case_roots_sha256)?;
        if event.schema != K2_UNCERTAINTY_BATCH_JOURNAL_EVENT_SCHEMA_V1
            || event.experiment_id_sha256 != file.experiment_id_sha256
            || event.sequence != sequence as u64
            || event.kind != kind
            || event.case_id_sha256.as_deref() != case
            || event.previous_event_root_sha256 != previous
            || event.event_root_sha256 != event.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_chain_invalid",
            ));
        }
        previous = Some(event.event_root_sha256.clone());
    }
    let expected = uncertainty_root_v1(&(
        K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1,
        &file.experiment_id_sha256,
        &file.execution_order_case_roots_sha256,
        &file.events,
        &file.authority,
    ))?;
    if file.file_root_sha256 != expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_batch_journal_root_mismatch",
        ));
    }
    Ok(())
}

fn reseal_file_v1(file: &mut K2UncertaintyBatchJournalFileV1) -> K2CompositionResultV1<()> {
    file.file_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_BATCH_JOURNAL_SCHEMA_V1,
        &file.experiment_id_sha256,
        &file.execution_order_case_roots_sha256,
        &file.events,
        &file.authority,
    ))?;
    validate_file_v1(file, &file.experiment_id_sha256)
}

fn atomic_write_v1(
    root: &Path,
    path: &Path,
    bytes: &[u8],
    fault: K2UncertaintyBatchJournalFaultV1,
) -> K2CompositionResultV1<()> {
    let name =
        path.file_name()
            .and_then(|value| value.to_str())
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_batch_journal_name_invalid",
            ))?;
    let temporary = root.join(format!(".{name}.tmp"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_batch_journal_temp"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_batch_journal_temp"))?;
    if fault == K2UncertaintyBatchJournalFaultV1::BeforeRename {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "self_formed_batch_journal_fault_before_rename",
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_batch_journal"))?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_batch_journal_directory"))?;
    if fault == K2UncertaintyBatchJournalFaultV1::AfterRename {
        return Err(K2CompositionErrorV1::Io(
            "self_formed_batch_journal_fault_after_rename",
        ));
    }
    Ok(())
}
