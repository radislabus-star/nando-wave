use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_bytes_v1, composition_decode_v1,
    composition_root_v1, require_composition_root_v1,
};

const JOURNAL_SCHEMA_V1: &str = "nando.k2-inquiry-journal.v1";
const EVENT_SCHEMA_V1: &str = "nando.k2-inquiry-journal-event.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2InquiryJournalEventKindV1 {
    ExperimentFrozen,
    BaselinesFrozen,
    SelectionDispatched,
    SelectionPrecommitted,
    SelectionVerified,
    ProbeDispatched,
    ProbeObserved,
    ModelsUpdated,
    ControlsFrozen,
    TerminalFrozen,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryJournalEventV1 {
    pub schema: String,
    pub sequence: u64,
    pub kind: K2InquiryJournalEventKindV1,
    pub previous_event_root_sha256: Option<String>,
    pub payload_root_sha256: String,
    pub event_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct K2InquiryJournalFileV1 {
    schema: String,
    experiment_id_sha256: String,
    events: Vec<K2InquiryJournalEventV1>,
    file_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2InquiryJournalProjectionV1 {
    pub event_count: u64,
    pub last_kind: Option<K2InquiryJournalEventKindV1>,
    pub last_event_root_sha256: Option<String>,
    pub selection_precommitted: bool,
    pub selection_verified: bool,
    pub probe_dispatched: bool,
    pub probe_observed: bool,
    pub terminal: bool,
    pub indeterminate_selection_dispatch: bool,
    pub indeterminate_probe_dispatch: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2InquiryJournalFaultV1 {
    None,
    BeforeRename,
    AfterRename,
}

pub struct K2InquiryJournalV1 {
    root: PathBuf,
    path: PathBuf,
    file: K2InquiryJournalFileV1,
}

impl K2InquiryJournalV1 {
    pub fn create(root: &Path, experiment_id_sha256: String) -> K2CompositionResultV1<Self> {
        require_composition_root_v1(&experiment_id_sha256)?;
        fs::create_dir_all(root)
            .map_err(|_| K2CompositionErrorV1::Io("create_inquiry_journal_root"))?;
        let path = root.join(format!("{experiment_id_sha256}.json"));
        if path.exists() {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_journal_identity_exists",
            ));
        }
        let mut file = K2InquiryJournalFileV1 {
            schema: JOURNAL_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            events: Vec::new(),
            file_root_sha256: String::new(),
        };
        reseal_file_v1(&mut file)?;
        atomic_write_v1(
            root,
            &path,
            &composition_bytes_v1(&file)?,
            K2InquiryJournalFaultV1::None,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn open_existing(root: &Path, experiment_id_sha256: String) -> K2CompositionResultV1<Self> {
        let path = root.join(format!("{experiment_id_sha256}.json"));
        let bytes =
            fs::read(&path).map_err(|_| K2CompositionErrorV1::Io("read_inquiry_journal"))?;
        let file: K2InquiryJournalFileV1 = composition_decode_v1(&bytes)?;
        validate_file_v1(&file, &experiment_id_sha256)?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            file,
        })
    }

    pub fn append(
        &mut self,
        kind: K2InquiryJournalEventKindV1,
        payload_root_sha256: String,
    ) -> K2CompositionResultV1<String> {
        self.append_with_fault(kind, payload_root_sha256, K2InquiryJournalFaultV1::None)
    }

    pub fn append_with_fault(
        &mut self,
        kind: K2InquiryJournalEventKindV1,
        payload_root_sha256: String,
        fault: K2InquiryJournalFaultV1,
    ) -> K2CompositionResultV1<String> {
        require_composition_root_v1(&payload_root_sha256)?;
        let sequence = self.file.events.len() as u64 + 1;
        if expected_kind_v1(sequence) != Some(kind) {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_journal_event_order_invalid",
            ));
        }
        let previous_event_root_sha256 = self
            .file
            .events
            .last()
            .map(|event| event.event_root_sha256.clone());
        let event_root_sha256 = composition_root_v1(&(
            EVENT_SCHEMA_V1,
            sequence,
            kind,
            &previous_event_root_sha256,
            &payload_root_sha256,
        ))?;
        let event = K2InquiryJournalEventV1 {
            schema: EVENT_SCHEMA_V1.to_owned(),
            sequence,
            kind,
            previous_event_root_sha256,
            payload_root_sha256,
            event_root_sha256: event_root_sha256.clone(),
        };
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
        Ok(event_root_sha256)
    }

    #[must_use]
    pub fn projection(&self) -> K2InquiryJournalProjectionV1 {
        let count = self.file.events.len();
        let last_kind = self.file.events.last().map(|event| event.kind);
        K2InquiryJournalProjectionV1 {
            event_count: count as u64,
            last_kind,
            last_event_root_sha256: self
                .file
                .events
                .last()
                .map(|event| event.event_root_sha256.clone()),
            selection_precommitted: count >= 4,
            selection_verified: count >= 5,
            probe_dispatched: count >= 6,
            probe_observed: count >= 7,
            terminal: count == 10,
            indeterminate_selection_dispatch: last_kind
                == Some(K2InquiryJournalEventKindV1::SelectionDispatched),
            indeterminate_probe_dispatch: last_kind
                == Some(K2InquiryJournalEventKindV1::ProbeDispatched),
        }
    }

    pub fn cleanup(self) -> K2CompositionResultV1<()> {
        fs::remove_file(&self.path)
            .map_err(|_| K2CompositionErrorV1::Io("remove_inquiry_journal"))?;
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| K2CompositionErrorV1::Io("sync_inquiry_journal_cleanup"))
    }
}

fn expected_kind_v1(sequence: u64) -> Option<K2InquiryJournalEventKindV1> {
    use K2InquiryJournalEventKindV1::*;
    [
        ExperimentFrozen,
        BaselinesFrozen,
        SelectionDispatched,
        SelectionPrecommitted,
        SelectionVerified,
        ProbeDispatched,
        ProbeObserved,
        ModelsUpdated,
        ControlsFrozen,
        TerminalFrozen,
    ]
    .get(sequence.saturating_sub(1) as usize)
    .copied()
}

fn validate_file_v1(
    file: &K2InquiryJournalFileV1,
    experiment_id_sha256: &str,
) -> K2CompositionResultV1<()> {
    if file.schema != JOURNAL_SCHEMA_V1 || file.experiment_id_sha256 != experiment_id_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_journal_identity_invalid",
        ));
    }
    let mut previous = None;
    for (index, event) in file.events.iter().enumerate() {
        let sequence = index as u64 + 1;
        let expected_root = composition_root_v1(&(
            EVENT_SCHEMA_V1,
            sequence,
            event.kind,
            &previous,
            &event.payload_root_sha256,
        ))?;
        if event.schema != EVENT_SCHEMA_V1
            || event.sequence != sequence
            || expected_kind_v1(sequence) != Some(event.kind)
            || event.previous_event_root_sha256 != previous
            || event.event_root_sha256 != expected_root
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_journal_chain_invalid",
            ));
        }
        previous = Some(event.event_root_sha256.clone());
    }
    let expected_file_root =
        composition_root_v1(&(JOURNAL_SCHEMA_V1, &file.experiment_id_sha256, &file.events))?;
    if file.file_root_sha256 != expected_file_root {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_journal_file_root_mismatch",
        ));
    }
    Ok(())
}

fn reseal_file_v1(file: &mut K2InquiryJournalFileV1) -> K2CompositionResultV1<()> {
    file.file_root_sha256 =
        composition_root_v1(&(JOURNAL_SCHEMA_V1, &file.experiment_id_sha256, &file.events))?;
    Ok(())
}

fn atomic_write_v1(
    root: &Path,
    path: &Path,
    bytes: &[u8],
    fault: K2InquiryJournalFaultV1,
) -> K2CompositionResultV1<()> {
    let temporary =
        root.join(format!(
            ".{}.tmp",
            path.file_name().and_then(|name| name.to_str()).ok_or(
                K2CompositionErrorV1::Invalid("inquiry_journal_path_invalid")
            )?
        ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_inquiry_journal_temp"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_inquiry_journal_temp"))?;
    if fault == K2InquiryJournalFaultV1::BeforeRename {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "inquiry_journal_fault_before_rename",
        ));
    }
    fs::rename(&temporary, path).map_err(|_| K2CompositionErrorV1::Io("rename_inquiry_journal"))?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_inquiry_journal_directory"))?;
    if fault == K2InquiryJournalFaultV1::AfterRename {
        return Err(K2CompositionErrorV1::Io(
            "inquiry_journal_fault_after_rename",
        ));
    }
    Ok(())
}
