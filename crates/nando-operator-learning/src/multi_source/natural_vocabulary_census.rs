use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceContainerClassV1, MultiSourceTypeClassV1, RelationFrame, ResponseOperation,
    ResponseProgram, canonical_json_bytes, canonical_json_sha256,
    response_program_version_root_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use super::{
    BlindThenRevealJoinedTransitionV1, CompletedEffectFormV1,
    K1_CANDIDATE_READINESS_MIN_LINEAGES_V1, K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1,
    K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1, K1ConsequenceTypeV1,
    MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2, MultiSourceJoinLedgerV1, MultiSourceJoinReportV1,
    PreActionTopologyAuditRowV1, active_t1_protocol_mode_root_v1, factor_multi_source_row_v1,
};

pub const NATURAL_VOCABULARY_CENSUS_SCHEMA_V1: &str = "nando.natural-vocabulary-census.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NaturalVocabularyOperationFormV1 {
    UniqueConsensus,
    AdvancePlan,
    FunctionCallFromRoles,
    CustomToolCallFromRoles,
    ProjectSelectedValue,
    ProjectStatus,
    ComposeCollection,
    CopyAfterPrefix,
    TestResultSummary,
    WaitOnYieldedCell,
    WaitOnAnyYieldedCell,
    WaitOnYieldedSurfaces,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NaturalVocabularyCensusVerdictV1 {
    ReadyForm,
    NoReadyForm,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalVocabularyFormCensusV1 {
    pub operation_form: NaturalVocabularyOperationFormV1,
    pub form_root_sha256: String,
    pub joined_rows: u64,
    pub accepted_rows: u64,
    pub input_tokens: u64,
    pub accepted_input_tokens: u64,
    pub independent_lineages: u64,
    pub physical_candidate_rows: u64,
    pub physical_candidate_programs: u64,
    pub exact_replay_rows: u64,
    pub verifier_compilable_rows: u64,
    pub capture_v2_rows: u64,
    pub readiness_settled_rows: u64,
    pub readiness_verified_rows: u64,
    pub readiness_independent_lineages: u64,
    pub readiness_pass: bool,
    pub readiness_blocker: String,
    pub source_neutral_candidate_rows: u64,
    pub source_neutral_candidate_programs: u64,
    pub current_protocol_mode_represented_rows: u64,
    pub current_protocol_mode_roots: u64,
    pub completed_effect_classes: BTreeMap<CompletedEffectFormV1, u64>,
    pub consequence_classes: BTreeMap<K1ConsequenceTypeV1, u64>,
    pub blocker_counts: BTreeMap<String, u64>,
    pub bounded_discovery_cost_units: u64,
    pub expected_verified_input_tokens: u64,
    pub missing_from_current_vocabulary: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalVocabularyCensusV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub topology_archive_root_sha256: String,
    pub frame_archive_root_sha256: String,
    pub joined_archive_root_sha256: String,
    pub join_report_root_sha256: String,
    pub source_root_sha256: String,
    pub topology_rows: u64,
    pub frame_rows: u64,
    pub joined_rows: u64,
    pub accepted_rows: u64,
    pub rows_without_physical_candidates: u64,
    pub forms: Vec<NaturalVocabularyFormCensusV1>,
    pub selected_missing_form: Option<NaturalVocabularyOperationFormV1>,
    pub verdict: NaturalVocabularyCensusVerdictV1,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct NaturalVocabularyCensusDigestV1<'a> {
    schema: &'a str,
    topology_archive_root_sha256: &'a str,
    frame_archive_root_sha256: &'a str,
    joined_archive_root_sha256: &'a str,
    join_report_root_sha256: &'a str,
    source_root_sha256: &'a str,
    topology_rows: u64,
    frame_rows: u64,
    joined_rows: u64,
    accepted_rows: u64,
    rows_without_physical_candidates: u64,
    forms: &'a [NaturalVocabularyFormCensusV1],
    selected_missing_form: Option<NaturalVocabularyOperationFormV1>,
    verdict: NaturalVocabularyCensusVerdictV1,
    blocker: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Default)]
struct FormAccumulatorV1 {
    joined_rows: u64,
    accepted_rows: u64,
    input_tokens: u64,
    accepted_input_tokens: u64,
    lineages: BTreeSet<String>,
    physical_program_roots: BTreeSet<String>,
    exact_replay_rows: u64,
    verifier_compilable_rows: u64,
    capture_v2_rows: u64,
    readiness_settled_rows: u64,
    readiness_verified_rows: u64,
    readiness_lineages: BTreeSet<String>,
    source_neutral_candidate_rows: u64,
    source_neutral_program_roots: BTreeSet<String>,
    current_protocol_mode_represented_rows: u64,
    current_protocol_mode_roots: BTreeSet<String>,
    completed_effect_classes: BTreeMap<CompletedEffectFormV1, u64>,
    consequence_classes: BTreeMap<K1ConsequenceTypeV1, u64>,
    blocker_counts: BTreeMap<String, u64>,
}

#[derive(Default)]
struct ProgramPartitionV1 {
    programs: BTreeMap<NaturalVocabularyOperationFormV1, BTreeMap<String, ResponseProgram>>,
    blockers: BTreeMap<NaturalVocabularyOperationFormV1, BTreeMap<String, u64>>,
}

pub fn build_natural_vocabulary_census_v1(
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
) -> Result<NaturalVocabularyCensusV1, &'static str> {
    let topology_archive_root_sha256 =
        archive_manifest_root_v1("nando.natural-vocabulary-topology-archive.v1", topologies)?;
    let frame_archive_root_sha256 =
        archive_manifest_root_v1("nando.natural-vocabulary-frame-archive.v1", frames)?;
    let ledger = MultiSourceJoinLedgerV1::build(topologies, frames);
    let joined = ledger.rows();
    let join_report = ledger.report();
    build_from_joined_v1(
        topology_archive_root_sha256,
        frame_archive_root_sha256,
        u64::try_from(topologies.len()).map_err(|_| "natural_vocabulary_census_count")?,
        u64::try_from(frames.len()).map_err(|_| "natural_vocabulary_census_count")?,
        &joined,
        frames,
        join_report,
    )
}

impl NaturalVocabularyCensusV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let selected_consistent = match self.verdict {
            NaturalVocabularyCensusVerdictV1::ReadyForm => {
                self.selected_missing_form.is_some() && self.blocker.is_empty()
            }
            NaturalVocabularyCensusVerdictV1::NoReadyForm => {
                self.selected_missing_form.is_none() && self.blocker == "no_ready_form"
            }
        };
        if self.schema != NATURAL_VOCABULARY_CENSUS_SCHEMA_V1
            || !selected_consistent
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.accepted_rows > self.joined_rows
            || self
                .forms
                .windows(2)
                .any(|pair| pair[0].operation_form >= pair[1].operation_form)
            || [
                self.report_root_sha256.as_str(),
                self.topology_archive_root_sha256.as_str(),
                self.frame_archive_root_sha256.as_str(),
                self.joined_archive_root_sha256.as_str(),
                self.join_report_root_sha256.as_str(),
                self.source_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.forms.iter().any(|form| form.validate().is_err())
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("natural_vocabulary_census_invalid");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        canonical_json_bytes(self)
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&NaturalVocabularyCensusDigestV1 {
            schema: self.schema.as_str(),
            topology_archive_root_sha256: self.topology_archive_root_sha256.as_str(),
            frame_archive_root_sha256: self.frame_archive_root_sha256.as_str(),
            joined_archive_root_sha256: self.joined_archive_root_sha256.as_str(),
            join_report_root_sha256: self.join_report_root_sha256.as_str(),
            source_root_sha256: self.source_root_sha256.as_str(),
            topology_rows: self.topology_rows,
            frame_rows: self.frame_rows,
            joined_rows: self.joined_rows,
            accepted_rows: self.accepted_rows,
            rows_without_physical_candidates: self.rows_without_physical_candidates,
            forms: &self.forms,
            selected_missing_form: self.selected_missing_form,
            verdict: self.verdict,
            blocker: self.blocker.as_str(),
            authority_ready: self.authority_ready,
            phase_mutation_allowed: self.phase_mutation_allowed,
        })
    }
}

impl NaturalVocabularyFormCensusV1 {
    fn validate(&self) -> Result<(), &'static str> {
        let readiness_pass = self.readiness_settled_rows
            >= K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1
            && self.readiness_verified_rows >= K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1
            && self.readiness_independent_lineages >= K1_CANDIDATE_READINESS_MIN_LINEAGES_V1;
        let expected_blocker = readiness_blocker_v1(
            self.readiness_settled_rows,
            self.readiness_verified_rows,
            self.readiness_independent_lineages,
        );
        if !valid_nonzero_sha256(&self.form_root_sha256)
            || self.form_root_sha256
                != canonical_json_sha256(&(
                    "nando.natural-vocabulary-operation-form.v1",
                    self.operation_form,
                ))?
            || self.joined_rows != self.physical_candidate_rows
            || self.accepted_rows > self.joined_rows
            || self.capture_v2_rows > self.joined_rows
            || self.exact_replay_rows > self.joined_rows
            || self.verifier_compilable_rows > self.joined_rows
            || self.readiness_settled_rows > self.capture_v2_rows
            || self.readiness_verified_rows > self.readiness_settled_rows
            || self.source_neutral_candidate_rows > self.joined_rows
            || self.current_protocol_mode_represented_rows > self.joined_rows
            || self.readiness_pass != readiness_pass
            || self.readiness_blocker != expected_blocker
            || self.bounded_discovery_cost_units != self.physical_candidate_programs.max(1)
            || self.expected_verified_input_tokens != self.accepted_input_tokens
            || self.missing_from_current_vocabulary
                != (self.current_protocol_mode_represented_rows == 0)
            || self.authority_ready
            || self.phase_mutation_allowed
        {
            return Err("natural_vocabulary_form_census_invalid");
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn build_from_joined_v1(
    topology_archive_root_sha256: String,
    frame_archive_root_sha256: String,
    topology_rows: u64,
    frame_rows: u64,
    joined: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    join_report: MultiSourceJoinReportV1,
) -> Result<NaturalVocabularyCensusV1, &'static str> {
    let frame_by_root = frames
        .iter()
        .map(|frame| Ok((canonical_json_sha256(frame)?, frame)))
        .collect::<Result<BTreeMap<_, _>, &'static str>>()?;
    let joined_archive_root_sha256 =
        archive_manifest_root_v1("nando.natural-vocabulary-joined-archive.v1", joined)?;
    let join_report_root_sha256 = canonical_json_sha256(&join_report)?;
    let source_root_sha256 = canonical_json_sha256(&(
        "nando.natural-vocabulary-census-source.v1",
        topology_archive_root_sha256.as_str(),
        frame_archive_root_sha256.as_str(),
        joined_archive_root_sha256.as_str(),
        join_report_root_sha256.as_str(),
    ))?;

    let mut accumulators = BTreeMap::<NaturalVocabularyOperationFormV1, FormAccumulatorV1>::new();
    let mut rows_without_physical_candidates = 0u64;
    for row in joined {
        row.validate()?;
        let frame = frame_by_root
            .get(&row.completed_frame_root_sha256)
            .copied()
            .ok_or("natural_vocabulary_joined_frame_missing")?;
        let physical =
            crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(frame));
        if physical.is_empty() {
            rows_without_physical_candidates = rows_without_physical_candidates.saturating_add(1);
            continue;
        }
        let physical = partition_programs_v1(physical);
        let physical_forms = physical
            .programs
            .keys()
            .chain(physical.blockers.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let source_neutral = super::source_neutral_t1::enumerate_source_neutral_t1_candidates(
            row, frame,
        )
        .map(|candidates| partition_programs_v1(candidates.into_values().collect::<Vec<_>>()));
        let factorized = factor_multi_source_row_v1(row);
        let consequence = consequence_type_v1(row, factorized.completed_effect);
        let capture_v2 = row.schema == super::BLIND_THEN_REVEAL_JOIN_SCHEMA_V2
            && row.capture_generation_root_sha256
                == canonical_json_sha256(&(
                    MULTI_SOURCE_CAPTURE_GENERATION_SCHEMA_V2,
                    row.extractor_root_sha256.as_str(),
                    row.extractor_config_root_sha256.as_str(),
                ))?;

        for form in physical_forms {
            let empty_programs = BTreeMap::new();
            let programs = physical.programs.get(&form).unwrap_or(&empty_programs);
            let accumulator = accumulators.entry(form).or_default();
            accumulator.joined_rows = accumulator.joined_rows.saturating_add(1);
            accumulator.input_tokens = accumulator.input_tokens.saturating_add(row.input_tokens);
            accumulator
                .lineages
                .insert(row.session_lineage_sha256.clone());
            if row.accepted {
                accumulator.accepted_rows = accumulator.accepted_rows.saturating_add(1);
                accumulator.accepted_input_tokens = accumulator
                    .accepted_input_tokens
                    .saturating_add(row.input_tokens);
            }
            if capture_v2 {
                accumulator.capture_v2_rows = accumulator.capture_v2_rows.saturating_add(1);
            }
            increment(
                &mut accumulator.completed_effect_classes,
                factorized.completed_effect,
            );
            increment(&mut accumulator.consequence_classes, consequence);
            accumulator
                .physical_program_roots
                .extend(programs.keys().cloned());
            if let Some(blockers) = physical.blockers.get(&form) {
                merge_counts(&mut accumulator.blocker_counts, blockers);
            }
            let protocol_roots = programs
                .values()
                .filter_map(active_t1_protocol_mode_root_v1)
                .collect::<BTreeSet<_>>();
            if !protocol_roots.is_empty() {
                accumulator.current_protocol_mode_represented_rows = accumulator
                    .current_protocol_mode_represented_rows
                    .saturating_add(1);
                accumulator
                    .current_protocol_mode_roots
                    .extend(protocol_roots);
            }

            let exact_replay = programs
                .values()
                .any(|program| crate::synthesis::program_is_consistent(program, frame));
            let verifier_compilable = programs
                .values()
                .any(|program| crate::synthesis::compile_independent_verifier(program).is_ok());
            if exact_replay {
                accumulator.exact_replay_rows = accumulator.exact_replay_rows.saturating_add(1);
            }
            if verifier_compilable {
                accumulator.verifier_compilable_rows =
                    accumulator.verifier_compilable_rows.saturating_add(1);
            }
            if capture_v2 && exact_replay && verifier_compilable {
                accumulator.readiness_settled_rows =
                    accumulator.readiness_settled_rows.saturating_add(1);
                accumulator
                    .readiness_lineages
                    .insert(row.session_lineage_sha256.clone());
                if row.accepted {
                    accumulator.readiness_verified_rows =
                        accumulator.readiness_verified_rows.saturating_add(1);
                }
            }

            match &source_neutral {
                Ok(partition) => {
                    if let Some(blockers) = partition.blockers.get(&form) {
                        merge_counts(&mut accumulator.blocker_counts, blockers);
                    }
                    if let Some(neutral) = partition.programs.get(&form) {
                        accumulator.source_neutral_candidate_rows =
                            accumulator.source_neutral_candidate_rows.saturating_add(1);
                        accumulator
                            .source_neutral_program_roots
                            .extend(neutral.keys().cloned());
                    } else {
                        increment(
                            &mut accumulator.blocker_counts,
                            "source_neutral_form_missing".to_owned(),
                        );
                    }
                }
                Err(blocker) => increment(&mut accumulator.blocker_counts, (*blocker).to_owned()),
            }
        }
    }

    let mut forms = accumulators
        .into_iter()
        .map(|(form, accumulator)| seal_form_v1(form, accumulator))
        .collect::<Result<Vec<_>, _>>()?;
    forms.sort_by_key(|form| form.operation_form);
    let selected_missing_form = forms
        .iter()
        .filter(|form| form.readiness_pass && form.missing_from_current_vocabulary)
        .min_by(|left, right| rank_ready_forms_v1(left, right))
        .map(|form| form.operation_form);
    let (verdict, blocker) = if selected_missing_form.is_some() {
        (NaturalVocabularyCensusVerdictV1::ReadyForm, String::new())
    } else {
        (
            NaturalVocabularyCensusVerdictV1::NoReadyForm,
            "no_ready_form".to_owned(),
        )
    };
    let mut report = NaturalVocabularyCensusV1 {
        schema: NATURAL_VOCABULARY_CENSUS_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        topology_archive_root_sha256,
        frame_archive_root_sha256,
        joined_archive_root_sha256,
        join_report_root_sha256,
        source_root_sha256,
        topology_rows,
        frame_rows,
        joined_rows: join_report.joined_rows,
        accepted_rows: join_report.accepted_rows,
        rows_without_physical_candidates,
        forms,
        selected_missing_form,
        verdict,
        blocker,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    report.report_root_sha256 = report.expected_root()?;
    report.validate()?;
    Ok(report)
}

fn partition_programs_v1(programs: Vec<ResponseProgram>) -> ProgramPartitionV1 {
    let mut partition = ProgramPartitionV1::default();
    for program in programs {
        let form = operation_form_v1(&program);
        match program
            .validate()
            .and_then(|()| response_program_version_root_sha256(&program))
        {
            Ok(root) => {
                partition
                    .programs
                    .entry(form)
                    .or_default()
                    .insert(root, program);
            }
            Err(blocker) => increment(
                partition.blockers.entry(form).or_default(),
                blocker.to_owned(),
            ),
        }
    }
    partition
}

fn seal_form_v1(
    operation_form: NaturalVocabularyOperationFormV1,
    accumulator: FormAccumulatorV1,
) -> Result<NaturalVocabularyFormCensusV1, &'static str> {
    let readiness_independent_lineages = u64::try_from(accumulator.readiness_lineages.len())
        .map_err(|_| "natural_vocabulary_census_count")?;
    let readiness_pass = accumulator.readiness_settled_rows
        >= K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1
        && accumulator.readiness_verified_rows >= K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1
        && readiness_independent_lineages >= K1_CANDIDATE_READINESS_MIN_LINEAGES_V1;
    let form = NaturalVocabularyFormCensusV1 {
        operation_form,
        form_root_sha256: canonical_json_sha256(&(
            "nando.natural-vocabulary-operation-form.v1",
            operation_form,
        ))?,
        joined_rows: accumulator.joined_rows,
        accepted_rows: accumulator.accepted_rows,
        input_tokens: accumulator.input_tokens,
        accepted_input_tokens: accumulator.accepted_input_tokens,
        independent_lineages: u64::try_from(accumulator.lineages.len())
            .map_err(|_| "natural_vocabulary_census_count")?,
        physical_candidate_rows: accumulator.joined_rows,
        physical_candidate_programs: u64::try_from(accumulator.physical_program_roots.len())
            .map_err(|_| "natural_vocabulary_census_count")?,
        exact_replay_rows: accumulator.exact_replay_rows,
        verifier_compilable_rows: accumulator.verifier_compilable_rows,
        capture_v2_rows: accumulator.capture_v2_rows,
        readiness_settled_rows: accumulator.readiness_settled_rows,
        readiness_verified_rows: accumulator.readiness_verified_rows,
        readiness_independent_lineages,
        readiness_pass,
        readiness_blocker: readiness_blocker_v1(
            accumulator.readiness_settled_rows,
            accumulator.readiness_verified_rows,
            readiness_independent_lineages,
        ),
        source_neutral_candidate_rows: accumulator.source_neutral_candidate_rows,
        source_neutral_candidate_programs: u64::try_from(
            accumulator.source_neutral_program_roots.len(),
        )
        .map_err(|_| "natural_vocabulary_census_count")?,
        current_protocol_mode_represented_rows: accumulator.current_protocol_mode_represented_rows,
        current_protocol_mode_roots: u64::try_from(accumulator.current_protocol_mode_roots.len())
            .map_err(|_| "natural_vocabulary_census_count")?,
        completed_effect_classes: accumulator.completed_effect_classes,
        consequence_classes: accumulator.consequence_classes,
        blocker_counts: accumulator.blocker_counts,
        bounded_discovery_cost_units: u64::try_from(accumulator.physical_program_roots.len())
            .map_err(|_| "natural_vocabulary_census_count")?
            .max(1),
        expected_verified_input_tokens: accumulator.accepted_input_tokens,
        missing_from_current_vocabulary: accumulator.current_protocol_mode_represented_rows == 0,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    form.validate()?;
    Ok(form)
}

fn operation_form_v1(program: &ResponseProgram) -> NaturalVocabularyOperationFormV1 {
    match &program.operation {
        ResponseOperation::UniqueConsensus { .. } => {
            NaturalVocabularyOperationFormV1::UniqueConsensus
        }
        ResponseOperation::AdvancePlan { .. } => NaturalVocabularyOperationFormV1::AdvancePlan,
        ResponseOperation::FunctionCallFromRoles { .. } => {
            NaturalVocabularyOperationFormV1::FunctionCallFromRoles
        }
        ResponseOperation::CustomToolCallFromRoles { .. } => {
            NaturalVocabularyOperationFormV1::CustomToolCallFromRoles
        }
        ResponseOperation::ProjectSelectedValue { .. } => {
            NaturalVocabularyOperationFormV1::ProjectSelectedValue
        }
        ResponseOperation::ProjectStatus { .. } => NaturalVocabularyOperationFormV1::ProjectStatus,
        ResponseOperation::ComposeCollection { .. } => {
            NaturalVocabularyOperationFormV1::ComposeCollection
        }
        ResponseOperation::CopyAfterPrefix { .. } => {
            NaturalVocabularyOperationFormV1::CopyAfterPrefix
        }
        ResponseOperation::TestResultSummary { .. } => {
            NaturalVocabularyOperationFormV1::TestResultSummary
        }
        ResponseOperation::WaitOnYieldedCell { .. } => {
            NaturalVocabularyOperationFormV1::WaitOnYieldedCell
        }
        ResponseOperation::WaitOnAnyYieldedCell { .. } => {
            NaturalVocabularyOperationFormV1::WaitOnAnyYieldedCell
        }
        ResponseOperation::WaitOnYieldedSurfaces { .. } => {
            NaturalVocabularyOperationFormV1::WaitOnYieldedSurfaces
        }
    }
}

fn readiness_blocker_v1(settled: u64, verified: u64, lineages: u64) -> String {
    if settled < K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1 {
        "settled_evidence_below_freeze_minimum"
    } else if verified < K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1 {
        "verified_evidence_below_freeze_minimum"
    } else if lineages < K1_CANDIDATE_READINESS_MIN_LINEAGES_V1 {
        "independent_lineages_below_freeze_minimum"
    } else {
        ""
    }
    .to_owned()
}

fn consequence_type_v1(
    joined: &BlindThenRevealJoinedTransitionV1,
    effect: CompletedEffectFormV1,
) -> K1ConsequenceTypeV1 {
    match effect {
        CompletedEffectFormV1::StatusValueBranch => K1ConsequenceTypeV1::Boolean,
        CompletedEffectFormV1::CollectionTransform => K1ConsequenceTypeV1::Collection,
        CompletedEffectFormV1::MultiRoleRendering => K1ConsequenceTypeV1::RenderedSequence,
        CompletedEffectFormV1::CrossOutputComposition => K1ConsequenceTypeV1::Record,
        CompletedEffectFormV1::SingleRoleProjection => {
            if joined
                .topology
                .roles
                .iter()
                .any(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
            {
                K1ConsequenceTypeV1::Collection
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Boolean)
            {
                K1ConsequenceTypeV1::Boolean
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Object)
            {
                K1ConsequenceTypeV1::Record
            } else {
                K1ConsequenceTypeV1::Scalar
            }
        }
        CompletedEffectFormV1::Unexplained => K1ConsequenceTypeV1::Record,
    }
}

fn rank_ready_forms_v1(
    left: &NaturalVocabularyFormCensusV1,
    right: &NaturalVocabularyFormCensusV1,
) -> Ordering {
    left.bounded_discovery_cost_units
        .cmp(&right.bounded_discovery_cost_units)
        .then_with(|| {
            right
                .expected_verified_input_tokens
                .cmp(&left.expected_verified_input_tokens)
        })
        .then_with(|| left.form_root_sha256.cmp(&right.form_root_sha256))
}

fn archive_manifest_root_v1<T: Serialize>(
    schema: &'static str,
    rows: &[T],
) -> Result<String, &'static str> {
    let mut roots = rows
        .iter()
        .map(canonical_json_sha256)
        .collect::<Result<Vec<_>, _>>()?;
    roots.sort();
    canonical_json_sha256(&(schema, roots))
}

fn increment<K: Ord>(counts: &mut BTreeMap<K, u64>, key: K) {
    let value = counts.entry(key).or_default();
    *value = value.saturating_add(1);
}

fn merge_counts(target: &mut BTreeMap<String, u64>, source: &BTreeMap<String, u64>) {
    for (key, count) in source {
        let value = target.entry(key.clone()).or_default();
        *value = value.saturating_add(*count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_plan_is_a_distinct_missing_vocabulary_form() {
        let program = ResponseProgram::advance_plan("update_plan");
        assert_eq!(
            operation_form_v1(&program),
            NaturalVocabularyOperationFormV1::AdvancePlan
        );
        assert!(crate::synthesis::compile_independent_verifier(&program).is_ok());
        assert!(active_t1_protocol_mode_root_v1(&program).is_none());
    }

    #[test]
    fn protocol_vocabulary_is_measured_before_source_neutral_binding() {
        let program = ResponseProgram::function_call_from_roles(
            "exec",
            nando_operator_kernel::ResponseValueSelector::UniqueScalar {
                value_type: nando_operator_kernel::AtomValueType::String,
            },
            vec![nando_operator_kernel::ResponseArgument::Role {
                name: "input".to_owned(),
                role: nando_operator_kernel::SemanticRole::SourceValue,
                value_type: Some(nando_operator_kernel::AtomValueType::String),
            }],
        );
        program.validate().expect("valid physical program");
        assert!(active_t1_protocol_mode_root_v1(&program).is_some());
    }

    #[test]
    fn invalid_legacy_program_is_counted_without_aborting_the_census() {
        let program = ResponseProgram::function_call_from_roles(
            "exec",
            nando_operator_kernel::ResponseValueSelector::UniqueScalar {
                value_type: nando_operator_kernel::AtomValueType::String,
            },
            vec![
                nando_operator_kernel::ResponseArgument::Role {
                    name: "input".to_owned(),
                    role: nando_operator_kernel::SemanticRole::SourceValue,
                    value_type: Some(nando_operator_kernel::AtomValueType::String),
                },
                nando_operator_kernel::ResponseArgument::String {
                    name: "legacy".to_owned(),
                    value: "\0".to_owned(),
                },
            ],
        );
        let partition = partition_programs_v1(vec![program]);
        assert!(partition.programs.is_empty());
        assert_eq!(
            partition
                .blockers
                .get(&NaturalVocabularyOperationFormV1::FunctionCallFromRoles)
                .and_then(|counts| counts.get("invalid_string_argument")),
            Some(&1)
        );
    }

    #[test]
    fn empty_census_is_deterministic_and_has_no_authority() {
        let first = build_natural_vocabulary_census_v1(&[], &[]).expect("empty census");
        let second = build_natural_vocabulary_census_v1(&[], &[]).expect("empty census");
        assert_eq!(first, second);
        assert_eq!(first.verdict, NaturalVocabularyCensusVerdictV1::NoReadyForm);
        assert!(!first.authority_ready);
        assert!(!first.phase_mutation_allowed);
        first.validate().expect("valid census");
    }

    #[test]
    fn readiness_thresholds_match_the_frozen_k1_contract() {
        assert_eq!(
            readiness_blocker_v1(7, 2, 2),
            "settled_evidence_below_freeze_minimum"
        );
        assert_eq!(
            readiness_blocker_v1(8, 1, 2),
            "verified_evidence_below_freeze_minimum"
        );
        assert_eq!(
            readiness_blocker_v1(8, 2, 1),
            "independent_lineages_below_freeze_minimum"
        );
        assert_eq!(readiness_blocker_v1(8, 2, 2), "");
    }

    #[test]
    fn operation_form_root_is_stable() {
        let first = canonical_json_sha256(&(
            "nando.natural-vocabulary-operation-form.v1",
            NaturalVocabularyOperationFormV1::AdvancePlan,
        ))
        .expect("form root");
        let second = canonical_json_sha256(&(
            "nando.natural-vocabulary-operation-form.v1",
            NaturalVocabularyOperationFormV1::AdvancePlan,
        ))
        .expect("form root");
        assert_eq!(first, second);
        assert!(valid_nonzero_sha256(&first));
    }
}
