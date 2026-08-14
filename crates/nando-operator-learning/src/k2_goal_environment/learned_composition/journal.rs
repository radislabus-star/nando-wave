use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::model::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionPrivateMappingV1,
    K2CompositionResultV1, composition_bytes_v1, composition_decode_v1, composition_root_v1,
    composition_sha256_bytes_v1,
};

pub const K2_COMPOSITION_JOURNAL_EVENT_SCHEMA_V1: &str = "nando.k2-composition-journal-event.v1";
pub const K2_COMPOSITION_JOURNAL_PROJECTION_SCHEMA_V1: &str =
    "nando.k2-composition-journal-projection.v1";
pub const K2_COMPOSITION_JOURNAL_EVENTS_V1: u64 = 29;
const K2_COMPOSITION_JOURNAL_MAX_EVENT_BYTES_V1: u64 = 128 * 1024;
const K2_COMPOSITION_PRIVATE_MAPPING_ARTIFACT_SCHEMA_V1: &str =
    "nando.k2-composition-private-mapping-artifact.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionPrivateMappingArtifactReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub mapping_root_sha256: String,
    pub artifact_sha256: String,
    pub artifact_byte_len: u64,
    pub receipt_root_sha256: String,
}

pub fn publish_private_mapping_artifact_v1(
    directory: &Path,
    mapping: &K2CompositionPrivateMappingV1,
) -> K2CompositionResultV1<K2CompositionPrivateMappingArtifactReceiptV1> {
    mapping.validate()?;
    fs::create_dir_all(directory)
        .map_err(|_| K2CompositionErrorV1::Io("create_private_mapping_directory"))?;
    let bytes = composition_bytes_v1(mapping)?;
    let artifact_sha256 = composition_sha256_bytes_v1(&bytes);
    let artifact_byte_len = bytes.len() as u64;
    let final_path = directory.join("private-mapping.json");
    let temp_path = directory.join(".private-mapping.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_path)
        .map_err(|_| K2CompositionErrorV1::Io("create_private_mapping_temp"))?;
    file.write_all(&bytes)
        .map_err(|_| K2CompositionErrorV1::Io("write_private_mapping_temp"))?;
    file.sync_all()
        .map_err(|_| K2CompositionErrorV1::Io("sync_private_mapping_temp"))?;
    drop(file);
    fs::hard_link(&temp_path, &final_path)
        .map_err(|_| K2CompositionErrorV1::Io("publish_private_mapping"))?;
    fs::remove_file(&temp_path)
        .map_err(|_| K2CompositionErrorV1::Io("remove_private_mapping_temp"))?;
    sync_directory_v1(directory)?;
    let receipt_root_sha256 = composition_root_v1(&(
        K2_COMPOSITION_PRIVATE_MAPPING_ARTIFACT_SCHEMA_V1,
        &mapping.experiment_id_sha256,
        &mapping.mapping_root_sha256,
        &artifact_sha256,
        artifact_byte_len,
    ))?;
    Ok(K2CompositionPrivateMappingArtifactReceiptV1 {
        schema: K2_COMPOSITION_PRIVATE_MAPPING_ARTIFACT_SCHEMA_V1.to_owned(),
        experiment_id_sha256: mapping.experiment_id_sha256.clone(),
        mapping_root_sha256: mapping.mapping_root_sha256.clone(),
        artifact_sha256,
        artifact_byte_len,
        receipt_root_sha256,
    })
}

pub fn reopen_private_mapping_artifact_v1(
    directory: &Path,
    receipt: &K2CompositionPrivateMappingArtifactReceiptV1,
) -> K2CompositionResultV1<K2CompositionPrivateMappingV1> {
    let expected_receipt = composition_root_v1(&(
        K2_COMPOSITION_PRIVATE_MAPPING_ARTIFACT_SCHEMA_V1,
        &receipt.experiment_id_sha256,
        &receipt.mapping_root_sha256,
        &receipt.artifact_sha256,
        receipt.artifact_byte_len,
    ))?;
    if receipt.schema != K2_COMPOSITION_PRIVATE_MAPPING_ARTIFACT_SCHEMA_V1
        || expected_receipt != receipt.receipt_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "private_mapping_receipt_invalid",
        ));
    }
    let bytes = fs::read(directory.join("private-mapping.json"))
        .map_err(|_| K2CompositionErrorV1::Io("read_private_mapping"))?;
    if bytes.len() as u64 != receipt.artifact_byte_len
        || composition_sha256_bytes_v1(&bytes) != receipt.artifact_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "private_mapping_artifact_mismatch",
        ));
    }
    let mapping: K2CompositionPrivateMappingV1 = composition_decode_v1(&bytes)?;
    mapping.validate()?;
    if mapping.experiment_id_sha256 != receipt.experiment_id_sha256
        || mapping.mapping_root_sha256 != receipt.mapping_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "private_mapping_binding_mismatch",
        ));
    }
    Ok(mapping)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2CompositionJournalEventKindV1 {
    ExperimentFrozen,
    SupportDispatched,
    SupportObserved,
    LearnedLawsFrozen,
    TargetAndGoalFrozen,
    PlanningRequestFrozen,
    PlanFrozen,
    IndependentPlanVerificationFrozen,
    ExecutionDispatched,
    ExecutionObserved,
    ExactGoalVerified,
    AblationsFrozen,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2CompositionJournalStateV1 {
    Empty,
    Frozen,
    SupportRunning,
    SupportComplete,
    LawsFrozen,
    TargetFrozen,
    PlanningRequestFrozen,
    PlanFrozen,
    PlanVerified,
    ExecutionRunning,
    ExecutionObserved,
    GoalVerified,
    AblationsFrozen,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionJournalEventV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub sequence: u64,
    pub kind: K2CompositionJournalEventKindV1,
    pub payload_schema: String,
    pub payload_root_sha256: String,
    pub previous_entry_root_sha256: Option<String>,
    pub recorded_at_unix_ms: u64,
    pub entry_root_sha256: String,
}

impl K2CompositionJournalEventV1 {
    fn seal(
        experiment_id_sha256: String,
        sequence: u64,
        kind: K2CompositionJournalEventKindV1,
        payload_schema: String,
        payload_root_sha256: String,
        previous_entry_root_sha256: Option<String>,
        recorded_at_unix_ms: u64,
    ) -> K2CompositionResultV1<Self> {
        let entry_root_sha256 = canonical_json_sha256(&(
            K2_COMPOSITION_JOURNAL_EVENT_SCHEMA_V1,
            &experiment_id_sha256,
            sequence,
            kind,
            &payload_schema,
            &payload_root_sha256,
            &previous_entry_root_sha256,
            recorded_at_unix_ms,
        ))
        .map_err(|_| K2CompositionErrorV1::Serialization)?;
        let event = Self {
            schema: K2_COMPOSITION_JOURNAL_EVENT_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            sequence,
            kind,
            payload_schema,
            payload_root_sha256,
            previous_entry_root_sha256,
            recorded_at_unix_ms,
            entry_root_sha256,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.schema != K2_COMPOSITION_JOURNAL_EVENT_SCHEMA_V1
            || self.kind != expected_kind_v1(self.sequence)?
            || self.payload_schema.is_empty()
            || !valid_nonzero_sha256(&self.experiment_id_sha256)
            || !valid_nonzero_sha256(&self.payload_root_sha256)
            || !valid_nonzero_sha256(&self.entry_root_sha256)
            || (self.sequence == 0) != self.previous_entry_root_sha256.is_none()
            || self
                .previous_entry_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
        {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_event_invalid",
            ));
        }
        let expected = canonical_json_sha256(&(
            K2_COMPOSITION_JOURNAL_EVENT_SCHEMA_V1,
            &self.experiment_id_sha256,
            self.sequence,
            self.kind,
            &self.payload_schema,
            &self.payload_root_sha256,
            &self.previous_entry_root_sha256,
            self.recorded_at_unix_ms,
        ))
        .map_err(|_| K2CompositionErrorV1::Serialization)?;
        if expected != self.entry_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_event_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionJournalProjectionV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub state: K2CompositionJournalStateV1,
    pub event_count: u64,
    pub latest_entry_root_sha256: Option<String>,
    pub indeterminate_after_execution_dispatch: bool,
    pub same_identity_execution_dispatch_allowed: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub projection_root_sha256: String,
}

impl K2CompositionJournalProjectionV1 {
    pub fn project(
        experiment_id_sha256: &str,
        events: &[K2CompositionJournalEventV1],
    ) -> K2CompositionResultV1<Self> {
        let mut previous = None;
        for (sequence, event) in events.iter().enumerate() {
            event.validate()?;
            if event.experiment_id_sha256 != experiment_id_sha256
                || event.sequence != sequence as u64
                || event.previous_entry_root_sha256 != previous
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "composition_journal_chain_invalid",
                ));
            }
            previous = Some(event.entry_root_sha256.clone());
        }
        let event_count = events.len() as u64;
        if event_count > K2_COMPOSITION_JOURNAL_EVENTS_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_event_budget",
            ));
        }
        let state = state_for_count_v1(event_count)?;
        let indeterminate_after_execution_dispatch = event_count == 25;
        let same_identity_execution_dispatch_allowed = event_count < 25;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let projection_root_sha256 = canonical_json_sha256(&(
            K2_COMPOSITION_JOURNAL_PROJECTION_SCHEMA_V1,
            experiment_id_sha256,
            state,
            event_count,
            &previous,
            indeterminate_after_execution_dispatch,
            same_identity_execution_dispatch_allowed,
            &authority,
        ))
        .map_err(|_| K2CompositionErrorV1::Serialization)?;
        Ok(Self {
            schema: K2_COMPOSITION_JOURNAL_PROJECTION_SCHEMA_V1.to_owned(),
            experiment_id_sha256: experiment_id_sha256.to_owned(),
            state,
            event_count,
            latest_entry_root_sha256: previous,
            indeterminate_after_execution_dispatch,
            same_identity_execution_dispatch_allowed,
            authority,
            projection_root_sha256,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2CompositionJournalFaultPointV1 {
    None,
    AfterTempSync,
    AfterPublishBeforeDirectorySync,
}

pub struct K2CompositionJournalV1 {
    directory: PathBuf,
    experiment_id_sha256: String,
    events: Vec<K2CompositionJournalEventV1>,
    projection: K2CompositionJournalProjectionV1,
    next_fault: K2CompositionJournalFaultPointV1,
}

impl K2CompositionJournalV1 {
    pub fn create(store: &Path, experiment_id_sha256: String) -> K2CompositionResultV1<Self> {
        if !valid_nonzero_sha256(&experiment_id_sha256) {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_id_invalid",
            ));
        }
        fs::create_dir_all(store)
            .map_err(|_| K2CompositionErrorV1::Io("create_composition_journal_store"))?;
        let directory = store.join(&experiment_id_sha256);
        fs::create_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("create_composition_journal"))?;
        sync_directory_v1(store)?;
        let projection = K2CompositionJournalProjectionV1::project(&experiment_id_sha256, &[])?;
        Ok(Self {
            directory,
            experiment_id_sha256,
            events: Vec::new(),
            projection,
            next_fault: K2CompositionJournalFaultPointV1::None,
        })
    }

    pub fn open_existing(
        store: &Path,
        experiment_id_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let directory = store.join(&experiment_id_sha256);
        if !directory.is_dir() {
            return Err(K2CompositionErrorV1::Invalid("composition_journal_missing"));
        }
        let mut paths = fs::read_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("read_composition_journal"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| K2CompositionErrorV1::Io("read_composition_journal_entry"))?;
        paths.sort();
        let mut events = Vec::with_capacity(paths.len());
        for (sequence, path) in paths.iter().enumerate() {
            if path.file_name().and_then(|name| name.to_str())
                != Some(event_filename_v1(sequence as u64).as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "composition_journal_unknown_entry",
                ));
            }
            let bytes = fs::read(path)
                .map_err(|_| K2CompositionErrorV1::Io("read_composition_journal_event"))?;
            if bytes.len() as u64 > K2_COMPOSITION_JOURNAL_MAX_EVENT_BYTES_V1 {
                return Err(K2CompositionErrorV1::Invalid(
                    "composition_journal_event_bytes",
                ));
            }
            let event: K2CompositionJournalEventV1 = serde_json::from_slice(&bytes)
                .map_err(|_| K2CompositionErrorV1::Invalid("composition_journal_decode"))?;
            if canonical_json_bytes(&event).map_err(|_| K2CompositionErrorV1::Serialization)?
                != bytes
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "composition_journal_not_canonical",
                ));
            }
            events.push(event);
        }
        let projection = K2CompositionJournalProjectionV1::project(&experiment_id_sha256, &events)?;
        Ok(Self {
            directory,
            experiment_id_sha256,
            events,
            projection,
            next_fault: K2CompositionJournalFaultPointV1::None,
        })
    }

    #[must_use]
    pub fn events(&self) -> &[K2CompositionJournalEventV1] {
        &self.events
    }

    #[must_use]
    pub fn projection(&self) -> &K2CompositionJournalProjectionV1 {
        &self.projection
    }

    pub fn set_next_fault_for_test_v1(&mut self, fault: K2CompositionJournalFaultPointV1) {
        self.next_fault = fault;
    }

    pub fn append(
        &mut self,
        kind: K2CompositionJournalEventKindV1,
        payload_schema: &str,
        payload_root_sha256: &str,
        recorded_at_unix_ms: u64,
    ) -> K2CompositionResultV1<K2CompositionJournalEventV1> {
        let sequence = self.events.len() as u64;
        if kind != expected_kind_v1(sequence)? {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_kind_order",
            ));
        }
        let previous = self
            .events
            .last()
            .map(|event| event.entry_root_sha256.clone());
        let event = K2CompositionJournalEventV1::seal(
            self.experiment_id_sha256.clone(),
            sequence,
            kind,
            payload_schema.to_owned(),
            payload_root_sha256.to_owned(),
            previous,
            recorded_at_unix_ms,
        )?;
        let bytes =
            canonical_json_bytes(&event).map_err(|_| K2CompositionErrorV1::Serialization)?;
        if bytes.len() as u64 > K2_COMPOSITION_JOURNAL_MAX_EVENT_BYTES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "composition_journal_event_bytes",
            ));
        }
        let final_path = self.directory.join(event_filename_v1(sequence));
        let temp_path = self.directory.join(format!(".{sequence:03}.tmp"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(|_| K2CompositionErrorV1::Io("create_composition_journal_temp"))?;
        file.write_all(&bytes)
            .map_err(|_| K2CompositionErrorV1::Io("write_composition_journal_temp"))?;
        file.sync_all()
            .map_err(|_| K2CompositionErrorV1::Io("sync_composition_journal_temp"))?;
        drop(file);
        if self.next_fault == K2CompositionJournalFaultPointV1::AfterTempSync {
            self.next_fault = K2CompositionJournalFaultPointV1::None;
            let _ = fs::remove_file(&temp_path);
            return Err(K2CompositionErrorV1::Process("injected_after_temp_sync"));
        }
        fs::hard_link(&temp_path, &final_path)
            .map_err(|_| K2CompositionErrorV1::Io("publish_composition_journal_event"))?;
        fs::remove_file(&temp_path)
            .map_err(|_| K2CompositionErrorV1::Io("remove_composition_journal_temp"))?;
        if self.next_fault == K2CompositionJournalFaultPointV1::AfterPublishBeforeDirectorySync {
            self.next_fault = K2CompositionJournalFaultPointV1::None;
            return Err(K2CompositionErrorV1::Process(
                "injected_after_event_publish",
            ));
        }
        sync_directory_v1(&self.directory)?;
        self.events.push(event.clone());
        self.projection =
            K2CompositionJournalProjectionV1::project(&self.experiment_id_sha256, &self.events)?;
        Ok(event)
    }
}

fn expected_kind_v1(sequence: u64) -> K2CompositionResultV1<K2CompositionJournalEventKindV1> {
    match sequence {
        0 => Ok(K2CompositionJournalEventKindV1::ExperimentFrozen),
        1..=18 if sequence % 2 == 1 => Ok(K2CompositionJournalEventKindV1::SupportDispatched),
        2..=18 => Ok(K2CompositionJournalEventKindV1::SupportObserved),
        19 => Ok(K2CompositionJournalEventKindV1::LearnedLawsFrozen),
        20 => Ok(K2CompositionJournalEventKindV1::TargetAndGoalFrozen),
        21 => Ok(K2CompositionJournalEventKindV1::PlanningRequestFrozen),
        22 => Ok(K2CompositionJournalEventKindV1::PlanFrozen),
        23 => Ok(K2CompositionJournalEventKindV1::IndependentPlanVerificationFrozen),
        24 => Ok(K2CompositionJournalEventKindV1::ExecutionDispatched),
        25 => Ok(K2CompositionJournalEventKindV1::ExecutionObserved),
        26 => Ok(K2CompositionJournalEventKindV1::ExactGoalVerified),
        27 => Ok(K2CompositionJournalEventKindV1::AblationsFrozen),
        28 => Ok(K2CompositionJournalEventKindV1::Terminal),
        _ => Err(K2CompositionErrorV1::Invalid(
            "composition_journal_sequence_invalid",
        )),
    }
}

fn state_for_count_v1(count: u64) -> K2CompositionResultV1<K2CompositionJournalStateV1> {
    match count {
        0 => Ok(K2CompositionJournalStateV1::Empty),
        1 => Ok(K2CompositionJournalStateV1::Frozen),
        2..=18 => Ok(K2CompositionJournalStateV1::SupportRunning),
        19 => Ok(K2CompositionJournalStateV1::SupportComplete),
        20 => Ok(K2CompositionJournalStateV1::LawsFrozen),
        21 => Ok(K2CompositionJournalStateV1::TargetFrozen),
        22 => Ok(K2CompositionJournalStateV1::PlanningRequestFrozen),
        23 => Ok(K2CompositionJournalStateV1::PlanFrozen),
        24 => Ok(K2CompositionJournalStateV1::PlanVerified),
        25 => Ok(K2CompositionJournalStateV1::ExecutionRunning),
        26 => Ok(K2CompositionJournalStateV1::ExecutionObserved),
        27 => Ok(K2CompositionJournalStateV1::GoalVerified),
        28 => Ok(K2CompositionJournalStateV1::AblationsFrozen),
        29 => Ok(K2CompositionJournalStateV1::Terminal),
        _ => Err(K2CompositionErrorV1::Invalid(
            "composition_journal_count_invalid",
        )),
    }
}

fn event_filename_v1(sequence: u64) -> String {
    format!("{sequence:03}.json")
}

fn sync_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_composition_journal_directory"))
}
