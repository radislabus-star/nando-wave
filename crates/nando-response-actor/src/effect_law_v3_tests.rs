use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::*;
use crate::{
    AtomSource, AtomValueType, CaptureEvidenceReceipt, CaptureRecordCommitment,
    CollectionOutputRenderer, CustomToolResultProjection, DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1,
    DurableRuntimeParityReceipt, ProjectStatusMapping, ResponseArgument, ResponseExecutionStatus,
    ResponseValueSelector, RuntimeFrame, RuntimeParityCase, SemanticRole, TeacherActionAst,
    TeacherOutcome, TeacherVerifierEvidence, ValueProjectionFormat, canonical_json_bytes,
    canonical_json_sha256, execute_response, response_actor_program_digest,
    response_independent_verifier_program_digest, sha256_bytes,
    source_neutral_verifier_for_program, teacher_program_signature_from_action_atoms,
    valid_nonzero_sha256, verify_response_independently_with_request,
};

#[derive(Clone)]
enum FixtureSurface {
    Function(String),
    CustomTool { outer: String, inner: String },
}

#[derive(Clone)]
pub(super) struct FixtureSpec {
    seed: u64,
    episode: String,
    surface: FixtureSurface,
    actor_surface: FixtureSurface,
    selector_field: String,
    role_argument: String,
    chars_argument: String,
    terminate_argument: String,
    yield_argument: String,
    output_field: String,
    continuation_field: String,
    continuation_prefix: String,
    left_role: String,
    right_role: String,
    completion: String,
    observed_completion: Option<String>,
    response_shape: String,
    output_status: String,
    pub(super) renderer: CollectionOutputRenderer,
    pub(super) status_mapping: ProjectStatusMapping,
    pub(super) temporal_predecessor: String,
    pub(super) temporal_successor: String,
    cardinality_role: String,
    cardinality: u32,
    chars: String,
    pub(super) terminate: bool,
    pub(super) yield_time_ms: u64,
    pub(super) extra_preserved_role: bool,
    symmetric_roles: bool,
    reverse_atoms: bool,
}

impl FixtureSpec {
    pub(super) fn direct(seed: u64) -> Self {
        let surface = FixtureSurface::Function("wait".to_owned());
        Self {
            seed,
            episode: digest(&format!("episode-{seed}")),
            surface: surface.clone(),
            actor_surface: surface,
            selector_field: "handle".to_owned(),
            role_argument: "handle".to_owned(),
            chars_argument: "chars".to_owned(),
            terminate_argument: "terminate".to_owned(),
            yield_argument: "yield_time_ms".to_owned(),
            output_field: "output".to_owned(),
            continuation_field: "continuation".to_owned(),
            continuation_prefix: "next=".to_owned(),
            left_role: "active_execution".to_owned(),
            right_role: "continued_execution".to_owned(),
            completion: "pending".to_owned(),
            observed_completion: None,
            response_shape: "function_call".to_owned(),
            output_status: "success".to_owned(),
            renderer: CollectionOutputRenderer::Direct,
            status_mapping: ProjectStatusMapping::ZeroIsSuccess,
            temporal_predecessor: "active_execution".to_owned(),
            temporal_successor: "continued_execution".to_owned(),
            cardinality_role: "continuation".to_owned(),
            cardinality: 1,
            chars: String::new(),
            terminate: false,
            yield_time_ms: 1_000,
            extra_preserved_role: false,
            symmetric_roles: false,
            reverse_atoms: false,
        }
    }

    pub(super) fn wrapped(seed: u64) -> Self {
        let surface = FixtureSurface::CustomTool {
            outer: "exec".to_owned(),
            inner: "write_stdin".to_owned(),
        };
        Self {
            surface: surface.clone(),
            actor_surface: surface,
            selector_field: "job_ref".to_owned(),
            role_argument: "session_id".to_owned(),
            chars_argument: "input_text".to_owned(),
            terminate_argument: "stop".to_owned(),
            yield_argument: "budget_ms".to_owned(),
            output_field: "body".to_owned(),
            continuation_field: "cursor".to_owned(),
            continuation_prefix: "resume=".to_owned(),
            left_role: "running_job".to_owned(),
            right_role: "resumed_job".to_owned(),
            temporal_predecessor: "running_job".to_owned(),
            temporal_successor: "resumed_job".to_owned(),
            cardinality_role: "resume_target".to_owned(),
            ..Self::direct(seed)
        }
    }
}

#[derive(Clone)]
struct Fixture {
    transition: TeacherTransition,
    capture_record: CaptureRecordCommitment,
    capture_receipt_root_sha256: String,
    parity_receipt: DurableRuntimeParityReceipt,
    observed_state: IndependentEffectStateV3,
    program: ResponseProgram,
    verifier: VerifierProgram,
}

pub(super) struct SealedFixtureSet {
    pub(super) transitions: Vec<TeacherTransition>,
    pub(super) observations: Vec<SealedEffectObservationV3>,
    pub(super) trusted: TrustedEffectEvidenceSetV3,
}

fn digest(value: &str) -> String {
    sha256_bytes(value.as_bytes())
}

fn raw_json_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("fixture serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn fixture_program(spec: &FixtureSpec) -> ResponseProgram {
    let selector = ResponseValueSelector::JsonField {
        field: spec.selector_field.clone(),
        value_type: AtomValueType::Identifier,
    };
    let arguments = vec![
        ResponseArgument::Role {
            name: spec.role_argument.clone(),
            role: SemanticRole::ContinuationHandle,
            value_type: Some(AtomValueType::Identifier),
        },
        ResponseArgument::String {
            name: spec.chars_argument.clone(),
            value: spec.chars.clone(),
        },
        ResponseArgument::Boolean {
            name: spec.terminate_argument.clone(),
            value: spec.terminate,
        },
        ResponseArgument::Integer {
            name: spec.yield_argument.clone(),
            value: spec.yield_time_ms,
        },
    ];
    match &spec.actor_surface {
        FixtureSurface::Function(function_name) => {
            ResponseProgram::function_call_from_roles(function_name, selector, arguments)
        }
        FixtureSurface::CustomTool { outer, inner } => {
            ResponseProgram::custom_tool_call_from_roles(
                outer,
                inner,
                selector,
                arguments,
                CustomToolResultProjection::OutputAndContinuation {
                    output_field: spec.output_field.clone(),
                    continuation_field: spec.continuation_field.clone(),
                    continuation_prefix: spec.continuation_prefix.clone(),
                },
            )
        }
    }
}

fn fixture(spec: &FixtureSpec) -> Fixture {
    let handle_value = format!("handle-{}", spec.seed);
    let handle_value_sha256 = digest(&handle_value);
    let mut before_atoms = vec![
        RelationAtom::TypedSlot {
            slot_id: 1,
            value_type: AtomValueType::Identifier,
            source: AtomSource::Observation,
            value_sha256: handle_value_sha256.clone(),
        },
        RelationAtom::UniqueSlot { slot_id: 1 },
        RelationAtom::ObservationSelector {
            slot_id: 1,
            selector: ResponseValueSelector::JsonField {
                field: spec.selector_field.clone(),
                value_type: AtomValueType::Identifier,
            },
        },
        RelationAtom::RequestPhaseAtom { atom_id: 7 },
    ];
    let mut action_atoms = vec![
        RelationAtom::TypedSlot {
            slot_id: 7,
            value_type: AtomValueType::Identifier,
            source: AtomSource::Action,
            value_sha256: handle_value_sha256,
        },
        RelationAtom::SlotEquality {
            left_slot: 1,
            right_slot: 7,
        },
        RelationAtom::ActionRoleArgument {
            name: spec.role_argument.clone(),
            slot_id: 7,
            value_type: Some(AtomValueType::Identifier),
        },
        RelationAtom::ActionStringArgument {
            name: spec.chars_argument.clone(),
            value: spec.chars.clone(),
        },
        RelationAtom::ActionBooleanArgument {
            name: spec.terminate_argument.clone(),
            value: spec.terminate,
        },
        RelationAtom::ActionIntegerArgument {
            name: spec.yield_argument.clone(),
            value: spec.yield_time_ms,
        },
        RelationAtom::ActionValueProjection {
            format: ValueProjectionFormat::PlainText,
            renderer: spec.renderer.clone(),
        },
        RelationAtom::ActionStatusProjection {
            mapping: spec.status_mapping,
        },
        RelationAtom::CompletionState {
            value: spec.completion.clone(),
        },
        RelationAtom::ResponseShape {
            value: spec.response_shape.clone(),
        },
        RelationAtom::OutputStatus {
            value: spec.output_status.clone(),
        },
        RelationAtom::TypedEquality {
            left_role: spec.left_role.clone(),
            right_role: spec.right_role.clone(),
        },
        RelationAtom::TemporalEdge {
            predecessor: spec.temporal_predecessor.clone(),
            successor: spec.temporal_successor.clone(),
        },
        RelationAtom::Cardinality {
            role: spec.cardinality_role.clone(),
            count: spec.cardinality,
        },
    ];
    match &spec.surface {
        FixtureSurface::Function(value) => {
            action_atoms.push(RelationAtom::ActionFunction {
                value: value.clone(),
            });
        }
        FixtureSurface::CustomTool { outer, inner } => {
            action_atoms.extend([
                RelationAtom::ActionCustomTool {
                    value: outer.clone(),
                },
                RelationAtom::ActionInnerTool {
                    value: inner.clone(),
                },
                RelationAtom::ActionResultProjection {
                    output_field: spec.output_field.clone(),
                    continuation_field: spec.continuation_field.clone(),
                    continuation_prefix: spec.continuation_prefix.clone(),
                },
            ]);
        }
    }
    if spec.extra_preserved_role {
        let peer_value_sha256 = digest(&format!("peer-{}", spec.seed));
        let peer_type = if spec.symmetric_roles {
            AtomValueType::Identifier
        } else {
            AtomValueType::String
        };
        before_atoms.extend([
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: peer_type,
                source: AtomSource::Observation,
                value_sha256: peer_value_sha256.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 2 },
            RelationAtom::ObservationSelector {
                slot_id: 2,
                selector: ResponseValueSelector::UniqueScalar {
                    value_type: peer_type,
                },
            },
        ]);
        action_atoms.extend([
            RelationAtom::TypedSlot {
                slot_id: 8,
                value_type: peer_type,
                source: AtomSource::Action,
                value_sha256: peer_value_sha256,
            },
            RelationAtom::SlotEquality {
                left_slot: 2,
                right_slot: 8,
            },
            RelationAtom::ActionRoleArgument {
                name: "peer".to_owned(),
                slot_id: 8,
                value_type: Some(peer_type),
            },
        ]);
    }
    before_atoms.sort();
    action_atoms.sort();
    if spec.reverse_atoms {
        before_atoms.reverse();
        action_atoms.reverse();
    }

    let frame_id_sha256 = digest(&format!("frame-{}", spec.seed));
    let mut payload = Map::new();
    payload.insert(spec.selector_field.clone(), Value::String(handle_value));
    payload.insert("fixture".to_owned(), Value::from(spec.seed));
    let provider_payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": serde_json::to_string(&Value::Object(payload)).expect("embedded payload"),
        }],
    });
    let program = fixture_program(spec);
    let verifier = source_neutral_verifier_for_program(&program).expect("fixture verifier");
    let execution = execute_response(&program, "", &provider_payload);
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "actor reason: {}",
        execution.reason
    );
    let expected_response = execution.response.expect("fixture actor response");
    verify_response_independently_with_request(
        &verifier,
        "",
        &provider_payload,
        &expected_response,
    )
    .expect("fixture independent verifier");

    let capture_record = CaptureRecordCommitment {
        sequence: spec.seed,
        record_sha256: digest(&format!("capture-record-{}", spec.seed)),
    };
    let capture_receipt =
        CaptureEvidenceReceipt::new(vec![capture_record.clone()]).expect("capture receipt");
    let capture_receipt_root_sha256 = capture_receipt.records_root_sha256.clone();
    let parity_case = RuntimeParityCase {
        evidence_ref_sha256: frame_id_sha256.clone(),
        capture_receipt: Some(capture_receipt),
        request_text: String::new(),
        provider_payload: provider_payload.clone(),
        expected_response: expected_response.clone(),
    };
    let verifier_evidence_ref_sha256 = frame_id_sha256.clone();
    let verifier_output_digest_sha256 =
        raw_json_sha256(&(verifier_evidence_ref_sha256.as_str(), &action_atoms, true));
    let signature_sha256 =
        teacher_program_signature_from_action_atoms(&action_atoms).expect("teacher signature");
    let transition = TeacherTransition {
        schema: crate::TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before: RuntimeFrame {
            schema: crate::RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256: frame_id_sha256.clone(),
            event_id_sha256: digest(&format!("event-{}", spec.seed)),
            client_intent_id_sha256: digest(&format!("intent-{}", spec.seed)),
            session_id_sha256: spec.episode.clone(),
            observed_at_unix_nanos: spec.seed,
            extractor_version: "f2r4-trusted-fixture".to_owned(),
            atoms: before_atoms,
            evidence_ref_sha256: digest(&format!("provider-evidence-{}", spec.seed)),
        },
        outcome: TeacherOutcome {
            schema: crate::TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
            action: TeacherActionAst {
                signature_sha256,
                action_symbol: "trusted-fixture".to_owned(),
                atoms: action_atoms.clone(),
            },
            verifier: TeacherVerifierEvidence {
                accepted: true,
                evidence_ref_sha256: verifier_evidence_ref_sha256,
                output_digest_sha256: verifier_output_digest_sha256,
            },
            completed_at_unix_nanos: spec.seed + 1,
        },
        economics: None,
        runtime_parity_case: Some(parity_case),
    };
    let program_sha256 = response_actor_program_digest(&program).expect("program digest");
    let verifier_sha256 =
        response_independent_verifier_program_digest(&verifier).expect("verifier digest");
    let mut parity_receipt = DurableRuntimeParityReceipt {
        schema: DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        evidence_ref_sha256: frame_id_sha256.clone(),
        program_sha256,
        verifier_sha256,
        input_sha256: canonical_json_sha256(&provider_payload).expect("payload digest"),
        teacher_response_sha256: sha256_bytes(expected_response.as_bytes()),
        actor_response_sha256: sha256_bytes(expected_response.as_bytes()),
        actor_executed: true,
        teacher_authority_match: true,
        independent_verifier_pass: true,
        exact_teacher_match: true,
    };
    parity_receipt.seal_digest().expect("parity receipt seal");

    let mut observed_effect_atoms = action_atoms;
    if let Some(completion) = &spec.observed_completion {
        for atom in &mut observed_effect_atoms {
            if let RelationAtom::CompletionState { value } = atom {
                value.clone_from(completion);
            }
        }
    }
    observed_effect_atoms.sort();
    observed_effect_atoms.dedup();
    let mut observed_state = IndependentEffectStateV3 {
        schema: INDEPENDENT_EFFECT_STATE_SCHEMA_V3.to_owned(),
        evidence_ref_sha256: frame_id_sha256,
        before_atoms_root_sha256: evidence::sha256_serialized(&transition.before.atoms)
            .expect("before root"),
        actor_response_sha256: sha256_bytes(expected_response.as_bytes()),
        effect_atoms: observed_effect_atoms,
        observer_root_sha256: digest("independent-effect-state-observer-v3"),
        receipt_sha256: String::new(),
    };
    observed_state.receipt_sha256 =
        trust::independent_effect_state_digest(&observed_state).expect("state receipt");
    Fixture {
        transition,
        capture_record,
        capture_receipt_root_sha256,
        parity_receipt,
        observed_state,
        program,
        verifier,
    }
}

fn manifest_wire(fixtures: &[Fixture]) -> TrustedGenerationManifestWireV3 {
    let mut capture_records = fixtures
        .iter()
        .map(|item| item.capture_record.clone())
        .collect::<Vec<_>>();
    capture_records.sort_by_key(|item| item.sequence);
    let capture_index = CaptureCommitmentIndex::new(capture_records).expect("capture index");
    let mut parity_receipts = fixtures
        .iter()
        .map(|item| item.parity_receipt.clone())
        .collect::<Vec<_>>();
    parity_receipts.sort_by(|left, right| left.evidence_ref_sha256.cmp(&right.evidence_ref_sha256));
    let mut observed_states = fixtures
        .iter()
        .map(|item| item.observed_state.clone())
        .collect::<Vec<_>>();
    observed_states.sort_by(|left, right| left.evidence_ref_sha256.cmp(&right.evidence_ref_sha256));
    let mut entries = fixtures
        .iter()
        .map(|item| {
            let evidence_ref_sha256 = item.transition.before.frame_id_sha256.clone();
            TrustedGenerationEvidenceEntryV3 {
                evidence_ref_sha256,
                transition_sha256: evidence::sha256_serialized(&item.transition)
                    .expect("transition root"),
                episode_lineage_sha256: item.transition.before.session_id_sha256.clone(),
                surface_root_sha256: evidence::build_protocol_facet(&item.transition)
                    .expect("protocol facet")
                    .root_sha256,
                physical_program_id: item.parity_receipt.program_sha256.clone(),
                capture_receipt_root_sha256: item.capture_receipt_root_sha256.clone(),
                parity_receipt_root_sha256: item.parity_receipt.receipt_sha256.clone(),
                observed_state_root_sha256: item.observed_state.receipt_sha256.clone(),
            }
        })
        .collect::<Vec<_>>();
    entries.sort();
    let delta_verifier_material = entries
        .iter()
        .map(|entry| {
            let fixture = fixtures
                .iter()
                .find(|item| item.parity_receipt.evidence_ref_sha256 == entry.evidence_ref_sha256)
                .expect("fixture entry");
            (
                entry.evidence_ref_sha256.as_str(),
                fixture.parity_receipt.verifier_sha256.as_str(),
                fixture.observed_state.observer_root_sha256.as_str(),
            )
        })
        .collect::<Vec<_>>();
    TrustedGenerationManifestWireV3 {
        schema: TRUSTED_GENERATION_MANIFEST_SCHEMA_V3.to_owned(),
        generation_id_sha256: digest("f2r4-trusted-generation"),
        delta_verifier_root_sha256: evidence::sha256_serialized(&(
            VERIFIED_EFFECT_DELTA_RECEIPT_SCHEMA_V3,
            &delta_verifier_material,
        ))
        .expect("delta verifier root"),
        capture_index,
        parity_receipts,
        observed_states,
        entries,
    }
}

fn trusted_from_wire(
    wire: &TrustedGenerationManifestWireV3,
) -> Result<TrustedEffectEvidenceSetV3, EffectLawV3Error> {
    let bytes = canonical_json_bytes(wire).expect("manifest bytes");
    let external_root = sha256_bytes(&bytes);
    let pinned_root =
        trust::pin_trusted_generation_manifest_root(&external_root).expect("trusted owner pin");
    resolve_trusted_effect_evidence_set_v3(&bytes, &pinned_root)
}

pub(super) fn sealed_set(specs: &[FixtureSpec]) -> SealedFixtureSet {
    let fixtures = specs.iter().map(fixture).collect::<Vec<_>>();
    let trusted = trusted_from_wire(&manifest_wire(&fixtures)).expect("trusted evidence set");
    let observations = fixtures
        .iter()
        .map(|item| {
            let candidate =
                observe_effect_transition_v3(item.transition.clone()).expect("candidate");
            seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier)
                .expect("sealed observation")
        })
        .collect();
    SealedFixtureSet {
        transitions: fixtures
            .iter()
            .map(|item| item.transition.clone())
            .collect(),
        observations,
        trusted,
    }
}

fn quotient(
    left: FixtureSpec,
    right: FixtureSpec,
) -> Result<EffectLawQuotientReportV3, EffectLawV3Error> {
    let sealed = sealed_set(&[left, right]);
    search_effect_law_quotient_v3(
        &sealed.observations,
        &EffectLawDictionaryV3::builtin().expect("dictionary"),
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
}

fn assert_no_common_law(left: FixtureSpec, right: FixtureSpec) {
    let report = quotient(left, right).expect("bounded quotient report");
    assert!(report.candidate.is_none());
    assert_eq!(report.blocker.as_deref(), Some("no_invariant_effect_delta"));
}

fn recompute_restart_bundle_digests(bundle: &mut EffectLawRestartBundleV3) {
    bundle
        .proofs
        .sort_by(|left, right| left.observation_sha256.cmp(&right.observation_sha256));
    bundle.proof_set_root_sha256 =
        evidence::sha256_serialized(&bundle.proofs).expect("recomputed proof root");
    bundle.bundle_sha256 = evidence::sha256_serialized(&(
        EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3,
        &bundle.law,
        &bundle.proofs,
        bundle.proof_set_root_sha256.as_str(),
    ))
    .expect("recomputed bundle root");
}

#[test]
fn recomputed_fake_index_and_receipts_fail_the_external_manifest_root() {
    let original = fixture(&FixtureSpec::direct(1));
    let original_wire = manifest_wire(std::slice::from_ref(&original));
    let original_bytes = canonical_json_bytes(&original_wire).expect("original manifest");
    let external_root = sha256_bytes(&original_bytes);
    let pinned_root =
        trust::pin_trusted_generation_manifest_root(&external_root).expect("external root");

    let mut forged = original.clone();
    forged.capture_record.record_sha256 = digest("forged-capture-record");
    forged.parity_receipt.input_sha256 = digest("forged-input");
    forged
        .parity_receipt
        .seal_digest()
        .expect("fully recomputed parity receipt");
    let forged_wire = manifest_wire(&[forged]);
    let forged_bytes = canonical_json_bytes(&forged_wire).expect("forged manifest");
    assert!(matches!(
        resolve_trusted_effect_evidence_set_v3(&forged_bytes, &pinned_root),
        Err(EffectLawV3Error::InvalidTrustRoot)
    ));
}

#[test]
fn missing_capture_receipt_cannot_be_sealed() {
    let mut item = fixture(&FixtureSpec::direct(2));
    item.transition
        .runtime_parity_case
        .as_mut()
        .expect("parity case")
        .capture_receipt = None;
    let trusted =
        trusted_from_wire(&manifest_wire(std::slice::from_ref(&item))).expect("trusted evidence");
    let candidate = observe_effect_transition_v3(item.transition).expect("candidate");
    assert_eq!(
        seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier),
        Err(EffectLawV3Error::InvalidCaptureReceipt)
    );
}

#[test]
fn capture_receipt_missing_from_trusted_index_is_rejected() {
    let item = fixture(&FixtureSpec::direct(3));
    let mut wire = manifest_wire(std::slice::from_ref(&item));
    wire.capture_index = CaptureCommitmentIndex::new(Vec::new()).expect("empty index");
    let trusted = trusted_from_wire(&wire).expect("trusted but empty capture index");
    let candidate = observe_effect_transition_v3(item.transition).expect("candidate");
    assert_eq!(
        seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier),
        Err(EffectLawV3Error::InvalidCaptureReceipt)
    );
}

#[test]
fn recomputed_candidate_checksum_cannot_bypass_trusted_transition_membership() {
    let item = fixture(&FixtureSpec::direct(4));
    let trusted =
        trusted_from_wire(&manifest_wire(std::slice::from_ref(&item))).expect("trusted evidence");
    let mut forged_transition = item.transition;
    forged_transition.before.event_id_sha256 = digest("different-event");
    let candidate = observe_effect_transition_v3(forged_transition).expect("recomputed candidate");
    assert_eq!(
        seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier),
        Err(EffectLawV3Error::InvalidTrustRoot)
    );
}

#[test]
fn verifier_observed_postcondition_disagreement_rejects_teacher_claim() {
    let mut spec = FixtureSpec::direct(5);
    spec.observed_completion = Some("terminated".to_owned());
    let item = fixture(&spec);
    let trusted =
        trusted_from_wire(&manifest_wire(std::slice::from_ref(&item))).expect("trusted evidence");
    let candidate = observe_effect_transition_v3(item.transition.clone()).expect("candidate");
    assert_eq!(
        seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier),
        Err(EffectLawV3Error::EffectDeltaDisagreement)
    );
}

#[test]
fn receipt_is_built_through_real_actor_and_independent_verifier_route() {
    let item = fixture(&FixtureSpec::wrapped(6));
    let trusted =
        trusted_from_wire(&manifest_wire(std::slice::from_ref(&item))).expect("trusted evidence");
    let candidate = observe_effect_transition_v3(item.transition.clone()).expect("candidate");
    let observation =
        seal_effect_observation_v3(candidate, &trusted, &item.program, &item.verifier)
            .expect("real actor and verifier receipt");
    assert_eq!(
        observation.physical_program_id,
        item.parity_receipt.program_sha256
    );
    assert_eq!(
        observation.verifier_root_sha256,
        item.parity_receipt.verifier_sha256
    );
    assert!(valid_nonzero_sha256(
        &observation.verified_delta_receipt_root_sha256
    ));
}

#[test]
fn renamed_arguments_fields_selectors_and_role_labels_preserve_law_id() {
    let report = quotient(FixtureSpec::direct(7), FixtureSpec::wrapped(8)).expect("quotient");
    let candidate = report.candidate.expect("source-neutral law");
    assert!(valid_nonzero_sha256(
        candidate.law().effect_law_id().expect("law id").as_str()
    ));
    assert_eq!(report.independence.surface_roots, 2);
    assert_eq!(report.independence.physical_program_ids, 2);
    assert!(
        candidate
            .law()
            .relation_program
            .iter()
            .all(|clause| clause.argument_ordinal.is_none_or(|ordinal| ordinal < 16))
    );
}

#[test]
fn wait_and_terminate_do_not_share_a_law() {
    let direct = FixtureSpec::direct(10);
    let mut terminate = FixtureSpec::wrapped(11);
    terminate.completion = "terminated".to_owned();
    terminate.terminate = true;
    assert_no_common_law(direct, terminate);
}

#[test]
fn completion_state_is_effect_significant() {
    let direct = FixtureSpec::direct(12);
    let mut wrapped = FixtureSpec::wrapped(13);
    wrapped.completion = "completed".to_owned();
    assert_no_common_law(direct, wrapped);
}

#[test]
fn response_shape_is_effect_significant() {
    let direct = FixtureSpec::direct(14);
    let mut wrapped = FixtureSpec::wrapped(15);
    wrapped.response_shape = "assistant_message".to_owned();
    assert_no_common_law(direct, wrapped);
}

#[test]
fn output_status_is_effect_significant() {
    let direct = FixtureSpec::direct(16);
    let mut wrapped = FixtureSpec::wrapped(17);
    wrapped.output_status = "failure".to_owned();
    assert_no_common_law(direct, wrapped);
}

#[test]
fn renderer_contract_is_effect_significant() {
    let direct = FixtureSpec::direct(18);
    let mut wrapped = FixtureSpec::wrapped(19);
    wrapped.renderer = CollectionOutputRenderer::RenderTemplate {
        prefix: "[".to_owned(),
        suffix: "]".to_owned(),
    };
    assert_no_common_law(direct, wrapped);
}

#[test]
fn status_mapping_is_effect_significant() {
    let direct = FixtureSpec::direct(20);
    let mut wrapped = FixtureSpec::wrapped(21);
    wrapped.status_mapping = ProjectStatusMapping::ZeroIsPass;
    assert_no_common_law(direct, wrapped);
}

#[test]
fn temporal_contract_is_effect_significant() {
    let direct = FixtureSpec::direct(22);
    let mut wrapped = FixtureSpec::wrapped(23);
    wrapped.temporal_successor = wrapped.temporal_predecessor.clone();
    assert_no_common_law(direct, wrapped);
}

#[test]
fn cardinality_contract_is_effect_significant() {
    let direct = FixtureSpec::direct(24);
    let mut wrapped = FixtureSpec::wrapped(25);
    wrapped.cardinality = 2;
    assert_no_common_law(direct, wrapped);
}

#[test]
fn changed_preserved_frame_contract_does_not_merge() {
    let direct = FixtureSpec::direct(26);
    let mut wrapped = FixtureSpec::wrapped(27);
    wrapped.extra_preserved_role = true;
    assert_no_common_law(direct, wrapped);
}

#[test]
fn atom_order_does_not_change_law_identity() {
    let direct = FixtureSpec::direct(28);
    let mut wrapped = FixtureSpec::wrapped(29);
    wrapped.reverse_atoms = true;
    let report = quotient(direct, wrapped).expect("quotient");
    assert!(report.candidate.is_some());
}

#[test]
fn direct_and_wrapped_transports_can_share_one_effect_law() {
    let report = quotient(FixtureSpec::direct(30), FixtureSpec::wrapped(31)).expect("quotient");
    assert!(report.candidate.is_some());
    assert_eq!(report.independence.observations, 2);
    assert_eq!(report.independence.episode_lineages, 2);
    assert_eq!(report.independence.surface_roots, 2);
    assert_eq!(report.independence.physical_program_ids, 2);
}

#[test]
fn same_episode_is_not_independent() {
    let direct = FixtureSpec::direct(32);
    let mut wrapped = FixtureSpec::wrapped(33);
    wrapped.episode.clone_from(&direct.episode);
    assert_eq!(
        quotient(direct, wrapped),
        Err(EffectLawV3Error::InsufficientIndependentEvidence)
    );
}

#[test]
fn same_surface_is_not_independent() {
    let direct = FixtureSpec::direct(34);
    let same_surface = FixtureSpec::direct(35);
    assert_eq!(
        quotient(direct, same_surface),
        Err(EffectLawV3Error::InsufficientIndependentEvidence)
    );
}

#[test]
fn same_physical_program_is_not_independent() {
    let direct = FixtureSpec::direct(36);
    let mut wrapped = FixtureSpec::wrapped(37);
    wrapped.actor_surface = direct.actor_surface.clone();
    wrapped.selector_field.clone_from(&direct.selector_field);
    wrapped.role_argument.clone_from(&direct.role_argument);
    wrapped.chars_argument.clone_from(&direct.chars_argument);
    wrapped
        .terminate_argument
        .clone_from(&direct.terminate_argument);
    wrapped.yield_argument.clone_from(&direct.yield_argument);
    assert_eq!(
        quotient(direct, wrapped),
        Err(EffectLawV3Error::InsufficientIndependentEvidence)
    );
}

#[test]
fn symmetric_mappings_with_distinct_action_classes_abstain() {
    let mut direct = FixtureSpec::direct(38);
    direct.extra_preserved_role = true;
    direct.symmetric_roles = true;
    let mut wrapped = FixtureSpec::wrapped(39);
    wrapped.extra_preserved_role = true;
    wrapped.symmetric_roles = true;
    assert_eq!(
        quotient(direct, wrapped),
        Err(EffectLawV3Error::AmbiguousActionEquivalence)
    );
}

#[test]
fn dictionary_meaning_schema_and_version_commit_law_identity() {
    let sealed = sealed_set(&[FixtureSpec::direct(40), FixtureSpec::wrapped(41)]);
    let builtin = EffectLawDictionaryV3::builtin().expect("builtin dictionary");
    let baseline = search_effect_law_quotient_v3(
        &sealed.observations,
        &builtin,
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
    .expect("baseline quotient")
    .candidate
    .expect("baseline law");
    for variant in 0..3 {
        let mut entries = builtin.entries.clone();
        match variant {
            0 => entries[0].meaning_sha256 = digest("changed meaning"),
            1 => entries[0].operand_schema_sha256 = digest("changed operands"),
            _ => {
                for entry in &mut entries {
                    entry.version = 4;
                }
            }
        }
        let version = if variant == 2 {
            4
        } else {
            EFFECT_LAW_IR_VERSION_V3
        };
        let dictionary = EffectLawDictionaryV3::new(version, entries).expect("variant dictionary");
        let candidate = search_effect_law_quotient_v3(
            &sealed.observations,
            &dictionary,
            &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
        )
        .expect("variant quotient")
        .candidate
        .expect("variant law");
        assert_ne!(
            baseline.law().effect_law_id().expect("baseline id"),
            candidate.law().effect_law_id().expect("variant id")
        );
    }
}

#[test]
fn restart_revalidates_trust_and_three_dimensional_independence() {
    let sealed = sealed_set(&[FixtureSpec::direct(42), FixtureSpec::wrapped(43)]);
    let report = search_effect_law_quotient_v3(
        &sealed.observations,
        &EffectLawDictionaryV3::builtin().expect("dictionary"),
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
    .expect("quotient");
    let candidate = report.candidate.expect("candidate");
    let bytes = candidate.restart_bundle().canonical_bytes().expect("bytes");
    let bundle_root =
        trust::pin_trusted_effect_law_bundle_root(candidate.restart_bundle(), &sealed.trusted)
            .expect("externally pinned bundle root");
    let restored =
        EffectLawRestartBundleV3::from_canonical_bytes(&bytes, &sealed.trusted, &bundle_root)
            .expect("trusted restart");
    assert_eq!(&restored, candidate.restart_bundle());
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
    assert_eq!(restored.law(), candidate.law());
    assert_eq!(restored.proofs().len(), 2);
    assert_eq!(
        trust::validate_restart_proofs(restored.proofs(), &sealed.trusted)
            .expect("independence")
            .physical_program_ids,
        2
    );
}

#[test]
fn restart_rejects_a_different_external_manifest_root() {
    let sealed = sealed_set(&[FixtureSpec::direct(44), FixtureSpec::wrapped(45)]);
    let report = search_effect_law_quotient_v3(
        &sealed.observations,
        &EffectLawDictionaryV3::builtin().expect("dictionary"),
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
    .expect("quotient");
    let candidate = report.candidate.expect("candidate");
    let bytes = candidate.restart_bundle().canonical_bytes().expect("bytes");
    let bundle_root =
        trust::pin_trusted_effect_law_bundle_root(candidate.restart_bundle(), &sealed.trusted)
            .expect("externally pinned bundle root");
    let other = sealed_set(&[FixtureSpec::direct(46), FixtureSpec::wrapped(47)]);
    assert!(matches!(
        EffectLawRestartBundleV3::from_canonical_bytes(&bytes, &other.trusted, &bundle_root),
        Err(EffectLawV3Error::InvalidTrustRoot | EffectLawV3Error::InvalidRestartBundle)
    ));
}

#[test]
fn fully_recomputed_restart_forgery_is_rejected_by_external_bundle_root() {
    let sealed = sealed_set(&[FixtureSpec::direct(48), FixtureSpec::wrapped(49)]);
    let report = search_effect_law_quotient_v3(
        &sealed.observations,
        &EffectLawDictionaryV3::builtin().expect("dictionary"),
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
    .expect("quotient");
    let original = report.candidate.expect("candidate").restart_bundle;
    let bundle_root = trust::pin_trusted_effect_law_bundle_root(&original, &sealed.trusted)
        .expect("externally pinned bundle root");

    let mut mapping_tamper = original.clone();
    let first = mapping_tamper.proofs[0].node_mapping[0].canonical_node;
    let second = mapping_tamper.proofs[0].node_mapping[1].canonical_node;
    mapping_tamper.proofs[0].node_mapping[0].canonical_node = second;
    mapping_tamper.proofs[0].node_mapping[1].canonical_node = first;
    recompute_restart_bundle_digests(&mut mapping_tamper);
    let bytes = mapping_tamper.canonical_bytes().expect("tampered bytes");
    assert_eq!(
        EffectLawRestartBundleV3::from_canonical_bytes(&bytes, &sealed.trusted, &bundle_root,),
        Err(EffectLawV3Error::InvalidTrustRoot)
    );

    let mut law_tamper = original.clone();
    law_tamper.law.effect_invariant_root_sha256 = digest("forged-effect-invariant");
    law_tamper.law.action_equivalence_root_sha256 = evidence::sha256_serialized(&(
        &law_tamper.law.relation_program,
        &law_tamper.law.effect_invariant_root_sha256,
        &law_tamper.law.preserved_frame_root_sha256,
    ))
    .expect("recomputed action equivalence root");
    recompute_restart_bundle_digests(&mut law_tamper);
    let bytes = law_tamper.canonical_bytes().expect("forged law bytes");
    assert_eq!(
        EffectLawRestartBundleV3::from_canonical_bytes(&bytes, &sealed.trusted, &bundle_root,),
        Err(EffectLawV3Error::InvalidTrustRoot)
    );

    let mut delta_tamper = original;
    delta_tamper.proofs[0].exact_delta_root_sha256 = digest("forged-exact-delta");
    delta_tamper.proofs[0].verified_delta_receipt_root_sha256 =
        digest("forged-verified-delta-receipt");
    recompute_restart_bundle_digests(&mut delta_tamper);
    let bytes = delta_tamper.canonical_bytes().expect("forged delta bytes");
    assert_eq!(
        EffectLawRestartBundleV3::from_canonical_bytes(&bytes, &sealed.trusted, &bundle_root,),
        Err(EffectLawV3Error::InvalidTrustRoot)
    );
}

#[test]
fn empty_false_and_integer_constants_remain_effect_significant() {
    let cases = [
        {
            let mut item = FixtureSpec::wrapped(51);
            item.chars = "input".to_owned();
            item
        },
        {
            let mut item = FixtureSpec::wrapped(52);
            item.terminate = true;
            item
        },
        {
            let mut item = FixtureSpec::wrapped(53);
            item.yield_time_ms = 30_000;
            item
        },
    ];
    for (offset, changed) in cases.into_iter().enumerate() {
        assert_no_common_law(FixtureSpec::direct(60 + offset as u64), changed);
    }
}

#[test]
fn unknown_dictionary_opcode_is_data_not_an_executable_clause() {
    const UNKNOWN: u16 = 0x7f01;
    let builtin = EffectLawDictionaryV3::builtin().expect("builtin dictionary");
    assert!(builtin.entries.iter().all(|entry| entry.code != UNKNOWN));
    let mut entries = builtin.entries.clone();
    entries.push(EffectDictionaryEntryV3 {
        code: UNKNOWN,
        meaning_sha256: digest("unknown relation"),
        operand_schema_sha256: digest("opaque operands"),
        version: EFFECT_LAW_IR_VERSION_V3,
    });
    let dictionary = EffectLawDictionaryV3::new(EFFECT_LAW_IR_VERSION_V3, entries)
        .expect("unknown dictionary entry remains bounded data");
    let sealed = sealed_set(&[FixtureSpec::direct(70), FixtureSpec::wrapped(71)]);
    let candidate = search_effect_law_quotient_v3(
        &sealed.observations,
        &dictionary,
        &EffectQuotientHypothesisV3::physical_adapters_only().expect("hypothesis"),
    )
    .expect("quotient")
    .candidate
    .expect("law");
    assert!(
        candidate
            .law()
            .relation_program
            .iter()
            .all(|clause| clause.relation_code != UNKNOWN)
    );
}
