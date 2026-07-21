#[cfg(test)]
mod tests {
    use super::*;

    fn frame(extractor_version: &str) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: extractor_version.to_owned(),
            verifier_label: Some(true),
            atoms: Vec::new(),
            evidence_ref_sha256: "e".repeat(64),
        }
    }

    #[test]
    fn source_neutral_generation_filter_rejects_legacy_frames() {
        assert!(is_source_neutral_relation_frame(&frame(
            SOURCE_NEUTRAL_EXTRACTOR_VERSION
        )));
        assert!(is_source_neutral_relation_frame(&frame(
            PREVIOUS_SOURCE_NEUTRAL_EXTRACTOR_VERSION
        )));
        assert!(is_source_neutral_relation_frame(&frame(
            OLDEST_SOURCE_NEUTRAL_EXTRACTOR_VERSION
        )));
        assert!(!is_source_neutral_relation_frame(&frame(
            "response-relation-extractor.v4"
        )));
    }

    #[test]
    fn status_family_ignores_names_free_text_and_proof_labels() {
        let mut first = frame(SOURCE_NEUTRAL_EXTRACTOR_VERSION);
        first.atoms = vec![
            RelationAtom::ToolKind {
                value: "first_tool".to_owned(),
            },
            RelationAtom::ObservationCallShape {
                value: "first_route".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::OutputStatus {
                value: "free text success".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "first_target".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: crate::ResponseValueSelector::JsonField {
                    field: "opaque_value".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
            RelationAtom::ActionStatusProjection {
                mapping: crate::ProjectStatusMapping::ZeroIsSuccess,
            },
        ];
        let mut second = first.clone();
        second.verifier_label = Some(false);
        for atom in &mut second.atoms {
            match atom {
                RelationAtom::ToolKind { value }
                | RelationAtom::ObservationCallShape { value }
                | RelationAtom::OutputStatus { value }
                | RelationAtom::ResponseShape { value } => value.push_str("_renamed"),
                _ => {}
            }
        }

        let first = ground_roles(&first);
        let second = ground_roles(&second);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(first[0].frame_family_id, second[0].frame_family_id);
        assert!(
            first[0]
                .bindings
                .contains_key(&SemanticRole::StatusOrResult)
        );
    }

    #[test]
    fn discovery_family_is_identical_with_or_without_future_action() {
        let pre_action = vec![
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
        ];
        let mut with_action = pre_action.clone();
        with_action.extend([
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "cell_id".to_owned(),
                slot_id: 2,
                value_type: None,
            },
            RelationAtom::ActionIntegerArgument {
                name: "max_tokens".to_owned(),
                value: 5_000,
            },
            RelationAtom::ResponseShape {
                value: "function_call".to_owned(),
            },
        ]);
        assert_eq!(family_id(&pre_action), family_id(&with_action));
    }

    #[test]
    fn continuation_selector_overrides_legacy_completion_state_for_role_grounding() {
        let mut completed = frame(SOURCE_NEUTRAL_EXTRACTOR_VERSION);
        completed.atoms = vec![
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: crate::ResponseValueSelector::ContentLinePrefix {
                    prefix: "Script running with cell ID ".to_owned(),
                    value_type: AtomValueType::Identifier,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
        ];

        let hypotheses = ground_roles(&completed);
        assert_eq!(hypotheses.len(), 1);
        assert!(
            hypotheses[0]
                .bindings
                .contains_key(&SemanticRole::ContinuationHandle)
        );
        assert!(
            !hypotheses[0]
                .bindings
                .contains_key(&SemanticRole::SourceValue)
        );

        let equality = completed
            .atoms
            .iter_mut()
            .find_map(|atom| match atom {
                RelationAtom::SlotEquality {
                    left_slot,
                    right_slot,
                } => Some((left_slot, right_slot)),
                _ => None,
            })
            .expect("slot equality");
        std::mem::swap(equality.0, equality.1);
        let reversed = ground_roles(&completed);
        assert_eq!(reversed, hypotheses);
    }
}
