use std::collections::BTreeMap;

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl,
    BoundedCircuitBeam, BoundedRoleAligner, FrozenOperatorBlueprintSet, LocalRelationFragment,
    OPERATOR_ROLE_NONE, RoleAlignmentConfig, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TypedProgramAtom, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, CollectionOutputRenderer, CollectionSynthesisExample,
    CrystallizationParityReceipt, CrystallizedOperator, ResponseOperation, ResponsePackage,
    ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState, ResponseProgram,
    ResponseRenderSegment, ResponseValueSelector, RuntimeRoleAnchor, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, TeacherTransition, ValueProjectionFormat,
    enumerate_source_neutral_response_programs, execute_response,
    is_privacy_safe_online_response_program, is_source_neutral_response_program,
};

const LIVE_SCALAR_SUPPORT_ROWS: usize = 32;
const LIVE_SCALAR_FUTURE_ROWS: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveScalarCircuitSample {
    pub bundle: SurfaceFragmentBundle,
    pub anchors: Box<[RuntimeRoleAnchor]>,
    pub actor_template: ResponseProgram,
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
    ExactStatusProgram,
    ExactCollectionProgram,
    UnsupportedRendererProgram,
    UnsupportedScalarProgram,
    InvalidCommitment,
    InvalidBundle,
    FutureSessionReused,
    FutureCapacityReached,
    HistoricalSupportCapacityReached,
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
    pub admission_candidates: usize,
    pub ingest_accounting_complete: bool,
    pub blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarAdmissionCandidate {
    pub package: ResponsePackage,
    pub support: Vec<TeacherTransition>,
    pub future: Vec<TeacherTransition>,
    pub support_root_sha256: String,
    pub future_evidence_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub winner_seal_sha256: String,
    pub executable_parity_seal_sha256: String,
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
        if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS {
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
        if law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
            law.future.push(transition.clone());
        } else {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::FutureCapacityReached)
                .or_default() += 1;
        }
    }

    /// Rebuilds bounded support after a strategy upgrade without reclassifying
    /// historical receipts as post-freeze future evidence.
    pub(crate) fn observe_historical_support(&mut self, transition: &TeacherTransition) {
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
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if law.support.len() >= LIVE_SCALAR_SUPPORT_ROWS {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::HistoricalSupportCapacityReached)
                .or_default() += 1;
            return;
        }
        law.support.push(transition.clone());
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
        let mut candidates = Vec::new();
        for law in self.laws.values() {
            evaluate_live_law(law, &mut report, &mut candidates);
        }
        report.admission_candidates = candidates.len();
        report
    }

    #[must_use]
    pub fn admission_candidates(&self) -> Vec<LiveScalarAdmissionCandidate> {
        let mut report = LiveScalarShadowReport::default();
        let mut candidates = Vec::new();
        for law in self.laws.values() {
            evaluate_live_law(law, &mut report, &mut candidates);
        }
        candidates
    }
}

fn evaluate_live_law(
    law: &LiveScalarLawState,
    report: &mut LiveScalarShadowReport,
    candidates: &mut Vec<LiveScalarAdmissionCandidate>,
) {
    if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS {
        increment_report_blocker(report, "support_below_32");
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
        .fold(BTreeMap::new(), |mut sessions, sample| {
            sessions
                .entry(*sample.bundle.lineage_sha256())
                .or_insert_with(|| sample.bundle.clone());
            sessions
        })
        .into_values()
        .take(3)
        .collect::<Vec<_>>();
    if support_bundles.len() < 3 {
        increment_report_blocker(report, "support_sessions_below_3");
        return;
    }
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
    if synthesis.blueprints.is_empty() {
        for blocker in &synthesis.blockers {
            increment_report_blocker(
                report,
                &format!("circuit_synthesis_{:?}", blocker.blocker).to_lowercase(),
            );
        }
        return;
    }
    let frozen = match FrozenOperatorBlueprintSet::freeze(
        1,
        &support_bundles,
        Default::default(),
        &synthesis,
    ) {
        Ok(frozen) => frozen,
        Err(error) => {
            increment_report_blocker(
                report,
                &format!("blueprint_freeze_{error:?}").to_lowercase(),
            );
            return;
        }
    };
    report.frozen_laws = report.frozen_laws.saturating_add(1);
    if law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
        increment_report_blocker(report, "future_below_32");
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
            anchors: sample.anchors.clone(),
            request_text: sample.request_text.clone(),
            provider_payload: sample.provider_payload.clone(),
            expected_response: sample.expected_response.clone(),
        })
        .collect::<Vec<_>>();
    match CrystallizedOperator::crystallize(&future_window, winner, &future_evidence, &receipts) {
        Ok(operator) => {
            report.verified_shadow_operators = report.verified_shadow_operators.saturating_add(1);
            report.shadow_executions = report.shadow_executions.saturating_add(receipts.len());
            match live_admission_candidate(law, &operator) {
                Ok(candidate) => candidates.push(candidate),
                Err(blocker) => increment_report_blocker(report, &blocker),
            }
        }
        Err(error) => {
            increment_report_blocker(report, &format!("crystallization_{error:?}").to_lowercase())
        }
    }
}

fn live_admission_candidate(
    law: &LiveScalarLawState,
    operator: &crate::VerifiedCrystallizedOperator,
) -> Result<LiveScalarAdmissionCandidate, String> {
    if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS || law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
        return Err("admission_rows_below_32".to_owned());
    }
    let program = operator
        .routing_program()
        .map_err(|_| "admission_routing_program_failed".to_owned())?;
    let verifier = operator
        .routing_verifier()
        .map_err(|_| "admission_routing_verifier_failed".to_owned())?;
    let distinct_sessions = law
        .support
        .iter()
        .chain(&law.future)
        .map(|row| row.before.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_surfaces = law
        .support
        .iter()
        .chain(&law.future)
        .map(|row| row.before.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let package_id = format!(
        "crystallized-scalar-{}",
        &commitment_hex(operator.blueprint_sha256())[..16]
    );
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id,
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program,
        verifier: Some(verifier),
        routing_predicates: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        // The legacy vector is retained for package ABI validation. Runtime
        // authority comes from the restored circuit binding below, not from a
        // generic response-program atom masquerading as learned evidence.
        phase_centers: vec![operator.relation_program().fingerprint64()],
        anti_centers: Vec::new(),
        wave_margin_micro: 1,
        learned_wave_route: None,
        crystallized_operator: Some(
            operator
                .restart_bundle()
                .map_err(|_| "admission_restart_bundle_failed".to_owned())?,
        ),
        proof: ResponsePackageProof {
            support_rows: law.support.len(),
            future_rows: law.future.len(),
            distinct_sessions,
            distinct_surfaces,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: crate::VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
        },
    };
    package
        .validate()
        .map_err(|error| format!("admission_package_{error}"))?;
    Ok(LiveScalarAdmissionCandidate {
        package,
        support: law.support.clone(),
        future: law.future.clone(),
        support_root_sha256: commitment_hex(operator.support_root_sha256()),
        future_evidence_root_sha256: commitment_hex(operator.future_evidence_root_sha256()),
        future_lineage_root_sha256: commitment_hex(operator.future_lineage_root_sha256()),
        winner_seal_sha256: commitment_hex(operator.winner_seal_sha256()),
        executable_parity_seal_sha256: commitment_hex(operator.parity_seal().seal_sha256()),
    })
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
    if version_space.programs.is_empty() {
        return Err(LiveScalarShadowBlocker::NoExactSourceNeutralProgram);
    }
    let mut scalar_programs = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_scalar_program(
                program,
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
        })
        .filter_map(project_scalar_program)
        .collect::<Vec<_>>();
    scalar_programs.sort_by(|left, right| left.0.cmp(&right.0));
    let rich_exact = version_space
        .programs
        .iter()
        .find(|program| rich_scalar_program_roles(program).is_some_and(|roles| roles.len() > 1));
    let actor_template = if let Some(program) = rich_exact {
        program.clone()
    } else if let Some((_, selector, _, _, renderer)) = scalar_programs.first() {
        ResponseProgram::project_selected_value(selector.clone(), scalar_programs[0].3, "completed")
            .with_value_renderer(renderer.clone())
    } else {
        version_space
            .programs
            .iter()
            .find(|program| rich_scalar_program_roles(program).is_some())
            .cloned()
            .ok_or_else(|| classify_exact_program_blocker(&version_space.programs))?
    };
    let roles = rich_scalar_program_roles(&actor_template)
        .ok_or_else(|| classify_exact_program_blocker(&version_space.programs))?;

    let lineage_sha256 = parse_commitment(&transition.before.session_id_sha256)
        .or_else(|| parse_commitment(&transition.before.client_intent_id_sha256))
        .ok_or(LiveScalarShadowBlocker::InvalidCommitment)?;
    let surface_sha256 = digest_parts(
        b"nando.live-scalar-surface.v2",
        &[
            transition.before.frame_id_sha256.as_bytes(),
            transition.before.extractor_version.as_bytes(),
            &payload_bytes,
        ],
    );
    let observed = observed_rich_scalar_surface(
        &parity.request_text,
        &parity.provider_payload,
        &roles,
        &transition.before.frame_id_sha256,
        lineage_sha256,
        surface_sha256,
    )?;
    let raw_input_sha256 = digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[parity.request_text.as_bytes(), &payload_bytes],
    );
    let canonical_program = canonicalize_scalar_program_roles(&actor_template)
        .ok_or(LiveScalarShadowBlocker::UnsupportedRendererProgram)?;
    let law_shape = scalar_law_shape(&actor_template)
        .ok_or(LiveScalarShadowBlocker::UnsupportedRendererProgram)?;
    let law_sha256 = digest_parts(b"nando.live-scalar-law.v3", &[&law_shape]);
    Ok(LiveScalarCircuitSample {
        bundle: observed.bundle,
        anchors: observed.anchors,
        actor_template: canonical_program,
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response: parity.expected_response.clone(),
        raw_input_sha256,
        extractor_version: extractor_version(&transition.before.extractor_version),
        law_sha256,
    })
}

fn rich_scalar_program_roles(
    program: &ResponseProgram,
) -> Option<Vec<(ResponseValueSelector, ValueProjectionFormat)>> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut roles = vec![(selector.clone(), *format)];
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, format } = segment {
                roles.push((selector.clone(), *format));
            }
        }
    }
    (roles.len() <= 16).then_some(roles)
}

fn canonicalize_scalar_program_roles(program: &ResponseProgram) -> Option<ResponseProgram> {
    let mut canonical = program.clone();
    let ResponseOperation::ProjectSelectedValue {
        selector,
        renderer,
        completion_state,
        ..
    } = &mut canonical.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    *selector = canonical_role_selector(selector_value_type(selector));
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, .. } = segment {
                *selector = canonical_role_selector(selector_value_type(selector));
            }
        }
    }
    Some(canonical)
}

fn scalar_law_shape(program: &ResponseProgram) -> Option<Vec<u8>> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut shape = vec![
        1,
        value_type_tag(selector_value_type(selector)),
        u8::from(*format == ValueProjectionFormat::CanonicalJson),
    ];
    match renderer {
        CollectionOutputRenderer::Direct => shape.push(0),
        CollectionOutputRenderer::RenderTemplate { .. } => shape.push(1),
        CollectionOutputRenderer::RenderSequence { segments } => {
            shape.push(2);
            shape.extend_from_slice(&(segments.len() as u16).to_le_bytes());
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { .. } => shape.push(0),
                    ResponseRenderSegment::Primary => shape.push(1),
                    ResponseRenderSegment::Selected { selector, format } => {
                        shape.extend_from_slice(&[
                            2,
                            value_type_tag(selector_value_type(selector)),
                            u8::from(*format == ValueProjectionFormat::CanonicalJson),
                        ]);
                    }
                }
            }
        }
        CollectionOutputRenderer::RequestTemplate { marker } => {
            shape.extend_from_slice(&[3, *marker as u8]);
        }
    }
    Some(shape)
}

fn canonical_role_selector(value_type: AtomValueType) -> ResponseValueSelector {
    ResponseValueSelector::UniqueScalar { value_type }
}

fn observed_rich_scalar_surface(
    request_text: &str,
    provider_payload: &Value,
    program_roles: &[(ResponseValueSelector, ValueProjectionFormat)],
    frame_id: &str,
    lineage_sha256: [u8; 32],
    surface_sha256: [u8; 32],
) -> Result<crate::RuntimeSurfaceEvidence, LiveScalarShadowBlocker> {
    let role_count = program_roles.len().saturating_add(2);
    if !(3..=18).contains(&role_count) {
        return Err(LiveScalarShadowBlocker::InvalidBundle);
    }
    let semantic_to_local = local_role_permutation_n(frame_id, role_count);
    let context = semantic_to_local[0];
    let output = semantic_to_local[role_count - 1];
    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); role_count];
    let planes = (0..program_roles.len())
        .map(|index| u8::try_from(index).unwrap_or(u8::MAX))
        .collect::<Vec<_>>();
    roles[usize::from(context)] = StructuralRoleSignature::new(5, 1, 0, 1, planes.clone());
    roles[usize::from(output)] = StructuralRoleSignature::new(
        value_type_tag(selector_value_type(&program_roles[0].0)),
        1,
        2,
        4,
        Vec::new(),
    );
    let mut relations = Vec::with_capacity(program_roles.len());
    let mut atoms = Vec::with_capacity(program_roles.len());
    let mut anchors = Vec::with_capacity(program_roles.len());
    for (index, (selector, format)) in program_roles.iter().enumerate() {
        let source = semantic_to_local[index + 1];
        let plane = u8::try_from(index).map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
        let value_type = selector_value_type(selector);
        roles[usize::from(source)] =
            StructuralRoleSignature::new(value_type_tag(value_type), 1, 1, 2, vec![plane]);
        let phase_atom = format!("scalar_role:{index}:type:{}", value_type_tag(value_type));
        let phase = phase_vector_from_atoms([phase_atom.as_str()], 1)[0];
        relations.push(LocalRelationFragment {
            plane,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        });
        atoms.push(TypedProgramAtom {
            opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            output_local_role: output,
            source_a_local_role: source,
            source_b_local_role: OPERATOR_ROLE_NONE,
            parameter: transform_parameter(value_type)
                | (u16::try_from(index).unwrap_or(u16::MAX) << 8),
            flags: if *format == ValueProjectionFormat::CanonicalJson {
                TRANSFORM_FLAG_CANONICAL_JSON
            } else {
                0
            },
        });
        anchors.push(RuntimeRoleAnchor {
            local_role: source,
            selector: selector.clone(),
        });
    }
    let bundle =
        SurfaceFragmentBundle::new(lineage_sha256, surface_sha256, roles, relations, atoms)
            .map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
    Ok(crate::RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: anchors.into_boxed_slice(),
    })
}

fn project_scalar_program(
    program: ResponseProgram,
) -> Option<(
    Vec<u8>,
    ResponseValueSelector,
    AtomValueType,
    ValueProjectionFormat,
    CollectionOutputRenderer,
)> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let renderer = normalized_scalar_renderer(renderer)?;
    let value_type = selector_value_type(selector);
    let bytes = serde_json::to_vec(&program).ok()?;
    Some((bytes, selector.clone(), value_type, *format, renderer))
}

fn derive_exact_scalar_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let direct = ResponseProgram::project_selected_value(
        selector.clone(),
        *format,
        completion_state.clone(),
    );
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_value_renderer(renderer);
    if derived.validate().is_err()
        || !is_privacy_safe_online_response_program(&derived)
        || execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            != Some(expected_response)
    {
        return None;
    }
    Some(derived)
}

fn infer_scalar_renderer(computed: &str, expected: &str) -> Option<CollectionOutputRenderer> {
    if computed == expected {
        return Some(CollectionOutputRenderer::Direct);
    }
    if computed.is_empty() {
        return None;
    }
    let mut matches = expected.match_indices(computed);
    let (offset, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(CollectionOutputRenderer::RenderTemplate {
        prefix: expected[..offset].to_owned(),
        suffix: expected[offset + computed.len()..].to_owned(),
    })
}

fn normalized_scalar_renderer(
    renderer: &CollectionOutputRenderer,
) -> Option<CollectionOutputRenderer> {
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RenderTemplate { .. } => {
            Some(renderer.clone())
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            let mut prefix = String::new();
            let mut suffix = String::new();
            let mut primary_seen = false;
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { text } if primary_seen => {
                        suffix.push_str(text);
                    }
                    ResponseRenderSegment::Static { text } => prefix.push_str(text),
                    ResponseRenderSegment::Primary if !primary_seen => primary_seen = true,
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => {
                        return None;
                    }
                }
            }
            primary_seen.then_some(CollectionOutputRenderer::RenderTemplate { prefix, suffix })
        }
        CollectionOutputRenderer::RequestTemplate { .. } => None,
    }
}

fn classify_exact_program_blocker(programs: &[ResponseProgram]) -> LiveScalarShadowBlocker {
    if programs
        .iter()
        .any(|program| matches!(&program.operation, ResponseOperation::ProjectStatus { .. }))
    {
        LiveScalarShadowBlocker::ExactStatusProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ComposeCollection { .. }
        )
    }) {
        LiveScalarShadowBlocker::ExactCollectionProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ProjectSelectedValue { .. }
        )
    }) {
        LiveScalarShadowBlocker::UnsupportedRendererProgram
    } else {
        LiveScalarShadowBlocker::UnsupportedScalarProgram
    }
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

fn local_role_permutation_n(frame_id: &str, role_count: usize) -> Vec<u8> {
    let digest = Sha256::digest(frame_id.as_bytes());
    let mut roles = (0..role_count)
        .map(|role| u8::try_from(role).expect("bounded live role count"))
        .collect::<Vec<_>>();
    for index in (1..roles.len()).rev() {
        roles.swap(
            index,
            usize::from(digest[index % digest.len()]) % (index + 1),
        );
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

    fn multi_value_transition(
        first_field: &str,
        second_field: &str,
        expected_response: &str,
    ) -> TeacherTransition {
        let mut row = transition(first_field, true);
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = format!("Return {first_field} and {second_field}");
        parity.provider_payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("{{\"{first_field}\":7,\"{second_field}\":2}}")
            }]
        });
        parity.expected_response = expected_response.to_owned();
        row
    }

    #[test]
    fn verified_scalar_trace_becomes_source_neutral_circuit_evidence() {
        let first =
            extract_live_scalar_circuit_sample(&transition("total", true)).expect("scalar trace");
        let renamed = extract_live_scalar_circuit_sample(&transition("renamed_total", true))
            .expect("renamed scalar trace");

        assert_eq!(first.bundle.roles().len(), 3);
        // Only the pre-action context/source relation is observed. The output
        // role belongs to the learned transform and must not leak backward as
        // a fabricated support relation.
        assert_eq!(first.bundle.relations().len(), 1);
        assert_eq!(first.bundle.program_atoms().len(), 1);
        assert_eq!(first.law_sha256, renamed.law_sha256);
        assert_eq!(first.anchors, renamed.anchors);
    }

    #[test]
    fn rejected_teacher_cannot_create_wave_evidence() {
        assert_eq!(
            extract_live_scalar_circuit_sample(&transition("total", false)),
            Err(LiveScalarShadowBlocker::TeacherRejected)
        );
    }

    #[test]
    fn multi_value_surfaces_share_one_structural_law() {
        let first = extract_live_scalar_circuit_sample(&multi_value_transition(
            "total",
            "failed",
            "Total: 7; failed: 2",
        ))
        .expect("first multi-value trace");
        let renamed = extract_live_scalar_circuit_sample(&multi_value_transition(
            "records",
            "errors",
            "Records: 7; errors: 2",
        ))
        .expect("renamed multi-value trace");

        assert_eq!(first.anchors.len(), 2);
        assert_eq!(first.bundle.roles().len(), 4);
        assert_eq!(first.bundle.relations().len(), 2);
        assert_eq!(first.bundle.program_atoms().len(), 2);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn historical_rebuild_never_creates_frozen_future() {
        let mut state = LiveScalarShadowState::default();
        for index in 1_u64..=40 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{index:064x}");
            row.before.session_id_sha256 = format!("{:064x}", index + 100);
            state.observe_historical_support(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, LIVE_SCALAR_SUPPORT_ROWS);
        assert_eq!(report.future_rows, 0);
        assert_eq!(
            report
                .blockers
                .get("historicalsupportcapacityreached")
                .copied(),
            Some(8)
        );
    }

    #[test]
    fn repeated_support_session_fills_rows_but_cannot_freeze() {
        let mut state = LiveScalarShadowState::default();
        for index in 1_u64..=LIVE_SCALAR_SUPPORT_ROWS as u64 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{index:064x}");
            state.observe_historical_support(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, LIVE_SCALAR_SUPPORT_ROWS);
        assert_eq!(report.future_rows, 0);
        assert_eq!(report.blockers.get("support_sessions_below_3"), Some(&1));
    }

    #[test]
    fn single_primary_sequence_normalizes_to_typed_template() {
        let renderer = CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Static {
                    text: "Total records: ".to_owned(),
                },
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Static {
                    text: ".".to_owned(),
                },
            ],
        };
        assert_eq!(
            normalized_scalar_renderer(&renderer),
            Some(CollectionOutputRenderer::RenderTemplate {
                prefix: "Total records: ".to_owned(),
                suffix: ".".to_owned(),
            })
        );
    }

    #[test]
    fn templated_live_rows_reach_verified_scalar_shadow_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
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
                .expected_response = format!("Total records: {}.", index + 7);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.executable, 64, "{report:#?}");
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.frozen_laws, 1, "{report:#?}");
        assert!(report.ingest_accounting_complete, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        assert_eq!(candidates.len(), 1, "{report:#?}");
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates sealed operator")
        .expect("sealed operator reaches registry");
        assert_eq!(snapshot.registry.packages.len(), 1);
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores crystallized operator");
        let execution = executor.execute_shadow(
            "",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_surface_total\":91}"
                }]
            }),
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Total records: 91."),
            "{execution:#?}"
        );
        let ambiguous = executor.execute_shadow(
            "",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"left\":91,\"right\":92}"
                }]
            }),
        );
        assert_eq!(ambiguous.status, crate::ResponseExecutionStatus::Abstain);
        let incompatible = executor.execute_shadow(
            "",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_surface_total\":true}"
                }]
            }),
        );
        assert_eq!(incompatible.status, crate::ResponseExecutionStatus::Abstain);

        let mut tampered_support = candidates.clone();
        tampered_support[0].support[0]
            .runtime_parity_case
            .as_mut()
            .expect("support parity")
            .expected_response = "999".to_owned();
        assert!(matches!(
            crate::build_crystallized_admission_snapshot(
                &tampered_support,
                "test-project",
                2,
                100,
                30,
                &"a".repeat(64),
                &"b".repeat(64),
            ),
            Err("crystallized_admission_resynthesis_failed")
        ));

        let mut tampered_seal = candidates;
        tampered_seal[0].executable_parity_seal_sha256 = "c".repeat(64);
        assert!(matches!(
            crate::build_crystallized_admission_snapshot(
                &tampered_seal,
                "test-project",
                3,
                100,
                30,
                &"a".repeat(64),
                &"b".repeat(64),
            ),
            Err("crystallized_admission_resynthesis_mismatch")
        ));
    }
}
