use super::*;
use crate::{
    AtomSource, AtomValueType, CaptureEvidenceReceipt, CaptureRecordCommitment,
    CaptureTransitionBinding, RELATION_FRAME_SCHEMA, RelationAtom, ResponseValueSelector,
    SOURCE_NEUTRAL_EXTRACTOR_VERSION,
};

fn frame(index: usize, action: &str, accepted: bool) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: format!("{index:064x}"),
        event_id_sha256: format!("{:064x}", index + 1),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: format!("{:064x}", index % 4),
        observed_at_unix_nanos: index as u64,
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(accepted),
        atoms: vec![
            RelationAtom::ToolKind {
                value: "exec".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "a".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Identifier,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "a".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::ActionFunction {
                value: action.to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 2,
                value_type: None,
            },
        ],
        evidence_ref_sha256: format!("{:064x}", index + 10_000),
    }
}

fn plan_frame(index: usize) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: format!("{:064x}", index + 50_000),
        event_id_sha256: format!("{:064x}", index + 60_000),
        client_intent_id_sha256: format!("{:064x}", index + 70_000),
        session_id_sha256: format!("{:064x}", index % 4 + 80_000),
        observed_at_unix_nanos: index as u64,
        estimated_input_tokens: 500,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
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
        evidence_ref_sha256: format!("{:064x}", index + 90_000),
    }
}

fn plan_parity_case(index: usize) -> crate::RuntimeParityCase {
    // Replay support is valid only when the production actor can execute
    // the captured before-state and reproduce the completed teacher call.
    let plan = serde_json::json!([
        {"step": "inspect", "status": "in_progress"},
        {"step": "repair", "status": "pending"},
        {"step": "verify", "status": "pending"}
    ]);
    let provider_payload = serde_json::json!({
        "input": [
            {
                "type": "function_call",
                "name": "update_plan",
                "arguments": {"plan": plan}
            },
            {
                "type": "function_call_output",
                "output": {"ok": true}
            }
        ]
    });
    let expected_response = serde_json::json!({
        "name": "update_plan",
        "arguments": {
            "plan": [
                {"step": "inspect", "status": "completed"},
                {"step": "repair", "status": "in_progress"},
                {"step": "verify", "status": "pending"}
            ]
        }
    })
    .to_string();
    crate::RuntimeParityCase {
        evidence_ref_sha256: format!("{:064x}", index + 90_000),
        capture_receipt: None,
        request_text: String::new(),
        provider_payload,
        expected_response,
    }
}

fn write_stdin_parity_case(index: usize, prefix: &str) -> crate::RuntimeParityCase {
    // Model the real continuation surface; an empty payload would create
    // a receipt that cannot prove runtime parity.
    crate::RuntimeParityCase {
        evidence_ref_sha256: String::new(),
        capture_receipt: None,
        request_text: String::new(),
        provider_payload: serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("{prefix}handle-{index}")
            }]
        }),
        expected_response: serde_json::json!({
            "name": "write_stdin",
            "arguments": {"session_id": format!("handle-{index}")}
        })
        .to_string(),
    }
}

fn bind_parity_to_capture(
    parity: &mut crate::RuntimeParityCase,
    capture_frame_id_sha256: &str,
    sequence: u64,
) {
    let record = CaptureRecordCommitment {
        sequence,
        record_sha256: format!("{:064x}", sequence + 100_000),
    };
    let mut receipt = CaptureEvidenceReceipt::new(vec![record]).expect("capture receipt");
    let binding = CaptureTransitionBinding::new(sequence, capture_frame_id_sha256, &receipt)
        .expect("binding");
    receipt.bind_transition(binding).expect("bind transition");
    parity.evidence_ref_sha256 = capture_frame_id_sha256.to_owned();
    parity.capture_receipt = Some(receipt);
}

#[test]
fn duplicate_frame_upgrades_only_durable_capture_provenance() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    let completed = frame(777, "write_stdin", true);
    let capture_frame_id_sha256 = completed.frame_id_sha256.clone();
    let mut unbound =
        crate::teacher_transition_from_completed(&completed, None).expect("unbound transition");
    unbound.runtime_parity_case =
        Some(write_stdin_parity_case(777, "Script running with cell ID "));
    miner
        .observe_teacher_transition(unbound.clone())
        .expect("general frame");
    assert_eq!(miner.report().live_scalar_shadow.support_rows, 1);

    let mut bound = unbound;
    bind_parity_to_capture(
        bound.runtime_parity_case.as_mut().expect("parity"),
        &capture_frame_id_sha256,
        777,
    );
    let receipt = bound
        .runtime_parity_case
        .as_ref()
        .and_then(|parity| parity.capture_receipt.as_ref())
        .expect("bound receipt");
    let binding = receipt.transition_binding.as_ref().expect("bound binding");
    receipt.validate().expect("valid receipt");
    bound
        .verify_capture_frame_id(&binding.frame_id_sha256)
        .expect("transition bound to capture frame");
    miner
        .observe_teacher_transition(bound)
        .expect("bound duplicate");

    let report = miner.report();
    assert_eq!(report.rows_seen, 1, "{report:#?}");
    assert_eq!(report.live_scalar_shadow.support_rows, 1, "{report:#?}");
    assert_eq!(report.live_scalar_shadow.duplicate_rows, 1, "{report:#?}");
    assert_eq!(
        report
            .live_scalar_shadow
            .blockers
            .get("capture_lineage_evidence_empty:support=1:future=0"),
        Some(&1),
        "{report:#?}"
    );
}

#[test]
fn replay_imports_only_capture_bound_rows_as_support_never_future() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    let completed = frame(778, "write_stdin", true);
    let frame_id_sha256 = completed.frame_id_sha256.clone();
    let mut unbound =
        crate::teacher_transition_from_completed(&completed, None).expect("unbound transition");
    unbound.runtime_parity_case =
        Some(write_stdin_parity_case(778, "Script running with cell ID "));

    miner
        .import_teacher_transition(unbound.clone())
        .expect("unbound replay");
    let unbound_report = miner.report().live_scalar_shadow;
    assert_eq!(unbound_report.support_rows, 0, "{unbound_report:#?}");
    assert_eq!(unbound_report.future_rows, 0, "{unbound_report:#?}");

    let mut capture_bound = unbound;
    bind_parity_to_capture(
        capture_bound
            .runtime_parity_case
            .as_mut()
            .expect("runtime parity"),
        &frame_id_sha256,
        778,
    );
    miner
        .import_teacher_transition(capture_bound)
        .expect("capture-bound replay");

    let report = miner.report().live_scalar_shadow;
    assert_eq!(report.support_rows, 1, "{report:#?}");
    assert_eq!(report.future_rows, 0, "{report:#?}");
    assert_eq!(report.transfer_proofs, 0, "{report:#?}");
    assert_eq!(report.admission_candidates, 0, "{report:#?}");
}

#[test]
fn online_checkpoint_has_one_process_owner() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-checkpoint-owner-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let config = OnlineResponseTailConfig {
        input_path: root.join("relations.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };

    let owner = OnlineResponseStream::open_streaming(config.clone()).expect("first owner");
    let error = OnlineResponseStream::open_streaming(config.clone())
        .err()
        .expect("second owner must fail closed");
    assert!(error.starts_with("online_checkpoint_owned:"), "{error}");

    drop(owner);
    OnlineResponseStream::open_streaming(config).expect("owner released after drop");
}

#[test]
fn plan_teacher_frame_reaches_a_synthesizable_online_bucket() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    miner
        .observe_frame(plan_frame(1))
        .expect("plan frame ingest");
    let report = miner.report();
    assert_eq!(report.rows_ambiguous, 0);
    assert_eq!(report.bucket_count, 2);
    assert_eq!(report.candidates.len(), 0);
    assert_eq!(report.buckets.len(), 2);
    assert!(
        report
            .buckets
            .iter()
            .all(|bucket| { bucket.synthesized_operation.as_deref() == Some("advance_plan") })
    );
}

#[test]
fn restored_broad_teacher_bucket_accumulates_across_structural_surfaces() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    for index in 1..=64 {
        let mut row = frame(index, "write_stdin", true);
        row.atoms.push(RelationAtom::ClientCapabilityAtom {
            atom_id: if index % 2 == 0 { 100 } else { 200 },
        });
        miner.observe_frame(row).expect("teacher row");
    }

    let signature =
        teacher_program_signature(&frame(1, "write_stdin", true)).expect("teacher signature");
    let broad_family =
        stable_restored_family_id("broad_action", "function:write_stdin", &signature, &[]);
    let broad_id = stable_bucket_id(broad_family, &signature);
    let broad = miner.buckets.get(&broad_id).expect("restored broad bucket");
    assert_eq!(broad.positive_rows, 64);
    assert_eq!(broad.positives.len(), 32);
    assert_eq!(broad.future_positives.len(), 32);
    let structural_surfaces = [100, 200].map(|atom_id| {
        let mut row = frame(1000 + atom_id as usize, "write_stdin", true);
        row.atoms
            .push(RelationAtom::ClientCapabilityAtom { atom_id });
        online_bucket_identity(&row)
            .map(|identity| identity.0)
            .expect("structural surface")
    });
    for structural_family in structural_surfaces {
        let structural_id = stable_bucket_id(structural_family, &signature);
        let structural = miner
            .buckets
            .get(&structural_id)
            .expect("structural bucket");
        assert_eq!(structural.positives.len(), 32);
        assert_eq!(structural.future_positives.len(), 0);
    }
}

#[test]
fn restored_core_compiles_provisional_candidate_before_admission_gate() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("restored miner");
    for index in 1..=16 {
        miner
            .observe_frame(frame(index, "write_stdin", true))
            .expect("positive teacher row");
    }
    for index in 100..104 {
        let mut negative = frame(index, "exec_command", true);
        negative.atoms[0] = RelationAtom::ToolKind {
            value: "exec_command".to_owned(),
        };
        miner
            .observe_frame(negative)
            .expect("competing teacher row");
    }

    let report = miner.report();
    assert!(report.active_bucket_count >= 1);
    assert!(report.candidates.iter().any(|candidate| {
        candidate.teacher_signature_sha256
            == teacher_program_signature(&frame(1, "write_stdin", true)).expect("teacher signature")
    }));
    assert_eq!(report.admission_ready_cohorts, 0);
    assert_eq!(report.emitted_candidate_cohorts, 0);
    assert!(
        report
            .buckets
            .iter()
            .filter(|bucket| bucket.teacher_action_symbol == "function:write_stdin")
            .all(|bucket| bucket.frozen_future_rows == 0)
    );
}

#[test]
fn restored_core_emits_a_verifier_bound_admission_candidate() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        reservoir_rows: 32,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 1..=64 {
        let mut source = frame(index, "write_stdin", true);
        source.client_intent_id_sha256 = format!("{:064x}", index + 500_000);
        source.session_id_sha256 = format!("{:064x}", index + 600_000);
        source
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
        source.atoms.push(RelationAtom::ObservationSelector {
            slot_id: 1,
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "Process running with session ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
        });
        source.atoms.push(RelationAtom::ClientCapabilityAtom {
            atom_id: if index % 2 == 0 { 100 } else { 200 },
        });
        let evidence_ref_sha256 = source.evidence_ref_sha256.clone();
        let mut transition =
            crate::teacher_transition_from_completed(&source, None).expect("teacher transition");
        let provider_payload = serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("Process running with session ID handle-{index}")
            }]
        });
        let training_frame = transition.as_training_relation_frame();
        let program = crate::synthesize_response_operator(&[training_frame])
            .expect("typed program")
            .candidate
            .program;
        let execution = crate::execute_response(&program, "", &provider_payload);
        assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
        transition.runtime_parity_case = Some(crate::RuntimeParityCase {
            evidence_ref_sha256,
            capture_receipt: None,
            request_text: String::new(),
            provider_payload,
            expected_response: execution.response.expect("exact response"),
        });
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    for index in 100..104 {
        let mut negative = frame(index, "exec_command", true);
        negative.atoms[0] = RelationAtom::ToolKind {
            value: "exec_command".to_owned(),
        };
        miner.observe_frame(negative).expect("negative teacher row");
    }

    let evaluation = miner.restored_core_admission_evaluation();
    assert!(evaluation.ready_cohorts >= 1);
    assert_eq!(
        evaluation.ready_cohorts,
        evaluation
            .candidates
            .len()
            .saturating_add(evaluation.blockers.len())
    );
    assert!(evaluation.candidates.is_empty());
    assert!(evaluation.blockers.iter().any(|blocker| {
        blocker
            .blocker
            .starts_with("legacy_control_receipt_backed_partition_below_32:")
    }));
    assert_eq!(miner.report().false_accepts, 0);
}

#[test]
fn proven_subcenter_trains_real_negative_before_calibration() {
    let guard_atom = crate::package::stable_atom_id("test:clean-subcenter");
    let support = (1..=32)
        .map(|index| {
            let mut frame = frame(index, "write_stdin", true);
            frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: guard_atom,
            });
            frame
        })
        .collect::<Vec<_>>();
    let future = (33..=64)
        .map(|index| {
            let mut frame = frame(index, "write_stdin", true);
            frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: guard_atom,
            });
            frame
        })
        .collect::<Vec<_>>();
    let mut negative = frame(100, "exec_command", false);
    negative.atoms[0] = RelationAtom::ToolKind {
        value: "network".to_owned(),
    };
    let synthesized = synthesize_response_operator(&support).expect("program");
    let bucket = ResponseBucket {
        structural_family_id: 1,
        teacher_signature_sha256: "d".repeat(64),
        teacher_action_symbol: "function:write_stdin".to_owned(),
        positives: support
            .iter()
            .cloned()
            .map(SharedRelationFrame::new)
            .collect(),
        negatives: VecDeque::from([SharedRelationFrame::new(negative.clone())]),
        future_positives: future
            .iter()
            .cloned()
            .map(SharedRelationFrame::new)
            .collect(),
        future_negatives: VecDeque::new(),
        positive_rows: 64,
        negative_rows: 1,
        positive_tokens: 6_400,
        negative_tokens: 100,
        first_false_accept_frame_id: None,
        cardinality_guard_rejects: 0,
        positive_cardinality_bounds: BTreeMap::new(),
        positive_cardinality_signatures: BTreeSet::new(),
        support_watermark_event_time_unix_nanos: 32,
        late_or_missing_time_rows: 0,
        exact_guard_atom_ids: Vec::new(),
    };
    let candidate = build_subcenter_admission_candidate(
        OnlineResponseMinerConfig::default(),
        &bucket,
        &[guard_atom],
        support.clone(),
        future.clone(),
        vec![negative.clone()],
        Some(ProvenAdmissionProgram {
            program: &synthesized.candidate.program,
            phase_rank: synthesized.candidate.phase_rank,
            exact_checks: synthesized.candidate.exact_checks,
        }),
    )
    .expect("subcenter candidate");
    assert_eq!(candidate.required_routing_atom_ids, [guard_atom]);
    assert!(!candidate.wave_runtime_package.is_empty());
}

#[test]
fn frozen_admission_guard_repair_adds_clean_atom_without_repartitioning_future() {
    let base_atom = crate::package::stable_atom_id("test:base-guard");
    let clean_atom = crate::package::stable_atom_id("test:clean-guard");
    let add_atoms = |mut frame: RelationFrame| {
        frame
            .atoms
            .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
        frame.atoms.push(RelationAtom::ClientCapabilityAtom {
            atom_id: clean_atom,
        });
        frame
    };
    let support = (1..=32)
        .map(|index| add_atoms(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let future = (33..=64)
        .map(|index| add_atoms(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let mut negative = support[0].clone();
    negative.frame_id_sha256 = format!("{:064x}", 100_000);
    negative.event_id_sha256 = format!("{:064x}", 100_001);
    negative.session_id_sha256 = format!("{:064x}", 100_002);
    negative.observed_at_unix_nanos = 100_000;
    negative.verifier_label = Some(false);
    negative.atoms.retain(|atom| {
        !matches!(atom, RelationAtom::ClientCapabilityAtom { atom_id } if *atom_id == clean_atom)
    });
    let program = synthesize_response_operator(&support)
        .expect("program")
        .candidate
        .program;
    let negative_atoms = relation_frame_online_routing_atom_ids(&negative);

    let (required, repaired_support, repaired_future) =
        repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
            .expect("clean exact guard");
    assert!(required.contains(&base_atom));
    assert!(required.len() > 1);
    assert!(
        !required
            .iter()
            .all(|atom| negative_atoms.binary_search(atom).is_ok())
    );
    assert_eq!(
        repaired_support
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<Vec<_>>(),
        support
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        repaired_future
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<Vec<_>>(),
        future
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn frozen_admission_guard_delegates_unseparable_negative_to_wave() {
    let base_atom = crate::package::stable_atom_id("test:base-guard");
    let add_atom = |mut frame: RelationFrame| {
        frame
            .atoms
            .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
        frame
    };
    let support = (1..=32)
        .map(|index| add_atom(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let future = (33..=64)
        .map(|index| add_atom(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let negative = add_atom(frame(100, "exec_command", false));
    let program = synthesize_response_operator(&support)
        .expect("program")
        .candidate
        .program;
    let (required, repaired_support, repaired_future) =
        repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
            .expect("anti-center delegation");
    assert_eq!(required, [base_atom]);
    assert_eq!(repaired_support.len(), 32);
    assert_eq!(repaired_future.len(), 32);
}

#[test]
fn frozen_admission_guard_ignores_negative_where_actor_cannot_run() {
    let base_atom = crate::package::stable_atom_id("test:base-guard");
    let add_atom = |mut frame: RelationFrame| {
        frame
            .atoms
            .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
        frame
    };
    let support = (1..=32)
        .map(|index| add_atom(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let future = (33..=64)
        .map(|index| add_atom(frame(index, "write_stdin", true)))
        .collect::<Vec<_>>();
    let program = synthesize_response_operator(&support)
        .expect("program")
        .candidate
        .program;
    let mut negative = add_atom(frame(100, "exec_command", false));
    negative.atoms.retain(|atom| {
        !matches!(
            atom,
            RelationAtom::ObservationSelector { .. } | RelationAtom::UniqueSlot { .. }
        )
    });

    let (required, repaired_support, repaired_future) =
        repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
            .expect("actor abstain makes routed negative harmless");
    assert_eq!(required, [base_atom]);
    assert_eq!(repaired_support.len(), 32);
    assert_eq!(repaired_future.len(), 32);
}

#[test]
fn reconstructed_support_capability_matches_observed_future_family() {
    let capability = crate::package::stable_atom_id("client_capability:function:wait");
    let mut historical = frame(1, "wait", true);
    reconstruct_online_client_capability(&mut historical);
    assert!(historical.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ReconstructedClientCapabilityAtom { atom_id }
                if *atom_id == capability
        )
    }));

    let mut future = frame(2, "wait", true);
    future.atoms.push(RelationAtom::ClientCapabilityAtom {
        atom_id: capability,
    });
    reconstruct_online_client_capability(&mut future);
    assert!(
        !future
            .atoms
            .iter()
            .any(|atom| { matches!(atom, RelationAtom::ReconstructedClientCapabilityAtom { .. }) })
    );
    assert_eq!(
        online_bucket_identity(&historical).map(|value| value.0),
        online_bucket_identity(&future).map(|value| value.0)
    );
}

#[test]
fn online_response_miner_learns_before_update_and_bounds_support() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        reservoir_rows: 4,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 0..12 {
        miner
            .observe_frame(frame(index, "write_stdin", true))
            .expect("observe");
    }
    for index in 12..18 {
        miner
            .observe_frame(frame(index, "other_action", true))
            .expect("competing action");
    }
    let report = miner.report();
    assert_eq!(report.rows_seen, 18);
    assert_eq!(report.rows_learned, 18);
    assert!(report.competing_negative_updates > 0);
    assert_eq!(report.false_accepts, 0);
    assert!(
        miner
            .buckets
            .values()
            .all(|bucket| { bucket.positives.len() <= 4 && bucket.negatives.len() <= 4 })
    );
}

#[test]
fn exact_support_guard_rejects_a_foreign_preaction_surface() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        reservoir_rows: 4,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 0..4 {
        miner
            .train_frame(frame(index, "wait", true))
            .expect("support");
    }
    let bucket = miner.buckets.values().next().expect("bucket");
    let mut foreign = frame(10, "write_stdin", true);
    foreign.atoms[0] = RelationAtom::ToolKind {
        value: "exec_command".to_owned(),
    };
    assert!(!cardinality_guard_matches(bucket, &foreign));
}

#[test]
fn grounded_program_family_merges_surface_shapes_without_merging_programs() {
    let mut first = frame(100, "write_stdin", true);
    first.atoms.insert(
        0,
        RelationAtom::ObservationCallShape {
            value: "surface_a".to_owned(),
        },
    );
    first.atoms.push(RelationAtom::ActionIntegerArgument {
        name: "max_output_tokens".to_owned(),
        value: 3_000,
    });
    let mut second = frame(101, "write_stdin", true);
    second.atoms.insert(
        0,
        RelationAtom::ObservationCallShape {
            value: "surface_b".to_owned(),
        },
    );
    second.atoms.push(RelationAtom::ActionIntegerArgument {
        name: "max_output_tokens".to_owned(),
        value: 12_000,
    });
    assert_ne!(
        crate::relation_frame_structural_family_id(&first),
        crate::relation_frame_structural_family_id(&second)
    );
    assert_eq!(
        online_bucket_identity(&first),
        online_bucket_identity(&second)
    );

    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    miner.train_frame(first).expect("first surface");
    miner.train_frame(second).expect("second surface");
    assert_eq!(miner.report().bucket_count, 2);

    let different_program = frame(102, "exec_command", true);
    assert_ne!(
        online_bucket_identity(
            miner
                .buckets
                .values()
                .next()
                .and_then(|bucket| bucket.positives.front())
                .expect("support frame")
        ),
        online_bucket_identity(&different_program)
    );
}

#[test]
fn online_response_miner_dedupes_metadata_enrichment_and_rejects_semantic_conflicts() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
    let original = frame(1, "write_stdin", true);
    miner.observe_frame(original.clone()).expect("first");
    miner
        .observe_frame(original.clone())
        .expect("duplicate no-op");
    assert_eq!(miner.report().rows_seen, 1);

    let mut enriched = original.clone();
    enriched.estimated_input_tokens = enriched.estimated_input_tokens.saturating_add(1);
    enriched.observed_at_unix_nanos = enriched.observed_at_unix_nanos.saturating_add(1);
    enriched.evidence_ref_sha256 = "new-receipt-for-the-same-transition".to_owned();
    enriched.client_intent_id_sha256 = "enriched-client-intent".to_owned();
    enriched.session_id_sha256 = "enriched-session-lineage".to_owned();
    enriched.extractor_version = "enriched-extractor-provenance".to_owned();
    miner
        .observe_frame(enriched)
        .expect("metadata and receipt enrichment is a duplicate");
    assert_eq!(miner.report().rows_seen, 1);

    let mut family_miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("family miner");
    family_miner
        .observe_teacher_transition(
            crate::teacher_transition_from_completed(&original, None).expect("transition"),
        )
        .expect("first family transition");
    family_miner.seen_frame_sha256.clear();
    let mut new_receipt = original.clone();
    new_receipt.evidence_ref_sha256 = "new-family-receipt".to_owned();
    new_receipt.client_intent_id_sha256 = "new-family-intent".to_owned();
    new_receipt.session_id_sha256 = "new-family-session".to_owned();
    family_miner
        .observe_teacher_transition(
            crate::teacher_transition_from_completed(&new_receipt, None)
                .expect("enriched transition"),
        )
        .expect("family receipt enrichment is a duplicate");
    assert_eq!(
        family_miner
            .report()
            .self_training_v2
            .discovery
            .duplicate_rows,
        1
    );

    let mut conflict = original;
    conflict
        .atoms
        .push(RelationAtom::RequestPhaseAtom { atom_id: 999 });
    assert_eq!(
        miner.observe_frame(conflict),
        Err("online_frame_id_content_conflict".to_owned())
    );
    assert_eq!(miner.report().rows_seen, 1);
}

#[test]
fn historical_training_does_not_claim_future_accepts() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 0..20 {
        miner
            .train_frame(frame(index, "write_stdin", index % 3 != 0))
            .expect("train");
    }
    let report = miner.report();
    assert_eq!(report.false_accepts, 0);
    assert_eq!(report.candidate_bucket_count, 0);
}

#[test]
fn response_checkpoint_restores_wave_and_bounded_synthesis_state() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        reservoir_rows: 4,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 0..18 {
        miner
            .observe_frame(frame(index, "write_stdin", index % 5 != 0))
            .expect("observe");
    }
    let before = miner.report();
    let checkpoint = miner.checkpoint(11, 22, 33, 18, 1).expect("checkpoint");
    let encoded = serde_json::to_vec(&checkpoint).expect("checkpoint encoding");
    let decoded = serde_json::from_slice(&encoded).expect("checkpoint decoding");
    let restored = OnlineResponseMiner::from_checkpoint(decoded).expect("restore");
    assert_eq!(restored.report(), before);
    assert!(
        restored
            .buckets
            .values()
            .all(|bucket| bucket.positives.len() <= 4 && bucket.negatives.len() <= 4)
    );
}

#[test]
fn online_response_miner_freezes_support_before_collecting_future() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        reservoir_rows: 32,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in 0..64 {
        miner
            .replay_chronological_frame(frame(index, "write_stdin", true))
            .expect("chronological replay");
    }
    let signature =
        teacher_program_signature(&frame(0, "write_stdin", true)).expect("teacher signature");
    let broad_family =
        stable_restored_family_id("broad_action", "function:write_stdin", &signature, &[]);
    let bucket = miner
        .buckets
        .get(&stable_bucket_id(broad_family, &signature))
        .expect("broad bucket");
    let support_ids = bucket
        .positives
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let future_ids = bucket
        .future_positives
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(support_ids.len(), 32);
    assert_eq!(future_ids.len(), 32);
    assert!(support_ids.is_disjoint(&future_ids));
}

#[test]
fn online_response_miner_rejects_late_or_missing_event_time_from_future() {
    let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
        min_bucket_events: 2,
        calibration_events: 2,
        reservoir_rows: 2,
        ..OnlineResponseMinerConfig::default()
    })
    .expect("miner");
    for index in [10, 11] {
        miner
            .replay_chronological_frame(frame(index, "write_stdin", true))
            .expect("support");
    }
    miner
        .observe_frame(frame(5, "write_stdin", true))
        .expect("late event");
    let bucket = miner.buckets.values().next().expect("bucket");
    assert!(bucket.future_positives.is_empty());
    assert_eq!(bucket.late_or_missing_time_rows, 1);

    miner
        .observe_frame(frame(12, "write_stdin", true))
        .expect("future event");
    let bucket = miner.buckets.values().next().expect("bucket");
    assert_eq!(bucket.future_positives.len(), 1);
}

#[test]
fn frozen_future_reservoir_retains_new_sessions() {
    let mut rows = VecDeque::new();
    for index in 0..32 {
        let mut row = frame(index, "write_stdin", true);
        row.session_id_sha256 = "a".repeat(64);
        push_session_diverse_future(&mut rows, SharedRelationFrame::new(row), 32);
    }
    for (index, session) in ['b', 'c'].into_iter().enumerate() {
        let mut row = frame(100 + index, "write_stdin", true);
        row.session_id_sha256 = session.to_string().repeat(64);
        push_session_diverse_future(&mut rows, SharedRelationFrame::new(row), 32);
    }

    assert_eq!(rows.len(), 32);
    assert_eq!(
        rows.iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
}

#[test]
fn response_stream_restarts_at_checkpoint_and_ingests_each_frame_once() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-stream-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    {
        let mut audit = File::create(&config.input_path).expect("audit");
        for index in 0..10 {
            serde_json::to_writer(&mut audit, &frame(index, "write_stdin", true))
                .expect("frame encoding");
            audit.write_all(b"\n").expect("newline");
        }
    }
    let mut stream = OnlineResponseStream::open(config.clone()).expect("initial stream");
    assert_eq!(stream.report().rows_seen, 10);
    assert_eq!(
        stream
            .ingest(frame(10, "write_stdin", true))
            .expect("first streamed frame")
            .rows_seen,
        11
    );
    stream.persist().expect("persist");
    drop(stream);

    let mut restored = OnlineResponseStream::open(config.clone()).expect("restored stream");
    assert_eq!(restored.report().rows_seen, 11);
    assert_eq!(
        restored
            .ingest(frame(11, "write_stdin", true))
            .expect("second streamed frame")
            .rows_seen,
        12
    );
    let line_count = BufReader::new(File::open(&config.input_path).expect("audit read"))
        .lines()
        .count();
    assert_eq!(line_count, 12);
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn teacher_transition_is_idempotent_across_checkpoint_restart() {
    let root = std::env::temp_dir().join(format!(
        "nando-teacher-restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    File::create(&config.input_path).expect("empty audit");
    let transition = crate::teacher_transition_from_completed(&frame(1, "write_stdin", true), None)
        .expect("teacher transition");
    {
        let mut stream = OnlineResponseStream::open_streaming(config.clone()).expect("stream");
        stream
            .apply_teacher_transition(transition.clone())
            .expect("first transition");
        stream.persist_now().expect("checkpoint");
        assert_eq!(stream.report().rows_seen, 1);
        assert_eq!(stream.report().bucket_count, 2);
    }

    let mut restored =
        OnlineResponseStream::open_streaming(config.clone()).expect("restored stream");
    let shadow_before_duplicate = restored.report().live_scalar_shadow;
    restored
        .apply_teacher_transition(transition)
        .expect("duplicate transition after restart");
    assert_eq!(restored.report().rows_seen, 1);
    assert_eq!(restored.report().bucket_count, 2);
    assert_eq!(
        restored.report().live_scalar_shadow,
        shadow_before_duplicate
    );
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn v43_teacher_pools_seed_wave_support_without_future_claims() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut transition =
            crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                .expect("teacher transition");
        transition.runtime_parity_case = Some(write_stdin_parity_case(
            index,
            "Process running with session ID ",
        ));
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    checkpoint.bucket_strategy_version = 43;

    let restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
    let report = restored.report();
    assert!(report.bucket_count > 0);
    assert!(
        report
            .buckets
            .iter()
            .all(|bucket| bucket.frozen_future_rows == 0)
    );
}

#[test]
fn v74_migration_preserves_shadow_support_without_future_claims() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut transition =
            crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                .expect("teacher transition");
        transition.before.session_id_sha256 = format!("{:064x}", index + 1_000);
        transition.runtime_parity_case = Some(write_stdin_parity_case(
            index,
            "Script running with cell ID ",
        ));
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    let before = miner.report().live_scalar_shadow;
    assert!(before.support_rows > 0, "{before:#?}");
    assert!(before.support_rows <= 64, "{before:#?}");
    assert_eq!(before.future_rows, 0, "{before:#?}");

    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    checkpoint.bucket_strategy_version = 74;
    let restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
    let after = restored.report().live_scalar_shadow;
    assert_eq!(after.support_rows, before.support_rows, "{after:#?}");
    assert_eq!(after.future_rows, 0, "{after:#?}");
}

#[test]
fn v96_migration_starts_fresh_archive_backed_scalar_generation() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut transition =
            crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                .expect("teacher transition");
        transition.before.session_id_sha256 = format!("{:064x}", index + 3_000);
        transition.runtime_parity_case = Some(write_stdin_parity_case(
            index,
            "Script running with cell ID ",
        ));
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    let before = miner.report().live_scalar_shadow;
    assert_eq!(before.support_rows, 40, "{before:#?}");
    assert_eq!(before.future_rows, 0, "{before:#?}");
    let teacher_pools_before = miner.self_training_v2.teacher_pool_count();

    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    checkpoint.bucket_strategy_version = 96;
    checkpoint.live_scalar_generation_version = 0;
    let mut restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
    let after = restored.report().live_scalar_shadow;
    assert_eq!(after.support_rows, 0, "{after:#?}");
    assert_eq!(after.future_rows, 0, "{after:#?}");
    assert_eq!(
        restored.self_training_v2.teacher_pool_count(),
        teacher_pools_before
    );

    let mut fresh =
        crate::teacher_transition_from_completed(&frame(100, "write_stdin", true), None)
            .expect("fresh transition");
    fresh.before.session_id_sha256 = format!("{:064x}", 9_000);
    fresh.runtime_parity_case = Some(write_stdin_parity_case(100, "Script running with cell ID "));
    restored
        .observe_teacher_transition(fresh)
        .expect("observe fresh transition");
    let fresh_report = restored.report().live_scalar_shadow;
    assert_eq!(fresh_report.support_rows, 1, "{fresh_report:#?}");
    assert_eq!(fresh_report.future_rows, 0, "{fresh_report:#?}");
}

#[test]
fn current_strategy_checkpoint_rotates_scalar_generation_once() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut transition =
            crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                .expect("teacher transition");
        transition.before.session_id_sha256 = format!("{:064x}", index + 4_000);
        transition.runtime_parity_case = Some(write_stdin_parity_case(
            index,
            "Script running with cell ID ",
        ));
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    checkpoint.bucket_strategy_version = ONLINE_BUCKET_STRATEGY_VERSION;
    checkpoint.live_scalar_generation_version = 0;

    let restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("rotated miner");
    let report = restored.report().live_scalar_shadow;
    assert_eq!(report.support_rows, 0, "{report:#?}");
    assert_eq!(report.future_rows, 0, "{report:#?}");

    let checkpoint = restored.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    assert_eq!(
        checkpoint.live_scalar_generation_version,
        LIVE_SCALAR_GENERATION_VERSION
    );
}

#[test]
fn v95_numeric_handle_migration_preserves_frozen_generations() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut transition =
            crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                .expect("teacher transition");
        transition.before.session_id_sha256 = format!("{:064x}", index + 2_000);
        transition.runtime_parity_case = Some(write_stdin_parity_case(
            index,
            "Script running with cell ID ",
        ));
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    for _ in 0..2_048 {
        if !miner.self_training_v2.has_pending_work() {
            break;
        }
        miner.self_training_v2.run_work_slice();
    }
    let before = miner.report().self_training_v2.generations;
    let before = before.first().expect("expected frozen generation");
    let immutable_before = (
        before.generation_id_sha256.clone(),
        before.support_watermark_unix_nanos,
        before.support_rows,
        before.future_rows,
        before.wrong_future_rows,
    );

    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    checkpoint.bucket_strategy_version = 95;
    let restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
    let after = restored.report().self_training_v2.generations;
    let after = after.first().expect("preserved frozen generation");
    let immutable_after = (
        after.generation_id_sha256.clone(),
        after.support_watermark_unix_nanos,
        after.support_rows,
        after.future_rows,
        after.wrong_future_rows,
    );

    assert_eq!(immutable_after, immutable_before);
    assert!(restored.self_training_v2.has_pending_work());
}

#[test]
fn replay_parity_batch_builds_support_without_claiming_live_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-replay-parity-batch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp root");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    File::create(&config.input_path).expect("empty audit");
    let mut stream = OnlineResponseStream::open_streaming(config).expect("stream");
    let target_signature =
        crate::teacher_program_signature(&frame(0, "write_stdin", true)).expect("target signature");
    let target_signatures = BTreeSet::from([target_signature]);
    let cases = (0..40).map(|index| {
        (
            frame(index, "write_stdin", true),
            Some(write_stdin_parity_case(
                index,
                "Script running with cell ID ",
            )),
        )
    });
    stream
        .train_replay_cases_batch(cases)
        .expect("replay support import");
    for _ in 0..512 {
        let checks = stream.run_self_training_work_slice_for_signatures(&target_signatures);
        if checks == 0 && !stream.has_self_training_work_for_signatures(&target_signatures) {
            break;
        }
    }

    let report = stream.report().self_training_v2;
    assert_eq!(report.runtime_parity_cases_total, 0);
    assert_eq!(
        report.replay_support_parity_cases_total,
        OnlineResponseMinerConfig::default().reservoir_rows
    );
    assert!(report.generations.iter().all(|generation| {
        (1..=40).contains(&generation.support_rows)
            && generation.future_rows == 0
            && generation.runtime_parity_rows == 0
    }));
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn v48_canonical_parity_migration_preserves_teacher_pools_and_parity() {
    let mut miner =
        OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
    for index in 0..40 {
        let mut source_frame = frame(index, "write_stdin", true);
        source_frame
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
        source_frame.atoms.push(RelationAtom::ObservationSelector {
            slot_id: 1,
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "Process running with session ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
        });
        source_frame.atoms.sort();
        let mut transition = crate::teacher_transition_from_completed(&source_frame, None)
            .expect("teacher transition");
        let provider_payload = serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("Process running with session ID handle-{index}")
            }]
        });
        let expected_response = serde_json::json!({
            "name": "write_stdin",
            "arguments": {"session_id": format!("handle-{index}")}
        })
        .to_string();
        transition.runtime_parity_case = Some(crate::RuntimeParityCase {
            evidence_ref_sha256: String::new(),
            capture_receipt: None,
            request_text: String::new(),
            provider_payload,
            expected_response,
        });
        let enriched = transition.as_training_relation_frame();
        let synthesized = crate::synthesize_response_operator(&[enriched])
            .expect("typed synthesis")
            .candidate
            .program;
        let parity = transition.runtime_parity_case.as_ref().expect("parity");
        let execution =
            crate::execute_response(&synthesized, &parity.request_text, &parity.provider_payload);
        assert_eq!(
            execution.response.as_deref(),
            Some(parity.expected_response.as_str())
        );
        miner
            .observe_teacher_transition(transition)
            .expect("observe teacher transition");
    }
    let accepted = miner
        .report()
        .self_training_v2
        .discovery
        .accepted_transitions;
    let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
    let parity_before_report = checkpoint.self_training_v2.report(0);
    let parity_before = parity_before_report
        .runtime_parity_cases_total
        .saturating_add(parity_before_report.replay_support_parity_cases_total);
    // One signature retains at most 32 bounded parity cases; migration
    // must preserve that complete retained set without inventing future.
    assert_eq!(parity_before, 32);
    checkpoint.bucket_strategy_version = 48;

    let mut restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
    assert_eq!(
        restored
            .report()
            .self_training_v2
            .discovery
            .accepted_transitions,
        accepted
    );
    assert!(restored.self_training_v2.has_pending_work());
    assert!(restored.report().self_training_v2.generations.is_empty());
    // A strategy migration demotes historical live parity to support-only
    // evidence so it cannot be reinterpreted as post-freeze future.
    let migrated_parity = restored.report().self_training_v2;
    assert_eq!(migrated_parity.runtime_parity_cases_total, 0);
    assert_eq!(
        migrated_parity
            .runtime_parity_cases_total
            .saturating_add(migrated_parity.replay_support_parity_cases_total),
        parity_before
    );
    for _ in 0..1_024 {
        if !restored.self_training_v2.has_pending_work() {
            break;
        }
        let _ = restored.self_training_v2.run_work_slice();
    }
    assert!(!restored.self_training_v2.has_pending_work());
    let migrated_report = restored.report().self_training_v2;
    assert_eq!(
        migrated_report
            .generations
            .iter()
            .map(|generation| generation.support_rows)
            .max(),
        Some(32),
        "parity_overlap={} accepted={} signature_match={} cegis={:?} semantic_blockers={:?} generations={:?}",
        migrated_report.parity_discovery_key_overlap,
        migrated_report.parity_accepted_frame_rows,
        migrated_report.parity_signature_match_rows,
        migrated_report.cegis,
        migrated_report.semantic_law_blockers,
        migrated_report.generations,
    );
}

#[test]
fn response_stream_rejects_checkpoint_when_committed_prefix_changes() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-prefix-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
    stream
        .ingest(frame(1, "write_stdin", true))
        .expect("ingest");
    stream.persist().expect("checkpoint");
    drop(stream);

    let mut bytes = fs::read(&config.input_path).expect("ledger");
    let changed = bytes
        .iter_mut()
        .find(|byte| **byte == b'a')
        .expect("mutable payload byte");
    *changed = b'b';
    fs::write(&config.input_path, bytes).expect("corrupt committed prefix");

    assert!(matches!(
        OnlineResponseStream::open(config),
        Err(error) if error == "online_checkpoint_source_prefix_mismatch"
    ));
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn response_stream_appends_canonical_frame_bytes() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-canonical-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    let mut expected_frame = frame(2, "write_stdin", true);
    canonicalize_online_frame(&mut expected_frame);
    let mut expected = crate::canonical_json_bytes(&expected_frame).expect("canonical frame");
    expected.push(b'\n');

    let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
    stream.ingest(expected_frame).expect("ingest");
    assert_eq!(fs::read(&config.input_path).expect("ledger"), expected);

    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn historical_loader_rejects_conflicting_duplicate_without_future_claims() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-history-conflict-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    let first = frame(9, "write_stdin", true);
    let mut conflict = first.clone();
    conflict
        .atoms
        .push(RelationAtom::RequestPhaseAtom { atom_id: 999 });
    let mut audit = File::create(&config.input_path).expect("audit");
    for value in [first, conflict] {
        serde_json::to_writer(&mut audit, &value).expect("frame encoding");
        audit.write_all(b"\n").expect("newline");
    }
    drop(audit);
    let stream = OnlineResponseStream::open(config).expect("historical rebuild");
    assert_eq!(stream.report().rows_seen, 1);
    assert_eq!(stream.report().false_accepts, 0);
    assert_eq!(stream.parse_errors, 1);
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn replay_training_does_not_append_or_claim_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-replay-training-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
    stream
        .train_replay_batch((20..60).map(|index| frame(index, "write_stdin", true)))
        .expect("replay train");
    let report = stream.report();
    assert_eq!(report.rows_seen, 40);
    assert_eq!(stream.report().false_accepts, 0);
    assert!(report.self_training_v2.generations.is_empty());
    assert_eq!(
        report
            .self_training_v2
            .discovery
            .teacher_pools
            .iter()
            .map(|pool| pool.positive_rows)
            .sum::<u64>(),
        40
    );
    assert_eq!(fs::metadata(&config.input_path).expect("audit").len(), 0);
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn replay_parity_receipts_enable_support_but_never_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-response-replay-parity-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let config = OnlineResponseTailConfig {
        input_path: root.join("frames.jsonl"),
        report_path: root.join("report.json"),
        checkpoint_path: root.join("miner.checkpoint"),
        idle_sleep: Duration::from_millis(1),
    };
    let mut stream = OnlineResponseStream::open_streaming(config.clone()).expect("stream");
    stream
        .train_replay_cases_batch((0..40).map(|index| {
            let frame = plan_frame(index);
            let parity = plan_parity_case(index);
            (frame, Some(parity))
        }))
        .expect("replay parity train");
    for _ in 0..2_048 {
        if !stream.has_self_training_work() {
            break;
        }
        stream.run_self_training_work_slice();
    }
    assert!(!stream.has_self_training_work());
    stream.persist_now().expect("persist replay parity");
    let report = stream.report();
    assert!(
        report
            .self_training_v2
            .generations
            .iter()
            .any(|generation| generation.support_rows == 32)
    );
    assert!(
        report
            .self_training_v2
            .generations
            .iter()
            .all(|generation| generation.future_rows == 0)
    );
    drop(stream);

    let restored = OnlineResponseStream::open_streaming(config).expect("restored stream");
    assert!(
        restored
            .report()
            .self_training_v2
            .generations
            .iter()
            .all(|generation| generation.future_rows == 0)
    );
    fs::remove_dir_all(root).expect("temp cleanup");
}

#[test]
fn restored_future_reservoir_is_compacted_to_authority_bound() {
    let mut rows = (0..128)
        .map(|index| frame(index, "wait", true))
        .map(SharedRelationFrame::new)
        .collect::<VecDeque<_>>();

    trim_session_diverse_future(&mut rows, MAX_FROZEN_FUTURE_ROWS_PER_BUCKET);

    assert_eq!(rows.len(), MAX_FROZEN_FUTURE_ROWS_PER_BUCKET);
    assert_eq!(
        rows.iter()
            .map(|row| row.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn restored_bucket_evidence_interns_equal_learning_variants() {
    fn bucket(id: u64, evidence: RelationFrame) -> ResponseBucket {
        ResponseBucket {
            structural_family_id: id,
            teacher_signature_sha256: format!("{id:064x}"),
            teacher_action_symbol: "function:wait".to_owned(),
            positives: VecDeque::from([SharedRelationFrame::new(evidence)]),
            negatives: VecDeque::new(),
            future_positives: VecDeque::new(),
            future_negatives: VecDeque::new(),
            positive_rows: 1,
            negative_rows: 0,
            positive_tokens: 1,
            negative_tokens: 0,
            first_false_accept_frame_id: None,
            cardinality_guard_rejects: 0,
            positive_cardinality_bounds: BTreeMap::new(),
            positive_cardinality_signatures: BTreeSet::new(),
            support_watermark_event_time_unix_nanos: 1,
            late_or_missing_time_rows: 0,
            exact_guard_atom_ids: Vec::new(),
        }
    }

    let evidence = frame(1, "wait", true);
    let mut buckets = BTreeMap::from([
        (1, bucket(1, evidence.clone())),
        (2, bucket(2, evidence.clone())),
    ]);
    intern_bucket_evidence(&mut buckets).expect("equal evidence interns");
    assert!(Arc::ptr_eq(
        &buckets[&1].positives[0].0,
        &buckets[&2].positives[0].0,
    ));

    let mut conflicting = evidence;
    conflicting.atoms.push(RelationAtom::ToolKind {
        value: "conflicting-tool".to_owned(),
    });
    let mut conflict_buckets = BTreeMap::from([
        (1, bucket(1, buckets[&1].positives[0].materialize())),
        (2, bucket(2, conflicting)),
    ]);
    intern_bucket_evidence(&mut conflict_buckets).expect("distinct variants remain valid");
    assert!(!Arc::ptr_eq(
        &conflict_buckets[&1].positives[0].0,
        &conflict_buckets[&2].positives[0].0,
    ));
}
