use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::*;
use crate::{
    AtomSource, EffectGraphBuilder, EffectGraphCompleteness, RelationAtom, ResponseExecutionStatus,
    canonical_json_bytes, canonical_json_sha256, execute_response, response_actor_program_digest,
    response_independent_verifier_program_digest, sha256_bytes,
    teacher_program_signature_from_action_atoms, valid_nonzero_sha256,
    verify_response_independently_with_request,
};

const PHASE_BEFORE: u16 = 1;
const PHASE_ACTION: u16 = 2;
const NODE_SCALAR: u16 = 1;
const NODE_COLLECTION: u16 = 2;
const NODE_OPERATION: u16 = 3;

#[derive(Clone, Eq, PartialEq)]
struct SlotMaterial {
    source: AtomSource,
    value_type: AtomValueType,
    value_sha256: String,
    unique: bool,
}

pub(super) fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, EffectLawV3Error> {
    canonical_json_bytes(value).map_err(|_| EffectLawV3Error::Serialization)
}

pub(super) fn sha256_serialized<T: Serialize>(value: &T) -> Result<String, EffectLawV3Error> {
    canonical_json_sha256(value).map_err(|_| EffectLawV3Error::Serialization)
}

fn dictionary_entry(code: u16, meaning: &str, operand_schema: &str) -> EffectDictionaryEntryV3 {
    EffectDictionaryEntryV3 {
        code,
        meaning_sha256: sha256_bytes(meaning.as_bytes()),
        operand_schema_sha256: sha256_bytes(operand_schema.as_bytes()),
        version: EFFECT_LAW_IR_VERSION_V3,
    }
}

pub(super) fn builtin_dictionary() -> Result<EffectLawDictionaryV3, EffectLawV3Error> {
    build_dictionary(
        EFFECT_LAW_IR_VERSION_V3,
        vec![
            dictionary_entry(EFFECT_REL_EQUAL, "symmetric role equality", "role,role"),
            dictionary_entry(EFFECT_REL_COPY, "preserved value copy", "source,action"),
            dictionary_entry(
                EFFECT_REL_CONSUME,
                "operation consumes role",
                "role,operation",
            ),
            dictionary_entry(EFFECT_REL_REQUIRE, "role is required", "role"),
            dictionary_entry(
                EFFECT_REL_CONSTANT,
                "typed argument constant",
                "operation,constant",
            ),
            dictionary_entry(
                EFFECT_OPERATION_CALL_V3,
                "call operation",
                "roles,arguments",
            ),
            dictionary_entry(
                EFFECT_OPERATION_PROJECT_V3,
                "projection operation",
                "source,renderer",
            ),
            dictionary_entry(
                EFFECT_OPERATION_STATUS_V3,
                "status mapping operation",
                "status,mapping",
            ),
            dictionary_entry(
                EFFECT_OPERATION_PLAN_ADVANCE_V3,
                "plan advance operation",
                "plan_state",
            ),
            dictionary_entry(
                EFFECT_ATOM_PRECONDITION,
                "precondition evidence atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_ACTION_RELATION,
                "action relation atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_POSTCONDITION,
                "postcondition evidence atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_RENDERER,
                "renderer contract atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_TEMPORAL,
                "temporal contract atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_CARDINALITY,
                "cardinality contract atom",
                "relation_atom",
            ),
            dictionary_entry(
                EFFECT_ATOM_PHYSICAL_SURFACE,
                "physical adapter surface atom",
                "relation_atom",
            ),
            dictionary_entry(EFFECT_VALUE_STRING_V3, "string value", "utf8"),
            dictionary_entry(EFFECT_VALUE_INTEGER_V3, "unsigned integer value", "u64"),
            dictionary_entry(EFFECT_VALUE_BOOLEAN_V3, "boolean value", "bool"),
            dictionary_entry(
                EFFECT_VALUE_IDENTIFIER_V3,
                "opaque identifier value",
                "identifier",
            ),
            dictionary_entry(EFFECT_VALUE_COLLECTION_V3, "collection value", "collection"),
            dictionary_entry(
                EFFECT_VALUE_OPERATION_V3,
                "operation node",
                "operation_code",
            ),
        ],
    )
}

pub(super) fn build_dictionary(
    version: u16,
    mut entries: Vec<EffectDictionaryEntryV3>,
) -> Result<EffectLawDictionaryV3, EffectLawV3Error> {
    entries.sort();
    if version == 0
        || entries.is_empty()
        || entries.len() > MAX_DICTIONARY_ENTRIES_V3
        || entries.windows(2).any(|pair| pair[0].code == pair[1].code)
        || entries.iter().any(|entry| {
            entry.code == 0
                || entry.version != version
                || !valid_nonzero_sha256(&entry.meaning_sha256)
                || !valid_nonzero_sha256(&entry.operand_schema_sha256)
        })
    {
        return Err(EffectLawV3Error::InvalidDictionary);
    }
    let root_sha256 = sha256_serialized(&("nando.effect-law-dictionary.v3", version, &entries))?;
    Ok(EffectLawDictionaryV3 {
        schema: "nando.effect-law-dictionary.v3".to_owned(),
        version,
        entries,
        root_sha256,
    })
}

pub(super) fn dictionary_from_bytes(
    bytes: &[u8],
) -> Result<EffectLawDictionaryV3, EffectLawV3Error> {
    let wire: EffectLawDictionaryWireV3 =
        serde_json::from_slice(bytes).map_err(|_| EffectLawV3Error::InvalidDictionary)?;
    if wire.schema != "nando.effect-law-dictionary.v3" {
        return Err(EffectLawV3Error::InvalidDictionary);
    }
    let dictionary = build_dictionary(wire.version, wire.entries)?;
    if dictionary.root_sha256 != wire.root_sha256 || dictionary.canonical_bytes()? != bytes {
        return Err(EffectLawV3Error::InvalidDictionary);
    }
    Ok(dictionary)
}

pub(super) fn physical_adapter_hypothesis() -> Result<EffectQuotientHypothesisV3, EffectLawV3Error>
{
    let projected_atom_classes = vec![EFFECT_ATOM_PHYSICAL_SURFACE];
    let root_sha256 = sha256_serialized(&(
        EFFECT_QUOTIENT_HYPOTHESIS_SCHEMA_V3,
        EFFECT_LAW_IR_VERSION_V3,
        &projected_atom_classes,
    ))?;
    Ok(EffectQuotientHypothesisV3 {
        schema: EFFECT_QUOTIENT_HYPOTHESIS_SCHEMA_V3.to_owned(),
        version: EFFECT_LAW_IR_VERSION_V3,
        projected_atom_classes,
        root_sha256,
    })
}

pub(super) fn observe_transition(
    transition: TeacherTransition,
) -> Result<EffectObservationCandidateV3, EffectLawV3Error> {
    validate_transition_contract(&transition)?;
    let candidate_sha256 =
        sha256_serialized(&(EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3, &transition))?;
    Ok(EffectObservationCandidateV3 {
        schema: EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3.to_owned(),
        candidate_sha256,
        transition,
    })
}

fn validate_transition_contract(transition: &TeacherTransition) -> Result<(), EffectLawV3Error> {
    if transition.schema != crate::TEACHER_TRANSITION_SCHEMA_V1
        || transition.before.schema != crate::RUNTIME_FRAME_SCHEMA_V1
        || transition.outcome.schema != crate::TEACHER_OUTCOME_SCHEMA_V1
        || !transition.outcome.verifier.accepted
        || !valid_nonzero_sha256(&transition.before.frame_id_sha256)
        || !valid_nonzero_sha256(&transition.before.event_id_sha256)
        || !valid_nonzero_sha256(&transition.before.client_intent_id_sha256)
        || !valid_nonzero_sha256(&transition.before.session_id_sha256)
        || !valid_nonzero_sha256(&transition.before.evidence_ref_sha256)
        || !valid_nonzero_sha256(&transition.outcome.verifier.evidence_ref_sha256)
        || !valid_nonzero_sha256(&transition.outcome.verifier.output_digest_sha256)
        || transition.runtime_parity_case.is_none()
    {
        return Err(EffectLawV3Error::InvalidCandidate);
    }
    let expected_signature =
        teacher_program_signature_from_action_atoms(&transition.outcome.action.atoms)
            .ok_or(EffectLawV3Error::InvalidCandidate)?;
    if transition.outcome.action.signature_sha256 != expected_signature {
        return Err(EffectLawV3Error::InvalidCandidate);
    }
    let expected_verifier_digest = raw_json_sha256(&(
        transition.outcome.verifier.evidence_ref_sha256.as_str(),
        &transition.outcome.action.atoms,
        true,
    ))?;
    if transition.outcome.verifier.output_digest_sha256 != expected_verifier_digest {
        return Err(EffectLawV3Error::InvalidVerifierReceipt);
    }
    if EffectGraphBuilder::default().build(transition).completeness
        != EffectGraphCompleteness::Complete
    {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    Ok(())
}

fn raw_json_sha256<T: Serialize>(value: &T) -> Result<String, EffectLawV3Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| EffectLawV3Error::Serialization)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn atom_class(phase: u16, atom: &RelationAtom) -> u16 {
    match atom {
        RelationAtom::ActionFunction { .. }
        | RelationAtom::ActionCustomTool { .. }
        | RelationAtom::ActionInnerTool { .. }
        | RelationAtom::ObservationCallShape { .. }
        | RelationAtom::ToolKind { .. }
        | RelationAtom::ActionResultProjection { .. }
        | RelationAtom::ActionOutputProjection { .. }
        | RelationAtom::ActionJsonResultProjection => EFFECT_ATOM_PHYSICAL_SURFACE,
        RelationAtom::CompletionState { .. }
        | RelationAtom::ResponseShape { .. }
        | RelationAtom::OutputStatus { .. } => EFFECT_ATOM_POSTCONDITION,
        RelationAtom::ActionValueProjection { .. }
        | RelationAtom::ActionStatusProjection { .. } => EFFECT_ATOM_RENDERER,
        RelationAtom::TemporalEdge { .. } => EFFECT_ATOM_TEMPORAL,
        RelationAtom::Cardinality { .. } => EFFECT_ATOM_CARDINALITY,
        _ if phase == PHASE_BEFORE => EFFECT_ATOM_PRECONDITION,
        _ => EFFECT_ATOM_ACTION_RELATION,
    }
}

fn build_delta_contract(
    before_atoms: &[RelationAtom],
    effect_atoms: &[RelationAtom],
) -> Result<EffectDeltaContractV3, EffectLawV3Error> {
    let mut exact_atoms = before_atoms
        .iter()
        .map(|atom| ExactEffectAtomV3 {
            phase: PHASE_BEFORE,
            class_code: atom_class(PHASE_BEFORE, atom),
            atom: atom.clone(),
        })
        .chain(effect_atoms.iter().map(|atom| ExactEffectAtomV3 {
            phase: PHASE_ACTION,
            class_code: atom_class(PHASE_ACTION, atom),
            atom: atom.clone(),
        }))
        .collect::<Vec<_>>();
    exact_atoms.sort();
    let has_completion = exact_atoms.iter().any(|item| {
        matches!(item.atom, RelationAtom::CompletionState { .. })
            && item.class_code == EFFECT_ATOM_POSTCONDITION
    });
    let has_response_shape = exact_atoms.iter().any(|item| {
        matches!(item.atom, RelationAtom::ResponseShape { .. })
            && item.class_code == EFFECT_ATOM_POSTCONDITION
    });
    let has_surface = exact_atoms
        .iter()
        .any(|item| item.class_code == EFFECT_ATOM_PHYSICAL_SURFACE);
    if before_atoms.is_empty()
        || exact_atoms.len() > MAX_EFFECT_ATOMS_V3
        || !has_completion
        || !has_response_shape
        || !has_surface
    {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    let roots_for = |class_code| {
        sha256_serialized(
            &exact_atoms
                .iter()
                .filter(|item| item.class_code == class_code)
                .collect::<Vec<_>>(),
        )
    };
    Ok(EffectDeltaContractV3 {
        schema: EFFECT_DELTA_CONTRACT_SCHEMA_V3.to_owned(),
        exact_root_sha256: sha256_serialized(&exact_atoms)?,
        surface_root_sha256: roots_for(EFFECT_ATOM_PHYSICAL_SURFACE)?,
        postcondition_root_sha256: roots_for(EFFECT_ATOM_POSTCONDITION)?,
        renderer_root_sha256: roots_for(EFFECT_ATOM_RENDERER)?,
        temporal_root_sha256: roots_for(EFFECT_ATOM_TEMPORAL)?,
        cardinality_root_sha256: roots_for(EFFECT_ATOM_CARDINALITY)?,
        exact_atoms,
    })
}

pub(super) fn seal_observation(
    candidate: EffectObservationCandidateV3,
    trusted_evidence: &TrustedEffectEvidenceSetV3,
    actor: &ResponseProgram,
    verifier: &VerifierProgram,
) -> Result<SealedEffectObservationV3, EffectLawV3Error> {
    if candidate.schema != EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3
        || candidate.candidate_sha256
            != sha256_serialized(&(
                EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3,
                &candidate.transition,
            ))?
    {
        return Err(EffectLawV3Error::InvalidCandidate);
    }
    validate_transition_contract(&candidate.transition)?;
    let transition = &candidate.transition;
    let parity_case = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(EffectLawV3Error::InvalidParityReceipt)?;
    if parity_case.evidence_ref_sha256 != transition.before.frame_id_sha256 {
        return Err(EffectLawV3Error::InvalidParityReceipt);
    }
    let (trusted_entry, parity_receipt, observed_state) =
        trust::entry(trusted_evidence, &parity_case.evidence_ref_sha256)?;
    let transition_sha256 = sha256_serialized(transition)?;
    if transition_sha256 != trusted_entry.transition_sha256
        || transition.before.session_id_sha256 != trusted_entry.episode_lineage_sha256
    {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    let capture_receipt = parity_case
        .capture_receipt
        .as_ref()
        .ok_or(EffectLawV3Error::InvalidCaptureReceipt)?;
    trusted_evidence
        .capture_index
        .verify_receipt(capture_receipt)
        .map_err(|_| EffectLawV3Error::InvalidCaptureReceipt)?;
    parity_receipt
        .validate_sealed()
        .map_err(|_| EffectLawV3Error::InvalidParityReceipt)?;
    if capture_receipt.records_root_sha256 != trusted_entry.capture_receipt_root_sha256
        || parity_receipt.input_sha256
            != canonical_json_sha256(&parity_case.provider_payload)
                .map_err(|_| EffectLawV3Error::Serialization)?
        || parity_receipt.teacher_response_sha256
            != sha256_bytes(parity_case.expected_response.as_bytes())
        || parity_receipt.actor_response_sha256 != parity_receipt.teacher_response_sha256
    {
        return Err(EffectLawV3Error::InvalidParityReceipt);
    }
    let actor_program_sha256 =
        response_actor_program_digest(actor).map_err(|_| EffectLawV3Error::Serialization)?;
    let verifier_program_sha256 = response_independent_verifier_program_digest(verifier)
        .map_err(|_| EffectLawV3Error::Serialization)?;
    if actor_program_sha256 != parity_receipt.program_sha256
        || verifier_program_sha256 != parity_receipt.verifier_sha256
        || actor_program_sha256 != trusted_entry.physical_program_id
    {
        return Err(EffectLawV3Error::InvalidParityReceipt);
    }
    let execution = execute_response(
        actor,
        &parity_case.request_text,
        &parity_case.provider_payload,
    );
    if execution.status != ResponseExecutionStatus::Executed {
        return Err(EffectLawV3Error::InvalidParityReceipt);
    }
    let actor_response = execution
        .response
        .as_deref()
        .ok_or(EffectLawV3Error::InvalidParityReceipt)?;
    verify_response_independently_with_request(
        verifier,
        &parity_case.request_text,
        &parity_case.provider_payload,
        actor_response,
    )
    .map_err(|_| EffectLawV3Error::InvalidVerifierReceipt)?;
    let actor_response_sha256 = sha256_bytes(actor_response.as_bytes());
    if actor_response_sha256 != parity_receipt.actor_response_sha256
        || actor_response_sha256 != observed_state.actor_response_sha256
        || actor_response != parity_case.expected_response
        || observed_state.before_atoms_root_sha256 != sha256_serialized(&transition.before.atoms)?
    {
        return Err(EffectLawV3Error::InvalidVerifierReceipt);
    }
    let teacher_claim =
        build_delta_contract(&transition.before.atoms, &transition.outcome.action.atoms)?;
    let independently_observed_delta =
        build_delta_contract(&transition.before.atoms, &observed_state.effect_atoms)?;
    if teacher_claim.exact_root_sha256 != independently_observed_delta.exact_root_sha256 {
        return Err(EffectLawV3Error::EffectDeltaDisagreement);
    }
    let mut delta_receipt = VerifiedEffectDeltaReceiptV3 {
        schema: VERIFIED_EFFECT_DELTA_RECEIPT_SCHEMA_V3.to_owned(),
        receipt_sha256: String::new(),
        evidence_ref_sha256: parity_case.evidence_ref_sha256.clone(),
        transition_sha256: transition_sha256.clone(),
        trust_manifest_root_sha256: trusted_evidence.trust_manifest_root_sha256.clone(),
        observed_state_root_sha256: observed_state.receipt_sha256.clone(),
        actor_program_sha256,
        verifier_program_sha256,
        delta_verifier_root_sha256: trusted_evidence.delta_verifier_root_sha256.clone(),
        teacher_claim_root_sha256: teacher_claim.exact_root_sha256,
        delta: independently_observed_delta,
    };
    delta_receipt.receipt_sha256 = verified_delta_receipt_digest(&delta_receipt)?;
    let protocol_facet = build_protocol_facet(transition)?;
    if protocol_facet.root_sha256 != trusted_entry.surface_root_sha256 {
        return Err(EffectLawV3Error::InvalidTrustRoot);
    }
    if !valid_nonzero_sha256(&parity_receipt.verifier_sha256) {
        return Err(EffectLawV3Error::InvalidVerifierReceipt);
    }
    let (physical_graph, slot_material) = build_physical_graph(transition)?;
    let role_bindings = build_role_bindings(transition, &slot_material)?;
    let constants = build_constants(transition)?;
    if !has_preserved_frame_evidence(&physical_graph) {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    let mut observation = SealedEffectObservationV3 {
        schema: SEALED_EFFECT_OBSERVATION_SCHEMA_V3.to_owned(),
        observation_sha256: String::new(),
        evidence_ref_sha256: parity_case.evidence_ref_sha256.clone(),
        transition_sha256,
        episode_lineage_sha256: transition.before.session_id_sha256.clone(),
        surface_root_sha256: protocol_facet.root_sha256.clone(),
        physical_program_id: parity_receipt.program_sha256.clone(),
        capture_receipt_root_sha256: capture_receipt.records_root_sha256.clone(),
        parity_receipt_root_sha256: parity_receipt.receipt_sha256.clone(),
        verifier_root_sha256: parity_receipt.verifier_sha256.clone(),
        resolver_root_sha256: trusted_evidence.resolver_root_sha256.clone(),
        trust_manifest_root_sha256: trusted_evidence.trust_manifest_root_sha256.clone(),
        observed_state_root_sha256: observed_state.receipt_sha256.clone(),
        verified_delta_receipt_root_sha256: delta_receipt.receipt_sha256.clone(),
        delta_verifier_root_sha256: trusted_evidence.delta_verifier_root_sha256.clone(),
        delta: delta_receipt.delta,
        protocol_facet,
        physical_graph,
        role_bindings,
        constants,
    };
    observation.observation_sha256 = sealed_observation_digest(&observation)?;
    Ok(observation)
}

fn verified_delta_receipt_digest(
    receipt: &VerifiedEffectDeltaReceiptV3,
) -> Result<String, EffectLawV3Error> {
    sha256_serialized(&(
        receipt.schema.as_str(),
        receipt.evidence_ref_sha256.as_str(),
        receipt.transition_sha256.as_str(),
        receipt.trust_manifest_root_sha256.as_str(),
        receipt.observed_state_root_sha256.as_str(),
        receipt.actor_program_sha256.as_str(),
        receipt.verifier_program_sha256.as_str(),
        receipt.delta_verifier_root_sha256.as_str(),
        receipt.teacher_claim_root_sha256.as_str(),
        &receipt.delta,
    ))
}

pub(super) fn build_protocol_facet(
    transition: &TeacherTransition,
) -> Result<ProtocolFacetV3, EffectLawV3Error> {
    let mut physical_atoms = transition
        .before
        .atoms
        .iter()
        .map(|atom| (PHASE_BEFORE, atom))
        .chain(
            transition
                .outcome
                .action
                .atoms
                .iter()
                .map(|atom| (PHASE_ACTION, atom)),
        )
        .filter_map(|(phase, atom)| {
            protocol_surface_atom(atom).map(|surface| {
                json!({
                    "phase": phase,
                    "surface": surface,
                })
            })
        })
        .collect::<Vec<_>>();
    physical_atoms.sort_by_cached_key(|atom| canonical_json_bytes(atom).unwrap_or_default());
    physical_atoms.dedup();
    if physical_atoms.is_empty() || physical_atoms.len() > MAX_EFFECT_ATOMS_V3 {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    let root_sha256 = sha256_serialized(&(PROTOCOL_FACET_SCHEMA_V3, &physical_atoms))?;
    Ok(ProtocolFacetV3 {
        schema: PROTOCOL_FACET_SCHEMA_V3.to_owned(),
        physical_atoms,
        root_sha256,
    })
}

fn protocol_surface_atom(atom: &RelationAtom) -> Option<serde_json::Value> {
    match atom {
        RelationAtom::ObservationSelector { .. }
        | RelationAtom::ObservationCallShape { .. }
        | RelationAtom::ActionFunction { .. }
        | RelationAtom::ActionCustomTool { .. }
        | RelationAtom::ActionInnerTool { .. }
        | RelationAtom::ActionRoleArgument { .. }
        | RelationAtom::ActionIntegerArgument { .. }
        | RelationAtom::ActionStringArgument { .. }
        | RelationAtom::ActionBooleanArgument { .. }
        | RelationAtom::ActionResultProjection { .. }
        | RelationAtom::ActionOutputProjection { .. }
        | RelationAtom::ActionJsonResultProjection
        | RelationAtom::ToolKind { .. }
        | RelationAtom::TypedEquality { .. }
        | RelationAtom::Cardinality { .. }
        | RelationAtom::TemporalEdge { .. } => serde_json::to_value(atom).ok(),
        _ => None,
    }
}

pub(super) fn validate_sealed_observation(
    observation: &SealedEffectObservationV3,
) -> Result<(), EffectLawV3Error> {
    if observation.schema != SEALED_EFFECT_OBSERVATION_SCHEMA_V3
        || !valid_nonzero_sha256(&observation.transition_sha256)
        || !valid_nonzero_sha256(&observation.evidence_ref_sha256)
        || !valid_nonzero_sha256(&observation.episode_lineage_sha256)
        || !valid_nonzero_sha256(&observation.surface_root_sha256)
        || !valid_nonzero_sha256(&observation.physical_program_id)
        || !valid_nonzero_sha256(&observation.capture_receipt_root_sha256)
        || !valid_nonzero_sha256(&observation.parity_receipt_root_sha256)
        || !valid_nonzero_sha256(&observation.verifier_root_sha256)
        || !valid_nonzero_sha256(&observation.resolver_root_sha256)
        || !valid_nonzero_sha256(&observation.trust_manifest_root_sha256)
        || !valid_nonzero_sha256(&observation.observed_state_root_sha256)
        || !valid_nonzero_sha256(&observation.verified_delta_receipt_root_sha256)
        || !valid_nonzero_sha256(&observation.delta_verifier_root_sha256)
        || observation.delta.schema != EFFECT_DELTA_CONTRACT_SCHEMA_V3
        || observation.protocol_facet.schema != PROTOCOL_FACET_SCHEMA_V3
        || observation.protocol_facet.root_sha256 != observation.surface_root_sha256
        || sha256_serialized(&(
            PROTOCOL_FACET_SCHEMA_V3,
            &observation.protocol_facet.physical_atoms,
        ))? != observation.protocol_facet.root_sha256
        || observation.observation_sha256 != sealed_observation_digest(observation)?
    {
        return Err(EffectLawV3Error::InvalidCandidate);
    }
    Ok(())
}

fn sealed_observation_digest(
    observation: &SealedEffectObservationV3,
) -> Result<String, EffectLawV3Error> {
    sha256_serialized(&(
        (
            observation.schema.as_str(),
            observation.evidence_ref_sha256.as_str(),
            observation.transition_sha256.as_str(),
            observation.episode_lineage_sha256.as_str(),
            observation.surface_root_sha256.as_str(),
            observation.physical_program_id.as_str(),
            observation.capture_receipt_root_sha256.as_str(),
            observation.parity_receipt_root_sha256.as_str(),
            observation.verifier_root_sha256.as_str(),
            observation.resolver_root_sha256.as_str(),
        ),
        (
            observation.trust_manifest_root_sha256.as_str(),
            observation.observed_state_root_sha256.as_str(),
            observation.verified_delta_receipt_root_sha256.as_str(),
            observation.delta_verifier_root_sha256.as_str(),
            &observation.delta,
            &observation.protocol_facet,
            &observation.physical_graph,
            &observation.role_bindings,
            &observation.constants,
        ),
    ))
}

fn build_physical_graph(
    transition: &TeacherTransition,
) -> Result<(PhysicalEffectGraphV3, BTreeMap<u16, SlotMaterial>), EffectLawV3Error> {
    let atoms = transition
        .before
        .atoms
        .iter()
        .chain(&transition.outcome.action.atoms)
        .collect::<Vec<_>>();
    let mut slots = BTreeMap::<u16, SlotMaterial>::new();
    for atom in &atoms {
        if let RelationAtom::TypedSlot {
            slot_id,
            value_type,
            source,
            value_sha256,
        } = atom
        {
            if !valid_nonzero_sha256(value_sha256) {
                return Err(EffectLawV3Error::InvalidCandidate);
            }
            let material = SlotMaterial {
                source: *source,
                value_type: *value_type,
                value_sha256: value_sha256.clone(),
                unique: false,
            };
            if let Some(existing) = slots.get(slot_id) {
                if existing != &material {
                    return Err(EffectLawV3Error::InvalidCandidate);
                }
            } else {
                slots.insert(*slot_id, material);
            }
        }
    }
    for atom in &atoms {
        if let RelationAtom::UniqueSlot { slot_id } = atom {
            slots
                .get_mut(slot_id)
                .ok_or(EffectLawV3Error::InvalidCandidate)?
                .unique = true;
        }
    }
    if slots.is_empty() || slots.len() > MAX_EFFECT_NODES_V3 {
        return Err(EffectLawV3Error::OverBudget);
    }
    let mut nodes = slots
        .iter()
        .map(|(slot_id, material)| PhysicalEffectNodeV3 {
            physical_node: *slot_id,
            source: EffectSource::from(material.source),
            node_kind_code: if material.value_type == AtomValueType::Collection {
                NODE_COLLECTION
            } else {
                NODE_SCALAR
            },
            value_type_code: value_type_code(material.value_type),
            unique: material.unique,
            operation_code: None,
        })
        .collect::<Vec<_>>();
    let operation_codes = operation_codes(&atoms);
    let mut next_node = slots
        .keys()
        .next_back()
        .copied()
        .unwrap_or_default()
        .checked_add(1)
        .ok_or(EffectLawV3Error::OverBudget)?;
    let mut operation_nodes = BTreeMap::new();
    for operation_code in operation_codes {
        operation_nodes.insert(operation_code, next_node);
        nodes.push(PhysicalEffectNodeV3 {
            physical_node: next_node,
            source: EffectSource::Derived,
            node_kind_code: NODE_OPERATION,
            value_type_code: EFFECT_VALUE_OPERATION_V3,
            unique: true,
            operation_code: Some(operation_code),
        });
        next_node = next_node
            .checked_add(1)
            .ok_or(EffectLawV3Error::OverBudget)?;
    }
    if nodes.len() > MAX_EFFECT_NODES_V3 {
        return Err(EffectLawV3Error::OverBudget);
    }
    let mut edges = BTreeSet::new();
    for atom in &atoms {
        match atom {
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } => {
                if !slots.contains_key(left_slot) || !slots.contains_key(right_slot) {
                    return Err(EffectLawV3Error::InvalidCandidate);
                }
                edges.insert(PhysicalEffectEdgeV3 {
                    from: (*left_slot).min(*right_slot),
                    to: (*left_slot).max(*right_slot),
                    relation_code: EFFECT_REL_EQUAL,
                });
            }
            RelationAtom::ActionRoleArgument { slot_id, .. } => {
                let call = operation_nodes
                    .get(&EFFECT_OPERATION_CALL_V3)
                    .copied()
                    .ok_or(EffectLawV3Error::IncompleteEffectDelta)?;
                if !slots.contains_key(slot_id) {
                    return Err(EffectLawV3Error::InvalidCandidate);
                }
                edges.insert(PhysicalEffectEdgeV3 {
                    from: *slot_id,
                    to: call,
                    relation_code: EFFECT_REL_CONSUME,
                });
            }
            _ => {}
        }
    }
    let action_slots = slots
        .iter()
        .filter(|(_, slot)| matches!(slot.source, AtomSource::Action | AtomSource::Outcome))
        .map(|(slot_id, _)| *slot_id)
        .collect::<Vec<_>>();
    for action_slot in action_slots {
        if edges.iter().any(|edge| {
            edge.relation_code == EFFECT_REL_EQUAL
                && (edge.from == action_slot || edge.to == action_slot)
        }) {
            continue;
        }
        let action = slots
            .get(&action_slot)
            .ok_or(EffectLawV3Error::InvalidCandidate)?;
        let candidates = slots
            .iter()
            .filter(|(_, source)| {
                matches!(source.source, AtomSource::Request | AtomSource::Observation)
                    && source.value_type == action.value_type
                    && source.value_sha256 == action.value_sha256
            })
            .map(|(slot_id, _)| *slot_id)
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [source] => {
                edges.insert(PhysicalEffectEdgeV3 {
                    from: action_slot,
                    to: *source,
                    relation_code: EFFECT_REL_COPY,
                });
            }
            [] => return Err(EffectLawV3Error::IncompleteEffectDelta),
            _ => return Err(EffectLawV3Error::AmbiguousActionEquivalence),
        }
    }
    if edges.len() > MAX_EFFECT_EDGES_V3 {
        return Err(EffectLawV3Error::OverBudget);
    }
    nodes.sort();
    Ok((
        PhysicalEffectGraphV3 {
            nodes,
            edges: edges.into_iter().collect(),
        },
        slots,
    ))
}

fn operation_codes(atoms: &[&RelationAtom]) -> BTreeSet<u16> {
    atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionFunction { .. }
            | RelationAtom::ActionCustomTool { .. }
            | RelationAtom::ActionInnerTool { .. }
            | RelationAtom::ActionRoleArgument { .. }
            | RelationAtom::ActionIntegerArgument { .. }
            | RelationAtom::ActionStringArgument { .. }
            | RelationAtom::ActionBooleanArgument { .. } => Some(EFFECT_OPERATION_CALL_V3),
            RelationAtom::ActionResultProjection { .. }
            | RelationAtom::ActionOutputProjection { .. }
            | RelationAtom::ActionJsonResultProjection
            | RelationAtom::ActionValueProjection { .. } => Some(EFFECT_OPERATION_PROJECT_V3),
            RelationAtom::ActionStatusProjection { .. } => Some(EFFECT_OPERATION_STATUS_V3),
            RelationAtom::ActionPlanAdvance => Some(EFFECT_OPERATION_PLAN_ADVANCE_V3),
            _ => None,
        })
        .collect()
}

fn build_role_bindings(
    transition: &TeacherTransition,
    slots: &BTreeMap<u16, SlotMaterial>,
) -> Result<Vec<PhysicalRoleBindingV3>, EffectLawV3Error> {
    let mut bindings = transition
        .outcome
        .action
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument {
                name,
                slot_id,
                value_type,
            } => Some((name, *slot_id, *value_type)),
            _ => None,
        })
        .map(|(name, slot_id, declared_type)| {
            let slot = slots
                .get(&slot_id)
                .ok_or(EffectLawV3Error::InvalidCandidate)?;
            if declared_type.is_some_and(|value| value != slot.value_type) {
                return Err(EffectLawV3Error::InvalidCandidate);
            }
            Ok(PhysicalRoleBindingV3 {
                argument_key_sha256: sha256_bytes(name.as_bytes()),
                physical_node: slot_id,
                value_type_code: value_type_code(slot.value_type),
            })
        })
        .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
    bindings.sort();
    if bindings.is_empty() {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    Ok(bindings)
}

fn build_constants(
    transition: &TeacherTransition,
) -> Result<Vec<PhysicalConstantV3>, EffectLawV3Error> {
    let mut constants = transition
        .outcome
        .action
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionIntegerArgument { name, value } => {
                Some((name, EFFECT_VALUE_INTEGER_V3, serde_json::json!(value)))
            }
            RelationAtom::ActionStringArgument { name, value } => {
                Some((name, EFFECT_VALUE_STRING_V3, serde_json::json!(value)))
            }
            RelationAtom::ActionBooleanArgument { name, value } => {
                Some((name, EFFECT_VALUE_BOOLEAN_V3, serde_json::json!(value)))
            }
            _ => None,
        })
        .map(|(name, value_type_code, value)| {
            Ok(PhysicalConstantV3 {
                argument_key_sha256: sha256_bytes(name.as_bytes()),
                value_type_code,
                value_sha256: sha256_serialized(&(value_type_code, value))?,
            })
        })
        .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
    constants.sort();
    Ok(constants)
}

fn has_preserved_frame_evidence(graph: &PhysicalEffectGraphV3) -> bool {
    graph.edges.iter().any(|edge| {
        matches!(edge.relation_code, EFFECT_REL_EQUAL | EFFECT_REL_COPY)
            && graph.nodes.iter().any(|node| {
                (node.physical_node == edge.from || node.physical_node == edge.to)
                    && matches!(
                        node.source,
                        EffectSource::Request | EffectSource::Observation
                    )
            })
            && graph.nodes.iter().any(|node| {
                (node.physical_node == edge.from || node.physical_node == edge.to)
                    && matches!(node.source, EffectSource::Action | EffectSource::Outcome)
            })
    })
}
