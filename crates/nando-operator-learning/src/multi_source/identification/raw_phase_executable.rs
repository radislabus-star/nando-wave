use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintSynthesisReport, BoundedCircuitBeam, BoundedRoleAligner,
    FrozenOperatorBlueprintSet, LocalRelationFragment, OPERATOR_BLUEPRINT_MAX_BUNDLES,
    OPERATOR_ROLE_NONE, RoleAlignmentConfig, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TypedProgramAtom, phase_vector_from_atom_ids,
};
use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, ProjectStatusMapping, RelationFrame,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_OK,
    TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS, TRANSFORM_STATUS_ZERO_IS_TRUE,
    TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, ValueProjectionFormat, canonical_json_sha256,
    relation_frame_phase_atom_ids, response_program_version_root_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use super::RawPhaseT1HypothesisEnvelopeV1;
use crate::CandidateFreezeReceiptV1;
use crate::multi_source::source_neutral_t1_binding::{
    program_role_selectors, role_for_witness, witness_for_selector,
};
use crate::multi_source::{
    BlindThenRevealJoinedTransitionV1, PreActionT1ConsumedInputV1,
    pre_action_applicability_shape_root_v1, pre_action_t1_input_binding_manifest_v1,
};

pub const RAW_PHASE_EXECUTABLE_BLUEPRINT_ENVELOPE_SCHEMA_V1: &str =
    "nando.raw-phase-executable-blueprint-envelope.v1";
pub const RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1: &str =
    "nando.raw-phase.source-neutral-bounded-circuit-beam.v1";
pub const RAW_PHASE_EXECUTABLE_EVIDENCE_SCHEMA_V1: &str = "nando.raw-phase-executable-evidence.v1";
pub const RAW_PHASE_SELECTED_EXECUTABLE_RECEIPT_SCHEMA_V1: &str =
    "nando.raw-phase-selected-executable-receipt.v1";

const RAW_PHASE_EXECUTABLE_MAX_PROGRAMS: usize = 4_096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RawPhaseExecutableBlueprintExclusionV1 {
    ActorProgramInvalid,
    VerifierUnavailable,
    UnsupportedTypedProgram,
    SupportBindingUnavailable,
    SurfaceBundleInvalid,
    IndependentSupportLineagesInsufficient,
    RoleAlignmentIncomplete,
    CircuitSearchIncomplete,
    CircuitCandidateEmpty,
    BlueprintFreezeFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseExecutableBlueprintDispositionV1 {
    pub program_root_sha256: String,
    pub actor_root_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifier_root_sha256: Option<String>,
    pub support_bundle_count: usize,
    pub role_alignment_complete: bool,
    pub circuit_search_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_bundle_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_blueprint_candidate_set_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blueprint_fingerprints_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusion: Option<RawPhaseExecutableBlueprintExclusionV1>,
}

impl RawPhaseExecutableBlueprintDispositionV1 {
    #[must_use]
    pub const fn executable(&self) -> bool {
        self.exclusion.is_none()
    }

    fn validate(&self) -> bool {
        if !valid_nonzero_sha256(&self.program_root_sha256)
            || !valid_nonzero_sha256(&self.actor_root_sha256)
            || self
                .verifier_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || self.support_bundle_count > OPERATOR_BLUEPRINT_MAX_BUNDLES
        {
            return false;
        }
        if self.executable() {
            self.verifier_root_sha256.is_some()
                && self.support_bundle_count >= 2
                && self.role_alignment_complete
                && self.circuit_search_complete
                && self
                    .support_bundle_root_sha256
                    .as_deref()
                    .is_some_and(valid_nonzero_sha256)
                && self
                    .frozen_blueprint_candidate_set_root_sha256
                    .as_deref()
                    .is_some_and(valid_nonzero_sha256)
                && strict_roots(&self.blueprint_fingerprints_sha256, false)
        } else {
            self.support_bundle_root_sha256.is_none()
                && self.frozen_blueprint_candidate_set_root_sha256.is_none()
                && self.blueprint_fingerprints_sha256.is_empty()
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseExecutableBlueprintEnvelopeV1 {
    pub schema: String,
    pub envelope_root_sha256: String,
    pub raw_phase_hypothesis_root_sha256: String,
    pub frozen_domain_root_sha256: String,
    pub support_watermark: u64,
    pub support_surface_root_sha256: String,
    pub support_rows: usize,
    pub candidate_program_roots_sha256: Vec<String>,
    pub executable_program_roots_sha256: Vec<String>,
    pub dispositions: Vec<RawPhaseExecutableBlueprintDispositionV1>,
    pub builder_schema: String,
    pub bounded_disposition_complete: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseExecutableEvidenceV1 {
    pub schema: String,
    pub evidence_root_sha256: String,
    pub joined: BlindThenRevealJoinedTransitionV1,
    pub frame: RelationFrame,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawPhaseSelectedExecutableReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub executable_envelope_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub canonical_program_root_sha256: String,
    pub support_watermark: u64,
    pub selected_disposition: RawPhaseExecutableBlueprintDispositionV1,
    pub support_evidence: Vec<RawPhaseExecutableEvidenceV1>,
    pub builder_schema: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct RawPhaseSelectedExecutableReceiptDigestV1<'a> {
    schema: &'static str,
    executable_envelope_root_sha256: &'a str,
    candidate_freeze_root_sha256: &'a str,
    canonical_program_root_sha256: &'a str,
    support_watermark: u64,
    selected_disposition: &'a RawPhaseExecutableBlueprintDispositionV1,
    support_evidence_roots_sha256: Vec<&'a str>,
    builder_schema: &'static str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl RawPhaseExecutableEvidenceV1 {
    pub fn seal(
        joined: BlindThenRevealJoinedTransitionV1,
        frame: RelationFrame,
    ) -> Result<Self, &'static str> {
        let mut evidence = Self {
            schema: RAW_PHASE_EXECUTABLE_EVIDENCE_SCHEMA_V1.to_owned(),
            evidence_root_sha256: String::new(),
            joined,
            frame,
        };
        evidence.evidence_root_sha256 = evidence.expected_root()?;
        evidence.validate()?;
        Ok(evidence)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.joined.validate()?;
        let completed_frame_root_sha256 = canonical_json_sha256(&self.frame)
            .map_err(|_| "raw_phase_executable_evidence_frame_root_failed")?;
        if self.schema != RAW_PHASE_EXECUTABLE_EVIDENCE_SCHEMA_V1
            || !self.joined.accepted
            || self.frame.verifier_label != Some(true)
            || self.joined.completed_frame_root_sha256 != completed_frame_root_sha256
            || self.joined.action_event_id_sha256 != self.frame.event_id_sha256
            || self.joined.turn_intent_id_sha256 != self.frame.client_intent_id_sha256
            || self.joined.session_id_sha256 != self.frame.session_id_sha256
            || self.joined.completed_at_unix_nanos != self.frame.observed_at_unix_nanos
            || self.evidence_root_sha256 != self.expected_root()?
        {
            return Err("raw_phase_executable_evidence_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            RAW_PHASE_EXECUTABLE_EVIDENCE_SCHEMA_V1,
            self.joined.join_root_sha256.as_str(),
            self.joined.completed_frame_root_sha256.as_str(),
            self.frame.frame_id_sha256.as_str(),
        ))
        .map_err(|_| "raw_phase_executable_evidence_root_failed")
    }

    fn support(&self) -> RawPhaseExecutableSupportV1<'_> {
        RawPhaseExecutableSupportV1 {
            capture_sequence: self.joined.capture_sequence,
            lineage_root_sha256: &self.joined.session_lineage_sha256,
            topology: &self.joined.topology,
            frame: &self.frame,
        }
    }
}

impl RawPhaseSelectedExecutableReceiptV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let support_lineages = self
            .support_evidence
            .iter()
            .map(|evidence| evidence.joined.session_lineage_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if self.schema != RAW_PHASE_SELECTED_EXECUTABLE_RECEIPT_SCHEMA_V1
            || self.builder_schema != RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1
            || !valid_nonzero_sha256(&self.receipt_root_sha256)
            || !valid_nonzero_sha256(&self.executable_envelope_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_freeze_root_sha256)
            || !valid_nonzero_sha256(&self.canonical_program_root_sha256)
            || self.support_watermark == 0
            || !self.selected_disposition.validate()
            || !self.selected_disposition.executable()
            || self.selected_disposition.program_root_sha256 != self.canonical_program_root_sha256
            || self.support_evidence.len() != self.selected_disposition.support_bundle_count
            || self.support_evidence.len() < 2
            || self.support_evidence.len() > OPERATOR_BLUEPRINT_MAX_BUNDLES
            || support_lineages.len() != self.support_evidence.len()
            || self.support_evidence.iter().any(|evidence| {
                evidence.validate().is_err()
                    || evidence.joined.capture_sequence > self.support_watermark
            })
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err("raw_phase_selected_executable_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&RawPhaseSelectedExecutableReceiptDigestV1 {
            schema: RAW_PHASE_SELECTED_EXECUTABLE_RECEIPT_SCHEMA_V1,
            executable_envelope_root_sha256: &self.executable_envelope_root_sha256,
            candidate_freeze_root_sha256: &self.candidate_freeze_root_sha256,
            canonical_program_root_sha256: &self.canonical_program_root_sha256,
            support_watermark: self.support_watermark,
            selected_disposition: &self.selected_disposition,
            support_evidence_roots_sha256: self
                .support_evidence
                .iter()
                .map(|evidence| evidence.evidence_root_sha256.as_str())
                .collect(),
            builder_schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(|_| "raw_phase_selected_executable_receipt_root_failed")
    }
}

#[derive(Serialize)]
struct RawPhaseExecutableBlueprintEnvelopeDigestV1<'a> {
    schema: &'static str,
    raw_phase_hypothesis_root_sha256: &'a str,
    frozen_domain_root_sha256: &'a str,
    support_watermark: u64,
    support_surface_root_sha256: &'a str,
    support_rows: usize,
    candidate_program_roots_sha256: &'a [String],
    executable_program_roots_sha256: &'a [String],
    dispositions: &'a [RawPhaseExecutableBlueprintDispositionV1],
    builder_schema: &'static str,
    bounded_disposition_complete: bool,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub(super) struct RawPhaseExecutableSupportV1<'a> {
    pub capture_sequence: u64,
    pub lineage_root_sha256: &'a str,
    pub topology: &'a PreActionMultiSourceTopologyV1,
    pub frame: &'a RelationFrame,
}

struct RawPhaseExecutableBuildV1 {
    disposition: RawPhaseExecutableBlueprintDispositionV1,
    selected_support_frame_ids_sha256: Vec<String>,
    support_bundles: Vec<SurfaceFragmentBundle>,
    frozen: Option<FrozenOperatorBlueprintSet>,
}

impl RawPhaseExecutableBuildV1 {
    fn excluded(disposition: RawPhaseExecutableBlueprintDispositionV1) -> Self {
        Self {
            disposition,
            selected_support_frame_ids_sha256: Vec::new(),
            support_bundles: Vec::new(),
            frozen: None,
        }
    }
}

pub(super) fn seal_raw_phase_executable_blueprint_envelope_v1(
    hypothesis: &RawPhaseT1HypothesisEnvelopeV1,
    support: &[RawPhaseExecutableSupportV1<'_>],
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<RawPhaseExecutableBlueprintEnvelopeV1, &'static str> {
    hypothesis.validate()?;
    if support.is_empty()
        || support.len() != hypothesis.support_frame_roots_sha256.len()
        || programs.is_empty()
        || programs.len() > RAW_PHASE_EXECUTABLE_MAX_PROGRAMS
        || programs.keys().cloned().collect::<Vec<_>>() != hypothesis.candidate_program_roots_sha256
    {
        return Err("raw_phase_executable_input_invalid");
    }

    let support_frame_roots = support
        .iter()
        .map(|row| row.frame.frame_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let support_lineage_roots = support
        .iter()
        .map(|row| row.lineage_root_sha256.to_owned())
        .collect::<BTreeSet<_>>();
    if support_frame_roots.into_iter().collect::<Vec<_>>() != hypothesis.support_frame_roots_sha256
        || support_lineage_roots.into_iter().collect::<Vec<_>>()
            != hypothesis.support_lineage_roots_sha256
        || support.iter().any(|row| {
            row.capture_sequence == 0
                || row.capture_sequence > hypothesis.support_watermark
                || !valid_nonzero_sha256(row.lineage_root_sha256)
                || row.topology.validate().is_err()
        })
    {
        return Err("raw_phase_executable_support_mismatch");
    }

    let mut surface_rows = support
        .iter()
        .map(|row| {
            let shape_root = pre_action_applicability_shape_root_v1(row.topology)?;
            Ok((
                row.frame.frame_id_sha256.clone(),
                row.lineage_root_sha256.to_owned(),
                shape_root,
                relation_frame_phase_atom_ids(row.frame),
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    surface_rows.sort();
    let support_surface_root_sha256 = canonical_json_sha256(&(
        "nando.raw-phase-executable-support-surfaces.v1",
        hypothesis.support_watermark,
        &surface_rows,
    ))
    .map_err(|_| "raw_phase_executable_support_surface_root_failed")?;

    let mut dispositions = programs
        .iter()
        .map(|(program_root, program)| {
            executable_build(program_root, program, hypothesis.support_watermark, support)
                .disposition
        })
        .collect::<Vec<_>>();
    dispositions.sort_by(|left, right| left.program_root_sha256.cmp(&right.program_root_sha256));
    let executable_program_roots_sha256 = dispositions
        .iter()
        .filter(|disposition| disposition.executable())
        .map(|disposition| disposition.program_root_sha256.clone())
        .collect::<Vec<_>>();

    let mut envelope = RawPhaseExecutableBlueprintEnvelopeV1 {
        schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_root_sha256: String::new(),
        raw_phase_hypothesis_root_sha256: hypothesis.envelope_root_sha256.clone(),
        frozen_domain_root_sha256: hypothesis.frozen_domain_root_sha256.clone(),
        support_watermark: hypothesis.support_watermark,
        support_surface_root_sha256,
        support_rows: support.len(),
        candidate_program_roots_sha256: hypothesis.candidate_program_roots_sha256.clone(),
        executable_program_roots_sha256,
        dispositions,
        builder_schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1.to_owned(),
        bounded_disposition_complete: true,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    envelope.envelope_root_sha256 = envelope.expected_root()?;
    envelope.validate()?;
    Ok(envelope)
}

impl RawPhaseExecutableBlueprintEnvelopeV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let disposition_program_roots = self
            .dispositions
            .iter()
            .map(|disposition| disposition.program_root_sha256.clone())
            .collect::<Vec<_>>();
        let executable_program_roots = self
            .dispositions
            .iter()
            .filter(|disposition| disposition.executable())
            .map(|disposition| disposition.program_root_sha256.clone())
            .collect::<Vec<_>>();
        if self.schema != RAW_PHASE_EXECUTABLE_BLUEPRINT_ENVELOPE_SCHEMA_V1
            || self.builder_schema != RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1
            || !valid_nonzero_sha256(&self.envelope_root_sha256)
            || !valid_nonzero_sha256(&self.raw_phase_hypothesis_root_sha256)
            || !valid_nonzero_sha256(&self.frozen_domain_root_sha256)
            || !valid_nonzero_sha256(&self.support_surface_root_sha256)
            || self.support_watermark == 0
            || self.support_rows == 0
            || self.candidate_program_roots_sha256.is_empty()
            || self.candidate_program_roots_sha256.len() > RAW_PHASE_EXECUTABLE_MAX_PROGRAMS
            || !strict_roots(&self.candidate_program_roots_sha256, false)
            || !strict_roots(&self.executable_program_roots_sha256, true)
            || self.candidate_program_roots_sha256 != disposition_program_roots
            || self.executable_program_roots_sha256 != executable_program_roots
            || self.dispositions.iter().any(|entry| !entry.validate())
            || !self.bounded_disposition_complete
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.expected_root()? != self.envelope_root_sha256
        {
            return Err("raw_phase_executable_blueprint_envelope_invalid");
        }
        Ok(())
    }

    pub(super) fn executable_programs(
        &self,
        programs: &BTreeMap<String, ResponseProgram>,
    ) -> BTreeMap<String, ResponseProgram> {
        self.executable_program_roots_sha256
            .iter()
            .filter_map(|root| {
                programs
                    .get(root)
                    .cloned()
                    .map(|program| (root.clone(), program))
            })
            .collect()
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&RawPhaseExecutableBlueprintEnvelopeDigestV1 {
            schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_ENVELOPE_SCHEMA_V1,
            raw_phase_hypothesis_root_sha256: &self.raw_phase_hypothesis_root_sha256,
            frozen_domain_root_sha256: &self.frozen_domain_root_sha256,
            support_watermark: self.support_watermark,
            support_surface_root_sha256: &self.support_surface_root_sha256,
            support_rows: self.support_rows,
            candidate_program_roots_sha256: &self.candidate_program_roots_sha256,
            executable_program_roots_sha256: &self.executable_program_roots_sha256,
            dispositions: &self.dispositions,
            builder_schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1,
            bounded_disposition_complete: self.bounded_disposition_complete,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(|_| "raw_phase_executable_blueprint_envelope_root_failed")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawPhaseRebuiltExecutableBlueprintV1 {
    pub frozen: FrozenOperatorBlueprintSet,
    pub support_bundles: Vec<SurfaceFragmentBundle>,
}

pub(super) fn seal_raw_phase_selected_executable_receipt_v1(
    envelope: &RawPhaseExecutableBlueprintEnvelopeV1,
    freeze: &CandidateFreezeReceiptV1,
    program: &ResponseProgram,
    support_evidence: Vec<RawPhaseExecutableEvidenceV1>,
) -> Result<RawPhaseSelectedExecutableReceiptV1, &'static str> {
    envelope.validate()?;
    freeze
        .validate()
        .map_err(|_| "raw_phase_selected_candidate_freeze_invalid")?;
    let program_root_sha256 = response_program_version_root_sha256(program)?;
    if program_root_sha256 != freeze.canonical_program_root_sha256()
        || freeze.support_watermark_next_sequence() != envelope.support_watermark.saturating_add(1)
        || support_evidence.is_empty()
        || support_evidence
            .iter()
            .any(|evidence| evidence.validate().is_err())
    {
        return Err("raw_phase_selected_executable_binding_invalid");
    }
    let support = support_evidence
        .iter()
        .map(RawPhaseExecutableEvidenceV1::support)
        .collect::<Vec<_>>();
    let build = executable_build(
        &program_root_sha256,
        program,
        envelope.support_watermark,
        &support,
    );
    let envelope_disposition = envelope
        .dispositions
        .iter()
        .find(|disposition| disposition.program_root_sha256 == program_root_sha256)
        .ok_or("raw_phase_selected_executable_disposition_missing")?;
    if !build.disposition.executable() || &build.disposition != envelope_disposition {
        return Err("raw_phase_selected_executable_disposition_mismatch");
    }
    let mut selected_support = Vec::with_capacity(build.selected_support_frame_ids_sha256.len());
    for frame_id in &build.selected_support_frame_ids_sha256 {
        let mut matches = support_evidence
            .iter()
            .filter(|evidence| evidence.frame.frame_id_sha256 == *frame_id);
        let evidence = matches
            .next()
            .cloned()
            .ok_or("raw_phase_selected_support_evidence_missing")?;
        if matches.next().is_some() {
            return Err("raw_phase_selected_support_evidence_ambiguous");
        }
        selected_support.push(evidence);
    }
    let selected_support_rows = selected_support
        .iter()
        .map(RawPhaseExecutableEvidenceV1::support)
        .collect::<Vec<_>>();
    let selected_build = executable_build(
        &program_root_sha256,
        program,
        envelope.support_watermark,
        &selected_support_rows,
    );
    if selected_build.disposition != build.disposition {
        return Err("raw_phase_selected_support_rebuild_mismatch");
    }
    let mut receipt = RawPhaseSelectedExecutableReceiptV1 {
        schema: RAW_PHASE_SELECTED_EXECUTABLE_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_root_sha256: String::new(),
        executable_envelope_root_sha256: envelope.envelope_root_sha256.clone(),
        candidate_freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
        canonical_program_root_sha256: program_root_sha256,
        support_watermark: envelope.support_watermark,
        selected_disposition: selected_build.disposition,
        support_evidence: selected_support,
        builder_schema: RAW_PHASE_EXECUTABLE_BLUEPRINT_BUILDER_V1.to_owned(),
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    receipt.receipt_root_sha256 = receipt.expected_root()?;
    receipt.validate()?;
    Ok(receipt)
}

pub fn rebuild_raw_phase_selected_executable_v1(
    receipt: &RawPhaseSelectedExecutableReceiptV1,
    freeze: &CandidateFreezeReceiptV1,
    program: &ResponseProgram,
) -> Result<RawPhaseRebuiltExecutableBlueprintV1, &'static str> {
    receipt.validate()?;
    freeze
        .validate()
        .map_err(|_| "raw_phase_selected_candidate_freeze_invalid")?;
    let program_root_sha256 = response_program_version_root_sha256(program)?;
    if receipt.candidate_freeze_root_sha256 != freeze.freeze_root_sha256()
        || receipt.canonical_program_root_sha256 != program_root_sha256
        || freeze.canonical_program_root_sha256() != program_root_sha256
        || freeze.support_watermark_next_sequence() != receipt.support_watermark.saturating_add(1)
    {
        return Err("raw_phase_selected_executable_rebuild_binding_mismatch");
    }
    let support = receipt
        .support_evidence
        .iter()
        .map(RawPhaseExecutableEvidenceV1::support)
        .collect::<Vec<_>>();
    let build = executable_build(
        &program_root_sha256,
        program,
        receipt.support_watermark,
        &support,
    );
    if build.disposition != receipt.selected_disposition {
        return Err("raw_phase_selected_executable_rebuild_disposition_mismatch");
    }
    let frozen = build
        .frozen
        .ok_or("raw_phase_selected_executable_rebuild_frozen_missing")?;
    Ok(RawPhaseRebuiltExecutableBlueprintV1 {
        frozen,
        support_bundles: build.support_bundles,
    })
}

pub fn raw_phase_executable_surface_bundle_v1(
    program: &ResponseProgram,
    evidence: &RawPhaseExecutableEvidenceV1,
) -> Result<SurfaceFragmentBundle, RawPhaseExecutableBlueprintExclusionV1> {
    evidence
        .validate()
        .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    program_surface_bundle(program, &evidence.support())
}

pub fn raw_phase_executable_runtime_selectors_v1(
    program: &ResponseProgram,
    evidence: &RawPhaseExecutableEvidenceV1,
) -> Result<Vec<ResponseValueSelector>, RawPhaseExecutableBlueprintExclusionV1> {
    evidence
        .validate()
        .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    program_source_bindings(program, &evidence.joined.topology)
        .map(|bindings| bindings.into_iter().map(|(_, selector)| selector).collect())
}

fn executable_build(
    program_root: &str,
    program: &ResponseProgram,
    support_watermark: u64,
    support: &[RawPhaseExecutableSupportV1<'_>],
) -> RawPhaseExecutableBuildV1 {
    let actor_root = canonical_json_sha256(program).unwrap_or_default();
    let mut disposition = RawPhaseExecutableBlueprintDispositionV1 {
        program_root_sha256: program_root.to_owned(),
        actor_root_sha256: actor_root.clone(),
        verifier_root_sha256: None,
        support_bundle_count: 0,
        role_alignment_complete: false,
        circuit_search_complete: false,
        support_bundle_root_sha256: None,
        frozen_blueprint_candidate_set_root_sha256: None,
        blueprint_fingerprints_sha256: Vec::new(),
        exclusion: None,
    };
    if !valid_nonzero_sha256(&actor_root) || program.validate().is_err() {
        disposition.actor_root_sha256 = program_root.to_owned();
        disposition.exclusion = Some(RawPhaseExecutableBlueprintExclusionV1::ActorProgramInvalid);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    }
    let verifier = match crate::synthesis::compile_independent_verifier(program) {
        Ok(verifier) => verifier,
        Err(_) => {
            disposition.exclusion =
                Some(RawPhaseExecutableBlueprintExclusionV1::VerifierUnavailable);
            return RawPhaseExecutableBuildV1::excluded(disposition);
        }
    };
    let verifier_root = match canonical_json_sha256(&verifier) {
        Ok(root) if valid_nonzero_sha256(&root) => root,
        _ => {
            disposition.exclusion =
                Some(RawPhaseExecutableBlueprintExclusionV1::VerifierUnavailable);
            return RawPhaseExecutableBuildV1::excluded(disposition);
        }
    };
    disposition.verifier_root_sha256 = Some(verifier_root.clone());

    let mut bundles_by_lineage = BTreeMap::new();
    for row in support {
        let bundle = match program_surface_bundle(program, row) {
            Ok(bundle) => bundle,
            Err(exclusion) => {
                disposition.exclusion = Some(exclusion);
                return RawPhaseExecutableBuildV1::excluded(disposition);
            }
        };
        let lineage = *bundle.lineage_sha256();
        let surface = *bundle.surface_sha256();
        match bundles_by_lineage.entry(lineage) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((surface, row.frame.frame_id_sha256.clone(), bundle));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if surface < entry.get().0 {
                    entry.insert((surface, row.frame.frame_id_sha256.clone(), bundle));
                }
            }
        }
    }
    let selected = bundles_by_lineage
        .into_values()
        .take(OPERATOR_BLUEPRINT_MAX_BUNDLES)
        .collect::<Vec<_>>();
    let selected_support_frame_ids_sha256 = selected
        .iter()
        .map(|(_, frame_id, _)| frame_id.clone())
        .collect::<Vec<_>>();
    let bundles = selected
        .into_iter()
        .map(|(_, _, bundle)| bundle)
        .collect::<Vec<_>>();
    disposition.support_bundle_count = bundles.len();
    if bundles.len() < 2 {
        disposition.exclusion =
            Some(RawPhaseExecutableBlueprintExclusionV1::IndependentSupportLineagesInsufficient);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    }

    let alignments = BoundedRoleAligner::align(&bundles, RoleAlignmentConfig::default());
    disposition.role_alignment_complete = alignments.completion.is_complete();
    if !disposition.role_alignment_complete {
        disposition.exclusion =
            Some(RawPhaseExecutableBlueprintExclusionV1::RoleAlignmentIncomplete);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    }
    let synthesis =
        BoundedCircuitBeam::synthesize(&bundles, &alignments, BlueprintBeamConfig::default());
    disposition.circuit_search_complete = synthesis.completion.is_complete();
    if !disposition.circuit_search_complete {
        disposition.exclusion =
            Some(RawPhaseExecutableBlueprintExclusionV1::CircuitSearchIncomplete);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    }
    if synthesis.blueprints.is_empty() {
        disposition.exclusion = Some(RawPhaseExecutableBlueprintExclusionV1::CircuitCandidateEmpty);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    }

    let Some(actor_commitment) = parse_commitment(&actor_root) else {
        disposition.exclusion = Some(RawPhaseExecutableBlueprintExclusionV1::ActorProgramInvalid);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    };
    let Some(verifier_commitment) = parse_commitment(&verifier_root) else {
        disposition.exclusion = Some(RawPhaseExecutableBlueprintExclusionV1::VerifierUnavailable);
        return RawPhaseExecutableBuildV1::excluded(disposition);
    };
    let bound_synthesis = BlueprintSynthesisReport {
        blueprints: synthesis
            .blueprints
            .iter()
            .cloned()
            .map(|blueprint| {
                blueprint.bind_executable_contracts(actor_commitment, verifier_commitment)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        expansions: synthesis.expansions,
        completion: synthesis.completion,
        blockers: synthesis.blockers.clone(),
    };
    let frozen = match FrozenOperatorBlueprintSet::freeze(
        support_watermark,
        &bundles,
        BlueprintBeamConfig::default(),
        &bound_synthesis,
    ) {
        Ok(frozen) => frozen,
        Err(_) => {
            disposition.exclusion =
                Some(RawPhaseExecutableBlueprintExclusionV1::BlueprintFreezeFailed);
            return RawPhaseExecutableBuildV1::excluded(disposition);
        }
    };
    disposition.support_bundle_root_sha256 =
        Some(commitment_hex(frozen.support_bundle_root_sha256()));
    disposition.frozen_blueprint_candidate_set_root_sha256 =
        Some(commitment_hex(frozen.candidate_set_sha256()));
    disposition.blueprint_fingerprints_sha256 = frozen
        .blueprints()
        .iter()
        .map(|blueprint| commitment_hex(blueprint.fingerprint_sha256()))
        .collect();
    disposition.blueprint_fingerprints_sha256.sort();
    disposition.blueprint_fingerprints_sha256.dedup();
    RawPhaseExecutableBuildV1 {
        disposition,
        selected_support_frame_ids_sha256,
        support_bundles: bundles,
        frozen: Some(frozen),
    }
}

fn program_surface_bundle(
    program: &ResponseProgram,
    support: &RawPhaseExecutableSupportV1<'_>,
) -> Result<SurfaceFragmentBundle, RawPhaseExecutableBlueprintExclusionV1> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. }
        | ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. } => projection_surface_bundle(program, support),
        ResponseOperation::ComposeCollection { .. } => collection_surface_bundle(program, support),
        _ => Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram),
    }
}

#[derive(Clone)]
struct ProjectionInput {
    selector: ResponseValueSelector,
    opcode: u8,
    parameter: u16,
    flags: u16,
}

fn projection_surface_bundle(
    program: &ResponseProgram,
    support: &RawPhaseExecutableSupportV1<'_>,
) -> Result<SurfaceFragmentBundle, RawPhaseExecutableBlueprintExclusionV1> {
    let inputs = projection_inputs(program)?;
    let expected_selectors = program_role_selectors(program)
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram)?;
    if inputs.len() != expected_selectors.len() || inputs.is_empty() {
        return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram);
    }
    let source_bindings = projection_source_bindings(&inputs, support.topology)?;

    let input_count = inputs.len();
    let role_count = 1_usize
        .saturating_add(input_count)
        .saturating_add(input_count);
    if role_count > 32 {
        return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram);
    }
    let planes = (0..input_count)
        .map(|index| u8::try_from(index).ok())
        .collect::<Option<Vec<_>>>()
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram)?;
    let phase = support_phase(support.frame, input_count)?;
    let mut roles = Vec::with_capacity(role_count);
    roles.push(runtime_context_role_signature(planes.clone()));
    for (index, (_, selector)) in source_bindings.iter().enumerate() {
        roles.push(runtime_source_role_signature(
            selector,
            planes[index],
            input_count,
        ));
    }
    for input in &inputs {
        roles.push(StructuralRoleSignature::new(
            output_type_tag(input),
            1,
            2,
            4,
            Vec::new(),
        ));
    }
    let relations = planes
        .iter()
        .enumerate()
        .map(|(index, plane)| LocalRelationFragment {
            plane: *plane,
            source_local_role: 0,
            target_local_role: u8::try_from(index + 1).expect("bounded role count"),
            state: TernaryRelationState::Supported,
            phase_anchor: phase[index],
        })
        .collect::<Vec<_>>();
    let atoms = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| TypedProgramAtom {
            opcode: input.opcode,
            output_local_role: u8::try_from(1 + input_count + index).expect("bounded role count"),
            source_a_local_role: u8::try_from(index + 1).expect("bounded role count"),
            source_b_local_role: OPERATOR_ROLE_NONE,
            parameter: input.parameter | (u16::try_from(index).unwrap_or(u16::MAX) << 8),
            flags: input.flags,
        })
        .collect();
    seal_surface_bundle(support, program, roles, relations, atoms)
}

fn projection_inputs(
    program: &ResponseProgram,
) -> Result<Vec<ProjectionInput>, RawPhaseExecutableBlueprintExclusionV1> {
    let mut inputs = Vec::new();
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            push_projection_input(
                &mut inputs,
                selector,
                TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
                transform_parameter(selector_value_type(selector)),
                0,
            );
        }
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            ..
        } => {
            push_projection_input(
                &mut inputs,
                selector,
                TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
                transform_parameter(selector_value_type(selector)),
                format_flag(*format),
            );
            push_renderer_inputs(&mut inputs, renderer);
        }
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            ..
        } => {
            push_projection_input(
                &mut inputs,
                selector,
                TRANSFORM_OPCODE_PROJECT_STATUS,
                transform_parameter(selector_value_type(selector)),
                status_mapping_flags(*mapping),
            );
            push_renderer_inputs(&mut inputs, renderer);
        }
        _ => return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram),
    }
    Ok(inputs)
}

fn push_renderer_inputs(inputs: &mut Vec<ProjectionInput>, renderer: &CollectionOutputRenderer) {
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, format } = segment {
                push_projection_input(
                    inputs,
                    selector,
                    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
                    transform_parameter(selector_value_type(selector)),
                    format_flag(*format),
                );
            }
        }
    }
}

fn push_projection_input(
    inputs: &mut Vec<ProjectionInput>,
    selector: &ResponseValueSelector,
    opcode: u8,
    parameter: u16,
    flags: u16,
) {
    if inputs.iter().any(|input| input.selector == *selector) {
        return;
    }
    inputs.push(ProjectionInput {
        selector: selector.clone(),
        opcode,
        parameter,
        flags,
    });
}

fn program_source_bindings(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<Vec<(u16, ResponseValueSelector)>, RawPhaseExecutableBlueprintExclusionV1> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. }
        | ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. } => {
            projection_source_bindings(&projection_inputs(program)?, topology)
        }
        ResponseOperation::ComposeCollection { .. } => {
            let manifest = pre_action_t1_input_binding_manifest_v1(program, topology)
                .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
            collection_source_bindings(&manifest.inputs, topology)
        }
        _ => Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram),
    }
}

fn projection_source_bindings(
    inputs: &[ProjectionInput],
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<Vec<(u16, ResponseValueSelector)>, RawPhaseExecutableBlueprintExclusionV1> {
    let mut bindings = Vec::with_capacity(inputs.len());
    let mut used_role_ids = BTreeSet::new();
    for input in inputs {
        let witness = witness_for_selector(&input.selector, topology)
            .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
        let role = role_for_witness(topology, witness)
            .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
        if !used_role_ids.insert(role.local_role_id) {
            return Err(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable);
        }
        bindings.push((role.local_role_id, input.selector.clone()));
    }
    Ok(bindings)
}

fn collection_source_bindings(
    inputs: &[PreActionT1ConsumedInputV1],
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<Vec<(u16, ResponseValueSelector)>, RawPhaseExecutableBlueprintExclusionV1> {
    let mut bindings = Vec::<(u16, ResponseValueSelector)>::new();
    let mut collection_role_id = None;
    for input in inputs {
        let (role_id, selector) = match input {
            PreActionT1ConsumedInputV1::CollectionSource { local_role_id, .. } => {
                collection_role_id = Some(*local_role_id);
                (
                    *local_role_id,
                    ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Collection,
                    },
                )
            }
            PreActionT1ConsumedInputV1::SelectedValue {
                local_role_id,
                selector,
                ..
            } => (*local_role_id, selector.clone()),
            PreActionT1ConsumedInputV1::ImplicitRequestValue {
                local_role_id,
                value_type,
                ..
            } => {
                let witness = topology
                    .role_witnesses
                    .iter()
                    .find(|witness| witness.local_role_id == *local_role_id)
                    .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
                let ordinal = witness.request_reference_ordinal.or_else(|| {
                    let [ordinal] = witness.request_reference_ordinal_candidates.as_slice() else {
                        return None;
                    };
                    Some(*ordinal)
                });
                let ordinal = ordinal
                    .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
                (
                    *local_role_id,
                    ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                        ordinal,
                        value_type: collection_scalar_atom_type(*value_type),
                    },
                )
            }
        };
        if let Some((_, known)) = bindings.iter().find(|(known, _)| *known == role_id) {
            if known != &selector {
                return Err(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable);
            }
        } else {
            bindings.push((role_id, selector));
        }
    }
    let collection_role_id = collection_role_id
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
    let collection_index = bindings
        .iter()
        .position(|(role_id, _)| *role_id == collection_role_id)
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
    let collection = bindings.remove(collection_index);
    bindings.insert(0, collection);
    Ok(bindings)
}

fn runtime_source_role_signature(
    selector: &ResponseValueSelector,
    plane: u8,
    input_count: usize,
) -> StructuralRoleSignature {
    let temporal_position = if input_count > 1 {
        match selector {
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal, .. } => {
                u8::try_from(ordinal.saturating_add(1)).unwrap_or(u8::MAX)
            }
            ResponseValueSelector::RequestLastToken
            | ResponseValueSelector::RequestUniqueLiteral => 0,
            _ => 1,
        }
    } else if matches!(
        selector,
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral
    ) {
        0
    } else {
        1
    };
    StructuralRoleSignature::new(
        type_class_tag(match selector_value_type(selector) {
            AtomValueType::String | AtomValueType::Identifier => MultiSourceTypeClassV1::String,
            AtomValueType::Integer => MultiSourceTypeClassV1::Number,
            AtomValueType::Boolean => MultiSourceTypeClassV1::Boolean,
            AtomValueType::Collection => MultiSourceTypeClassV1::Array,
        }),
        1,
        temporal_position,
        2 | if temporal_position == 0 {
            0x0100
        } else {
            0x0200
        },
        vec![plane],
    )
}

fn runtime_context_role_signature(planes: Vec<u8>) -> StructuralRoleSignature {
    StructuralRoleSignature::new(5, 1, 0, 1, planes)
}

fn collection_surface_bundle(
    program: &ResponseProgram,
    support: &RawPhaseExecutableSupportV1<'_>,
) -> Result<SurfaceFragmentBundle, RawPhaseExecutableBlueprintExclusionV1> {
    let ResponseOperation::ComposeCollection { steps, format, .. } = &program.operation else {
        return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram);
    };
    let manifest = pre_action_t1_input_binding_manifest_v1(program, support.topology)
        .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
    let source_bindings = collection_source_bindings(&manifest.inputs, support.topology)?;
    let role_ids = source_bindings
        .iter()
        .map(|(role_id, _)| *role_id)
        .collect::<Vec<_>>();
    let collection_role_id = manifest.inputs.iter().find_map(|input| match input {
        PreActionT1ConsumedInputV1::CollectionSource { local_role_id, .. } => Some(*local_role_id),
        _ => None,
    });
    let collection_role_id = collection_role_id
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
    if role_ids.first().copied() != Some(collection_role_id) || source_bindings.is_empty() {
        return Err(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable);
    }

    let mut effective_steps = steps.as_slice();
    if matches!(
        effective_steps.first(),
        Some(CollectionProgramStep::SelectTurnOutput { .. })
    ) {
        effective_steps = &effective_steps[1..];
    }
    if matches!(
        effective_steps.first(),
        Some(CollectionProgramStep::SelectOnlyArrayField)
    ) {
        effective_steps = &effective_steps[1..];
    }
    let filter = effective_steps.first().filter(|step| {
        matches!(
            step,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. }
                | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
        )
    });
    if filter.is_some() {
        effective_steps = &effective_steps[1..];
    }
    let count = matches!(effective_steps.first(), Some(CollectionProgramStep::Count));
    if count {
        effective_steps = &effective_steps[1..];
    }
    if !effective_steps.is_empty() {
        return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram);
    }

    let predicate_role_id = filter.and_then(|filter| match filter {
        CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. } => {
            manifest.inputs.iter().find_map(|input| match input {
                PreActionT1ConsumedInputV1::SelectedValue {
                    selector: bound,
                    local_role_id,
                    ..
                } if bound == selector => Some(*local_role_id),
                _ => None,
            })
        }
        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
            manifest.inputs.iter().find_map(|input| match input {
                PreActionT1ConsumedInputV1::ImplicitRequestValue {
                    value_type: bound,
                    local_role_id,
                    ..
                } if bound == value_type => Some(*local_role_id),
                _ => None,
            })
        }
        _ => None,
    });
    if filter.is_some() && predicate_role_id.is_none() {
        return Err(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable);
    }

    let input_count = source_bindings.len();
    let transform_count = if filter.is_some() {
        1 + usize::from(count)
    } else {
        1
    };
    let role_count = 1_usize
        .saturating_add(input_count)
        .saturating_add(transform_count);
    if role_count > 32 {
        return Err(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram);
    }
    let planes = (0..input_count)
        .map(|index| u8::try_from(index).ok())
        .collect::<Option<Vec<_>>>()
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram)?;
    let phase = support_phase(support.frame, input_count)?;
    let mut roles = Vec::with_capacity(role_count);
    roles.push(runtime_context_role_signature(planes.clone()));
    for (index, (_, selector)) in source_bindings.iter().enumerate() {
        roles.push(runtime_source_role_signature(
            selector,
            planes[index],
            input_count,
        ));
    }
    let first_output_type = if filter.is_some() {
        type_class_tag(MultiSourceTypeClassV1::Array)
    } else if count {
        type_class_tag(MultiSourceTypeClassV1::Number)
    } else {
        type_class_tag(MultiSourceTypeClassV1::Array)
    };
    roles.push(StructuralRoleSignature::new(
        first_output_type,
        1,
        2,
        4,
        Vec::new(),
    ));
    if filter.is_some() && count {
        roles.push(StructuralRoleSignature::new(
            type_class_tag(MultiSourceTypeClassV1::Number),
            1,
            2,
            4,
            Vec::new(),
        ));
    }
    let relations = planes
        .iter()
        .enumerate()
        .map(|(index, plane)| LocalRelationFragment {
            plane: *plane,
            source_local_role: 0,
            target_local_role: u8::try_from(index + 1).expect("bounded role count"),
            state: TernaryRelationState::Supported,
            phase_anchor: phase[index],
        })
        .collect::<Vec<_>>();
    let collection_local = 1_u8;
    let first_output = u8::try_from(1 + input_count).expect("bounded role count");
    let mut atoms = Vec::with_capacity(transform_count);
    if let Some(filter) = filter {
        let predicate_role_id = predicate_role_id.expect("filter predicate checked");
        let predicate_index = role_ids
            .iter()
            .position(|role_id| *role_id == predicate_role_id)
            .ok_or(RawPhaseExecutableBlueprintExclusionV1::SupportBindingUnavailable)?;
        let predicate_local = u8::try_from(predicate_index + 1).expect("bounded role count");
        let predicate_type = match filter {
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { value_type, .. }
            | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
                collection_scalar_atom_type(*value_type)
            }
            _ => unreachable!(),
        };
        atoms.push(TypedProgramAtom {
            opcode: TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
            output_local_role: first_output,
            source_a_local_role: collection_local,
            source_b_local_role: predicate_local,
            parameter: transform_parameter(predicate_type),
            flags: TRANSFORM_FLAG_CANONICAL_JSON,
        });
        if count {
            atoms.push(TypedProgramAtom {
                opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
                output_local_role: first_output + 1,
                source_a_local_role: first_output,
                source_b_local_role: TRANSFORM_ROLE_NONE,
                parameter: (1 << 8) | TRANSFORM_VALUE_COLLECTION,
                flags: 0,
            });
        }
    } else {
        atoms.push(TypedProgramAtom {
            opcode: if count {
                TRANSFORM_OPCODE_COUNT_COLLECTION
            } else {
                TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR
            },
            output_local_role: first_output,
            source_a_local_role: collection_local,
            source_b_local_role: TRANSFORM_ROLE_NONE,
            parameter: TRANSFORM_VALUE_COLLECTION,
            flags: format_flag(*format),
        });
    }
    seal_surface_bundle(support, program, roles, relations, atoms)
}

fn seal_surface_bundle(
    support: &RawPhaseExecutableSupportV1<'_>,
    program: &ResponseProgram,
    roles: Vec<StructuralRoleSignature>,
    relations: Vec<LocalRelationFragment>,
    atoms: Vec<TypedProgramAtom>,
) -> Result<SurfaceFragmentBundle, RawPhaseExecutableBlueprintExclusionV1> {
    let lineage = parse_commitment(support.lineage_root_sha256)
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    let shape_root = pre_action_applicability_shape_root_v1(support.topology)
        .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    let surface_root = canonical_json_sha256(&(
        "nando.raw-phase-executable-program-surface.v1",
        support.frame.frame_id_sha256.as_str(),
        shape_root.as_str(),
        relation_frame_phase_atom_ids(support.frame),
        response_program_version_root_sha256(program).unwrap_or_default(),
    ))
    .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    let surface = parse_commitment(&surface_root)
        .ok_or(RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)?;
    SurfaceFragmentBundle::new(lineage, surface, roles, relations, atoms)
        .map_err(|_| RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid)
}

fn support_phase(
    frame: &RelationFrame,
    cells: usize,
) -> Result<Vec<nando_core::wave::PhaseCenterCell>, RawPhaseExecutableBlueprintExclusionV1> {
    let atom_ids = relation_frame_phase_atom_ids(frame);
    if atom_ids.is_empty() || cells == 0 {
        return Err(RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid);
    }
    let phase = phase_vector_from_atom_ids(atom_ids, cells);
    if phase.iter().any(|cell| {
        !cell.re.is_finite() || !cell.im.is_finite() || cell.re.hypot(cell.im) <= f64::EPSILON
    }) {
        return Err(RawPhaseExecutableBlueprintExclusionV1::SurfaceBundleInvalid);
    }
    Ok(phase)
}

const fn type_class_tag(value: MultiSourceTypeClassV1) -> u8 {
    match value {
        MultiSourceTypeClassV1::Null => 0,
        MultiSourceTypeClassV1::String => 1,
        MultiSourceTypeClassV1::Number => 2,
        MultiSourceTypeClassV1::Boolean => 3,
        MultiSourceTypeClassV1::Array => 5,
        MultiSourceTypeClassV1::Object => 6,
    }
}

const fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => *value_type,
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => AtomValueType::String,
    }
}

const fn transform_parameter(value_type: AtomValueType) -> u16 {
    match value_type {
        AtomValueType::String => TRANSFORM_VALUE_STRING,
        AtomValueType::Integer => TRANSFORM_VALUE_INTEGER,
        AtomValueType::Boolean => TRANSFORM_VALUE_BOOLEAN,
        AtomValueType::Identifier => TRANSFORM_VALUE_IDENTIFIER,
        AtomValueType::Collection => TRANSFORM_VALUE_COLLECTION,
    }
}

const fn collection_scalar_atom_type(value_type: CollectionScalarType) -> AtomValueType {
    match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    }
}

const fn format_flag(format: ValueProjectionFormat) -> u16 {
    if matches!(format, ValueProjectionFormat::CanonicalJson) {
        TRANSFORM_FLAG_CANONICAL_JSON
    } else {
        0
    }
}

const fn status_mapping_flags(mapping: ProjectStatusMapping) -> u16 {
    match mapping {
        ProjectStatusMapping::ZeroIsSuccess => TRANSFORM_STATUS_ZERO_IS_SUCCESS,
        ProjectStatusMapping::ZeroIsPass => TRANSFORM_STATUS_ZERO_IS_PASS,
        ProjectStatusMapping::ZeroIsOk => TRANSFORM_STATUS_ZERO_IS_OK,
        ProjectStatusMapping::ZeroIsTrue => TRANSFORM_STATUS_ZERO_IS_TRUE,
    }
}

fn output_type_tag(input: &ProjectionInput) -> u8 {
    type_class_tag(match selector_value_type(&input.selector) {
        AtomValueType::String | AtomValueType::Identifier => MultiSourceTypeClassV1::String,
        AtomValueType::Integer => MultiSourceTypeClassV1::Number,
        AtomValueType::Boolean => MultiSourceTypeClassV1::Boolean,
        AtomValueType::Collection => MultiSourceTypeClassV1::Array,
    })
}

fn parse_commitment(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16).ok()?;
    }
    (digest != [0; 32]).then_some(digest)
}

fn commitment_hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn strict_roots(roots: &[String], allow_empty: bool) -> bool {
    (allow_empty || !roots.is_empty())
        && roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::{
        MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
        MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
        MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, RELATION_FRAME_SCHEMA, RelationAtom,
        sha256_bytes,
    };

    struct ExecutableFixture {
        hypothesis: RawPhaseT1HypothesisEnvelopeV1,
        frames: Vec<RelationFrame>,
        lineages: Vec<String>,
        topologies: Vec<PreActionMultiSourceTopologyV1>,
        programs: BTreeMap<String, ResponseProgram>,
    }

    impl ExecutableFixture {
        fn new(
            programs: BTreeMap<String, ResponseProgram>,
            topologies: Vec<PreActionMultiSourceTopologyV1>,
        ) -> Self {
            assert_eq!(topologies.len(), 2);
            let frames = vec![frame("support-a"), frame("support-b")];
            let lineages = vec![root("lineage-a"), root("lineage-b")];
            let hypothesis = super::super::seal_raw_phase_t1_hypothesis_envelope_v1(
                root("frozen-domain"),
                2,
                &frames,
                lineages.clone(),
                Vec::new(),
                &programs,
            )
            .expect("hypothesis envelope");
            Self {
                hypothesis,
                frames,
                lineages,
                topologies,
                programs,
            }
        }

        fn support(&self, order: &[usize]) -> Vec<RawPhaseExecutableSupportV1<'_>> {
            order
                .iter()
                .map(|index| RawPhaseExecutableSupportV1 {
                    capture_sequence: u64::try_from(index + 1).expect("bounded support"),
                    lineage_root_sha256: &self.lineages[*index],
                    topology: &self.topologies[*index],
                    frame: &self.frames[*index],
                })
                .collect()
        }

        fn seal(
            &self,
            order: &[usize],
        ) -> Result<RawPhaseExecutableBlueprintEnvelopeV1, &'static str> {
            seal_raw_phase_executable_blueprint_envelope_v1(
                &self.hypothesis,
                &self.support(order),
                &self.programs,
            )
        }
    }

    fn root(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn frame(label: &str) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: root(&format!("frame:{label}")),
            event_id_sha256: root(&format!("event:{label}")),
            client_intent_id_sha256: root(&format!("intent:{label}")),
            session_id_sha256: root(&format!("session:{label}")),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 1,
            extractor_version: "test".to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::RequestPhaseAtom { atom_id: 7 },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
            ],
            evidence_ref_sha256: root(&format!("evidence:{label}")),
        }
    }

    fn scalar_topology(permuted: bool) -> PreActionMultiSourceTopologyV1 {
        let semantic_roles = [(0_u16, 1_u16), (1_u16, 2_u16)];
        let mut roles = Vec::new();
        let mut witnesses = Vec::new();
        let mut relations = Vec::new();
        for (ordinal, structural_flags) in semantic_roles {
            let local_role_id = if permuted { 1 - ordinal } else { ordinal };
            roles.push(MultiSourceRoleNodeV1 {
                local_role_id,
                source_ordinal: ordinal,
                value_ordinal: ordinal,
                type_class: MultiSourceTypeClassV1::String,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags,
            });
            witnesses.push(MultiSourceRoleWitnessV1 {
                local_role_id,
                value_sha256: root(&format!("value:{ordinal}")),
                request_reference_ordinal: Some(ordinal),
                request_reference_ordinal_candidates: Vec::new(),
            });
            relations.push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: local_role_id,
                target_role_id: local_role_id,
            });
        }
        roles.sort_by_key(|role| role.local_role_id);
        witnesses.sort_by_key(|witness| witness.local_role_id);
        relations.sort();
        let topology = PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 2,
            roles,
            role_witnesses: witnesses,
            relations,
        };
        topology.validate().expect("scalar topology");
        topology
    }

    fn collection_topology() -> PreActionMultiSourceTopologyV1 {
        let topology = PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 2,
            roles: vec![
                MultiSourceRoleNodeV1 {
                    local_role_id: 0,
                    source_ordinal: 0,
                    value_ordinal: 0,
                    type_class: MultiSourceTypeClassV1::Array,
                    container_class: MultiSourceContainerClassV1::Sequence,
                    cardinality_class: MultiSourceCardinalityClassV1::Many,
                    temporal_class: MultiSourceTemporalClassV1::Latest,
                    depth_bucket: 1,
                    structural_flags: 1,
                },
                MultiSourceRoleNodeV1 {
                    local_role_id: 1,
                    source_ordinal: 1,
                    value_ordinal: 1,
                    type_class: MultiSourceTypeClassV1::String,
                    container_class: MultiSourceContainerClassV1::Scalar,
                    cardinality_class: MultiSourceCardinalityClassV1::One,
                    temporal_class: MultiSourceTemporalClassV1::Latest,
                    depth_bucket: 1,
                    structural_flags: 2,
                },
            ],
            role_witnesses: vec![
                MultiSourceRoleWitnessV1 {
                    local_role_id: 0,
                    value_sha256: root("collection"),
                    request_reference_ordinal: Some(0),
                    request_reference_ordinal_candidates: Vec::new(),
                },
                MultiSourceRoleWitnessV1 {
                    local_role_id: 1,
                    value_sha256: root("predicate"),
                    request_reference_ordinal: Some(1),
                    request_reference_ordinal_candidates: Vec::new(),
                },
            ],
            relations: vec![
                MultiSourceRelationEdgeV1 {
                    relation: MultiSourceRelationKindV1::RequestReferencesRole,
                    source_role_id: 0,
                    target_role_id: 0,
                },
                MultiSourceRelationEdgeV1 {
                    relation: MultiSourceRelationKindV1::RequestReferencesRole,
                    source_role_id: 1,
                    target_role_id: 1,
                },
            ],
        };
        topology.validate().expect("collection topology");
        topology
    }

    fn rich_projection_program() -> ResponseProgram {
        ResponseProgram::project_selected_value(
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal: 0,
                value_type: AtomValueType::String,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_value_renderer(CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Selected {
                    selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                        ordinal: 1,
                        value_type: AtomValueType::String,
                    },
                    format: ValueProjectionFormat::PlainText,
                },
            ],
        })
    }

    fn program_map(
        programs: impl IntoIterator<Item = ResponseProgram>,
    ) -> BTreeMap<String, ResponseProgram> {
        programs
            .into_iter()
            .map(|program| {
                let root = response_program_version_root_sha256(&program).expect("program root");
                (root, program)
            })
            .collect()
    }

    #[test]
    fn executable_envelope_is_support_order_independent() {
        let fixture = ExecutableFixture::new(
            program_map([rich_projection_program()]),
            vec![scalar_topology(false), scalar_topology(false)],
        );

        assert_eq!(
            fixture.seal(&[0, 1]).expect("forward envelope"),
            fixture.seal(&[1, 0]).expect("reversed envelope")
        );
    }

    #[test]
    fn executable_envelope_rejects_post_watermark_support() {
        let fixture = ExecutableFixture::new(
            program_map([rich_projection_program()]),
            vec![scalar_topology(false), scalar_topology(false)],
        );
        let support = [
            RawPhaseExecutableSupportV1 {
                capture_sequence: 1,
                lineage_root_sha256: &fixture.lineages[0],
                topology: &fixture.topologies[0],
                frame: &fixture.frames[0],
            },
            RawPhaseExecutableSupportV1 {
                capture_sequence: 3,
                lineage_root_sha256: &fixture.lineages[1],
                topology: &fixture.topologies[1],
                frame: &fixture.frames[1],
            },
        ];

        assert_eq!(
            seal_raw_phase_executable_blueprint_envelope_v1(
                &fixture.hypothesis,
                &support,
                &fixture.programs,
            ),
            Err("raw_phase_executable_support_mismatch")
        );
    }

    #[test]
    fn executable_candidate_set_is_local_role_permutation_invariant() {
        let programs = program_map([rich_projection_program()]);
        let canonical = ExecutableFixture::new(
            programs.clone(),
            vec![scalar_topology(false), scalar_topology(false)],
        )
        .seal(&[0, 1])
        .expect("canonical envelope");
        let permuted =
            ExecutableFixture::new(programs, vec![scalar_topology(true), scalar_topology(true)])
                .seal(&[0, 1])
                .expect("permuted envelope");

        assert_eq!(canonical.dispositions.len(), 1);
        assert_eq!(permuted.dispositions.len(), 1);
        assert_eq!(
            canonical.dispositions[0].frozen_blueprint_candidate_set_root_sha256,
            permuted.dispositions[0].frozen_blueprint_candidate_set_root_sha256
        );
        assert_eq!(
            canonical.dispositions[0].blueprint_fingerprints_sha256,
            permuted.dispositions[0].blueprint_fingerprints_sha256
        );
    }

    #[test]
    fn unsupported_typed_program_gets_explicit_terminal_exclusion() {
        let unsupported = ResponseProgram::advance_plan("update_plan");
        let unsupported_root =
            response_program_version_root_sha256(&unsupported).expect("unsupported root");
        let fixture = ExecutableFixture::new(
            program_map([rich_projection_program(), unsupported]),
            vec![scalar_topology(false), scalar_topology(false)],
        );
        let envelope = fixture.seal(&[0, 1]).expect("mixed envelope");
        let disposition = envelope
            .dispositions
            .iter()
            .find(|entry| entry.program_root_sha256 == unsupported_root)
            .expect("unsupported disposition");

        assert_eq!(
            disposition.exclusion,
            Some(RawPhaseExecutableBlueprintExclusionV1::UnsupportedTypedProgram)
        );
        assert!(!disposition.executable());
        assert_eq!(envelope.executable_program_roots_sha256.len(), 1);
    }

    #[test]
    fn authority_and_phase_tampering_are_vetoed() {
        let fixture = ExecutableFixture::new(
            program_map([rich_projection_program()]),
            vec![scalar_topology(false), scalar_topology(false)],
        );
        let envelope = fixture.seal(&[0, 1]).expect("envelope");

        let mut authority = envelope.clone();
        authority.authority_ready = true;
        assert_eq!(
            authority.validate(),
            Err("raw_phase_executable_blueprint_envelope_invalid")
        );

        let mut phase = envelope;
        phase.phase_mutation_allowed = true;
        assert_eq!(
            phase.validate(),
            Err("raw_phase_executable_blueprint_envelope_invalid")
        );
    }

    #[test]
    fn collection_filter_count_uses_structural_output_types() {
        let program = ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::FilterUniqueFieldEqualsRequestValue {
                    value_type: CollectionScalarType::String,
                },
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let topology = collection_topology();
        let frame = frame("collection");
        let lineage = root("collection-lineage");
        let support = RawPhaseExecutableSupportV1 {
            capture_sequence: 1,
            lineage_root_sha256: &lineage,
            topology: &topology,
            frame: &frame,
        };

        let bundle = program_surface_bundle(&program, &support).expect("collection bundle");
        let roles = bundle.roles();
        assert_eq!(roles[roles.len() - 2].type_class(), 5);
        assert_eq!(roles[roles.len() - 1].type_class(), 2);
        assert_eq!(bundle.program_atoms().len(), 2);
    }
}
