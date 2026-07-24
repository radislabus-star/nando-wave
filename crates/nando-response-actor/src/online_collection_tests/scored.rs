//! Restart, frozen-future, and external-admission test families.

use super::*;

#[test]
fn teacher_prose_trains_canonical_count_without_storing_static_text() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-canonical-count-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let mut miner = OnlineCollectionMiner::open(
        root.join("checkpoint.cbor"),
        OnlineCollectionConfig {
            proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        },
    )
    .expect("miner");
    miner
        .observe_replay_training_buffered(observation(
            1,
            "The verified batch contains 3 usable objects.",
        ))
        .expect("support");

    assert_eq!(miner.checkpoint.unsupported_total, 0);
    assert_eq!(
        miner
            .checkpoint
            .exact_executable_observations_total
            .saturating_add(miner.checkpoint.semantic_executable_observations_total),
        1
    );
    assert_eq!(miner.checkpoint.buckets.len(), 1);
    assert!(
        miner.checkpoint.buckets[0]
            .support
            .iter()
            .all(|receipt| receipt.verifier_pass)
    );
    assert!(miner.checkpoint.buckets[0].frozen_program_sha256.is_none());
    assert!(
        miner.checkpoint.buckets[0]
            .programs
            .values()
            .any(|program| {
                matches!(
                    &program.operation,
                    crate::ResponseOperation::ComposeCollection {
                        steps,
                        renderer: crate::CollectionOutputRenderer::Direct,
                        ..
                    } if matches!(steps.last(), Some(crate::CollectionProgramStep::Count))
                )
            })
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn output_ordinal_surfaces_share_one_count_law_and_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-output-ordinal-law-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let mut miner = OnlineCollectionMiner::open(
        root.join("checkpoint.cbor"),
        OnlineCollectionConfig {
            proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
            support_rows: 4,
            future_rows: 4,
            max_buckets: 8,
            max_receipts_per_bucket: 16,
        },
    )
    .expect("miner");
    for index in 1..=4 {
        let observation =
            ordinal_count_observation(index, "3", if index.is_multiple_of(2) { 2 } else { 1 });
        miner
            .observe_replay_training_buffered(observation)
            .expect("support");
    }
    for index in 5..=8 {
        let observation =
            ordinal_count_observation(index, "3", if index.is_multiple_of(2) { 2 } else { 1 });
        miner.observe(observation).expect("future");
    }

    let status = miner.status();
    assert_eq!(status.frozen_buckets_total, 1);
    assert_eq!(status.future_receipts_unique_total, 4);
    assert_eq!(status.runtime_parity_cases_total, 4);
    assert_eq!(status.wrong_accepts_total, 0);
    assert_eq!(miner.quarantine_packages().expect("packages").len(), 1);
    fs::remove_dir_all(root).ok();
}

#[test]
fn converged_unfrozen_program_pools_merge_without_touching_frozen_pool() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-merge-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 32,
        future_rows: 32,
        max_buckets: 64,
        max_receipts_per_bucket: 64,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    miner
        .observe_replay_training_buffered(observation(1, "3"))
        .expect("support");
    let original = miner.checkpoint.buckets[0].clone();
    let program_digests = original.programs.keys().cloned().collect::<Vec<_>>();
    let mut duplicate = original.clone();
    duplicate.bucket_id = "d".repeat(64);
    miner.checkpoint.buckets.push(duplicate);
    miner
        .merge_converged_unfrozen_buckets()
        .expect("merge converged");
    assert_eq!(
        miner
            .checkpoint
            .buckets
            .iter()
            .filter(|bucket| bucket.programs.keys().eq(program_digests.iter()))
            .count(),
        1
    );
    assert_eq!(miner.checkpoint.buckets[0].support.len(), 1);

    let mut frozen = miner.checkpoint.buckets[0].clone();
    frozen.bucket_id = "f".repeat(64);
    frozen.frozen_program_sha256 = frozen.programs.keys().next().cloned();
    miner.checkpoint.buckets.push(frozen);
    miner
        .merge_converged_unfrozen_buckets()
        .expect("keep frozen");
    assert_eq!(
        miner
            .checkpoint
            .buckets
            .iter()
            .filter(|bucket| bucket.programs.keys().eq(program_digests.iter()))
            .count(),
        2
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn overlapping_unfrozen_version_spaces_merge_to_proven_intersection() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-overlap-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 32,
        future_rows: 32,
        max_buckets: 64,
        max_receipts_per_bucket: 64,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    miner
        .observe_replay_training_buffered(observation(1, "3"))
        .expect("support");
    let original = miner.checkpoint.buckets[0].clone();
    let shared = original.programs.keys().next().cloned().expect("program");
    let shared_program = original.programs.get(&shared).cloned().expect("program");
    let mut left = original.clone();
    left.programs.retain(|digest, _| digest == &shared);
    let extra_left = canonical_json_sha256(&"left").expect("digest");
    left.programs.insert(extra_left, shared_program.clone());
    let mut right = original;
    right.bucket_id = "d".repeat(64);
    right.programs.retain(|digest, _| digest == &shared);
    let extra_right = canonical_json_sha256(&"right").expect("digest");
    right.programs.insert(extra_right, shared_program);
    miner.checkpoint.buckets = vec![left, right];
    miner
        .merge_converged_unfrozen_buckets()
        .expect("merge overlap");
    assert_eq!(miner.checkpoint.buckets.len(), 1);
    assert_eq!(miner.checkpoint.buckets[0].programs.len(), 3);
    assert!(miner.checkpoint.buckets[0].programs.contains_key(&shared));
    fs::remove_dir_all(root).ok();
}

#[test]
fn prefreeze_support_receipts_survive_restart_without_raw_examples() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-prefreeze-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    for index in 1..=2 {
        miner
            .observe_replay_training_buffered(observation(index, "3"))
            .expect("support before restart");
    }
    miner.flush().expect("flush support");
    let before = miner.status();
    assert_eq!(before.support_receipts_unique_total, 2);
    assert_eq!(before.future_receipts_unique_total, 0);
    assert_eq!(before.frozen_buckets_total, 0);
    drop(miner);

    let durable = fs::read(&path).expect("checkpoint");
    for private in [b"provider_payload".as_slice(), b"surface_".as_slice()] {
        assert!(
            !durable
                .windows(private.len())
                .any(|window| window == private),
            "checkpoint leaked replay input"
        );
    }

    let mut restored = OnlineCollectionMiner::open(&path, config).expect("restart");
    let after = restored.status();
    assert_eq!(after.support_receipts_unique_total, 2);
    assert_eq!(after.future_receipts_unique_total, 0);
    assert_eq!(after.unreplayable_support_discarded_total, 0);
    for index in 3..=4 {
        restored
            .observe_replay_training_buffered(observation(index, "3"))
            .expect("support after restart");
    }
    let frozen = restored.status();
    assert_eq!(frozen.support_receipts_unique_total, 4);
    assert_eq!(frozen.future_receipts_unique_total, 0);
    assert_eq!(frozen.frozen_buckets_total, 1);
    assert_eq!(frozen.wrong_accepts_total, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn v4_checkpoint_restarts_as_non_authoritative_teacher_history() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-v5-migration-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    for index in 1..=8 {
        miner.observe(observation(index, "3")).expect("evidence");
    }
    assert_eq!(miner.status().future_receipts_unique_total, 4);
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V4;
    miner.persist().expect("persist v4 marker");
    drop(miner);

    let migrated = OnlineCollectionMiner::open(&path, config).expect("migrated miner");
    let status = migrated.status();
    assert_eq!(
        status.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V38
    );
    assert_eq!(status.frozen_buckets_total, 0);
    assert_eq!(status.future_receipts_unique_total, 0);
    assert!(status.observation_accounting_complete);
    assert_eq!(
        status.teacher_only_observations_total,
        status.observations_total
    );
    assert!(migrated.checkpoint.buckets.iter().all(|bucket| {
        bucket.frozen_program_sha256.is_none()
            && bucket.future.is_empty()
            && bucket.support.is_empty()
    }));
    assert!(status.unreplayable_support_discarded_total >= 4);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn v7_repairs_historical_variant_digest_and_revokes_invalid_freeze() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-v7-witness-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let config = legacy_default_config();
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    let observation = observation(1, "3");
    let program = enumerate_source_neutral_response_programs(&observation.example)
        .expect("programs")
        .programs
        .into_iter()
        .find(|program| {
            crate::response_program_exactly_matches_example(program, &observation.example)
                && is_privacy_safe_online_response_program(program)
        })
        .expect("exact program");
    let program_digest = canonical_json_sha256(&program).expect("program digest");
    let programs = BTreeMap::from([(program_digest.clone(), program.clone())]);
    let mut receipt = receipt_with_program_atoms(&observation, true, &programs).expect("receipt");
    let historical_digest = "c".repeat(64);
    receipt
        .matched_program_sha256
        .push(historical_digest.clone());
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V7;
    miner.checkpoint.buckets = vec![OnlineCollectionBucket {
        bucket_id: "legacy-v7-witness".to_owned(),
        archetype_id: response_program_archetype_id(&program).expect("archetype"),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support: vec![receipt.clone()],
        future: vec![receipt],
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: Some(program_digest),
        support_watermark_event_time_unix_nanos: Some(1),
        support_manifest_sha256: Some("d".repeat(64)),
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::from([42]),
        wrong_accepts: 0,
    }];
    miner.persist().expect("persist v7 checkpoint");
    drop(miner);

    let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v9");
    let bucket = migrated.checkpoint.buckets.first().expect("bucket");
    assert_eq!(
        migrated.status().pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V38
    );
    assert!(bucket.rejected_program_sha256.contains(&historical_digest));
    assert!(bucket.learned_anti_atom_ids.is_empty());
    assert!(bucket.support.is_empty());
    assert!(bucket.future.is_empty());
    assert!(bucket.frozen_program_sha256.is_none());
    assert!(bucket.support_manifest_sha256.is_none());
    assert!(migrated.status().unreplayable_support_discarded_total >= 1);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_program_pool_reuses_across_rephrased_requests() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-rephrased-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    miner
        .observe(routed_observation(
            1,
            "Total records: 3",
            "Count records for this batch",
            false,
        ))
        .expect("first surface");
    miner
        .observe(routed_observation(
            2,
            "Total records: 3",
            "How many entries are present?",
            true,
        ))
        .expect("rephrased surface");
    let status = miner.status();
    assert_eq!(status.buckets_total, 1);
    assert_eq!(status.program_pool_reuse_total, 1);
    assert_eq!(status.buckets[0].support_rows, 2);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn latest_output_program_pool_reuses_across_different_output_ordinals() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-latest-output-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    for (index, preceding_outputs) in [(1_usize, 1_usize), (2, 3)] {
        let mut input = vec![json!({
            "type":"message",
            "role":"user",
            "content":format!("Select the completed value for layout {index}")
        })];
        input.extend((0..preceding_outputs).map(|ordinal| {
            json!({
                "type":"function_call_output",
                "output":format!("unrelated-{index}-{ordinal}")
            })
        }));
        input.push(json!({
            "type":"function_call_output",
            "output":"header\nselected-result\nfooter"
        }));
        miner
            .observe(OnlineCollectionObservation {
                evidence_graph_sha256: format!("{:064x}", index + 30_000),
                client_intent_id_sha256: format!("{:064x}", index + 40_000),
                session_id_sha256: format!("{:064x}", index + 50_000),
                event_time_unix_nanos: Some(index as u64),
                estimated_input_tokens: 100,
                capture_binding: None,
                example: CollectionSynthesisExample {
                    provider_payload: json!({"input":input}),
                    expected_response: "selected-result".to_owned(),
                },
            })
            .expect("observation");
    }
    assert_eq!(miner.checkpoint.buckets.len(), 1);
    let bucket = &miner.checkpoint.buckets[0];
    assert_eq!(bucket.support.len(), 2);
    assert!(bucket.programs.values().any(|program| {
        matches!(
            &program.operation,
            crate::ResponseOperation::ProjectSelectedValue {
                selector: crate::ResponseValueSelector::LatestTurnOutputLine { line_index: 1, .. },
                ..
            }
        )
    }));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn replay_training_freezes_support_but_never_claims_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-replay-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    for index in 1..=8 {
        miner
            .observe_replay_training_buffered(observation(index, "3"))
            .expect("replay support");
    }
    miner.flush().expect("flush replay support");
    let replay_status = miner.status();
    assert_eq!(replay_status.buckets.len(), 1);
    assert!(replay_status.buckets[0].frozen);
    assert_eq!(replay_status.buckets[0].support_rows, 4);
    assert_eq!(replay_status.buckets[0].future_rows, 0);

    for index in 9..=12 {
        let mut live = observation(index, "3");
        live.session_id_sha256 = format!("{:064x}", index + 60_000);
        miner.observe(live).expect("live future");
    }
    let live_status = miner.status();
    assert_eq!(live_status.buckets[0].future_rows, 4);
    assert_eq!(live_status.buckets[0].wrong_accepts, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn replay_training_rehydrates_discarded_support_without_double_accounting() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-rehydrate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    let evidence = observation(1, "3");
    miner
        .observe(evidence.clone())
        .expect("initial observation");
    assert_eq!(miner.status().observations_total, 1);
    assert_eq!(miner.status().support_receipts_unique_total, 1);

    for bucket in &mut miner.checkpoint.buckets {
        bucket.support.clear();
        bucket.runtime_examples.clear();
    }
    assert_eq!(miner.status().support_receipts_unique_total, 0);

    miner
        .observe_replay_training_buffered(evidence.clone())
        .expect("rehydrate discarded support");
    let repaired = miner.status();
    assert_eq!(repaired.observations_total, 1);
    assert_eq!(repaired.duplicate_observations_total, 0);
    assert_eq!(repaired.support_receipts_unique_total, 1);
    assert!(repaired.observation_accounting_complete);

    let stale_digest = "f".repeat(64);
    miner.checkpoint.buckets[0].support[0].matched_program_sha256 = vec![stale_digest];
    miner
        .observe_replay_training_buffered(evidence)
        .expect("refresh retained stale receipt");
    let refreshed = miner.status();
    assert_eq!(refreshed.observations_total, 1);
    assert_eq!(refreshed.duplicate_observations_total, 0);
    assert_eq!(refreshed.support_receipts_unique_total, 1);
    assert!(
        miner.checkpoint.buckets[0].support[0]
            .matched_program_sha256
            .iter()
            .any(|digest| miner.checkpoint.buckets[0].programs.contains_key(digest))
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn v20_restart_preserves_support_without_bulk_revalidation() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-v20-revalidate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.cbor");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    miner
        .observe(observation(1, "3"))
        .expect("initial observation");
    let observations = miner.status().observations_total;
    let support = miner.status().support_receipts_unique_total;
    let original_digest = miner.checkpoint.buckets[0].support[0]
        .matched_program_sha256
        .first()
        .cloned()
        .expect("at least one proven program link");
    let mut equivalent = miner.checkpoint.buckets[0].programs[&original_digest].clone();
    equivalent.max_output_bytes = equivalent.max_output_bytes.saturating_add(1);
    let equivalent_digest = canonical_json_sha256(&equivalent).expect("equivalent digest");
    assert_ne!(equivalent_digest, original_digest);
    miner.checkpoint.buckets[0]
        .programs
        .insert(equivalent_digest.clone(), equivalent);
    assert!(
        !miner.checkpoint.buckets[0].support[0]
            .matched_program_sha256
            .contains(&equivalent_digest)
    );
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V19;
    miner.persist().expect("persist v19 checkpoint");
    drop(miner);

    let restored = OnlineCollectionMiner::open(&path, config).expect("migrate to v20");
    let status = restored.status();
    assert_eq!(
        status.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V38
    );
    assert_eq!(status.observations_total, observations);
    assert_eq!(status.support_receipts_unique_total, support);
    assert_eq!(status.future_receipts_unique_total, 0);
    assert_eq!(status.wrong_accepts_total, 0);
    assert!(
        !restored.checkpoint.buckets[0].support[0]
            .matched_program_sha256
            .contains(&equivalent_digest)
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn legacy_rehydration_joins_unique_verified_receipt_across_identity_and_layout_versions() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-legacy-rehydrate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    let original = observation(1, "3");
    miner
        .observe_replay_training_buffered(original.clone())
        .expect("legacy support");

    let legacy_evidence = "e".repeat(64);
    let legacy_session = "d".repeat(64);
    let bucket = &mut miner.checkpoint.buckets[0];
    bucket.runtime_examples.clear();
    bucket.support[0].evidence_graph_sha256 = legacy_evidence.clone();
    bucket.support[0].session_id_sha256 = legacy_session.clone();

    let mut reconstructed = original;
    reconstructed.evidence_graph_sha256 = "c".repeat(64);
    reconstructed.session_id_sha256 = "b".repeat(64);
    reconstructed.example.provider_payload["migration_layout_marker"] = json!(true);
    let stats = miner
        .rehydrate_legacy_replay_training_buffered(reconstructed, &BTreeSet::from([legacy_session]))
        .expect("verified legacy join");
    assert_eq!(stats.session_receipts, 1);
    assert_eq!(stats.event_time_matches, 1);
    assert_eq!(stats.token_matches, 1);
    assert_eq!(stats.verifier_matches, 1);
    assert_eq!(stats.layout_matches, 0);
    assert_eq!(stats.ambiguous_matches, 0);
    assert_eq!(stats.attached_receipts, 1);
    assert!(
        miner.checkpoint.buckets[0]
            .runtime_examples
            .contains_key(&legacy_evidence)
    );
    assert_eq!(miner.status().future_receipts_unique_total, 0);
    assert_eq!(miner.status().wrong_accepts_total, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn frozen_wave_center_uses_support_invariants_across_new_request_wording() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-invariant-wave-center-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = legacy_default_config();
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    let support_wordings = [
        "Count records for this batch",
        "How many entries are present?",
        "Return the collection cardinality",
        "Determine the row total",
        "Summarize the number of items",
        "Report how large this list is",
        "Give the amount of matching objects",
        "Calculate the payload size in rows",
    ];
    for index in 1..=32 {
        miner
            .observe_replay_training_buffered(routed_observation(
                index,
                "Total records: 3",
                support_wordings[(index - 1) % support_wordings.len()],
                index.is_multiple_of(2),
            ))
            .expect("support");
    }
    miner.flush().expect("flush support");
    let bucket = miner
        .checkpoint
        .buckets
        .iter()
        .find(|bucket| bucket.frozen_program_sha256.is_some())
        .expect("frozen bucket");
    let mut expected_center = bucket_program_atom_ids(bucket);
    expected_center.extend(bucket.common_request_atom_ids.iter().copied());
    assert_eq!(
        bucket_phase_center_atom_ids(bucket)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected_center
    );

    let future = routed_observation(
        10_000,
        "Total records: 3",
        "Provide the cardinality using this unseen wording",
        true,
    );
    let frozen_program = bucket
        .frozen_program_sha256
        .as_ref()
        .and_then(|digest| bucket.programs.get(digest))
        .expect("frozen program");
    assert_eq!(
        independently_verified_authority_response(frozen_program, &future.example).as_deref(),
        Some(future.example.expected_response.as_str()),
        "frozen program did not transfer: {frozen_program:#?}"
    );
    miner.observe(future).expect("future");
    let status = miner.status();
    assert_eq!(status.future_receipts_unique_total, 1, "{status:#?}");
    assert_eq!(status.frozen_future_accepted_total, 1);
    assert!(status.frozen_route_accounting_complete);
    assert_eq!(status.wrong_accepts_total, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn frozen_consensus_counterexample_creates_witness_successor() {
    let example = observation(1, "3");
    let witness_example = observation(10, "3");
    let version_space =
        enumerate_source_neutral_response_programs(&example.example).expect("version space");
    let correct = version_space
        .programs
        .into_iter()
        .find(|program| {
            program_any(program, is_count_operation)
                && independently_verified_authority_response(program, &example.example).as_deref()
                    == Some("3")
                && independently_verified_authority_response(program, &witness_example.example)
                    .as_deref()
                    == Some("3")
        })
        .expect("count program");
    let mut competing = correct.clone();
    let crate::ResponseOperation::ComposeCollection { renderer, .. } = &mut competing.operation
    else {
        panic!("count operation");
    };
    *renderer = crate::CollectionOutputRenderer::RenderTemplate {
        prefix: "(".to_owned(),
        suffix: ")".to_owned(),
    };
    let correct_digest = canonical_json_sha256(&correct).expect("correct digest");
    let competing_digest = canonical_json_sha256(&competing).expect("competing digest");
    let consensus = ResponseProgram::unique_consensus(vec![
        ResponseConsensusVariant {
            program: correct,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        },
        ResponseConsensusVariant {
            program: competing,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        },
    ]);
    let consensus_digest = canonical_json_sha256(&consensus).expect("consensus digest");
    assert!(independently_verified_authority_response(&consensus, &example.example).is_none());
    let support_programs = BTreeMap::from([(
        correct_digest.clone(),
        match &consensus.operation {
            crate::ResponseOperation::UniqueConsensus { variants, .. } => {
                variants[0].program.clone()
            }
            _ => unreachable!(),
        },
    )]);
    let support = (1..=4)
        .map(|index| {
            receipt_with_program_atoms(&observation(index, "3"), true, &support_programs)
                .expect("support receipt")
        })
        .collect::<Vec<_>>();
    let bucket = OnlineCollectionBucket {
        bucket_id: "a".repeat(64),
        archetype_id: "count".to_owned(),
        programs: BTreeMap::from([(consensus_digest.clone(), consensus)]),
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: Some(consensus_digest.clone()),
        support_watermark_event_time_unix_nanos: Some(4),
        support_manifest_sha256: Some("b".repeat(64)),
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    let decision = active_witness_decision(&bucket, &consensus_digest, &witness_example, 16)
        .expect("witness decision");
    let ActiveWitnessDecision::Successor {
        bucket: successor,
        resolved,
    } = decision
    else {
        panic!("witness successor");
    };
    assert!(resolved);
    assert_eq!(successor.programs.len(), 1);
    assert!(successor.programs.contains_key(&correct_digest));
    assert!(!successor.programs.contains_key(&competing_digest));
    let witness = successor.support.last().expect("witness receipt");
    assert_eq!(witness.witness_round, Some(1));
    assert_eq!(witness.witness_candidates_before, Some(2));
    assert_eq!(witness.witness_candidates_after, Some(1));
    assert!(valid_witness_receipt_metadata(witness));
}

#[test]
fn frozen_future_accepts_independently_verified_canonical_response() {
    assert_eq!(authority_rejection_reason(&Ok("3".to_owned())), None);
    assert_eq!(
        authority_rejection_reason(&Err("actor_abstain")),
        Some("actor_abstain")
    );
    assert!(!is_hard_teacher_counterexample("authority_mismatch"));
    assert!(!is_hard_teacher_counterexample("actor_abstain"));
    assert!(is_hard_teacher_counterexample("verifier_rejected"));
}

#[test]
fn collection_wave_ablation_builds_external_admission_after_frozen_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-admission-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let config = legacy_default_config();
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.json"), config).expect("miner");
    for index in 1..=32 {
        miner
            .observe(routed_observation(
                index,
                "3",
                "Count records in the verified collection",
                false,
            ))
            .expect("support");
    }
    miner
        .observe(routed_observation(
            10_000,
            "Rows: 3",
            "Summarize the selected payload",
            false,
        ))
        .expect("competing family");
    for index in 33..=64 {
        miner
            .observe(routed_observation(
                index,
                "3",
                "Count records in the verified collection",
                index % 2 == 0,
            ))
            .expect("future");
    }
    let candidates = miner.admission_candidates().expect("candidates");
    assert_eq!(candidates.len(), 1, "{:#?}", miner.status());
    let candidate = &candidates[0];
    assert_eq!(candidate.causal_report.verdict, "PASS");
    assert_eq!(candidate.causal_report.full_phase_correct, 32);
    assert!(
        candidate.causal_report.full_phase_exact_checks
            < candidate.causal_report.no_phase_exact_checks
    );
    assert_eq!(candidate.package.state, ResponsePackageState::Active);
    let snapshot = crate::build_online_collection_admission_snapshot(
        &candidates,
        "project",
        1,
        100,
        60,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission")
    .expect("authorized snapshot");
    assert_eq!(snapshot.registry.packages.len(), 1);
    assert!(snapshot.admission.eligible_for_local_accept);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn applicability_anti_atom_requires_three_distinct_sessions() {
    let mut evidence = BTreeMap::new();
    let candidates = BTreeSet::from([7, 9]);

    assert!(
        update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-a")
            .is_empty()
    );
    assert!(
        update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-a")
            .is_empty()
    );
    assert!(
        update_applicability_negative_sessions(&mut evidence, candidates.clone(), "session-b")
            .is_empty()
    );
    assert_eq!(
        update_applicability_negative_sessions(&mut evidence, candidates, "session-c"),
        BTreeSet::from([7, 9])
    );
}
