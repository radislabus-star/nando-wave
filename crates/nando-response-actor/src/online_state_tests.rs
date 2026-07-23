use super::*;
use serde_json::json;

#[test]
fn cross_pool_negative_refresh_is_bounded_to_batch_boundary() {
    assert!(!cross_pool_negative_refresh_due(0));
    assert!(!cross_pool_negative_refresh_due(63));
    assert!(cross_pool_negative_refresh_due(64));
    assert!(!cross_pool_negative_refresh_due(65));
    assert!(cross_pool_negative_refresh_due(128));
}

fn frame(index: usize) -> crate::RelationFrame {
    crate::RelationFrame {
        schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: format!("{index:064x}"),
        event_id_sha256: format!("{:064x}", index + 1_000),
        client_intent_id_sha256: format!("{:064x}", index + 2_000),
        session_id_sha256: format!("{:064x}", index + 3_000),
        observed_at_unix_nanos: u64::try_from(index).unwrap_or(u64::MAX),
        estimated_input_tokens: 1,
        extractor_version: "test".to_owned(),
        verifier_label: Some(true),
        atoms: Vec::new(),
        evidence_ref_sha256: format!("{:064x}", index + 4_000),
    }
}

fn continuation_frame(
    index: usize,
    function_name: &str,
    argument_name: &str,
    prefix: &str,
    tool_kind: &str,
    accepted: bool,
) -> crate::RelationFrame {
    crate::RelationFrame {
        schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: format!("{:064x}", index + 100_000),
        event_id_sha256: format!("{:064x}", index + 200_000),
        client_intent_id_sha256: format!("{:064x}", index + 300_000),
        session_id_sha256: format!("{:064x}", index + 400_000),
        observed_at_unix_nanos: u64::try_from(index + 1).unwrap_or(u64::MAX),
        estimated_input_tokens: 100,
        extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(accepted),
        atoms: vec![
            crate::RelationAtom::ToolKind {
                value: tool_kind.to_owned(),
            },
            crate::RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            crate::RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: crate::AtomValueType::Identifier,
                source: crate::AtomSource::Observation,
                value_sha256: "a".repeat(64),
            },
            crate::RelationAtom::UniqueSlot { slot_id: 1 },
            crate::RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: crate::ResponseValueSelector::ContentLinePrefix {
                    prefix: prefix.to_owned(),
                    value_type: crate::AtomValueType::Identifier,
                },
            },
            crate::RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: crate::AtomValueType::Identifier,
                source: crate::AtomSource::Action,
                value_sha256: "a".repeat(64),
            },
            crate::RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            crate::RelationAtom::ActionFunction {
                value: function_name.to_owned(),
            },
            crate::RelationAtom::ActionRoleArgument {
                name: argument_name.to_owned(),
                slot_id: 2,
                value_type: Some(crate::AtomValueType::Identifier),
            },
        ],
        evidence_ref_sha256: format!("{:064x}", index + 500_000),
    }
}

fn continuation_transition(
    index: usize,
    function_name: &str,
    argument_name: &str,
    prefix: &str,
    request_text: &str,
) -> crate::TeacherTransition {
    let frame = continuation_frame(index, function_name, argument_name, prefix, "exec", true);
    let provider_payload = json!({
        "input": [
            {
                "role": "user",
                "content": [{"type": "input_text", "text": request_text}]
            },
            {
                "type": "function_call_output",
                "output": format!("{prefix}handle-{index}")
            }
        ]
    });
    let program = crate::ResponseProgram::function_call_from_roles(
        function_name,
        crate::ResponseValueSelector::ContentLinePrefix {
            prefix: prefix.to_owned(),
            value_type: crate::AtomValueType::Identifier,
        },
        vec![crate::ResponseArgument::Role {
            name: argument_name.to_owned(),
            role: crate::SemanticRole::ContinuationHandle,
            value_type: Some(crate::AtomValueType::Identifier),
        }],
    );
    let expected = crate::execute_response(&program, request_text, &provider_payload);
    assert_eq!(expected.status, crate::ResponseExecutionStatus::Executed);
    let mut transition =
        crate::teacher_transition_from_completed(&frame, None).expect("teacher transition");
    transition.runtime_parity_case = Some(crate::RuntimeParityCase {
        evidence_ref_sha256: String::new(),
        capture_receipt: None,
        request_text: request_text.to_owned(),
        provider_payload,
        expected_response: expected.response.expect("exact response"),
    });
    transition
}

fn generation() -> FrozenGeneration {
    FrozenGeneration {
        partition_version: FROZEN_PARTITION_VERSION,
        generation_id_sha256: "1".repeat(64),
        generation: 0,
        teacher_signature_sha256: "2".repeat(64),
        cohort_id_sha256: "3".repeat(64),
        support: (0..32).map(frame).collect(),
        future: Vec::new(),
        negatives: Vec::new(),
        support_watermark_unix_nanos: 31,
        support_sessions: 32,
        future_sessions: 0,
        surfaces: 2,
        wrong_future_rows: 0,
        blocker: Some("future_rows_below_32".to_owned()),
    }
}

fn parity_case(index: usize) -> crate::RuntimeParityCase {
    crate::RuntimeParityCase {
        evidence_ref_sha256: format!("{index:064x}"),
        capture_receipt: None,
        request_text: format!("request-{index}"),
        provider_payload: json!({"index": index}),
        expected_response: format!("response-{index}"),
    }
}

#[test]
fn frozen_support_requires_complete_runtime_parity() {
    let generation = generation();
    let mut receipts = GenerationParityReceipts::default();
    for frame in generation.support.iter().take(31) {
        receipts.support.insert(
            frame.frame_id_sha256.clone(),
            parity_case(usize::try_from(frame.observed_at_unix_nanos).unwrap_or_default()),
        );
    }

    assert!(!generation_support_parity_complete(
        &generation,
        Some(&receipts),
        RolloverPolicy::default()
    ));
    receipts.support.insert(
        generation.support[31].frame_id_sha256.clone(),
        parity_case(31),
    );
    assert!(generation_support_parity_complete(
        &generation,
        Some(&receipts),
        RolloverPolicy::default()
    ));
}

#[test]
fn generation_receipts_survive_candidate_eviction_and_restart() {
    let mut state = StreamingSelfTrainingState::new(0);
    let mut frozen = generation();
    let stable_generation_id = frozen.generation_id_sha256.clone();
    let stable_support = frozen.support.clone();
    let stable_watermark = frozen.support_watermark_unix_nanos;

    for index in 0..32 {
        let frame = frame(index);
        let frame_id = frame.frame_id_sha256.clone();
        state
            .runtime_parity_cases
            .insert(frame_id.clone(), parity_case(index));
        state.runtime_parity_frames.insert(frame_id, frame);
    }
    state.enforce_parity_reservoir_limit();
    let support_receipts = state.parity_receipts_for_generation(&frozen);
    state
        .generation_parity_receipts
        .insert(stable_generation_id.clone(), support_receipts);

    frozen.future = (32..64).map(frame).collect();
    frozen.future_sessions = 32;
    for index in 32..64 {
        let frame = frame(index);
        let frame_id = frame.frame_id_sha256.clone();
        state
            .runtime_parity_cases
            .insert(frame_id.clone(), parity_case(index));
        state.runtime_parity_frames.insert(frame_id, frame);
    }
    state.enforce_parity_reservoir_limit();
    let complete_receipts = state.parity_receipts_for_generation(&frozen);
    state
        .generation_parity_receipts
        .insert(stable_generation_id.clone(), complete_receipts);
    state
        .generations
        .insert(frozen.cohort_id_sha256.clone(), frozen.clone());

    for index in 64..96 {
        let frame = frame(index);
        let frame_id = frame.frame_id_sha256.clone();
        state
            .runtime_parity_cases
            .insert(frame_id.clone(), parity_case(index));
        state.runtime_parity_frames.insert(frame_id, frame);
    }
    state.enforce_parity_reservoir_limit();
    let retained = state.parity_receipts_for_generation(&frozen);
    state
        .generation_parity_receipts
        .insert(stable_generation_id.clone(), retained);

    let encoded = serde_json::to_vec(&state).expect("encode self-training state");
    let restored: StreamingSelfTrainingState =
        serde_json::from_slice(&encoded).expect("restore self-training state");
    let reencoded = serde_json::to_vec(&restored).expect("re-encode self-training state");
    assert_eq!(reencoded, encoded);
    let restored_generation = restored
        .generations
        .get(&frozen.cohort_id_sha256)
        .expect("restored generation");
    let restored_receipts = restored
        .generation_parity_receipts
        .get(&stable_generation_id)
        .expect("restored generation receipts");
    assert_eq!(
        restored_generation.generation_id_sha256,
        stable_generation_id
    );
    assert_eq!(restored_generation.support, stable_support);
    assert_eq!(
        restored_generation.support_watermark_unix_nanos,
        stable_watermark
    );
    assert_eq!(restored_receipts.support.len(), 32);
    assert_eq!(restored_receipts.future.len(), 32);
    assert!(generation_support_parity_complete(
        restored_generation,
        Some(restored_receipts),
        RolloverPolicy::default()
    ));
}

#[test]
fn generation_evidence_never_trades_immutable_support_for_future() {
    let current = generation();
    let mut degraded = current.clone();
    degraded.support.pop();
    degraded.future = (100..164).map(frame).collect();
    assert!(!generation_evidence_improves(&current, &degraded));

    let mut improved = current.clone();
    improved.future.push(frame(100));
    assert!(generation_evidence_improves(&current, &improved));
}

#[test]
fn stale_frozen_partition_schedules_one_migration_slice() {
    let mut state = StreamingSelfTrainingState::new(0);
    let mut stale = generation();
    stale.partition_version = FROZEN_PARTITION_VERSION.saturating_sub(1);
    state
        .generations
        .insert(stale.cohort_id_sha256.clone(), stale);
    assert!(state.has_pending_work());

    state
        .generations
        .values_mut()
        .for_each(|generation| generation.partition_version = FROZEN_PARTITION_VERSION);
    assert!(!state.has_pending_work());
}

#[test]
fn metadata_only_rebuild_slice_reports_progress() {
    let mut state = StreamingSelfTrainingState::new(0);
    state
        .rebuild_queue
        .push_back("retired-signature".to_owned());

    let (checks, progressed) = state.run_work_slice_with_progress();

    assert_eq!(checks, 0);
    assert!(progressed);
    assert!(!state.has_pending_work());
}

#[test]
fn startup_reconciliation_queues_only_discovery_signatures_missing_from_cegis() {
    let mut state = StreamingSelfTrainingState::new(0);
    for index in 0..32 {
        state
            .observe_migration_transition(&continuation_transition(
                index,
                "wait",
                "cell_id",
                "Script running with cell ID ",
                "continue",
            ))
            .expect("wait transition");
        state
            .observe_migration_transition(&continuation_transition(
                index + 32,
                "write_stdin",
                "session_id",
                "Script running with session ID ",
                "continue",
            ))
            .expect("write transition");
    }
    let pools = state.discovery.pool_snapshots();
    let represented = pools
        .iter()
        .find(|pool| pool.action_symbol == "function:wait")
        .expect("represented pool")
        .teacher_signature_sha256
        .clone();
    let missing = pools
        .iter()
        .find(|pool| pool.action_symbol == "function:write_stdin")
        .expect("missing pool")
        .teacher_signature_sha256
        .clone();
    let represented_pool = state
        .pool_snapshot_with_parity(&represented)
        .expect("represented snapshot");
    state.cegis.refresh_pool(&represented_pool);
    assert!(state.cegis.teacher_signatures().contains(&represented));
    assert!(!state.cegis.teacher_signatures().contains(&missing));

    let stable_generation = generation();
    let stable_generation_id = stable_generation.generation_id_sha256.clone();
    state.generations.insert(
        stable_generation.cohort_id_sha256.clone(),
        stable_generation,
    );
    state.repair_missing_synthesis_state();

    assert!(state.rebuild_queue.contains(&missing));
    assert!(!state.rebuild_queue.contains(&represented));
    assert!(
        state
            .generations
            .values()
            .any(|generation| generation.generation_id_sha256 == stable_generation_id)
    );
}

#[test]
fn effect_law_migration_moves_old_live_parity_to_support_only() {
    let mut state = StreamingSelfTrainingState::new(0);
    for index in 0..32 {
        state
            .observe_transition(&continuation_transition(
                index,
                "wait",
                "cell_id",
                "Script running with cell ID ",
                "continue",
            ))
            .expect("live transition");
    }
    state
        .generations
        .insert(generation().cohort_id_sha256.clone(), generation());
    assert_eq!(state.runtime_parity_cases.len(), 32);

    state.prepare_effect_law_migration();

    assert_eq!(state.schema, SELF_TRAINING_STATE_SCHEMA_V5);
    assert!(state.runtime_parity_cases.is_empty());
    assert_eq!(state.replay_support_parity_cases.len(), 32);
    assert!(state.generations.is_empty());
    let alias = state.discovery.semantic_alias_graph().report();
    assert_eq!(alias.rows_seen, 32);
    assert!(alias.accounting_complete);
}

#[test]
fn runtime_parity_is_keyed_by_the_frozen_training_frame() {
    let transition = crate::TeacherTransition {
        schema: crate::TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before: crate::RuntimeFrame {
            schema: crate::RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: "test".to_owned(),
            atoms: Vec::new(),
            evidence_ref_sha256: "e".repeat(64),
        },
        outcome: crate::TeacherOutcome {
            schema: crate::TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
            action: crate::TeacherActionAst {
                signature_sha256: "f".repeat(64),
                action_symbol: "function:wait".to_owned(),
                atoms: Vec::new(),
            },
            verifier: crate::TeacherVerifierEvidence {
                accepted: true,
                evidence_ref_sha256: "1".repeat(64),
                output_digest_sha256: "2".repeat(64),
            },
            completed_at_unix_nanos: 2,
        },
        economics: None,
        runtime_parity_case: Some(crate::RuntimeParityCase {
            evidence_ref_sha256: "old".to_owned(),
            capture_receipt: None,
            request_text: "continue".to_owned(),
            provider_payload: json!({"input": []}),
            expected_response: "{}".to_owned(),
        }),
    };
    let training_frame_id = transition.as_training_relation_frame().frame_id_sha256;
    let teacher_signature = parity_teacher_signature(&transition.as_training_relation_frame());
    let mut state = StreamingSelfTrainingState::default();
    state.observe_runtime_parity_case(&transition);

    let parity = state
        .runtime_parity_cases
        .get(&training_frame_id)
        .expect("training-frame parity");
    assert_eq!(parity.evidence_ref_sha256, training_frame_id);
    assert!(
        !state
            .runtime_parity_cases
            .contains_key(&transition.before.frame_id_sha256)
    );
    assert!(
        state
            .dirty_generation_signatures
            .contains(&teacher_signature)
    );
}

#[test]
fn parity_reservoir_is_bounded_across_live_and_replay() {
    let mut state = StreamingSelfTrainingState::default();
    for index in 0..1_100 {
        let frame = frame(index);
        let frame_id = frame.frame_id_sha256.clone();
        state.replay_support_parity_cases.insert(
            frame_id.clone(),
            crate::RuntimeParityCase {
                evidence_ref_sha256: frame_id.clone(),
                capture_receipt: None,
                request_text: "replay".to_owned(),
                provider_payload: json!({"index": index}),
                expected_response: "{}".to_owned(),
            },
        );
        state.replay_support_parity_frames.insert(frame_id, frame);
    }
    for index in 1_100..1_200 {
        let frame = frame(index);
        let frame_id = frame.frame_id_sha256.clone();
        state.runtime_parity_cases.insert(
            frame_id.clone(),
            crate::RuntimeParityCase {
                evidence_ref_sha256: frame_id.clone(),
                capture_receipt: None,
                request_text: "live".to_owned(),
                provider_payload: json!({"index": index}),
                expected_response: "{}".to_owned(),
            },
        );
        state.runtime_parity_frames.insert(frame_id, frame);
    }

    state.enforce_parity_reservoir_limit();

    assert_eq!(state.runtime_parity_cases.len(), 32);
    assert!(state.replay_support_parity_cases.is_empty());
    assert_eq!(
        state.runtime_parity_cases.len(),
        state.runtime_parity_frames.len()
    );
    assert_eq!(
        state.replay_support_parity_cases.len(),
        state.replay_support_parity_frames.len()
    );
}

#[test]
fn restored_state_replaces_raw_parity_frame_with_canonical_discovery_frame() {
    let mut canonical = frame(42);
    canonical.atoms = vec![
        crate::RelationAtom::ActionFunction {
            value: "wait".to_owned(),
        },
        crate::RelationAtom::ActionRoleArgument {
            name: "cell_id".to_owned(),
            slot_id: 7,
            value_type: Some(crate::AtomValueType::Integer),
        },
    ];
    let transition = crate::teacher_transition_from_completed(&canonical, None)
        .expect("canonical teacher transition");
    let canonical_training = transition.as_training_relation_frame();
    let mut state = StreamingSelfTrainingState::default();
    assert_eq!(state.discovery.observe_transition(&transition), Ok(true));

    let mut raw = canonical_training.clone();
    for atom in &mut raw.atoms {
        if let crate::RelationAtom::ActionRoleArgument { value_type, .. } = atom {
            *value_type = None;
        }
    }
    assert_ne!(
        crate::teacher_program_signature(&raw),
        crate::teacher_program_signature(&canonical_training)
    );
    state.runtime_parity_cases.insert(
        canonical_training.frame_id_sha256.clone(),
        crate::RuntimeParityCase {
            evidence_ref_sha256: canonical_training.frame_id_sha256.clone(),
            capture_receipt: None,
            request_text: "continue".to_owned(),
            provider_payload: json!({"input": []}),
            expected_response: "{}".to_owned(),
        },
    );
    state
        .runtime_parity_frames
        .insert(canonical_training.frame_id_sha256.clone(), raw);

    state.repair_missing_synthesis_state();

    assert_eq!(
        state
            .runtime_parity_frames
            .get(&canonical_training.frame_id_sha256),
        Some(&canonical_training)
    );
}

#[test]
fn teacher_signature_migration_rekeys_replay_before_schema_enrichment() {
    let mut transition = continuation_transition(
        43,
        "wait",
        "cell_id",
        "Script running with cell ID ",
        "continue",
    );
    for atom in &mut transition.outcome.action.atoms {
        if let crate::RelationAtom::ActionRoleArgument { value_type, .. } = atom {
            *value_type = None;
        }
    }
    let canonical = transition.as_training_relation_frame();
    let canonical_id = canonical.frame_id_sha256.clone();
    let mut state = StreamingSelfTrainingState::default();
    assert_eq!(state.discovery.observe_transition(&transition), Ok(true));

    let stale_id = "f".repeat(64);
    let mut stale_frame = canonical.clone();
    stale_frame.frame_id_sha256.clone_from(&stale_id);
    let mut parity = transition
        .runtime_parity_case
        .take()
        .expect("runtime parity case");
    parity.evidence_ref_sha256.clone_from(&stale_id);
    state
        .replay_support_parity_cases
        .insert(stale_id.clone(), parity);
    state
        .replay_support_parity_frames
        .insert(stale_id.clone(), stale_frame);

    state
        .prepare_teacher_signature_migration()
        .expect("teacher signature migration");

    assert!(
        state
            .replay_support_parity_cases
            .contains_key(&canonical_id)
    );
    assert!(!state.replay_support_parity_cases.contains_key(&stale_id));
    let migrated = state
        .discovery
        .pool_snapshots()
        .into_iter()
        .flat_map(|pool| pool.positives)
        .find(|frame| frame.frame_id_sha256 == canonical_id)
        .expect("migrated canonical frame");
    assert!(migrated.atoms.iter().any(|atom| matches!(
        atom,
        crate::RelationAtom::ActionRoleArgument {
            name,
            value_type: Some(crate::AtomValueType::String),
            ..
        } if name == "cell_id"
    )));
}

#[test]
fn semantic_law_cohort_combines_two_verified_physical_adapters() {
    let mut state = StreamingSelfTrainingState::new(0);
    for index in 0..16 {
        let negative = continuation_frame(
            index,
            "cancel",
            "handle",
            "Cancelled handle ",
            "negative",
            false,
        );
        let mut transition = crate::teacher_transition_from_completed(&negative, None)
            .expect("negative teacher transition");
        transition.runtime_parity_case = continuation_transition(
            index,
            "cancel",
            "handle",
            "Cancelled handle ",
            "cancel handle",
        )
        .runtime_parity_case;
        state
            .observe_transition(&transition)
            .expect("observe negative");
    }
    for index in 0..32 {
        let transition = continuation_transition(
            1_000 + index,
            "wait",
            "cell_id",
            "Script running with cell ID ",
            "wait for script",
        );
        state
            .observe_transition(&transition)
            .expect("observe wait adapter");
    }
    for index in 0..32 {
        let transition = continuation_transition(
            2_000 + index,
            "continue_process",
            "session",
            "Process running with session ID ",
            "continue process",
        );
        state
            .observe_transition(&transition)
            .expect("observe process adapter");
    }
    for _ in 0..2_048 {
        if state.run_work_slice() == 0 && !state.has_pending_work() {
            break;
        }
    }

    let exact = state.cegis.winners();
    assert_eq!(exact.len(), 2);
    let mut same_signature_peer = exact[0].clone();
    same_signature_peer.cohort_id_sha256 = "same-signature-peer".to_owned();
    let same_signature = state
        .build_semantic_law_cohort(
            &exact[0].teacher_signature_sha256,
            &[exact[0].clone(), same_signature_peer],
        )
        .expect("same-signature structural cohort");
    assert_eq!(same_signature.members.len(), 2);
    assert_eq!(same_signature.member_signatures.len(), 1);
    let mut additional_same_program = exact[0].clone();
    additional_same_program.cohort_id_sha256 = "additional-same-program-peer".to_owned();
    let expanded_same_signature = state
        .build_semantic_law_cohort(
            &exact[0].teacher_signature_sha256,
            &[
                exact[0].clone(),
                same_signature.members[1].clone(),
                additional_same_program,
            ],
        )
        .expect("expanded same-signature structural cohort");
    assert_eq!(
        same_signature.winner.cohort_id_sha256, expanded_same_signature.winner.cohort_id_sha256,
        "physical evidence membership must not change semantic-law identity"
    );
    let law = state
        .discovery
        .semantic_law_signature(&exact[0].teacher_signature_sha256)
        .expect("first semantic law");
    assert_eq!(
        Some(law.as_str()),
        state
            .discovery
            .semantic_law_signature(&exact[1].teacher_signature_sha256)
            .as_deref()
    );
    let semantic = state
        .build_semantic_law_cohort(&law, &exact)
        .unwrap_or_else(|blocker| panic!("semantic law cohort: {blocker}"));
    assert_eq!(semantic.member_signatures.len(), 2);
    assert!(matches!(
        semantic.winner.program.operation,
        crate::ResponseOperation::UniqueConsensus {
            adapter_wave: None,
            ..
        }
    ));
    let same_law_frame = state
        .pool_snapshot_with_parity(&exact[0].teacher_signature_sha256)
        .expect("physical member pool")
        .positives
        .into_iter()
        .next()
        .expect("same-law frame");
    let parity = state
        .runtime_parity_cases
        .get_mut(&same_law_frame.frame_id_sha256)
        .expect("same-law runtime parity");
    let mut budget_variant: serde_json::Value =
        serde_json::from_str(&parity.expected_response).expect("response json");
    budget_variant["arguments"]["yield_time_ms"] = serde_json::json!(30_000);
    parity.expected_response = serde_json::to_string(&budget_variant).expect("response encode");
    assert_eq!(
        state
            .classify_semantic_frame(&semantic, &same_law_frame, false)
            .0,
        SemanticEvidenceOutcome::VerifiedEquivalent,
        "a same-law execution-budget variant cannot become an anti-center"
    );
    let saved_parity = state
        .runtime_parity_cases
        .remove(&same_law_frame.frame_id_sha256)
        .expect("saved parity");
    assert_eq!(
        state
            .classify_semantic_frame(&semantic, &same_law_frame, false)
            .0,
        SemanticEvidenceOutcome::CensoredUnknown,
        "missing parity cannot update either phase"
    );
    state
        .runtime_parity_cases
        .insert(same_law_frame.frame_id_sha256.clone(), saved_parity);
    let parity = state
        .runtime_parity_cases
        .get_mut(&same_law_frame.frame_id_sha256)
        .expect("restored parity");
    let verified_equivalent_response = parity.expected_response.clone();
    parity.expected_response = r#"{"name":"different_action","arguments":{}}"#.to_owned();
    assert_eq!(
        state
            .classify_semantic_frame(&semantic, &same_law_frame, false)
            .0,
        SemanticEvidenceOutcome::HardContradiction,
        "same-law parity mismatch must remain visible"
    );
    let mut foreign_law_frame = same_law_frame.clone();
    foreign_law_frame.frame_id_sha256 = "f".repeat(64);
    foreign_law_frame
        .atoms
        .push(crate::RelationAtom::ActionStringArgument {
            name: "chars".to_owned(),
            value: "foreign-effect".to_owned(),
        });
    assert_ne!(
        crate::teacher_semantic_law_signature(&foreign_law_frame).as_deref(),
        Some(semantic.law_signature_sha256.as_str()),
        "fixture must represent a different effect law"
    );
    let mut foreign_parity = state
        .runtime_parity_cases
        .get(&same_law_frame.frame_id_sha256)
        .expect("same-law parity fixture")
        .clone();
    foreign_parity.evidence_ref_sha256 = foreign_law_frame.frame_id_sha256.clone();
    foreign_parity.expected_response =
        r#"{"name":"wait","arguments":{"cell_id":"handle-1000","chars":"foreign-effect"}}"#
            .to_owned();
    state
        .runtime_parity_cases
        .insert(foreign_law_frame.frame_id_sha256.clone(), foreign_parity);
    state.runtime_parity_frames.insert(
        foreign_law_frame.frame_id_sha256.clone(),
        foreign_law_frame.clone(),
    );
    assert_eq!(
        state
            .classify_semantic_frame(&semantic, &foreign_law_frame, false)
            .0,
        SemanticEvidenceOutcome::ApplicabilityNegative,
        "a different effect law with the same protocol shape must not poison the member law"
    );
    state
        .runtime_parity_cases
        .get_mut(&same_law_frame.frame_id_sha256)
        .expect("parity restore target")
        .expected_response = verified_equivalent_response;
    let classified = state
        .classified_cohort_pool(&semantic, &[])
        .expect("combined classified law pool");
    let outcome_counts = classified.evidence.iter().fold(
        BTreeMap::<SemanticEvidenceOutcome, usize>::new(),
        |mut counts, receipt| {
            *counts.entry(receipt.outcome).or_default() += 1;
            counts
        },
    );
    assert_eq!(
        outcome_counts
            .get(&SemanticEvidenceOutcome::HardContradiction)
            .copied()
            .unwrap_or(0),
        0
    );
    let pool = classified.pool;
    assert!(pool.positives.len() >= 64);
    assert!(
        pool.negatives.len() >= 16,
        "cross-law negatives missing: outcomes={outcome_counts:?}"
    );
    let positive_ids = pool
        .positives
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        pool.negatives
            .iter()
            .all(|frame| !positive_ids.contains(frame.frame_id_sha256.as_str())),
        "typed semantic evidence cannot train the same frame in both phases"
    );

    state.refresh_generations_filtered(None);
    let frozen = state.report(0);
    assert!(frozen.generations.iter().any(|generation| {
        generation.physical_adapter_count == 2
            && generation.support_rows == 32
            && generation.future_rows == 0
    }));
    let constrained_discovery = crate::FamilyDiscoveryConfig {
        positive_reservoir_rows: 1,
        ..crate::FamilyDiscoveryConfig::default()
    };
    state
        .discovery
        .enforce_runtime_limits(constrained_discovery);
    let retained_semantic = state
        .build_semantic_law_cohort(&law, &exact)
        .expect("generation-owned semantic support survives discovery eviction");
    assert_eq!(retained_semantic.member_signatures.len(), 2);
    let frozen_generation_ids = state.generations.keys().cloned().collect::<BTreeSet<_>>();
    let mut orphan = state
        .generations
        .values()
        .next()
        .cloned()
        .expect("frozen generation");
    orphan.cohort_id_sha256 = "orphan-semantic-cohort".to_owned();
    orphan.generation_id_sha256 = "orphan-generation".to_owned();
    state
        .generations
        .insert(orphan.cohort_id_sha256.clone(), orphan);
    state
        .dirty_generation_signatures
        .insert("unrelated-evidence-signature".to_owned());
    state.refresh_dirty_generation_evidence(None);
    assert_eq!(
        state.generations.keys().cloned().collect::<BTreeSet<_>>(),
        frozen_generation_ids
    );

    let frozen_cohort_id = state
        .generations
        .keys()
        .next()
        .cloned()
        .expect("frozen cohort");
    let support_session = state.generations[&frozen_cohort_id].support[0]
        .session_id_sha256
        .clone();
    let baseline_pending = state
        .report(0)
        .generations
        .into_iter()
        .find(|generation| generation.cohort_id_sha256 == frozen_cohort_id)
        .expect("baseline pending generation report");
    for index in 0..5 {
        let mut transition = continuation_transition(
            2_500 + index,
            "wait",
            "cell_id",
            "Script running with cell ID ",
            "wait in support session",
        );
        transition.before.session_id_sha256 = support_session.clone();
        state
            .observe_transition(&transition)
            .expect("observe same-session pending future");
    }
    let pending = state
        .report(0)
        .generations
        .into_iter()
        .find(|generation| generation.cohort_id_sha256 == frozen_cohort_id)
        .expect("pending generation report");
    assert_eq!(
        pending.after_future_watermark_rows,
        baseline_pending.after_future_watermark_rows + 5
    );
    assert_eq!(
        pending.support_session_rejects,
        baseline_pending.support_session_rejects + 5
    );

    let encoded = serde_json::to_vec(&state).expect("encode pending future state");
    let mut restored: StreamingSelfTrainingState =
        serde_json::from_slice(&encoded).expect("restore pending future state");
    restored.repair_missing_synthesis_state();
    let restored_pending = restored
        .report(0)
        .generations
        .into_iter()
        .find(|generation| generation.cohort_id_sha256 == frozen_cohort_id)
        .expect("restored pending generation report");
    assert_eq!(
        restored_pending.generation_id_sha256,
        pending.generation_id_sha256
    );
    assert_eq!(
        restored_pending.support_watermark_unix_nanos,
        pending.support_watermark_unix_nanos
    );
    assert_eq!(
        restored_pending.after_future_watermark_rows,
        pending.after_future_watermark_rows
    );
    assert_eq!(
        restored_pending.support_session_rejects,
        pending.support_session_rejects
    );
    assert_eq!(
        restored_pending.independent_future_rows,
        pending.independent_future_rows
    );

    for index in 0..16 {
        let transition = continuation_transition(
            3_000 + index,
            "wait",
            "cell_id",
            "Script running with cell ID ",
            "wait for script",
        );
        state
            .observe_transition(&transition)
            .expect("observe wait future");
    }
    for index in 0..16 {
        let transition = continuation_transition(
            4_000 + index,
            "continue_process",
            "session",
            "Process running with session ID ",
            "continue process",
        );
        state
            .observe_transition(&transition)
            .expect("observe process future");
    }
    assert!(!state.dirty_generation_signatures.is_empty());
    state.rebuild_queue.extend([
        "unrelated-pending-a".to_owned(),
        "unrelated-pending-b".to_owned(),
    ]);
    state.run_work_slice();
    assert!(!state.rebuild_queue.is_empty());
    let report = state.report(0);
    assert_eq!(report.semantic_law_cohorts, 1);
    assert_eq!(report.semantic_law_physical_adapters, 2);
    assert!(report.semantic_law_blockers.is_empty());
    assert!(report.generations.iter().any(|generation| {
        generation.physical_adapter_count == 2
            && generation.support_rows == 32
            && generation.future_rows >= 32
            && generation.blocker.is_none()
    }));
    assert_eq!(report.admission_ready_cohorts, 1);
    let admission = state.admission_cohorts();
    assert_eq!(admission.len(), 1);
    let positive_ids = admission[0]
        .pool
        .positives
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        admission[0]
            .pool
            .negatives
            .iter()
            .all(|frame| !positive_ids.contains(frame.frame_id_sha256.as_str())),
        "semantic evidence cannot be both positive and negative"
    );
}

#[test]
fn semantic_law_winner_owns_every_verified_physical_adapter() {
    let mut state = StreamingSelfTrainingState::new(0);
    for index in 0..16 {
        let negative = continuation_frame(
            index,
            "cancel",
            "handle",
            "Cancelled handle ",
            "negative",
            false,
        );
        let mut transition = crate::teacher_transition_from_completed(&negative, None)
            .expect("negative teacher transition");
        transition.runtime_parity_case = continuation_transition(
            index,
            "cancel",
            "handle",
            "Cancelled handle ",
            "cancel handle",
        )
        .runtime_parity_case;
        state
            .observe_transition(&transition)
            .expect("observe negative");
    }
    for index in 0..32 {
        state
            .observe_transition(&continuation_transition(
                10_000 + index,
                "wait",
                "cell_id",
                "Script running with cell ID ",
                "wait for script",
            ))
            .expect("observe wait adapter");
    }
    for index in 0..33 {
        state
            .observe_transition(&continuation_transition(
                20_000 + index,
                "write_stdin",
                "session_id",
                "Script running with session ID ",
                "continue session",
            ))
            .expect("observe write_stdin adapter");
    }
    for _ in 0..2_048 {
        if state.run_work_slice() == 0 && !state.has_pending_work() {
            break;
        }
    }

    let exact = state
        .cegis
        .winners()
        .into_iter()
        .filter(|winner| {
            matches!(
                winner.action_symbol.as_str(),
                "function:wait" | "function:write_stdin"
            )
        })
        .collect::<Vec<_>>();
    assert!(exact.len() >= 2, "missing physical adapters: {exact:?}");
    let law = state
        .discovery
        .semantic_law_signature(&exact[0].teacher_signature_sha256)
        .expect("shared semantic law");
    assert!(
        exact.iter().all(|winner| {
            state
                .discovery
                .semantic_law_signature(&winner.teacher_signature_sha256)
                .as_deref()
                == Some(law.as_str())
        }),
        "physical adapters did not join one semantic law"
    );
    let semantic = state
        .build_semantic_law_cohort(&law, &exact)
        .unwrap_or_else(|blocker| panic!("semantic law cohort: {blocker}"));
    let classified = state
        .classified_cohort_pool(&semantic, &[])
        .expect("classified semantic law");
    let contradictions = classified
        .evidence
        .iter()
        .filter(|row| row.outcome == SemanticEvidenceOutcome::HardContradiction)
        .count();

    assert_eq!(contradictions, 0, "winner dropped a physical adapter");
    assert_eq!(
        semantic.physical_adapter_count,
        semantic.member_signatures.len(),
        "reported adapters must be the member-law signatures"
    );
    assert!(semantic.physical_adapter_count >= 2);
    assert!(matches!(
        semantic.winner.program.operation,
        crate::ResponseOperation::UniqueConsensus { .. }
    ));
}

#[test]
fn semantic_actor_rejects_threshold_only_partial_member_coverage() {
    let wait = continuation_transition(
        30_000,
        "wait",
        "cell_id",
        "Script running with cell ID ",
        "wait for script",
    );
    let write_stdin = continuation_transition(
        40_000,
        "write_stdin",
        "session_id",
        "Script running with session ID ",
        "continue session",
    );
    let wait_program = crate::ResponseProgram::function_call_from_roles(
        "wait",
        crate::ResponseValueSelector::ContentLinePrefix {
            prefix: "Script running with cell ID ".to_owned(),
            value_type: crate::AtomValueType::Identifier,
        },
        vec![crate::ResponseArgument::Role {
            name: "cell_id".to_owned(),
            role: crate::SemanticRole::ContinuationHandle,
            value_type: Some(crate::AtomValueType::Identifier),
        }],
    );
    let wait_parity = wait.runtime_parity_case.as_ref().expect("wait parity");
    let write_stdin_parity = write_stdin
        .runtime_parity_case
        .as_ref()
        .expect("write_stdin parity");
    let mut member_parity = vec![wait_parity; 32];
    member_parity.push(write_stdin_parity);

    assert!(semantic_program_matches_runtime_parity(
        &wait_program,
        wait_parity
    ));
    assert!(!semantic_program_matches_runtime_parity(
        &wait_program,
        write_stdin_parity
    ));
    assert!(
        !semantic_program_covers_all_runtime_parity(&wait_program, &member_parity),
        "32 passing rows cannot hide one dropped member adapter"
    );
}
