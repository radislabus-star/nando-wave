//! Adaptive version-space freeze and transfer proof tests.

use super::*;

fn adaptive_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nando-online-collection-adaptive-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

fn adaptive_count_program() -> ResponseProgram {
    ResponseProgram::compose_collection(
        vec![
            crate::CollectionProgramStep::SelectOnlyArrayField,
            crate::CollectionProgramStep::Count,
        ],
        crate::ValueProjectionFormat::PlainText,
        "completed",
    )
}

fn adaptive_static_frame_prefix() -> String {
    format!("Result:\n{}\n", "stable frame ".repeat(48))
}

fn adaptive_static_frame_program() -> ResponseProgram {
    ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::UniqueTurnScalar {
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    )
    .with_value_renderer(crate::CollectionOutputRenderer::RenderSequence {
        segments: vec![
            crate::ResponseRenderSegment::Static {
                text: adaptive_static_frame_prefix(),
            },
            crate::ResponseRenderSegment::Primary,
        ],
    })
}

fn adaptive_static_frame_observation(index: usize, value: i64) -> OnlineCollectionObservation {
    OnlineCollectionObservation {
        evidence_graph_sha256: format!("{index:064x}"),
        client_intent_id_sha256: format!("{:064x}", index + 30_000),
        session_id_sha256: format!("{:064x}", index + 40_000),
        event_time_unix_nanos: Some(index as u64),
        estimated_input_tokens: 100,
        example: CollectionSynthesisExample {
            provider_payload: serde_json::json!({
                "input": [
                    {"type":"message","role":"user","content":[{
                        "type":"input_text","text":"Report the result"
                    }]},
                    {"type":"function_call_output","output":serde_json::json!({
                        "value": value
                    }).to_string()}
                ]
            }),
            expected_response: format!("{}{value}", adaptive_static_frame_prefix()),
        },
    }
}

fn adaptive_bucket(
    bucket_id: &str,
    archetype_id: &str,
    programs: BTreeMap<String, ResponseProgram>,
    support_observation: &OnlineCollectionObservation,
) -> OnlineCollectionBucket {
    let support = receipt_with_program_atoms(support_observation, true, &programs)
        .expect("adaptive support receipt");
    OnlineCollectionBucket {
        bucket_id: bucket_id.to_owned(),
        archetype_id: archetype_id.to_owned(),
        programs,
        common_request_atom_ids: observation_request_atom_ids(support_observation),
        support: vec![support],
        future: Vec::new(),
        runtime_examples: BTreeMap::from([(
            support_observation.evidence_graph_sha256.clone(),
            support_observation.example.clone(),
        )]),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    }
}

#[test]
fn adaptive_singleton_freezes_after_one_support_without_fixed_rows() {
    let root = adaptive_root("singleton");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let config = OnlineCollectionConfig::default();
    assert_eq!(
        config.proof_mode,
        OnlineCollectionProofMode::AdaptiveVersionSpace
    );

    let support = observation(1, "3");
    let program = adaptive_count_program();
    let digest = canonical_json_sha256(&program).expect("program digest");
    let archetype = response_program_archetype_id(&program).expect("archetype");
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"1".repeat(64),
        &archetype,
        BTreeMap::from([(digest.clone(), program)]),
        &support,
    ));

    miner.maybe_freeze(0).expect("adaptive freeze");
    let bucket = &miner.checkpoint.buckets[0];
    assert_eq!(bucket.support.len(), 1);
    assert_eq!(
        bucket.frozen_program_sha256.as_deref(),
        Some(digest.as_str())
    );
    assert!(bucket.adaptive_candidate_freeze.is_some());
    assert!(bucket.support_manifest_sha256.is_some());
    assert_eq!(
        miner.status().buckets[0].admission_blocker.as_deref(),
        Some("adaptive_future_missing")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn legacy_frozen_singleton_migrates_to_adaptive_without_creating_future() {
    let root = adaptive_root("legacy-frozen-migration");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let support = observation(1, "3");
    let program = adaptive_count_program();
    let digest = canonical_json_sha256(&program).expect("program digest");
    let archetype = response_program_archetype_id(&program).expect("archetype");
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"d".repeat(64),
        &archetype,
        BTreeMap::from([(digest.clone(), program)]),
        &support,
    ));
    miner.maybe_freeze(0).expect("initial freeze");
    let support_manifest = miner.checkpoint.buckets[0].support_manifest_sha256.clone();
    let support_watermark = miner.checkpoint.buckets[0].support_watermark_event_time_unix_nanos;

    miner.checkpoint.buckets[0].adaptive_candidate_freeze = None;
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V36;
    miner.persist().expect("persist legacy frozen checkpoint");
    drop(miner);

    let migrated =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("migrate");
    let bucket = &migrated.checkpoint.buckets[0];
    assert_eq!(
        bucket.frozen_program_sha256.as_deref(),
        Some(digest.as_str())
    );
    assert!(bucket.adaptive_candidate_freeze.is_some());
    assert_eq!(bucket.support_manifest_sha256, support_manifest);
    assert_eq!(
        bucket.support_watermark_event_time_unix_nanos,
        support_watermark
    );
    assert!(bucket.future.is_empty());
    assert!(bucket.durable_runtime_parity_receipts.is_empty());
    assert_eq!(
        migrated.status().buckets[0].admission_blocker.as_deref(),
        Some("adaptive_future_missing")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn adaptive_ambiguous_version_space_does_not_freeze() {
    let root = adaptive_root("ambiguous");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let support = observation(1, "3");
    let direct = adaptive_count_program();
    let ordinal = ResponseProgram::compose_collection(
        vec![
            crate::CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 },
            crate::CollectionProgramStep::SelectOnlyArrayField,
            crate::CollectionProgramStep::Count,
        ],
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    assert_eq!(
        independently_verified_authority_response(&direct, &support.example).as_deref(),
        Some("3")
    );
    assert_eq!(
        independently_verified_authority_response(&ordinal, &support.example).as_deref(),
        Some("3")
    );
    let programs = [direct, ordinal]
        .into_iter()
        .map(|program| {
            let digest = canonical_json_sha256(&program).expect("program digest");
            (digest, program)
        })
        .collect::<BTreeMap<_, _>>();
    let archetype =
        response_program_archetype_id(programs.values().next().expect("candidate program"))
            .expect("archetype");
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"2".repeat(64),
        &archetype,
        programs,
        &support,
    ));

    miner.maybe_freeze(0).expect("adaptive evaluation");
    let bucket = &miner.checkpoint.buckets[0];
    assert!(bucket.frozen_program_sha256.is_none());
    assert!(bucket.adaptive_candidate_freeze.is_none());
    assert_eq!(
        miner.status().buckets[0].admission_blocker.as_deref(),
        Some("adaptive_version_space_ambiguous_2")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn adaptive_future_requires_new_intent_and_seals_transfer_proof() {
    let root = adaptive_root("future");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let support = observation(1, "3");
    let program = adaptive_count_program();
    let digest = canonical_json_sha256(&program).expect("program digest");
    let archetype = response_program_archetype_id(&program).expect("archetype");
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"3".repeat(64),
        &archetype,
        BTreeMap::from([(digest, program.clone())]),
        &support,
    ));
    miner.maybe_freeze(0).expect("adaptive freeze");

    let mut reused = support.clone();
    reused.evidence_graph_sha256 = "4".repeat(64);
    reused.event_time_unix_nanos = Some(2);
    assert!(
        miner
            .evaluate_frozen_candidates(&reused)
            .expect("support-reuse evaluation")
    );
    assert!(miner.checkpoint.buckets[0].future.is_empty());
    assert_eq!(miner.checkpoint.future_intent_rejected_total, 1);

    let mut future = support;
    future.evidence_graph_sha256 = "5".repeat(64);
    future.client_intent_id_sha256 = "6".repeat(64);
    future.session_id_sha256 = "7".repeat(64);
    future.event_time_unix_nanos = Some(3);
    assert!(
        miner
            .evaluate_frozen_candidates(&future)
            .expect("independent future evaluation")
    );
    let bucket = &miner.checkpoint.buckets[0];
    assert_eq!(bucket.future.len(), 1);
    assert_eq!(bucket.durable_runtime_parity_receipts.len(), 1);
    assert!(miner.status().buckets[0].admission_blocker.is_none());

    let package = miner
        .quarantine_packages()
        .expect("quarantine packages")
        .into_iter()
        .next()
        .expect("adaptive package");
    let proof = package
        .proof
        .adaptive_identification
        .as_ref()
        .expect("adaptive identification proof");
    proof.validate().expect("sealed adaptive proof");
    assert_eq!(package.proof.support_rows, 1);
    assert_eq!(package.proof.future_rows, 1);
    assert_eq!(package.proof.distinct_sessions, 2);
    assert_eq!(package.proof.wrong_accepts, 0);
    assert_eq!(
        proof.canonical_program_root_sha256(),
        nando_operator_kernel::response_program_version_root_sha256(&program)
            .expect("version root")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn adaptive_package_reaches_external_admission_after_one_independent_future() {
    let root = adaptive_root("external-admission");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let support = observation(1, "3");
    let program = adaptive_count_program();
    let digest = canonical_json_sha256(&program).expect("program digest");
    let archetype = response_program_archetype_id(&program).expect("archetype");
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"8".repeat(64),
        &archetype,
        BTreeMap::from([(digest, program.clone())]),
        &support,
    ));
    miner.maybe_freeze(0).expect("adaptive freeze");

    let mut future = support.clone();
    future.evidence_graph_sha256 = "9".repeat(64);
    future.client_intent_id_sha256 = "a".repeat(64);
    future.session_id_sha256 = "b".repeat(64);
    future.event_time_unix_nanos = Some(2);
    assert!(
        miner
            .evaluate_frozen_candidates(&future)
            .expect("independent future")
    );

    // This unresolved alternative is part of the no-phase search space. It
    // cannot receive authority, but proves that phase routing avoids an exact
    // check that an unranked search would perform.
    let mut alternative = adaptive_bucket(
        &"c".repeat(64),
        &archetype,
        BTreeMap::from([(
            canonical_json_sha256(&program).expect("alternative digest"),
            program,
        )]),
        &support,
    );
    alternative.support.clear();
    alternative.runtime_examples.clear();
    miner.checkpoint.buckets.push(alternative);

    let candidates = miner.admission_candidates().expect("admission candidates");
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.causal_report.verdict, "PASS");
    assert_eq!(candidate.package.proof.support_rows, 1);
    assert_eq!(candidate.package.proof.future_rows, 1);

    let snapshot = crate::build_online_collection_admission_snapshot(
        &candidates,
        "project",
        1,
        100,
        60,
        &"d".repeat(64),
        &"e".repeat(64),
    )
    .expect("external admission")
    .expect("active snapshot");
    assert_eq!(snapshot.registry.packages.len(), 1);
    assert!(snapshot.admission.eligible_for_local_accept);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn adaptive_static_frame_requires_transfer_to_a_new_dynamic_value() {
    let root = adaptive_root("static-frame-transfer");
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let support = adaptive_static_frame_observation(101, 7);
    let program = adaptive_static_frame_program();
    assert!(adaptive_static_frame_prefix().len() > 512);
    program.validate().expect("intrinsically safe static frame");
    assert!(response_program_requires_static_frame_transfer(&program));
    let digest = canonical_json_sha256(&program).expect("program digest");
    let archetype = response_program_archetype_id(&program).expect("archetype");
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("miner");
    miner.checkpoint.buckets.push(adaptive_bucket(
        &"f".repeat(64),
        &archetype,
        BTreeMap::from([(digest, program.clone())]),
        &support,
    ));
    miner.maybe_freeze(0).expect("adaptive freeze");

    let same_value = adaptive_static_frame_observation(102, 7);
    assert!(
        miner
            .evaluate_frozen_candidates(&same_value)
            .expect("same-value future")
    );
    let same_value_package = miner
        .quarantine_packages()
        .expect("same-value package")
        .into_iter()
        .next()
        .expect("quarantine package");
    assert!(same_value_package.proof.adaptive_identification.is_none());

    let transferred = adaptive_static_frame_observation(103, 11);
    assert!(
        miner
            .evaluate_frozen_candidates(&transferred)
            .expect("transferred future")
    );
    let transferred_package = miner
        .quarantine_packages()
        .expect("transferred package")
        .into_iter()
        .next()
        .expect("quarantine package");
    transferred_package
        .proof
        .adaptive_identification
        .as_ref()
        .expect("static frame transfer proof")
        .validate()
        .expect("valid transfer proof");
    miner.persist().expect("persist static frame proof");
    drop(miner);
    let mut miner =
        OnlineCollectionMiner::open(&path, OnlineCollectionConfig::default()).expect("restart");
    let restarted_package = miner
        .quarantine_packages()
        .expect("restarted package")
        .into_iter()
        .next()
        .expect("restarted static frame package");
    assert!(restarted_package.proof.adaptive_identification.is_some());
    let mut alternative = adaptive_bucket(
        &"e".repeat(64),
        &archetype,
        BTreeMap::from([(
            canonical_json_sha256(&program).expect("alternative digest"),
            program,
        )]),
        &support,
    );
    alternative.support.clear();
    alternative.runtime_examples.clear();
    miner.checkpoint.buckets.push(alternative);
    let candidates = miner
        .admission_candidates()
        .expect("static frame admission candidates");
    assert_eq!(candidates.len(), 1);
    let snapshot = crate::build_online_collection_admission_snapshot(
        &candidates,
        "project",
        1,
        100,
        60,
        &"d".repeat(64),
        &"e".repeat(64),
    )
    .expect("static frame external admission")
    .expect("static frame active snapshot");
    assert_eq!(snapshot.registry.packages.len(), 1);
    assert!(snapshot.admission.eligible_for_local_accept);
    fs::remove_dir_all(root).expect("cleanup");
}
