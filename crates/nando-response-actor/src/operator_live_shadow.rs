use std::collections::BTreeMap;

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl,
    BoundedCircuitBeam, BoundedRoleAligner, FrozenOperatorBlueprintSet, LocalRelationFragment,
    OPERATOR_ROLE_NONE, PhaseCenterCell, RoleAlignmentConfig, StructuralRoleSignature,
    SurfaceFragmentBundle, TernaryRelationState, TypedProgramAtom, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, CollectionOutputRenderer, CollectionSynthesisExample,
    CrystallizationParityReceipt, CrystallizedOperator, ResponseOperation, ResponseProgram,
    ResponseValueSelector, RuntimeRoleAnchor, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, TeacherTransition, ValueProjectionFormat,
    enumerate_source_neutral_response_programs, relation_frame_online_routing_atom_ids,
    response_program_exactly_matches_example,
};

const LIVE_SCALAR_ROLE_COUNT: usize = 3;
const ROLE_CONTEXT: u8 = 0;
const ROLE_SOURCE: u8 = 1;
const ROLE_OUTPUT: u8 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveScalarCircuitSample {
    pub bundle: SurfaceFragmentBundle,
    pub anchor: RuntimeRoleAnchor,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
    pub raw_input_sha256: [u8; 32],
    pub extractor_version: u32,
    pub law_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveScalarShadowBlocker {
    TeacherRejected,
    MissingParityCase,
    PayloadTooLarge,
    NoExactSourceNeutralProgram,
    UnsupportedScalarProgram,
    InvalidCommitment,
    InvalidBundle,
    SupportSessionReused,
    FutureSessionReused,
    FutureCapacityReached,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LiveScalarShadowState {
    observations: usize,
    executable: usize,
    duplicate_rows: usize,
    blockers: BTreeMap<LiveScalarShadowBlocker, usize>,
    laws: BTreeMap<String, LiveScalarLawState>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LiveScalarLawState {
    support: Vec<TeacherTransition>,
    future: Vec<TeacherTransition>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarShadowReport {
    pub observations: usize,
    pub executable: usize,
    pub duplicate_rows: usize,
    pub law_count: usize,
    pub support_rows: usize,
    pub future_rows: usize,
    pub frozen_laws: usize,
    pub full_phase_winners: usize,
    pub causal_control_passes: usize,
    pub verified_shadow_operators: usize,
    pub shadow_executions: usize,
    pub ingest_accounting_complete: bool,
    pub blockers: BTreeMap<String, usize>,
}

impl LiveScalarShadowState {
    pub fn observe(&mut self, transition: &TeacherTransition) {
        self.observations = self.observations.saturating_add(1);
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
        };
        self.executable = self.executable.saturating_add(1);
        let law_key = commitment_hex(&sample.law_sha256);
        let law = self.laws.entry(law_key).or_default();
        if law
            .support
            .iter()
            .chain(&law.future)
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if law.support.len() < 3 {
            if law
                .support
                .iter()
                .any(|row| row.before.session_id_sha256 == transition.before.session_id_sha256)
            {
                *self
                    .blockers
                    .entry(LiveScalarShadowBlocker::SupportSessionReused)
                    .or_default() += 1;
                return;
            }
            law.support.push(transition.clone());
            return;
        }
        let support_sessions = law
            .support
            .iter()
            .map(|row| row.before.session_id_sha256.as_str());
        if support_sessions
            .chain(
                law.future
                    .iter()
                    .map(|row| row.before.session_id_sha256.as_str()),
            )
            .any(|session| session == transition.before.session_id_sha256)
        {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::FutureSessionReused)
                .or_default() += 1;
            return;
        }
        if law.future.len() < 32 {
            law.future.push(transition.clone());
        } else {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::FutureCapacityReached)
                .or_default() += 1;
        }
    }

    #[must_use]
    pub fn report(&self) -> LiveScalarShadowReport {
        let support_rows = self.laws.values().map(|law| law.support.len()).sum();
        let future_rows = self.laws.values().map(|law| law.future.len()).sum();
        let ingest_blocker_rows = self.blockers.values().copied().sum::<usize>();
        let mut report = LiveScalarShadowReport {
            observations: self.observations,
            executable: self.executable,
            duplicate_rows: self.duplicate_rows,
            law_count: self.laws.len(),
            support_rows,
            future_rows,
            ingest_accounting_complete: self.observations
                == support_rows
                    .saturating_add(future_rows)
                    .saturating_add(self.duplicate_rows)
                    .saturating_add(ingest_blocker_rows),
            blockers: self
                .blockers
                .iter()
                .map(|(blocker, count)| (format!("{blocker:?}").to_lowercase(), *count))
                .collect(),
            ..LiveScalarShadowReport::default()
        };
        for law in self.laws.values() {
            evaluate_live_law(law, &mut report);
        }
        report
    }
}

fn evaluate_live_law(law: &LiveScalarLawState, report: &mut LiveScalarShadowReport) {
    if law.support.len() < 3 {
        increment_report_blocker(report, "support_below_3");
        return;
    }
    let Ok(support) = law
        .support
        .iter()
        .map(extract_live_scalar_circuit_sample)
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "support_reextract_failed");
        return;
    };
    let support_bundles = support
        .iter()
        .map(|sample| sample.bundle.clone())
        .collect::<Vec<_>>();
    let alignments = BoundedRoleAligner::align(&support_bundles, RoleAlignmentConfig::default());
    if !alignments.completion.is_complete() {
        increment_report_blocker(report, "role_alignment_exhausted");
        return;
    }
    let synthesis = BoundedCircuitBeam::synthesize(
        &support_bundles,
        &alignments,
        BlueprintBeamConfig::default(),
    );
    if !synthesis.completion.is_complete() {
        increment_report_blocker(report, "circuit_synthesis_exhausted");
        return;
    }
    let Ok(frozen) =
        FrozenOperatorBlueprintSet::freeze(1, &support_bundles, Default::default(), &synthesis)
    else {
        increment_report_blocker(report, "blueprint_freeze_failed");
        return;
    };
    report.frozen_laws = report.frozen_laws.saturating_add(1);
    if law.future.len() < 3 {
        increment_report_blocker(report, "future_below_3");
        return;
    }
    let Ok(future) = law
        .future
        .iter()
        .map(extract_live_scalar_circuit_sample)
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "future_reextract_failed");
        return;
    };
    let future_evidence = future
        .iter()
        .map(|sample| {
            BlueprintFutureEvidence::new(
                sample.raw_input_sha256,
                sample.extractor_version.max(1),
                sample.bundle.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>();
    let Ok(future_evidence) = future_evidence else {
        increment_report_blocker(report, "future_evidence_invalid");
        return;
    };
    let full = BlueprintFutureEvaluator::evaluate_and_seal(
        &frozen,
        &future_evidence,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let Some(winner) = full.winner_receipt() else {
        increment_report_blocker(report, "full_phase_no_winner");
        return;
    };
    report.full_phase_winners = report.full_phase_winners.saturating_add(1);
    let controls_pass = [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ]
    .into_iter()
    .all(|control| {
        BlueprintFutureEvaluator::evaluate_and_seal(
            &frozen,
            &future_evidence,
            Default::default(),
            control,
        )
        .winner_receipt()
        .is_none()
    });
    if !controls_pass {
        increment_report_blocker(report, "phase_control_selected_winner");
        return;
    }
    report.causal_control_passes = report.causal_control_passes.saturating_add(1);
    let mut future_window = frozen.future_window();
    for sample in &future {
        if future_window.admit_lineage(&sample.bundle).is_err() {
            increment_report_blocker(report, "future_lineage_rejected");
            return;
        }
    }
    let receipts = future
        .iter()
        .zip(&future_evidence)
        .map(|(sample, evidence)| CrystallizationParityReceipt {
            future_lineage_sha256: *sample.bundle.lineage_sha256(),
            future_surface_sha256: *sample.bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256: sample.raw_input_sha256,
            extractor_version: sample.extractor_version.max(1),
            anchors: vec![sample.anchor.clone()].into_boxed_slice(),
            request_text: sample.request_text.clone(),
            provider_payload: sample.provider_payload.clone(),
            expected_response: sample.expected_response.clone(),
        })
        .collect::<Vec<_>>();
    match CrystallizedOperator::crystallize(&future_window, winner, &future_evidence, &receipts) {
        Ok(_) => {
            report.verified_shadow_operators = report.verified_shadow_operators.saturating_add(1);
            report.shadow_executions = report.shadow_executions.saturating_add(receipts.len());
        }
        Err(error) => {
            increment_report_blocker(report, &format!("crystallization_{error:?}").to_lowercase())
        }
    }
}

fn increment_report_blocker(report: &mut LiveScalarShadowReport, blocker: &str) {
    *report.blockers.entry(blocker.to_owned()).or_default() += 1;
}

/// Converts one completed, verified trace into source-neutral circuit evidence.
/// The teacher response selects a hypothesis only after the action; runtime
/// binding remains restricted to the pre-action provider payload.
pub fn extract_live_scalar_circuit_sample(
    transition: &TeacherTransition,
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    if !transition.outcome.verifier.accepted {
        return Err(LiveScalarShadowBlocker::TeacherRejected);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(LiveScalarShadowBlocker::MissingParityCase)?;
    let payload_bytes = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::UnsupportedScalarProgram)?;
    if payload_bytes.len() > 64 * 1024 || parity.expected_response.len() > 4 * 1024 {
        return Err(LiveScalarShadowBlocker::PayloadTooLarge);
    }
    let example = CollectionSynthesisExample {
        provider_payload: parity.provider_payload.clone(),
        expected_response: parity.expected_response.clone(),
    };
    let version_space = enumerate_source_neutral_response_programs(&example)
        .map_err(|_| LiveScalarShadowBlocker::NoExactSourceNeutralProgram)?;
    let mut programs = version_space
        .programs
        .into_iter()
        .filter(|program| response_program_exactly_matches_example(program, &example))
        .filter_map(project_scalar_program)
        .collect::<Vec<_>>();
    programs.sort_by(|left, right| left.0.cmp(&right.0));
    let Some((_, selector, value_type, format)) = programs.into_iter().next() else {
        return Err(LiveScalarShadowBlocker::NoExactSourceNeutralProgram);
    };

    let frame = transition.before.as_routing_relation_frame();
    let routing_atoms = relation_frame_online_routing_atom_ids(&frame);
    let phases = relation_phases(&routing_atoms);
    let local_roles = local_role_permutation(&transition.before.frame_id_sha256);
    let context = local_roles[usize::from(ROLE_CONTEXT)];
    let source = local_roles[usize::from(ROLE_SOURCE)];
    let output = local_roles[usize::from(ROLE_OUTPUT)];
    let roles = (0..LIVE_SCALAR_ROLE_COUNT)
        .map(|local| {
            let semantic = local_roles
                .iter()
                .position(|candidate| usize::from(*candidate) == local)
                .expect("role permutation is complete") as u8;
            role_signature(semantic, local as u8, value_type)
        })
        .collect::<Vec<_>>();
    let relations = vec![
        relation(0, context, source, phases[0]),
        relation(1, source, output, phases[1]),
        relation(2, context, output, phases[2]),
    ];
    let transform = TypedProgramAtom {
        opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
        output_local_role: output,
        source_a_local_role: source,
        source_b_local_role: OPERATOR_ROLE_NONE,
        parameter: transform_parameter(value_type),
        flags: if format == ValueProjectionFormat::CanonicalJson {
            TRANSFORM_FLAG_CANONICAL_JSON
        } else {
            0
        },
    };
    let lineage_sha256 = parse_commitment(&transition.before.session_id_sha256)
        .or_else(|| parse_commitment(&transition.before.client_intent_id_sha256))
        .ok_or(LiveScalarShadowBlocker::InvalidCommitment)?;
    let surface_sha256 = digest_parts(
        b"nando.live-scalar-surface.v1",
        &[
            transition.before.frame_id_sha256.as_bytes(),
            transition.before.extractor_version.as_bytes(),
        ],
    );
    let bundle = SurfaceFragmentBundle::new(
        lineage_sha256,
        surface_sha256,
        roles,
        relations,
        vec![transform],
    )
    .map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
    let raw_input_sha256 = digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[parity.request_text.as_bytes(), &payload_bytes],
    );
    let law_sha256 = digest_parts(
        b"nando.live-scalar-law.v1",
        &[
            &[value_type_tag(value_type)],
            &[u8::from(format == ValueProjectionFormat::CanonicalJson)],
        ],
    );
    Ok(LiveScalarCircuitSample {
        bundle,
        anchor: RuntimeRoleAnchor {
            local_role: source,
            selector,
        },
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response: parity.expected_response.clone(),
        raw_input_sha256,
        extractor_version: extractor_version(&transition.before.extractor_version),
        law_sha256,
    })
}

fn project_scalar_program(
    program: ResponseProgram,
) -> Option<(
    Vec<u8>,
    ResponseValueSelector,
    AtomValueType,
    ValueProjectionFormat,
)> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer: CollectionOutputRenderer::Direct,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let value_type = selector_value_type(selector);
    let bytes = serde_json::to_vec(&program).ok()?;
    Some((bytes, selector.clone(), value_type, *format))
}

fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
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

fn role_signature(
    semantic_role: u8,
    _local_role: u8,
    value_type: AtomValueType,
) -> StructuralRoleSignature {
    match semantic_role {
        ROLE_CONTEXT => StructuralRoleSignature::new(5, 2, 0, 1, vec![0, 2]),
        ROLE_SOURCE => {
            StructuralRoleSignature::new(value_type_tag(value_type), 1, 1, 2, vec![0, 1])
        }
        ROLE_OUTPUT => {
            StructuralRoleSignature::new(value_type_tag(value_type), 1, 2, 4, vec![1, 2])
        }
        _ => unreachable!("three fixed semantic roles"),
    }
}

fn relation(
    plane: u8,
    source_local_role: u8,
    target_local_role: u8,
    phase_anchor: PhaseCenterCell,
) -> LocalRelationFragment {
    LocalRelationFragment {
        plane,
        source_local_role,
        target_local_role,
        state: TernaryRelationState::Supported,
        phase_anchor,
    }
}

fn relation_phases(atom_ids: &[u64]) -> [PhaseCenterCell; 3] {
    let mut phases = [PhaseCenterCell { re: 1.0, im: 0.0 }; 3];
    for (plane, phase) in phases.iter_mut().enumerate() {
        let mut plane_atoms = atom_ids
            .iter()
            .map(|atom| format!("{atom:016x}"))
            .collect::<Vec<_>>();
        plane_atoms.push(format!("live_scalar_plane:{plane}"));
        let encoded = phase_vector_from_atoms(plane_atoms.iter().map(String::as_str), 3);
        *phase = encoded[plane];
        if phase.re.hypot(phase.im) <= f64::EPSILON {
            *phase = PhaseCenterCell { re: 1.0, im: 0.0 };
        }
    }
    phases
}

fn local_role_permutation(frame_id: &str) -> [u8; LIVE_SCALAR_ROLE_COUNT] {
    let digest = Sha256::digest(frame_id.as_bytes());
    let mut roles = [0_u8, 1, 2];
    for index in (1..roles.len()).rev() {
        roles.swap(index, usize::from(digest[index]) % (index + 1));
    }
    roles
}

const fn transform_parameter(value_type: AtomValueType) -> u16 {
    match value_type {
        AtomValueType::String => TRANSFORM_VALUE_STRING,
        AtomValueType::Integer => TRANSFORM_VALUE_INTEGER,
        AtomValueType::Boolean => TRANSFORM_VALUE_BOOLEAN,
        AtomValueType::Identifier => TRANSFORM_VALUE_IDENTIFIER,
        AtomValueType::Collection => TRANSFORM_VALUE_STRING,
    }
}

const fn value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
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

fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn extractor_version(value: &str) -> u32 {
    let digest = Sha256::digest(value.as_bytes());
    u32::from_le_bytes(digest[..4].try_into().expect("fixed digest width"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RUNTIME_FRAME_SCHEMA_V1, RuntimeFrame,
        TEACHER_OUTCOME_SCHEMA_V1, TEACHER_TRANSITION_SCHEMA_V1, TeacherActionAst, TeacherOutcome,
        TeacherVerifierEvidence,
    };

    fn transition(field: &str, accepted: bool) -> TeacherTransition {
        let hash = |byte: char| byte.to_string().repeat(64);
        TeacherTransition {
            schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
            before: RuntimeFrame {
                schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
                frame_id_sha256: hash('1'),
                event_id_sha256: hash('2'),
                client_intent_id_sha256: hash('3'),
                session_id_sha256: hash('4'),
                observed_at_unix_nanos: 1,
                extractor_version: "live-test-v1".to_owned(),
                atoms: Vec::new(),
                evidence_ref_sha256: hash('5'),
            },
            outcome: TeacherOutcome {
                schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
                action: TeacherActionAst {
                    signature_sha256: hash('6'),
                    action_symbol: "response".to_owned(),
                    atoms: Vec::new(),
                },
                verifier: TeacherVerifierEvidence {
                    accepted,
                    evidence_ref_sha256: hash('7'),
                    output_digest_sha256: hash('8'),
                },
                completed_at_unix_nanos: 2,
            },
            economics: Some(EconomicsReceipt {
                schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                exact_input_tokens: 100,
                ordinary: true,
                controlled: false,
                replay: false,
                dedupe_eligible: true,
                provider_evidence_ref_sha256: hash('9'),
            }),
            runtime_parity_case: Some(crate::RuntimeParityCase {
                evidence_ref_sha256: hash('a'),
                request_text: "Return the count".to_owned(),
                provider_payload: json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": format!("{{\"{field}\":7}}")
                    }]
                }),
                expected_response: "7".to_owned(),
            }),
        }
    }

    #[test]
    fn verified_scalar_trace_becomes_source_neutral_circuit_evidence() {
        let first =
            extract_live_scalar_circuit_sample(&transition("total", true)).expect("scalar trace");
        let renamed = extract_live_scalar_circuit_sample(&transition("renamed_total", true))
            .expect("renamed scalar trace");

        assert_eq!(first.bundle.roles().len(), 3);
        assert_eq!(first.bundle.relations().len(), 3);
        assert_eq!(first.bundle.program_atoms().len(), 1);
        assert_eq!(first.law_sha256, renamed.law_sha256);
        assert_ne!(first.anchor.selector, renamed.anchor.selector);
    }

    #[test]
    fn rejected_teacher_cannot_create_wave_evidence() {
        assert_eq!(
            extract_live_scalar_circuit_sample(&transition("total", false)),
            Err(LiveScalarShadowBlocker::TeacherRejected)
        );
    }

    #[test]
    fn independent_live_rows_reach_verified_scalar_shadow_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..6_u8 {
            let mut row = transition(&format!("field_{index}"), true);
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            row.runtime_parity_case
                .as_mut()
                .expect("parity")
                .provider_payload = json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("{{\"field_{index}\":{}}}", index + 7)
                }]
            });
            row.runtime_parity_case
                .as_mut()
                .expect("parity")
                .expected_response = (index + 7).to_string();
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.executable, 6, "{report:#?}");
        assert_eq!(report.support_rows, 3, "{report:#?}");
        assert_eq!(report.future_rows, 3, "{report:#?}");
        assert_eq!(report.frozen_laws, 1, "{report:#?}");
        assert!(report.ingest_accounting_complete, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 3, "{report:#?}");
    }
}
