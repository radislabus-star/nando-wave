use nando_response_actor::{
    ProjectStatusMapping, ResponseExecutor, ResponseProgram, ResponseRegistry,
    ValueProjectionFormat,
};
use sha2::{Digest, Sha256};

use super::*;

static PROJECT_STATUS_LIFECYCLE_TEST_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn sha256_text(value: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(value.as_ref()))
}

fn v7_project_status_frame(
    index: usize,
    session: usize,
    observed_at_unix_nanos: u64,
) -> RelationFrame {
    let value = if index.is_multiple_of(2) { "0" } else { "23" };
    RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: sha256_text(format!("status-frame-{index}")),
        event_id_sha256: sha256_text(format!("status-event-{index}")),
        client_intent_id_sha256: sha256_text(format!("status-intent-{index}")),
        session_id_sha256: sha256_text(format!("status-session-{session}")),
        observed_at_unix_nanos,
        estimated_input_tokens: 64,
        extractor_version: "response-relation-extractor.v7".to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::ToolKind {
                value: if session.is_multiple_of(2) {
                    "exec".to_owned()
                } else {
                    "write_stdin".to_owned()
                },
            },
            RelationAtom::ObservationCallShape {
                value: "function_call".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "assistant_message".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: sha256_text(value),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: ResponseValueSelector::JsonField {
                    field: "exit_code".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
            RelationAtom::Cardinality {
                role: "turn_call_count_band".to_owned(),
                count: 1,
            },
            RelationAtom::Cardinality {
                role: "turn_output_count_band".to_owned(),
                count: 1,
            },
            RelationAtom::Cardinality {
                role: "turn_pending_count_band".to_owned(),
                count: 0,
            },
            RelationAtom::Cardinality {
                role: "turn_message_count_band".to_owned(),
                count: 0,
            },
            RelationAtom::Cardinality {
                role: "turn_call_shape_count_band".to_owned(),
                count: 1,
            },
            RelationAtom::ActionStatusProjection {
                mapping: ProjectStatusMapping::ZeroIsSuccess,
            },
        ],
        evidence_ref_sha256: sha256_text(format!("status-evidence-{index}")),
    }
}

fn v7_cross_family_projection_frame(
    index: usize,
    session: usize,
    observed_at_unix_nanos: u64,
) -> RelationFrame {
    let mut frame = v7_project_status_frame(index, session, observed_at_unix_nanos);
    let observation_hash = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::TypedSlot {
            source: AtomSource::Observation,
            value_sha256,
            ..
        } => Some(value_sha256.clone()),
        _ => None,
    });
    frame
        .atoms
        .retain(|atom| !matches!(atom, RelationAtom::ActionStatusProjection { .. }));
    frame.atoms.extend([
        RelationAtom::TypedSlot {
            slot_id: 2,
            value_type: AtomValueType::Integer,
            source: AtomSource::Action,
            value_sha256: observation_hash.expect("observation hash"),
        },
        RelationAtom::SlotEquality {
            left_slot: 1,
            right_slot: 2,
        },
        RelationAtom::ActionValueProjection {
            format: ValueProjectionFormat::CanonicalJson,
            renderer: nando_response_actor::CollectionOutputRenderer::Direct,
        },
    ]);
    frame.frame_id_sha256 = sha256_text(format!("projection-frame-{index}"));
    frame.event_id_sha256 = sha256_text(format!("projection-event-{index}"));
    frame.client_intent_id_sha256 = sha256_text(format!("projection-intent-{index}"));
    frame.evidence_ref_sha256 = sha256_text(format!("projection-evidence-{index}"));
    frame
}

fn write_relation_frames(path: &Path, frames: &[RelationFrame]) {
    let mut bytes = Vec::new();
    for frame in frames {
        serde_json::to_writer(&mut bytes, frame).expect("frame json");
        bytes.push(b'\n');
    }
    fs::write(path, bytes).expect("write relation frames");
}

#[test]
fn zero_future_receipts_are_not_evaluated() {
    assert_eq!(verifier_coverage_state(0, 0), "NOT_EVALUATED");
    assert_eq!(verifier_coverage_state(3, 2), "PARTIAL");
    assert_eq!(verifier_coverage_state(3, 3), "COMPLETE");
}

#[test]
fn relation_frame_replays_are_deduped_and_conflicts_are_counted() {
    let frame = RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "a".repeat(64),
        event_id_sha256: "b".repeat(64),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: "d".repeat(64),
        observed_at_unix_nanos: 1,
        estimated_input_tokens: 0,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![RelationAtom::CompletionState {
            value: "pending".to_owned(),
        }],
        evidence_ref_sha256: "e".repeat(64),
    };
    let mut conflicting = frame.clone();
    conflicting.verifier_label = Some(false);
    let (unique, duplicate_rows, conflicting_ids) =
        dedupe_relation_frames(vec![frame.clone(), frame, conflicting]);
    assert_eq!(unique.len(), 1);
    assert_eq!(duplicate_rows, 2);
    assert_eq!(conflicting_ids, 1);
}

#[test]
fn grounded_family_scoreboard_reports_positive_and_total_token_opportunity() {
    let frame = |id: char, event: char, label: bool, tokens: u64| RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: id.to_string().repeat(64),
        event_id_sha256: event.to_string().repeat(64),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: "d".repeat(64),
        observed_at_unix_nanos: 1,
        estimated_input_tokens: tokens,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(label),
        atoms: vec![RelationAtom::ActionValueProjection {
            format: nando_response_actor::ValueProjectionFormat::PlainText,
            renderer: nando_response_actor::CollectionOutputRenderer::Direct,
        }],
        evidence_ref_sha256: "e".repeat(64),
    };
    let report = grounded_family_report(
        7,
        &[
            frame('1', 'a', true, 400),
            frame('2', 'a', true, 390),
            frame('3', 'b', false, 90),
            frame('4', 'b', false, 80),
        ],
    );
    assert_eq!(report["positive_estimated_input_tokens"], 400);
    assert_eq!(report["total_estimated_input_tokens"], 490);
    assert_eq!(
        report["action_symbols"][0],
        "value_projection:PlainText:direct"
    );
}

#[test]
fn generic_operation_is_reported_separately_from_wait_templates() {
    let generic = ResponseOperation::FunctionCallFromRoles {
        function_name: "resume_job".to_owned(),
        selector: nando_response_actor::ResponseValueSelector::UniqueScalar {
            value_type: nando_response_actor::AtomValueType::Identifier,
        },
        arguments: Vec::new(),
    };
    assert_eq!(program_operation_name(&generic), "function_call_from_roles");
    assert_eq!(
        program_operation_name(&ResponseOperation::ProjectSelectedValue {
            selector: nando_response_actor::ResponseValueSelector::UniqueScalar {
                value_type: nando_response_actor::AtomValueType::Integer,
            },
            format: nando_response_actor::ValueProjectionFormat::CanonicalJson,
            renderer: nando_response_actor::CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
        }),
        "project_selected_value"
    );
    assert_eq!(
        program_operation_name(&ResponseOperation::WaitOnAnyYieldedCell {
            function_name: "wait".to_owned(),
            yield_time_ms: 1_000,
            max_tokens: 5_000,
        }),
        "wait_on_any_yielded_cell"
    );
    let status = ResponseProgram::project_status(
        ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        },
        nando_response_actor::ProjectStatusMapping::ZeroIsSuccess,
        "completed",
    );
    assert_eq!(program_operation_name(&status.operation), "project_status");
    assert_eq!(
        response_program_external_verifier_schema(&status),
        Some("status_projection_external_evidence.v1")
    );
}

#[test]
fn project_status_v7_frames_complete_automatic_support_future_and_causal_lifecycle() {
    let root = env::temp_dir().join(format!(
        "nando-response-miner-project-status-{}-{}",
        std::process::id(),
        PROJECT_STATUS_LIFECYCLE_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("test root");
    let args = [
        "relations.jsonl",
        "shadows.jsonl",
        "causal.json",
        "registry.json",
        "status.json",
        "frames.jsonl",
        "manifests.json",
        "receipts.json",
        "grounded-causal.json",
    ]
    .map(|name| root.join(name));
    atomic_write_value(
        &args[2],
        &serde_json::json!({
            "schema":"nando.response-wave-causal-proof.v1",
            "verdict":"PASS",
            "heldout_correct":32,
            "heldout_total":32,
            "full_phase_exact_checks":32,
            "no_phase_exact_checks":64,
            "shuffled_phase_exact_checks":64,
        }),
    )
    .expect("causal proof");

    let support = (0..32)
        .map(|index| v7_project_status_frame(index, index % 4, index as u64 + 1))
        .collect::<Vec<_>>();
    write_relation_frames(&args[5], &support);
    run_with_args(&args).expect("support cycle");

    let manifests: ResponseSupportManifestSet = read_json(&args[6]).expect("support manifests");
    assert_eq!(manifests.manifests.len(), 1);
    assert_eq!(manifests.manifests[0].support_frame_ids.len(), 32);
    let freeze_time = manifests.manifests[0].created_at_unix_nanos;

    let mut support_and_future = support;
    support_and_future.extend((32..64).map(|index| {
        v7_project_status_frame(
            index,
            100 + index % 4,
            freeze_time.saturating_add(index as u64 + 1),
        )
    }));
    write_relation_frames(&args[5], &support_and_future);
    run_with_args(&args).expect("future cycle");

    let registry: ResponseRegistry = read_json(&args[3]).expect("runtime registry");
    assert_eq!(registry.packages.len(), 1);
    let package = &registry.packages[0];
    assert!(matches!(
        package.program.operation,
        ResponseOperation::ProjectStatus { .. }
    ));
    assert_eq!(package.state, ResponsePackageState::Active);
    assert_eq!(package.proof.support_rows, 32);
    assert_eq!(package.proof.future_rows, 32);
    assert_eq!(package.proof.distinct_sessions, 4);
    assert_eq!(package.proof.distinct_surfaces, 2);
    assert_eq!(package.proof.wrong_accepts, 0);
    assert!(package.proof.wave_causal_pass);
    assert!(package.eligible_for_admission_candidate());
    assert!(!package.eligible_for_local_accept());

    let executor = ResponseExecutor::load(&args[3]).expect("miner-built candidate registry");
    assert_eq!(executor.active_package_count(), 0);
    assert_eq!(executor.diagnostic_package_count(), 1);
    let blocked = executor.execute(
        "",
        &serde_json::json!({
            "input":[{"type":"function_call_output","output":"{\"exit_code\":0}"}]
        }),
    );
    assert_eq!(blocked.status, ResponseExecutionStatus::Abstain);
    assert_eq!(blocked.reason, "execution_authority_missing");

    let execution = executor.execute_shadow(
        "",
        &serde_json::json!({
            "input":[
                {"type":"message","role":"user","content":"status"},
                {
                    "type":"function_call",
                    "name":"exec",
                    "call_id":"status-call",
                    "arguments":"{}"
                },
                {
                    "type":"function_call_output",
                    "call_id":"status-call",
                    "output":"{\"exit_code\":0}"
                }
            ]
        }),
    );
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        execution.reason
    );
    assert_eq!(execution.response.as_deref(), Some("success"));

    let status: Value = read_json(&args[4]).expect("miner status");
    assert_eq!(status["grounded_promotion_ready"], Value::Bool(true));
    assert_eq!(status["grounded_causal_verdict"], "PASS");
    assert_eq!(status["future_eligibility"]["verifier_accepted_rows"], 32);
    let receipts: Value = read_json(&args[7]).expect("verifier receipts");
    let packages = receipts["packages"].as_array().expect("receipt packages");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["package_id"], package.package_id);
    let receipts = packages[0]["receipts"].as_array().expect("receipt rows");
    assert_eq!(receipts.len(), 32);
    assert!(receipts.iter().all(|receipt| {
        receipt["accepted"] == Value::Bool(true)
            && receipt["schema"] == RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2
            && receipt["actor_program_sha256"]
                .as_str()
                .is_some_and(nando_response_actor::valid_nonzero_sha256)
            && receipt["independent_verifier_program_sha256"]
                .as_str()
                .is_some_and(nando_response_actor::valid_nonzero_sha256)
            && receipt["evidence_sha256"]
                .as_str()
                .is_some_and(nando_response_actor::valid_nonzero_sha256)
            && receipt["output_sha256"]
                .as_str()
                .is_some_and(nando_response_actor::valid_nonzero_sha256)
    }));

    let cross_family = v7_cross_family_projection_frame(64, 200, freeze_time.saturating_add(1_000));
    assert_eq!(
        relation_frame_routing_atom_ids(&support_and_future[0]),
        relation_frame_routing_atom_ids(&cross_family),
        "post-action family labels must not enter runtime routing atoms"
    );
    support_and_future.push(cross_family);
    write_relation_frames(&args[5], &support_and_future);
    run_with_args(&args).expect("cross-family negative cycle");
    let rejected_registry: ResponseRegistry = read_json(&args[3]).expect("rejected registry");
    assert_eq!(
        rejected_registry.packages[0].state,
        ResponsePackageState::Quarantine
    );
    assert_eq!(rejected_registry.packages[0].proof.future_rows, 32);
    assert_eq!(rejected_registry.packages[0].proof.wrong_accepts, 1);
    assert_eq!(
        ResponseExecutor::load(&args[3])
            .expect("valid quarantined registry")
            .active_package_count(),
        0
    );

    fs::remove_dir_all(root).expect("cleanup test root");
}

#[test]
fn identical_pre_action_atoms_are_reported_as_unseparable() {
    let frame = RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "a".repeat(64),
        event_id_sha256: "b".repeat(64),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: "d".repeat(64),
        observed_at_unix_nanos: 1,
        estimated_input_tokens: 0,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(false),
        atoms: vec![
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "function_call".to_owned(),
            },
        ],
        evidence_ref_sha256: "e".repeat(64),
    };
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: "test".to_owned(),
        origin: nando_response_actor::ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program: nando_response_actor::ResponseProgram::wait_on_any_yielded_cell(),
        verifier: None,
        routing_predicates: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        phase_centers: relation_frame_routing_atom_ids(&frame),
        anti_centers: Vec::new(),
        wave_margin_micro: 850_000,
        learned_wave_route: None,
        crystallized_operator: None,
        proof: nando_response_actor::ResponsePackageProof {
            support_rows: 0,
            future_rows: 0,
            distinct_sessions: 0,
            distinct_surfaces: 0,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: false,
            verifier_schema: String::new(),
            adaptive_identification: None,
        },
    };
    assert!(relation_frame_routes_to_package(&package, &frame));
    let mut legacy = package.clone();
    legacy.package_id = "legacy-template".to_owned();
    legacy.origin = ResponsePackageOrigin::LegacyTemplate;
    let runtime_registry = compile_runtime_registry(9, vec![legacy, package]);
    assert_eq!(runtime_registry.revision, 9);
    assert_eq!(runtime_registry.packages.len(), 1);
    assert_eq!(
        runtime_registry.packages[0].origin,
        ResponsePackageOrigin::GroundedSynthesis
    );
}

#[test]
fn runtime_registry_preserves_distinct_phase_profiles_for_the_same_actor() {
    let base = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: "phase-a".to_owned(),
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Active,
        program: ResponseProgram::wait_on_yielded_cell(),
        verifier: None,
        routing_predicates: Vec::new(),
        required_routing_atom_ids: vec![1],
        phase_centers: vec![1, 2],
        anti_centers: Vec::new(),
        wave_margin_micro: 1,
        learned_wave_route: None,
        crystallized_operator: None,
        proof: ResponsePackageProof {
            support_rows: 32,
            future_rows: 32,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: String::new(),
            adaptive_identification: None,
        },
    };
    let mut distinct = base.clone();
    distinct.package_id = "phase-b".to_owned();
    distinct.phase_centers.push(3);
    let registry = compile_runtime_registry(1, vec![base, distinct]);
    assert_eq!(
        registry
            .packages
            .iter()
            .filter(|package| package.state == ResponsePackageState::Active)
            .count(),
        2
    );
}

#[test]
fn causal_aggregate_cannot_pass_when_a_later_package_is_watch() {
    let report = |package_id: &str, verdict: &str| nando_response_actor::GroundedWaveCausalReport {
        schema: nando_response_actor::RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
        package_id: package_id.to_owned(),
        verdict: verdict.to_owned(),
        support_rows: 32,
        future_rows: 32,
        negative_rows: 1,
        full_phase_correct: 32,
        no_phase_correct: 32,
        shuffled_phase_correct: 0,
        random_center_correct: 0,
        magnitude_only_correct: 0,
        no_anti_center_correct: 32,
        negative_accepts: 0,
        no_phase_negative_accepts: 1,
        shuffled_negative_accepts: 0,
        random_center_negative_accepts: 0,
        magnitude_only_negative_accepts: 1,
        no_anti_center_negative_accepts: 0,
        full_margin_mean_micro: 1,
        shuffled_margin_mean_micro: 0,
        random_margin_mean_micro: 0,
        no_phase_exact_checks: 64,
        full_phase_exact_checks: 32,
    };
    let reports = BTreeMap::from([
        ("first".to_owned(), report("first", "PASS")),
        ("second".to_owned(), report("second", "WATCH")),
    ]);
    assert_eq!(
        aggregate_causal_verdict(["first", "second"], &reports),
        "WATCH"
    );
    assert_eq!(aggregate_causal_verdict(["first"], &reports), "PASS");
    assert_eq!(
        aggregate_causal_verdict(Vec::<String>::new(), &reports),
        "MISSING"
    );
}

#[test]
fn package_evidence_is_partitioned_by_operator_family() {
    let frame = |completion: &str, value_type: AtomValueType, function: &str, frame_id: char| {
        RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: frame_id.to_string().repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 0,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "source".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: completion.to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "function_call".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type,
                    source: AtomSource::Observation,
                    value_sha256: "1".repeat(64),
                },
                RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type,
                    source: AtomSource::Action,
                    value_sha256: "1".repeat(64),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                RelationAtom::ActionFunction {
                    value: function.to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: 2,
                    value_type: Some(value_type),
                },
            ],
            evidence_ref_sha256: "e".repeat(64),
        }
    };
    let source = frame("completed", AtomValueType::String, "route_result", '1');
    let wait = frame("pending", AtomValueType::Identifier, "wait", '2');
    let package = |program: nando_response_actor::ResponseProgram, support: &RelationFrame| {
        let required_routing_atom_ids =
            nando_response_actor::response_program_required_routing_atom_ids(&program);
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: format!("package-{}", support.frame_id_sha256),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Quarantine,
            program,
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids,
            phase_centers: relation_frame_routing_atom_ids(support),
            anti_centers: Vec::new(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: nando_response_actor::ResponsePackageProof {
                support_rows: 1,
                future_rows: 0,
                distinct_sessions: 1,
                distinct_surfaces: 1,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: false,
                verifier_schema: String::new(),
                adaptive_identification: None,
            },
        }
    };
    let source_package = package(
        nando_response_actor::ResponseProgram::function_call_from_roles(
            "route_result",
            nando_response_actor::ResponseValueSelector::UniqueScalar {
                value_type: nando_response_actor::AtomValueType::String,
            },
            vec![nando_response_actor::ResponseArgument::Role {
                name: "value".to_owned(),
                role: nando_response_actor::SemanticRole::SourceValue,
                value_type: Some(nando_response_actor::AtomValueType::String),
            }],
        ),
        &source,
    );
    let wait_package = package(
        nando_response_actor::ResponseProgram::function_call_from_roles(
            "wait",
            nando_response_actor::ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: nando_response_actor::AtomValueType::Identifier,
            },
            vec![nando_response_actor::ResponseArgument::Role {
                name: "value".to_owned(),
                role: nando_response_actor::SemanticRole::ContinuationHandle,
                value_type: Some(nando_response_actor::AtomValueType::Identifier),
            }],
        ),
        &wait,
    );
    let frames = [source.clone(), wait.clone()];
    let source_negatives = package_negative_frame_refs(&source_package, &[source], &frames);
    let wait_negatives = package_negative_frame_refs(&wait_package, &[wait], &frames);
    assert_eq!(source_negatives.len(), 1);
    assert_eq!(source_negatives[0].frame_id_sha256, "2".repeat(64));
    assert_eq!(wait_negatives.len(), 1);
    assert_eq!(wait_negatives[0].frame_id_sha256, "1".repeat(64));
}

#[test]
fn custom_tool_support_is_not_classified_as_its_own_negative() {
    let frame = RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: "a".repeat(64),
        event_id_sha256: "b".repeat(64),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: "d".repeat(64),
        observed_at_unix_nanos: 1,
        estimated_input_tokens: 0,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::ToolKind {
                value: "exec".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "custom_tool_call".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Integer,
                source: AtomSource::Action,
                value_sha256: "1".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: nando_response_actor::ResponseValueSelector::JsonField {
                    field: "session_id".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::ActionCustomTool {
                value: "exec".to_owned(),
            },
            RelationAtom::ActionInnerTool {
                value: "write_stdin".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 2,
                value_type: Some(AtomValueType::Integer),
            },
            RelationAtom::ActionJsonResultProjection,
        ],
        evidence_ref_sha256: "e".repeat(64),
    };
    let program = nando_response_actor::ResponseProgram::custom_tool_call_from_roles(
        "exec",
        "write_stdin",
        nando_response_actor::ResponseValueSelector::JsonField {
            field: "session_id".to_owned(),
            value_type: AtomValueType::Integer,
        },
        vec![nando_response_actor::ResponseArgument::Role {
            name: "session_id".to_owned(),
            role: nando_response_actor::SemanticRole::ContinuationHandle,
            value_type: Some(AtomValueType::Integer),
        }],
        nando_response_actor::CustomToolResultProjection::JsonStringifyResult,
    );
    let required_routing_atom_ids =
        nando_response_actor::response_program_required_routing_atom_ids(&program);
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: "custom".to_owned(),
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program,
        verifier: None,
        routing_predicates: Vec::new(),
        required_routing_atom_ids,
        phase_centers: relation_frame_routing_atom_ids(&frame),
        anti_centers: Vec::new(),
        wave_margin_micro: 850_000,
        learned_wave_route: None,
        crystallized_operator: None,
        proof: nando_response_actor::ResponsePackageProof {
            support_rows: 1,
            future_rows: 0,
            distinct_sessions: 1,
            distinct_surfaces: 1,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: false,
            verifier_schema: String::new(),
            adaptive_identification: None,
        },
    };
    assert!(
        package_negative_frame_refs(
            &package,
            std::slice::from_ref(&frame),
            std::slice::from_ref(&frame),
        )
        .is_empty()
    );
}

#[test]
fn evidence_refresh_requires_more_independent_sessions_without_inheriting_authority() {
    let manifest = |package_id: &str,
                    generation: u64,
                    supersedes_package_id: Option<&str>,
                    support_prefix: &str,
                    support_rows: usize,
                    reserved_sessions: usize| {
        ResponseSupportManifest {
            schema: "nando.response-support-manifest.v1".to_owned(),
            package_id: package_id.to_owned(),
            lineage_id: "lineage".to_owned(),
            generation,
            routing_refinement_version: ROUTING_REFINEMENT_VERSION,
            supersedes_package_id: supersedes_package_id.map(str::to_owned),
            created_at_unix_nanos: generation,
            support_boundary_unix_nanos: generation,
            support_frame_ids: (0..support_rows)
                .map(|index| format!("{support_prefix}-{index}"))
                .collect(),
            support_session_ids: vec![format!("support-{generation}")],
            support_intent_ids: vec![format!("intent-{generation}")],
            reserved_future_session_ids: (0..reserved_sessions)
                .map(|index| format!("reserved-{index}"))
                .collect(),
            learned_center_atom_ids: vec![1],
            learned_anti_center_atom_ids: Vec::new(),
            selected_routing_atom_ids: Vec::new(),
            selected_routing_predicates: Vec::new(),
            split_negative_frame_ids: Vec::new(),
            holdout_negative_frame_ids: Vec::new(),
            split_parent_support_rows: support_rows,
            manifest_sha256: format!("manifest-{generation}"),
        }
    };
    let current = manifest("g1", 1, None, "old", 32, 0);
    let improved = manifest("g2", 2, Some("g1"), "new", 32, 3);
    assert!(evidence_refresh_improves(&current, &improved));

    let same_support = manifest("g2", 2, Some("g1"), "old", 32, 3);
    assert!(!evidence_refresh_improves(&current, &same_support));
    let mut legacy_policy = current.clone();
    legacy_policy.routing_refinement_version = 0;
    let policy_migration = manifest("g2", 2, Some("g1"), "old", 32, 0);
    assert!(evidence_refresh_improves(&legacy_policy, &policy_migration));
    let undersized = manifest("g2", 2, Some("g1"), "new", 31, 3);
    assert!(!evidence_refresh_improves(&current, &undersized));
    let authority_mismatch = manifest("g2", 2, Some("other"), "new", 32, 3);
    assert!(!evidence_refresh_improves(&current, &authority_mismatch));
}

#[test]
fn rollover_requires_a_new_route_contract_or_material_support_gain() {
    let manifest =
        |package_id: &str, generation: u64, support_rows: usize| ResponseSupportManifest {
            schema: "nando.response-support-manifest.v1".to_owned(),
            package_id: package_id.to_owned(),
            lineage_id: "lineage".to_owned(),
            generation,
            routing_refinement_version: ROUTING_REFINEMENT_VERSION,
            supersedes_package_id: (generation > 1).then(|| "g1".to_owned()),
            created_at_unix_nanos: generation,
            support_boundary_unix_nanos: generation,
            support_frame_ids: (0..support_rows)
                .map(|index| format!("f-{index}"))
                .collect(),
            support_session_ids: vec!["session".to_owned()],
            support_intent_ids: vec!["intent".to_owned()],
            reserved_future_session_ids: Vec::new(),
            learned_center_atom_ids: vec![1, 2],
            learned_anti_center_atom_ids: Vec::new(),
            selected_routing_atom_ids: Vec::new(),
            selected_routing_predicates: Vec::new(),
            split_negative_frame_ids: Vec::new(),
            holdout_negative_frame_ids: Vec::new(),
            split_parent_support_rows: support_rows,
            manifest_sha256: String::new(),
        };
    let current = manifest("g1", 1, 64);
    let repeated = manifest("g2", 2, 64);
    assert!(!rollover_manifest_improves(&current, &repeated));

    let mut center_drift = repeated.clone();
    center_drift.learned_center_atom_ids.push(3);
    assert!(!rollover_manifest_improves(&current, &center_drift));

    let expanded = manifest("g2", 2, 96);
    assert!(rollover_manifest_improves(&current, &expanded));

    let mut refined = repeated;
    refined.selected_routing_atom_ids.push(3);
    assert!(rollover_manifest_improves(&current, &refined));
}

#[test]
fn token_opportunity_dedupes_replayed_events() {
    let frame = |id: char, tokens: u64, label| RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: id.to_string().repeat(64),
        event_id_sha256: "event".repeat(12) + &id.to_string(),
        client_intent_id_sha256: "c".repeat(64),
        session_id_sha256: "d".repeat(64),
        observed_at_unix_nanos: 1,
        estimated_input_tokens: tokens,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: label,
        atoms: Vec::new(),
        evidence_ref_sha256: "e".repeat(64),
    };
    let report = token_opportunity_report(&[
        frame('a', 100, Some(true)),
        frame('b', 90, Some(true)),
        frame('c', 50, Some(false)),
    ]);
    assert_eq!(report["deduplicated_events"], 3);
    assert_eq!(report["deduplicated_input_tokens"], 240);
}

#[test]
fn verified_future_self_training_keeps_a_newer_frozen_exam() {
    let mut future = Vec::new();
    for session in 0_u64..6 {
        for row in 0_u64..11 {
            future.push(RelationFrame {
                schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
                frame_id_sha256: sha256_text(format!("frame-{session}-{row}")),
                event_id_sha256: sha256_text(format!("event-{session}-{row}")),
                client_intent_id_sha256: sha256_text(format!("intent-{session}-{row}")),
                session_id_sha256: sha256_text(format!("session-{session}")),
                observed_at_unix_nanos: session * 1_000 + row,
                estimated_input_tokens: 10,
                extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION
                    .to_owned(),
                verifier_label: Some(true),
                atoms: Vec::new(),
                evidence_ref_sha256: sha256_text(format!("evidence-{session}-{row}")),
            });
        }
    }
    let selected = verified_future_sessions_for_self_training(&future);
    assert_eq!(selected.len(), 3);
    for session in 0_u64..3 {
        assert!(selected.contains(&sha256_text(format!("session-{session}"))));
    }
    for session in 3_u64..6 {
        assert!(!selected.contains(&sha256_text(format!("session-{session}"))));
    }
}

#[test]
fn verified_future_self_training_rejects_small_or_unverified_evidence() {
    let frame = |session: u64, row: u64, label| RelationFrame {
        schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: sha256_text(format!("frame-{session}-{row}-{label:?}")),
        event_id_sha256: sha256_text(format!("event-{session}-{row}-{label:?}")),
        client_intent_id_sha256: sha256_text(format!("intent-{session}-{row}-{label:?}")),
        session_id_sha256: sha256_text(format!("session-{session}")),
        observed_at_unix_nanos: session * 1_000 + row,
        estimated_input_tokens: 10,
        extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: label,
        atoms: Vec::new(),
        evidence_ref_sha256: sha256_text(format!("evidence-{session}-{row}-{label:?}")),
    };
    let too_few_sessions = (0_u64..64)
        .map(|row| frame(row % 5, row, Some(true)))
        .collect::<Vec<_>>();
    assert!(verified_future_sessions_for_self_training(&too_few_sessions).is_empty());

    let unverified = (0_u64..66)
        .map(|row| frame(row % 6, row, Some(false)))
        .collect::<Vec<_>>();
    assert!(verified_future_sessions_for_self_training(&unverified).is_empty());
}

#[test]
fn quarantined_registry_package_has_no_execution_authority() {
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: "quarantined".to_owned(),
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program: ResponseProgram::wait_on_yielded_cell(),
        verifier: None,
        routing_predicates: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        phase_centers: vec![1],
        anti_centers: Vec::new(),
        wave_margin_micro: 1,
        learned_wave_route: None,
        crystallized_operator: None,
        proof: nando_response_actor::ResponsePackageProof {
            support_rows: 32,
            future_rows: 31,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: String::new(),
            adaptive_identification: None,
        },
    };
    let registry = compile_runtime_registry(4, vec![package]);
    let executor = ResponseExecutor::from_registry(registry).expect("valid v5 registry");
    assert_eq!(executor.active_package_count(), 0);
}

#[test]
fn cold_collection_evidence_builds_generic_quarantine_without_authority() {
    let mut rows = Vec::new();
    for session in 0_u64..7 {
        let (collection, predicate, projected) = if session % 2 == 0 {
            ("rows", "kind", "value")
        } else {
            ("entries", "tag", "amount")
        };
        let count = if session < 4 { 10 } else { 11 };
        for ordinal in 0..count {
            let output = serde_json::json!({
                (collection): [
                    {(predicate):"keep", (projected):3},
                    {(predicate):"drop", (projected):4},
                    {(predicate):"keep", (projected):5}
                ]
            });
            rows.push(ColdCollectionRow {
                frame_id_sha256: sha256_text(format!("frame-{session}-{ordinal}")),
                session_id_sha256: sha256_text(format!("session-{session}")),
                client_intent_id_sha256: sha256_text(format!("intent-{session}-{ordinal}")),
                observed_at_unix_nanos: session * 1_000 + ordinal,
                surface_sha256: sha256_text(format!("{collection}:{predicate}:{projected}")),
                phase_valid: true,
                request_phase_atom_ids: Vec::new(),
                example: CollectionSynthesisExample {
                    provider_payload: serde_json::json!({
                        "input":[{"type":"function_call_output","output":output.to_string()}]
                    }),
                    expected_response: "[3,5]".to_owned(),
                },
            });
        }
    }
    let package = compile_collection_quarantine_package(&rows).expect("collection package");
    assert_eq!(package.state, ResponsePackageState::Quarantine);
    assert_eq!(package.proof.support_rows, 40);
    assert_eq!(package.proof.future_rows, 33);
    assert_eq!(package.proof.distinct_sessions, 3);
    assert_eq!(package.proof.wrong_accepts, 0);
    assert!(package.proof.wave_causal_pass);
    assert!(matches!(
        package.program.operation,
        ResponseOperation::ComposeCollection { .. }
    ));
    assert!(!package.eligible_for_admission_candidate());
    let mut mixed = rows.clone();
    mixed.extend(rows.iter().cloned().map(|mut row| {
        row.frame_id_sha256 = sha256_text(format!("drop:{}", row.frame_id_sha256));
        row.client_intent_id_sha256 = sha256_text(format!("drop:{}", row.client_intent_id_sha256));
        row.example.expected_response = "[4]".to_owned();
        row
    }));
    let split = collection_families(&mixed);
    assert_eq!(split.len(), 2);
    assert!(split.iter().all(|family| family.len() == rows.len()));
    let manifest = build_collection_support_manifest(&rows, &package).expect("manifest");
    let future_package =
        compile_collection_package(&rows, Some(&manifest)).expect("future package");
    assert_eq!(future_package.state, ResponsePackageState::Active);
    assert!(future_package.eligible_for_admission_candidate());
    let relation_rows = rows
        .iter()
        .map(|row| RelationFrame {
            schema: nando_response_actor::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: row.frame_id_sha256.clone(),
            event_id_sha256: sha256_text(format!("event:{}", row.frame_id_sha256)),
            client_intent_id_sha256: row.client_intent_id_sha256.clone(),
            session_id_sha256: row.session_id_sha256.clone(),
            observed_at_unix_nanos: row.observed_at_unix_nanos,
            estimated_input_tokens: 10,
            extractor_version: nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::CollectionShape {
                    array_fields: 1,
                    row_fields: 2,
                },
                RelationAtom::ResponseShape {
                    value: "assistant_message".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
            ],
            evidence_ref_sha256: sha256_text(format!("evidence:{}", row.frame_id_sha256)),
        })
        .collect::<Vec<_>>();
    let support_ids = manifest
        .support_frame_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let causal_support = relation_rows
        .iter()
        .filter(|frame| support_ids.contains(&frame.frame_id_sha256))
        .cloned()
        .collect::<Vec<_>>();
    let causal_future = relation_rows
        .iter()
        .filter(|frame| frame.observed_at_unix_nanos > manifest.support_boundary_unix_nanos)
        .cloned()
        .collect::<Vec<_>>();
    let causal = nando_response_actor::evaluate_grounded_wave_causality(
        &future_package,
        &causal_support,
        &causal_future,
        &[],
    );
    assert_eq!(causal.verdict, "PASS", "{causal:?}");
    let executor =
        ResponseExecutor::from_registry(compile_runtime_registry(9, vec![future_package]))
            .expect("diagnostic registry");
    assert_eq!(executor.active_package_count(), 0);
}

#[test]
fn miner_cycle_builds_collection_manifest_receipts_and_authority_candidate() {
    let root = env::temp_dir().join(format!(
        "nando-collection-miner-cycle-{}-{}",
        std::process::id(),
        unix_now()
    ));
    fs::create_dir_all(&root).expect("root");
    let args = [
        "relations.jsonl",
        "shadows.jsonl",
        "causal.json",
        "registry.json",
        "status.json",
        "frames.jsonl",
        "manifests.json",
        "receipts.json",
        "grounded-causal.json",
        "parity.json",
    ]
    .map(|name| root.join(name));
    fs::write(&args[0], "").expect("relations");
    fs::write(&args[1], "").expect("shadows");
    atomic_write_value(
        &args[2],
        &serde_json::json!({
            "schema":"nando.response-wave-causal-proof.v1",
            "verdict":"PASS",
            "heldout_correct":32,
            "heldout_total":32,
            "full_phase_exact_checks":32,
            "no_phase_exact_checks":64,
            "shuffled_phase_exact_checks":64,
        }),
    )
    .expect("global causal");
    let mut lines = String::new();
    for session in 0_u64..7 {
        let (collection, predicate, projected) = if session % 2 == 0 {
            ("rows", "kind", "value")
        } else {
            ("entries", "tag", "amount")
        };
        let count = if session < 4 { 10 } else { 11 };
        for ordinal in 0..count {
            let first_value = 3 + session;
            let middle_value = 4 + session;
            let last_value = 5 + session;
            let output = serde_json::json!({
                (collection): [
                    {(predicate):"keep", (projected):first_value},
                    {(predicate):"drop", (projected):middle_value},
                    {(predicate):"keep", (projected):last_value}
                ]
            });
            let project_expected = format!("[{first_value},{last_value}]");
            for (family, expected_response, request_atom) in [
                (
                    "project",
                    project_expected.as_str(),
                    13_665_181_768_394_347_299_u64,
                ),
                ("count", "2", 15_291_052_347_829_727_369_u64),
            ] {
                let provider_payload = serde_json::json!({
                    "input":[{"type":"function_call_output","output":output.to_string()}]
                });
                let cold = ColdCollectionEvidence {
                    schema: "nando.response-collection-synthesis-example.v1".to_owned(),
                    provider_payload,
                    expected_response: expected_response.to_owned(),
                };
                let frame_id = sha256_text(format!("cycle-{family}-frame-{session}-{ordinal}"));
                let row = serde_json::json!({
                    "schema": nando_response_actor::RELATION_FRAME_SCHEMA,
                    "frame_id_sha256": frame_id,
                    "event_id_sha256": sha256_text(format!("cycle-{family}-event-{session}-{ordinal}")),
                    "client_intent_id_sha256": sha256_text(format!("cycle-{family}-intent-{session}-{ordinal}")),
                    "session_id_sha256": sha256_text(format!("cycle-{family}-session-{session}")),
                    "observed_at_unix_nanos": session * 10_000 + ordinal as u64 * 2 + u64::from(family == "count") + 1,
                    "estimated_input_tokens": 100,
                    "extractor_version": nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION,
                    "verifier_label": true,
                    "atoms": [
                        {"kind":"collection_shape","array_fields":1,"row_fields":2},
                        {"kind":"request_phase_atom","atom_id":request_atom},
                        {"kind":"observation_call_shape","value":"function_call"},
                        {"kind":"response_shape","value":"assistant_message"},
                        {"kind":"completion_state","value":"completed"}
                    ],
                    "evidence_ref_sha256": canonical_json_sha256(&cold).expect("cold digest"),
                    "cold_collection_example": cold,
                });
                lines.push_str(&serde_json::to_string(&row).expect("row"));
                lines.push('\n');
            }
        }
    }
    fs::write(&args[5], lines).expect("frames");
    run_with_args(&args).expect("miner cycle");
    let registry: ResponseRegistry = read_json(&args[3]).expect("registry");
    assert_eq!(registry.schema, RESPONSE_REGISTRY_SCHEMA_V6);
    assert_eq!(registry.packages.len(), 2);
    assert!(
        registry
            .packages
            .iter()
            .all(|package| package.state == ResponsePackageState::Active)
    );
    assert!(
        registry
            .packages
            .iter()
            .all(ResponsePackage::eligible_for_admission_candidate)
    );
    let manifests: ResponseSupportManifestSet = read_json(&args[6]).expect("manifests");
    assert_eq!(manifests.manifests.len(), 2);
    assert!(
        manifests
            .manifests
            .iter()
            .all(|manifest| manifest.support_frame_ids.len() == 40)
    );
    let status: Value = read_json(&args[4]).expect("status");
    assert_eq!(status["collection_synthesis"]["future_rows"], 66);
    assert_eq!(status["collection_synthesis"]["future_wrong_accepts"], 0);
    let bindings: Vec<ResponsePackageAuthorityBindingV2> =
        serde_json::from_value(status["response_authority_candidate"]["packages"].clone())
            .expect("authority bindings");
    assert_eq!(bindings.len(), 2);
    let now = unix_now();
    let gate_build = sha256_text("gate-build");
    let runtime_build = sha256_text("runtime-build");
    let admission = nando_response_actor::CompositeResponseAdmissionV2 {
        schema: nando_response_actor::COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
        project_id: "nando-wave".to_owned(),
        generated_at_unix: now,
        expires_at_unix: now + 30,
        verdict: "PASS".to_owned(),
        eligible_for_local_accept: true,
        response_authority: nando_response_actor::ResponseAuthorityV2 {
            schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
            registry_schema: registry.schema.clone(),
            registry_revision: registry.revision,
            registry_sha256: response_registry_digest(&registry).expect("registry digest"),
            gate_build_sha256: gate_build.clone(),
            runtime_build_sha256: runtime_build.clone(),
            packages: bindings,
        },
    };
    let executor = ResponseExecutor::from_registry_with_admission(
        registry,
        admission,
        "nando-wave",
        &gate_build,
        &runtime_build,
        now,
        30,
    )
    .expect("authorized executor");
    let heldout = serde_json::json!({
        "input":[{"type":"function_call_output","output":serde_json::json!({
            "records":[
                {"marker":"keep","score":3},
                {"marker":"drop","score":4},
                {"marker":"keep","score":5}
            ]
        }).to_string()}]
    });
    let execution = executor.execute("project", &heldout);
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        execution.reason
    );
    assert_eq!(execution.response.as_deref(), Some("[3,5]"));
    let count_execution = executor.execute("count", &heldout);
    assert_eq!(
        count_execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        count_execution.reason
    );
    assert_eq!(count_execution.response.as_deref(), Some("2"));
    let projected = serde_json::json!({"output_text":"[3,5]"});
    let runtime_receipt = executor
        .finalize_runtime_receipt(
            &execution,
            &sha256_text("heldout-request"),
            "test-projector.v1",
            &sha256_text("test-projector"),
            &projected,
        )
        .expect("runtime receipt");
    assert_eq!(
        runtime_receipt.receipt.schema,
        nando_response_actor::RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2
    );
    let negative_output = serde_json::json!({
        "records":[
            {"marker":"keep","score":3},
            {"marker":"drop","score":4},
            {"marker":"keep","score":5}
        ]
    });
    let negative_cold = ColdCollectionEvidence {
        schema: "nando.response-collection-synthesis-example.v1".to_owned(),
        provider_payload: serde_json::json!({
            "input":[{"type":"function_call_output","output":negative_output.to_string()}]
        }),
        expected_response: "[999]".to_owned(),
    };
    let negative_row = serde_json::json!({
        "schema": nando_response_actor::RELATION_FRAME_SCHEMA,
        "frame_id_sha256": sha256_text("drift-frame"),
        "event_id_sha256": sha256_text("drift-event"),
        "client_intent_id_sha256": sha256_text("drift-intent"),
        "session_id_sha256": sha256_text("drift-session"),
        "observed_at_unix_nanos": 100_000_u64,
        "estimated_input_tokens": 100,
        "extractor_version": nando_response_actor::SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        "verifier_label": true,
        "atoms": [
            {"kind":"collection_shape","array_fields":1,"row_fields":2},
            {"kind":"request_phase_atom","atom_id":13_665_181_768_394_347_299_u64},
            {"kind":"observation_call_shape","value":"function_call"},
            {"kind":"response_shape","value":"assistant_message"},
            {"kind":"completion_state","value":"completed"}
        ],
        "evidence_ref_sha256": canonical_json_sha256(&negative_cold).expect("negative digest"),
        "cold_collection_example": negative_cold,
    });
    let mut frame_file = fs::OpenOptions::new()
        .append(true)
        .open(&args[5])
        .expect("frames append");
    writeln!(
        frame_file,
        "{}",
        serde_json::to_string(&negative_row).expect("negative row")
    )
    .expect("negative append");
    run_with_args(&args).expect("drift miner cycle");
    let drift_registry: ResponseRegistry = read_json(&args[3]).expect("drift registry");
    assert_eq!(
        drift_registry
            .packages
            .iter()
            .filter(|package| package.state == ResponsePackageState::Active)
            .count(),
        1
    );
    let demoted = drift_registry
        .packages
        .iter()
        .find(|package| package.proof.wrong_accepts > 0)
        .expect("demoted package");
    assert_eq!(demoted.state, ResponsePackageState::Quarantine);
    let drift_status: Value = read_json(&args[4]).expect("drift status");
    assert_eq!(
        drift_status["collection_synthesis"]["future_wrong_accepts"],
        1
    );
    assert_eq!(
        drift_status["response_authority_candidate"]["packages"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    fs::remove_dir_all(root).expect("cleanup");
}
