use std::collections::BTreeMap;
use std::fmt;

use nando_operator_kernel::ResponseProgram;
use serde::{Deserialize, Serialize};

use crate::OperatorIdentificationMachineV1;

use super::super::{Ms3LinkedFrameReceiptV1, PassiveT1ProbeContractV1};

pub const MS3_FROZEN_VERSION_SPACE_CONTRACT_SCHEMA_V1: &str =
    "nando.ms3-frozen-version-space-contract.v1";
pub const MS3_FROZEN_VERSION_SPACE_ENVELOPE_SCHEMA_V1: &str =
    "nando.ms3-frozen-version-space-envelope.v1";
pub const MS3_PRE_FREEZE_BUFFER_EXCLUDED: &str = "PRE_FREEZE_BUFFER_EXCLUDED";
pub(super) const MS3_T1_GRAMMAR_SCHEMA_V1: &str = "nando.multi-source-t1-grammar.v1";
pub(super) const MAX_ENVELOPE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3ZeroClassReasonV1 {
    ProgramAlgebraGap,
    UnsupportedRenderer,
    SelfReplayInconsistency,
    InvalidHypothesisGeneration,
    PermanentAbstain,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Ms3FrozenVersionSpaceStateV1 {
    ZeroClasses {
        reason: Ms3ZeroClassReasonV1,
        blocker: String,
    },
    UniqueLawFrozen {
        semantic_class_root_sha256: String,
        candidate_freeze_root_sha256: String,
    },
    Ambiguous {
        semantic_classes: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenVersionSpaceContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub acquisition_report_root_sha256: String,
    pub linked_receipt_root_sha256: String,
    pub topology_root_sha256: String,
    pub frame_root_sha256: String,
    pub terminal_root_sha256: String,
    pub transport_binding_root_sha256: String,
    pub session_lineage_sha256: String,
    pub session_id_sha256: String,
    pub turn_intent_id_sha256: String,
    pub request_event_id_sha256: String,
    pub action_event_id_sha256: String,
    pub extractor_schema: String,
    pub extractor_version: String,
    pub generator_version: String,
    pub grammar_root_sha256: String,
    pub compiler_version: String,
    pub vm_abi: String,
    pub verifier_schema: String,
    pub support_rows_root_sha256: String,
    pub support_watermark: u64,
    pub contract_watermark: u64,
    pub future_min_sequence: u64,
    pub pre_freeze_buffer_sequence_span: u64,
    pub pre_freeze_buffer_disposition: String,
    pub candidate_program_roots_sha256: Vec<String>,
    pub semantic_class_roots_sha256: Vec<String>,
    pub quotient_root_sha256: String,
    pub class_predictions_root_sha256: String,
    pub machine_checkpoint_sha256: String,
    pub machine_checkpoint_bytes: usize,
    pub passive_probe: Option<PassiveT1ProbeContractV1>,
    pub state: Ms3FrozenVersionSpaceStateV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenVersionSpaceEnvelopeV1 {
    pub schema: String,
    pub envelope_root_sha256: String,
    pub contract: FrozenVersionSpaceContractV1,
    #[serde(with = "serde_bytes")]
    pub(super) machine_checkpoint: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ms3VersionSpaceVersionsV1 {
    pub compiler_version: String,
    pub vm_abi: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ms3FrozenVersionSpaceErrorV1 {
    InvalidAcquisition,
    LinkedReceiptMissing,
    LinkedEvidenceMismatch,
    RepresentationGapReopened,
    InvalidContractWatermark,
    CandidateGeneration(String),
    CandidateRegistration(String),
    CandidateSearchIncomplete,
    SupportReplay(String),
    Freeze(String),
    Serialization,
    InvalidEnvelope,
}

impl fmt::Display for Ms3FrozenVersionSpaceErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAcquisition => formatter.write_str("MS3 acquisition report is invalid"),
            Self::LinkedReceiptMissing => formatter.write_str("NO_GAP linked receipt is missing"),
            Self::LinkedEvidenceMismatch => {
                formatter.write_str("linked topology, frame, and terminal roots do not match")
            }
            Self::RepresentationGapReopened => {
                formatter.write_str("NO_GAP cannot be silently reclassified as representation gap")
            }
            Self::InvalidContractWatermark => {
                formatter.write_str("contract watermark precedes support")
            }
            Self::CandidateGeneration(blocker) => {
                write!(formatter, "candidate generation failed: {blocker}")
            }
            Self::CandidateRegistration(error) => {
                write!(formatter, "candidate registration failed: {error}")
            }
            Self::CandidateSearchIncomplete => formatter.write_str("candidate search incomplete"),
            Self::SupportReplay(error) => write!(formatter, "support replay failed: {error}"),
            Self::Freeze(error) => write!(formatter, "candidate freeze failed: {error}"),
            Self::Serialization => formatter.write_str("version-space serialization failed"),
            Self::InvalidEnvelope => {
                formatter.write_str("frozen version-space envelope is invalid")
            }
        }
    }
}

impl std::error::Error for Ms3FrozenVersionSpaceErrorV1 {}

pub struct PreparedMs3VersionSpaceV1 {
    pub(super) acquisition_report_root_sha256: String,
    pub(super) linked_receipt: Ms3LinkedFrameReceiptV1,
    pub(super) extractor_schema: String,
    pub(super) extractor_version: String,
    pub(super) support_rows_root_sha256: String,
    pub(super) support_watermark: u64,
    pub(super) candidate_program_roots_sha256: Vec<String>,
    pub(super) semantic_class_roots_sha256: Vec<String>,
    pub(super) quotient_root_sha256: String,
    pub(super) class_predictions_root_sha256: String,
    pub(super) passive_probe: Option<PassiveT1ProbeContractV1>,
    pub(super) state: PreparedStateV1,
    pub(super) machine: OperatorIdentificationMachineV1,
}

pub(super) enum PreparedStateV1 {
    ZeroClasses {
        reason: Ms3ZeroClassReasonV1,
        blocker: String,
    },
    Unique {
        semantic_class_root_sha256: String,
        canonical_program: Box<ResponseProgram>,
        protocol_mode_root_sha256: String,
    },
    Ambiguous {
        semantic_classes: usize,
    },
}

#[derive(Serialize)]
pub(super) struct ContractDigestV1<'a> {
    pub(super) schema: &'static str,
    pub(super) acquisition_report_root_sha256: &'a str,
    pub(super) linked_receipt_root_sha256: &'a str,
    pub(super) topology_root_sha256: &'a str,
    pub(super) frame_root_sha256: &'a str,
    pub(super) terminal_root_sha256: &'a str,
    pub(super) transport_binding_root_sha256: &'a str,
    pub(super) session_lineage_sha256: &'a str,
    pub(super) session_id_sha256: &'a str,
    pub(super) turn_intent_id_sha256: &'a str,
    pub(super) request_event_id_sha256: &'a str,
    pub(super) action_event_id_sha256: &'a str,
    pub(super) extractor_schema: &'a str,
    pub(super) extractor_version: &'a str,
    pub(super) generator_version: &'a str,
    pub(super) grammar_root_sha256: &'a str,
    pub(super) compiler_version: &'a str,
    pub(super) vm_abi: &'a str,
    pub(super) verifier_schema: &'a str,
    pub(super) support_rows_root_sha256: &'a str,
    pub(super) support_watermark: u64,
    pub(super) contract_watermark: u64,
    pub(super) future_min_sequence: u64,
    pub(super) pre_freeze_buffer_sequence_span: u64,
    pub(super) pre_freeze_buffer_disposition: &'a str,
    pub(super) candidate_program_roots_sha256: &'a [String],
    pub(super) semantic_class_roots_sha256: &'a [String],
    pub(super) quotient_root_sha256: &'a str,
    pub(super) class_predictions_root_sha256: &'a str,
    pub(super) machine_checkpoint_sha256: &'a str,
    pub(super) machine_checkpoint_bytes: usize,
    pub(super) passive_probe: &'a Option<PassiveT1ProbeContractV1>,
    pub(super) state: &'a Ms3FrozenVersionSpaceStateV1,
    pub(super) authority_ready: bool,
    pub(super) phase_mutation_allowed: bool,
}

pub(super) type ClassPredictionsV1 = Vec<(String, String, BTreeMap<String, String>)>;
