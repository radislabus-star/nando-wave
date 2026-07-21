#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame};

    #[test]
    fn wave_atoms_preserve_observable_tool_protocol_symbol_without_splitting_family() {
        let first = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![RelationAtom::ToolKind {
                value: "first_observed_tool".to_owned(),
            }],
            evidence_ref_sha256: "e".repeat(64),
        };
        let mut second = first.clone();
        for atom in &mut second.atoms {
            if let RelationAtom::ToolKind { value } = atom {
                *value = "different_observed_tool".to_owned();
            }
        }
        assert_eq!(
            relation_frame_structural_family_id(&first),
            relation_frame_structural_family_id(&second)
        );
        assert_ne!(
            crate::package::relation_frame_online_routing_atom_ids(&first),
            crate::package::relation_frame_online_routing_atom_ids(&second)
        );
    }
}
