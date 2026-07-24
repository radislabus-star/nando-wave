use super::*;
use crate::{
    AtomSource, AtomValueType, OnlineResponseCandidate, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseValueSelector, SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    synthesize_response_operator,
};
use nando_core::wave::{PhaseCenterCell, PhaseCenterFlatRecord, phase_vector_from_atom_ids};

fn frame(index: usize) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: format!("{index:064x}"),
        event_id_sha256: format!("{:064x}", index + 1),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: format!("{:064x}", index % 4),
        observed_at_unix_nanos: index as u64,
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::ToolKind {
                value: "exec".to_owned(),
            },
            RelationAtom::ObservationCallShape {
                value: format!("surface-{}", index % 2),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "function_call".to_owned(),
            },
            RelationAtom::ClientCapabilityAtom {
                atom_id: crate::package::stable_atom_id("client_capability:function:write_stdin"),
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
                value: "write_stdin".to_owned(),
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

fn candidate(
    support: Vec<RelationFrame>,
    future: Vec<RelationFrame>,
) -> OnlineResponseAdmissionCandidate {
    let synthesized = synthesize_response_operator(&support).expect("synthesis");
    let program = synthesized.candidate.program;
    let runtime_parity_cases = future
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let provider_payload = serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": serde_json::to_string(&format!("session-{index}"))
                        .expect("scalar")
                }]
            });
            let expected_response = crate::execute_response(&program, "", &provider_payload)
                .response
                .expect("runtime fixture executes");
            crate::RuntimeParityCase {
                evidence_ref_sha256: frame.frame_id_sha256.clone(),
                capture_receipt: None,
                request_text: String::new(),
                provider_payload,
                expected_response,
            }
        })
        .collect();
    let mut positive_center = vec![PhaseCenterCell::default(); 32];
    for frame in &support {
        for (sum, cell) in positive_center.iter_mut().zip(phase_vector_from_atom_ids(
            crate::relation_frame_online_routing_atom_ids(frame),
            32,
        )) {
            sum.re += cell.re / support.len() as f64;
            sum.im += cell.im / support.len() as f64;
        }
    }
    let wave_runtime_package = PhaseCenterFlatRuntime::new(
        32,
        vec![PhaseCenterFlatRecord {
            positive_center: positive_center.into_boxed_slice(),
            negative_center: vec![PhaseCenterCell::default(); 32].into_boxed_slice(),
        }],
    )
    .expect("wave runtime")
    .to_bytes()
    .expect("wave package");
    OnlineResponseAdmissionCandidate {
        candidate: OnlineResponseCandidate {
            bucket_id: 1,
            structural_family_id: 1,
            teacher_signature_sha256: "d".repeat(64),
            positive_rows: support.len() + future.len(),
            negative_rows: 0,
            positive_tokens: 0,
            negative_tokens: 0,
            distinct_sessions: 4,
            wave_threshold_micro: 1,
            wave_runtime_bytes: 1,
            wave_runtime_fingerprint64: 1,
            program,
            verifier: synthesized.verifier,
            phase_rank: synthesized.candidate.phase_rank,
            exact_checks: synthesized.candidate.exact_checks,
        },
        wave_runtime_package,
        support,
        future,
        negatives: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        runtime_parity_cases,
        semantic_alias_edges: Vec::new(),
        semantic_evidence_receipts: Vec::new(),
        semantic_evidence_root_sha256: String::new(),
    }
}

fn seal_semantic_evidence(candidate: &mut OnlineResponseAdmissionCandidate) {
    let program_sha256 = canonical_json_sha256(&candidate.candidate.program).expect("program");
    candidate.semantic_evidence_receipts = candidate
        .support
        .iter()
        .chain(&candidate.future)
        .map(|frame| crate::SemanticEvidenceReceipt {
            schema: crate::SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1.to_owned(),
            generation_id_sha256: "a".repeat(64),
            cohort_id_sha256: "b".repeat(64),
            winner_program_sha256: program_sha256.clone(),
            frame_id_sha256: frame.frame_id_sha256.clone(),
            evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
            outcome: crate::SemanticEvidenceOutcome::VerifiedEquivalent,
            reason: "test_verified_equivalent".to_owned(),
        })
        .chain(
            candidate
                .negatives
                .iter()
                .map(|frame| crate::SemanticEvidenceReceipt {
                    schema: crate::SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1.to_owned(),
                    generation_id_sha256: "a".repeat(64),
                    cohort_id_sha256: "b".repeat(64),
                    winner_program_sha256: program_sha256.clone(),
                    frame_id_sha256: frame.frame_id_sha256.clone(),
                    evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
                    outcome: crate::SemanticEvidenceOutcome::ApplicabilityNegative,
                    reason: "test_applicability_negative".to_owned(),
                }),
        )
        .collect();
    candidate.semantic_evidence_root_sha256 = sha256_bytes(
        &serde_json::to_vec(&(
            "nando.semantic-evidence-set.v1",
            &candidate.semantic_evidence_receipts,
        ))
        .expect("evidence encode"),
    );
}

#[test]
fn semantic_negative_is_independently_rejected_when_parity_proves_equivalence() {
    let support = (0..32).map(frame).collect::<Vec<_>>();
    let future = (32..64).map(frame).collect::<Vec<_>>();
    let mut candidate = candidate(support, future);
    let mut negative = candidate.future[0].clone();
    negative.frame_id_sha256 = "f".repeat(64);
    negative.evidence_ref_sha256 = "e".repeat(64);
    let mut parity = candidate.runtime_parity_cases[0].clone();
    parity.evidence_ref_sha256 = negative.frame_id_sha256.clone();
    candidate.runtime_parity_cases.push(parity);
    candidate.negatives.push(negative);
    seal_semantic_evidence(&mut candidate);

    assert_eq!(
        validate_semantic_evidence(&candidate),
        Err("semantic_negative_is_verified_equivalent".to_owned())
    );
}

#[test]
fn online_admission_rejects_support_future_overlap() {
    let support = (0..32).map(frame).collect::<Vec<_>>();
    let candidate = candidate(support.clone(), support);
    let snapshot = build_online_admission_snapshot(
        &[candidate],
        "project",
        1,
        100,
        60,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation");
    assert!(snapshot.is_none());
}

#[test]
fn merged_authority_revision_depends_on_active_content_not_candidate_revision() {
    let support = (0..32).map(frame).collect::<Vec<_>>();
    let future = (32..64)
        .map(|index| {
            let mut row = frame(index);
            row.session_id_sha256 = format!("{:064x}", index + 10_000);
            row
        })
        .collect::<Vec<_>>();
    let build = |revision| {
        let snapshot = build_online_admission_snapshot(
            &[candidate(support.clone(), future.clone())],
            "project",
            revision,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission evaluation")
        .expect("complete candidate");
        merge_online_admission_snapshots(vec![snapshot])
            .expect("merge")
            .expect("merged authority")
    };
    let first = build(7);
    let second = build(99);
    assert_eq!(first.registry.revision, second.registry.revision);
    assert_eq!(
        response_registry_digest(&first.registry),
        response_registry_digest(&second.registry)
    );
    assert_eq!(
        first.admission.response_authority.packages,
        second.admission.response_authority.packages
    );
}

fn frames_for_function(
    range: std::ops::Range<usize>,
    function_name: &str,
    argument_name: &str,
) -> Vec<RelationFrame> {
    range
        .map(|index| {
            let mut row = frame(index);
            for atom in &mut row.atoms {
                match atom {
                    RelationAtom::ActionFunction { value } => {
                        *value = function_name.to_owned();
                    }
                    RelationAtom::ActionRoleArgument { name, .. } => {
                        *name = argument_name.to_owned();
                    }
                    RelationAtom::ClientCapabilityAtom { atom_id } => {
                        *atom_id = crate::package::stable_atom_id(&format!(
                            "client_capability:function:{function_name}"
                        ));
                    }
                    _ => {}
                }
            }
            row
        })
        .collect()
}

fn admission_snapshot_for_function(
    offset: usize,
    function_name: &str,
    argument_name: &str,
    now_unix: u64,
) -> OnlineAdmissionSnapshot {
    let support = frames_for_function(offset..offset + 32, function_name, argument_name);
    let future = frames_for_function(offset + 32..offset + 64, function_name, argument_name);
    build_online_admission_snapshot(
        &[candidate(support, future)],
        "project",
        offset as u64 + 1,
        now_unix,
        60,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation")
    .expect("complete candidate")
}

#[test]
fn active_admission_merge_preserves_distinct_operator_packages() {
    let active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let candidate = admission_snapshot_for_function(100, "write_stdin", "session_id", 100);
    let merged = merge_with_active_online_admission(
        candidate,
        active.registry,
        active.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        60,
    )
    .expect("additive admission");

    assert_eq!(merged.registry.packages.len(), 2);
    let executor = crate::ResponseExecutor::from_registry_with_admission(
        merged.registry,
        merged.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        60,
    )
    .expect("merged authority");
    assert_eq!(executor.active_package_count(), 2);
}

#[test]
fn active_admission_merge_deduplicates_identical_package() {
    let active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let candidate = active.clone();
    let merged = merge_with_active_online_admission(
        candidate,
        active.registry,
        active.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        60,
    )
    .expect("deduplicated admission");

    assert_eq!(merged.registry.packages.len(), 1);
    assert_eq!(
        merged.admission.response_authority.packages.len(),
        merged.registry.packages.len()
    );
}

#[test]
fn active_admission_merge_rejects_expired_authority() {
    let active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let candidate = admission_snapshot_for_function(100, "write_stdin", "session_id", 200);
    assert!(
        merge_with_active_online_admission(
            candidate,
            active.registry,
            active.admission,
            "project",
            &"a".repeat(64),
            &"b".repeat(64),
            200,
            60,
        )
        .is_err()
    );
}

#[test]
fn proven_active_merge_reissues_expired_static_material() {
    let active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let candidate = admission_snapshot_for_function(100, "write_stdin", "session_id", 200);
    let merged = merge_with_proven_active_online_admission(
        candidate,
        active.registry,
        active.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        200,
        60,
    )
    .expect("static package proofs are reissued under the current lease");

    assert_eq!(merged.registry.packages.len(), 2);
    assert_eq!(merged.admission.generated_at_unix, 200);
    assert_eq!(merged.admission.expires_at_unix, 260);
}

#[test]
fn proven_active_merge_keeps_equivalent_authorized_payload_immutable() {
    let active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let candidate = admission_snapshot_for_function(100, "wait", "cell_id", 200);
    assert_eq!(
        active.registry.packages[0].package_id,
        candidate.registry.packages[0].package_id
    );
    let first_reissue = merge_with_proven_active_online_admission(
        candidate,
        active.registry,
        active.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        200,
        60,
    )
    .expect("equivalent evidence reissues the existing payload");
    let stable_revision = first_reissue.registry.revision;
    let stable_package = first_reissue.registry.packages[0].clone();
    let next_candidate = admission_snapshot_for_function(200, "wait", "cell_id", 300);

    let second_reissue = merge_with_proven_active_online_admission(
        next_candidate,
        first_reissue.registry,
        first_reissue.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        300,
        60,
    )
    .expect("later equivalent evidence preserves the active payload");

    assert_eq!(second_reissue.registry.revision, stable_revision);
    assert_eq!(second_reissue.registry.packages, vec![stable_package]);
    assert_eq!(second_reissue.admission.generated_at_unix, 300);
    assert_eq!(second_reissue.admission.expires_at_unix, 360);
}

#[test]
fn runtime_revocation_removes_only_the_matching_execution_identity() {
    let wait = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    let write_stdin = admission_snapshot_for_function(100, "write_stdin", "session_id", 100);
    let active = merge_with_active_online_admission(
        write_stdin,
        wait.registry,
        wait.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        60,
    )
    .expect("active");
    let revoked = active
        .registry
        .packages
        .iter()
        .find(|package| {
            matches!(
                package.program.operation,
                crate::ResponseOperation::FunctionCallFromRoles {
                    ref function_name,
                    ..
                } if function_name == "wait"
            )
        })
        .expect("wait package");
    let revoked_id = revoked.package_id.clone();
    let mut revocations = nando_operator_admission::RuntimePackageRevocationLedgerV1::default();
    revocations
        .record(nando_operator_admission::RuntimePackageRevocationV1 {
            package_id: revoked_id.clone(),
            execution_payload_sha256: crate::response_execution_payload_digest(revoked)
                .expect("payload"),
            request_sha256: "55".repeat(32),
            observed_at_unix: 150,
            reason: "runtime_false_accept".to_owned(),
        })
        .expect("record revocation");

    let reissued = reissue_unrevoked_active_online_admission(
        active.registry,
        active.admission,
        &revocations,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        200,
        60,
    )
    .expect("reissue")
    .expect("remaining package");
    assert_eq!(reissued.registry.packages.len(), 1);
    assert!(
        reissued
            .registry
            .packages
            .iter()
            .all(|package| package.package_id != revoked_id)
    );
    assert_eq!(reissued.admission.generated_at_unix, 200);
    assert_eq!(reissued.admission.expires_at_unix, 260);
}

#[test]
fn proven_active_merge_rejects_veto_material() {
    let mut active = admission_snapshot_for_function(0, "wait", "cell_id", 100);
    active.admission.verdict = "VETO".to_owned();
    active.admission.eligible_for_local_accept = false;
    let candidate = admission_snapshot_for_function(100, "write_stdin", "session_id", 200);

    assert_eq!(
        merge_with_proven_active_online_admission(
            candidate,
            active.registry,
            active.admission,
            "project",
            &"a".repeat(64),
            &"b".repeat(64),
            200,
            60,
        )
        .expect_err("VETO material must not be reissued"),
        "response_admission_not_pass"
    );
}

#[test]
fn semantic_law_binding_preserves_consensus_actor_and_independent_verifier() {
    let first = (0..32).map(frame).collect::<Vec<_>>();
    let mut second = (100..132).map(frame).collect::<Vec<_>>();
    for row in &mut second {
        for atom in &mut row.atoms {
            match atom {
                RelationAtom::ActionFunction { value } => *value = "wait".to_owned(),
                RelationAtom::ActionRoleArgument { name, .. } => {
                    *name = "cell_id".to_owned();
                }
                _ => {}
            }
        }
    }
    let first_program = synthesize_response_operator(&first)
        .expect("first adapter")
        .candidate
        .program;
    let second_program = synthesize_response_operator(&second)
        .expect("second adapter")
        .candidate
        .program;
    let consensus = crate::ResponseProgram::unique_consensus(vec![
        crate::ResponseConsensusVariant {
            program: first_program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        },
        crate::ResponseConsensusVariant {
            program: second_program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        },
    ]);
    let mut package = compile_source_neutral_quarantine_packages(&first, true)
        .into_iter()
        .next()
        .expect("package shell");
    let training = first.into_iter().chain(second).collect::<Vec<_>>();

    bind_proven_semantic_law_program(&mut package, &consensus, &training)
        .expect("bind semantic law");

    assert_eq!(package.program, consensus);
    assert!(matches!(
        package.verifier,
        Some(crate::VerifierProgram::UniqueConsensus { .. })
    ));
    assert!(package.validate().is_ok());
}

#[test]
fn runtime_parity_normalizes_only_execution_budgets() {
    let actual = r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":10000}}"#;
    let teacher = r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":30000}}"#;
    let wrong_handle = r#"{"name":"wait","arguments":{"cell_id":"xyz","yield_time_ms":30000}}"#;
    let destructive =
        r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":30000,"terminate":true}}"#;

    assert!(responses_match_after_execution_budget_normalization(
        actual, teacher
    ));
    assert!(!responses_match_after_execution_budget_normalization(
        actual,
        wrong_handle
    ));
    assert!(!responses_match_after_execution_budget_normalization(
        actual,
        destructive
    ));

    let custom_actual = r#"{"kind":"custom_tool_call","name":"exec","input":"const r=await tools.write_stdin({\"chars\":\"\",\"max_output_tokens\":1000,\"session_id\":7,\"yield_time_ms\":10000});text(r.output);"}"#;
    let custom_teacher = r#"{"kind":"custom_tool_call","name":"exec","input":"const r=await tools.write_stdin({\"chars\":\"\",\"max_output_tokens\":9000,\"session_id\":7,\"yield_time_ms\":30000});text(r.output);"}"#;
    let custom_wrong_role = r#"{"kind":"custom_tool_call","name":"exec","input":"const r=await tools.write_stdin({\"session_id\":8});text(r.output);"}"#;
    assert!(responses_match_after_execution_budget_normalization(
        custom_actual,
        custom_teacher
    ));
    assert!(!responses_match_after_execution_budget_normalization(
        custom_actual,
        custom_wrong_role
    ));
}

#[test]
fn durable_parity_rejects_non_exact_teacher_match_with_valid_digest() {
    let program_sha256 = "a".repeat(64);
    let verifier_sha256 = "b".repeat(64);
    let mut receipt = DurableRuntimeParityReceipt {
        schema: DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        evidence_ref_sha256: "c".repeat(64),
        program_sha256: program_sha256.clone(),
        verifier_sha256: verifier_sha256.clone(),
        input_sha256: "d".repeat(64),
        teacher_response_sha256: "e".repeat(64),
        actor_response_sha256: "f".repeat(64),
        actor_executed: true,
        teacher_authority_match: true,
        independent_verifier_pass: true,
        exact_teacher_match: false,
    };
    receipt.seal_digest().expect("receipt digest");

    assert!(!validate_durable_runtime_parity_receipt(
        &receipt,
        &program_sha256,
        &verifier_sha256
    ));
}

#[test]
fn support_parity_restores_typed_role_and_empty_poll_argument() {
    let mut support = frame(1);
    support
        .atoms
        .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
    support.atoms.push(RelationAtom::ObservationSelector {
        slot_id: 1,
        selector: ResponseValueSelector::ContentLinePrefix {
            prefix: "Process running with session ID ".to_owned(),
            value_type: AtomValueType::Identifier,
        },
    });
    support.atoms.sort();
    let expected_response = serde_json::json!({
        "name": "write_stdin",
        "arguments": {
            "session_id": 42,
            "chars": "",
            "yield_time_ms": 1000,
            "max_output_tokens": 8000
        }
    })
    .to_string();
    let parity = crate::RuntimeParityCase {
        evidence_ref_sha256: support.frame_id_sha256.clone(),
        capture_receipt: None,
        request_text: String::new(),
        provider_payload: serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "Process running with session ID 42"
            }]
        }),
        expected_response: expected_response.clone(),
    };
    let enriched = action_schema_enriched_frame(&support, Some(&parity));
    let synthesized = synthesize_response_operator(&[enriched]).expect("typed synthesis");
    let execution =
        crate::execute_response(&synthesized.candidate.program, "", &parity.provider_payload);
    assert_eq!(
        execution.response.as_deref(),
        Some(expected_response.as_str())
    );
    crate::verify_response_independently(
        &synthesized.verifier,
        &parity.provider_payload,
        &expected_response,
    )
    .expect("independent typed verifier");
}

#[test]
fn online_admission_builds_authorized_executor_for_disjoint_proof() {
    let support = (0..32).map(frame).collect::<Vec<_>>();
    let future = (32..64).map(frame).collect::<Vec<_>>();
    synthesize_response_operator(&support).expect("direct synthesis");
    let packages = compile_source_neutral_quarantine_packages(&support, true);
    assert!(!packages.is_empty(), "package synthesis failed");
    let package = packages.into_iter().next().expect("package");
    let causal = evaluate_grounded_wave_causality(&package, &support, &future, &[]);
    let snapshot = build_online_admission_snapshot(
        &[candidate(support, future)],
        "project",
        64,
        100,
        60,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation")
    .unwrap_or_else(|| panic!("proven admission: {causal:?}"));
    let executor = crate::ResponseExecutor::from_registry_with_admission(
        snapshot.registry,
        snapshot.admission,
        "project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        60,
    )
    .expect("authorized executor");
    assert_eq!(executor.active_package_count(), 1);
}

#[test]
fn online_admission_uses_clean_subcenter_atoms_as_wave_query() {
    let guard_atom = crate::package::stable_atom_id("test:clean-subcenter");
    let support = (0..32)
        .map(|index| {
            let mut frame = frame(index);
            frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: guard_atom,
            });
            frame
        })
        .collect::<Vec<_>>();
    let future = (32..64)
        .map(|index| {
            let mut frame = frame(index);
            frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: guard_atom,
            });
            frame
        })
        .collect::<Vec<_>>();
    let mut candidate = candidate(support, future);
    let center = phase_vector_from_atom_ids([guard_atom], 32);
    candidate.wave_runtime_package = PhaseCenterFlatRuntime::new(
        32,
        vec![PhaseCenterFlatRecord {
            positive_center: center.into_boxed_slice(),
            negative_center: vec![PhaseCenterCell::default(); 32].into_boxed_slice(),
        }],
    )
    .expect("wave runtime")
    .to_bytes()
    .expect("wave package");
    candidate.required_routing_atom_ids = vec![guard_atom];
    candidate.candidate.wave_threshold_micro = 900_000;

    let snapshot = build_online_admission_snapshot(
        &[candidate],
        "project",
        64,
        100,
        60,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation");
    assert!(snapshot.is_some());
}

#[test]
fn learned_wave_route_uses_two_clean_centers_for_two_positive_clusters() {
    let cluster_frame = |index: usize, atom_id: u64| {
        let mut frame = frame(index);
        frame.atoms = vec![RelationAtom::RequestPhaseAtom { atom_id }];
        frame
    };
    let route_accepts = |route: &LearnedWaveRoute, frame: &RelationFrame| {
        let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
        if !route.query_atom_ids.is_empty() {
            atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
        }
        let query = phase_vector_from_atom_ids(atoms, usize::from(route.cells));
        let score = |center_delta_micro: &[i32]| {
            phase_margin_to_micro(
                query
                    .iter()
                    .zip(center_delta_micro.chunks_exact(2))
                    .map(|(cell, delta)| {
                        cell.re * f64::from(delta[0]) / 1_000_000.0
                            + cell.im * f64::from(delta[1]) / 1_000_000.0
                    })
                    .sum::<f64>()
                    / f64::from(route.cells),
            )
            .expect("finite route score")
        };
        score(&route.center_delta_micro) >= route.threshold_micro
            || route
                .subcenters
                .iter()
                .any(|subcenter| score(&subcenter.center_delta_micro) >= subcenter.threshold_micro)
    };

    let negatives = (0..16)
        .map(|index| cluster_frame(10_000 + index, 10_000))
        .collect::<Vec<_>>();
    let (support, route) = (2..256)
        .find_map(|second_atom| {
            let support = (0..32)
                .map(|index| cluster_frame(index, if index < 16 { 1 } else { second_atom }))
                .collect::<Vec<_>>();
            let route = learned_wave_route_from_support_medoid(&support, &negatives, 32)?;
            let primary_only = LearnedWaveRoute {
                subcenters: Vec::new(),
                ..route.clone()
            };
            (!route.subcenters.is_empty()
                && support
                    .iter()
                    .any(|frame| !route_accepts(&primary_only, frame)))
            .then_some((support, route))
        })
        .expect("two separable positive clusters");

    assert_eq!(route.subcenters.len(), 1);
    assert!(support.iter().all(|frame| route_accepts(&route, frame)));
    assert!(negatives.iter().all(|frame| !route_accepts(&route, frame)));
    let primary_only = LearnedWaveRoute {
        subcenters: Vec::new(),
        ..route.clone()
    };
    assert!(
        support
            .iter()
            .any(|frame| !route_accepts(&primary_only, frame))
    );
}

#[test]
fn learned_wave_route_keeps_clean_support_and_abstains_on_collisions() {
    let cluster_frame = |index: usize, atom_id: u64| {
        let mut frame = frame(index);
        frame.atoms = vec![RelationAtom::RequestPhaseAtom { atom_id }];
        frame
    };
    let route_accepts = |route: &LearnedWaveRoute, frame: &RelationFrame| {
        let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
        atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
        let query = phase_vector_from_atom_ids(atoms, usize::from(route.cells));
        let score = |center_delta_micro: &[i32]| {
            phase_margin_to_micro(
                query
                    .iter()
                    .zip(center_delta_micro.chunks_exact(2))
                    .map(|(cell, delta)| {
                        cell.re * f64::from(delta[0]) / 1_000_000.0
                            + cell.im * f64::from(delta[1]) / 1_000_000.0
                    })
                    .sum::<f64>()
                    / f64::from(route.cells),
            )
            .expect("finite route score")
        };
        score(&route.center_delta_micro) >= route.threshold_micro
            || route
                .subcenters
                .iter()
                .any(|subcenter| score(&subcenter.center_delta_micro) >= subcenter.threshold_micro)
    };

    let support = (0..40)
        .map(|index| cluster_frame(index, if index < 32 { 1 } else { 10_000 }))
        .collect::<Vec<_>>();
    let negatives = (0..16)
        .map(|index| cluster_frame(10_000 + index, 10_000))
        .collect::<Vec<_>>();
    let route = learned_wave_route_from_support_medoid(&support, &negatives, 32)
        .expect("clean support subcenter");

    assert_eq!(
        support
            .iter()
            .filter(|frame| route_accepts(&route, frame))
            .count(),
        32
    );
    assert!(negatives.iter().all(|frame| !route_accepts(&route, frame)));
}

#[test]
fn process_session_protocol_has_structural_capability_but_cell_wait_does_not() {
    let process = crate::ResponseProgram::function_call_from_roles(
        "write_stdin",
        ResponseValueSelector::ContentLinePrefix {
            prefix: "Process running with session ID ".to_owned(),
            value_type: AtomValueType::Identifier,
        },
        vec![],
    );
    let cell = crate::ResponseProgram::function_call_from_roles(
        "wait",
        ResponseValueSelector::ContentLinePrefix {
            prefix: "Script running with cell ID ".to_owned(),
            value_type: AtomValueType::Identifier,
        },
        vec![],
    );
    assert!(program_required_client_capability_atom(&process).is_none());
    assert!(program_required_client_capability_atom(&cell).is_some());
}

#[test]
fn future_selection_preserves_session_diversity_before_filling() {
    let future = (0..80)
        .map(|index| {
            let mut frame = frame(index + 100);
            frame.session_id_sha256 = if index < 40 {
                "1".repeat(64)
            } else if index < 70 {
                "2".repeat(64)
            } else {
                "3".repeat(64)
            };
            frame
        })
        .collect::<Vec<_>>();
    let selected = select_diverse_future(&future, 32);
    assert_eq!(selected.len(), 32);
    assert_eq!(
        selected
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
}
