use super::*;
use crate::{
    AtomSource, AtomValueType, ProjectStatusMapping, RELATION_FRAME_SCHEMA, ResponseValueSelector,
    SOURCE_NEUTRAL_EXTRACTOR_VERSION, ValueProjectionFormat,
};

fn pending_projection_frame() -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "1".repeat(64),
        event_id_sha256: "2".repeat(64),
        client_intent_id_sha256: "3".repeat(64),
        session_id_sha256: "4".repeat(64),
        observed_at_unix_nanos: 1,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        estimated_input_tokens: 321,
        atoms: vec![
            RelationAtom::ToolKind {
                value: "source_neutral_tool".to_owned(),
            },
            RelationAtom::ObservationCallShape {
                value: "function_call".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "assistant_message".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "5".repeat(64),
            },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: ResponseValueSelector::ContentLinePrefix {
                    prefix: "value=".to_owned(),
                    value_type: AtomValueType::Identifier,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "5".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 7,
                right_slot: 11,
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ActionValueProjection {
                format: ValueProjectionFormat::PlainText,
                renderer: crate::CollectionOutputRenderer::Direct,
            },
        ],
        evidence_ref_sha256: "6".repeat(64),
    }
}

#[test]
fn continuation_surfaces_share_one_semantic_selector() {
    let mut script = pending_projection_frame();
    script.extractor_version = "legacy-before-source-neutral".to_owned();
    script.verifier_label = None;
    for atom in &mut script.atoms {
        if let RelationAtom::CompletionState { value } = atom {
            *value = "completed".to_owned();
        }
    }
    let mut process = script.clone();
    process.frame_id_sha256 = "7".repeat(64);
    for (frame, physical_selector) in [
        (
            &mut script,
            ResponseValueSelector::TurnOutputScalarOrdinal {
                output_ordinal: 1,
                scalar_ordinal: 3,
                value_type: AtomValueType::Identifier,
            },
        ),
        (
            &mut process,
            ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                reverse_ordinal: 0,
                value_type: AtomValueType::Identifier,
            },
        ),
    ] {
        for atom in &mut frame.atoms {
            if let RelationAtom::ObservationSelector { selector, .. } = atom {
                *selector = physical_selector.clone();
            }
        }
    }
    assert!(ground_roles(&script).is_empty());
    assert!(ground_roles(&process).is_empty());

    let physical = ResponseProgram::function_call_from_roles(
        "wait",
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal: 1,
            scalar_ordinal: 3,
            value_type: AtomValueType::Integer,
        },
        vec![ResponseArgument::Role {
            name: "cell_id".to_owned(),
            role: SemanticRole::SourceValue,
            value_type: None,
        }],
    );
    let canonical = canonicalize_continuation_role_program(&physical, &[script, process])
        .expect("shared continuation role");
    assert!(matches!(
        canonical.operation,
        ResponseOperation::FunctionCallFromRoles {
            selector: ResponseValueSelector::ContinuationHandle {
                value_type: AtomValueType::Identifier
            },
            ..
        }
    ));
}

#[test]
fn grounded_family_splits_on_client_capability_but_not_request_tokens() {
    let base = pending_projection_frame();
    let base_hypothesis = ground_roles(&base).remove(0);
    let base_family =
        grounded_program_family_id(&base, &base_hypothesis).expect("base grounded family");

    let mut request_variant = base.clone();
    request_variant
        .atoms
        .push(RelationAtom::RequestPhaseAtom { atom_id: 11 });
    let request_hypothesis = ground_roles(&request_variant).remove(0);
    assert_eq!(
        base_family,
        grounded_program_family_id(&request_variant, &request_hypothesis)
            .expect("request variant grounded family")
    );

    let mut capability_variant = base.clone();
    capability_variant
        .atoms
        .push(RelationAtom::ClientCapabilityAtom { atom_id: 22 });
    let capability_hypothesis = ground_roles(&capability_variant).remove(0);
    assert_ne!(
        base_family,
        grounded_program_family_id(&capability_variant, &capability_hypothesis)
            .expect("capability variant grounded family")
    );
}

#[test]
fn teacher_signature_treats_poll_modifiers_as_semantic_noops() {
    let mut base = pending_projection_frame();
    base.atoms
        .retain(|atom| !matches!(atom, RelationAtom::ActionValueProjection { .. }));
    base.atoms.push(RelationAtom::ActionFunction {
        value: "wait".to_owned(),
    });
    let mut budgeted = base.clone();
    budgeted.atoms.extend([
        RelationAtom::ActionStringArgument {
            name: "chars".to_owned(),
            value: String::new(),
        },
        RelationAtom::ActionIntegerArgument {
            name: "yield_time_ms".to_owned(),
            value: 30_000,
        },
        RelationAtom::ActionIntegerArgument {
            name: "max_tokens".to_owned(),
            value: 5_000,
        },
    ]);
    assert_eq!(
        teacher_program_signature(&base),
        teacher_program_signature(&budgeted)
    );

    budgeted.atoms.push(RelationAtom::ActionIntegerArgument {
        name: "result_limit".to_owned(),
        value: 10,
    });
    assert_ne!(
        teacher_program_signature(&base),
        teacher_program_signature(&budgeted)
    );
}

fn status_projection_frame() -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "8".repeat(64),
        event_id_sha256: "9".repeat(64),
        client_intent_id_sha256: "a".repeat(64),
        session_id_sha256: "b".repeat(64),
        observed_at_unix_nanos: 2,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: None,
        estimated_input_tokens: 99,
        atoms: vec![
            RelationAtom::ToolKind {
                value: "unrelated_tool_name".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 3,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: "c".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 3 },
            RelationAtom::ObservationSelector {
                slot_id: 3,
                selector: ResponseValueSelector::JsonField {
                    field: "opaque_code".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
            RelationAtom::ActionStatusProjection {
                mapping: ProjectStatusMapping::ZeroIsSuccess,
            },
        ],
        evidence_ref_sha256: "d".repeat(64),
    }
}

fn plan_advance_frame() -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "e".repeat(64),
        event_id_sha256: "f".repeat(64),
        client_intent_id_sha256: "1".repeat(64),
        session_id_sha256: "2".repeat(64),
        observed_at_unix_nanos: 3,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        estimated_input_tokens: 500,
        atoms: vec![
            RelationAtom::ToolKind {
                value: "exec_command".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::OutputStatus {
                value: "success".to_owned(),
            },
            RelationAtom::PlanState {
                step_count: 3,
                completed_count: 0,
                active_index: 0,
            },
            RelationAtom::ActionFunction {
                value: "update_plan".to_owned(),
            },
            RelationAtom::ActionPlanAdvance,
        ],
        evidence_ref_sha256: "3".repeat(64),
    }
}

#[test]
fn plan_advance_bypasses_scalar_grounding_and_synthesizes_typed_operator() {
    let frame = plan_advance_frame();
    let runtime = crate::RuntimeFrame::from_completed(&frame);
    assert!(!runtime.contains_teacher_atoms());
    assert!(
        !runtime
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
    );
    assert!(
        runtime
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::PlanState { .. }))
    );
    assert_eq!(
        partition_teacher_training_families(std::slice::from_ref(&frame)).len(),
        1
    );
    let version_space = enumerate_response_program_candidates(std::slice::from_ref(&frame));
    assert_eq!(version_space.len(), 1);
    assert!(matches!(
        version_space[0].operation,
        ResponseOperation::AdvancePlan { .. }
    ));
    let operator =
        synthesize_response_operator(std::slice::from_ref(&frame)).expect("plan advance synthesis");
    assert!(matches!(
        &operator.candidate.program.operation,
        ResponseOperation::AdvancePlan { function_name } if function_name == "update_plan"
    ));
    assert!(matches!(
        &operator.verifier,
        VerifierProgram::AdvancePlan {
            function_name,
            require_explicit_tool_success: true,
            require_canonical_plan: true,
        } if function_name == "update_plan"
    ));
    assert_eq!(operator.candidate.guard.required_atom_indices.len(), 2);
    assert_eq!(
        response_program_required_routing_atom_ids(&operator.candidate.program).len(),
        3
    );
}

#[test]
fn value_projection_synthesis_preserves_observed_completion_state() {
    let frame = pending_projection_frame();
    let operator = synthesize_response_operator(std::slice::from_ref(&frame))
        .expect("pending projection synthesizes");
    assert!(matches!(
        &operator.candidate.program.operation,
        ResponseOperation::ProjectSelectedValue {
            completion_state,
            ..
        } if completion_state == "pending"
    ));
    assert!(matches!(
        &operator.verifier,
        VerifierProgram::ProjectSelectedValue {
            completion_state,
            require_unique_value: true,
            ..
        } if completion_state == "pending"
    ));
    assert!(verify_operator_structure(&frame, &operator));
}

#[test]
fn value_projection_synthesis_rejects_inconsistent_completion_state() {
    let pending = pending_projection_frame();
    let mut completed = pending.clone();
    completed.frame_id_sha256 = "7".repeat(64);
    for atom in &mut completed.atoms {
        if let RelationAtom::CompletionState { value } = atom {
            *value = "completed".to_owned();
        }
    }
    assert!(synthesize_response_operator(&[pending, completed]).is_err());
}

#[test]
fn status_synthesis_binds_actor_and_independent_verifier_exactly() {
    let frame = status_projection_frame();
    let operator = synthesize_response_operator(std::slice::from_ref(&frame))
        .expect("status projection synthesizes");
    let expected_selector = ResponseValueSelector::JsonField {
        field: "opaque_code".to_owned(),
        value_type: AtomValueType::Integer,
    };
    assert!(matches!(
        &operator.candidate.program.operation,
        ResponseOperation::ProjectStatus {
            selector,
            mapping: ProjectStatusMapping::ZeroIsSuccess,
            completion_state,
            ..
        } if selector == &expected_selector && completion_state == "completed"
    ));
    assert!(matches!(
        &operator.verifier,
        VerifierProgram::ProjectStatus {
            selector,
            mapping: ProjectStatusMapping::ZeroIsSuccess,
            completion_state,
            require_unique_value: true,
            ..
        } if selector == &expected_selector && completion_state == "completed"
    ));
    assert!(verify_operator_structure(&frame, &operator));
}

#[test]
fn status_synthesis_rejects_name_only_and_ambiguous_selector_evidence() {
    let mut name_only = status_projection_frame();
    name_only
        .atoms
        .retain(|atom| !matches!(atom, RelationAtom::ActionStatusProjection { .. }));
    name_only.atoms.push(RelationAtom::OutputStatus {
        value: "success".to_owned(),
    });
    assert!(synthesize_response_operator(&[name_only]).is_err());

    let mut ambiguous = status_projection_frame();
    ambiguous.atoms.push(RelationAtom::ObservationSelector {
        slot_id: 3,
        selector: ResponseValueSelector::ContentLinePrefix {
            prefix: "opaque=".to_owned(),
            value_type: AtomValueType::Integer,
        },
    });
    assert!(synthesize_response_operator(&[ambiguous]).is_err());
}

#[test]
fn cegis_counterexample_eliminates_matching_program() {
    let positive = pending_projection_frame();
    let mut counterexample = positive.clone();
    counterexample.frame_id_sha256 = "e".repeat(64);
    counterexample.verifier_label = Some(false);
    assert_eq!(
        synthesize_response_operator(&[positive, counterexample]),
        Err(SynthesisError::NoConsistentProgram)
    );
}

#[test]
fn cegis_keeps_program_when_counterexample_has_different_action() {
    let positive = pending_projection_frame();
    let mut counterexample = positive.clone();
    counterexample.frame_id_sha256 = "f".repeat(64);
    counterexample.verifier_label = Some(false);
    for atom in &mut counterexample.atoms {
        if let RelationAtom::ActionValueProjection { format, .. } = atom {
            *format = ValueProjectionFormat::CanonicalJson;
        }
    }
    let operator = synthesize_response_operator(&[positive, counterexample])
        .expect("counterexample should eliminate only the conflicting action");
    assert!(matches!(
        operator.candidate.program.operation,
        ResponseOperation::ProjectSelectedValue {
            format: ValueProjectionFormat::PlainText,
            ..
        }
    ));
    assert!(operator.candidate.phase_rank > 0);
    assert!(operator.candidate.exact_checks > 0);
}

#[test]
fn teacher_actions_partition_training_without_changing_structural_family() {
    let direct = pending_projection_frame();
    let mut templated = direct.clone();
    templated.frame_id_sha256 = "9".repeat(64);
    for atom in &mut templated.atoms {
        if let RelationAtom::ActionValueProjection { renderer, .. } = atom {
            *renderer = crate::CollectionOutputRenderer::RenderTemplate {
                prefix: "Result: ".to_owned(),
                suffix: ".".to_owned(),
            };
        }
    }
    let mut negative = direct.clone();
    negative.frame_id_sha256 = "8".repeat(64);
    negative.verifier_label = Some(false);

    assert_eq!(
        ground_roles(&direct)[0].frame_family_id,
        ground_roles(&templated)[0].frame_family_id
    );
    let partitions = partition_teacher_training_families(&[direct, templated, negative]);
    assert_eq!(partitions.len(), 2);
    assert!(partitions.values().all(|frames| {
        frames
            .iter()
            .filter(|frame| frame.verifier_label == Some(true))
            .count()
            == 1
            && frames
                .iter()
                .filter(|frame| frame.verifier_label == Some(false))
                .count()
                == 1
    }));
}
