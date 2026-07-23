use nando_operator_kernel::{AtomSource, RelationAtom};

use super::*;

#[test]
fn capture_frame_identity_projects_into_runtime_identity() {
    let capture_frame_id = "a".repeat(64);
    let atoms = vec![RelationAtom::TypedSlot {
        slot_id: 1,
        value_type: nando_operator_kernel::AtomValueType::Integer,
        source: AtomSource::Observation,
        value_sha256: "b".repeat(64),
    }];
    let runtime = RuntimeFrame {
        schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
        frame_id_sha256: runtime_frame_id_from_capture_frame(&capture_frame_id, &atoms),
        event_id_sha256: "c".repeat(64),
        client_intent_id_sha256: "d".repeat(64),
        session_id_sha256: "e".repeat(64),
        observed_at_unix_nanos: 1,
        extractor_version: "test".to_owned(),
        atoms,
        evidence_ref_sha256: "f".repeat(64),
    };

    assert_ne!(runtime.frame_id_sha256, capture_frame_id);
    assert_eq!(runtime.verify_capture_frame_id(&capture_frame_id), Ok(()));
}

#[test]
fn capture_projection_rejects_observable_atom_substitution() {
    let capture_frame_id = "1".repeat(64);
    let original_atoms = vec![RelationAtom::UniqueSlot { slot_id: 1 }];
    let mut runtime = RuntimeFrame {
        schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
        frame_id_sha256: runtime_frame_id_from_capture_frame(&capture_frame_id, &original_atoms),
        event_id_sha256: "2".repeat(64),
        client_intent_id_sha256: "3".repeat(64),
        session_id_sha256: "4".repeat(64),
        observed_at_unix_nanos: 1,
        extractor_version: "test".to_owned(),
        atoms: original_atoms,
        evidence_ref_sha256: "5".repeat(64),
    };
    runtime.atoms = vec![RelationAtom::UniqueSlot { slot_id: 2 }];

    assert_eq!(
        runtime.verify_capture_frame_id(&capture_frame_id),
        Err("runtime_frame_capture_binding_mismatch")
    );
}

#[test]
fn canonical_teacher_transition_preserves_capture_lineage() {
    let capture_frame_id = "6".repeat(64);
    let before_atoms = vec![RelationAtom::UniqueSlot { slot_id: 1 }];
    let captured_runtime_id = runtime_frame_id_from_capture_frame(&capture_frame_id, &before_atoms);
    let action_atoms = vec![RelationAtom::ActionFunction {
        value: "wait".to_owned(),
    }];
    let mut training_atoms = before_atoms.clone();
    training_atoms.extend(action_atoms.clone());
    training_atoms.sort();
    let action_signature = "7".repeat(64);
    let training_id = training_relation_frame_id(
        &captured_runtime_id,
        &action_signature,
        true,
        &training_atoms,
    );
    let transition = TeacherTransition {
        schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before: RuntimeFrame {
            schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256: runtime_frame_id_from_capture_frame(&training_id, &before_atoms),
            event_id_sha256: "8".repeat(64),
            client_intent_id_sha256: "9".repeat(64),
            session_id_sha256: "a".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: "test".to_owned(),
            atoms: before_atoms,
            evidence_ref_sha256: "b".repeat(64),
        },
        outcome: TeacherOutcome {
            schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
            action: TeacherActionAst {
                signature_sha256: action_signature,
                action_symbol: "function:wait".to_owned(),
                atoms: action_atoms,
            },
            verifier: TeacherVerifierEvidence {
                accepted: true,
                evidence_ref_sha256: "c".repeat(64),
                output_digest_sha256: "d".repeat(64),
            },
            completed_at_unix_nanos: 1,
        },
        economics: None,
        runtime_parity_case: None,
    };

    assert_eq!(
        transition.verify_capture_frame_id(&capture_frame_id),
        Ok(())
    );
}
